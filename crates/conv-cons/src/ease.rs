//! Eased moves and zooms ([`Op::MoveEase`]): the engine's `spmotion`
//! reaches the key along an `spcurve` table sampled from the curve, one
//! table per curve a game uses, defined at `*start`.

use std::fmt::Write;

/// The `spcurve` id of curve `ease` (see
/// [`Op::MoveEase`](conv_core::ir::Op::MoveEase)): 10-69 for 1-3 with
/// `exponent` (past 20 drawn as 20), 70-73 for 4-7.
pub(crate) fn curve_id(ease: u8, exponent: u32) -> u32 {
    match ease {
        4..=7 => 66 + u32::from(ease),
        _ => 10 + u32::from(ease.clamp(1, 3) - 1) * 20 + exponent.clamp(1, 20) - 1,
    }
}

/// The eased share (0-1) of the time gone `t` (0-1).
pub(crate) fn eased(ease: u8, exponent: u32, t: f64) -> f64 {
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

/// The `spcurve` line defining the curve: its share (0-1024) at 33 even
/// steps of the time.
pub(crate) fn curve_def(ease: u8, exponent: u32) -> String {
    let mut out = format!("spcurve {}", curve_id(ease, exponent));
    for i in 0..=32 {
        write!(out, ",{}", (eased(ease, exponent, f64::from(i) / 32.0) * 1024.0).round() as i64).unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_as_the_mages_engine() {
        // ease_value (0x8103cc18) with power p is these with exponent 2p.
        for (ease, exponent) in [(1, 2), (2, 2), (3, 2), (3, 3)] {
            let p = exponent as f64 / 2.0;
            for t in [0, 100, 250, 499, 500, 750, 1000] {
                let f = t as f64 / 1000.0;
                let want = match ease {
                    1 => (f * f).powf(p),
                    2 => 1.0 - ((1.0 - f) * (1.0 - f)).powf(p),
                    _ if f < 0.5 => (4.0 * f * f).powf(p) / 2.0,
                    _ => 1.0 - (4.0 * (1.0 - f) * (1.0 - f)).powf(p) / 2.0,
                };
                assert!((eased(ease, exponent, f) - want).abs() < 1e-9, "{ease} {exponent} {t}");
            }
        }
    }

    #[test]
    fn writes_tables() {
        let d = curve_def(1, 2);
        assert!(d.starts_with("spcurve 11,0,1,4,9,16,"), "{d}");
        assert!(d.ends_with(",1024"));
        assert_eq!(d.split(',').count(), 34);
        assert_eq!((curve_id(1, 1), curve_id(3, 2), curve_id(3, 99), curve_id(4, 1), curve_id(7, 1)), (10, 51, 69, 70, 73));
        // the sine ones (NSS's AxlAuto, DxlAuto, AxlDxl, DxlAxl): half way
        let half = |e| curve_def(e, 1).split(',').nth(17).unwrap().parse::<i64>().unwrap();
        assert_eq!([4, 5, 6, 7].map(half), [300, 724, 512, 512]);
    }
}
