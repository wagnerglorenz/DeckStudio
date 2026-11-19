//! Cliente leve de clima/previsão via Open-Meteo (atualiza a cada 15 min em background).

use deck_core::WeatherConfig;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Default)]
pub struct WeatherSnapshot {
    pub temp_c: Option<f64>,
    pub tmin: Option<f64>,
    pub tmax: Option<f64>,
    pub weather_code: i32,
    pub city: String,
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

pub fn spawn_weather_loop(
    cfg: Arc<RwLock<WeatherConfig>>,
    snap: Arc<RwLock<WeatherSnapshot>>,
    on_log: Arc<dyn Fn(String) + Send + Sync>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
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
                            city: c.city.clone(),
                        };
                    }
                    on_log(format!(
                        "[INFO] Clima {} atualizado: {:.1}°C",
                        c.city, parsed.current.temperature_2m
                    ));
                }
            }
            sleep(Duration::from_secs((c.refresh_minutes.max(1) * 60) as u64)).await;
        }
    });
}
