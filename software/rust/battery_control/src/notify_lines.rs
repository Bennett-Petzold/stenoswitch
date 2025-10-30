use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{JoinHandle, sleep},
    time::Duration,
};

use gpio_cdev::{Chip, EventRequestFlags, EventType, LineRequestFlags};
use log::debug;

const BATMON_LINE: &str = if let Some(x) = option_env!("ALERT_BATMON") {
    x
} else {
    "NaN"
};
const CHG_ON_LINE: &str = if let Some(x) = option_env!("CHG_ON") {
    x
} else {
    "NaN "
};
const USB_ON_LINE: &str = if let Some(x) = option_env!("USB_ON") {
    x
} else {
    "NaN "
};
const BAT_ON_LINE: &str = if let Some(x) = option_env!("BAT_ON") {
    x
} else {
    "NaN "
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NotifySource {
    Batmon,
    ChgOn,
    UsbOn,
    BatOn,
    StoreOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineNotification {
    pub source: NotifySource,
    pub value: bool,
}

pub struct NotifyLines {
    notifications: mpsc::Receiver<LineNotification>,
}

impl NotifyLines {
    /// Creates a line monitor, panicking on hardware failures.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel(0);

        let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null")).unwrap();

        let _both_edges = [
            (CHG_ON_LINE, NotifySource::ChgOn),
            (USB_ON_LINE, NotifySource::UsbOn),
            (BAT_ON_LINE, NotifySource::BatOn),
            (BAT_ON_LINE, NotifySource::StoreOn),
        ]
        .map(|(line, source)| {
            let tx = tx.clone();
            let line = chip.get_line(str::parse(line).unwrap()).unwrap();
            std::thread::spawn(move || {
                let init_value = line
                    .request(LineRequestFlags::INPUT, 0, "battery_control")
                    .unwrap()
                    .get_value()
                    .unwrap()
                    != 0;
                tx.send(LineNotification {
                    source,
                    value: init_value,
                })
                .unwrap();

                // Small race possibility if the line flips between sending the
                // initial read and notification monitoring.

                for event in line
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::BOTH_EDGES,
                        "battery_control",
                    )
                    .unwrap()
                {
                    let value = match event.unwrap().event_type() {
                        EventType::RisingEdge => true,
                        EventType::FallingEdge => false,
                    };
                    debug!("Line event: {source:#?} -> {value}");
                    // Small race possibility if the line flips while waiting
                    // for this to be consumed.
                    tx.send(LineNotification { source, value }).unwrap();
                }
            })
        });

        let _batmon = {
            let line = chip.get_line(str::parse(BATMON_LINE).unwrap()).unwrap();
            std::thread::spawn(move || {
                let init_value = line
                    .request(LineRequestFlags::INPUT, 0, "battery_control")
                    .unwrap()
                    .get_value()
                    .unwrap()
                    != 0;
                tx.send(LineNotification {
                    source: NotifySource::Batmon,
                    value: init_value,
                })
                .unwrap();

                // Small race possibility if the line flips between sending the
                // initial read and notification monitoring.

                for event in line
                    .events(
                        LineRequestFlags::INPUT,
                        EventRequestFlags::RISING_EDGE,
                        "battery_control",
                    )
                    .unwrap()
                {
                    debug!("Line event: {:#?} -> {}", NotifySource::Batmon, true);
                    // Small missed event possibility if the line triggers
                    // again while waiting for this to be consumed.
                    tx.send(LineNotification {
                        source: NotifySource::Batmon,
                        value: true,
                    })
                    .unwrap();
                }
            })
        };

        Self { notifications: rx }
    }

    /// Blocks until a new notification, then returns that event.
    pub fn next_notification(&self) -> LineNotification {
        let notification = self.notifications.recv().unwrap();
        debug!(
            "Recv line event: {:#?} -> {}",
            notification.source, notification.value
        );
        notification
    }
}
