//! Telemetria nativa 100% embutida no Windows + GPU NVIDIA via `nvml.dll` dinâmica.
//! Não requer AIDA64, HWiNFO ou LibreHardwareMonitor abertos.

use libloading::Library;
use std::ffi::c_void;
use std::time::Instant;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::NetworkManagement::IpHelper::{GetIfTable, MIB_IFTABLE};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

#[derive(Debug, Clone, Default)]
pub struct NativeSnapshot {
    pub cpu_util: f64,
    pub ram_util: f64,
    pub ram_used_mb: f64,
    pub net_dl_mbps: f64,
    pub net_ul_mbps: f64,
    pub nic_name: String,
    pub nic_discards: u64,
    pub nvml_available: bool,
    pub gpu_name: String,
    pub gpu_util: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub gpu_power_w: Option<f64>,
    pub gpu_fan_pct: Option<f64>,
    pub gpu_clock_mhz: Option<f64>,
    pub vram_used_mb: Option<f64>,
    pub vram_total_mb: Option<f64>,
    pub vram_pct: Option<f64>,
}

pub struct NativeMonitor {
    prev_cpu: Option<(u64, u64, u64)>,
    prev_net: Option<(u64, u64, Instant)>,
    nvml: Option<NvmlHandle>,
    nvml_tried: bool,
}

struct NvmlHandle {
    _lib: Library,
    device: *mut c_void,
    fn_shutdown: unsafe extern "C" fn() -> i32,
    fn_get_temp: unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32,
    fn_get_util: unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> i32,
    fn_get_mem: unsafe extern "C" fn(*mut c_void, *mut NvmlMemory) -> i32,
    fn_get_power: Option<unsafe extern "C" fn(*mut c_void, *mut u32) -> i32>,
    fn_get_fan: Option<unsafe extern "C" fn(*mut c_void, *mut u32) -> i32>,
    fn_get_clock: Option<unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32>,
    gpu_name: String,
}

unsafe impl Send for NvmlHandle {}
unsafe impl Sync for NvmlHandle {}

#[repr(C)]
#[derive(Default)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}

#[repr(C)]
#[derive(Default)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

impl Drop for NvmlHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = (self.fn_shutdown)();
        }
    }
}

impl NvmlHandle {
    fn try_open() -> Option<Self> {
        unsafe {
            let lib = Library::new("nvml.dll").ok()?;
            let init: unsafe extern "C" fn() -> i32 = *lib.get::<unsafe extern "C" fn() -> i32>(b"nvmlInit_v2\0").ok()?;
            if init() != 0 {
                return None;
            }
            let shutdown: unsafe extern "C" fn() -> i32 = *lib.get::<unsafe extern "C" fn() -> i32>(b"nvmlShutdown\0").ok()?;
            let get_dev: unsafe extern "C" fn(u32, *mut *mut c_void) -> i32 =
                *lib.get::<unsafe extern "C" fn(u32, *mut *mut c_void) -> i32>(b"nvmlDeviceGetHandleByIndex_v2\0").ok()?;
            let mut dev: *mut c_void = std::ptr::null_mut();
            if get_dev(0, &mut dev) != 0 || dev.is_null() {
                let _ = shutdown();
                return None;
            }

            let mut name_buf = [0u8; 96];
            let mut gpu_name = "NVIDIA GPU".to_string();
            if let Ok(get_name) = lib.get::<unsafe extern "C" fn(*mut c_void, *mut u8, u32) -> i32>(b"nvmlDeviceGetName\0") {
                if get_name(dev, name_buf.as_mut_ptr(), name_buf.len() as u32) == 0 {
                    let nul = name_buf.iter().position(|&b| b == 0).unwrap_or(name_buf.len());
                    gpu_name = String::from_utf8_lossy(&name_buf[..nul]).to_string();
                }
            }

            let fn_get_temp = *lib.get::<unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32>(b"nvmlDeviceGetTemperature\0").ok()?;
            let fn_get_util = *lib.get::<unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> i32>(b"nvmlDeviceGetUtilizationRates\0").ok()?;
            let fn_get_mem = *lib.get::<unsafe extern "C" fn(*mut c_void, *mut NvmlMemory) -> i32>(b"nvmlDeviceGetMemoryInfo\0").ok()?;
            let fn_get_power = lib.get::<unsafe extern "C" fn(*mut c_void, *mut u32) -> i32>(b"nvmlDeviceGetPowerUsage\0").ok().map(|s| *s);
            let fn_get_fan = lib.get::<unsafe extern "C" fn(*mut c_void, *mut u32) -> i32>(b"nvmlDeviceGetFanSpeed\0").ok().map(|s| *s);
            let fn_get_clock = lib.get::<unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32>(b"nvmlDeviceGetClockInfo\0").ok().map(|s| *s);

            Some(Self {
                _lib: lib,
                device: dev,
                fn_shutdown: shutdown,
                fn_get_temp,
                fn_get_util,
                fn_get_mem,
                fn_get_power,
                fn_get_fan,
                fn_get_clock,
                gpu_name,
            })
        }
    }

    fn poll(&self, snap: &mut NativeSnapshot) {
        snap.nvml_available = true;
        snap.gpu_name = self.gpu_name.clone();
        unsafe {
            let mut temp = 0u32;
            if (self.fn_get_temp)(self.device, 0, &mut temp) == 0 {
                snap.gpu_temp = Some(temp as f64);
            }
            let mut util = NvmlUtilization::default();
            if (self.fn_get_util)(self.device, &mut util) == 0 {
                snap.gpu_util = Some(util.gpu as f64);
            }
            let mut mem = NvmlMemory::default();
            if (self.fn_get_mem)(self.device, &mut mem) == 0 && mem.total > 0 {
                let used_mb = mem.used as f64 / (1024.0 * 1024.0);
                let total_mb = mem.total as f64 / (1024.0 * 1024.0);
                snap.vram_used_mb = Some(used_mb);
                snap.vram_total_mb = Some(total_mb);
                snap.vram_pct = Some((used_mb / total_mb) * 100.0);
            }
            if let Some(f) = self.fn_get_power {
                let mut mw = 0u32;
                if f(self.device, &mut mw) == 0 {
                    snap.gpu_power_w = Some(mw as f64 / 1000.0);
                }
            }
            if let Some(f) = self.fn_get_fan {
                let mut pct = 0u32;
                if f(self.device, &mut pct) == 0 {
                    snap.gpu_fan_pct = Some(pct as f64);
                }
            }
            if let Some(f) = self.fn_get_clock {
                let mut mhz = 0u32;
                if f(self.device, 0, &mut mhz) == 0 {
                    snap.gpu_clock_mhz = Some(mhz as f64);
                }
            }
        }
    }
}

impl NativeMonitor {
    pub fn new() -> Self {
        Self {
            prev_cpu: None,
            prev_net: None,
            nvml: None,
            nvml_tried: false,
        }
    }

    pub fn sample(&mut self, poll_nvml: bool) -> NativeSnapshot {
        let mut snap = NativeSnapshot::default();
        snap.cpu_util = self.sample_cpu();
        let (ram_pct, ram_used) = sample_ram();
        snap.ram_util = ram_pct;
        snap.ram_used_mb = ram_used;

        let (rx, tx, discards, nic) = sample_net_octets();
        snap.nic_name = nic;
        snap.nic_discards = discards;
        let now = Instant::now();
        if let Some((prx, ptx, pt)) = self.prev_net {
            let dt = now.duration_since(pt).as_secs_f64();
            if dt > 0.05 {
                let drx = rx.wrapping_sub(prx) & 0xFFFF_FFFF;
                let dtx = tx.wrapping_sub(ptx) & 0xFFFF_FFFF;
                snap.net_dl_mbps = (drx as f64) * 8.0 / 1e6 / dt;
                snap.net_ul_mbps = (dtx as f64) * 8.0 / 1e6 / dt;
            }
        }
        self.prev_net = Some((rx, tx, now));

        if poll_nvml {
            if !self.nvml_tried {
                self.nvml_tried = true;
                self.nvml = NvmlHandle::try_open();
            }
            if let Some(nvml) = &self.nvml {
                nvml.poll(&mut snap);
            }
        } else if self.nvml.is_some() {
            self.nvml = None;
            self.nvml_tried = false;
        }
        snap
    }

    fn sample_cpu(&mut self) -> f64 {
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe {
            if GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).is_err() {
                return 0.0;
            }
        }
        let i = ft_u64(idle);
        let k = ft_u64(kernel);
        let u = ft_u64(user);
        let pct = if let Some((pi, pk, pu)) = self.prev_cpu {
            let di = i.saturating_sub(pi);
            let dk = k.saturating_sub(pk);
            let du = u.saturating_sub(pu);
            let total = dk + du;
            if total > 0 {
                ((total.saturating_sub(di)) as f64 / total as f64) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        self.prev_cpu = Some((i, k, u));
        pct.clamp(0.0, 100.0)
    }
}

fn ft_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

fn sample_ram() -> (f64, f64) {
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut ms).is_ok() && ms.ullTotalPhys > 0 {
            let used = ms.ullTotalPhys.saturating_sub(ms.ullAvailPhys) as f64 / (1024.0 * 1024.0);
            return (ms.dwMemoryLoad as f64, used);
        }
    }
    (0.0, 0.0)
}

fn is_virtual_or_filter(descr: &str) -> bool {
    let low = descr.to_ascii_lowercase();
    for pat in [
        "hyper-v",
        "virtual",
        "tailscale",
        "bluetooth",
        "wireguard",
        "loopback",
        "vpn",
        "tap",
        "wan miniport",
        "filter",
        "qos",
        "packet scheduler",
        "lightweight",
    ] {
        if low.contains(pat) {
            return true;
        }
    }
    false
}

fn sample_net_octets() -> (u64, u64, u64, String) {
    unsafe {
        let mut size: u32 = 0;
        let _ = GetIfTable(None, &mut size, false);
        if size == 0 {
            return (0, 0, 0, String::new());
        }
        let mut buf = vec![0u8; size as usize];
        let table_ptr = buf.as_mut_ptr() as *mut MIB_IFTABLE;
        if GetIfTable(Some(table_ptr), &mut size, false) != 0 {
            return (0, 0, 0, String::new());
        }
        let num = (*table_ptr).dwNumEntries as usize;
        let rows = std::slice::from_raw_parts((*table_ptr).table.as_ptr(), num);
        let mut rx: u64 = 0;
        let mut tx: u64 = 0;
        let mut discards: u64 = 0;
        let mut nic_name = String::new();
        for r in rows {
            // MIB_IF_OPER_STATUS 4/5 (CONNECTED/OPERATIONAL), tipo 6=Ethernet, 71=Wi-Fi
            let oper = r.dwOperStatus.0 as u32;
            if (oper == 4 || oper == 5) && (r.dwType == 6 || r.dwType == 71) {
                let dlen = (r.dwDescrLen as usize).min(r.bDescr.len());
                let descr: String = r.bDescr[..dlen]
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as char)
                    .collect();
                if !is_virtual_or_filter(&descr) {
                    rx += r.dwInOctets as u64;
                    tx += r.dwOutOctets as u64;
                    discards += r.dwInDiscards as u64;
                    if nic_name.is_empty() {
                        nic_name = descr;
                    }
                }
            }
        }
        (rx, tx, discards, nic_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_filter_drops_qos_and_wfp() {
        assert!(is_virtual_or_filter("Realtek PCIe GbE Family Controller #2-QoS Packet Scheduler-0000"));
        assert!(is_virtual_or_filter("Realtek PCIe GbE Family Controller #2-WFP Native MAC Layer LightWeight Filter-0000"));
        assert!(!is_virtual_or_filter("Realtek PCIe GbE Family Controller #2"));
    }
}
