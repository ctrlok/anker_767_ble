//! Command types and serialization for Anker PowerHouse 767 (F2000).

use thiserror::Error;

/// Header bytes for all commands
const HEADER: [u8; 6] = [0x08, 0xee, 0x00, 0x00, 0x00, 0x02];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandType {
    AcTimer = 0x02,
    TwelveVoltTimer = 0x03,
    RechargePower = 0x80,
    ScreenTimeout = 0x82,
    AcOutput = 0x86,
    TwelveVoltOutput = 0x87,
    ScreenBrightness = 0x88,
    PowerSave = 0x8A,
    Led = 0x8B,
}

impl CommandType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandType::AcTimer => "ac_timer",
            CommandType::TwelveVoltTimer => "twelve_volt_timer",
            CommandType::RechargePower => "recharge_power",
            CommandType::ScreenTimeout => "screen_timeout",
            CommandType::AcOutput => "ac_output",
            CommandType::TwelveVoltOutput => "twelve_volt_output",
            CommandType::ScreenBrightness => "screen_brightness",
            CommandType::PowerSave => "power_save",
            CommandType::Led => "led",
        }
    }
}

impl TryFrom<u8> for CommandType {
    type Error = CommandError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x02 => Ok(CommandType::AcTimer),
            0x03 => Ok(CommandType::TwelveVoltTimer),
            0x80 => Ok(CommandType::RechargePower),
            0x82 => Ok(CommandType::ScreenTimeout),
            0x86 => Ok(CommandType::AcOutput),
            0x87 => Ok(CommandType::TwelveVoltOutput),
            0x88 => Ok(CommandType::ScreenBrightness),
            0x8A => Ok(CommandType::PowerSave),
            0x8B => Ok(CommandType::Led),
            _ => Err(CommandError::UnknownCommandType(value)),
        }
    }
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Unknown command type: 0x{0:02x}")]
    UnknownCommandType(u8),
    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

/// Trait for commands that can be serialized to bytes
pub trait Command {
    fn command_type(&self) -> CommandType;
    fn length(&self) -> u8;
    fn parameters(&self) -> Vec<u8>;

    fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(16);
        output.extend_from_slice(&HEADER);
        output.push(self.command_type() as u8);
        output.push(self.length());
        output.extend_from_slice(&self.parameters());

        let checksum = output.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        output.push(checksum);

        output
    }
}

/// Turns Power Save mode on and off
#[derive(Debug, Clone)]
pub struct PowerSaveCommand {
    pub is_on: bool,
}

impl PowerSaveCommand {
    pub fn new(is_on: bool) -> Self {
        Self { is_on }
    }
}

impl Command for PowerSaveCommand {
    fn command_type(&self) -> CommandType {
        CommandType::PowerSave
    }

    fn length(&self) -> u8 {
        11
    }

    fn parameters(&self) -> Vec<u8> {
        vec![0x00, self.is_on as u8]
    }
}

/// Turns the AC output on and off
#[derive(Debug, Clone)]
pub struct AcOutputCommand {
    pub is_on: bool,
}

impl AcOutputCommand {
    pub fn new(is_on: bool) -> Self {
        Self { is_on }
    }
}

impl Command for AcOutputCommand {
    fn command_type(&self) -> CommandType {
        CommandType::AcOutput
    }

    fn length(&self) -> u8 {
        11
    }

    fn parameters(&self) -> Vec<u8> {
        vec![0x00, self.is_on as u8]
    }
}

/// Turns the 12V output on and off
#[derive(Debug, Clone)]
pub struct TwelveVoltOutputCommand {
    pub is_on: bool,
}

impl TwelveVoltOutputCommand {
    pub fn new(is_on: bool) -> Self {
        Self { is_on }
    }
}

impl Command for TwelveVoltOutputCommand {
    fn command_type(&self) -> CommandType {
        CommandType::TwelveVoltOutput
    }

    fn length(&self) -> u8 {
        11
    }

    fn parameters(&self) -> Vec<u8> {
        vec![0x00, self.is_on as u8]
    }
}

/// Sets the display screen brightness (0-3)
#[derive(Debug, Clone)]
pub struct ScreenBrightnessCommand {
    pub brightness: u8,
}

impl ScreenBrightnessCommand {
    pub fn new(brightness: u8) -> Result<Self, CommandError> {
        if brightness > 3 {
            return Err(CommandError::InvalidValue(format!(
                "brightness must be 0-3, got {}",
                brightness
            )));
        }
        Ok(Self { brightness })
    }
}

impl Command for ScreenBrightnessCommand {
    fn command_type(&self) -> CommandType {
        CommandType::ScreenBrightness
    }

    fn length(&self) -> u8 {
        11
    }

    fn parameters(&self) -> Vec<u8> {
        vec![0x00, self.brightness]
    }
}

/// Sets the LED strip level (0-4, where 4 is SOS mode)
#[derive(Debug, Clone)]
pub struct LedCommand {
    pub level: u8,
}

impl LedCommand {
    pub fn new(level: u8) -> Result<Self, CommandError> {
        if level > 4 {
            return Err(CommandError::InvalidValue(format!(
                "LED level must be 0-4, got {}",
                level
            )));
        }
        Ok(Self { level })
    }
}

impl Command for LedCommand {
    fn command_type(&self) -> CommandType {
        CommandType::Led
    }

    fn length(&self) -> u8 {
        11
    }

    fn parameters(&self) -> Vec<u8> {
        vec![0x00, self.level]
    }
}

/// Sets the recharge power in watts (200-1440)
#[derive(Debug, Clone)]
pub struct RechargePowerCommand {
    pub watts: u16,
}

impl RechargePowerCommand {
    pub fn new(watts: u16) -> Result<Self, CommandError> {
        if !(200..=1440).contains(&watts) {
            return Err(CommandError::InvalidValue(format!(
                "recharge power must be 200-1440, got {}",
                watts
            )));
        }
        Ok(Self { watts })
    }
}

impl Command for RechargePowerCommand {
    fn command_type(&self) -> CommandType {
        CommandType::RechargePower
    }

    fn length(&self) -> u8 {
        12
    }

    fn parameters(&self) -> Vec<u8> {
        let bytes = self.watts.to_le_bytes();
        vec![0x00, bytes[0], bytes[1]]
    }
}

/// Sets the screen timeout in seconds (0-65535)
#[derive(Debug, Clone)]
pub struct ScreenTimeoutCommand {
    pub seconds: u16,
}

impl ScreenTimeoutCommand {
    pub fn new(seconds: u16) -> Self {
        Self { seconds }
    }
}

impl Command for ScreenTimeoutCommand {
    fn command_type(&self) -> CommandType {
        CommandType::ScreenTimeout
    }

    fn length(&self) -> u8 {
        12
    }

    fn parameters(&self) -> Vec<u8> {
        let bytes = self.seconds.to_le_bytes();
        vec![0x00, bytes[0], bytes[1]]
    }
}

/// Sets a timer for AC output auto-off (0-65535 seconds, 0 disables)
#[derive(Debug, Clone)]
pub struct AcTimerCommand {
    pub seconds: u16,
}

impl AcTimerCommand {
    pub fn new(seconds: u16) -> Self {
        Self { seconds }
    }
}

impl Command for AcTimerCommand {
    fn command_type(&self) -> CommandType {
        CommandType::AcTimer
    }

    fn length(&self) -> u8 {
        14
    }

    fn parameters(&self) -> Vec<u8> {
        let bytes = self.seconds.to_le_bytes();
        vec![0x00, bytes[0], bytes[1], 0x00, 0x00]
    }
}

/// Sets a timer for 12V output auto-off (0-65535 seconds, 0 disables)
#[derive(Debug, Clone)]
pub struct TwelveVoltTimerCommand {
    pub seconds: u16,
}

impl TwelveVoltTimerCommand {
    pub fn new(seconds: u16) -> Self {
        Self { seconds }
    }
}

impl Command for TwelveVoltTimerCommand {
    fn command_type(&self) -> CommandType {
        CommandType::TwelveVoltTimer
    }

    fn length(&self) -> u8 {
        14
    }

    fn parameters(&self) -> Vec<u8> {
        let bytes = self.seconds.to_le_bytes();
        vec![0x00, bytes[0], bytes[1], 0x00, 0x00]
    }
}

/// Enum wrapping all command types for dynamic dispatch
#[derive(Debug, Clone)]
pub enum AnkerCommand {
    PowerSave(PowerSaveCommand),
    AcOutput(AcOutputCommand),
    TwelveVoltOutput(TwelveVoltOutputCommand),
    ScreenBrightness(ScreenBrightnessCommand),
    Led(LedCommand),
    RechargePower(RechargePowerCommand),
    ScreenTimeout(ScreenTimeoutCommand),
    AcTimer(AcTimerCommand),
    TwelveVoltTimer(TwelveVoltTimerCommand),
}

impl AnkerCommand {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            AnkerCommand::PowerSave(cmd) => cmd.to_bytes(),
            AnkerCommand::AcOutput(cmd) => cmd.to_bytes(),
            AnkerCommand::TwelveVoltOutput(cmd) => cmd.to_bytes(),
            AnkerCommand::ScreenBrightness(cmd) => cmd.to_bytes(),
            AnkerCommand::Led(cmd) => cmd.to_bytes(),
            AnkerCommand::RechargePower(cmd) => cmd.to_bytes(),
            AnkerCommand::ScreenTimeout(cmd) => cmd.to_bytes(),
            AnkerCommand::AcTimer(cmd) => cmd.to_bytes(),
            AnkerCommand::TwelveVoltTimer(cmd) => cmd.to_bytes(),
        }
    }

    pub fn command_type(&self) -> CommandType {
        match self {
            AnkerCommand::PowerSave(_) => CommandType::PowerSave,
            AnkerCommand::AcOutput(_) => CommandType::AcOutput,
            AnkerCommand::TwelveVoltOutput(_) => CommandType::TwelveVoltOutput,
            AnkerCommand::ScreenBrightness(_) => CommandType::ScreenBrightness,
            AnkerCommand::Led(_) => CommandType::Led,
            AnkerCommand::RechargePower(_) => CommandType::RechargePower,
            AnkerCommand::ScreenTimeout(_) => CommandType::ScreenTimeout,
            AnkerCommand::AcTimer(_) => CommandType::AcTimer,
            AnkerCommand::TwelveVoltTimer(_) => CommandType::TwelveVoltTimer,
        }
    }
}
