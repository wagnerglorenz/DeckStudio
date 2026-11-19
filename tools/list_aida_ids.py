"""Lista os IDs e valores publicados atualmente na memória compartilhada do AIDA64."""
from __future__ import annotations

import ctypes
import ctypes.wintypes as wt
import re

SHM_NAME = "AIDA64_SensorValues"
FILE_MAP_READ = 4

k32 = ctypes.windll.kernel32
k32.OpenFileMappingW.restype = wt.HANDLE
k32.MapViewOfFile.restype = ctypes.c_void_p
k32.UnmapViewOfFile.argtypes = [ctypes.c_void_p]


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


def read_aida_raw() -> str | None:
    h = k32.OpenFileMappingW(FILE_MAP_READ, False, SHM_NAME)
    if not h:
        return None
    try:
        p = k32.MapViewOfFile(h, FILE_MAP_READ, 0, 0, 0)
        if not p:
            return None
        try:
            mbi = _MBI()
            k32.VirtualQuery(ctypes.c_void_p(p), ctypes.byref(mbi), ctypes.sizeof(mbi))
            return ctypes.string_at(p, mbi.RS).split(b"\0")[0].decode("latin1")
        finally:
            k32.UnmapViewOfFile(p)
    finally:
        k32.CloseHandle(h)


def main() -> int:
    txt = read_aida_raw()
    if not txt:
        print("AIDA64_SensorValues não encontrado (AIDA64 fechado ou memória compartilhada desativada).")
        return 1
    rows = re.findall(r"<(\w+)><id>(.*?)</id><label>(.*?)</label><value>(.*?)</value>", txt)
    for kind, sid, label, val in rows:
        print(f"{kind:<5} | {sid:<18} | {label:<32} | {val}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
