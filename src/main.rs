use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbip_device::UsbIpBus;

use std::net::Ipv6Addr;

mod cdc_eem;
mod dns;
mod ethernet;
mod icmpv6;
mod ipv6;
mod udp;

const PROTOCOL_NUMBER_TCP: u8 = 6;
const PROTOCOL_NUMBER_UDP: u8 = 17;
const PROTOCOL_NUMBER_ICMPV6: u8 = 58;

const LINK_LOCAL_MULTICAST_ADDR: Ipv6Addr = Ipv6Addr::new(0xFF02, 0, 0, 0, 0, 0, 0, 0x00FB);

fn main() {
    println!("Hello, world!");
    let bus_allocator = UsbBusAllocator::new(UsbIpBus::new());

    let mut eem_class = cdc_eem::CdcEemClass::new(&bus_allocator);

    let mut usb_bus = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x4242, 0x4242))
        .product("USB CDC EEM")
        .device_class(cdc_eem::USB_CLASS_CDC)
        .build();

    let ip_addr: Ipv6Addr = "fe80::4242".parse().unwrap();
    let mac_address = [42, 42, 42, 42, 42, 42];

    loop {
        usb_bus.poll(&mut [&mut eem_class]);

        let read = match eem_class.read() {
            Ok(r) => r,
            Err(UsbError::WouldBlock) => continue,
            Err(e) => panic!("Error {:?}", e),
        };

        for packet in read.iter() {
            let frame = match packet {
                cdc_eem::CdcEemPacket::Data { crc: _, frame } => frame,
            };

            let ethernet_frame = ethernet::EthernetFrame::parse(frame);
            if ethernet_frame.ether_type != ethernet::EtherType::Ipv6 {
                println!("Unhandled {:?}", ethernet_frame.ether_type);
                continue;
            }

            let ipv6 = ipv6::Ipv6::parse(ethernet_frame.payload);

            match ipv6.next_header {
                PROTOCOL_NUMBER_TCP => {
                    println!("tcp");
                }
                PROTOCOL_NUMBER_UDP => {
                    let udp = udp::Udp::parse(ipv6.payload);
                    if ipv6.destination_address == LINK_LOCAL_MULTICAST_ADDR
                        && udp.destination_port == 5353
                    {
                        let dns = dns::parse(udp.payload);
                        println!("mdns {:?}", dns);

                        for question in dns.questions.iter() {
                            if let Some(_) = question.name.parts().nth(0) {
                                println!("for me");

                                let aaaa_data = ip_addr.octets();
                                let ptr_data = [
                                    0x07, 0x6c, 0x69, 0x63, 0x6f, 0x72, 0x6e, 0x65, 0x05, 0x6c,
                                    0x6f, 0x63, 0x61, 0x6c, 0x00,
                                ];


                                let resource = match question.qtype {
                                    12 => dns::Resource {
                                        name: question.name,
                                        rtype: 12,
                                        class: 1 | 0x8000,
                                        ttl: 1,
                                        data: &ptr_data,
                                    },
                                    28 => dns::Resource {
                                        name: question.name,
                                        rtype: 28,
                                        class: 1 | 0x8000,
                                        ttl: 1,
                                        data: &aaaa_data,
                                    },
                                    _ => continue,
                                };

                                let dns_payload = dns::to_bytes(
                                    dns::Header {
                                        id: dns.header.id,
                                        query: false,
                                        opcode: 0,
                                        authoritative_answer: true,
                                        truncation: false,
                                        recursion_desired: false,
                                        recursion_available: false,
                                        rcode: 0,
                                    },
                                    &[resource],
                                );

                                let udp_payload = udp::Udp {
                                    source_port: 5353,
                                    destination_port: 5353,
                                    payload: &dns_payload,
                                }
                                .to_bytes(&ip_addr, &LINK_LOCAL_MULTICAST_ADDR);

                                let ipv6_payload = ipv6::Ipv6 {
                                    flags: 0x60000000,
                                    next_header: PROTOCOL_NUMBER_UDP,
                                    hop_limit: 255,
                                    source_address: ip_addr,
                                    destination_address: LINK_LOCAL_MULTICAST_ADDR,
                                    payload: &udp_payload,
                                }
                                .to_bytes();

                                let packet = ethernet::EthernetFrame {
                                    destination_mac: ethernet_frame.source_mac,
                                    source_mac: mac_address,
                                    ether_type: ethernet::EtherType::Ipv6,
                                    payload: &ipv6_payload,
                                    crc: 0xdeadbeef,
                                }
                                .to_bytes();

                                eem_class.write(&packet);
                            }
                        }
                    }
                }
                PROTOCOL_NUMBER_ICMPV6 => {
                    let icmpv6 = icmpv6::Icmpv6::parse(ipv6.payload);
                    println!("{:?}", icmpv6);
                    match icmpv6 {
                        icmpv6::Icmpv6::NeighborSolicitation { target_address } => {
                            if target_address == ip_addr {
                                let icmpv6_payload = icmpv6::Icmpv6::NeighborAdvertisement {
                                    router: false,
                                    solicited: true,
                                    override_: false,
                                    target_address,
                                    link_layer_address: mac_address,
                                }
                                .to_bytes(&ip_addr, &ipv6.source_address);

                                let ipv6_payload = ipv6::Ipv6 {
                                    flags: 0x60000000,
                                    next_header: PROTOCOL_NUMBER_ICMPV6,
                                    hop_limit: 255,
                                    source_address: ip_addr,
                                    destination_address: ipv6.source_address,
                                    payload: &icmpv6_payload,
                                }
                                .to_bytes();

                                let packet = ethernet::EthernetFrame {
                                    destination_mac: ethernet_frame.source_mac,
                                    source_mac: mac_address,
                                    ether_type: ethernet::EtherType::Ipv6,
                                    payload: &ipv6_payload,
                                    crc: 0xdeadbeef,
                                }
                                .to_bytes();

                                eem_class.write(&packet);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}
