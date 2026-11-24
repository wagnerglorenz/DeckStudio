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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObsCatalog {
    pub connected: bool,
    pub current_scene: String,
    pub scenes: Vec<String>,
    pub audio_inputs: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPreviewResponse {
    pub profile: String,
    pub page: String,
    pub folder_stack: Vec<String>,
    pub tiles_b64: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportManifestRequest {
    pub path: String,
    pub profile_id: String,
    pub profile_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherCepRequest {
    pub cep: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutostartStatus {
    pub engine_enabled: bool,
    pub dash_enabled: bool,
    pub engine_path: String,
    pub dash_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutostartUpdate {
    pub engine_enabled: bool,
    pub dash_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCheckResponse {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_notes: String,
    pub download_url: String,
    pub sha256: Option<String>,
    pub published_at: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateInstallStatus {
    pub state: String, // "idle" | "downloading" | "verifying" | "installing" | "error"
    pub progress_pct: u8,
    pub message: String,
}
