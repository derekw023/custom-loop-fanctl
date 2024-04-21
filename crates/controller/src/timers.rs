/// Generally, timers and timed events in the system
/// Right now just a timer each for poking USB (may eventually move this) and a timer for driving the fan curve
/// If the USB stuff is exfiltrated into ta USB crate (it should be) then this will just become the fan controller mod
use crate::{
    util::{self},
    HEARTBEAT_PERIOD, PERIPHERALS, STATUS_PERIOD,
};
use controller_lib::{Degrees, FanCurve};

use pimoroni_tiny2040 as bsp;

use bsp::hal;
use hal::pac::interrupt;
use hal::timer::{Alarm, Alarm0, Alarm2};

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::{digital::v2::InputPin, prelude::*};
use once_cell::unsync::Lazy;

pub(crate) static mut CURRENT_TEMP: Degrees = Degrees::from_int(65);
static mut TEMP: Lazy<crate::dsp::MovingAverage<Degrees>> =
    Lazy::new(crate::dsp::MovingAverage::new);
pub(crate) static mut CURRENT_DUTY: u16 = 0;

static mut ALARM0: Option<Alarm0> = None;
static mut ALARM2: Option<Alarm2> = None;

pub(crate) fn setup() {
    // Safe enough, given interrupts aren't enabled until later
    let peripherals = unsafe { PERIPHERALS.as_mut().unwrap_unchecked() };

    // setup the fan controller to run on a timer
    let mut control_loop_alarm = peripherals.timer.alarm_0().unwrap();
    let mut status_timer = peripherals.timer.alarm_2().unwrap();

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
    // TODO figure out a better way to do this
    let peripherals = PERIPHERALS.as_mut().unwrap_unchecked();

    // // heartbeat at half our operating frequency
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
