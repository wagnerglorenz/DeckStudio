//! Cliente nativo OBS WebSocket v5 via tokio-tungstenite com reconexão automática e auto-descoberta.

use base64::Engine as _;
use deck_core::{ObsCatalog, ObsConfig};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Default)]
pub struct ObsState {
    pub connected: bool,
    pub recording: bool,
    pub paused: bool,
    pub streaming: bool,
    pub current_scene: String,
    pub scenes: Vec<String>,
    pub audio_inputs: Vec<String>,
    pub muted: HashMap<String, bool>,
}

#[derive(Debug, Clone)]
pub enum ObsCmd {
    ToggleRecord,
    ToggleRecordPause,
    ToggleStream,
    SetScene(String),
    ToggleMute(String),
    SaveReplay,
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
        }
    }

    async fn run_session(
        &self,
        cfg: &ObsConfig,
        on_event: &Arc<dyn Fn() + Send + Sync>,
        on_log: &Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<(), String> {
        let url = format!("ws://{}:{}", cfg.host, cfg.port);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| format!("connect: {e}"))?;
        let (mut write, mut read) = ws_stream.split();

        // 1. Hello (op 0)
        let hello_msg = read
            .next()
            .await
            .ok_or_else(|| "EOF antes do Hello".to_string())?
            .map_err(|e| e.to_string())?;
        let hello: Value = serde_json::from_str(hello_msg.to_text().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        let mut identify_d = json!({
            "rpcVersion": 1,
            "eventSubscriptions": 1 | 4 | 8 | 64
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
            .send(Message::Text(identify.to_string().into()))
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
            ("init_scn", "GetSceneList"),
            ("init_inp", "GetInputList"),
        ] {
            let req = json!({
                "op": 6,
                "d": { "requestType": req_type, "requestId": req_id }
            });
            write
                .send(Message::Text(req.to_string().into()))
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
                    let (rtype, rdata) = match cmd {
                        ObsCmd::ToggleRecord => ("ToggleRecord", json!({})),
                        ObsCmd::ToggleRecordPause => ("ToggleRecordPause", json!({})),
                        ObsCmd::ToggleStream => ("ToggleStream", json!({})),
                        ObsCmd::SetScene(name) => ("SetCurrentProgramScene", json!({"sceneName": name})),
                        ObsCmd::ToggleMute(inp) => ("ToggleInputMute", json!({"inputName": inp})),
                        ObsCmd::SaveReplay => ("SaveReplayBuffer", json!({})),
                    };
                    let msg = json!({
                        "op": 6,
                        "d": { "requestType": rtype, "requestId": req_id, "requestData": rdata }
                    });
                    write.send(Message::Text(msg.to_string().into())).await.map_err(|e| e.to_string())?;
                }
                msg_opt = read.next() => {
                    let Some(msg_res) = msg_opt else { break; };
                    let msg = msg_res.map_err(|e| e.to_string())?;
                    if let Ok(txt) = msg.to_text() {
                        if let Ok(v) = serde_json::from_str::<Value>(txt) {
                            let op = v.get("op").and_then(|x| x.as_i64()).unwrap_or(-1);
                            if op == 5 {
                                Self::handle_event(&self.state, &v);
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

    fn handle_event(state: &Arc<RwLock<ObsState>>, v: &Value) {
        let etype = v.pointer("/d/eventType").and_then(|x| x.as_str()).unwrap_or("");
        let edata = v.pointer("/d/eventData");
        let Ok(mut s) = state.write() else { return };
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
            _ => {}
        }
    }

    async fn handle_response<S>(state: &Arc<RwLock<ObsState>>, v: &Value, write: &mut S)
    where
        S: futures_util::Sink<Message> + Unpin,
    {
        let rtype = v.pointer("/d/requestType").and_then(|x| x.as_str()).unwrap_or("");
        let rdata = v.pointer("/d/responseData");
        let mut inputs_to_query = Vec::new();
        if let Ok(mut s) = state.write() {
            match rtype {
                "GetRecordStatus" => {
                    s.recording = rdata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()).unwrap_or(false);
                    s.paused = rdata.and_then(|d| d.get("outputPaused")).and_then(|x| x.as_bool()).unwrap_or(false);
                }
                "GetStreamStatus" => {
                    s.streaming = rdata.and_then(|d| d.get("outputActive")).and_then(|x| x.as_bool()).unwrap_or(false);
                }
                "GetSceneList" => {
                    s.current_scene = rdata.and_then(|d| d.get("currentProgramSceneName")).and_then(|x| x.as_str()).unwrap_or("").to_string();
                    if let Some(arr) = rdata.and_then(|d| d.get("scenes")).and_then(|x| x.as_array()) {
                        s.scenes = arr.iter().filter_map(|it| it.get("sceneName").and_then(|x| x.as_str()).map(|s| s.to_string())).collect();
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
                    let rid = v.pointer("/d/requestId").and_then(|x| x.as_str()).unwrap_or("");
                    if let Some(name) = rid.strip_prefix("mute_") {
                        if let Some(m) = rdata.and_then(|d| d.get("inputMuted")).and_then(|x| x.as_bool()) {
                            s.muted.insert(name.to_string(), m);
                        }
                    }
                }
                _ => {}
            }
        }
        for inp in inputs_to_query {
            let req = json!({
                "op": 6,
                "d": {
                    "requestType": "GetInputMute",
                    "requestId": format!("mute_{inp}"),
                    "requestData": { "inputName": inp }
                }
            });
            let _ = write.send(Message::Text(req.to_string().into())).await;
        }
    }
}

fn sha256_b64(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(digest)
}
