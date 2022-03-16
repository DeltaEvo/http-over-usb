use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
use usbip_device::UsbIpBus;

use std::net::Ipv6Addr;

mod cdc_eem;
mod ethernet;
mod icmpv6;
mod ipv6;

const PROTOCOL_NUMBER_ICMPV6: u8 = 58;

fn main() {
    println!("Hello, world!");
    let bus_allocator = UsbBusAllocator::new(UsbIpBus::new());

    let mut eem_class = cdc_eem::CdcEemClass::new(&bus_allocator);

    let mut usb_bus = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x4242, 0x4242))
        .product("USB CDC EEM")
        .device_class(cdc_eem::USB_CLASS_CDC)
        .build();

    let ip_addr = "fe80::4242".parse().unwrap();
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
            assert!(ethernet_frame.ether_type == ethernet::EtherType::Ipv6);

            let ipv6 = ipv6::Ipv6::parse(ethernet_frame.payload);

            match ipv6.next_header {
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
