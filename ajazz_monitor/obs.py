"""Cliente OBS WebSocket v5 com reconexão automática em segundo plano."""
from __future__ import annotations

import logging
import os
import threading
import time
from typing import Optional

import obsws_python as obsws

_logger = logging.getLogger(__name__)


class ObsClient:
    """Gerencia conexão com OBS Studio sem travar o loop principal."""

    def __init__(
        self,
        host: str = "127.0.0.1",
        port: int = 4455,
        password_env: str = "OBS_WS_PASSWORD",
        reconnect_interval_s: float = 10.0,
    ) -> None:
        self._host = host
        self._port = port
        self._password_env = password_env
        self._reconnect_s = reconnect_interval_s

        self._lock = threading.Lock()
        self._req: Optional[obsws.ReqClient] = None
        self._evt: Optional[obsws.EventClient] = None
        self._connected = False

        self.recording = False
        self.recording_paused = False
        self.streaming = False
        self.current_scene = ""
        self.muted_inputs: dict[str, bool] = {}

        self._stop_event = threading.Event()
        self._thread: Optional[threading.Thread] = None

    @property
    def connected(self) -> bool:
        with self._lock:
            return self._connected

    def start(self) -> None:
        self._stop_event.clear()
        self._thread = threading.Thread(target=self._loop, daemon=True, name="obs-reconnect")
        self._thread.start()

    def stop(self) -> None:
        self._stop_event.set()
        self._disconnect()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=2.0)

    def _disconnect(self) -> None:
        with self._lock:
            self._connected = False
            req, evt = self._req, self._evt
            self._req = None
            self._evt = None
        for client in (evt, req):
            if client is not None:
                try:
                    client.disconnect()
                except Exception:
                    pass

    def _connect_once(self) -> None:
        password = os.environ.get(self._password_env, "")
        req = obsws.ReqClient(host=self._host, port=self._port, password=password, timeout=3)
        evt = obsws.EventClient(host=self._host, port=self._port, password=password)

        evt.callback.register(self._on_record_state_changed)
        evt.callback.register(self._on_stream_state_changed)
        evt.callback.register(self._on_input_mute_state_changed)
        evt.callback.register(self._on_current_program_scene_changed)

        rec = req.get_record_status()
        stm = req.get_stream_status()
        scn = req.get_current_program_scene()

        muted_map: dict[str, bool] = {}
        try:
            for item in req.get_input_list().inputs:
                iname = item.get("inputName")
                if iname:
                    try:
                        m = req.get_input_mute(iname)
                        muted_map[str(iname)] = bool(getattr(m, "input_muted", False))
                    except Exception:
                        pass
        except Exception:
            pass

        with self._lock:
            self._req = req
            self._evt = evt
            self._connected = True
            self.recording = bool(getattr(rec, "output_active", False))
            self.recording_paused = bool(getattr(rec, "output_paused", False))
            self.streaming = bool(getattr(stm, "output_active", False))
            self.current_scene = str(getattr(scn, "current_program_scene_name", ""))
            self.muted_inputs.update(muted_map)
        _logger.info("Conectado ao OBS WebSocket em %s:%s", self._host, self._port)

    def _loop(self) -> None:
        while not self._stop_event.is_set():
            if not self.connected:
                try:
                    self._connect_once()
                except Exception:
                    self._disconnect()
                    self._stop_event.wait(timeout=self._reconnect_s)
                    continue
            else:
                try:
                    with self._lock:
                        req = self._req
                    if req is not None:
                        req.get_version()
                except Exception:
                    _logger.info("OBS desconectado; tentando reconectar em %.0fs", self._reconnect_s)
                    self._disconnect()
                    self._stop_event.wait(timeout=self._reconnect_s)
                    continue
            self._stop_event.wait(timeout=3.0)

    def _on_record_state_changed(self, data) -> None:
        state = str(getattr(data, "output_state", ""))
        active = bool(getattr(data, "output_active", False))
        with self._lock:
            if state == "OBS_WEBSOCKET_OUTPUT_PAUSED":
                self.recording = True
                self.recording_paused = True
            elif state in ("OBS_WEBSOCKET_OUTPUT_RESUMED", "OBS_WEBSOCKET_OUTPUT_STARTED"):
                self.recording = True
                self.recording_paused = False
            elif state in ("OBS_WEBSOCKET_OUTPUT_STOPPED", "OBS_WEBSOCKET_OUTPUT_STOPPING"):
                self.recording = False
                self.recording_paused = False
            else:
                self.recording = active
                if not active:
                    self.recording_paused = False

    def _on_stream_state_changed(self, data) -> None:
        with self._lock:
            self.streaming = bool(getattr(data, "output_active", False))

    def _on_input_mute_state_changed(self, data) -> None:
        name = getattr(data, "input_name", None)
        if name is not None:
            with self._lock:
                self.muted_inputs[str(name)] = bool(getattr(data, "input_muted", False))

    def _on_current_program_scene_changed(self, data) -> None:
        name = getattr(data, "scene_name", None)
        if name is not None:
            with self._lock:
                self.current_scene = str(name)

    def _call(self, fn) -> bool:
        with self._lock:
            req = self._req
        if req is None:
            return False
        try:
            fn(req)
            return True
        except Exception as exc:
            _logger.warning("Falha na ação OBS: %s", exc)
            self._disconnect()
            return False

    def start_record(self) -> bool:
        return self._call(lambda r: r.start_record())

    def stop_record(self) -> bool:
        return self._call(lambda r: r.stop_record())

    def toggle_record(self) -> bool:
        return self._call(lambda r: r.toggle_record())

    def toggle_record_pause(self) -> bool:
        return self._call(lambda r: r.toggle_record_pause())

    def toggle_stream(self) -> bool:
        return self._call(lambda r: r.toggle_stream())

    def set_scene(self, scene_name: str) -> bool:
        return self._call(lambda r: r.set_current_program_scene(scene_name))

    def toggle_mute(self, input_name: str) -> bool:
        return self._call(lambda r: r.toggle_input_mute(input_name))

    def save_replay(self) -> bool:
        return self._call(lambda r: r.save_replay_buffer())
