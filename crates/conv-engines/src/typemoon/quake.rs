//! The layer quakes (`_SQK2`, `_SHK2`, `_SSW2`; `layer_quake_start`
//! 0x163520, updated by `FUN_0013c970` at 60 fps) as `spshake` offset
//! lists ([`ScreenEffect::ShakeList`]): the exe's update worked out frame
//! by frame here. Its offsets are in 1/3 script px (`FUN_00134440` draws
//! `off/3`).

use std::f64::consts::TAU;

use conv_core::ir::ScreenEffect;

/// What a layer quake does, as its command's arguments set it (frames are
/// the exe's, ms × 3/50).
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Kind {
    /// `_SQK2` (kind 1): every `hold` + 1 frames a new x, y = amp × env ×
    /// uniform(-1, 1); env rises over `fade_in` frames, else falls
    /// straight to 0 over a finite quake.
    Quake { x: i32, y: i32, hold: u32, fade_in: u32 },
    /// `_SHK2` (kind 2): x, y = amp × sin(2π t / `period`) × env; `decay`:
    /// it falls straight to 0 over a finite one; `flicker` n > 1: amp / n
    /// on 1 frame in 5 (rand() % 5 == 0), else 0.
    Sway { x: i32, y: i32, period: u32, decay: bool, fade_in: u32, flicker: i32 },
    /// `_SSW2` (kind 3): each axis on its own, each turn a new amp in
    /// [amp.0, amp.1] and turn length in [period.0, period.1] frames,
    /// sign × amp × gain × sin(2π counter / length), the sign picked once.
    Drift { amp: (i32, i32), period: (u32, u32), gain: i32 },
}

/// A quake: its kind, its length in frames (`None`: for ever) and its
/// delay in frames.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Quake {
    pub kind: Kind,
    pub frames: Option<u32>,
    pub delay: u32,
}

/// The exe's frames for `ms` (`ms * 3 / 50`, 0x163520).
pub(super) fn frames(ms: i32) -> u32 {
    (ms.max(0) * 3 / 50) as u32
}

/// A quake's length: `ms` < 0 runs for ever (0xffff), and so does one of
/// 0 frames (ms 0-16: the update ends it only when its length is 1 or
/// more, 0x13c970).
pub(super) fn length(ms: i32) -> Option<u32> {
    Some(frames(ms)).filter(|&f| ms >= 0 && f > 0)
}

// ours: the exe's RNG (the object at *0xbe0190+0x50, libc rand()) can't be replayed: a seeded xorshift.
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng((seed.wrapping_mul(0x9E37_79B9) ^ 0x2545_F491) | 1) // ours: any odd seed
    }

    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// In [0, 1).
    fn unit(&mut self) -> f64 {
        f64::from(self.next()) / 4_294_967_296.0
    }
}

// ours: CONscripter refuses a quoted string of 4096 chars or more (ScriptHandler.cpp, STRING_BUFFER_LENGTH).
const MAX_LIST: usize = 4000;
// ours: a for-ever quake loops this many frames of its offsets (10 s) at most.
const LOOP_FRAMES: u32 = 600;

/// The `spshake` list for `quake`, `seed` its random numbers; `None` when
/// it never moves a pixel (amp 0 or below 3).
pub(super) fn effect(quake: &Quake, seed: u32) -> Option<ScreenEffect> {
    // A for-ever quake loops: kind 1 and 3 over (at most) 10 s, kind 2 one turn.
    let base = match quake.kind {
        Kind::Quake { hold, .. } => hold + 1,
        _ => 1,
    };
    if quake.frames.is_some_and(|f| f < base) {
        return None; // src: exe 0x13c970 ends it (t > length) before its first value
    }
    let mut total = match (quake.frames, &quake.kind) {
        (Some(f), _) => f,
        // ours: flicker loops 2 s of whole turns, so its spikes don't repeat each turn
        (None, Kind::Sway { period, flicker, .. }) if *flicker > 1 => period * LOOP_FRAMES.div_ceil(5 * period),
        (None, Kind::Sway { period, .. }) => *period,
        (None, _) => (LOOP_FRAMES / base).max(1) * base,
    };
    loop {
        let offsets = offsets(quake, total, seed);
        if offsets.iter().all(|&o| o == (0, 0)) {
            return None;
        }
        // ours: 4096-char string limit: a coarser step (a multiple of the exe's), or a shorter loop
        for k in 1..=total / base {
            let step = (base * k) as usize;
            let list: Vec<(i32, i32)> = offsets.iter().skip(step - 1).step_by(step).copied().collect();
            let chars: usize = list.iter().map(|(x, y)| format!("{x},{y} ").len()).sum();
            if chars < MAX_LIST {
                // Each entry from the frame its value is worked out: the first after step - 1 frames.
                let ms = |f: u32| (f * 50 + 1) / 3; // src: exe 0x163520 (ms * 3/50 frames)
                return Some(ScreenEffect::ShakeList {
                    interval_ms: ms(step as u32),
                    offsets: list,
                    looped: quake.frames.is_none(),
                    delay_ms: ms(quake.delay + step as u32 - 1),
                });
            }
            if quake.frames.is_none() && !matches!(quake.kind, Kind::Sway { .. }) && total > 2 * base {
                break;
            }
        }
        total = (total / 2 / base).max(1) * base;
    }
}

/// The drawn offset (script px) of each of the `total` frames from the
/// start (frame 1 first), as `FUN_0013c970` works them out.
fn offsets(quake: &Quake, total: u32, seed: u32) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    let dur = quake.frames.unwrap_or(0);
    let px = |v: f64| (v / 3.0).round() as i32; // src: exe FUN_00134440 (off/3)
    match quake.kind {
        Kind::Quake { x, y, hold, fade_in } => {
            // src: exe 0x191540: the fall lasts the whole quake when it is finite with no fade-in
            let fall = if fade_in == 0 { dur } else { 0 };
            let (mut counter, mut now) = (0, (0, 0));
            (1..=total)
                .map(|t| {
                    counter += 1;
                    if counter > hold {
                        counter = 0;
                        let env = if t < fade_in {
                            f64::from(t) / f64::from(fade_in)
                        } else if fall > 0 && t > dur - fall {
                            1.0 - f64::from(t - (dur - fall)) / f64::from(fall)
                        } else {
                            1.0
                        };
                        let mut u = || rng.unit() * 2.0 - 1.0;
                        now = (px(f64::from(x) * env * u()), px(f64::from(y) * env * u()));
                    }
                    now
                })
                .collect()
        }
        Kind::Sway { x, y, period, decay, fade_in, flicker } => (1..=total)
            .map(|t| {
                let mut v = (TAU * f64::from(t % period) / f64::from(period)).sin();
                if t < fade_in {
                    v *= f64::from(t) / f64::from(fade_in);
                }
                if decay && dur > 0 {
                    v *= f64::from(dur - t.min(dur)) / f64::from(dur);
                }
                if flicker > 1 {
                    v /= f64::from(flicker);
                    // src: exe FUN_001b5290 (rand() % 5)
                    if rng.next() % 5 != 0 {
                        v = 0.0;
                    }
                }
                (px(v * f64::from(x)), px(v * f64::from(y)))
            })
            .collect(),
        Kind::Drift { amp, period, gain } => {
            let mut axis = || drift(&mut rng, amp, period, gain, total, quake.frames.is_none());
            let (xs, ys) = (axis(), axis());
            xs.into_iter().zip(ys).map(|(x, y)| (px(x), px(y))).collect()
        }
    }
}

/// One axis of `_SSW2` over `total` frames: each turn (amp, length)
/// draws counter 0..=length; `looped`: the turns' lengths scaled to end
/// on the last frame, so the loop starts over at 0.
fn drift(rng: &mut Rng, amp: (i32, i32), period: (u32, u32), gain: i32, total: u32, looped: bool) -> Vec<f64> {
    let sign = if rng.unit() < 0.5 { 1.0 } else { -1.0 };
    let mut turns = Vec::new();
    let mut sum = 0;
    while sum < total {
        let a = f64::from(amp.0) + rng.unit() * f64::from(amp.1 - amp.0);
        let p = (f64::from(period.0) + rng.unit() * (f64::from(period.1) - f64::from(period.0))) as u32;
        turns.push((a, p.max(1)));
        sum += p.max(1) + 1;
    }
    if looped {
        // ours: the last turns stretched or squeezed so the loop ends on 0
        let (mut left, n) = (total, turns.len());
        for (i, (_, p)) in turns.iter_mut().enumerate() {
            let frames = if i + 1 == n { left } else { (*p + 1) * total / sum };
            *p = frames.max(2) - 1;
            left = left.saturating_sub(frames);
        }
    }
    turns
        .into_iter()
        .flat_map(|(a, p)| (0..=p).map(move |c| sign * a * f64::from(gain) * (TAU * f64::from(c) / f64::from(p)).sin()))
        .take(total as usize)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(e: Option<ScreenEffect>) -> (u32, Vec<(i32, i32)>, bool, u32) {
        match e {
            Some(ScreenEffect::ShakeList { interval_ms, offsets, looped, delay_ms }) => (interval_ms, offsets, looped, delay_ms),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn quake_falls_over_its_length() {
        // _SQK2(-1,15,30,0,600,0,0,0,0): hold 2 frames, 36 frames, the fall over all of them
        let q = Quake { kind: Kind::Quake { x: 15, y: 30, hold: 2, fade_in: 0 }, frames: length(600), delay: 0 };
        let (interval, offsets, looped, delay) = list(effect(&q, 7));
        assert_eq!((interval, offsets.len(), looped, delay), (50, 12, false, 33));
        assert!(offsets.iter().all(|&(x, y)| x.abs() <= 5 && y.abs() <= 10));
        assert!(offsets.iter().any(|&(_, y)| y.abs() >= 5));
        assert_eq!(*offsets.last().unwrap(), (0, 0));
        assert_eq!(list(effect(&q, 7)).1, offsets, "the same seed, the same list");
    }

    #[test]
    fn forever_lists_loop_and_fit() {
        let q = Quake { kind: Kind::Quake { x: 300, y: 300, hold: 0, fade_in: 0 }, frames: length(-1), delay: 0 };
        let (interval, offsets, looped, _) = list(effect(&q, 1));
        assert!(looped && interval == 17);
        assert!(offsets.iter().map(|(x, y)| format!("{x},{y} ").len()).sum::<usize>() < MAX_LIST);
        // a sway loops one turn: 300 ms = 18 frames
        let s = Quake { kind: Kind::Sway { x: 0, y: 15, period: 18, decay: false, fade_in: 0, flicker: 1 }, frames: None, delay: 3 };
        let (interval, offsets, looped, delay) = list(effect(&s, 1));
        assert_eq!((interval, offsets.len(), looped, delay), (17, 18, true, 50));
        assert_eq!((offsets[3], offsets[17]), ((0, 5), (0, 0)));
        // a drift loops 600 frames, its axes ending on 0
        let d = Quake { kind: Kind::Drift { amp: (7, 15), period: (60, 120), gain: 1 }, frames: None, delay: 0 };
        let (_, offsets, looped, _) = list(effect(&d, 3));
        assert!(looped && offsets.len() == 600 && offsets[599] == (0, 0) && offsets.iter().all(|&(x, y)| x.abs() <= 5 && y.abs() <= 5));
        // amp 1: never a pixel
        assert_eq!(effect(&Quake { kind: Kind::Quake { x: 1, y: 1, hold: 2, fade_in: 0 }, frames: None, delay: 0 }, 1), None);
    }
}
