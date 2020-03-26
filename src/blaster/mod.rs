use arrayvec::ArrayVec;
use hal::usb::usb_device::{class_prelude::*, prelude::*, Result};

mod class;
mod ft245;
mod port;

use class::BlasterClass;

pub const ALTERA_BLASTER_USB_VID_PID: UsbVidPid = UsbVidPid(0x09FB, 0x6001);

// Depending on the underlying USB library (libusb or similar) the OS may send/receive more bytes than declared in the USB endpoint
// This will change the endpoint size (OS side) so it's less likely to send more than 64 bytes in a single chunk.
const BLASTER_READ_SIZE: u16 = 32;
const BLASTER_WRITE_SIZE: u16 = 64;

pub struct USBBlaster<'a, B: UsbBus> {
    class: BlasterClass<'a, B>,
    port: port::Port,
    send_buffer: ArrayVec<[u8; BLASTER_WRITE_SIZE as usize]>,
    recv_buffer: ArrayVec<[u8; BLASTER_READ_SIZE as usize]>,
    first_send: bool,
    send_ready: bool,
}

impl<'a, B: UsbBus> USBBlaster<'a, B> {
    const BLASTER_HEARTBEAT_TIME: u16 = 10;
    pub fn new(
        alloc: &'a UsbBusAllocator<B>,
        tdi: hal::gpio::Pa12<hal::gpio::Output<hal::gpio::PushPull>>,
        tck: hal::gpio::Pa13<hal::gpio::Output<hal::gpio::PushPull>>,
        tms: hal::gpio::Pa14<hal::gpio::Output<hal::gpio::PushPull>>,
        tdo: hal::gpio::Pa15<hal::gpio::Input<hal::gpio::Floating>>,
    ) -> USBBlaster<'a, B> {
        USBBlaster {
            class: BlasterClass::new(alloc, BLASTER_WRITE_SIZE, BLASTER_READ_SIZE),
            port: port::Port::new(tdi, tck, tms, tdo),
            send_buffer: ArrayVec::new(),
            recv_buffer: ArrayVec::new(),
            first_send: true,
            send_ready: true,
        }
    }

    fn read(&mut self) -> Result<usize> {
        self.send_ready = true;
        self.class.read(&mut self.recv_buffer)
    }

    fn write(&mut self, heartbeat: bool) -> Result<usize> {
        if !self.send_ready {
            return Ok(0);
        }

        let end = self.send_buffer.len().min(self.send_buffer.len() - 2);
        if end != 0 || self.first_send || heartbeat {
            self.send_buffer
                .push(BlasterClass::<'_, B>::FTDI_MODEM_STA_DUMMY[0]);
            self.send_buffer
                .push(BlasterClass::<'_, B>::FTDI_MODEM_STA_DUMMY[1]);
            self.first_send = false;
        } else {
            return Ok(0);
        }
        let amount = self.class.write(&self.send_buffer[0..end]);
        for _i in 0..*amount.as_ref().unwrap_or(&0) {
            self.send_buffer.pop_at(0);
        }
        /* Reset the control token to inform upper layer that a transfer is ongoing */
        // TODO: should this be enabled? Testing needed
        // self.send_ready = false;
        amount
    }

    pub fn process(&mut self, heartbeat: bool) -> Result<usize> {
        self.read()?;
        self.port
            .handle(&mut self.recv_buffer, &mut self.send_buffer);
        self.write(heartbeat)
    }
}

impl<B> UsbClass<B> for USBBlaster<'_, B>
where
    B: UsbBus,
{
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        self.class.get_configuration_descriptors(writer)
    }

    fn reset(&mut self) {
        self.class.reset();
        self.port.reset();
        self.first_send = true;
        self.send_ready = true;
        self.send_buffer.clear();
        self.recv_buffer.clear();
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        self.class.control_in(xfer);
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        self.class.control_out(xfer);
    }
}
