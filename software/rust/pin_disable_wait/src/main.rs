use std::env::args;

use gpio_cdev::{Chip, EventRequestFlags, LineRequestFlags};

/// Waits to exit until the GPIO line given as the only argument to is disabled.
fn main() {
    let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null")).unwrap();

    let line = chip
        .get_line(str::parse(&args().nth(1).unwrap()).unwrap())
        .unwrap();

    // Exit on either false or first falling edge.
    if line
        .request(LineRequestFlags::INPUT, 0, "GPIO disable monitor")
        .unwrap()
        .get_value()
        .unwrap()
        == 0
    {
        line.events(
            LineRequestFlags::INPUT,
            EventRequestFlags::FALLING_EDGE,
            "GPIO disable monitor",
        )
        .unwrap();
    }
}
