//! Sprites moving by themselves ([`Op::SpriteMotion`]) with CONscripter's
//! `spmotion`, which the engine steps during text and click waits too.

use std::fmt::Write;

use conv_core::ir::{MotionEnd, MotionKey};

use crate::SPRITE_BASE;

/// One `spmotion` key: ms, offset, zoom (x and y), angle, alpha 0-256,
/// reached in a straight line (curve 0).
fn key(ms: u32, x: i32, y: i32, zoom: u32, angle: i32, alpha: u32) -> String {
    key2(ms, x, y, (zoom, zoom), angle, alpha)
}

/// [`key`] with its zoom across and up and down.
fn key2(ms: u32, x: i32, y: i32, (zx, zy): (u32, u32), angle: i32, alpha: u32) -> String {
    format!("{ms},{x},{y},{zx},{zy},{angle},{},0", alpha.min(256))
}

/// A fade in over `ms` from alpha 0 to the sprite's own (a loop 0 motion).
pub(crate) fn fade_in(slot: impl std::fmt::Display, ms: u32) -> String {
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
                zoom_y: f(a.zoom_y.into(), b.zoom_y.into()) as u32,
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
    let (from, to) = start_zoom(out, slot, zoom);
    // shortcut: a fade goes in at the two keys only (the place is an expression), so it lasts as long as the move; add a key between when a game's fades must keep their own length.
    let (a0, a1) = fade.map_or((256, 256), |f| (0, faded(ms, f)));
    writeln!(out, "getspsize {slot},%1,%2:getsppos {slot},%3,%4").unwrap();
    write!(out, "spmotion {slot},2,0,0,0,{from},{from},0,{a0},0,{ms},{x}-%1/2-%3,{y}-%2/2-%4,{to},{to},0,{a1},{curve}").unwrap();
    if wait {
        write!(out, ":spmotionwait {slot}").unwrap();
    }
    writeln!(out).unwrap();
}

/// Sets the zoom a move starts from (spzoom); the motion's zoom keys
/// (percent of it) at its start and end.
fn start_zoom(out: &mut String, slot: u32, zoom: Option<(u32, u32)>) -> (u32, u32) {
    match zoom {
        Some((from, to)) if from > 0 => {
            write!(out, "spzoom {slot},{from}:").unwrap();
            (100, to * 100 / from)
        }
        Some((_, to)) => {
            write!(out, "spzoom {slot},{to}:").unwrap();
            (100, 100)
        }
        None => (100, 100),
    }
}

/// As [`move_to`], along an arc ([`conv_core::ir::Op::MoveArc`]) that
/// bows by `bow`/1000: a key every frame (60 a second), the place eased
/// by `ease` ([`crate::ease::eased`]), the zoom and the fade straight
/// with the time. The move (the offset to its end) goes in %3, %4.
#[allow(clippy::too_many_arguments)]
pub(crate) fn arc_to(out: &mut String, slot: u32, x: i32, y: i32, ms: u32, zoom: Option<(u32, u32)>, ease: Option<(u8, u32)>, bow: i32, fade: Option<u32>, wait: bool) {
    let (from, to) = start_zoom(out, slot, zoom);
    let (a0, a1) = fade.map_or((256, 256), |f| (0, faded(ms, f)));
    writeln!(out, "getspsize {slot},%1,%2:getsppos {slot},%3,%4:mov %3,{x}-%1/2-%3:mov %4,{y}-%2/2-%4").unwrap();
    // %v times c/1024, signed.
    let term = |v: u32, c: i64| if c == 0 { String::new() } else { format!("{}%{v}*{}/1024", if c < 0 { "-" } else { "+" }, c.abs()) };
    let n = (ms * 60 / 1000).clamp(2, 63);
    let mut keys = vec![format!("0,0,0,{from},{from},0,{a0},0")];
    for i in 1..=n {
        let u = f64::from(i) / f64::from(n);
        let t = match ease {
            Some((e, p)) if e > 0 => crate::ease::eased(e, p, u),
            _ => u,
        };
        // The bow across the move, of its length: perpendicular (dy, -dx).
        // (The screen's 3/4 × 544/720 scaling bends it by under 1 %.)
        let h = f64::from(bow) / 1000.0 * (std::f64::consts::PI * t).sin() / 2.0;
        let (a, b) = ((t * 1024.0).round() as i64, (h * 1024.0).round() as i64);
        let ms_i = ms * i / n;
        let z = (i64::from(from) + (i64::from(to) - i64::from(from)) * i64::from(i) / i64::from(n)) as u32;
        let alpha = (i64::from(a0) + (i64::from(a1) - i64::from(a0)) * i64::from(i) / i64::from(n)) as u32;
        let (kx, ky) = (format!("{}{}", term(3, a), term(4, b)), format!("{}{}", term(4, a), term(3, -b)));
        let k = |s: &str| if s.is_empty() { "0".to_string() } else { s.trim_start_matches('+').to_string() };
        keys.push(format!("{ms_i},{},{},{z},{z},0,{alpha},0", k(&kx), k(&ky)));
    }
    write!(out, "spmotion {slot},2,{}", keys.join(",")).unwrap();
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
    let keys: Vec<MotionKey> = keys.iter().map(|&(ms, x, y)| MotionKey { ms, x, y, zoom: 100, angle: 0, opacity: 256, zoom_y: 100 }).collect();
    emit2(out, layer, &keys, if looped { MotionEnd::Loop } else { MotionEnd::Back }, fade, 0);
}

/// Writes an [`conv_core::ir::Op::SpriteMotion2`] with `spmotion`. A fade
/// in over `fade` ms started with it goes into its keys; each key is
/// reached along `curve` (0 straight, or an `spcurve` id).
pub(crate) fn emit2(out: &mut String, layer: u32, keys: &[MotionKey], end: MotionEnd, fade: Option<u32>, curve: u32) {
    let slot = SPRITE_BASE + layer;
    let mode = match end {
        MotionEnd::Back => "0".to_string(),
        MotionEnd::Loop => "1".to_string(),
        MotionEnd::Keep => "2".to_string(),
        MotionEnd::LoopFrom(k) => format!("4,{k}"),
    };
    let mut keys = keys.to_vec();
    // shortcut: a looping motion would repeat the fade, so the sprite shows at once; fold it into a first pass when a game needs it.
    if let Some(fade) = fade.filter(|_| !matches!(end, MotionEnd::Loop | MotionEnd::LoopFrom(_))) {
        fold_fade(&mut keys, fade);
    }
    let keys: Vec<String> = keys.iter().map(|k| key2(k.ms, k.x, k.y, (k.zoom, k.zoom_y), k.angle, k.opacity)).collect();
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
        let key = |ms, x, zoom, angle, opacity| MotionKey { ms, x, y: 0, zoom, angle, opacity, zoom_y: zoom };
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
    fn arcs_bow_across_the_move() {
        let mut out = String::new();
        arc_to(&mut out, 67, 400, 300, 200, Some((100, 250)), None, 3000, None, true);
        assert!(out.starts_with("spzoom 67,100:getspsize 67,%1,%2:getsppos 67,%3,%4:mov %3,400-%1/2-%3:mov %4,300-%2/2-%4\nspmotion 67,2,0,0,0,100,100,0,256,0,"), "{out}");
        // 12 frames; halfway the bow is 3/2 of the move across: (dy, -dx) × 1536
        assert!(out.contains(",100,%3*512/1024+%4*1536/1024,%4*512/1024-%3*1536/1024,175,175,0,256,0,"), "{out}");
        // ends on the straight line's end, zoomed
        assert!(out.ends_with(",200,%3*1024/1024,%4*1024/1024,250,250,0,256,0:spmotionwait 67\n"), "{out}");
        assert_eq!(out.matches(",0,256,0").count(), 13);
        // bow 0: eased place, straight zoom
        let mut out = String::new();
        arc_to(&mut out, 67, 400, 300, 200, Some((100, 200)), Some((1, 2)), 0, None, true);
        assert!(out.contains(",100,%3*256/1024,%4*256/1024,150,150,0,256,0,"), "{out}");
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
        let key = |ms, x, opacity| MotionKey { ms, x, y: 0, zoom: 100, angle: 0, opacity, zoom_y: 100 };
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

    #[test]
    fn a_loop_after_a_lead_in_names_its_first_key() {
        let k = |ms, y| MotionKey { ms, x: 0, y, zoom: 100, angle: 0, opacity: 256, zoom_y: 100 };
        let mut out = String::new();
        emit2(&mut out, 5, &[k(0, -40), k(500, 0), k(1500, 6), k(2500, 0)], MotionEnd::LoopFrom(1), Some(300), 0);
        // (A fade isn't folded into a loop.)
        assert_eq!(out, format!("spmotion {},4,1,0,0,-40,100,100,0,256,0,500,0,0,100,100,0,256,0,1500,0,6,100,100,0,256,0,2500,0,0,100,100,0,256,0\n", SPRITE_BASE + 5));
    }
}
