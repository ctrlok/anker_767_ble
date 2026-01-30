//! Anker PowerHouse 767 BLE Web Server

use anker_767_ble_webserver::api::{self, AppState};
use anker_767_ble_webserver::ble::{AnkerDevice, Telemetry};
use anker_767_ble_webserver::metrics;
use axum::routing::{get, post};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::get_status,
        api::get_telemetry,
        api::set_power_save,
        api::set_ac_output,
        api::set_twelve_volt_output,
        api::set_screen_brightness,
        api::set_led,
        api::set_recharge_power,
        api::set_screen_timeout,
        api::set_ac_timer,
        api::set_twelve_volt_timer,
    ),
    components(schemas(
        api::StatusResponse,
        api::ApiError,
        api::ApiSuccess,
        api::BoolRequest,
        api::BrightnessRequest,
        api::LedRequest,
        api::WattsRequest,
        api::SecondsRequest,
        Telemetry,
        anker_767_ble_webserver::ble::telemetry::Output,
        anker_767_ble_webserver::ble::telemetry::Battery,
        anker_767_ble_webserver::ble::telemetry::BatteryState,
        anker_767_ble_webserver::ble::telemetry::LedState,
        anker_767_ble_webserver::ble::telemetry::StateAck,
    )),
    tags(
        (name = "status", description = "Connection status"),
        (name = "telemetry", description = "Device telemetry"),
        (name = "commands", description = "Device commands")
    ),
    info(
        title = "Anker PowerHouse 767 API",
        version = "0.1.0",
        description = "REST API for controlling Anker PowerHouse 767 via BLE"
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting Anker PowerHouse 767 BLE Web Server");

    // Create BLE device manager
    let device = Arc::new(AnkerDevice::new());
    let state: AppState = device.state();

    // Spawn BLE connection loop
    let device_clone = Arc::clone(&device);
    tokio::spawn(async move {
        if let Err(e) = device_clone.run().await {
            tracing::error!("BLE device error: {}", e);
        }
    });

    // Spawn telemetry metrics updater
    let mut telemetry_rx = device.subscribe_telemetry();
    tokio::spawn(async move {
        while let Ok(telemetry) = telemetry_rx.recv().await {
            metrics::update_from_telemetry(&telemetry);
        }
    });

    // Spawn connection state metrics updater
    let mut state_rx = device.subscribe_state();
    tokio::spawn(async move {
        while state_rx.changed().await.is_ok() {
            let state = *state_rx.borrow();
            metrics::update_connection_state(state);
        }
    });

    // Build router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_router = Router::new()
        .route("/status", get(api::get_status))
        .route("/telemetry", get(api::get_telemetry))
        .route("/power-save", post(api::set_power_save))
        .route("/ac-output", post(api::set_ac_output))
        .route("/twelve-volt-output", post(api::set_twelve_volt_output))
        .route("/screen-brightness", post(api::set_screen_brightness))
        .route("/led", post(api::set_led))
        .route("/recharge-power", post(api::set_recharge_power))
        .route("/screen-timeout", post(api::set_screen_timeout))
        .route("/ac-timer", post(api::set_ac_timer))
        .route("/twelve-volt-timer", post(api::set_twelve_volt_timer))
        .with_state(state);

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/api-docs", get(|| async { axum::Json(ApiDoc::openapi()) }))
        .route("/metrics", get(api::get_metrics))
        .nest("/api", api_router)
        .fallback_service(ServeDir::new("static").append_index_html_on_directories(true))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Server listening on http://{}", addr);
    info!("Swagger UI: http://{}/swagger-ui/", addr);
    info!("Metrics: http://{}/metrics", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
