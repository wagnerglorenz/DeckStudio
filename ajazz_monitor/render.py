"""Renderização de alta qualidade (supersampling 3x) com suporte a múltiplos temas e cores por temperatura."""
from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache
import math
from typing import Optional

from PIL import Image, ImageDraw, ImageFont

_SS = 3  # Fator de supersampling (288x288 -> 96x96 LANCZOS)


@dataclass(frozen=True)
class Theme:
    name: str = "ajazz"
    style: str = "ajazz"  # "ajazz" | "llm_dash"
    background: str = "#12142D"
    card_bg: str = "#162032"
    card_border: str = "#24324A"
    chart: str = "#F700E7"
    text: str = "#FFFFFF"
    muted: str = "#94A3B8"
    active: str = "#E11D48"
    accent: str = "#38BDF8"
    temp_colors: bool = True


BUILTIN_THEMES: dict[str, Theme] = {
    "ajazz": Theme(
        name="ajazz",
        style="ajazz",
        background="#12142D",
        card_bg="#12142D",
        card_border="#272A4D",
        chart="#F700E7",
        text="#FFFFFF",
        muted="#94A3B8",
        active="#E11D48",
        accent="#38BDF8",
        temp_colors=True,
    ),
    "llm_dash": Theme(
        name="llm_dash",
        style="llm_dash",
        background="#0D1117",
        card_bg="#162032",
        card_border="#24324A",
        chart="#22D3EE",
        text="#FFFFFF",
        muted="#94A3B8",
        active="#EF4444",
        accent="#22D3EE",
        temp_colors=True,
    ),
}

GROUP_ACCENTS: dict[str, str] = {
    "cpu": "#22D3EE",       # ciano (CPU na dashboard)
    "gpu": "#22C55E",       # verde neon (GPU na dashboard)
    "ram": "#EAB308",       # âmbar (RAM na dashboard)
    "vram": "#A855F7",      # roxo (VRAM na dashboard)
    "net": "#38BDF8",       # azul claro (Rede)
    "net_down": "#22C55E",  # verde (Download)
    "net_up": "#F59E0B",    # âmbar (Upload)
    "fps": "#F97316",       # laranja (FPS)
}


def temp_color(temp_c: float) -> str:
    """Escala de cor dinâmica por temperatura (°C): Verde -> Amarelo -> Vermelho."""
    if temp_c < 60.0:
        return "#22C55E"  # verde (frio / normal)
    if temp_c < 75.0:
        return "#EAB308"  # amarelo (atenção)
    return "#EF4444"      # vermelho (quente / alerta)


def usage_color(pct: float) -> str:
    """Escala de cor padrão para uso de memória (RAM / VRAM): Verde -> Amarelo -> Vermelho."""
    if pct < 65.0:
        return "#22C55E"  # verde (folgado)
    if pct < 85.0:
        return "#EAB308"  # amarelo (atenção)
    return "#EF4444"      # vermelho (alto / alerta)


def _hex_to_rgb(color: str) -> tuple[int, int, int]:
    c = color.lstrip("#")
    return int(c[0:2], 16), int(c[2:4], 16), int(c[4:6], 16)


def _font(size: int, bold: bool = True) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = (
        ("segoeuib.ttf", "arialbd.ttf", "DejaVuSans-Bold.ttf", "segoeui.ttf")
        if bold
        else ("segoeui.ttf", "arial.ttf", "DejaVuSans.ttf")
    )
    for name in candidates:
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            continue
    return ImageFont.load_default(size=size)


def _catmull_rom(points: list[tuple[float, float]], steps: int = 5) -> list[tuple[float, float]]:
    if len(points) < 3:
        return points
    out: list[tuple[float, float]] = []
    pts = [points[0]] + points + [points[-1]]
    for i in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[i - 1], pts[i], pts[i + 1], pts[i + 2]
        for s in range(steps):
            t = s / steps
            t2 = t * t
            t3 = t2 * t
            x = 0.5 * (
                (2 * p1[0])
                + (-p0[0] + p2[0]) * t
                + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2
                + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3
            )
            y = 0.5 * (
                (2 * p1[1])
                + (-p0[1] + p2[1]) * t
                + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2
                + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3
            )
            out.append((x, y))
    out.append(points[-1])
    return out


def _downsample(img: Image.Image, size: int) -> Image.Image:
    return img.convert("RGB").resize((size, size), Image.Resampling.LANCZOS)


@lru_cache(maxsize=16)
def _bevel_keycap(
    size: int,
    top_hex: str = "#1B2840",
    bot_hex: str = "#0E1625",
    hi_hex: str = "#2F4468",
    sh_hex: str = "#060A12",
) -> Image.Image:
    """Superfície de tecla com relevo 3D suave (degradê vertical + brilho sutil no topo, sem bordas grossas)."""
    S = size * _SS
    tr, tg, tb = _hex_to_rgb(top_hex)
    br, bg, bb = _hex_to_rgb(bot_hex)
    img = Image.new("RGB", (S, S))
    draw = ImageDraw.Draw(img)
    for y in range(S):
        t = y / max(1, S - 1)
        r = int(tr + (br - tr) * t)
        g = int(tg + (bg - tg) * t)
        b = int(tb + (bb - tb) * t)
        draw.line([(0, y), (S - 1, y)], fill=(r, g, b))
    # Brilho sutil de relevo no topo e sombra suave na base
    draw.line([(4 * _SS, 1 * _SS), (S - 5 * _SS, 1 * _SS)], fill=hi_hex, width=2 * _SS)
    draw.line([(4 * _SS, S - 2 * _SS), (S - 5 * _SS, S - 2 * _SS)], fill=sh_hex, width=2 * _SS)
    return img


def _render_llm_dash_ring(
    title: str,
    value_text: str,
    unit: str,
    pct: float,
    accent: str,
    theme: Theme,
    size: int,
) -> Image.Image:
    """Card estilo anel (CPU LOAD / GPU LOAD / RAM / VRAM) com relevo 3D sem borda."""
    S = size * _SS
    img = _bevel_keycap(size).copy()
    draw = ImageDraw.Draw(img)

    # Anel de progresso (trilha escura + arco colorido a partir das 12h)
    r_pad = 10 * _SS
    arc_box = [r_pad, r_pad, S - r_pad, S - r_pad]
    track_w = 7 * _SS
    draw.arc(arc_box, start=0, end=360, fill="#243044", width=track_w)
    clamped = max(0.0, min(100.0, pct))
    sweep = max(4.0, (clamped / 100.0) * 360.0) if clamped > 0 else 0.0
    if sweep > 0:
        draw.arc(arc_box, start=-90, end=-90 + sweep, fill=accent, width=track_w)

    # Valor central grande (ex.: 40%)
    val_str = f"{value_text.strip()}{unit.strip()}"
    f_val = _font((21 if len(val_str) <= 4 else 17) * _SS, bold=True)
    vb = draw.textbbox((0, 0), val_str, font=f_val)
    vw, vh = vb[2] - vb[0], vb[3] - vb[1]
    draw.text(((S - vw) // 2 - vb[0], 31 * _SS - vb[1]), val_str, fill=theme.text, font=f_val)

    # Título pequeno em caixa alta abaixo do valor
    lbl = title.upper()[:9]
    f_lbl = _font(10 * _SS, bold=True)
    lb = draw.textbbox((0, 0), lbl, font=f_lbl)
    lw = lb[2] - lb[0]
    draw.text(((S - lw) // 2 - lb[0], 56 * _SS - lb[1]), lbl, fill=theme.muted, font=f_lbl)

    return _downsample(img, size)


def _render_llm_dash_card(
    title: str,
    value_text: str,
    unit: str,
    pct: Optional[float],
    accent: str,
    is_temp: bool,
    theme: Theme,
    size: int,
) -> Image.Image:
    """Card métrico com relevo 3D sem borda (Potência, Clock, Temp com cor dinâmica, Rede, FPS)."""
    S = size * _SS
    img = _bevel_keycap(size).copy()
    draw = ImageDraw.Draw(img)

    # Título em caixa alta no topo
    lbl = title.upper()[:10]
    f_lbl = _font(11 * _SS, bold=True)
    lb = draw.textbbox((0, 0), lbl, font=f_lbl)
    lw = lb[2] - lb[0]
    draw.text(((S - lw) // 2 - lb[0], 13 * _SS - lb[1]), lbl, fill=theme.muted, font=f_lbl)

    # Número grande branco + unidade colorida ao lado (ex.: 56 °C / 104 W / 3550 MHz)
    val_clean = value_text.strip()
    if len(val_clean) <= 2:
        v_sz = 30 * _SS
    elif len(val_clean) == 3:
        v_sz = 26 * _SS
    elif len(val_clean) == 4:
        v_sz = 21 * _SS
    else:
        v_sz = 17 * _SS

    f_val = _font(v_sz, bold=True)
    vb = draw.textbbox((0, 0), val_clean, font=f_val)
    vw, vh = vb[2] - vb[0], vb[3] - vb[1]

    u_clean = ("°C" if is_temp and unit.strip() in ("°", "°C") else unit.strip())
    uw = 0
    f_unit = None
    if u_clean:
        u_sz = (14 if len(u_clean) <= 2 else 10) * _SS
        f_unit = _font(u_sz, bold=True)
        ub = draw.textbbox((0, 0), u_clean, font=f_unit)
        uw = ub[2] - ub[0] + 3 * _SS

    total_w = vw + uw
    vx = max(8 * _SS, (S - total_w) // 2)
    vy = 36 * _SS
    val_fill = accent if is_temp else theme.text
    draw.text((vx - vb[0], vy - vb[1]), val_clean, fill=val_fill, font=f_val)

    if u_clean and f_unit is not None:
        ub = draw.textbbox((0, 0), u_clean, font=f_unit)
        uh = ub[3] - ub[1]
        draw.text((vx + vw + 3 * _SS - ub[0], vy + vh - uh - ub[1]), u_clean, fill=accent, font=f_unit)

    # Barra de progresso fina e arredondada na base (estilo RAM/Ventoínhas/VRAM da dashboard)
    if pct is not None:
        bar_l, bar_r = 12 * _SS, S - 12 * _SS
        bar_t, bar_b = S - 16 * _SS, S - 10 * _SS
        draw.rounded_rectangle([bar_l, bar_t, bar_r, bar_b], radius=3 * _SS, fill="#243044")
        clamped = max(0.0, min(100.0, pct))
        fill_w = int((bar_r - bar_l) * (clamped / 100.0))
        if fill_w > 4 * _SS:
            draw.rounded_rectangle([bar_l, bar_t, bar_l + fill_w, bar_b], radius=3 * _SS, fill=accent)

    return _downsample(img, size)


def render_sensor_tile(
    title: str,
    value_text: str,
    unit: str = "",
    history: Optional[list[float]] = None,
    vmin: Optional[float] = 0.0,
    vmax: Optional[float] = 100.0,
    theme: Theme = BUILTIN_THEMES["ajazz"],
    size: int = 96,
    raw_value: Optional[float] = None,
    group: Optional[str] = None,
    card_style: Optional[str] = None,
    is_temp: bool = False,
) -> Image.Image:
    """Renderiza tile de sensor no tema ativo (AJAZZ Clássico ou LLM Dashboard)."""
    # Resolve cor dinâmica (temperatura e memória RAM/VRAM usam Verde -> Amarelo -> Vermelho)
    if is_temp and raw_value is not None and theme.temp_colors:
        accent = temp_color(raw_value)
    elif group in ("ram", "vram") and raw_value is not None and theme.temp_colors:
        accent = usage_color(raw_value)
    elif group and group in GROUP_ACCENTS:
        accent = GROUP_ACCENTS[group]
    else:
        accent = theme.chart

    pct: Optional[float] = None
    if raw_value is not None and vmin is not None and vmax is not None and vmax > vmin:
        pct = max(0.0, min(100.0, ((raw_value - vmin) / (vmax - vmin)) * 100.0))

    if theme.style == "llm_dash":
        if card_style == "ring" and pct is not None:
            return _render_llm_dash_ring(title, value_text, unit, pct, accent, theme, size)
        return _render_llm_dash_card(title, value_text, unit, pct, accent, is_temp, theme, size)

    # --- Estilo AJAZZ Clássico (System Vitals) ---
    S = size * _SS
    bg_rgb = _hex_to_rgb(theme.background)
    base = Image.new("RGBA", (S, S), (*bg_rgb, 255))

    chart_color = accent if (is_temp and theme.temp_colors) else theme.chart
    has_chart = history is not None and vmin is not None and vmax is not None
    if has_chart:
        left, right = 4 * _SS, S - 4 * _SS
        top, bottom = 32 * _SS, S - 6 * _SS
        pts = history[-30:] if history else [0.0, 0.0]
        if len(pts) == 1:
            pts = [pts[0], pts[0]]
        span = (vmax - vmin) if (vmax is not None and vmin is not None and vmax > vmin) else 1.0
        n = len(pts)
        raw_coords: list[tuple[float, float]] = []
        for i, val in enumerate(pts):
            x = left + (right - left) * (i / max(n - 1, 1))
            norm = max(0.0, min(1.0, (val - (vmin or 0.0)) / span))
            y = bottom - norm * (bottom - top)
            raw_coords.append((x, y))

        smooth = _catmull_rom(raw_coords, steps=5)
        clamped = [(x, max(float(top), min(float(bottom), y))) for x, y in smooth]

        chart_rgb = _hex_to_rgb(chart_color)
        mask = Image.new("L", (S, S), 0)
        mdraw = ImageDraw.Draw(mask)
        poly = [(left, bottom)] + clamped + [(right, bottom)]
        mdraw.polygon(poly, fill=255)

        grad = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        gdraw = ImageDraw.Draw(grad)
        min_y = int(min(y for _, y in clamped))
        grad_h = max(1, bottom - min_y)
        for y in range(min_y, bottom + 1):
            alpha = int(135 * (1.0 - (y - min_y) / grad_h) ** 1.1)
            gdraw.line([(left, y), (right, y)], fill=(*chart_rgb, alpha))
        grad.putalpha(Image.composite(grad.getchannel("A"), Image.new("L", (S, S), 0), mask))

        base = Image.alpha_composite(base, grad)
        draw = ImageDraw.Draw(base)
        draw.line(clamped, fill=(*chart_rgb, 255), width=3 * _SS, joint="curve")
    else:
        draw = ImageDraw.Draw(base)

    f_title = _font(13 * _SS, bold=True)
    tb = draw.textbbox((0, 0), title, font=f_title)
    tw = tb[2] - tb[0]
    draw.text(
        ((S - tw) // 2, 11 * _SS),
        title,
        fill=theme.text,
        font=f_title,
        stroke_width=2 * _SS,
        stroke_fill=theme.background,
    )

    val_clean = value_text.strip()
    if len(val_clean) <= 2:
        v_size = 33 * _SS
    elif len(val_clean) == 3:
        v_size = 29 * _SS
    elif len(val_clean) == 4:
        v_size = 24 * _SS
    else:
        v_size = 19 * _SS

    f_val = _font(v_size, bold=True)
    vb = draw.textbbox((0, 0), val_clean, font=f_val)
    vw, vh = vb[2] - vb[0], vb[3] - vb[1]

    u_clean = unit.strip()
    uw = 0
    f_unit = None
    if u_clean:
        u_size = (15 if len(u_clean) <= 2 else 11) * _SS
        f_unit = _font(u_size, bold=True)
        ub = draw.textbbox((0, 0), u_clean, font=f_unit)
        uw = ub[2] - ub[0] + 2 * _SS

    total_w = vw + uw
    vx = max(4 * _SS, (S - total_w) // 2)
    vy = 32 * _SS if has_chart else (S - vh) // 2 + 2 * _SS
    val_fill = chart_color if (is_temp and theme.temp_colors) else theme.text
    draw.text(
        (vx - vb[0], vy - vb[1]),
        val_clean,
        fill=val_fill,
        font=f_val,
        stroke_width=3 * _SS,
        stroke_fill=theme.background,
    )

    if u_clean and f_unit is not None:
        ub = draw.textbbox((0, 0), u_clean, font=f_unit)
        draw.text(
            (vx + vw + 2 * _SS - ub[0], vy - ub[1] - 1 * _SS),
            u_clean,
            fill=val_fill,
            font=f_unit,
            stroke_width=2 * _SS,
            stroke_fill=theme.background,
        )

    return _downsample(base, size)


def render_nav_circle_tile(direction: str = "prev", theme: Theme = BUILTIN_THEMES["ajazz"], size: int = 96) -> Image.Image:
    """Botão de navegação (< ou >) com relevo 3D sem borda externa."""
    S = size * _SS
    if theme.style == "llm_dash":
        img = _bevel_keycap(size).copy()
        draw = ImageDraw.Draw(img)
        c_pad = 16 * _SS
        draw.ellipse([c_pad, c_pad, S - c_pad, S - c_pad], outline=theme.accent, width=3 * _SS)
        arrow_col = "#FFFFFF"
    else:
        img = Image.new("RGB", (S, S), "#181920")
        draw = ImageDraw.Draw(img)
        pad = 8 * _SS
        draw.ellipse([pad, pad, S - pad, S - pad], outline="#FFFFFF", width=4 * _SS)
        arrow_col = "#FFFFFF"

    cx, cy = S // 2, S // 2
    dx = 9 * _SS
    dy = 15 * _SS
    if direction == "prev":
        pts = [(cx + dx // 2, cy - dy), (cx - dx, cy), (cx + dx // 2, cy + dy)]
    else:
        pts = [(cx - dx // 2, cy - dy), (cx + dx, cy), (cx - dx // 2, cy + dy)]
    draw.line(pts, fill=arrow_col, width=4 * _SS, joint="curve")
    return _downsample(img, size)


def render_obs_logo_tile(
    theme: Theme = BUILTIN_THEMES["llm_dash"],
    size: int = 96,
) -> Image.Image:
    """Botão OBS Studio com logo vetorial (3 hélices) e relevo 3D, sem borda."""
    S = size * _SS
    img = _bevel_keycap(size).copy() if theme.style == "llm_dash" else Image.new("RGB", (S, S), theme.background)
    draw = ImageDraw.Draw(img)

    cx, cy = S // 2, 38 * _SS
    r_outer = 24 * _SS
    # Disco escuro do logo OBS
    draw.ellipse([cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer], fill="#0B0E14", outline="#CBD5E1", width=2 * _SS)

    # 3 hélices/arcos característicos do logo OBS (120° entre si)
    blade_r = 12 * _SS
    orbit = 9 * _SS
    for idx, base_deg in enumerate((270, 30, 150)):
        rad = math.radians(base_deg)
        bx = cx + int(orbit * math.cos(rad))
        by = cy + int(orbit * math.sin(rad))
        bbox = [bx - blade_r, by - blade_r, bx + blade_r, by + blade_r]
        start_a = (base_deg - 10) % 360
        end_a = (base_deg + 195) % 360
        draw.arc(bbox, start=start_a, end=end_a, fill="#FFFFFF", width=5 * _SS)

    # Texto "OBS" limpo abaixo do logo
    f = _font(13 * _SS, bold=True)
    tb = draw.textbbox((0, 0), "OBS", font=f)
    draw.text(((S - (tb[2] - tb[0])) // 2 - tb[0], 71 * _SS - tb[1]), "OBS", fill="#E2E8F0", font=f)
    return _downsample(img, size)


def render_net_strip_tile(
    dl_text: str,
    ul_text: str,
    theme: Theme = BUILTIN_THEMES["ajazz"],
    size: int = 96,
) -> Image.Image:
    """Widget lateral de rede."""
    S = size * _SS
    if theme.style == "llm_dash":
        img = Image.new("RGB", (S, S), theme.background)
        draw = ImageDraw.Draw(img)
        pad = 3 * _SS
        draw.rounded_rectangle(
            [pad, pad, S - pad, S - pad],
            radius=12 * _SS,
            fill=theme.card_bg,
            outline=theme.card_border,
            width=2 * _SS,
        )
        f_lbl = _font(10 * _SS, bold=True)
        draw.text((12 * _SS, 11 * _SS), "REDE", fill=theme.muted, font=f_lbl)
        f = _font(14 * _SS, bold=True)
        draw.text((12 * _SS, 31 * _SS), f"↑ {ul_text}", fill="#F59E0B", font=f)
        draw.text((12 * _SS, 55 * _SS), f"↓ {dl_text}", fill="#22C55E", font=f)
        return _downsample(img, size)

    img = Image.new("RGB", (S, S), "#14161D")
    draw = ImageDraw.Draw(img)
    y_top = 15 * _SS
    x_l, x_r = 12 * _SS, S - 16 * _SS
    draw.line([(x_l, y_top), (x_r, y_top), (x_r, 9 * _SS)], fill="#1EE61E", width=3 * _SS, joint="curve")
    draw.polygon([(x_r, 5 * _SS), (x_r - 6 * _SS, 13 * _SS), (x_r + 6 * _SS, 13 * _SS)], fill="#1EE61E")

    y_bot = S - 15 * _SS
    draw.line([(x_l, y_bot), (x_r, y_bot), (x_r, S - 9 * _SS)], fill="#18C8FF", width=3 * _SS, joint="curve")
    draw.polygon([(x_r, S - 5 * _SS), (x_r - 6 * _SS, S - 13 * _SS), (x_r + 6 * _SS, S - 13 * _SS)], fill="#18C8FF")

    f_sz = (18 if max(len(dl_text), len(ul_text)) <= 7 else 14) * _SS
    f = _font(f_sz, bold=True)
    for txt, y in ((ul_text, 26 * _SS), (dl_text, 50 * _SS)):
        bb = draw.textbbox((0, 0), txt, font=f)
        tw = bb[2] - bb[0]
        draw.text(((S - tw) // 2 - bb[0], y), txt, fill="#FFFFFF", font=f)

    return _downsample(img, size)


@lru_cache(maxsize=4)
def _cpu_chip_base(size: int) -> Image.Image:
    S = size * _SS
    img = Image.new("RGB", (S, S), "#1D2A3F")
    draw = ImageDraw.Draw(img)

    pin_color = "#CFA036"
    offsets = [25 * _SS, 38 * _SS, 51 * _SS, 64 * _SS]
    pw = 6 * _SS
    for o in offsets:
        draw.rounded_rectangle([o, 4 * _SS, o + pw, 18 * _SS], radius=2 * _SS, fill=pin_color)
        draw.rounded_rectangle([o, S - 18 * _SS, o + pw, S - 4 * _SS], radius=2 * _SS, fill=pin_color)
        draw.rounded_rectangle([4 * _SS, o, 18 * _SS, o + pw], radius=2 * _SS, fill=pin_color)
        draw.rounded_rectangle([S - 18 * _SS, o, S - 4 * _SS, o + pw], radius=2 * _SS, fill=pin_color)

    outer = [18 * _SS, 18 * _SS, S - 18 * _SS, S - 18 * _SS]
    inner = [23 * _SS, 23 * _SS, S - 23 * _SS, S - 23 * _SS]

    grad = Image.new("RGB", (S, S), "#000000")
    gpx = grad.load()
    cx, cy = S / 2.0, S / 2.0
    for y in range(outer[1], outer[3]):
        for x in range(outer[0], outer[2]):
            ang = (math.atan2(y - cy, x - cx) + math.pi) / (2 * math.pi)
            h = ang * 6.0
            i = int(h) % 6
            f = h - int(h)
            if i == 0:
                r, g, b = 255, int(255 * f), 0
            elif i == 1:
                r, g, b = int(255 * (1 - f)), 255, 0
            elif i == 2:
                r, g, b = 0, 255, int(255 * f)
            elif i == 3:
                r, g, b = 0, int(255 * (1 - f)), 255
            elif i == 4:
                r, g, b = int(255 * f), 0, 255
            else:
                r, g, b = 255, 0, int(255 * (1 - f))
            gpx[x, y] = (r, g, b)

    ring_mask = Image.new("L", (S, S), 0)
    rmd = ImageDraw.Draw(ring_mask)
    rmd.rounded_rectangle(outer, radius=11 * _SS, fill=255)
    rmd.rounded_rectangle(inner, radius=7 * _SS, fill=0)
    img.paste(grad, (0, 0), ring_mask)
    draw.rounded_rectangle(inner, radius=7 * _SS, fill="#08090D")
    return img


def render_cpu_chip_tile(
    temp_text: str,
    temp_value: Optional[float] = None,
    theme: Theme = BUILTIN_THEMES["ajazz"],
    size: int = 96,
) -> Image.Image:
    """Widget lateral de CPU °C com cor dinâmica por temperatura."""
    t_col = temp_color(temp_value) if (temp_value is not None and theme.temp_colors) else "#FFFFFF"

    if theme.style == "llm_dash":
        val_only = temp_text.replace("°C", "").replace("°", "").strip()
        pct = max(0.0, min(100.0, temp_value)) if temp_value is not None else None
        return _render_llm_dash_card("CPU", val_only, "°C", pct, t_col, True, theme, size)

    S = size * _SS
    img = _cpu_chip_base(size).copy()
    draw = ImageDraw.Draw(img)
    f = _font(21 * _SS, bold=True)
    bb = draw.textbbox((0, 0), temp_text, font=f)
    tw, th = bb[2] - bb[0], bb[3] - bb[1]
    draw.text(((S - tw) // 2 - bb[0], (S - th) // 2 - bb[1]), temp_text, fill=t_col, font=f)
    return _downsample(img, size)


def render_ram_gauge_tile(
    pct_value: float,
    theme: Theme = BUILTIN_THEMES["ajazz"],
    size: int = 96,
) -> Image.Image:
    """Widget lateral de RAM %."""
    if theme.style == "llm_dash":
        col = usage_color(pct_value) if theme.temp_colors else GROUP_ACCENTS["ram"]
        return _render_llm_dash_ring("RAM", f"{int(round(pct_value))}", "%", pct_value, col, theme, size)

    S = size * _SS
    img = Image.new("RGB", (S, S), "#1D2A3F")
    draw = ImageDraw.Draw(img)

    arc_box = [11 * _SS, 8 * _SS, S - 11 * _SS, S - 14 * _SS]
    start_deg, end_deg = 165, 375
    steps = 36
    for i in range(steps):
        t = i / (steps - 1)
        a0 = start_deg + (end_deg - start_deg) * (i / steps)
        a1 = start_deg + (end_deg - start_deg) * ((i + 1.2) / steps)
        if t < 0.35:
            k = t / 0.35
            col = (0, int(212 + 30 * k), int(255 * (1 - k) + 90 * k))
        elif t < 0.7:
            k = (t - 0.35) / 0.35
            col = (int(245 * k), int(242 - 50 * k), int(90 * (1 - k)))
        else:
            k = (t - 0.7) / 0.3
            col = (255, int(192 * (1 - k) + 55 * k), 0)
        draw.arc(arc_box, start=a0, end=a1, fill=col, width=4 * _SS)

    txt = f"{int(round(pct_value))}%"
    f = _font(23 * _SS, bold=True)
    bb = draw.textbbox((0, 0), txt, font=f)
    tw = bb[2] - bb[0]
    draw.text(((S - tw) // 2 - bb[0], 33 * _SS), txt, fill="#FFFFFF", font=f)

    draw.rounded_rectangle([17 * _SS, 64 * _SS, S - 17 * _SS, 86 * _SS], radius=4 * _SS, outline="#16A34A", width=2 * _SS, fill="#162235")
    for i in range(4):
        x0 = (22 + i * 13) * _SS
        draw.rounded_rectangle([x0, 68 * _SS, x0 + 9 * _SS, 79 * _SS], radius=2 * _SS, fill="#64748B")
    for x in range(23, size - 23, 4):
        if abs(x - size // 2) > 3:
            draw.line([(x * _SS, 81 * _SS), (x * _SS, 87 * _SS)], fill="#D4A017", width=2 * _SS)

    return _downsample(img, size)


def render_clock_tile(
    time_str: str,
    date_str: str,
    theme: Theme = BUILTIN_THEMES["llm_dash"],
    size: int = 96,
) -> Image.Image:
    """Widget lateral de Relógio + Data integrado à faixa preta (sem recortes e sem corte à esquerda)."""
    S = size * _SS
    img = Image.new("RGB", (S, S), "#000000")
    draw = ImageDraw.Draw(img)
    cx = S // 2 + 5 * _SS  # compensa leve recorte da moldura esquerda na faixa lateral

    f_time = _font(21 * _SS, bold=True)
    tb = draw.textbbox((0, 0), time_str, font=f_time)
    draw.text((cx - (tb[2] - tb[0]) // 2 - tb[0], 26 * _SS - tb[1]), time_str, fill="#FFFFFF", font=f_time)

    f_date = _font(13 * _SS, bold=True)
    db = draw.textbbox((0, 0), date_str, font=f_date)
    draw.text((cx - (db[2] - db[0]) // 2 - db[0], 56 * _SS - db[1]), date_str, fill=theme.accent, font=f_date)
    return _downsample(img, size)


def render_obs_status_tile(
    status: str,
    scene_short: str,
    theme: Theme = BUILTIN_THEMES["llm_dash"],
    size: int = 96,
) -> Image.Image:
    """Widget lateral de Status do OBS integrado à faixa preta (sem recortes)."""
    S = size * _SS
    img = Image.new("RGB", (S, S), "#000000")
    draw = ImageDraw.Draw(img)
    cx = S // 2 + 5 * _SS

    if status == "REC":
        obs_col = "#EF4444"
    elif status == "PAUSADO":
        obs_col = "#EAB308"
    elif status == "PRONTO":
        obs_col = "#22C55E"
    else:
        obs_col = "#64748B"

    draw.ellipse([cx - 24 * _SS, 15 * _SS, cx - 14 * _SS, 25 * _SS], fill=obs_col)
    draw.text((cx - 9 * _SS, 13 * _SS), "OBS", fill=theme.muted, font=_font(13 * _SS, bold=True))

    f_st = _font(15 * _SS, bold=True)
    sb = draw.textbbox((0, 0), status, font=f_st)
    draw.text((cx - (sb[2] - sb[0]) // 2 - sb[0], 39 * _SS - sb[1]), status, fill=obs_col, font=f_st)

    f_sc = _font(11 * _SS, bold=True)
    sc = (scene_short or "---")[:10]
    scb = draw.textbbox((0, 0), sc, font=f_sc)
    draw.text((cx - (scb[2] - scb[0]) // 2 - scb[0], 64 * _SS - scb[1]), sc, fill="#CBD5E1", font=f_sc)
    return _downsample(img, size)


def render_weather_tile(
    temp_c: Optional[float],
    tmin: Optional[float],
    tmax: Optional[float],
    city: str,
    weather_code: int = 0,
    theme: Theme = BUILTIN_THEMES["llm_dash"],
    size: int = 96,
) -> Image.Image:
    """Widget lateral de Clima integrado à faixa preta (sem recortes e sem corte à esquerda)."""
    S = size * _SS
    img = Image.new("RGB", (S, S), "#000000")
    draw = ImageDraw.Draw(img)
    cx = S // 2 + 5 * _SS

    # Ícone vetorial centralizado acima + temperatura ao lado
    ix = cx - 28 * _SS
    if weather_code <= 1:
        draw.ellipse([ix + 2 * _SS, 12 * _SS, ix + 22 * _SS, 32 * _SS], fill="#F59E0B")
    elif weather_code <= 3:
        draw.ellipse([ix, 11 * _SS, ix + 15 * _SS, 26 * _SS], fill="#F59E0B")
        draw.rounded_rectangle([ix + 6 * _SS, 19 * _SS, ix + 27 * _SS, 30 * _SS], radius=5 * _SS, fill="#CBD5E1")
        draw.ellipse([ix + 9 * _SS, 13 * _SS, ix + 22 * _SS, 26 * _SS], fill="#CBD5E1")
    elif weather_code < 60:
        draw.rounded_rectangle([ix + 2 * _SS, 18 * _SS, ix + 27 * _SS, 30 * _SS], radius=5 * _SS, fill="#94A3B8")
        draw.ellipse([ix + 6 * _SS, 12 * _SS, ix + 21 * _SS, 26 * _SS], fill="#CBD5E1")
    else:
        draw.rounded_rectangle([ix + 2 * _SS, 15 * _SS, ix + 27 * _SS, 26 * _SS], radius=5 * _SS, fill="#94A3B8")
        draw.ellipse([ix + 6 * _SS, 10 * _SS, ix + 21 * _SS, 23 * _SS], fill="#CBD5E1")
        drop_col = "#F59E0B" if weather_code >= 95 else "#38BDF8"
        for dx in (8, 15, 22):
            draw.line([(ix + dx * _SS, 28 * _SS), (ix + (dx - 2) * _SS, 34 * _SS)], fill=drop_col, width=2 * _SS)

    t_str = f"{int(round(temp_c))}°" if temp_c is not None else "--°"
    f_t = _font(20 * _SS, bold=True)
    tb = draw.textbbox((0, 0), t_str, font=f_t)
    draw.text((cx + 3 * _SS - tb[0], 14 * _SS - tb[1]), t_str, fill="#FFFFFF", font=f_t)

    mm = f"{int(round(tmin))}°/{int(round(tmax))}°" if (tmin is not None and tmax is not None) else "--/--"
    f_mm = _font(12 * _SS, bold=True)
    mb = draw.textbbox((0, 0), mm, font=f_mm)
    draw.text((cx - (mb[2] - mb[0]) // 2 - mb[0], 46 * _SS - mb[1]), mm, fill="#38BDF8", font=f_mm)

    c_lbl = city[:11].upper()
    f_c = _font(9 * _SS, bold=True)
    cb = draw.textbbox((0, 0), c_lbl, font=f_c)
    draw.text((cx - (cb[2] - cb[0]) // 2 - cb[0], 68 * _SS - cb[1]), c_lbl, fill=theme.muted, font=f_c)
    return _downsample(img, size)


def render_compact_health_tile(
    label: str,
    pct: Optional[float],
    temp_c: Optional[float],
    accent: str,
    theme: Theme = BUILTIN_THEMES["llm_dash"],
    size: int = 96,
) -> Image.Image:
    """Card compacto para página OBS com relevo 3D sem borda."""
    S = size * _SS
    img = _bevel_keycap(size).copy()
    draw = ImageDraw.Draw(img)

    f_lbl = _font(11 * _SS, bold=True)
    lb = draw.textbbox((0, 0), label, font=f_lbl)
    draw.text(((S - (lb[2] - lb[0])) // 2 - lb[0], 10 * _SS - lb[1]), label, fill=theme.muted, font=f_lbl)

    p_str = f"{int(round(pct))}%" if pct is not None else "--%"
    f_p = _font(20 * _SS, bold=True)
    pb = draw.textbbox((0, 0), p_str, font=f_p)
    draw.text(((S - (pb[2] - pb[0])) // 2 - pb[0], 28 * _SS - pb[1]), p_str, fill=accent, font=f_p)

    t_str = f"{int(round(temp_c))}°C" if temp_c is not None else "--°C"
    t_col = temp_color(temp_c) if temp_c is not None else theme.text
    f_t = _font(19 * _SS, bold=True)
    tb = draw.textbbox((0, 0), t_str, font=f_t)
    draw.text(((S - (tb[2] - tb[0])) // 2 - tb[0], 56 * _SS - tb[1]), t_str, fill=t_col, font=f_t)
    return _downsample(img, size)


def _draw_button_icon(
    draw: ImageDraw.ImageDraw,
    icon: str,
    cx: int,
    cy: int,
    color: str,
    state: str,
) -> None:
    """Desenha ícones vetoriais limpos em 3x supersampling para os botões do OBS."""
    s = _SS
    if icon == "record":
        r = 16 * s
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=color, width=3 * s)
        if state == "active":
            # Quadrado de STOP dentro do anel quando gravando
            sq = 7 * s
            draw.rounded_rectangle([cx - sq, cy - sq, cx + sq, cy + sq], radius=2 * s, fill="#FFFFFF")
        else:
            dot = 9 * s
            dot_col = "#EF4444" if state == "idle" else ("#FEF08A" if state == "warning" else color)
            draw.ellipse([cx - dot, cy - dot, cx + dot, cy + dot], fill=dot_col)

    elif icon == "pause":
        if state == "warning":
            # Triângulo Play (Retomar)
            pts = [(cx - 7 * s, cy - 12 * s), (cx + 12 * s, cy), (cx - 7 * s, cy + 12 * s)]
            draw.polygon(pts, fill="#FFFFFF")
        else:
            # Duas barras verticais de Pause
            draw.rounded_rectangle([cx - 10 * s, cy - 12 * s, cx - 3 * s, cy + 12 * s], radius=2 * s, fill=color)
            draw.rounded_rectangle([cx + 3 * s, cy - 12 * s, cx + 10 * s, cy + 12 * s], radius=2 * s, fill=color)

    elif icon == "audio":
        # Cone do alto-falante
        draw.rounded_rectangle([cx - 15 * s, cy - 6 * s, cx - 7 * s, cy + 6 * s], radius=1 * s, fill=color)
        draw.polygon([(cx - 8 * s, cy - 6 * s), (cx + 1 * s, cy - 13 * s), (cx + 1 * s, cy + 13 * s), (cx - 8 * s, cy + 6 * s)], fill=color)
        if state == "active":
            # X vermelho/amarelo de mudo
            draw.line([(cx + 6 * s, cy - 7 * s), (cx + 16 * s, cy + 7 * s)], fill="#FEF08A", width=3 * s)
            draw.line([(cx + 16 * s, cy - 7 * s), (cx + 6 * s, cy + 7 * s)], fill="#FEF08A", width=3 * s)
        else:
            # Ondas sonoras
            draw.arc([cx - 2 * s, cy - 9 * s, cx + 11 * s, cy + 9 * s], start=-50, end=50, fill=color, width=2 * s)
            draw.arc([cx + 1 * s, cy - 14 * s, cx + 17 * s, cy + 14 * s], start=-55, end=55, fill=color, width=2 * s)

    elif icon == "studio":
        # Tela principal + barra lateral + câmera
        draw.rounded_rectangle([cx - 18 * s, cy - 13 * s, cx + 18 * s, cy + 11 * s], radius=3 * s, outline=color, width=2 * s)
        draw.rounded_rectangle([cx - 14 * s, cy - 9 * s, cx + 3 * s, cy + 7 * s], radius=2 * s, fill=color)
        draw.rounded_rectangle([cx + 6 * s, cy - 9 * s, cx + 14 * s, cy - 1 * s], radius=2 * s, outline=color, width=2 * s)
        draw.rounded_rectangle([cx + 6 * s, cy + 2 * s, cx + 14 * s, cy + 7 * s], radius=1 * s, fill=color)

    elif icon == "game_cam":
        # Tela 16:9 cheia + PiP no canto inferior direito
        draw.rounded_rectangle([cx - 18 * s, cy - 12 * s, cx + 18 * s, cy + 12 * s], radius=3 * s, outline=color, width=2 * s)
        draw.rounded_rectangle([cx + 3 * s, cy + 1 * s, cx + 14 * s, cy + 9 * s], radius=2 * s, fill="#FEF08A" if state == "active" else color)

    elif icon == "split":
        # Duas colunas divididas (Lateral Split)
        draw.rounded_rectangle([cx - 18 * s, cy - 12 * s, cx - 2 * s, cy + 12 * s], radius=3 * s, outline=color, width=2 * s)
        draw.rounded_rectangle([cx + 2 * s, cy - 12 * s, cx + 18 * s, cy + 12 * s], radius=3 * s, outline=color, width=2 * s)
        draw.rounded_rectangle([cx - 14 * s, cy - 8 * s, cx - 6 * s, cy + 8 * s], radius=1 * s, fill=color)

    elif icon == "focus":
        # Moldura de foco central (4 cantoneiras)
        l, r, t, b = cx - 17 * s, cx + 17 * s, cy - 11 * s, cy + 11 * s
        k = 7 * s
        w = 3 * s
        draw.line([(l, t + k), (l, t), (l + k, t)], fill=color, width=w)
        draw.line([(r - k, t), (r, t), (r, t + k)], fill=color, width=w)
        draw.line([(l, b - k), (l, b), (l + k, b)], fill=color, width=w)
        draw.line([(r - k, b), (r, b), (r, b - k)], fill=color, width=w)
        draw.ellipse([cx - 4 * s, cy - 4 * s, cx + 4 * s, cy + 4 * s], fill=color)

    elif icon == "multi":
        # 3 telas Command Center
        draw.rounded_rectangle([cx - 19 * s, cy - 10 * s, cx - 7 * s, cy + 10 * s], radius=2 * s, outline=color, width=2 * s)
        draw.rounded_rectangle([cx - 5 * s, cy - 13 * s, cx + 5 * s, cy + 10 * s], radius=2 * s, fill=color)
        draw.rounded_rectangle([cx + 7 * s, cy - 10 * s, cx + 19 * s, cy + 10 * s], radius=2 * s, outline=color, width=2 * s)

    elif icon == "cam4k":
        # Corpo de câmera + lente
        draw.rounded_rectangle([cx - 17 * s, cy - 10 * s, cx + 17 * s, cy + 12 * s], radius=4 * s, outline=color, width=2 * s)
        draw.rounded_rectangle([cx - 7 * s, cy - 15 * s, cx + 7 * s, cy - 10 * s], radius=2 * s, fill=color)
        draw.ellipse([cx - 7 * s, cy - 5 * s, cx + 7 * s, cy + 9 * s], outline=color, width=3 * s)
        draw.ellipse([cx - 3 * s, cy - 1 * s, cx + 3 * s, cy + 5 * s], fill=color)

    elif icon == "track":
        # Volante de simracing
        r = 15 * s
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=color, width=3 * s)
        draw.line([(cx - r + 2 * s, cy), (cx + r - 2 * s, cy)], fill=color, width=3 * s)
        draw.line([(cx, cy), (cx, cy + r - 2 * s)], fill=color, width=3 * s)
        draw.ellipse([cx - 4 * s, cy - 4 * s, cx + 4 * s, cy + 4 * s], fill=color)

    elif icon == "logo":
        # Badge de canal (triângulo play dentro de escudo/retângulo)
        draw.rounded_rectangle([cx - 17 * s, cy - 12 * s, cx + 17 * s, cy + 12 * s], radius=6 * s, outline=color, width=3 * s)
        draw.polygon([(cx - 4 * s, cy - 6 * s), (cx + 8 * s, cy), (cx - 4 * s, cy + 6 * s)], fill=color)

    elif icon == "theme":
        # Círculo de contraste (metade preenchida)
        r = 14 * s
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=color, width=3 * s)
        draw.pieslice([cx - r, cy - r, cx + r, cy + r], start=90, end=270, fill=color)

    elif icon == "discord":
        # Logo estilizado do Discord (Clyde)
        draw.rounded_rectangle([cx - 17 * s, cy - 11 * s, cx + 17 * s, cy + 10 * s], radius=9 * s, fill="#FFFFFF")
        draw.polygon([(cx - 14 * s, cy - 9 * s), (cx - 7 * s, cy - 14 * s), (cx - 4 * s, cy - 10 * s)], fill="#FFFFFF")
        draw.polygon([(cx + 14 * s, cy - 9 * s), (cx + 7 * s, cy - 14 * s), (cx + 4 * s, cy - 10 * s)], fill="#FFFFFF")
        # Olhos em blurple
        draw.ellipse([cx - 10 * s, cy - 4 * s, cx - 3 * s, cy + 3 * s], fill="#5865F2")
        draw.ellipse([cx + 3 * s, cy - 4 * s, cx + 10 * s, cy + 3 * s], fill="#5865F2")
        # Recorte inferior sutil
        draw.pieslice([cx - 7 * s, cy + 6 * s, cx + 7 * s, cy + 16 * s], start=180, end=360, fill="#4752C4")


def render_button_tile(
    title: str,
    subtitle: str = "",
    state: str = "idle",
    theme: Theme = BUILTIN_THEMES["ajazz"],
    size: int = 96,
    icon: Optional[str] = None,
) -> Image.Image:
    """Renderiza um tile de botão com relevo 3D suave e ícone vetorial opcional."""
    S = size * _SS
    text_color = theme.text
    sub_color = theme.accent
    icon_color = theme.accent

    if state == "active":
        img = _bevel_keycap(size, "#E11D48", "#881337", "#FB7185", "#4C0519").copy()
        sub_color = "#FEF08A"
        icon_color = "#FFFFFF"
    elif state == "warning":
        img = _bevel_keycap(size, "#D97706", "#78350F", "#FCD34D", "#451A03").copy()
        sub_color = "#FEF08A"
        icon_color = "#FFFFFF"
    elif state == "disabled":
        img = _bevel_keycap(size, "#161B24", "#0D1017", "#1F2937", "#06080C").copy()
        text_color = "#64748B"
        sub_color = "#475569"
        icon_color = "#475569"
    elif icon == "discord":
        img = _bevel_keycap(size, "#5865F2", "#3C45A5", "#7983F5", "#23275E").copy()
        sub_color = "#E0E7FF"
        icon_color = "#FFFFFF"
    else:
        img = _bevel_keycap(size).copy()

    draw = ImageDraw.Draw(img)

    if icon:
        _draw_button_icon(draw, icon, S // 2, 31 * _SS, icon_color, state)
        f_title = _font((13 if len(title) <= 8 else 11) * _SS, bold=True)
        tb = draw.textbbox((0, 0), title, font=f_title)
        tw, th = tb[2] - tb[0], tb[3] - tb[1]
        ty = 56 * _SS if subtitle else 62 * _SS
        draw.text(((S - tw) // 2 - tb[0], ty - tb[1]), title, fill=text_color, font=f_title)
        if subtitle:
            f_sub = _font(10 * _SS, bold=True)
            sb = draw.textbbox((0, 0), subtitle, font=f_sub)
            sw = sb[2] - sb[0]
            draw.text(((S - sw) // 2 - sb[0], 74 * _SS - sb[1]), subtitle, fill=sub_color, font=f_sub)
    else:
        f_title = _font((17 if len(title) <= 7 else 14) * _SS, bold=True)
        bbox = draw.textbbox((0, 0), title, font=f_title)
        tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
        ty = (S - th) // 2 - (8 * _SS if subtitle else 2 * _SS)
        draw.text(((S - tw) // 2 - bbox[0], ty - bbox[1]), title, fill=text_color, font=f_title)

        if subtitle:
            f_sub = _font(11 * _SS, bold=True)
            sb = draw.textbbox((0, 0), subtitle, font=f_sub)
            sw = sb[2] - sb[0]
            draw.text(
                ((S - sw) // 2 - sb[0], ty + th + 6 * _SS),
                subtitle,
                fill=sub_color,
                font=f_sub,
            )

    return _downsample(img, size)
