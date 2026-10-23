//! `CreateEffect(…, "Rain")`: the exe's rain (`00435e60` sets it up,
//! `00436390` draws it) as a looping strip of frames.
//!
//! The exe draws into a texture of the effect's size, at most every 42 ms:
//! `$SYSTEM_effect_rain_dencity` drops (at most 128) each fall along a
//! line from above the top to the bottom, a little aslant, and each is
//! drawn as a grey streak 196 px tall whose alpha rises and falls along it.
//! A drop moves its speed (one of `$SYSTEM_effect_rain_speed`, 8 less, …,
//! 88 less, picked per drop) in pixels per 16 ms, and starts again at the
//! top once past the bottom. The script zooms and turns the texture as a
//! picture ([`super::sync`]).

use super::{Compiler, StripObject, Value};

// src: exe 00435e60 (setup: 128 drops, speed 0x98), 00436390 (draw: 42 ms, 196 px streak, 0xadadad), 0042f680 (alpha)
/// The exe redraws the rain at most this often, ms.
const TICK_MS: u32 = 42;
/// The exe divides by its frame period.
// src: exe init_frame_tables 0048c5d0 (ms per frame 005dcb0c[4] = 16, index 005dcaf0 = 4)
const FRAME_MS: i64 = 16;
// ours: CELLS, WARM_UP: our looped strip of pre-drawn rain (the exe
// draws its rain live)
/// Frames in the strip: a second of rain, looped.
const CELLS: u32 = 24;
/// Frames run before the strip's first, so the drops aren't all at the top.
const WARM_UP: u32 = 100;
const DROPS: usize = 128;
const STREAK: usize = 196;
/// The streak's colour (`0xadadad`).
const GREY: u8 = 0xad;

impl Compiler<'_, '_> {
    /// The rain strip for an effect of `w`×`h`, with the rain settings the
    /// script has set.
    pub(super) fn rain(&mut self, w: i64, h: i64) -> Option<StripObject> {
        let var = |k: &str| self.vars.get(k).map(Value::int);
        let density = var("$SYSTEM_effect_rain_dencity").map_or(DROPS, |d| d.clamp(0, DROPS as i64) as usize);
        let speed = var("$SYSTEM_effect_rain_speed").unwrap_or(152);
        let (wu, hu) = (u32::try_from(w).ok()?, u32::try_from(h).ok()?);
        let frames = heads(w, h, density, speed);
        let image = self.assets.frames_strip(&format!("rain_{w}x{h}_{density}_{speed}"), wu, hu, CELLS, &mut |i, out| draw(&frames[i as usize], out))?;
        Some(StripObject { last: image.clone(), image, cells: CELLS, frame_ms: TICK_MS, w, h, zoom: (100, 100), looped: true })
    }
}

/// Where each drop's head is in each frame of the strip.
fn heads(w: i64, h: i64, density: usize, speed: i64) -> Vec<Vec<(i64, i64)>> {
    // The script's rain looks the same in every build: a fixed seed.
    let mut seed = (w * 31 + h) as u64 * 7919 + density as u64 * 131 + speed as u64;
    let mut rand = move |n: i64| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as i64).rem_euclid(n.max(1))
    };
    // (x0, y0, x at the bottom, speed, step)
    let mut drops: Vec<(i64, i64, i64, i64, i64)> = (0..density)
        .map(|_| {
            let x0 = rand(w);
            let y0 = -(rand(h / 2) + rand(128));
            let x1 = (x0 + if rand(2) == 0 { -rand(64) } else { rand(64) }).clamp(0, w);
            (x0, y0, x1, (speed - 8 * rand(12)).max(1), 0)
        })
        .collect();
    let per_tick = i64::from(TICK_MS) * 65536 / FRAME_MS;
    let mut out = Vec::new();
    for tick in 0..WARM_UP + CELLS {
        if tick >= WARM_UP {
            out.push(drops.iter().map(|&(x0, y0, x1, _, s)| (x0 + (x1 - x0) * (s >> 16) / (h - y0), y0 + (s >> 16))).collect());
        }
        for d in &mut drops {
            d.4 += per_tick * d.3;
            if d.4 >> 16 >= h - d.1 {
                d.4 = 0;
            }
        }
    }
    out
}

/// One frame: a streak down from each head.
fn draw(heads: &[(i64, i64)], out: &mut image::RgbaImage) {
    let (w, h) = (i64::from(out.width()), i64::from(out.height()));
    for &(x, y) in heads {
        if !(0..w).contains(&x) || y <= -(STREAK as i64) {
            continue;
        }
        // (Cut at the top, the exe starts the streak's colours there.)
        let top = y.max(0);
        for (k, row) in (top..(y + STREAK as i64).min(h)).enumerate() {
            out.put_pixel(x as u32, row as u32, image::Rgba([GREY, GREY, GREY, alpha(k)]));
        }
    }
}

/// The streak's alpha `k` px down it: up by 8/7 a pixel for 98 px, then
/// down by 1 (`0042f680`).
fn alpha(k: usize) -> u8 {
    let v = if k < 98 { (k as f64 * 8.0 / 7.0).round() as usize } else { 111 - (k - 97) };
    (v + 20) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_streak_brightens_then_fades() {
        assert_eq!((alpha(0), alpha(97), alpha(98), alpha(STREAK - 1)), (20, 131, 130, 33));
    }

    #[test]
    fn drops_fall_and_start_again() {
        let f = heads(512, 288, 16, 128);
        assert_eq!((f.len(), f[0].len()), (CELLS as usize, 16));
        // 128 px a 16 ms: 336 px a frame, or back at the top.
        let (a, b) = (f[0][0].1, f[1][0].1);
        assert!(b - a == 336 || b < a, "{a} → {b}");
        assert!(f.iter().flatten().all(|&(x, y)| (0..=512).contains(&x) && y < 288), "{f:?}");
    }
}
