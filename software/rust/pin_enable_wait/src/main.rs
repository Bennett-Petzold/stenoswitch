use std::env::args;

use gpio_cdev::{Chip, EventRequestFlags, LineRequestFlags};

/// Waits to exit until the GPIO line given as the only argument to is enabled.
fn main() {
    let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null")).unwrap();

    let line = chip
        .get_line(str::parse(&args().nth(1).unwrap()).unwrap())
        .unwrap();

    let mut events = line
        .events(
            LineRequestFlags::INPUT,
            EventRequestFlags::RISING_EDGE,
            "GPIO enable monitor",
        )
        .unwrap();

    // Exit on true or first rising edge.
    if events.get_value().unwrap() != 0 {
        let _ = events.next();
    }
}
