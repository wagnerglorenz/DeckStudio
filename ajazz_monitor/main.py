"""Ponto de entrada principal do ajazz-monitor (loop, bandeja e sinais)."""
from __future__ import annotations

import argparse
from datetime import datetime
import logging
from logging.handlers import RotatingFileHandler
from pathlib import Path
import threading
import time
from typing import Any

from PIL import Image, ImageDraw
import pystray

from .actions import ActionDispatcher
from .config import AppConfig, load_config
from .device import DEFAULT_GRID_TO_INDEX, DeviceManager
from .device_proto import InputAction
from .obs import ObsClient
from .pages import PageManager
from .render import (
    GROUP_ACCENTS,
    render_button_tile,
    render_clock_tile,
    render_compact_health_tile,
    render_cpu_chip_tile,
    render_nav_circle_tile,
    render_net_strip_tile,
    render_obs_logo_tile,
    render_obs_status_tile,
    render_ram_gauge_tile,
    render_sensor_tile,
    render_weather_tile,
)
from .sensors import SensorStore, format_rate_human
from .weather import WeatherClient

_logger = logging.getLogger("ajazz_monitor")


def _tray_icon_image() -> Image.Image:
    img = Image.new("RGB", (64, 64), "#12142D")
    draw = ImageDraw.Draw(img)
    draw.rectangle([6, 6, 57, 57], outline="#F700E7", width=4)
    draw.rectangle([18, 18, 45, 45], fill="#38BDF8")
    return img


class AjazzMonitorApp:
    def __init__(self, config_path: Path, enable_watchdog: bool = False) -> None:
        self._config_path = config_path
        self._cfg: AppConfig = load_config(config_path)
        self._active_theme_name = self._cfg.active_theme
        self._lock = threading.Lock()
        self._paused = False
        self._stop_event = threading.Event()
        self._force_redraw = threading.Event()

        self._sensors = SensorStore(history_len=30, enable_watchdog=enable_watchdog)
        self._weather = WeatherClient(
            city=self._cfg.weather.city,
            lat=self._cfg.weather.lat,
            lon=self._cfg.weather.lon,
            refresh_minutes=self._cfg.weather.refresh_minutes,
        )
        self._obs = ObsClient(
            host=self._cfg.obs.host,
            port=self._cfg.obs.port,
            password_env=self._cfg.obs.password_env,
        )
        self._pages = PageManager(self._cfg.pages)
        self._device = DeviceManager(max_tiles_per_s=self._cfg.device.max_tiles_per_s)
        self._actions = ActionDispatcher(
            pages=self._pages,
            device=self._device,
            obs=self._obs,
            on_page_changed=self._force_redraw.set,
            on_theme_changed=self.set_theme,
        )
        self._actions._brightness = self._cfg.device.brightness
        self._device.set_button_callback(self._on_button)

        self._render_thread: threading.Thread | None = None
        self._tray: pystray.Icon | None = None

    def _on_button(self, pos: int, grid_key: str | None, action: InputAction) -> None:
        if action != InputAction.PRESS or grid_key is None:
            return
        with self._lock:
            item = self._pages.current_items.get(grid_key)
        if not item:
            return
        act = item.get("action")
        if isinstance(act, dict):
            self._actions.dispatch(act)

    def _button_state(self, item: dict[str, Any]) -> tuple[str, str, str]:
        title = str(item.get("title", ""))
        default_sub = str(item.get("subtitle", ""))
        act = item.get("action", {})
        if isinstance(act, dict) and "obs" in act and not self._obs.connected:
            return "disabled", "OFF", title

        state_kind = item.get("state")
        if state_kind == "record_confirm":
            if self._actions.record_confirm_armed:
                if self._obs.recording:
                    return "warning", "CONFIRMA", "PARAR?"
                return "warning", "CONFIRMA", "INICIAR?"
            if self._obs.recording and self._obs.recording_paused:
                return "warning", "PAUSADO", "GRAVANDO"
            if self._obs.recording:
                return "active", "PARAR", "GRAVANDO"
            return "idle", "CLIQUE", title or "Gravar"

        if state_kind == "record_pause":
            if not self._obs.recording:
                return "disabled", "PARADO", title or "Pausar"
            if self._obs.recording_paused:
                return "warning", "RETOMAR", "PAUSADO"
            return "idle", "REC ATIVO", title or "Pausar"

        if state_kind == "record":
            if self._obs.recording and self._obs.recording_paused:
                return "warning", "PAUSADO", "GRAVANDO"
            if self._obs.recording:
                return "active", "PARAR", "GRAVANDO"
            return "idle", "CLIQUE", title or "Gravar"
        if state_kind == "stream":
            return ("active", "LIVE", title) if self._obs.streaming else ("idle", "OFF", title)
        if state_kind == "mute":
            inp = str(act.get("input", "Mic/Aux")) if isinstance(act, dict) else "Mic/Aux"
            muted = self._obs.muted_inputs.get(inp, False)
            return ("active", "MUDO", title) if muted else ("idle", "ATIVO", title)
        if isinstance(act, dict) and act.get("obs") == "scene":
            if self._obs.current_scene == act.get("name"):
                return "active", "ATIVA", title
            return "idle", default_sub, title
        return "idle", default_sub, title

    def _short_scene_name(self) -> str:
        cur = self._obs.current_scene
        if not cur:
            return "---"
        # Busca rótulo curto na página obs se existir
        obs_page = self._cfg.pages.get("obs", {})
        for item in obs_page.values():
            act = item.get("action", {})
            if isinstance(act, dict) and act.get("obs") == "scene" and act.get("name") == cur:
                return str(item.get("title", cur))
        return cur[:11]

    def _current_theme(self):
        return self._cfg.themes.get(self._active_theme_name, self._cfg.theme)

    def set_theme(self, target: str) -> None:
        with self._lock:
            names = list(self._cfg.themes.keys())
            if target == "next":
                idx = names.index(self._active_theme_name) if self._active_theme_name in names else 0
                self._active_theme_name = names[(idx + 1) % len(names)]
            elif target in self._cfg.themes:
                self._active_theme_name = target
            else:
                return
            chosen = self._active_theme_name
        self._device.invalidate_cache()
        self._force_redraw.set()
        _logger.info("Tema alterado para: %s", chosen)

    def _render_once(self) -> None:
        values = self._sensors.refresh()
        with self._lock:
            theme = self._current_theme()
            items = dict(self._pages.current_items)

        for grid_key in DEFAULT_GRID_TO_INDEX:
            item = items.get(grid_key)
            if not item:
                continue
            kind = item.get("type", "button")
            widget = item.get("widget")
            title = str(item.get("title", ""))

            if widget == "nav_prev":
                img = render_nav_circle_tile("prev", theme=theme)
            elif widget == "nav_next":
                img = render_nav_circle_tile("next", theme=theme)
            elif widget == "obs_logo":
                img = render_obs_logo_tile(theme=theme)
            elif widget == "clock":
                now_dt = datetime.now()
                img = render_clock_tile(now_dt.strftime("%H:%M"), now_dt.strftime("%d/%m"), theme=theme)
            elif widget == "obs_status":
                if not self._obs.connected:
                    st_txt = "OFF"
                elif self._obs.recording and self._obs.recording_paused:
                    st_txt = "PAUSADO"
                elif self._obs.recording:
                    st_txt = "REC"
                else:
                    st_txt = "PRONTO"
                img = render_obs_status_tile(st_txt, self._short_scene_name(), theme=theme)
            elif widget == "weather":
                w = self._weather.get_snapshot()
                if w is not None:
                    img = render_weather_tile(w.temp_c, w.tmin, w.tmax, w.city, w.weather_code, theme=theme)
                else:
                    img = render_weather_tile(None, None, None, self._cfg.weather.city, 0, theme=theme)
            elif widget == "cpu_health":
                img = render_compact_health_tile("CPU", values.get("SCPUUTI"), values.get("TCPU"), GROUP_ACCENTS["cpu"], theme=theme)
            elif widget == "gpu_health":
                img = render_compact_health_tile("GPU", values.get("SGPU1UTI"), values.get("TGPU1DIO"), GROUP_ACCENTS["gpu"], theme=theme)
            elif widget == "net_strip":
                dl = format_rate_human(values.get("NET_DL", 0.0))
                ul = format_rate_human(values.get("NET_UL", 0.0))
                img = render_net_strip_tile(dl_text=dl, ul_text=ul, theme=theme)
            elif widget == "cpu_chip":
                tcpu = values.get("TCPU")
                txt = f"{int(round(tcpu))}°C" if tcpu is not None else "--°C"
                img = render_cpu_chip_tile(txt, temp_value=tcpu, theme=theme)
            elif widget == "ram_gauge":
                ram = values.get("SMEMUTI", 0.0)
                img = render_ram_gauge_tile(ram, theme=theme)
            elif kind == "sensor":
                sid = str(item.get("id", ""))
                unit = str(item.get("unit", ""))
                decimals = int(item.get("decimals", 0))
                val_txt, raw_num = self._sensors.format_sensor(sid, values, unit="", decimals=decimals)
                vmin = item.get("min")
                vmax = item.get("max")
                hist = None
                if vmin is not None and vmax is not None:
                    hist_id = "NET_TOTAL" if sid == "NET_BOTH" else sid
                    hist = self._sensors.get_history(hist_id)
                img = render_sensor_tile(
                    title=title,
                    value_text=val_txt,
                    unit=unit,
                    history=hist,
                    vmin=float(vmin) if vmin is not None else None,
                    vmax=float(vmax) if vmax is not None else None,
                    theme=theme,
                    raw_value=raw_num,
                    group=item.get("group"),
                    card_style=item.get("style"),
                    is_temp=bool(item.get("is_temp", False)),
                )
            else:
                st, sub, btn_title = self._button_state(item)
                img = render_button_tile(
                    title=btn_title,
                    subtitle=sub,
                    state=st,
                    theme=theme,
                    icon=item.get("icon"),
                )

            self._device.queue_grid_image(grid_key, img)

    def _render_loop(self) -> None:
        while not self._stop_event.is_set():
            if not self._paused:
                try:
                    self._render_once()
                except Exception as exc:
                    _logger.exception("Erro no loop de renderização: %s", exc)
            interval_s = max(0.2, self._cfg.device.refresh_ms / 1000.0)
            self._force_redraw.wait(timeout=interval_s)
            self._force_redraw.clear()

    def reload_config(self) -> None:
        new_cfg = load_config(self._config_path)
        with self._lock:
            self._cfg = new_cfg
            self._active_theme_name = new_cfg.active_theme
            self._pages = PageManager(new_cfg.pages)
            self._actions._pages = self._pages
        self._device.clear_all()
        self._actions.set_brightness_value(new_cfg.device.brightness)
        self._force_redraw.set()
        _logger.info("Configuração recarregada de %s", self._config_path)

    def toggle_pause(self) -> None:
        self._paused = not self._paused
        _logger.info("Renderização %s", "pausada" if self._paused else "retomada")

    def run(self, no_tray: bool = False) -> None:
        self._sensors.start()
        self._weather.start()
        self._obs.start()
        self._device.start(brightness=self._cfg.device.brightness)

        self._render_thread = threading.Thread(target=self._render_loop, daemon=True, name="render-loop")
        self._render_thread.start()

        if no_tray:
            try:
                while not self._stop_event.is_set():
                    time.sleep(0.5)
            except KeyboardInterrupt:
                pass
            finally:
                self.stop()
            return

        menu = pystray.Menu(
            pystray.MenuItem(lambda _: "Retomar" if self._paused else "Pausar", lambda: self.toggle_pause()),
            pystray.MenuItem("Recarregar config", lambda: self.reload_config()),
            pystray.MenuItem(
                "Tema",
                pystray.Menu(
                    pystray.MenuItem(
                        "LLM Dashboard",
                        lambda: self.set_theme("llm_dash"),
                        checked=lambda _: self._active_theme_name == "llm_dash",
                    ),
                    pystray.MenuItem(
                        "AJAZZ Clássico",
                        lambda: self.set_theme("ajazz"),
                        checked=lambda _: self._active_theme_name == "ajazz",
                    ),
                ),
            ),
            pystray.MenuItem(
                "Brilho",
                pystray.Menu(
                    pystray.MenuItem("20%", lambda: self._actions.set_brightness_value(20)),
                    pystray.MenuItem("40%", lambda: self._actions.set_brightness_value(40)),
                    pystray.MenuItem("60%", lambda: self._actions.set_brightness_value(60)),
                    pystray.MenuItem("80%", lambda: self._actions.set_brightness_value(80)),
                    pystray.MenuItem("100%", lambda: self._actions.set_brightness_value(100)),
                ),
            ),
            pystray.MenuItem("Sair", lambda: self.stop()),
        )
        self._tray = pystray.Icon("ajazz-monitor", _tray_icon_image(), "ajazz-monitor", menu)
        self._tray.run()

    def stop(self) -> None:
        self._stop_event.set()
        self._force_redraw.set()
        if self._tray is not None:
            try:
                self._tray.stop()
            except Exception:
                pass
        if self._render_thread and self._render_thread.is_alive():
            self._render_thread.join(timeout=2.0)
        self._obs.stop()
        self._weather.stop()
        self._sensors.stop()
        self._device.stop(clear=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Monitor e controlador para AJAZZ AKP153E")
    parser.add_argument("--config", default="config.yaml", help="Caminho do config.yaml")
    parser.add_argument("--no-tray", action="store_true", help="Executa sem ícone na bandeja (modo teste)")
    parser.add_argument("--watchdog", action="store_true", help="Ativa vigia do AIDA64")
    args = parser.parse_args()

    handlers: list[logging.Handler] = [
        RotatingFileHandler("ajazz_monitor.log", maxBytes=1_000_000, backupCount=2, encoding="utf-8"),
        logging.StreamHandler(),
    ]
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        handlers=handlers,
    )
    logging.getLogger("obsws_python").setLevel(logging.CRITICAL)
    _logger.info("Iniciando ajazz-monitor (config=%s, watchdog=%s)", args.config, args.watchdog)
    app = AjazzMonitorApp(Path(args.config), enable_watchdog=args.watchdog)
    app.run(no_tray=args.no_tray)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
