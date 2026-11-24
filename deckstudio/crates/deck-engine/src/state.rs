//! Estado compartilhado do Engine (config, navegação, preview, métricas Win32 e logs).

use crate::actions::NavCmd;
use crate::device::DeviceManager;
use crate::obs::ObsClient;
use crate::sensors::SensorHub;
use crate::weather::WeatherSnapshot;
use deck_core::{
    save_config, AppConfig, EngineHealth, ObsConfig, TileConfig, UpdateInstallStatus, WeatherConfig,
};
use image::RgbImage;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct NavState {
    pub profile: String,
    pub page: String,
    pub folder_stack: Vec<String>,
}

impl Default for NavState {
    fn default() -> Self {
        Self {
            profile: "default".into(),
            page: "monitor".into(),
            folder_stack: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimerState {
    pub running: bool,
    pub remaining_secs: u64,
    pub initial_secs: u64,
    pub last_tick: Instant,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            running: false,
            remaining_secs: 25 * 60,
            initial_secs: 25 * 60,
            last_tick: Instant::now(),
        }
    }
}

pub struct EngineShared {
    pub config_path: PathBuf,
    pub config: Arc<RwLock<AppConfig>>,
    pub weather_cfg: Arc<RwLock<WeatherConfig>>,
    pub obs_cfg: Arc<RwLock<ObsConfig>>,
    pub weather_snap: Arc<RwLock<WeatherSnapshot>>,
    pub sensors: Arc<tokio::sync::Mutex<SensorHub>>,
    pub obs: Arc<ObsClient>,
    pub device: Arc<DeviceManager>,
    pub rt_handle: tokio::runtime::Handle,
    pub nav: Arc<RwLock<NavState>>,
    pub last_previews: Arc<RwLock<BTreeMap<String, RgbImage>>>,
    pub render_cache: Mutex<HashMap<String, (u64, RgbImage)>>,
    pub nic_info: RwLock<(String, u64)>,
    pub logs: Mutex<VecDeque<String>>,
    pub paused: AtomicBool,
    pub brightness: AtomicU8,
    pub started_at: Instant,
    pub redraw_notify: Arc<Notify>,
    pub weather_notify: Arc<Notify>,
    pub should_exit: AtomicBool,
    pub update_status: Arc<RwLock<UpdateInstallStatus>>,
    pub press_times: Mutex<HashMap<u8, Instant>>,
    pub timer_state: Mutex<TimerState>,
    pub ipc_token: String,
    pub update_in_flight: AtomicBool,
    pub last_update_check: RwLock<Option<deck_core::UpdateCheckResponse>>,
}

impl EngineShared {
    pub fn clear_render_cache(&self) {
        if let Ok(mut g) = self.render_cache.lock() {
            g.clear();
        }
    }
    pub fn push_log(&self, msg: String) {
        let ts = chrono::Local::now().format("%H:%M:%S");
        let line = format!("{ts} {msg}");
        if let Ok(mut g) = self.logs.lock() {
            if g.len() >= 80 {
                g.pop_front();
            }
            g.push_back(line);
        }
    }

    pub fn notify_redraw(&self) {
        self.redraw_notify.notify_one();
    }

    pub fn dispatch_button_press(
        self: &Arc<Self>,
        pos: u8,
        grid_key: &str,
        act: crate::hid_proto::InputAction,
    ) -> Option<String> {
        let tiles = self.current_tiles();
        let tile = tiles.get(grid_key)?;
        let label = if !tile.title.is_empty() {
            tile.title.as_str()
        } else if let Some(w) = tile.widget.as_deref() {
            w
        } else {
            tile.kind.as_str()
        };

        let chosen_action = if tile.long_press_action.is_some() {
            match act {
                crate::hid_proto::InputAction::Press => {
                    if let Ok(mut pt) = self.press_times.lock() {
                        pt.insert(pos, Instant::now());
                    }
                    return Some(label.to_string());
                }
                crate::hid_proto::InputAction::Release => {
                    let held_ms = self
                        .press_times
                        .lock()
                        .ok()
                        .and_then(|mut pt| pt.remove(&pos))
                        .map(|t| t.elapsed().as_millis())
                        .unwrap_or(0);
                    if held_ms >= 500 {
                        self.push_log(format!("[INPUT] Long Press {grid_key} ({held_ms}ms) -> {label}"));
                        tile.long_press_action.clone()
                    } else {
                        self.push_log(format!("[INPUT] Clique {grid_key} -> {label}"));
                        tile.action.clone()
                    }
                }
            }
        } else {
            if act != crate::hid_proto::InputAction::Press {
                return None;
            }
            self.push_log(format!("[INPUT] Tecla {grid_key} (pos={pos}) -> {label}"));
            tile.action.clone()
        };

        if let Some(action) = chosen_action {
            let s_nav = Arc::clone(self);
            let s_log = Arc::clone(self);
            crate::actions::execute_action(
                &self.rt_handle,
                action,
                Arc::clone(&self.obs),
                Arc::new(move |cmd| s_nav.handle_nav(cmd)),
                Arc::new(move |msg| s_log.push_log(msg)),
            );
        }
        Some(label.to_string())
    }

    pub fn snapshot_timer(&self) -> (String, bool) {
        let Ok(mut t) = self.timer_state.lock() else {
            return ("25:00".into(), false);
        };
        if t.running {
            let elapsed = t.last_tick.elapsed().as_secs();
            if elapsed > 0 {
                t.remaining_secs = t.remaining_secs.saturating_sub(elapsed);
                t.last_tick = Instant::now();
                if t.remaining_secs == 0 {
                    t.running = false;
                }
            }
        }
        let mins = t.remaining_secs / 60;
        let secs = t.remaining_secs % 60;
        (format!("{mins:02}:{secs:02}"), t.running)
    }

    pub fn current_tiles(&self) -> BTreeMap<String, TileConfig> {
        let cfg = self.config.read().map(|g| g.clone()).unwrap_or_default();
        let nav = self.nav.read().map(|g| g.clone()).unwrap_or_default();
        let Some(prof) = cfg.profiles.get(&nav.profile).or_else(|| cfg.profiles.values().next()) else {
            return BTreeMap::new();
        };
        if let Some(folder_id) = nav.folder_stack.last() {
            if let Some(f) = prof.folders.get(folder_id) {
                return f.clone();
            }
        }
        prof.pages
            .get(&nav.page)
            .or_else(|| prof.pages.values().next())
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_profile(&self, profile_id: &str, page: Option<String>) {
        let Ok(cfg) = self.config.read() else { return };
        let Some(prof) = cfg.profiles.get(profile_id) else { return };
        let target_page = page
            .filter(|p| prof.pages.contains_key(p))
            .or_else(|| prof.pages.keys().next().cloned())
            .unwrap_or_else(|| "monitor".into());
        drop(cfg);

        let changed = if let Ok(mut nav) = self.nav.write() {
            let diff = nav.profile != profile_id || nav.page != target_page || !nav.folder_stack.is_empty();
            nav.profile = profile_id.to_string();
            nav.page = target_page.clone();
            nav.folder_stack.clear();
            diff
        } else {
            false
        };
        if changed {
            if let Ok(mut pt) = self.press_times.lock() {
                pt.clear();
            }
            self.clear_render_cache();
            self.device.clear_all();
            self.notify_redraw();
            self.push_log(format!("[PROFILE] -> {profile_id}/{target_page}"));
        }
    }

    pub fn handle_nav(&self, cmd: NavCmd) {
        match cmd {
            NavCmd::Page(target) => {
                let Ok(cfg) = self.config.read() else { return };
                let Ok(mut nav) = self.nav.write() else { return };
                let Some(prof) = cfg.profiles.get(&nav.profile) else { return };
                let names: Vec<String> = prof.pages.keys().cloned().collect();
                if names.is_empty() {
                    return;
                }
                let idx = names.iter().position(|n| n == &nav.page).unwrap_or(0);
                let next = match target.as_str() {
                    "next" => names[(idx + 1) % names.len()].clone(),
                    "prev" => names[(idx + names.len() - 1) % names.len()].clone(),
                    other if prof.pages.contains_key(other) => other.to_string(),
                    _ => return,
                };
                nav.page = next.clone();
                nav.folder_stack.clear();
                drop(nav);
                drop(cfg);
                self.clear_render_cache();
                self.device.clear_all();
                self.notify_redraw();
                self.push_log(format!("[NAV] Página -> {next}"));
            }
            NavCmd::Folder(target) => {
                if let Ok(mut nav) = self.nav.write() {
                    nav.folder_stack.push(target.clone());
                }
                self.clear_render_cache();
                self.device.clear_all();
                self.notify_redraw();
                self.push_log(format!("[NAV] Pasta -> {target}"));
            }
            NavCmd::FolderBack => {
                if let Ok(mut nav) = self.nav.write() {
                    nav.folder_stack.pop();
                }
                self.clear_render_cache();
                self.device.clear_all();
                self.notify_redraw();
            }
            NavCmd::Theme(target) => self.set_theme(&target),
            NavCmd::Brightness { value, relative } => {
                let cur = self.brightness.load(Ordering::Relaxed) as i16;
                let next = if relative { cur + value } else { value }.clamp(0, 100) as u8;
                self.set_brightness(next);
            }
            NavCmd::Timer { op, seconds } => {
                if let Ok(mut t) = self.timer_state.lock() {
                    match op.as_str() {
                        "reset" => {
                            let target = if seconds > 0 { seconds } else { t.initial_secs };
                            t.initial_secs = target;
                            t.remaining_secs = target;
                            t.running = false;
                            t.last_tick = Instant::now();
                        }
                        "pomodoro" => {
                            let target = if seconds > 0 { seconds } else { 25 * 60 };
                            t.initial_secs = target;
                            t.remaining_secs = target;
                            t.running = true;
                            t.last_tick = Instant::now();
                        }
                        _ => {
                            if t.running {
                                let elapsed = t.last_tick.elapsed().as_secs();
                                t.remaining_secs = t.remaining_secs.saturating_sub(elapsed);
                                t.running = false;
                            } else {
                                if t.remaining_secs == 0 {
                                    t.remaining_secs = if seconds > 0 { seconds } else { t.initial_secs };
                                }
                                t.running = true;
                                t.last_tick = Instant::now();
                            }
                        }
                    }
                }
                self.notify_redraw();
            }
        }
    }

    pub fn set_theme(&self, target: &str) {
        let chosen = if let Ok(mut cfg) = self.config.write() {
            let names: Vec<String> = cfg.themes.keys().cloned().collect();
            if names.is_empty() {
                return;
            }
            let next = if target == "next" {
                let idx = names.iter().position(|n| n == &cfg.active_theme).unwrap_or(0);
                names[(idx + 1) % names.len()].clone()
            } else if cfg.themes.contains_key(target) {
                target.to_string()
            } else {
                return;
            };
            cfg.active_theme = next.clone();
            let _ = save_config(&self.config_path, &cfg);
            next
        } else {
            return;
        };
        self.clear_render_cache();
        self.device.invalidate_cache();
        self.notify_redraw();
        self.push_log(format!("[THEME] Tema -> {chosen}"));
    }

    pub fn set_brightness(&self, val: u8) {
        let clamped = val.min(100);
        self.brightness.store(clamped, Ordering::Relaxed);
        self.device.set_brightness(clamped);
        if let Ok(mut cfg) = self.config.write() {
            cfg.device.brightness = clamped;
            let _ = save_config(&self.config_path, &cfg);
        }
        self.push_log(format!("[DEVICE] Brilho -> {clamped}%"));
    }

    pub fn build_health(&self) -> EngineHealth {
        let obs = self.obs.state.read().map(|g| g.clone()).unwrap_or_default();
        let w = self.weather_snap.read().map(|g| g.clone()).unwrap_or_default();
        let nav = self.nav.read().map(|g| g.clone()).unwrap_or_default();
        let cfg = self.config.read().map(|g| g.clone()).unwrap_or_default();
        let logs = self.logs.lock().map(|g| g.iter().cloned().collect()).unwrap_or_default();

        let (nic_name, nic_discards) = self
            .nic_info
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| (String::new(), 0));

        let uptime_s = self.started_at.elapsed().as_secs().max(1);
        let tiles_rate = self.device.total_tiles_sent() as f32 / uptime_s as f32;

        EngineHealth {
            ram_mb: current_process_ram_mb(),
            cpu_pct: 0.0,
            hid_connected: self.device.is_connected(),
            hid_tiles_per_s: (tiles_rate * 10.0).round() / 10.0,
            nic_name,
            nic_discards,
            uptime_s,
            obs_connected: obs.connected,
            obs_recording: obs.recording,
            obs_paused: obs.paused,
            obs_scene: obs.current_scene,
            weather_temp_c: w.temp_c,
            weather_tmin: w.tmin,
            weather_tmax: w.tmax,
            weather_city: cfg.weather.city,
            active_profile: nav.profile,
            active_page: nav.page,
            folder_stack: nav.folder_stack,
            active_theme: cfg.active_theme,
            brightness: self.brightness.load(Ordering::Relaxed),
            paused: self.paused.load(Ordering::Relaxed),
            recent_logs: logs,
        }
    }
}

fn current_process_ram_mb() -> f64 {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut pmc = PROCESS_MEMORY_COUNTERS::default();
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32).is_ok() {
            (pmc.WorkingSetSize as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0
        } else {
            0.0
        }
    }
}
