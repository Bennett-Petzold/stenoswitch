#![allow(dead_code)]

use std::{
    fs::{self, read_to_string},
    io,
    process::Command,
    sync::Mutex,
    thread::sleep,
    time::{Duration, Instant},
};

use log::{debug, trace};

use crate::i2c::{I2C, I2CErrorS};

/// Chemical ID for charging at 4.2V is 0x1202 (CHEM_B).
/// BQ27427 Technical Reference Manual, page 20.
const CHEM_ID: u16 = 0x32;
/// From the EVE 35V datasheet.
pub const BATTERY_DESIGN_CAPACITY: u16 = 3500;

/// Puts the battery monitor in sleep mode during battery operation.
/// This will be exceeded during constant current charging.
const SLEEP_CURRENT: u16 = 300;

const STATE_SUBCLASS: u8 = 0x52;
/// BQ27427 Technical Reference Manual, page 34.
const REGISTERS_SUBCLASS: u8 = 64;
const OPCONFIG_OFFSET: u8 = 0;

type MilliAmpHours = u16;
type MilliAmps = u16;
type MilliWatts = u16;
type Percent = u16;

/// All registers are two bytes wide.
///
/// From BQ27427 Technical Reference Manual, page 19.
mod standard_commands {
    pub const CONTROL: u8 = 0x00;
    pub const TEMPERATURE: u8 = 0x02;
    pub const VOLTAGE: u8 = 0x04;
    pub const FLAGS: u8 = 0x06;
    pub const NOMINAL_AVAILABLE_CAPACITY: u8 = 0x08;
    pub const FULL_AVAILABLE_CAPACITY: u8 = 0x0A;
    pub const REMAINING_CAPACITY: u8 = 0x0C;
    pub const FULL_CHARGE_CAPACITY: u8 = 0x0E;
    pub const AVERAGE_CURRENT: u8 = 0x10;
    pub const AVERAGE_POWER: u8 = 0x18;
    pub const STATE_OF_CHARGE: u8 = 0x1C;
    pub const INTERNAL_TEMPERATURE: u8 = 0x1E;
    pub const REMAINING_CAPACITY_UNFILTERED: u8 = 0x28;
    pub const REMAINING_CAPACITY_FILTERED: u8 = 0x2A;
    pub const FULL_CHARGE_CAPACITY_UNFILTERED: u8 = 0x2C;
    pub const FULL_CHARGE_CAPACITY_FILTERED: u8 = 0x2E;
    pub const STATE_OF_CHARGE_UNFILTERED: u8 = 0x30;
}

/// From BQ27427 Technical Reference Manual, page 19.
mod extended_commands {
    pub const DATA_CLASS: u8 = 0x3E;
    pub const DATA_BLOCK: u8 = 0x3F;
    pub const BLOCK_DATA_START: u8 = 0x40;
    pub const BLOCK_DATA_CHECKSUM: u8 = 0x60;
    pub const BLOCK_DATA_CONTROL: u8 = 0x61;
}

const CONFIG_DIR: &str = "/user/data/.config/";
const SOC_OFFSET_FILE: &str = "/user/data/.config/state_of_charge_offset";
const SOC_MAX_FILE: &str = "/user/data/.config/state_of_charge_max";

/// From BQ27427 Technical Reference Manual, page 16.
const ALL_COMMAND_SPACING: Duration = Duration::from_micros(66);
/// From BQ27427 Technical Reference Manual, page 16
const WRITE_READ_COMMAND_SPACING: Duration = Duration::from_secs(2);
/// From BQ27427 Technical Reference Manual, page 16
///
/// This must be inserted between every two commands.
const TWO_COMMAND_SPACING: Duration = Duration::from_secs(1);

pub struct CommandSpacing {
    last_command_time: Instant,
    last_command_write: bool,
}

impl CommandSpacing {
    pub fn new() -> Self {
        Self {
            last_command_time: Instant::now(),
            last_command_write: false,
        }
    }
}

impl Default for CommandSpacing {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandSpacing {
    /// Updates internal state and returns how long to wait before sending.
    ///
    /// The spacing might be longer than necessary but is always sufficent.
    pub fn next_spacing(&mut self, is_write: bool) -> Duration {
        // Swap in the current time as the new last command time.
        // Preserve the old value for calculations.
        let mut last_command_time = Instant::now();
        std::mem::swap(&mut self.last_command_time, &mut last_command_time);

        let time_passed = self.last_command_time.duration_since(last_command_time);

        let command_spacing = if self.last_command_write && !is_write {
            WRITE_READ_COMMAND_SPACING
        } else if time_passed < Duration::from_secs(1) {
            ALL_COMMAND_SPACING
        } else {
            TWO_COMMAND_SPACING
        };

        // Update the new write/read status.
        self.last_command_write = is_write;

        // 0 seconds wait is a valid result.
        command_spacing.saturating_sub(time_passed)
    }
}

/// 1010101 -> 0x2E.
const MONITOR_ADDR: u8 = 0x55;

/// BQ27427 Battery Monitor communications.
///
/// All functions will panic on hardware failures.
pub struct BatteryMonitor<'a> {
    i2c: &'a Mutex<I2C>,
    spacing: CommandSpacing,
    soc_offset: Percent,
    soc_max: Percent,
}

impl<'a> BatteryMonitor<'a> {
    /// Sets up the battery monitor, panicking on hardware failures.
    pub fn new(i2c: &'a Mutex<I2C>) -> Result<Self, I2CErrorS> {
        debug!("Creating battery monitor instance");

        // Get previously recorded lowest SOC.
        // If this is a fresh system, 100% guarantees we write in lower percents.
        // New systems are expected to do a full charge and discharge cycle.
        // This will underestimate remaining charge until fully discharged once.
        let soc_offset =
            str::parse(&read_to_string(SOC_OFFSET_FILE).unwrap_or("100".to_string())).unwrap();

        // Get a maximum charge.
        // If this is a fresh system, 80% is assumed for improved battery life.
        let soc_max =
            str::parse(&read_to_string(SOC_MAX_FILE).unwrap_or("80".to_string())).unwrap();

        let mut this = Self {
            i2c,
            spacing: CommandSpacing::new(),
            soc_offset,
            soc_max,
        };
        this.init()?;

        Ok(this)
    }
}

impl BatteryMonitor<'_> {
    /// Write out the data, blocking for the necessary time to space commands.
    ///
    /// All writes must be 1-byte at 100kHz, see BQ27427 manual page 7.
    fn write(&mut self, command: u8, data: u8) -> Result<(), I2CErrorS> {
        sleep(self.spacing.next_spacing(true));
        trace!("Writing {command} to battery monitor with {data}");
        self.i2c
            .lock()
            .unwrap()
            .write(MONITOR_ADDR, [command, data])
    }

    /// Write out the two-byte data, blocking for the necessary time to space commands.
    fn write_u16(&mut self, command: u8, data: u16) -> Result<(), I2CErrorS> {
        trace!("Writing {command} to battery monitor with u16 {data}");
        let [lsb, msb] = data.to_le_bytes();
        self.write(command, lsb)?;
        self.write(command + 1, msb)
    }

    /// Read the data to a buffer, blocking for the necessary time to space commands.
    fn read(&mut self, data: &mut [u8], register: u8) -> Result<(), I2CErrorS> {
        trace!(
            "Reading {}-bytes of {register} from battery monitor",
            data.len()
        );
        sleep(self.spacing.next_spacing(false));
        self.i2c.lock().unwrap().read(MONITOR_ADDR, register, data)
    }

    /// Read a single byte, blocking for the necessary time to space commands.
    fn read_byte(&mut self, register: u8) -> Result<u8, I2CErrorS> {
        let mut buffer = [0];
        self.read(&mut buffer, register)?;
        Ok(buffer[0])
    }

    /// Read a two-wide value, blocking for the necessary time to space commands.
    fn read_u16(&mut self, register: u8) -> Result<u16, I2CErrorS> {
        let msb = self.read_byte(register)?;
        let lsb = self.read_byte(register + 1)?;
        Ok(u16::from_be_bytes([msb, lsb]))
    }

    /// Config modifications start with this and end with [`Self::seal`].
    ///
    /// Makes config read/write.
    fn unseal(&mut self) -> Result<(), I2CErrorS> {
        for _ in 0..2 {
            self.write_u16(standard_commands::CONTROL, 0x8000)?;
        }
        Ok(())
    }

    /// Config modifications end with this and start with [`Self::unseal`].
    ///
    /// Makes config read-only.
    #[inline]
    fn seal(&mut self) -> Result<(), I2CErrorS> {
        self.write_u16(standard_commands::CONTROL, 0x0020)
    }

    /// Enables configuration updates on the monitor.
    ///
    /// Must be followed by the config change and a soft reset.
    #[inline]
    fn set_cfgupdate(&mut self) -> Result<(), I2CErrorS> {
        self.write_u16(standard_commands::CONTROL, 0x0013)?;

        // Assert Flags() bit 4 is set.
        #[cfg(debug_assertions)]
        {
            let lower_flag = self.read_byte(standard_commands::FLAGS);
            assert_ne!(lower_flag? | 0x10, 0);
        }
        Ok(())
    }

    fn write_blockupdate(&mut self, subclass: u8, offset: u8, value: u16) -> Result<(), I2CErrorS> {
        // Enable block data memory control
        self.write(extended_commands::BLOCK_DATA_CONTROL, 0)?;
        // Access subclass
        self.write(extended_commands::DATA_CLASS, subclass)?;
        // Set block offset
        self.write(extended_commands::DATA_BLOCK, offset / 32)?;

        let inner_register = extended_commands::BLOCK_DATA_START + (offset % 32);

        let old_checksum = self.read_byte(extended_commands::BLOCK_DATA_CHECKSUM)?;
        let old_msb = self.read_byte(inner_register)?;
        let old_lsb = self.read_byte(inner_register + 1)?;

        let value_bytes = value.to_be_bytes();
        debug!(
            "Writing blockupdate of {value}, MSB bytes {:?}",
            value_bytes
        );
        self.write_u16(inner_register, value)?;

        // Magic formula
        let checksum = ((255 - old_checksum - old_msb - old_lsb) as u16) % 256;

        self.write(extended_commands::BLOCK_DATA_CHECKSUM, checksum as u8)
    }

    fn read_block(&mut self, subclass: u8, offset: u8) -> Result<u16, I2CErrorS> {
        // Enable block data memory control
        self.write(extended_commands::BLOCK_DATA_CONTROL, 0)?;
        // Access subclass
        self.write(extended_commands::DATA_CLASS, subclass)?;
        // Set block offset
        self.write(extended_commands::DATA_BLOCK, offset / 32)?;

        let inner_register = extended_commands::BLOCK_DATA_START + (offset % 32);
        self.read_u16(inner_register)
    }

    /// Reboots the monitor; critical after any configuration changes.
    #[inline]
    fn soft_reset(&mut self) -> Result<(), I2CErrorS> {
        trace!("Soft resetting battery monitor");
        self.write_u16(standard_commands::CONTROL, 0x0042)
    }

    /// Assert Flags() bit 4 is not set.
    #[cfg(debug_assertions)]
    fn assert_no_cfgupdate(&mut self) -> Result<(), I2CErrorS> {
        let lower_flag = self.read_byte(standard_commands::FLAGS)?;
        assert_eq!(lower_flag | 0x10, 0);
        Ok(())
    }

    /// Full configuration update routine for block values.
    fn update_block_cfg(&mut self, subclass: u8, offset: u8, value: u16) -> Result<(), I2CErrorS> {
        self.set_cfgupdate()?;
        self.write_blockupdate(subclass, offset, value)?;
        self.soft_reset()?;
        #[cfg(debug_assertions)]
        self.assert_no_cfgupdate()?;

        Ok(())
    }

    /// Programs the design capacity.
    ///
    /// From BQ27427 Technical Reference Manual, page 17-18
    #[inline]
    fn set_design_capacity(&mut self) -> Result<(), I2CErrorS> {
        self.update_block_cfg(STATE_SUBCLASS, 0, BATTERY_DESIGN_CAPACITY)
    }

    /// Programs the chemistry.
    ///
    /// From BQ27427 Technical Reference Manual, page 18
    fn set_chemistry_profile(&mut self) -> Result<(), I2CErrorS> {
        self.set_cfgupdate()?;
        self.write_u16(standard_commands::CONTROL, CHEM_ID)?;
        self.soft_reset()?;
        #[cfg(debug_assertions)]
        self.assert_no_cfgupdate()?;

        // Assert the chemical ID is set correctly.
        debug_assert_eq!(
            CHEM_ID as u8,
            self.read_byte(standard_commands::NOMINAL_AVAILABLE_CAPACITY)?
        );

        Ok(())
    }

    /// Programs the sleep current.
    ///
    /// From BQ27427 Technical Reference Manual, page 46
    #[inline]
    fn set_sleep_current(&mut self) -> Result<(), I2CErrorS> {
        self.update_block_cfg(STATE_SUBCLASS, 23, SLEEP_CURRENT)
    }

    /// Programs the notification delta to single percents.
    ///
    /// From BQ27427 Technical Reference Manual, page 46
    #[inline]
    fn set_soci_delta(&mut self) -> Result<(), I2CErrorS> {
        self.update_block_cfg(STATE_SUBCLASS, 20, 1)
    }

    fn set_default_opconfig(&mut self) -> Result<(), I2CErrorS> {
        /// From on BQ27427 Technical Reference Manual, page 34.
        ///
        /// The defaults are unmodified, but being explicit doesn't hurt.
        const DEFAULT_OPCONFIG: u16 = u16::from_be_bytes([0x64, 0x78]);

        self.unseal()?;
        self.update_block_cfg(REGISTERS_SUBCLASS, OPCONFIG_OFFSET, DEFAULT_OPCONFIG)?;
        self.seal()
    }

    pub fn sleep(&mut self, enable: bool) -> Result<(), I2CErrorS> {
        self.set_cfgupdate()?;

        let opconfig = self.read_block(REGISTERS_SUBCLASS, OPCONFIG_OFFSET)?;

        // Mask in a 1 to enable, 0 to disable.
        let new_opconfig = if enable {
            opconfig | 0x00_20
        } else {
            opconfig & 0xFF_DF
        };

        self.write_blockupdate(REGISTERS_SUBCLASS, OPCONFIG_OFFSET, new_opconfig)?;
        self.soft_reset()?;
        #[cfg(debug_assertions)]
        self.assert_no_cfgupdate()?;

        Ok(())
    }

    /// Performs the full initialization sequence, blocking as necessary.
    fn init(&mut self) -> Result<(), I2CErrorS> {
        self.unseal()?;

        self.set_design_capacity()?;
        self.set_chemistry_profile()?;
        self.set_sleep_current()?;
        self.set_soci_delta()?;
        self.set_default_opconfig()?;

        self.seal()
    }

    #[inline]
    pub fn millivolts(&mut self) -> Result<u16, I2CErrorS> {
        self.read_u16(standard_commands::VOLTAGE)
    }

    #[inline]
    pub fn full_available_capacity(&mut self) -> Result<MilliAmpHours, I2CErrorS> {
        self.read_u16(standard_commands::FULL_AVAILABLE_CAPACITY)
    }

    #[inline]
    pub fn remaining_capacity(&mut self) -> Result<MilliAmpHours, I2CErrorS> {
        self.read_u16(standard_commands::REMAINING_CAPACITY)
    }

    #[inline]
    pub fn full_charge_capacity(&mut self) -> Result<MilliAmpHours, I2CErrorS> {
        self.read_u16(standard_commands::FULL_CHARGE_CAPACITY)
    }

    #[inline]
    pub fn average_current(&mut self) -> Result<MilliAmps, I2CErrorS> {
        self.read_u16(standard_commands::AVERAGE_CURRENT)
    }

    #[inline]
    pub fn average_power(&mut self) -> Result<MilliWatts, I2CErrorS> {
        self.read_u16(standard_commands::AVERAGE_POWER)
    }

    /// Returns a raw state of charge.
    ///
    /// Does not take into account any adjustments. Call result with
    /// [`Self::state_of_charge`] for a system percent.
    #[inline]
    pub fn raw_state_of_charge(&mut self) -> Result<Percent, I2CErrorS> {
        self.read_u16(standard_commands::STATE_OF_CHARGE)
    }

    /// Returns the adjusted state of charge.
    ///
    /// Also updates the SOC offset if this is lower than previously achieved.
    pub fn state_of_charge(
        &mut self,
        raw_charge: Percent,
        is_discharging: bool,
    ) -> io::Result<Percent> {
        // Usual case
        if raw_charge > self.soc_offset {
            let num = raw_charge - self.soc_offset;
            // Going to do integer division by max soc, want percent * 100.
            let adjusted_num = num * 100;
            Ok(adjusted_num / (self.soc_max - self.soc_offset))
        } else {
            if is_discharging {
                // Have to update for the new minimum state of charge.
                let _ = Command::new("mount")
                    .args(["-o", "remount,rw", "/user/data/"])
                    .output()?;
                fs::create_dir_all(CONFIG_DIR)?;
                fs::write(SOC_OFFSET_FILE, raw_charge.to_string())?;
                let _ = Command::new("mount")
                    .args(["-o", "remount,ro", "/user/data/"])
                    .output()?;

                self.soc_offset = raw_charge;
            }

            Ok(0)
        }
    }

    /// Get the set full charge percent.
    pub fn state_of_charge_max(&self) -> Percent {
        self.soc_max
    }
}
