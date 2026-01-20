use std::{
    fs::File,
    io::{Read, Write},
    iter::FusedIterator,
    mem::MaybeUninit,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not},
    os::fd::{AsFd, AsRawFd},
    slice,
    thread::{self, sleep},
    time::Duration,
};

use log::trace;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use sd_notify::NotifyState;
use systemd_journal_logger::JournalLog;
use usb_gadget::{
    Class, Config, Gadget, Id, Strings, default_udc,
    function::{Handle, hid::Hid},
};

use crate::shared::{MAX_CHARGE_MA, MAX_SYSTEM_MA, usb_config};

mod shared;

const KEYBOARD_REPORT_DESCRIPTOR: [u8; 69] = [
    0x5, 0x1, 0x9, 0x6, 0xA1, 0x1, 0x5, 0x7, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x0, 0x25, 0x1, 0x75,
    0x1, 0x95, 0x8, 0x81, 0x2, 0x19, 0x0, 0x29, 0xFF, 0x26, 0xFF, 0x0, 0x75, 0x8, 0x95, 0x1, 0x81,
    0x3, 0x5, 0x8, 0x19, 0x1, 0x29, 0x5, 0x25, 0x1, 0x75, 0x1, 0x95, 0x5, 0x91, 0x2, 0x95, 0x3,
    0x91, 0x3, 0x5, 0x7, 0x19, 0x0, 0x29, 0xDD, 0x26, 0xFF, 0x0, 0x75, 0x8, 0x95, 0x6, 0x81, 0x0,
    0xC0,
];

pub const PLOVER_OUTPUT: &str = "/tmp/plover_output";
/// From ASCII table.
pub const END_OF_TEXT: u8 = 0x03;

/// Special handling indicator using unused ASCII characters.
pub const KEY_PRESSED: u8 = 0x81;
/// Special handling indicator using unused ASCII characters.
pub const KEY_RELEASED: u8 = 0x8D;

pub struct KeyboardReport {
    pub modifiers: u8,
    pub keycodes: [u8; 6],
}

impl Default for KeyboardReport {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardReport {
    pub fn new() -> Self {
        Self {
            modifiers: 0,
            keycodes: [0; 6],
        }
    }

    pub fn full(&self) -> [u8; 8] {
        // Always leave reserved and LEDs as zero.
        let mut full = [0; 8];
        full[0] = self.modifiers;
        full[2..].copy_from_slice(&self.keycodes);
        full
    }
}

/// Reads the `plover_output` fifo into `buf`, returning the filled bytes.
pub fn read_plover<'a>(plover_output: &mut File, mut buf: &'a mut [u8]) -> &'a [u8] {
    poll(
        &mut [PollFd::new(plover_output.as_fd(), PollFlags::POLLIN)],
        PollTimeout::NONE,
    )
    .unwrap();

    let mut total_read_len = 0;

    loop {
        let read_len = plover_output.read(buf).unwrap();
        if read_len == 0 {
            break;
        } else {
            total_read_len += read_len;
            buf = &mut buf[read_len..];
        }

        let bytes_remaining = poll(
            &mut [PollFd::new(plover_output.as_fd(), PollFlags::POLLIN)],
            PollTimeout::ZERO,
        )
        .unwrap();
        if bytes_remaining == 0 {
            break;
        }
    }

    &buf[..total_read_len]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModifierKeys {
    // [control, shift, alt, super, ALL ZERO]
    pub bitfield: u8,
}

impl ModifierKeys {
    pub fn none() -> Self {
        Self { bitfield: 0 }
    }

    pub fn control_pressed() -> Self {
        Self { bitfield: 1 }
    }

    pub fn shift_pressed() -> Self {
        Self { bitfield: 1 << 1 }
    }

    pub fn alt_pressed() -> Self {
        Self { bitfield: 1 << 2 }
    }

    pub fn super_pressed() -> Self {
        Self { bitfield: 1 << 3 }
    }

    /// Moves the slice forward and returns self on any control sequence.
    ///
    /// Returns None and does not modification otherwise.
    pub fn from_plover_combo(combo: &mut &[u8]) -> Option<Self> {
        let mut seq_match = |sequence, this| {
            if combo.starts_with(sequence) {
                *combo = &combo[sequence.len()..];
                Some(this)
            } else {
                None
            }
        };

        seq_match("control".as_bytes(), Self::control_pressed())
            .or(seq_match("shift".as_bytes(), Self::shift_pressed()))
            .or(seq_match("super".as_bytes(), Self::super_pressed()))
            .or(seq_match("alt".as_bytes(), Self::alt_pressed()))
    }
}

impl BitOr for ModifierKeys {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.bitfield |= rhs.bitfield;
        self
    }
}

impl BitOrAssign for ModifierKeys {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitAnd for ModifierKeys {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self.bitfield &= rhs.bitfield;
        self
    }
}

impl BitAndAssign for ModifierKeys {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl Not for ModifierKeys {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        self.bitfield = !self.bitfield;
        self
    }
}

impl From<ModifierKeys> for u8 {
    fn from(value: ModifierKeys) -> Self {
        value.bitfield
    }
}

#[derive(Debug, Clone, Copy)]
struct KeyPress {
    modifiers: ModifierKeys,
    value: u8,
}

impl KeyPress {
    pub fn from_ascii(code: u8) -> Self {
        Self::ascii_match(code).unwrap_or_else(|| panic!("Unhandled code: {code}"))
    }

    fn ascii_match(code: u8) -> Option<Self> {
        Some(match code {
            uppercase_code if (uppercase_code > 0x40) && (uppercase_code < 0x5B) => {
                // From https://gist.github.com/ekaitz-zarraga/2b25b94b711684ba4e969e5a5723969b
                let value = (uppercase_code - 0x41) + 0x04;
                Self {
                    modifiers: ModifierKeys::shift_pressed(),
                    value,
                }
            }
            lowercase_code if (lowercase_code > 0x60) && (lowercase_code < 0x7B) => {
                // From https://gist.github.com/ekaitz-zarraga/2b25b94b711684ba4e969e5a5723969b
                let value = (lowercase_code - 0x61) + 0x04;
                Self {
                    modifiers: ModifierKeys::none(),
                    value,
                }
            }
            number_code if (number_code > 0x30) && (number_code < 0x3A) => {
                // From https://gist.github.com/ekaitz-zarraga/2b25b94b711684ba4e969e5a5723969b
                let value = (number_code - 0x31) + 0x1E;
                Self {
                    modifiers: ModifierKeys::none(),
                    value,
                }
            }
            // -- specific mappings -- //
            // 0
            0x30 => Self {
                modifiers: ModifierKeys::none(),
                value: 0x27,
            },
            // !
            0x21 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x1E,
            },
            // @
            0x40 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x1F,
            },
            // #
            0x23 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x20,
            },
            // $
            0x24 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x21,
            },
            // %
            0x25 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x22,
            },
            // ^
            0x5E => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x23,
            },
            // &
            0x26 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x24,
            },
            // *
            0x2A => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x25,
            },
            // (
            0x28 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x26,
            },
            // )
            0x29 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x27,
            },
            // Return (ENTER)
            0x0D => Self {
                modifiers: ModifierKeys::none(),
                value: 0x28,
            },
            // ESCAPE
            0x1B => Self {
                modifiers: ModifierKeys::none(),
                value: 0x29,
            },
            // DELETE (Backspace)
            0x08 => Self {
                modifiers: ModifierKeys::none(),
                value: 0x2A,
            },
            // DELETE Forward
            0x4C => Self {
                modifiers: ModifierKeys::none(),
                value: 0x7F,
            },
            // Tab
            0x09 => Self {
                modifiers: ModifierKeys::none(),
                value: 0x2B,
            },
            // Spacebar
            0x20 => Self {
                modifiers: ModifierKeys::none(),
                value: 0x2C,
            },
            // -
            0x2D => Self {
                modifiers: ModifierKeys::none(),
                value: 0x2D,
            },
            // _
            0x5F => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x2D,
            },
            // =
            0x3D => Self {
                modifiers: ModifierKeys::none(),
                value: 0x2E,
            },
            // +
            0x2B => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x2E,
            },
            // [
            0x5B => Self {
                modifiers: ModifierKeys::none(),
                value: 0x2F,
            },
            // {
            0x7B => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x2F,
            },
            // ]
            0x5D => Self {
                modifiers: ModifierKeys::none(),
                value: 0x30,
            },
            // }
            0x7D => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x30,
            },
            // \
            0x5C => Self {
                modifiers: ModifierKeys::none(),
                value: 0x31,
            },
            // |
            0x7C => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x31,
            },
            // ;
            0x3B => Self {
                modifiers: ModifierKeys::none(),
                value: 0x33,
            },
            // :
            0x3A => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x33,
            },
            // '
            0x27 => Self {
                modifiers: ModifierKeys::none(),
                value: 0x34,
            },
            // "
            0x22 => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x34,
            },
            // `
            0x60 => Self {
                modifiers: ModifierKeys::none(),
                value: 0x35,
            },
            // ~
            0x7E => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x35,
            },
            // ,
            0x2C => Self {
                modifiers: ModifierKeys::none(),
                value: 0x36,
            },
            // <
            0x3C => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x36,
            },
            // .
            0x2E => Self {
                modifiers: ModifierKeys::none(),
                value: 0x37,
            },
            // >
            0x3E => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x37,
            },
            // /
            0x2F => Self {
                modifiers: ModifierKeys::none(),
                value: 0x38,
            },
            // ?
            0x3F => Self {
                modifiers: ModifierKeys::shift_pressed(),
                value: 0x38,
            },
            _ => return None,
        })
    }
}

#[derive(Debug)]
struct PloverKeyIter<'a> {
    buf: &'a [u8],
    stored_mods: ModifierKeys,
}

impl<'a> From<&'a [u8]> for PloverKeyIter<'a> {
    fn from(buf: &'a [u8]) -> Self {
        Self {
            buf,
            stored_mods: ModifierKeys::none(),
        }
    }
}

impl Iterator for PloverKeyIter<'_> {
    type Item = KeyPress;

    fn next(&mut self) -> Option<Self::Item> {
        let (next_char, remaining) = self.buf.split_first()?;
        self.buf = remaining;

        match *next_char {
            KEY_PRESSED => {
                if let Some(modifier) = ModifierKeys::from_plover_combo(&mut self.buf) {
                    // Add to modifiers
                    self.stored_mods |= modifier;
                    self.next()
                } else {
                    // Not mod, get the pressed key
                    let (next_char, remaining) = self.buf.split_first()?;
                    self.buf = remaining;
                    let mut key = KeyPress::from_ascii(*next_char);

                    // Replace modifiers before sending
                    key.modifiers = self.stored_mods;
                    Some(key)
                }
            }
            KEY_RELEASED => {
                if let Some(modifier) = ModifierKeys::from_plover_combo(&mut self.buf) {
                    // Remove from modifiers
                    self.stored_mods &= !modifier;
                    self.next()
                } else {
                    let (_next_char, remaining) = self.buf.split_first()?;
                    self.buf = remaining;

                    // Released regular keys are ignored
                    if cfg!(debug_assertions) {
                        let _key = KeyPress::from_ascii(*_next_char);
                    }
                    self.next()
                }
            }
            x => {
                debug_assert_eq!(
                    self.stored_mods,
                    ModifierKeys::none(),
                    "{:?} is not blank! Plover should always undo the sequence pressed keys.",
                    self.stored_mods
                );

                Some(KeyPress::from_ascii(x))
            }
        }
    }
}

impl FusedIterator for PloverKeyIter<'_> {}

#[derive(Debug)]
struct KeysToReports<'a> {
    key_iter: PloverKeyIter<'a>,
    trailing_packet: Option<KeyPress>,
}

impl Iterator for KeysToReports<'_> {
    type Item = KeyboardReport;

    fn next(&mut self) -> Option<Self::Item> {
        let packet_init = self.trailing_packet.take().or(self.key_iter.next())?;
        let packet_modifiers = packet_init.modifiers;

        let mut keycodes = [0; 6];
        keycodes[0] = packet_init.value;

        for idx in 1..keycodes.len() {
            if let Some(next_packet) = self.key_iter.next() {
                let mod_match = next_packet.modifiers == packet_modifiers;
                let no_dup = !keycodes[..idx].contains(&next_packet.value);

                if mod_match && no_dup {
                    keycodes[idx] = next_packet.value;
                } else {
                    self.trailing_packet = Some(next_packet);
                    break;
                }
            } else {
                break;
            }
        }

        Some(KeyboardReport {
            modifiers: packet_modifiers.into(),
            keycodes,
        })
    }
}

impl FusedIterator for KeysToReports<'_> {}

impl<'a> From<&'a [u8]> for KeysToReports<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self {
            key_iter: PloverKeyIter::from(value),
            trailing_packet: None,
        }
    }
}

fn main() {
    JournalLog::new().unwrap().install().unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });

    let mut keyboard = Hid::builder();
    // Boot interface supported
    keyboard.sub_class = 1;
    // Keyboard protocol
    keyboard.protocol = 1;
    keyboard.report_len = 8;
    keyboard.report_desc = KEYBOARD_REPORT_DESCRIPTOR.to_vec();

    let hid = shared::init(Some((keyboard, "Keyboard Translation Mode"))).unwrap();

    let mut keyboard_out_file = {
        let (major, minor) = hid.device().unwrap();

        File::options()
            .append(true)
            .open(format!("/dev/char/{major}:{minor}"))
            .unwrap()
    };

    if log::max_level() >= log::LevelFilter::Trace {
        thread::spawn(move || {
            loop {
                sleep(Duration::from_secs(30));
                trace!("HID Status = {:?}", hid.status());
            }
        });
    }

    // Buffer size is arbitrary
    let mut buf = [0; 2_usize.pow(16)];
    let mut plover_output = File::open(PLOVER_OUTPUT).unwrap();

    sd_notify::notify(true, &[NotifyState::Ready]).unwrap();

    // Regular operation loop that translates plover output to USB keystrokes.
    loop {
        let read_buf = read_plover(&mut plover_output, &mut buf);

        for mut report in KeysToReports::from(read_buf) {
            keyboard_out_file.write_all(&report.full()).unwrap();

            // Always follow with an empty report so keycodes register.
            report.keycodes = [0; 6];
            keyboard_out_file.write_all(&report.full()).unwrap();
        }
    }
}
