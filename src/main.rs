#![no_std]
#![no_main]

extern crate stm32g0xx_hal as hal;

extern crate panic_halt;

use hal::prelude::*;

use hal::analog::adc::{Precision, SampleTime};
use hal::stm32::{CorePeripherals, Peripherals};

use cortex_m_rt::entry;

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

    //TODO: Temperature conversion
    //TODO: Fan curve implementation
    loop {
        let water_temp: u16 = converter.read(&mut temp_sensor).expect("ADC read failed");
        fan_level.set_duty(if water_temp > fan_max {
            fan_max
        } else {
            water_temp
        });
        delay.delay(100.ms());
    }
}
