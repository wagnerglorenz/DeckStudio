//! Leitor nativo da memória compartilhada do AIDA64 (`AIDA64_SensorValues`).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::process::Command;
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, VirtualQuery, FILE_MAP_READ,
    MEMORY_BASIC_INFORMATION,
};

#[derive(Debug, Clone)]
pub struct AidaSensorItem {
    pub kind: String,
    pub id: String,
    pub label: String,
    pub value: f64,
}

pub struct Aida64Reader {
    last_items: Vec<AidaSensorItem>,
    last_hash: u64,
    last_ok: Option<Instant>,
    last_change: Instant,
    last_restart: Option<Instant>,
}

impl Aida64Reader {
    pub fn new() -> Self {
        Self {
            last_items: Vec::new(),
            last_hash: 0,
            last_ok: None,
            last_change: Instant::now(),
            last_restart: None,
        }
    }

    pub fn read(&mut self, enable_watchdog: bool) -> (Vec<AidaSensorItem>, bool) {
        if let Some(raw) = read_aida_raw_shm() {
            let hash = simple_hash(raw.as_bytes());
            let items = parse_aida_xml(&raw);
            if !items.is_empty() {
                if hash != self.last_hash {
                    self.last_hash = hash;
                    self.last_change = Instant::now();
                }
                self.last_items = items;
                self.last_ok = Some(Instant::now());
                return (self.last_items.clone(), true);
            }
        }

        // Vigia de AIDA64 congelado (45s sem publicar após já ter estado ativo, cooldown 90s)
        if enable_watchdog && self.last_ok.is_some() && self.last_change.elapsed() >= Duration::from_secs(45) {
            let can_restart = self
                .last_restart
                .map(|t| t.elapsed() >= Duration::from_secs(90))
                .unwrap_or(true);
            if can_restart {
                self.last_restart = Some(Instant::now());
                self.last_change = Instant::now();
                std::thread::spawn(|| {
                    use std::os::windows::process::CommandExt as _;
                    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
                    let schtasks = std::path::PathBuf::from(root).join("System32").join("schtasks.exe");
                    for args in [
                        ["/End", "/TN", "AIDA64 AutoStart"],
                        ["/Run", "/TN", "AIDA64 AutoStart"],
                    ] {
                        let _ = Command::new(&schtasks).creation_flags(0x0800_0000).args(args).output();
                        std::thread::sleep(Duration::from_secs(2));
                    }
                });
            }
        }

        // Cache de até 30 segundos
        if let Some(ok) = self.last_ok {
            if ok.elapsed() <= Duration::from_secs(30) {
                return (self.last_items.clone(), false);
            }
        }
        (Vec::new(), false)
    }
}

fn simple_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn read_aida_raw_shm() -> Option<String> {
    let name: Vec<u16> = OsStr::new("AIDA64_SensorValues")
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        let Ok(handle) = OpenFileMappingW(FILE_MAP_READ.0, false, PCWSTR(name.as_ptr())) else {
            return None;
        };
        if handle.is_invalid() {
            return None;
        }
        let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0);
        if view.Value.is_null() {
            let _ = CloseHandle(handle);
            return None;
        }
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        let queried = VirtualQuery(
            Some(view.Value),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        );
        let result = if queried > 0
            && mbi.State == windows::Win32::System::Memory::MEM_COMMIT
            && mbi.RegionSize > 0
        {
            let cap = mbi.RegionSize.min(1_048_576);
            let mut buf = vec![0u8; cap];
            std::ptr::copy_nonoverlapping(view.Value as *const u8, buf.as_mut_ptr(), cap);
            let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            // Latin-1 decode (AIDA64 escreve em ANSI/Latin1)
            Some(buf[..nul].iter().map(|&b| b as char).collect())
        } else {
            None
        };
        let _ = UnmapViewOfFile(view);
        let _ = CloseHandle(handle);
        result
    }
}

pub fn parse_aida_xml(raw: &str) -> Vec<AidaSensorItem> {
    let mut out = Vec::new();
    let mut pos = 0;
    let bytes = raw.as_bytes();
    while pos < bytes.len() && out.len() < 2048 {
        let Some(rel_start) = raw[pos..].find('<') else { break };
        let tag_start = pos + rel_start;
        let Some(rel_end) = raw[tag_start + 1..].find('>') else { break };
        let kind = &raw[tag_start + 1..tag_start + 1 + rel_end];
        if kind.is_empty()
            || kind.len() > 24
            || kind.starts_with('/')
            || kind.contains(|c: char| c.is_whitespace() || c == '<')
            || kind == "id"
            || kind == "label"
            || kind == "value"
        {
            pos = tag_start + 1 + rel_end + 1;
            continue;
        }
        let close_pat = format!("</{kind}>");
        let after_kind = tag_start + 1 + rel_end + 1;
        let mut search_end = (after_kind + 512).min(raw.len());
        while search_end > after_kind && !raw.is_char_boundary(search_end) {
            search_end -= 1;
        }
        let Some(rel_close) = raw[after_kind..search_end].find(&close_pat) else {
            pos = after_kind;
            continue;
        };
        let block = &raw[after_kind..after_kind + rel_close];
        if let (Some(id), Some(label), Some(val_str)) = (
            extract_tag(block, "id"),
            extract_tag(block, "label"),
            extract_tag(block, "value"),
        ) {
            if let Ok(value) = val_str.trim().replace(',', ".").parse::<f64>() {
                if value.is_finite() {
                    out.push(AidaSensorItem {
                        kind: kind.to_string(),
                        id: id.to_string(),
                        label: label.to_string(),
                        value,
                    });
                }
            }
        }
        pos = after_kind + rel_close + close_pat.len();
    }
    out
}

fn extract_tag<'a>(s: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = s.find(&open)? + open.len();
    let end = s[start..].find(&close)? + start;
    Some(&s[start..end])
}

pub fn aida_unit_hint(kind: &str, id: &str) -> &'static str {
    match kind {
        "temp" => "°C",
        "fan" => "RPM",
        "duty" => "%",
        "volt" => "V",
        "curr" => "A",
        "pwr" => "W",
        "sys" => {
            if id.ends_with("CLK") || id.ends_with("CLOCK") {
                "MHz"
            } else if id.contains("UTI") {
                "%"
            } else if id.contains("MEM") {
                "MB"
            } else if id.contains("FPS") {
                "fps"
            } else {
                ""
            }
        }
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aida_xml_payload() {
        let sample = "<temp><id>TCPU</id><label>CPU</label><value>57.6</value></temp><pwr><id>PGPU1</id><label>GPU</label><value>150.3</value></pwr><sys><id>SCPUCLK</id><label>CPU Clock</label><value>3550</value></sys>";
        let items = parse_aida_xml(sample);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "TCPU");
        assert!((items[0].value - 57.6).abs() < 0.01);
        assert_eq!(items[1].kind, "pwr");
        assert_eq!(items[2].id, "SCPUCLK");
    }
}
