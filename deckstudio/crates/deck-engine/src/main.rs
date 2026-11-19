//! `deck-engine` — Daemon nativo 24/7 para controladores Stream Dock compatíveis (HID + Sensores + OBS + Smart Profiles + Bandeja).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod device;
mod hid_proto;
mod ipc;
mod obs;
mod render;
mod sensors;
mod smart_profile;
mod state;
mod weather;

use chrono::{Local, Timelike};
use deck_core::{load_or_create_config, ActionConfig, ThemeConfig, TileConfig};
use device::DeviceManager;
use image::RgbImage;
use obs::{ObsClient, ObsState};
use render::{
    decode_custom_image, group_accent, render_button_tile, render_clock_tile,
    render_compact_health_tile, render_nav_circle_tile, render_obs_logo_tile,
    render_obs_status_tile, render_sensor_tile, render_weather_tile,
};
use sensors::{format_rate_human, SensorHub};
use smart_profile::ForegroundHook;
use state::{EngineShared, NavState};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIconBuilder};
use weather::{spawn_weather_loop, WeatherSnapshot};

fn default_config_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(appdata).join("DeckStudio").join("config.json")
}

fn make_tray_icon() -> Icon {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32u32 {
        for x in 0..32u32 {
            // Squircle exterior (borda ciano -> verde)
            let on_border = x < 3 || x >= 29 || y < 3 || y >= 29;
            // 2 colunas x 3 linhas de mini teclas (esquerda/centro)
            let in_key = (6..=11).contains(&x) || (13..=18).contains(&x);
            let in_row = (6..=11).contains(&y) || (13..=18).contains(&y) || (20..=25).contains(&y);
            // Faixa lateral direita estilo D
            let in_strip = (20..=25).contains(&x) && (6..=25).contains(&y);
            if on_border {
                rgba.extend_from_slice(&[34, 211, 238, 255]); // #22D3EE
            } else if in_strip {
                rgba.extend_from_slice(&[34, 197, 94, 255]); // #22C55E
            } else if in_key && in_row {
                rgba.extend_from_slice(&[56, 189, 248, 255]); // #38BDF8
            } else {
                rgba.extend_from_slice(&[13, 17, 23, 255]); // #0D1117
            }
        }
    }
    Icon::from_rgba(rgba, 32, 32).unwrap()
}

fn launch_studio_editor() {
    let Ok(exe) = std::env::current_exe() else { return };
    if let Some(dir) = exe.parent() {
        for candidate in [
            dir.join("deck-studio.exe"),
            dir.join("DeckStudio.exe"),
            PathBuf::from(r"C:\Users\wagne\workspace\StreamDeck\deckstudio\apps\studio\src-tauri\target\release\deck-studio.exe"),
        ] {
            if candidate.exists() {
                let _ = Command::new(candidate).spawn();
                return;
            }
        }
    }
}

fn button_visual_state(tile: &TileConfig, obs: &ObsState, cfg_pages: &BTreeMap<String, BTreeMap<String, TileConfig>>) -> (String, String, String) {
    let mut title = tile.title.clone();
    let mut sub = tile.subtitle.clone();
    let is_obs = matches!(tile.action, Some(ActionConfig::Obs { .. }));
    if is_obs && !obs.connected {
        return ("disabled".into(), "OFF".into(), title);
    }
    match tile.state.as_deref() {
        Some("record") => {
            if obs.recording && obs.paused {
                return ("warning".into(), "PAUSADO".into(), "GRAVANDO".into());
            } else if obs.recording {
                return ("active".into(), "PARAR".into(), "GRAVANDO".into());
            } else {
                return ("idle".into(), "CLIQUE".into(), if title.is_empty() { "Gravar".into() } else { title });
            }
        }
        Some("record_pause") => {
            if !obs.recording {
                return ("disabled".into(), "PARADO".into(), title);
            } else if obs.paused {
                return ("warning".into(), "RETOMAR".into(), "PAUSADO".into());
            } else {
                return ("idle".into(), "REC ATIVO".into(), title);
            }
        }
        Some("stream") => {
            return if obs.streaming {
                ("active".into(), "LIVE".into(), title)
            } else {
                ("idle".into(), "OFF".into(), title)
            };
        }
        Some("mute") => {
            let inp = match &tile.action {
                Some(ActionConfig::Obs { input: Some(i), .. }) => i.as_str(),
                _ => "Mic/Aux",
            };
            let muted = obs.muted.get(inp).copied().unwrap_or(false);
            return if muted {
                ("active".into(), "MUDO".into(), title)
            } else {
                ("idle".into(), "ATIVO".into(), title)
            };
        }
        _ => {}
    }

    if let Some(ActionConfig::Obs { op, name: Some(sc_name), .. }) = &tile.action {
        if op == "scene" && obs.current_scene == *sc_name {
            return ("active".into(), "ATIVA".into(), title);
        }
    }
    if tile.kind == "folder" && sub.is_empty() {
        sub = "PASTA".into();
    }
    if title.is_empty() {
        if let Some(p) = cfg_pages.keys().next() {
            title = p.clone();
        }
    }
    ("idle".into(), sub, title)
}

fn short_scene(obs: &ObsState, tiles: &BTreeMap<String, TileConfig>) -> String {
    if obs.current_scene.is_empty() {
        return "---".into();
    }
    for t in tiles.values() {
        if let Some(ActionConfig::Obs { op, name: Some(n), .. }) = &t.action {
            if op == "scene" && *n == obs.current_scene {
                return t.title.clone();
            }
        }
    }
    obs.current_scene.chars().take(11).collect()
}

fn tile_base_hash(base_theme_sig: u64, grid_key: &str, tile: &TileConfig) -> u64 {
    let mut h = DefaultHasher::new();
    base_theme_sig.hash(&mut h);
    grid_key.hash(&mut h);
    tile.kind.hash(&mut h);
    tile.title.hash(&mut h);
    tile.subtitle.hash(&mut h);
    tile.icon.hash(&mut h);
    tile.custom_image_b64.hash(&mut h);
    tile.widget.hash(&mut h);
    tile.id.hash(&mut h);
    tile.unit.hash(&mut h);
    tile.decimals.hash(&mut h);
    tile.min.map(f64::to_bits).hash(&mut h);
    tile.max.map(f64::to_bits).hash(&mut h);
    tile.group.hash(&mut h);
    tile.style.hash(&mut h);
    tile.is_temp.hash(&mut h);
    tile.state.hash(&mut h);
    h.finish()
}

fn render_all_tiles(
    shared: &EngineShared,
    hub: &mut SensorHub,
    values: &std::collections::HashMap<String, f64>,
) -> BTreeMap<String, RgbImage> {
    let cfg = shared.config.read().map(|g| g.clone()).unwrap_or_default();
    let theme: ThemeConfig = cfg.themes.get(&cfg.active_theme).cloned().unwrap_or_default();
    let rules = &cfg.color_rules;
    let tiles = shared.current_tiles();
    let obs = shared.obs.state.read().map(|g| g.clone()).unwrap_or_default();
    let w = shared.weather_snap.read().map(|g| g.clone()).unwrap_or_default();
    let now_dt = Local::now();

    let base_theme_sig = {
        let mut h = DefaultHasher::new();
        if let Ok(bytes) = serde_json::to_vec(&(&theme, rules)) {
            bytes.hash(&mut h);
        }
        h.finish()
    };

    let prof_name = shared.nav.read().map(|n| n.profile.clone()).unwrap_or_else(|_| "default".into());
    let obs_page_tiles = cfg
        .profiles
        .get(&prof_name)
        .and_then(|p| p.pages.get("obs"))
        .cloned()
        .unwrap_or_default();
    let pages_map = cfg
        .profiles
        .get(&prof_name)
        .map(|p| p.pages.clone())
        .unwrap_or_default();

    let mut cache = shared.render_cache.lock().ok();
    let mut rendered = BTreeMap::new();
    for (grid_key, tile) in &tiles {
        let base_sig = tile_base_hash(base_theme_sig, grid_key, tile);
        let mut sig_h = DefaultHasher::new();
        base_sig.hash(&mut sig_h);

        // Calcula assinatura específica do conteúdo dinâmico antes de renderizar
        let dyn_plan = if let Some(b64) = &tile.custom_image_b64 {
            "custom".hash(&mut sig_h);
            Err(b64.as_str())
        } else {
            Ok(match tile.widget.as_deref() {
                Some("nav_prev") => {
                    "nav_prev".hash(&mut sig_h);
                    DynKind::NavPrev
                }
                Some("nav_next") => {
                    "nav_next".hash(&mut sig_h);
                    DynKind::NavNext
                }
                Some("obs_logo") => {
                    "obs_logo".hash(&mut sig_h);
                    DynKind::ObsLogo
                }
                Some("clock") => {
                    let hm = now_dt.format("%H:%M").to_string();
                    let dm = now_dt.format("%d/%m").to_string();
                    hm.hash(&mut sig_h);
                    dm.hash(&mut sig_h);
                    DynKind::Clock(hm, dm)
                }
                Some("obs_status") => {
                    let st = if !obs.connected {
                        "OFF"
                    } else if obs.recording && obs.paused {
                        "PAUSADO"
                    } else if obs.recording {
                        "REC"
                    } else {
                        "PRONTO"
                    };
                    let sc = short_scene(&obs, &obs_page_tiles);
                    st.hash(&mut sig_h);
                    sc.hash(&mut sig_h);
                    DynKind::ObsStatus(st, sc)
                }
                Some("weather") => {
                    w.temp_c.map(|v| (v * 10.0).round() as i64).hash(&mut sig_h);
                    w.tmin.map(|v| (v * 10.0).round() as i64).hash(&mut sig_h);
                    w.tmax.map(|v| (v * 10.0).round() as i64).hash(&mut sig_h);
                    cfg.weather.city.hash(&mut sig_h);
                    w.weather_code.hash(&mut sig_h);
                    DynKind::Weather
                }
                Some("cpu_health") => {
                    let u = values.get("CPU_UTIL").copied();
                    let t = values.get("CPU_TEMP").copied();
                    u.map(|v| v.round() as i64).hash(&mut sig_h);
                    t.map(|v| v.round() as i64).hash(&mut sig_h);
                    DynKind::CpuHealth(u, t)
                }
                Some("gpu_health") => {
                    let u = values.get("GPU_UTIL").copied();
                    let t = values.get("GPU_TEMP").copied();
                    u.map(|v| v.round() as i64).hash(&mut sig_h);
                    t.map(|v| v.round() as i64).hash(&mut sig_h);
                    DynKind::GpuHealth(u, t)
                }
                Some("net_strip") => {
                    let dl = format_rate_human(values.get("NET_DL").copied().unwrap_or(0.0));
                    let ul = format_rate_human(values.get("NET_UL").copied().unwrap_or(0.0));
                    dl.hash(&mut sig_h);
                    ul.hash(&mut sig_h);
                    DynKind::NetStrip(dl, ul)
                }
                _ if tile.kind == "sensor" => {
                    let sid = tile.id.as_deref().unwrap_or("");
                    let (val_txt, raw) = hub.format_sensor(sid, tile.decimals);
                    let hist = hub.get_history(sid);
                    val_txt.hash(&mut sig_h);
                    raw.map(|v| (v * 10.0).round() as i64).hash(&mut sig_h);
                    if tile.style.as_deref() != Some("ring") {
                        for &pt in &hist {
                            ((pt * 10.0).round() as i64).hash(&mut sig_h);
                        }
                    }
                    DynKind::Sensor(val_txt, raw, hist)
                }
                _ => {
                    let (st, sub, btn_title) = button_visual_state(tile, &obs, &pages_map);
                    st.hash(&mut sig_h);
                    sub.hash(&mut sig_h);
                    btn_title.hash(&mut sig_h);
                    DynKind::Button(st, sub, btn_title)
                }
            })
        };

        let sig = sig_h.finish();
        if let Some(ref c) = cache {
            if let Some((prev_sig, cached_img)) = c.get(grid_key) {
                if *prev_sig == sig {
                    rendered.insert(grid_key.clone(), cached_img.clone());
                    continue;
                }
            }
        }

        let img = match dyn_plan {
            Err(b64) => decode_custom_image(b64)
                .unwrap_or_else(|| render_button_tile(&tile.title, &tile.subtitle, "idle", &theme, None)),
            Ok(DynKind::NavPrev) => render_nav_circle_tile("prev", &theme),
            Ok(DynKind::NavNext) => render_nav_circle_tile("next", &theme),
            Ok(DynKind::ObsLogo) => render_obs_logo_tile(),
            Ok(DynKind::Clock(hm, dm)) => render_clock_tile(&hm, &dm, &theme),
            Ok(DynKind::ObsStatus(st, sc)) => render_obs_status_tile(st, &sc, &theme),
            Ok(DynKind::Weather) => render_weather_tile(w.temp_c, w.tmin, w.tmax, &cfg.weather.city, w.weather_code, &theme),
            Ok(DynKind::CpuHealth(u, t)) => render_compact_health_tile(
                "CPU",
                u,
                t,
                &group_accent(Some("cpu"), &theme),
                rules,
                &theme,
            ),
            Ok(DynKind::GpuHealth(u, t)) => render_compact_health_tile(
                "GPU",
                u,
                t,
                &group_accent(Some("gpu"), &theme),
                rules,
                &theme,
            ),
            Ok(DynKind::NetStrip(dl, ul)) => render_button_tile(&format!("↓{dl}"), &format!("↑{ul}"), "idle", &theme, None),
            Ok(DynKind::Sensor(val_txt, raw, hist)) => render_sensor_tile(
                &tile.title,
                &val_txt,
                &tile.unit,
                &hist,
                tile.min,
                tile.max,
                &theme,
                rules,
                raw,
                tile.group.as_deref(),
                tile.style.as_deref(),
                tile.is_temp,
            ),
            Ok(DynKind::Button(st, sub, btn_title)) => {
                render_button_tile(&btn_title, &sub, &st, &theme, tile.icon.as_deref())
            }
        };

        let _ = shared.device.queue_grid_image(grid_key, &img);
        if let Some(ref mut c) = cache {
            c.insert(grid_key.clone(), (sig, img.clone()));
        }
        rendered.insert(grid_key.clone(), img);
    }
    rendered
}

enum DynKind {
    NavPrev,
    NavNext,
    ObsLogo,
    Clock(String, String),
    ObsStatus(&'static str, String),
    Weather,
    CpuHealth(Option<f64>, Option<f64>),
    GpuHealth(Option<f64>, Option<f64>),
    NetStrip(String, String),
    Sensor(String, Option<f64>, Vec<f64>),
    Button(String, String, String),
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .expect("Tokio runtime");

    let cfg_path = default_config_path();
    let cfg = load_or_create_config(&cfg_path).unwrap_or_default();

    let redraw_notify = Arc::new(Notify::new());
    let cfg_arc = Arc::new(RwLock::new(cfg.clone()));
    let w_cfg_arc = Arc::new(RwLock::new(cfg.weather.clone()));
    let obs_cfg_arc = Arc::new(RwLock::new(cfg.obs.clone()));
    let w_snap = Arc::new(RwLock::new(WeatherSnapshot {
        city: cfg.weather.city.clone(),
        ..Default::default()
    }));

    let obs_notify = Arc::clone(&redraw_notify);
    let logs_buf: Mutex<VecDeque<String>> = Mutex::new(VecDeque::with_capacity(80));
    let shared_slot: Arc<RwLock<Option<Arc<EngineShared>>>> = Arc::new(RwLock::new(None));
    let slot_obs = Arc::clone(&shared_slot);
    let slot_btn = Arc::clone(&shared_slot);

    let _guard = rt.enter();
    let obs = ObsClient::start(
        Arc::clone(&obs_cfg_arc),
        Arc::new(move || obs_notify.notify_one()),
        Arc::new(move |msg| {
            if let Some(s) = slot_obs.read().ok().and_then(|g| g.clone()) {
                s.push_log(msg);
            }
        }),
    );

    let device = match DeviceManager::start(
        cfg.device.max_tiles_per_s,
        cfg.device.brightness,
        move |pos, grid_key, act| {
            let Some(shared) = slot_btn.read().ok().and_then(|g| g.clone()) else {
                return;
            };
            shared.dispatch_button_press(pos, &grid_key, act);
        },
    ) {
        Ok(d) => Arc::new(d),
        Err(e) => {
            eprintln!("Erro ao abrir dispositivo HID compatível: {e}");
            return;
        }
    };

    let mut init_nav = NavState::default();
    if let Some(p) = cfg.profiles.get(&cfg.active_profile) {
        init_nav.profile = cfg.active_profile.clone();
        if let Some(first_pg) = p.pages.keys().next() {
            init_nav.page = first_pg.clone();
        }
    }

    let shared = Arc::new(EngineShared {
        config_path: cfg_path,
        config: cfg_arc,
        weather_cfg: w_cfg_arc,
        obs_cfg: obs_cfg_arc,
        weather_snap: Arc::clone(&w_snap),
        sensors: Arc::new(tokio::sync::Mutex::new(SensorHub::new())),
        obs,
        device,
        rt_handle: rt.handle().clone(),
        nav: Arc::new(RwLock::new(init_nav)),
        last_previews: Arc::new(RwLock::new(BTreeMap::new())),
        render_cache: Mutex::new(HashMap::new()),
        nic_info: RwLock::new((String::new(), 0)),
        logs: logs_buf,
        paused: AtomicBool::new(false),
        brightness: AtomicU8::new(cfg.device.brightness),
        started_at: Instant::now(),
        redraw_notify: Arc::clone(&redraw_notify),
        should_exit: AtomicBool::new(false),
    });

    if let Ok(mut slot) = shared_slot.write() {
        *slot = Some(Arc::clone(&shared));
    }
    shared.push_log("[INFO] deck-engine iniciado (HID 0300:1010, 512B)".into());

    // Clima em background
    let s_wlog = Arc::clone(&shared);
    spawn_weather_loop(
        Arc::clone(&shared.weather_cfg),
        Arc::clone(&shared.weather_snap),
        Arc::new(move |msg| s_wlog.push_log(msg)),
    );

    // Servidor IPC local (127.0.0.1:19876)
    rt.spawn(ipc::start_ipc_server(Arc::clone(&shared)));

    // Loop de sensores + renderização
    let s_loop = Arc::clone(&shared);
    rt.spawn(async move {
        let mut last_night_hour: Option<u32> = None;
        loop {
            if s_loop.should_exit.load(Ordering::Relaxed) {
                break;
            }
            if !s_loop.paused.load(Ordering::Relaxed) {
                let (src_cfg, refresh_ms, night) = {
                    let c = s_loop.config.read().map(|g| g.clone()).unwrap_or_default();
                    (c.sensor_sources, c.device.refresh_ms.max(250), c.device.night_mode)
                };
                if night.enabled {
                    let hr = Local::now().hour();
                    if last_night_hour != Some(hr) {
                        last_night_hour = Some(hr);
                        let is_night = if night.night_hour > night.day_hour {
                            hr >= night.night_hour as u32 || hr < night.day_hour as u32
                        } else {
                            hr >= night.night_hour as u32 && hr < night.day_hour as u32
                        };
                        let target_b = if is_night { night.night_brightness } else { night.day_brightness };
                        if s_loop.brightness.load(Ordering::Relaxed) != target_b {
                            s_loop.set_brightness(target_b);
                        }
                    }
                }

                let rendered = {
                    let mut hub = s_loop.sensors.lock().await;
                    let vals = hub.refresh(&src_cfg).await;
                    if let Ok(mut g) = s_loop.nic_info.write() {
                        *g = (hub.last_native.nic_name.clone(), hub.last_native.nic_discards);
                    }
                    render_all_tiles(&s_loop, &mut hub, &vals)
                };
                if let Ok(mut prev) = s_loop.last_previews.write() {
                    *prev = rendered;
                }
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(refresh_ms)) => {}
                    _ = s_loop.redraw_notify.notified() => {}
                }
            } else {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    });

    // Smart Profile hook (Win32 SetWinEventHook na thread da bandeja — 0% polling)
    let s_smart = Arc::clone(&shared);
    let _fg_hook = ForegroundHook::install(move |exe_name| {
        let Ok(cfg) = s_smart.config.read() else { return };
        if !cfg.smart_profiles.enabled {
            return;
        }
        let matched = cfg
            .smart_profiles
            .rules
            .iter()
            .find(|r| r.enabled && r.exe_name.eq_ignore_ascii_case(&exe_name))
            .cloned();
        drop(cfg);
        if let Some(rule) = matched {
            s_smart.set_profile(&rule.profile, rule.page);
        }
    });

    // Ícone da bandeja + Win32 message pump
    let menu = Menu::new();
    let m_open = MenuItem::new("Abrir DeckStudio", true, None);
    let m_pause = MenuItem::new("Pausar / Retomar", true, None);
    let m_theme_llm = MenuItem::new("Tema: LLM Dashboard", true, None);
    let m_theme_ajazz = MenuItem::new("Tema: AJAZZ Clássico", true, None);
    let m_br_sub = Submenu::new("Brilho", true);
    let m_b20 = MenuItem::new("20%", true, None);
    let m_b40 = MenuItem::new("40%", true, None);
    let m_b60 = MenuItem::new("60%", true, None);
    let m_b80 = MenuItem::new("80%", true, None);
    let m_b100 = MenuItem::new("100%", true, None);
    let _ = m_br_sub.append_items(&[&m_b20, &m_b40, &m_b60, &m_b80, &m_b100]);
    let m_exit = MenuItem::new("Sair", true, None);

    let _ = menu.append_items(&[
        &m_open,
        &PredefinedMenuItem::separator(),
        &m_theme_llm,
        &m_theme_ajazz,
        &m_br_sub,
        &m_pause,
        &PredefinedMenuItem::separator(),
        &m_exit,
    ]);

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("DeckStudio Engine")
        .with_icon(make_tray_icon())
        .build()
        .ok();

    let rx_menu = MenuEvent::receiver();
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
        };
        let mut msg = MSG::default();
        while !shared.should_exit.load(Ordering::Relaxed) {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            while let Ok(ev) = rx_menu.try_recv() {
                if ev.id == m_open.id() {
                    launch_studio_editor();
                } else if ev.id == m_pause.id() {
                    let cur = shared.paused.load(Ordering::Relaxed);
                    shared.paused.store(!cur, Ordering::Relaxed);
                } else if ev.id == m_theme_llm.id() {
                    shared.set_theme("llm_dash");
                } else if ev.id == m_theme_ajazz.id() {
                    shared.set_theme("ajazz");
                } else if ev.id == m_b20.id() {
                    shared.set_brightness(20);
                } else if ev.id == m_b40.id() {
                    shared.set_brightness(40);
                } else if ev.id == m_b60.id() {
                    shared.set_brightness(60);
                } else if ev.id == m_b80.id() {
                    shared.set_brightness(80);
                } else if ev.id == m_b100.id() {
                    shared.set_brightness(100);
                } else if ev.id == m_exit.id() {
                    shared.should_exit.store(true, Ordering::Relaxed);
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod button_simulation_tests {
    use super::*;
    use crate::actions::{execute_action, NavCmd};
    use crate::device::index_to_grid;
    use crate::hid_proto::{parse_input_report, InputAction, INPUT_REQUEST_PREFIX};
    use deck_core::{default_config, ActionStep, ObsConfig};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::collections::HashSet;
    use tokio_tungstenite::tungstenite::Message;

    fn build_v1_press_packet(pos: u8) -> Vec<u8> {
        let mut pkt = Vec::with_capacity(11);
        pkt.extend_from_slice(&INPUT_REQUEST_PREFIX);
        pkt.push(pos);
        pkt.push(0x00);
        pkt
    }

    fn wait_until<F: Fn() -> bool>(timeout_ms: u64, cond: F) -> bool {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while Instant::now() < deadline {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        cond()
    }

    async fn start_mock_obs_v5() -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            #[derive(Default)]
            struct MockObs {
                recording: bool,
                paused: bool,
                streaming: bool,
                current_scene: String,
                muted: HashMap<String, bool>,
            }
            let st = Arc::new(Mutex::new(MockObs {
                current_scene: "1. ACC - Pro Studio (16:9)".into(),
                muted: HashMap::from([("Áudio do Jogo (A50 X Game)".into(), false)]),
                ..Default::default()
            }));
            let scenes = vec![
                "AC-TrackTrainee",
                "logo canal",
                "1. ACC - Pro Studio (16:9)",
                "2. ACC - Jogo Puro + Cam Sobreposta (16:9)",
                "3. ACC - Lateral Split (16:9)",
                "4. Vídeos - Foco Total (Centro 16:9)",
                "5. Vídeos - Command Center (Multi-Screen)",
                "6. Câmera Tela Cheia (4K)",
            ];

            while let Ok((stream, _)) = listener.accept().await {
                let st_conn = Arc::clone(&st);
                let scenes_c = scenes.clone();
                tokio::spawn(async move {
                    let Ok(ws) = tokio_tungstenite::accept_async(stream).await else { return };
                    let (mut tx, mut rx) = ws.split();
                    let _ = tx
                        .send(Message::Text(
                            json!({"op": 0, "d": {"obsWebSocketVersion": "5.3.0", "rpcVersion": 1}})
                                .to_string()
                                .into(),
                        ))
                        .await;
                    let Some(Ok(_)) = rx.next().await else { return };
                    let _ = tx
                        .send(Message::Text(
                            json!({"op": 2, "d": {"negotiatedRpcVersion": 1}})
                                .to_string()
                                .into(),
                        ))
                        .await;

                    while let Some(Ok(msg)) = rx.next().await {
                        let Ok(txt) = msg.to_text() else { continue };
                        let Ok(v) = serde_json::from_str::<Value>(txt) else { continue };
                        if v.get("op").and_then(|x| x.as_i64()) != Some(6) {
                            continue;
                        }
                        let rtype = v.pointer("/d/requestType").and_then(|x| x.as_str()).unwrap_or("");
                        let rid = v.pointer("/d/requestId").and_then(|x| x.as_str()).unwrap_or("");
                        let rdata = v.pointer("/d/requestData");

                        let (resp_data, event_opt) = {
                            let mut s = st_conn.lock().unwrap();
                            match rtype {
                                "GetRecordStatus" => (
                                    json!({"outputActive": s.recording, "outputPaused": s.paused}),
                                    None,
                                ),
                                "GetStreamStatus" => (json!({"outputActive": s.streaming}), None),
                                "GetSceneList" => (
                                    json!({
                                        "currentProgramSceneName": s.current_scene,
                                        "scenes": scenes_c.iter().map(|n| json!({"sceneName": n})).collect::<Vec<_>>()
                                    }),
                                    None,
                                ),
                                "GetInputList" => (
                                    json!({"inputs": [{"inputName": "Áudio do Jogo (A50 X Game)"}]}),
                                    None,
                                ),
                                "GetInputMute" => {
                                    let inp = rdata.and_then(|d| d.get("inputName")).and_then(|x| x.as_str()).unwrap_or("");
                                    let m = s.muted.get(inp).copied().unwrap_or(false);
                                    (json!({"inputMuted": m}), None)
                                }
                                "ToggleRecord" => {
                                    s.recording = !s.recording;
                                    s.paused = false;
                                    let ev_state = if s.recording {
                                        "OBS_WEBSOCKET_OUTPUT_STARTED"
                                    } else {
                                        "OBS_WEBSOCKET_OUTPUT_STOPPED"
                                    };
                                    (
                                        json!({"outputActive": s.recording}),
                                        Some(json!({"op": 5, "d": {"eventType": "RecordStateChanged", "eventData": {"outputActive": s.recording, "outputState": ev_state}}})),
                                    )
                                }
                                "ToggleRecordPause" => {
                                    if s.recording {
                                        s.paused = !s.paused;
                                    }
                                    let ev_state = if s.paused {
                                        "OBS_WEBSOCKET_OUTPUT_PAUSED"
                                    } else {
                                        "OBS_WEBSOCKET_OUTPUT_RESUMED"
                                    };
                                    (
                                        json!({"outputPaused": s.paused}),
                                        Some(json!({"op": 5, "d": {"eventType": "RecordStateChanged", "eventData": {"outputActive": s.recording, "outputState": ev_state}}})),
                                    )
                                }
                                "SetCurrentProgramScene" => {
                                    if let Some(name) = rdata.and_then(|d| d.get("sceneName")).and_then(|x| x.as_str()) {
                                        s.current_scene = name.to_string();
                                        (
                                            json!({}),
                                            Some(json!({"op": 5, "d": {"eventType": "CurrentProgramSceneChanged", "eventData": {"sceneName": name}}})),
                                        )
                                    } else {
                                        (json!({}), None)
                                    }
                                }
                                "ToggleInputMute" => {
                                    let inp = rdata.and_then(|d| d.get("inputName")).and_then(|x| x.as_str()).unwrap_or("");
                                    let cur = s.muted.get(inp).copied().unwrap_or(false);
                                    s.muted.insert(inp.to_string(), !cur);
                                    (
                                        json!({"inputMuted": !cur}),
                                        Some(json!({"op": 5, "d": {"eventType": "InputMuteStateChanged", "eventData": {"inputName": inp, "inputMuted": !cur}}})),
                                    )
                                }
                                _ => (json!({}), None),
                            }
                        };

                        let resp = json!({
                            "op": 7,
                            "d": {
                                "requestType": rtype,
                                "requestId": rid,
                                "requestStatus": {"result": true, "code": 100},
                                "responseData": resp_data
                            }
                        });
                        let _ = tx.send(Message::Text(resp.to_string().into())).await;
                        if let Some(ev) = event_opt {
                            let _ = tx.send(Message::Text(ev.to_string().into())).await;
                        }
                    }
                });
            }
        });
        port
    }

    #[test]
    fn os_thread_button_dispatch_does_not_panic() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .unwrap();
        let obs_cfg = Arc::new(RwLock::new(ObsConfig {
            port: 1, // fechado de propósito
            ..Default::default()
        }));
        let obs = {
            let _g = rt.enter();
            ObsClient::start(obs_cfg, Arc::new(|| {}), Arc::new(|_| {}))
        };

        let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let handle = rt.handle().clone();

        for (name, action) in [
            ("Page", ActionConfig::Page { target: "obs".into() }),
            ("Folder", ActionConfig::Folder { target: "macros".into() }),
            ("FolderBack", ActionConfig::FolderBack),
            ("Theme", ActionConfig::Theme { target: "ajazz".into() }),
            ("Brightness", ActionConfig::Brightness { value: 80, relative: false }),
            (
                "MultiAction",
                ActionConfig::MultiAction {
                    name: "seq".into(),
                    steps: vec![
                        ActionStep::Action { action: ActionConfig::Page { target: "monitor".into() } },
                        ActionStep::Delay { ms: 20 },
                        ActionStep::Action { action: ActionConfig::Theme { target: "llm_dash".into() } },
                    ],
                },
            ),
            (
                "Run",
                ActionConfig::Run {
                    program: "cmd.exe".into(),
                    args: vec!["/C".into(), "exit".into(), "0".into()],
                },
            ),
        ] {
            let h = handle.clone();
            let o = Arc::clone(&obs);
            let ev_n = Arc::clone(&events);
            let ev_l = Arc::clone(&events);
            std::thread::spawn(move || {
                execute_action(
                    &h,
                    action,
                    o,
                    Arc::new(move |cmd| ev_n.lock().unwrap().push(format!("NAV:{cmd:?}"))),
                    Arc::new(move |m| ev_l.lock().unwrap().push(m)),
                );
            })
            .join()
            .unwrap_or_else(|_| panic!("Thread OS entrou em pânico em {name}"));
        }

        assert!(wait_until(2000, || {
            let g = events.lock().unwrap();
            g.iter().any(|x| x.contains("Theme(\"llm_dash\")")) && g.len() >= 7
        }));
    }

    #[test]
    fn simulate_all_buttons_monitor_and_obs_pages() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .unwrap();
        let mock_port = rt.block_on(start_mock_obs_v5());
        let logs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let logs_obs = Arc::clone(&logs);
        let obs_cfg = Arc::new(RwLock::new(ObsConfig {
            host: "127.0.0.1".into(),
            port: mock_port,
            ..Default::default()
        }));
        let obs = {
            let _g = rt.enter();
            ObsClient::start(obs_cfg, Arc::new(|| {}), Arc::new(move |m| logs_obs.lock().unwrap().push(m)))
        };
        assert!(wait_until(3000, || obs.is_connected()), "OBS mock não conectou");

        let cfg = default_config();
        let pages = cfg.profiles.get("default").unwrap().pages.clone();
        let current_page = Arc::new(RwLock::new("monitor".to_string()));
        let handle = rt.handle().clone();

        let simulate_hid_click = |pos: u8, skip_run: bool| -> (String, Option<String>) {
            let pkt = build_v1_press_packet(pos);
            let mut v2 = HashSet::new();
            let (p, act) = parse_input_report(&pkt, true, &mut v2).expect("pacote v1 válido");
            assert_eq!(act, InputAction::Press);
            let gk = index_to_grid(p).expect("posição 1..18 válida");

            let h = handle.clone();
            let o = Arc::clone(&obs);
            let pg = Arc::clone(&current_page);
            let pages_c = pages.clone();
            let logs_c = Arc::clone(&logs);
            let gk_ret = gk.clone();

            let label = std::thread::spawn(move || {
                let page_name = pg.read().unwrap().clone();
                let tiles = pages_c.get(&page_name).unwrap();
                let tile = tiles.get(&gk).unwrap();
                let lbl = if !tile.title.is_empty() {
                    Some(tile.title.clone())
                } else {
                    tile.widget.clone()
                };
                if let Some(act_cfg) = tile.action.clone() {
                    if skip_run && matches!(act_cfg, ActionConfig::Run { .. }) {
                        return lbl;
                    }
                    let pg_nav = Arc::clone(&pg);
                    let pages_nav = pages_c.clone();
                    let l_nav = Arc::clone(&logs_c);
                    execute_action(
                        &h,
                        act_cfg,
                        o,
                        Arc::new(move |cmd| {
                            if let NavCmd::Page(target) = cmd {
                                let names: Vec<String> = pages_nav.keys().cloned().collect();
                                let cur = pg_nav.read().unwrap().clone();
                                let idx = names.iter().position(|n| n == &cur).unwrap_or(0);
                                let next = match target.as_str() {
                                    "next" => names[(idx + 1) % names.len()].clone(),
                                    "prev" => names[(idx + names.len() - 1) % names.len()].clone(),
                                    other if pages_nav.contains_key(other) => other.to_string(),
                                    _ => cur,
                                };
                                l_nav.lock().unwrap().push(format!("[NAV] -> {next}"));
                                *pg_nav.write().unwrap() = next;
                            }
                        }),
                        Arc::new(move |m| logs_c.lock().unwrap().push(m)),
                    );
                }
                lbl
            })
            .join()
            .unwrap();
            (gk_ret, label)
        };

        // 1. Página monitor: todas as 16 teclas passivas + 2 de navegação (pos=6 OBS logo, pos=3 seta >)
        for pos in 1u8..=18 {
            if pos == 6 || pos == 3 {
                continue;
            }
            let (gk, lbl) = simulate_hid_click(pos, true);
            assert_eq!(*current_page.read().unwrap(), "monitor", "Tecla passiva {gk} mudou página");
            println!("[SIM monitor] pos={pos:02} ({gk}) '{lbl:?}' -> PASS");
        }
        let (gk6, _) = simulate_hid_click(6, true);
        assert!(wait_until(1000, || *current_page.read().unwrap() == "obs"), "OBS logo ({gk6}) não abriu página obs");
        println!("[SIM monitor] pos=06 ({gk6}) obs_logo -> page=obs PASS");

        let (gk6b, _) = simulate_hid_click(6, true);
        assert!(wait_until(1000, || *current_page.read().unwrap() == "monitor"), "Voltar ({gk6b}) não voltou a monitor");
        println!("[SIM obs]     pos=06 ({gk6b}) nav_prev -> page=monitor PASS");

        let (gk3, _) = simulate_hid_click(3, true);
        assert!(wait_until(1000, || *current_page.read().unwrap() == "obs"), "Seta > ({gk3}) não abriu página obs");
        println!("[SIM monitor] pos=03 ({gk3}) nav_next -> page=obs PASS");

        // 2. Página obs: teclas de estado/widget passivas
        for pos in [15u8, 12, 16, 17, 18] {
            let (gk, lbl) = simulate_hid_click(pos, true);
            println!("[SIM obs]     pos={pos:02} ({gk}) '{lbl:?}' (widget) -> PASS");
        }

        // Gravar (pos=13 / 0,0) - ligar e desligar
        let obs_tiles = pages.get("obs").unwrap();
        let rec_tile = obs_tiles.get("0,0").unwrap();
        simulate_hid_click(13, true);
        assert!(wait_until(1000, || obs.state.read().unwrap().recording));
        assert_eq!(button_visual_state(rec_tile, &obs.state.read().unwrap(), &pages), ("active".into(), "PARAR".into(), "GRAVANDO".into()));
        println!("[SIM obs]     pos=13 (0,0) Gravar clique #1 -> GRAVANDO (active/PARAR) PASS");

        // Pausar (pos=10 / 1,0) enquanto gravando
        let pause_tile = obs_tiles.get("1,0").unwrap();
        assert_eq!(button_visual_state(pause_tile, &obs.state.read().unwrap(), &pages), ("idle".into(), "REC ATIVO".into(), "Pausar".into()));
        simulate_hid_click(10, true);
        assert!(wait_until(1000, || obs.state.read().unwrap().paused));
        assert_eq!(button_visual_state(pause_tile, &obs.state.read().unwrap(), &pages), ("warning".into(), "RETOMAR".into(), "PAUSADO".into()));
        simulate_hid_click(10, true);
        assert!(wait_until(1000, || !obs.state.read().unwrap().paused));
        println!("[SIM obs]     pos=10 (1,0) Pausar/Retomar -> warning/idle PASS");

        simulate_hid_click(13, true);
        assert!(wait_until(1000, || !obs.state.read().unwrap().recording));
        assert_eq!(button_visual_state(rec_tile, &obs.state.read().unwrap(), &pages), ("idle".into(), "CLIQUE".into(), "Gravar".into()));
        println!("[SIM obs]     pos=13 (0,0) Gravar clique #2 -> Parado (idle/CLIQUE) PASS");

        // Som Jogo (pos=7 / 2,0) - mutar e desmutar
        let mute_tile = obs_tiles.get("2,0").unwrap();
        simulate_hid_click(7, true);
        assert!(wait_until(1000, || obs.state.read().unwrap().muted.get("Áudio do Jogo (A50 X Game)").copied() == Some(true)));
        assert_eq!(button_visual_state(mute_tile, &obs.state.read().unwrap(), &pages), ("active".into(), "MUDO".into(), "Som Jogo".into()));
        simulate_hid_click(7, true);
        assert!(wait_until(1000, || obs.state.read().unwrap().muted.get("Áudio do Jogo (A50 X Game)").copied() == Some(false)));
        assert_eq!(button_visual_state(mute_tile, &obs.state.read().unwrap(), &pages), ("idle".into(), "ATIVO".into(), "Som Jogo".into()));
        println!("[SIM obs]     pos=07 (2,0) Som Jogo mute/unmute -> MUDO/ATIVO PASS");

        // Todas as 8 cenas OBS
        for (pos, gk, expected_scene) in [
            (4u8, "3,0", "AC-TrackTrainee"),
            (1, "4,0", "logo canal"),
            (14, "0,1", "1. ACC - Pro Studio (16:9)"),
            (11, "1,1", "2. ACC - Jogo Puro + Cam Sobreposta (16:9)"),
            (8, "2,1", "3. ACC - Lateral Split (16:9)"),
            (5, "3,1", "4. Vídeos - Foco Total (Centro 16:9)"),
            (2, "4,1", "5. Vídeos - Command Center (Multi-Screen)"),
            (9, "2,2", "6. Câmera Tela Cheia (4K)"),
        ] {
            let (got_gk, lbl) = simulate_hid_click(pos, true);
            assert_eq!(got_gk, gk);
            assert!(
                wait_until(1000, || obs.state.read().unwrap().current_scene == expected_scene),
                "Cena não trocou para {expected_scene}"
            );
            let st_btn = button_visual_state(obs_tiles.get(gk).unwrap(), &obs.state.read().unwrap(), &pages);
            assert_eq!(st_btn.0, "active");
            assert_eq!(st_btn.1, "ATIVA");
            println!("[SIM obs]     pos={pos:02} ({gk}) {lbl:?} -> '{expected_scene}' (active/ATIVA) PASS");
        }

        // Discord (pos=3 / 4,2)
        let discord_tile = obs_tiles.get("4,2").unwrap();
        match &discord_tile.action {
            Some(ActionConfig::Run { program, args }) => {
                assert_eq!(program, r"C:\Users\wagne\AppData\Local\Discord\Update.exe");
                assert_eq!(args, &vec!["--processStart".to_string(), "Discord.exe".to_string()]);
                println!("[SIM obs]     pos=03 (4,2) Discord Run config verificado PASS");
            }
            other => panic!("Ação inesperada em Discord: {other:?}"),
        }

        // Voltar (<) pos=6
        simulate_hid_click(6, true);
        assert!(wait_until(1000, || *current_page.read().unwrap() == "monitor"));
        println!("[SIM obs]     pos=06 (3,2) nav_prev -> voltou para monitor PASS");
    }
}
