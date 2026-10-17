//! [`PictureReplay`] as NScripter code, a mode of
//! [`picture_extras`](crate::picture_extras) entered at [`REPLAY`].
//!
//! The list is drawn again after every click (`btnwait` drops its
//! buttons). A scene is a gosub into the story's code ([`REPLAYING`] is 1
//! meanwhile); afterwards everything is cleared, the text window too, and
//! the list comes back with the extras' music.

use conv_core::ir::PictureReplay;

use crate::picture_extras::pic;
use crate::picture_gallery::FADE;
use crate::{path, var_name, BUTTON_VAR, MAX_SPRITE, MENU_SPRITES};

pub(crate) const REPLAY: &str = "conv_pextras_recollection";
const MENU: &str = "conv_pextras_draw";
const DRAW: &str = "conv_prp_draw";
const WAIT: &str = "conv_prp_wait";
/// 1 while a scene plays from the replay (a local, cleared at the title).
pub(crate) const REPLAYING: &str = "%150";
/// Set by back to the title during a replay: after the `reset`, the
/// title goes on to [`END`] (a global, it outlasts the reset).
pub(crate) const BACK_TO_LIST: &str = "%2962";
/// After a scene: the fades, then the list.
const END: &str = "conv_prp_end";
// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const THUMB: u32 = 900; // + scene
const CURSOR: u32 = 860; // + scene
const MAX_SCENES: usize = 40;
const EXIT: u32 = MENU_SPRITES.start;
const BTN_EXIT: usize = 201;

/// The title's first line: back to the list after a `reset` from a replay.
pub(crate) fn after_reset() -> String {
    format!("if {BACK_TO_LIST} == 1 mov {BACK_TO_LIST},0:goto *{END}")
}

/// `music`: the lines that play the extras' music again (none without a
/// sound room).
pub(crate) fn emit(out: &mut String, r: &PictureReplay, border: u32, music: Option<&str>) {
    assert!(r.scenes.len() <= MAX_SCENES, "a replay with more scenes than its sprites");
    let place = |i: u32| place(r, i);
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    // The list: the engine's screen (closing with the scene, 0 to leave,
    // holding its frame), or a script loop.
    let screen = r.engine_screen;
    w(format!("*{REPLAY}"));
    if screen {
        w(format!("mov {FADE},{}", r.open_ms));
        w(format!("*{WAIT}"));
        w(format!("csp -1:screen \"replay\",{BUTTON_VAR}"));
        w(format!("if {BUTTON_VAR} == 0 goto *{MENU}"));
    } else {
        w(format!("gosub *{DRAW}:print 10,{}", r.open_ms));
        w(format!("*{WAIT}"));
        w(format!("btnwait {BUTTON_VAR}"));
        w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} goto *{MENU}"));
    }
    for (i, _) in r.scenes.iter().enumerate() {
        w(format!("if {BUTTON_VAR} == {} goto *conv_prp_s{i}", i + 1));
    }
    w(if screen { format!("goto *{WAIT}") } else { format!("gosub *{DRAW}:print 1:goto *{WAIT}") });
    for (i, s) in r.scenes.iter().enumerate() {
        w(format!("*conv_prp_s{i}"));
        if let Some((slot, (x, y))) = r.hold {
            // The screen's held frame into a sprite first: the freeze copies the sprites.
            if screen {
                w(format!("spgrab {BACKGROUND},-2"));
            }
            w(format!("screenzoom {slot},1,1,1000,{x},{y},0,0,0,0:print 1"));
        }
        w(format!("chvolfade -1,0,{}:wait {}", r.start_ms, r.start_ms));
        w(format!("csp -1:bgmstop:chvolfade -1,100:mov {REPLAYING},1"));
        w(format!("gosub *{}:goto *{END}", s.label));
    }
    w(format!("*{END}"));
    w(format!("mov {REPLAYING},0"));
    if let Some((slot, _)) = r.hold {
        w(format!("screenzoom {slot}"));
    }
    w(format!("chvolfade -1,0,{}:wait {}", r.end_ms, r.end_ms));
    w("csp -1:csp2 -1:bgmstop:stopallsound:chvolfade -1,100:textclear:textoff:bg black,1".into());
    if let Some(m) = music {
        w(m.to_string());
    }
    if screen {
        w(format!("mov {FADE},{}:goto *{WAIT}", r.open_ms));
        return;
    }
    w(format!("gosub *{DRAW}:print 10,{}", r.open_ms));
    w(format!("goto *{WAIT}"));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("lsp {BACKGROUND},\"{}\",0,0", pic(&r.background)));
    w(format!("lsp {EXIT},\":a/3,0,3;{}\",{},{}:spbtn {EXIT},{BTN_EXIT},2", path(&r.exit.image), r.exit.x, r.exit.y));
    for (i, s) in r.scenes.iter().enumerate() {
        let (x, y) = place(i as u32);
        let v = var_name(s.unlocked_by, border);
        w(format!("lsp {},\"{}\",{x},{y}", THUMB + i as u32, pic(&r.blank)));
        w(format!(
            "if {v} != 0 lsp {},\"{}\",{x},{y}:lsp {},\":a/2,0,3;{}\",{x},{y}:spbtn {},{}",
            THUMB + i as u32,
            pic(&s.thumbnail),
            CURSOR + i as u32,
            path(&r.cursor),
            CURSOR + i as u32,
            i + 1
        ));
    }
    w("return".into());
}

/// `screens/replay.lua`: the picture, each scene's thumbnail (`blank`
/// while locked) under its cursor button, the exit; a scene closes the
/// screen holding its frame with its number, the exit and a right-click
/// with 0.
pub(crate) fn lua_file(r: &PictureReplay, border: u32) -> String {
    use crate::screens::lua;
    use std::fmt::Write as _;
    let opaque = |p: &str| crate::sprite_mode(p) == 'c';
    let mut out = format!(
        "-- The scene replay's list (conscripter-convert, from PictureReplay).\nlocal widgets = {{{{\"image\", file={}, opaque={}, x=0, y=0}}}}\nlocal function add(w) widgets[#widgets + 1] = w end\n",
        lua(&r.background),
        opaque(&r.background)
    );
    for (i, s) in r.scenes.iter().enumerate() {
        let (x, y) = place(r, i as u32);
        let v = lua(&var_name(s.unlocked_by, border));
        writeln!(
            out,
            "if var({v}) ~= 0 then\n  add({{\"image\", file={}, opaque={}, x={x}, y={y}}})\n  add({{\"button\", file={}, cells=2, x={x}, y={y}, action={{\"close\", {}, hold=true}}}})\nelse\n  add({{\"image\", file={}, opaque={}, x={x}, y={y}}})\nend",
            lua(&s.thumbnail),
            opaque(&s.thumbnail),
            lua(&r.cursor),
            i + 1,
            lua(&r.blank),
            opaque(&r.blank)
        )
        .unwrap();
    }
    let e = &r.exit;
    writeln!(
        out,
        "add({{\"button\", file={}, cells=3, pressed=2, x={}, y={}, action={{\"close\", 0, hold=true}}}})\nreturn {{\n  kind = \"extras\",\n  fade = var({}),\n  cancel = {{\"close\", 0, hold=true}},\n  widgets = widgets,\n}}",
        lua(&e.image),
        e.x,
        e.y,
        lua(FADE)
    )
    .unwrap();
    out
}

/// Scene i's place.
fn place(r: &PictureReplay, i: u32) -> (i32, i32) {
    let (column, row) = match r.across {
        Some(n) => (i % n, i / n),
        None => (i / r.rows, i % r.rows),
    };
    (r.first.0 + r.column_step * column as i32, r.first.1 + r.row_step * row as i32)
}

#[cfg(test)]
mod screen_tests {
    use super::*;
    use conv_core::ir::{Placed, ReplayEntry, Var};

    #[test]
    fn replay_list_becomes_a_screen() {
        let r = PictureReplay {
            background: "sys/bg.png".into(),
            exit: Placed { image: "sys/exit.png".into(), x: 0, y: 438, w: 60, h: 30 },
            first: (672, 38),
            row_step: 100,
            column_step: -100,
            rows: 5,
            blank: "sys/blank.png".into(),
            cursor: "sys/cursor.png".into(),
            scenes: vec![ReplayEntry { thumbnail: "t/a.jpg".into(), unlocked_by: Var { global: true, index: 3 }, label: "reco_a".into() }],
            open_ms: 500,
            start_ms: 1000,
            end_ms: 1000,
            across: None,
            hold: Some((4, (512, 288))),
            engine_screen: true,
        };
        let lua = lua_file(&r, 4000);
        assert!(lua.contains("if var(\"%4003\") ~= 0 then\n  add({\"image\", file=\"t/a.jpg\", opaque=true, x=672, y=38})\n  add({\"button\", file=\"sys/cursor.png\", cells=2, x=672, y=38, action={\"close\", 1, hold=true}})\nelse"), "{lua}");
        let mut out = String::new();
        emit(&mut out, &r, 4000, None);
        for part in ["*conv_prp_wait\ncsp -1:screen \"replay\",%9\nif %9 == 0 goto *conv_pextras_draw\nif %9 == 1 goto *conv_prp_s0\n", "spgrab 999,-2\nscreenzoom 4,", "mov %132,500:goto *conv_prp_wait\n"] {
            assert!(out.contains(part), "missing {part:?} in\n{out}");
        }
        assert!(!out.contains("*conv_prp_draw"), "{out}");
    }
}
