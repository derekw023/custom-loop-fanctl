//! Generally, timers and timed events in the system
//! Right now just a timer each for poking USB (may eventually move this) and a timer for driving the fan curve
//! If the USB stuff is exfiltrated into ta USB crate (it should be) then this will just become the fan controller mod
use core::borrow::BorrowMut;

use crate::{
    adc,
    bsp::hal::{
        adc::{AdcFifo, DmaReadTarget},
        dma::{single_buffer::Config, single_buffer::Transfer, Channel, SingleChannel, CH0},
        pac::interrupt,
    },
    dma,
    dsp::MovingAverage,
    util::{ControllerStatusPin, FanPin},
};
use controller_lib::{Degrees, FanCurve};

use cortex_m::interrupt::{CriticalSection, Mutex};
use embedded_hal::{
    digital::{OutputPin, StatefulOutputPin},
    pwm::SetDutyCycle,
};

// Singletons
type DmaBuf = [u16; 32];
static mut ACTIVE_LOOP: Option<ControlLoop> = None;
static mut DMA_BUFFER: DmaBuf = [0; 32];

struct ControlLoop {
    // Data for fan controller. optional to allow for not connecting any output, to only sense temperature
    curve: Option<FanCurve<u16>>,
    current_duty: u16,
    // temperature: MovingAverage<Degrees>,

    // HW resources
    status_led: ControllerStatusPin,

    // Loop is scheduled by DMA interrupts and also ADC interval
    // that connection is ideally configured in this module
    // loop_timer: Alarm0,
    fan: Option<FanPin>,

    /// DMA transfer things, access buffer through static
    transfer: Transfer<Channel<CH0>, DmaReadTarget<u16>, &'static mut [u16; 32]>,

    // Circular buffer fields that are modified from interrupt context
    buffer_idx: Mutex<usize>,
    data_valid: Mutex<bool>,
}

pub(crate) struct Token {
    handle: &'static ControlLoop,
}
// Alarm 0 timer, used for fan control stuff
#[allow(non_snake_case)]
#[interrupt]
unsafe fn DMA_IRQ_0() {
    if let Some(current_loop) = ACTIVE_LOOP.as_mut() {
        let cs = unsafe { CriticalSection::new() };

        // Do the processing in a safe function
        current_loop.update(&cs);
    }
}

impl Token {
    pub(crate) fn new<'a>(
        adc: &mut adc::Token,
        dma: &mut dma::Token,
        status_led: crate::util::ControllerStatusPin,
    ) -> Option<Self> {
        unsafe {
            if ACTIVE_LOOP.is_some() {
                return None;
            }
        }

        // Configure DMA against the static reference
        let mut chan = dma.take_ch0()?;
        chan.enable_irq0();

        // DMA transfer
        let cfg: Config<Channel<CH0>, DmaReadTarget<u16>, &mut [u16; 32]> =
            Config::new(chan, adc.adc_fifo.dma_read_target(), unsafe {
                &mut DMA_BUFFER
            });
        let trans = cfg.start();

        let handle = unsafe {
            ACTIVE_LOOP = Some(ControlLoop {
                current_duty: 0,
                fan: None,
                // fan: controller.fan.take().unwrap(),
                // temperature: MovingAverage::new(),
                // curve: FanCurve::new(
                //     u16::try_from(util::PWM_TICKS).unwrap(),
                //     u16::try_from((util::PWM_TICKS * 2) / 10).unwrap(),
                //     Degrees::from_int(48),
                //     Degrees::from_int(35),
                // ),
                curve: None,
                status_led,
                transfer: trans,
                buffer_idx: Mutex::new(0),
                data_valid: Mutex::new(false),
            });
            ACTIVE_LOOP.as_mut().unwrap_unchecked()
        };

        adc.adc_fifo.resume();

        Some(Self { handle })
    }

    pub fn current_temp(&self) -> Degrees {
        // Critical Section for consistency
        let sum: i32 = cortex_m::interrupt::free(|cs| unsafe {
            DMA_BUFFER.iter().fold(0i32, |mut acc, i| {
                acc += i32::try_from(*i).unwrap();
                acc
            })
        });

        // Don't actually need to glance buffer idx
        let average = sum / 32;

        Degrees::from_int(average.try_into().unwrap())
    }
}

impl ControlLoop {
    /// Loop update function, called on every new sample
    ///
    /// ADC conversion and data accumulation is done entirely in hw
    /// Needs a critical section to lock asynchronously updated fields
    fn update(&mut self, cs: &CriticalSection) {
        // // heartbeat at half the real operating frequency
        if self.status_led.is_set_high().unwrap() {
            self.status_led.set_low().unwrap();
        } else {
            self.status_led.set_high().unwrap();
        }

        // Maintain circular buffer ptr
        let buffer_idx: &mut usize = &mut self.buffer_idx.get_mut(cs);
        *buffer_idx = *buffer_idx + 1;
        if *buffer_idx >= 32 {
            let valid = &mut self.data_valid.get_mut(cs);
            **valid = true;
            *buffer_idx = 0;
        }

        // Set outputs if configured
        // if let Some(ref curve) = self.curve {
        //     // self.current_duty = curve.fan_curve(current_temp);

        //     if let Some(ref mut fan) = self.fan {
        //         fan.set_duty_cycle(self.current_duty).unwrap()
        //     }
        // }

        // TODO if usb needs a shared data buffer updated or something
    }
}
