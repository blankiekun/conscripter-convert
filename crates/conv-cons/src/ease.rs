//! Eased moves and zooms ([`Op::MoveEase`]): the move and zoom loops
//! with their progress (%7, per mille) eased, one subroutine per curve a
//! game uses. Integer arithmetic only: the exponent is a whole number.

use std::fmt::Write;

/// The subroutine moving (`zoom` false) or zooming along curve `ease`
/// with `exponent` (see [`Op::MoveEase`](conv_core::ir::Op::MoveEase)).
pub(crate) fn label(zoom: bool, ease: u8, exponent: u32) -> String {
    format!("conv_{}_e{ease}_{exponent}", if zoom { "zoom" } else { "move" })
}

/// Lines turning %7 (0-1000, the share of the time gone) into the eased
/// share. `mov` reads all of %7 before it writes it.
fn progress(ease: u8, exponent: u32) -> String {
    let pow = |base: &str| {
        let mut e = base.to_string();
        for _ in 1..exponent.max(1) {
            write!(e, "*{base}/1000").unwrap();
        }
        e
    };
    match ease {
        1 => format!("mov %7,{}\n", pow("%7")),
        2 => format!("mov %7,1000-{}\n", pow("(1000-%7)")),
        // The second half first: its result (500 or more) skips the other.
        3 => format!("if %7 >= 500 mov %7,1000-{}/2\nif %7 < 500 mov %7,{}/2\n", pow("(2000-2*%7)"), pow("(2*%7)")),
        _ => String::new(),
    }
}

/// The subroutine: as `conv_move` (sprite %1's centre to (%2, %3) over %4
/// ms) or `conv_zoom` (and from %8 to %0 percent), eased. Uses the
/// scratch variables %0-%8.
pub(crate) fn routine(zoom: bool, ease: u8, exponent: u32) -> String {
    let name = label(zoom, ease, exponent);
    let (end, step) = if zoom { (":spzoom %1,%0", "\nspzoom %1,%8+(%0-%8)*%7/1000") } else { ("", "") };
    let mut out = String::new();
    writeln!(out, "*{name}").unwrap();
    out.push_str("getspsize %1,%5,%6:div %5,2:div %6,2:sub %2,%5:sub %3,%6\ngetsppos %1,%5,%6\nresettimer\n");
    writeln!(out, "*{name}_loop").unwrap();
    writeln!(out, "gettimer %7").unwrap();
    writeln!(out, "if %7 >= %4 amsp %1,%2,%3{end}:print 1:return").unwrap();
    writeln!(out, "mov %7,%7*1000/%4").unwrap();
    out.push_str(&progress(ease, exponent));
    writeln!(out, "amsp %1,%5+(%2-%5)*%7/1000,%6+(%3-%6)*%7/1000{step}").unwrap();
    writeln!(out, "print 1\nwait 10\ngoto *{name}_loop").unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the progress lines on t (per mille), as NScripter would.
    fn eased(ease: u8, exponent: u32, t: i64) -> i64 {
        let n = exponent.max(1);
        let pow = |b: i64| (1..n).fold(b, |e, _| e * b / 1000);
        match ease {
            1 => pow(t),
            2 => 1000 - pow(1000 - t),
            3 if t >= 500 => 1000 - pow(2000 - 2 * t) / 2,
            3 => pow(2 * t) / 2,
            _ => t,
        }
    }

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
                assert!((eased(ease, exponent, t) as f64 / 1000.0 - want).abs() < 0.005, "{ease} {exponent} {t}");
            }
        }
    }

    #[test]
    fn writes_the_same_expressions() {
        assert_eq!(progress(1, 3), "mov %7,%7*%7/1000*%7/1000\n");
        assert_eq!(progress(2, 2), "mov %7,1000-(1000-%7)*(1000-%7)/1000\n");
        assert!(progress(3, 2).starts_with("if %7 >= 500 mov %7,1000-(2000-2*%7)*(2000-2*%7)/1000/2\n"));
        let r = routine(true, 3, 2);
        assert!(r.starts_with("*conv_zoom_e3_2\n"));
        assert!(r.contains("spzoom %1,%8+(%0-%8)*%7/1000"));
    }
}
