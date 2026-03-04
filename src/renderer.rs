use anyhow::{Context, Result};
use image::{RgbImage, Rgb};
use std::env;

use crate::collection::Artwork;

// ---------------------------------------------------------------------------
// Font loading
// ---------------------------------------------------------------------------

pub fn load_font() -> Option<fontdue::Font> {
    const BYTES: &[u8] = include_bytes!("../assets/fonts/LibreBaskerville.ttf");
    if let Ok(font) = fontdue::Font::from_bytes(BYTES, fontdue::FontSettings::default()) {
        return Some(font);
    }

    let home = env::var("HOME").unwrap_or_default();

    let candidates: Vec<String> = vec![
        // Project-relative (dev) — highest priority for custom fonts
        "assets/fonts/font.ttf".to_string(),
        // User data dir
        format!("{}/.local/share/artgg/font.ttf", home),
        // Alongside the binary
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("font.ttf").to_string_lossy().into_owned()))
            .unwrap_or_default(),
    ];

    for path in candidates {
        if path.is_empty() { continue; }
        if let Ok(data) = std::fs::read(&path) {
            match fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()) {
                Ok(font) => {
                    return Some(font);
                }
                Err(_) => continue,
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

pub fn parse_hex_color(s: &str) -> Rgb<u8> {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        Rgb([r, g, b])
    } else {
        Rgb([255, 255, 255])
    }
}

// ---------------------------------------------------------------------------
// Text rendering (fontdue)
// ---------------------------------------------------------------------------

/// Returns pixel advance width for the given text at the given size.
#[allow(dead_code)]
fn text_width(font: &fontdue::Font, text: &str, size: f32) -> u32 {
    text.chars()
        .map(|c| font.metrics(c, size).advance_width as u32)
        .sum()
}

/// Draw a single line of text onto the image.
/// `baseline_y` is the y-coordinate of the text baseline.
fn draw_text(
    img: &mut RgbImage,
    font: &fontdue::Font,
    text: &str,
    x: i32,
    baseline_y: i32,
    size: f32,
    color: [u8; 3],
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        if metrics.width == 0 || metrics.height == 0 {
            cursor_x += metrics.advance_width as i32;
            continue;
        }

        // Position glyph so its bottom aligns with baseline_y.
        // metrics.ymin = distance from baseline to bottom of glyph (can be negative for descenders).
        let glyph_top = baseline_y - metrics.height as i32 - metrics.ymin;
        let glyph_left = cursor_x + metrics.xmin;

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let alpha = bitmap[row * metrics.width + col];
                if alpha == 0 {
                    continue;
                }
                let px = glyph_left + col as i32;
                let py = glyph_top + row as i32;
                if px < 0 || py < 0 || px >= img.width() as i32 || py >= img.height() as i32 {
                    continue;
                }
                let existing = *img.get_pixel(px as u32, py as u32);
                let a = alpha as f32 / 255.0;
                let r = (color[0] as f32 * a + existing[0] as f32 * (1.0 - a)) as u8;
                let g = (color[1] as f32 * a + existing[1] as f32 * (1.0 - a)) as u8;
                let b = (color[2] as f32 * a + existing[2] as f32 * (1.0 - a)) as u8;
                img.put_pixel(px as u32, py as u32, Rgb([r, g, b]));
            }
        }
        cursor_x += metrics.advance_width as i32;
    }
}

// ---------------------------------------------------------------------------
// Main render function
// ---------------------------------------------------------------------------

pub fn render_wallpaper(
    source_image_path: &str,
    artwork: &Artwork,
    canvas_w: u32,
    canvas_h: u32,
    bg_color: Rgb<u8>,
    placard_color: Rgb<u8>,
    placard_text_color: Rgb<u8>,
    placard_opacity: f32, // 0.0 – 1.0
    font: Option<&fontdue::Font>,
) -> Result<RgbImage> {
    // 1. Create canvas.
    let mut canvas = RgbImage::from_pixel(canvas_w, canvas_h, bg_color);

    // 2. Load artwork image.
    let art_dyn = image::open(source_image_path)
        .with_context(|| format!("Cannot open cached image '{}'", source_image_path))?;
    let art = art_dyn.to_rgb8();
    let (art_w, art_h) = art.dimensions();

    // 3. Scale artwork to fit within 70% width × 65% height (preserving aspect ratio).
    let max_art_w = (canvas_w as f32 * 0.70) as u32;
    let max_art_h = (canvas_h as f32 * 0.65) as u32;
    let scale = (max_art_w as f32 / art_w as f32).min(max_art_h as f32 / art_h as f32);
    let scaled_w = ((art_w as f32 * scale) as u32).max(1);
    let scaled_h = ((art_h as f32 * scale) as u32).max(1);

    let scaled = image::imageops::resize(&art, scaled_w, scaled_h, image::imageops::FilterType::Lanczos3);

    // 4. Center artwork horizontally; place in upper portion leaving placard room.
    let art_x = (canvas_w.saturating_sub(scaled_w)) / 2;
    let art_y = (canvas_h as f32 * 0.08) as u32;

    // 5. Composite artwork onto canvas.
    image::imageops::overlay(&mut canvas, &scaled, art_x as i64, art_y as i64);

    // 6. Draw placard.
    let placard_w = (canvas_w as f32 * 0.52) as u32;
    let placard_h = (canvas_h as f32 * 0.17) as u32;
    let placard_x = (canvas_w.saturating_sub(placard_w)) / 2;
    let placard_gap = (canvas_h as f32 * 0.03) as u32;
    let placard_y = art_y + scaled_h + placard_gap;

    // Clamp placard to canvas
    let placard_y_end = (placard_y + placard_h).min(canvas_h);

    // Draw placard background with opacity blending against whatever is already on canvas.
    for py in placard_y..placard_y_end {
        for px in placard_x..(placard_x + placard_w).min(canvas_w) {
            let behind = *canvas.get_pixel(px, py);
            canvas.put_pixel(px, py, blend(behind, placard_color, placard_opacity));
        }
    }

    // Accent bar at the top of the placard — uses text color for auto-contrast.
    let accent_h = 3u32;
    for py in placard_y..(placard_y + accent_h).min(canvas_h) {
        for px in placard_x..(placard_x + placard_w).min(canvas_w) {
            // Accent is always fully opaque.
            canvas.put_pixel(px, py, placard_text_color);
        }
    }

    // 7. Draw text onto placard (if font available).
    if let Some(font) = font {
        let text_color = [placard_text_color[0], placard_text_color[1], placard_text_color[2]];
        let margin_x = (placard_w as f32 * 0.05) as u32;
        let text_left = placard_x + margin_x;

        let title_size: f32 = (canvas_h as f32 * 0.022).max(18.0).min(36.0);
        let sub_size: f32   = (title_size * 0.75).max(12.0);
        let line_gap: i32   = (title_size * 1.35) as i32;
        let sub_gap: i32    = (sub_size * 1.4) as i32;

        // Start baseline just below the accent bar + top margin.
        let top_margin = (placard_h as f32 * 0.25) as i32;
        let mut baseline_y = placard_y as i32 + accent_h as i32 + top_margin;

        // Title (truncated to fit placard width)
        let max_title_chars = ((placard_w as f32 * 0.85) / (title_size * 0.52)) as usize;
        let title_str = truncate(&artwork.title, max_title_chars.max(10));
        draw_text(&mut canvas, font, &title_str, text_left as i32, baseline_y, title_size, text_color);
        baseline_y += line_gap;

        // Artist
        if let Some(artist) = &artwork.artist_display {
            let max_artist_chars = ((placard_w as f32 * 0.90) / (sub_size * 0.52)) as usize;
            let artist_str = truncate(artist, max_artist_chars.max(10));
            draw_text(&mut canvas, font, &artist_str, text_left as i32, baseline_y, sub_size, text_color);
            baseline_y += sub_gap;
        }

        // Date · Medium
        let meta = build_meta_line(artwork);
        if !meta.is_empty() {
            let max_meta_chars = ((placard_w as f32 * 0.90) / (sub_size * 0.52)) as usize;
            let meta_str = truncate(&meta, max_meta_chars.max(10));
            draw_text(&mut canvas, font, &meta_str, text_left as i32, baseline_y, sub_size, text_color);
        }
    }

    Ok(canvas)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Alpha-blend `fg` over `bg` with the given opacity (0.0 = fully transparent, 1.0 = opaque).
fn blend(bg: Rgb<u8>, fg: Rgb<u8>, opacity: f32) -> Rgb<u8> {
    let a = opacity.clamp(0.0, 1.0);
    Rgb([
        (fg[0] as f32 * a + bg[0] as f32 * (1.0 - a)) as u8,
        (fg[1] as f32 * a + bg[1] as f32 * (1.0 - a)) as u8,
        (fg[2] as f32 * a + bg[2] as f32 * (1.0 - a)) as u8,
    ])
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        chars[..max.saturating_sub(1)].iter().collect::<String>() + "…"
    }
}

fn build_meta_line(artwork: &Artwork) -> String {
    let mut parts = Vec::new();
    if let Some(d) = &artwork.date_display {
        if !d.is_empty() { parts.push(d.as_str()); }
    }
    if let Some(m) = &artwork.medium {
        if !m.is_empty() { parts.push(m.as_str()); }
    }
    parts.join("  ·  ")
}
