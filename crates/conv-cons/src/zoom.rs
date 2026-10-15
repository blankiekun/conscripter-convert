//! Camera zooms ([`Op::SpriteZoom`]): a sprite moved and zoomed about its
//! centre with CONscripter's `spzoom`, which keeps the sprite in its place
//! among the others (unlike `lsp2`).

use std::fmt::Write;

use crate::SPRITE_BASE;

const ZOOM: &str = "conv_zoom";

/// Writes a zoom of layer `layer`; returns whether it uses a subroutine:
/// [`routine`], or `eased` (an eased one, [`crate::ease`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit(out: &mut String, layer: u32, x: i32, y: i32, from: u32, to: u32, ms: u32, in_transition: bool, eased: Option<&str>) -> bool {
    let slot = SPRITE_BASE + layer;
    if ms > 0 && !in_transition {
        let routine = eased.unwrap_or(ZOOM);
        writeln!(out, "mov %1,{slot}:mov %2,{x}:mov %3,{y}:mov %4,{ms}:mov %8,{from}:mov %0,{to}:gosub *{routine}").unwrap();
        return true;
    }
    writeln!(out, "getspsize {slot},%1,%2:amsp {slot},{x}-%1/2,{y}-%2/2:spzoom {slot},{to}").unwrap();
    if !in_transition {
        writeln!(out, "print 1").unwrap();
    }
    false
}

/// Subroutine moving sprite %1's centre to (%2, %3) and zooming it from %8
/// to %0 percent over %4 ms, drawn about every 10 ms and timed by the
/// timer, like the move's. Uses the scratch variables %0-%8.
pub(crate) fn routine() -> String {
    format!(
        "*{ZOOM}
getspsize %1,%5,%6:div %5,2:div %6,2:sub %2,%5:sub %3,%6
getsppos %1,%5,%6
resettimer
*{ZOOM}_loop
gettimer %7
if %7 >= %4 amsp %1,%2,%3:spzoom %1,%0:print 1:return
amsp %1,%5+(%2-%5)*%7/%4,%6+(%3-%6)*%7/%4
spzoom %1,%8+(%0-%8)*%7/%4
print 1
wait 10
goto *{ZOOM}_loop
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zooms_at_once_or_over_time() {
        let mut out = String::new();
        assert!(!emit(&mut out, 5, 480, 272, 100, 200, 0, false, None));
        assert_eq!(out, "getspsize 67,%1,%2:amsp 67,480-%1/2,272-%2/2:spzoom 67,200\nprint 1\n");
        let mut out = String::new();
        assert!(!emit(&mut out, 5, 480, 272, 100, 200, 300, true, None));
        assert!(!out.contains("print"));
        let mut out = String::new();
        assert!(emit(&mut out, 5, 400, 300, 200, 100, 300, false, None));
        assert_eq!(out, "mov %1,67:mov %2,400:mov %3,300:mov %4,300:mov %8,200:mov %0,100:gosub *conv_zoom\n");
        assert!(routine().contains("spzoom %1,%8+(%0-%8)*%7/%4"));
    }
}
