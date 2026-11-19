export interface NightModeConfig {
  enabled: boolean;
  day_hour: number;
  day_brightness: number;
  night_hour: number;
  night_brightness: number;
}

export interface DeviceConfig {
  brightness: number;
  refresh_ms: number;
  max_tiles_per_s: number;
  night_mode: NightModeConfig;
}

export interface ThemeConfig {
  name: string;
  style: "llm_dash" | "ajazz" | string;
  background: string;
  card_bg: string;
  card_border: string;
  chart: string;
  text: string;
  muted: string;
  active: string;
  accent: string;
  temp_colors: boolean;
}

export interface ColorRules {
  temp_yellow: number;
  temp_red: number;
  mem_yellow: number;
  mem_red: number;
}

export interface SensorSourcesConfig {
  priority: string[];
  aida64_enabled: boolean;
  aida64_watchdog: boolean;
  librehwm_enabled: boolean;
  librehwm_url: string;
  hwinfo_enabled: boolean;
  native_nvml_enabled: boolean;
  vram_total_mb: number;
}

export interface ObsConfig {
  host: string;
  port: number;
  password_env: string;
  password: string;
}

export interface WeatherConfig {
  city: string;
  lat: number;
  lon: number;
  refresh_minutes: number;
}

export interface SmartProfileRule {
  id: string;
  name: string;
  exe_name: string;
  profile: string;
  page?: string | null;
  enabled: boolean;
}

export interface SmartProfilesConfig {
  enabled: boolean;
  rules: SmartProfileRule[];
}

export type ActionStep =
  | { step_kind: "delay"; ms: number }
  | { step_kind: "action"; action: ActionConfig };

export type ActionConfig =
  | { kind: "page"; target: string }
  | { kind: "folder"; target: string }
  | { kind: "folder_back" }
  | { kind: "theme"; target: string }
  | { kind: "brightness"; value: number; relative: boolean }
  | { kind: "obs"; op: string; name?: string | null; input?: string | null }
  | { kind: "hotkey"; keys: string[] }
  | { kind: "run"; program: string; args: string[] }
  | { kind: "text"; content: string }
  | { kind: "media"; op: string }
  | { kind: "soundboard"; path: string }
  | { kind: "webhook"; url: string; method: string }
  | { kind: "multi_action"; name: string; steps: ActionStep[] };

export interface TileConfig {
  type: "sensor" | "button" | "folder" | string;
  title: string;
  subtitle: string;
  icon?: string | null;
  custom_image_b64?: string | null;
  widget?: string | null;
  id?: string | null;
  unit: string;
  decimals: number;
  min?: number | null;
  max?: number | null;
  group?: string | null;
  style?: "ring" | "card" | string | null;
  is_temp: boolean;
  state?: string | null;
  action?: ActionConfig | null;
}

export interface ProfileConfig {
  name: string;
  pages: Record<string, Record<string, TileConfig>>;
  folders?: Record<string, Record<string, TileConfig>>;
}

export interface AppConfig {
  device: DeviceConfig;
  active_theme: string;
  themes: Record<string, ThemeConfig>;
  color_rules: ColorRules;
  sensor_sources: SensorSourcesConfig;
  obs: ObsConfig;
  weather: WeatherConfig;
  smart_profiles: SmartProfilesConfig;
  active_profile: string;
  profiles: Record<string, ProfileConfig>;
}

export interface EngineHealth {
  ram_mb: number;
  cpu_pct: number;
  hid_connected: boolean;
  hid_tiles_per_s: number;
  nic_name: string;
  nic_discards: number;
  uptime_s: number;
  obs_connected: boolean;
  obs_recording: boolean;
  obs_paused: boolean;
  obs_scene: string;
  weather_temp_c: number | null;
  weather_tmin: number | null;
  weather_tmax: number | null;
  weather_city: string;
  active_profile: string;
  active_page: string;
  folder_stack: string[];
  active_theme: string;
  brightness: number;
  paused: boolean;
  recent_logs: string[];
}

export interface SourceStatus {
  id: string;
  name: string;
  active: boolean;
  sensor_count: number;
  status_text: string;
  setup_guide: string[];
  endpoint?: string | null;
}

export interface SensorReading {
  id: string;
  label: string;
  source: string;
  value: number;
  unit: string;
}

export interface ObsCatalog {
  connected: boolean;
  current_scene: string;
  scenes: string[];
  audio_inputs: string[];
}

export interface GridPreviewResponse {
  profile: string;
  page: string;
  folder_stack: string[];
  tiles_b64: Record<string, string>;
}
