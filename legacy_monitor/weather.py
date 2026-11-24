"""Cliente leve de clima/previsão via Open-Meteo (atualiza em segundo plano a cada 15 min)."""
from __future__ import annotations

from dataclasses import dataclass
import json
import logging
import threading
from typing import Optional
import urllib.request

_logger = logging.getLogger(__name__)


@dataclass(frozen=True)
class WeatherData:
    temp_c: float
    tmin: float
    tmax: float
    weather_code: int
    city: str


class WeatherClient:
    def __init__(
        self,
        city: str = "Santa Maria",
        lat: float = -29.6842,
        lon: float = -53.8069,
        refresh_minutes: float = 15.0,
    ) -> None:
        self._city = city
        self._lat = lat
        self._lon = lon
        self._interval_s = max(60.0, refresh_minutes * 60.0)

        self._lock = threading.Lock()
        self._data: Optional[WeatherData] = None
        self._stop_event = threading.Event()
        self._thread: Optional[threading.Thread] = None

    def start(self) -> None:
        self._stop_event.clear()
        self._thread = threading.Thread(target=self._loop, daemon=True, name="weather-poll")
        self._thread.start()

    def stop(self) -> None:
        self._stop_event.set()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=1.5)

    def get_snapshot(self) -> Optional[WeatherData]:
        with self._lock:
            return self._data

    def _fetch_once(self) -> None:
        url = (
            f"https://api.open-meteo.com/v1/forecast"
            f"?latitude={self._lat}&longitude={self._lon}"
            f"&current=temperature_2m,weather_code"
            f"&daily=temperature_2m_max,temperature_2m_min"
            f"&timezone=auto&forecast_days=1"
        )
        with urllib.request.urlopen(url, timeout=6) as resp:
            payload = json.loads(resp.read().decode("utf-8"))
        cur = payload["current"]
        daily = payload["daily"]
        data = WeatherData(
            temp_c=float(cur["temperature_2m"]),
            tmin=float(daily["temperature_2m_min"][0]),
            tmax=float(daily["temperature_2m_max"][0]),
            weather_code=int(cur.get("weather_code", 0)),
            city=self._city,
        )
        with self._lock:
            self._data = data
        _logger.info("Clima atualizado (%s): %.1f°C (%.0f°/%.0f°)", self._city, data.temp_c, data.tmin, data.tmax)

    def _loop(self) -> None:
        while not self._stop_event.is_set():
            try:
                self._fetch_once()
            except Exception as exc:
                _logger.warning("Falha ao consultar Open-Meteo: %s", exc)
            self._stop_event.wait(timeout=self._interval_s)
