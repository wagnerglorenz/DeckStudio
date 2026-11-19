"""Testes unitários para DeviceManager (deduplicação, rate limiting e eventos)."""
import time

from PIL import Image

from ajazz_monitor.device import DeviceManager
from ajazz_monitor.device_proto import InputAction


class FakeProto:
    def __init__(self) -> None:
        self.opened = False
        self.closed = False
        self.sent: list[tuple[int, bytes, float]] = []
        self.events: list[tuple[int, InputAction]] = []
        self.brightness_calls: list[int] = []

    def open(self) -> None:
        self.opened = True

    def close(self, clear: bool = True) -> None:
        self.closed = True

    def clear_all(self) -> None:
        pass

    def keepalive(self) -> None:
        pass

    def set_brightness(self, brightness: int) -> None:
        self.brightness_calls.append(brightness)

    def set_button_jpeg(self, position: int, jpeg_bytes: bytes) -> None:
        self.sent.append((position, jpeg_bytes, time.monotonic()))

    def read_event(self, timeout_ms: int = 100):
        if self.events:
            return self.events.pop(0)
        time.sleep(0.02)
        return None


def test_deduplicates_identical_tiles():
    proto = FakeProto()
    mgr = DeviceManager(proto=proto, max_tiles_per_s=50, check_official_app=False)
    mgr.start()
    try:
        img = Image.new("RGB", (96, 96), (10, 20, 30))
        assert mgr.queue_image(1, img) is True
        time.sleep(0.08)
        assert mgr.queue_image(1, img) is False
        assert len(proto.sent) == 1
    finally:
        mgr.stop()


def test_rate_limits_bursts():
    proto = FakeProto()
    mgr = DeviceManager(proto=proto, max_tiles_per_s=20, check_official_app=False)
    mgr.start()
    try:
        for pos in range(1, 5):
            img = Image.new("RGB", (96, 96), (pos * 30, 0, 0))
            mgr.queue_image(pos, img)
        time.sleep(0.30)
        assert len(proto.sent) == 4
        elapsed = proto.sent[-1][2] - proto.sent[0][2]
        assert elapsed >= 0.13
    finally:
        mgr.stop()


def test_button_callback_receives_grid_key():
    proto = FakeProto()
    proto.events.append((1, InputAction.PRESS))
    received = []

    mgr = DeviceManager(proto=proto, check_official_app=False)
    mgr.set_button_callback(lambda pos, grid, act: received.append((pos, grid, act)))
    mgr.start()
    try:
        time.sleep(0.1)
    finally:
        mgr.stop()

    assert received == [(1, "4,0", InputAction.PRESS)]
