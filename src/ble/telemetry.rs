//! Telemetry parsing for Anker PowerHouse 767 (F2000).

use crate::ble::command::CommandType;
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

const EXPECTED_PACKET_LENGTH: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BatteryState {
    Idle,
    Discharging,
    Charging,
}

impl TryFrom<u8> for BatteryState {
    type Error = TelemetryError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BatteryState::Idle),
            1 => Ok(BatteryState::Discharging),
            2 => Ok(BatteryState::Charging),
            _ => Err(TelemetryError::UnknownBatteryState(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LedState {
    Off,
    Low,
    Mid,
    High,
    Sos,
}

impl TryFrom<u8> for LedState {
    type Error = TelemetryError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(LedState::Off),
            1 => Ok(LedState::Low),
            2 => Ok(LedState::Mid),
            3 => Ok(LedState::High),
            4 => Ok(LedState::Sos),
            _ => Err(TelemetryError::UnknownLedState(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Telemetry = 1,
    CommandAck = 2,
}

impl TryFrom<u8> for PacketType {
    type Error = TelemetryError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(PacketType::Telemetry),
            2 => Ok(PacketType::CommandAck),
            _ => Err(TelemetryError::UnknownPacketType(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TelemetryType {
    StateAck = 0x48,
    Telemetry = 0x49,
}

impl TryFrom<u8> for TelemetryType {
    type Error = TelemetryError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x48 => Ok(TelemetryType::StateAck),
            0x49 => Ok(TelemetryType::Telemetry),
            _ => Err(TelemetryError::UnknownTelemetryType(value)),
        }
    }
}

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("Data too short: expected at least {expected}, got {actual}")]
    DataTooShort { expected: usize, actual: usize },
    #[error("Unknown packet type: {0}")]
    UnknownPacketType(u8),
    #[error("Unknown telemetry type: 0x{0:02x}")]
    UnknownTelemetryType(u8),
    #[error("Unknown battery state: {0}")]
    UnknownBatteryState(u8),
    #[error("Unknown LED state: {0}")]
    UnknownLedState(u8),
    #[error("Invalid UTF-8 in serial: {0}")]
    InvalidSerial(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, Clone)]
pub struct Header {
    pub packet_type: PacketType,
    pub telemetry_id: u8,
    pub packet_length: u16,
}

impl Header {
    pub fn from_bytes(data: &[u8]) -> Result<Self, TelemetryError> {
        if data.len() < EXPECTED_PACKET_LENGTH {
            return Err(TelemetryError::DataTooShort {
                expected: EXPECTED_PACKET_LENGTH,
                actual: data.len(),
            });
        }

        let packet_id = data[5];
        let telemetry_id = data[6];
        let packet_length = u16::from_le_bytes([data[7], data[8]]);
        let packet_type = PacketType::try_from(packet_id)?;

        Ok(Header {
            packet_type,
            telemetry_id,
            packet_length,
        })
    }
}

/// Extract a 16-bit little-endian integer from data
fn extract16(data: &[u8], index: usize) -> u16 {
    u16::from_le_bytes([data[index], data[index + 1]])
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Output {
    pub is_on: bool,
    pub watts: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_remaining_seconds: Option<u16>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Battery {
    pub temperature: u8,
    pub percentage: u8,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Telemetry {
    pub battery_remaining_hours: f32,
    pub ac_outlet: Output,
    pub twelve_volt: Vec<Output>,
    pub usb_c: Vec<Output>,
    pub usb_a: Vec<Output>,
    pub total_output_watts: u16,
    pub ac_input_watts: u16,
    pub solar_input_watts: u16,
    pub total_input_watts: u16,
    pub internal_battery: Battery,
    pub external_battery: Battery,
    pub battery_state: BatteryState,
    pub total_battery_percentage: u8,
    pub device_serial: String,
}

impl Telemetry {
    pub fn from_bytes(data: &[u8]) -> Result<Self, TelemetryError> {
        if data.len() < 102 {
            return Err(TelemetryError::DataTooShort {
                expected: 102,
                actual: data.len(),
            });
        }

        let battery_remaining_hours = data[18] as f32 * 24.0 + data[17] as f32 / 10.0;

        let ac_outlet = Output {
            is_on: data[63] != 0,
            watts: extract16(data, 21),
            time_remaining_seconds: None,
        };

        let twelve_volt_time = extract16(data, 13);
        let twelve_volt = vec![
            Output {
                is_on: data[80] != 0,
                watts: extract16(data, 33),
                time_remaining_seconds: Some(twelve_volt_time),
            },
            Output {
                is_on: data[81] != 0,
                watts: extract16(data, 35),
                time_remaining_seconds: Some(twelve_volt_time),
            },
        ];

        let usb_c = vec![
            Output {
                is_on: data[75] != 0,
                watts: extract16(data, 23),
                time_remaining_seconds: None,
            },
            Output {
                is_on: data[76] != 0,
                watts: extract16(data, 25),
                time_remaining_seconds: None,
            },
            Output {
                is_on: data[77] != 0,
                watts: extract16(data, 27),
                time_remaining_seconds: None,
            },
        ];

        let usb_a = vec![
            Output {
                is_on: data[78] != 0,
                watts: extract16(data, 29),
                time_remaining_seconds: None,
            },
            Output {
                is_on: data[79] != 0,
                watts: extract16(data, 31),
                time_remaining_seconds: None,
            },
        ];

        let internal_battery = Battery {
            temperature: data[66],
            percentage: data[70],
        };

        let external_battery = Battery {
            temperature: data[67],
            percentage: data[71],
        };

        let device_serial = String::from_utf8(data[85..101].to_vec())?;

        Ok(Telemetry {
            battery_remaining_hours,
            ac_outlet,
            twelve_volt,
            usb_c,
            usb_a,
            total_output_watts: extract16(data, 41),
            ac_input_watts: extract16(data, 19),
            solar_input_watts: extract16(data, 37),
            total_input_watts: extract16(data, 39),
            internal_battery,
            external_battery,
            battery_state: BatteryState::try_from(data[68])?,
            total_battery_percentage: data[72],
            device_serial,
        })
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StateAck {
    pub ac_outlet_on: bool,
    pub twelve_volt_on: bool,
    pub power_save_on: bool,
    pub led_state: LedState,
}

impl StateAck {
    pub fn from_bytes(data: &[u8]) -> Result<Self, TelemetryError> {
        if data.len() < 13 {
            return Err(TelemetryError::DataTooShort {
                expected: 13,
                actual: data.len(),
            });
        }

        Ok(StateAck {
            ac_outlet_on: data[9] != 0,
            twelve_volt_on: data[10] != 0,
            power_save_on: data[11] != 0,
            led_state: LedState::try_from(data[12])?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommandAck {
    pub command_type: CommandType,
}

/// Parsed notification packet from the device
#[derive(Debug, Clone)]
pub enum NotificationPacket {
    Telemetry(Telemetry),
    StateAck(StateAck),
    CommandAck(CommandAck),
}

impl NotificationPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, TelemetryError> {
        let header = Header::from_bytes(data)?;

        match header.packet_type {
            PacketType::Telemetry => {
                let telemetry_type = TelemetryType::try_from(header.telemetry_id)?;
                match telemetry_type {
                    TelemetryType::Telemetry => {
                        Ok(NotificationPacket::Telemetry(Telemetry::from_bytes(data)?))
                    }
                    TelemetryType::StateAck => {
                        Ok(NotificationPacket::StateAck(StateAck::from_bytes(data)?))
                    }
                }
            }
            PacketType::CommandAck => {
                let command_type = CommandType::try_from(header.telemetry_id)
                    .map_err(|_| TelemetryError::UnknownTelemetryType(header.telemetry_id))?;
                Ok(NotificationPacket::CommandAck(CommandAck { command_type }))
            }
        }
    }
}
