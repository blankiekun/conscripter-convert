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

/// Moves sprite `slot`'s centre to (`x`, `y`) over `ms`, zooming it from
/// `zoom.0` to `zoom.1` percent (spzoom) when given, along `curve` (0
/// straight, or an `spcurve` id), with one `spmotion` that keeps its last
/// key, and waits for it: the engine steps it every frame. A looping
/// motion the sprite had ends. Uses %1-%4.
pub(crate) fn move_to(out: &mut String, slot: u32, x: i32, y: i32, ms: u32, zoom: Option<(u32, u32)>, curve: u32) {
    let (from, to) = match zoom {
        Some((from, to)) if from > 0 => {
            write!(out, "spzoom {slot},{from}:").unwrap();
            (100, to * 100 / from)
        }
        Some((_, to)) => {
            write!(out, "spzoom {slot},{to}:").unwrap();
            (100, 100)
        }
        None => (100, 100),
    };
    writeln!(out, "getspsize {slot},%1,%2:getsppos {slot},%3,%4").unwrap();
    writeln!(out, "spmotion {slot},2,0,0,0,{from},{from},0,256,0,{ms},{x}-%1/2-%3,{y}-%2/2-%4,{to},{to},0,256,{curve}:spmotionwait {slot}").unwrap();
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
    fn moves_and_zooms_with_one_motion() {
        let mut out = String::new();
        move_to(&mut out, 67, 400, 300, 500, None, 0);
        assert_eq!(out, "getspsize 67,%1,%2:getsppos 67,%3,%4\nspmotion 67,2,0,0,0,100,100,0,256,0,500,400-%1/2-%3,300-%2/2-%4,100,100,0,256,0:spmotionwait 67\n");
        let mut out = String::new();
        move_to(&mut out, 67, 400, 300, 500, Some((200, 100)), 51);
        assert!(out.starts_with("spzoom 67,200:getspsize"), "{out}");
        assert!(out.contains(",500,400-%1/2-%3,300-%2/2-%4,50,50,0,256,51:"), "{out}");
    }

    #[test]
    fn alpha_keys_and_loops() {
        assert_eq!(alpha_keys(&[(0, 76), (97, 300)]), "0,0,0,100,100,0,76,0,97,0,0,100,100,0,256,0");
        assert_eq!((alpha_loop(0), alpha_loop(1), alpha_loop(60)), (1, 2, -60));
    }
}
