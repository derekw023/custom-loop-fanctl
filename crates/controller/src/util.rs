//! Ostensibly, BSP initialization code
//!
//! Realistically... too much. controller init code here should be in the controller, usb should be in usb, what's left (if anything) should remain here
// use crate::{timers::CURRENT_DUTY, timers::CURRENT_TEMP};

use cortex_m::delay::Delay;

use bsp::hal;
use pimoroni_tiny2040 as bsp;

use core::sync::atomic::{AtomicBool, Ordering};
// use embedded_hal::{digital::v2::OutputPin, watchdog::WatchdogEnable, PwmPin};
use embedded_hal::digital::OutputPin;
use fugit::ExtU32;
use hal::{
    gpio::{
        bank0::{Gpio18, Gpio19, Gpio20, Gpio26},
        FunctionSio, Pin, PullDown, PullNone, SioInput, SioOutput,
    },
    pac::interrupt,
    pwm::{Channel, FreeRunning, Pwm2, Slice, A},
    usb::UsbBus,
    Adc, Clock, Timer, Watchdog,
};

// USB Device support
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

// USB Singletons
static mut USB_DEVICE: Option<UsbDevice<UsbBus>> = None;
static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;
use core::fmt::Write;

pub static USB_SEND_STATUS_PENDING: AtomicBool = AtomicBool::new(false);

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
    resets: hal::pac::RESETS,
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
}

#[allow(clippy::cast_possible_truncation)]
impl ControllerPeripherals {
    /// Make the instance of this singleton, return None if somehow called twice
    pub fn take() -> Option<Self> {
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
        watchdog.start(1.secs());

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

        let mut pwm = pwm_slices.pwm2;

        pwm.enable();
        pwm.set_top((PWM_TICKS / PWM_DIV) as u16);
        // pwm.set_ph_correct();

        let mut fan = pwm.channel_a;
        // fan.set_inverted();
        let mut fan_io = board.gpio4;

        fan_io.set_drive_strength(hal::gpio::OutputDriveStrength::TwelveMilliAmps);
        fan_io.set_slew_rate(hal::gpio::OutputSlewRate::Fast);

        fan.output_to(fan_io);

        let timer = hal::Timer::new(pac_peripherals.TIMER, &mut pac_peripherals.RESETS, &clocks);

        // Individually capture these for USB here
        let ctrl_reg = pac_peripherals.USBCTRL_REGS;
        let ctrl_dpram = pac_peripherals.USBCTRL_DPRAM;
        let usb_clock = clocks.usb_clock;

        // Take ownership of RESETS from peripherals explicitly to hand it to the USB singleton
        let usb_resets = &mut pac_peripherals.RESETS;

        // Initialize USB bus
        let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
            ctrl_reg, ctrl_dpram, usb_clock, true, usb_resets,
        ));

        let bus_ref = unsafe {
            USB_BUS.replace(usb_bus);
            USB_BUS.as_ref().unwrap()
        };

        let serial = SerialPort::new(bus_ref);
        let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x16c0, 0x27dd))
            // .manufacturer("DEXCorp")
            // .product("DEXFANS")
            // .serial_number("TEST")
            // .device_class(2) //   from: https://www.usb.org/defined-class-codes\
            .build();
        unsafe {
            USB_SERIAL.replace(serial);
            USB_DEVICE.replace(usb_dev);
            hal::pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        }

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
        };

        ret.init();

        Some(ret)
    }

    /// Initializer function, called from `take()` once
    fn init(&mut self) {
        // self.red.as_ref_mut().unwrap().set_high().unwrap();
        self.green.set_high().unwrap();
        self.blue.set_high().unwrap();

        // Globally enable interrupts at the very end before returning to main
        unsafe {
            cortex_m::interrupt::enable();
        }
    }

    /// Consume ADC and provide constructed HAL structure
    pub fn take_adc(&mut self) -> Option<Adc> {
        let converter = self.adc.take()?;
        Some(Adc::new(converter, &mut self.resets))
    }

    /// Deinitialize ADC and restore the resource to this
    pub fn put_adc(&mut self, adc: Adc) {
        let converter = adc.free();
        self.adc.replace(converter);
    }
}

#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    let usb_dev = USB_DEVICE.as_mut().unwrap();
    let serial = USB_SERIAL.as_mut().unwrap();

    if usb_dev.poll(&mut [serial]) {
        let mut buf = [0u8; 64];
        match serial.read(&mut buf) {
            Err(_e) => {
                // Do nothing
            }
            Ok(0) => {
                // Do nothing
            }
            Ok(count) => {
                // scan for key command letters
                buf.into_iter().take(count).for_each(|b| {
                    if b == b't' {
                        USB_SEND_STATUS_PENDING.store(true, Ordering::Relaxed);
                    }
                    if b == b'u' {
                        // reset into BL mode
                        hal::rom_data::reset_to_usb_boot(0, 0);
                    }
                });
            }
        }
    }
    let mut report_buf: heapless::String<64> = heapless::String::new();
    let pending = USB_SEND_STATUS_PENDING.load(Ordering::Relaxed);
    if pending {
        USB_SEND_STATUS_PENDING.store(false, Ordering::SeqCst);

        // if command was received, overwrite buffer contents with a response instead
        let duty_percentish = 0;
        // let duty_percentish = (u32::from(CURRENT_DUTY) * 10000) / PWM_TICKS;
        let duty_pct = duty_percentish / 100;
        let duty_decimals = duty_percentish % 100;
        writeln!(
            report_buf,
            "T: 0°C, D: {duty_pct:3}.{duty_decimals:02}%" // "T: {CURRENT_TEMP:02}°C, D: {duty_pct:3}.{duty_decimals:02}%"
        )
        .unwrap();

        let mut wr_ptr = report_buf.as_bytes();
        while !wr_ptr.is_empty() {
            match serial.write(wr_ptr) {
                Ok(len) => wr_ptr = &wr_ptr[len..],
                // On error, just drop unwritten data.
                // One possible error is Err(WouldBlock), meaning the USB
                // write buffer is full.
                Err(_) => break,
            };
        }
    };
}
