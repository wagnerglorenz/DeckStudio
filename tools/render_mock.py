"""Gera mockup fiel da página OBS Studio com ícones vetoriais 3x e relevo 3D."""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from legacy_monitor.render import (
    BUILTIN_THEMES,
    _font,
    render_button_tile,
    render_clock_tile,
    render_compact_health_tile,
    render_nav_circle_tile,
    render_obs_logo_tile,
    render_obs_status_tile,
    render_weather_tile,
)


def build_obs_mock(out_path: Path) -> None:
    theme = BUILTIN_THEMES["llm_dash"]
    tiles = {
        (0, 0): render_button_tile("GRAVANDO", subtitle="PARAR", state="active", theme=theme, icon="record"),
        (1, 0): render_button_tile("Pausar", subtitle="REC ATIVO", state="idle", theme=theme, icon="pause"),
        (2, 0): render_button_tile("Som Jogo", subtitle="ATIVO", state="idle", theme=theme, icon="audio"),
        (3, 0): render_button_tile("Track", subtitle="TRAINEE", state="idle", theme=theme, icon="track"),
        (4, 0): render_button_tile("Logo", subtitle="CANAL", state="idle", theme=theme, icon="logo"),
        (5, 0): render_clock_tile("21:28", "18/11", theme=theme),

        (0, 1): render_button_tile("1.Studio", subtitle="ACC", state="idle", theme=theme, icon="studio"),
        (1, 1): render_button_tile("2.Jogo", subtitle="ATIVA", state="active", theme=theme, icon="game_cam"),
        (2, 1): render_button_tile("3.Split", subtitle="ACC", state="idle", theme=theme, icon="split"),
        (3, 1): render_button_tile("4.Foco", subtitle="16:9", state="idle", theme=theme, icon="focus"),
        (4, 1): render_button_tile("5.CmdCtr", subtitle="MULTI", state="idle", theme=theme, icon="multi"),
        (5, 1): render_obs_status_tile("REC", "2.Jogo", theme=theme),

        (0, 2): render_compact_health_tile("CPU", 17, 56, "#22D3EE", theme=theme),
        (1, 2): render_compact_health_tile("GPU", 40, 46, "#22C55E", theme=theme),
        (2, 2): render_button_tile("6.Cam 4K", subtitle="CHEIA", state="idle", theme=theme, icon="cam4k"),
        (3, 2): render_nav_circle_tile("prev", theme=theme),
        (4, 2): render_button_tile("Tema", subtitle="TROCAR", state="idle", theme=theme, icon="theme"),
        (5, 2): render_weather_tile(24.7, 20.2, 30.8, "Sta Cruz Sul", 3, theme=theme),
    }

    tile_sz = 96
    gap = 14
    strip_gap = 26
    pad_x, pad_y = 28, 44
    w = pad_x * 2 + tile_sz * 6 + gap * 4 + strip_gap
    h = pad_y + tile_sz * 3 + gap * 2 + 24

    canvas = Image.new("RGB", (w, h), "#1C1F26")
    draw = ImageDraw.Draw(canvas)
    draw.rounded_rectangle([6, 6, w - 7, h - 7], radius=20, fill="#242832", outline="#3A404F", width=2)
    draw.text((pad_x, 14), "PÁGINA 2 — OBS STUDIO (Botões com ícones vetoriais 3x + relevo 3D sem bordas)", fill="#E2E8F0", font=_font(15, bold=True))

    sx0 = pad_x + 5 * (tile_sz + gap) - gap + strip_gap - 6
    sy0 = pad_y - 6
    draw.rounded_rectangle([sx0, sy0, sx0 + tile_sz + 12, sy0 + 3 * tile_sz + 2 * gap + 12], radius=14, fill="#000000", outline="#334155", width=2)

    for (col, row), img in tiles.items():
        if col < 5:
            x = pad_x + col * (tile_sz + gap)
            y = pad_y + row * (tile_sz + gap)
            draw.rounded_rectangle([x - 4, y - 4, x + tile_sz + 3, y + tile_sz + 3], radius=12, fill="#0A0D12", outline="#334155", width=2)
        else:
            x = sx0 + 6
            y = pad_y + row * (tile_sz + gap)
        canvas.paste(img, (x, y))

    out_path.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(out_path, format="PNG")
    print(f"Mock OBS salvo em: {out_path}")


if __name__ == "__main__":
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("mock_obs_bonito.png")
    build_obs_mock(out)
