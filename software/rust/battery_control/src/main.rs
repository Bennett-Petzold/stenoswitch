mod battery_monitor;
mod chg_en;
mod current_monitor;
mod current_rheostat;
mod i2c;
mod notify_lines;

use std::{
    error::Error,
    mem,
    process::exit,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, sleep},
    time::Duration,
};

use bare_err_tree::{AsErrTree, WrapErr};
use log::{debug, error, info, warn};
use systemd_journal_logger::JournalLog;

use crate::{
    battery_monitor::BatteryMonitor,
    chg_en::ChgEn,
    current_monitor::CurrentMonitor,
    current_rheostat::CurrentRheostat,
    i2c::I2C,
    notify_lines::{NotifyLines, NotifySource},
};

const STORAGE_CHARGE_LIMIT_MILLIVOLTS: u16 = 3700;
const I2C_HERTZ: u32 = 400_000;

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
///
/// Even stepping to the rheostat max, this should execute in under a second.
fn set_rheostat_from_cc(
    chg_en: &ChgEn,
    cur_rheostat: &mut CurrentRheostat,
    cur_mon: &mut CurrentMonitor,
) {
    // Always start by setting to a safe maximum
    bare_err_unwrap(cur_rheostat.set_max());

    // Sleep until the current limit can actually be read.
    while !std_unwrap(cur_mon.current_limit_energized()) {
        // Current limit will never energize if charging is disabled.
        // Prevent infinite looping.
        if !std_unwrap(chg_en.is_enabled()) {
            return;
        }

        debug!("Waiting for current limit to energize...");
        sleep(Duration::from_secs(1));
    }

    let cc_limit = |cur_mon: &mut CurrentMonitor| std_unwrap(cur_mon.read_cc()).to_amps();
    let current_limit = |cur_mon: &mut CurrentMonitor| std_unwrap(cur_mon.read_current_limit());

    while current_limit(cur_mon) < cc_limit(cur_mon) {
        // Prevent infinite loops if the rheostat can't raise far enough.
        if cur_rheostat.setting() >= current_rheostat::CUR_LIMIT_MAX {
            warn!(
                "Rheostat topped out below target limit: {} < {}",
                std_unwrap(cur_mon.read_current_limit()),
                cc_limit(cur_mon)
            );
            break;
        }
        bare_err_unwrap(cur_rheostat.step_up());
        sleep(current_rheostat::WIPER_SET_WAIT);
    }

    while current_limit(cur_mon) > cc_limit(cur_mon) {
        // Prevent infinite loops if the rheostat can't lower far enough.
        if cur_rheostat.setting() == 0 {
            error!(
                "Rheostat bottomed out above target limit: {} < {}",
                std_unwrap(cur_mon.read_current_limit()),
                cc_limit(cur_mon)
            );
            break;
        }
        bare_err_unwrap(cur_rheostat.step_down());
        sleep(current_rheostat::WIPER_SET_WAIT);
    }
}

/// Gets new stats from the battery.
///
/// Returns the raw state of charge.
/// TODO battery monitor status stuff
/// Should be doing writes to a SQLite database in /tmp/
/// This can be used for data displayed over Bluetooth etc.
/// Probably also have files in mass storage filesystem with
/// some of this info.
/// And/or capture in special log fake device.
fn update_battery_stats(
    bat_mon: &mut BatteryMonitor,
    is_discharging: bool,
) -> battery_monitor::Percent {
    let raw_soc = bare_err_unwrap(bat_mon.raw_state_of_charge());
    debug!(
        "Battery stats: {}% charge, {}% raw charge, {}% of design, {} mV, {} mA, {} mW",
        std_unwrap(bat_mon.state_of_charge(raw_soc, is_discharging)),
        raw_soc,
        (bare_err_unwrap(bat_mon.full_available_capacity()) as f32)
            / (battery_monitor::BATTERY_DESIGN_CAPACITY as f32),
        bare_err_unwrap(bat_mon.millivolts()),
        bare_err_unwrap(bat_mon.average_current()),
        bare_err_unwrap(bat_mon.average_power()),
    );

    raw_soc
}

fn main() {
    static CHG_EN: LazyLock<ChgEn> = LazyLock::new(|| std_unwrap(ChgEn::new()));

    static I2C_BUS: LazyLock<Mutex<I2C>> = LazyLock::new(|| {
        info!("Intializing I2C bus...");
        let sda_pin = std_unwrap(str::parse(option_env!("SDA_PIN").unwrap_or("/dev/null")));
        let scl_pin = std_unwrap(str::parse(option_env!("SCL_PIN").unwrap_or("/dev/null")));
        Mutex::new(bare_err_unwrap(I2C::new(I2C_HERTZ, sda_pin, scl_pin)))
    });

    // ---------------------------------- //
    // Establish safe state on the En pin //
    // ---------------------------------- //
    std_unwrap(CHG_EN.disable());

    // Disable charging on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Err(e) = CHG_EN.disable() {
            error!("Failed to disable charging on panic! {e:#?}");
        };
        default_panic(info);
        // Child threads also terminate the program.
        exit(1);
    }));

    // Try disabling charge just once on Ctrl-C
    std_unwrap(ctrlc::set_handler(|| {
        if let Err(e) = CHG_EN.disable() {
            error!("Failed to disable charging on ctrl-c! {e:#?}");
        };
        exit(2);
    }));

    // ---------------------------------- //

    std_unwrap(std_unwrap(JournalLog::new()).install());
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });

    // This repeats the earlier initialization set.
    info!("Set charging to pre-setup disable");
    CHG_EN.disable().unwrap();

    info!("Intializing SPI and I2C devices...");
    let (cur_rheostat, bat_mon, mut cur_mon) = thread::scope(|s| {
        // SPI can occur at the same time as I2C communications.
        let current_thread = s.spawn(|| std_unwrap(CurrentMonitor::new()));

        (
            Mutex::new(
                std::iter::from_fn(|| {
                    Some(match CurrentRheostat::new(&I2C_BUS) {
                        Ok(rheo) => Some(rheo),
                        Err(e) => {
                            error!("Rheostat init failure: {e:#?}");
                            std::thread::sleep(Duration::from_millis(500));
                            std_unwrap(std_unwrap(I2C_BUS.lock()).reset());
                            std::thread::sleep(Duration::from_millis(500));
                            None
                        }
                    })
                })
                .flatten()
                .next()
                .unwrap(),
            ),
            Mutex::new(bare_err_unwrap(BatteryMonitor::new(&I2C_BUS))),
            current_thread.join().unwrap(),
        )
    });
    info!("Initialized SPI and I2C devices");
    sd_notify::notify(true, &[sd_notify::NotifyState::Ready]).unwrap();

    // Initial states
    info!("CC Lines: {:?}", cur_mon.read_cc());
    info!("Current limit = {:?} amps", cur_mon.read_current_limit());

    let cur_mon = Mutex::new(cur_mon);
    let notify_lines = NotifyLines::new();
    let storage_voltage = AtomicBool::new(false);
    let charging = AtomicBool::new(false);
    thread::scope(|s| {
        let mut charge_enabled_thread = s.spawn(|| {});

        let _ = s.spawn(|| {
            let mut bat_mon = bat_mon.lock().unwrap();
            update_battery_stats(&mut bat_mon, !charging.load(Ordering::Relaxed))
        });

        loop {
            // Initial states
            if log::max_level() >= log::LevelFilter::Debug {
                let mut cur_mon = cur_mon.lock().unwrap();
                debug!("CC Lines: {:?}", cur_mon.read_cc());
                debug!("Current limit = {:?} amps", cur_mon.read_current_limit());
            }

            let notification = notify_lines.next_notification();
            match notification.source {
                NotifySource::Batmon => {
                    debug!("Battery monitor notification");

                    let _ = s.spawn(|| {
                        let mut bat_mon = bat_mon.lock().unwrap();
                        let raw_soc =
                            update_battery_stats(&mut bat_mon, !charging.load(Ordering::Relaxed));

                        let storage_stop = storage_voltage.load(Ordering::Relaxed)
                            && (bare_err_unwrap(bat_mon.millivolts())
                                > STORAGE_CHARGE_LIMIT_MILLIVOLTS);
                        let soc_stop = raw_soc >= bat_mon.state_of_charge_max();
                        drop(bat_mon); // No need to hold the mutex past this point.

                        let disable_charging = soc_stop || storage_stop;
                        let prev_charge_state = charging.swap(!disable_charging, Ordering::Relaxed);

                        // If they match, the charging state flipped.
                        if disable_charging == prev_charge_state {
                            if disable_charging {
                                std_unwrap(CHG_EN.disable());
                                info!("Battery beyond limits, turned off charging");
                            } else {
                                std_unwrap(CHG_EN.enable());
                                info!("Battery fell below limits, turned on charging");
                            }
                        }
                    });
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
                    std_unwrap(CHG_EN.disable());

                    // Tune the charge limit when USB is attached AND there
                    // isn't an existing tuning thread.
                    // Skipping the tuning thread check could cause
                    // uncontrolled thread spawning.
                    if notification.value && charge_enabled_thread.is_finished() {
                        let old_thread = mem::replace(
                            &mut charge_enabled_thread,
                            s.spawn(|| {
                                info!("Switched to USB power");

                                let skip_charging = s.spawn(|| {
                                    let mut bat_mon = bat_mon.lock().unwrap();
                                    bare_err_unwrap(bat_mon.sleep(false));
                                    info!("Turned off battery monitor sleep");

                                    let storage_stop = storage_voltage.load(Ordering::Relaxed)
                                        && (bare_err_unwrap(bat_mon.millivolts())
                                            > STORAGE_CHARGE_LIMIT_MILLIVOLTS);
                                    let soc_stop = bare_err_unwrap(bat_mon.raw_state_of_charge())
                                        >= bat_mon.state_of_charge_max();

                                    storage_stop || soc_stop
                                });

                                let skip_charging = skip_charging.join().unwrap();
                                charging.store(!skip_charging, Ordering::Relaxed);
                                if skip_charging {
                                    info!("Battery beyond limits, keeping charging disabled");
                                } else {
                                    {
                                        let mut cur_rheostat = std_unwrap(cur_rheostat.lock());
                                        let mut cur_mon = std_unwrap(cur_mon.lock());

                                        std_unwrap(cur_rheostat.set_max());

                                        std_unwrap(CHG_EN.enable());
                                        info!("Enabled charging");

                                        let cc_limit = std_unwrap(cur_mon.read_cc());

                                        set_rheostat_from_cc(
                                            &CHG_EN,
                                            &mut cur_rheostat,
                                            &mut cur_mon,
                                        );
                                        info!(
                                            "Configured rheostat with {} mA limit",
                                            cc_limit.to_milliamps()
                                        );

                                        // TODO: spawn background thread that watches for a cc
                                        // change and reruns rheostat set.
                                    }
                                }
                            }),
                        );

                        // Catch any panic from the previous invocation.
                        old_thread.join().unwrap();
                    }
                }
                NotifySource::BatOn => {
                    if notification.value {
                        info!("Switched to battery power");
                        let _ = s.spawn(|| {
                            bare_err_unwrap(bat_mon.lock().unwrap().sleep(true));
                            info!("Turned on battery monitor sleep");
                        });
                    }
                }
                NotifySource::StoreOn => {
                    storage_voltage.store(notification.value, Ordering::Relaxed);

                    if notification.value {
                        info!("Set battery charging to stop at 3.7 V");
                    } else {
                        info!("Set battery charging to stop at configured max state of charge");
                    };
                }
            }
        }
    });
}
