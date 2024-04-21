#![no_std]
#![no_main]
// Panics are OK here as is using these libraries the way they want to be used
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::wildcard_imports)]

use pimoroni_tiny2040 as bsp;

use bsp::entry;

use cortex_m::prelude::*;
use panic_halt as _;

mod timers;
mod util;

use controller_lib::dsp;
pub(crate) static mut PERIPHERALS: Option<util::ControllerPeripherals> = None;
pub(crate) const HEARTBEAT_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(100);
pub(crate) const STATUS_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(1);

#[entry]
fn main() -> ! {
    let peripherals = util::ControllerPeripherals::take().unwrap();

    // Access to peripherals happens thru this global static in critical sections
    // or where some other mechanism ensures exclusive mutable access to specific resources
    unsafe {
        PERIPHERALS = Some(peripherals);
    }

    timers::setup();

    // It should be safe enough to hold a ref to the watchdog to avoid going into a critical section every wake
    let watchdog = cortex_m::interrupt::free(|_cs| unsafe {
        let peripherals = PERIPHERALS.as_mut().unwrap_unchecked();
        &mut peripherals.watchdog
    });
    // let watchdog = &mut peripherals.watchdog;

    loop {
        watchdog.feed();

        // event loop
        cortex_m::asm::wfi();
    }
}
