use std::{cmp::min, time::Duration};

use i2cdev::{
    core::{I2CDevice, I2CMessage, I2CTransfer},
    linux::{LinuxI2CBus, LinuxI2CDevice, LinuxI2CMessage},
};
use log::warn;

/// Maximum code for the current limiter rheostat.
const CUR_LIMIT_MAX: u8 = 0x7F;
const CUR_LIMIT_DEFAULT: u8 = 0x3F;
/// Maximum variance in ohms for the current limit rheostat.
pub const WIPER_RES_VARIANCE: u8 = 100;
/// Wiper can be safely measured after waiting for this period.
/// Worst settling time on datasheet is 817 microseconds.
pub const WIPER_SET_WAIT: Duration = Duration::from_millis(1);

/// 0101110 -> 0x2E.
const RHEO_ADDR: u16 = 0x2E;

pub struct CurrentRheostat {
    i2c: LinuxI2CDevice,
    setting: u8,
}

impl CurrentRheostat {
    pub fn new() -> Self {
        // Write the reset sequence with extra 1 bits to fill out the byte.
        // Ignore the reset sequence for now
        /*
        let _ = LinuxI2CBus::new(env!("BATTERY_I2C"))
            .unwrap()
            .transfer(&mut [LinuxI2CMessage::write(&[0xFF, 0xFF])]);
        */

        // Initialize as minimum allowed current
        // On some MCP40D19T-104s, the max kOhm is 80,
        // so this gives a 0.5A limit.
        let mut this = Self {
            i2c: LinuxI2CDevice::new(env!("BATTERY_I2C"), RHEO_ADDR).unwrap(),
            setting: CUR_LIMIT_MAX,
        };

        // Debug check about state of system.
        {
            let default_value = this.read();
            if default_value != CUR_LIMIT_DEFAULT {
                warn!("{default_value} is not {CUR_LIMIT_DEFAULT}, not a clean boot!");
            }
        }

        // Put the minimum current to hardware
        this.apply();

        this
    }
}

impl Default for CurrentRheostat {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentRheostat {
    pub fn setting(&self) -> u8 {
        self.setting
    }

    fn apply(&mut self) {
        self.i2c.write(&[0, self.setting]).unwrap();
    }

    /// Panics if the rheostat set value != intended value
    fn verify(&mut self) {
        let actual = self.read();
        assert_eq!(self.setting, actual);
    }

    /// Returns the current rheostat value.
    fn read(&mut self) -> u8 {
        // See MCP4017 datasheet pages 38 and 40.
        let mut response = [0];
        let mut transactions = [
            LinuxI2CMessage::write(&[0]),
            LinuxI2CMessage::read(&mut response),
        ];
        self.i2c.transfer(&mut transactions).unwrap();
        response[0]
    }

    pub fn new_setting(&mut self, setting: u8) {
        self.setting = min(setting, CUR_LIMIT_MAX);
        self.apply();
        self.verify();
    }

    pub fn step_up(&mut self) {
        self.new_setting(self.setting + 1);
    }

    pub fn step_down(&mut self) {
        self.new_setting(self.setting.saturating_sub(1));
    }

    pub fn set_max(&mut self) {
        self.new_setting(CUR_LIMIT_MAX);
    }
}
