#![no_std]
#![no_main]
// Panics are OK here as is using these libraries the way they want to be used
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::wildcard_imports)]

use hal::timer::{Alarm, Alarm0, Alarm2};
use pimoroni_tiny2040 as bsp;

use bsp::entry;
use bsp::hal;
use hal::pac::interrupt;

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::{digital::v2::InputPin, prelude::*};
use once_cell::unsync::Lazy;
use panic_halt as _;
use util::ControllerPeripherals;

mod util;

use controller_lib::{dsp, Degrees, FanCurve};

const HEARTBEAT_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(100);
const STATUS_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(1);

static mut CURRENT_TEMP: Degrees = Degrees::from_int(65);
static mut TEMP: Lazy<dsp::MovingAverage<Degrees>> = Lazy::new(dsp::MovingAverage::new);
static mut CURRENT_DUTY: u16 = 0;

static mut PERIPHERALS: Option<ControllerPeripherals> = None;
static mut ALARM0: Option<Alarm0> = None;
static mut ALARM2: Option<Alarm2> = None;

#[entry]
fn main() -> ! {
    let mut peripherals = util::ControllerPeripherals::take().unwrap();

    // setup the fan controller to run on a timer
    let mut control_loop_alarm = peripherals.timer.alarm_0().unwrap();
    let mut status_timer = peripherals.timer.alarm_2().unwrap();

    unsafe {
        PERIPHERALS = Some(peripherals);
    }

    control_loop_alarm.schedule(HEARTBEAT_PERIOD).unwrap();
    control_loop_alarm.enable_interrupt();

    unsafe {
        ALARM0 = Some(control_loop_alarm);
    }

    status_timer.schedule(STATUS_PERIOD).unwrap();
    status_timer.enable_interrupt();
    unsafe {
        ALARM2 = Some(status_timer);
    }

    unsafe {
        hal::pac::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_0);
        hal::pac::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_2);
    }

    // It should be safe enough to hold a ref to the watchdog to avoid going into a critical section every wake
    let watchdog = cortex_m::interrupt::free(|_cs| unsafe {
        let peripherals = PERIPHERALS.as_mut().unwrap_unchecked();
        &mut peripherals.watchdog
    });

    loop {
        watchdog.feed();

        // event loop
        cortex_m::asm::wfi();
    }
}

// Alarm 0 timer, used for fan control stuff
#[allow(non_snake_case)]
#[interrupt]
unsafe fn TIMER_IRQ_0() {
    static mut CONTROLLER: Lazy<FanCurve<u16>> = Lazy::new(|| {
        FanCurve::new(
            u16::try_from(util::PWM_TICKS).unwrap(),
            u16::try_from((util::PWM_TICKS * 2) / 10).unwrap(),
            Degrees::from_int(48),
            Degrees::from_int(35),
        )
    });

    ALARM0.as_mut().unwrap_unchecked().clear_interrupt();
    ALARM0
        .as_mut()
        .unwrap_unchecked()
        .schedule(HEARTBEAT_PERIOD)
        .unwrap();

    // mutably borrow... safe because no one else can borrow this during our execution
    let peripherals = PERIPHERALS.as_mut().unwrap_unchecked();

    // heartbeat at half our operating frequency
    if peripherals.red.is_high().unwrap() {
        peripherals.red.set_low().unwrap();
    } else {
        peripherals.red.set_high().unwrap();
    }

    // Read ADC, filter, convert to temperature and apply fan curve
    let conversion = peripherals.adc.read(&mut peripherals.thermistor).unwrap();
    CURRENT_TEMP = TEMP.update(conversion);
    CURRENT_DUTY = CONTROLLER.fan_curve(CURRENT_TEMP);

    // Apply output
    peripherals.fan.set_duty(CURRENT_DUTY);
}

// Alarm 1 timer, used only for scheduling events for the USB IRQ right now
#[allow(non_snake_case)]
#[interrupt]
unsafe fn TIMER_IRQ_2() {
    let status_timer = ALARM2.as_mut().unwrap_unchecked();

    status_timer.clear_interrupt();
    status_timer.schedule(STATUS_PERIOD).unwrap();
    crate::util::USB_SEND_STATUS_PENDING.store(true, core::sync::atomic::Ordering::Relaxed);
    hal::pac::NVIC::pend(hal::pac::interrupt::USBCTRL_IRQ);
}
