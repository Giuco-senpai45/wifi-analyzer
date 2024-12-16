use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PacketInfo {
    pub src_mac: String,
    pub dst_mac: String,
    pub src_ip: Option<String>,
    pub dst_ip: Option<String>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub protocol: String,
    pub length: usize,
    pub payload: Option<String>,
    pub timestamp: u64,
}

pub struct PacketCapture {
    pub running: Arc<Mutex<bool>>,
    pub device: Arc<Mutex<Option<String>>>,
    pub captured_packets: Arc<Mutex<Vec<PacketInfo>>>,
    pub last_fetch_timestamp: Arc<Mutex<u64>>,
}

impl PacketCapture {
    pub fn new() -> Self {
        PacketCapture {
            running: Arc::new(Mutex::new(false)),
            device: Arc::new(Mutex::new(None)),
            captured_packets: Arc::new(Mutex::new(Vec::new())),
            last_fetch_timestamp: Arc::new(Mutex::new(0)),
        }
    }
}

// Protocol numbers
const IP_PROTO_TCP: u8 = 6;
const IP_PROTO_UDP: u8 = 17;

// Ethernet frame parsing
fn parse_mac_address(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<String>>()
        .join(":")
}

// IPv4 header parsing
struct Ipv4Header {
    version: u8,
    ihl: u8,
    total_length: u16,
    protocol: u8,
    src_addr: Ipv4Addr,
    dst_addr: Ipv4Addr,
}

fn parse_ipv4_header(data: &[u8]) -> Option<(Ipv4Header, usize)> {
    if data.len() < 20 {
        return None;
    }

    let version_ihl = data[0];
    let version = version_ihl >> 4;
    let ihl = (version_ihl & 0x0F) * 4; // IHL is in 4-byte units

    if version != 4 || data.len() < ihl as usize {
        return None;
    }

    let total_length = u16::from_be_bytes([data[2], data[3]]);
    let protocol = data[9];

    let src_addr = Ipv4Addr::new(data[12], data[13], data[14], data[15]);
    let dst_addr = Ipv4Addr::new(data[16], data[17], data[18], data[19]);

    Some((
        Ipv4Header {
            version,
            ihl,
            total_length,
            protocol,
            src_addr,
            dst_addr,
        },
        ihl as usize,
    ))
}

// IPv6 header parsing
struct Ipv6Header {
    version: u8,
    next_header: u8,
    src_addr: Ipv6Addr,
    dst_addr: Ipv6Addr,
}

fn parse_ipv6_header(data: &[u8]) -> Option<(Ipv6Header, usize)> {
    if data.len() < 40 {
        return None;
    }

    let version = data[0] >> 4;
    if version != 6 {
        return None;
    }

    let next_header = data[6];

    let mut src_addr_bytes = [0u8; 16];
    src_addr_bytes.copy_from_slice(&data[8..24]);
    let src_addr = Ipv6Addr::from(src_addr_bytes);

    let mut dst_addr_bytes = [0u8; 16];
    dst_addr_bytes.copy_from_slice(&data[24..40]);
    let dst_addr = Ipv6Addr::from(dst_addr_bytes);

    Some((
        Ipv6Header {
            version,
            next_header,
            src_addr,
            dst_addr,
        },
        40,
    ))
}

// TCP header parsing
struct TcpHeader {
    src_port: u16,
    dst_port: u16,
    data_offset: u8,
}

fn parse_tcp_header(data: &[u8]) -> Option<(TcpHeader, usize)> {
    if data.len() < 20 {
        return None;
    }

    let src_port = u16::from_be_bytes([data[0], data[1]]);
    let dst_port = u16::from_be_bytes([data[2], data[3]]);
    let data_offset = (data[12] >> 4) * 4; // Data offset is in 4-byte units

    if data.len() < data_offset as usize {
        return None;
    }

    Some((
        TcpHeader {
            src_port,
            dst_port,
            data_offset,
        },
        data_offset as usize,
    ))
}

// UDP header parsing
struct UdpHeader {
    src_port: u16,
    dst_port: u16,
}

fn parse_udp_header(data: &[u8]) -> Option<(UdpHeader, usize)> {
    if data.len() < 8 {
        return None;
    }

    let src_port = u16::from_be_bytes([data[0], data[1]]);
    let dst_port = u16::from_be_bytes([data[2], data[3]]);

    Some((UdpHeader { src_port, dst_port }, 8))
}

pub fn parse_packet(packet: &pcap::Packet) -> Result<PacketInfo, Box<dyn std::error::Error>> {
    let data = packet.data;

    // Ensure we have at least an Ethernet header (14 bytes)
    if data.len() < 14 {
        return Err("Packet too short for Ethernet header".into());
    }

    // Parse Ethernet header
    let dst_mac = parse_mac_address(&data[0..6]);
    let src_mac = parse_mac_address(&data[6..12]);
    let ethertype = u16::from_be_bytes([data[12], data[13]]);

    let mut offset = 14;
    let mut protocol = String::new();
    let mut src_ip = None;
    let mut dst_ip = None;
    let mut src_port = None;
    let mut dst_port = None;
    let mut payload = None;

    // Parse IP header
    match ethertype {
        0x0800 => {
            // IPv4
            if let Some((ip_header, ip_header_len)) = parse_ipv4_header(&data[offset..]) {
                src_ip = Some(ip_header.src_addr.to_string());
                dst_ip = Some(ip_header.dst_addr.to_string());
                protocol = format!("IPv4 ({})", ip_header.protocol);
                offset += ip_header_len;

                // Parse TCP/UDP
                match ip_header.protocol {
                    IP_PROTO_TCP => {
                        if let Some((tcp_header, tcp_header_len)) =
                            parse_tcp_header(&data[offset..])
                        {
                            src_port = Some(tcp_header.src_port);
                            dst_port = Some(tcp_header.dst_port);
                            offset += tcp_header_len;

                            // Extract HTTP payload if port 80
                            if tcp_header.dst_port == 80 {
                                payload = String::from_utf8(data[offset..].to_vec()).ok();
                            }
                        }
                    }
                    IP_PROTO_UDP => {
                        if let Some((udp_header, udp_header_len)) =
                            parse_udp_header(&data[offset..])
                        {
                            src_port = Some(udp_header.src_port);
                            dst_port = Some(udp_header.dst_port);
                            offset += udp_header_len;
                        }
                    }
                    _ => {}
                }
            }
        }
        0x86DD => {
            // IPv6
            if let Some((ip_header, ip_header_len)) = parse_ipv6_header(&data[offset..]) {
                src_ip = Some(ip_header.src_addr.to_string());
                dst_ip = Some(ip_header.dst_addr.to_string());
                protocol = format!("IPv6 ({})", ip_header.next_header);
                offset += ip_header_len;

                // Parse TCP/UDP
                match ip_header.next_header {
                    IP_PROTO_TCP => {
                        if let Some((tcp_header, tcp_header_len)) =
                            parse_tcp_header(&data[offset..])
                        {
                            src_port = Some(tcp_header.src_port);
                            dst_port = Some(tcp_header.dst_port);
                            offset += tcp_header_len;

                            // Extract HTTP payload if port 80
                            if tcp_header.dst_port == 80 {
                                payload = String::from_utf8(data[offset..].to_vec()).ok();
                            }
                        }
                    }
                    IP_PROTO_UDP => {
                        if let Some((udp_header, udp_header_len)) =
                            parse_udp_header(&data[offset..])
                        {
                            src_port = Some(udp_header.src_port);
                            dst_port = Some(udp_header.dst_port);
                            offset += udp_header_len;
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {
            protocol = format!("Unknown (0x{:04X})", ethertype);
        }
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    Ok(PacketInfo {
        src_mac,
        dst_mac,
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        protocol,
        length: data.len(),
        payload,
        timestamp,
    })
}
