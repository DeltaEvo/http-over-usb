use super::ipv6::checksum;
use std::net::Ipv6Addr;

#[derive(Debug)]
pub enum Icmpv6 {
    NeighborSolicitation {
        target_address: Ipv6Addr,
    },
    NeighborAdvertisement {
        router: bool,
        solicited: bool,
        override_: bool,
        target_address: Ipv6Addr,
        link_layer_address: [u8; 6],
    },
    RouterSolicitation,
    Echo,
}

impl Icmpv6 {
    pub fn parse(packet: &[u8]) -> Self {
        let packet_type = packet[0];
        let code = packet[1];
        let checksum = u16::from_be_bytes([packet[2], packet[3]]);
        let body = &packet[4..];

        match packet_type {
            135 => {
                let target_address: [u8; 16] = (&body[4..20]).try_into().unwrap();
                let target_address = Ipv6Addr::from(target_address);

                Icmpv6::NeighborSolicitation { target_address }
            }
            133 => Icmpv6::RouterSolicitation,
            128 => Icmpv6::Echo,
            v => todo!("{}", v),
        }
    }
    pub fn to_bytes(&self, src_addr: &Ipv6Addr, dst_addr: &Ipv6Addr) -> Vec<u8> {
        match self {
            Icmpv6::NeighborAdvertisement {
                router,
                solicited,
                override_,
                target_address,
                link_layer_address,
            } => {
                let flags = if *router { 0b10000000 } else { 0 }
                    | if *solicited { 0b01000000 } else { 0 }
                    | if *override_ { 0b00100000 } else { 0 };

                let mut packet = vec![136, 0, 0, 0];

                packet.extend_from_slice(&[flags, 0, 0, 0]);
                packet.extend_from_slice(&target_address.octets());
                packet.extend_from_slice(&[2, 1]);
                packet.extend_from_slice(link_layer_address);

                let crc = !checksum::combine(&[
                    checksum::pseudo_header(src_addr, dst_addr, 58, packet.len() as u32),
                    checksum::data(&packet),
                ]);
                (&mut packet[2..4]).copy_from_slice(&crc.to_be_bytes());
                packet
            }
            v => todo!("{:?}", v),
        }
    }
}
