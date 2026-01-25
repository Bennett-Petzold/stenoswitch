use std::{
    fs::{File, read_to_string},
    os::fd::AsRawFd,
    thread::{self, sleep},
    time::Duration,
};

use log::{debug, trace};
use usb_gadget::{
    Class, Config, Gadget, Id, Strings, default_udc,
    function::{
        Handle,
        hid::{Hid, HidBuilder},
        util::{State, Status},
    },
};

/// From R_{ISET}.
pub const MAX_CHARGE_MA: u16 = 1600;
/// From R_{OLIM}.
pub const MAX_SYSTEM_MA: u16 = 200;

const BATTERY_REPORT_DESCRIPTOR: [u8; 69] = [
    0x5, 0x1, 0x9, 0x6, 0xA1, 0x1, 0x5, 0x7, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x0, 0x25, 0x1, 0x75,
    0x1, 0x95, 0x8, 0x81, 0x2, 0x19, 0x0, 0x29, 0xFF, 0x26, 0xFF, 0x0, 0x75, 0x8, 0x95, 0x1, 0x81,
    0x3, 0x5, 0x8, 0x19, 0x1, 0x29, 0x5, 0x25, 0x1, 0x75, 0x1, 0x95, 0x5, 0x91, 0x2, 0x95, 0x3,
    0x91, 0x3, 0x5, 0x7, 0x19, 0x0, 0x29, 0xDD, 0x26, 0xFF, 0x0, 0x75, 0x8, 0x95, 0x6, 0x81, 0x0,
    0xC0,
];

pub fn usb_config(description: &str, function_handle: Handle, max_power: u16) -> Config {
    let mut config =
        Config::new("Stenoswitch ".to_string() + description).with_function(function_handle);
    config.max_power = max_power;
    config.self_powered = false;
    config.remote_wakeup = true;
    config
}

pub fn init(hid_desc: Option<(HidBuilder, &str)>) -> Option<Hid> {
    let mut hid = None;

    let udc = default_udc().unwrap();
    usb_gadget::remove_all().unwrap();

    let (battery_hid, battery_handle) = {
        let mut battery = Hid::builder();
        // Boot interface supported
        battery.sub_class = 1;
        // Battery protocol
        battery.protocol = 1;
        battery.report_len = 8;
        battery.report_desc = BATTERY_REPORT_DESCRIPTOR.to_vec();

        battery.build()
    };

    let device_serial = read_to_string("/sys/firmware/devicetree/base/serial-number").unwrap();

    let mut reg = Gadget::new(
        Class::interface_specific(),
        // Testing USB ID
        Id::new(0x1209, 0x001),
        // TODO: get serial from RPi
        Strings::new("Bennett Petzold", "Stenoswitch", device_serial),
    )
    .with_config(usb_config(
        "Battery",
        battery_handle,
        if hid_desc.is_none() {
            MAX_CHARGE_MA
        } else {
            MAX_CHARGE_MA + MAX_SYSTEM_MA
        },
    ));

    if let Some((hid_builder, desc)) = hid_desc {
        let (inst_hid, handle) = hid_builder.build();
        hid = Some(inst_hid);
        reg = reg.with_config(usb_config(desc, handle, MAX_SYSTEM_MA));
    };

    let reg = reg.bind(&udc).unwrap();
    debug!("PATH: {:#?}", reg.path());
    let reg = Box::leak(Box::new(reg));

    if log::max_level() >= log::LevelFilter::Trace {
        thread::spawn(move || {
            loop {
                sleep(Duration::from_secs(30));
                trace!("UDC State = {:?}", udc.state());
            }
        });
    };

    let battery_ready = || battery_hid.status().state() == State::Bound;
    let secondary_ready = || {
        hid.as_ref()
            .is_none_or(|hid| hid.status().state() == State::Bound)
    };

    while !(battery_ready() && secondary_ready()) {
        debug!(
            "Battery, secondary status: {}, {}",
            battery_ready(),
            secondary_ready()
        );
        sleep(Duration::from_secs(1));
    }

    hid
}
