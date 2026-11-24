//! Executor de ações: Páginas, Pastas aninhadas, OBS, Hotkeys Win32, Soundboard, Webhooks e Multi-Actions.

use crate::obs::{ObsClient, ObsCmd};
use deck_core::{ActionConfig, ActionStep};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY, VK_ADD, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1,
    VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME,
    VK_LEFT, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK,
    VK_MEDIA_STOP, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE,
    VK_SUBTRACT, VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};

#[derive(Debug, Clone)]
pub enum NavCmd {
    Page(String),
    Folder(String),
    FolderBack,
    Theme(String),
    Brightness { value: i16, relative: bool },
    Timer { op: String, seconds: u64 },
}

pub fn execute_action(
    rt: &tokio::runtime::Handle,
    action: ActionConfig,
    obs: Arc<ObsClient>,
    on_nav: Arc<dyn Fn(NavCmd) + Send + Sync>,
    on_log: Arc<dyn Fn(String) + Send + Sync>,
) {
    rt.spawn(async move {
        run_action_async(action, &obs, &on_nav, &on_log).await;
    });
}

fn run_action_async<'a>(
    action: ActionConfig,
    obs: &'a Arc<ObsClient>,
    on_nav: &'a Arc<dyn Fn(NavCmd) + Send + Sync>,
    on_log: &'a Arc<dyn Fn(String) + Send + Sync>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    run_action_depth(action, obs, on_nav, on_log, 0)
}

fn run_action_depth<'a>(
    action: ActionConfig,
    obs: &'a Arc<ObsClient>,
    on_nav: &'a Arc<dyn Fn(NavCmd) + Send + Sync>,
    on_log: &'a Arc<dyn Fn(String) + Send + Sync>,
    depth: u8,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if depth > 4 {
            on_log("[MULTI] Profundidade máxima de MultiAction excedida".into());
            return;
        }
        match action {
            ActionConfig::Page { target } => on_nav(NavCmd::Page(target)),
            ActionConfig::Folder { target } => on_nav(NavCmd::Folder(target)),
            ActionConfig::FolderBack => on_nav(NavCmd::FolderBack),
            ActionConfig::Theme { target } => on_nav(NavCmd::Theme(target)),
            ActionConfig::Brightness { value, relative } => on_nav(NavCmd::Brightness { value, relative }),
            ActionConfig::Timer { op, seconds } => on_nav(NavCmd::Timer { op, seconds }),
            ActionConfig::AudioApp { op, process, step } => {
                let delta = step.unwrap_or(0.08).clamp(0.01, 0.5);
                match adjust_wasapi_audio(&op, &process, delta) {
                    Ok(msg) => on_log(format!("[AUDIO] {msg}")),
                    Err(e) => on_log(format!("[AUDIO] Falha: {e}")),
                }
            }
            ActionConfig::Obs { op, name, input, source, scene } => {
                if !obs.is_connected() {
                    on_log(format!("[OBS] Ignorado (OBS offline): {op}"));
                    return;
                }
                match op.as_str() {
                    "toggle_record" => {
                        obs.send(ObsCmd::ToggleRecord);
                        on_log("[OBS] Comando: Gravar/Parar".into());
                    }
                    "toggle_record_pause" => {
                        obs.send(ObsCmd::ToggleRecordPause);
                        on_log("[OBS] Comando: Pausar/Retomar Gravação".into());
                    }
                    "toggle_stream" => {
                        obs.send(ObsCmd::ToggleStream);
                        on_log("[OBS] Comando: Transmissão (Live)".into());
                    }
                    "toggle_virtualcam" => {
                        obs.send(ObsCmd::ToggleVirtualCam);
                        on_log("[OBS] Comando: Câmera Virtual".into());
                    }
                    "toggle_replay" => {
                        obs.send(ObsCmd::ToggleReplay);
                        on_log("[OBS] Comando: Replay Buffer".into());
                    }
                    "save_replay" => {
                        obs.send(ObsCmd::SaveReplay);
                        on_log("[OBS] Comando: Salvar Replay".into());
                    }
                    "toggle_studio_mode" => {
                        obs.send(ObsCmd::ToggleStudioMode);
                        on_log("[OBS] Comando: Modo Estúdio".into());
                    }
                    "trigger_studio_transition" => {
                        obs.send(ObsCmd::TriggerStudioTransition);
                        on_log("[OBS] Comando: Transição Modo Estúdio".into());
                    }
                    "scene" => {
                        if let Some(n) = name.or(scene) {
                            on_log(format!("[OBS] Cena -> {n}"));
                            obs.send(ObsCmd::SetScene(n));
                        }
                    }
                    "toggle_mute" => {
                        let inp = input.or(name).unwrap_or_else(|| "Mic/Aux".into());
                        on_log(format!("[OBS] Mutar Áudio -> {inp}"));
                        obs.send(ObsCmd::ToggleMute(inp));
                    }
                    "toggle_source" => {
                        let src = source.or(name).unwrap_or_default();
                        if !src.is_empty() {
                            on_log(format!("[OBS] Alternar Fonte: {src}"));
                            obs.send(ObsCmd::ToggleSource { scene, source: src });
                        }
                    }
                    "show_source" => {
                        let src = source.or(name).unwrap_or_default();
                        if !src.is_empty() {
                            on_log(format!("[OBS] Exibir Fonte: {src}"));
                            obs.send(ObsCmd::SetSourceEnabled { scene, source: src, enabled: true });
                        }
                    }
                    "hide_source" => {
                        let src = source.or(name).unwrap_or_default();
                        if !src.is_empty() {
                            on_log(format!("[OBS] Ocultar Fonte: {src}"));
                            obs.send(ObsCmd::SetSourceEnabled { scene, source: src, enabled: false });
                        }
                    }
                    other => on_log(format!("[ACTION] Operação OBS desconhecida: {other}")),
                }
            }
            ActionConfig::Hotkey { keys } => send_hotkey(&keys),
            ActionConfig::Text { content } => {
                let capped: String = content.chars().take(1024).collect();
                send_unicode_text(&capped);
            }
            ActionConfig::Media { op } => {
                let vk = match op.as_str() {
                    "play_pause" => Some(VK_MEDIA_PLAY_PAUSE),
                    "next" => Some(VK_MEDIA_NEXT_TRACK),
                    "prev" => Some(VK_MEDIA_PREV_TRACK),
                    "stop" => Some(VK_MEDIA_STOP),
                    "vol_up" => Some(VK_VOLUME_UP),
                    "vol_down" => Some(VK_VOLUME_DOWN),
                    "mute" => Some(VK_VOLUME_MUTE),
                    _ => None,
                };
                if let Some(k) = vk {
                    send_vk_combo(&[k]);
                }
            }
            ActionConfig::Run { program, args } => {
                if !program.trim().is_empty() {
                    match spawn_program(&program, &args) {
                        Ok(resolved) => on_log(format!("[RUN] Iniciado: {resolved}")),
                        Err(e) => on_log(format!("[RUN] Erro ao abrir {program}: {e}")),
                    }
                }
            }
            ActionConfig::Soundboard { path } => {
                if !path.trim().is_empty() {
                    if let Err(e) = play_wav_async(&path) {
                        on_log(format!("[SOUNDBOARD] Rejeitado: {e}"));
                    }
                }
            }
            ActionConfig::Webhook { url, method } => {
                let Ok((parsed, addrs)) = resolve_safe_webhook(&url).await else {
                    on_log("[WEBHOOK] URL ou IP bloqueado pela política anti-SSRF".into());
                    return;
                };
                let mut builder = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .redirect(reqwest::redirect::Policy::none());
                if let Some(host) = parsed.host_str() {
                    if let Some(&first_addr) = addrs.first() {
                        builder = builder.resolve(host, first_addr);
                    }
                }
                let Ok(client) = builder.build() else {
                    return;
                };
                let req = if method.eq_ignore_ascii_case("POST") {
                    client.post(parsed.clone())
                } else {
                    client.get(parsed.clone())
                };
                if let Err(e) = req.send().await {
                    let safe_url = url.split('?').next().unwrap_or(&url);
                    let status = e.status().map(|s| s.to_string()).unwrap_or_else(|| "falha de rede".into());
                    on_log(format!("[WEBHOOK] Erro em {safe_url}: {status}"));
                }
            }
            ActionConfig::MultiAction { name, steps } => {
                let capped_steps = steps.into_iter().take(32).collect::<Vec<_>>();
                on_log(format!("[MULTI] Executando '{name}' ({} passos)", capped_steps.len()));
                for step in capped_steps {
                    match step {
                        ActionStep::Delay { ms } => {
                            let bounded = ms.min(60_000);
                            tokio::time::sleep(std::time::Duration::from_millis(bounded)).await;
                        }
                        ActionStep::Action { action: inner } => {
                            run_action_depth(inner, obs, on_nav, on_log, depth + 1).await;
                        }
                    }
                }
            }
        }
    })
}

fn send_hotkey(keys: &[String]) {
    let vks: Vec<VIRTUAL_KEY> = keys.iter().take(16).filter_map(|k| parse_vk(k)).collect();
    if !vks.is_empty() {
        send_vk_combo(&vks);
    }
}

fn send_vk_combo(vks: &[VIRTUAL_KEY]) {
    let mut inputs = Vec::with_capacity(vks.len() * 2);
    for &vk in vks {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    ..Default::default()
                },
            },
        });
    }
    for &vk in vks.iter().rev() {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        });
    }
    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn send_unicode_text(text: &str) {
    let mut inputs = Vec::new();
    for code in text.encode_utf16() {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wScan: code,
                    dwFlags: KEYEVENTF_UNICODE,
                    ..Default::default()
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wScan: code,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        });
    }
    if !inputs.is_empty() {
        unsafe {
            let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

fn parse_vk(name: &str) -> Option<VIRTUAL_KEY> {
    let up = name.trim().to_ascii_uppercase();
    match up.as_str() {
        "CTRL" | "CONTROL" => Some(VK_CONTROL),
        "SHIFT" => Some(VK_SHIFT),
        "ALT" => Some(VK_MENU),
        "WIN" | "CMD" => Some(VK_LWIN),
        "ENTER" | "RETURN" => Some(VK_RETURN),
        "ESC" | "ESCAPE" => Some(VK_ESCAPE),
        "SPACE" => Some(VK_SPACE),
        "TAB" => Some(VK_TAB),
        "BACKSPACE" => Some(VK_BACK),
        "DEL" | "DELETE" => Some(VK_DELETE),
        "HOME" => Some(VK_HOME),
        "END" => Some(VK_END),
        "PGUP" => Some(VK_PRIOR),
        "PGDN" => Some(VK_NEXT),
        "UP" => Some(VK_UP),
        "DOWN" => Some(VK_DOWN),
        "LEFT" => Some(VK_LEFT),
        "RIGHT" => Some(VK_RIGHT),
        "PLUS" => Some(VK_ADD),
        "MINUS" => Some(VK_SUBTRACT),
        "F1" => Some(VK_F1),
        "F2" => Some(VK_F2),
        "F3" => Some(VK_F3),
        "F4" => Some(VK_F4),
        "F5" => Some(VK_F5),
        "F6" => Some(VK_F6),
        "F7" => Some(VK_F7),
        "F8" => Some(VK_F8),
        "F9" => Some(VK_F9),
        "F10" => Some(VK_F10),
        "F11" => Some(VK_F11),
        "F12" => Some(VK_F12),
        s if s.len() == 1 => {
            let b = s.as_bytes()[0];
            if b.is_ascii_alphanumeric() {
                Some(VIRTUAL_KEY(b as u16))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_private_or_special_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let oct = v4.octets();
            let is_cgnat = oct[0] == 100 && (oct[1] & 0b1100_0000) == 64;
            oct[0] == 0
                || v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || is_cgnat
        }
        std::net::IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_private_or_special_ip(std::net::IpAddr::V4(mapped));
            }
            let seg0 = v6.segments()[0];
            let is_ula = (seg0 & 0xfe00) == 0xfc00;
            let is_link_local = (seg0 & 0xffc0) == 0xfe80;
            v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() || is_ula || is_link_local
        }
    }
}

fn is_safe_webhook_url(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url.trim()) else {
        return false;
    };
    if !matches!(parsed.scheme(), "http" | "https") || !parsed.username().is_empty() || parsed.password().is_some() {
        return false;
    }
    let Some(raw_host) = parsed.host_str() else {
        return false;
    };
    let host = raw_host.trim_matches(['[', ']']).trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return false;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return !is_private_or_special_ip(ip);
    }
    true
}

async fn resolve_safe_webhook(url: &str) -> Result<(reqwest::Url, Vec<std::net::SocketAddr>), ()> {
    if !is_safe_webhook_url(url) {
        return Err(());
    }
    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| ())?;
    let host = parsed.host_str().ok_or(())?.trim_matches(['[', ']']).to_string();
    let port = parsed.port_or_known_default().ok_or(())?;
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        if is_private_or_special_ip(ip) {
            return Err(());
        }
        return Ok((parsed, vec![std::net::SocketAddr::new(ip, port)]));
    }
    let resolved: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| ())?
        .collect();
    if resolved.is_empty() || resolved.iter().any(|sa| is_private_or_special_ip(sa.ip())) {
        return Err(());
    }
    Ok((parsed, resolved))
}

static WINMM_LIB: std::sync::OnceLock<Option<libloading::Library>> = std::sync::OnceLock::new();

fn has_unc_or_device_prefix(t: &str) -> bool {
    let norm = t.replace('/', "\\");
    t.contains('\0')
        || norm.starts_with(r"\\")
        || norm.starts_with(r"\?")
        || norm.starts_with(r"\.")
}

fn play_wav_async(path: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    let t = path.trim();
    if has_unc_or_device_prefix(t) {
        return Err("Caminho UNC ou inválido não permitido".into());
    }
    let p = Path::new(t);
    if !p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("wav"))
        .unwrap_or(false)
    {
        return Err("Apenas arquivos .wav são permitidos".into());
    }
    let canon = std::fs::canonicalize(p).map_err(|_| "Arquivo WAV não encontrado".to_string())?;
    let canon_str = canon.to_string_lossy();
    if canon_str.starts_with(r"\\?\UNC\") || !canon.is_file() {
        return Err("Caminho UNC ou não-arquivo não permitido".into());
    }
    let lib = WINMM_LIB
        .get_or_init(|| {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
            let dll_path = Path::new(&root).join("System32").join("winmm.dll");
            unsafe { libloading::Library::new(dll_path).ok() }
        })
        .as_ref()
        .ok_or_else(|| "Falha ao carregar winmm.dll".to_string())?;

    let play = unsafe {
        lib.get::<unsafe extern "system" fn(*const u16, usize, u32) -> i32>(b"PlaySoundW\0")
            .map_err(|e| e.to_string())?
    };
    let wide: Vec<u16> = OsStr::new(&canon).encode_wide().chain(Some(0)).collect();
    // SND_FILENAME (0x20000) | SND_ASYNC (0x1) | SND_NODEFAULT (0x2)
    unsafe {
        let _ = play(wide.as_ptr(), 0, 0x0002_0003);
    }
    Ok(())
}

/// Expande variáveis de ambiente no estilo Windows (`%LOCALAPPDATA%`, `%APPDATA%`, etc.).
pub fn expand_env_vars(input: &str) -> String {
    let trimmed = input.trim().trim_matches('"');
    let mut out = String::with_capacity(trimmed.len());
    let mut rest = trimmed;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('%') {
            let var_name = &after[..end];
            if !var_name.is_empty() {
                if let Ok(val) = std::env::var(var_name) {
                    out.push_str(&val);
                    rest = &after[end + 1..];
                    continue;
                }
            }
            out.push('%');
            out.push_str(var_name);
            out.push('%');
            rest = &after[end + 1..];
        } else {
            out.push('%');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn is_shell_target(target: &str) -> bool {
    let low = target.to_ascii_lowercase();
    low.starts_with("http://")
        || low.starts_with("https://")
        || low.starts_with("steam://")
        || low.starts_with("discord://")
        || low.starts_with("ms-settings:")
        || low.ends_with(".lnk")
        || low.ends_with(".url")
}

/// Inicia executáveis (ajustando `current_dir` para o diretório pai, essencial para `obs64.exe`),
/// ou abre URLs / atalhos `.lnk`/`.url` diretamente via Win32 `ShellExecuteW` (sem `cmd.exe`).
fn spawn_program(program: &str, args: &[String]) -> std::io::Result<String> {
    let resolved = expand_env_vars(program);
    if resolved.contains('\0') || resolved.contains('\r') || resolved.contains('\n') || has_unc_or_device_prefix(&resolved) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Alvo contém caracteres ou prefixo UNC inválido",
        ));
    }
    if is_shell_target(&resolved) {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let op: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = OsStr::new(&resolved).encode_wide().chain(Some(0)).collect();
        let res = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(op.as_ptr()),
                PCWSTR(target.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if (res.0 as isize) <= 32 {
            return Err(std::io::Error::other(format!("ShellExecuteW falhou ({})", res.0 as isize)));
        }
        return Ok(resolved);
    }

    let mut cmd = Command::new(&resolved);
    cmd.args(args);
    if let Some(parent) = Path::new(&resolved).parent() {
        if parent.is_dir() {
            cmd.current_dir(parent);
        }
    }
    cmd.spawn()?;
    Ok(resolved)
}

fn pid_to_exe_map() -> std::collections::HashMap<u32, String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    let mut map = std::collections::HashMap::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return map;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_ascii_lowercase();
                map.insert(entry.th32ProcessID, name);
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    map
}

/// Controle nativo de volume/mute WASAPI (Master ou por processo `.exe`, ex: `spotify.exe`).
fn adjust_wasapi_audio(op: &str, process_filter: &str, step: f32) -> Result<String, String> {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator,
        ISimpleAudioVolume, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    unsafe {
        let hr_init = CoInitializeEx(None, COINIT_MULTITHREADED);
        let did_init = hr_init.is_ok();
        let _guard = scopeguard_uninit(did_init);

        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| format!("MMDeviceEnumerator: {e}"))?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .map_err(|e| format!("GetDefaultAudioEndpoint: {e}"))?;

        let target = process_filter.trim().to_ascii_lowercase();
        if target.is_empty() {
            let ep: IAudioEndpointVolume = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("IAudioEndpointVolume: {e}"))?;
            match op {
                "mute" => {
                    let muted = ep.GetMute().map_err(|e| e.to_string())?.as_bool();
                    ep.SetMute(!muted, std::ptr::null()).map_err(|e| e.to_string())?;
                    return Ok(format!("Master {}", if !muted { "MUDO" } else { "ATIVO" }));
                }
                "vol_up" | "vol_down" => {
                    let cur = ep.GetMasterVolumeLevelScalar().map_err(|e| e.to_string())?;
                    let next = if op == "vol_up" { (cur + step).min(1.0) } else { (cur - step).max(0.0) };
                    ep.SetMasterVolumeLevelScalar(next, std::ptr::null()).map_err(|e| e.to_string())?;
                    return Ok(format!("Master {}%", (next * 100.0).round() as i32));
                }
                _ => return Err(format!("op inválida: {op}")),
            }
        }

        if !matches!(op, "mute" | "vol_up" | "vol_down") {
            return Err(format!("Operação de áudio inválida: {op}"));
        }

        let mgr: IAudioSessionManager2 = device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("IAudioSessionManager2: {e}"))?;
        let sess_enum = mgr.GetSessionEnumerator().map_err(|e| e.to_string())?;
        let count = sess_enum.GetCount().map_err(|e| e.to_string())?;
        let pids = pid_to_exe_map();
        let mut matched = 0usize;
        let target_exe = if target.ends_with(".exe") {
            target.clone()
        } else {
            format!("{target}.exe")
        };

        for i in 0..count {
            let Ok(ctrl) = sess_enum.GetSession(i) else { continue };
            let Ok(ctrl2) = ctrl.cast::<IAudioSessionControl2>() else { continue };
            let Ok(pid) = ctrl2.GetProcessId() else { continue };
            let Some(exe) = pids.get(&pid) else { continue };
            if exe != &target && exe != &target_exe {
                continue;
            }
            let Ok(vol) = ctrl.cast::<ISimpleAudioVolume>() else { continue };
            matched += 1;
            match op {
                "mute" => {
                    let muted = vol.GetMute().map_err(|e| e.to_string())?.as_bool();
                    vol.SetMute(!muted, std::ptr::null()).map_err(|e| e.to_string())?;
                }
                "vol_up" | "vol_down" => {
                    let cur = vol.GetMasterVolume().map_err(|e| e.to_string())?;
                    let next = if op == "vol_up" { (cur + step).min(1.0) } else { (cur - step).max(0.0) };
                    vol.SetMasterVolume(next, std::ptr::null()).map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }
        if matched == 0 {
            return Err(format!("Nenhuma sessão de áudio ativa para '{target}'"));
        }
        return Ok(format!("{target} ({op})"));
    }

    struct ComGuard(bool);
    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }
    fn scopeguard_uninit(init: bool) -> ComGuard {
        ComGuard(init)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_env_vars_strips_quotes_and_resolves() {
        std::env::set_var("DECK_TEST_DIR", r"C:\Apps");
        assert_eq!(
            expand_env_vars(r#""%DECK_TEST_DIR%\obs64.exe""#),
            r"C:\Apps\obs64.exe"
        );
        assert_eq!(expand_env_vars("  wt.exe  "), "wt.exe");
    }

    #[test]
    fn detects_shell_targets_for_urls_and_shortcuts() {
        assert!(is_shell_target("https://www.twitch.tv"));
        assert!(is_shell_target("steam://rungameid/123"));
        assert!(is_shell_target(r"C:\Users\Public\Desktop\OBS.lnk"));
        assert!(!is_shell_target(r"C:\Program Files\obs-studio\bin\64bit\obs64.exe"));
    }

    #[test]
    fn webhook_ssrf_and_soundboard_guards_work() {
        assert!(is_safe_webhook_url("https://hooks.example.com/notify"));
        assert!(!is_safe_webhook_url("http://127.0.0.1:19876/api/health"));
        assert!(!is_safe_webhook_url("http://localhost:8085/data.json"));
        assert!(!is_safe_webhook_url("http://169.254.169.254/latest/meta-data/"));
        assert!(!is_safe_webhook_url("http://192.168.1.10/hook"));
        assert!(play_wav_async(r"\\evil\share\a.wav").is_err());
        assert!(play_wav_async(r"C:\Windows\System32\cmd.exe").is_err());
    }
}
