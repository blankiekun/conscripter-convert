//! The scene replay (see [`SceneReplay`]), reached from the extras menu.
//!
//! The menu of lists uses the extras' sprite slots; an open list uses
//! [`TEXT`] and up, drawn over the darkened menu. Only unlocked scenes
//! are listed (a row routine places them one after another), each with a
//! "new" mark until it has been replayed ([`REPLAYED_BASE`]).
//!
//! A title jumps to its scene's label with [`REPLAYING`] set; the scene's
//! end ([`conv_core::ir::Op::SceneEnd`]) and the settings screen's
//! "back to the list" jump to [`END`], which stops the sound, clears the
//! screen and shows the list again. Both leave the scene's subroutine
//! calls unfinished: the replay never returns into them, and loading or
//! going back to the title starts over.

use conv_core::ir::SceneReplay;

use crate::{path, sprite_text, BUTTON_VAR, SCENE_UNLOCKED_BASE};

pub(crate) const REPLAY: &str = "conv_replay";
/// Where a replayed scene ends.
pub(crate) const END: &str = "conv_replay_end";
/// 1 while a scene is replayed (saving is off, the settings screen leads
/// back to the list).
pub(crate) const REPLAYING: &str = "%152";
/// Global flags: scene n has been replayed (no "new" mark); above the
/// settings switches, n up to 445 (the switches' flags are from 4096).
const REPLAYED_BASE: u32 = 3650;
const VARIABLE_RANGE: u32 = 4096;
const DRAW: &str = "conv_replay_draw";
const LIST: &str = "conv_replay_list";
const LIST_WAIT: &str = "conv_replay_list_wait";
const ROW: &str = "conv_replay_row";
const BEGIN: &str = "conv_replay_begin";
// Sprites: the menu in the extras' slots, the list above it (a lower
// number is drawn on top).
const HEADER: u32 = 100;
const EXIT: u32 = 101;
const LIST_BUTTON: u32 = 102; // + list
const TEXT: u32 = 200; // + row
const NEW: u32 = 270; // + row
const SELECTOR: u32 = 340; // + row
const CLOSE: u32 = 410;
const FRAME: u32 = 411;
const SELECTED: u32 = 412;
const MAX_ROWS: usize = 69;
// Buttons.
const BTN_EXIT: i32 = 99;
const BTN_CLOSE: i32 = 999;
// Variables (cleared when the extras close).
const LIST_VAR: &str = "%153";
const ROW_VAR: &str = "%154";
/// The row's button, "new" (1) and highlight (1) for the row routine.
const ROW_BUTTON: &str = "%155";
const ROW_NEW: &str = "%156";
const ROW_HIGHLIGHT: &str = "%157";
const X: &str = "%158";
const Y: &str = "%159";
const TMP: &str = "%160";
const ROW_TITLE: &str = "$130";
/// The replay's variables are %152-%160.
const FIRST_VAR: u32 = 152;
const LAST_VAR: u32 = 160;
pub(crate) const VARS: [&str; 9] = [REPLAYING, LIST_VAR, ROW_VAR, ROW_BUTTON, ROW_NEW, ROW_HIGHLIGHT, X, Y, TMP];

fn unlocked(scene: u32) -> String {
    format!("%{}", SCENE_UNLOCKED_BASE + scene)
}

/// `background`: the extras menu's; `back`: the extras menu's label.
pub(crate) fn replay(out: &mut String, r: &SceneReplay, background: &str, back: &str) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let lists = &r.lists[..r.lists.len().min(98)];
    let max_rows = (r.rows * r.columns).min(MAX_ROWS);

    // The menu: a list is disabled while none of its scenes is unlocked.
    w(format!("*{REPLAY}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(background)));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("lsp {HEADER},\":a;{}\",{},{}", path(&r.header.image), r.header.x, r.header.y));
    w(format!("lsp {EXIT},\":a/2,0,3;{}\",{},{}:spbtn {EXIT},{BTN_EXIT}", path(&r.exit.image), r.exit.x, r.exit.y));
    for (i, l) in lists.iter().enumerate() {
        let slot = LIST_BUTTON + i as u32;
        w(format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&l.button.image), l.button.x, l.button.y));
        w(format!("mov {TMP},0"));
        for chunk in l.scenes.chunks(8) {
            w(chunk.iter().map(|s| format!("if {} == 1 mov {TMP},1", unlocked(s.scene))).collect::<Vec<_>>().join(":"));
        }
        w(format!("if {TMP} == 0 cell {slot},2"));
        w(format!("if {TMP} == 1 spbtn {slot},{}", i + 1));
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} csp -1:goto *{back}"));
    w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {} mov {LIST_VAR},{BUTTON_VAR}-1:goto *{LIST}", lists.len()));
    w(format!("goto *{DRAW}"));

    // An open list: the unlocked scenes' rows, in order.
    w(format!("*{LIST}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(&r.list_background)));
    for (i, l) in lists.iter().enumerate() {
        w(format!("if {LIST_VAR} == {i} lsp {SELECTED},\":a;{}\",{},{}", path(&l.selected.image), l.selected.x, l.selected.y));
    }
    w(format!("lsp {FRAME},\":a;{}\",{},{}", path(&r.frame.image), r.frame.x, r.frame.y));
    w(format!("lsp {CLOSE},\":a/2,0,3;{}\",{},{}:spbtn {CLOSE},{BTN_CLOSE}", path(&r.close.image), r.close.x, r.close.y));
    w(format!("mov {ROW_VAR},0"));
    let rows_label = |i: usize| format!("*conv_replay_rows{i}");
    w(format!("tablegoto {LIST_VAR},{}", (0..lists.len()).map(rows_label).collect::<Vec<_>>().join(",")));
    w(format!("goto *{LIST_WAIT}"));
    for (i, l) in lists.iter().enumerate() {
        w(rows_label(i));
        for (k, s) in l.scenes.iter().enumerate().take(max_rows) {
            let new = if REPLAYED_BASE + s.scene < VARIABLE_RANGE { format!("1-%{}", REPLAYED_BASE + s.scene) } else { "0".into() };
            w(format!(
                "if {} == 1 mov {ROW_TITLE},\"{}\":mov {ROW_BUTTON},{}:mov {ROW_NEW},{new}:mov {ROW_HIGHLIGHT},{}:gosub *{ROW}",
                unlocked(s.scene),
                sprite_text(&s.title),
                k + 1,
                u8::from(s.highlight)
            ));
        }
        w(format!("goto *{LIST_WAIT}"));
    }
    w(format!("*{LIST_WAIT}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *{DRAW}"));
    w(format!("if {BUTTON_VAR} < 1 goto *{LIST}"));
    w(format!("mov {TMP},{BUTTON_VAR}-1"));
    w(format!("tablegoto {LIST_VAR},{}", (0..lists.len()).map(|i| format!("*conv_replay_play{i}")).collect::<Vec<_>>().join(",")));
    w(format!("goto *{LIST}"));
    for (i, l) in lists.iter().enumerate() {
        let scenes = &l.scenes[..l.scenes.len().min(max_rows)];
        w(format!("*conv_replay_play{i}"));
        w(format!("tablegoto {TMP},{}", (0..scenes.len()).map(|k| format!("*conv_replay_go{i}_{k}")).collect::<Vec<_>>().join(",")));
        w(format!("goto *{LIST}"));
        for (k, s) in scenes.iter().enumerate() {
            w(format!("*conv_replay_go{i}_{k}"));
            if REPLAYED_BASE + s.scene < VARIABLE_RANGE {
                w(format!("mov %{},1", REPLAYED_BASE + s.scene));
            }
            w(format!("gosub *{BEGIN}:goto *{}", crate::label(&s.label)));
        }
    }

    // A row at ROW_VAR: column by column, top to bottom.
    w(format!("*{ROW}"));
    w(format!("if {ROW_VAR} >= {max_rows} return"));
    w(format!("mov {X},{ROW_VAR}/{}*{}:mov {Y},{ROW_VAR} mod {}*{}", r.rows, r.column_dx, r.rows, r.row_dy));
    w(format!("mov {TMP},{SELECTOR}+{ROW_VAR}"));
    w(format!("lsp {TMP},\":a/2,0,3;{}\",{X}+{},{Y}+{}:spbtn {TMP},{ROW_BUTTON}", path(&r.selector), r.selector_x, r.row_y));
    w(format!("mov {TMP},{NEW}+{ROW_VAR}"));
    w(format!("if {ROW_NEW} == 1 lsp {TMP},\":a;{}\",{X}+{},{Y}+{}", path(&r.new_mark), r.new_x, r.row_y));
    w(format!("mov {TMP},{TEXT}+{ROW_VAR}"));
    let size = r.font_size;
    for (h, color) in [(0, &r.text_color), (1, &r.highlight_color)] {
        w(format!(
            "if {ROW_HIGHLIGHT} == {h} strsp {TMP},{ROW_TITLE},{X}+{},{Y}+{},40,1,{size},{size},0,0,0,1,\"{color}\"",
            r.text_x, r.text_y
        ));
    }
    w(format!("add {ROW_VAR},1"));
    w("return".into());

    // A clean screen and story state for the scene.
    w(format!("*{BEGIN}"));
    w("csp -1:csp2 -1:bgmstop".into());
    w("bg black,1".into());
    // The playthrough's variables as a new game has them, but the replay's.
    w(format!("for {TMP}=10 to {}:mov %{TMP},0:next", FIRST_VAR - 1));
    w(format!("for {TMP}={} to 199:mov %{TMP},0:next", LAST_VAR + 1));
    w(format!("mov {},\"\"", crate::menu::CHAPTER));
    w(format!("mov {REPLAYING},1"));
    w("return".into());
    w(format!("*{END}"));
    w(format!("mov {REPLAYING},0"));
    w(format!("bgmstop:{}", (0..10).map(|c| format!("dwavestop {c}")).collect::<Vec<_>>().join(":")));
    w("textclear:textoff".into());
    w("csp -1:csp2 -1".into());
    w(format!("goto *{LIST}"));
}
