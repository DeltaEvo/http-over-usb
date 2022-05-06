use usb_device::class_prelude::*;
use usb_device::Result;

pub const USB_CLASS_CDC: u8 = 0x02;
const CDC_SUBCLASS_EEM: u8 = 0x0C;
const CDC_PROTOCOL_EEM: u8 = 0x07;

const EEM_PACKET_TYPE_DATA: u8 = 0;
const EEM_PACKET_TYPE_COMMAND: u8 = 1;

const MAX_PACKET_SIZE: u16 = 64;
const MAX_TRANSFER_SIZE: usize = 1024;

pub struct CdcEemClass<'a, B: UsbBus> {
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

impl<B: UsbBus> CdcEemClass<'_, B> {
    pub fn read(&mut self) -> Result<CdcEemRead> {
        let mut buffer = [0; MAX_TRANSFER_SIZE];
        let mut index = 0;
        loop {
            match self.out_ep.read(&mut buffer[index..]) {
                Ok(bytes) => index += bytes,
                Err(UsbError::WouldBlock) => break,
                Err(e) => return Err(e),
            }
        }
        if index == 0 {
            Err(UsbError::WouldBlock)
        } else {
            Ok(CdcEemRead {
                buffer,
                length: index,
            })
        }
    }
    pub fn write(&mut self, packet: &[u8]) {
        let mut buffer = vec![];
        buffer.extend_from_slice(&(packet.len() as u16 & 0x3FFF).to_le_bytes());
        buffer.extend_from_slice(packet);

        self.in_ep.write(&buffer).unwrap();
    }
}

pub struct CdcEemRead {
    buffer: [u8; MAX_TRANSFER_SIZE],
    length: usize,
}

pub struct CdcEemReadIterator<'a>(&'a [u8]);

impl CdcEemRead {
    pub fn iter(&self) -> CdcEemReadIterator<'_> {
        CdcEemReadIterator(&self.buffer[..self.length])
    }
}

pub enum CdcEemPacket<'a> {
    Data { crc: bool, frame: &'a [u8] },
}

impl<'a> Iterator for CdcEemReadIterator<'a> {
    type Item = CdcEemPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        let header = u16::from_le_bytes([self.0[0], self.0[1]]);
        let bm_type = header >> 15;

        match bm_type as u8 {
            EEM_PACKET_TYPE_DATA => {
                let crc = (header >> 14 & 0b1) == 1;
                let frame_length: usize = (header & 0x3FFF).into();
                let frame = &self.0[2..frame_length + 2];
                self.0 = &self.0[frame_length + 2..];
                Some(CdcEemPacket::Data { crc, frame })
            }
            EEM_PACKET_TYPE_COMMAND => {
                todo!();
            }
            _ => unreachable!(),
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
