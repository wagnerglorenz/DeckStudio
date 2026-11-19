//! Executor de ações: Páginas, Pastas aninhadas, OBS, Hotkeys Win32, Soundboard, Webhooks e Multi-Actions.

use crate::obs::{ObsClient, ObsCmd};
use deck_core::{ActionConfig, ActionStep};
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
    Box::pin(async move {
        match action {
            ActionConfig::Page { target } => on_nav(NavCmd::Page(target)),
            ActionConfig::Folder { target } => on_nav(NavCmd::Folder(target)),
            ActionConfig::FolderBack => on_nav(NavCmd::FolderBack),
            ActionConfig::Theme { target } => on_nav(NavCmd::Theme(target)),
            ActionConfig::Brightness { value, relative } => on_nav(NavCmd::Brightness { value, relative }),
            ActionConfig::Obs { op, name, input } => {
                if !obs.is_connected() {
                    on_log(format!("[OBS] Ignorado (OBS offline): {op}"));
                    return;
                }
                match op.as_str() {
                    "toggle_record" => {
                        obs.send(ObsCmd::ToggleRecord);
                        on_log("[OBS] Comando enviado: toggle_record".into());
                    }
                    "toggle_record_pause" => {
                        obs.send(ObsCmd::ToggleRecordPause);
                        on_log("[OBS] Comando enviado: toggle_record_pause".into());
                    }
                    "toggle_stream" => {
                        obs.send(ObsCmd::ToggleStream);
                        on_log("[OBS] Comando enviado: toggle_stream".into());
                    }
                    "scene" => {
                        if let Some(n) = name {
                            on_log(format!("[OBS] Comando enviado: scene -> {n}"));
                            obs.send(ObsCmd::SetScene(n));
                        }
                    }
                    "toggle_mute" => {
                        let inp = input.unwrap_or_else(|| "Mic/Aux".into());
                        on_log(format!("[OBS] Comando enviado: toggle_mute -> {inp}"));
                        obs.send(ObsCmd::ToggleMute(inp));
                    }
                    "save_replay" => {
                        obs.send(ObsCmd::SaveReplay);
                        on_log("[OBS] Comando enviado: save_replay".into());
                    }
                    other => on_log(format!("[ACTION] Operação OBS desconhecida: {other}")),
                }
            }
            ActionConfig::Hotkey { keys } => send_hotkey(&keys),
            ActionConfig::Text { content } => send_unicode_text(&content),
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
                    match Command::new(&program).args(&args).spawn() {
                        Ok(_) => on_log(format!("[RUN] Iniciado: {program}")),
                        Err(e) => on_log(format!("[RUN] Erro ao abrir {program}: {e}")),
                    }
                }
            }
            ActionConfig::Soundboard { path } => {
                if !path.trim().is_empty() {
                    play_wav_async(&path);
                }
            }
            ActionConfig::Webhook { url, method } => {
                let client = reqwest::Client::new();
                let req = if method.eq_ignore_ascii_case("POST") {
                    client.post(&url)
                } else {
                    client.get(&url)
                };
                if let Err(e) = req.send().await {
                    on_log(format!("[WEBHOOK] Erro em {url}: {e}"));
                }
            }
            ActionConfig::MultiAction { name, steps } => {
                on_log(format!("[MULTI] Executando '{name}' ({} passos)", steps.len()));
                for step in steps {
                    match step {
                        ActionStep::Delay { ms } => {
                            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                        }
                        ActionStep::Action { action: inner } => {
                            run_action_async(inner, obs, on_nav, on_log).await;
                        }
                    }
                }
            }
        }
    })
}

fn send_hotkey(keys: &[String]) {
    let vks: Vec<VIRTUAL_KEY> = keys.iter().filter_map(|k| parse_vk(k)).collect();
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

fn play_wav_async(path: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    let Ok(lib) = (unsafe { libloading::Library::new("winmm.dll") }) else { return };
    let Ok(play) = (unsafe { lib.get::<unsafe extern "system" fn(*const u16, usize, u32) -> i32>(b"PlaySoundW\0") }) else {
        return;
    };
    let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    // SND_FILENAME (0x20000) | SND_ASYNC (0x1) | SND_NODEFAULT (0x2)
    unsafe {
        let _ = play(wide.as_ptr(), 0, 0x0002_0003);
    }
    std::mem::forget(lib);
}
