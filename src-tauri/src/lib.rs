use log::{debug, error, info, warn};
use pcap::{Capture, Device};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::result::Result;
use std::sync::Arc;
use std::thread;
use tauri::Emitter;

use packet_sniffer::{parse_packet, PacketCapture, PacketInfo};
use wifi_scanner::{scan_wifi_internal, WiFiNetwork};

mod packet_sniffer;
mod radiotap;
mod wifi_scanner;

#[tauri::command]
async fn scan_wifi(window: tauri::Window) -> Result<Vec<WiFiNetwork>, String> {
    info!("Scanning WiFi networks");

    match scan_wifi_internal("wlxa86e84531e13") {
        Ok((stop_tx, progress_rx)) => {
            let mut final_networks = Vec::new();
            let timeout = std::time::Duration::from_secs(10);
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match progress_rx.try_recv() {
                    Ok(progress) => {
                        // Emit progress through window
                        if let Err(e) = window.emit("wifi_scan_progress", &progress.networks) {
                            warn!("Failed to emit progress: {}", e);
                        }

                        if progress.is_complete {
                            final_networks = progress.networks;
                            break;
                        }

                        final_networks = progress.networks;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        warn!("Channel error: {}", e);
                        break;
                    }
                }
            }

            // Stop the scanner
            let _ = stop_tx.send(());

            info!(
                "WiFi scan completed successfully, found {} networks",
                final_networks.len()
            );
            Ok(final_networks)
        }
        Err(e) => {
            error!("Failed to scan networks: {:?}", e);
            Err(format!("Failed to scan networks: {:?}", e))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChannelData {
    channel: u32,
    occupancy: f32,
}

#[tauri::command]
async fn get_channel_data(networks: Vec<WiFiNetwork>) -> Result<Vec<ChannelData>, String> {
    debug!("Calculating channel data for {} networks", networks.len());
    let mut channel_count: HashMap<u32, u32> = HashMap::new();
    let mut channel_signal: HashMap<u32, u32> = HashMap::new();

    // Initialize data for all 13 channels
    for channel in 1..=13 {
        channel_count.insert(channel, 0);
        channel_signal.insert(channel, 0);
    }

    // Process network data
    for network in &networks {
        if network.channel >= 1 && network.channel <= 13 {
            *channel_count.entry(network.channel).or_insert(0) += 1;
            *channel_signal.entry(network.channel).or_insert(0) += network.signal_quality;
        }
    }

    let total_networks = networks.len() as f32;
    let mut channel_data: Vec<ChannelData> = Vec::new();

    // Calculate occupancy for all channels
    for channel in 1..=13 {
        let count = *channel_count.get(&channel).unwrap_or(&0);
        let signal = *channel_signal.get(&channel).unwrap_or(&0);
        let avg_signal = if count > 0 {
            signal as f32 / count as f32
        } else {
            0.0
        };
        let occupancy = if total_networks > 0.0 {
            (count as f32 / total_networks) * (avg_signal / 100.0)
        } else {
            0.0
        };

        channel_data.push(ChannelData { channel, occupancy });
    }

    info!("Channel data calculation completed for all 13 channels");
    Ok(channel_data)
}

#[tauri::command]
fn list_devices() -> Result<Vec<String>, String> {
    info!("Listing network devices");
    match Device::list() {
        Ok(devices) => {
            let device_names: Vec<String> = devices.into_iter().map(|d| d.name).collect();
            info!("Found {} devices", device_names.len());
            Ok(device_names)
        }
        Err(e) => {
            error!("Failed to list devices: {:?}", e);
            Err(format!("Failed to list devices: {:?}", e))
        }
    }
}

#[tauri::command]
fn get_latest_packets(state: tauri::State<PacketCapture>) -> Result<Vec<PacketInfo>, String> {
    let captured_packets = state.captured_packets.lock().unwrap();
    let mut last_fetch_timestamp = state.last_fetch_timestamp.lock().unwrap();

    let new_packets: Vec<PacketInfo> = captured_packets
        .iter()
        .filter(|packet| packet.timestamp > *last_fetch_timestamp)
        .cloned()
        .collect();

    if let Some(latest_packet) = new_packets.last() {
        *last_fetch_timestamp = latest_packet.timestamp;
    }

    Ok(new_packets)
}

#[tauri::command]
async fn start_packet_capture(
    device_name: String,
    state: tauri::State<'_, PacketCapture>,
    window: tauri::Window,
) -> Result<(), String> {
    info!("Starting packet capture on device: {}", device_name);

    *state.running.lock().unwrap() = true;
    *state.device.lock().unwrap() = Some(device_name.clone());

    // Clone Arc for state and window to move into the thread
    let running = Arc::clone(&state.running);
    let captured_packets = Arc::clone(&state.captured_packets);
    let window = window.clone();

    thread::spawn(move || {
        let mut cap = match Capture::from_device(device_name.as_str())
            .unwrap()
            .immediate_mode(true)
            .open()
        {
            Ok(cap) => cap,
            Err(e) => {
                error!("Error opening device: {:?}", e);
                return;
            }
        };

        info!("Packet capture started successfully");

        while *running.lock().unwrap() {
            let cap = &mut cap;
            match cap.next_packet() {
                Ok(packet) => {
                    if let Ok(packet_info) = parse_packet(&packet) {
                        let cloned_packet_info = packet_info.clone();
                        let mut packets = captured_packets.lock().unwrap();
                        packets.push(cloned_packet_info);

                        // Emit the event
                        if let Err(err) = window.emit("packet", packet_info) {
                            warn!("Error emitting packet event: {}", err);
                        }
                    }
                }
                Err(e) => error!("Error receiving packet: {:?}", e),
            }
        }
    });

    Ok(())
}

#[tauri::command]
fn stop_packet_capture(state: tauri::State<PacketCapture>) -> Result<(), String> {
    info!("Stopping packet capture");
    *state.running.lock().unwrap() = false;
    *state.device.lock().unwrap() = None;
    info!("Packet capture stopped");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(PacketCapture::new())
        .invoke_handler(tauri::generate_handler![
            scan_wifi,
            list_devices,
            start_packet_capture,
            stop_packet_capture,
            get_channel_data,
            get_latest_packets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
