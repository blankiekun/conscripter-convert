//! Words drawn with a game's font into pictures (ab_glyph): menu labels
//! (俺たちに翼はない's), subtitles (Full Metal Daemon Muramasa's English
//! port's).

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use image::{Rgba, RgbaImage};

/// Draws labels with the game's font.
pub struct Labels<'f> {
    font: FontRef<'f>,
}

impl<'f> Labels<'f> {
    pub fn new(font: &'f [u8]) -> Option<Labels<'f>> {
        Some(Labels { font: FontRef::try_from_slice(font).ok()? })
    }

    /// `text` at `size` pixels: `rgb` letters with a `glow` halo (`alpha`
    /// 0-255 for the whole label).
    pub(crate) fn draw(&self, text: &str, size: f32, rgb: [u8; 3], glow: [u8; 3], alpha: u8) -> RgbaImage {
        let font = self.font.as_scaled(PxScale::from(size));
        let pad = 4;
        let width: f32 = text.chars().map(|c| font.h_advance(font.glyph_id(c))).sum();
        let (w, h) = (width.ceil() as u32 + 2 * pad, (font.ascent() - font.descent()).ceil() as u32 + 2 * pad);
        let mut mask = vec![0f32; (w * h) as usize];
        let mut x = pad as f32;
        for c in text.chars() {
            let id = font.glyph_id(c);
            let glyph = id.with_scale_and_position(size, point(x, pad as f32 + font.ascent()));
            x += font.h_advance(id);
            if let Some(g) = self.font.outline_glyph(glyph) {
                let b = g.px_bounds();
                g.draw(|gx, gy, v| {
                    let (px, py) = (b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32);
                    if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                        let m = &mut mask[(py as u32 * w + px as u32) as usize];
                        *m = m.max(v);
                    }
                });
            }
        }
        let halo = spread(&mask, w, h);
        let mut img = RgbaImage::new(w, h);
        for (i, px) in img.pixels_mut().enumerate() {
            let (m, g) = (mask[i], halo[i] * 0.85);
            let a = m + g * (1.0 - m);
            if a <= 0.0 {
                continue;
            }
            let mix = |t: u8, gl: u8| ((t as f32 * m + gl as f32 * g * (1.0 - m)) / a) as u8;
            *px = Rgba([mix(rgb[0], glow[0]), mix(rgb[1], glow[1]), mix(rgb[2], glow[2]), (a * alpha as f32) as u8]);
        }
        img
    }
}

/// A halo around a coverage mask: the letters spread by 2 pixels.
pub(crate) fn spread(mask: &[f32], w: u32, h: u32) -> Vec<f32> {
    let mut halo = vec![0f32; mask.len()];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut sum = 0.0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let (sx, sy) = (x + dx, y + dy);
                    if sx >= 0 && sy >= 0 && sx < w as i32 && sy < h as i32 {
                        sum += mask[(sy as u32 * w + sx as u32) as usize];
                    }
                }
            }
            halo[(y as u32 * w + x as u32) as usize] = (sum / 8.0).min(1.0);
        }
    }
    halo
}

