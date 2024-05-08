use crate::util::ControllerPeripherals;
use crate::STATUS_PERIOD;

use bsp::hal;
use core::{
    fmt::Write,
    sync::atomic::{AtomicBool, Ordering},
};
use hal::pac::interrupt;
use hal::timer::{Alarm, Alarm2};
use hal::usb::UsbBus;
use pimoroni_tiny2040 as bsp;

// USB Device support
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

static mut ALARM2: Option<Alarm2> = None;

// USB Singletons
static mut USB_DEVICE: Option<UsbDevice<UsbBus>> = None;
static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;

// Poll every 10ms
const USB_PERIOD: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::Hz(100);

pub static USB_SEND_STATUS_PENDING: AtomicBool = AtomicBool::new(false);

pub(crate) fn setup(controller: &mut ControllerPeripherals) {
    let (dpram, regs, usb_clock) = controller.usb_peripherals.take().unwrap();
    // Initialize USB bus
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        regs,
        dpram,
        usb_clock,
        true,
        &mut controller.resets,
    ));

    let bus_ref = unsafe {
        USB_BUS.replace(usb_bus);
        USB_BUS.as_ref().unwrap()
    };

    let serial = SerialPort::new(bus_ref);
    let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("DEXCORP")
            .product("Dex Fan Controller")
            .serial_number("FOO")])
        .unwrap()
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    // Timer init, to schedule polls
    let mut status_timer = controller.timer.alarm_2().unwrap();
    status_timer.schedule(USB_PERIOD).unwrap();
    status_timer.enable_interrupt();

    unsafe {
        ALARM2 = Some(status_timer);
        USB_SERIAL.replace(serial);
        USB_DEVICE.replace(usb_dev);
    }

    unsafe {
        // hal::pac::NVIC::unmask(hal::pac::interrupt::TIMER_IRQ_2);
        hal::pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
    }
}

// Alarm 1 timer, used only for scheduling events for the USB IRQ right now
#[allow(non_snake_case)]
#[interrupt]
unsafe fn TIMER_IRQ_2() {
    let status_timer = ALARM2.as_mut().unwrap_unchecked();

    status_timer.clear_interrupt();
    status_timer.schedule(USB_PERIOD).unwrap();
    USB_SEND_STATUS_PENDING.store(true, core::sync::atomic::Ordering::Relaxed);
    hal::pac::NVIC::pend(hal::pac::interrupt::USBCTRL_IRQ);
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
