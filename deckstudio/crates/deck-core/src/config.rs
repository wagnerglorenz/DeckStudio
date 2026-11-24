use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub fn default_update_feed_url() -> String {
    "https://api.github.com/repos/wagnerglorenz/DeckStudio/releases/latest".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub device: DeviceConfig,
    pub active_theme: String,
    pub themes: BTreeMap<String, ThemeConfig>,
    pub color_rules: ColorRules,
    pub sensor_sources: SensorSourcesConfig,
    pub obs: ObsConfig,
    pub weather: WeatherConfig,
    pub smart_profiles: SmartProfilesConfig,
    pub active_profile: String,
    pub profiles: BTreeMap<String, ProfileConfig>,
    #[serde(default = "default_update_feed_url")]
    pub update_feed_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    pub brightness: u8,
    pub refresh_ms: u64,
    pub max_tiles_per_s: f32,
    pub night_mode: NightModeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NightModeConfig {
    pub enabled: bool,
    pub day_hour: u8,
    pub day_brightness: u8,
    pub night_hour: u8,
    pub night_brightness: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub name: String,
    pub style: String,         // "llm_dash" | "classic_spark" | "cyberpunk" | "matrix" | "obsidian_pro"
    pub surface_style: String, // "oled_frame" | "sparkline" | "cyber_hud" | "matrix_tty" | "obsidian_gold"
    pub background: String,
    pub card_bg: String,
    pub card_border: String,
    pub chart: String,
    pub text: String,
    pub muted: String,
    pub active: String,
    pub accent: String,
    pub glow_color: String,
    pub temp_colors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorRules {
    pub temp_yellow: f64,
    pub temp_red: f64,
    pub mem_yellow: f64,
    pub mem_red: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorSourcesConfig {
    /// Ordem de prioridade: "aida64", "librehwm", "hwinfo", "native_nvml"
    pub priority: Vec<String>,
    pub aida64_enabled: bool,
    pub aida64_watchdog: bool,
    pub librehwm_enabled: bool,
    pub librehwm_url: String,
    pub hwinfo_enabled: bool,
    pub native_nvml_enabled: bool,
    pub vram_total_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ObsConfig {
    pub host: String,
    pub port: u16,
    pub password_env: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherConfig {
    pub cep: String,
    pub city: String,
    pub lat: f64,
    pub lon: f64,
    pub refresh_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartProfilesConfig {
    pub enabled: bool,
    pub rules: Vec<SmartProfileRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartProfileRule {
    pub id: String,
    pub name: String,
    pub exe_name: String,
    pub profile: String,
    #[serde(default)]
    pub page: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    pub name: String,
    pub pages: BTreeMap<String, BTreeMap<String, TileConfig>>,
    #[serde(default)]
    pub folders: BTreeMap<String, BTreeMap<String, TileConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TileConfig {
    #[serde(rename = "type")]
    pub kind: String, // "sensor" | "button" | "folder"
    pub title: String,
    pub subtitle: String,
    pub icon: Option<String>,
    pub custom_image_b64: Option<String>,
    pub widget: Option<String>,
    pub id: Option<String>,
    pub unit: String,
    pub decimals: u8,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub group: Option<String>,
    pub style: Option<String>, // "ring" | "card"
    pub is_temp: bool,
    pub state: Option<String>,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub text_color: Option<String>,
    pub icon_color: Option<String>,
    pub action: Option<ActionConfig>,
    #[serde(default)]
    pub long_press_action: Option<ActionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionConfig {
    Page { target: String },
    Folder { target: String },
    FolderBack,
    Theme { target: String },
    Brightness { value: i16, relative: bool },
    Obs {
        op: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        input: Option<String>,
        #[serde(default)]
        source: Option<String>,
        #[serde(default)]
        scene: Option<String>,
    },
    Hotkey { keys: Vec<String> },
    Run { program: String, args: Vec<String> },
    Text { content: String },
    Media { op: String },
    Soundboard { path: String },
    Webhook { url: String, method: String },
    AudioApp {
        op: String, // "vol_up" | "vol_down" | "mute"
        #[serde(default)]
        process: String, // ex: "spotify.exe", "discord.exe" ou "" para padrão
        #[serde(default)]
        step: Option<f32>,
    },
    Timer {
        op: String, // "toggle" | "reset" | "pomodoro"
        #[serde(default)]
        seconds: u64,
    },
    MultiAction {
        name: String,
        steps: Vec<ActionStep>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step_kind", rename_all = "snake_case")]
pub enum ActionStep {
    Delay { ms: u64 },
    Action { action: ActionConfig },
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            brightness: 60,
            refresh_ms: 1000,
            max_tiles_per_s: 15.0,
            night_mode: NightModeConfig::default(),
        }
    }
}

impl Default for NightModeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            day_hour: 8,
            day_brightness: 65,
            night_hour: 22,
            night_brightness: 30,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "llm_dash".to_string(),
            style: "llm_dash".to_string(),
            surface_style: "oled_frame".to_string(),
            background: "#04070E".to_string(),
            card_bg: "#070C17".to_string(),
            card_border: "#1E3A5F".to_string(),
            chart: "#22D3EE".to_string(),
            text: "#FFFFFF".to_string(),
            muted: "#94A3B8".to_string(),
            active: "#E11D48".to_string(),
            accent: "#22D3EE".to_string(),
            glow_color: "#22D3EE".to_string(),
            temp_colors: true,
        }
    }
}

impl Default for ColorRules {
    fn default() -> Self {
        Self {
            temp_yellow: 60.0,
            temp_red: 75.0,
            mem_yellow: 65.0,
            mem_red: 85.0,
        }
    }
}

impl ColorRules {
    pub fn temp_color(&self, temp_c: f64) -> &'static str {
        if temp_c < self.temp_yellow {
            "#22C55E"
        } else if temp_c < self.temp_red {
            "#EAB308"
        } else {
            "#EF4444"
        }
    }

    pub fn usage_color(&self, pct: f64) -> &'static str {
        if pct < self.mem_yellow {
            "#22C55E"
        } else if pct < self.mem_red {
            "#EAB308"
        } else {
            "#EF4444"
        }
    }
}

impl Default for SensorSourcesConfig {
    fn default() -> Self {
        Self {
            priority: vec![
                "aida64".into(),
                "librehwm".into(),
                "hwinfo".into(),
                "native_nvml".into(),
            ],
            aida64_enabled: true,
            aida64_watchdog: false,
            librehwm_enabled: true,
            librehwm_url: "http://127.0.0.1:8085/data.json".into(),
            hwinfo_enabled: true,
            native_nvml_enabled: true,
            vram_total_mb: 24564.0,
        }
    }
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 4455,
            password_env: "OBS_WS_PASSWORD".into(),
            password: String::new(),
        }
    }
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            cep: String::new(),
            city: "Santa Maria".into(),
            lat: -29.6842,
            lon: -53.8069,
            refresh_minutes: 15,
        }
    }
}

impl Default for SmartProfilesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rules: vec![
                SmartProfileRule {
                    id: "obs-auto".into(),
                    name: "OBS Studio (Gravação)".into(),
                    exe_name: "obs64.exe".into(),
                    profile: "default".into(),
                    page: Some("obs".into()),
                    enabled: false,
                },
                SmartProfileRule {
                    id: "acc-auto".into(),
                    name: "Assetto Corsa Competizione".into(),
                    exe_name: "AC2-Win64-Shipping.exe".into(),
                    profile: "default".into(),
                    page: Some("monitor".into()),
                    enabled: false,
                },
            ],
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        default_config()
    }
}

#[allow(clippy::too_many_arguments)]
fn sensor_tile(
    title: &str,
    id: &str,
    unit: &str,
    min: f64,
    max: f64,
    group: &str,
    style: Option<&str>,
    is_temp: bool,
) -> TileConfig {
    TileConfig {
        kind: "sensor".into(),
        title: title.into(),
        id: Some(id.into()),
        unit: unit.into(),
        min: Some(min),
        max: Some(max),
        group: Some(group.into()),
        style: style.map(|s| s.into()),
        is_temp,
        ..Default::default()
    }
}

fn widget_tile(widget: &str, action: Option<ActionConfig>) -> TileConfig {
    TileConfig {
        kind: if action.is_some() { "button" } else { "sensor" }.into(),
        widget: Some(widget.into()),
        action,
        ..Default::default()
    }
}

fn button_tile(
    title: &str,
    subtitle: &str,
    icon: Option<&str>,
    state: Option<&str>,
    action: ActionConfig,
) -> TileConfig {
    TileConfig {
        kind: "button".into(),
        title: title.into(),
        subtitle: subtitle.into(),
        icon: icon.map(|s| s.into()),
        state: state.map(|s| s.into()),
        action: Some(action),
        ..Default::default()
    }
}

pub fn default_themes() -> BTreeMap<String, ThemeConfig> {
    let mut map = BTreeMap::new();
    map.insert("llm_dash".into(), ThemeConfig::default());
    map.insert(
        "classic_spark".into(),
        ThemeConfig {
            name: "classic_spark".into(),
            style: "classic_spark".into(),
            surface_style: "sparkline".into(),
            background: "#090B1A".into(),
            card_bg: "#090B1A".into(),
            card_border: "#272A4D".into(),
            chart: "#F700E7".into(),
            text: "#FFFFFF".into(),
            muted: "#94A3B8".into(),
            active: "#E11D48".into(),
            accent: "#38BDF8".into(),
            glow_color: "#F700E7".into(),
            temp_colors: true,
        },
    );
    map.insert(
        "cyberpunk".into(),
        ThemeConfig {
            name: "cyberpunk".into(),
            style: "cyberpunk".into(),
            surface_style: "cyber_hud".into(),
            background: "#050508".into(),
            card_bg: "#080810".into(),
            card_border: "#FCEE0A".into(),
            chart: "#00F0FF".into(),
            text: "#FFFFFF".into(),
            muted: "#00F0FF".into(),
            active: "#FF003C".into(),
            accent: "#FCEE0A".into(),
            glow_color: "#FF003C".into(),
            temp_colors: true,
        },
    );
    map.insert(
        "matrix".into(),
        ThemeConfig {
            name: "matrix".into(),
            style: "matrix".into(),
            surface_style: "matrix_tty".into(),
            background: "#000300".into(),
            card_bg: "#000800".into(),
            card_border: "#00AA2B".into(),
            chart: "#00FF41".into(),
            text: "#E6FFE6".into(),
            muted: "#22C55E".into(),
            active: "#FF3333".into(),
            accent: "#00FF41".into(),
            glow_color: "#00FF41".into(),
            temp_colors: true,
        },
    );
    map.insert(
        "obsidian_pro".into(),
        ThemeConfig {
            name: "obsidian_pro".into(),
            style: "obsidian_pro".into(),
            surface_style: "obsidian_gold".into(),
            background: "#000000".into(),
            card_bg: "#050507".into(),
            card_border: "#3F3F46".into(),
            chart: "#F59E0B".into(),
            text: "#FFFFFF".into(),
            muted: "#A1A1AA".into(),
            active: "#EF4444".into(),
            accent: "#F59E0B".into(),
            glow_color: "#F59E0B".into(),
            temp_colors: true,
        },
    );
    map
}

pub fn default_config() -> AppConfig {
    let mut monitor = BTreeMap::new();
    monitor.insert("0,0".into(), sensor_tile("GPU", "GPU_UTIL", "%", 0.0, 100.0, "gpu", Some("ring"), false));
    monitor.insert("1,0".into(), sensor_tile("GPU °C", "GPU_TEMP", "°", 0.0, 100.0, "gpu", None, true));
    monitor.insert("2,0".into(), sensor_tile("GPU Watt", "GPU_WATT", "W", 0.0, 450.0, "gpu", None, false));
    monitor.insert("3,0".into(), sensor_tile("VRAM", "VRAM_PCT", "%", 0.0, 100.0, "vram", Some("ring"), false));
    monitor.insert("4,0".into(), sensor_tile("GPU Fan", "GPU_FAN", "%", 0.0, 100.0, "gpu", Some("ring"), false));
    monitor.insert("5,0".into(), widget_tile("clock", None));

    monitor.insert("0,1".into(), sensor_tile("CPU", "CPU_UTIL", "%", 0.0, 100.0, "cpu", Some("ring"), false));
    monitor.insert("1,1".into(), sensor_tile("CPU °C", "CPU_TEMP", "°", 0.0, 100.0, "cpu", None, true));
    monitor.insert("2,1".into(), sensor_tile("CPU Watt", "CPU_WATT", "W", 0.0, 200.0, "cpu", None, false));
    monitor.insert("3,1".into(), sensor_tile("RAM", "RAM_UTIL", "%", 0.0, 100.0, "ram", Some("ring"), false));
    monitor.insert("4,1".into(), sensor_tile("Core", "CPU_CLOCK", "MHz", 0.0, 5500.0, "cpu", None, false));
    monitor.insert("5,1".into(), widget_tile("obs_status", None));

    monitor.insert("0,2".into(), sensor_tile("DOWN", "NET_DL", "Mbps", 0.0, 1000.0, "net_down", None, false));
    monitor.insert("1,2".into(), sensor_tile("FPS", "FPS", "", 0.0, 240.0, "fps", None, false));
    monitor.insert("2,2".into(), sensor_tile("UP", "NET_UL", "Mbps", 0.0, 500.0, "net_up", None, false));
    monitor.insert("3,2".into(), widget_tile("obs_logo", Some(ActionConfig::Page { target: "obs".into() })));
    monitor.insert("4,2".into(), widget_tile("nav_next", Some(ActionConfig::Page { target: "next".into() })));
    monitor.insert("5,2".into(), widget_tile("weather", None));

    let mut obs = BTreeMap::new();
    obs.insert(
        "0,0".into(),
        button_tile(
            "Gravar",
            "CLIQUE",
            Some("record"),
            Some("record"),
            ActionConfig::Obs { op: "toggle_record".into(), name: None, input: None, source: None, scene: None },
        ),
    );
    obs.insert(
        "1,0".into(),
        button_tile(
            "Pausar",
            "PARADO",
            Some("pause"),
            Some("record_pause"),
            ActionConfig::Obs { op: "toggle_record_pause".into(), name: None, input: None, source: None, scene: None },
        ),
    );
    obs.insert(
        "2,0".into(),
        button_tile(
            "Som Jogo",
            "ATIVO",
            Some("audio"),
            Some("mute"),
            ActionConfig::Obs {
                op: "toggle_mute".into(),
                name: None,
                input: Some("Áudio do Jogo (A50 X Game)".into()),
                source: None,
                scene: None,
            },
        ),
    );
    obs.insert(
        "3,0".into(),
        button_tile(
            "Track",
            "TRAINEE",
            Some("track"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("AC-TrackTrainee".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert(
        "4,0".into(),
        button_tile(
            "Logo",
            "CANAL",
            Some("logo"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("Logo Canal".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert("5,0".into(), widget_tile("clock", None));

    obs.insert(
        "0,1".into(),
        button_tile(
            "1.Studio",
            "ACC",
            Some("studio"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("1. ACC - Pro Studio (16:9)".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert(
        "1,1".into(),
        button_tile(
            "2.Jogo",
            "CAM",
            Some("game_cam"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("2. ACC - Jogo Puro + Cam Sobreposta (16:9)".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert(
        "2,1".into(),
        button_tile(
            "3.Split",
            "ACC",
            Some("split"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("3. ACC - Lateral Split (16:9)".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert(
        "3,1".into(),
        button_tile(
            "4.Foco",
            "16:9",
            Some("focus"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("4. Vídeos - Foco Total (Centro 16:9)".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert(
        "4,1".into(),
        button_tile(
            "5.CmdCtr",
            "MULTI",
            Some("multi"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("5. Vídeos - Command Center (Multi-Screen)".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert("5,1".into(), widget_tile("obs_status", None));

    obs.insert("0,2".into(), widget_tile("cpu_health", None));
    obs.insert("1,2".into(), widget_tile("gpu_health", None));
    obs.insert(
        "2,2".into(),
        button_tile(
            "6.Cam 4K",
            "CHEIA",
            Some("cam4k"),
            None,
            ActionConfig::Obs { op: "scene".into(), name: Some("6. Câmera Tela Cheia (4K)".into()), input: None, source: None, scene: None },
        ),
    );
    obs.insert("3,2".into(), widget_tile("nav_prev", Some(ActionConfig::Page { target: "monitor".into() })));
    obs.insert(
        "4,2".into(),
        button_tile(
            "Discord",
            "ABRIR",
            Some("discord"),
            None,
            ActionConfig::Run {
                program: r"C:\Users\wagne\AppData\Local\Discord\Update.exe".into(),
                args: vec!["--processStart".into(), "Discord.exe".into()],
            },
        ),
    );
    obs.insert("5,2".into(), widget_tile("weather", None));

    let mut pages = BTreeMap::new();
    pages.insert("monitor".into(), monitor);
    pages.insert("obs".into(), obs);

    let mut profiles = BTreeMap::new();
    profiles.insert(
        "default".into(),
        ProfileConfig {
            name: "Padrão (Monitor + OBS)".into(),
            pages,
            folders: BTreeMap::new(),
        },
    );

    AppConfig {
        device: DeviceConfig::default(),
        active_theme: "llm_dash".into(),
        themes: default_themes(),
        color_rules: ColorRules::default(),
        sensor_sources: SensorSourcesConfig::default(),
        obs: ObsConfig::default(),
        weather: WeatherConfig::default(),
        smart_profiles: SmartProfilesConfig::default(),
        active_profile: "default".into(),
        profiles,
        update_feed_url: default_update_feed_url(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCandidate {
    pub path: String,
    pub source: String,
    pub modified_ts: u64,
    pub profiles_count: usize,
}

fn backup_search_paths(primary_path: &Path) -> Vec<(String, std::path::PathBuf)> {
    let mut out = Vec::new();
    if let Some(parent) = primary_path.parent() {
        out.push((
            "Backup Local (AppData)".into(),
            parent.join("backups").join("config-latest.json"),
        ));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = std::path::PathBuf::from(local).join("DeckStudio").join("config.json");
        if p != primary_path {
            out.push(("Instalação Anterior (LocalAppData)".into(), p));
        }
    }
    if let Ok(userprof) = std::env::var("USERPROFILE") {
        out.push((
            "Documentos (DeckStudio Backup)".into(),
            std::path::PathBuf::from(userprof)
                .join("Documents")
                .join("DeckStudio")
                .join("backups")
                .join("config-latest.json"),
        ));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p1 = dir.join("config.json");
            if p1 != primary_path {
                out.push(("Pasta Portátil (config.json)".into(), p1));
            }
            out.push((
                "Pasta Portátil (backups)".into(),
                dir.join("backups").join("config-latest.json"),
            ));
        }
    }
    out
}

pub fn find_previous_backups(primary_path: &Path) -> Vec<BackupCandidate> {
    let mut list = Vec::new();
    for (source, p) in backup_search_paths(primary_path) {
        if !p.exists() {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&raw) {
                let modified_ts = std::fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                list.push(BackupCandidate {
                    path: p.display().to_string(),
                    source,
                    modified_ts,
                    profiles_count: cfg.profiles.len(),
                });
            }
        }
    }
    list.sort_by_key(|a| std::cmp::Reverse(a.modified_ts));
    list
}

pub fn load_or_create_config(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        // Auto-restaura silenciosamente apenas de backups privados em AppData/LocalAppData;
        // Documentos e pasta portátil permanecem listados para restauração manual na UI.
        let mut private_candidates = Vec::new();
        if let Some(parent) = path.parent() {
            private_candidates.push(parent.join("backups").join("config-latest.json"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = std::path::PathBuf::from(local).join("DeckStudio").join("config.json");
            if p != path {
                private_candidates.push(p);
            }
        }
        for cand in private_candidates {
            if cand.exists() {
                if let Ok(raw) = std::fs::read_to_string(&cand) {
                    if let Ok(mut restored) = serde_json::from_str::<AppConfig>(&raw) {
                        for (k, v) in default_themes() {
                            restored.themes.entry(k).or_insert(v);
                        }
                        let _ = save_config(path, &restored);
                        return Ok(restored);
                    }
                }
            }
        }
        let cfg = default_config();
        save_config(path, &cfg)?;
        return Ok(cfg);
    }
    let raw = std::fs::read_to_string(path).map_err(|e| format!("Falha ao ler config: {e}"))?;
    let mut cfg: AppConfig = serde_json::from_str(&raw).map_err(|e| format!("Config JSON inválido: {e}"))?;
    for (k, v) in default_themes() {
        cfg.themes.entry(k).or_insert(v);
    }
    Ok(cfg)
}

pub fn save_config(path: &Path, cfg: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Falha ao criar diretório config: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("Falha serializar config: {e}"))?;
    std::fs::write(path, &json).map_err(|e| format!("Falha gravar config: {e}"))?;

    // Backup automático redundante (AppData/backups + Documentos/DeckStudio/backups)
    if let Some(parent) = path.parent() {
        let bdir = parent.join("backups");
        if std::fs::create_dir_all(&bdir).is_ok() {
            let _ = std::fs::write(bdir.join("config-latest.json"), &json);
        }
    }
    if let Ok(userprof) = std::env::var("USERPROFILE") {
        let doc_bdir = std::path::PathBuf::from(userprof)
            .join("Documents")
            .join("DeckStudio")
            .join("backups");
        if std::fs::create_dir_all(&doc_bdir).is_ok() {
            let mut sanitized = cfg.clone();
            sanitized.obs.password.clear();
            if let Ok(doc_json) = serde_json::to_string_pretty(&sanitized) {
                let _ = std::fs::write(doc_bdir.join("config-latest.json"), doc_json);
            }
        }
    }
    Ok(())
}

/// Importa um arquivo `manifest.json` de perfil Stream Dock compatível para um `ProfileConfig`.
pub fn import_stream_dock_manifest_json(raw_json: &str, profile_name: &str) -> Result<ProfileConfig, String> {
    let val: serde_json::Value = serde_json::from_str(raw_json).map_err(|e| format!("JSON inválido: {e}"))?;
    let actions = val
        .get("Actions")
        .and_then(|v| v.as_object())
        .or_else(|| {
            val.get("Controllers")
                .and_then(|c| c.get(0))
                .and_then(|c0| c0.get("Actions"))
                .and_then(|v| v.as_object())
        })
        .ok_or_else(|| "Manifest sem objeto Actions".to_string())?;

    let mut page = BTreeMap::new();
    for (grid_key, item) in actions {
        let uuid = item.get("UUID").and_then(|v| v.as_str()).unwrap_or("");
        let settings = item.get("Settings");
        let title = settings
            .and_then(|s| s.get("my_title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tile = if uuid.contains("page.previous") {
            widget_tile("nav_prev", Some(ActionConfig::Page { target: "prev".into() }))
        } else if uuid.contains("page.next") {
            widget_tile("nav_next", Some(ActionConfig::Page { target: "next".into() }))
        } else if uuid.contains("NetWork") {
            widget_tile("net_strip", None)
        } else if uuid.contains("cpu.temperature") {
            widget_tile("cpu_chip", None)
        } else if uuid.contains("memory") {
            widget_tile("ram_gauge", None)
        } else if uuid.contains("systemvitals") {
            let ev = settings.and_then(|s| s.get("my_monitor_event")).and_then(|v| v.as_str()).unwrap_or("");
            let mapped_id = match ev {
                "CPU_TEMP_LHW" => "CPU_TEMP",
                "CPU_WATT_LHW" => "CPU_WATT",
                "CPU_PERCENT" => "CPU_UTIL",
                "GPU_TEMP_NVIDIA" => "GPU_TEMP",
                "GPU_WATT_NVIDIA" => "GPU_WATT",
                "GPU_PERCENT" => "GPU_UTIL",
                "VRAM_PERCENT_NVIDIA" => "VRAM_PCT",
                "RAM_PERCENT" => "RAM_UTIL",
                "LAN_MB" => "NET_TOTAL_MBS",
                _ => {
                    let lk = settings.and_then(|s| s.get("my_libre_key")).and_then(|v| v.as_str()).unwrap_or("");
                    if lk.contains("GPU_Fan") {
                        "GPU_FAN"
                    } else if lk.contains("Core") {
                        "CPU_CLOCK"
                    } else if lk.contains("FPS") {
                        "FPS"
                    } else if lk.contains("Rede") {
                        "NET_TOTAL"
                    } else {
                        "CPU_UTIL"
                    }
                }
            };
            sensor_tile(
                if title.is_empty() { mapped_id } else { &title },
                mapped_id,
                if mapped_id.contains("TEMP") { "°" } else if mapped_id.contains("WATT") { "W" } else if mapped_id.contains("CLOCK") { "MHz" } else { "%" },
                0.0,
                100.0,
                if mapped_id.starts_with("GPU") { "gpu" } else if mapped_id.starts_with("VRAM") { "vram" } else if mapped_id.starts_with("RAM") { "ram" } else { "cpu" },
                None,
                mapped_id.contains("TEMP"),
            )
        } else {
            button_tile(if title.is_empty() { "Ação" } else { &title }, "", None, None, ActionConfig::Text { content: String::new() })
        };
        page.insert(grid_key.clone(), tile);
    }

    let mut pages = BTreeMap::new();
    pages.insert("principal".into(), page);
    Ok(ProfileConfig {
        name: profile_name.into(),
        pages,
        folders: BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrip() {
        let cfg = default_config();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.active_theme, "llm_dash");
        assert!(parsed.profiles["default"].pages.contains_key("monitor"));
        assert!(parsed.profiles["default"].pages.contains_key("obs"));
        assert_eq!(parsed.weather.city, "Santa Maria");
    }

    #[test]
    fn color_rules_thresholds() {
        let rules = ColorRules::default();
        assert_eq!(rules.temp_color(45.0), "#22C55E");
        assert_eq!(rules.temp_color(66.0), "#EAB308");
        assert_eq!(rules.temp_color(80.0), "#EF4444");
        assert_eq!(rules.usage_color(38.0), "#22C55E");
        assert_eq!(rules.usage_color(70.0), "#EAB308");
        assert_eq!(rules.usage_color(90.0), "#EF4444");
    }

    #[test]
    fn import_stream_dock_manifest_parses_actions() {
        let sample = r#"{
            "Actions": {
                "0,0": { "UUID": "com.vivremotion.systemvitals.plugin", "Settings": { "my_title": "GPU Fan", "my_monitor_event": "CUSTOM_LHW_1", "my_libre_key": "GPU_Fan_1" } },
                "0,1": { "UUID": "com.hotspot.streamdock.page.previous" },
                "5,0": { "UUID": "com.hotspot.stream.NetWork" }
            }
        }"#;
        let prof = import_stream_dock_manifest_json(sample, "Importado").unwrap();
        let p = &prof.pages["principal"];
        assert_eq!(p["0,0"].id.as_deref(), Some("GPU_FAN"));
        assert_eq!(p["0,1"].widget.as_deref(), Some("nav_prev"));
        assert_eq!(p["5,0"].widget.as_deref(), Some("net_strip"));
    }
}
