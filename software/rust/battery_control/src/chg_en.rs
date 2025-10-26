use gpio_cdev::{Chip, LineHandle, LineRequestFlags};

#[derive(Debug)]
pub struct ChgEn(LineHandle);

impl ChgEn {
    /// Panics if the system is misconfigured.
    ///
    /// Defaults to disabled.
    pub fn new() -> Self {
        let mut chip = Chip::new(env!("GPIO_CHIP")).unwrap();
        let this = Self(
            chip.get_line(str::parse(env!("CHG_EN")).unwrap())
                .unwrap()
                .request(LineRequestFlags::OUTPUT, 0, "battery_control")
                .unwrap(),
        );
        this.disable().unwrap();
        this
    }

    /// [`Self::new`] returning None instead of panicking.
    ///
    /// Defaults to disabled.
    pub fn maybe_new() -> Option<Self> {
        let mut chip = Chip::new(env!("GPIO_CHIP")).ok()?;
        let this = Self(
            chip.get_line(str::parse(env!("CHG_EN")).unwrap())
                .ok()?
                .request(LineRequestFlags::OUTPUT, 0, "battery_control")
                .ok()?,
        );
        this.disable().ok()?;
        Some(this)
    }
}

impl Default for ChgEn {
    fn default() -> Self {
        Self::new()
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
