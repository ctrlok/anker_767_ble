//! BLE device connection manager for Anker PowerHouse 767.
//! Maintains always-connected state with auto-reconnect.

use crate::ble::command::AnkerCommand;
use crate::ble::telemetry::{NotificationPacket, StateAck, Telemetry, TelemetryError};
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{broadcast, watch, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

const DEVICE_NAME: &str = "767_PowerHouse";
const WRITE_UUID: Uuid = Uuid::from_u128(0x00007777_0000_1000_8000_00805f9b34fb);
const NOTIFY_UUID: Uuid = Uuid::from_u128(0x00008888_0000_1000_8000_00805f9b34fb);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("BLE error: {0}")]
    Ble(#[from] btleplug::Error),
    #[error("Device not found")]
    NotFound,
    #[error("Characteristic not found: {0}")]
    CharacteristicNotFound(Uuid),
    #[error("Not connected")]
    NotConnected,
    #[error("Telemetry error: {0}")]
    Telemetry(#[from] TelemetryError),
    #[error("Write timeout")]
    WriteTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Scanning,
    Connecting,
    Connected,
}

/// Tracks the last values we've set via commands
#[derive(Debug, Clone, Default, serde::Serialize, utoipa::ToSchema)]
pub struct SetState {
    pub ac_output: Option<bool>,
    pub twelve_volt_output: Option<bool>,
    pub power_save: Option<bool>,
    pub led_level: Option<u8>,
    pub screen_brightness: Option<u8>,
    pub recharge_power: Option<u16>,
    pub screen_timeout: Option<u16>,
    pub ac_timer: Option<u16>,
    pub twelve_volt_timer: Option<u16>,
}

/// Shared state for the BLE device
pub struct DeviceState {
    pub connection_state: ConnectionState,
    pub last_telemetry: Option<Telemetry>,
    pub last_state_ack: Option<StateAck>,
    pub set_state: SetState,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            connection_state: ConnectionState::Disconnected,
            last_telemetry: None,
            last_state_ack: None,
            set_state: SetState::default(),
        }
    }
}

/// BLE device manager - maintains connection and handles commands
pub struct AnkerDevice {
    state: Arc<RwLock<DeviceState>>,
    state_tx: watch::Sender<ConnectionState>,
    telemetry_tx: broadcast::Sender<Telemetry>,
}

impl AnkerDevice {
    pub fn new() -> Self {
        let (state_tx, _) = watch::channel(ConnectionState::Disconnected);
        let (telemetry_tx, _) = broadcast::channel(16);

        Self {
            state: Arc::new(RwLock::new(DeviceState::default())),
            state_tx,
            telemetry_tx,
        }
    }

    pub fn state(&self) -> Arc<RwLock<DeviceState>> {
        Arc::clone(&self.state)
    }

    pub fn subscribe_state(&self) -> watch::Receiver<ConnectionState> {
        self.state_tx.subscribe()
    }

    pub fn subscribe_telemetry(&self) -> broadcast::Receiver<Telemetry> {
        self.telemetry_tx.subscribe()
    }

    async fn set_connection_state(&self, state: ConnectionState) {
        let mut device_state = self.state.write().await;
        device_state.connection_state = state;
        let _ = self.state_tx.send(state);
    }

    async fn update_telemetry(&self, telemetry: Telemetry) {
        let mut state = self.state.write().await;
        state.last_telemetry = Some(telemetry.clone());
        let _ = self.telemetry_tx.send(telemetry);
    }

    async fn update_state_ack(&self, state_ack: StateAck) {
        let mut state = self.state.write().await;
        state.last_state_ack = Some(state_ack);
    }

    /// Start the connection loop - runs forever, auto-reconnecting
    pub async fn run(self: Arc<Self>) -> Result<(), DeviceError> {
        loop {
            match self.connect_and_listen().await {
                Ok(()) => {
                    info!("Connection closed normally, reconnecting...");
                }
                Err(e) => {
                    error!("Connection error: {}, reconnecting...", e);
                }
            }

            self.set_connection_state(ConnectionState::Disconnected).await;
            sleep(RECONNECT_DELAY).await;
        }
    }

    async fn connect_and_listen(&self) -> Result<(), DeviceError> {
        self.set_connection_state(ConnectionState::Scanning).await;

        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let adapter = adapters.into_iter().next().ok_or(DeviceError::NotFound)?;

        info!("Scanning for {} ...", DEVICE_NAME);
        adapter.start_scan(ScanFilter::default()).await?;

        let peripheral = self.find_device(&adapter).await?;
        adapter.stop_scan().await?;

        self.set_connection_state(ConnectionState::Connecting).await;
        info!("Connecting to device...");
        peripheral.connect().await?;
        peripheral.discover_services().await?;

        let write_char = self.find_characteristic(&peripheral, WRITE_UUID)?;
        let notify_char = self.find_characteristic(&peripheral, NOTIFY_UUID)?;

        peripheral.subscribe(&notify_char).await?;

        self.set_connection_state(ConnectionState::Connected).await;
        info!("Connected and subscribed to notifications");

        // Store peripheral for command sending
        let peripheral = Arc::new(peripheral);
        let write_char = Arc::new(write_char);

        // Store in state for command sending
        {
            let mut state = self.state.write().await;
            // We'll use a different approach - store the peripheral reference
            state.connection_state = ConnectionState::Connected;
        }

        CURRENT_PERIPHERAL
            .lock()
            .await
            .replace((Arc::clone(&peripheral), Arc::clone(&write_char)));

        // Listen for notifications
        let mut notification_stream = peripheral.notifications().await?;

        while let Some(data) = notification_stream.next().await {
            debug!("Received notification: {} bytes", data.value.len());

            match NotificationPacket::from_bytes(&data.value) {
                Ok(NotificationPacket::Telemetry(telemetry)) => {
                    debug!("Telemetry: battery={}%", telemetry.total_battery_percentage);
                    self.update_telemetry(telemetry).await;
                }
                Ok(NotificationPacket::StateAck(state_ack)) => {
                    debug!("State ack: {:?}", state_ack);
                    self.update_state_ack(state_ack).await;
                }
                Ok(NotificationPacket::CommandAck(cmd_ack)) => {
                    debug!("Command ack: {:?}", cmd_ack.command_type);
                }
                Err(e) => {
                    warn!("Failed to parse notification: {}", e);
                }
            }
        }

        info!("Notification stream ended");
        CURRENT_PERIPHERAL.lock().await.take();
        Ok(())
    }

    async fn find_device(&self, adapter: &Adapter) -> Result<Peripheral, DeviceError> {
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > SCAN_TIMEOUT {
                return Err(DeviceError::NotFound);
            }

            let peripherals = adapter.peripherals().await?;

            for peripheral in peripherals {
                if let Some(props) = peripheral.properties().await? {
                    if let Some(name) = props.local_name {
                        if name.contains(DEVICE_NAME) {
                            info!("Found device: {}", name);
                            return Ok(peripheral);
                        }
                    }
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    fn find_characteristic(
        &self,
        peripheral: &Peripheral,
        uuid: Uuid,
    ) -> Result<Characteristic, DeviceError> {
        for service in peripheral.services() {
            for char in &service.characteristics {
                if char.uuid == uuid {
                    return Ok(char.clone());
                }
            }
        }
        Err(DeviceError::CharacteristicNotFound(uuid))
    }
}

impl Default for AnkerDevice {
    fn default() -> Self {
        Self::new()
    }
}

// Global storage for the current peripheral (for sending commands)
use tokio::sync::Mutex;
static CURRENT_PERIPHERAL: Mutex<Option<(Arc<Peripheral>, Arc<Characteristic>)>> =
    Mutex::const_new(None);

/// Send a command to the device
pub async fn send_command(command: AnkerCommand) -> Result<(), DeviceError> {
    debug!("send_command: acquiring mutex...");
    let lock_start = std::time::Instant::now();
    let guard = CURRENT_PERIPHERAL.lock().await;
    debug!("send_command: mutex acquired in {:?}", lock_start.elapsed());

    let (peripheral, write_char) = guard.as_ref().ok_or(DeviceError::NotConnected)?;

    let bytes = command.to_bytes();
    debug!(
        "send_command: sending {:?} ({} bytes): {:02x?}",
        command.command_type(),
        bytes.len(),
        bytes
    );

    let write_start = std::time::Instant::now();
    timeout(
        WRITE_TIMEOUT,
        peripheral.write(write_char, &bytes, WriteType::WithoutResponse),
    )
    .await
    .map_err(|_| {
        error!("send_command: write timed out after {:?}", WRITE_TIMEOUT);
        DeviceError::WriteTimeout
    })?
    .map_err(DeviceError::Ble)?;

    debug!("send_command: write completed in {:?}", write_start.elapsed());
    Ok(())
}
