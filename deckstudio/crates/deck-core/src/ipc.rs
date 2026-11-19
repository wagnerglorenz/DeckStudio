use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineHealth {
    pub ram_mb: f64,
    pub cpu_pct: f64,
    pub hid_connected: bool,
    pub hid_tiles_per_s: f32,
    pub nic_name: String,
    pub nic_discards: u64,
    pub uptime_s: u64,
    pub obs_connected: bool,
    pub obs_recording: bool,
    pub obs_paused: bool,
    pub obs_scene: String,
    pub weather_temp_c: Option<f64>,
    pub weather_tmin: Option<f64>,
    pub weather_tmax: Option<f64>,
    pub weather_city: String,
    pub active_profile: String,
    pub active_page: String,
    pub folder_stack: Vec<String>,
    pub active_theme: String,
    pub brightness: u8,
    pub paused: bool,
    pub recent_logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatus {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub sensor_count: usize,
    pub status_text: String,
    pub setup_guide: Vec<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub id: String,
    pub label: String,
    pub source: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsCatalog {
    pub connected: bool,
    pub current_scene: String,
    pub scenes: Vec<String>,
    pub audio_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPreviewResponse {
    pub profile: String,
    pub page: String,
    pub folder_stack: Vec<String>,
    pub tiles_b64: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAjazzRequest {
    pub path: String,
    pub profile_id: String,
    pub profile_name: String,
}
