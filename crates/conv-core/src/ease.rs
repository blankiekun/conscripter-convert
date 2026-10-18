//! Eased curves: the share of a move done at a share of its time, as the
//! source engines ease their moves (shared by the readers, which work out
//! values mid-move, and conv-cons, which writes them as `spcurve` tables).

/// The eased share (0-1) of the time gone `t` (0-1).
pub fn eased(ease: u8, exponent: u32, t: f64) -> f64 {
    let n = exponent.clamp(1, 20) as i32;
    let pi = std::f64::consts::PI;
    // 9-29 as in 月姫's exe (FUN_00130c80, Penner's set): each family's in
    // and out; in-out and out-in put them in either half.
    if ease >= 9 {
        let base = 10 + ease.saturating_sub(10) / 4 * 4;
        let (ein, eout) = (|u| eased(base, 0, u), |u| eased(base + 1, 0, u));
        return match ease {
            9 if t < 0.5 => (pi * t).sin() / 2.0,
            9 => 1.0 - ((2.0 * t - 1.0) * pi / 2.0).cos() / 2.0,
            _ if t <= 0.0 => 0.0,
            _ if t >= 1.0 => 1.0,
            10 => 1.0 - (1.0 - t * t).sqrt(),
            11 => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
            14 => 2f64.powf(10.0 * (t - 1.0)) - 0.001,
            15 => 1.001 * (1.0 - 2f64.powf(-10.0 * t)),
            16 if t < 0.5 => 2f64.powf(10.0 * (2.0 * t - 1.0)) / 2.0 - 0.0005,
            16 => 1.0005 * (2.0 - 2f64.powf(-10.0 * (2.0 * t - 1.0))) / 2.0,
            18 => -(2f64.powf(10.0 * (t - 1.0)) * ((t - 1.075) * 2.0 * pi / 0.3).sin()),
            19 => 2f64.powf(-10.0 * t) * ((t - 0.075) * 2.0 * pi / 0.3).sin() + 1.0,
            20 if t < 0.5 => -(2f64.powf(10.0 * (2.0 * t - 1.0)) * ((2.0 * t - 1.1125) * 2.0 * pi / 0.45).sin()) / 2.0,
            20 => 2f64.powf(-10.0 * (2.0 * t - 1.0)) * ((2.0 * t - 1.1125) * 2.0 * pi / 0.45).sin() / 2.0 + 1.0,
            22 => t * t * (2.70158 * t - 1.70158),
            23 => (t - 1.0) * (t - 1.0) * (2.70158 * (t - 1.0) + 1.70158) + 1.0,
            24 if t < 0.5 => 2.0 * t * t * (7.1898194 * t - 2.5949097),
            24 => ((2.0 * t - 2.0).powi(2) * (3.5949097 * (2.0 * t - 2.0) + 2.5949097) + 2.0) / 2.0,
            26 => 1.0 - bounce_out(1.0 - t),
            27 => bounce_out(t),
            _ if ease.is_multiple_of(4) && t < 0.5 => ein(2.0 * t) / 2.0,
            _ if ease.is_multiple_of(4) => 0.5 + eout(2.0 * t - 1.0) / 2.0,
            _ if t < 0.5 => eout(2.0 * t) / 2.0,
            _ => 0.5 + ein(2.0 * t - 1.0) / 2.0,
        };
    }
    match ease {
        4 => 1.0 - (pi * t / 2.0).cos(),
        5 => (pi * t / 2.0).sin(),
        6 => 0.5 - (pi * t).cos() / 2.0,
        7 => (1.0 - 2.0 * t).acos() / pi,
        8 if t < 0.5 => (1.0 - (1.0 - 2.0 * t).powi(n)) / 2.0,
        8 => (1.0 + (2.0 * t - 1.0).powi(n)) / 2.0,
        1 => t.powi(n),
        2 => 1.0 - (1.0 - t).powi(n),
        _ if t < 0.5 => (2.0 * t).powi(n) / 2.0,
        _ => 1.0 - (2.0 - 2.0 * t).powi(n) / 2.0,
    }
}

/// Penner's bounce out.
fn bounce_out(t: f64) -> f64 {
    let (k, d) = (7.5625, 2.75);
    match t {
        _ if t < 1.0 / d => k * t * t,
        _ if t < 2.0 / d => k * (t - 1.5 / d).powi(2) + 0.75,
        _ if t < 2.5 / d => k * (t - 2.25 / d).powi(2) + 0.9375,
        _ => k * (t - 2.625 / d).powi(2) + 0.984375,
    }
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
    fn penner_as_tsukihimes_exe() {
        // FUN_00130c80's cases (b = 0, c = d = 1), written from the decompile
        let pi = std::f64::consts::PI;
        let bounce = |t: f64| bounce_out(t);
        type Case = (u8, u32, fn(f64) -> f64);
        let cases: [Case; 12] = [
            (8, 2, |t| if t >= 0.5 { let p = 2.0 * t - 1.0; p * p * 0.5 + 0.5 } else { let p = 2.0 * t; -0.5 * p * (p - 2.0) }), // 5
            (9, 0, |t| if t >= 0.5 { 0.5 + 0.5 - ((2.0 * t - 1.0) * std::f64::consts::FRAC_PI_2).cos() * 0.5 } else { 0.5 * (2.0 * t * std::f64::consts::FRAC_PI_2).sin() }), // 0x15
            (12, 0, |t| { let p = 2.0 * t; if p >= 1.0 { 0.5 * ((1.0 - (p - 2.0) * (p - 2.0)).sqrt() + 1.0) } else { -0.5 * ((1.0 - p * p).sqrt() - 1.0) } }), // 0x18
            (13, 0, |t| if t >= 0.5 { 0.5 - ((1.0 - (2.0 * t - 1.0).powi(2)).sqrt() - 1.0) * 0.5 } else { 0.5 * (1.0 - (2.0 * t - 1.0).powi(2)).sqrt() }), // 0x19
            (14, 0, |t| 2f64.powf(10.0 * (t - 1.0)) - 0.001), // 0x1a
            (15, 0, |t| 1.001 * (1.0 - 2f64.powf(-10.0 * t))), // 0x1b
            (17, 0, |t| if t < 0.5 { 0.5 * 1.001 * (1.0 - 2f64.powf(-20.0 * t)) } else { 0.5 + 0.5 * 2f64.powf(10.0 * (2.0 * t - 2.0)) - 0.0005 }), // 0x1d
            (18, 0, |t| { let u = t - 1.0; -(2f64.powf(10.0 * u) * ((u - 0.075) * 2.0 * std::f64::consts::PI / 0.3).sin()) }), // 0x1e
            (21, 0, |t| if t >= 0.5 { let u = 2.0 * t - 2.0; 0.5 - 2f64.powf(10.0 * u) * 0.5 * ((u - 0.075) * 2.0 * std::f64::consts::PI / 0.3).sin() } else { let u = 2.0 * t; 2f64.powf(-10.0 * u) * 0.5 * ((u - 0.075) * 2.0 * std::f64::consts::PI / 0.3).sin() + 0.5 }), // 0x21
            (25, 0, |t| if t >= 0.5 { let p = 2.0 * t - 1.0; p * p * 0.5 * (p * 2.70158 - 1.70158) + 0.5 } else { let u = 2.0 * t - 1.0; 0.5 * (u * u * (u * 2.70158 + 1.70158) + 1.0) }), // 0x25
            (28, 0, |t| if t >= 0.5 { 0.5 + bounce_out(2.0 * t - 1.0) * 0.5 } else { (1.0 - bounce_out(1.0 - 2.0 * t)) * 0.5 }), // 0x28
            (29, 0, |t| if t >= 0.5 { (1.0 - bounce_out(1.0 - (2.0 * t - 1.0))) * 0.5 + 0.5 } else { bounce_out(2.0 * t) * 0.5 }), // 0x29
        ];
        for (e, n, f) in cases {
            for t in [0.05, 0.2, 0.35, 0.45, 0.55, 0.7, 0.85, 0.95] {
                assert!((eased(e, n, t) - f(t)).abs() < 1e-6, "{e} {t}: {} {}", eased(e, n, t), f(t));
            }
        }
        assert!((eased(19, 0, 0.5) - (2f64.powf(-5.0) * ((0.5 - 0.075) * 2.0 * pi / 0.3).sin() + 1.0)).abs() < 1e-9);
        assert!((eased(27, 0, 0.5) - bounce(0.5)).abs() < 1e-9 && (bounce(0.5) - (7.5625 * (0.5f64 - 1.5 / 2.75).powi(2) + 0.75)).abs() < 1e-9);
        for e in 9..=29 {
            assert!(eased(e, 0, 0.0).abs() < 1e-9 && (eased(e, 0, 1.0) - 1.0).abs() < 1e-9, "{e}");
        }
    }
}
