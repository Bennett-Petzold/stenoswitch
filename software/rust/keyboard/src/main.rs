#![feature(portable_simd)]

use std::{fs::remove_file, io::Write, process::Command, sync::mpsc, thread, time::Duration};

use log::{debug, info};
use serialport::{Parity, SerialPort};
use systemd_journal_logger::JournalLog;
use virtual_serialport::VirtualPort;

use crate::scan::KeyScanner;

mod scan;

// General fastest baud for serial protocols.
const GEMINI_BAUD_RATE: u32 = 115_200;
// Arbitrary.
const SERIAL_BUFFER_SIZE: u32 = 1024;
// Arbitrary.
const PACKET_DELAY_SIZE: usize = 1024;

fn main() {
    JournalLog::new().unwrap().install().unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });

    let (put, recv) = mpsc::sync_channel(PACKET_DELAY_SIZE);

    thread::spawn(move || {
        let mut scanner = KeyScanner::new().unwrap();
        info!("Initialized key scanner");

        scanner.verify_scan();

        loop {
            scanner.wait_for_input();
            let packet = scanner.scan().unwrap();
            debug!("Gemini packet: {packet:?}");
            put.send(packet).unwrap();
        }
    });

    let _ = remove_file("/tmp/ttyKeyboardIn");
    let _ = remove_file("/tmp/ttyKeyboardOut");
    /*
    let _socat = Command::new("socat")
        .args([
            "pty,rawer,link=/tmp/ttyKeyboardIn,b115200",
            "pty,rawer,link=/tmp/ttyKeyboardOut,b115200",
        ])
        .spawn()
        .unwrap();
    */

    let mut port = VirtualPort::loopback(GEMINI_BAUD_RATE, SERIAL_BUFFER_SIZE).unwrap();
    info!("Initialized virtual port");

    sd_notify::notify(true, &[sd_notify::NotifyState::Ready]).unwrap();

    loop {
        let packet = recv.recv().unwrap();
        debug!("Recv packet: {packet:?}");
        port.write(&*packet).unwrap();
    }
}
