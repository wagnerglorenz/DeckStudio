//! Cliente leve de clima/previsão via Open-Meteo e busca de cidade/coordenadas por CEP.

use deck_core::WeatherConfig;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;

#[derive(Debug, Clone, Default)]
pub struct WeatherSnapshot {
    pub temp_c: Option<f64>,
    pub tmin: Option<f64>,
    pub tmax: Option<f64>,
    pub weather_code: i32,
}

#[derive(Deserialize)]
struct OpenMeteoResp {
    current: OmCurrent,
    daily: OmDaily,
}

#[derive(Deserialize)]
struct OmCurrent {
    temperature_2m: f64,
    #[serde(default)]
    weather_code: i32,
}

#[derive(Deserialize)]
struct OmDaily {
    temperature_2m_min: Vec<f64>,
    temperature_2m_max: Vec<f64>,
}

#[derive(Deserialize)]
struct BrasilApiCepResp {
    city: Option<String>,
    state: Option<String>,
    location: Option<BrasilApiLocation>,
}

#[derive(Deserialize)]
struct BrasilApiLocation {
    coordinates: Option<BrasilApiCoords>,
}

#[derive(Deserialize)]
struct BrasilApiCoords {
    latitude: Option<serde_json::Value>,
    longitude: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ViaCepResp {
    erro: Option<bool>,
    localidade: Option<String>,
    uf: Option<String>,
}

#[derive(Deserialize)]
struct OpenMeteoGeoResp {
    results: Option<Vec<OpenMeteoGeoItem>>,
}

#[derive(Deserialize)]
struct OpenMeteoGeoItem {
    latitude: f64,
    longitude: f64,
    country_code: Option<String>,
    admin1: Option<String>,
}

fn parse_coord_val(v: &serde_json::Value) -> Option<f64> {
    if let Some(n) = v.as_f64() {
        return n.is_finite().then_some(n);
    }
    if let Some(s) = v.as_str() {
        if let Ok(n) = s.trim().parse::<f64>() {
            return n.is_finite().then_some(n);
        }
    }
    None
}

pub fn normalize_cep(raw: &str) -> Result<(String, String), String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 8 {
        return Err("CEP inválido: informe 8 dígitos (ex.: 97010-000)".into());
    }
    let formatted = format!("{}-{}", &digits[..5], &digits[5..]);
    Ok((digits, formatted))
}

fn uf_to_state_name(uf: &str) -> &'static str {
    match uf.trim().to_ascii_uppercase().as_str() {
        "AC" => "Acre",
        "AL" => "Alagoas",
        "AP" => "Amapá",
        "AM" => "Amazonas",
        "BA" => "Bahia",
        "CE" => "Ceará",
        "DF" => "Distrito Federal",
        "ES" => "Espírito Santo",
        "GO" => "Goiás",
        "MA" => "Maranhão",
        "MT" => "Mato Grosso",
        "MS" => "Mato Grosso do Sul",
        "MG" => "Minas Gerais",
        "PA" => "Pará",
        "PB" => "Paraíba",
        "PR" => "Paraná",
        "PE" => "Pernambuco",
        "PI" => "Piauí",
        "RJ" => "Rio de Janeiro",
        "RN" => "Rio Grande do Norte",
        "RS" => "Rio Grande do Sul",
        "RO" => "Rondônia",
        "RR" => "Roraima",
        "SC" => "Santa Catarina",
        "SP" => "São Paulo",
        "SE" => "Sergipe",
        "TO" => "Tocantins",
        _ => "",
    }
}

pub async fn lookup_brazil_cep(raw_cep: &str) -> Result<(String, String, f64, f64), String> {
    let (digits, formatted) = normalize_cep(raw_cep)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| format!("Falha ao inicializar cliente HTTP: {e}"))?;

    let mut city = String::new();
    let mut uf = String::new();
    let mut coords: Option<(f64, f64)> = None;

    let brasil_url = format!("https://brasilapi.com.br/api/cep/v2/{digits}");
    if let Ok(resp) = client.get(&brasil_url).send().await {
        if resp.status().is_success() {
            if let Ok(parsed) = resp.json::<BrasilApiCepResp>().await {
                if let Some(c) = parsed.city {
                    city = c.trim().to_string();
                }
                if let Some(s) = parsed.state {
                    uf = s.trim().to_string();
                }
                if let Some(c) = parsed.location.and_then(|l| l.coordinates) {
                    if let (Some(lat), Some(lon)) = (
                        c.latitude.as_ref().and_then(parse_coord_val),
                        c.longitude.as_ref().and_then(parse_coord_val),
                    ) {
                        if lat.abs() > 0.001 && lon.abs() > 0.001 {
                            coords = Some((lat, lon));
                        }
                    }
                }
            }
        }
    }

    if city.is_empty() {
        let viacep_url = format!("https://viacep.com.br/ws/{digits}/json/");
        let resp = client
            .get(&viacep_url)
            .send()
            .await
            .map_err(|_| "Não foi possível consultar o CEP".to_string())?;
        if !resp.status().is_success() {
            return Err("CEP não encontrado".into());
        }
        let parsed = resp
            .json::<ViaCepResp>()
            .await
            .map_err(|_| "Resposta inválida ao consultar CEP".to_string())?;
        if parsed.erro == Some(true) {
            return Err("CEP não encontrado".into());
        }
        city = parsed.localidade.unwrap_or_default().trim().to_string();
        uf = parsed.uf.unwrap_or_default().trim().to_string();
    }

    if city.is_empty() {
        return Err("Cidade não encontrada para este CEP".into());
    }

    let (lat, lon) = if let Some((la, lo)) = coords {
        (la, lo)
    } else {
        let mut geo_url = reqwest::Url::parse("https://geocoding-api.open-meteo.com/v1/search")
            .map_err(|e| e.to_string())?;
        geo_url
            .query_pairs_mut()
            .append_pair("name", &city)
            .append_pair("count", "8")
            .append_pair("language", "pt")
            .append_pair("format", "json");
        let geo_resp = client
            .get(geo_url)
            .send()
            .await
            .map_err(|_| "Falha ao obter coordenadas da cidade".to_string())?;
        let geo = geo_resp
            .json::<OpenMeteoGeoResp>()
            .await
            .map_err(|_| "Resposta inválida de geocodificação".to_string())?;
        let results = geo.results.unwrap_or_default();
        let state_full = uf_to_state_name(&uf);
        let picked = results
            .iter()
            .find(|r| {
                r.country_code.as_deref() == Some("BR")
                    && !state_full.is_empty()
                    && r.admin1
                        .as_deref()
                        .map(|a| a.eq_ignore_ascii_case(state_full))
                        .unwrap_or(false)
            })
            .or_else(|| results.iter().find(|r| r.country_code.as_deref() == Some("BR")))
            .or_else(|| results.first())
            .ok_or_else(|| format!("Coordenadas não encontradas para {city}"))?;
        (picked.latitude, picked.longitude)
    };

    Ok((formatted, city, lat, lon))
}

pub fn spawn_weather_loop(
    cfg: Arc<RwLock<WeatherConfig>>,
    snap: Arc<RwLock<WeatherSnapshot>>,
    weather_notify: Arc<Notify>,
    redraw_notify: Arc<Notify>,
    on_log: Arc<dyn Fn(String) + Send + Sync>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        loop {
            let c = cfg.read().map(|g| g.clone()).unwrap_or_default();
            let url = format!(
                "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=1",
                c.lat, c.lon
            );
            if let Ok(resp) = client.get(&url).timeout(Duration::from_secs(6)).send().await {
                if let Ok(parsed) = resp.json::<OpenMeteoResp>().await {
                    let tmin = parsed.daily.temperature_2m_min.first().copied();
                    let tmax = parsed.daily.temperature_2m_max.first().copied();
                    if let Ok(mut g) = snap.write() {
                        *g = WeatherSnapshot {
                            temp_c: Some(parsed.current.temperature_2m),
                            tmin,
                            tmax,
                            weather_code: parsed.current.weather_code,
                        };
                    }
                    redraw_notify.notify_one();
                    on_log(format!(
                        "[INFO] Clima {} atualizado: {:.1}°C",
                        c.city, parsed.current.temperature_2m
                    ));
                }
            }
            let wait_dur = Duration::from_secs(c.refresh_minutes.max(1) * 60);
            tokio::select! {
                _ = sleep(wait_dur) => {}
                _ = weather_notify.notified() => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_cep_formats_valid_and_rejects_invalid() {
        assert_eq!(normalize_cep("97010-000").unwrap(), ("97010000".into(), "97010-000".into()));
        assert_eq!(normalize_cep("01310100").unwrap(), ("01310100".into(), "01310-100".into()));
        assert!(normalize_cep("1234").is_err());
        assert!(normalize_cep("123456789").is_err());
    }
}
