use std::collections::HashSet;

use pnet::datalink::{self, Channel::Ethernet, NetworkInterface};
use pnet::packet::tcp::TcpPacket;
use pnet::packet::{
    ethernet::{EtherTypes, EthernetPacket},
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
    udp::UdpPacket,
    Packet,
};

use thiserror::Error;
use tokio::sync::mpsc::Sender;
use tokio::time::{self, Duration};

use super::config::Config;
use super::global::check_capture_state;
use xdump::xlog;

pub struct Capture {
    interface: NetworkInterface,
    excluded_ports: HashSet<u16>,
    sender: Sender<Vec<u8>>,
}

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("Network interface not found: {0}")]
    InterfaceNotFound(String),
}

impl Capture {
    pub fn new(config: &Config, sender: Sender<Vec<u8>>) -> Result<Self, CaptureError> {
        let interface = Self::find_interface(config.interface.as_str())?;
        xlog!('I', format!("Listening on interface: {:?}", interface));

        let excluded_ports: HashSet<u16> = config.excluded_ports.iter().cloned().collect();

        Ok(Self {
            interface,
            excluded_ports,
            sender,
        })
    }

    fn find_interface(interface_name: &str) -> Result<NetworkInterface, CaptureError> {
        let filter = |iface: &NetworkInterface| iface.name == interface_name;

        pnet::datalink::interfaces()
            .into_iter()
            .find(filter)
            .ok_or_else(|| CaptureError::InterfaceNotFound(interface_name.into()))
    }

    pub async fn start(&self) -> Result<(), CaptureError> {
        let (_, mut rx) = match datalink::channel(&self.interface, Default::default()) {
            Ok(Ethernet(_tx, _rx)) => (_tx, _rx),
            Ok(_) => panic!("xdump: unhandled channel type"),
            Err(e) => panic!("xdump: unable to create channel: {}", e),
        };

        loop {
            match rx.next() {
                Ok(packet) => {
                    if check_capture_state() {
                        let ethernet = EthernetPacket::new(packet).unwrap();
                        self.handle_ethernet_frame(&ethernet).await;
                    }
                }
                Err(e) => {
                    xlog!('W', format!("Failed to receive packet: {}", e));
                }
            }

            // 나노초 단위로 수행
            tokio::time::sleep(std::time::Duration::from_nanos(1_000_000)).await;
        }
    }

    async fn handle_ethernet_frame(&self, ethernet: &EthernetPacket<'_>) {
        match ethernet.get_ethertype() {
            EtherTypes::Ipv4 => {
                let ipv4_packet = match Ipv4Packet::new(ethernet.payload()) {
                    Some(packet) => packet,
                    None => {
                        xlog!('W', "Invalid IPv4 packet.");
                        return;
                    }
                };

                match ipv4_packet.get_next_level_protocol() {
                    IpNextHeaderProtocols::Udp => {
                        self.handle_udp_packet(&ipv4_packet, ethernet).await
                    }
                    IpNextHeaderProtocols::Tcp => {
                        self.handle_tcp_packet(&ipv4_packet, ethernet).await
                    }
                    _ => {} // Do nothing
                }
            }
            _ => {
                // IPv4 이외의 모든 Ethernet 패킷 처리
                let packet = ethernet.packet().to_vec();
                if self.should_send_packet(ethernet).await {
                    let _ = self.sender.send(packet).await;
                } else {
                    xlog!('W', "Ethernet packet excluded based on custom logic.");
                }
            }
        }
    }

    async fn handle_udp_packet(&self, ipv4_packet: &Ipv4Packet<'_>, ethernet: &EthernetPacket<'_>) {
        let udp = match UdpPacket::new(ipv4_packet.payload()) {
            Some(packet) => packet,
            None => {
                xlog!('W', "Invalid UDP packet.");
                return;
            }
        };

        if self.should_send_packet(ethernet).await {
            self.process_transport_layer_packet(udp.get_source(), udp.get_destination(), ethernet)
                .await;
        }
    }

    async fn handle_tcp_packet(&self, ipv4_packet: &Ipv4Packet<'_>, ethernet: &EthernetPacket<'_>) {
        let tcp = match TcpPacket::new(ipv4_packet.payload()) {
            Some(packet) => packet,
            None => {
                xlog!('W', "Invalid TCP packet.");
                return;
            }
        };

        if self.should_send_packet(ethernet).await {
            self.process_transport_layer_packet(tcp.get_source(), tcp.get_destination(), ethernet)
                .await;
        }
    }

    async fn process_transport_layer_packet(
        &self,
        source_port: u16,
        destination_port: u16,
        ethernet: &EthernetPacket<'_>,
    ) {
        if self.excluded_ports.contains(&source_port)
            || self.excluded_ports.contains(&destination_port)
        {
            return;
        }

        let packet = ethernet.packet().to_vec();
        if self.should_send_packet(ethernet).await {
            let _ = self.sender.send(packet).await;
        } else {
            xlog!('W', "Ethernet packet excluded based on custom logic.");
        }
    }

    async fn should_send_packet(&self, _: &EthernetPacket<'_>) -> bool {
        /*
        TODO
        */
        true
    }
}
