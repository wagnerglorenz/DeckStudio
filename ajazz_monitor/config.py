"""Carregamento e validação do config.yaml com suporte a múltiplos temas."""
from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

from .render import BUILTIN_THEMES, Theme


@dataclass(frozen=True)
class DeviceConfig:
    brightness: int = 60
    refresh_ms: int = 1000
    max_tiles_per_s: float = 15.0


@dataclass(frozen=True)
class ObsConfig:
    host: str = "127.0.0.1"
    port: int = 4455
    password_env: str = "OBS_WS_PASSWORD"


@dataclass(frozen=True)
class WeatherConfig:
    city: str = "Santa Maria"
    lat: float = -29.6842
    lon: float = -53.8069
    refresh_minutes: float = 15.0


@dataclass(frozen=True)
class AppConfig:
    device: DeviceConfig = field(default_factory=DeviceConfig)
    active_theme: str = "ajazz"
    themes: dict[str, Theme] = field(default_factory=lambda: dict(BUILTIN_THEMES))
    obs: ObsConfig = field(default_factory=ObsConfig)
    weather: WeatherConfig = field(default_factory=WeatherConfig)
    pages: dict[str, dict[str, dict[str, Any]]] = field(default_factory=dict)

    @property
    def theme(self) -> Theme:
        return self.themes.get(self.active_theme, BUILTIN_THEMES["ajazz"])


def _parse_theme(name: str, data: dict[str, Any]) -> Theme:
    base = BUILTIN_THEMES.get(name, BUILTIN_THEMES["ajazz"])
    return Theme(
        name=name,
        style=str(data.get("style", base.style)),
        background=str(data.get("background", base.background)),
        card_bg=str(data.get("card_bg", base.card_bg)),
        card_border=str(data.get("card_border", base.card_border)),
        chart=str(data.get("chart", base.chart)),
        text=str(data.get("text", base.text)),
        muted=str(data.get("muted", base.muted)),
        active=str(data.get("active", base.active)),
        accent=str(data.get("accent", base.accent)),
        temp_colors=bool(data.get("temp_colors", base.temp_colors)),
    )


def load_config(path: str | Path) -> AppConfig:
    raw = yaml.safe_load(Path(path).read_text(encoding="utf-8")) or {}

    dev_raw = raw.get("device", {})
    device = DeviceConfig(
        brightness=int(dev_raw.get("brightness", 60)),
        refresh_ms=int(dev_raw.get("refresh_ms", 1000)),
        max_tiles_per_s=float(dev_raw.get("max_tiles_per_s", 15.0)),
    )
    if not 0 <= device.brightness <= 100:
        raise ValueError("device.brightness deve estar entre 0 e 100")

    themes: dict[str, Theme] = dict(BUILTIN_THEMES)
    if "theme" in raw and isinstance(raw["theme"], dict):
        themes["ajazz"] = _parse_theme("ajazz", raw["theme"])
    if "themes" in raw and isinstance(raw["themes"], dict):
        for tname, tdata in raw["themes"].items():
            if isinstance(tdata, dict):
                themes[str(tname)] = _parse_theme(str(tname), tdata)

    active_theme = str(raw.get("active_theme", "ajazz"))
    if active_theme not in themes:
        active_theme = "ajazz"

    obs_raw = raw.get("obs", {})
    obs = ObsConfig(
        host=str(obs_raw.get("host", "127.0.0.1")),
        port=int(obs_raw.get("port", 4455)),
        password_env=str(obs_raw.get("password_env", "OBS_WS_PASSWORD")),
    )

    w_raw = raw.get("weather", {})
    weather = WeatherConfig(
        city=str(w_raw.get("city", "Santa Maria")),
        lat=float(w_raw.get("lat", -29.6842)),
        lon=float(w_raw.get("lon", -53.8069)),
        refresh_minutes=float(w_raw.get("refresh_minutes", 15.0)),
    )

    pages = raw.get("pages", {})
    if not isinstance(pages, dict) or not pages:
        raise ValueError("config.yaml precisa definir ao menos uma página em 'pages'")

    return AppConfig(
        device=device,
        active_theme=active_theme,
        themes=themes,
        obs=obs,
        weather=weather,
        pages=pages,
    )
