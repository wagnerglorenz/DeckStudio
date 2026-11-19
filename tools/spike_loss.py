"""Fase 0 — Prova de viabilidade de rede (iperf3) com o AJAZZ AKP153E.

Uso:
  python tools/spike_loss.py --arm B --duration 35
  python tools/spike_loss.py --arm C --hz 1 --duration 35
  python tools/spike_loss.py --arm D --hz 4 --duration 35
"""
from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ajazz_monitor.device_proto import AKP153EDevice, sanitize_image

OFFICIAL_PROCESSES = ("Stream Dock AJAZZ.exe", "QtWebEngineProcess.exe")


def official_app_running() -> list[str]:
    out = subprocess.check_output(
        ["tasklist", "/FO", "CSV", "/NH"],
        text=True,
        creationflags=0x08000000,
    )
    found = []
    for name in OFFICIAL_PROCESSES:
        if f'"{name}"' in out:
            found.append(name)
    return sorted(set(found))


def make_test_tiles(count: int = 13) -> list[bytes]:
    tiles = []
    for i in range(count):
        img = Image.new("RGB", (96, 96), (18, 20, 45))
        draw = ImageDraw.Draw(img)
        draw.rectangle([4, 4, 91, 91], outline=(247, 0, 231), width=2)
        draw.text((14, 36), f"T{i + 1:02d}", fill=(255, 255, 255))
        tiles.append(sanitize_image(img, size=96))
    return tiles


def main() -> int:
    parser = argparse.ArgumentParser(description="Fase 0 — Teste HID sem o app oficial")
    parser.add_argument("--arm", choices=["B", "C", "D"], required=True)
    parser.add_argument("--hz", type=float, default=1.0, help="Taxa de envio em Hz (braços C/D)")
    parser.add_argument("--duration", type=float, default=35.0, help="Duração em segundos")
    parser.add_argument("--keys", type=int, default=13, help="Quantidade de teclas (1-15)")
    args = parser.parse_args()

    running = official_app_running()
    if running:
        print(f"ERRO: app oficial ainda rodando ({', '.join(running)}). Feche-o antes de rodar.")
        return 2

    dev = AKP153EDevice()
    dev.open()
    dev.set_brightness(60)
    dev.clear_all()
    print(f"HID aberto com sucesso. Executando braço {args.arm} por {args.duration:.0f}s...")

    tiles_a = make_test_tiles(args.keys)
    # Segunda variação para simular mudança real a cada ciclo
    tiles_b = []
    for i in range(args.keys):
        img = Image.new("RGB", (96, 96), (18, 20, 45))
        draw = ImageDraw.Draw(img)
        draw.rectangle([4, 4, 91, 91], outline=(0, 220, 255), width=2)
        draw.text((14, 36), f"T{i + 1:02d}*", fill=(255, 255, 255))
        tiles_b.append(sanitize_image(img, size=96))

    start = time.monotonic()
    tick = 0
    interval = 1.0 / args.hz if args.arm in ("C", "D") else 1.0
    next_send = start

    try:
        while time.monotonic() - start < args.duration:
            ev = dev.read_event(timeout_ms=50)
            if ev is not None:
                print(f"Evento botão: pos={ev[0]} action={ev[1].value}")

            now = time.monotonic()
            if args.arm in ("C", "D") and now >= next_send:
                batch = tiles_a if tick % 2 == 0 else tiles_b
                for pos in range(1, args.keys + 1):
                    dev.set_button_jpeg(pos, batch[pos - 1])
                tick += 1
                next_send += interval
    finally:
        dev.close(clear=True)
        print("HID fechado com segurança.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
