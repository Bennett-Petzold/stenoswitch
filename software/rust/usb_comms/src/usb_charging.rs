use std::{
    thread::{self, sleep},
    time::Duration,
};

use log::trace;
use sd_notify::NotifyState;
use systemd_journal_logger::JournalLog;

mod shared;

fn main() {
    JournalLog::new().unwrap().install().unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });

    let hid = shared::init(None).unwrap();
    sd_notify::notify(true, &[NotifyState::Ready]).unwrap();

    if log::max_level() >= log::LevelFilter::Trace {
        thread::spawn(move || {
            loop {
                sleep(Duration::from_secs(30));
                trace!("HID Status = {:?}", hid.status());
            }
        });
    }

    loop {
        sleep(Duration::MAX);
    }
}
