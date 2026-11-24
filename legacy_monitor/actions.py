"""Despacho de ações de botões (página, brilho, atalho, programa, OBS)."""
from __future__ import annotations

import logging
import subprocess
import time
from typing import Any, Callable, Optional

from pynput.keyboard import Controller as KbController, Key

from .device import DeviceManager
from .obs import ObsClient
from .pages import PageManager

_logger = logging.getLogger(__name__)

_SPECIAL_KEYS = {
    "ctrl": Key.ctrl_l,
    "control": Key.ctrl_l,
    "shift": Key.shift,
    "alt": Key.alt_l,
    "win": Key.cmd,
    "cmd": Key.cmd,
    "enter": Key.enter,
    "esc": Key.esc,
    "space": Key.space,
    "tab": Key.tab,
    **{f"f{i}": getattr(Key, f"f{i}") for i in range(1, 13)},
}


def send_hotkey(keys: list[str] | str) -> None:
    if isinstance(keys, str):
        parts = [p.strip().lower() for p in keys.split("+") if p.strip()]
    else:
        parts = [str(p).strip().lower() for p in keys]
    if not parts:
        return
    resolved = []
    for p in parts:
        if p in _SPECIAL_KEYS:
            resolved.append(_SPECIAL_KEYS[p])
        elif len(p) == 1:
            resolved.append(p)
        else:
            raise ValueError(f"Tecla inválida em hotkey: {p!r}")
    kb = KbController()
    pressed = []
    try:
        for k in resolved:
            kb.press(k)
            pressed.append(k)
    finally:
        for k in reversed(pressed):
            try:
                kb.release(k)
            except Exception:
                pass


class ActionDispatcher:
    def __init__(
        self,
        pages: PageManager,
        device: DeviceManager,
        obs: ObsClient,
        on_page_changed: Optional[Callable[[], None]] = None,
        on_theme_changed: Optional[Callable[[str], None]] = None,
    ) -> None:
        self._pages = pages
        self._device = device
        self._obs = obs
        self._on_page_changed = on_page_changed
        self._on_theme_changed = on_theme_changed
        self._brightness = 60
        self._record_confirm_until = 0.0
        self._confirm_window_s = 5.0

    @property
    def record_confirm_armed(self) -> bool:
        return time.monotonic() < self._record_confirm_until

    def clear_confirm(self) -> None:
        self._record_confirm_until = 0.0

    def set_brightness_value(self, value: int) -> None:
        self._brightness = max(0, min(100, value))
        self._device.set_brightness(self._brightness)

    def dispatch(self, action: dict[str, Any]) -> None:
        if not action:
            return

        if "page" in action:
            self._pages.set_page(str(action["page"]))
            self._device.clear_all()
            if self._on_page_changed:
                self._on_page_changed()
            return

        if "theme" in action:
            if self._on_theme_changed:
                self._on_theme_changed(str(action["theme"]))
            return

        if "brightness" in action:
            val = action["brightness"]
            if isinstance(val, str) and (val.startswith("+") or val.startswith("-")):
                self.set_brightness_value(self._brightness + int(val))
            else:
                self.set_brightness_value(int(val))
            return

        if "hotkey" in action:
            send_hotkey(action["hotkey"])
            return

        if "run" in action:
            import shlex
            cmd = action["run"]
            argv = shlex.split(cmd, posix=False) if isinstance(cmd, str) else [str(x) for x in cmd]
            if argv:
                subprocess.Popen(argv, shell=False)
            return

        if "obs" in action:
            op = str(action["obs"])
            if op == "record_confirm":
                now = time.monotonic()
                if now < self._record_confirm_until:
                    self._record_confirm_until = 0.0
                    if getattr(self._obs, "recording", False):
                        self._obs.stop_record()
                    else:
                        self._obs.start_record()
                else:
                    self._record_confirm_until = now + self._confirm_window_s
                if self._on_page_changed:
                    self._on_page_changed()
            elif op == "toggle_record_pause":
                if getattr(self._obs, "recording", False):
                    self._obs.toggle_record_pause()
                    if self._on_page_changed:
                        self._on_page_changed()
            elif op == "toggle_record":
                self._obs.toggle_record()
                if self._on_page_changed:
                    self._on_page_changed()
            elif op == "toggle_stream":
                self._obs.toggle_stream()
            elif op == "scene":
                self._obs.set_scene(str(action.get("name", "")))
                if self._on_page_changed:
                    self._on_page_changed()
            elif op == "toggle_mute":
                self._obs.toggle_mute(str(action.get("input", "Mic/Aux")))
                if self._on_page_changed:
                    self._on_page_changed()
            elif op == "save_replay":
                self._obs.save_replay()
            else:
                _logger.warning("Ação OBS desconhecida: %s", op)
