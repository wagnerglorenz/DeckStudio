//! Cliente nativo OBS WebSocket v5 via tokio-tungstenite com reconexão automática e auto-descoberta.

use base64::Engine as _;
use deck_core::{ObsCatalog, ObsConfig};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsSceneItem {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ObsState {
    pub connected: bool,
    pub recording: bool,
    pub paused: bool,
    pub streaming: bool,
    pub virtual_cam: bool,
    pub studio_mode: bool,
    pub current_scene: String,
    pub scenes: Vec<String>,
    pub audio_inputs: Vec<String>,
    pub muted: HashMap<String, bool>,
    pub scene_items: HashMap<String, Vec<ObsSceneItem>>,
    pub all_sources: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ObsCmd {
    ToggleRecord,
    ToggleRecordPause,
    ToggleStream,
    ToggleVirtualCam,
    ToggleReplay,
    SaveReplay,
    ToggleStudioMode,
    TriggerStudioTransition,
    SetScene(String),
    ToggleMute(String),
    ToggleSource { scene: Option<String>, source: String },
    SetSourceEnabled { scene: Option<String>, source: String, enabled: bool },
}

pub struct ObsClient {
    pub state: Arc<RwLock<ObsState>>,
    tx: Mutex<Option<mpsc::UnboundedSender<ObsCmd>>>,
}

impl ObsClient {
    pub fn start(
        cfg: Arc<RwLock<ObsConfig>>,
        on_event: Arc<dyn Fn() + Send + Sync>,
        on_log: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Arc<Self> {
        let state = Arc::new(RwLock::new(ObsState::default()));
        let client = Arc::new(Self {
            state: Arc::clone(&state),
            tx: Mutex::new(None),
        });
        let weak = Arc::clone(&client);
        tokio::spawn(async move {
            loop {
                let c = cfg.read().map(|g| g.clone()).unwrap_or_default();
                if let Err(e) = Self::run_session(&weak, &c, &on_event, &on_log).await {
                    let was_conn = state.read().map(|s| s.connected).unwrap_or(false);
                    if let Ok(mut s) = state.write() {
                        s.connected = false;
                    }
                    if was_conn {
                        on_log(format!("[OBS] Desconectado ({e}); reconectando em 10s"));
                        on_event();
                    }
                }
                sleep(Duration::from_secs(10)).await;
            }
        });
        client
    }

    pub fn is_connected(&self) -> bool {
        self.state.read().map(|s| s.connected).unwrap_or(false)
    }

    pub fn send(&self, cmd: ObsCmd) {
        if let Ok(guard) = self.tx.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(cmd);
            }
        }
    }

    pub fn catalog(&self) -> ObsCatalog {
        let s = self.state.read().map(|g| g.clone()).unwrap_or_default();
        ObsCatalog {
            connected: s.connected,
            current_scene: s.current_scene,
            scenes: s.scenes,
            audio_inputs: s.audio_inputs,
            sources: s.all_sources,
        }
    }

    async fn run_session(
        &self,
        cfg: &ObsConfig,
        on_event: &Arc<dyn Fn() + Send + Sync>,
        on_log: &Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<(), String> {
        let host = cfg.host.trim();
        if host.is_empty()
            || host.contains(char::is_whitespace)
            || host.contains('@')
            || host.contains('/')
            || host.contains('\\')
            || host.contains('?')
            || host.contains('#')
        {
            return Err("Host OBS inválido".into());
        }
        let url = format!("ws://{}:{}", host, cfg.port);
        let (ws_stream, _) = tokio::time::timeout(
            Duration::from_secs(5),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        .map_err(|_| "connect timeout".to_string())?
        .map_err(|e| format!("connect: {e}"))?;
        let (mut write, mut read) = ws_stream.split();

        // 1. Hello (op 0)
        let hello_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
            .await
            .map_err(|_| "Hello timeout".to_string())?
            .ok_or_else(|| "EOF antes do Hello".to_string())?
            .map_err(|e| e.to_string())?;
        let hello: Value = serde_json::from_str(hello_msg.to_text().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        // Subscreve a eventos: General (1), Scenes (4), Inputs (8), Transitions (16), Outputs (64), SceneItems (128), UI (1024)
        let mut identify_d = json!({
            "rpcVersion": 1,
            "eventSubscriptions": 1 | 4 | 8 | 16 | 64 | 128 | 1024
        });

        if let Some(auth) = hello.pointer("/d/authentication") {
            let salt = auth.get("salt").and_then(|v| v.as_str()).unwrap_or("");
            let challenge = auth.get("challenge").and_then(|v| v.as_str()).unwrap_or("");
            let pw = if !cfg.password.is_empty() {
                cfg.password.clone()
            } else {
                std::env::var(&cfg.password_env).unwrap_or_default()
            };
            let secret = sha256_b64(format!("{pw}{salt}").as_bytes());
            let auth_resp = sha256_b64(format!("{secret}{challenge}").as_bytes());
            identify_d["authentication"] = Value::String(auth_resp);
        }

        // 2. Identify (op 1)
        let identify = json!({ "op": 1, "d": identify_d });
        write
            .send(Message::Text(identify.to_string()))
            .await
            .map_err(|e| e.to_string())?;

        // Aguarda Identified (op 2)
        loop {
            let msg = read
                .next()
                .await
                .ok_or_else(|| "EOF antes do Identified".to_string())?
                .map_err(|e| e.to_string())?;
            if let Ok(txt) = msg.to_text() {
                let v: Value = serde_json::from_str(txt).unwrap_or_default();
                if v.get("op").and_then(|x| x.as_i64()) == Some(2) {
                    break;
                }
            }
        }

        if let Ok(mut s) = self.state.write() {
            s.connected = true;
        }
        on_log(format!("[OBS] Conectado em {url}"));

        // Solicita catálogo e estado inicial
        for (req_id, req_type) in [
            ("init_rec", "GetRecordStatus"),
            ("init_stm", "GetStreamStatus"),
            ("init_vcam", "GetVirtualCamStatus"),
            ("init_stmode", "GetStudioModeEnabled"),
            ("init_scn", "GetSceneList"),
            ("init_inp", "GetInputList"),
        ] {
            let req = json!({
                "op": 6,
                "d": { "requestType": req_type, "requestId": req_id }
            });
            write
                .send(Message::Text(req.to_string()))
                .await
                .map_err(|e| e.to_string())?;
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<ObsCmd>();
        if let Ok(mut guard) = self.tx.lock() {
            *guard = Some(tx);
        }

        let mut req_counter = 100u64;
        loop {
            tokio::select! {
                Some(cmd) = rx.recv() => {
                    req_counter += 1;
                    let req_id = format!("cmd_{req_counter}");
                    match cmd {
                        ObsCmd::ToggleRecord => {
                            send_req(&mut write, "ToggleRecord", req_id, json!({})).await?;
                        }
                        ObsCmd::ToggleRecordPause => {
                            send_req(&mut write, "ToggleRecordPause", req_id, json!({})).await?;
                        }
                        ObsCmd::ToggleStream => {
                            send_req(&mut write, "ToggleStream", req_id, json!({})).await?;
                        }
                        ObsCmd::ToggleVirtualCam => {
                            send_req(&mut write, "ToggleVirtualCam", req_id, json!({})).await?;
                        }
                        ObsCmd::ToggleReplay => {
                            send_req(&mut write, "ToggleReplayBuffer", req_id, json!({})).await?;
                        }
                        ObsCmd::SaveReplay => {
                            send_req(&mut write, "SaveReplayBuffer", req_id, json!({})).await?;
                        }
                        ObsCmd::ToggleStudioMode => {
                            send_req(&mut write, "ToggleStudioMode", req_id, json!({})).await?;
                        }
                        ObsCmd::TriggerStudioTransition => {
                            send_req(&mut write, "TriggerStudioModeTransition", req_id, json!({})).await?;
                        }
                        ObsCmd::SetScene(name) => {
                            send_req(&mut write, "SetCurrentProgramScene", req_id, json!({"sceneName": name})).await?;
                        }
                        ObsCmd::ToggleMute(inp) => {
                            send_req(&mut write, "ToggleInputMute", req_id, json!({"inputName": inp})).await?;
                        }
                        ObsCmd::ToggleSource { scene, source } => {
                            let sc_name = scene.unwrap_or_else(|| {
                                self.state.read().map(|s| s.current_scene.clone()).unwrap_or_default()
                            });
                            let cached = self.state.read().ok().and_then(|st| {
                                st.scene_items.get(&sc_name).and_then(|items| {
                                    items.iter().find(|it| it.name.eq_ignore_ascii_case(&source)).cloned()
                                })
                            });
                            if let Some(item) = cached {
                                send_req(&mut write, "SetSceneItemEnabled", req_id, json!({
                                    "sceneName": sc_name,
                                    "sceneItemId": item.id,
                                    "sceneItemEnabled": !item.enabled
                                })).await?;
                            } else {
                                // Solicita item ID para alternar via resposta
                                send_req(&mut write, "GetSceneItemId", format!("togsrc:{sc_name}:{source}"), json!({
                                    "sceneName": sc_name,
                                    "sourceName": source
                                })).await?;
                            }
                        }
                        ObsCmd::SetSourceEnabled { scene, source, enabled } => {
                            let sc_name = scene.unwrap_or_else(|| {
                                self.state.read().map(|s| s.current_scene.clone()).unwrap_or_default()
                            });
                            let cached = self.state.read().ok().and_then(|st| {
                                st.scene_items.get(&sc_name).and_then(|items| {
                                    items.iter().find(|it| it.name.eq_ignore_ascii_case(&source)).cloned()
                                })
                            });
                            if let Some(item) = cached {
                                send_req(&mut write, "SetSceneItemEnabled", req_id, json!({
                                    "sceneName": sc_name,
                                    "sceneItemId": item.id,
                                    "sceneItemEnabled": enabled
                                })).await?;
                            } else {
                                let op_flag = if enabled { "1" } else { "0" };
                                send_req(&mut write, "GetSceneItemId", format!("setsrc:{op_flag}:{sc_name}:{source}"), json!({
                                    "sceneName": sc_name,
                                    "sourceName": source
                                })).await?;
                            }
                        }
                    }
                }
                msg_opt = read.next() => {
                    let Some(msg_res) = msg_opt else { break; };
                    let msg = msg_res.map_err(|e| e.to_string())?;
                    if let Ok(txt) = msg.to_text() {
                        if let Ok(v) = serde_json::from_str::<Value>(txt) {
                            let op = v.get("op").and_then(|x| x.as_i64()).unwrap_or(-1);
                            if op == 5 {
                                Self::handle_event(&self.state, &v, &mut write).await;
                                on_event();
                            } else if op == 7 {
                                Self::handle_response(&self.state, &v, &mut write).await;
                                on_event();
                            }
                        }
                    }
                }
            }
        }

        Err("Conexão encerrada".into())
    }

    async fn handle_event<S>(state: &Arc<RwLock<ObsState>>, v: &Value, write: &mut S)
    where
        S: futures_util::Sink<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        let etype = v.pointer("/d/eventType").and_then(|x| x.as_str()).unwrap_or("");
        let edata = v.pointer("/d/eventData");
        let mut need_refresh = false;

        if let Ok(mut s) = state.write() {
            match etype {
                "RecordStateChanged" => {
                    let st_str = edata.and_then(|d| d.get("outputState")).and_then(|x| x.as_str()).unwrap_or("");
                    let active = edata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()).unwrap_or(false);
                    match st_str {
                        "OBS_WEBSOCKET_OUTPUT_PAUSED" => { s.recording = true; s.paused = true; }
                        "OBS_WEBSOCKET_OUTPUT_RESUMED" | "OBS_WEBSOCKET_OUTPUT_STARTED" => { s.recording = true; s.paused = false; }
                        "OBS_WEBSOCKET_OUTPUT_STOPPED" | "OBS_WEBSOCKET_OUTPUT_STOPPING" => { s.recording = false; s.paused = false; }
                        _ => { s.recording = active; if !active { s.paused = false; } }
                    }
                }
                "StreamStateChanged" => {
                    if let Some(a) = edata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()) {
                        s.streaming = a;
                    }
                }
                "VirtualcamStateChanged" => {
                    if let Some(a) = edata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()) {
                        s.virtual_cam = a;
                    }
                }
                "StudioModeStateChanged" => {
                    if let Some(a) = edata.and_then(|d| d.get("studioModeEnabled")).and_then(|x| x.as_bool()) {
                        s.studio_mode = a;
                    }
                }
                "CurrentProgramSceneChanged" => {
                    if let Some(name) = edata.and_then(|d| d.get("sceneName")).and_then(|x| x.as_str()) {
                        s.current_scene = name.to_string();
                    }
                }
                "InputMuteStateChanged" => {
                    if let (Some(name), Some(m)) = (
                        edata.and_then(|d| d.get("inputName")).and_then(|x| x.as_str()),
                        edata.and_then(|d| d.get("inputMuted")).and_then(|x| x.as_bool()),
                    ) {
                        s.muted.insert(name.to_string(), m);
                    }
                }
                "SceneItemEnableStateChanged" => {
                    if let (Some(sc), Some(id), Some(en)) = (
                        edata.and_then(|d| d.get("sceneName")).and_then(|x| x.as_str()),
                        edata.and_then(|d| d.get("sceneItemId")).and_then(|x| x.as_i64()),
                        edata.and_then(|d| d.get("sceneItemEnabled")).and_then(|x| x.as_bool()),
                    ) {
                        if let Some(items) = s.scene_items.get_mut(sc) {
                            if let Some(it) = items.iter_mut().find(|i| i.id == id) {
                                it.enabled = en;
                            }
                        }
                    }
                }
                "SceneItemListReindexed" | "SceneCreated" | "SceneRemoved" | "SceneNameChanged" => {
                    need_refresh = true;
                }
                _ => {}
            }
        }

        if need_refresh {
            let req = json!({
                "op": 6,
                "d": { "requestType": "GetSceneList", "requestId": "evt_refresh_scenes" }
            });
            let _ = write.send(Message::Text(req.to_string())).await;
        }
    }

    async fn handle_response<S>(state: &Arc<RwLock<ObsState>>, v: &Value, write: &mut S)
    where
        S: futures_util::Sink<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        let rtype = v.pointer("/d/requestType").and_then(|x| x.as_str()).unwrap_or("");
        let rdata = v.pointer("/d/responseData");
        let rid = v.pointer("/d/requestId").and_then(|x| x.as_str()).unwrap_or("");

        let mut inputs_to_query = Vec::new();
        let mut scenes_to_query = Vec::new();
        let mut extra_requests = Vec::new();

        if let Ok(mut s) = state.write() {
            match rtype {
                "GetRecordStatus" => {
                    s.recording = rdata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()).unwrap_or(false);
                    s.paused = rdata.and_then(|d| d.get("outputPaused")).and_then(|x| x.as_bool()).unwrap_or(false);
                }
                "GetStreamStatus" => {
                    s.streaming = rdata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()).unwrap_or(false);
                }
                "GetVirtualCamStatus" => {
                    s.virtual_cam = rdata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()).unwrap_or(false);
                }
                "GetStudioModeEnabled" => {
                    s.studio_mode = rdata.and_then(|d| d.get("studioModeEnabled")).and_then(|x| x.as_bool()).unwrap_or(false);
                }
                "GetSceneList" => {
                    s.current_scene = rdata.and_then(|d| d.get("currentProgramSceneName")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                    if let Some(arr) = rdata.and_then(|d| d.get("scenes")).and_then(|x| x.as_array()) {
                        s.scenes = arr.iter().filter_map(|it| it.get("sceneName").and_then(|x| x.as_str()).map(|s| s.to_string())).collect();
                        scenes_to_query = s.scenes.clone();
                    }
                }
                "GetSceneItemList" => {
                    if let Some(sc_name) = rid.strip_prefix("scitems_") {
                        if let Some(arr) = rdata.and_then(|d| d.get("sceneItems")).and_then(|x| x.as_array()) {
                            let items: Vec<ObsSceneItem> = arr.iter().filter_map(|it| {
                                let id = it.get("sceneItemId").and_then(|x| x.as_i64())?;
                                let name = it.get("sourceName").and_then(|x| x.as_str())?.to_string();
                                let enabled = it.get("sceneItemEnabled").and_then(|x| x.as_bool()).unwrap_or(true);
                                Some(ObsSceneItem { id, name, enabled })
                            }).collect();
                            s.scene_items.insert(sc_name.to_string(), items);

                            // Atualiza lista única de todas as fontes
                            let mut all_set = BTreeSet::new();
                            for list in s.scene_items.values() {
                                for item in list {
                                    all_set.insert(item.name.clone());
                                }
                            }
                            s.all_sources = all_set.into_iter().collect();
                        }
                    }
                }
                "GetInputList" => {
                    if let Some(arr) = rdata.and_then(|d| d.get("inputs")).and_then(|x| x.as_array()) {
                        s.audio_inputs = arr
                            .iter()
                            .filter_map(|it| it.get("inputName").and_then(|x| x.as_str()).map(|n| n.to_string()))
                            .collect();
                        inputs_to_query = s.audio_inputs.clone();
                    }
                }
                "GetInputMute" => {
                    if let Some(name) = rid.strip_prefix("mute_") {
                        if let Some(m) = rdata.and_then(|d| d.get("inputMuted")).and_then(|x| x.as_bool()) {
                            s.muted.insert(name.to_string(), m);
                        }
                    }
                }
                "GetSceneItemId" => {
                    if let Some(id) = rdata.and_then(|d| d.get("sceneItemId")).and_then(|x| x.as_i64()) {
                        if let Some(rest) = rid.strip_prefix("togsrc:") {
                            let parts: Vec<&str> = rest.splitn(2, ':').collect();
                            if parts.len() == 2 {
                                let (sc_name, source) = (parts[0], parts[1]);
                                let curr_en = s.scene_items.get(sc_name).and_then(|l| {
                                    l.iter().find(|i| i.id == id).map(|i| i.enabled)
                                }).unwrap_or(false);
                                extra_requests.push(json!({
                                    "op": 6,
                                    "d": {
                                        "requestType": "SetSceneItemEnabled",
                                        "requestId": format!("set_tog_{id}"),
                                        "requestData": {
                                            "sceneName": sc_name,
                                            "sceneItemId": id,
                                            "sceneItemEnabled": !curr_en
                                        }
                                    }
                                }));
                                if let Some(l) = s.scene_items.get_mut(sc_name) {
                                    if let Some(i) = l.iter_mut().find(|i| i.id == id) {
                                        i.enabled = !curr_en;
                                    } else {
                                        l.push(ObsSceneItem { id, name: source.to_string(), enabled: !curr_en });
                                    }
                                }
                            }
                        } else if let Some(rest) = rid.strip_prefix("setsrc:") {
                            let parts: Vec<&str> = rest.splitn(3, ':').collect();
                            if parts.len() == 3 {
                                let (flag, sc_name, source) = (parts[0], parts[1], parts[2]);
                                let target_en = flag == "1";
                                extra_requests.push(json!({
                                    "op": 6,
                                    "d": {
                                        "requestType": "SetSceneItemEnabled",
                                        "requestId": format!("set_en_{id}"),
                                        "requestData": {
                                            "sceneName": sc_name,
                                            "sceneItemId": id,
                                            "sceneItemEnabled": target_en
                                        }
                                    }
                                }));
                                if let Some(l) = s.scene_items.get_mut(sc_name) {
                                    if let Some(i) = l.iter_mut().find(|i| i.id == id) {
                                        i.enabled = target_en;
                                    } else {
                                        l.push(ObsSceneItem { id, name: source.to_string(), enabled: target_en });
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Executa envios fora do lock do estado
        for req in extra_requests {
            let _ = write.send(Message::Text(req.to_string())).await;
        }

        // Carrega itens de cada cena
        for sc in scenes_to_query {
            let req = json!({
                "op": 6,
                "d": {
                    "requestType": "GetSceneItemList",
                    "requestId": format!("scitems_{sc}"),
                    "requestData": { "sceneName": sc }
                }
            });
            let _ = write.send(Message::Text(req.to_string())).await;
        }

        // Carrega estado de mute de cada entrada de áudio
        for inp in inputs_to_query {
            let req = json!({
                "op": 6,
                "d": {
                    "requestType": "GetInputMute",
                    "requestId": format!("mute_{inp}"),
                    "requestData": { "inputName": inp }
                }
            });
            let _ = write.send(Message::Text(req.to_string())).await;
        }
    }
}

async fn send_req<S>(write: &mut S, rtype: &str, req_id: String, rdata: Value) -> Result<(), String>
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let msg = json!({
        "op": 6,
        "d": { "requestType": rtype, "requestId": req_id, "requestData": rdata }
    });
    write
        .send(Message::Text(msg.to_string()))
        .await
        .map_err(|e| e.to_string())
}

fn sha256_b64(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(digest)
}
