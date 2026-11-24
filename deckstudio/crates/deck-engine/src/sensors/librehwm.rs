//! Cliente HTTP leve para LibreHardwareMonitor Remote Web Server (`data.json`).

use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LibreReading {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
}

pub fn is_loopback_librehwm_url(url: &str) -> bool {
    let u = url.trim();
    let Some(rest) = u.strip_prefix("http://") else {
        return false;
    };
    let host_port = rest.split('/').next().unwrap_or("");
    let (host, port_str) = if let Some(after) = host_port.strip_prefix("[::1]:") {
        ("[::1]", after)
    } else if let Some((h, p)) = host_port.split_once(':') {
        (h, p)
    } else {
        return false;
    };
    let valid_host = host == "127.0.0.1" || host.eq_ignore_ascii_case("localhost") || host == "[::1]";
    let valid_port = port_str.parse::<u16>().map(|p| p > 0).unwrap_or(false);
    valid_host && valid_port
}

pub async fn fetch_librehwm(client: &reqwest::Client, url: &str) -> Result<Vec<LibreReading>, String> {
    if !is_loopback_librehwm_url(url) {
        return Err("URL do LibreHWM deve ser http://127.0.0.1:<porta>/... ou localhost".into());
    }
    let mut resp = client
        .get(url)
        .timeout(Duration::from_millis(300))
        .send()
        .await
        .map_err(|e| format!("Offline: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    if resp.content_length().unwrap_or(0) > 2 * 1024 * 1024 {
        return Err("Resposta LibreHWM excedeu 2 MiB".into());
    }
    let mut bytes = Vec::with_capacity(65536);
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Falha ao ler corpo: {e}"))? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err("Resposta LibreHWM excedeu 2 MiB".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let json: Value = serde_json::from_slice(&bytes).map_err(|e| format!("JSON inválido: {e}"))?;
    let mut out = Vec::new();
    let mut visited = 0usize;
    walk_tree(&json, "", &mut out, 0, &mut visited);
    Ok(out)
}

fn walk_tree(node: &Value, path: &str, out: &mut Vec<LibreReading>, depth: usize, visited: &mut usize) {
    if depth > 32 || *visited >= 4096 || out.len() >= 2048 {
        return;
    }
    *visited += 1;
    let text = node.get("Text").and_then(|v| v.as_str()).unwrap_or("");
    let val_str = node.get("Value").and_then(|v| v.as_str()).unwrap_or("");
    let next_path = if text.is_empty() {
        path.to_string()
    } else if path.is_empty() {
        text.chars().take(128).collect()
    } else {
        format!("{path}/{text}").chars().take(256).collect()
    };

    if let Some((num, unit)) = parse_lhm_value(val_str) {
        let clean_id = format!("LHM_{}", next_path.replace([' ', '/', '-', '(', ')', '#'], "_"));
        out.push(LibreReading {
            id: clean_id,
            label: format!("{text} ({path})"),
            value: num,
            unit,
        });
    }

    if let Some(children) = node.get("Children").and_then(|v| v.as_array()) {
        for ch in children {
            walk_tree(ch, &next_path, out, depth + 1, visited);
        }
    }
}

fn parse_lhm_value(raw: &str) -> Option<(f64, String)> {
    let s = raw.trim();
    if s.is_empty() || s == "Value" || s == "-" {
        return None;
    }
    let mut num_end = 0;
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' || (i == 0 && ch == '-') {
            num_end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if num_end == 0 {
        return None;
    }
    let val: f64 = s[..num_end].replace(',', ".").parse().ok()?;
    if !val.is_finite() {
        return None;
    }
    let unit = s[num_end..].trim().to_string();
    Some((val, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lhm_value_strings() {
        assert_eq!(parse_lhm_value("56,4 °C"), Some((56.4, "°C".into())));
        assert_eq!(parse_lhm_value("104.2 W"), Some((104.2, "W".into())));
        assert_eq!(parse_lhm_value("Value"), None);
    }
}
