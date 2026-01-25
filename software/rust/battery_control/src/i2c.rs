//! TODO: Handle bus free times when switching between devices.
use std::{
    hint::spin_loop,
    time::{Duration, Instant},
};

use bare_err_tree::err_tree;
use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use log::{debug, info, trace};
use thiserror::Error;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct I2CDevice {
    pub address: u8,
    pub bus_free_time: Duration,
}

/// Special I2C device used to indicate a reset command.
/// Free time is set arbitrarily high to capture any reasonable duration.
const I2C_RESET: I2CDevice = I2CDevice {
    address: 0,
    bus_free_time: Duration::from_millis(1),
};

#[err_tree(I2CErrorS)]
#[derive(Debug, Error)]
pub enum I2CError {
    #[error(transparent)]
    Gpio(#[from] gpio_cdev::Error),
    #[error("NACK from slave")]
    Nack,
    #[error("Configuration update mode timeout")]
    CfgupdateTimeout,
    #[error("Oversized block write")]
    OversizedBlockWrite,
}

impl From<gpio_cdev::Error> for I2CErrorS {
    #[track_caller]
    fn from(value: gpio_cdev::Error) -> Self {
        I2CError::from(value).into()
    }
}

impl I2CErrorS {
    #[track_caller]
    fn nack() -> Self {
        I2CError::Nack.into()
    }
}

/// Software implementation of I2C.
///
/// Necessary because the Raspberry Pi Zero 2 W I2C hardware does not implement
/// clock stretching properly.
///
/// I probably could have used the built in I2C driver, but I didn't.
#[derive(Debug)]
pub struct I2C {
    // Half the clock duration, single edge of the square signal.
    line_hold_time: Duration,
    sda: LineHandle,
    scl: LineHandle,
    last_scl_change: Instant,
    device_address: u8,
    #[cfg(debug_assertions)]
    mid_transaction: bool,
}

impl I2C {
    /// Initializes the i2c connection.
    ///
    /// Frequency is in hertz.
    pub fn new(frequency: u32, sda_pin: u32, scl_pin: u32) -> Result<Self, I2CErrorS> {
        // Frequency is doubled to get the duration for a single edge.
        let line_hold_time = Duration::from_secs(1) / (frequency * 2);
        info!("I2C frequency, line hold time: {frequency} Hz, {line_hold_time:?}");

        // Both lines sit in the default high state.
        let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null"))?;
        let scl = chip.get_line(scl_pin)?.request(
            LineRequestFlags::OPEN_DRAIN.union(LineRequestFlags::OUTPUT),
            1,
            "battery_control",
        )?;
        let sda = chip.get_line(sda_pin)?.request(
            LineRequestFlags::OPEN_DRAIN.union(LineRequestFlags::OUTPUT),
            1,
            "battery_control",
        )?;

        let mut this = Self {
            line_hold_time,
            sda,
            scl,
            last_scl_change: Instant::now(),
            device_address: I2C_RESET.address,
            #[cfg(debug_assertions)]
            mid_transaction: false,
        };

        // Establishes consistent I2C state.
        this.reset()?;

        Ok(this)
    }

    fn clock_tick(&mut self) -> Result<(), I2CErrorS> {
        let next_tick = self.last_scl_change + self.line_hold_time;

        #[cfg(debug_assertions)]
        if self.mid_transaction {
            let now = Instant::now();
            assert!(
                next_tick > Instant::now(),
                "next tick ({next_tick:?}) should be ahead of now ({now:?})"
            );
        }

        spin_sleep::sleep_until(next_tick);
        Ok(())
    }

    /// Assumes SCL is low, reads a single bit over a whole clock cycle.
    ///
    /// Sets SCL to low at the end.
    fn read_bit(&mut self) -> Result<u8, I2CErrorS> {
        debug_assert_eq!(self.scl.get_value().unwrap(), 0);

        self.scl_high()?;

        // Read halfway through the clock edge period.
        spin_sleep::sleep_until(self.last_scl_change + (self.line_hold_time / 2));
        let bit = self.sda.get_value()?;

        self.scl_low()?;

        Ok(bit)
    }

    fn scl_low(&mut self) -> Result<(), I2CErrorS> {
        debug_assert_eq!(self.scl.get_value()?, 1);
        self.clock_tick()?;
        self.scl.set_value(0)?;
        self.last_scl_change = Instant::now();
        Ok(())
    }

    fn scl_high(&mut self) -> Result<(), I2CErrorS> {
        self.clock_tick()?;
        self.scl.set_value(1)?;

        // Wait for clock stretching to finish.
        // If the clock isn't being stretched, the loop runs zero times.
        #[cfg(debug_assertions)]
        let mut stretched = None;

        while self.scl.get_value()? == 0 {
            trace!("Clock stretching at {:?}", Instant::now());
            #[cfg(debug_assertions)]
            if stretched.is_none() {
                stretched = Some(Instant::now());
            }

            spin_loop();
        }

        #[cfg(debug_assertions)]
        if let Some(start) = stretched {
            trace!("Clock was stretched for {:?}", Instant::now() - start);
        }

        self.last_scl_change = Instant::now();
        Ok(())
    }

    /// Assumes SDA and SCL are already set high.
    fn start(&mut self) -> Result<(), I2CErrorS> {
        trace!("I2C start");
        debug_assert_eq!(self.sda.get_value()?, 1);
        debug_assert_eq!(self.scl.get_value()?, 1);
        // Spacing an extra period so start is properly registered.
        self.scl_high()?;
        self.sda.set_value(0)?;
        self.scl_low()?;

        Ok(())
    }

    /// Assumes SDA and SCL are set low.
    fn repeated_start(&mut self) -> Result<(), I2CErrorS> {
        trace!("I2C repeat start");
        debug_assert_eq!(self.scl.get_value()?, 0);
        self.sda.set_value(1)?;
        self.scl_high()?;
        self.sda.set_value(0)?;
        self.scl_low()
    }

    /// Assumes SCL is already set low.
    fn stop(&mut self) -> Result<(), I2CErrorS> {
        trace!("I2C stop");
        // Set to initial low before clock to set up rise
        debug_assert_eq!(self.scl.get_value()?, 0);
        self.sda.set_value(0)?;

        self.scl_high()?;
        debug_assert_eq!(self.sda.get_value()?, 0);

        // Spacing an extra period so stop is properly registered.
        self.scl_high()?;
        self.sda.set_value(1)?;

        #[cfg(debug_assertions)]
        {
            self.mid_transaction = false;
        }

        Ok(())
    }

    /// Wraps a function to always I2C stop on failure.
    fn emergency_stop<T, F>(&mut self, function: F) -> Result<T, I2CErrorS>
    where
        F: FnOnce(&mut Self) -> Result<T, I2CErrorS>,
    {
        match function(self) {
            Ok(res) => Ok(res),
            Err(e) => {
                trace!("I2C emergency stop on failure");
                self.reset()?;

                Err(e)
            }
        }
    }

    /// Sends a single byte over i2c.
    ///
    /// Assumes write was already sent and SCL is set low.
    fn send_byte(&mut self, byte: u8) -> Result<(), I2CErrorS> {
        trace!("I2C send byte");
        let mut mask: u8 = 0b1000_0000;

        for _ in 0..8 {
            // Isolates the particular bit as 0 or 1.
            //
            // See the bool definition -- true is 1, false is 0 in an int cast.
            let bit = ((byte & mask) != 0) as u8;
            self.sda.set_value(bit)?;

            self.scl_high()?;
            self.scl_low()?;

            // Shift mask to the next bit
            mask >>= 1;
        }

        // Get the ack bit
        self.sda.set_value(1)?;
        if self.read_bit()? == 0 {
            Ok(())
        } else {
            Err(I2CErrorS::nack())
        }
    }

    /// Reads a single byte over i2c.
    ///
    /// Assumes read was already set up and SCL is set low.
    fn read_byte(&mut self, final_read: bool) -> Result<u8, I2CErrorS> {
        let mut buffer: u8 = 0;

        debug_assert_eq!(self.scl.get_value().unwrap(), 0);
        // Set to drain so slave can drive the line.
        self.sda.set_value(1)?;

        for shift in (0..8).rev() {
            buffer |= self.read_bit()? << shift;
        }

        // Send nack on final part of read
        self.sda.set_value(final_read as u8)?;
        self.scl_high()?;
        self.scl_low()?;

        Ok(buffer)
    }

    // Holds both SDA and SCL low for at least the given time.
    pub fn hold_bus_low(&mut self, length: Duration) -> Result<(), I2CErrorS> {
        self.scl_low()?;
        self.sda.set_value(0)?;
        std::thread::sleep(length);
        self.stop()
    }

    pub fn bus_clear(&mut self) -> Result<(), I2CErrorS> {
        debug_assert_eq!(self.scl.get_value().unwrap(), 1);
        self.sda.set_value(1)?;

        // Arbitrarily high bus rest time
        std::thread::sleep(Duration::from_secs(1));

        if self.sda.get_value()? == 0 {
            trace!("I2C bus stuck. Clearing...");
        } else {
            trace!("I2C bus not stuck.");
            self.start()?;
            self.sda.set_value(1)?;
        }

        self.scl_high()?;
        for _pulse in 0..8 {
            self.scl_low()?;
            self.scl_high()?;
        }
        assert_eq!(self.sda.get_value().unwrap(), 1);
        self.scl_low()?;

        self.stop()?;

        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), I2CErrorS> {
        self.emergency_stop(|this| {
            debug!("I2C reset");
            this.device_address = I2C_RESET.address;

            // Allows all devices to meet bus free time periods.
            std::thread::sleep(I2C_RESET.bus_free_time);

            this.sda.set_value(1)?;
            this.scl_high()?;
            this.start()?;

            // Nine pulses of highs.
            this.sda.set_value(1)?;
            for _ in 0..9 {
                this.scl_high()?;
                this.scl_low()?;
            }

            // -- Special start and stop combined sequence. -- //
            debug_assert_eq!(this.sda.get_value()?, 1);
            debug_assert_eq!(this.scl.get_value()?, 0);

            // Start
            this.scl_high()?;
            this.sda.set_value(0)?;

            // Stop
            this.scl_high()?;
            this.sda.set_value(1)?;
            #[cfg(debug_assertions)]
            {
                this.mid_transaction = false;
            }
            debug_assert_eq!(this.scl.get_value()?, 1);
            debug_assert_eq!(this.sda.get_value()?, 1);
            // -- //

            debug!("Finished I2C reset");
            Ok(())
        })
    }

    /// Waits for bus free times on device switches.
    fn free_time(&mut self, device: I2CDevice) {
        if self.device_address != device.address {
            debug!("Switching I2C devices");
            self.device_address = device.address;
        }
        // Doubling the time helps devices hit their target waits.
        trace!("Waiting for bus free time...");
        //std::thread::sleep(device.bus_free_time * 2);
        spin_sleep::sleep_until(self.last_scl_change + device.bus_free_time);
    }

    /// Send data over I2C.
    pub fn write(&mut self, device: I2CDevice, bytes: &[u8]) -> Result<(), I2CErrorS> {
        trace!("I2C write to {:02x}", device.address);
        // Fill in the LSB 0 bit for a write.
        let address_write = device.address << 1;

        self.emergency_stop(|this| {
            this.free_time(device);

            this.start()?;
            this.send_byte(address_write)?;

            for byte in bytes {
                this.send_byte(*byte)?;
            }

            this.stop()
        })
    }

    /// Receive data over I2C.
    ///
    /// Reads in the full length of dest. Slice for the appropriate length.
    pub fn read(
        &mut self,
        device: I2CDevice,
        register: u8,
        dest: &mut [u8],
    ) -> Result<(), I2CErrorS> {
        debug!("I2C read 0x{register:02x} from 0x{:02x}", device.address);
        debug_assert_eq!(self.scl.get_value()?, 1);
        // Fill in the LSB 0 bit for a write.
        let address_write = device.address << 1;
        // Fill in the LSB 1 bit for a read.
        let address_read = (device.address << 1) | 0x01;

        self.emergency_stop(|this| {
            this.free_time(device);

            this.start()?;
            this.send_byte(address_write)?;
            this.send_byte(register)?;
            this.repeated_start()?;
            this.send_byte(address_read)?;

            let dest_final_entry = dest.len().saturating_sub(1);
            for byte in &mut dest[..dest_final_entry] {
                *byte = this.read_byte(false)?;
            }
            dest[dest_final_entry] = this.read_byte(true)?;

            this.stop()
        })
    }

    /// Receive single byte over I2C.
    #[allow(unused)]
    pub fn quick_read(&mut self, device: I2CDevice) -> Result<u8, I2CErrorS> {
        debug!("Quick read from 0x{:02x}", device.address);
        // Fill in the LSB 1 bit for a read.
        let address_read = (device.address << 1) | 0x01;

        self.free_time(device);

        self.start()?;
        self.send_byte(address_read)?;
        let data = self.read_byte(true)?;

        self.stop()?;
        Ok(data)
    }
}
