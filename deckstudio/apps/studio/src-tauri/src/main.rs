#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use tauri::WindowEvent;

fn sha256_hex(input: &[u8]) -> String {
    // SHA-256 simples via Windows CNG ou inline não necessário: usamos sha2? Vamos checar Cargo.toml;
    // Para manter zero dependências novas em src-tauri, usamos BLAKE3/SHA-256? Não temos sha2 em src-tauri.
    // Melhor: adicionamos sha2 = "0.10" em src-tauri/Cargo.toml!
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input);
    format!("{:x}", h.finalize())
}

fn is_engine_running() -> bool {
    use std::io::{Read, Write};
    let addr: SocketAddr = "127.0.0.1:19876".parse().unwrap();
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(120)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));
    let Ok(token) = get_ipc_token() else {
        return false;
    };
    let nonce = format!(
        "{}{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let expected = sha256_hex(format!("{token}:{nonce}").as_bytes());
    let req = format!(
        "GET /api/ping?c={nonce} HTTP/1.1\r\nHost: 127.0.0.1:19876\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 512];
    let n = stream.read(&mut buf).unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf[..n]);
    resp.contains(&expected)
}

fn ensure_engine_started() {
    if is_engine_running() {
        return;
    }

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };

    let mut candidates = vec![
        dir.join("deck-engine.exe"),
        dir.join("binaries").join("deck-engine.exe"),
    ];
    #[cfg(debug_assertions)]
    {
        candidates.push(dir.join("..\\..\\..\\..\\target\\release\\deck-engine.exe"));
        candidates.push(dir.join("..\\..\\..\\target\\release\\deck-engine.exe"));
    }

    let Some(engine_path) = candidates.into_iter().find(|p| p.exists()) else {
        return;
    };

    let mut cmd = std::process::Command::new(&engine_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    if cmd.spawn().is_ok() {
        for _ in 0..15 {
            if is_engine_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(80));
        }
    }
}

#[tauri::command]
fn get_ipc_token() -> Result<String, String> {
    let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA indisponível".to_string())?;
    let p = std::path::PathBuf::from(appdata).join("DeckStudio").join("ipc.token");
    std::fs::read_to_string(p)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Token IPC não encontrado: {e}"))
}

fn main() {
    ensure_engine_started();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_ipc_token])
        .on_window_event(|_window, event| {
            // Encerra o processo do editor por completo ao fechar a janela (0 MB WebView2 em idle).
            if let WindowEvent::CloseRequested { .. } = event {
                std::process::exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("Erro ao iniciar DeckStudio");
}
