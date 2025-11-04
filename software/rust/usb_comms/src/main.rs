use std::{fs::File, thread::sleep, time::Duration};

use usb_gadget::{
    Class, Config, Gadget, Id, Strings, default_udc,
    function::{Handle, hid::Hid},
};

const KEYBOARD_REPORT_DESCRIPTOR: [u8; 69] = [
    0x5, 0x1, 0x9, 0x6, 0xA1, 0x1, 0x5, 0x7, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x0, 0x25, 0x1, 0x75,
    0x1, 0x95, 0x8, 0x81, 0x2, 0x19, 0x0, 0x29, 0xFF, 0x26, 0xFF, 0x0, 0x75, 0x8, 0x95, 0x1, 0x81,
    0x3, 0x5, 0x8, 0x19, 0x1, 0x29, 0x5, 0x25, 0x1, 0x75, 0x1, 0x95, 0x5, 0x91, 0x2, 0x95, 0x3,
    0x91, 0x3, 0x5, 0x7, 0x19, 0x0, 0x29, 0xDD, 0x26, 0xFF, 0x0, 0x75, 0x8, 0x95, 0x6, 0x81, 0x0,
    0xC0,
];

pub struct KeyboardReport {
    //pub modifier: u8,
    //pub reserved: u8,
    //pub leds: u8,
    //pub keycodes: [u8; 6],
    pub modifier: u8,
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
            modifier: 0,
            keycodes: [0; 6],
        }
    }

    pub fn full(&self) -> [u8; 9] {
        // Always leave reserved and LEDs as zero.
        let mut full = [0; 9];
        full[0] = self.modifier;
        full[3..].copy_from_slice(&self.keycodes);
        full
    }
}

/// From R_{ISET}.
const MAX_CHARGE_MA: u16 = 1600;
/// From R_{OLIM}.
const MAX_SYSTEM_MA: u16 = 200;

fn usb_config(description: &str, function_handle: Handle) -> Config {
    let mut config =
        Config::new("Stenoswitch ".to_string() + description).with_function(function_handle);
    config.max_power = MAX_CHARGE_MA + MAX_SYSTEM_MA;
    config.self_powered = false;
    config.remote_wakeup = true;
    config
}

fn main() {
    let udc = default_udc().unwrap();
    usb_gadget::remove_all().unwrap();

    let mut keyboard = Hid::builder();
    // Boot interface supported
    keyboard.sub_class = 1;
    // Keyboard protocol
    keyboard.protocol = 1;
    keyboard.report_len = 8;
    keyboard.report_desc = KEYBOARD_REPORT_DESCRIPTOR.to_vec();
    let (hid, handle) = keyboard.build();

    let reg = Gadget::new(
        Class::interface_specific(),
        // Testing USB ID
        Id::new(0x1209, 0x001),
        // TODO: get serial from RPi
        Strings::new("Bennett Petzold", "Stenoswitch", "0"),
    )
    .with_config(usb_config("Keyboard Mode", handle))
    .bind(&udc)
    .unwrap();

    println!("PATH: {:#?}", reg.path());
    let (major, minor) = hid.device().unwrap();

    let _keyboard_file = File::options()
        .append(true)
        .open(format!("/dev/char/{major}:{minor}"))
        .unwrap();

    loop {
        sleep(Duration::from_secs(1));
        println!("HID Status = {:?}", hid.status());
        println!("UDC State = {:?}", udc.state());
    }
}
