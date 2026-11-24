//! API IPC HTTP local (127.0.0.1:19876) entre o `deck-engine` e o editor `deck-studio`.

use crate::actions::execute_action;
use crate::device::index_to_grid;
use crate::hid_proto::{parse_input_report, INPUT_REQUEST_PREFIX};
use crate::state::EngineShared;
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderValue, Method, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use deck_core::{
    default_themes, find_previous_backups, import_stream_dock_manifest_json, save_config, ActionConfig,
    AppConfig, AutostartStatus, AutostartUpdate, BackupCandidate, EngineHealth,
    GridPreviewResponse, ImportManifestRequest, ObsCatalog, SensorReading, SourceStatus,
    UpdateCheckResponse, UpdateInstallStatus,
};
use serde::Deserialize;
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

async fn auth_and_host_middleware(
    State(st): State<Arc<EngineShared>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == Method::OPTIONS {
        return Ok(next.run(req).await);
    }
    match req.headers().get(header::HOST).and_then(|v| v.to_str().ok()) {
        Some("127.0.0.1:19876") | Some("localhost:19876") | Some("[::1]:19876") => {}
        _ => return Err(StatusCode::FORBIDDEN),
    }
    if req.uri().path() == "/api/ping" {
        return Ok(next.run(req).await);
    }
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    match auth {
        Some(tok) if !st.ipc_token.is_empty() && constant_time_eq(tok.as_bytes(), st.ipc_token.as_bytes()) => {
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn build_cors_layer() -> CorsLayer {
    let mut origins = vec![
        HeaderValue::from_static("http://tauri.localhost"),
        HeaderValue::from_static("https://tauri.localhost"),
        HeaderValue::from_static("tauri://localhost"),
    ];
    #[cfg(debug_assertions)]
    origins.push(HeaderValue::from_static("http://localhost:1420"));

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

pub fn build_ipc_router(shared: Arc<EngineShared>) -> Router {
    let cors = build_cors_layer();
    Router::new()
        .route("/api/ping", get(get_ping))
        .route("/api/health", get(get_health))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/sensors/sources", get(get_sources))
        .route("/api/sensors/readings", get(get_readings))
        .route("/api/obs/catalog", get(get_obs_catalog))
        .route("/api/preview/grid", get(get_preview_grid))
        .route("/api/actions/test", post(post_test_action))
        .route("/api/input/press/:pos", post(post_simulate_press))
        .route("/api/theme/:name", post(post_set_theme))
        .route("/api/brightness/:val", post(post_set_brightness))
        .route("/api/page/:name", post(post_set_page))
        .route("/api/profile/:name", post(post_set_profile))
        .route("/api/import/manifest", post(post_import_manifest))
        .route("/api/backup/export", get(get_backup_export))
        .route("/api/backup/import", post(post_backup_import))
        .route("/api/backup/sources", get(get_backup_sources))
        .route("/api/backup/restore", post(post_backup_restore))
        .route("/api/system/autostart", get(get_autostart).post(post_autostart))
        .route("/api/system/sensor-doctor", post(post_sensor_doctor))
        .route("/api/system/update/check", get(get_update_check))
        .route("/api/system/update/install", post(post_update_install))
        .route("/api/system/update/status", get(get_update_status))
        .layer(middleware::from_fn_with_state(Arc::clone(&shared), auth_and_host_middleware))
        .layer(cors)
        .with_state(shared)
}

pub async fn start_ipc_server(shared: Arc<EngineShared>) {
    let app = build_ipc_router(shared);
    let addr = SocketAddr::from(([127, 0, 0, 1], 19876));
    let Ok(listener) = tokio::net::TcpListener::bind(addr).await else {
        return;
    };
    let _ = axum::serve(listener, app).await;
}

async fn get_ping(
    State(st): State<Arc<EngineShared>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<String, StatusCode> {
    use sha2::{Digest, Sha256};
    let c = q.get("c").map(String::as_str).unwrap_or("");
    if c.len() < 8 || c.len() > 128 || !c.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut h = Sha256::new();
    h.update(st.ipc_token.as_bytes());
    h.update(b":");
    h.update(c.as_bytes());
    Ok(format!("{:x}", h.finalize()))
}

async fn get_health(State(st): State<Arc<EngineShared>>) -> Json<EngineHealth> {
    Json(st.build_health())
}

fn redact_config_for_ipc(mut cfg: AppConfig) -> AppConfig {
    cfg.obs.password.clear();
    cfg
}

fn is_unc_or_device_path(p: &str) -> bool {
    let t = p.trim();
    let norm = t.replace('/', "\\");
    t.contains('\0')
        || norm.starts_with(r"\\")
        || norm.starts_with(r"\?")
        || norm.starts_with(r"\.")
}

async fn get_config(State(st): State<Arc<EngineShared>>) -> Json<AppConfig> {
    let cfg = st.config.read().map(|g| g.clone()).unwrap_or_default();
    Json(redact_config_for_ipc(cfg))
}

async fn put_config(
    State(st): State<Arc<EngineShared>>,
    Json(mut new_cfg): Json<AppConfig>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    if new_cfg.obs.password.trim().is_empty() {
        if let Ok(cur) = st.config.read() {
            if new_cfg.obs.host == cur.obs.host
                && new_cfg.obs.port == cur.obs.port
                && new_cfg.obs.password_env == cur.obs.password_env
            {
                new_cfg.obs.password = cur.obs.password.clone();
            }
        }
    }
    save_config(&st.config_path, &new_cfg).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if let Ok(mut g) = st.config.write() {
        *g = new_cfg.clone();
    }
    if let Ok(mut w) = st.weather_cfg.write() {
        *w = new_cfg.weather.clone();
    }
    if let Ok(mut o) = st.obs_cfg.write() {
        *o = new_cfg.obs.clone();
    }
    st.sensors.lock().await.reset_backoff();
    st.clear_render_cache();
    st.device.set_brightness(new_cfg.device.brightness);
    st.device.invalidate_cache();
    st.notify_redraw();
    st.push_log("[CONFIG] Configuração atualizada pelo editor".into());
    Ok(Json(redact_config_for_ipc(new_cfg)))
}

async fn get_sources(State(st): State<Arc<EngineShared>>) -> Json<Vec<SourceStatus>> {
    let hub = st.sensors.lock().await;
    Json(hub.last_sources.clone())
}

async fn get_readings(State(st): State<Arc<EngineShared>>) -> Json<Vec<SensorReading>> {
    let hub = st.sensors.lock().await;
    Json(hub.last_readings.clone())
}

async fn get_obs_catalog(State(st): State<Arc<EngineShared>>) -> Json<ObsCatalog> {
    Json(st.obs.catalog())
}

async fn get_preview_grid(State(st): State<Arc<EngineShared>>) -> Json<GridPreviewResponse> {
    let nav = st.nav.read().map(|g| g.clone()).unwrap_or_default();
    let tiles = st.last_previews.read().map(|g| g.clone()).unwrap_or_default();
    let mut tiles_b64 = BTreeMap::new();
    for (k, rgb) in tiles {
        let mut png_bytes = Vec::with_capacity(4096);
        let enc = PngEncoder::new(&mut png_bytes);
        if enc
            .write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)
            .is_ok()
        {
            tiles_b64.insert(k, base64::engine::general_purpose::STANDARD.encode(&png_bytes));
        }
    }
    Json(GridPreviewResponse {
        profile: nav.profile,
        page: nav.page,
        folder_stack: nav.folder_stack,
        tiles_b64,
    })
}

async fn post_test_action(State(st): State<Arc<EngineShared>>, Json(act): Json<ActionConfig>) -> StatusCode {
    let st_nav = Arc::clone(&st);
    let st_log = Arc::clone(&st);
    execute_action(
        &st.rt_handle,
        act,
        Arc::clone(&st.obs),
        Arc::new(move |cmd| st_nav.handle_nav(cmd)),
        Arc::new(move |msg| st_log.push_log(msg)),
    );
    StatusCode::OK
}

#[derive(Serialize)]
struct SimulatePressResponse {
    pos: u8,
    grid_key: String,
    label: Option<String>,
    page_before: String,
    page_after: String,
    active_theme: String,
    brightness: u8,
}

async fn post_simulate_press(
    State(st): State<Arc<EngineShared>>,
    AxumPath(pos): AxumPath<u8>,
) -> Result<Json<SimulatePressResponse>, (StatusCode, String)> {
    let mut pkt = Vec::with_capacity(11);
    pkt.extend_from_slice(&INPUT_REQUEST_PREFIX);
    pkt.push(pos);
    pkt.push(0x00);

    let (parsed_pos, act) = parse_input_report(&pkt, true, &mut HashSet::new())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Pacote HID inválido para pos={pos}")))?;
    let grid_key = index_to_grid(parsed_pos)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Posição inválida: {parsed_pos}")))?;

    let page_before = st.nav.read().map(|g| g.page.clone()).unwrap_or_default();
    let st_thread = Arc::clone(&st);
    let gk_thread = grid_key.clone();
    let label = tokio::task::spawn_blocking(move || {
        std::thread::spawn(move || st_thread.dispatch_button_press(parsed_pos, &gk_thread, act))
            .join()
            .unwrap_or(None)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let page_after = st.nav.read().map(|g| g.page.clone()).unwrap_or_default();
    let active_theme = st.config.read().map(|g| g.active_theme.clone()).unwrap_or_default();
    let brightness = st.brightness.load(std::sync::atomic::Ordering::Relaxed);

    Ok(Json(SimulatePressResponse {
        pos: parsed_pos,
        grid_key,
        label,
        page_before,
        page_after,
        active_theme,
        brightness,
    }))
}

async fn post_set_theme(State(st): State<Arc<EngineShared>>, AxumPath(name): AxumPath<String>) -> StatusCode {
    st.set_theme(&name);
    StatusCode::OK
}

async fn post_set_brightness(State(st): State<Arc<EngineShared>>, AxumPath(val): AxumPath<u8>) -> StatusCode {
    st.set_brightness(val);
    StatusCode::OK
}

async fn post_set_page(State(st): State<Arc<EngineShared>>, AxumPath(name): AxumPath<String>) -> StatusCode {
    st.handle_nav(crate::actions::NavCmd::Page(name));
    StatusCode::OK
}

async fn post_set_profile(State(st): State<Arc<EngineShared>>, AxumPath(name): AxumPath<String>) -> StatusCode {
    st.set_profile(&name, None);
    StatusCode::OK
}

async fn post_import_manifest(
    State(st): State<Arc<EngineShared>>,
    Json(req): Json<ImportManifestRequest>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    if is_unc_or_device_path(&req.path) {
        return Err((StatusCode::BAD_REQUEST, "Caminho UNC ou inválido não permitido".into()));
    }
    let p = std::path::Path::new(&req.path);
    if !p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        return Err((StatusCode::BAD_REQUEST, "Apenas arquivos .json são permitidos".into()));
    }
    let canon = std::fs::canonicalize(p)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Arquivo não encontrado ou inacessível".into()))?;
    if canon.to_string_lossy().starts_with(r"\\?\UNC\") || !canon.is_file() {
        return Err((StatusCode::BAD_REQUEST, "Apenas arquivos locais regulares são permitidos".into()));
    }
    use std::io::Read as _;
    let mut f = std::fs::File::open(&canon)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Falha ao abrir arquivo".into()))?;
    let mut raw = String::new();
    f.by_ref()
        .take(1_048_576)
        .read_to_string(&mut raw)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Falha ao ler arquivo".into()))?;
    let prof = import_stream_dock_manifest_json(&raw, &req.profile_name)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let updated = {
        let mut cfg = st.config.write().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "lock".into()))?;
        cfg.profiles.insert(req.profile_id.clone(), prof);
        cfg.active_profile = req.profile_id;
        let _ = save_config(&st.config_path, &cfg);
        cfg.clone()
    };
    st.device.clear_all();
    st.notify_redraw();
    Ok(Json(redact_config_for_ipc(updated)))
}

#[derive(Serialize, Deserialize)]
struct BackupPackage {
    exported_at: String,
    version: String,
    config: AppConfig,
}

#[derive(Deserialize)]
struct RestoreBackupRequest {
    path: String,
}

async fn get_backup_export(State(st): State<Arc<EngineShared>>) -> Result<Json<BackupPackage>, StatusCode> {
    let cfg = st.config.read().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.clone();
    Ok(Json(BackupPackage {
        exported_at: chrono::Local::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").into(),
        config: redact_config_for_ipc(cfg),
    }))
}

fn apply_imported_config(st: &Arc<EngineShared>, mut cfg: AppConfig) -> Result<AppConfig, (StatusCode, String)> {
    for (k, v) in default_themes() {
        cfg.themes.entry(k).or_insert(v);
    }
    if cfg.obs.password.trim().is_empty() {
        if let Ok(cur) = st.config.read() {
            if cfg.obs.host == cur.obs.host
                && cfg.obs.port == cur.obs.port
                && cfg.obs.password_env == cur.obs.password_env
            {
                cfg.obs.password = cur.obs.password.clone();
            }
        }
    }
    if let Ok(mut w) = st.weather_cfg.write() {
        *w = cfg.weather.clone();
    }
    if let Ok(mut o) = st.obs_cfg.write() {
        *o = cfg.obs.clone();
    }
    if let Ok(mut g) = st.config.write() {
        *g = cfg.clone();
    }
    save_config(&st.config_path, &cfg).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    st.clear_render_cache();
    st.device.clear_all();
    st.notify_redraw();
    st.push_log("[BACKUP] Configuração restaurada/importada com sucesso".into());
    Ok(redact_config_for_ipc(cfg))
}

async fn post_backup_import(
    State(st): State<Arc<EngineShared>>,
    Json(raw): Json<serde_json::Value>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    let cfg: AppConfig = if let Some(inner) = raw.get("config") {
        serde_json::from_value(inner.clone())
    } else {
        serde_json::from_value(raw)
    }
    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Backup JSON inválido: {e}")))?;

    let applied = apply_imported_config(&st, cfg)?;
    Ok(Json(applied))
}

async fn get_backup_sources(State(st): State<Arc<EngineShared>>) -> Json<Vec<BackupCandidate>> {
    Json(find_previous_backups(&st.config_path))
}

async fn post_backup_restore(
    State(st): State<Arc<EngineShared>>,
    Json(req): Json<RestoreBackupRequest>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    if is_unc_or_device_path(&req.path) {
        return Err((StatusCode::BAD_REQUEST, "Caminho UNC não permitido".into()));
    }
    let matched_path = find_previous_backups(&st.config_path)
        .into_iter()
        .find(|c| c.path.eq_ignore_ascii_case(req.path.trim()))
        .map(|c| c.path)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Caminho de backup fora da lista permitida".to_string()))?;
    let req_canon = std::fs::canonicalize(&matched_path)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Backup não encontrado".into()))?;
    let raw = std::fs::read_to_string(&req_canon)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Falha ao ler backup".into()))?;
    let val: serde_json::Value =
        serde_json::from_str(&raw).map_err(|_| (StatusCode::BAD_REQUEST, "JSON de backup inválido".into()))?;
    let cfg: AppConfig = if let Some(inner) = val.get("config") {
        serde_json::from_value(inner.clone())
    } else {
        serde_json::from_value(val)
    }
    .map_err(|_| (StatusCode::BAD_REQUEST, "Formato de config inválido".into()))?;

    let applied = apply_imported_config(&st, cfg)?;
    Ok(Json(applied))
}

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const REG_NAME_ENGINE: &str = "DeckStudioEngine";
const REG_NAME_DASH: &str = "DeckStudioDash";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn system32_exe(name: &str) -> std::path::PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    std::path::PathBuf::from(root).join("System32").join(name)
}

fn reg_value_exists(name: &str) -> bool {
    let mut cmd = std::process::Command::new(system32_exe("reg.exe"));
    cmd.args(["query", RUN_KEY, "/v", name]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

fn set_reg_run(name: &str, exe_path: Option<&str>) -> Result<(), String> {
    let mut cmd = std::process::Command::new(system32_exe("reg.exe"));
    if let Some(p) = exe_path {
        let quoted = format!("\"{p}\"");
        cmd.args(["add", RUN_KEY, "/v", name, "/t", "REG_SZ", "/d", &quoted, "/f"]);
    } else {
        cmd.args(["delete", RUN_KEY, "/v", name, "/f"]);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() && exe_path.is_some() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

fn resolve_exe_paths() -> (String, String) {
    let exe = std::env::current_exe().unwrap_or_default();
    let engine_str = exe.to_string_lossy().to_string();
    let dir = exe.parent().unwrap_or_else(|| std::path::Path::new("."));

    let mut candidates = vec![dir.join("DeckStudio.exe"), dir.join("deck-studio.exe")];
    #[cfg(debug_assertions)]
    {
        candidates.push(dir.join("../../apps/studio/src-tauri/target/release/deck-studio.exe"));
        candidates.push(dir.join("../../../apps/studio/src-tauri/target/release/deck-studio.exe"));
    }
    let dash_str = candidates
        .iter()
        .find(|p| p.exists())
        .and_then(|p| std::fs::canonicalize(p).ok())
        .map(|p| {
            let s = p.to_string_lossy().to_string();
            s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
        })
        .unwrap_or_else(|| dir.join("DeckStudio.exe").to_string_lossy().to_string());

    (engine_str, dash_str)
}

fn build_autostart_status() -> AutostartStatus {
    let (engine_path, dash_path) = resolve_exe_paths();
    AutostartStatus {
        engine_enabled: reg_value_exists(REG_NAME_ENGINE),
        dash_enabled: reg_value_exists(REG_NAME_DASH),
        engine_path,
        dash_path,
    }
}

async fn get_autostart() -> Json<AutostartStatus> {
    Json(build_autostart_status())
}

async fn post_autostart(
    State(st): State<Arc<EngineShared>>,
    Json(req): Json<AutostartUpdate>,
) -> Result<Json<AutostartStatus>, (StatusCode, String)> {
    let (engine_path, dash_path) = resolve_exe_paths();
    set_reg_run(REG_NAME_ENGINE, if req.engine_enabled { Some(&engine_path) } else { None })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    set_reg_run(REG_NAME_DASH, if req.dash_enabled { Some(&dash_path) } else { None })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    st.push_log(format!(
        "[BOOT] Auto-start atualizado (Engine={}, Dash={})",
        req.engine_enabled, req.dash_enabled
    ));
    Ok(Json(build_autostart_status()))
}

async fn post_sensor_doctor(State(st): State<Arc<EngineShared>>) -> Result<StatusCode, (StatusCode, String)> {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut candidates = vec![
        dir.join("scripts").join("sensor-doctor.ps1"),
        dir.join("sensor-doctor.ps1"),
    ];
    #[cfg(debug_assertions)]
    {
        candidates.push(dir.join("../../../scripts/sensor-doctor.ps1"));
        candidates.push(dir.join("../../scripts/sensor-doctor.ps1"));
    }
    let Some(script_path) = candidates.into_iter().find(|p| p.exists()) else {
        return Err((StatusCode::NOT_FOUND, "Script sensor-doctor.ps1 não encontrado".into()));
    };
    let ps_exe = system32_exe(r"WindowsPowerShell\v1.0\powershell.exe");
    let mut cmd = std::process::Command::new(ps_exe);
    cmd.args(["-NoProfile", "-NoExit", "-File"]).arg(&script_path);
    cmd.spawn().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    st.push_log("[SENSORES] Script Sensor Doctor aberto em nova janela PowerShell".into());
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct InstallUpdateRequest {
    download_url: String,
    sha256: Option<String>,
}

async fn get_update_check(State(st): State<Arc<EngineShared>>) -> Json<UpdateCheckResponse> {
    let feed_url = st
        .config
        .read()
        .map(|g| g.update_feed_url.clone())
        .unwrap_or_else(|_| deck_core::default_update_feed_url());
    let res = crate::updater::check_for_update(&feed_url).await;
    if res.update_available {
        if let Ok(mut g) = st.last_update_check.write() {
            *g = Some(res.clone());
        }
    }
    Json(res)
}

async fn get_update_status(State(st): State<Arc<EngineShared>>) -> Json<UpdateInstallStatus> {
    let snap = st.update_status.read().map(|g| g.clone()).unwrap_or_default();
    Json(snap)
}

async fn post_update_install(
    State(st): State<Arc<EngineShared>>,
    Json(req): Json<InstallUpdateRequest>,
) -> StatusCode {
    if !crate::updater::is_trusted_download_url(&req.download_url)
        || !crate::updater::is_valid_sha256(req.sha256.as_deref())
    {
        return StatusCode::BAD_REQUEST;
    }
    let pinned_ok = st
        .last_update_check
        .read()
        .ok()
        .and_then(|g| g.clone())
        .map(|chk| {
            chk.update_available
                && chk.download_url == req.download_url
                && chk.sha256.as_deref().map(str::to_ascii_lowercase)
                    == req.sha256.as_deref().map(str::to_ascii_lowercase)
        })
        .unwrap_or(false);
    if !pinned_ok {
        return StatusCode::BAD_REQUEST;
    }
    if st
        .update_in_flight
        .compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
        .is_err()
    {
        return StatusCode::CONFLICT;
    }
    let status_arc = Arc::clone(&st.update_status);
    let st_flag = Arc::clone(&st);
    st.push_log("[UPDATER] Baixando atualização oficial verificada".into());
    st.rt_handle.spawn(async move {
        crate::updater::download_and_install_update(req.download_url, req.sha256, status_arc).await;
        st_flag.update_in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
    });
    StatusCode::ACCEPTED
}
