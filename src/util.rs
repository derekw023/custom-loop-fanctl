use crate::{bsp, hal, CURRENT_DUTY, CURRENT_TEMP};

use cortex_m::delay::Delay;

use core::sync::atomic::{AtomicBool, Ordering};
use embedded_hal::{digital::v2::OutputPin, watchdog::WatchdogEnable, PwmPin};
use fugit::ExtU32;
use hal::{
    adc::AdcPin,
    gpio::{
        bank0::{Gpio18, Gpio19, Gpio20, Gpio26},
        FunctionNull, FunctionSio, Pin, PullDown, SioOutput,
    },
    pac::interrupt,
    pwm::{Channel, FreeRunning, Pwm1, Slice, A},
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
pub const PWM_TICKS: u32 = (REF_CLK_HZ / PWM_TARGET_HZ) / (PWM_DIV * 2);

pub struct ControllerPeripherals {
    pub watchdog: Watchdog,
    pub systick_delay: Delay,
    pub timer: Timer,
    pub adc: Adc,
    pub thermistor: AdcPin<Pin<Gpio26, FunctionNull, PullDown>>,
    pub red: Pin<Gpio18, FunctionSio<SioOutput>, PullDown>,
    pub green: Pin<Gpio19, FunctionSio<SioOutput>, PullDown>,
    pub blue: Pin<Gpio20, FunctionSio<SioOutput>, PullDown>,
    pub fan: Channel<Slice<Pwm1, FreeRunning>, A>,
}

#[allow(clippy::cast_possible_truncation)]
impl ControllerPeripherals {
    pub fn take() -> Option<Self> {
        // Wrap things in this singleton to promote refs to 'static, and add an extra assurance this only occurs once
        let mut peripherals = hal::pac::Peripherals::take()?;
        let core = hal::pac::CorePeripherals::take()?;

        let mut watchdog = hal::Watchdog::new(peripherals.WATCHDOG);

        let clocks = hal::clocks::init_clocks_and_plls(
            bsp::XOSC_CRYSTAL_FREQ,
            peripherals.XOSC,
            peripherals.CLOCKS,
            peripherals.PLL_SYS,
            peripherals.PLL_USB,
            &mut peripherals.RESETS,
            &mut watchdog,
        )
        .ok()?;

        // Watchdog init for hangup prevention, only allow panic up to 1 second
        watchdog.pause_on_debug(true);
        watchdog.start(1.secs());

        let systick_delay =
            cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

        let sio = hal::Sio::new(peripherals.SIO);
        // now that USB is done with it take ownership of these for the BSP
        let board: bsp::Pins = bsp::Pins::new(
            peripherals.IO_BANK0,
            peripherals.PADS_BANK0,
            sio.gpio_bank0,
            &mut peripherals.RESETS,
        );

        // Configure LEDs
        let mut red = board
            .led_red
            .into_push_pull_output_in_state(hal::gpio::PinState::Low);
        let mut green = board
            .led_green
            .into_push_pull_output_in_state(hal::gpio::PinState::Low);
        let mut blue = board
            .led_blue
            .into_push_pull_output_in_state(hal::gpio::PinState::Low);

        // Signal that HAL is passed init
        red.set_high().unwrap();

        // Configure PWMs
        let pwm_slices = hal::pwm::Slices::new(peripherals.PWM, &mut peripherals.RESETS);

        let mut pwm = pwm_slices.pwm1;

        pwm.enable();
        pwm.set_top((PWM_TICKS / PWM_DIV) as u16);
        pwm.set_ph_correct();

        let mut fan = pwm.channel_a;
        fan.set_inverted();
        let mut fan_io = board.gpio2;

        fan_io.set_drive_strength(hal::gpio::OutputDriveStrength::TwelveMilliAmps);
        fan_io.set_slew_rate(hal::gpio::OutputSlewRate::Fast);

        fan.output_to(fan_io);

        fan.enable();

        // Configure analog stuff
        let adc = hal::Adc::new(peripherals.ADC, &mut peripherals.RESETS);
        let thermistor = AdcPin::new(board.gpio26);

        let timer = hal::Timer::new(peripherals.TIMER, &mut peripherals.RESETS, &clocks);

        // Signal peripheral init passed, short of USB
        green.set_high().unwrap();

        // Individually capture these for USB here
        let ctrl_reg = peripherals.USBCTRL_REGS;
        let ctrl_dpram = peripherals.USBCTRL_DPRAM;
        let usb_clock = clocks.usb_clock;

        // Take ownership of RESETS from peripherals explicitly to hand it to the USB singleton
        let mut usb_resets = peripherals.RESETS;

        // Initialize USB bus
        let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
            ctrl_reg,
            ctrl_dpram,
            usb_clock,
            true,
            &mut usb_resets,
        ));

        let bus_ref = unsafe {
            USB_BUS.replace(usb_bus);
            USB_BUS.as_ref().unwrap()
        };

        let serial = SerialPort::new(bus_ref);
        let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("DEXCorp")
            .product("DEXFANS")
            .serial_number("TEST")
            .device_class(2) //   from: https://www.usb.org/defined-class-codes
            .max_packet_size_0(64)
            .build();
        unsafe {
            USB_SERIAL.replace(serial);
            USB_DEVICE.replace(usb_dev);
            hal::pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        }

        // Everything else is finished
        blue.set_high().unwrap();

        // Globally enable interrupts at the end
        unsafe {
            cortex_m::interrupt::enable();
        }

        Some(Self {
            watchdog,
            systick_delay,
            timer,
            adc,
            thermistor,
            red,
            green,
            blue,
            fan,
        })
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
        writeln!(
            report_buf,
            "T: {CURRENT_TEMP}°C, D: {CURRENT_DUTY}/{PWM_TICKS}",
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
