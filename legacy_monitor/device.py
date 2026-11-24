"""Camada de gerenciamento do AJAZZ AKP153E com fila limitada e cache de hash."""
from __future__ import annotations

import hashlib
import logging
import subprocess
import threading
import time
from typing import Callable, Optional

from PIL import Image

from .device_proto import AKP153EDevice, InputAction, sanitize_image

_logger = logging.getLogger(__name__)

OFFICIAL_PROCESSES = ("Stream Dock AJAZZ.exe", "QtWebEngineProcess.exe")

# Mapa físico verificado do AKP153E (6x3: col,lin -> índice 1..18):
# Colunas 0..4 da esquerda para a direita: 13..15, 10..12, 07..09, 04..06, 01..03
# Coluna 5 (faixa lateral direita): 16, 17, 18
DEFAULT_GRID_TO_INDEX: dict[str, int] = {
    **{f"{col},{row}": (4 - col) * 3 + row + 1 for col in range(5) for row in range(3)},
    **{f"5,{row}": 16 + row for row in range(3)},
}
DEFAULT_INDEX_TO_GRID: dict[int, str] = {v: k for k, v in DEFAULT_GRID_TO_INDEX.items()}


def find_official_app_processes() -> list[str]:
    """Retorna os nomes de processos do app oficial que estejam em execução."""
    try:
        out = subprocess.check_output(
            ["tasklist", "/FO", "CSV", "/NH"],
            text=True,
            creationflags=0x08000000,
        )
    except Exception:
        return []
    return sorted({name for name in OFFICIAL_PROCESSES if f'"{name}"' in out})


class DeviceManager:
    """Controla o AKP153E com envio limitado (sem rajadas) e leitura assíncrona."""

    def __init__(
        self,
        proto: Optional[AKP153EDevice] = None,
        max_tiles_per_s: float = 15.0,
        keepalive_s: float = 5.0,
        tile_size: int = 96,
        grid_to_index: Optional[dict[str, int]] = None,
        check_official_app: bool = True,
    ) -> None:
        if max_tiles_per_s <= 0:
            raise ValueError("max_tiles_per_s deve ser > 0")
        self._proto = proto or AKP153EDevice()
        self._min_send_interval = 1.0 / max_tiles_per_s
        self._keepalive_s = keepalive_s
        self._tile_size = tile_size
        self._grid_to_index = dict(grid_to_index or DEFAULT_GRID_TO_INDEX)
        self._index_to_grid = {v: k for k, v in self._grid_to_index.items()}
        self._check_official_app = check_official_app

        self._lock = threading.Lock()
        self._pending_tiles: dict[int, tuple[bytes, bytes]] = {}  # pos -> (digest, jpeg)
        self._pending_order: list[int] = []
        self._sent_hashes: dict[int, bytes] = {}
        self._pending_brightness: Optional[int] = None
        self._pending_clear_all = False

        self._wake_event = threading.Event()
        self._stop_event = threading.Event()
        self._writer_thread: Optional[threading.Thread] = None
        self._reader_thread: Optional[threading.Thread] = None

        self._on_button: Optional[Callable[[int, Optional[str], InputAction], None]] = None

    def set_button_callback(
        self, callback: Optional[Callable[[int, Optional[str], InputAction], None]]
    ) -> None:
        self._on_button = callback

    def start(self, brightness: Optional[int] = None) -> None:
        if self._check_official_app:
            running = find_official_app_processes()
            if running:
                raise RuntimeError(
                    f"App oficial em execução ({', '.join(running)}). Feche-o antes de abrir o HID."
                )
        self._stop_event.clear()
        self._proto.open()
        self._proto.clear_all()
        if brightness is not None:
            self._proto.set_brightness(brightness)

        self._writer_thread = threading.Thread(target=self._writer_loop, daemon=True, name="akp-writer")
        self._reader_thread = threading.Thread(target=self._reader_loop, daemon=True, name="akp-reader")
        self._writer_thread.start()
        self._reader_thread.start()

    def stop(self, clear: bool = True) -> None:
        self._stop_event.set()
        self._wake_event.set()
        if self._reader_thread and self._reader_thread.is_alive():
            self._reader_thread.join(timeout=2.0)
        if self._writer_thread and self._writer_thread.is_alive():
            self._writer_thread.join(timeout=2.0)
        self._proto.close(clear=clear)

    def invalidate_cache(self) -> None:
        with self._lock:
            self._sent_hashes.clear()

    def set_brightness(self, brightness: int) -> None:
        if not 0 <= brightness <= 100:
            raise ValueError("Brilho deve estar entre 0 e 100")
        with self._lock:
            self._pending_brightness = brightness
        self._wake_event.set()

    def clear_all(self) -> None:
        with self._lock:
            self._pending_tiles.clear()
            self._pending_order.clear()
            self._sent_hashes.clear()
            self._pending_clear_all = True
        self._wake_event.set()

    def queue_image(self, position: int, img: Image.Image) -> bool:
        """Sanitiza e enfileira uma imagem Pillow se diferente da última enviada."""
        jpeg = sanitize_image(img, size=self._tile_size)
        return self.queue_jpeg(position, jpeg)

    def queue_grid_image(self, grid_key: str, img: Image.Image) -> bool:
        pos = self._grid_to_index.get(grid_key)
        if pos is None:
            raise KeyError(f"Posição de grade inválida: {grid_key}")
        return self.queue_image(pos, img)

    def queue_jpeg(self, position: int, jpeg_bytes: bytes) -> bool:
        digest = hashlib.blake2b(jpeg_bytes, digest_size=16).digest()
        with self._lock:
            if self._sent_hashes.get(position) == digest and position not in self._pending_tiles:
                return False
            if position not in self._pending_tiles:
                self._pending_order.append(position)
            self._pending_tiles[position] = (digest, jpeg_bytes)
        self._wake_event.set()
        return True

    def _pop_next_work(self):
        with self._lock:
            clear_all = self._pending_clear_all
            self._pending_clear_all = False

            brightness = self._pending_brightness
            self._pending_brightness = None

            tile = None
            if self._pending_order:
                pos = self._pending_order.pop(0)
                digest, jpeg = self._pending_tiles.pop(pos)
                if self._sent_hashes.get(pos) != digest:
                    tile = (pos, digest, jpeg)
            return clear_all, brightness, tile

    def _writer_loop(self) -> None:
        last_send = 0.0
        last_activity = time.monotonic()

        while not self._stop_event.is_set():
            clear_all, brightness, tile = self._pop_next_work()
            if not clear_all and brightness is None and tile is None:
                now = time.monotonic()
                if now - last_activity >= self._keepalive_s:
                    try:
                        self._proto.keepalive()
                        last_activity = now
                    except Exception as exc:
                        _logger.error("Erro no keepalive HID: %s", exc)
                self._wake_event.wait(timeout=0.1)
                self._wake_event.clear()
                continue

            try:
                if clear_all:
                    self._proto.clear_all()
                    last_activity = time.monotonic()
                if brightness is not None:
                    self._proto.set_brightness(brightness)
                    last_activity = time.monotonic()
                if tile is not None:
                    pos, digest, jpeg = tile
                    now = time.monotonic()
                    wait_s = self._min_send_interval - (now - last_send)
                    if wait_s > 0:
                        time.sleep(wait_s)
                    self._proto.set_button_jpeg(pos, jpeg)
                    last_send = time.monotonic()
                    last_activity = last_send
                    with self._lock:
                        self._sent_hashes[pos] = digest
            except Exception as exc:
                _logger.error("Erro ao escrever no HID: %s", exc)
                time.sleep(0.25)

    def _reader_loop(self) -> None:
        while not self._stop_event.is_set():
            try:
                ev = self._proto.read_event(timeout_ms=100)
            except Exception as exc:
                if not self._stop_event.is_set():
                    _logger.error("Erro ao ler HID: %s", exc)
                time.sleep(0.25)
                continue

            if ev is None:
                continue
            pos, action = ev
            grid_key = self._index_to_grid.get(pos)
            cb = self._on_button
            if cb is not None:
                try:
                    cb(pos, grid_key, action)
                except Exception as exc:
                    _logger.exception("Erro no callback de botão (pos=%s): %s", pos, exc)
