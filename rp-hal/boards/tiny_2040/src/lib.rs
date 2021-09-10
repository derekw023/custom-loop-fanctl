#![no_std]

pub extern crate rp2040_hal as hal;

#[cfg(feature = "rt")]
extern crate cortex_m_rt;
#[cfg(feature = "rt")]
pub use cortex_m_rt::entry;

pub use embedded_time;

pub use hal::pac;
use hal::pac::RESETS;

use hal::sio::SioGpioBank0;

hal::bsp_pins!(
    Gpio0 { name: gpio0 },
    Gpio1 { name: gpio1 },
    Gpio2 { name: gpio2 },
    Gpio3 { name: gpio3 },
    Gpio4 { name: gpio4 },
    Gpio5 { name: gpio5 },
    Gpio6 { name: gpio6 },
    Gpio7 { name: gpio7 },
    Gpio18 {
        name: led_r,
        aliases: { PushPullOutput: LedR }
    },
    Gpio19 {
        name: led_g,
        aliases: { PushPullOutput: LedG }
    },
    Gpio20 {
        name: led_b,
        aliases: { PushPullOutput: LedB }
    },
    Gpio26 { name: adc0 },
    Gpio27 { name: adc1 },
    Gpio28 { name: adc2 },
    Gpio29 { name: adc3 },
);
pub const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

pub struct Tiny2040 {
    pub pins: Pins,
}

impl Tiny2040 {
    pub fn new(
        io: pac::IO_BANK0,
        pads: pac::PADS_BANK0,
        sio: SioGpioBank0,
        resets: &mut RESETS,
    ) -> Self {
        Self {
            pins: Pins::new(io, pads, sio, resets),
        }
    }
}
