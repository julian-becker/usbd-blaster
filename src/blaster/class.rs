use hal::gpio::IntoFunction;
use hal::prelude::*;
use hal::usb::usb_device::{class_prelude::*, control::RequestType, descriptor, Result};
use hal::Pins;

use super::ft245::Rom;

pub struct BlasterClass<'a, B: UsbBus> {
    rom: Rom,
    iface: InterfaceNumber,
    in_ep: EndpointIn<'a, B>,
    out_ep: EndpointOut<'a, B>,
}

impl<'a, B: hal::usb::usb_device::bus::UsbBus> UsbClass<B> for BlasterClass<'a, B> {
    fn get_configuration_descriptors(&self, w: &mut DescriptorWriter) -> Result<()> {
        w.interface(self.iface, 0xFF, 0xFF, 0xFF)?;
        w.endpoint(&self.in_ep)?;
        w.endpoint(&self.out_ep)?;
        Ok(())
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        if xfer.request().request_type == RequestType::Vendor {
            match xfer.request().request {
                Self::FTDI_VEN_REQ_RD_EEPROM => {
                    let addr = ((xfer.request().index << 1) & 0x7F) as usize;
                    xfer.accept_with(&self.rom.buf[addr..addr + 2]).unwrap();
                }
                Self::FTDI_VEN_REQ_GET_MODEM_STA => {
                    xfer.accept_with(&[0x01, 0x60]).unwrap();
                }
                Self::FTDI_VEN_REQ_GET_LAT_TIMER => {
                    xfer.accept_with(&['6' as u8; 1]).unwrap();
                }
                _ => {
                    // return dummy data
                    xfer.accept_with(&[0u8; 2]).unwrap();
                }
            }
        }
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        if xfer.request().request_type == RequestType::Vendor {
            xfer.accept().unwrap();
            // usbd.epBank1SetByteCount(ep, 0); ??
        }
    }
}

impl<B: UsbBus> BlasterClass<'_, B> {
    const FTDI_VEN_REQ_RESET: u8 = 0x00;
    const FTDI_VEN_REQ_SET_BAUDRATE: u8 = 0x01;
    const FTDI_VEN_REQ_SET_DATA_CHAR: u8 = 0x02;
    const FTDI_VEN_REQ_SET_FLOW_CTRL: u8 = 0x03;
    const FTDI_VEN_REQ_SET_MODEM_CTRL: u8 = 0x04;
    const FTDI_VEN_REQ_GET_MODEM_STA: u8 = 0x05;
    const FTDI_VEN_REQ_SET_EVENT_CHAR: u8 = 0x06;
    const FTDI_VEN_REQ_SET_ERR_CHAR: u8 = 0x07;
    const FTDI_VEN_REQ_SET_LAT_TIMER: u8 = 0x09;
    const FTDI_VEN_REQ_GET_LAT_TIMER: u8 = 0x0A;
    const FTDI_VEN_REQ_SET_BITMODE: u8 = 0x0B;
    const FTDI_VEN_REQ_RD_PINS: u8 = 0x0C;
    const FTDI_VEN_REQ_RD_EEPROM: u8 = 0x90;
    const FTDI_VEN_REQ_WR_EEPROM: u8 = 0x91;
    const FTDI_VEN_REQ_ES_EEPROM: u8 = 0x92;
    pub fn new(alloc: &UsbBusAllocator<B>, max_packet_size: u16) -> BlasterClass<'_, B> {
        BlasterClass {
            rom: Rom::new(),
            iface: alloc.interface(),
            in_ep: alloc
                .alloc(None, EndpointType::Bulk, max_packet_size, 1)
                .unwrap(),
            out_ep: alloc
                .alloc(None, EndpointType::Bulk, max_packet_size, 1)
                .unwrap(),
        }
    }
}