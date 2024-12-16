use log::debug;
use log::{error, info, warn};
use pcap::{Active, Capture};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::radiotap::RadiotapParser;

#[derive(Clone, Debug)]
pub struct ScanProgress {
    pub networks: Vec<WiFiNetwork>,
    pub is_complete: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WiFiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal_quality: u32,
    pub frequency: u32,
    pub channel: u32,
    pub security: String,
    pub last_seen: std::time::SystemTime,
    pub beacon_count: u32,
    pub avg_signal: i32,
}

pub struct WiFiScanner {
    networks: Arc<Mutex<HashMap<String, WiFiNetwork>>>,
    capture: Capture<Active>,
    stop_flag: Arc<Mutex<bool>>,
}

impl WiFiScanner {
    pub fn new(interface: &str) -> Result<Self, String> {
        let mut capture = match Capture::from_device(interface)
            .map_err(|e| e.to_string())?
            .promisc(true)
            .snaplen(2048)
            .timeout(100)
            .open()
        {
            Ok(cap) => cap,
            Err(e) => return Err(format!("Failed to open capture: {}", e)),
        };

        capture
            .set_datalink(pcap::Linktype::IEEE802_11_RADIOTAP)
            .map_err(|e| format!("Failed to set datalink type: {}", e))?;

        let filter = "type mgt subtype beacon";
        debug!("Setting pcap filter: {}", filter);
        capture
            .filter(filter, true)
            .map_err(|e| format!("Failed to set filter: {}", e))?;

        Ok(Self {
            networks: Arc::new(Mutex::new(HashMap::new())),
            capture,
            stop_flag: Arc::new(Mutex::new(false)),
        })
    }

    pub fn start_scanning(&mut self) -> Result<(), String> {
        info!("Starting WiFi scan");
        *self.stop_flag.lock().unwrap() = false;

        while !*self.stop_flag.lock().unwrap() {
            let packet_data = match self.capture.next_packet() {
                Ok(packet) => packet.data.to_vec(),
                Err(pcap::Error::TimeoutExpired) => continue,
                Err(e) => {
                    error!("Error capturing packet: {}", e);
                    break;
                }
            };

            match self.process_packet(&packet_data) {
                Ok(_) => (),
                Err(e) => warn!("Error processing packet: {}", e),
            }
        }

        Ok(())
    }

    pub fn stop_scanning(&mut self) {
        *self.stop_flag.lock().unwrap() = true;
    }

    fn process_packet(&self, data: &[u8]) -> Result<(), String> {
        debug!("Processing packet of size: {} bytes", data.len());

        if data.len() < 8 {
            return Err(format!("Packet too small: {} bytes", data.len()));
        }

        let mut parser = RadiotapParser::new(data);
        match parser.parse_wifi_frame() {
            Ok(frame) => {
                // Only process beacon frames (type = 0, subtype = 8)
                let frame_type = (frame.frame_control & 0x000C) >> 2;
                let frame_subtype = (frame.frame_control & 0x00F0) >> 4;

                debug!(
                    "Frame type: {}, subtype: {}, frame control: {:04X}",
                    frame_type, frame_subtype, frame.frame_control
                );

                if frame_type == 0 && (frame_subtype == 8) {
                    if let Some(ssid) = frame.ssid {
                        // Skip hidden networks
                        if ssid.is_empty() {
                            debug!("Skipping hidden network");
                            return Ok(());
                        }

                        let bssid = format!(
                            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            frame.addr3[0],
                            frame.addr3[1],
                            frame.addr3[2],
                            frame.addr3[3],
                            frame.addr3[4],
                            frame.addr3[5]
                        );

                        debug!("Processing network - SSID: {}, BSSID: {}", ssid, bssid);

                        if let Ok(mut networks) = self.networks.lock() {
                            let network = networks.entry(bssid.clone()).or_insert_with(|| {
                                info!("Found new network: {} ({})", ssid, bssid);
                                WiFiNetwork {
                                    ssid: ssid.clone(),
                                    bssid: bssid.clone(),
                                    signal_quality: 0,
                                    frequency: frame.radiotap.channel_freq.unwrap_or(0) as u32,
                                    channel: frame.channel.unwrap_or(0) as u32,
                                    security: parse_security_info(frame.frame_control),
                                    last_seen: std::time::SystemTime::now(),
                                    beacon_count: 0,
                                    avg_signal: 0,
                                }
                            });

                            network.last_seen = std::time::SystemTime::now();
                            network.beacon_count += 1;

                            // Safe signal quality calculation
                            if let Some(signal) = frame.radiotap.antenna_signal {
                                // Convert to positive scale
                                let normalized_signal = (signal + 100).max(0) as u32;
                                // Scale to 0-100 range, capping at 100
                                network.signal_quality =
                                    normalized_signal.saturating_mul(2).min(100);

                                debug!(
                                    "Updated signal quality for {}: {} (raw: {} dBm)",
                                    ssid, network.signal_quality, signal
                                );

                                // Safe average signal calculation
                                let beacon_count = network.beacon_count as i32;
                                if beacon_count > 1 {
                                    network.avg_signal = (network.avg_signal * (beacon_count - 1)
                                        + signal as i32)
                                        / beacon_count;
                                } else {
                                    network.avg_signal = signal as i32;
                                }
                            }
                        } else {
                            warn!("Failed to acquire lock for networks");
                        }
                    } else {
                        debug!("Skipping frame with no SSID");
                    }
                } else {
                    debug!("Skipping non-beacon/probe frame");
                }
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Failed to parse packet: {}. First 16 bytes: {:02X?}",
                    e,
                    &data[..16.min(data.len())]
                );
                Ok(())
            }
        }
    }

    pub fn get_networks(&self) -> Vec<WiFiNetwork> {
        match self.networks.lock() {
            Ok(networks) => {
                let result: Vec<WiFiNetwork> = networks
                    .values()
                    .filter(|network| {
                        network.last_seen.elapsed().unwrap_or_default() < Duration::from_secs(10)
                    })
                    .cloned()
                    .collect();

                info!(
                    "Retrieved {} networks (total in cache: {})",
                    result.len(),
                    networks.len()
                );

                result
            }
            Err(e) => {
                warn!("Failed to acquire lock for networks: {:?}", e);
                Vec::new()
            }
        }
    }
}

fn parse_security_info(frame_control: u16) -> String {
    // Extract capability information bits
    let privacy = (frame_control & 0x0010) != 0;

    if privacy {
        "WPA/WPA2".to_string()
    } else {
        "Open".to_string()
    }
}

pub fn scan_wifi_internal(
    interface: &str,
) -> Result<(Sender<()>, std::sync::mpsc::Receiver<ScanProgress>), String> {
    info!("Initializing WiFi scanner for interface: {}", interface);

    let scanner = Arc::new(Mutex::new(WiFiScanner::new(interface)?));
    let scanner_clone = Arc::clone(&scanner);

    let (progress_tx, progress_rx) = channel();
    let (stop_tx, stop_rx) = channel();

    info!("Starting scan...");
    thread::spawn(move || {
        if let Ok(mut scanner) = scanner_clone.lock() {
            let mut last_update_time = std::time::Instant::now();
            let update_interval = Duration::from_millis(500); // Reduced interval for more frequent updates

            while stop_rx.try_recv().is_err() {
                let packet_data = match scanner.capture.next_packet() {
                    Ok(packet) => packet.data.to_vec(),
                    Err(pcap::Error::TimeoutExpired) => {
                        // Send progress update even on timeout
                        if last_update_time.elapsed() >= update_interval {
                            let current_networks = scanner.get_networks();
                            let progress = ScanProgress {
                                networks: current_networks,
                                is_complete: false,
                            };
                            if let Err(e) = progress_tx.send(progress) {
                                warn!("Failed to send progress update: {}", e);
                            }
                            last_update_time = std::time::Instant::now();
                        }
                        continue;
                    }
                    Err(e) => {
                        error!("Error capturing packet: {}", e);
                        break;
                    }
                };

                if let Err(e) = scanner.process_packet(&packet_data) {
                    warn!("Error processing packet: {}", e);
                }

                // Send progress update if interval elapsed
                if last_update_time.elapsed() >= update_interval {
                    let current_networks = scanner.get_networks();
                    debug!(
                        "Sending progress update with {} networks",
                        current_networks.len()
                    );
                    let progress = ScanProgress {
                        networks: current_networks,
                        is_complete: false,
                    };
                    if let Err(e) = progress_tx.send(progress) {
                        warn!("Failed to send progress update: {}", e);
                    }
                    last_update_time = std::time::Instant::now();
                }
            }

            // Send final update with actual networks
            let final_networks = scanner.get_networks();
            info!(
                "Scan completed, sending final update with {} networks",
                final_networks.len()
            );
            let progress = ScanProgress {
                networks: final_networks,
                is_complete: true,
            };
            if let Err(e) = progress_tx.send(progress) {
                warn!("Failed to send final progress update: {}", e);
            }
        }
    });

    Ok((stop_tx, progress_rx))
}
