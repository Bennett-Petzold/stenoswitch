use std::env::args;

use gpio_cdev::{Chip, EventRequestFlags, EventType, LineRequestFlags};
use nix::{
    sys::time::TimeValLike,
    time::{ClockId, clock_gettime},
};

/// Waits to exit until the GPIO line given as the only argument to is disabled.
fn main() {
    let start_time = clock_gettime(ClockId::CLOCK_MONOTONIC)
        .expect("Well defined Linux call")
        .num_nanoseconds()
        .try_into()
        .unwrap_or(0_u64);

    let mut chip = Chip::new(option_env!("GPIO_CHIP").unwrap_or("/dev/null")).unwrap();

    let line = chip
        .get_line(str::parse(&args().nth(1).unwrap()).unwrap())
        .unwrap();

    let mut events = line
        .events(
            LineRequestFlags::INPUT,
            EventRequestFlags::FALLING_EDGE,
            "GPIO disable monitor",
        )
        .unwrap();

    // Exit on either false or first falling edge.
    if events.get_value().unwrap() != 0 {
        loop {
            let event = events.next().expect("GPIO events are infinite").unwrap();
            if event.event_type() == EventType::FallingEdge && event.timestamp() >= start_time {
                break;
            }
        }
    }
}
