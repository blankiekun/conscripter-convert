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

/// A fade in over `ms` from alpha 0 to the sprite's own (a loop 0 motion).
pub(crate) fn fade_in(slot: u32, ms: u32) -> String {
    format!("spmotion {slot},0,{},{}", key(0, 0, 0, 100, 0, 0), key(ms, 0, 0, 100, 0, 256))
}

/// A fade out over `ms`, then the sprite is removed (loop 3).
pub(crate) fn fade_out(slot: u32, ms: u32) -> String {
    format!("spmotion {slot},3,{},{}", key(0, 0, 0, 100, 0, 256), key(ms, 0, 0, 100, 0, 0))
}

/// The alpha (0-256) at `t` ms of a fade in over `fade` ms.
fn faded(t: u32, fade: u32) -> u32 {
    t.min(fade) * 256 / fade.max(1)
}

/// Folds a fade in over `fade` ms that starts with the motion into its
/// keys, as a sprite has one motion: a key where the fade ends (straight
/// between its neighbours, or the last key held), each key's alpha times
/// the fade's then.
fn fold_fade(keys: &mut Vec<MotionKey>, fade: u32) {
    match keys.iter().position(|k| k.ms >= fade) {
        Some(i) if i > 0 && keys[i].ms > fade => {
            let (a, b) = (keys[i - 1], keys[i]);
            let f = |p: i64, q: i64| p + (q - p) * i64::from(fade - a.ms) / i64::from(b.ms - a.ms);
            let k = MotionKey {
                ms: fade,
                x: f(a.x.into(), b.x.into()) as i32,
                y: f(a.y.into(), b.y.into()) as i32,
                zoom: f(a.zoom.into(), b.zoom.into()) as u32,
                angle: f(a.angle.into(), b.angle.into()) as i32,
                opacity: f(a.opacity.into(), b.opacity.into()) as u32,
            };
            keys.insert(i, k);
        }
        Some(_) => {}
        None => {
            if let Some(&last) = keys.last() {
                keys.push(MotionKey { ms: fade, ..last });
            }
        }
    }
    for k in keys.iter_mut() {
        k.opacity = k.opacity.min(256) * faded(k.ms, fade) / 256;
    }
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
/// key, and waits for it unless `wait` is false: the engine steps it
/// every frame. A looping
/// motion the sprite had ends. A fade in over `fade` ms started with it
/// goes into its alpha. Uses %1-%4.
#[allow(clippy::too_many_arguments)]
pub(crate) fn move_to(out: &mut String, slot: u32, x: i32, y: i32, ms: u32, zoom: Option<(u32, u32)>, curve: u32, fade: Option<u32>, wait: bool) {
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
    // shortcut: a fade goes in at the two keys only (the place is an expression), so it lasts as long as the move; add a key between when a game's fades must keep their own length.
    let (a0, a1) = fade.map_or((256, 256), |f| (0, faded(ms, f)));
    writeln!(out, "getspsize {slot},%1,%2:getsppos {slot},%3,%4").unwrap();
    write!(out, "spmotion {slot},2,0,0,0,{from},{from},0,{a0},0,{ms},{x}-%1/2-%3,{y}-%2/2-%4,{to},{to},0,{a1},{curve}").unwrap();
    if wait {
        write!(out, ":spmotionwait {slot}").unwrap();
    }
    writeln!(out).unwrap();
}

/// Writes a motion of layer `layer` (no keys: stops it). A fade in over
/// `fade` ms started with it goes into its keys.
pub(crate) fn emit(out: &mut String, layer: u32, keys: &[(u32, i32, i32)], looped: bool, fade: Option<u32>) {
    if keys.is_empty() {
        writeln!(out, "spmotion {}", SPRITE_BASE + layer).unwrap();
        return;
    }
    let keys: Vec<MotionKey> = keys.iter().map(|&(ms, x, y)| MotionKey { ms, x, y, zoom: 100, angle: 0, opacity: 256 }).collect();
    emit2(out, layer, &keys, if looped { MotionEnd::Loop } else { MotionEnd::Back }, fade, 0);
}

/// Writes an [`conv_core::ir::Op::SpriteMotion2`] with `spmotion`. A fade
/// in over `fade` ms started with it goes into its keys; each key is
/// reached along `curve` (0 straight, or an `spcurve` id).
pub(crate) fn emit2(out: &mut String, layer: u32, keys: &[MotionKey], end: MotionEnd, fade: Option<u32>, curve: u32) {
    let slot = SPRITE_BASE + layer;
    let mode = match end {
        MotionEnd::Back => 0,
        MotionEnd::Loop => 1,
        MotionEnd::Keep => 2,
    };
    let mut keys = keys.to_vec();
    // shortcut: a looping motion would repeat the fade, so the sprite shows at once; fold it into a first pass when a game needs it.
    if let Some(fade) = fade.filter(|_| end != MotionEnd::Loop) {
        fold_fade(&mut keys, fade);
    }
    let keys: Vec<String> = keys.iter().map(|k| key(k.ms, k.x, k.y, k.zoom, k.angle, k.opacity)).collect();
    // (The first key's curve is unused: the motion starts there.)
    let keys: Vec<String> = keys.into_iter().map(|k| if curve == 0 { k } else { format!("{},{curve}", k.strip_suffix(",0").unwrap()) }).collect();
    writeln!(out, "spmotion {slot},{mode},{}", keys.join(",")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_keys_or_stops() {
        let mut out = String::new();
        emit(&mut out, 5, &[(0, 0, 0), (250, 0, -12), (500, 0, 0)], true, None);
        emit(&mut out, 5, &[], false, None);
        assert_eq!(out, "spmotion 67,1,0,0,0,100,100,0,256,0,250,0,-12,100,100,0,256,0,500,0,0,100,100,0,256,0\nspmotion 67\n");
    }

    #[test]
    fn writes_zoom_angle_and_opacity_keys() {
        let key = |ms, x, zoom, angle, opacity| MotionKey { ms, x, y: 0, zoom, angle, opacity };
        let mut out = String::new();
        emit2(&mut out, 5, &[key(0, 0, 100, 0, 256), key(500, 300, 150, -45, 0)], MotionEnd::Keep, None, 0);
        assert_eq!(out, "spmotion 67,2,0,0,0,100,100,0,256,0,500,300,0,150,150,-45,0,0\n");
        let mut out = String::new();
        emit2(&mut out, 5, &[key(0, 0, 100, 0, 256), key(500, 300, 100, 0, 256)], MotionEnd::Keep, None, 72);
        assert_eq!(out, "spmotion 67,2,0,0,0,100,100,0,256,72,500,300,0,100,100,0,256,72\n");
    }

    #[test]
    fn moves_and_zooms_with_one_motion() {
        let mut out = String::new();
        move_to(&mut out, 67, 400, 300, 500, None, 0, None, true);
        assert_eq!(out, "getspsize 67,%1,%2:getsppos 67,%3,%4\nspmotion 67,2,0,0,0,100,100,0,256,0,500,400-%1/2-%3,300-%2/2-%4,100,100,0,256,0:spmotionwait 67\n");
        let mut out = String::new();
        move_to(&mut out, 67, 400, 300, 500, Some((200, 100)), 51, None, true);
        assert!(out.starts_with("spzoom 67,200:getspsize"), "{out}");
        assert!(out.contains(",500,400-%1/2-%3,300-%2/2-%4,50,50,0,256,51:"), "{out}");
    }

    #[test]
    fn fades_fold_into_the_motion() {
        assert_eq!(fade_in(60, 2000), "spmotion 60,0,0,0,0,100,100,0,0,0,2000,0,0,100,100,0,256,0");
        assert_eq!(fade_out(60, 300), "spmotion 60,3,0,0,0,100,100,0,256,0,300,0,0,100,100,0,0,0");
        // a 300 ms fade over a hop: a key at 300 between 183 and 383, alphas rising to it
        let mut out = String::new();
        emit(&mut out, 5, &[(0, 0, 0), (83, 0, 37), (183, 0, 0), (383, 0, 0), (583, 0, 37)], false, Some(300));
        assert_eq!(out, "spmotion 67,0,0,0,0,100,100,0,0,0,83,0,37,100,100,0,70,0,183,0,0,100,100,0,156,0,300,0,0,100,100,0,256,0,383,0,0,100,100,0,256,0,583,0,37,100,100,0,256,0\n");
        // a key's own alpha is kept times the fade's; a fade past the last key holds it
        let key = |ms, x, opacity| MotionKey { ms, x, y: 0, zoom: 100, angle: 0, opacity };
        let mut out = String::new();
        emit2(&mut out, 5, &[key(0, 0, 256), key(100, 50, 128)], MotionEnd::Keep, Some(200), 0);
        assert_eq!(out, "spmotion 67,2,0,0,0,100,100,0,0,0,100,50,0,100,100,0,64,0,200,50,0,100,100,0,128,0\n");
        // a loop isn't folded
        let mut out = String::new();
        emit(&mut out, 5, &[(0, 0, 0), (100, 0, 5)], true, Some(300));
        assert!(out.ends_with(",256,0\n") && out.contains(",0,256,0,100,"), "{out}");
        let mut out = String::new();
        move_to(&mut out, 67, 400, 300, 600, None, 0, Some(300), true);
        assert!(out.contains(",100,100,0,0,0,600,400-%1/2-%3,300-%2/2-%4,100,100,0,256,0:"), "{out}");
    }

    #[test]
    fn alpha_keys_and_loops() {
        assert_eq!(alpha_keys(&[(0, 76), (97, 300)]), "0,0,0,100,100,0,76,0,97,0,0,100,100,0,256,0");
        assert_eq!((alpha_loop(0), alpha_loop(1), alpha_loop(60)), (1, 2, -60));
    }
}
