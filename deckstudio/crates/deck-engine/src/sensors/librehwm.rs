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

pub async fn fetch_librehwm(client: &reqwest::Client, url: &str) -> Result<Vec<LibreReading>, String> {
    let resp = client
        .get(url)
        .timeout(Duration::from_millis(300))
        .send()
        .await
        .map_err(|e| format!("Offline: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let json: Value = resp.json().await.map_err(|e| format!("JSON inválido: {e}"))?;
    let mut out = Vec::new();
    walk_tree(&json, "", &mut out);
    Ok(out)
}

fn walk_tree(node: &Value, path: &str, out: &mut Vec<LibreReading>) {
    let text = node.get("Text").and_then(|v| v.as_str()).unwrap_or("");
    let val_str = node.get("Value").and_then(|v| v.as_str()).unwrap_or("");
    let next_path = if text.is_empty() {
        path.to_string()
    } else if path.is_empty() {
        text.to_string()
    } else {
        format!("{path}/{text}")
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
            walk_tree(ch, &next_path, out);
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
