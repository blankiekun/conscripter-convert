//! `DrawTransition` stopped partway: the opacity it leaves on a picture
//! for each grey of its mask.

/// Muramasa.exe's table from a mask's grey (its green, `0044ff90`) to the
/// picture's opacity (0-255), left by `DrawTransition(…, ms, from, to,
/// softness, …)` once done; `None` when the transition ends shown
/// (`to` × 0.255 = 255: the table goes, `00457c58`). The picture is drawn
/// times it, the mask stretched over the screen (`00418320`).
///
/// src: the start `FUN_0044fe00` (levels `from`, `to` × 0.255, a band step
/// 255 × 255 ÷ softness ÷ level, `time` × 60 ÷ 1000 frames, at least 1) and
/// each frame `FUN_00457bf0` (opaque up to the level, a ramp down past it,
/// the entries after the ramp cleared up to the first clear one).
// shortcut: frames before the last take a straight curve, not the tempo's;
// they only decide which leftovers of the start's ramp the clearing reaches.
pub(super) fn table(from: i64, to: i64, softness: i64, ms: i64) -> Option<[u8; 256]> {
    let (a, b) = ((from as f64 * 0.255) as i64, (to as f64 * 0.255) as i64);
    if b == 255 {
        return None;
    }
    let mut t = [0u8; 256];
    if b == 0 {
        return Some(t);
    }
    let per = 255.0 / softness as f64;
    for (i, e) in t.iter_mut().enumerate() {
        let i = i as i64;
        *e = if a < b { if i < a { 255 } else { 0 } } else if b < a && a < i { 0 } else { 255 };
    }
    let ramp = |t: &mut [u8; 256], mut i: i64, step: f64| {
        let mut v = 255.0;
        loop {
            if i > 255 {
                break;
            }
            t[i as usize] = v as u8;
            i += 1;
            v -= step;
            if v < 0.0 {
                break;
            }
        }
        i
    };
    if a != 0 {
        ramp(&mut t, a, per * 255.0 / a as f64);
    }
    let frames = ((ms as f64 * 60.0 / 1000.0) as i64).max(1);
    let dist = (a - b).abs();
    for k in 1..=frames {
        let d = dist * k / frames;
        let mut i = a;
        let p = if a < b {
            while i < d + a + 1 && i <= 255 {
                t[i as usize] = 255;
                i += 1;
            }
            d + a + 1
        } else {
            while i > a - d - 1 {
                if i < 0 {
                    break;
                }
                t[i as usize] = 0;
                i -= 1;
            }
            if a - d - 1 == 0 { 1 } else { a - d - 1 }
        };
        if i < 0 {
            continue;
        }
        let mut i = ramp(&mut t, i, 255.0 / p as f64 * per);
        while i <= 255 && t[i as usize] != 0 {
            t[i as usize] = 0;
            i += 1;
        }
    }
    Some(t)
}

/// The English port's table for a `DrawTransition` ending at `to`
/// with `softness` (Mware's `psUpdateTransitionRule`, decoded from its
/// DXBC: grey u ÷ 255 at sat(k(now − u) + u·now), k = 2000 ÷ softness, now
/// `to` ÷ 1000; softness 0 `psUpdateTransitionRule_Rate0`: u ≤ now);
/// `None` when it ends shown everywhere.
pub(super) fn diesel_table(to: i64, softness: i64) -> Option<[u8; 256]> {
    let now = to as f64 / 1000.0;
    let mut t = [0u8; 256];
    for (g, e) in t.iter_mut().enumerate() {
        let u = g as f64 / 255.0;
        let v = if softness > 0 { 2000.0 / softness as f64 * (now - u) + u * now } else { f64::from(u8::from(u <= now)) };
        *e = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    Some(t).filter(|t| t.iter().any(|&v| v < 255))
}

#[cfg(test)]
mod tests {
    use super::{diesel_table, table};

    #[test]
    fn the_ports_band() {
        // softness 500 (k 4) to 500: shown to u 1/3.5, gone from u 4/7
        let t = diesel_table(500, 500).unwrap();
        assert_eq!((t[72], t[80], t[100], t[146]), (255, 230, 160, 0));
        assert!(diesel_table(1000, 1000).is_none());
        assert_eq!(diesel_table(500, 0).unwrap()[128], 0);
    }

    #[test]
    fn levels() {
        assert_eq!(table(0, 1000, 200, 500), None);
        assert_eq!(table(1000, 0, 200, 500), Some([0; 256]));
        // ma03_024's 0 → 200 (level 51), softness 200: opaque to 52, then
        // 255 × 255 ÷ 200 ÷ 52 ≈ 6.25 less each grey.
        let t = table(0, 200, 200, 500).unwrap();
        assert!(t[..=52].iter().all(|&v| v == 255));
        assert_eq!(&t[53..56], &[248, 242, 236]);
        assert_eq!(t[92], 4);
        assert!(t[93..].iter().all(|&v| v == 0));
        // Falling 250 → 150 (63 → 38): opaque below 37, the ramp from it.
        let t = table(250, 150, 200, 500).unwrap();
        assert!(t[..=37].iter().all(|&v| v == 255));
        assert!(t[38] < 255 && t[38] > 240);
        assert!(t[70..].iter().all(|&v| v == 0));
    }
}
