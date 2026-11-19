"""Protocolo HID vendor-defined e sanitização de imagens para o AJAZZ AKP153E.

Baseado no protocolo da pyajazz (MIT), mantendo apenas os comandos seguros:
INIT, KEEPALIVE, FLUSH_BUFFER, BACKLIGHT_BRIGHTNESS, CLEAR_BUTTON, CLEAR_ALL,
IMG_ANNOUNCE e leitura de botões. Sem comandos de logo/boot, sleep ou reset.
"""
from __future__ import annotations

import enum
import io
import logging
from typing import Optional

import hid
from PIL import Image

_logger = logging.getLogger(__name__)

VENDOR_ID = 0x0300
PRODUCT_ID = 0x1010

IMAGE_CHUNK_SIZE = 512
REPORT_PAYLOAD_SIZE = 512
BUTTON_IMAGE_MAX_SIZE = 1024 * 10  # 10 KiB
DEFAULT_TILE_SIZE = 96

INPUT_REQUEST_PREFIX = bytes([0x41, 0x43, 0x4B, 0x00, 0x00, 0x4F, 0x4B, 0x00, 0x00])
OUTPUT_REQUEST_PREFIX = bytes([0x43, 0x52, 0x54, 0x00, 0x00])

INPUT_ID_OFFSET = 9
INPUT_ACTION_OFFSET = 10
BACKLIGHT_BRIGHTNESS_OFFSET = 10
CLEAR_BUTTON_POS_OFFSET = 11
IMG_SIZE_1_OFFSET = 10
IMG_SIZE_0_OFFSET = 11
IMG_POS_OFFSET = 12


def _cmd(suffix: list[int]) -> bytes:
    return OUTPUT_REQUEST_PREFIX + bytes(suffix)


class SafeCommand(enum.Enum):
    INIT = _cmd([0x44, 0x49, 0x53])
    KEEPALIVE = _cmd([0x43, 0x4F, 0x4E, 0x4E, 0x45, 0x43, 0x54])
    FLUSH_BUFFER = _cmd([0x53, 0x54, 0x50])
    BACKLIGHT_BRIGHTNESS = _cmd([0x4C, 0x49, 0x47, 0x00, 0x00, 0x32])
    CLEAR_BUTTON = _cmd([0x43, 0x4C, 0x45, 0x00, 0x00, 0x00, 0x00])
    CLEAR_ALL = _cmd([0x43, 0x4C, 0x45, 0x00, 0x00, 0x00, 0xFF])
    IMG_ANNOUNCE = _cmd([0x42, 0x41, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00])


class InputAction(enum.Enum):
    PRESS = "press"
    RELEASE = "release"


_ACTION_MAP = {
    0x00: InputAction.RELEASE,
    0x01: InputAction.PRESS,
}


def find_vendor_hid_path(vid: int = VENDOR_ID, pid: int = PRODUCT_ID) -> Optional[bytes]:
    """Localiza o caminho HID vendor-defined (usage_page >= 0xFF00) do aparelho."""
    for info in hid.enumerate(vid, pid):
        if info.get("usage_page", 0) >= 0xFF00:
            return info["path"]
    return None


def sanitize_image(
    img: Image.Image,
    size: int = DEFAULT_TILE_SIZE,
    max_bytes: int = BUTTON_IMAGE_MAX_SIZE,
    rotate_ccw: int = 90,
) -> bytes:
    """Converte uma imagem Pillow para JPEG seguro para o AKP153E.

    Regras:
    - RGB (sem canal alfa)
    - Dimensões múltiplas de 16 e <= 96 px
    - Rotacionada anti-horário (padrão 90°)
    - Sem metadados EXIF/ICC
    - Tamanho <= 10 KiB (reduz quality até caber)
    """
    if size <= 0 or size > 96 or size % 16 != 0:
        raise ValueError(f"size deve ser múltiplo de 16 entre 16 e 96, recebido: {size}")

    rgb = img.convert("RGB")
    if rgb.size != (size, size):
        rgb = rgb.resize((size, size), Image.Resampling.LANCZOS)
    if rotate_ccw:
        rgb = rgb.rotate(rotate_ccw, expand=False)

    # Cria imagem limpa sem info/exif/icc_profile
    clean = Image.new("RGB", rgb.size)
    clean.paste(rgb)

    for quality in (90, 82, 74, 65, 55, 45, 35, 25, 15, 10):
        buf = io.BytesIO()
        clean.save(buf, format="JPEG", quality=quality, optimize=True)
        data = buf.getvalue()
        if len(data) <= max_bytes:
            return data

    raise ValueError(f"Não foi possível comprimir a imagem para <= {max_bytes} bytes")


def parse_input_report(
    data: bytes,
    v1_press_only: bool = True,
    pressed_v2: Optional[set[int]] = None,
) -> Optional[tuple[int, InputAction]]:
    """Extrai (position, InputAction) de um relatório HID, ou None se não reconhecido.

    No protocolo v1 (AKP153E 0300:1010), o firmware envia apenas um pacote por clique
    com byte de ação 0x00 (sem distinguir press/release). Se já vimos 0x01 para essa
    posição (protocolo v2), 0x00 é tratado como RELEASE; caso contrário, como PRESS.
    """
    if not data.startswith(INPUT_REQUEST_PREFIX):
        return None
    if len(data) <= max(INPUT_ID_OFFSET, INPUT_ACTION_OFFSET):
        return None
    pos = data[INPUT_ID_OFFSET]
    raw_act = data[INPUT_ACTION_OFFSET]
    if raw_act == 0x01:
        if pressed_v2 is not None:
            pressed_v2.add(pos)
        return pos, InputAction.PRESS
    if raw_act == 0x00:
        if pressed_v2 is not None and pos in pressed_v2:
            pressed_v2.discard(pos)
            return pos, InputAction.RELEASE
        if v1_press_only:
            return pos, InputAction.PRESS
        return pos, InputAction.RELEASE
    return None


class AKP153EDevice:
    """Wrapper minimalista e seguro sobre hid.device para o AKP153E."""

    def __init__(self, vid: int = VENDOR_ID, pid: int = PRODUCT_ID, append_zero_byte: bool = True) -> None:
        self.vid = vid
        self.pid = pid
        self.append_zero_byte = append_zero_byte
        self._dev = hid.device()
        self._connected = False
        self._pressed_v2: set[int] = set()

    @property
    def connected(self) -> bool:
        return self._connected

    def open(self) -> None:
        path = find_vendor_hid_path(self.vid, self.pid)
        if path is None:
            raise OSError(f"Interface HID vendor-defined não encontrada para {self.vid:04x}:{self.pid:04x}")
        self._dev.open_path(path)
        self._dev.set_nonblocking(False)
        self._connected = True
        self._write_raw(SafeCommand.INIT.value)

    def close(self, clear: bool = True) -> None:
        if not self._connected:
            return
        try:
            if clear:
                self.clear_all()
        finally:
            self._dev.close()
            self._connected = False

    def _write_raw(self, payload: bytes) -> None:
        if not self._connected:
            raise RuntimeError("Dispositivo não está aberto")
        padded = payload.ljust(REPORT_PAYLOAD_SIZE, b"\x00")
        packet = (b"\x00" + padded) if self.append_zero_byte else padded
        self._dev.write(packet)

    def keepalive(self) -> None:
        self._write_raw(SafeCommand.KEEPALIVE.value)

    def set_brightness(self, brightness: int) -> None:
        if not 0 <= brightness <= 100:
            raise ValueError("Brilho deve estar entre 0 e 100")
        pkt = bytearray(SafeCommand.BACKLIGHT_BRIGHTNESS.value)
        pkt[BACKLIGHT_BRIGHTNESS_OFFSET] = brightness
        self._write_raw(bytes(pkt))

    def clear_all(self) -> None:
        self._write_raw(SafeCommand.CLEAR_ALL.value)
        self._write_raw(SafeCommand.FLUSH_BUFFER.value)

    def clear_button(self, position: int) -> None:
        pkt = bytearray(SafeCommand.CLEAR_BUTTON.value)
        pkt[CLEAR_BUTTON_POS_OFFSET] = position
        self._write_raw(bytes(pkt))
        self._write_raw(SafeCommand.FLUSH_BUFFER.value)

    def set_button_jpeg(self, position: int, jpeg_bytes: bytes) -> None:
        size = len(jpeg_bytes)
        if size == 0 or size > BUTTON_IMAGE_MAX_SIZE:
            raise ValueError(f"JPEG inválido ou maior que {BUTTON_IMAGE_MAX_SIZE} bytes: {size}")
        pkt = bytearray(SafeCommand.IMG_ANNOUNCE.value)
        pkt[IMG_SIZE_1_OFFSET] = size // 256
        pkt[IMG_SIZE_0_OFFSET] = size % 256
        pkt[IMG_POS_OFFSET] = position
        self._write_raw(bytes(pkt))
        for start in range(0, size, IMAGE_CHUNK_SIZE):
            self._write_raw(jpeg_bytes[start : start + IMAGE_CHUNK_SIZE])
        self._write_raw(SafeCommand.FLUSH_BUFFER.value)

    def read_event(self, timeout_ms: int = 100) -> Optional[tuple[int, InputAction]]:
        """Lê um relatório HID com timeout >= 50 ms (sem busy-loop)."""
        if timeout_ms < 50:
            timeout_ms = 50
        raw = self._dev.read(512, timeout_ms=timeout_ms)
        if not raw:
            return None
        b = bytes(raw)
        _logger.info("HID input raw: %s", b[:16].hex())
        return parse_input_report(b, v1_press_only=True, pressed_v2=self._pressed_v2)
