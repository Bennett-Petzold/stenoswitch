use std::{fs::File, thread::sleep, time::Duration};

use systemd_journal_logger::JournalLog;
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

fn main() {
    /*
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

    loop {
        sleep(Duration::from_secs(1));
        println!("HID Status = {:?}", hid.status());
    }
    */
}
