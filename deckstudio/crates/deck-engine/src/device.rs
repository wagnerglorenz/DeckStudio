//! Gerenciador thread-safe do AKP153E com fila limitada, cache blake3 e leitor sem busy-loop.

use crate::hid_proto::{
    find_official_app_processes, sanitize_jpeg, Akp153eProto, InputAction, DEFAULT_TILE_SIZE,
};
use image::RgbImage;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub fn grid_to_index(grid_key: &str) -> Option<u8> {
    let mut parts = grid_key.split(',');
    let col: i32 = parts.next()?.trim().parse().ok()?;
    let row: i32 = parts.next()?.trim().parse().ok()?;
    if (0..=4).contains(&col) && (0..=2).contains(&row) {
        Some(((4 - col) * 3 + row + 1) as u8)
    } else if col == 5 && (0..=2).contains(&row) {
        Some((16 + row) as u8)
    } else {
        None
    }
}

pub fn index_to_grid(pos: u8) -> Option<String> {
    if (1..=15).contains(&pos) {
        let p = (pos - 1) as i32;
        let col = 4 - (p / 3);
        let row = p % 3;
        Some(format!("{col},{row}"))
    } else if (16..=18).contains(&pos) {
        Some(format!("5,{}", pos - 16))
    } else {
        None
    }
}

struct PendingState {
    pending_tiles: HashMap<u8, ([u8; 32], Vec<u8>)>,
    pending_order: VecDeque<u8>,
    sent_hashes: HashMap<u8, [u8; 32]>,
    pending_brightness: Option<u8>,
    pending_clear_all: bool,
    connected: bool,
}

pub struct DeviceManager {
    state: Arc<(Mutex<PendingState>, Condvar)>,
    stop: Arc<AtomicBool>,
    tiles_sent_counter: Arc<AtomicU64>,
    writer_handle: Option<JoinHandle<()>>,
    reader_handle: Option<JoinHandle<()>>,
}

impl DeviceManager {
    pub fn start<F>(
        max_tiles_per_s: f32,
        initial_brightness: u8,
        on_button: F,
    ) -> Result<Self, String>
    where
        F: Fn(u8, String, InputAction) + Send + Sync + 'static,
    {
        let running = find_official_app_processes();
        if !running.is_empty() {
            return Err(format!(
                "App oficial em execução ({}). Feche-o antes de abrir o HID.",
                running.join(", ")
            ));
        }

        let mut proto = Akp153eProto::new();
        proto.open()?;
        proto.clear_all()?;
        proto.set_brightness(initial_brightness)?;

        // HidDevice em Windows permite leitura e escrita em threads distintas ou compartilhando mutex fino;
        // aqui abrimos uma referência protegida com timeout curto de leitura para evitar contenção.
        let proto_arc = Arc::new(Mutex::new(proto));
        let state = Arc::new((
            Mutex::new(PendingState {
                pending_tiles: HashMap::new(),
                pending_order: VecDeque::new(),
                sent_hashes: HashMap::new(),
                pending_brightness: None,
                pending_clear_all: false,
                connected: true,
            }),
            Condvar::new(),
        ));
        let stop = Arc::new(AtomicBool::new(false));
        let tiles_sent_counter = Arc::new(AtomicU64::new(0));

        let min_interval = Duration::from_secs_f32(1.0 / max_tiles_per_s.max(1.0));
        let writer_handle = {
            let proto_w = Arc::clone(&proto_arc);
            let st_w = Arc::clone(&state);
            let stop_w = Arc::clone(&stop);
            let cnt_w = Arc::clone(&tiles_sent_counter);
            thread::Builder::new()
                .name("akp-writer".into())
                .spawn(move || Self::writer_loop(proto_w, st_w, stop_w, cnt_w, min_interval))
                .map_err(|e| format!("Falha thread writer: {e}"))?
        };

        let reader_handle = {
            let proto_r = Arc::clone(&proto_arc);
            let stop_r = Arc::clone(&stop);
            thread::Builder::new()
                .name("akp-reader".into())
                .spawn(move || Self::reader_loop(proto_r, stop_r, on_button))
                .map_err(|e| format!("Falha thread reader: {e}"))?
        };

        Ok(Self {
            state,
            stop,
            tiles_sent_counter,
            writer_handle: Some(writer_handle),
            reader_handle: Some(reader_handle),
        })
    }

    pub fn is_connected(&self) -> bool {
        self.state.0.lock().map(|s| s.connected).unwrap_or(false)
    }

    pub fn total_tiles_sent(&self) -> u64 {
        self.tiles_sent_counter.load(Ordering::Relaxed)
    }

    pub fn set_brightness(&self, brightness: u8) {
        if let Ok(mut st) = self.state.0.lock() {
            st.pending_brightness = Some(brightness.min(100));
            self.state.1.notify_one();
        }
    }

    pub fn clear_all(&self) {
        if let Ok(mut st) = self.state.0.lock() {
            st.pending_tiles.clear();
            st.pending_order.clear();
            st.sent_hashes.clear();
            st.pending_clear_all = true;
            self.state.1.notify_one();
        }
    }

    pub fn invalidate_cache(&self) {
        if let Ok(mut st) = self.state.0.lock() {
            st.sent_hashes.clear();
        }
    }

    pub fn queue_grid_image(&self, grid_key: &str, img: &RgbImage) -> Result<bool, String> {
        let pos = grid_to_index(grid_key).ok_or_else(|| format!("Posição inválida: {grid_key}"))?;
        let jpeg = sanitize_jpeg(img, DEFAULT_TILE_SIZE, true)?;
        Ok(self.queue_jpeg(pos, jpeg))
    }

    pub fn queue_jpeg(&self, position: u8, jpeg: Vec<u8>) -> bool {
        let digest = *blake3::hash(&jpeg).as_bytes();
        let Ok(mut st) = self.state.0.lock() else {
            return false;
        };
        if st.sent_hashes.get(&position) == Some(&digest) && !st.pending_tiles.contains_key(&position) {
            return false;
        }
        if !st.pending_tiles.contains_key(&position) {
            st.pending_order.push_back(position);
        }
        st.pending_tiles.insert(position, (digest, jpeg));
        self.state.1.notify_one();
        true
    }

    fn writer_loop(
        proto: Arc<Mutex<Akp153eProto>>,
        state: Arc<(Mutex<PendingState>, Condvar)>,
        stop: Arc<AtomicBool>,
        counter: Arc<AtomicU64>,
        min_interval: Duration,
    ) {
        let mut last_send = Instant::now() - min_interval;
        let mut last_activity = Instant::now();

        while !stop.load(Ordering::Relaxed) {
            let work = {
                let Ok(mut st) = state.0.lock() else { break };
                let clear = st.pending_clear_all;
                st.pending_clear_all = false;
                let br = st.pending_brightness.take();
                let mut tile = None;
                while let Some(pos) = st.pending_order.pop_front() {
                    if let Some((digest, jpeg)) = st.pending_tiles.remove(&pos) {
                        if st.sent_hashes.get(&pos) != Some(&digest) {
                            tile = Some((pos, digest, jpeg));
                            break;
                        }
                    }
                }
                if !clear && br.is_none() && tile.is_none() {
                    let (_guard, _) = state.1.wait_timeout(st, Duration::from_millis(120)).unwrap();
                    None
                } else {
                    Some((clear, br, tile))
                }
            };

            let Some((clear, br, tile)) = work else {
                if last_activity.elapsed() >= Duration::from_secs(5) {
                    if let Ok(p) = proto.lock() {
                        if p.keepalive().is_ok() {
                            last_activity = Instant::now();
                        }
                    }
                }
                continue;
            };

            if clear {
                if let Ok(p) = proto.lock() {
                    let _ = p.clear_all();
                    last_activity = Instant::now();
                }
            }
            if let Some(b) = br {
                if let Ok(p) = proto.lock() {
                    let _ = p.set_brightness(b);
                    last_activity = Instant::now();
                }
            }
            if let Some((pos, digest, jpeg)) = tile {
                let elapsed = last_send.elapsed();
                if elapsed < min_interval {
                    thread::sleep(min_interval - elapsed);
                }
                let ok = if let Ok(p) = proto.lock() {
                    p.set_button_jpeg(pos, &jpeg).is_ok()
                } else {
                    false
                };
                if ok {
                    last_send = Instant::now();
                    last_activity = last_send;
                    counter.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut st) = state.0.lock() {
                        st.sent_hashes.insert(pos, digest);
                    }
                }
            }
        }

        if let Ok(mut p) = proto.lock() {
            p.close(true);
        }
    }

    fn reader_loop<F>(proto: Arc<Mutex<Akp153eProto>>, stop: Arc<AtomicBool>, on_button: F)
    where
        F: Fn(u8, String, InputAction) + Send + Sync + 'static,
    {
        while !stop.load(Ordering::Relaxed) {
            let ev = {
                let Ok(mut p) = proto.lock() else { break };
                p.read_event(60)
            };
            match ev {
                Ok(Some((pos, action))) => {
                    if let Some(grid_key) = index_to_grid(pos) {
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            on_button(pos, grid_key, action);
                        }));
                    }
                }
                Ok(None) => {
                    // Breve pausa fora do mutex para não bloquear writer
                    thread::sleep(Duration::from_millis(15));
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(250));
                }
            }
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.state.1.notify_all();
        if let Some(h) = self.reader_handle.take() {
            let _ = h.join();
        }
        if let Some(h) = self.writer_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for DeviceManager {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_mapping_matches_akp153e_photo() {
        assert_eq!(grid_to_index("4,0"), Some(1));
        assert_eq!(grid_to_index("0,0"), Some(13));
        assert_eq!(grid_to_index("0,2"), Some(15));
        assert_eq!(grid_to_index("5,0"), Some(16));
        assert_eq!(grid_to_index("5,2"), Some(18));
        assert_eq!(index_to_grid(1).as_deref(), Some("4,0"));
        assert_eq!(index_to_grid(13).as_deref(), Some("0,0"));
        assert_eq!(index_to_grid(18).as_deref(), Some("5,2"));
    }
}
