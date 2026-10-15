//! [`PictureReplay`] as NScripter code, a mode of
//! [`picture_extras`](crate::picture_extras) entered at [`REPLAY`].
//!
//! The list is drawn again after every click (`btnwait` drops its
//! buttons). A scene is a gosub into the story's code ([`REPLAYING`] is 1
//! meanwhile); afterwards everything is cleared, the text window too, and
//! the list comes back with the extras' music.

use conv_core::ir::PictureReplay;

use crate::picture_extras::pic;
use crate::{path, var_name, BUTTON_VAR, MAX_SPRITE, MENU_SPRITES};

pub(crate) const REPLAY: &str = "conv_pextras_recollection";
const MENU: &str = "conv_pextras_draw";
const DRAW: &str = "conv_prp_draw";
const WAIT: &str = "conv_prp_wait";
/// 1 while a scene plays from the replay (a local, cleared at the title).
pub(crate) const REPLAYING: &str = "%150";
// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const THUMB: u32 = 900; // + scene
const CURSOR: u32 = 860; // + scene
const MAX_SCENES: usize = 40;
const EXIT: u32 = MENU_SPRITES.start;
const BTN_EXIT: usize = 201;

/// `music`: the lines that play the extras' music again (none without a
/// sound room).
pub(crate) fn emit(out: &mut String, r: &PictureReplay, border: u32, music: Option<&str>) {
    assert!(r.scenes.len() <= MAX_SCENES, "a replay with more scenes than its sprites");
    let place = |i: u32| (r.first.0 + r.column_step * (i / r.rows) as i32, r.first.1 + r.row_step * (i % r.rows) as i32);
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    w(format!("*{REPLAY}"));
    w(format!("gosub *{DRAW}:print 10,{}", r.open_ms));
    w(format!("*{WAIT}"));
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} goto *{MENU}"));
    for (i, _) in r.scenes.iter().enumerate() {
        w(format!("if {BUTTON_VAR} == {} goto *conv_prp_s{i}", i + 1));
    }
    w(format!("gosub *{DRAW}:print 1:goto *{WAIT}"));
    for (i, s) in r.scenes.iter().enumerate() {
        w(format!("*conv_prp_s{i}"));
        w(format!("bgmvolfade 0,{}:wait {}", r.start_ms, r.start_ms));
        w(format!("csp -1:bgmstop:bgmvolfade 100:mov {REPLAYING},1"));
        w(format!("gosub *{}", s.label));
        w(format!("mov {REPLAYING},0"));
        w(format!("bgmvolfade 0,{}:wait {}", r.end_ms, r.end_ms));
        w("csp -1:csp2 -1:bgmstop:stopallsound:bgmvolfade 100:textclear:erasetextwindow 1:bg black,1".into());
        if let Some(m) = music {
            w(m.to_string());
        }
        w(format!("gosub *{DRAW}:print 10,{}", r.open_ms));
        w(format!("goto *{WAIT}"));
    }
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
