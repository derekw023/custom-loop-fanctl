#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::PwmPin;
use hal::pac;
use hal::pwm::Slices;
use hal::sio::Sio;
use hal::watchdog::Watchdog;
use tiny_2040::embedded_time::duration::*;
use tiny_2040::hal;
use tiny_2040::hal::adc::Adc;
use tiny_2040::hal::clocks::ClockSource;
extern crate panic_halt;
mod fan_controller;
use embedded_hal::digital::v2::OutputPin;
extern crate jlink_rtt;
use core::fmt::Write;

use crate::fan_controller::{controller::FanController, temperature::Degrees};

// Second stage bootloader configures flash
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER;

#[entry]
fn main() -> ! {
    let mut output = jlink_rtt::Output::new();
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

    let _ = writeln!(output, "Hello world!");

    let sio = Sio::new(peripherals.SIO);
    let board = tiny_2040::Tiny2040::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    let mut red = board.pins.led_r.into_push_pull_output();
    let mut green = board.pins.led_g.into_push_pull_output();
    let mut blue = board.pins.led_b.into_push_pull_output();

    red.set_high().unwrap();
    green.set_high().unwrap();
    blue.set_high().unwrap();

    // systick based delay
    let mut delay =
        cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.get_freq().integer());

    // Configure PWMs
    let mut slices = Slices::new(peripherals.PWM, &mut peripherals.RESETS);
    slices.pwm0.set_top(u16::MAX / 16);
    let mut pwm = slices.pwm0;
    pwm.enable();
    pwm.set_ph_correct();

    let fan = &mut pwm.channel_a;

    let _fanpin = fan.output_to(board.pins.gpio0);
    fan.enable();

    let mut converter = Adc::new(peripherals.ADC, &mut peripherals.RESETS);

    // Initialize fan control block
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

        let temp = controller.last_temp;
        //TODO: On-the-fly updating of fan curves over USB and/or a way to report temperatures measured

        writeln!(output, "T: {}°C, D: {}", temp, controller.current_duty).unwrap();

        delay.delay_ms(100);
    }
}
