use usb_device::bus::UsbBusAllocator;
use usb_device::Result;
use usb_device::{class_prelude::*, prelude::*};
use usbip_device::UsbIpBus;

use std::net::Ipv6Addr;

const USB_CLASS_CDC: u8 = 0x02;
const CDC_SUBCLASS_EEM: u8 = 0x0C;
const CDC_PROTOCOL_EEM: u8 = 0x07;

const MAX_PACKET_SIZE: u16 = 64;

struct CdcEemClass<'a, B: UsbBus> {
    intf: InterfaceNumber,
    in_ep: EndpointIn<'a, B>,
    out_ep: EndpointOut<'a, B>,
}

impl<B: UsbBus> CdcEemClass<'_, B> {
    pub fn new(alloc: &UsbBusAllocator<B>) -> CdcEemClass<'_, B> {
        CdcEemClass {
            intf: alloc.interface(),
            in_ep: alloc.bulk(MAX_PACKET_SIZE),
            out_ep: alloc.bulk(MAX_PACKET_SIZE),
        }
    }
}

impl<B: UsbBus> UsbClass<B> for CdcEemClass<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.iad(
            self.intf,
            1,
            USB_CLASS_CDC,
            CDC_SUBCLASS_EEM,
            CDC_PROTOCOL_EEM,
        )?;

        writer.interface(self.intf, USB_CLASS_CDC, CDC_SUBCLASS_EEM, CDC_PROTOCOL_EEM)?;

        writer.endpoint(&self.in_ep)?;
        writer.endpoint(&self.out_ep)?;

        Ok(())
    }

    fn reset(&mut self) {}

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();

        if !(req.request_type == control::RequestType::Class
            && req.recipient == control::Recipient::Interface
            && req.index == u8::from(self.intf) as u16)
        {
            return;
        }
        println!("Control in");
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();

        if !(req.request_type == control::RequestType::Class
            && req.recipient == control::Recipient::Interface
            && req.index == u8::from(self.intf) as u16)
        {
            return;
        }
        println!("Control out");
    }
}

fn main() {
    println!("Hello, world!");
    let bus_allocator = UsbBusAllocator::new(UsbIpBus::new());

    let mut eem_class = CdcEemClass::new(&bus_allocator);

    let mut usb_bus = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x4242, 0x4242))
        .product("USB CDC EEM")
        .device_class(USB_CLASS_CDC)
        .build();

    let mut read = move |buffer: &mut [u8]| loop {
        usb_bus.poll(&mut [&mut eem_class]);
        match eem_class.out_ep.read(&mut *buffer) {
            Ok(b) => return Ok(b),
            Err(UsbError::WouldBlock) => (),
            Err(e) => return Err(e),
        }
    };

    loop {
        let mut buffer = [0; 1024];
        let mut length = 0;

        if length < 2 {
            length += read(&mut buffer[length..]).unwrap();
        }

        let header = u16::from_le_bytes([buffer[0], buffer[1]]);
        let bmType = header >> 15;
        println!("bmType {}", bmType);
        match bmType {
            0 => {
                let bmCRC = header >> 14 & 0b1;
                let frame_length: usize = (header & 0x3FFF).into();
                println!("bmCRC {} length {}", bmCRC, length);
                while length < frame_length + 2 {
                    length += read(&mut buffer[length..]).unwrap();
                }
                println!("Frame {:?}", &buffer[2..frame_length + 2]);
                println!("Buffer {:?}", &buffer[0..length]);
                let ethertype = u16::from_be_bytes([buffer[2 + 12], buffer[2 + 12 + 1]]);
                println!("Ether type {:x}", ethertype);
                if ethertype == 0x86DD {
                    println!("IPV6");
                    let offset = 2 + 12 + 2;
                    let payload_length =
                        u16::from_be_bytes([buffer[offset + 4], buffer[offset + 4 + 1]]);
                    println!("Payload length {}", payload_length);
                    let source_address: [u8; 16] = (&buffer[(offset + 8)..(offset + 8 + 16)])
                        .try_into()
                        .unwrap();
                    let source_address = Ipv6Addr::from(source_address);

                    let dest_address: [u8; 16] = (&buffer
                        [(offset + 8 + 16)..(offset + 8 + 16 + 16)])
                        .try_into()
                        .unwrap();
                    let dest_address = Ipv6Addr::from(dest_address);

                    println!("{} -> {}", source_address, dest_address);
                    println!("Payload {:?}", &buffer[(offset + 8 + 32)..frame_length + 2]);
                }
                buffer.copy_within(frame_length + 2..length, 0);
                length -= frame_length + 2;
                println!("End of buffer {:?}", &buffer[0..length]);
            }
            1 => {}
            _ => (),
        }
        println!("BM type {} {}", bmType, header);
    }
}
