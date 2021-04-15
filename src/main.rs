#![no_std]
#![no_main]

extern crate stm32g0xx_hal as hal;

extern crate panic_halt;

use hal::prelude::*;

use hal::analog::adc::{Precision, SampleTime};
use hal::stm32::{CorePeripherals, Peripherals};

use core::cmp::{PartialEq, PartialOrd};
use core::convert::From;

use cortex_m_rt::entry;

#[derive(PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

// Convert 12 bit ADC reads into degrees celsius using a best-effort linear fit
impl From<u16> for Degrees {
    fn from(val: u16) -> Self {
        // Vout = Vin*R / (R + 10000) // External voltage divider with 10k
        // Vout * R + Vout * 10000 = 3.3*R
        // Vout * 10000 = R*(Vin - Vout)
        // R = Vout * 10000/(Vin - Vout)

        // Vout = Vin * val / 2^12 // ADC conversion

        // Simplify out Vin and Vout to operate only on counts
        // R = (Vin * val / 2^12) * 10000/(Vin - Vin * val / 2^12)
        // R = (val / 2^12) * 10000/(1- val / 2^12)
        // R = (val) * 10000/(2^12 - val) // Check my math (no really please check my math)

        // From curve fit on R-T table this is the function for a 2 point cal on 25deg and 50deg
        // C = −4.2725*R+65.753

        let r: f32 = ((val) * 10000 / ((2 ^ 12) - val)).into(); // Culminate all of the above math

        // Would be cool to figure out a trick to avoid floating point here, the size cost is nearly 1.4k
        Degrees(-4.2725 * r + 65.753)
    }
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();

    let mut rcc = peripherals.RCC.constrain();

    let mut delay = core.SYST.delay(&mut rcc);

    let pins = peripherals.GPIOA.split(&mut rcc);
    let pwm = peripherals.TIM1.pwm(10.khz(), &mut rcc);

    let mut fan_level = pwm.bind_pin(pins.pa8); // D
    let fan_max = fan_level.get_max_duty();
    fan_level.enable();

    let mut converter = peripherals.ADC.constrain(&mut rcc);

    converter.set_sample_time(SampleTime::T_80);
    converter.set_precision(Precision::B_12);
    delay.delay(20.us()); // Wait for ADC voltage regulator to stabilize

    let mut temp_sensor = pins.pa0.into_analog();

    //TODO: Fan curve implementation
    loop {
        let counts: u16 = converter.read(&mut temp_sensor).expect("ADC read failed");
        let water_temp: Degrees = counts.into();
        fan_level.set_duty(if water_temp > Degrees(50.) {
            fan_max
        } else {
            0
        });
        delay.delay(100.ms());
    }
}
