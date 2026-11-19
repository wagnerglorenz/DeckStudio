"""Leitura de sensores via memória compartilhada do AIDA64 e contadores de rede do Windows.

Reaproveita a lógica validada de aida64_bridge.py (sem servidor HTTP),
incluindo cache de último valor bom (30 s), vigia de AIDA64 travado (45 s)
e histórico circular (máx. 30 pontos) para sparklines.
"""
from __future__ import annotations

from collections import deque
import ctypes
import ctypes.wintypes as wt
import hashlib
import logging
import re
import subprocess
import threading
import time
from typing import Optional

_logger = logging.getLogger(__name__)

SHM_NAME = "AIDA64_SensorValues"
FILE_MAP_READ = 4

CACHE_S = 30.0
TRAVADO_S = 45.0
ESPERA_REINICIO_S = 90.0
TAREFA_AIDA = "AIDA64 AutoStart"

_VIRTUAL_IF = re.compile(
    rb"Hyper-V|Virtual|Tailscale|Bluetooth|WireGuard|Loopback|VPN|TAP|WAN Miniport|Filter|QoS|Packet Scheduler|LightWeight",
    re.I,
)
_SENSOR_RE = re.compile(r"<(\w+)><id>(.*?)</id><label>(.*?)</label><value>(.*?)</value>")


class _IFROW(ctypes.Structure):
    _fields_ = [
        ("name", ctypes.c_wchar * 256),
        ("idx", wt.DWORD),
        ("tipo", wt.DWORD),
        ("mtu", wt.DWORD),
        ("speed", wt.DWORD),
        ("physlen", wt.DWORD),
        ("phys", ctypes.c_ubyte * 8),
        ("admin", wt.DWORD),
        ("oper", wt.DWORD),
        ("lastchg", wt.DWORD),
        ("in_oct", wt.DWORD),
        ("in_uc", wt.DWORD),
        ("in_nuc", wt.DWORD),
        ("in_dis", wt.DWORD),
        ("in_err", wt.DWORD),
        ("in_unk", wt.DWORD),
        ("out_oct", wt.DWORD),
        ("out_uc", wt.DWORD),
        ("out_nuc", wt.DWORD),
        ("out_dis", wt.DWORD),
        ("out_err", wt.DWORD),
        ("out_q", wt.DWORD),
        ("descrlen", wt.DWORD),
        ("descr", ctypes.c_char * 256),
    ]


class _MBI(ctypes.Structure):
    _fields_ = [
        ("B", ctypes.c_void_p),
        ("AB", ctypes.c_void_p),
        ("AP", wt.DWORD),
        ("RS", ctypes.c_size_t),
        ("S", wt.DWORD),
        ("P", wt.DWORD),
        ("T", wt.DWORD),
    ]


_k32 = ctypes.windll.kernel32
_k32.OpenFileMappingW.restype = wt.HANDLE
_k32.MapViewOfFile.restype = ctypes.c_void_p
_k32.UnmapViewOfFile.argtypes = [ctypes.c_void_p]


def read_aida_raw() -> Optional[str]:
    """Lê o texto bruto da memória compartilhada AIDA64_SensorValues, ou None."""
    h = _k32.OpenFileMappingW(FILE_MAP_READ, False, SHM_NAME)
    if not h:
        return None
    try:
        p = _k32.MapViewOfFile(h, FILE_MAP_READ, 0, 0, 0)
        if not p:
            return None
        try:
            mbi = _MBI()
            _k32.VirtualQuery(ctypes.c_void_p(p), ctypes.byref(mbi), ctypes.sizeof(mbi))
            return ctypes.string_at(p, mbi.RS).split(b"\0")[0].decode("latin1")
        finally:
            _k32.UnmapViewOfFile(p)
    finally:
        _k32.CloseHandle(h)


def parse_aida_xml(txt: str) -> list[tuple[str, str, str, str]]:
    return _SENSOR_RE.findall(txt)


def _get_if_octets() -> tuple[int, int]:
    tam = wt.DWORD(0)
    ctypes.windll.iphlpapi.GetIfTable(None, ctypes.byref(tam), False)
    buf = ctypes.create_string_buffer(tam.value)
    if ctypes.windll.iphlpapi.GetIfTable(buf, ctypes.byref(tam), False) != 0:
        return 0, 0
    n = wt.DWORD.from_buffer(buf).value
    rx = tx = 0
    for r in (_IFROW * n).from_buffer(buf, 4):
        if r.oper in (4, 5) and r.tipo in (6, 71) and not _VIRTUAL_IF.search(r.descr):
            rx += r.in_oct
            tx += r.out_oct
    return rx, tx


def format_mbps(val: float) -> str:
    return ("%.0f" if val >= 100 else "%.1f") % val


def format_rate_human(mbps: float) -> str:
    """Formata taxa em Mbps para exibição compacta (ex.: 0Kb/s, 450Kb/s, 12.4Mb/s)."""
    if mbps < 1.0:
        kbps = int(round(mbps * 1000.0))
        return f"{kbps}Kb/s"
    if mbps < 100.0:
        return f"{mbps:.1f}Mb/s"
    return f"{int(round(mbps))}Mb/s"


class SensorStore:
    """Agrega AIDA64 + rede Windows com cache, histórico para sparkline e vigia opcional."""

    def __init__(
        self,
        history_len: int = 30,
        enable_watchdog: bool = False,
        vram_total_mb: float = 24564.0,
    ) -> None:
        self._history_len = history_len
        self._enable_watchdog = enable_watchdog
        self._vram_total_mb = vram_total_mb
        self._lock = threading.Lock()
        self._last_aida: dict[str, float] = {}
        self._last_aida_ts = 0.0
        self._net_dl = 0.0
        self._net_ul = 0.0
        self._history: dict[str, deque[float]] = {}
        self._stop_event = threading.Event()
        self._net_thread: Optional[threading.Thread] = None
        self._watchdog_thread: Optional[threading.Thread] = None

    def start(self) -> None:
        self._stop_event.clear()
        self._net_thread = threading.Thread(target=self._net_loop, daemon=True, name="net-sampler")
        self._net_thread.start()
        if self._enable_watchdog:
            self._watchdog_thread = threading.Thread(target=self._watchdog_loop, daemon=True, name="aida-watchdog")
            self._watchdog_thread.start()

    def stop(self) -> None:
        self._stop_event.set()
        if self._net_thread and self._net_thread.is_alive():
            self._net_thread.join(timeout=1.5)
        if self._watchdog_thread and self._watchdog_thread.is_alive():
            self._watchdog_thread.join(timeout=1.5)

    def _net_loop(self) -> None:
        prev: Optional[tuple[int, int]] = None
        t0 = time.monotonic()
        while not self._stop_event.is_set():
            try:
                rx, tx = _get_if_octets()
                t1 = time.monotonic()
                if prev is not None and t1 > t0:
                    dl = ((rx - prev[0]) % 2**32) * 8 / 1e6 / (t1 - t0)
                    ul = ((tx - prev[1]) % 2**32) * 8 / 1e6 / (t1 - t0)
                    with self._lock:
                        self._net_dl = dl
                        self._net_ul = ul
                prev, t0 = (rx, tx), t1
            except Exception:
                pass
            self._stop_event.wait(timeout=1.0)

    def _restart_aida(self) -> None:
        _logger.warning("AIDA64 sem publicar há %.0fs; reiniciando tarefa %s", TRAVADO_S, TAREFA_AIDA)
        for args in (["schtasks", "/End", "/TN", TAREFA_AIDA], ["schtasks", "/Run", "/TN", TAREFA_AIDA]):
            try:
                subprocess.run(args, capture_output=True, timeout=30, creationflags=0x08000000)
            except Exception:
                pass
            time.sleep(2)

    def _watchdog_loop(self) -> None:
        last_hash: Optional[str] = None
        since = time.monotonic()
        next_allowed = 0.0
        while not self._stop_event.wait(timeout=5.0):
            try:
                txt = read_aida_raw()
                now = time.monotonic()
                h = hashlib.md5(txt.encode("latin1")).hexdigest() if txt else None
                if h is not None and h != last_hash:
                    last_hash, since = h, now
                    continue
                if now - since >= TRAVADO_S and now >= next_allowed:
                    self._restart_aida()
                    next_allowed = time.monotonic() + ESPERA_REINICIO_S
                    since = time.monotonic()
            except Exception:
                pass

    def refresh(self) -> dict[str, float]:
        """Lê os valores atuais (com cache de até CACHE_S) e atualiza históricos."""
        txt = read_aida_raw()
        now = time.monotonic()
        parsed: dict[str, float] = {}
        if txt:
            for _, sid, _, raw_val in parse_aida_xml(txt):
                try:
                    parsed[sid] = float(raw_val)
                except ValueError:
                    pass

        with self._lock:
            if parsed:
                self._last_aida = parsed
                self._last_aida_ts = now
            elif now - self._last_aida_ts <= CACHE_S:
                parsed = dict(self._last_aida)
            else:
                parsed = {}

            parsed["NET_DL"] = self._net_dl
            parsed["NET_UL"] = self._net_ul
            parsed["NET_TOTAL"] = self._net_dl + self._net_ul
            parsed["NET_TOTAL_MBS"] = (self._net_dl + self._net_ul) / 8.0
            if "SGPU1USEDDEMEM" in parsed and self._vram_total_mb > 0:
                parsed["VRAM_PCT"] = (parsed["SGPU1USEDDEMEM"] / self._vram_total_mb) * 100.0

            for sid, val in parsed.items():
                hist = self._history.setdefault(sid, deque(maxlen=self._history_len))
                hist.append(val)

            return parsed

    def get_history(self, sensor_id: str) -> list[float]:
        with self._lock:
            hist = self._history.get(sensor_id)
            return list(hist) if hist else []

    def format_sensor(
        self,
        sensor_id: str,
        values: dict[str, float],
        unit: str = "",
        decimals: int = 0,
    ) -> tuple[str, Optional[float]]:
        """Retorna (texto_formatado, valor_numerico_para_sparkline)."""
        if sensor_id == "NET_BOTH":
            dl = values.get("NET_DL")
            ul = values.get("NET_UL")
            if dl is None or ul is None:
                return "--/--", None
            return f"{format_mbps(dl)}/{format_mbps(ul)}", dl + ul

        val = values.get(sensor_id)
        if val is None:
            return "--", None

        rounded = round(val, decimals)
        if decimals == 0:
            num_str = f"{int(round(val))}"
        else:
            num_str = f"{rounded:.{decimals}f}"
        text = f"{num_str}{unit}" if unit in ("%", "°C") else f"{num_str} {unit}".strip()
        return text, float(rounded)
