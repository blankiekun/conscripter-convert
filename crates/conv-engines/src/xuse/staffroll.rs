//! The staff roll of opcode 0x91 (`StaffRoll\Staff.cpp`, task 0x454a60).
//!
//! The exe has two rolls, picked by the opcode's arg through the table at
//! 0x517f30 (descriptors at 0x4fbf48 and 0x4fc720). Each one plays a BGM
//! track (0x47aab0, not looped) and draws, every frame:
//!
//! - a panel: a clip rectangle with two layers (B under A), each a picture
//!   of the roll's `.004` at screen position (dx, dy) and an opacity
//!   (0-256). The values come from a list of key frames (13 i32 each:
//!   the time since the previous key frame in ms, the rectangle as left,
//!   top, right, bottom, then picture, dx, dy, opacity for A and for B),
//!   moved linearly towards the next key frame; pictures switch to the
//!   target key frame's at once (always while the layer is hidden);
//! - the credits: the pictures from `first_part` on, each centred on
//!   `centre_x`, one under the other with `gaps[i]` pixels after picture
//!   i. The column is D = Σ(gap + height) + 600 high and its top is at
//!   y = 600 − D·(t − start)/(end − start): it enters at the bottom at
//!   `start` and has left the screen at `end` (setup 0x453e20, layout
//!   0x454570).
//!
//! Checked against the original under wine (roll 1): the panel's pictures
//! sit at x = dx (not relative to the rectangle), and the credits agree to
//! within 7 px (about 0.2 s of scrolling). Neither a click nor a key ends
//! the roll.

use image::{RgbImage, RgbaImage};

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;

/// One staff roll (descriptor fields named by use).
#[derive(Debug)]
pub struct Roll {
    /// The `.004` in `HD/gd.arc` with the panel pictures and the credits.
    pub file: &'static str,
    /// Index of the first credit picture (+0xc).
    pub first_part: usize,
    /// Pixels after each credit picture (+0x0, ends with -1 in the exe).
    pub gaps: &'static [i32],
    /// The credits' centre x (+0x14).
    pub centre_x: i32,
    /// When the credits start and finish scrolling, in ms (+0x18, +0x1c).
    pub start_ms: i32,
    pub end_ms: i32,
    /// When the roll ends (+0x20).
    pub total_ms: u32,
    /// The BGM track it plays (+0x28, 1-based as in 0x57).
    pub bgm: usize,
    /// Panel key frames (+0x4, ends with -1 in the exe).
    pub keys: &'static [[i32; 13]],
}

/// Indexed by 0x91's arg.
pub const ROLLS: [Roll; 2] = [
    Roll {
        file: "StaffRoll1.004",
        first_part: 1,
        gaps: &[324, 324, 108, 108, 108, 36, 108, 108, 108],
        centre_x: 400,
        start_ms: 5000,
        end_ms: 63000,
        total_ms: 63000,
        bgm: 23,
        keys: &[[100, 0, 0, 800, 600, 0, 0, 0, 0, 0, 0, 0, 256]],
    },
    Roll {
        file: "StaffRoll2.004",
        first_part: 16,
        gaps: &[360, 180, 180, 180, 180, 36, 180, 180, 180, 36, 180, 36, 180, 180, 36, 180, 180, 180, 180, 1224, 36],
        centre_x: 600,
        start_ms: 14000,
        end_ms: 222000,
        total_ms: 223000,
        bgm: 22,
        keys: &[
            [100, 215, 0, 215, 600, 0, 0, 0, 256, 1, 0, 0, 256],
            [1900, 215, 0, 215, 600, 0, 0, 0, 256, 1, 0, 0, 256],
            [12500, 30, 0, 400, 600, 2, 0, 0, 0, 1, 0, 0, 256],
            [1000, 30, 0, 400, 600, 2, -14, 0, 256, 1, 0, 0, 256],
            [10000, 30, 0, 400, 600, 2, -186, 0, 256, 3, 0, 0, 256],
            [1000, 30, 0, 400, 600, 2, -200, 0, 0, 3, -14, 0, 256],
            [11500, 30, 0, 400, 600, 4, 0, 0, 0, 3, -186, 0, 256],
            [1000, 30, 0, 400, 600, 4, -14, 0, 256, 3, -200, 0, 256],
            [13000, 30, 0, 400, 600, 4, -186, 0, 256, 5, -200, 0, 256],
            [1000, 30, 0, 400, 600, 4, -200, 0, 0, 5, -214, 0, 256],
            [11000, 30, 0, 400, 600, 6, 0, 0, 0, 5, -386, 0, 256],
            [1000, 30, 0, 400, 600, 6, -14, 0, 256, 5, -400, 0, 256],
            [11000, 30, 0, 400, 600, 6, -186, 0, 256, 7, 0, 0, 256],
            [1000, 30, 0, 400, 600, 6, -200, 0, 0, 7, -14, 0, 256],
            [11000, 30, 0, 400, 600, 8, 0, 0, 0, 7, -186, 0, 256],
            [1000, 30, 0, 400, 600, 8, -14, 0, 256, 7, -200, 0, 256],
            [14000, 30, 0, 400, 600, 8, -186, 0, 256, 9, 0, 0, 256],
            [1000, 30, 0, 400, 600, 8, -200, 0, 0, 9, -14, 0, 256],
            [14000, 30, 0, 400, 600, 10, 0, 0, 0, 9, -186, 0, 256],
            [1000, 30, 0, 400, 600, 10, -14, 0, 256, 9, -200, 0, 256],
            [13000, 30, 0, 400, 600, 10, -186, 0, 256, 11, 0, 0, 256],
            [1000, 30, 0, 400, 600, 10, -200, 0, 0, 11, -14, 0, 256],
            [13000, 30, 0, 400, 600, 12, 0, 0, 0, 11, -186, 0, 256],
            [1000, 30, 0, 400, 600, 12, -14, 0, 256, 11, -200, 0, 256],
            [15000, 30, 0, 400, 600, 12, -186, 0, 256, 13, 0, 0, 256],
            [1000, 30, 0, 400, 600, 12, -200, 0, 0, 13, -14, 0, 256],
            [12000, 30, 0, 400, 600, 14, 0, 0, 0, 13, -186, 0, 256],
            [1000, 30, 0, 400, 600, 14, -14, 0, 256, 13, -200, 0, 256],
            [12000, 30, 0, 400, 600, 14, -186, 0, 256, 15, -200, 0, 256],
            [1000, 30, 0, 400, 600, 14, -200, 0, 0, 15, -200, 0, 256],
            [2000, 30, 0, 400, 600, 14, -200, 0, 0, 15, -200, 0, 256],
            [8000, 0, 0, 800, 600, 1, 0, 0, 0, 15, 0, 0, 256],
            [5000, 0, 0, 800, 600, 1, 0, 0, 0, 15, 0, 0, 256],
            [5000, 0, 0, 800, 600, 1, 0, 0, 0, 15, 0, 0, 256],
            [10000, 0, 0, 800, 600, 1, 0, 0, 256, 15, 0, 0, 256],
        ],
    },
];

/// The panel at time `t_ms`: the key frame values, moved linearly from the
/// previous key frame; before the first key frame its values hold.
pub fn panel(keys: &[[i32; 13]], t_ms: f64) -> [i32; 13] {
    let mut at = 0.0;
    let mut prev = keys[0];
    for k in keys {
        let end = at + f64::from(k[0]);
        if t_ms <= end {
            let f = if k[0] > 0 { ((t_ms - at) / f64::from(k[0])).clamp(0.0, 1.0) } else { 1.0 };
            let mut v = *k;
            for i in [1, 2, 3, 4, 6, 7, 8, 10, 11, 12] {
                v[i] = (f64::from(prev[i]) + f64::from(k[i] - prev[i]) * f).round() as i32;
            }
            return v;
        }
        at = end;
        prev = *k;
    }
    prev
}

/// Draws the roll at `t_ms` into `frame` (cleared to black first).
/// `pictures` are the `.004`'s images in order.
pub fn render(roll: &Roll, pictures: &[RgbaImage], t_ms: f64, frame: &mut RgbImage) {
    frame.pixels_mut().for_each(|p| p.0 = [0; 3]);
    let v = panel(roll.keys, t_ms);
    let clip = (v[1].max(0), v[2].max(0), v[3].min(WIDTH as i32), v[4].min(HEIGHT as i32));
    for layer in [&v[9..13], &v[5..9]] {
        if let Some(p) = pictures.get(layer[0] as usize) {
            draw(frame, p, layer[1], layer[2], layer[3].clamp(0, 256) as u32, clip, false);
        }
    }
    let screen = (0, 0, WIDTH as i32, HEIGHT as i32);
    let parts = pictures.iter().skip(roll.first_part).take(roll.gaps.len());
    let column: i32 = roll.gaps.iter().sum::<i32>() + parts.clone().map(|p| p.height() as i32).sum::<i32>() + HEIGHT as i32;
    let scrolled = f64::from(column) * (t_ms - f64::from(roll.start_ms)) / f64::from(roll.end_ms - roll.start_ms);
    let mut y = HEIGHT as i32 - scrolled.round() as i32;
    for (p, gap) in parts.zip(roll.gaps) {
        draw(frame, p, roll.centre_x - (p.width() as i32 >> 1), y, 256, screen, true);
        y += p.height() as i32 + gap;
    }
}

/// Lays `pic` at (x, y) over `frame` inside `clip` (left, top, right,
/// bottom) at `opacity` / 256, using the picture's alpha if `alpha`.
fn draw(frame: &mut RgbImage, pic: &RgbaImage, x: i32, y: i32, opacity: u32, clip: (i32, i32, i32, i32), alpha: bool) {
    if opacity == 0 {
        return;
    }
    let (w, h) = (pic.width() as i32, pic.height() as i32);
    let (x0, x1) = (x.max(clip.0), (x + w).min(clip.2));
    let (y0, y1) = (y.max(clip.1), (y + h).min(clip.3));
    for sy in y0..y1 {
        for sx in x0..x1 {
            let s = pic.get_pixel((sx - x) as u32, (sy - y) as u32).0;
            let a = if alpha { opacity * u32::from(s[3]) / 255 } else { opacity };
            if a == 0 {
                continue;
            }
            let d = &mut frame.get_pixel_mut(sx as u32, sy as u32).0;
            for c in 0..3 {
                d[c] = ((u32::from(s[c]) * a + u32::from(d[c]) * (256 - a)) / 256) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_frames_move_linearly() {
        let keys = ROLLS[1].keys;
        // Before the first key frame and at it.
        assert_eq!(panel(keys, 0.0)[1..5], [215, 0, 215, 600]);
        // Halfway through the panel opening (2000-14500 ms).
        assert_eq!(panel(keys, 8250.0)[1..5], [123, 0, 308, 600]);
        // The first picture pans while it fades in (14500-15500 ms).
        let v = panel(keys, 15000.0);
        assert_eq!((v[5], v[6], v[8], v[9]), (2, -7, 128, 1));
        // After the last key frame: white over everything.
        assert_eq!(panel(keys, 222000.0)[5..9], [1, 0, 0, 256]);
    }

    #[test]
    fn credits_enter_at_the_bottom_and_leave_at_the_top() {
        let roll = &ROLLS[0];
        let white = |w, h| RgbaImage::from_pixel(w, h, image::Rgba([255; 4]));
        let mut pictures = vec![RgbaImage::from_pixel(800, 600, image::Rgba([0, 0, 0, 255]))];
        pictures.extend((0..9).map(|_| white(100, 20)));
        let mut frame = RgbImage::new(WIDTH, HEIGHT);
        render(roll, &pictures, f64::from(roll.start_ms), &mut frame);
        assert!(frame.pixels().all(|p| p.0 == [0; 3]), "nothing on screen yet");
        render(roll, &pictures, f64::from(roll.start_ms) + 1000.0, &mut frame);
        assert_eq!(frame.get_pixel(400, 570).0, [255; 3], "the first credit enters");
        assert_eq!(frame.get_pixel(340, 570).0, [0; 3], "centred on x 400");
        render(roll, &pictures, f64::from(roll.end_ms), &mut frame);
        assert!(frame.pixels().all(|p| p.0 == [0; 3]), "all gone");
    }
}
