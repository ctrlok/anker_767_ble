//! Prometheus metrics for Anker PowerHouse 767.

use crate::ble::{ConnectionState, Telemetry};
use prometheus::{
    Encoder, Gauge, GaugeVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

static REGISTRY: OnceLock<Metrics> = OnceLock::new();

pub struct Metrics {
    pub registry: Registry,
    // Battery
    pub battery_percentage: IntGauge,
    pub battery_percentage_individual: GaugeVec,
    pub battery_remaining_hours: Gauge,
    pub battery_temperature: GaugeVec,
    pub battery_state: IntGauge,
    // Power totals
    pub total_output_watts: IntGauge,
    pub total_input_watts: IntGauge,
    pub ac_input_watts: IntGauge,
    pub solar_input_watts: IntGauge,
    // AC outlet
    pub ac_outlet_on: IntGauge,
    pub ac_outlet_watts: IntGauge,
    // 12V outlets (2 ports)
    pub twelve_volt_on: GaugeVec,
    pub twelve_volt_watts: GaugeVec,
    pub twelve_volt_timer_seconds: IntGauge,
    // USB-C (3 ports)
    pub usb_c_on: GaugeVec,
    pub usb_c_watts: GaugeVec,
    // USB-A (2 ports)
    pub usb_a_on: GaugeVec,
    pub usb_a_watts: GaugeVec,
    // Connection
    pub connected: IntGauge,
    pub commands_total: IntCounterVec,
    /// Per-metric timestamps in milliseconds (metric key -> timestamp)
    pub timestamps: RwLock<HashMap<String, u64>>,
}

impl Metrics {
    fn new() -> Self {
        let registry = Registry::new();

        // Battery metrics
        let battery_percentage = IntGauge::new(
            "anker_battery_percentage",
            "Total battery percentage",
        )
        .unwrap();

        let battery_percentage_individual = GaugeVec::new(
            Opts::new("anker_battery_percentage_individual", "Individual battery percentage"),
            &["battery"],
        )
        .unwrap();

        let battery_remaining_hours = Gauge::new(
            "anker_battery_remaining_hours",
            "Estimated battery remaining time in hours",
        )
        .unwrap();

        let battery_temperature = GaugeVec::new(
            Opts::new("anker_battery_temperature", "Battery temperature in celsius"),
            &["battery"],
        )
        .unwrap();

        let battery_state = IntGauge::new(
            "anker_battery_state",
            "Battery state (0=idle, 1=discharging, 2=charging)",
        )
        .unwrap();

        // Power totals
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

        // AC outlet
        let ac_outlet_on = IntGauge::new(
            "anker_ac_outlet_on",
            "AC outlet status (0=off, 1=on)",
        )
        .unwrap();

        let ac_outlet_watts = IntGauge::new(
            "anker_ac_outlet_watts",
            "AC outlet power in watts",
        )
        .unwrap();

        // 12V outlets
        let twelve_volt_on = GaugeVec::new(
            Opts::new("anker_twelve_volt_on", "12V outlet status (0=off, 1=on)"),
            &["port"],
        )
        .unwrap();

        let twelve_volt_watts = GaugeVec::new(
            Opts::new("anker_twelve_volt_watts", "12V outlet power in watts"),
            &["port"],
        )
        .unwrap();

        let twelve_volt_timer_seconds = IntGauge::new(
            "anker_twelve_volt_timer_seconds",
            "12V outlet timer remaining in seconds",
        )
        .unwrap();

        // USB-C outlets
        let usb_c_on = GaugeVec::new(
            Opts::new("anker_usb_c_on", "USB-C port status (0=off, 1=on)"),
            &["port"],
        )
        .unwrap();

        let usb_c_watts = GaugeVec::new(
            Opts::new("anker_usb_c_watts", "USB-C port power in watts"),
            &["port"],
        )
        .unwrap();

        // USB-A outlets
        let usb_a_on = GaugeVec::new(
            Opts::new("anker_usb_a_on", "USB-A port status (0=off, 1=on)"),
            &["port"],
        )
        .unwrap();

        let usb_a_watts = GaugeVec::new(
            Opts::new("anker_usb_a_watts", "USB-A port power in watts"),
            &["port"],
        )
        .unwrap();

        // Connection
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

        // Register all metrics
        registry.register(Box::new(battery_percentage.clone())).unwrap();
        registry.register(Box::new(battery_percentage_individual.clone())).unwrap();
        registry.register(Box::new(battery_remaining_hours.clone())).unwrap();
        registry.register(Box::new(battery_temperature.clone())).unwrap();
        registry.register(Box::new(battery_state.clone())).unwrap();
        registry.register(Box::new(total_output_watts.clone())).unwrap();
        registry.register(Box::new(total_input_watts.clone())).unwrap();
        registry.register(Box::new(ac_input_watts.clone())).unwrap();
        registry.register(Box::new(solar_input_watts.clone())).unwrap();
        registry.register(Box::new(ac_outlet_on.clone())).unwrap();
        registry.register(Box::new(ac_outlet_watts.clone())).unwrap();
        registry.register(Box::new(twelve_volt_on.clone())).unwrap();
        registry.register(Box::new(twelve_volt_watts.clone())).unwrap();
        registry.register(Box::new(twelve_volt_timer_seconds.clone())).unwrap();
        registry.register(Box::new(usb_c_on.clone())).unwrap();
        registry.register(Box::new(usb_c_watts.clone())).unwrap();
        registry.register(Box::new(usb_a_on.clone())).unwrap();
        registry.register(Box::new(usb_a_watts.clone())).unwrap();
        registry.register(Box::new(connected.clone())).unwrap();
        registry.register(Box::new(commands_total.clone())).unwrap();

        Self {
            registry,
            battery_percentage,
            battery_percentage_individual,
            battery_remaining_hours,
            battery_temperature,
            battery_state,
            total_output_watts,
            total_input_watts,
            ac_input_watts,
            solar_input_watts,
            ac_outlet_on,
            ac_outlet_watts,
            twelve_volt_on,
            twelve_volt_watts,
            twelve_volt_timer_seconds,
            usb_c_on,
            usb_c_watts,
            usb_a_on,
            usb_a_watts,
            connected,
            commands_total,
            timestamps: RwLock::new(HashMap::new()),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn set_timestamp(m: &Metrics, key: &str) {
    if let Ok(mut ts) = m.timestamps.write() {
        ts.insert(key.to_string(), now_ms());
    }
}

pub fn metrics() -> &'static Metrics {
    REGISTRY.get_or_init(Metrics::new)
}

pub fn update_from_telemetry(telemetry: &Telemetry) {
    let m = metrics();

    // Battery metrics
    m.battery_percentage.set(telemetry.total_battery_percentage as i64);
    set_timestamp(m, "anker_battery_percentage");

    m.battery_percentage_individual
        .with_label_values(&["internal"])
        .set(telemetry.internal_battery.percentage as f64);
    set_timestamp(m, "anker_battery_percentage_individual{battery=\"internal\"}");

    m.battery_percentage_individual
        .with_label_values(&["external"])
        .set(telemetry.external_battery.percentage as f64);
    set_timestamp(m, "anker_battery_percentage_individual{battery=\"external\"}");

    m.battery_remaining_hours.set(telemetry.battery_remaining_hours as f64);
    set_timestamp(m, "anker_battery_remaining_hours");

    m.battery_temperature
        .with_label_values(&["internal"])
        .set(telemetry.internal_battery.temperature as f64);
    set_timestamp(m, "anker_battery_temperature{battery=\"internal\"}");

    m.battery_temperature
        .with_label_values(&["external"])
        .set(telemetry.external_battery.temperature as f64);
    set_timestamp(m, "anker_battery_temperature{battery=\"external\"}");

    m.battery_state.set(telemetry.battery_state.clone() as i64);
    set_timestamp(m, "anker_battery_state");

    // Power totals
    m.total_output_watts.set(telemetry.total_output_watts as i64);
    set_timestamp(m, "anker_total_output_watts");

    m.total_input_watts.set(telemetry.total_input_watts as i64);
    set_timestamp(m, "anker_total_input_watts");

    m.ac_input_watts.set(telemetry.ac_input_watts as i64);
    set_timestamp(m, "anker_ac_input_watts");

    m.solar_input_watts.set(telemetry.solar_input_watts as i64);
    set_timestamp(m, "anker_solar_input_watts");

    // AC outlet
    m.ac_outlet_on.set(telemetry.ac_outlet.is_on as i64);
    set_timestamp(m, "anker_ac_outlet_on");

    m.ac_outlet_watts.set(telemetry.ac_outlet.watts as i64);
    set_timestamp(m, "anker_ac_outlet_watts");

    // 12V outlets (2 ports)
    for (i, output) in telemetry.twelve_volt.iter().enumerate() {
        let port = i.to_string();
        m.twelve_volt_on.with_label_values(&[&port]).set(output.is_on as i64 as f64);
        set_timestamp(m, &format!("anker_twelve_volt_on{{port=\"{}\"}}", port));

        m.twelve_volt_watts.with_label_values(&[&port]).set(output.watts as f64);
        set_timestamp(m, &format!("anker_twelve_volt_watts{{port=\"{}\"}}", port));
    }

    // 12V timer (shared between ports, use first)
    if let Some(output) = telemetry.twelve_volt.first() {
        if let Some(seconds) = output.time_remaining_seconds {
            m.twelve_volt_timer_seconds.set(seconds as i64);
            set_timestamp(m, "anker_twelve_volt_timer_seconds");
        }
    }

    // USB-C (3 ports)
    for (i, output) in telemetry.usb_c.iter().enumerate() {
        let port = i.to_string();
        m.usb_c_on.with_label_values(&[&port]).set(output.is_on as i64 as f64);
        set_timestamp(m, &format!("anker_usb_c_on{{port=\"{}\"}}", port));

        m.usb_c_watts.with_label_values(&[&port]).set(output.watts as f64);
        set_timestamp(m, &format!("anker_usb_c_watts{{port=\"{}\"}}", port));
    }

    // USB-A (2 ports)
    for (i, output) in telemetry.usb_a.iter().enumerate() {
        let port = i.to_string();
        m.usb_a_on.with_label_values(&[&port]).set(output.is_on as i64 as f64);
        set_timestamp(m, &format!("anker_usb_a_on{{port=\"{}\"}}", port));

        m.usb_a_watts.with_label_values(&[&port]).set(output.watts as f64);
        set_timestamp(m, &format!("anker_usb_a_watts{{port=\"{}\"}}", port));
    }
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
    let output = String::from_utf8(buffer).unwrap();

    let timestamps = match m.timestamps.read() {
        Ok(ts) => ts,
        Err(_) => return output,
    };

    if timestamps.is_empty() {
        return output;
    }

    // Append per-metric timestamps
    output
        .lines()
        .map(|line| {
            // Skip HELP/TYPE lines and empty lines
            if line.starts_with('#') || line.is_empty() {
                return line.to_string();
            }
            // Extract metric key (name + labels, before the value)
            // Format: "metric_name{labels} value" or "metric_name value"
            let metric_key = line.split_whitespace().next().unwrap_or("");
            if let Some(&ts) = timestamps.get(metric_key) {
                format!("{} {}", line, ts)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
