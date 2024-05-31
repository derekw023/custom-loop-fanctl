//! Generally, timers and timed events in the system
//! Right now just a timer each for poking USB (may eventually move this) and a timer for driving the fan curve
//! If the USB stuff is exfiltrated into ta USB crate (it should be) then this will just become the fan controller mod

use crate::{
    adc,
    bsp::hal::{
        adc::DmaReadTarget,
        dma::{single_buffer::Config, single_buffer::Transfer, Channel, SingleChannel, CH0},
        pac::{interrupt, Interrupt, NVIC},
    },
    dma,
    util::ControllerStatusPin,
};
use controller_lib::Degrees;

use cortex_m::interrupt::CriticalSection;
use embedded_hal::digital::{OutputPin, StatefulOutputPin};

// Singletons
type DmaBuf = [u16; 32];
static mut ACTIVE_LOOP: Option<ControlLoop> = None;
static mut DMA_BUFFER: DmaBuf = [0; 32];
static mut BUFFER_VALID: bool = false;

static mut STATUS_LED: Option<ControllerStatusPin> = None;

struct ControlLoop {
    /// DMA transfer things, access buffer through static
    transfer: Option<Transfer<Channel<CH0>, DmaReadTarget<u16>, &'static mut [u16; 32]>>,
    // Circular buffer fields that are modified from interrupt context through global statics
    // Unfortunately at this time this struct is singleton
}

pub(crate) struct Token {
    _handle: &'static ControlLoop,
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

impl Drop for Token {
    fn drop(&mut self) {
        unsafe {
            ACTIVE_LOOP = None;
            BUFFER_VALID = false;
        }
    }
}

impl Token {
    /// Start sampling with the given ADC FIFO and DMA channel to singleton buffer
    /// You can use the returned token to query the resultant data
    #[must_use]
    pub(crate) fn new(
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

        // The actual loop is stored in a singleton, but the caller can have a reference to it.
        // See `Drop` impl for the RAII-ness of it all
        let handle = unsafe {
            STATUS_LED = Some(status_led);
            ACTIVE_LOOP = Some(ControlLoop {
                transfer: Some(trans),
            });
            ACTIVE_LOOP.as_mut().unwrap_unchecked()
        };

        adc.adc_fifo.resume();

        // Finally, unmask relevant interrupt for our handler
        unsafe {
            NVIC::unmask(Interrupt::DMA_IRQ_0);
        }

        Some(Self { _handle: handle })
    }

    pub fn current_temp(&self) -> Degrees {
        static mut LEN: usize = 0;
        // Critical Section for consistency
        let sum: Option<i32> = cortex_m::interrupt::free(|_cs| unsafe {
            if BUFFER_VALID {
                LEN = DMA_BUFFER.len();
                Some(DMA_BUFFER.iter().fold(0i32, |mut acc, i| {
                    acc += *i as i32;
                    acc
                }))
            } else {
                None
            }
        });

        // If there aren't enough samples in the buffer then failsafe to 50 degrees
        if let Some(sum) = sum {
            let average: i64 = (sum as i64) / unsafe { LEN as i64 };
            Degrees::from(average)
        } else {
            Degrees::from_int(50)
        }
    }
}

impl ControlLoop {
    /// Loop update function, called on every DMA transfer completion, every 32 samples
    ///
    /// ADC conversion is done entirely in hw
    /// Needs a critical section to lock asynchronously updated fields
    fn update(&mut self, _cs: &CriticalSection) {
        unsafe {
            BUFFER_VALID = true;
            if let Some(ref mut status) = STATUS_LED {
                // heartbeat at half the real operating frequency
                if status.is_set_high().unwrap() {
                    status.set_low().unwrap();
                } else {
                    status.set_high().unwrap();
                }
            }
        }

        // Requeue transfer
        if let Some(trans) = self.transfer.take() {
            let (ch, rd, wr) = trans.wait();
            self.transfer = Some(Config::new(ch, rd, wr).start());
        }

        // TODO if usb needs a shared data buffer updated or somethi
    }
}
