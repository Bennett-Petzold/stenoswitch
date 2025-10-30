use std::{
    fs::{self, read_to_string},
    process::Command,
    thread::sleep,
    time::{Duration, Instant},
};

use i2cdev::{
    core::{I2CDevice, I2CMessage, I2CTransfer},
    linux::{LinuxI2CDevice, LinuxI2CMessage},
};
use log::debug;

/// Chemical ID for charging at 4.2V is 0x1202 (CHEM_B).
/// BQ27427 Technical Reference Manual, page 20.
const CHEM_ID: u8 = 0x32;
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

const SOC_OFFSET_FILE: &str = "/user/data/.config/state_of_charge_offset";

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
        } else {
            if time_passed < Duration::from_secs(1) {
                ALL_COMMAND_SPACING
            } else {
                TWO_COMMAND_SPACING
            }
        };

        // Update the new write/read status.
        self.last_command_write = is_write;

        // 0 seconds wait is a valid result.
        time_passed.saturating_sub(command_spacing)
    }
}

/// BQ27427 Battery Monitor communications.
///
/// All functions will panic on hardware failures.
pub struct BatteryMonitor {
    i2c: LinuxI2CDevice,
    spacing: CommandSpacing,
    soc_offset: u16,
}

impl BatteryMonitor {
    /// Sets up the battery monitor, panicking on hardware failures.
    pub fn new() -> Self {
        // Get previously recorded lowest SOC.
        // If this is a fresh system, 100% guarantees we write in lower percents.
        // New systems are expected to do a full charge and discharge cycle.
        // This will underestimate remaining charge until fully discharged once.
        let soc_offset =
            str::parse(&read_to_string(SOC_OFFSET_FILE).unwrap_or("100".to_string())).unwrap();

        let i2c = {
            /// 1010101 -> 0x2E.
            const MONITOR_ADDR: u16 = 0x55;
            LinuxI2CDevice::new(
                option_env!("BATTERY_I2C").unwrap_or("/dev/null"),
                MONITOR_ADDR,
            )
            .unwrap()
        };

        let mut this = Self {
            i2c,
            spacing: CommandSpacing::new(),
            soc_offset,
        };
        this.init();

        this
    }
}

impl Default for BatteryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl BatteryMonitor {
    /// Write out the data, blocking for the necessary time to space commands.
    fn write(&mut self, data: &[u8]) {
        sleep(self.spacing.next_spacing(true));
        self.i2c.write(data).unwrap();
    }

    /// Read the data to a buffer, blocking for the necessary time to space commands.
    fn read_transfer(&mut self, data: &mut [u8], address: u16) {
        sleep(self.spacing.next_spacing(false));
        self.i2c
            .transfer(&mut [LinuxI2CMessage::read(data).with_address(address)])
            .unwrap();
    }

    /// Read the data to a buffer, blocking for the necessary time to space commands.
    fn read_byte(&mut self, register: u8) -> u8 {
        sleep(self.spacing.next_spacing(false));
        self.i2c.smbus_read_byte_data(register).unwrap()
    }

    /// Config modifications start with this and end with [`Self::seal`].
    ///
    /// Makes config read/write.
    fn unseal(&mut self) {
        self.write(&[standard_commands::CONTROL, 0, 0x80]);
        self.write(&[standard_commands::CONTROL, 0, 0x80]);
    }

    /// Config modifications end with this and start with [`Self::unseal`].
    ///
    /// Makes config read-only.
    fn seal(&mut self) {
        self.write(&[standard_commands::CONTROL, 0x20, 0]);
    }

    fn set_cfgupdate(&mut self) {
        self.write(&[standard_commands::CONTROL, 0x13, 0]);

        // Assert Flags() bit 4 is set.
        let lower_flag = self.read_byte(standard_commands::FLAGS);
        assert_ne!(lower_flag | 0x10, 0);
    }

    fn write_blockupdate(&mut self, subclass: u8, offset: u8, value: u16) {
        // Enable block data memory control
        self.write(&[extended_commands::BLOCK_DATA_CONTROL, 0]);
        // Access subclass
        self.write(&[extended_commands::DATA_CLASS, subclass]);
        // Set block offset
        self.write(&[extended_commands::DATA_BLOCK, offset / 32]);

        let inner_address = extended_commands::BLOCK_DATA_START + (offset % 32);

        let old_checksum = self.read_byte(extended_commands::BLOCK_DATA_CHECKSUM);
        let old_msb = self.read_byte(inner_address);
        let old_lsb = self.read_byte(inner_address + 1);

        // Writes are MSB to the lower address (big endian).
        let value_bytes = value.to_be_bytes();
        debug!(
            "Writing blockupdate of {value}, MSB bytes {:?}",
            value_bytes
        );
        self.write(&[inner_address, value_bytes[0]]);
        self.write(&[inner_address + 1, value_bytes[1]]);

        // Magic formula
        let checksum = ((255 - old_checksum - old_msb - old_lsb) as u16) % 256;

        self.write(&[extended_commands::BLOCK_DATA_CHECKSUM, checksum as u8]);
    }

    fn read_block(&mut self, subclass: u8, offset: u8) -> u16 {
        // Enable block data memory control
        self.write(&[extended_commands::BLOCK_DATA_CONTROL, 0]);
        // Access subclass
        self.write(&[extended_commands::DATA_CLASS, subclass]);
        // Set block offset
        self.write(&[extended_commands::DATA_BLOCK, offset / 32]);

        let inner_address = extended_commands::BLOCK_DATA_START + (offset % 32);

        let msb = self.read_byte(inner_address);
        let lsb = self.read_byte(inner_address + 1);
        u16::from_be_bytes([msb, lsb])
    }

    fn soft_reset(&mut self) {
        self.write(&[standard_commands::CONTROL, 0x42, 0]);
    }

    /// Assert Flags() bit 4 is not set.
    fn assert_no_cfgupdate(&mut self) {
        let lower_flag = self.read_byte(standard_commands::FLAGS);
        assert_eq!(lower_flag | 0x10, 0);
    }

    /// Programs the design capacity.
    ///
    /// From BQ27427 Technical Reference Manual, page 17-18
    fn set_design_capacity(&mut self) {
        self.set_cfgupdate();
        self.write_blockupdate(STATE_SUBCLASS, 0, BATTERY_DESIGN_CAPACITY);
        self.soft_reset();
        self.assert_no_cfgupdate();
    }

    /// Programs the chemistry.
    ///
    /// From BQ27427 Technical Reference Manual, page 18
    fn set_chemistry_profile(&mut self) {
        self.set_cfgupdate();
        self.write(&[standard_commands::CONTROL, CHEM_ID, 0x00]);
        self.soft_reset();
        self.assert_no_cfgupdate();

        // Assert the chemical ID is set correctly.
        assert_eq!(
            CHEM_ID,
            self.read_byte(standard_commands::NOMINAL_AVAILABLE_CAPACITY)
        );
    }

    /// Programs the sleep current.
    ///
    /// From BQ27427 Technical Reference Manual, page 46
    fn set_sleep_current(&mut self) {
        self.set_cfgupdate();
        self.write_blockupdate(STATE_SUBCLASS, 23, SLEEP_CURRENT);
        self.soft_reset();
        self.assert_no_cfgupdate();
    }

    /// Programs the notification delta to single percents.
    ///
    /// From BQ27427 Technical Reference Manual, page 46
    fn set_soci_delta(&mut self) {
        self.set_cfgupdate();
        self.write_blockupdate(STATE_SUBCLASS, 20, 1);
        self.soft_reset();
        self.assert_no_cfgupdate();
    }

    fn set_default_opconfig(&mut self) {
        /// From on BQ27427 Technical Reference Manual, page 34.
        ///
        /// The defaults are unmodified, but being explicit doesn't hurt.
        const DEFAULT_OPCONFIG: u16 = u16::from_be_bytes([0x64, 0x78]);

        self.unseal();
        self.set_cfgupdate();
        self.write_blockupdate(REGISTERS_SUBCLASS, OPCONFIG_OFFSET, DEFAULT_OPCONFIG);
        self.soft_reset();
        self.assert_no_cfgupdate();
        self.seal();
    }

    pub fn sleep(&mut self, enable: bool) {
        self.set_cfgupdate();

        let opconfig = self.read_block(REGISTERS_SUBCLASS, OPCONFIG_OFFSET);

        // Mask in a 1 to enable, 0 to disable.
        let new_opconfig = if enable {
            opconfig | 0x00_20
        } else {
            opconfig & 0xFF_DF
        };

        self.write_blockupdate(REGISTERS_SUBCLASS, OPCONFIG_OFFSET, new_opconfig);

        self.soft_reset();
        self.assert_no_cfgupdate();
    }

    /// Performs the full initialization sequence, blocking as necessary.
    fn init(&mut self) {
        self.unseal();

        self.set_design_capacity();
        self.set_chemistry_profile();
        self.set_sleep_current();
        self.set_soci_delta();
        self.set_default_opconfig();

        self.seal();
    }

    pub fn read_u16_reg(&mut self, address: u8) -> u16 {
        u16::from_be_bytes([self.read_byte(address), self.read_byte(address + 1)])
    }

    pub fn millivolts(&mut self) -> u16 {
        self.read_u16_reg(standard_commands::VOLTAGE)
    }

    pub fn full_available_capacity(&mut self) -> MilliAmpHours {
        self.read_u16_reg(standard_commands::FULL_AVAILABLE_CAPACITY)
    }

    pub fn remaining_capacity(&mut self) -> MilliAmpHours {
        self.read_u16_reg(standard_commands::REMAINING_CAPACITY)
    }

    pub fn full_charge_capacity(&mut self) -> MilliAmpHours {
        self.read_u16_reg(standard_commands::FULL_CHARGE_CAPACITY)
    }

    pub fn average_current(&mut self) -> MilliAmps {
        self.read_u16_reg(standard_commands::AVERAGE_CURRENT)
    }

    pub fn average_power(&mut self) -> MilliWatts {
        self.read_u16_reg(standard_commands::AVERAGE_POWER)
    }

    /// Returns a raw state of charge.
    ///
    /// Does not take into account the voltage cutoff circuit.
    pub fn raw_state_of_charge(&mut self) -> Percent {
        self.read_u16_reg(standard_commands::STATE_OF_CHARGE)
    }

    /// Returns the state of charge.
    ///
    /// Also updates the SOC offset if this is lower than previously achieved.
    pub fn state_of_charge(&mut self) -> Percent {
        let charge = self.raw_state_of_charge();

        // Usual case
        if charge > self.soc_offset {
            let num = (charge - self.soc_offset);
            // Going to do integer division by 100, want percent * 100.
            let adjusted_num = num * 100;
            adjusted_num / (100 - self.soc_offset)
        } else {
            // Have to update for the new minimum state of charge.
            let _ = Command::new("mount")
                .args(["-o", "remount,rw", "/user/data/"])
                .output()
                .unwrap();
            fs::write(SOC_OFFSET_FILE, charge.to_string()).unwrap();
            let _ = Command::new("mount")
                .args(["-o", "remount,ro", "/user/data/"])
                .output()
                .unwrap();

            0
        }
    }
}
