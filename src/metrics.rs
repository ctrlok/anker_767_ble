//! Prometheus metrics for Anker PowerHouse 767.

use crate::ble::{ConnectionState, Telemetry};
use prometheus::{
    Encoder, Gauge, GaugeVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};
use std::sync::OnceLock;

static REGISTRY: OnceLock<Metrics> = OnceLock::new();

pub struct Metrics {
    pub registry: Registry,
    pub battery_percentage: IntGauge,
    pub battery_percentage_individual: GaugeVec,
    pub battery_remaining_hours: Gauge,
    pub total_output_watts: IntGauge,
    pub total_input_watts: IntGauge,
    pub ac_input_watts: IntGauge,
    pub solar_input_watts: IntGauge,
    pub battery_temperature: GaugeVec,
    pub ac_outlet_on: IntGauge,
    pub twelve_volt_on: IntGauge,
    pub connected: IntGauge,
    pub commands_total: IntCounterVec,
}

impl Metrics {
    fn new() -> Self {
        let registry = Registry::new();

        let battery_percentage = IntGauge::new(
            "anker_battery_percentage",
            "Current battery percentage",
        )
        .unwrap();

        let battery_percentage_individual = GaugeVec::new(
            Opts::new(
                "anker_battery_percentage_individual",
                "Individual battery percentage",
            ),
            &["battery"],
        )
        .unwrap();

        let battery_remaining_hours = Gauge::new(
            "anker_battery_remaining_hours",
            "Estimated battery remaining time in hours",
        )
        .unwrap();

        let total_output_watts = IntGauge::new(
            "anker_total_output_watts",
            "Total output power in watts",
        )
        .unwrap();

        let total_input_watts = IntGauge::new(
            "anker_total_input_watts",
            "Total input power in watts",
        )
        .unwrap();

        let ac_input_watts = IntGauge::new(
            "anker_ac_input_watts",
            "AC input power in watts",
        )
        .unwrap();

        let solar_input_watts = IntGauge::new(
            "anker_solar_input_watts",
            "Solar input power in watts",
        )
        .unwrap();

        let battery_temperature = GaugeVec::new(
            Opts::new("anker_battery_temperature", "Battery temperature in celsius"),
            &["battery"],
        )
        .unwrap();

        let ac_outlet_on = IntGauge::new(
            "anker_ac_outlet_on",
            "AC outlet status (0=off, 1=on)",
        )
        .unwrap();

        let twelve_volt_on = IntGauge::new(
            "anker_twelve_volt_on",
            "12V outlet status (0=off, 1=on)",
        )
        .unwrap();

        let connected = IntGauge::new(
            "anker_connected",
            "BLE connection status (0=disconnected, 1=connected)",
        )
        .unwrap();

        let commands_total = IntCounterVec::new(
            Opts::new("anker_commands_total", "Total commands sent by type"),
            &["command"],
        )
        .unwrap();

        registry.register(Box::new(battery_percentage.clone())).unwrap();
        registry.register(Box::new(battery_percentage_individual.clone())).unwrap();
        registry.register(Box::new(battery_remaining_hours.clone())).unwrap();
        registry.register(Box::new(total_output_watts.clone())).unwrap();
        registry.register(Box::new(total_input_watts.clone())).unwrap();
        registry.register(Box::new(ac_input_watts.clone())).unwrap();
        registry.register(Box::new(solar_input_watts.clone())).unwrap();
        registry.register(Box::new(battery_temperature.clone())).unwrap();
        registry.register(Box::new(ac_outlet_on.clone())).unwrap();
        registry.register(Box::new(twelve_volt_on.clone())).unwrap();
        registry.register(Box::new(connected.clone())).unwrap();
        registry.register(Box::new(commands_total.clone())).unwrap();

        Self {
            registry,
            battery_percentage,
            battery_percentage_individual,
            battery_remaining_hours,
            total_output_watts,
            total_input_watts,
            ac_input_watts,
            solar_input_watts,
            battery_temperature,
            ac_outlet_on,
            twelve_volt_on,
            connected,
            commands_total,
        }
    }
}

pub fn metrics() -> &'static Metrics {
    REGISTRY.get_or_init(Metrics::new)
}

pub fn update_from_telemetry(telemetry: &Telemetry) {
    let m = metrics();

    m.battery_percentage.set(telemetry.total_battery_percentage as i64);
    m.total_output_watts.set(telemetry.total_output_watts as i64);
    m.total_input_watts.set(telemetry.total_input_watts as i64);
    m.ac_input_watts.set(telemetry.ac_input_watts as i64);
    m.solar_input_watts.set(telemetry.solar_input_watts as i64);

    m.battery_percentage_individual
        .with_label_values(&["internal"])
        .set(telemetry.internal_battery.percentage as f64);
    m.battery_percentage_individual
        .with_label_values(&["external"])
        .set(telemetry.external_battery.percentage as f64);
    m.battery_remaining_hours.set(telemetry.battery_remaining_hours as f64);

    m.battery_temperature
        .with_label_values(&["internal"])
        .set(telemetry.internal_battery.temperature as f64);
    m.battery_temperature
        .with_label_values(&["external"])
        .set(telemetry.external_battery.temperature as f64);

    m.ac_outlet_on.set(telemetry.ac_outlet.is_on as i64);

    // Use first 12V output status
    let twelve_volt_on = telemetry.twelve_volt.first().map(|o| o.is_on).unwrap_or(false);
    m.twelve_volt_on.set(twelve_volt_on as i64);
}

pub fn update_connection_state(state: ConnectionState) {
    let m = metrics();
    m.connected.set(if state == ConnectionState::Connected { 1 } else { 0 });
}

pub fn increment_command(command_type: &str) {
    let m = metrics();
    m.commands_total.with_label_values(&[command_type]).inc();
}

pub fn render() -> String {
    let m = metrics();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&m.registry.gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
