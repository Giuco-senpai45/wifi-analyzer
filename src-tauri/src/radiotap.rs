use byteorder::{ByteOrder, LittleEndian};
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RadiotapData {
    pub version: u8,
    pub pad: u8,
    pub length: u16,
    pub present_flags: u32,
    pub mac_timestamp: Option<u64>,
    pub flags: Option<u8>,
    pub rate: Option<u8>,
    pub channel_freq: Option<u16>,
    pub channel_flags: Option<u16>,
    pub antenna_signal: Option<i8>,
    pub antenna: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WiFiFrame {
    pub radiotap: RadiotapData,
    pub frame_control: u16,
    pub duration: u16,
    pub addr1: [u8; 6],
    pub addr2: [u8; 6],
    pub addr3: [u8; 6],
    pub seq_ctrl: u16,
    pub ssid: Option<String>,
    pub channel: Option<u8>,
    pub rates: Vec<u8>,
}

#[repr(u32)]
#[derive(Debug)]
pub enum RadiotapPresent {
    TSFT = 1 << 0,
    Flags = 1 << 1,
    Rate = 1 << 2,
    Channel = 1 << 3,
    FHSS = 1 << 4,
    AntennaSignal = 1 << 5,
    AntennaNoise = 1 << 6,
    LockQuality = 1 << 7,
    TxAttenuation = 1 << 8,
    DbTxAttenuation = 1 << 9,
    DbmTxPower = 1 << 10,
    Antenna = 1 << 11,
    DbAntennaSignal = 1 << 12,
    DbAntennaNoise = 1 << 13,
    RxFlags = 1 << 14,
}

pub struct RadiotapParser<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> RadiotapParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn parse_radiotap_header(&mut self) -> Result<RadiotapData, String> {
        if self.data.len() < 8 {
            return Err("Buffer too small for radiotap header".to_string());
        }

        debug!(
            "First 8 bytes of packet: {:02X?}",
            &self.data[..8.min(self.data.len())]
        );

        let version = self.data[0];
        debug!("Radiotap version byte: {:02X}", version);
        if version != 0 {
            return Err(format!(
                "Unsupported radiotap version: {} (hex: {:02X}). First 8 bytes: {:02X?}",
                version,
                version,
                &self.data[..8.min(self.data.len())]
            ));
        }

        let pad = self.data[1];
        let length = LittleEndian::read_u16(&self.data[2..4]);
        let present_flags = LittleEndian::read_u32(&self.data[4..8]);
        debug!(
            "Radiotap header: version={}, pad={}, length={}, present_flags={:08X}",
            version, pad, length, present_flags
        );

        self.offset = 8;
        let mut radiotap = RadiotapData {
            version,
            pad,
            length,
            present_flags,
            mac_timestamp: None,
            flags: None,
            rate: None,
            channel_freq: None,
            channel_flags: None,
            antenna_signal: None,
            antenna: None,
        };

        // Parse present flags with safe error handling
        if present_flags & (RadiotapPresent::TSFT as u32) != 0 {
            radiotap.mac_timestamp = self.read_u64().ok();
        }
        if present_flags & (RadiotapPresent::Flags as u32) != 0 {
            radiotap.flags = self.read_u8().ok();
        }
        if present_flags & (RadiotapPresent::Rate as u32) != 0 {
            radiotap.rate = self.read_u8().ok();
        }
        if present_flags & (RadiotapPresent::Channel as u32) != 0 {
            radiotap.channel_freq = self.read_u16().ok();
            radiotap.channel_flags = self.read_u16().ok();
        }
        if present_flags & (RadiotapPresent::AntennaSignal as u32) != 0 {
            radiotap.antenna_signal = self.read_i8().ok();
        }
        if present_flags & (RadiotapPresent::Antenna as u32) != 0 {
            radiotap.antenna = self.read_u8().ok();
        }

        Ok(radiotap)
    }

    pub fn parse_wifi_frame(&mut self) -> Result<WiFiFrame, String> {
        let radiotap = self.parse_radiotap_header()?;

        // Move offset to start of 802.11 frame
        self.offset = radiotap.length as usize;
        if self.offset >= self.data.len() {
            return Err("Invalid radiotap length".to_string());
        }

        // Parse 802.11 frame header with safe error handling
        let frame_control = self
            .read_u16()
            .map_err(|e| format!("Failed to read frame control: {}", e))?;
        let duration = self
            .read_u16()
            .map_err(|e| format!("Failed to read duration: {}", e))?;

        // Safe address reading
        let addr1 = self
            .read_mac_address()
            .map_err(|e| format!("Failed to read addr1: {}", e))?;
        let addr2 = self
            .read_mac_address()
            .map_err(|e| format!("Failed to read addr2: {}", e))?;
        let addr3 = self
            .read_mac_address()
            .map_err(|e| format!("Failed to read addr3: {}", e))?;

        let seq_ctrl = self
            .read_u16()
            .map_err(|e| format!("Failed to read sequence control: {}", e))?;

        let frame_type = (frame_control & 0x000C) >> 2;
        let frame_subtype = (frame_control & 0x00F0) >> 4;

        let mut ssid = None;
        let mut channel = None;
        let mut rates = Vec::new();

        if frame_type == 0 && (frame_subtype == 8 || frame_subtype == 5) {
            // Skip fixed parameters safely
            if self.offset + 12 <= self.data.len() {
                self.offset += 12;

                // Parse tagged parameters
                while self.offset + 2 <= self.data.len() {
                    let tag_number = match self.read_u8() {
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    let tag_length = match self.read_u8() {
                        Ok(n) => n as usize,
                        Err(_) => break,
                    };

                    if self.offset + tag_length > self.data.len() {
                        break;
                    }

                    match tag_number {
                        0 => {
                            // SSID
                            if tag_length > 0 {
                                ssid = String::from_utf8_lossy(
                                    &self.data[self.offset..self.offset + tag_length],
                                )
                                .to_string()
                                .into();
                            }
                        }
                        3 => {
                            // Channel
                            if tag_length > 0 {
                                channel = Some(self.data[self.offset]);
                            }
                        }
                        1 | 50 => {
                            // Supported rates
                            rates.extend_from_slice(
                                &self.data[self.offset..self.offset + tag_length],
                            );
                        }
                        _ => {}
                    }

                    self.offset += tag_length;
                }
            }
        }

        Ok(WiFiFrame {
            radiotap,
            frame_control,
            duration,
            addr1,
            addr2,
            addr3,
            seq_ctrl,
            ssid,
            channel,
            rates,
        })
    }

    fn read_mac_address(&mut self) -> Result<[u8; 6], String> {
        if self.offset + 6 > self.data.len() {
            return Err("Buffer overflow reading MAC address".to_string());
        }
        let mut addr = [0u8; 6];
        addr.copy_from_slice(&self.data[self.offset..self.offset + 6]);
        self.offset += 6;
        Ok(addr)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.offset >= self.data.len() {
            return Err("Buffer overflow reading u8".to_string());
        }
        let value = self.data[self.offset];
        self.offset += 1;
        Ok(value)
    }

    fn read_i8(&mut self) -> Result<i8, String> {
        self.read_u8().map(|v| v as i8)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        if self.offset + 2 > self.data.len() {
            return Err("Buffer overflow reading u16".to_string());
        }
        let value = LittleEndian::read_u16(&self.data[self.offset..]);
        self.offset += 2;
        Ok(value)
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        if self.offset + 8 > self.data.len() {
            return Err("Buffer overflow reading u64".to_string());
        }
        let value = LittleEndian::read_u64(&self.data[self.offset..]);
        self.offset += 8;
        Ok(value)
    }
}
