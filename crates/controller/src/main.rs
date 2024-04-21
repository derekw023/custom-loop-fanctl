#![no_std]
#![no_main]
// Panics are OK here as is using these libraries the way they want to be used
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::wildcard_imports)]

use hal::timer::{Alarm0, Alarm2};
use pimoroni_tiny2040 as bsp;

use bsp::entry;
use bsp::hal;

use cortex_m::prelude::*;
use once_cell::unsync::Lazy;
use panic_halt as _;

mod interrupts;
mod util;

use controller_lib::{dsp, Degrees};
pub(crate) static mut PERIPHERALS: Option<util::ControllerPeripherals> = None;
pub(crate) const HEARTBEAT_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(100);
pub(crate) const STATUS_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(1);

pub(crate) static mut CURRENT_TEMP: Degrees = Degrees::from_int(65);
pub(crate) static mut TEMP: Lazy<dsp::MovingAverage<Degrees>> = Lazy::new(dsp::MovingAverage::new);
pub(crate) static mut CURRENT_DUTY: u16 = 0;

pub(crate) static mut ALARM0: Option<Alarm0> = None;
pub(crate) static mut ALARM2: Option<Alarm2> = None;

#[entry]
fn main() -> ! {
    let peripherals = util::ControllerPeripherals::take().unwrap();

    interrupts::setup(peripherals);

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
