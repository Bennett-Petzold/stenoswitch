use gpio_cdev::{Chip, LineHandle, LineRequestFlags};

use crate::std_unwrap;

#[derive(Debug)]
pub struct ChgEn(LineHandle);

impl ChgEn {
    /// Defaults to disabled.
    pub fn new() -> Result<Self, gpio_cdev::Error> {
        let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null"))?;
        let this = Self(
            chip.get_line(std_unwrap(str::parse(
                option_env!("CHG_EN").unwrap_or("/dev/null"),
            )))?
            .request(LineRequestFlags::OUTPUT, 0, "battery_control")?,
        );
        this.disable()?;
        Ok(this)
    }
}

impl ChgEn {
    pub fn enable(&self) -> Result<(), gpio_cdev::Error> {
        self.0.set_value(1)
    }

    pub fn disable(&self) -> Result<(), gpio_cdev::Error> {
        self.0.set_value(0)
    }
}

impl Drop for ChgEn {
    fn drop(&mut self) {
        // Attempt to disable on drop
        let _ = self.disable();
    }
}
