//! Sprites moving by themselves ([`Op::SpriteMotion`]) with CONscripter's
//! `spmotion`, which the engine steps during text and click waits too.

use std::fmt::Write;

use conv_core::ir::{MotionEnd, MotionKey};

use crate::SPRITE_BASE;

/// One `spmotion` key: ms, offset, zoom (x and y), angle, alpha 0-256,
/// reached in a straight line (curve 0).
fn key(ms: u32, x: i32, y: i32, zoom: u32, angle: i32, alpha: u32) -> String {
    format!("{ms},{x},{y},{zoom},{zoom},{angle},{},0", alpha.min(256))
}

/// `spmotion` keys that change only the alpha (glows, flashes).
pub(crate) fn alpha_keys(keys: &[(u32, u32)]) -> String {
    keys.iter().map(|&(t, a)| key(t, 0, 0, 100, 0, a)).collect::<Vec<_>>().join(",")
}

/// `spmotion`'s loop for an alpha run `times` times (0: for ever).
pub(crate) fn alpha_loop(times: u32) -> i64 {
    match times {
        0 => 1,
        1 => 2,
        n => -i64::from(n),
    }
}

/// Writes a motion of layer `layer` (no keys: stops it).
pub(crate) fn emit(out: &mut String, layer: u32, keys: &[(u32, i32, i32)], looped: bool) {
    let slot = SPRITE_BASE + layer;
    if keys.is_empty() {
        writeln!(out, "spmotion {slot}").unwrap();
        return;
    }
    let keys: Vec<String> = keys.iter().map(|&(t, x, y)| key(t, x, y, 100, 0, 256)).collect();
    writeln!(out, "spmotion {slot},{},{}", u8::from(looped), keys.join(",")).unwrap();
}

/// Writes an [`conv_core::ir::Op::SpriteMotion2`] with `spmotion`.
pub(crate) fn emit2(out: &mut String, layer: u32, keys: &[MotionKey], end: MotionEnd) {
    let slot = SPRITE_BASE + layer;
    let mode = match end {
        MotionEnd::Back => 0,
        MotionEnd::Loop => 1,
        MotionEnd::Keep => 2,
    };
    let keys: Vec<String> = keys.iter().map(|k| key(k.ms, k.x, k.y, k.zoom, k.angle, k.opacity)).collect();
    writeln!(out, "spmotion {slot},{mode},{}", keys.join(",")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_keys_or_stops() {
        let mut out = String::new();
        emit(&mut out, 5, &[(0, 0, 0), (250, 0, -12), (500, 0, 0)], true);
        emit(&mut out, 5, &[], false);
        assert_eq!(out, "spmotion 67,1,0,0,0,100,100,0,256,0,250,0,-12,100,100,0,256,0,500,0,0,100,100,0,256,0\nspmotion 67\n");
    }

    #[test]
    fn writes_zoom_angle_and_opacity_keys() {
        let key = |ms, x, zoom, angle, opacity| MotionKey { ms, x, y: 0, zoom, angle, opacity };
        let mut out = String::new();
        emit2(&mut out, 5, &[key(0, 0, 100, 0, 256), key(500, 300, 150, -45, 0)], MotionEnd::Keep);
        assert_eq!(out, "spmotion 67,2,0,0,0,100,100,0,256,0,500,300,0,150,150,-45,0,0\n");
    }

    #[test]
    fn alpha_keys_and_loops() {
        assert_eq!(alpha_keys(&[(0, 76), (97, 300)]), "0,0,0,100,100,0,76,0,97,0,0,100,100,0,256,0");
        assert_eq!((alpha_loop(0), alpha_loop(1), alpha_loop(60)), (1, 2, -60));
    }
}
