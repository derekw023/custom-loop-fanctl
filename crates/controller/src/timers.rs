//! Generally, timers and timed events in the system
//! Right now just a timer each for poking USB (may eventually move this) and a timer for driving the fan curve
//! If the USB stuff is exfiltrated into ta USB crate (it should be) then this will just become the fan controller mod
use crate::{
    dsp::MovingAverage,
    util::{self, ControllerPeripherals, ControllerStatusPin, FanPin, ThermistorPin},
    HEARTBEAT_PERIOD, STATUS_PERIOD,
};
use controller_lib::{Degrees, FanCurve};

use pimoroni_tiny2040 as bsp;

use bsp::hal;
use hal::{
    adc::AdcPin,
    timer::{Alarm, Alarm0, Alarm2},
};
use hal::{pac::interrupt, Adc};

use embedded_hal::{
    digital::{OutputPin, StatefulOutputPin},
    pwm::SetDutyCycle,
};

static mut ACTIVE_LOOP: Option<ControlLoop> = None;
static mut ALARM2: Option<Alarm2> = None;

struct ControlLoop {
    // Data for fan controller
    curve: FanCurve<u16>,
    current_duty: u16,
    temperature: MovingAverage<Degrees>,

    // HW resources
    status_led: ControllerStatusPin,
    thermistor_pin: AdcPin<ThermistorPin>,
    adc: Adc,
    loop_timer: Alarm0,
    fan: FanPin,
}

pub(crate) fn setup(controller: &mut ControllerPeripherals) {
    // setup the fan controller to run on a timer
    let mut control_loop_alarm = controller.timer.alarm_0().unwrap();
    control_loop_alarm.schedule(HEARTBEAT_PERIOD).unwrap();
    control_loop_alarm.enable_interrupt();

    // USB timer (TODO move out)
    let mut status_timer = controller.timer.alarm_2().unwrap();
    status_timer.schedule(STATUS_PERIOD).unwrap();
    status_timer.enable_interrupt();

    unsafe {
        ALARM2 = Some(status_timer);
    }

    unsafe {
        hal::pac::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_0);
        hal::pac::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_2);
    }

    let adc = controller.take_adc().unwrap();

    let thermistor = AdcPin::new(controller.thermistor_pin.take().unwrap()).unwrap();

    let controller = ControlLoop {
        adc,
        loop_timer: control_loop_alarm,
        current_duty: 0,
        fan: controller.fan.take().unwrap(),
        temperature: MovingAverage::new(),
        curve: FanCurve::new(
            u16::try_from(util::PWM_TICKS).unwrap(),
            u16::try_from((util::PWM_TICKS * 2) / 10).unwrap(),
            Degrees::from_int(48),
            Degrees::from_int(35),
        ),
        status_led: controller.red.take().unwrap(),
        thermistor_pin: thermistor,
    };

    unsafe {
        ACTIVE_LOOP.replace(controller);
    }
}

impl ControlLoop {
    /// Loop update function, called from interrupt context
    fn update(&mut self) {
        self.loop_timer.clear_interrupt();
        self.loop_timer.schedule(HEARTBEAT_PERIOD).unwrap();

        // // heartbeat at half our operating frequency
        if self.status_led.is_set_high().unwrap() {
            self.status_led.set_low().unwrap();
        } else {
            self.status_led.set_high().unwrap();
        }

        // Read ADC, filter, convert to temperature and apply fan curve
        // TODO: this won't work
        // let conversion = self.adc.read_single();
        let conversion: u16 = unimplemented!("conversion");

        // CURRENT_TEMP = TEMP.update(conversion);
        let current_temp = self.temperature.update(conversion.into());
        self.current_duty = self.curve.fan_curve(current_temp);

        // Apply output
        self.fan.set_duty_cycle(self.current_duty).unwrap();
    }
}

// Alarm 0 timer, used for fan control stuff
#[allow(non_snake_case)]
#[interrupt]
unsafe fn TIMER_IRQ_0() {
    if let Some(current_loop) = ACTIVE_LOOP.as_mut() {
        // Do the processing in a safe function
        current_loop.update();
    }
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
