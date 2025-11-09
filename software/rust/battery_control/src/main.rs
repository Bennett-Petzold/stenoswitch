mod battery_monitor;
mod chg_en;
mod current_monitor;
mod current_rheostat;
mod i2c;
mod notify_lines;

use std::{
    error::Error,
    sync::Mutex,
    thread::{self},
};

use bare_err_tree::{AsErrTree, WrapErr, tree};
use log::{debug, info};
use systemd_journal_logger::JournalLog;

use crate::{
    battery_monitor::BatteryMonitor,
    chg_en::ChgEn,
    current_monitor::CurrentMonitor,
    current_rheostat::CurrentRheostat,
    i2c::I2C,
    notify_lines::{NotifyLines, NotifySource},
};

#[track_caller]
pub fn bare_err_unwrap<T, E>(res: Result<T, E>) -> T
where
    E: AsErrTree,
{
    const ERROR_DEPTH: usize = 10;
    bare_err_tree::tree_unwrap::<{ ERROR_DEPTH * 6 }, _, _>(res)
}

#[track_caller]
pub fn std_unwrap<T, E>(res: Result<T, E>) -> T
where
    E: Error,
{
    bare_err_unwrap(res.map_err(WrapErr))
}

/// Calibrates the charging rheostat based on the CC pins.
///
/// Will block to for the necessary spacing.
fn set_rheostat_from_cc(_cur_rheostat: &mut CurrentRheostat, _cur_mon: &mut CurrentMonitor) {}

fn main() {
    // ---------- //
    // Establish safe state on the En pin //
    // ---------- //

    // Disable charging on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        while ChgEn::new().is_err() {}
        default_panic(info);
    }));

    // Try disabling charge just once on Ctrl-C
    std_unwrap(ctrlc::set_handler(|| {
        let _ = ChgEn::new();
    }));

    let chg_en = std_unwrap(ChgEn::new());
    // ---------- //

    std_unwrap(std_unwrap(JournalLog::new()).install());
    log::set_max_level(log::LevelFilter::Trace);
    info!("Set charging to pre-setup disable");

    info!("Intializing I2C bus...");
    let sda_pin = std_unwrap(str::parse(option_env!("SDA_PIN").unwrap_or("/dev/null")));
    let scl_pin = std_unwrap(str::parse(option_env!("SCL_PIN").unwrap_or("/dev/null")));
    let i2c = Mutex::new(bare_err_unwrap(I2C::new(400_000, sda_pin, scl_pin)));

    info!("Intializing SPI and I2C devices...");
    let (mut cur_rheostat, mut bat_mon, mut cur_mon) = thread::scope(|s| {
        let rheostat_thread = s.spawn(|| bare_err_unwrap(CurrentRheostat::new(&i2c)));
        let battery_thread = s.spawn(|| bare_err_unwrap(BatteryMonitor::new(&i2c)));
        let current_thread = s.spawn(|| std_unwrap(CurrentMonitor::new()));

        (
            rheostat_thread.join().unwrap(),
            battery_thread.join().unwrap(),
            current_thread.join().unwrap(),
        )
    });
    info!("Initialized SPI and I2C devices");
    sd_notify::notify(true, &[sd_notify::NotifyState::Ready]).unwrap();

    // Initial states
    debug!("CC Lines: {:?}", cur_mon.read_cc());
    debug!(
        "Current limit = {} amps",
        std_unwrap(cur_mon.read_current_limit())
    );
    let raw_soc = bare_err_unwrap(bat_mon.raw_state_of_charge());
    info!(
        "Battery stats: {}% charge, {}% raw charge, {}% of design, {} mV, {} mA, {} mW",
        std_unwrap(bat_mon.state_of_charge(raw_soc)),
        raw_soc,
        (bare_err_unwrap(bat_mon.full_available_capacity()) as f32)
            / (battery_monitor::BATTERY_DESIGN_CAPACITY as f32),
        bare_err_unwrap(bat_mon.millivolts()),
        bare_err_unwrap(bat_mon.average_current()),
        bare_err_unwrap(bat_mon.average_power()),
    );

    let notify_lines = NotifyLines::new();

    loop {
        debug!("CC Lines: {:?}", cur_mon.read_cc());
        debug!(
            "Current limit = {} amps",
            std_unwrap(cur_mon.read_current_limit())
        );

        let notification = notify_lines.next_notification();
        match notification.source {
            NotifySource::Batmon => {
                // TODO battery monitor status stuff
                // Should be doing writes to a SQLite database in /tmp/
                // This can be used for data displayed over Bluetooth etc.
                // Probably also have files in mass storage filesystem with
                // some of this info.
                // And/or capture in special log fake device.
                debug!("Battery monitor notification");
                if log::max_level() >= log::LevelFilter::Debug {
                    let raw_soc = bare_err_unwrap(bat_mon.raw_state_of_charge());
                    debug!(
                        "Battery stats: {}% charge, {}% raw charge, {}% of design, {} mV, {} mA, {} mW",
                        std_unwrap(bat_mon.state_of_charge(raw_soc)),
                        raw_soc,
                        (bare_err_unwrap(bat_mon.full_available_capacity()) as f32)
                            / (battery_monitor::BATTERY_DESIGN_CAPACITY as f32),
                        bare_err_unwrap(bat_mon.millivolts()),
                        bare_err_unwrap(bat_mon.average_current()),
                        bare_err_unwrap(bat_mon.average_power()),
                    );
                }
            }
            NotifySource::ChgOn => {
                if notification.value {
                    info!("Started charging battery from USB")
                } else {
                    info!("Stopped charging battery from USB")
                }
            }
            NotifySource::UsbOn => {
                // Always start USB power changes by setting to a safe state.
                // If USB was connected, this prevents overdrawing.
                // If USB was disconnected, this is extra protection for the
                // next connection.
                std_unwrap(chg_en.disable());
                bare_err_unwrap(cur_rheostat.set_max());

                if notification.value {
                    info!("Switched to USB power");

                    set_rheostat_from_cc(&mut cur_rheostat, &mut cur_mon);
                    chg_en.enable().unwrap();
                    info!(
                        "Configured rheostat and enabled charging with {} mA limit",
                        std_unwrap(cur_mon.read_cc()).to_milliamps()
                    );

                    bat_mon.sleep(false);
                    info!("Turned off battery monitor sleep");
                }
            }
            NotifySource::BatOn => {
                if notification.value {
                    info!("Switched to battery power");

                    bat_mon.sleep(true);
                    info!("Turned on battery monitor sleep");
                }
            }
            NotifySource::StoreOn => {
                todo!("Set battery charging to stop at 3.7 V")
            }
        }
    }
}
