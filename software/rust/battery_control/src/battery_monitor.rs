#![allow(dead_code)]

use std::{
    fs::{self, read_to_string},
    io,
    process::Command,
    sync::{Mutex, MutexGuard},
    thread::sleep,
    time::{Duration, Instant},
};

use log::{debug, info, trace, warn};

use crate::i2c::{I2C, I2CDevice, I2CError, I2CErrorS};

/// Chemical ID for charging at 4.2V is 0x1202 (CHEM_B).
/// BQ27427 Technical Reference Manual, page 20.
const CHEM_ID: u16 = 0x0032;
/// From the EVE 35V datasheet.
pub const BATTERY_DESIGN_CAPACITY: u16 = 3500;

/// Puts the battery monitor in sleep mode during battery operation.
/// This may be exceeded during constant current charging.
const SLEEP_CURRENT: u16 = 300;

const STATE_SUBCLASS: u8 = 0x52;
/// BQ27427 Technical Reference Manual, page 34.
const REGISTERS_SUBCLASS: u8 = 64;
const OPCONFIG_OFFSET: u8 = 0;

pub type MilliAmpHours = u16;
pub type MilliAmps = u16;
pub type MilliWatts = u16;
pub type Percent = u8;

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
/// From BQ27427 Technical Reference Manual, page 18
const SOFT_RESET_DELAY: Duration = Duration::from_secs(1);

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
            trace!("Write command spacing");
            WRITE_READ_COMMAND_SPACING
        } else if time_passed < Duration::from_secs(1) {
            trace!("All command spacing");
            ALL_COMMAND_SPACING
        } else {
            trace!("Two command spacing");
            TWO_COMMAND_SPACING
        };

        // Update the new write/read status.
        self.last_command_write = is_write;

        // 0 seconds wait is a valid result.
        command_spacing.saturating_sub(time_passed)
    }
}

/// From BQ27427 manual pages 7 and 10.
const MONITOR_DEVICE: I2CDevice = I2CDevice {
    address: 0b1010101,
    bus_free_time: ALL_COMMAND_SPACING,
};

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

        debug!("Battery monitor set up.");
        Ok(this)
    }
}

impl BatteryMonitor<'_> {
    /// Write out the data, blocking for the necessary time to space commands.
    ///
    /// All writes must be 1-byte at 400kHz, see BQ27427 manual page 7.
    fn write(
        spacing: &mut CommandSpacing,
        held_i2c: &mut MutexGuard<I2C>,
        command: u8,
        data: u8,
    ) -> Result<(), I2CErrorS> {
        sleep(spacing.next_spacing(true));
        trace!("Writing {command:02x} to battery monitor with {data:02x}");

        held_i2c.write(MONITOR_DEVICE, &[command, data])
    }

    /// Write out the two-byte data, blocking for the necessary time to space commands.
    fn write_u16(
        spacing: &mut CommandSpacing,
        held_i2c: &mut MutexGuard<I2C>,
        command: u8,
        data: u16,
    ) -> Result<(), I2CErrorS> {
        trace!("Writing {command:02x} to battery monitor with u16 {data:04x}");
        let [lsb, msb] = data.to_le_bytes();

        Self::write(spacing, held_i2c, command, lsb)?;
        Self::write(spacing, held_i2c, command + 1, msb)
    }

    /// Read the data to a buffer, blocking for the necessary time to space commands.
    fn read(
        spacing: &mut CommandSpacing,
        held_i2c: &mut MutexGuard<I2C>,
        data: &mut [u8],
        register: u8,
    ) -> Result<(), I2CErrorS> {
        trace!(
            "Reading {}-byte(s) of {register:02x} from battery monitor",
            data.len()
        );
        sleep(spacing.next_spacing(false));

        held_i2c.read(MONITOR_DEVICE, register, data)
    }

    /// Read a single byte, blocking for the necessary time to space commands.
    fn read_byte(
        spacing: &mut CommandSpacing,
        held_i2c: &mut MutexGuard<I2C>,
        register: u8,
    ) -> Result<u8, I2CErrorS> {
        let mut data = [0];
        Self::read(spacing, held_i2c, &mut data, register)?;
        Ok(data[0])
    }

    /// Read a two-wide value, blocking for the necessary time to space commands.
    fn read_u16(
        spacing: &mut CommandSpacing,
        held_i2c: &mut MutexGuard<I2C>,
        register: u8,
    ) -> Result<u16, I2CErrorS> {
        let lsb = Self::read_byte(spacing, held_i2c, register)?;
        let msb = Self::read_byte(spacing, held_i2c, register + 1)?;
        Ok(u16::from_le_bytes([lsb, msb]))
    }

    /// Config modifications start with this and end with [`Self::seal`].
    ///
    /// Makes config read/write.
    fn unseal(spacing: &mut CommandSpacing, i2c: &mut MutexGuard<I2C>) -> Result<(), I2CErrorS> {
        for _ in 0..2 {
            Self::write_u16(spacing, i2c, standard_commands::CONTROL, 0x8000)?;
        }
        Ok(())
    }

    /// Config modifications end with this and start with [`Self::unseal`].
    ///
    /// Makes config read-only.
    #[inline]
    fn seal(spacing: &mut CommandSpacing, i2c: &mut MutexGuard<I2C>) -> Result<(), I2CErrorS> {
        Self::write_u16(spacing, i2c, standard_commands::CONTROL, 0x0020)
    }

    /// Enables configuration updates on the monitor.
    ///
    /// Must be followed by the config change and a soft reset.
    #[inline]
    fn set_cfgupdate(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
    ) -> Result<(), I2CErrorS> {
        trace!("Setting cfgupdate mode");
        Self::write_u16(spacing, i2c, standard_commands::CONTROL, 0x0013)?;
        // Complete mandatory sleep period before any parameters can be modified.
        // From BQ27427 Reference Manual Page 21: SET_CFGUPDATE
        std::thread::sleep(Duration::from_millis(1_200));
        Self::wait_for_cfgupdate(spacing, i2c, true)
    }

    fn write_blockupdate(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
        subclass: u8,
        offset: u8,
        value: &[u8],
    ) -> Result<(), I2CErrorS> {
        // Can't go beyond the 32 byte block
        if ((offset % 32) as usize + value.len()) >= 32 {
            return Err(I2CError::OversizedBlockWrite.into());
        }

        // Enable block data memory control
        Self::write(spacing, i2c, extended_commands::BLOCK_DATA_CONTROL, 0)?;
        // Access subclass
        Self::write(spacing, i2c, extended_commands::DATA_CLASS, subclass)?;
        // Set block offset
        Self::write(spacing, i2c, extended_commands::DATA_BLOCK, offset / 32)?;

        let inner_register = extended_commands::BLOCK_DATA_START + (offset % 32);

        // Magic formula from manual page 17
        let checksum = {
            let old_checksum =
                Self::read_byte(spacing, i2c, extended_commands::BLOCK_DATA_CHECKSUM)?;

            let mut checksum_temp = 255_u8.wrapping_sub(old_checksum);

            for i in 0..value.len() {
                let remove_byte = Self::read_byte(spacing, i2c, inner_register + (i as u8))?;
                checksum_temp = checksum_temp.wrapping_sub(remove_byte);
            }

            for byte in value {
                checksum_temp = checksum_temp.wrapping_add(*byte);
            }

            255 - checksum_temp
        };

        debug!("Writing blockupdate of {value:02x?} to {subclass:02x} at {offset:02x}");
        for (write_offset, byte) in value.iter().enumerate() {
            Self::write(spacing, i2c, inner_register + write_offset as u8, *byte)?;
        }

        trace!("Writing blockupdate checksum of {checksum:02x}");
        Self::write(
            spacing,
            i2c,
            extended_commands::BLOCK_DATA_CHECKSUM,
            checksum,
        )
    }

    fn write_blockupdate_u16(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
        subclass: u8,
        offset: u8,
        value: u16,
    ) -> Result<(), I2CErrorS> {
        Self::write_blockupdate(spacing, i2c, subclass, offset, &value.to_le_bytes())
    }

    fn read_block(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
        subclass: u8,
        offset: u8,
        data: &mut [u8],
    ) -> Result<(), I2CErrorS> {
        // Enable block data memory control
        Self::write(spacing, i2c, extended_commands::BLOCK_DATA_CONTROL, 0)?;
        // Access subclass
        Self::write(spacing, i2c, extended_commands::DATA_CLASS, subclass)?;
        // Set block offset
        Self::write(spacing, i2c, extended_commands::DATA_BLOCK, offset / 32)?;

        let inner_register = extended_commands::BLOCK_DATA_START + (offset % 32);

        Self::read(spacing, i2c, data, inner_register)
    }

    /// Reboots the monitor; critical after any configuration changes.
    #[inline]
    fn soft_reset(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
    ) -> Result<(), I2CErrorS> {
        trace!("Soft resetting battery monitor");
        Self::write_u16(spacing, i2c, standard_commands::CONTROL, 0x0042)?;
        Self::wait_for_cfgupdate(spacing, i2c, false)
    }

    /// Wait until Flags() bit 4 is at `set`.
    fn wait_for_cfgupdate(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
        set: bool,
    ) -> Result<(), I2CErrorS> {
        use crate::i2c::I2CError;

        const DEADLINE_MULTIPLIER: u32 = 5;

        let deadline = Instant::now() + (SOFT_RESET_DELAY * DEADLINE_MULTIPLIER);

        #[cfg(debug_assertions)]
        let mut try_count = 0_u64;

        while Instant::now() < deadline {
            let lower_flag = Self::read_byte(spacing, i2c, standard_commands::FLAGS)?;
            trace!("Cfgupdate lower flag value: {lower_flag:08b}");
            let lower_flag_true = (lower_flag & 0x10) != 0;
            trace!(
                "Cfgupdate Mask: {} -> {} -> {lower_flag_true}",
                0x10,
                (lower_flag & 0x10)
            );

            if lower_flag_true == set {
                #[cfg(debug_assertions)]
                trace!("Cfgupdate after {try_count} failed checks.");

                return Ok(());
            }

            #[cfg(debug_assertions)]
            {
                try_count += 1;
            }

            // Sleeping half the remaining period will speed up polling as the
            // deadline approaches.
            sleep((deadline.saturating_duration_since(Instant::now())) / 2);
        }

        #[cfg(debug_assertions)]
        trace!("Cfgupdate failure after {try_count} failed checks.");
        Err(I2CError::CfgupdateTimeout.into())
    }

    /// Full configuration update routine for block values.
    fn update_block_cfg(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
        subclass: u8,
        offset: u8,
        value: u16,
    ) -> Result<(), I2CErrorS> {
        Self::write_blockupdate_u16(spacing, i2c, subclass, offset, value)
    }

    /// Programs the design capacity.
    ///
    /// From BQ27427 Technical Reference Manual, page 17-18
    #[inline]
    fn set_design_capacity(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
    ) -> Result<(), I2CErrorS> {
        Self::update_block_cfg(spacing, i2c, STATE_SUBCLASS, 0, BATTERY_DESIGN_CAPACITY)
    }

    /// Programs the chemistry.
    ///
    /// From BQ27427 Technical Reference Manual, page 18
    fn set_chemistry_profile(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
    ) -> Result<(), I2CErrorS> {
        Self::write_u16(spacing, i2c, standard_commands::CONTROL, CHEM_ID)
    }

    /// Programs the sleep current.
    ///
    /// From BQ27427 Technical Reference Manual, page 46
    #[inline]
    fn set_sleep_current(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
    ) -> Result<(), I2CErrorS> {
        Self::update_block_cfg(spacing, i2c, STATE_SUBCLASS, 23, SLEEP_CURRENT)
    }

    /// Programs the notification delta to single percents.
    ///
    /// From BQ27427 Technical Reference Manual, page 46
    #[inline]
    fn set_soci_delta(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
    ) -> Result<(), I2CErrorS> {
        Self::update_block_cfg(spacing, i2c, STATE_SUBCLASS, 20, 1)
    }

    fn set_default_opconfig(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
    ) -> Result<(), I2CErrorS> {
        /// From on BQ27427 Technical Reference Manual, page 34.
        ///
        /// The defaults are unmodified, but being explicit doesn't hurt.
        const DEFAULT_OPCONFIG: u16 = u16::from_be_bytes([0x64, 0x78]);

        Self::unseal(spacing, i2c)?;
        Self::update_block_cfg(
            spacing,
            i2c,
            REGISTERS_SUBCLASS,
            OPCONFIG_OFFSET,
            DEFAULT_OPCONFIG,
        )?;
        Self::seal(spacing, i2c)
    }

    fn inner_sleep(
        spacing: &mut CommandSpacing,
        i2c: &mut MutexGuard<I2C>,
        enable: bool,
    ) -> Result<(), I2CErrorS> {
        let opconfig = {
            let mut opconfig_arr = [0];
            Self::read_block(
                spacing,
                i2c,
                REGISTERS_SUBCLASS,
                OPCONFIG_OFFSET + 1,
                &mut opconfig_arr,
            )?;
            opconfig_arr[0]
        };

        info!("Existing opconfig: {opconfig:08b}");

        // Mask in a 1 to enable, 0 to disable.
        let new_opconfig = if enable {
            opconfig | 0x20
        } else {
            opconfig & 0xDF
        };

        info!("New opconfig: {new_opconfig:08b}");

        Self::write_blockupdate(
            spacing,
            i2c,
            REGISTERS_SUBCLASS,
            OPCONFIG_OFFSET + 1,
            &[new_opconfig],
        )
    }

    pub fn sleep(&mut self, enable: bool) -> Result<(), I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();

        Self::unseal(&mut self.spacing, &mut held_i2c)?;
        Self::set_cfgupdate(&mut self.spacing, &mut held_i2c)?;

        Self::inner_sleep(&mut self.spacing, &mut held_i2c, enable)?;

        Self::soft_reset(&mut self.spacing, &mut held_i2c)?;
        Self::seal(&mut self.spacing, &mut held_i2c)
    }

    /// Performs the full initialization sequence, blocking as necessary.
    fn init(&mut self) -> Result<(), I2CErrorS> {
        // Trigger a time out as per manual page 15.
        info!("Battery monitor force timeout");
        self.i2c
            .lock()
            .unwrap()
            .hold_bus_low(Duration::from_secs(3))?;

        let mut held_i2c = self.i2c.lock().unwrap();

        info!("Battery monitor unseal");
        Self::unseal(&mut self.spacing, &mut held_i2c)?;
        info!("Battery monitor enter cfgupdate");
        Self::set_cfgupdate(&mut self.spacing, &mut held_i2c)?;

        // Disabling sleep gives other devices unstreched I2C clocks.
        info!("Battery monitor disable sleep");
        Self::inner_sleep(&mut self.spacing, &mut held_i2c, false)?;

        info!("Battery monitor design capacity");
        Self::set_design_capacity(&mut self.spacing, &mut held_i2c)?;
        info!("Battery monitor chem profile");
        Self::set_chemistry_profile(&mut self.spacing, &mut held_i2c)?;
        info!("Battery monitor sleep current");
        Self::set_sleep_current(&mut self.spacing, &mut held_i2c)?;
        info!("Battery monitor SOCI Delta");
        Self::set_soci_delta(&mut self.spacing, &mut held_i2c)?;

        info!("Battery monitor soft reset");
        Self::soft_reset(&mut self.spacing, &mut held_i2c)?;

        // Assert the chemical ID is set correctly.
        debug_assert_eq!(
            CHEM_ID,
            Self::read_u16(&mut self.spacing, &mut held_i2c, 0x0008)?
        );

        info!("Battery monitor seal");
        Self::seal(&mut self.spacing, &mut held_i2c)
    }

    #[inline]
    pub fn millivolts(&mut self) -> Result<u16, I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();
        Self::read_u16(&mut self.spacing, &mut held_i2c, standard_commands::VOLTAGE)
    }

    #[inline]
    pub fn full_available_capacity(&mut self) -> Result<MilliAmpHours, I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();
        Self::read_u16(
            &mut self.spacing,
            &mut held_i2c,
            standard_commands::FULL_AVAILABLE_CAPACITY,
        )
    }

    #[inline]
    pub fn remaining_capacity(&mut self) -> Result<MilliAmpHours, I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();
        Self::read_u16(
            &mut self.spacing,
            &mut held_i2c,
            standard_commands::REMAINING_CAPACITY,
        )
    }

    #[inline]
    pub fn full_charge_capacity(&mut self) -> Result<MilliAmpHours, I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();
        Self::read_u16(
            &mut self.spacing,
            &mut held_i2c,
            standard_commands::FULL_CHARGE_CAPACITY,
        )
    }

    #[inline]
    pub fn average_current(&mut self) -> Result<MilliAmps, I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();
        Self::read_u16(
            &mut self.spacing,
            &mut held_i2c,
            standard_commands::AVERAGE_CURRENT,
        )
    }

    #[inline]
    pub fn average_power(&mut self) -> Result<MilliWatts, I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();
        Self::read_u16(
            &mut self.spacing,
            &mut held_i2c,
            standard_commands::AVERAGE_POWER,
        )
    }

    /// Returns a raw state of charge.
    ///
    /// Does not take into account any adjustments. Call result with
    /// [`Self::state_of_charge`] for a system percent.
    #[inline]
    pub fn raw_state_of_charge(&mut self) -> Result<Percent, I2CErrorS> {
        let mut held_i2c = self.i2c.lock().unwrap();
        Self::read_u16(
            &mut self.spacing,
            &mut held_i2c,
            standard_commands::STATE_OF_CHARGE,
        )
        .inspect(|x| warn!("State of charge: 0x{x:04x}"))
        .map(|x| x as Percent)
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
            // Extension allows for the following multiplication
            let extended_num = raw_charge - self.soc_offset;
            // Going to do integer division by max soc, want percent * 100.
            let adjusted_num = extended_num * 100;
            Ok((adjusted_num / (self.soc_max - self.soc_offset)) as Percent)
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
