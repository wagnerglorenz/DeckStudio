//! Renderizador vetorial 3x (288x288 -> 96x96 Lanczos3) com tiny-skia e ab_glyph.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use base64::Engine as _;
use deck_core::{ColorRules, ThemeConfig};
use image::imageops::FilterType;
use image::{DynamicImage, RgbImage};
use std::sync::OnceLock;
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, PathBuilder, Pixmap,
    Point, SpreadMode, Stroke, Transform,
};

const SS: f32 = 3.0;
const CANVAS_SZ: u32 = 288; // 96 * 3
const TILE_SZ: u32 = 96;

static BOLD_FONT_BYTES: OnceLock<Vec<u8>> = OnceLock::new();

fn font_ref() -> Option<FontRef<'static>> {
    let bytes = BOLD_FONT_BYTES.get_or_init(|| {
        for path in [
            r"C:\Windows\Fonts\segoeuib.ttf",
            r"C:\Windows\Fonts\arialbd.ttf",
            r"C:\Windows\Fonts\segoeui.ttf",
        ] {
            if let Ok(b) = std::fs::read(path) {
                return b;
            }
        }
        Vec::new()
    });
    if bytes.is_empty() {
        None
    } else {
        FontRef::try_from_slice(bytes).ok()
    }
}

fn hex_rgba(hex: &str, a: u8) -> Color {
    let h = hex.trim().trim_start_matches('#');
    let (r, g, b) = if h.len() >= 6 {
        (
            u8::from_str_radix(&h[0..2], 16).unwrap_or(255),
            u8::from_str_radix(&h[2..4], 16).unwrap_or(255),
            u8::from_str_radix(&h[4..6], 16).unwrap_or(255),
        )
    } else {
        (255, 255, 255)
    };
    Color::from_rgba8(r, g, b, a)
}

fn fill_solid(pm: &mut Pixmap, hex: &str) {
    pm.fill(hex_rgba(hex, 255));
}

fn paint_hex(hex: &str, a: u8) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(hex_rgba(hex, a));
    p.anti_alias = true;
    p
}

fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w * 0.5).min(h * 0.5);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

#[derive(Debug, Clone, Default)]
pub struct TileColorOverride<'a> {
    pub bg: Option<&'a str>,
    pub border: Option<&'a str>,
    pub text: Option<&'a str>,
    pub icon: Option<&'a str>,
}

fn draw_bevel_keycap(pm: &mut Pixmap, top_hex: &str, bot_hex: &str, hi_hex: &str, sh_hex: &str) {
    let grad = LinearGradient::new(
        Point::from_xy(0.0, 0.0),
        Point::from_xy(0.0, CANVAS_SZ as f32),
        vec![
            GradientStop::new(0.0, hex_rgba(top_hex, 255)),
            GradientStop::new(1.0, hex_rgba(bot_hex, 255)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    );
    if let (Some(shader), Some(rect)) = (
        grad,
        tiny_skia::Rect::from_xywh(0.0, 0.0, CANVAS_SZ as f32, CANVAS_SZ as f32),
    ) {
        let p = Paint {
            shader,
            ..Default::default()
        };
        pm.fill_rect(rect, &p, Transform::identity(), None);
    } else {
        fill_solid(pm, bot_hex);
    }

    // Brilho sutil de relevo no topo e sombra na base
    let mut pb = PathBuilder::new();
    pb.move_to(4.0 * SS, 1.5 * SS);
    pb.line_to((TILE_SZ as f32 - 4.0) * SS, 1.5 * SS);
    if let Some(path) = pb.finish() {
        let stroke = Stroke { width: 2.0 * SS, ..Default::default() };
        pm.stroke_path(&path, &paint_hex(hi_hex, 255), &stroke, Transform::identity(), None);
    }
    let mut pb2 = PathBuilder::new();
    pb2.move_to(4.0 * SS, (TILE_SZ as f32 - 1.5) * SS);
    pb2.line_to((TILE_SZ as f32 - 4.0) * SS, (TILE_SZ as f32 - 1.5) * SS);
    if let Some(path) = pb2.finish() {
        let stroke = Stroke { width: 2.0 * SS, ..Default::default() };
        pm.stroke_path(&path, &paint_hex(sh_hex, 255), &stroke, Transform::identity(), None);
    }
}

/// Superfície OLED de alto contraste (sem penumbra cinza) respeitando o tema ativo e a zona segura da lente acrílica.
fn draw_themed_surface(
    pm: &mut Pixmap,
    theme: &ThemeConfig,
    bg_override: Option<&str>,
    border_override: Option<&str>,
    accent_override: Option<&str>,
) {
    let bg = bg_override.unwrap_or(&theme.background);
    let border = border_override.unwrap_or(&theme.card_border);
    let accent = accent_override.unwrap_or(&theme.accent);
    fill_solid(pm, bg);

    let m = 4.5 * SS;
    let w = CANVAS_SZ as f32 - m * 2.0;
    let h = CANVAS_SZ as f32 - m * 2.0;

    match theme.surface_style.as_str() {
        "cyber_hud" => {
            // Cantos chanfrados estilo HUD Cyberpunk + barra superior elétrica
            let tick = 14.0 * SS;
            let mut pb = PathBuilder::new();
            // Top-left
            pb.move_to(m, m + tick);
            pb.line_to(m, m);
            pb.line_to(m + tick, m);
            // Top-right (chanfro)
            pb.move_to(m + w - tick, m);
            pb.line_to(m + w - 5.0 * SS, m);
            pb.line_to(m + w, m + 5.0 * SS);
            pb.line_to(m + w, m + tick);
            // Bottom-right
            pb.move_to(m + w, m + h - tick);
            pb.line_to(m + w, m + h);
            pb.line_to(m + w - tick, m + h);
            // Bottom-left
            pb.move_to(m + tick, m + h);
            pb.line_to(m, m + h);
            pb.line_to(m, m + h - tick);
            if let Some(p) = pb.finish() {
                let stroke = Stroke { width: 2.2 * SS, line_cap: LineCap::Square, ..Default::default() };
                pm.stroke_path(&p, &paint_hex(border, 255), &stroke, Transform::identity(), None);
            }
            if let Some(bar) = rounded_rect_path(m + 18.0 * SS, m - 1.0 * SS, w - 36.0 * SS, 2.5 * SS, 1.0 * SS) {
                pm.fill_path(&bar, &paint_hex(accent, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "matrix_tty" => {
            // Scanlines sutis de terminal + colchetes verdes
            let mut y = 6.0 * SS;
            while y < CANVAS_SZ as f32 - 6.0 * SS {
                if let Some(rect) = tiny_skia::Rect::from_xywh(m, y, w, 1.0 * SS) {
                    pm.fill_rect(rect, &paint_hex("#042208", 160), Transform::identity(), None);
                }
                y += 5.0 * SS;
            }
            if let Some(frame) = rounded_rect_path(m, m, w, h, 4.0 * SS) {
                let stroke = Stroke { width: 1.5 * SS, ..Default::default() };
                pm.stroke_path(&frame, &paint_hex(border, 210), &stroke, Transform::identity(), None);
            }
            // Pequenos colchetes no topo
            for (x0, x1) in [(m + 2.0 * SS, m + 12.0 * SS), (m + w - 12.0 * SS, m + w - 2.0 * SS)] {
                let mut pb = PathBuilder::new();
                pb.move_to(x0, m);
                pb.line_to(x1, m);
                if let Some(p) = pb.finish() {
                    let stroke = Stroke { width: 2.4 * SS, ..Default::default() };
                    pm.stroke_path(&p, &paint_hex(accent, 255), &stroke, Transform::identity(), None);
                }
            }
        }
        "obsidian_gold" => {
            // Moldura fina titânio sobre preto absoluto + filete superior ouro/acento
            if let Some(frame) = rounded_rect_path(m, m, w, h, 8.0 * SS) {
                let stroke = Stroke { width: 1.6 * SS, ..Default::default() };
                pm.stroke_path(&frame, &paint_hex(border, 255), &stroke, Transform::identity(), None);
            }
            if let Some(bar) = rounded_rect_path(m + 20.0 * SS, m - 0.5 * SS, w - 40.0 * SS, 2.5 * SS, 1.2 * SS) {
                pm.fill_path(&bar, &paint_hex(accent, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        _ => {
            // "oled_frame" / "sparkline" — moldura nítida sem véu/penumbra
            if let Some(frame) = rounded_rect_path(m, m, w, h, 9.0 * SS) {
                let stroke = Stroke { width: 1.8 * SS, ..Default::default() };
                pm.stroke_path(&frame, &paint_hex(border, 255), &stroke, Transform::identity(), None);
            }
            if let Some(bar) = rounded_rect_path(m + 22.0 * SS, m, w - 44.0 * SS, 2.0 * SS, 1.0 * SS) {
                pm.fill_path(&bar, &paint_hex(accent, 200), FillRule::Winding, Transform::identity(), None);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_arc_deg(
    pm: &mut Pixmap,
    cx: f32,
    cy: f32,
    radius: f32,
    start_deg: f32,
    sweep_deg: f32,
    width: f32,
    color_hex: &str,
    round_cap: bool,
) {
    if sweep_deg.abs() < 0.5 {
        return;
    }
    let steps = ((sweep_deg.abs() / 4.0).ceil() as usize).max(6);
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let ang = (start_deg + sweep_deg * t).to_radians();
        let x = cx + radius * ang.cos();
        let y = cy + radius * ang.sin();
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    if let Some(path) = pb.finish() {
        let stroke = Stroke {
            width,
            line_cap: if round_cap { LineCap::Round } else { LineCap::Butt },
            line_join: LineJoin::Round,
            ..Default::default()
        };
        pm.stroke_path(&path, &paint_hex(color_hex, 255), &stroke, Transform::identity(), None);
    }
}

fn measure_text(text: &str, px: f32) -> (f32, f32) {
    let Some(font) = font_ref() else {
        return (text.len() as f32 * px * 0.55, px);
    };
    let scaled = font.as_scaled(PxScale::from(px));
    let mut w = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        if let Some(p) = prev {
            w += scaled.kern(p, gid);
        }
        w += scaled.h_advance(gid);
        prev = Some(gid);
    }
    (w, scaled.ascent() - scaled.descent())
}

fn draw_text(pm: &mut Pixmap, text: &str, x: f32, top_y: f32, px: f32, color_hex: &str) {
    let Some(font) = font_ref() else { return };
    let scaled = font.as_scaled(PxScale::from(px));
    let col = hex_rgba(color_hex, 255);
    let cr = col.red();
    let cg = col.green();
    let cb = col.blue();

    let pw = pm.width() as i32;
    let ph = pm.height() as i32;
    let data = pm.pixels_mut();

    let mut cursor_x = x;
    let baseline_y = top_y + scaled.ascent();
    let mut prev = None;

    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        if let Some(p) = prev {
            cursor_x += scaled.kern(p, gid);
        }
        let glyph = gid.with_scale_and_position(PxScale::from(px), ab_glyph::point(cursor_x, baseline_y));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let px_x = bounds.min.x as i32 + gx as i32;
                let px_y = bounds.min.y as i32 + gy as i32;
                if px_x >= 0 && px_x < pw && px_y >= 0 && px_y < ph && cov > 0.01 {
                    let idx = (px_y * pw + px_x) as usize;
                    let dst = data[idx];
                    let dr = dst.red() as f32 / 255.0;
                    let dg = dst.green() as f32 / 255.0;
                    let db = dst.blue() as f32 / 255.0;
                    let nr = ((cr * cov + dr * (1.0 - cov)) * 255.0).round().clamp(0.0, 255.0) as u8;
                    let ng = ((cg * cov + dg * (1.0 - cov)) * 255.0).round().clamp(0.0, 255.0) as u8;
                    let nb = ((cb * cov + db * (1.0 - cov)) * 255.0).round().clamp(0.0, 255.0) as u8;
                    if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(nr, ng, nb, 255) {
                        data[idx] = c;
                    }
                }
            });
        }
        cursor_x += scaled.h_advance(gid);
        prev = Some(gid);
    }
}

fn draw_text_centered(pm: &mut Pixmap, text: &str, cx: f32, top_y: f32, px: f32, color_hex: &str) {
    let (w, _) = measure_text(text, px);
    draw_text(pm, text, cx - w * 0.5, top_y, px, color_hex);
}

fn draw_text_centered_shadow(pm: &mut Pixmap, text: &str, cx: f32, top_y: f32, px: f32, color_hex: &str) {
    let (w, _) = measure_text(text, px);
    let x = cx - w * 0.5;
    draw_text(pm, text, x, top_y + 1.5 * SS, px, "#000000");
    draw_text(pm, text, x, top_y, px, color_hex);
}

fn downsample(pm: Pixmap) -> RgbImage {
    let w = pm.width();
    let h = pm.height();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in pm.pixels() {
        rgb.push(px.red());
        rgb.push(px.green());
        rgb.push(px.blue());
    }
    let full = RgbImage::from_raw(w, h, rgb).unwrap_or_else(|| RgbImage::new(w, h));
    DynamicImage::ImageRgb8(full)
        .resize_exact(TILE_SZ, TILE_SZ, FilterType::Lanczos3)
        .to_rgb8()
}

pub fn decode_custom_image(b64: &str) -> Option<RgbImage> {
    let clean = b64.split(',').next_back().unwrap_or(b64);
    let bytes = base64::engine::general_purpose::STANDARD.decode(clean.trim()).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(img.resize_exact(TILE_SZ, TILE_SZ, FilterType::Lanczos3).to_rgb8())
}

pub fn group_accent(group: Option<&str>, theme: &ThemeConfig) -> String {
    match group {
        Some("cpu") => "#22D3EE".into(),
        Some("gpu") | Some("net_down") => "#22C55E".into(),
        Some("ram") => "#EAB308".into(),
        Some("vram") => "#A855F7".into(),
        Some("net") => "#38BDF8".into(),
        Some("net_up") => "#F59E0B".into(),
        Some("fps") => "#F97316".into(),
        _ => theme.chart.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_sensor_tile(
    title: &str,
    value_text: &str,
    unit: &str,
    history: &[f64],
    vmin: Option<f64>,
    vmax: Option<f64>,
    theme: &ThemeConfig,
    rules: &ColorRules,
    raw_value: Option<f64>,
    group: Option<&str>,
    card_style: Option<&str>,
    is_temp: bool,
) -> RgbImage {
    let accent = if is_temp && theme.temp_colors {
        raw_value.map(|v| rules.temp_color(v).to_string()).unwrap_or_else(|| group_accent(group, theme))
    } else if matches!(group, Some("ram") | Some("vram")) && theme.temp_colors {
        raw_value.map(|v| rules.usage_color(v).to_string()).unwrap_or_else(|| group_accent(group, theme))
    } else {
        group_accent(group, theme)
    };

    let pct = match (raw_value, vmin, vmax) {
        (Some(v), Some(mn), Some(mx)) if mx > mn => Some(((v - mn) / (mx - mn) * 100.0).clamp(0.0, 100.0) as f32),
        _ => None,
    };

    if theme.style == "ajazz" {
        return render_ajazz_sparkline(title, value_text, unit, history, vmin, vmax, theme, &accent, is_temp);
    }

    if card_style == Some("ring") {
        if let Some(p) = pct {
            return render_llm_ring(title, value_text, unit, p, &accent, theme);
        }
    }
    render_llm_card(title, value_text, unit, pct, &accent, is_temp, theme)
}

fn render_llm_ring(
    title: &str,
    value_text: &str,
    unit: &str,
    pct: f32,
    accent: &str,
    theme: &ThemeConfig,
) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    let ring_accent = match theme.style.as_str() {
        "matrix" => theme.accent.as_str(),
        "cyberpunk" => accent,
        _ => accent,
    };
    draw_themed_surface(&mut pm, theme, None, None, Some(ring_accent));

    let c = (CANVAS_SZ as f32) * 0.5;
    let r = (TILE_SZ as f32 * 0.5 - 15.0) * SS;
    let track_col = match theme.style.as_str() {
        "matrix" => "#062E0D",
        "cyberpunk" => "#1B1B2F",
        "obsidian_pro" => "#18181B",
        _ => "#182235",
    };
    draw_arc_deg(&mut pm, c, c, r, 0.0, 360.0, 6.5 * SS, track_col, false);
    if pct > 0.5 {
        let sweep = (pct / 100.0 * 360.0).max(4.0);
        let round = theme.style != "cyberpunk" && theme.style != "matrix";
        draw_arc_deg(&mut pm, c, c, r, -90.0, sweep, 6.5 * SS, ring_accent, round);
    }

    let val_str = format!("{}{}", value_text.trim(), unit.trim());
    let val_px = (if val_str.len() <= 4 { 21.0 } else { 17.0 }) * SS;
    draw_text_centered_shadow(&mut pm, &val_str, c, 32.0 * SS, val_px, &theme.text);

    let lbl: String = if theme.style == "matrix" {
        format!("[{}]", title.to_uppercase().chars().take(7).collect::<String>())
    } else {
        title.to_uppercase().chars().take(9).collect()
    };
    draw_text_centered(&mut pm, &lbl, c, 56.0 * SS, 10.5 * SS, &theme.muted);
    downsample(pm)
}

fn render_llm_card(
    title: &str,
    value_text: &str,
    unit: &str,
    pct: Option<f32>,
    accent: &str,
    is_temp: bool,
    theme: &ThemeConfig,
) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    let card_accent = if theme.style == "matrix" && !is_temp {
        theme.accent.as_str()
    } else {
        accent
    };
    draw_themed_surface(&mut pm, theme, None, None, Some(card_accent));

    let c = (CANVAS_SZ as f32) * 0.5;
    let lbl: String = if theme.style == "matrix" {
        format!("[{}]", title.to_uppercase().chars().take(8).collect::<String>())
    } else {
        title.to_uppercase().chars().take(10).collect()
    };
    draw_text_centered(&mut pm, &lbl, c, 12.5 * SS, 11.5 * SS, &theme.muted);

    let val_clean = value_text.trim();
    let v_sz = match val_clean.len() {
        0..=2 => 30.0,
        3 => 26.0,
        4 => 21.0,
        _ => 17.0,
    } * SS;
    let u_clean = if is_temp && (unit == "°" || unit == "°C") { "°C" } else { unit.trim() };
    let u_sz = (if u_clean.len() <= 2 { 14.0 } else { 10.5 }) * SS;

    let (vw, vh) = measure_text(val_clean, v_sz);
    let (uw, uh) = if u_clean.is_empty() { (0.0, 0.0) } else { measure_text(u_clean, u_sz) };
    let gap = if uw > 0.0 { 3.0 * SS } else { 0.0 };
    let total_w = vw + gap + uw;
    let vx = (c - total_w * 0.5).max(8.0 * SS);
    let vy = 34.0 * SS;

    let val_col = if is_temp { accent } else { &theme.text };
    draw_text(&mut pm, val_clean, vx, vy + 1.5 * SS, v_sz, "#000000");
    draw_text(&mut pm, val_clean, vx, vy, v_sz, val_col);
    if !u_clean.is_empty() {
        draw_text(&mut pm, u_clean, vx + vw + gap, vy + vh - uh - 2.0 * SS, u_sz, card_accent);
    }

    if let Some(p) = pct {
        let bar_l = 12.0 * SS;
        let bar_w = (TILE_SZ as f32 - 24.0) * SS;
        let bar_y = (TILE_SZ as f32 - 17.0) * SS;
        let bar_h = 6.0 * SS;
        let track_col = match theme.style.as_str() {
            "matrix" => "#062E0D",
            "cyberpunk" => "#1B1B2F",
            "obsidian_pro" => "#18181B",
            _ => "#182235",
        };
        if theme.style == "cyberpunk" || theme.style == "matrix" {
            // Barra segmentada em 10 blocos estilo HUD / Terminal
            let segs = 10usize;
            let seg_gap = 2.0 * SS;
            let seg_w = (bar_w - seg_gap * (segs as f32 - 1.0)) / segs as f32;
            let active_segs = ((p / 100.0) * segs as f32).round() as usize;
            for i in 0..segs {
                let sx = bar_l + i as f32 * (seg_w + seg_gap);
                let col = if i < active_segs { card_accent } else { track_col };
                if let Some(r) = rounded_rect_path(sx, bar_y, seg_w, bar_h, 1.0 * SS) {
                    pm.fill_path(&r, &paint_hex(col, 255), FillRule::Winding, Transform::identity(), None);
                }
            }
        } else {
            if let Some(track) = rounded_rect_path(bar_l, bar_y, bar_w, bar_h, 3.0 * SS) {
                pm.fill_path(&track, &paint_hex(track_col, 255), FillRule::Winding, Transform::identity(), None);
            }
            let fw = bar_w * (p / 100.0);
            if fw > 3.0 * SS {
                if let Some(fill) = rounded_rect_path(bar_l, bar_y, fw, bar_h, 3.0 * SS) {
                    pm.fill_path(&fill, &paint_hex(card_accent, 255), FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }

    downsample(pm)
}

#[allow(clippy::too_many_arguments)]
fn render_ajazz_sparkline(
    title: &str,
    value_text: &str,
    unit: &str,
    history: &[f64],
    vmin: Option<f64>,
    vmax: Option<f64>,
    theme: &ThemeConfig,
    accent: &str,
    is_temp: bool,
) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    fill_solid(&mut pm, &theme.background);

    let line_col = if is_temp && theme.temp_colors { accent } else { &theme.chart };
    if let (Some(mn), Some(mx)) = (vmin, vmax) {
        let left = 4.0 * SS;
        let right = (TILE_SZ as f32 - 4.0) * SS;
        let top = 34.0 * SS;
        let bottom = (TILE_SZ as f32 - 6.0) * SS;
        let span = if mx > mn { mx - mn } else { 1.0 };

        let pts: Vec<f64> = if history.is_empty() {
            vec![mn, mn]
        } else if history.len() == 1 {
            vec![history[0], history[0]]
        } else {
            history.to_vec()
        };

        let mut coords = Vec::with_capacity(pts.len());
        for (i, &v) in pts.iter().enumerate() {
            let x = left + (right - left) * (i as f32 / (pts.len() - 1).max(1) as f32);
            let norm = ((v - mn) / span).clamp(0.0, 1.0) as f32;
            let y = bottom - norm * (bottom - top);
            coords.push((x, y));
        }

        // Área preenchida com degradê 50% -> 0%
        let mut poly = PathBuilder::new();
        poly.move_to(left, bottom);
        for &(x, y) in &coords {
            poly.line_to(x, y);
        }
        poly.line_to(right, bottom);
        poly.close();
        if let Some(poly_path) = poly.finish() {
            if let Some(grad) = LinearGradient::new(
                Point::from_xy(0.0, top),
                Point::from_xy(0.0, bottom),
                vec![
                    GradientStop::new(0.0, hex_rgba(line_col, 130)),
                    GradientStop::new(1.0, hex_rgba(line_col, 0)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                let p = Paint {
                    shader: grad,
                    anti_alias: true,
                    ..Default::default()
                };
                pm.fill_path(&poly_path, &p, FillRule::Winding, Transform::identity(), None);
            }
        }

        // Linha do gráfico
        let mut line = PathBuilder::new();
        for (i, &(x, y)) in coords.iter().enumerate() {
            if i == 0 { line.move_to(x, y); } else { line.line_to(x, y); }
        }
        if let Some(lp) = line.finish() {
            let stroke = Stroke { width: 3.0 * SS, line_join: LineJoin::Round, ..Default::default() };
            pm.stroke_path(&lp, &paint_hex(line_col, 255), &stroke, Transform::identity(), None);
        }
    }

    let c = CANVAS_SZ as f32 * 0.5;
    draw_text_centered(&mut pm, title, c, 10.0 * SS, 13.5 * SS, &theme.text);

    let val_clean = value_text.trim();
    let v_sz = match val_clean.len() {
        0..=2 => 33.0,
        3 => 29.0,
        4 => 24.0,
        _ => 19.0,
    } * SS;
    let u_clean = unit.trim();
    let u_sz = (if u_clean.len() <= 2 { 15.0 } else { 11.0 }) * SS;
    let (vw, _) = measure_text(val_clean, v_sz);
    let (uw, _) = if u_clean.is_empty() { (0.0, 0.0) } else { measure_text(u_clean, u_sz) };
    let total_w = vw + if uw > 0.0 { uw + 2.0 * SS } else { 0.0 };
    let vx = (c - total_w * 0.5).max(4.0 * SS);
    let val_col = if is_temp && theme.temp_colors { accent } else { &theme.text };
    draw_text(&mut pm, val_clean, vx, 32.0 * SS, v_sz, val_col);
    if !u_clean.is_empty() {
        draw_text(&mut pm, u_clean, vx + vw + 2.0 * SS, 32.0 * SS, u_sz, val_col);
    }

    downsample(pm)
}

pub fn render_nav_circle_tile(direction: &str, theme: &ThemeConfig) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    draw_themed_surface(&mut pm, theme, None, None, Some(&theme.accent));
    let c = CANVAS_SZ as f32 * 0.5;
    let ring_col = if theme.style == "ajazz" { "#FFFFFF" } else { &theme.accent };
    draw_arc_deg(&mut pm, c, c, (TILE_SZ as f32 * 0.5 - 16.0) * SS, 0.0, 360.0, 3.5 * SS, ring_col, false);

    let dx = 9.0 * SS;
    let dy = 15.0 * SS;
    let mut pb = PathBuilder::new();
    if direction == "prev" {
        pb.move_to(c + dx * 0.5, c - dy);
        pb.line_to(c - dx, c);
        pb.line_to(c + dx * 0.5, c + dy);
    } else {
        pb.move_to(c - dx * 0.5, c - dy);
        pb.line_to(c + dx, c);
        pb.line_to(c - dx * 0.5, c + dy);
    }
    if let Some(p) = pb.finish() {
        let stroke = Stroke { width: 4.5 * SS, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
        pm.stroke_path(&p, &paint_hex("#FFFFFF", 255), &stroke, Transform::identity(), None);
    }
    downsample(pm)
}

pub fn render_obs_logo_tile_themed(theme: &ThemeConfig) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    draw_themed_surface(&mut pm, theme, None, Some("#38BDF8"), Some("#38BDF8"));

    let cx = CANVAS_SZ as f32 * 0.5;
    let cy = 38.0 * SS;
    let r_out = 24.0 * SS;

    // Disco preto com anel cinza claro
    if let Some(circle) = PathBuilder::from_circle(cx, cy, r_out) {
        pm.fill_path(&circle, &paint_hex("#0B0E14", 255), FillRule::Winding, Transform::identity(), None);
        let stroke = Stroke { width: 2.5 * SS, ..Default::default() };
        pm.stroke_path(&circle, &paint_hex("#CBD5E1", 255), &stroke, Transform::identity(), None);
    }
    // 3 hélices características do OBS
    let blade_r = 12.0 * SS;
    let orbit = 9.0 * SS;
    for base_deg in [270.0f32, 30.0, 150.0] {
        let rad = base_deg.to_radians();
        let bx = cx + orbit * rad.cos();
        let by = cy + orbit * rad.sin();
        draw_arc_deg(&mut pm, bx, by, blade_r, base_deg - 10.0, 200.0, 5.0 * SS, "#FFFFFF", true);
    }
    draw_text_centered(&mut pm, "OBS", cx, 69.0 * SS, 14.0 * SS, "#E2E8F0");
    downsample(pm)
}

pub fn render_clock_tile(time_str: &str, date_str: &str, theme: &ThemeConfig) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    fill_solid(&mut pm, "#000000");
    let cx = CANVAS_SZ as f32 * 0.5 + 5.0 * SS;
    draw_text_centered(&mut pm, time_str, cx, 26.0 * SS, 22.0 * SS, "#FFFFFF");
    draw_text_centered(&mut pm, date_str, cx, 56.0 * SS, 13.5 * SS, &theme.accent);
    downsample(pm)
}

pub fn render_obs_status_tile(status: &str, scene_short: &str, theme: &ThemeConfig) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    fill_solid(&mut pm, "#000000");
    let cx = CANVAS_SZ as f32 * 0.5 + 5.0 * SS;

    let col = match status {
        "REC" => "#EF4444",
        "PAUSADO" => "#EAB308",
        "PRONTO" => "#22C55E",
        _ => "#64748B",
    };

    if let Some(dot) = PathBuilder::from_circle(cx - 20.0 * SS, 20.0 * SS, 6.0 * SS) {
        pm.fill_path(&dot, &paint_hex(col, 255), FillRule::Winding, Transform::identity(), None);
    }
    draw_text(&mut pm, "OBS", cx - 10.0 * SS, 13.0 * SS, 13.5 * SS, &theme.muted);
    draw_text_centered(&mut pm, status, cx, 39.0 * SS, 15.5 * SS, col);
    let sc: String = scene_short.chars().take(10).collect();
    draw_text_centered(&mut pm, &sc, cx, 64.0 * SS, 11.5 * SS, "#CBD5E1");
    downsample(pm)
}

pub fn render_weather_tile(
    temp_c: Option<f64>,
    tmin: Option<f64>,
    tmax: Option<f64>,
    city: &str,
    weather_code: i32,
    theme: &ThemeConfig,
) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    fill_solid(&mut pm, "#000000");
    let cx = CANVAS_SZ as f32 * 0.5 + 5.0 * SS;
    let ix = cx - 26.0 * SS;

    if weather_code <= 1 {
        if let Some(sun) = PathBuilder::from_circle(ix + 12.0 * SS, 22.0 * SS, 10.0 * SS) {
            pm.fill_path(&sun, &paint_hex("#F59E0B", 255), FillRule::Winding, Transform::identity(), None);
        }
    } else if weather_code <= 3 {
        if let Some(sun) = PathBuilder::from_circle(ix + 8.0 * SS, 19.0 * SS, 7.5 * SS) {
            pm.fill_path(&sun, &paint_hex("#F59E0B", 255), FillRule::Winding, Transform::identity(), None);
        }
        if let Some(cloud) = rounded_rect_path(ix + 6.0 * SS, 19.0 * SS, 21.0 * SS, 11.0 * SS, 5.0 * SS) {
            pm.fill_path(&cloud, &paint_hex("#CBD5E1", 255), FillRule::Winding, Transform::identity(), None);
        }
    } else {
        if let Some(cloud) = rounded_rect_path(ix + 3.0 * SS, 16.0 * SS, 23.0 * SS, 12.0 * SS, 5.0 * SS) {
            pm.fill_path(&cloud, &paint_hex("#94A3B8", 255), FillRule::Winding, Transform::identity(), None);
        }
        if weather_code >= 60 {
            let drop_col = if weather_code >= 95 { "#F59E0B" } else { "#38BDF8" };
            for dx in [8.0, 15.0, 22.0] {
                let mut pb = PathBuilder::new();
                pb.move_to(ix + dx * SS, 29.0 * SS);
                pb.line_to(ix + (dx - 2.0) * SS, 35.0 * SS);
                if let Some(p) = pb.finish() {
                    let stroke = Stroke { width: 2.0 * SS, line_cap: LineCap::Round, ..Default::default() };
                    pm.stroke_path(&p, &paint_hex(drop_col, 255), &stroke, Transform::identity(), None);
                }
            }
        }
    }

    let t_str = temp_c.map(|v| format!("{}°", v.round() as i64)).unwrap_or_else(|| "--°".into());
    draw_text(&mut pm, &t_str, cx + 4.0 * SS, 13.0 * SS, 20.5 * SS, "#FFFFFF");

    let mm = match (tmin, tmax) {
        (Some(mn), Some(mx)) => format!("{}°/{}°", mn.round() as i64, mx.round() as i64),
        _ => "--/--".into(),
    };
    draw_text_centered(&mut pm, &mm, cx, 46.0 * SS, 12.5 * SS, "#38BDF8");

    let clbl: String = city.to_uppercase().chars().take(11).collect();
    draw_text_centered(&mut pm, &clbl, cx, 68.0 * SS, 9.5 * SS, &theme.muted);
    downsample(pm)
}

pub fn render_compact_health_tile(
    label: &str,
    pct: Option<f64>,
    temp_c: Option<f64>,
    accent: &str,
    rules: &ColorRules,
    theme: &ThemeConfig,
) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    draw_themed_surface(&mut pm, theme, None, None, Some(accent));
    let cx = CANVAS_SZ as f32 * 0.5;
    draw_text_centered(&mut pm, label, cx, 11.0 * SS, 11.5 * SS, &theme.muted);
    let p_str = pct.map(|v| format!("{}%", v.round() as i64)).unwrap_or_else(|| "--%".into());
    draw_text_centered_shadow(&mut pm, &p_str, cx, 28.0 * SS, 20.5 * SS, accent);
    let t_str = temp_c.map(|v| format!("{}°C", v.round() as i64)).unwrap_or_else(|| "--°C".into());
    let t_col = temp_c.map(|v| rules.temp_color(v)).unwrap_or(&theme.text);
    draw_text_centered_shadow(&mut pm, &t_str, cx, 55.0 * SS, 19.5 * SS, t_col);
    downsample(pm)
}

fn draw_button_icon(pm: &mut Pixmap, icon: &str, cx: f32, cy: f32, color: &str, state: &str) {
    let stroke = Stroke { width: 3.0 * SS, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
    match icon {
        "record" => {
            draw_arc_deg(pm, cx, cy, 16.0 * SS, 0.0, 360.0, 3.0 * SS, color, false);
            if state == "active" {
                if let Some(sq) = rounded_rect_path(cx - 7.0 * SS, cy - 7.0 * SS, 14.0 * SS, 14.0 * SS, 2.0 * SS) {
                    pm.fill_path(&sq, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
                }
            } else if let Some(dot) = PathBuilder::from_circle(cx, cy, 9.0 * SS) {
                let c = if state == "idle" { "#EF4444" } else if state == "warning" { "#FEF08A" } else { color };
                pm.fill_path(&dot, &paint_hex(c, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "pause" => {
            if state == "warning" {
                let mut pb = PathBuilder::new();
                pb.move_to(cx - 7.0 * SS, cy - 12.0 * SS);
                pb.line_to(cx + 12.0 * SS, cy);
                pb.line_to(cx - 7.0 * SS, cy + 12.0 * SS);
                pb.close();
                if let Some(p) = pb.finish() {
                    pm.fill_path(&p, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
                }
            } else {
                for ox in [-10.0, 3.0] {
                    if let Some(b) = rounded_rect_path(cx + ox * SS, cy - 12.0 * SS, 7.0 * SS, 24.0 * SS, 2.0 * SS) {
                        pm.fill_path(&b, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
                    }
                }
            }
        }
        "audio" => {
            if let Some(b) = rounded_rect_path(cx - 15.0 * SS, cy - 6.0 * SS, 8.0 * SS, 12.0 * SS, 1.5 * SS) {
                pm.fill_path(&b, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 8.0 * SS, cy - 6.0 * SS);
            pb.line_to(cx + 2.0 * SS, cy - 13.0 * SS);
            pb.line_to(cx + 2.0 * SS, cy + 13.0 * SS);
            pb.line_to(cx - 8.0 * SS, cy + 6.0 * SS);
            pb.close();
            if let Some(cone) = pb.finish() {
                pm.fill_path(&cone, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            if state == "active" {
                let mut xp = PathBuilder::new();
                xp.move_to(cx + 6.0 * SS, cy - 7.0 * SS);
                xp.line_to(cx + 16.0 * SS, cy + 7.0 * SS);
                xp.move_to(cx + 16.0 * SS, cy - 7.0 * SS);
                xp.line_to(cx + 6.0 * SS, cy + 7.0 * SS);
                if let Some(p) = xp.finish() {
                    pm.stroke_path(&p, &paint_hex("#FEF08A", 255), &stroke, Transform::identity(), None);
                }
            } else {
                draw_arc_deg(pm, cx + 4.0 * SS, cy, 8.0 * SS, -50.0, 100.0, 2.5 * SS, color, true);
                draw_arc_deg(pm, cx + 4.0 * SS, cy, 14.0 * SS, -55.0, 110.0, 2.5 * SS, color, true);
            }
        }
        "discord" => {
            if let Some(head) = rounded_rect_path(cx - 17.0 * SS, cy - 11.0 * SS, 34.0 * SS, 21.0 * SS, 9.0 * SS) {
                pm.fill_path(&head, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
            }
            for ex in [-6.5, 6.5] {
                if let Some(eye) = PathBuilder::from_circle(cx + ex * SS, cy - 0.5 * SS, 3.5 * SS) {
                    pm.fill_path(&eye, &paint_hex("#5865F2", 255), FillRule::Winding, Transform::identity(), None);
                }
            }
        }
        "studio" => {
            if let Some(r) = rounded_rect_path(cx - 18.0 * SS, cy - 12.0 * SS, 36.0 * SS, 24.0 * SS, 3.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            if let Some(m) = rounded_rect_path(cx - 14.0 * SS, cy - 8.0 * SS, 17.0 * SS, 16.0 * SS, 2.0 * SS) {
                pm.fill_path(&m, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(p) = rounded_rect_path(cx + 6.0 * SS, cy - 8.0 * SS, 8.0 * SS, 8.0 * SS, 1.5 * SS) {
                pm.stroke_path(&p, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "game_cam" => {
            if let Some(r) = rounded_rect_path(cx - 18.0 * SS, cy - 12.0 * SS, 36.0 * SS, 24.0 * SS, 3.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            let pip_col = if state == "active" { "#FEF08A" } else { color };
            if let Some(pip) = rounded_rect_path(cx + 3.0 * SS, cy + 1.0 * SS, 11.0 * SS, 8.0 * SS, 2.0 * SS) {
                pm.fill_path(&pip, &paint_hex(pip_col, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "split" => {
            if let Some(l) = rounded_rect_path(cx - 18.0 * SS, cy - 12.0 * SS, 16.0 * SS, 24.0 * SS, 3.0 * SS) {
                pm.stroke_path(&l, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            if let Some(r) = rounded_rect_path(cx + 2.0 * SS, cy - 12.0 * SS, 16.0 * SS, 24.0 * SS, 3.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "focus" => {
            if let Some(r) = rounded_rect_path(cx - 16.0 * SS, cy - 11.0 * SS, 32.0 * SS, 22.0 * SS, 3.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            if let Some(dot) = PathBuilder::from_circle(cx, cy, 4.5 * SS) {
                pm.fill_path(&dot, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "multi" => {
            if let Some(l) = rounded_rect_path(cx - 19.0 * SS, cy - 10.0 * SS, 11.0 * SS, 20.0 * SS, 2.0 * SS) {
                pm.stroke_path(&l, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            if let Some(m) = rounded_rect_path(cx - 5.0 * SS, cy - 13.0 * SS, 10.0 * SS, 23.0 * SS, 2.0 * SS) {
                pm.fill_path(&m, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(r) = rounded_rect_path(cx + 8.0 * SS, cy - 10.0 * SS, 11.0 * SS, 20.0 * SS, 2.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "cam4k" => {
            if let Some(b) = rounded_rect_path(cx - 17.0 * SS, cy - 10.0 * SS, 34.0 * SS, 22.0 * SS, 4.0 * SS) {
                pm.stroke_path(&b, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            draw_arc_deg(pm, cx, cy + 1.0 * SS, 7.0 * SS, 0.0, 360.0, 2.5 * SS, color, false);
        }
        "track" => {
            draw_arc_deg(pm, cx, cy, 15.0 * SS, 0.0, 360.0, 3.0 * SS, color, false);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 13.0 * SS, cy);
            pb.line_to(cx + 13.0 * SS, cy);
            pb.move_to(cx, cy);
            pb.line_to(cx, cy + 13.0 * SS);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "obs" => {
            if let Some(circle) = PathBuilder::from_circle(cx, cy, 17.5 * SS) {
                pm.fill_path(&circle, &paint_hex("#0B0E14", 255), FillRule::Winding, Transform::identity(), None);
                let ring_st = Stroke { width: 2.2 * SS, ..Default::default() };
                pm.stroke_path(&circle, &paint_hex("#CBD5E1", 255), &ring_st, Transform::identity(), None);
            }
            let blade_r = 8.6 * SS;
            let orbit = 6.4 * SS;
            for base_deg in [270.0f32, 30.0, 150.0] {
                let rad = base_deg.to_radians();
                let bx = cx + orbit * rad.cos();
                let by = cy + orbit * rad.sin();
                draw_arc_deg(pm, bx, by, blade_r, base_deg - 10.0, 200.0, 3.6 * SS, "#FFFFFF", true);
            }
        }
        "spotify" => {
            if let Some(c) = PathBuilder::from_circle(cx, cy, 17.5 * SS) {
                pm.fill_path(&c, &paint_hex("#1DB954", 255), FillRule::Winding, Transform::identity(), None);
            }
            draw_arc_deg(pm, cx, cy + 7.0 * SS, 14.0 * SS, 232.0, 76.0, 3.2 * SS, "#0A140E", true);
            draw_arc_deg(pm, cx, cy + 10.0 * SS, 12.0 * SS, 235.0, 70.0, 2.8 * SS, "#0A140E", true);
            draw_arc_deg(pm, cx, cy + 13.0 * SS, 10.0 * SS, 238.0, 64.0, 2.4 * SS, "#0A140E", true);
        }
        "chrome" => {
            if let Some(ring) = PathBuilder::from_circle(cx, cy, 17.0 * SS) {
                pm.fill_path(&ring, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
            }
            draw_arc_deg(pm, cx, cy, 13.5 * SS, 205.0, 130.0, 6.2 * SS, "#EA4335", false);
            draw_arc_deg(pm, cx, cy, 13.5 * SS, 335.0, 120.0, 6.2 * SS, "#FBBC05", false);
            draw_arc_deg(pm, cx, cy, 13.5 * SS, 95.0, 110.0, 6.2 * SS, "#34A853", false);
            if let Some(w) = PathBuilder::from_circle(cx, cy, 8.2 * SS) {
                pm.fill_path(&w, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(b) = PathBuilder::from_circle(cx, cy, 6.0 * SS) {
                pm.fill_path(&b, &paint_hex("#4285F4", 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "edge" => {
            draw_arc_deg(pm, cx, cy, 14.5 * SS, 150.0, 230.0, 5.5 * SS, "#0EA5E9", true);
            draw_arc_deg(pm, cx, cy + 2.0 * SS, 11.0 * SS, 20.0, 180.0, 4.5 * SS, "#22C55E", true);
            if let Some(c) = PathBuilder::from_circle(cx - 1.0 * SS, cy + 1.0 * SS, 5.5 * SS) {
                pm.fill_path(&c, &paint_hex("#0284C7", 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "vscode" => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 14.0 * SS, cy - 5.0 * SS);
            pb.line_to(cx + 8.0 * SS, cy - 15.0 * SS);
            pb.line_to(cx + 15.0 * SS, cy - 11.0 * SS);
            pb.line_to(cx + 15.0 * SS, cy + 11.0 * SS);
            pb.line_to(cx + 8.0 * SS, cy + 15.0 * SS);
            pb.line_to(cx - 14.0 * SS, cy + 5.0 * SS);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex("#FFFFFF", 255), &Stroke { width: 4.0 * SS, line_join: tiny_skia::LineJoin::Round, ..Default::default() }, Transform::identity(), None);
            }
            let mut bar = PathBuilder::new();
            bar.move_to(cx + 8.0 * SS, cy - 14.0 * SS);
            bar.line_to(cx + 8.0 * SS, cy + 14.0 * SS);
            if let Some(p) = bar.finish() {
                pm.stroke_path(&p, &paint_hex("#38BDF8", 255), &Stroke { width: 4.5 * SS, ..Default::default() }, Transform::identity(), None);
            }
        }
        "steam" => {
            if let Some(c) = PathBuilder::from_circle(cx, cy, 17.5 * SS) {
                pm.fill_path(&c, &paint_hex("#1B2838", 255), FillRule::Winding, Transform::identity(), None);
                pm.stroke_path(&c, &paint_hex("#66C0F4", 255), &Stroke { width: 2.0 * SS, ..Default::default() }, Transform::identity(), None);
            }
            if let Some(top) = PathBuilder::from_circle(cx + 5.5 * SS, cy - 5.0 * SS, 5.5 * SS) {
                pm.stroke_path(&top, &paint_hex("#FFFFFF", 255), &Stroke { width: 2.6 * SS, ..Default::default() }, Transform::identity(), None);
            }
            if let Some(bot) = PathBuilder::from_circle(cx - 5.0 * SS, cy + 6.5 * SS, 4.2 * SS) {
                pm.fill_path(&bot, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
            }
            let mut rod = PathBuilder::new();
            rod.move_to(cx - 14.0 * SS, cy + 1.0 * SS);
            rod.line_to(cx - 5.0 * SS, cy + 6.5 * SS);
            rod.line_to(cx + 5.5 * SS, cy - 5.0 * SS);
            if let Some(p) = rod.finish() {
                pm.stroke_path(&p, &paint_hex("#FFFFFF", 255), &Stroke { width: 3.4 * SS, line_cap: tiny_skia::LineCap::Round, ..Default::default() }, Transform::identity(), None);
            }
        }
        "twitch" => {
            if let Some(box_p) = rounded_rect_path(cx - 15.0 * SS, cy - 14.0 * SS, 30.0 * SS, 24.0 * SS, 4.0 * SS) {
                pm.fill_path(&box_p, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
            }
            for ex in [-3.5, 4.5] {
                if let Some(eye) = rounded_rect_path(cx + ex * SS, cy - 7.5 * SS, 3.2 * SS, 9.0 * SS, 1.0 * SS) {
                    pm.fill_path(&eye, &paint_hex("#9146FF", 255), FillRule::Winding, Transform::identity(), None);
                }
            }
        }
        "youtube" => {
            if let Some(r) = rounded_rect_path(cx - 18.0 * SS, cy - 12.5 * SS, 36.0 * SS, 25.0 * SS, 7.0 * SS) {
                pm.fill_path(&r, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
            }
            let mut tri = PathBuilder::new();
            tri.move_to(cx - 4.5 * SS, cy - 6.5 * SS);
            tri.line_to(cx + 8.0 * SS, cy);
            tri.line_to(cx - 4.5 * SS, cy + 6.5 * SS);
            tri.close();
            if let Some(p) = tri.finish() {
                pm.fill_path(&p, &paint_hex("#DC2626", 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "nvidia" => {
            if let Some(r) = rounded_rect_path(cx - 17.0 * SS, cy - 12.0 * SS, 34.0 * SS, 24.0 * SS, 5.0 * SS) {
                pm.stroke_path(&r, &paint_hex("#FFFFFF", 255), &Stroke { width: 2.5 * SS, ..Default::default() }, Transform::identity(), None);
            }
            draw_arc_deg(pm, cx, cy, 11.0 * SS, 140.0, 260.0, 3.2 * SS, "#FFFFFF", true);
            draw_arc_deg(pm, cx + 1.0 * SS, cy, 6.0 * SS, 120.0, 260.0, 2.8 * SS, "#BEF264", true);
        }
        "terminal" => {
            if let Some(r) = rounded_rect_path(cx - 17.0 * SS, cy - 13.0 * SS, 34.0 * SS, 26.0 * SS, 5.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 10.0 * SS, cy - 5.0 * SS);
            pb.line_to(cx - 3.0 * SS, cy);
            pb.line_to(cx - 10.0 * SS, cy + 5.0 * SS);
            pb.move_to(cx + 1.0 * SS, cy + 5.0 * SS);
            pb.line_to(cx + 10.0 * SS, cy + 5.0 * SS);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex("#22D3EE", 255), &stroke, Transform::identity(), None);
            }
        }
        "explorer" => {
            if let Some(tab) = rounded_rect_path(cx - 17.0 * SS, cy - 13.0 * SS, 14.0 * SS, 7.0 * SS, 2.5 * SS) {
                pm.fill_path(&tab, &paint_hex("#F59E0B", 255), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(body) = rounded_rect_path(cx - 17.0 * SS, cy - 8.5 * SS, 34.0 * SS, 22.0 * SS, 4.0 * SS) {
                pm.fill_path(&body, &paint_hex("#FBBF24", 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "mic" => {
            if let Some(cap) = rounded_rect_path(cx - 5.5 * SS, cy - 14.0 * SS, 11.0 * SS, 17.0 * SS, 5.5 * SS) {
                pm.fill_path(&cap, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            draw_arc_deg(pm, cx, cy - 1.0 * SS, 10.5 * SS, 0.0, 180.0, 2.6 * SS, color, true);
            let mut st = PathBuilder::new();
            st.move_to(cx, cy + 9.5 * SS);
            st.line_to(cx, cy + 15.0 * SS);
            st.move_to(cx - 6.0 * SS, cy + 15.0 * SS);
            st.line_to(cx + 6.0 * SS, cy + 15.0 * SS);
            if let Some(p) = st.finish() {
                pm.stroke_path(&p, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "play" => {
            let mut tri = PathBuilder::new();
            tri.move_to(cx - 10.0 * SS, cy - 11.0 * SS);
            tri.line_to(cx + 4.0 * SS, cy);
            tri.line_to(cx - 10.0 * SS, cy + 11.0 * SS);
            tri.close();
            if let Some(p) = tri.finish() {
                pm.fill_path(&p, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            for x in [7.0, 12.5] {
                if let Some(b) = rounded_rect_path(cx + x * SS, cy - 9.0 * SS, 3.0 * SS, 18.0 * SS, 1.0 * SS) {
                    pm.fill_path(&b, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
                }
            }
        }
        "next" => {
            let mut tri = PathBuilder::new();
            tri.move_to(cx - 11.0 * SS, cy - 10.0 * SS);
            tri.line_to(cx + 4.0 * SS, cy);
            tri.line_to(cx - 11.0 * SS, cy + 10.0 * SS);
            tri.close();
            if let Some(p) = tri.finish() {
                pm.fill_path(&p, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(b) = rounded_rect_path(cx + 7.0 * SS, cy - 10.0 * SS, 4.0 * SS, 20.0 * SS, 1.5 * SS) {
                pm.fill_path(&b, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "prev" => {
            if let Some(b) = rounded_rect_path(cx - 11.0 * SS, cy - 10.0 * SS, 4.0 * SS, 20.0 * SS, 1.5 * SS) {
                pm.fill_path(&b, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            let mut tri = PathBuilder::new();
            tri.move_to(cx + 11.0 * SS, cy - 10.0 * SS);
            tri.line_to(cx - 4.0 * SS, cy);
            tri.line_to(cx + 11.0 * SS, cy + 10.0 * SS);
            tri.close();
            if let Some(p) = tri.finish() {
                pm.fill_path(&p, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "logo" | "folder" => {
            if let Some(tab) = rounded_rect_path(cx - 17.0 * SS, cy - 13.0 * SS, 14.0 * SS, 6.5 * SS, 2.5 * SS) {
                pm.fill_path(&tab, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(body) = rounded_rect_path(cx - 17.0 * SS, cy - 8.5 * SS, 34.0 * SS, 22.0 * SS, 4.0 * SS) {
                pm.stroke_path(&body, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "cursor" | "windsurf" => {
            // Hexágono / prisma de IDE AI
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - 16.0 * SS);
            pb.line_to(cx + 15.0 * SS, cy - 7.0 * SS);
            pb.line_to(cx + 15.0 * SS, cy + 7.0 * SS);
            pb.line_to(cx, cy + 16.0 * SS);
            pb.line_to(cx - 15.0 * SS, cy + 7.0 * SS);
            pb.line_to(cx - 15.0 * SS, cy - 7.0 * SS);
            pb.close();
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            let mut tri = PathBuilder::new();
            tri.move_to(cx - 5.0 * SS, cy - 7.0 * SS);
            tri.line_to(cx + 9.0 * SS, cy);
            tri.line_to(cx - 5.0 * SS, cy + 7.0 * SS);
            tri.close();
            if let Some(p) = tri.finish() {
                pm.fill_path(&p, &paint_hex("#22D3EE", 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "docker" => {
            // Contêineres em grade + casco
            for (dx, dy) in [(-9.0, -7.0), (-2.0, -7.0), (5.0, -7.0), (-2.0, -13.0)] {
                if let Some(b) = rounded_rect_path(cx + dx * SS, cy + dy * SS, 5.5 * SS, 4.5 * SS, 1.0 * SS) {
                    pm.fill_path(&b, &paint_hex("#38BDF8", 255), FillRule::Winding, Transform::identity(), None);
                }
            }
            if let Some(hull) = rounded_rect_path(cx - 15.0 * SS, cy, 30.0 * SS, 10.0 * SS, 5.0 * SS) {
                pm.fill_path(&hull, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "git" => {
            // Ramificação Git
            draw_arc_deg(pm, cx - 7.0 * SS, cy - 9.0 * SS, 4.0 * SS, 0.0, 360.0, 2.5 * SS, "#F97316", false);
            draw_arc_deg(pm, cx - 7.0 * SS, cy + 9.0 * SS, 4.0 * SS, 0.0, 360.0, 2.5 * SS, "#F97316", false);
            draw_arc_deg(pm, cx + 8.0 * SS, cy - 2.0 * SS, 4.0 * SS, 0.0, 360.0, 2.5 * SS, "#22D3EE", false);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 7.0 * SS, cy - 5.0 * SS);
            pb.line_to(cx - 7.0 * SS, cy + 5.0 * SS);
            pb.move_to(cx - 7.0 * SS, cy + 4.0 * SS);
            pb.line_to(cx + 5.0 * SS, cy);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "powershell" => {
            if let Some(r) = rounded_rect_path(cx - 17.0 * SS, cy - 13.0 * SS, 34.0 * SS, 26.0 * SS, 5.0 * SS) {
                pm.fill_path(&r, &paint_hex("#1D4ED8", 255), FillRule::Winding, Transform::identity(), None);
                pm.stroke_path(&r, &paint_hex("#60A5FA", 255), &Stroke { width: 2.0 * SS, ..Default::default() }, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 9.0 * SS, cy - 5.0 * SS);
            pb.line_to(cx - 1.0 * SS, cy);
            pb.line_to(cx - 9.0 * SS, cy + 5.0 * SS);
            pb.move_to(cx + 2.0 * SS, cy + 5.0 * SS);
            pb.line_to(cx + 10.0 * SS, cy + 5.0 * SS);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex("#FFFFFF", 255), &stroke, Transform::identity(), None);
            }
        }
        "notion" | "clipboard" => {
            if let Some(r) = rounded_rect_path(cx - 13.0 * SS, cy - 15.0 * SS, 26.0 * SS, 30.0 * SS, 4.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            for dy in [-6.0, 0.0, 6.0] {
                let mut pb = PathBuilder::new();
                pb.move_to(cx - 7.0 * SS, cy + dy * SS);
                pb.line_to(cx + 7.0 * SS, cy + dy * SS);
                if let Some(p) = pb.finish() {
                    pm.stroke_path(&p, &paint_hex(color, 255), &Stroke { width: 2.2 * SS, line_cap: LineCap::Round, ..Default::default() }, Transform::identity(), None);
                }
            }
        }
        "slack" | "teams" | "zoom" => {
            if let Some(r) = rounded_rect_path(cx - 16.0 * SS, cy - 12.0 * SS, 22.0 * SS, 20.0 * SS, 5.0 * SS) {
                pm.fill_path(&r, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            let mut tri = PathBuilder::new();
            tri.move_to(cx + 7.0 * SS, cy - 5.0 * SS);
            tri.line_to(cx + 16.0 * SS, cy - 10.0 * SS);
            tri.line_to(cx + 16.0 * SS, cy + 6.0 * SS);
            tri.line_to(cx + 7.0 * SS, cy + 1.0 * SS);
            tri.close();
            if let Some(p) = tri.finish() {
                pm.fill_path(&p, &paint_hex("#38BDF8", 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "epic" | "battlenet" => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 13.0 * SS, cy - 15.0 * SS);
            pb.line_to(cx + 13.0 * SS, cy - 15.0 * SS);
            pb.line_to(cx + 13.0 * SS, cy + 6.0 * SS);
            pb.line_to(cx, cy + 16.0 * SS);
            pb.line_to(cx - 13.0 * SS, cy + 6.0 * SS);
            pb.close();
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            draw_text_centered(pm, if icon == "epic" { "E" } else { "B" }, cx, cy - 10.0 * SS, 16.0 * SS, color);
        }
        "firefox" | "brave" | "web" => {
            draw_arc_deg(pm, cx, cy, 14.5 * SS, 0.0, 360.0, 2.8 * SS, color, false);
            draw_arc_deg(pm, cx, cy, 7.5 * SS, 0.0, 360.0, 2.0 * SS, "#38BDF8", false);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - 14.5 * SS, cy);
            pb.line_to(cx + 14.5 * SS, cy);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex(color, 255), &Stroke { width: 2.0 * SS, ..Default::default() }, Transform::identity(), None);
            }
        }
        "photoshop" | "premiere" | "davinci" | "blender" => {
            if let Some(r) = rounded_rect_path(cx - 15.0 * SS, cy - 14.0 * SS, 30.0 * SS, 28.0 * SS, 6.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            let badge = match icon {
                "photoshop" => "Ps",
                "premiere" => "Pr",
                "davinci" => "DR",
                _ => "3D",
            };
            draw_text_centered(pm, badge, cx, cy - 9.0 * SS, 15.0 * SS, color);
        }
        "vlc" => {
            let mut cone = PathBuilder::new();
            cone.move_to(cx, cy - 15.0 * SS);
            cone.line_to(cx + 12.0 * SS, cy + 12.0 * SS);
            cone.line_to(cx - 12.0 * SS, cy + 12.0 * SS);
            cone.close();
            if let Some(p) = cone.finish() {
                pm.fill_path(&p, &paint_hex("#F97316", 255), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(base) = rounded_rect_path(cx - 15.0 * SS, cy + 11.0 * SS, 30.0 * SS, 4.0 * SS, 1.5 * SS) {
                pm.fill_path(&base, &paint_hex("#FFFFFF", 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "cpu" | "gpu" | "ram" => {
            if let Some(chip) = rounded_rect_path(cx - 11.0 * SS, cy - 11.0 * SS, 22.0 * SS, 22.0 * SS, 3.0 * SS) {
                pm.stroke_path(&chip, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            if let Some(core) = rounded_rect_path(cx - 5.0 * SS, cy - 5.0 * SS, 10.0 * SS, 10.0 * SS, 1.5 * SS) {
                pm.fill_path(&core, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            for d in [-6.0, 0.0, 6.0] {
                let mut pb = PathBuilder::new();
                pb.move_to(cx + d * SS, cy - 15.5 * SS);
                pb.line_to(cx + d * SS, cy - 11.5 * SS);
                pb.move_to(cx + d * SS, cy + 11.5 * SS);
                pb.line_to(cx + d * SS, cy + 15.5 * SS);
                if let Some(p) = pb.finish() {
                    pm.stroke_path(&p, &paint_hex(color, 255), &Stroke { width: 2.2 * SS, ..Default::default() }, Transform::identity(), None);
                }
            }
        }
        "fan" => {
            draw_arc_deg(pm, cx, cy, 4.0 * SS, 0.0, 360.0, 2.5 * SS, color, false);
            for deg in [0.0f32, 120.0, 240.0] {
                draw_arc_deg(pm, cx, cy, 12.0 * SS, deg, 70.0, 4.5 * SS, color, true);
            }
        }
        "volume_up" | "volume_down" | "mute_all" | "audio_switch" => {
            let mut spk = PathBuilder::new();
            spk.move_to(cx - 13.0 * SS, cy - 5.0 * SS);
            spk.line_to(cx - 7.0 * SS, cy - 5.0 * SS);
            spk.line_to(cx, cy - 11.0 * SS);
            spk.line_to(cx, cy + 11.0 * SS);
            spk.line_to(cx - 7.0 * SS, cy + 5.0 * SS);
            spk.line_to(cx - 13.0 * SS, cy + 5.0 * SS);
            spk.close();
            if let Some(p) = spk.finish() {
                pm.fill_path(&p, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
            if icon == "volume_up" {
                draw_arc_deg(pm, cx + 2.0 * SS, cy, 7.0 * SS, -50.0, 100.0, 2.6 * SS, color, true);
                draw_arc_deg(pm, cx + 2.0 * SS, cy, 12.5 * SS, -55.0, 110.0, 2.6 * SS, color, true);
            } else if icon == "volume_down" {
                draw_arc_deg(pm, cx + 2.0 * SS, cy, 7.0 * SS, -45.0, 90.0, 2.6 * SS, color, true);
            } else if icon == "mute_all" {
                let mut x = PathBuilder::new();
                x.move_to(cx + 5.0 * SS, cy - 6.0 * SS);
                x.line_to(cx + 14.0 * SS, cy + 6.0 * SS);
                x.move_to(cx + 14.0 * SS, cy - 6.0 * SS);
                x.line_to(cx + 5.0 * SS, cy + 6.0 * SS);
                if let Some(p) = x.finish() {
                    pm.stroke_path(&p, &paint_hex("#EF4444", 255), &stroke, Transform::identity(), None);
                }
            } else {
                draw_arc_deg(pm, cx + 8.0 * SS, cy, 6.0 * SS, -90.0, 220.0, 2.5 * SS, "#22D3EE", true);
            }
        }
        "lock_pc" => {
            draw_arc_deg(pm, cx, cy - 5.0 * SS, 7.5 * SS, 180.0, 180.0, 3.0 * SS, color, true);
            if let Some(body) = rounded_rect_path(cx - 11.0 * SS, cy - 3.0 * SS, 22.0 * SS, 17.0 * SS, 3.5 * SS) {
                pm.fill_path(&body, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        "screenshot" => {
            if let Some(r) = rounded_rect_path(cx - 16.0 * SS, cy - 11.0 * SS, 32.0 * SS, 22.0 * SS, 4.0 * SS) {
                pm.stroke_path(&r, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
            draw_arc_deg(pm, cx, cy, 6.0 * SS, 0.0, 360.0, 2.5 * SS, "#22D3EE", false);
        }
        "stopwatch" | "timer" => {
            draw_arc_deg(pm, cx, cy + 2.0 * SS, 13.0 * SS, 0.0, 360.0, 2.8 * SS, color, false);
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy + 2.0 * SS);
            pb.line_to(cx, cy - 6.0 * SS);
            pb.move_to(cx, cy + 2.0 * SS);
            pb.line_to(cx + 6.0 * SS, cy + 2.0 * SS);
            pb.move_to(cx - 4.0 * SS, cy - 14.0 * SS);
            pb.line_to(cx + 4.0 * SS, cy - 14.0 * SS);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint_hex(color, 255), &stroke, Transform::identity(), None);
            }
        }
        "lightbulb" => {
            draw_arc_deg(pm, cx, cy - 3.0 * SS, 9.5 * SS, 0.0, 360.0, 2.8 * SS, "#FBBF24", false);
            if let Some(base) = rounded_rect_path(cx - 5.0 * SS, cy + 8.0 * SS, 10.0 * SS, 6.0 * SS, 2.0 * SS) {
                pm.fill_path(&base, &paint_hex(color, 255), FillRule::Winding, Transform::identity(), None);
            }
        }
        _ => {
            draw_arc_deg(pm, cx, cy, 13.0 * SS, 0.0, 360.0, 3.0 * SS, color, false);
        }
    }
}

pub fn render_button_tile(
    title: &str,
    subtitle: &str,
    state: &str,
    theme: &ThemeConfig,
    icon: Option<&str>,
) -> RgbImage {
    render_button_tile_styled(title, subtitle, state, theme, icon, None)
}

pub fn render_button_tile_styled(
    title: &str,
    subtitle: &str,
    state: &str,
    theme: &ThemeConfig,
    icon: Option<&str>,
    overrides: Option<&TileColorOverride<'_>>,
) -> RgbImage {
    let mut pm = Pixmap::new(CANVAS_SZ, CANVAS_SZ).unwrap();
    let mut text_col = theme.text.as_str();
    let mut sub_col = theme.accent.as_str();
    let mut icon_col = "#FFFFFF";
    let mut bg_ov: Option<&str> = None;
    let mut border_ov: Option<&str> = None;
    let mut accent_ov: Option<&str> = None;

    match state {
        "active_red" => {
            draw_bevel_keycap(&mut pm, "#E11D48", "#881337", "#FB7185", "#4C0519");
            sub_col = "#FEF08A";
            icon_col = "#FFFFFF";
        }
        "active" | "active_cyan" => {
            draw_bevel_keycap(&mut pm, "#0284C7", "#0369A1", "#38BDF8", "#082F49");
            sub_col = "#E0F2FE";
            icon_col = "#FFFFFF";
        }
        "warning" => {
            draw_bevel_keycap(&mut pm, "#D97706", "#78350F", "#FCD34D", "#451A03");
            sub_col = "#FEF08A";
            icon_col = "#FFFFFF";
        }
        "disabled" => {
            bg_ov = Some("#05070B");
            border_ov = Some("#1E293B");
            accent_ov = Some("#334155");
            text_col = "#64748B";
            sub_col = "#475569";
            icon_col = "#475569";
            draw_themed_surface(&mut pm, theme, bg_ov, border_ov, accent_ov);
        }
        _ => {
            // Alto contraste OLED: fundo escuro do tema + borda/acento da marca (zero penumbra cinza!)
            match icon {
                Some("discord") => {
                    border_ov = Some("#5865F2");
                    accent_ov = Some("#7983F5");
                    sub_col = "#A5B4FC";
                }
                Some("obs") => {
                    border_ov = Some("#38BDF8");
                    accent_ov = Some("#38BDF8");
                    sub_col = "#38BDF8";
                }
                Some("spotify") => {
                    border_ov = Some("#1DB954");
                    accent_ov = Some("#22C55E");
                    sub_col = "#86EFAC";
                }
                Some("twitch") => {
                    border_ov = Some("#9146FF");
                    accent_ov = Some("#A78BFA");
                    sub_col = "#DDD6FE";
                }
                Some("youtube") => {
                    border_ov = Some("#EF4444");
                    accent_ov = Some("#F87171");
                    sub_col = "#FCA5A5";
                }
                Some("vscode") => {
                    border_ov = Some("#0EA5E9");
                    accent_ov = Some("#38BDF8");
                    sub_col = "#7DD3FC";
                }
                Some("steam") => {
                    border_ov = Some("#3B82F6");
                    accent_ov = Some("#60A5FA");
                    sub_col = "#93C5FD";
                }
                Some("nvidia") => {
                    border_ov = Some("#84CC16");
                    accent_ov = Some("#A3E635");
                    sub_col = "#D9F99D";
                }
                Some("powershell") => {
                    border_ov = Some("#2563EB");
                    accent_ov = Some("#60A5FA");
                    sub_col = "#93C5FD";
                }
                _ => {
                    icon_col = theme.accent.as_str();
                }
            }
            if let Some(ov) = overrides {
                if let Some(b) = ov.bg { bg_ov = Some(b); }
                if let Some(br) = ov.border { border_ov = Some(br); accent_ov = Some(br); }
                if let Some(tc) = ov.text { text_col = tc; }
                if let Some(ic) = ov.icon { icon_col = ic; }
            }
            draw_themed_surface(&mut pm, theme, bg_ov, border_ov, accent_ov);
        }
    }

    let cx = CANVAS_SZ as f32 * 0.5;
    if let Some(ic) = icon {
        draw_button_icon(&mut pm, ic, cx, 31.0 * SS, icon_col, state);
        let t_sz = (if title.len() <= 8 { 13.5 } else { 11.5 }) * SS;
        let ty = (if subtitle.is_empty() { 62.0 } else { 55.5 }) * SS;
        draw_text_centered_shadow(&mut pm, title, cx, ty, t_sz, text_col);
        if !subtitle.is_empty() {
            draw_text_centered_shadow(&mut pm, subtitle, cx, 72.5 * SS, 10.5 * SS, sub_col);
        }
    } else {
        let t_sz = (if title.len() <= 7 { 17.5 } else { 14.0 }) * SS;
        let ty = (if subtitle.is_empty() { 39.0 } else { 30.0 }) * SS;
        draw_text_centered_shadow(&mut pm, title, cx, ty, t_sz, text_col);
        if !subtitle.is_empty() {
            draw_text_centered_shadow(&mut pm, subtitle, cx, 57.0 * SS, 11.5 * SS, sub_col);
        }
    }

    downsample(pm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid_proto::{sanitize_jpeg, BUTTON_IMAGE_MAX_SIZE};

    #[test]
    fn rendered_tiles_sanitize_cleanly() {
        let rules = ColorRules::default();
        for theme in deck_core::default_themes().values() {
            let tiles = vec![
                render_sensor_tile("GPU", "40", "%", &[20.0, 40.0], Some(0.0), Some(100.0), theme, &rules, Some(40.0), Some("gpu"), Some("ring"), false),
                render_sensor_tile("CPU °C", "66", "°", &[55.0, 66.0], Some(0.0), Some(100.0), theme, &rules, Some(66.0), Some("cpu"), None, true),
                render_nav_circle_tile("next", theme),
                render_obs_logo_tile_themed(theme),
                render_clock_tile("11:05", "19/11", theme),
                render_obs_status_tile("REC", "2.Jogo", theme),
                render_weather_tile(Some(22.3), Some(18.0), Some(29.0), "Santa Maria", 1, theme),
                render_compact_health_tile("CPU", Some(17.0), Some(56.0), "#22D3EE", &rules, theme),
                render_button_tile("GRAVANDO", "PARAR", "active", theme, Some("record")),
                render_button_tile("Discord", "ABRIR", "idle", theme, Some("discord")),
                render_button_tile("OBS", "ABRIR", "idle", theme, Some("obs")),
                render_button_tile("Spotify", "ABRIR", "idle", theme, Some("spotify")),
                render_button_tile("Chrome", "ABRIR", "idle", theme, Some("chrome")),
                render_button_tile("VS Code", "ABRIR", "idle", theme, Some("vscode")),
                render_button_tile("Steam", "ABRIR", "idle", theme, Some("steam")),
                render_button_tile("Twitch", "ABRIR", "idle", theme, Some("twitch")),
                render_button_tile("YouTube", "ABRIR", "idle", theme, Some("youtube")),
                render_button_tile("NVIDIA", "ABRIR", "idle", theme, Some("nvidia")),
                render_button_tile("Terminal", "ABRIR", "idle", theme, Some("terminal")),
                render_button_tile("Pastas", "ABRIR", "idle", theme, Some("explorer")),
                render_button_tile("Cursor", "IDE", "idle", theme, Some("cursor")),
                render_button_tile("Docker", "RUN", "idle", theme, Some("docker")),
                render_button_tile("Git", "SYNC", "idle", theme, Some("git")),
                render_button_tile("PShell", "CLI", "idle", theme, Some("powershell")),
                render_button_tile("Vol +", "AUDIO", "idle", theme, Some("volume_up")),
                render_button_tile("Bloquear", "WIN+L", "idle", theme, Some("lock_pc")),
                render_button_tile("Timer", "25:00", "idle", theme, Some("stopwatch")),
                render_button_tile("Luz", "TOGGLE", "idle", theme, Some("lightbulb")),
            ];
            for t in tiles {
                let jpg = sanitize_jpeg(&t, 96, true).unwrap();
                assert!(jpg.len() <= BUTTON_IMAGE_MAX_SIZE);
            }
        }
    }
}
