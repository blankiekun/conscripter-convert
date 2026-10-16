//! `CreateText(name, priority, x, y, w, h, text)` in the story (装甲悪鬼村正's
//! 殺 害, 〝我は力〟, the date captions): the text drawn by the converter
//! into a picture, in the last `SetFont`'s style, then an ordinary picture
//! object (`Fade`, `Move`, `Rotate` work on it).
//!
//! `SetFont(face, size, colour, shadow colour, weight, shadow)`. A face
//! starting with `@` is Windows' sideways font: each upright letter turned a
//! quarter counterclockwise along a line the script then turns with
//! `Rotate(…, 90)`, so the line reads top to bottom; brackets, dashes and
//! ASCII lie along the line as they are. Under wine the date caption
//! (`md04_006`, size 30, `HEAVY`, `RIGHTDOWN`) shows upright letters with
//! a black shadow 2 px right and down on the screen, after the turn.

use std::hash::{Hash, Hasher};

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use image::{Rgba, RgbaImage, imageops};

use super::{Compiler, Kind, Value, color};

/// Where the pictures go: `system/strip/text_<hash>.png`.
const PREFIX: &str = "text_";

/// A picture `CreateText` drew.
pub(super) fn is_text(image: &str) -> bool {
    image.starts_with(&format!("system/strip/{PREFIX}"))
}

/// The shadow's offset, measured at size 30.
const SHADOW: i32 = 2;

/// Lie along a sideways line as they are (turned with it): their glyphs
/// are drawn for horizontal text.
fn along_line(c: char) -> bool {
    c.is_ascii() || "ー―‐－…‥～〜「」『』（）〔〕［］｛｝〈〉《》【】〝〟＝".contains(c)
}

/// The style of a `CreateText`: the last `SetFont`.
#[derive(Hash)]
pub(super) struct Style {
    pub size: u32,
    pub rgb: u32,
    /// The shadow's colour, when there is one, and its offset.
    pub shadow: Option<(u32, (i32, i32))>,
    pub heavy: bool,
    pub sideways: bool,
}

impl Style {
    fn from_set_font(a: &[Value]) -> Style {
        let arg = |i: usize| a.get(i).cloned().unwrap_or(Value::Null);
        let sideways = arg(0).string().starts_with('@');
        let heavy = match arg(4) {
            Value::Int(n) => n >= 600,
            v => matches!(v.string().to_ascii_lowercase().as_str(), "heavy" | "bold"),
        };
        let (dx, dy) = match arg(5).string().to_ascii_lowercase().as_str() {
            "rightdown" => (SHADOW, SHADOW),
            "right" => (SHADOW, 0),
            "down" => (0, SHADOW),
            _ => (0, 0),
        };
        // Sideways, the shadow is placed so that it is right and down once
        // the line is turned clockwise.
        let offset = if sideways { (dy, -dx) } else { (dx, dy) };
        Style {
            size: if a.is_empty() { 20 } else { arg(1).int().clamp(1, 400) as u32 },
            rgb: color(&arg(2)).unwrap_or(0xFFFFFF),
            shadow: (offset != (0, 0)).then(|| (color(&arg(3)).unwrap_or(0), offset)),
            heavy,
            sideways,
        }
    }
}

impl Compiler<'_, '_> {
    /// `arg` reads (and marks read: [`Compiler::builtin`]) an argument.
    pub(super) fn create_text(&mut self, arg: &dyn Fn(usize) -> Value) {
        let name = self.object_name(&arg(0));
        let mut text = arg(6).string();
        // 続・殺戮のジャンゴ's song (dja026): `<I>荒.野.開.く</I>`, drawn under
        // wine as the letters alone, upright, centred on the screen without
        // its `Move(…, @15, …)` and 13 px lower than drawn here.
        // shortcut: measured offset for these 13 lines only; find the rule
        // (the text object's margins) if another game's markup needs it.
        let mut nudge = (0, 0);
        if let Some(inner) = text.strip_prefix("<I>").and_then(|t| t.strip_suffix("</I>")) {
            text = inner.replace('.', "");
            nudge = (-15, 13);
        }
        // Left out whole (no note per argument): the choices' buttons are
        // the engine's own choice; markup is noted.
        if self.file == "function_select.nss" || text.contains('<') {
            (0..7).for_each(|i| drop(arg(i)));
            if text.contains('<') {
                self.note("CreateText with markup");
            }
            return;
        }
        let style = Style::from_set_font(&self.font);
        let wrap = match arg(4) {
            Value::Int(w) if w > 0 => Some(w as u32),
            _ => None,
        };
        let Some(mut img) = draw(self.assets.font(), &text, &style, wrap) else {
            self.note("CreateText without a font");
            return;
        };
        // The exe draws into a surface of the box's size (`auto`: the text's):
        // what passes the box's height isn't shown.
        let clip = match arg(5) {
            Value::Int(h) if h > 0 && (h as u32) < img.height() => Some(h as u32),
            _ => None,
        };
        if let Some(h) = clip {
            img = imageops::crop_imm(&img, 0, 0, img.width(), h).to_image();
        }
        let (w, h) = img.dimensions();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        (&text, &style, wrap).hash(&mut hash);
        clip.inspect(|h| h.hash(&mut hash));
        let file = format!("{PREFIX}{:016x}", hash.finish());
        let Some(image) = self.assets.frames_strip(&file, w, h, 1, &mut |_, out| out.copy_from_slice(&img)) else { return };
        let (sw, sh) = self.game.screen;
        let (w, h) = (w as i64, h as i64);
        // `inherit` (the date captions) is measured as the screen's middle.
        let y = match arg(3) {
            Value::Str(s) if s.eq_ignore_ascii_case("inherit") => (sh - h) / 2,
            v => v.position(0, h, sh),
        };
        let o = self.create(name, Kind::Image { image }, arg(1).int(), w, h);
        o.x = arg(2).position(0, w, sw) + nudge.0;
        o.y = y + nudge.1;
    }
}

/// `text` in `style`, lines broken at `\n` and, given a width, before the
/// letter that would pass it; the picture fits the text and its shadow.
pub(super) fn draw(font: &[u8], text: &str, style: &Style, wrap: Option<u32>) -> Option<RgbaImage> {
    let face = FontRef::try_from_slice(font).ok()?;
    // ab_glyph scales by the line's height, Windows by the em.
    let size = style.size as f32 * face.height_unscaled() / face.units_per_em().unwrap_or(1000.0);
    let scaled = face.as_scaled(PxScale::from(size));
    let cell = (scaled.ascent() - scaled.descent()).ceil() as u32;
    // Each letter as a coverage picture, in the line's direction.
    let letter = |c: char| -> (u32, Vec<f32>, u32) {
        // Sideways, the comma and stop of vertical text, when the font has them.
        let c = match c {
            '、' if style.sideways && face.glyph_id('︑').0 != 0 => '︑',
            '。' if style.sideways && face.glyph_id('︒').0 != 0 => '︒',
            c => c,
        };
        let id = face.glyph_id(c);
        let adv = scaled.h_advance(id).ceil().max(1.0) as u32;
        let mut mask = vec![0f32; (adv * cell) as usize];
        if let Some(g) = face.outline_glyph(id.with_scale_and_position(size, point(0.0, scaled.ascent()))) {
            let b = g.px_bounds();
            g.draw(|gx, gy, v| {
                let (x, y) = (b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32);
                if x >= 0 && y >= 0 && (x as u32) < adv && (y as u32) < cell {
                    let m = &mut mask[(y as u32 * adv + x as u32) as usize];
                    *m = m.max(v);
                }
            });
        }
        if style.sideways && !along_line(c) {
            // A quarter counterclockwise: (x, y) → (y, adv - 1 - x).
            let mut turned = vec![0f32; mask.len()];
            for y in 0..cell {
                for x in 0..adv {
                    turned[((adv - 1 - x) * cell + y) as usize] = mask[(y * adv + x) as usize];
                }
            }
            return (cell, turned, adv);
        }
        (adv, mask, cell)
    };
    // Lay the letters out in lines.
    let mut placed = Vec::new();
    let (mut x, mut y, mut line_h, mut width) = (0u32, 0u32, cell, 0u32);
    for c in text.chars() {
        if c == '\n' {
            (x, y, line_h) = (0, y + line_h, cell);
            continue;
        }
        let (w, mask, h) = letter(c);
        if wrap.is_some_and(|max| x > 0 && x + w > max) {
            (x, y, line_h) = (0, y + line_h, cell);
        }
        placed.push((x, y, w, h, mask));
        x += w;
        width = width.max(x);
        line_h = line_h.max(h);
    }
    let height = y + line_h;
    let bold = if style.heavy { (style.size / 20).max(1) } else { 0 };
    let (dx, dy) = style.shadow.map_or((0, 0), |s| s.1);
    let (pad_x, pad_y) = (dx.unsigned_abs(), dy.unsigned_abs());
    let (w, h) = (width.max(1) + bold + 2 * pad_x, height.max(1) + 2 * pad_y);
    let mut mask = vec![0f32; (w * h) as usize];
    for (lx, ly, lw, lh, m) in &placed {
        for gy in 0..*lh {
            for gx in 0..*lw {
                let v = m[(gy * lw + gx) as usize];
                // Heavy: the letter thickened to the right.
                for k in 0..=bold {
                    let i = ((pad_y + ly + gy) * w + pad_x + lx + gx + k) as usize;
                    mask[i] = mask[i].max(v);
                }
            }
        }
    }
    let rgba = |rgb: u32, a: f32| {
        let [_, r, g, b] = rgb.to_be_bytes();
        Rgba([r, g, b, (a * 255.0).round() as u8])
    };
    let mut out = RgbaImage::new(w, h);
    if let Some((shadow, _)) = style.shadow {
        let mut s = RgbaImage::new(w, h);
        for (i, &v) in mask.iter().enumerate() {
            s.put_pixel(i as u32 % w, i as u32 / w, rgba(shadow, v));
        }
        imageops::overlay(&mut out, &s, i64::from(dx), i64::from(dy));
    }
    let mut t = RgbaImage::new(w, h);
    for (i, &v) in mask.iter().enumerate() {
        t.put_pixel(i as u32 % w, i as u32 / w, rgba(style.rgb, v));
    }
    imageops::overlay(&mut out, &t, 0, 0);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../../../../conv-cli/fonts/ipag.ttf");

    #[test]
    fn sideways_lines_turn_letters_but_not_brackets() {
        let style = |sideways| Style { size: 30, rgb: 0xFFFFFF, shadow: Some((0, (SHADOW, -SHADOW))), heavy: false, sideways };
        let flat = draw(FONT, "日", &style(false), None).unwrap();
        let side = draw(FONT, "日", &style(true), None).unwrap();
        // 日 is taller than wide; turned, wider than tall.
        let ink = |img: &RgbaImage| {
            let px: Vec<_> = img.enumerate_pixels().filter(|p| p.2.0[3] > 128 && p.2.0[0] > 128).map(|p| (p.0, p.1)).collect();
            let w = px.iter().map(|p| p.0).max().unwrap() - px.iter().map(|p| p.0).min().unwrap();
            let h = px.iter().map(|p| p.1).max().unwrap() - px.iter().map(|p| p.1).min().unwrap();
            (w, h)
        };
        let (fw, fh) = ink(&flat);
        let (sw, sh) = ink(&side);
        assert!(fh > fw && sw > sh, "{fw}x{fh} {sw}x{sh}");
        // A dash lies along the line either way.
        let dash = draw(FONT, "ー", &style(true), None).unwrap();
        let (dw, dh) = ink(&dash);
        assert!(dw > dh, "{dw}x{dh}");
    }

    #[test]
    fn wraps_at_the_width() {
        let style = Style { size: 20, rgb: 0xFFFFFF, shadow: None, heavy: false, sideways: false };
        let one = draw(FONT, "あいうえお", &style, None).unwrap();
        let two = draw(FONT, "あいうえお", &style, Some(60)).unwrap();
        assert!(two.height() > one.height() && two.width() <= 60, "{:?} {:?}", one.dimensions(), two.dimensions());
    }
}
