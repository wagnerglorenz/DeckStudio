"""Fase 1 — Calibração de posições (1..18) e teste de eventos de botão.

Uso:
  python tools/map_keys.py --seconds 25
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ajazz_monitor.device import DeviceManager
from ajazz_monitor.device_proto import InputAction


def make_numbered_tile(idx: int) -> Image.Image:
    img = Image.new("RGB", (96, 96), (18, 20, 45))
    draw = ImageDraw.Draw(img)
    draw.rectangle([3, 3, 92, 92], outline=(247, 0, 231), width=2)
    font = ImageFont.load_default(size=28)
    label = f"{idx:02d}"
    bbox = draw.textbbox((0, 0), label, font=font)
    w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
    draw.text(((96 - w) // 2, (96 - h) // 2 - 4), label, fill=(255, 255, 255), font=font)
    return img


def main() -> int:
    parser = argparse.ArgumentParser(description="Envia números 1..18 para o AKP153E e registra cliques")
    parser.add_argument("--seconds", type=float, default=20.0, help="Tempo de escuta de botões")
    parser.add_argument("--count", type=int, default=18, help="Quantidade de posições para testar (padrão 18)")
    args = parser.parse_args()

    mgr = DeviceManager(max_tiles_per_s=15.0)
    pressed: list[int] = []

    def on_btn(pos: int, grid_key: str | None, action: InputAction) -> None:
        print(f"[{action.value:>7}] pos={pos} (grid={grid_key})")
        if action == InputAction.PRESS and pos not in pressed:
            pressed.append(pos)

    mgr.set_button_callback(on_btn)
    mgr.start(brightness=65)
    print(f"Enviando índices 1..{args.count} para o aparelho...")
    for idx in range(1, args.count + 1):
        mgr.queue_image(idx, make_numbered_tile(idx))

    print(f"Pressione as teclas no AKP153E nos próximos {args.seconds:.0f}s (Ctrl+C para sair)...")
    try:
        time.sleep(args.seconds)
    except KeyboardInterrupt:
        pass
    finally:
        mgr.stop(clear=True)

    print(f"Índices pressionados registrados: {sorted(pressed)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
