"""Testes unitários para sanitização de imagens e parsing HID."""
import io
import os
import random

import pytest
from PIL import Image

from ajazz_monitor.device_proto import (
    BUTTON_IMAGE_MAX_SIZE,
    INPUT_REQUEST_PREFIX,
    InputAction,
    parse_input_report,
    sanitize_image,
)


def test_sanitize_removes_exif_and_icc_and_fits_limits():
    img = Image.new("RGBA", (200, 200), (18, 20, 45, 255))
    img.info["icc_profile"] = b"fake-icc-profile-data"
    exif = Image.Exif()
    exif[0x010F] = "FakeCamera"
    buf = io.BytesIO()
    img.save(buf, format="PNG", exif=exif.tobytes())
    loaded = Image.open(io.BytesIO(buf.getvalue()))

    jpeg = sanitize_image(loaded, size=96)
    assert len(jpeg) <= BUTTON_IMAGE_MAX_SIZE

    out = Image.open(io.BytesIO(jpeg))
    assert out.format == "JPEG"
    assert out.mode == "RGB"
    assert out.size == (96, 96)
    assert "exif" not in out.info
    assert "icc_profile" not in out.info


def test_sanitize_high_entropy_noise_fits_under_10kb():
    rng = random.Random(42)
    noise = bytes(rng.getrandbits(8) for _ in range(96 * 96 * 3))
    img = Image.frombytes("RGB", (96, 96), noise)

    jpeg = sanitize_image(img, size=96)
    assert len(jpeg) <= BUTTON_IMAGE_MAX_SIZE
    out = Image.open(io.BytesIO(jpeg))
    assert out.size == (96, 96)


def test_sanitize_rejects_invalid_sizes():
    img = Image.new("RGB", (64, 64))
    with pytest.raises(ValueError):
        sanitize_image(img, size=100)
    with pytest.raises(ValueError):
        sanitize_image(img, size=112)
    with pytest.raises(ValueError):
        sanitize_image(img, size=0)


def test_parse_input_report():
    press_pkt = INPUT_REQUEST_PREFIX + bytes([5, 0x01])
    release_pkt = INPUT_REQUEST_PREFIX + bytes([5, 0x00])
    # Protocolo v2 (press 0x01 seguido de release 0x00)
    seen: set[int] = set()
    assert parse_input_report(press_pkt, pressed_v2=seen) == (5, InputAction.PRESS)
    assert parse_input_report(release_pkt, pressed_v2=seen) == (5, InputAction.RELEASE)
    # Protocolo v1 (AKP153E press-only envia 0x00 direto)
    assert parse_input_report(release_pkt, v1_press_only=True, pressed_v2=set()) == (5, InputAction.PRESS)
    assert parse_input_report(b"INVALID") is None
