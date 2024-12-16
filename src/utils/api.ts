import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface WiFiNetwork {
  ssid: string;
  bssid: string;
  signal_quality: number;
  frequency: number;
  channel: number;
  security: string;
  avg_signal: number;
  beacon_count: number;
  last_seen: number;
}

export interface PacketInfo {
  src_mac: string;
  dst_mac: string;
  src_ip: string | null;
  dst_ip: string | null;
  src_port: number | null;
  dst_port: number | null;
  protocol: string;
  length: number;
  payload: string | null;
  timestamp: number;
}

export interface ChannelData {
  channel: number;
  occupancy: number;
}

export async function scanWifi(): Promise<WiFiNetwork[]> {
  try {
    console.log("Starting WiFi scan...");
    const networks = await invoke<WiFiNetwork[]>("scan_wifi");
    console.log("Scan completed, found networks:", networks);
    return networks;
  } catch (error) {
    console.error("Failed to scan Wi-Fi networks:", error);
    throw error;
  }
}

export async function getChannelData(
  networks: WiFiNetwork[],
): Promise<ChannelData[]> {
  try {
    const channelData = await invoke<ChannelData[]>("get_channel_data", {
      networks,
    });
    return channelData;
  } catch (error) {
    console.error("Failed to get channel data:", error);
    throw error;
  }
}

export async function listDevices(): Promise<string[]> {
  try {
    const devices = await invoke<string[]>("list_devices");
    return devices;
  } catch (error) {
    console.error("Failed to list network devices:", error);
    throw error;
  }
}

export async function startPacketCapture(deviceName: string): Promise<void> {
  try {
    await invoke("start_packet_capture", { deviceName });
  } catch (error) {
    console.error("Failed to start packet capture:", error);
    throw error;
  }
}

export async function stopPacketCapture(): Promise<void> {
  try {
    await invoke("stop_packet_capture");
  } catch (error) {
    console.error("Failed to stop packet capture:", error);
    throw error;
  }
}

export async function listenForPackets(
  callback: (packet: PacketInfo) => void,
): Promise<void> {
  await listen<PacketInfo>("packet", (event) => {
    callback(event.payload);
  });
}

export async function getLatestPackets(): Promise<PacketInfo[]> {
  try {
    const latestPackets = await invoke<PacketInfo[]>("get_latest_packets");
    return latestPackets;
  } catch (error) {
    console.error("Failed to get latest packets:", error);
    throw error;
  }
}
