use std::{io::Write, mem::MaybeUninit, thread::sleep, time::Duration};

use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use log::debug;
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

/// Bounded [0V, 5V] as per [`REF_VOLTAGE`].
type Voltage = f32;

pub type Amps = f32;

/// Measurements assume the system keeps a clean 5V input as reference.
const REF_VOLTAGE: Voltage = 5.0;

/// Values from the MCP3204 datasheet.
/// <https://ww1.microchip.com/downloads/en/DeviceDoc/21298e.pdf>
mod mcp_defs {
    /// There are only two channel bits at the front.
    pub type ChannelId = u8;
    /// Packet size for a full communication.
    pub const REQUEST_LEN: usize = 3;
    /// A full communication packet.
    pub type Request = [u8; REQUEST_LEN];

    /// Stored in u16 but only 12 bits long.
    pub type AdcVal = u16;
    /// Stored as a float for fractional division.
    pub const ADC_MAX: f32 = (2_u16.pow(12) - 1) as f32;

    /// Request single line reading with the ignored D2 as 1.
    /// 00000111 = 7
    const START_SINGLE: u8 = 7;

    pub const CH0: ChannelId = 0;
    pub const CH1: ChannelId = 1 << 6;
    pub const CH2: ChannelId = 1 << 7;
    pub const CH3: ChannelId = CH1 | CH2;
    pub const ALL_CHANNELS: [ChannelId; 4] = [CH0, CH1, CH2, CH3];

    #[inline]
    /// Generate the write portion to read an ADC channel.
    pub const fn request(id: ChannelId) -> Request {
        // Last line is ignored, 0 avoids a line change.
        [START_SINGLE, id, 0]
    }
}

const CC1: mcp_defs::ChannelId = mcp_defs::CH2;
const CC2: mcp_defs::ChannelId = mcp_defs::CH3;
const CC_CHANNELS: [mcp_defs::ChannelId; 2] = [CC1, CC2];

const CHRG_DIVH: mcp_defs::ChannelId = mcp_defs::CH1;
const CHRG_DIVL: mcp_defs::ChannelId = mcp_defs::CH0;

const fn adc_to_voltage(adc_val: mcp_defs::AdcVal) -> Voltage {
    ((adc_val as f32) / mcp_defs::ADC_MAX) * REF_VOLTAGE
}

/// From USB Type-C Spec Release 2.0, Table 4-36
#[derive(Debug, Clone, Copy)]
pub enum AmpLimit {
    PreConnect,
    Standard,
    OneHalf,
    Three,
}

impl AmpLimit {
    fn from_cc_volts(volts: f32) -> Self {
        match volts {
            x if x >= 1.31 => Self::Three,
            x if x >= 0.70 => Self::OneHalf,
            x if x >= 0.25 => Self::Standard,
            _ => Self::PreConnect,
        }
    }

    pub fn to_milliamps(self) -> u16 {
        match self {
            Self::PreConnect => 100,
            Self::Standard => 500,
            Self::OneHalf => 1500,
            Self::Three => 3000,
        }
    }

    pub fn to_amps(self) -> Amps {
        match self {
            Self::PreConnect => 0.1,
            Self::Standard => 0.5,
            Self::OneHalf => 1.5,
            Self::Three => 3.0,
        }
    }
}

#[derive(Debug)]
pub struct CurrentMonitor {
    spi: Spidev,
}

impl CurrentMonitor {
    /// Creates all interfaces, panicking on hardware issues.
    pub fn new() -> Self {
        let spi = {
            let mut spi = Spidev::open(env!("CURRENT_MONITOR_SPI")).unwrap();
            spi.configure(
                &SpidevOptions::new()
                    // Standard bits per word
                    .bits_per_word(8)
                    // System runs at 2 MHz max
                    .max_speed_hz(2_000_000)
                    .mode(SpiModeFlags::SPI_MODE_0)
                    .build(),
            )
            .unwrap();
            spi
        };

        Self { spi }
    }
}

impl Default for CurrentMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentMonitor {
    /// Returns the ADC reading for a given channel.
    fn read_channel(&mut self, id: mcp_defs::ChannelId) -> mcp_defs::AdcVal {
        // For initial debug purposes, fill this to make null or not reading obvious
        //let mut read_buf: [u8; mcp_defs::REQUEST_LEN] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut read_buf = [255; mcp_defs::REQUEST_LEN];
        self.spi
            .transfer(&mut SpidevTransfer::read_write(
                &mcp_defs::request(id),
                &mut read_buf,
            ))
            .unwrap();

        let _ = self.spi.write_all(&[0]); // Guarantee spacing before next request.

        // Since the value is 12 bits long, set the first 4 garbage bits to 0.
        let be_bytes = [read_buf[1] & 0x0F, read_buf[2]];
        let adc_val = u16::from_be_bytes(be_bytes);

        let voltage = adc_to_voltage(adc_val);

        debug!(
            "Read current_monitor {id:?} as RAW {read_buf:?}, U16 {adc_val}, MEASURE {voltage}V"
        );

        adc_val
    }

    /// Returns the CC amperage limit.
    pub fn read_cc(&mut self) -> AmpLimit {
        let adc_val = CC_CHANNELS
            .iter()
            .map(|channel| self.read_channel(*channel))
            .max()
            .expect("Always >0 elements, CC_CHANNELS has 2.");

        AmpLimit::from_cc_volts(adc_to_voltage(adc_val))
    }

    /// Returns the charger input current limit in amps.
    pub fn read_current_limit(&mut self) -> Amps {
        const UPPER_DIV_OHMS: f32 = 10_000.0;

        let divh = self.read_channel(CHRG_DIVH);
        let divl = self.read_channel(CHRG_DIVL);

        let volt_diff = adc_to_voltage(divh - divl);
        let divider_current = volt_diff / UPPER_DIV_OHMS;
        let variable_resistor = adc_to_voltage(divl) / divider_current;

        // See the MP2637 datasheet page 27 for the I_ILIM equation used
        45_000.0 / (UPPER_DIV_OHMS + variable_resistor)
    }
}
