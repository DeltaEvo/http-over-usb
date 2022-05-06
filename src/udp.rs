use super::ipv6::checksum;
use std::net::Ipv6Addr;

#[derive(Debug)]
pub struct Udp<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub payload: &'a [u8],
}

impl Udp<'_> {
    pub fn parse(buffer: &[u8]) -> Udp<'_> {
        let source_port = u16::from_be_bytes([buffer[0], buffer[1]]);
        let destination_port = u16::from_be_bytes([buffer[2], buffer[3]]);
        let length = u16::from_be_bytes([buffer[4], buffer[5]]);
        let checksum = u16::from_be_bytes([buffer[6], buffer[7]]);
        let payload = &buffer[8..];
        assert!(payload.len() + 8 == length as usize);
        Udp {
            source_port,
            destination_port,
            payload,
        }
    }

    pub fn to_bytes(&self, src_addr: &Ipv6Addr, dst_addr: &Ipv6Addr) -> Vec<u8> {
        let mut packet = vec![];
        packet.extend_from_slice(&self.source_port.to_be_bytes());
        packet.extend_from_slice(&self.destination_port.to_be_bytes());
        packet.extend_from_slice(&((self.payload.len() + 8) as u16).to_be_bytes());
        packet.extend_from_slice(&[0, 0]);
        packet.extend_from_slice(self.payload);
        let mut checksum = !checksum::combine(&[
            checksum::pseudo_header(src_addr, dst_addr, 17, packet.len() as u32),
            checksum::data(&packet),
        ]);
        if checksum == 0 {
            checksum = 0xfff;
        }
        (&mut packet[6..8]).copy_from_slice(&checksum.to_be_bytes());
        packet
    }
}
