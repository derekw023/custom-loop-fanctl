//! Ostensibly, BSP initialization code
//!
//! Realistically... too much. controller init code here should be in the controller, usb should be in usb, what's left (if anything) should remain here
use cortex_m::delay::Delay;

use bsp::hal;
use pimoroni_tiny2040 as bsp;

use embedded_hal::digital::OutputPin;
use fugit::ExtU32;
use hal::{
    gpio::{
        bank0::{Gpio18, Gpio19, Gpio20, Gpio26},
        FunctionSio, Pin, PullDown, PullNone, SioInput, SioOutput,
    },
    pac,
    pwm::{Channel, FreeRunning, Pwm2, Slice, A},
    Clock, Timer, Watchdog,
};

// this must be configured for 25khz
// the input clock here is the system clock which is a big 125Mhz PLL
const REF_CLK_HZ: u32 = 125_000_000;
const PWM_TARGET_HZ: u32 = 25_000;
const PWM_DIV: u32 = 1;
pub const PWM_TICKS: u32 = (REF_CLK_HZ / PWM_TARGET_HZ) / (PWM_DIV);

pub(crate) type ControllerStatusPin = Pin<Gpio18, FunctionSio<SioOutput>, PullDown>;
pub(crate) type ThermistorPin = Pin<Gpio26, FunctionSio<SioInput>, PullNone>;
pub(crate) type FanPin = Channel<Slice<Pwm2, FreeRunning>, A>;

/// Global state struct, and central control point for peripheral/hardware access
pub struct ControllerPeripherals {
    pub resets: hal::pac::RESETS,
    /// Feed this hungry boi every second or else
    pub watchdog: Watchdog,
    pub systick_delay: Delay,
    /// Can be used to take ownership of timers and alarms
    pub timer: Timer,
    /// Hold the ADC, to allow it to be taken
    pub adc: Option<hal::pac::ADC>,
    pub thermistor_pin: Option<ThermistorPin>,
    pub red: Option<ControllerStatusPin>,
    pub green: Pin<Gpio19, FunctionSio<SioOutput>, PullDown>,
    pub blue: Pin<Gpio20, FunctionSio<SioOutput>, PullDown>,
    pub fan: Option<FanPin>,
    pub(crate) usb_peripherals:
        Option<(pac::USBCTRL_DPRAM, pac::USBCTRL_REGS, hal::clocks::UsbClock)>,
}

#[allow(clippy::cast_possible_truncation)]
impl ControllerPeripherals {
    /// Make the instance of this singleton, return None if somehow called twice
    pub fn take(start_watchdog: bool) -> Option<Self> {
        let mut pac_peripherals = hal::pac::Peripherals::take()?;
        let core = hal::pac::CorePeripherals::take()?;

        let mut watchdog = hal::Watchdog::new(pac_peripherals.WATCHDOG);

        let clocks = hal::clocks::init_clocks_and_plls(
            bsp::XOSC_CRYSTAL_FREQ,
            pac_peripherals.XOSC,
            pac_peripherals.CLOCKS,
            pac_peripherals.PLL_SYS,
            pac_peripherals.PLL_USB,
            &mut pac_peripherals.RESETS,
            &mut watchdog,
        )
        .ok()?;

        // Watchdog init for hangup prevention, only allow panic up to 1 second
        watchdog.pause_on_debug(true);
        if start_watchdog {
            watchdog.start(1.secs());
        }

        let systick_delay =
            cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

        let sio = hal::Sio::new(pac_peripherals.SIO);
        // now that USB is done with it take ownership of these for the BSP
        let board: bsp::Pins = bsp::Pins::new(
            pac_peripherals.IO_BANK0,
            pac_peripherals.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac_peripherals.RESETS,
        );

        // Configure PWMs
        let pwm_slices = hal::pwm::Slices::new(pac_peripherals.PWM, &mut pac_peripherals.RESETS);

        let mut pwm2 = pwm_slices.pwm2;

        pwm2.enable();
        pwm2.set_top((PWM_TICKS / PWM_DIV) as u16);
        // pwm.set_ph_correct();

        let mut fan = pwm2.channel_a;
        // fan.set_inverted();
        let mut fan_io = board.gpio4;

        fan_io.set_drive_strength(hal::gpio::OutputDriveStrength::TwelveMilliAmps);
        fan_io.set_slew_rate(hal::gpio::OutputSlewRate::Fast);

        fan.output_to(fan_io);

        let timer = hal::Timer::new(pac_peripherals.TIMER, &mut pac_peripherals.RESETS, &clocks);

        let mut ret = Self {
            watchdog,
            systick_delay,
            timer,
            adc: Some(pac_peripherals.ADC),
            thermistor_pin: Some(board.gpio26.into_floating_input()),
            red: Some(
                board
                    .led_red
                    .into_push_pull_output_in_state(hal::gpio::PinState::Low),
            ),
            green: board
                .led_green
                .into_push_pull_output_in_state(hal::gpio::PinState::Low),
            blue: board
                .led_blue
                .into_push_pull_output_in_state(hal::gpio::PinState::Low),
            fan: Some(fan),
            resets: pac_peripherals.RESETS,
            usb_peripherals: Some((
                pac_peripherals.USBCTRL_DPRAM,
                pac_peripherals.USBCTRL_REGS,
                clocks.usb_clock,
            )),
        };

        ret.init();

        Some(ret)
    }

    /// Initializer function, called from `take()` once
    fn init(&mut self) {
        self.green.set_high().unwrap();
        self.blue.set_high().unwrap();

        // Globally enable interrupts at the very end before returning to main
        unsafe {
            cortex_m::interrupt::enable();
        }
    }
}
