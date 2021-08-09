#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::adc::OneShot;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::watchdog::WatchdogEnable;
use hal::pac;
use hal::sio::Sio;
use hal::watchdog::Watchdog;
use panic_halt as _;
use tiny_2040::embedded_time::duration::*;
use tiny_2040::hal;
use tiny_2040::hal::adc::Adc;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER;

#[derive(PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

// Convert 12 bit ADC reads into degrees celsius using a best-effort linear fit
impl From<u16> for Degrees {
    fn from(val: u16) -> Self {
        // V_out = Vin*R / (R + 10000) // External voltage divider with 10k
        // V_out * R + V_out * 10000 = 3.3*R
        // V_out * 10000 = R*(Vin - V_out)
        // R = V_out * 10000/(Vin - V_out)

        // V_out = Vin * val / 2^12 // ADC conversion

        // Simplify out Vin and V_out to operate only on counts
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
    let mut peripherals = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(peripherals.WATCHDOG);

    if let Ok(clocks) = hal::clocks::init_clocks_and_plls(
        tiny_2040::XOSC_CRYSTAL_FREQ,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    ) {
        // Turn a light on and off
        let sio = Sio::new(peripherals.SIO);
        let (mut board, pins) = tiny_2040::Tiny2040::new(
            peripherals.IO_BANK0,
            peripherals.PADS_BANK0,
            sio.gpio_bank0,
            Adc::new(peripherals.ADC, &mut peripherals.RESETS),
            &mut peripherals.RESETS,
        );

        let mut temp_sensor = board.adc.enable_temp_sensor();
        let first_temp = board.adc.read(&mut temp_sensor).unwrap();
        board.led_b.set_high().unwrap();
        loop {
            let temp: u16 = board.adc.read(&mut temp_sensor).unwrap();

            if temp < first_temp {
                board.led_r.set_low().unwrap();
                board.led_g.set_high().unwrap();
            } else {
                board.led_r.set_high().unwrap();
                board.led_g.set_low().unwrap();
            }

            // TODO: Replace with proper 1s delays once we have clocks working
            cortex_m::asm::delay(5_000_000);
            cortex_m::asm::delay(5_000_000);
        }
    } else {
        loop {} // infinite loop in case of init error
    }

    //     let mut converter = unimplemented!();
    //
    //     let mut delay = unimplemented!();
    //
    //     let mut fan_level = unimplemented!();
    //
    //     delay.delay(20.us()); // Wait for ADC voltage regulator to stabilize
    //
    //     let mut temp_sensor = pins.pa0.into_analog();
    //
    //     //TODO: Fan curve implementation
    //     loop {
    //         let counts: u16 = converter.read(&mut temp_sensor).expect("ADC read failed");
    //         let water_temp: Degrees = counts.into();
    //         fan_level.set_duty(if water_temp > Degrees(50.) {
    //             fan_max
    //         } else {
    //             0
    //         });
    //         delay.delay(1000.ms());
    //     }
}
