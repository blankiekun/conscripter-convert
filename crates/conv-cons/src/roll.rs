//! A picture rolled over the rest of the music ([`Op::MusicRoll`]): how
//! far the music is through (`getbgmpos`, CONscripter after 0.26.0) gives
//! the time left, the next music starts then (`bgmlater`), and a loop
//! moves the picture (`amsp`) by the timer until that time and `after_ms`
//! more have gone, then a click; at `gamespeed 100`, as the timer counts
//! at the gamespeed. Uses the scratch variables %1-%7 and %9.

use std::fmt::Write;

use conv_core::ir::{Op, Script};

use crate::{path, BUTTON_VAR, SPRITE_BASE};

pub(crate) fn emit(out: &mut String, i: usize, op: &Op, script: &Script) {
    let Op::MusicRoll { layer, dy, song, song_ms, next, after_ms, skippable } = op else { return };
    let (slot, name) = (SPRITE_BASE + layer, format!("conv_roll{i}"));
    // (gettimer counts at the gamespeed.)
    writeln!(out, "gamespeed 100").unwrap();
    // Not started yet (a `bgmlater` still to come, which the next one
    // replaces): from its start.
    writeln!(out, "getbgmpos %1:if %1 == 0 bgmonce \"{}\"", path(song)).unwrap();
    writeln!(out, "mov %1,{song_ms}-%1:if %1 < 0 mov %1,0").unwrap();
    if let Some(next) = next {
        writeln!(out, "bgmlater %1,\"{}\",0", path(next)).unwrap();
    }
    // (In tenths below: dy × ms passes 2^31 in a few minutes.)
    writeln!(out, "mov %4,(%1+{after_ms})/10:if %4 < 1 mov %4,1").unwrap();
    writeln!(out, "getsppos {slot},%5,%6:resettimer").unwrap();
    if *skippable {
        writeln!(out, "btndef clear:btn 1,0,0,{},{},0,0", script.width, script.height).unwrap();
    }
    writeln!(out, "*{name}").unwrap();
    writeln!(out, "gettimer %7:mov %7,%7/10:if %7 > %4 mov %7,%4").unwrap();
    let sign = if *dy < 0 { '-' } else { '+' };
    writeln!(out, "amsp {slot},%5,%6{sign}{}*%7/%4:print 1", dy.unsigned_abs()).unwrap();
    writeln!(out, "if %7 >= %4 goto *{name}_end").unwrap();
    if *skippable {
        // The time ran out (-2, or -5 with `usewheel`) or the wheel: on.
        writeln!(out, "btntime 16:btnwait {BUTTON_VAR}:btntime -1").unwrap();
        writeln!(out, "if {BUTTON_VAR} != -1 && {BUTTON_VAR} < 1 goto *{name}").unwrap();
        writeln!(out, "btndef clear:goto *{name}_end2").unwrap();
    } else {
        writeln!(out, "wait 16:goto *{name}").unwrap();
    }
    writeln!(out, "*{name}_end").unwrap();
    if *skippable {
        writeln!(out, "btndef clear").unwrap();
    }
    writeln!(out, "click").unwrap();
    if *skippable {
        writeln!(out, "*{name}_end2").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolls_over_the_time_left() {
        let op = Op::MusicRoll { layer: 3, dy: -8765, song: "bgm/a.ogg".into(), song_ms: 162813, next: Some("bgm/b.ogg".into()), after_ms: 6000, skippable: true };
        let script = Script { width: 800, height: 600, ..Default::default() };
        let mut out = String::new();
        emit(&mut out, 7, &op, &script);
        assert!(out.contains("getbgmpos %1:if %1 == 0 bgmonce \"bgm\\a.ogg\"\nmov %1,162813-%1"));
        assert!(out.contains("bgmlater %1,\"bgm\\b.ogg\",0\nmov %4,(%1+6000)/10"));
        assert!(out.contains(&format!("amsp {},%5,%6-8765*%7/%4", SPRITE_BASE + 3)));
        // A click during the roll goes past the click at its end.
        assert!(out.contains("goto *conv_roll7_end2\n*conv_roll7_end\nbtndef clear\nclick\n*conv_roll7_end2\n"));
    }
}
