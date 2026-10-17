//! `Shake` of a picture as `spmotion` keys: the exe (`shake_frame`) moves
//! the picture alone by ±(x, y), flipping the sign each step, the size
//! going from (x0, y0) to (x1, y1) over the time.

/// How often a shaken picture turns.
// guess: the exe (FUN_004578e0) flips the sign once the time since the last
// flip passes the shake's ms ÷ DAT_005dcaf0 (a run-time frame value, not read)
const STEP_MS: u32 = 100;
/// `spmotion` takes 64 keys (engine `COMMANDS.md`; it was 32 before engine #44).
const MAX_KEYS: u32 = super::loops::MAX_KEYS as u32;

/// A part of a shake: ms, size (x, y) from, size to.
type Part = (u32, (i32, i32), (i32, i32));

/// `spmotion` keys (ms, x, y) for shake `parts` (ms, size from, size to;
/// 0 px a wait): to and fro every step (longer when the keys would run
/// out), back at 0 at the end.
pub(super) fn shake_keys(parts: &[Part]) -> Vec<(u32, i32, i32)> {
    shake_keys_at(parts, STEP_MS, None)
}

/// [`shake_keys`] turning every `step` ms at least, the size going from
/// one to the other along `ease` ([`super::turn::eased`]).
pub(super) fn shake_keys_at(parts: &[Part], step: u32, ease: Option<(u8, u32)>) -> Vec<(u32, i32, i32)> {
    let still = |p: &Part| p.1 == (0, 0) && p.2 == (0, 0);
    let waits = parts.iter().filter(|p| still(p)).count() as u32;
    let shaking: u32 = parts.iter().filter(|p| !still(p)).map(|p| p.0).sum();
    let step = step.max(1).max(shaking.div_ceil(MAX_KEYS - 2 - waits).div_ceil(10) * 10);
    let (mut keys, mut t, mut sign) = (vec![(0, 0, 0)], 0, 1);
    for p @ &(ms, from, to) in parts {
        if still(p) {
            t += ms;
            keys.push((t, 0, 0));
            continue;
        }
        let n = (ms / step).max(1);
        for i in 1..=n {
            let f = super::turn::eased(ease, f64::from(i) / f64::from(n));
            let part = |d: i32| if ease.is_some() { (f64::from(d) * f).round() as i32 } else { d * i as i32 / n as i32 };
            let at = |a: i32, b: i32| sign * (a + part(b - a));
            keys.push((t + ms * i / n, at(from.0, to.0), at(from.1, to.1)));
            sign = -sign;
        }
        t += ms;
    }
    if keys.last().is_some_and(|k| (k.1, k.2) != (0, 0)) {
        keys.push((t + step, 0, 0));
    }
    keys.dedup_by(|b, a| (a.1, a.2, b.1, b.2) == (0, 0, 0, 0) && b.0 == a.0);
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shake_goes_to_and_fro_in_at_most_32_keys() {
        // ShakeVLoop01 (dja016): shakes, a 700 ms wait, shakes, 1500 ms.
        let v = |ms, a, b| (ms, (0, a), (0, b));
        let parts = [v(1000, 3, 1), v(500, 2, 0), v(700, 0, 0), v(1000, 3, 1), v(500, 2, 0), v(1500, 0, 0)];
        let keys = shake_keys(&parts);
        assert!(keys.len() <= MAX_KEYS as usize, "{keys:?}");
        assert_eq!(keys[0], (0, 0, 0));
        assert_eq!(keys.last().unwrap().0, 5200);
        assert!(keys.iter().any(|k| k.2 > 0) && keys.iter().any(|k| k.2 < 0));
        assert!(keys.windows(2).all(|w| w[0].0 < w[1].0), "{keys:?}");
        // ShakeVLoop: 3 s of shaking, every 100 ms.
        let keys = shake_keys(&[v(1000, 3, 1), v(500, 2, 0), v(1000, 3, 1), v(500, 2, 0)]);
        assert_eq!(keys[1], (100, 0, 3));
        assert!(keys.len() <= MAX_KEYS as usize, "{keys:?}");
        // Both ways, 10 px dying out over 1 s: back at the place.
        let keys = shake_keys(&[(1000, (10, 4), (0, 0))]);
        assert_eq!((keys[1], keys[2]), ((100, 9, 4), (200, -8, -4)));
        assert_eq!(keys.last().unwrap(), &(1000, 0, 0));
        // 続・殺戮のジャンゴ: n 200 turns every 83 ms, Dxl1 dying out fast.
        let keys = shake_keys_at(&[(500, (0, 10), (0, 0))], 83, Some((2, 2)));
        assert_eq!((keys[1], keys[2]), ((83, 0, 7), (166, 0, -4)));
    }
}
