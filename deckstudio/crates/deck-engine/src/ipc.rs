//! API IPC HTTP local (127.0.0.1:19876) entre o `deck-engine` e o editor `deck-studio`.

use crate::actions::execute_action;
use crate::device::index_to_grid;
use crate::hid_proto::{parse_input_report, INPUT_REQUEST_PREFIX};
use crate::state::EngineShared;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use deck_core::{
    import_ajazz_manifest_json, save_config, ActionConfig, AppConfig, EngineHealth,
    GridPreviewResponse, ImportAjazzRequest, ObsCatalog, SensorReading, SourceStatus,
};
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

pub async fn start_ipc_server(shared: Arc<EngineShared>) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
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
        .route("/api/import/ajazz", post(post_import_ajazz))
        .layer(cors)
        .with_state(shared);

    let addr = SocketAddr::from(([127, 0, 0, 1], 19876));
    let Ok(listener) = tokio::net::TcpListener::bind(addr).await else {
        return;
    };
    let _ = axum::serve(listener, app).await;
}

async fn get_health(State(st): State<Arc<EngineShared>>) -> Json<EngineHealth> {
    Json(st.build_health())
}

async fn get_config(State(st): State<Arc<EngineShared>>) -> Json<AppConfig> {
    let cfg = st.config.read().map(|g| g.clone()).unwrap_or_default();
    Json(cfg)
}

async fn put_config(
    State(st): State<Arc<EngineShared>>,
    Json(new_cfg): Json<AppConfig>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
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
    Ok(Json(new_cfg))
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

async fn post_import_ajazz(
    State(st): State<Arc<EngineShared>>,
    Json(req): Json<ImportAjazzRequest>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    let raw = std::fs::read_to_string(&req.path)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Falha ao ler arquivo: {e}")))?;
    let prof = import_ajazz_manifest_json(&raw, &req.profile_name)
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
    Ok(Json(updated))
}
