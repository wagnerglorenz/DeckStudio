//! Leitor nativo da memória compartilhada do HWiNFO64 (`Global\HWiNFO_SENS_SM2`).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, VirtualQuery, FILE_MAP_READ,
    MEMORY_BASIC_INFORMATION,
};

const HWINFO_SIG: u32 = 0x5369_5748; // "HWiS"

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct HwinfoHeader {
    dw_signature: u32,
    dw_version: u32,
    dw_revision: u32,
    poll_time: i64,
    dw_offset_sensor_section: u32,
    dw_size_sensor_element: u32,
    dw_num_sensor_elements: u32,
    dw_offset_reading_section: u32,
    dw_size_reading_element: u32,
    dw_num_reading_elements: u32,
}

#[derive(Debug, Clone)]
pub struct HwinfoReading {
    pub id: String,
    pub label: String,
    pub unit: String,
    pub value: f64,
}

pub fn read_hwinfo_shm() -> Option<Vec<HwinfoReading>> {
    for map_name in [r"Global\HWiNFO_SENS_SM2", "HWiNFO_SENS_SM2"] {
        if let Some(items) = try_read_map(map_name) {
            return Some(items);
        }
    }
    None
}

fn try_read_map(map_name: &str) -> Option<Vec<HwinfoReading>> {
    let wide: Vec<u16> = OsStr::new(map_name).encode_wide().chain(Some(0)).collect();
    unsafe {
        let Ok(handle) = OpenFileMappingW(FILE_MAP_READ.0, false, PCWSTR(wide.as_ptr())) else {
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
        let q = VirtualQuery(
            Some(view.Value),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        );
        let parsed = if q > 0
            && mbi.State == windows::Win32::System::Memory::MEM_COMMIT
            && mbi.RegionSize >= std::mem::size_of::<HwinfoHeader>()
        {
            let cap = mbi.RegionSize.min(4_194_304);
            let mut buf = vec![0u8; cap];
            std::ptr::copy_nonoverlapping(view.Value as *const u8, buf.as_mut_ptr(), cap);
            parse_hwinfo_bytes(&buf)
        } else {
            None
        };
        let _ = UnmapViewOfFile(view);
        let _ = CloseHandle(handle);
        parsed
    }
}

fn parse_hwinfo_bytes(bytes: &[u8]) -> Option<Vec<HwinfoReading>> {
    if bytes.len() < std::mem::size_of::<HwinfoHeader>() {
        return None;
    }
    let hdr: HwinfoHeader = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const HwinfoHeader) };
    if hdr.dw_signature != HWINFO_SIG {
        return None;
    }
    let r_off = hdr.dw_offset_reading_section as usize;
    let r_sz = hdr.dw_size_reading_element as usize;
    let r_num = (hdr.dw_num_reading_elements as usize).min(8192);
    // Reading struct: dwReadingType(4) + dwSensorIndex(4) + dwReadingID(4) + szLabelOrig(128) + szLabelUser(128) + szUnit(16) + Value(f64)
    if r_sz < 292 || r_off.checked_add(r_sz.checked_mul(r_num)?)? > bytes.len() {
        return None;
    }

    let mut out = Vec::with_capacity(r_num);
    for i in 0..r_num {
        let base = r_off + i * r_sz;
        let rec = &bytes[base..base + r_sz];
        let reading_id = u32::from_le_bytes(rec[8..12].try_into().ok()?);
        let label_orig = cstr_latin1(&rec[12..140]);
        let label_user = cstr_latin1(&rec[140..268]);
        let unit = cstr_latin1(&rec[268..284]);
        let val = f64::from_le_bytes(rec[284..292].try_into().ok()?);
        let label = if !label_user.is_empty() { label_user } else { label_orig };
        if !label.is_empty() && val.is_finite() {
            out.push(HwinfoReading {
                id: format!("HWINFO_{reading_id:08X}"),
                label,
                unit,
                value: val,
            });
        }
    }
    Some(out)
}

fn cstr_latin1(buf: &[u8]) -> String {
    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf[..nul].iter().map(|&b| b as char).collect::<String>().trim().to_string()
}
