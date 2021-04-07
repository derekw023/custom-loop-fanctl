#![no_std]
#![no_main]

extern crate itsybitsy_m0 as hal;

extern crate usb_device;
extern crate usbd_serial;

use hal::clock::GenericClockController;
use hal::entry;
use hal::pac::{interrupt, CorePeripherals, Peripherals};
// use hal::prelude::*;

use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use cortex_m::asm;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();

    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    let mut pins = hal::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    let bus_allocator = hal::usb_allocator(
        peripherals.USB,
        &mut clocks,
        &mut peripherals.PM,
        pins.usb_dm,
        pins.usb_dp,
        &mut pins.port,
    );

    let mut serial = SerialPort::new(&bus_allocator);
    let mut bus = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(USB_CLASS_CDC)
        .build();

    loop {
        if !bus.poll(&mut [&mut serial]) {
            continue;
        }
        let mut buf = [0u8; 64];
        let count;

        match serial.read(&mut buf[..]) {
            Ok(c) => {
                // count bytes were read to &buf[..count]
                count = c;
            }
            Err(UsbError::WouldBlock) => continue, // No data received
            Err(_) => continue,                    // An error occurred
        };

        red_led.toggle();
        serial.write(&buf[..count]).unwrap();
    }
}
