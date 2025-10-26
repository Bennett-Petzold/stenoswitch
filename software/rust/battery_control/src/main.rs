mod battery_monitor;
mod chg_en;
mod current_monitor;
mod current_rheostat;
mod notify_lines;

use std::thread::{self};

use log::{debug, info};
use systemd_journal_logger::JournalLog;

use crate::{
    battery_monitor::BatteryMonitor,
    chg_en::ChgEn,
    current_monitor::CurrentMonitor,
    current_rheostat::CurrentRheostat,
    notify_lines::{NotifyLines, NotifySource},
};

/// Calibrates the charging rheostat based on the CC pins.
///
/// Will block to for the necessary spacing.
fn set_rheostat_from_cc(cur_rheostat: &mut CurrentRheostat, cur_mon: &mut CurrentMonitor) {}

fn main() {
    // ---------- //
    // Establish safe state on the En pin //
    // ---------- //

    // Disable charging on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        while !ChgEn::maybe_new().is_some() {}
        default_panic(info);
    }));

    // Try disabling charge just once on Ctrl-C
    ctrlc::set_handler(|| {
        ChgEn::maybe_new();
    });

    let chg_en = ChgEn::new();
    // ---------- //

    JournalLog::new().unwrap().install().unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    info!("Set charging to pre-setup disable");

    info!("Intializing SPI and I2C devices...");
    let (mut cur_rheostat, mut bat_mon, mut cur_mon) = thread::scope(|s| {
        let rheostat_thread = s.spawn(|| CurrentRheostat::new());
        let battery_thread = s.spawn(|| BatteryMonitor::new());
        let current_thread = s.spawn(|| CurrentMonitor::new());

        (
            rheostat_thread.join().unwrap(),
            battery_thread.join().unwrap(),
            current_thread.join().unwrap(),
        )
    });
    info!("Initialized SPI and I2C devices");

    // Initial states
    debug!("CC Lines: {:?}", cur_mon.read_cc());
    debug!("Current limit = {} amps", cur_mon.read_current_limit());
    info!(
        "Battery stats: {}% charge, {}% raw charge, {}% of design, {} mV, {} mA, {} mW",
        bat_mon.state_of_charge(),
        bat_mon.raw_state_of_charge(),
        (bat_mon.full_available_capacity() as f32)
            / (battery_monitor::BATTERY_DESIGN_CAPACITY as f32),
        bat_mon.millivolts(),
        bat_mon.average_current(),
        bat_mon.average_power(),
    );

    let notify_lines = NotifyLines::new();

    loop {
        debug!("CC Lines: {:?}", cur_mon.read_cc());
        debug!("Current limit = {} amps", cur_mon.read_current_limit());

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
                debug!(
                    "Battery stats: {}% charge, {}% raw charge, {}% of design, {} mV, {} mA, {} mW",
                    bat_mon.state_of_charge(),
                    bat_mon.raw_state_of_charge(),
                    (bat_mon.full_available_capacity() as f32)
                        / (battery_monitor::BATTERY_DESIGN_CAPACITY as f32),
                    bat_mon.millivolts(),
                    bat_mon.average_current(),
                    bat_mon.average_power(),
                );
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
                chg_en.disable().unwrap();
                cur_rheostat.set_max();

                if notification.value {
                    info!("Switched to USB power");

                    set_rheostat_from_cc(&mut cur_rheostat, &mut cur_mon);
                    chg_en.enable().unwrap();
                    info!(
                        "Configured rheostat and enabled charging with {} mA limit",
                        cur_mon.read_cc().to_milliamps()
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
        }
    }
}
