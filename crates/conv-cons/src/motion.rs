//! Sprites moving by themselves ([`Op::SpriteMotion`]) with CONscripter's
//! `spmotion`, which the engine steps during text and click waits too.

use std::fmt::Write;

use conv_core::ir::{MotionEnd, MotionKey};

use crate::SPRITE_BASE;

/// Writes a motion of layer `layer` (no keys: stops it).
pub(crate) fn emit(out: &mut String, layer: u32, keys: &[(u32, i32, i32)], looped: bool) {
    let slot = SPRITE_BASE + layer;
    if keys.is_empty() {
        writeln!(out, "spmotion {slot}").unwrap();
        return;
    }
    let keys: Vec<String> = keys.iter().map(|(t, x, y)| format!("{t},{x},{y}")).collect();
    writeln!(out, "spmotion {slot},{},{}", u8::from(looped), keys.join(",")).unwrap();
}

/// Writes an [`conv_core::ir::Op::SpriteMotion2`] with `spmotion2`.
pub(crate) fn emit2(out: &mut String, layer: u32, keys: &[MotionKey], end: MotionEnd) {
    let slot = SPRITE_BASE + layer;
    let mode = match end {
        MotionEnd::Back => 0,
        MotionEnd::Loop => 1,
        MotionEnd::Keep => 2,
    };
    let keys: Vec<String> = keys.iter().map(|k| format!("{},{},{},{},{},{}", k.ms, k.x, k.y, k.zoom, k.angle, k.opacity.min(256))).collect();
    writeln!(out, "spmotion2 {slot},{mode},{}", keys.join(",")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_keys_or_stops() {
        let mut out = String::new();
        emit(&mut out, 5, &[(0, 0, 0), (250, 0, -12), (500, 0, 0)], true);
        emit(&mut out, 5, &[], false);
        assert_eq!(out, "spmotion 67,1,0,0,0,250,0,-12,500,0,0\nspmotion 67\n");
    }

    #[test]
    fn writes_zoom_angle_and_opacity_keys() {
        let key = |ms, x, zoom, angle, opacity| MotionKey { ms, x, y: 0, zoom, angle, opacity };
        let mut out = String::new();
        emit2(&mut out, 5, &[key(0, 0, 100, 0, 256), key(500, 300, 150, -45, 0)], MotionEnd::Keep);
        assert_eq!(out, "spmotion2 67,2,0,0,0,100,0,256,500,300,0,150,-45,0\n");
    }
}
