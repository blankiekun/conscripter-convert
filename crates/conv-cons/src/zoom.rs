//! Camera zooms ([`Op::SpriteZoom`]): a sprite moved and zoomed about its
//! centre with CONscripter's `spzoom` and `spmotion`, which keeps the sprite in its place
//! among the others (unlike `lsp2`).

use std::fmt::Write;

use crate::SPRITE_BASE;

/// Writes a zoom of layer `layer`, along `curve` over time (0 straight, or
/// an `spcurve` id, [`crate::ease`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit(out: &mut String, layer: u32, x: i32, y: i32, from: u32, to: u32, ms: u32, in_transition: bool, curve: u32) {
    let slot = SPRITE_BASE + layer;
    if ms > 0 && !in_transition {
        crate::motion::move_to(out, slot, x, y, ms, Some((from, to)), curve);
        return;
    }
    writeln!(out, "getspsize {slot},%1,%2:amsp {slot},{x}-%1/2,{y}-%2/2:spzoom {slot},{to}").unwrap();
    if !in_transition {
        writeln!(out, "print 1").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zooms_at_once_or_over_time() {
        let mut out = String::new();
        emit(&mut out, 5, 480, 272, 100, 200, 0, false, 0);
        assert_eq!(out, "getspsize 67,%1,%2:amsp 67,480-%1/2,272-%2/2:spzoom 67,200\nprint 1\n");
        let mut out = String::new();
        emit(&mut out, 5, 480, 272, 100, 200, 300, true, 0);
        assert!(!out.contains("print"));
        let mut out = String::new();
        emit(&mut out, 5, 400, 300, 200, 100, 300, false, 0);
        assert!(out.starts_with("spzoom 67,200:") && out.contains("spmotionwait 67"), "{out}");
    }
}
