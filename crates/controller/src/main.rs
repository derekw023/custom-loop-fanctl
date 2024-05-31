#![no_std]
#![no_main]
// Panics are OK here as is using these libraries the way they want to be used
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::wildcard_imports)]

// Global config, global exports and imports

use panic_halt as _;
pub(crate) use pimoroni_tiny2040 as bsp;

// Imports
mod adc;
mod control_loop;
mod dma;
mod usb;
mod util;

// Use statements for main
use bsp::entry;

#[entry]
fn main() -> ! {
    let mut peripherals = util::ControllerPeripherals::take(false).unwrap();
    let red = peripherals.red.take().unwrap();

    let mut dma = dma::Token::new(peripherals.dma.take().unwrap(), &mut peripherals.resets);

    let mut adc = adc::Token::new(
        peripherals.adc.take().unwrap(),
        &mut peripherals.resets,
        peripherals.thermistor_pin.take().unwrap(),
    )
    .unwrap();

    // Initialize objects with the peripherals created before
    let controller = control_loop::Token::new(&mut adc, &mut dma, red).unwrap();
    usb::setup(&mut peripherals, controller);

    peripherals.unmask_interrupts();

    loop {
        peripherals.watchdog.feed();

        // event loop
        cortex_m::asm::wfi();
    }
}
