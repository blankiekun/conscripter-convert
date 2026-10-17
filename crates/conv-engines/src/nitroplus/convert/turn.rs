//! Pictures turned about x or y (`Rotate`): drawn squashed across (a turn
//! about y) or up and down (about x) by |cos| of the angle, about the
//! turned object's vertex (or centre), its children with it
//! (`Game::relative_children`); no perspective. A turn over time
//! ([`Turning`]) is an `spmotion` through the squashes on the way: a face
//! window turning in from edge-on unfolds.

use super::*;

/// A `Rotate` about x or y over `ms` of `target`, pending for the target
/// and each of its children.
#[derive(Debug, Clone)]
pub(super) struct Turning {
    pub target: String,
    pub from: (i64, i64),
    pub to: (i64, i64),
    pub ms: u32,
    pub ease: Option<(u8, u32)>,
}

/// Per axis (scale, offset): drawn = scale × place + offset.
pub(super) type Squash = [(f64, f64); 2];

/// No squash.
pub(super) const FLAT: Squash = [(1.0, 0.0); 2];

/// The eased share (0-1) of the time gone `t` (0-1), as conv-cons'
/// `ease::eased` (the same curves, [`Op::MoveEase`]).
fn eased(ease: Option<(u8, u32)>, t: f64) -> f64 {
    let Some((ease, exponent)) = ease else { return t };
    let n = exponent.clamp(1, 20) as i32;
    let pi = std::f64::consts::PI;
    match ease {
        4 => 1.0 - (pi * t / 2.0).cos(),
        5 => (pi * t / 2.0).sin(),
        6 => 0.5 - (pi * t).cos() / 2.0,
        7 => (1.0 - 2.0 * t).acos() / pi,
        1 => t.powi(n),
        2 => 1.0 - (1.0 - t).powi(n),
        _ if t < 0.5 => (2.0 * t).powi(n) / 2.0,
        _ => 1.0 - (2.0 - 2.0 * t).powi(n) / 2.0,
    }
}

fn centre(s: &Shown) -> [f64; 2] {
    [(s.x + s.w / 2) as f64, (s.y + s.h / 2) as f64]
}

impl Compiler<'_, '_> {
    /// How `name`'s own turn and its parents' squash it, `over` taking the
    /// place of one object's turn (x, y).
    pub(super) fn squash(&self, name: &str, over: Option<(&str, (i64, i64))>) -> Squash {
        let mut names = vec![name];
        if self.game.relative_children {
            names.extend(name.rmatch_indices('/').map(|(i, _)| &name[..i]));
        }
        let mut s = FLAT;
        for m in names {
            let Some(o) = self.objects.get(m) else { continue };
            let (tx, ty) = match over {
                Some((t, turn)) if t == m => turn,
                _ => (o.turn.0, o.turn.1),
            };
            if tx.rem_euclid(360) == 0 && ty.rem_euclid(360) == 0 {
                continue;
            }
            let (px, py) = self.parent_offset(m);
            let (vx, vy) = o.vertex.unwrap_or((o.w / 2, o.h / 2));
            let p = [(o.x + px + vx) as f64, (o.y + py + vy) as f64];
            let f = [(ty as f64).to_radians().cos().abs(), (tx as f64).to_radians().cos().abs()];
            for a in 0..2 {
                s[a] = (s[a].0 * f[a], p[a] + (s[a].1 - p[a]) * f[a]);
            }
        }
        s
    }

    /// The motion of `name` through its turn `t`, from the sprite placed
    /// as `placed` (at the turn's start, `at_end` false, or its end).
    /// `old` and `now`: the picture as shown at the start and end, if it
    /// is (edge-on it isn't); its unsquashed place goes straight between.
    pub(super) fn turn_motion(&self, name: &str, t: &Turning, layer: u32, old: Option<&Shown>, now: Option<&Shown>, at_end: bool) -> Option<Op> {
        let at = |turn| self.squash(name, Some((&t.target, turn)));
        let raw = |s: &Shown, sq: Squash| {
            let c = centre(s);
            (sq[0].0 > 1e-3 && sq[1].0 > 1e-3).then(|| [(c[0] - sq[0].1) / sq[0].0, (c[1] - sq[1].1) / sq[1].0])
        };
        let (s0, s1) = (at(t.from), at(t.to));
        let r0 = old.and_then(|s| raw(s, s0));
        let r1 = now.and_then(|s| raw(s, s1));
        let (r0, r1) = (r0.or(r1)?, r1.or(r0)?);
        let (placed, sp) = if at_end { (now?, s1) } else { (old?, s0) };
        let cp = centre(placed);
        let span = (t.to.0 - t.from.0).abs().max((t.to.1 - t.from.1).abs());
        // ours: a key every 10° (at most 63)
        let steps = (span / 10).clamp(4, 63) as u32;
        let keys = (0..=steps)
            .map(|i| {
                let u = f64::from(i) / f64::from(steps);
                let e = eased(t.ease, u);
                let turn = ((t.from.0 as f64 + (t.to.0 - t.from.0) as f64 * e).round() as i64, (t.from.1 as f64 + (t.to.1 - t.from.1) as f64 * e).round() as i64);
                let s = at(turn);
                let c = [0, 1].map(|a| s[a].0 * (r0[a] + (r1[a] - r0[a]) * u) + s[a].1);
                let zoom = [0, 1].map(|a| (s[a].0 / sp[a].0.max(1e-3) * 100.0).round().clamp(0.0, 10000.0) as u32);
                MotionKey { ms: (u * f64::from(t.ms)).round() as u32, x: (c[0] - cp[0]).round() as i32, y: (c[1] - cp[1]).round() as i32, zoom: zoom[0], angle: 0, opacity: 256, zoom_y: zoom[1] }
            })
            .collect();
        Some(Op::SpriteMotion2 { layer, keys, end: MotionEnd::Keep })
    }
}
