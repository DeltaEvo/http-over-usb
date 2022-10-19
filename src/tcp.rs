use super::ipv6::checksum;
use std::net::Ipv6Addr;

#[derive(Debug)]
pub struct Tcp<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub sequence_number: u32,
    pub acknowledgment_number: u32,
    pub data_offset: u8,
    pub urgent_pointer_is_significant: bool,
    pub acknowledgment: bool,
    pub push_function: bool,
    pub reset: bool,
    pub synchronize: bool,
    pub fin: bool,
    pub window: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
    pub payload: &'a [u8],
}

impl Tcp<'_> {
    pub fn parse(buffer: &[u8]) -> Tcp<'_> {
        let source_port = u16::from_be_bytes([buffer[0], buffer[1]]);
        let destination_port = u16::from_be_bytes([buffer[2], buffer[3]]);
        let sequence_number = u32::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
        let acknowledgment_number =
            u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
        let data_offset = buffer[12] >> 4;
        let urgent_pointer_is_significant = (buffer[13] & 0x20) != 0;
        let acknowledgment = (buffer[13] & 0x10) != 0;
        let push_function = (buffer[13] & 0x08) != 0;
        let reset = (buffer[13] & 0x04) != 0;
        let synchronize = (buffer[13] & 0x02) != 0;
        let fin = (buffer[13] & 0x01) != 0;
        let window = u16::from_be_bytes([buffer[14], buffer[15]]);
        let checksum = u16::from_be_bytes([buffer[16], buffer[17]]);
        let urgent_pointer = u16::from_be_bytes([buffer[18], buffer[19]]);
        let payload = &buffer[(data_offset as usize * 4)..];
        Tcp {
            source_port,
            destination_port,
            sequence_number,
            acknowledgment_number,
            data_offset,
            urgent_pointer_is_significant,
            acknowledgment,
            push_function,
            reset,
            synchronize,
            fin,
            window,
            checksum,
            urgent_pointer,
            payload,
        }
    }

    pub fn to_bytes(&self, src_addr: &Ipv6Addr, dst_addr: &Ipv6Addr) -> Vec<u8> {
        let mut packet = vec![];
        packet.extend_from_slice(&self.source_port.to_be_bytes());
        packet.extend_from_slice(&self.destination_port.to_be_bytes());
        packet.extend_from_slice(&self.sequence_number.to_be_bytes());
        packet.extend_from_slice(&self.acknowledgment_number.to_be_bytes());
        let data_offset: u8 = 5;
        packet.extend_from_slice(&(data_offset << 4).to_be_bytes());
        let mut flags: u8 = 0;
        flags |= (self.urgent_pointer_is_significant as u8) << 5;
        flags |= (self.acknowledgment as u8) << 4;
        flags |= (self.push_function as u8) << 3;
        flags |= (self.reset as u8) << 2;
        flags |= (self.synchronize as u8) << 1;
        flags |= (self.fin as u8) << 0;
        packet.extend_from_slice(&flags.to_be_bytes());
        packet.extend_from_slice(&self.window.to_be_bytes());
        packet.extend_from_slice(&[0, 0]);
        packet.extend_from_slice(&self.urgent_pointer.to_be_bytes());
        packet.extend_from_slice(self.payload);
        let mut checksum = !checksum::combine(&[
            checksum::pseudo_header(src_addr, dst_addr, 6, packet.len() as u32),
            checksum::data(&packet),
        ]);
        if checksum == 0 {
            checksum = 0xfff;
        }
        (&mut packet[16..18]).copy_from_slice(&checksum.to_be_bytes());
        packet
    }
}
