use std::net::Ipv6Addr;

#[derive(Debug)]
pub struct Ipv6<'a> {
    pub flags: u32,
    pub next_header: u8,
    pub hop_limit: u8,
    pub source_address: Ipv6Addr,
    pub destination_address: Ipv6Addr,
    pub payload: &'a [u8],
}

impl Ipv6<'_> {
    pub fn parse(buffer: &[u8]) -> Ipv6<'_> {
        let flags = (&buffer[0..4]).try_into().unwrap();
        let flags = u32::from_be_bytes(flags);
        let payload_length = u16::from_be_bytes([buffer[4], buffer[5]]);
        let next_header = buffer[6];
        let hop_limit = buffer[7];
        let source_address: [u8; 16] = (&buffer[8..24]).try_into().unwrap();
        let source_address = Ipv6Addr::from(source_address);

        let destination_address: [u8; 16] = (&buffer[24..40]).try_into().unwrap();
        let destination_address = Ipv6Addr::from(destination_address);

        let payload = &buffer[40..];

        assert_eq!(payload.len(), payload_length as usize);

        Ipv6 {
            flags,
            next_header,
            hop_limit,
            source_address,
            destination_address,
            payload,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = vec![];
        packet.extend_from_slice(&self.flags.to_be_bytes());
        packet.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        packet.push(self.next_header);
        packet.push(self.hop_limit);
        packet.extend_from_slice(&self.source_address.octets());
        packet.extend_from_slice(&self.destination_address.octets());
        packet.extend_from_slice(self.payload);
        packet
    }
}

// https://github.com/smoltcp-rs/smoltcp/blob/master/src/wire/ip.rs#L806
pub mod checksum {
    use std::net::Ipv6Addr;

    fn propagate_carries(word: u32) -> u16 {
        let sum = (word >> 16) + (word & 0xffff);
        ((sum >> 16) as u16) + (sum as u16)
    }

    /// Compute an RFC 1071 compliant checksum (without the final complement).
    pub fn data(mut data: &[u8]) -> u16 {
        let mut accum = 0;

        // For each 32-byte chunk...
        const CHUNK_SIZE: usize = 32;
        while data.len() >= CHUNK_SIZE {
            let mut d = &data[..CHUNK_SIZE];
            // ... take by 2 bytes and sum them.
            while d.len() >= 2 {
                accum += u16::from_be_bytes([d[0], d[1]]) as u32;
                d = &d[2..];
            }

            data = &data[CHUNK_SIZE..];
        }

        // Sum the rest that does not fit the last 32-byte chunk,
        // taking by 2 bytes.
        while data.len() >= 2 {
            accum += u16::from_be_bytes([data[0], data[1]]) as u32;
            data = &data[2..];
        }

        // Add the last remaining odd byte, if any.
        if let Some(&value) = data.first() {
            accum += (value as u32) << 8;
        }

        propagate_carries(accum)
    }

    /// Combine several RFC 1071 compliant checksums.
    pub fn combine(checksums: &[u16]) -> u16 {
        let mut accum: u32 = 0;
        for &word in checksums {
            accum += word as u32;
        }
        propagate_carries(accum)
    }

    /// Compute an IP pseudo header checksum.
    pub fn pseudo_header(
        src_addr: &Ipv6Addr,
        dst_addr: &Ipv6Addr,
        protocol: u8,
        length: u32,
    ) -> u16 {
        let mut proto_len = [0u8; 8];
        proto_len[7] = protocol;
        (&mut proto_len[0..4]).copy_from_slice(&length.to_be_bytes());
        combine(&[
            data(&src_addr.octets()),
            data(&dst_addr.octets()),
            data(&proto_len[..]),
        ])
    }
}
