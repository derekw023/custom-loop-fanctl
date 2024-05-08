#![no_std]
#![no_main]
// Panics are OK here as is using these libraries the way they want to be used
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::wildcard_imports)]

use pimoroni_tiny2040 as bsp;

use bsp::entry;

use cortex_m::prelude::*;
use panic_halt as _;

mod control_loop;
mod usb;
mod util;

use controller_lib::dsp;
pub(crate) const HEARTBEAT_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(100);
pub(crate) const STATUS_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(1);

#[entry]
fn main() -> ! {
    let mut peripherals = util::ControllerPeripherals::take(false).unwrap();

    // Hand off peripherals to the subsystems that need them
    control_loop::setup(&mut peripherals);
    usb::setup(&mut peripherals);

    loop {
        // peripherals.watchdog.feed();

        // event loop
        cortex_m::asm::wfi();
    }
}
