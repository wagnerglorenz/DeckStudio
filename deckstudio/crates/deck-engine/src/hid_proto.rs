//! Protocolo HID vendor-defined (512 bytes, v1/v2) e sanitização JPEG para AJAZZ AKP153E.
//! Regras obrigatórias: nunca enviar comandos de logo/boot, sleep/wakeup ou firmware.

use hidapi::{HidApi, HidDevice};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, RgbImage};
use std::collections::HashSet;
use std::ffi::CStr;

pub const VENDOR_ID: u16 = 0x0300;
pub const PRODUCT_ID: u16 = 0x1010;
pub const IMAGE_CHUNK_SIZE: usize = 512;
pub const REPORT_PAYLOAD_SIZE: usize = 512;
pub const BUTTON_IMAGE_MAX_SIZE: usize = 1024 * 10; // 10 KiB
pub const DEFAULT_TILE_SIZE: u32 = 96;

pub const INPUT_REQUEST_PREFIX: [u8; 9] = [0x41, 0x43, 0x4B, 0x00, 0x00, 0x4F, 0x4B, 0x00, 0x00];
pub const OUTPUT_REQUEST_PREFIX: [u8; 5] = [0x43, 0x52, 0x54, 0x00, 0x00];

const CMD_INIT: [u8; 8] = [0x43, 0x52, 0x54, 0x00, 0x00, 0x44, 0x49, 0x53];
const CMD_KEEPALIVE: [u8; 12] = [0x43, 0x52, 0x54, 0x00, 0x00, 0x43, 0x4F, 0x4E, 0x4E, 0x45, 0x43, 0x54];
const CMD_FLUSH_BUFFER: [u8; 8] = [0x43, 0x52, 0x54, 0x00, 0x00, 0x53, 0x54, 0x50];
const CMD_BACKLIGHT: [u8; 11] = [0x43, 0x52, 0x54, 0x00, 0x00, 0x4C, 0x49, 0x47, 0x00, 0x00, 0x32];
const CMD_CLEAR_BUTTON: [u8; 12] = [0x43, 0x52, 0x54, 0x00, 0x00, 0x43, 0x4C, 0x45, 0x00, 0x00, 0x00, 0x00];
const CMD_CLEAR_ALL: [u8; 12] = [0x43, 0x52, 0x54, 0x00, 0x00, 0x43, 0x4C, 0x45, 0x00, 0x00, 0x00, 0xFF];
const CMD_IMG_ANNOUNCE: [u8; 13] = [0x43, 0x52, 0x54, 0x00, 0x00, 0x42, 0x41, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Press,
    Release,
}

/// Verifica via Toolhelp se `Stream Dock AJAZZ.exe` ou `QtWebEngineProcess.exe` está aberto.
pub fn find_official_app_processes() -> Vec<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };

    let mut found = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return found;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if name.eq_ignore_ascii_case("Stream Dock AJAZZ.exe") {
                    if !found.contains(&name) {
                        found.push(name);
                    }
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    found
}

/// Sanitiza uma imagem RGB para JPEG seguro do AKP153E:
/// - múltiplo de 16 e <= 96 px
/// - rotacionada 90° anti-horário
/// - RGB puro, sem blocos EXIF/ICC
/// - tamanho <= 10 KiB
pub fn sanitize_jpeg(img: &RgbImage, size: u32, rotate_ccw_90: bool) -> Result<Vec<u8>, String> {
    if size == 0 || size > 96 || size % 16 != 0 {
        return Err(format!("size inválido: {size} (deve ser múltiplo de 16 entre 16 e 96)"));
    }

    let dyn_img = DynamicImage::ImageRgb8(img.clone());
    let resized = if dyn_img.width() != size || dyn_img.height() != size {
        dyn_img.resize_exact(size, size, FilterType::Lanczos3)
    } else {
        dyn_img
    };
    let rotated = if rotate_ccw_90 { resized.rotate270() } else { resized };
    let clean_rgb = rotated.to_rgb8();

    for quality in [90u8, 82, 74, 65, 55, 45, 35, 25, 15, 10, 5] {
        let mut buf = Vec::with_capacity(6144);
        let mut enc = JpegEncoder::new_with_quality(&mut buf, quality);
        enc.encode_image(&clean_rgb)
            .map_err(|e| format!("Falha ao codificar JPEG: {e}"))?;
        if buf.len() <= BUTTON_IMAGE_MAX_SIZE {
            return Ok(buf);
        }
    }
    Err(format!("Imagem excedeu limite de {} bytes mesmo na qualidade mínima", BUTTON_IMAGE_MAX_SIZE))
}

pub fn parse_input_report(
    data: &[u8],
    v1_press_only: bool,
    pressed_v2: &mut HashSet<u8>,
) -> Option<(u8, InputAction)> {
    if data.len() <= 10 || !data.starts_with(&INPUT_REQUEST_PREFIX) {
        return None;
    }
    let pos = data[9];
    let raw_act = data[10];
    match raw_act {
        0x01 => {
            pressed_v2.insert(pos);
            Some((pos, InputAction::Press))
        }
        0x00 => {
            if pressed_v2.remove(&pos) {
                Some((pos, InputAction::Release))
            } else if v1_press_only {
                Some((pos, InputAction::Press))
            } else {
                Some((pos, InputAction::Release))
            }
        }
        _ => None,
    }
}

pub struct Akp153eProto {
    dev: Option<HidDevice>,
    pressed_v2: HashSet<u8>,
}

impl Akp153eProto {
    pub fn new() -> Self {
        Self {
            dev: None,
            pressed_v2: HashSet::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.dev.is_some()
    }

    pub fn open(&mut self) -> Result<(), String> {
        let api = HidApi::new().map_err(|e| format!("HidApi::new falhou: {e}"))?;
        let target = api
            .device_list()
            .find(|info| {
                info.vendor_id() == VENDOR_ID
                    && info.product_id() == PRODUCT_ID
                    && info.usage_page() >= 0xFF00
            })
            .ok_or_else(|| "Interface HID vendor-defined (0300:1010) não encontrada".to_string())?;

        let path_cstr: &CStr = target.path();
        let dev = api
            .open_path(path_cstr)
            .map_err(|e| format!("Falha ao abrir path HID: {e}"))?;
        let _ = dev.set_blocking_mode(true);
        self.dev = Some(dev);
        self.write_raw(&CMD_INIT)?;
        Ok(())
    }

    pub fn close(&mut self, clear: bool) {
        if self.dev.is_some() {
            if clear {
                let _ = self.clear_all();
            }
            self.dev = None;
        }
    }

    fn write_raw(&self, payload: &[u8]) -> Result<(), String> {
        let dev = self.dev.as_ref().ok_or_else(|| "Dispositivo HID fechado".to_string())?;
        let mut pkt = vec![0u8; 1 + REPORT_PAYLOAD_SIZE];
        let copy_len = payload.len().min(REPORT_PAYLOAD_SIZE);
        pkt[1..1 + copy_len].copy_from_slice(&payload[..copy_len]);
        dev.write(&pkt).map_err(|e| format!("HID write falhou: {e}"))?;
        Ok(())
    }

    pub fn keepalive(&self) -> Result<(), String> {
        self.write_raw(&CMD_KEEPALIVE)
    }

    pub fn set_brightness(&self, brightness: u8) -> Result<(), String> {
        let clamped = brightness.min(100);
        let mut pkt = CMD_BACKLIGHT;
        pkt[10] = clamped;
        self.write_raw(&pkt)
    }

    pub fn clear_all(&self) -> Result<(), String> {
        self.write_raw(&CMD_CLEAR_ALL)?;
        self.write_raw(&CMD_FLUSH_BUFFER)
    }

    pub fn clear_button(&self, position: u8) -> Result<(), String> {
        let mut pkt = CMD_CLEAR_BUTTON;
        pkt[11] = position;
        self.write_raw(&pkt)?;
        self.write_raw(&CMD_FLUSH_BUFFER)
    }

    pub fn set_button_jpeg(&self, position: u8, jpeg: &[u8]) -> Result<(), String> {
        let size = jpeg.len();
        if size == 0 || size > BUTTON_IMAGE_MAX_SIZE {
            return Err(format!("JPEG inválido ou acima de 10 KB: {size}"));
        }
        let mut announce = CMD_IMG_ANNOUNCE;
        announce[10] = (size / 256) as u8;
        announce[11] = (size % 256) as u8;
        announce[12] = position;
        self.write_raw(&announce)?;
        for chunk in jpeg.chunks(IMAGE_CHUNK_SIZE) {
            self.write_raw(chunk)?;
        }
        self.write_raw(&CMD_FLUSH_BUFFER)
    }

    pub fn read_event(&mut self, timeout_ms: i32) -> Result<Option<(u8, InputAction)>, String> {
        let dev = self.dev.as_ref().ok_or_else(|| "Dispositivo HID fechado".to_string())?;
        let mut buf = [0u8; 512];
        let timeout = timeout_ms.max(50);
        let n = dev.read_timeout(&mut buf, timeout).map_err(|e| format!("HID read falhou: {e}"))?;
        if n == 0 {
            return Ok(None);
        }
        Ok(parse_input_report(&buf[..n], true, &mut self.pressed_v2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageReader;
    use std::io::Cursor;

    #[test]
    fn sanitize_high_entropy_noise_fits_under_10kb() {
        let mut raw = vec![0u8; 96 * 96 * 3];
        let mut state: u32 = 0x1234_5678;
        for b in &mut raw {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            *b = (state >> 24) as u8;
        }
        let img = RgbImage::from_raw(96, 96, raw).unwrap();
        let jpeg = sanitize_jpeg(&img, 96, true).unwrap();
        assert!(jpeg.len() <= BUTTON_IMAGE_MAX_SIZE);

        let decoded = ImageReader::new(Cursor::new(&jpeg)).with_guessed_format().unwrap().decode().unwrap();
        assert_eq!(decoded.width(), 96);
        assert_eq!(decoded.height(), 96);
    }

    #[test]
    fn sanitize_rejects_invalid_sizes() {
        let img = RgbImage::new(64, 64);
        assert!(sanitize_jpeg(&img, 100, true).is_err());
        assert!(sanitize_jpeg(&img, 112, true).is_err());
        assert!(sanitize_jpeg(&img, 0, true).is_err());
    }

    #[test]
    fn parse_input_v1_and_v2() {
        let mut press = INPUT_REQUEST_PREFIX.to_vec();
        press.extend_from_slice(&[5, 0x01]);
        let mut release = INPUT_REQUEST_PREFIX.to_vec();
        release.extend_from_slice(&[5, 0x00]);

        let mut v2 = HashSet::new();
        assert_eq!(parse_input_report(&press, true, &mut v2), Some((5, InputAction::Press)));
        assert_eq!(parse_input_report(&release, true, &mut v2), Some((5, InputAction::Release)));
        assert_eq!(parse_input_report(&release, true, &mut v2), Some((5, InputAction::Press)));
        assert_eq!(parse_input_report(b"INVALID_PKT_DATA", true, &mut v2), None);
    }
}
