//! SensorHub: agrega AIDA64, LibreHardwareMonitor, HWiNFO64 e Windows Nativo + NVIDIA NVML.
//! Fornece fallback automático por prioridade, aliases universais e guias de configuração.

pub mod aida64;
pub mod hwinfo;
pub mod librehwm;
pub mod native_win;

use aida64::{aida_unit_hint, Aida64Reader};
use deck_core::{SensorReading, SensorSourcesConfig, SourceStatus};
use native_win::{NativeMonitor, NativeSnapshot};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

pub fn should_poll_nvml(cfg: &SensorSourcesConfig, has_other_gpu: bool) -> bool {
    cfg.native_nvml_enabled
        && (cfg.priority.first().map(|s| s == "native_nvml").unwrap_or(false) || !has_other_gpu)
}

pub struct SensorHub {
    aida: Aida64Reader,
    native: NativeMonitor,
    http: reqwest::Client,
    last_libre: Vec<librehwm::LibreReading>,
    last_libre_err: Option<String>,
    next_libre_probe: Option<Instant>,
    history: HashMap<String, VecDeque<f64>>,
    pub last_sources: Vec<SourceStatus>,
    pub last_readings: Vec<SensorReading>,
    pub last_native: NativeSnapshot,
    values: HashMap<String, f64>,
}

impl SensorHub {
    pub fn new() -> Self {
        Self {
            aida: Aida64Reader::new(),
            native: NativeMonitor::new(),
            http: reqwest::Client::new(),
            last_libre: Vec::new(),
            last_libre_err: None,
            next_libre_probe: None,
            history: HashMap::new(),
            last_sources: Vec::new(),
            last_readings: Vec::new(),
            last_native: NativeSnapshot::default(),
            values: HashMap::new(),
        }
    }

    pub fn reset_backoff(&mut self) {
        self.next_libre_probe = None;
    }

    pub async fn refresh(&mut self, cfg: &SensorSourcesConfig) -> HashMap<String, f64> {
        let mut map: HashMap<String, f64> = HashMap::new();
        let mut readings: Vec<SensorReading> = Vec::new();
        let mut by_source: HashMap<String, HashMap<String, f64>> = HashMap::new();

        // 1. AIDA64 Shared Memory
        let mut aida_count = 0usize;
        let mut aida_live = false;
        let mut aida_map = HashMap::new();
        if cfg.aida64_enabled {
            let (items, fresh) = self.aida.read(cfg.aida64_watchdog);
            aida_live = fresh || !items.is_empty();
            aida_count = items.len();
            for it in &items {
                let u = aida_unit_hint(&it.kind, &it.id);
                push_reading(&mut readings, &mut aida_map, &mut map, &it.id, &it.label, "aida64", it.value, u);
            }
            if let Some(&vram_mb) = aida_map.get("SGPU1USEDDEMEM") {
                if cfg.vram_total_mb > 0.0 {
                    let pct = (vram_mb / cfg.vram_total_mb) * 100.0;
                    aida_map.insert("VRAM_PCT".into(), pct);
                    map.insert("VRAM_PCT".into(), pct);
                }
            }
        }

        // 2. LibreHardwareMonitor Web Server (com backoff de 30s quando offline)
        let mut libre_count = 0usize;
        let mut libre_live = false;
        let mut libre_map = HashMap::new();
        if cfg.librehwm_enabled && !cfg.librehwm_url.is_empty() {
            let can_probe = self.next_libre_probe.map(|t| Instant::now() >= t).unwrap_or(true);
            if can_probe {
                match librehwm::fetch_librehwm(&self.http, &cfg.librehwm_url).await {
                    Ok(items) => {
                        libre_live = !items.is_empty();
                        libre_count = items.len();
                        self.last_libre = items;
                        self.last_libre_err = None;
                        self.next_libre_probe = None;
                    }
                    Err(e) => {
                        self.last_libre.clear();
                        self.last_libre_err = Some(e);
                        self.next_libre_probe = Some(Instant::now() + Duration::from_secs(30));
                    }
                }
            } else if !self.last_libre.is_empty() {
                libre_live = true;
                libre_count = self.last_libre.len();
            } else if self.last_libre_err.is_none() {
                self.last_libre_err = Some("Standby (próxima checagem em 30s)".into());
            }
            for it in &self.last_libre {
                push_reading(&mut readings, &mut libre_map, &mut map, &it.id, &it.label, "librehwm", it.value, &it.unit);
                let low = it.label.to_ascii_lowercase();
                if low.contains("cpu") && low.contains("package") && it.unit.contains("°C") {
                    libre_map.entry("CPU_TEMP".into()).or_insert(it.value);
                } else if low.contains("cpu") && low.contains("package") && it.unit == "W" {
                    libre_map.entry("CPU_WATT".into()).or_insert(it.value);
                } else if low.contains("cpu total") && it.unit == "%" {
                    libre_map.entry("CPU_UTIL".into()).or_insert(it.value);
                } else if low.contains("gpu core") && it.unit.contains("°C") {
                    libre_map.entry("GPU_TEMP".into()).or_insert(it.value);
                } else if low.contains("gpu core") && it.unit == "%" {
                    libre_map.entry("GPU_UTIL".into()).or_insert(it.value);
                } else if low.contains("gpu power") && it.unit == "W" {
                    libre_map.entry("GPU_WATT".into()).or_insert(it.value);
                } else if low.contains("gpu fan") && it.unit == "%" {
                    libre_map.entry("GPU_FAN".into()).or_insert(it.value);
                } else if low.contains("memory") && it.unit == "%" && !low.contains("gpu") {
                    libre_map.entry("RAM_UTIL".into()).or_insert(it.value);
                }
            }
        } else {
            self.last_libre.clear();
            self.last_libre_err = Some("Desativado na configuração".into());
            self.next_libre_probe = None;
        }

        // 3. HWiNFO64 Shared Memory
        let mut hwinfo_count = 0usize;
        let mut hwinfo_live = false;
        let mut hwinfo_map = HashMap::new();
        if cfg.hwinfo_enabled {
            if let Some(items) = hwinfo::read_hwinfo_shm() {
                hwinfo_live = !items.is_empty();
                hwinfo_count = items.len();
                for it in &items {
                    push_reading(&mut readings, &mut hwinfo_map, &mut map, &it.id, &it.label, "hwinfo", it.value, &it.unit);
                    let low = it.label.to_ascii_lowercase();
                    if (low == "cpu (tctl/tdie)" || low == "cpu package") && it.unit.contains("C") {
                        hwinfo_map.entry("CPU_TEMP".into()).or_insert(it.value);
                    } else if low == "total cpu usage" && it.unit == "%" {
                        hwinfo_map.entry("CPU_UTIL".into()).or_insert(it.value);
                    } else if low == "cpu package power" && it.unit == "W" {
                        hwinfo_map.entry("CPU_WATT".into()).or_insert(it.value);
                    } else if low == "gpu temperature" && it.unit.contains("C") {
                        hwinfo_map.entry("GPU_TEMP".into()).or_insert(it.value);
                    } else if low == "gpu core load" && it.unit == "%" {
                        hwinfo_map.entry("GPU_UTIL".into()).or_insert(it.value);
                    } else if low == "gpu power" && it.unit == "W" {
                        hwinfo_map.entry("GPU_WATT".into()).or_insert(it.value);
                    }
                }
            }
        }

        // 4. Windows Nativo + NVML (NVML só ativa se nenhuma outra fonte fornece GPU ou se native_nvml for #1)
        let has_other_gpu = aida_map.contains_key("SGPU1UTI")
            || aida_map.contains_key("TGPU1DIO")
            || libre_map.contains_key("GPU_UTIL")
            || libre_map.contains_key("GPU_TEMP")
            || hwinfo_map.contains_key("GPU_UTIL")
            || hwinfo_map.contains_key("GPU_TEMP");
        let need_nvml = should_poll_nvml(cfg, has_other_gpu);

        let nat = if cfg.native_nvml_enabled {
            self.native.sample(need_nvml)
        } else {
            self.native.sample(false);
            NativeSnapshot::default()
        };
        let mut nat_map = HashMap::new();
        let mut nat_count = 0usize;
        if cfg.native_nvml_enabled {
            push_reading(&mut readings, &mut nat_map, &mut map, "NATIVE_CPU_UTIL", "CPU Load (Win32)", "native_nvml", nat.cpu_util, "%");
            push_reading(&mut readings, &mut nat_map, &mut map, "NATIVE_RAM_UTIL", "RAM Load (Win32)", "native_nvml", nat.ram_util, "%");
            push_reading(&mut readings, &mut nat_map, &mut map, "NATIVE_RAM_USED_MB", "RAM Used (Win32)", "native_nvml", nat.ram_used_mb, "MB");
            push_reading(&mut readings, &mut nat_map, &mut map, "NET_DL", "Download (GetIfTable 1x)", "native_nvml", nat.net_dl_mbps, "Mbps");
            push_reading(&mut readings, &mut nat_map, &mut map, "NET_UL", "Upload (GetIfTable 1x)", "native_nvml", nat.net_ul_mbps, "Mbps");
            let tot = nat.net_dl_mbps + nat.net_ul_mbps;
            push_reading(&mut readings, &mut nat_map, &mut map, "NET_TOTAL", "Rede Total (Mbps)", "native_nvml", tot, "Mbps");
            push_reading(&mut readings, &mut nat_map, &mut map, "NET_TOTAL_MBS", "Rede Total (MB/s)", "native_nvml", tot / 8.0, "MB/s");
            nat_count += 7;

            if nat.nvml_available {
                if let Some(v) = nat.gpu_util { push_reading(&mut readings, &mut nat_map, &mut map, "NVML_GPU_UTIL", "GPU Load (NVML)", "native_nvml", v, "%"); nat_count += 1; }
                if let Some(v) = nat.gpu_temp { push_reading(&mut readings, &mut nat_map, &mut map, "NVML_GPU_TEMP", "GPU Temp (NVML)", "native_nvml", v, "°C"); nat_count += 1; }
                if let Some(v) = nat.gpu_power_w { push_reading(&mut readings, &mut nat_map, &mut map, "NVML_GPU_POWER", "GPU Power (NVML)", "native_nvml", v, "W"); nat_count += 1; }
                if let Some(v) = nat.gpu_fan_pct { push_reading(&mut readings, &mut nat_map, &mut map, "NVML_GPU_FAN", "GPU Fan (NVML)", "native_nvml", v, "%"); nat_count += 1; }
                if let Some(v) = nat.gpu_clock_mhz { push_reading(&mut readings, &mut nat_map, &mut map, "NVML_GPU_CLOCK", "GPU Clock (NVML)", "native_nvml", v, "MHz"); nat_count += 1; }
                if let Some(v) = nat.vram_used_mb { push_reading(&mut readings, &mut nat_map, &mut map, "NVML_VRAM_USED_MB", "VRAM Used (NVML)", "native_nvml", v, "MB"); nat_count += 1; }
                if let Some(v) = nat.vram_pct { push_reading(&mut readings, &mut nat_map, &mut map, "NVML_VRAM_PCT", "VRAM % (NVML)", "native_nvml", v, "%"); nat_count += 1; }
            }
        }
        by_source.insert("native_nvml".into(), nat_map);
        by_source.insert("aida64".into(), aida_map);
        by_source.insert("librehwm".into(), libre_map);
        by_source.insert("hwinfo".into(), hwinfo_map);

        // Resolve aliases universais respeitando a ordem de prioridade do usuário
        let aliases = [
            ("GPU_UTIL", vec![("aida64", "SGPU1UTI"), ("librehwm", "GPU_UTIL"), ("hwinfo", "GPU_UTIL"), ("native_nvml", "NVML_GPU_UTIL")]),
            ("GPU_TEMP", vec![("aida64", "TGPU1DIO"), ("librehwm", "GPU_TEMP"), ("hwinfo", "GPU_TEMP"), ("native_nvml", "NVML_GPU_TEMP")]),
            ("GPU_WATT", vec![("aida64", "PGPU1"), ("librehwm", "GPU_WATT"), ("hwinfo", "GPU_WATT"), ("native_nvml", "NVML_GPU_POWER")]),
            ("GPU_FAN", vec![("aida64", "DGPU1"), ("librehwm", "GPU_FAN"), ("native_nvml", "NVML_GPU_FAN")]),
            ("VRAM_PCT", vec![("aida64", "VRAM_PCT"), ("native_nvml", "NVML_VRAM_PCT")]),
            ("CPU_UTIL", vec![("aida64", "SCPUUTI"), ("librehwm", "CPU_UTIL"), ("hwinfo", "CPU_UTIL"), ("native_nvml", "NATIVE_CPU_UTIL")]),
            ("CPU_TEMP", vec![("aida64", "TCPU"), ("librehwm", "CPU_TEMP"), ("hwinfo", "CPU_TEMP")]),
            ("CPU_WATT", vec![("aida64", "PCPUPKG"), ("librehwm", "CPU_WATT"), ("hwinfo", "CPU_WATT")]),
            ("CPU_CLOCK", vec![("aida64", "SCPUCLK")]),
            ("RAM_UTIL", vec![("aida64", "SMEMUTI"), ("librehwm", "RAM_UTIL"), ("native_nvml", "NATIVE_RAM_UTIL")]),
            ("FPS", vec![("aida64", "SRTSSFPS")]),
        ];

        for (alias, candidates) in aliases {
            let mut chosen: Option<f64> = None;
            for src in &cfg.priority {
                if let Some((_, key)) = candidates.iter().find(|(s, _)| s == src) {
                    if let Some(&v) = by_source.get(src).and_then(|m| m.get(*key)) {
                        chosen = Some(v);
                        break;
                    }
                }
            }
            if chosen.is_none() {
                for (src, key) in candidates {
                    if let Some(&v) = by_source.get(src).and_then(|m| m.get(key)) {
                        chosen = Some(v);
                        break;
                    }
                }
            }
            if let Some(v) = chosen {
                map.insert(alias.into(), v);
            }
        }

        // Atualiza históricos (máx 30 pontos)
        for (id, &val) in &map {
            let hist = self.history.entry(id.clone()).or_insert_with(|| VecDeque::with_capacity(30));
            if hist.len() >= 30 {
                hist.pop_front();
            }
            hist.push_back(val);
        }

        self.last_sources = vec![
            SourceStatus {
                id: "aida64".into(),
                name: "AIDA64 Shared Memory".into(),
                active: aida_live,
                sensor_count: aida_count,
                status_text: if aida_live { format!("{aida_count} sensores ativos") } else { "AIDA64 fechado ou SHM desativada".into() },
                setup_guide: vec![
                    "Abra o AIDA64 Extreme/Engineer.".into(),
                    "Vá em Arquivo -> Preferências -> Aplicativos Externos.".into(),
                    "Marque 'Ativar memória compartilhada' (AIDA64_SensorValues) e clique OK.".into(),
                    "Opcional: ative a Vigia Automática para reiniciar a tarefa AIDA64 AutoStart se congelar por 45s.".into(),
                ],
                endpoint: Some("Memória Compartilhada: AIDA64_SensorValues".into()),
            },
            SourceStatus {
                id: "librehwm".into(),
                name: "LibreHardwareMonitor (Gratuito / Open-Source)".into(),
                active: libre_live,
                sensor_count: libre_count,
                status_text: if libre_live {
                    format!("{libre_count} sensores via HTTP")
                } else {
                    self.last_libre_err.clone().unwrap_or_else(|| "Standby".into())
                },
                setup_guide: vec![
                    "Baixe o LibreHardwareMonitor (gratuito e open-source no GitHub).".into(),
                    "Execute o LibreHardwareMonitor.exe como Administrador (necessário para temperatura da CPU).".into(),
                    "No menu superior: Options -> Remote Web Server -> Run (porta padrão 8085).".into(),
                    "O DeckStudio lê automaticamente http://127.0.0.1:8085/data.json.".into(),
                ],
                endpoint: Some(cfg.librehwm_url.clone()),
            },
            SourceStatus {
                id: "hwinfo".into(),
                name: "HWiNFO64 Shared Memory".into(),
                active: hwinfo_live,
                sensor_count: hwinfo_count,
                status_text: if hwinfo_live { format!("{hwinfo_count} sensores ativos") } else { "HWiNFO fechado ou SHM desligada".into() },
                setup_guide: vec![
                    "Abra o HWiNFO64 marcando a opção 'Sensors-only'.".into(),
                    "Clique no ícone de engrenagem (Settings) no canto inferior direito.".into(),
                    "Na aba General / Main Settings, marque 'Shared Memory Support' e clique OK.".into(),
                    "Atenção: a versão gratuita desliga a memória compartilhada após 12h (use AIDA64 ou LibreHWM para 24/7).".into(),
                ],
                endpoint: Some(r"Memória Compartilhada: Global\HWiNFO_SENS_SM2".into()),
            },
            SourceStatus {
                id: "native_nvml".into(),
                name: "Windows Nativo + NVIDIA NVML".into(),
                active: cfg.native_nvml_enabled,
                sensor_count: nat_count,
                status_text: if !cfg.native_nvml_enabled {
                    "Desativado na configuração".into()
                } else if nat.nvml_available {
                    format!("Ativo ({} + Win32 + {})", nat.gpu_name, nat.nic_name)
                } else if !need_nvml {
                    format!("Ativo (Win32 CPU/RAM/Rede: {}; NVML em standby — GPU via fonte prioritária)", nat.nic_name)
                } else {
                    format!("Ativo (Win32 CPU/RAM/Rede; sem NVML)")
                },
                setup_guide: vec![
                    "Não precisa instalar nenhum aplicativo externo!".into(),
                    "Lê CPU % (GetSystemTimes), RAM % (GlobalMemoryStatusEx) e Rede 1x (GetIfTable filtrando filtros WFP/QoS).".into(),
                    "Se houver placa NVIDIA, carrega nvml.dll do próprio driver para ler GPU %, °C, Watt, Fan e VRAM.".into(),
                ],
                endpoint: Some("Win32 API + C:\\Windows\\System32\\nvml.dll".into()),
            },
        ];

        self.last_native = nat;
        self.last_readings = readings;
        self.values = map.clone();
        map
    }

    pub fn get_history(&self, id: &str) -> Vec<f64> {
        self.history
            .get(id)
            .map(|d| d.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn format_sensor(&self, id: &str, decimals: u8) -> (String, Option<f64>) {
        if let Some(&val) = self.values.get(id) {
            let txt = if decimals == 0 {
                format!("{}", val.round() as i64)
            } else {
                format!("{:.*}", decimals as usize, val)
            };
            (txt, Some(val))
        } else {
            ("--".into(), None)
        }
    }
}

fn push_reading(
    readings: &mut Vec<SensorReading>,
    src_map: &mut HashMap<String, f64>,
    global_map: &mut HashMap<String, f64>,
    id: &str,
    label: &str,
    source: &str,
    value: f64,
    unit: &str,
) {
    src_map.insert(id.into(), value);
    global_map.entry(id.into()).or_insert(value);
    readings.push(SensorReading {
        id: id.into(),
        label: label.into(),
        source: source.into(),
        value,
        unit: unit.into(),
    });
}

pub fn format_rate_human(mbps: f64) -> String {
    if mbps < 1.0 {
        format!("{}Kb/s", (mbps * 1000.0).round() as i64)
    } else if mbps < 100.0 {
        format!("{mbps:.1}Mb/s")
    } else {
        format!("{}Mb/s", mbps.round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sensor_hub_always_provides_native_and_4_sources() {
        let mut hub = SensorHub::new();
        let cfg = SensorSourcesConfig::default();
        let vals = hub.refresh(&cfg).await;
        assert!(vals.contains_key("NATIVE_CPU_UTIL"));
        assert!(vals.contains_key("NATIVE_RAM_UTIL"));
        assert!(vals.contains_key("NET_DL"));
        assert_eq!(hub.last_sources.len(), 4);
    }

    #[test]
    fn nvml_polled_only_without_other_gpu_or_when_primary() {
        let mut cfg = SensorSourcesConfig::default();
        assert!(!should_poll_nvml(&cfg, true));
        assert!(should_poll_nvml(&cfg, false));

        cfg.priority = vec!["native_nvml".into(), "aida64".into()];
        assert!(should_poll_nvml(&cfg, true));

        cfg.native_nvml_enabled = false;
        assert!(!should_poll_nvml(&cfg, false));
    }

    #[tokio::test]
    async fn librehwm_sets_backoff_after_connection_failure() {
        let mut hub = SensorHub::new();
        let cfg = SensorSourcesConfig {
            aida64_enabled: false,
            hwinfo_enabled: false,
            librehwm_enabled: true,
            librehwm_url: "http://127.0.0.1:1/data.json".into(),
            ..Default::default()
        };
        let _ = hub.refresh(&cfg).await;
        assert!(hub.next_libre_probe.is_some());
        hub.reset_backoff();
        assert!(hub.next_libre_probe.is_none());
    }
}
