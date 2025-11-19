#![feature(portable_simd)]

use std::io::Write;

use log::{debug, info};
use systemd_journal_logger::JournalLog;
use virtual_serialport::VirtualPort;

use crate::scan::KeyScanner;

mod scan;

// General fastest baud for serial protocols.
const GEMINI_BAUD_RATE: u32 = 115_200;
// Arbitrary.
const SERIAL_BUFFER_SIZE: u32 = 1024;

fn main() {
    JournalLog::new().unwrap().install().unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });

    let mut scanner = KeyScanner::new().unwrap();
    info!("Initialized key scanner");

    let mut port = VirtualPort::loopback(GEMINI_BAUD_RATE, SERIAL_BUFFER_SIZE).unwrap();
    info!("Initialized virtual port");

    sd_notify::notify(true, &[sd_notify::NotifyState::Ready]).unwrap();

    loop {
        scanner.wait_for_input();
        let packet = scanner.scan().unwrap();
        debug!("Gemini packet: {packet:?}");
        port.write(&*packet).unwrap();
    }
}
