use std::{cmp::min, sync::Mutex, time::Duration};

use log::{debug, warn};

use crate::i2c::{I2C, I2CErrorS};

/// Maximum code for the current limiter rheostat.
pub const CUR_LIMIT_MAX: u8 = 0x7F;
const CUR_LIMIT_DEFAULT: u8 = 0x3F;
/// Maximum variance in ohms for the current limit rheostat.
pub const WIPER_RES_VARIANCE: u8 = 100;
/// Wiper can be safely measured after waiting for this period.
/// Worst settling time on datasheet is 817 microseconds.
pub const WIPER_SET_WAIT: Duration = Duration::from_millis(1);

/// 0101110 -> 0x2E.
const RHEO_ADDR: u8 = 0x2E;

pub struct CurrentRheostat<'a> {
    i2c: &'a Mutex<I2C>,
    setting: u8,
}

impl<'a> CurrentRheostat<'a> {
    pub fn new(i2c: &'a Mutex<I2C>) -> Result<Self, I2CErrorS> {
        debug!("Creating rheostat instance");

        // Initialize as minimum allowed current
        // On the low tolerance MCP40D19T-104s, the max kOhm is 80,
        // so this gives a 0.5A limit.
        // On the high tolerance chips, the max kOhm is 120.
        let mut this = Self {
            i2c,
            setting: CUR_LIMIT_MAX,
        };

        // Debug check about state of system.
        {
            let default_value = this.read();
            if !default_value
                .as_ref()
                .is_ok_and(|val| *val == CUR_LIMIT_DEFAULT)
            {
                warn!("{default_value:?} is not {CUR_LIMIT_DEFAULT}, not a clean boot!");
            }
        }

        // Put the minimum current to hardware
        this.apply()?;

        Ok(this)
    }
}

impl CurrentRheostat<'_> {
    pub fn setting(&self) -> u8 {
        self.setting
    }

    fn apply(&mut self) -> Result<(), I2CErrorS> {
        debug!("Setting rheostat to {}", self.setting);
        self.i2c
            .lock()
            .unwrap()
            .write(RHEO_ADDR, [0, self.setting])?;
        debug!("Set rheostat to {}", self.setting);
        Ok(())
    }

    /// Returns the current rheostat value.
    fn read(&mut self) -> Result<u8, I2CErrorS> {
        // See MCP4017 datasheet pages 38 and 40.
        let mut response = [0];
        self.i2c.lock().unwrap().read(RHEO_ADDR, 0, &mut response)?;
        Ok(response[0])
    }

    /// Panics if the rheostat set value != intended value
    #[cfg(debug_assertions)]
    fn verify(&mut self) {
        let actual = self.read();
        assert_eq!(self.setting, actual.unwrap());
    }

    pub fn new_setting(&mut self, setting: u8) -> Result<(), I2CErrorS> {
        self.setting = min(setting, CUR_LIMIT_MAX);
        self.apply()?;
        #[cfg(debug_assertions)]
        self.verify();
        Ok(())
    }

    pub fn step_up(&mut self) -> Result<(), I2CErrorS> {
        self.new_setting(self.setting + 1)
    }

    pub fn step_down(&mut self) -> Result<(), I2CErrorS> {
        self.new_setting(self.setting.saturating_sub(1))
    }

    pub fn set_max(&mut self) -> Result<(), I2CErrorS> {
        self.new_setting(CUR_LIMIT_MAX)
    }
}
