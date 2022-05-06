#[derive(Debug, PartialEq)]
pub enum EtherType {
    Ipv4,
    Ipv6,
    Unknown(u16),
}

// Layer 2 Ethernet Frame
#[derive(Debug)]
pub struct EthernetFrame<'a> {
    pub destination_mac: [u8; 6],
    pub source_mac: [u8; 6],
    pub ether_type: EtherType,
    pub payload: &'a [u8],
    pub crc: u32,
}

impl EthernetFrame<'_> {
    pub fn parse(buffer: &[u8]) -> EthernetFrame<'_> {
        let destination_mac = (&buffer[0..6]).try_into().unwrap();
        let source_mac = (&buffer[6..12]).try_into().unwrap();
        let ether_type = u16::from_be_bytes([buffer[12], buffer[13]]);
        let ether_type = match ether_type {
            0x0800 => EtherType::Ipv4,
            0x86DD => EtherType::Ipv6,
            v => EtherType::Unknown(v),
        };
        let payload = &buffer[14..(buffer.len() - 4)];
        let crc = &buffer[(buffer.len() - 4)..];
        let crc = u32::from_be_bytes(crc.try_into().unwrap());
        EthernetFrame {
            destination_mac,
            source_mac,
            ether_type,
            payload,
            crc,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = vec![];
        packet.extend_from_slice(&self.destination_mac);
        packet.extend_from_slice(&self.source_mac);
        assert!(self.ether_type == EtherType::Ipv6);
        packet.extend_from_slice(&(0x86DD as u16).to_be_bytes());
        packet.extend_from_slice(self.payload);
        packet.extend_from_slice(&self.crc.to_be_bytes());
        packet
    }
}
