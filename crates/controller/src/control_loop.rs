//! Generally, timers and timed events in the system
//! Right now just a timer each for poking USB (may eventually move this) and a timer for driving the fan curve
//! If the USB stuff is exfiltrated into ta USB crate (it should be) then this will just become the fan controller mod
use crate::{
    adc,
    dsp::MovingAverage,
    util::{ControllerPeripherals, ControllerStatusPin, FanPin, ThermistorPin},
    HEARTBEAT_PERIOD,
};
use controller_lib::{Degrees, FanCurve};

use pimoroni_tiny2040 as bsp;

use bsp::hal;
use hal::pac::interrupt;
use hal::{adc::AdcPin, timer::Alarm};

use embedded_hal::{
    digital::{OutputPin, StatefulOutputPin},
    pwm::SetDutyCycle,
};

// Singleton pointer for IRQ
static ACTIVE_LOOP: Option<ControlLoop> = None;

struct ControlLoop {
    // Data for fan controller
    curve: Option<FanCurve<u16>>,
    current_duty: u16,
    temperature: MovingAverage<Degrees>,

    // HW resources
    status_led: ControllerStatusPin,
    thermistor_pin: AdcPin<ThermistorPin>,

    // Loop is scheduled by DMA interrupts and also ADC interval
    // that connection is ideally configured in this module
    // loop_timer: Alarm0,
    fan: Option<FanPin>,
}

pub(crate) struct Token {
    handle: &'static ControlLoop,
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

impl Token {
    pub(crate) fn new<'a>(
        controller: &mut ControllerPeripherals,
        adc: &adc::Token,
    ) -> Option<Self> {
        if ACTIVE_LOOP.is_some() {
            return None;
        }

        // setup the fan controller to run on a timer
        let mut control_loop_alarm = controller.timer.alarm_0().unwrap();
        control_loop_alarm.schedule(HEARTBEAT_PERIOD).unwrap();
        control_loop_alarm.enable_interrupt();

        // Can't schedule this until it won't panic
        // unsafe {
        //     hal::pac::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_0);
        // }

        let thermistor = AdcPin::new(controller.thermistor_pin.take().unwrap()).unwrap();

        let controller = ControlLoop {
            current_duty: 0,
            fan: None,
            // fan: controller.fan.take().unwrap(),
            temperature: MovingAverage::new(),
            // curve: FanCurve::new(
            //     u16::try_from(util::PWM_TICKS).unwrap(),
            //     u16::try_from((util::PWM_TICKS * 2) / 10).unwrap(),
            //     Degrees::from_int(48),
            //     Degrees::from_int(35),
            // ),
            curve: None,
            status_led: controller.red.take().unwrap(),
            thermistor_pin: thermistor,
        };

        let handle = unsafe {
            ACTIVE_LOOP = Some(controller);
            ACTIVE_LOOP.as_ref().unwrap_unchecked()
        };

        Some(Self { handle })
    }
}
impl ControlLoop {
    /// Loop update function, called on every new sample
    ///
    /// ADC conversion and data accumulation is done entirely in hw
    /// This could be called on dMA interrupt but could also be run asynchronously on the main thread after the sample lands
    fn update(&mut self) {
        // // heartbeat at half the real operating frequency
        if self.status_led.is_set_high().unwrap() {
            self.status_led.set_low().unwrap();
        } else {
            self.status_led.set_high().unwrap();
        }

        let conversion = 0u16;

        // Temperature conversion from ADC counts by From<u16> and also circular buffer based moving average advance
        // TODO: perform moving average on u16 samples instead of Degrees
        // TODO: perform moving average calculation on demand instead of on sample... far fewer operations
        let current_temp = self.temperature.update(conversion.into());

        // Set outputs if configured
        if let Some(ref curve) = self.curve {
            self.current_duty = curve.fan_curve(current_temp);

            if let Some(ref fan) = self.fan {
                fan.set_duty_cycle(self.current_duty).unwrap()
            }
        }

        // TODO if udb needs a shared data buffer updated or something
    }
}
