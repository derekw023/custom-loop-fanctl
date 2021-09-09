#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::PwmPin;
use hal::pac;
use hal::sio::Sio;
use hal::watchdog::Watchdog;
extern crate panic_halt;
use tiny_2040::embedded_time::duration::*;
use tiny_2040::hal;
use tiny_2040::hal::adc::Adc;
use tiny_2040::hal::clocks::ClockSource;
use tiny_2040::hal::gpio::{OutputDriveStrength::*, OutputSlewRate::*};
use tiny_2040::hal::pwm::Pwm0;

mod fan_controller;

use crate::fan_controller::{controller::FanController, temperature::Degrees};

// Second stage bootloader configures flash
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER;

#[entry]
fn main() -> ! {
    let mut peripherals = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(peripherals.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        tiny_2040::XOSC_CRYSTAL_FREQ,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // Configure PWMs first because HAL consumes PAC resources by reference
    let mut fan = Pwm0::new(0);
    fan.default_config(
        &mut peripherals.PWM,
        &mut peripherals.PADS_BANK0,
        &mut peripherals.IO_BANK0,
        &mut peripherals.RESETS,
    );

    let sio = Sio::new(peripherals.SIO);
    let mut board = tiny_2040::Tiny2040::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    // systick based delay
    let mut delay =
        cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.get_freq().integer());

    fan.enable();
    fan.set_top(u16::MAX / 16); // Increase frequency by a factor of 16 to push it out of the audible range

    board.pins.gpio0.set_drive_strength(EightMilliAmps);
    board.pins.gpio0.set_slew_rate(Fast);

    let mut converter = Adc::new(peripherals.ADC, &mut peripherals.RESETS);

    // Initialize fan control blocks
    let mut controller = FanController::new(
        board.pins.adc0.into_floating_input(),
        fan,
        u16::MAX / 16,
        u16::MAX / (16 * 8), // 12.5% or so of full scale, should spin everything up
        Degrees(55.),
        Degrees(35.),
    );

    loop {
        // Read ADC, filter, convert to temperature and apply fan curve
        controller.fan_curve(&mut converter);

        //TODO: On-the-fly updating of fan curves over USB and/or a way to report temperatures measured

        delay.delay_ms(100);
    }
}
