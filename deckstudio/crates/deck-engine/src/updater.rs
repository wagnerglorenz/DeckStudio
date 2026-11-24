//! Auto-Updater do DeckStudio: consulta GitHub Releases (ou feed customizado),
//! baixa o instalador NSIS com barra de progresso, valida SHA-256 e aplica o upgrade.

use deck_core::{UpdateCheckResponse, UpdateInstallStatus};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    // Ignora pré-releases (ex.: 0.4.0-alpha) para não instalar builds instáveis por engano
    if trimmed.contains('-') {
        return None;
    }
    let clean = trimmed.split('+').next().unwrap_or(trimmed);
    let mut parts = clean.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver(current), parse_semver(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

pub fn extract_sha256_for_file(sums_text: &str, filename: &str) -> Option<String> {
    let target = filename.to_ascii_lowercase();
    for line in sums_text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0].trim();
            let file = parts[1].trim().trim_start_matches('*').to_ascii_lowercase();
            if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
                && (file == target || file.ends_with(&format!("/{target}")))
            {
                return Some(hash.to_ascii_lowercase());
            }
        }
    }
    None
}

pub fn parse_release_json(v: &Value, current_version: &str) -> UpdateCheckResponse {
    // Suporta formato GitHub Releases API e formato simplificado { version, url, sha256, notes }
    let tag = v
        .get("tag_name")
        .or_else(|| v.get("version"))
        .and_then(|x| x.as_str())
        .unwrap_or(current_version);
    let latest_clean = tag.trim().trim_start_matches(['v', 'V']).to_string();
    let notes = v
        .get("body")
        .or_else(|| v.get("notes"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let published_at = v
        .get("published_at")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let mut download_url = v
        .get("url")
        .or_else(|| v.get("download_url"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let sha256 = v
        .get("sha256")
        .and_then(|x| x.as_str())
        .map(|s| s.to_ascii_lowercase());
    let mut asset_name = String::new();
    let mut sums_url = String::new();

    if let Some(assets) = v.get("assets").and_then(|x| x.as_array()) {
        for a in assets {
            let name = a.get("name").and_then(|x| x.as_str()).unwrap_or("");
            let url = a.get("browser_download_url").and_then(|x| x.as_str()).unwrap_or("");
            let low = name.to_ascii_lowercase();
            if low.ends_with("-setup.exe") || (download_url.is_empty() && low.ends_with(".exe")) {
                download_url = url.to_string();
                asset_name = name.to_string();
            } else if low.contains("sha256") && low.ends_with(".txt") {
                sums_url = url.to_string();
            }
        }
    }

    // Guarda temporariamente sums_url no campo error apenas se necessário? Não — resolveremos fora.
    let _ = (sums_url, asset_name);

    let update_available = is_newer_version(current_version, tag) && !download_url.is_empty();
    UpdateCheckResponse {
        current_version: current_version.to_string(),
        latest_version: latest_clean,
        update_available,
        release_notes: notes,
        download_url,
        sha256,
        published_at,
        error: None,
    }
}

const MAX_INSTALLER_BYTES: u64 = 128 * 1024 * 1024;

fn has_suspicious_url_parts(u: &str) -> bool {
    let low = u.to_ascii_lowercase();
    u.contains(char::is_whitespace)
        || u.contains('@')
        || u.contains('\\')
        || low.contains("/../")
        || low.ends_with("/..")
        || low.contains("/./")
        || low.contains("%2e")
}

pub fn is_trusted_feed_url(url: &str) -> bool {
    let u = url.trim();
    if u != url || has_suspicious_url_parts(u) {
        return false;
    }
    let prefixes = [
        "https://api.github.com/repos/wagnerglorenz/DeckStudio/releases",
        "https://github.com/wagnerglorenz/DeckStudio/releases",
    ];
    prefixes.iter().any(|p| {
        if let Some(rest) = u.strip_prefix(p) {
            rest.is_empty() || rest.starts_with('/') || rest.starts_with('?')
        } else {
            false
        }
    })
}

pub fn is_trusted_download_url(url: &str) -> bool {
    let u = url.trim();
    if u != url || has_suspicious_url_parts(u) {
        return false;
    }
    u.starts_with("https://github.com/wagnerglorenz/DeckStudio/releases/download/")
        || u.starts_with("https://objects.githubusercontent.com/")
        || u.starts_with("https://release-assets.githubusercontent.com/")
}

pub fn is_valid_sha256(sha: Option<&str>) -> bool {
    matches!(sha.map(str::trim), Some(s) if s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub async fn check_for_update(feed_url: &str) -> UpdateCheckResponse {
    if !is_trusted_feed_url(feed_url) {
        return UpdateCheckResponse {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            error: Some("URL de atualização não confiável (apenas HTTPS oficial)".into()),
            ..Default::default()
        };
    }

    let Ok(client) = reqwest::Client::builder()
        .user_agent(format!("DeckStudio/{CURRENT_VERSION}"))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.stop();
            }
            if is_trusted_feed_url(attempt.url().as_str()) || is_trusted_download_url(attempt.url().as_str()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .timeout(Duration::from_secs(8))
        .build()
    else {
        return UpdateCheckResponse {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            error: Some("Falha ao inicializar cliente HTTP".into()),
            ..Default::default()
        };
    };

    let resp = match client.get(feed_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return UpdateCheckResponse {
                current_version: CURRENT_VERSION.to_string(),
                latest_version: CURRENT_VERSION.to_string(),
                error: Some(format!("Falha de rede: {e}")),
                ..Default::default()
            };
        }
    };

    if !resp.status().is_success() {
        let code = resp.status();
        let hint = if code.as_u16() == 404 {
            "Nenhuma release pública encontrada (404). Publique uma release ou configure feed público."
                .to_string()
        } else {
            format!("Servidor retornou HTTP {code}")
        };
        return UpdateCheckResponse {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            error: Some(hint),
            ..Default::default()
        };
    }

    let Ok(v) = resp.json::<Value>().await else {
        return UpdateCheckResponse {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            error: Some("JSON de release inválido".into()),
            ..Default::default()
        };
    };

    let mut out = parse_release_json(&v, CURRENT_VERSION);

    // Se houver SHA256SUMS.txt nos assets e ainda não temos sha256, baixa para validar
    if out.sha256.is_none() && !out.download_url.is_empty() {
        if let Some(assets) = v.get("assets").and_then(|x| x.as_array()) {
            let exe_name = out.download_url.rsplit('/').next().unwrap_or("");
            if let Some(sums_url) = assets
                .iter()
                .find(|a| {
                    let n = a.get("name").and_then(|x| x.as_str()).unwrap_or("").to_ascii_lowercase();
                    n.contains("sha256") && n.ends_with(".txt")
                })
                .and_then(|a| a.get("browser_download_url").and_then(|x| x.as_str()))
            {
                if let Ok(sr) = client.get(sums_url).send().await {
                    if let Ok(txt) = sr.text().await {
                        out.sha256 = extract_sha256_for_file(&txt, exe_name);
                    }
                }
            }
        }
    }

    out
}

fn set_status(arc: &Arc<RwLock<UpdateInstallStatus>>, state: &str, pct: u8, msg: impl Into<String>) {
    if let Ok(mut g) = arc.write() {
        g.state = state.into();
        g.progress_pct = pct;
        g.message = msg.into();
    }
}

pub async fn download_and_install_update(
    download_url: String,
    expected_sha256: Option<String>,
    status: Arc<RwLock<UpdateInstallStatus>>,
) {
    if !is_trusted_download_url(&download_url) {
        set_status(&status, "error", 0, "URL de download rejeitada pela política de segurança");
        return;
    }
    let Some(exp_hash) = expected_sha256
        .as_deref()
        .map(str::trim)
        .filter(|s| is_valid_sha256(Some(s)))
        .map(|s| s.to_ascii_lowercase())
    else {
        set_status(&status, "error", 0, "SHA-256 obrigatório ausente ou inválido");
        return;
    };

    set_status(&status, "downloading", 2, "Conectando para baixar atualização...");

    let client = match reqwest::Client::builder()
        .user_agent(format!("DeckStudio/{CURRENT_VERSION}"))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.stop();
            }
            if is_trusted_download_url(attempt.url().as_str()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .timeout(Duration::from_secs(180))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            set_status(&status, "error", 0, format!("Erro HTTP: {e}"));
            return;
        }
    };

    let mut resp = match client.get(&download_url).send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            set_status(&status, "error", 0, format!("Download falhou: HTTP {}", r.status()));
            return;
        }
        Err(e) => {
            set_status(&status, "error", 0, format!("Erro de download: {e}"));
            return;
        }
    };

    let total = resp.content_length().unwrap_or(0);
    if total > MAX_INSTALLER_BYTES {
        set_status(&status, "error", 0, "Instalador excede o tamanho máximo permitido");
        return;
    }

    let updates_dir = std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("DeckStudio")
        .join("updates");
    let _ = std::fs::create_dir_all(&updates_dir);
    let ts_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = updates_dir.join(format!("DeckStudio_setup_{}_{ts_ns}.exe", std::process::id()));
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
    {
        Ok(f) => f,
        Err(e) => {
            set_status(&status, "error", 0, format!("Erro ao criar arquivo de atualização: {e}"));
            return;
        }
    };

    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;

    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                downloaded += chunk.len() as u64;
                if downloaded > MAX_INSTALLER_BYTES {
                    drop(file);
                    let _ = std::fs::remove_file(&temp_path);
                    set_status(&status, "error", 0, "Instalador excedeu o limite de 128 MiB");
                    return;
                }
                if let Err(e) = file.write_all(&chunk) {
                    drop(file);
                    let _ = std::fs::remove_file(&temp_path);
                    set_status(&status, "error", 0, format!("Erro ao gravar arquivo: {e}"));
                    return;
                }
                hasher.update(&chunk);
                let pct = (downloaded * 88)
                    .checked_div(total)
                    .map(|v| v.min(88) as u8 + 2)
                    .unwrap_or(45);
                set_status(
                    &status,
                    "downloading",
                    pct,
                    format!("Baixando instalador ({:.1} MB)...", downloaded as f64 / 1_048_576.0),
                );
            }
            Ok(None) => break,
            Err(e) => {
                drop(file);
                let _ = std::fs::remove_file(&temp_path);
                set_status(&status, "error", 0, format!("Conexão interrompida: {e}"));
                return;
            }
        }
    }
    let _ = file.flush();
    drop(file);

    set_status(&status, "verifying", 92, "Validando assinatura SHA-256 do instalador...");
    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != exp_hash {
        let _ = std::fs::remove_file(&temp_path);
        set_status(
            &status,
            "error",
            0,
            "Falha de segurança: hash SHA-256 não confere! Arquivo descartado.",
        );
        return;
    }

    // Re-verifica SHA-256 no disco com share_mode(FILE_SHARE_READ) para fechar janela TOCTOU
    {
        use std::io::Read as _;
        #[cfg(windows)]
        use std::os::windows::fs::OpenOptionsExt as _;
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true);
        #[cfg(windows)]
        opts.share_mode(1); // FILE_SHARE_READ
        let Ok(mut disk_f) = opts.open(&temp_path) else {
            let _ = std::fs::remove_file(&temp_path);
            set_status(&status, "error", 0, "Falha ao reabrir instalador para verificação");
            return;
        };
        let mut disk_hasher = Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            match disk_f.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => disk_hasher.update(&buf[..n]),
                Err(_) => {
                    let _ = std::fs::remove_file(&temp_path);
                    set_status(&status, "error", 0, "Erro ao revalidar arquivo no disco");
                    return;
                }
            }
        }
        let disk_hash = format!("{:x}", disk_hasher.finalize());
        if disk_hash != exp_hash {
            let _ = std::fs::remove_file(&temp_path);
            set_status(&status, "error", 0, "Falha de integridade pós-gravação (TOCTOU)");
            return;
        }
    }

    set_status(&status, "installing", 98, "Aplicando atualização silenciosa e reiniciando...");

    let mut cmd = std::process::Command::new(&temp_path);
    cmd.arg("/S");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB);
    }
    if let Err(e) = cmd.spawn() {
        set_status(&status, "error", 0, format!("Falha ao iniciar instalador: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn semver_comparison_works() {
        assert!(is_newer_version("0.3.0", "v0.3.1"));
        assert!(is_newer_version("0.3.0", "0.4.0"));
        assert!(is_newer_version("0.3.0", "1.0.0"));
        assert!(!is_newer_version("0.3.0", "0.3.0"));
        assert!(!is_newer_version("0.3.0", "v0.2.9"));
        assert!(!is_newer_version("0.3.0", "v0.3.1-beta.1"));
    }

    #[test]
    fn extracts_sha256_from_sums_file() {
        let sums = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  DeckStudio_0.3.1_x64-setup.exe\n";
        let got = extract_sha256_for_file(sums, "DeckStudio_0.3.1_x64-setup.exe");
        assert_eq!(
            got.as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
    }

    #[test]
    fn parses_github_release_payload() {
        let payload = json!({
            "tag_name": "v0.3.1",
            "body": "Novidades incríveis",
            "published_at": "2026-11-23T18:00:00Z",
            "assets": [
                {
                    "name": "DeckStudio_0.3.1_x64-setup.exe",
                    "browser_download_url": "https://example.com/DeckStudio_0.3.1_x64-setup.exe"
                }
            ]
        });
        let res = parse_release_json(&payload, "0.3.0");
        assert!(res.update_available);
        assert_eq!(res.latest_version, "0.3.1");
        assert_eq!(res.download_url, "https://example.com/DeckStudio_0.3.1_x64-setup.exe");
    }

    #[test]
    fn updater_url_and_sha256_policy_enforces_pinning() {
        assert!(is_trusted_feed_url("https://api.github.com/repos/wagnerglorenz/DeckStudio/releases/latest"));
        assert!(!is_trusted_feed_url("http://api.github.com/repos/wagnerglorenz/DeckStudio/releases/latest"));
        assert!(!is_trusted_feed_url("https://evil.example/releases/latest"));
        assert!(is_trusted_download_url("https://github.com/wagnerglorenz/DeckStudio/releases/download/v0.4.1/DeckStudio.exe"));
        assert!(is_trusted_download_url("https://objects.githubusercontent.com/github-production-release-asset/123"));
        assert!(!is_trusted_download_url("http://github.com/wagnerglorenz/DeckStudio/releases/download/v0.4.1/a.exe"));
        assert!(!is_trusted_download_url("https://evil.example/payload.exe"));
        assert!(!is_valid_sha256(None));
        assert!(!is_valid_sha256(Some("")));
        assert!(!is_valid_sha256(Some("not-hex")));
        assert!(is_valid_sha256(Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")));
    }
}
