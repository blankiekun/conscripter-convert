//! The title's intro (`Title\title.cpp`): the states of title mode 0
//! (table 0x561ee0, filled by 0x4f0a70; per-frame task 0x45b5d0), which
//! the exe runs at every title it enters from the warning screen or the
//! story (0x45b6f0: from 0x450870, opcode 0x96 and back-to-title 0x478300;
//! coming back from the extras' sub-modes is mode 2, 0x45b720, without it).
//! From black, with the pictures of `HD/gd.arc` → `Title.004`:
//!
//! - 0-2 s: the background (#0) cross-fades in (0x459cd0: 2000 ms
//!   `CrossFade` 0x442c70, then 0x459d50 sets the picture; 0x459e00 waits);
//! - 2-3 s: nothing moves (0x459eb0);
//! - 3-5 s: the background under the logo's place (590 × 262 at (0, 171))
//!   ripples: each pixel taken from d × sin(2π m/256) pixels away along a
//!   direction picked by `rand() & 0xff` at the title's start (0x459320),
//!   m being the pixel's grey level in the map #11 (Filter-Replace's pixel
//!   loop 0x43d9d0, sine tables 0x501538 and 0x501938), drawn over it at an
//!   opacity going 0 → 256 as d goes 0 → 25 px (0x459fa0, drawn 0x4599a0);
//! - 5-7 s: the logo (#10) fades in over it as d goes 25 → 50 px
//!   (0x45a070);
//! - 7 s: the ripple stops, "COMPLETE" (#14 at (100, 383), drawn only at
//!   the logo's full opacity) shows, and the four menu words fade in over
//!   500 ms each, 200 ms apart (0x45a330, table 0x4fb808; 0x4598e0).
//!
//! A click (or a key) at any point ends it on the title at rest (every
//! state sets 0x370 and jumps to its end). The title's music (BGM slot 13,
//! looped) starts with the black screen (0x459320, `0x47fe30(0xd, ...)`).

use image::{RgbImage, RgbaImage};

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;
/// The last change (the fourth word's fade ends at 7000 + 600 + 500);
/// the movie holds the rest a little past it.
pub const LENGTH_MS: u32 = 8200;

/// Where the logo, its ripple map and the ripple sit.
const LOGO_Y: i64 = 171;
/// The menu words: (x, y, delay ms, picture), from the exe's table at
/// 0x4fb808 (40 bytes a word); each a column of four 32-px cells.
pub const WORDS: [(i64, i64, u32, usize); 4] = [(626, 221, 0, 6), (626, 250, 200, 2), (626, 300, 400, 7), (626, 369, 600, 3)];
const WORD_FADE_MS: u32 = 500;
/// `rand() & 0xff` is the ripple's direction (256 steps a turn); ours is
/// one fixed draw.
// shortcut: one direction for every intro (the exe picks one of 256 at random each time); render a pool of movies if it matters.
pub const DIRECTION: i64 = 41;

/// Draws the intro `t` ms in, from `Title.004`'s pictures, into `frame`.
pub fn render(pictures: &[RgbaImage], t: u32, frame: &mut RgbImage) {
    let bg = &pictures[0];
    // 0x442c70's cross-fade from the black screen.
    let fade = (t.min(2000) * 256 / 2000) as i64;
    for (p, s) in frame.pixels_mut().zip(bg.pixels()) {
        for c in 0..3 {
            p[c] = (i64::from(s[c]) * fade / 256) as u8;
        }
    }
    // (ripple opacity, ripple size in 1/256 px, logo opacity) by state.
    let (ripple, size, logo) = match t {
        ..3000 => (0, 0, 0),
        3000..5000 => {
            let a = i64::from(t - 3000) * 256 / 2000;
            (a, (a >> 1) * 50, 0)
        }
        5000..7000 => {
            let a = i64::from(t - 5000) * 256 / 2000;
            (256, ((a >> 1) + 0x80) * 50, a)
        }
        _ => (0, 0, 256),
    };
    if size != 0 {
        ripple_over(frame, bg, &pictures[11], size, ripple);
    }
    blend(frame, &pictures[10], 0, LOGO_Y, logo);
    if logo == 256 {
        blend(frame, &pictures[14], 100, 383, 256);
    }
    for (x, y, delay, n) in WORDS {
        let gone = t.saturating_sub(7000 + delay);
        if gone > 0 {
            let cells = &pictures[n];
            let cell = image::imageops::crop_imm(cells, 0, 0, cells.width(), cells.height() / 4).to_image();
            blend(frame, &cell, x, y, i64::from(gone.min(WORD_FADE_MS) * 256 / WORD_FADE_MS));
        }
    }
}

/// 0x4599a0 + 0x43d9d0: the background under the map, displaced through
/// it, drawn over the frame at `opacity` (0-256).
fn ripple_over(frame: &mut RgbImage, bg: &RgbaImage, map: &RgbaImage, size: i64, opacity: i64) {
    let sin256 = |i: i64| ((i & 0xff) as f64 * std::f64::consts::TAU / 256.0).sin() * 65536.0;
    let (sx, sy) = (sin256(DIRECTION).round() as i64, sin256(DIRECTION + 0x40).round() as i64);
    // 64-bit products shifted right: floors, as the exe's __allshr.
    let (vx, vy) = ((sx * (size << 8)) >> 16, (sy * (size << 8)) >> 16);
    let (w, h) = (i64::from(map.width()), i64::from(map.height()));
    for y in 0..h {
        for x in 0..w {
            let m = i64::from(map.get_pixel(x as u32, y as u32)[0]);
            let s = ((((m * 4) & 0x3ff) as f64 * std::f64::consts::TAU / 1024.0).sin() * 1_073_741_824.0).round() as i64 >> 14;
            let mut fx = x + ((vx * s) >> 32);
            let mut fy = y + ((vy * s) >> 32);
            if !(0..w).contains(&fx) {
                fx = x;
            }
            if !(0..h).contains(&fy) {
                fy = y;
            }
            let src = bg.get_pixel(fx as u32, (fy + LOGO_Y) as u32);
            let dst = frame.get_pixel_mut(x as u32, (y + LOGO_Y) as u32);
            for c in 0..3 {
                let d = i64::from(dst[c]);
                dst[c] = (d + (i64::from(src[c]) - d) * opacity / 256) as u8;
            }
        }
    }
}

/// `pic` over `frame` at (x, y), its alpha times `opacity` (0-256).
fn blend(frame: &mut RgbImage, pic: &RgbaImage, x: i64, y: i64, opacity: i64) {
    if opacity <= 0 {
        return;
    }
    for (px, py, s) in pic.enumerate_pixels() {
        let (fx, fy) = (x + i64::from(px), y + i64::from(py));
        if fx < 0 || fy < 0 || fx >= i64::from(frame.width()) || fy >= i64::from(frame.height()) {
            continue;
        }
        let a = i64::from(s[3]) * opacity;
        let d = frame.get_pixel_mut(fx as u32, fy as u32);
        for c in 0..3 {
            let v = i64::from(d[c]);
            d[c] = (v + (i64::from(s[c]) - v) * a / (255 * 256)) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pictures() -> Vec<RgbaImage> {
        let mut p = vec![RgbaImage::new(1, 1); 15];
        p[0] = RgbaImage::from_fn(WIDTH, HEIGHT, |x, _| image::Rgba([(x % 256) as u8, 100, 200, 255]));
        p[10] = RgbaImage::from_pixel(590, 262, image::Rgba([255, 0, 0, 255]));
        // Grey 64: sin(π/2) = 1, the whole size along the direction.
        p[11] = RgbaImage::from_pixel(590, 262, image::Rgba([64, 64, 64, 255]));
        p[14] = RgbaImage::from_pixel(10, 10, image::Rgba([0, 255, 0, 255]));
        for n in [2, 3, 6, 7] {
            p[n] = RgbaImage::from_pixel(20, 128, image::Rgba([0, 0, 255, 255]));
        }
        p
    }

    #[test]
    fn states() {
        let p = pictures();
        let mut f = RgbImage::new(WIDTH, HEIGHT);
        render(&p, 0, &mut f);
        assert_eq!(f.get_pixel(5, 5).0, [0, 0, 0]);
        render(&p, 2500, &mut f);
        assert_eq!(f.get_pixel(5, 300).0, [5, 100, 200]);
        // 4.999 s: ripple opacity 255/256, d = (255 >> 1)·50/256 ≈ 24.8 px
        // times sin(2π·41/256) ≈ 0.84 along x (map grey 64: sin = 1).
        render(&p, 4999, &mut f);
        let shift = i64::from(f.get_pixel(100, 300)[0]) - 100;
        let vx = (sin_x() * (((255 >> 1) * 50) << 8)) >> 16;
        assert!((shift - ((vx * 65536) >> 32)).abs() <= 1, "{shift}");
        assert!(shift >= 18, "{shift}");
        // At rest: logo, COMPLETE, the words.
        render(&p, LENGTH_MS, &mut f);
        assert_eq!(f.get_pixel(5, 300).0, [255, 0, 0]);
        assert_eq!(f.get_pixel(105, 388).0, [0, 255, 0]);
        assert_eq!(f.get_pixel(630, 375).0, [0, 0, 255]);
        // The last word is still fading at 7.85 s.
        render(&p, 7850, &mut f);
        assert!(f.get_pixel(630, 375)[2] < 255);
    }

    fn sin_x() -> i64 {
        ((DIRECTION as f64) * std::f64::consts::TAU / 256.0).sin().mul_add(65536.0, 0.0).round() as i64
    }
}
