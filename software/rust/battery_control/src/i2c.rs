use std::{
    hint::spin_loop,
    time::{Duration, Instant},
};

use bare_err_tree::err_tree;
use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use log::{debug, info, trace};
use thiserror::Error;

#[err_tree(I2CErrorS)]
#[derive(Debug, Error)]
pub enum I2CError {
    #[error(transparent)]
    Gpio(#[from] gpio_cdev::Error),
    #[error("NACK from slave")]
    Nack,
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
#[derive(Debug)]
pub struct I2C {
    // Half the clock duration, single edge of the square signal.
    line_hold_time: Duration,
    sda: LineHandle,
    scl: LineHandle,
    last_scl_change: Instant,
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
        trace!("Before chip");
        let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null"))?;
        trace!("Before SDA");
        let sda = chip.get_line(sda_pin)?.request(
            LineRequestFlags::OPEN_DRAIN.union(LineRequestFlags::OUTPUT),
            1,
            "battery_control",
        )?;
        trace!("Before SCL");
        let scl = chip.get_line(scl_pin)?.request(
            LineRequestFlags::OPEN_DRAIN.union(LineRequestFlags::OUTPUT),
            1,
            "battery_control",
        )?;

        let mut this = Self {
            line_hold_time,
            sda,
            scl,
            last_scl_change: Instant::now(),
        };

        trace!("Before reset");
        this.reset()?;

        Ok(this)
    }

    fn clock_tick(&mut self) -> Result<(), I2CErrorS> {
        trace!("Start tick");
        spin_sleep::sleep_until(self.last_scl_change + self.line_hold_time);
        trace!("End tick");
        Ok(())
    }

    /// Assumes SCL is low, reads a single bit over a whole clock cycle.
    ///
    /// Sets SCL to low at the end.
    fn read_bit(&mut self) -> Result<u8, I2CErrorS> {
        trace!("I2C bit read");
        debug_assert_eq!(self.scl.get_value().unwrap(), 0);

        self.clock_tick()?;
        self.scl_high()?;

        // Read halfway through the clock edge period.
        spin_sleep::sleep_until(self.last_scl_change + (self.line_hold_time / 2));
        let bit = self.sda.get_value();
        spin_sleep::sleep_until(self.last_scl_change + self.line_hold_time);

        self.scl_low()?;

        Ok(bit?)
    }

    fn scl_low(&mut self) -> Result<(), I2CErrorS> {
        self.scl.set_value(0)?;
        self.last_scl_change = Instant::now();
        Ok(())
    }

    fn scl_high(&mut self) -> Result<(), I2CErrorS> {
        self.scl.set_value(1)?;

        // Wait for clock stretching to finish.
        // If the clock isn't being stretched, the loop runs zero times.
        while self.scl.get_value()? == 0 {
            trace!("Clock stretching at {:?}", Instant::now());
            spin_loop();
        }

        self.last_scl_change = Instant::now();
        Ok(())
    }

    /// Assumes SDA and SCL are already set high.
    fn start(&mut self) -> Result<(), I2CErrorS> {
        trace!("I2C start");
        self.clock_tick()?;
        self.sda.set_value(0)?;
        self.clock_tick()?;
        self.scl_low()
    }

    /// Assumes SDA and SCL are set low.
    fn repeated_start(&mut self) -> Result<(), I2CErrorS> {
        trace!("I2C repeat start");
        self.sda.set_value(1)?;
        self.clock_tick()?;
        self.scl_high()?;
        self.sda.set_value(0)?;
        self.clock_tick()?;
        self.scl_low()
    }

    /// Assumes SCL is already set low.
    fn stop(&mut self) -> Result<(), I2CErrorS> {
        trace!("I2C stop");
        self.sda.set_value(0)?;
        self.clock_tick()?;
        self.scl_high()?;
        self.sda.set_value(1)?;

        Ok(())
    }

    /// Sends a single byte over i2c.
    ///
    /// Assumes write was already sent and SCL is set low.
    fn send_byte(&mut self, byte: u8) -> Result<(), I2CErrorS> {
        trace!("I2C send byte: {byte}");
        let mut mask: u8 = 0b1000_0000;

        for _ in 0..8 {
            // Isolates the particular bit as 0 or 1.
            //
            // See the bool definition -- true is 1, false is 0 in an int cast.
            let bit = ((byte | mask) != 0) as u8;
            self.sda.set_value(bit)?;

            self.clock_tick()?;
            self.scl_high()?;
            self.clock_tick()?;
            self.scl_low()?;

            // Shift mask to the next bit
            mask >>= 1;
        }

        // Get the ack bit
        self.sda.set_value(0)?;
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
        trace!("I2C read byte (final_read: {final_read})");
        let mut buffer: u8 = 0;

        for shift in (0..8).rev() {
            buffer |= self.read_bit()? << shift;
        }

        // Send nack on final part of read
        self.sda.set_value(final_read as u8)?;
        self.clock_tick()?;
        self.scl_high()?;
        self.clock_tick()?;
        self.scl_low()?;

        Ok(buffer)
    }

    pub fn reset(&mut self) -> Result<(), I2CErrorS> {
        debug!("I2C reset");
        self.start()?;

        // Nine pulses of highs.
        self.sda.set_value(1)?;
        for _ in 0..9 {
            self.clock_tick()?;
            self.scl_high()?;
            self.clock_tick()?;
            self.scl_low()?;
        }

        self.clock_tick()?;

        // Special start and stop combined sequence.
        self.scl_high()?;
        self.sda.set_value(0)?;
        self.clock_tick()?;
        self.sda.set_value(1)?;

        Ok(())
    }

    /// Send data over I2C.
    pub fn write<I>(&mut self, device_address: u8, bytes: I) -> Result<(), I2CErrorS>
    where
        I: IntoIterator<Item = u8>,
    {
        debug!("I2C write to {device_address}");
        // Fill in the LSB 0 bit for a write.
        let address_write = device_address << 1;

        self.start()?;
        self.send_byte(address_write)?;

        for byte in bytes {
            self.send_byte(byte)?;
        }

        self.stop()
    }

    /// Receive data over I2C.
    ///
    /// Reads in the full length of dest. Slice for the appropriate length.
    pub fn read(
        &mut self,
        device_address: u8,
        register: u8,
        dest: &mut [u8],
    ) -> Result<(), I2CErrorS> {
        debug!("Read {register} from {device_address}");
        // Fill in the LSB 0 bit for a write.
        let address_write = device_address << 1;
        // Fill in the LSB 1 bit for a read.
        let address_read = (device_address << 1) | 0x01;

        self.start()?;
        self.send_byte(address_write)?;
        self.send_byte(register)?;
        self.repeated_start()?;
        self.send_byte(address_read)?;

        let dest_final_entry = dest.len().saturating_sub(1);
        for byte in &mut dest[..dest_final_entry] {
            *byte = self.read_byte(false)?;
        }
        dest[dest_final_entry] = self.read_byte(true)?;

        self.stop()
    }
}
