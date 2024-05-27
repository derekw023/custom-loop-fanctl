#![no_std]
#![no_main]
// Panics are OK here as is using these libraries the way they want to be used
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::wildcard_imports)]

// Global config, global exports and imports

use panic_halt as _;
pub(crate) use pimoroni_tiny2040 as bsp;
pub(crate) const HEARTBEAT_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(100);
pub(crate) const STATUS_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(1);

// Imports
mod adc;
mod control_loop;
mod dma;
mod usb;
mod util;

// Use statements for main
use bsp::entry;
use controller_lib::dsp;

#[entry]
fn main() -> ! {
    let mut peripherals = util::ControllerPeripherals::take(false).unwrap();

    let dma = dma::Token::new(peripherals.dma.take().unwrap(), &mut peripherals.resets);

    let adc = adc::Token::new(peripherals.adc.take().unwrap(), &mut peripherals.resets).unwrap();

    // Hand off peripherals to the subsystems that need them
    let controller = control_loop::Token::new(&mut peripherals, &adc);
    usb::setup(&mut peripherals);

    loop {
        // peripherals.watchdog.feed();

        // event loop
        cortex_m::asm::wfi();
    }
}
