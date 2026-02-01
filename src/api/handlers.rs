//! API endpoint handlers for Anker PowerHouse 767.

use crate::ble::command::{
    AcOutputCommand, AcTimerCommand, LedCommand, PowerSaveCommand, RechargePowerCommand,
    ScreenBrightnessCommand, ScreenTimeoutCommand, TwelveVoltOutputCommand, TwelveVoltTimerCommand,
};
use crate::ble::{send_command, AnkerCommand, ConnectionState, DeviceState, SetState, Telemetry};
use crate::metrics;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

pub type AppState = Arc<RwLock<DeviceState>>;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub error: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiSuccess {
    pub success: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatusResponse {
    pub connected: bool,
    pub state: String,
}

// Request types

#[derive(Debug, Deserialize, ToSchema)]
pub struct BoolRequest {
    pub is_on: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BrightnessRequest {
    /// Brightness level (0-3)
    pub level: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LedRequest {
    /// LED level (0-4, where 4 is SOS mode)
    pub level: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WattsRequest {
    /// Recharge power in watts (200-1440)
    pub watts: u16,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SecondsRequest {
    /// Timeout/timer in seconds (0-65535)
    pub seconds: u16,
}

// Handler implementations

/// Get current connection status
#[utoipa::path(
    get,
    path = "/api/status",
    responses(
        (status = 200, description = "Connection status", body = StatusResponse)
    ),
    tag = "status"
)]
pub async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let state = state.read().await;
    let state_str = match state.connection_state {
        ConnectionState::Disconnected => "disconnected",
        ConnectionState::Scanning => "scanning",
        ConnectionState::Connecting => "connecting",
        ConnectionState::Connected => "connected",
    };

    Json(StatusResponse {
        connected: state.connection_state == ConnectionState::Connected,
        state: state_str.to_string(),
    })
}

/// Get current telemetry data
#[utoipa::path(
    get,
    path = "/api/telemetry",
    responses(
        (status = 200, description = "Current telemetry", body = Telemetry),
        (status = 503, description = "No telemetry available", body = ApiError)
    ),
    tag = "telemetry"
)]
pub async fn get_telemetry(
    State(state): State<AppState>,
) -> Result<Json<Telemetry>, (StatusCode, Json<ApiError>)> {
    let state = state.read().await;

    state
        .last_telemetry
        .clone()
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiError {
                    error: "No telemetry available".to_string(),
                }),
            )
        })
}

/// Get current device state (last set values)
#[utoipa::path(
    get,
    path = "/api/device-state",
    responses(
        (status = 200, description = "Current device state", body = SetState)
    ),
    tag = "telemetry"
)]
pub async fn get_device_state(State(state): State<AppState>) -> Json<SetState> {
    let state = state.read().await;
    Json(state.set_state.clone())
}

/// Toggle power save mode
#[utoipa::path(
    post,
    path = "/api/power-save",
    request_body = BoolRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_power_save(
    State(state): State<AppState>,
    Json(req): Json<BoolRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let cmd = AnkerCommand::PowerSave(PowerSaveCommand::new(req.is_on));
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.power_save = Some(req.is_on);
    Ok(result)
}

/// Toggle AC output
#[utoipa::path(
    post,
    path = "/api/ac-output",
    request_body = BoolRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_ac_output(
    State(state): State<AppState>,
    Json(req): Json<BoolRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let cmd = AnkerCommand::AcOutput(AcOutputCommand::new(req.is_on));
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.ac_output = Some(req.is_on);
    Ok(result)
}

/// Toggle 12V output
#[utoipa::path(
    post,
    path = "/api/twelve-volt-output",
    request_body = BoolRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_twelve_volt_output(
    State(state): State<AppState>,
    Json(req): Json<BoolRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let cmd = AnkerCommand::TwelveVoltOutput(TwelveVoltOutputCommand::new(req.is_on));
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.twelve_volt_output = Some(req.is_on);
    Ok(result)
}

/// Set screen brightness
#[utoipa::path(
    post,
    path = "/api/screen-brightness",
    request_body = BrightnessRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 400, description = "Invalid brightness level", body = ApiError),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_screen_brightness(
    State(state): State<AppState>,
    Json(req): Json<BrightnessRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let inner = ScreenBrightnessCommand::new(req.level).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;
    let cmd = AnkerCommand::ScreenBrightness(inner);
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.screen_brightness = Some(req.level);
    Ok(result)
}

/// Set LED level
#[utoipa::path(
    post,
    path = "/api/led",
    request_body = LedRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 400, description = "Invalid LED level", body = ApiError),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_led(
    State(state): State<AppState>,
    Json(req): Json<LedRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let inner = LedCommand::new(req.level).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;
    let cmd = AnkerCommand::Led(inner);
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.led_level = Some(req.level);
    Ok(result)
}

/// Set recharge power
#[utoipa::path(
    post,
    path = "/api/recharge-power",
    request_body = WattsRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 400, description = "Invalid wattage", body = ApiError),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_recharge_power(
    State(state): State<AppState>,
    Json(req): Json<WattsRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let inner = RechargePowerCommand::new(req.watts).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;
    let cmd = AnkerCommand::RechargePower(inner);
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.recharge_power = Some(req.watts);
    Ok(result)
}

/// Set screen timeout
#[utoipa::path(
    post,
    path = "/api/screen-timeout",
    request_body = SecondsRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_screen_timeout(
    State(state): State<AppState>,
    Json(req): Json<SecondsRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let cmd = AnkerCommand::ScreenTimeout(ScreenTimeoutCommand::new(req.seconds));
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.screen_timeout = Some(req.seconds);
    Ok(result)
}

/// Set AC timer
#[utoipa::path(
    post,
    path = "/api/ac-timer",
    request_body = SecondsRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_ac_timer(
    State(state): State<AppState>,
    Json(req): Json<SecondsRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let cmd = AnkerCommand::AcTimer(AcTimerCommand::new(req.seconds));
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.ac_timer = Some(req.seconds);
    Ok(result)
}

/// Set 12V timer
#[utoipa::path(
    post,
    path = "/api/twelve-volt-timer",
    request_body = SecondsRequest,
    responses(
        (status = 200, description = "Command sent", body = ApiSuccess),
        (status = 503, description = "Not connected", body = ApiError)
    ),
    tag = "commands"
)]
pub async fn set_twelve_volt_timer(
    State(state): State<AppState>,
    Json(req): Json<SecondsRequest>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let cmd = AnkerCommand::TwelveVoltTimer(TwelveVoltTimerCommand::new(req.seconds));
    let result = send_and_track(cmd).await?;
    state.write().await.set_state.twelve_volt_timer = Some(req.seconds);
    Ok(result)
}

/// Prometheus metrics endpoint
pub async fn get_metrics() -> impl IntoResponse {
    metrics::render()
}

async fn send_and_track(
    cmd: AnkerCommand,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let cmd_type = cmd.command_type().as_str().to_string();

    send_command(cmd).await.map_err(|e| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    metrics::increment_command(&cmd_type);
    Ok(Json(ApiSuccess { success: true }))
}
