//! `CreateEffect(…, "Rain")` and `"Drop"`: the exe's live particles as
//! CONscripter's `"particle"` layer (COMMANDS.md `setlayer`), one layer an
//! effect object, drawn on a picture of the effect's size that the script
//! zooms, turns and fades ([`super::sync`]).
//!
//! Both move each particle along its own path from above the top to the
//! bottom (a line, or for Drop with a swing an S curve), its rate in path
//! points per 16 ms (`00449930` at game speed 1.0), redrawn at most every
//! 42 ms, back at the start once past the end.
//!
//! Rain (`00435e60` sets it up, `00436390` draws it): `$SYSTEM_effect_rain_dencity`
//! drops (at most 128), each a grey streak 196 px tall whose alpha rises
//! and falls along it, falling one of `$SYSTEM_effect_rain_speed`, 8
//! less, …, 88 less (at least 1), from x0 = rand % w, y0 = -(rand % (h/2)
//! + 0..127) to x0 ± 0..63 at y = h.
//!
//! Drop (`00434200` sets it up, `004351c0` moves it): `$SYSTEM_effect_drop_dencity`
//! copies (at most 256) of `$SYSTEM_effect_drop_image` in 12 classes 3c + k:
//! size `SCALE[c]`, rate `BASE + trunc(COEF·$SYSTEM_effect_drop_speed)`;
//! from x0 = rand % w, y0 = -(rand % (h/2)) to x0 ± 0..127 at h + twice the
//! picture's height, swaying `$SYSTEM_effect_drop_swing`, spinning by the
//! `_rotate_x/_y/_z` settings.

use super::{Compiler, ParticleObject, Value};
use conv_core::ir::Particles;

// src: exe 00435e60 (128 drops, speed 152), 00436390 (196 px streak, 0xadadad), 0042f680 (alpha)
const RAIN_DROPS: i64 = 128;
const RAIN_SPEED: i64 = 152;
const STREAK: u32 = 196;
/// The streak's colour (`0xadadad`).
const GREY: u8 = 0xad;
// src: exe 00434200 (256 drops; scale, base and coef tables)
const DROPS: i64 = 256;
/// Drawn size of class 3c + k, ‰.
const SCALE: [i64; 4] = [1000, 400, 800, 200];
/// Rate of class 3c + k, points per 16 ms, plus `COEF`·speed.
const BASE: [i64; 12] = [4, 3, 4, 3, 2, 3, 1, 2, 2, 1, 1, 2];
const COEF: [f64; 12] = [1.0, 0.75, 1.0, 0.75, 0.5, 0.75, 0.3, 0.5, 0.5, 0.3, 0.3, 0.5];
// ours: TICKS: the layer ticks 24 times a second (41.7 ms; the exe at
// most every 42 ms, but the layer's rate is a whole number), each moving
// the exe's rate over that time.
const TICKS: i64 = 24;

/// `rate` points per 16 ms as points per tick.
fn per_tick(rate: i64) -> String {
    let v = rate as f64 * 1000.0 / 16.0 / TICKS as f64;
    format!("{}", (v * 1000.0).round() / 1000.0)
}

impl Compiler<'_, '_> {
    fn setting(&self, k: &str) -> Option<i64> {
        self.vars.get(k).filter(|v| !matches!(v, Value::Null)).map(Value::int)
    }

    /// The lowest particle layer no live effect uses.
    fn free_particle_layer(&self) -> Option<u32> {
        (2..32).find(|l| !self.objects.values().any(|o| matches!(&o.kind, super::Kind::Particles(p) if p.layer == *l)))
    }

    /// The rain for an effect of `w`×`h`, with the rain settings the
    /// script has set.
    pub(super) fn rain(&mut self, w: i64, h: i64) -> Option<ParticleObject> {
        let n = self.setting("$SYSTEM_effect_rain_dencity").map_or(RAIN_DROPS, |d| d.clamp(0, RAIN_DROPS));
        let speed = self.setting("$SYSTEM_effect_rain_speed").unwrap_or(RAIN_SPEED);
        let image = self.assets.frames_strip("rain_streak", 1, STREAK, 1, &mut |_, out| {
            for k in 0..STREAK {
                out.put_pixel(0, k, image::Rgba([GREY, GREY, GREY, alpha(k)]));
            }
        })?;
        let vy: Vec<String> = (0..12).map(|i| per_tick((speed - 8 * i).max(1))).collect();
        // ours: the drop's end x isn't kept within [0, w]; a streak cut at
        // the top loses its top rows (the exe starts its colours there)
        let keys = format!("cw=1,ch={STREAK},n={n},hz={TICKS},cls=12:0:1,vy={},x=0:{w},y=0:{}:-128,dx=-63:63,ey={h},tex={w}:{h}:0:0", vy.join(":"), -(h / 2));
        Some(ParticleObject { layer: self.free_particle_layer()?, particles: Particles { image, keys }, w, h })
    }

    /// The drops for an effect of `w`×`h` (`None`: no picture set).
    pub(super) fn drop_effect(&mut self, w: i64, h: i64) -> Option<ParticleObject> {
        let path = self.vars.get("$SYSTEM_effect_drop_image").map(Value::string)?;
        let (image, cw, ch) = self.assets.image(&path)?;
        let n = self.setting("$SYSTEM_effect_drop_dencity").map_or(DROPS, |d| d.clamp(0, DROPS));
        let speed = self.setting("$SYSTEM_effect_drop_speed");
        let vy: Vec<String> = (0..12).map(|i| per_tick(BASE[i] + speed.map_or(0, |s| (COEF[i] * s as f64) as i64))).collect();
        let cscale: Vec<String> = (0..12).map(|i| SCALE[i / 3].to_string()).collect();
        let mut keys = format!("cw={cw},ch={ch},n={n},hz={TICKS},cls=12:0:1,vy={},cscale={},x=0:{w},y=0:{},dx=-127:127,ey={},tex={w}:{h}:0:0", vy.join(":"), cscale.join(":"), -(h / 2), h + 2 * i64::from(ch));
        // ours: swing: the layer's S curve progresses by y, the exe's
        // Bezier (`FUN_00414260`) by a point count the decompile lost
        if let Some(s) = self.setting("$SYSTEM_effect_drop_swing").filter(|&s| s != 0) {
            keys += &format!(",swing={s}");
        }
        let spin = ["x", "y", "z"].map(|a| self.setting(&format!("$SYSTEM_effect_drop_rotate_{a}")).unwrap_or(0));
        if spin.iter().any(|&m| m > 0) {
            keys += &format!(",spin={}:{}:{}:1", spin[0], spin[1], spin[2]);
        }
        Some(ParticleObject { layer: self.free_particle_layer()?, particles: Particles { image, keys }, w, h })
    }
}

/// The streak's alpha `k` px down it: 20 + floor(8k/7) to 130 at row 97,
/// then down by 1 to 32 (`0042f680`, research/nitroplus/rain_exe.py).
fn alpha(k: u32) -> u8 {
    let v = if k < 98 { 20 + k * 8 / 7 } else { 130 - (k - 97) };
    v as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_streak_brightens_then_fades() {
        assert_eq!((alpha(0), alpha(7), alpha(97), alpha(98), alpha(STREAK - 1)), (20, 28, 130, 129, 32));
    }

    #[test]
    fn rates_per_tick() {
        // 128 points a 16 ms: 8000 a second, 333.333 a 24th.
        assert_eq!(per_tick(128), "333.333");
        assert_eq!(per_tick(3), "7.813");
    }
}
