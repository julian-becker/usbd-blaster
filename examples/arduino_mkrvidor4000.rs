#![no_std]
#![no_main]

extern crate arduino_mkrvidor4000 as hal;

use hal::clock::GenericClockController;
use hal::entry;
use hal::gpio::{Floating, Input, IntoFunction, Output, Pa12, Pa13, Pa14, Pa15, PushPull};
use hal::pac::{interrupt, CorePeripherals, Peripherals, NVIC};
use hal::usb::usb_device::{bus::UsbBusAllocator, prelude::*};
use hal::usb::UsbBus;

use usbd_blaster::{Blaster, ALTERA_BLASTER_USB_VID_PID};

// #[link_section = "FLASH_FPGA"]
// const FLASH_FPGA: [u8; 2 * 1024 * 1024] = [0u8; 2 * 1024 * 1024];

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BLASTER: Option<
    Blaster<
        UsbBus,
        (),
        Pa12<Output<PushPull>>,
        Pa13<Output<PushPull>>,
        Pa14<Output<PushPull>>,
        Pa15<Input<Floating>>,
    >,
> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut LED: Option<hal::gpio::Pb8<hal::gpio::Output<hal::gpio::OpenDrain>>> = None;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    let mut pins = hal::Pins::new(peripherals.PORT);
    // Enable 48MHZ clock output for FPGA
    // https://github.com/arduino/ArduinoCore-samd/blob/master/variants/mkrvidor4000/variant.cpp#L229
    let _gclk: hal::gpio::Pa27<hal::gpio::PfH> = pins.gclk.into_function(&mut pins.port);

    let main_clk = clocks.gclk0();
    let usb_clock = clocks.usb(&main_clk).unwrap();

    let allocator = unsafe {
        USB_ALLOCATOR = UsbBusAllocator::new(UsbBus::new(
            &usb_clock,
            &mut peripherals.PM,
            pins.usb_n.into_function(&mut pins.port),
            pins.usb_p.into_function(&mut pins.port),
            peripherals.USB,
        ))
        .into();
        USB_ALLOCATOR.as_ref().unwrap()
    };
    unsafe {
        LED = pins
            .led_builtin
            .into_open_drain_output(&mut pins.port)
            .into();
        USB_BLASTER = Blaster::new(
            USB_ALLOCATOR.as_ref().unwrap(),
            pins.fpga_tdi.into_push_pull_output(&mut pins.port),
            pins.fpga_tck.into_push_pull_output(&mut pins.port),
            pins.fpga_tms.into_push_pull_output(&mut pins.port),
            pins.fpga_tdo.into_floating_input(&mut pins.port),
        )
        .into();
        USB_BUS = UsbDeviceBuilder::new(&allocator, ALTERA_BLASTER_USB_VID_PID)
            .manufacturer("Arduino LLC")
            .product("Arduino MKR Vidor 4000")
            .serial_number("12345678")
            .device_release(0x0400)
            .max_power(500)
            .build()
            .into();
        core.NVIC.set_priority(interrupt::USB, 0);
        NVIC::unmask(interrupt::USB);
    }

    loop {}
}

#[interrupt]
fn USB() {
    unsafe {
        USB_BUS.as_mut().map(|usb_dev| {
            USB_BLASTER.as_mut().map(|blaster| {
                usb_dev.poll(&mut [blaster]);
                if let Ok(_amount) = blaster.read() {
                    LED.as_mut().map(|led| led.toggle());
                }
                blaster.handle().unwrap();
                blaster.write(true).ok();
            });
        });
    };
}
