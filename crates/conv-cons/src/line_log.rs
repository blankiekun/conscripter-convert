//! [`LineLog`] as an NScripter subroutine under the backlog's label
//! ([`menu::BACKLOG`]), entered the same way as the backlog screen.
//!
//! CONscripter lays the history out as rows (`logrows`, cut at the log's
//! columns, ruby as its base text) and gives each row's text and its
//! page's voice (`getlogrow`, the voice on the page's first row only). The
//! screen takes the game's sprite range between `pushscreen` and
//! `popscreen`: the background, a `strsp` per row shown, a voice icon on
//! the rows that have one. [`TOP`] is the first row shown; drawing puts
//! the knob where it stands between the oldest and the newest rows.
//!
//! `btndown 1` (after each `btndef`, which turns it off) makes `btnwait` return when the button goes down, so a
//! press on the knob starts a drag: while `isdown`, the pointer's y sets
//! the first row. It also returns on the release and on every move with
//! the button held: the screen acts on a press (`isdown` 1) only, then
//! waits for the release itself (`*conv_ll_release`), so no release
//! reaches `btnwait` (a hold over 1.2 s would be a right-click) or, after
//! closing, the text's click wait.

use conv_core::ir::{LineLog, Placed};

use crate::menu::{self, IN_GAME, TMP};
use crate::{path, BUTTON_VAR, MAX_SPRITE, MENU_SPRITES};

// Variables.
/// The first row shown.
const TOP: &str = "%170";
/// The number of rows; the largest TOP.
const ROWS: &str = "%171";
const MAX_TOP: &str = "%174";
const K: &str = "%172";
const Y: &str = "%173";
const SPRITE: &str = "%175";
/// The pointer while dragging; the knob's y.
const MOUSE_X: &str = "%177";
const MOUSE_Y: &str = "%178";
const KNOB_Y: &str = "%179";
const TEXT: &str = "$107";
const VOICE: &str = "$108";
// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const ROW_TEXT: u32 = 960; // + row
const ROW_VOICE: u32 = 930; // + row
const MAX_ROWS: u32 = 30;
// Buttons in the menus' range.
const M: u32 = MENU_SPRITES.start;
const UP: u32 = M;
const DOWN: u32 = M + 1;
const CLOSE: u32 = M + 2;
const KNOB: u32 = M + 3;
const BTN_UP: u32 = 1;
const BTN_DOWN: u32 = 2;
const BTN_CLOSE: u32 = 3;
const BTN_KNOB: u32 = 4;
const BTN_VOICE: u32 = 10; // + row

/// `single_byte`: [`conv_core::ir::Script::single_byte_text`]; each row
/// is drawn after a backquote, which keeps its leading spaces (a
/// paragraph's indent).
pub(crate) fn emit(out: &mut String, l: &LineLog, single_byte: bool) {
    assert!(l.rows <= MAX_ROWS, "a line log shows more rows than its sprites");
    let mut w = lines!(out);
    let lsp = |slot: u32, cells: &str, p: &Placed| format!("lsp {slot},\":a{cells};{}\",{},{}", path(&p.image), p.x, p.y);
    let fade = if l.fade_ms > 0 { format!("print 10,{}", l.fade_ms) } else { "print 1".into() };
    let (k0, k1) = l.knob_y;
    let last = l.rows - 1;

    w(format!("*{}", menu::BACKLOG));
    w(format!("if {IN_GAME} == 1 textoff"));
    w(format!("pushscreen {},{MAX_SPRITE}", crate::SPRITE_BASE));
    // pushscreen keeps the scene's sprites up: the screen covers them.
    w(format!("for {TMP} = {} to {MAX_SPRITE}:csp {TMP}:next", crate::SPRITE_BASE));
    w(format!("logrows {ROWS},{}{}", l.columns, l.row_width.map_or(String::new(), |w| format!(",{w}"))));
    w(format!("mov {MAX_TOP},{ROWS}-{}:if {MAX_TOP} < 0 mov {MAX_TOP},0", l.rows));
    w(format!("mov {TOP},{MAX_TOP}"));
    w(format!("lsp {BACKGROUND},\":c;{}\",{},{}", path(&l.background.image), l.background.x, l.background.y));
    w(lsp(UP, "/4,0,3", &l.up));
    w(lsp(DOWN, "/4,0,3", &l.down));
    w(lsp(CLOSE, "/4,0,3", &l.close));
    w(lsp(KNOB, "/2,0,3", &l.knob));
    w("gosub *conv_ll_draw".into());
    w(fade.clone());

    w("*conv_ll_wait".into());
    w("btndef clear".into());
    for (slot, n) in [(UP, BTN_UP), (DOWN, BTN_DOWN), (CLOSE, BTN_CLOSE)] {
        w(format!("spbtn {slot},{n},3"));
    }
    w(format!("spbtn {KNOB},{BTN_KNOB}"));
    for k in 0..l.rows {
        w(format!("mov {TMP},{TOP}+{k}:getlogrow {TEXT},{VOICE},{TMP}:if {VOICE} != \"\" spbtn {},{},2", ROW_VOICE + k, BTN_VOICE + k));
    }
    // After btndef, which turns it off.
    w("btndown 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    // A right-click and the wheel come without the left button.
    w(format!("if {BUTTON_VAR} == -1 goto *conv_ll_close"));
    // Wheel-up and 逆 go back a row; wheel-down and 進 forward, the wheel
    // past the newest row closes.
    w(format!("if {BUTTON_VAR} == -2 goto *conv_ll_up"));
    w(format!("if {BUTTON_VAR} == -3 && {TOP} >= {MAX_TOP} goto *conv_ll_close"));
    w(format!("if {BUTTON_VAR} == -3 goto *conv_ll_down"));
    // A release, or a move with the button held: nothing.
    w(format!("isdown {TMP}"));
    w(format!("if {TMP} == 0 goto *conv_ll_wait"));
    w(format!("if {BUTTON_VAR} == {BTN_KNOB} goto *conv_ll_drag"));
    w("gosub *conv_ll_release".into());
    w(format!("if {BUTTON_VAR} == {BTN_CLOSE} goto *conv_ll_close"));
    w(format!("if {BUTTON_VAR} == {BTN_UP} goto *conv_ll_up"));
    w(format!("if {BUTTON_VAR} == {BTN_DOWN} goto *conv_ll_down"));
    w(format!(
        "if {BUTTON_VAR} >= {BTN_VOICE} && {BUTTON_VAR} <= {} mov {TMP},{TOP}+{BUTTON_VAR}-{BTN_VOICE}:getlogrow {TEXT},{VOICE},{TMP}:dwave 0,{VOICE}",
        BTN_VOICE + last
    ));
    w("goto *conv_ll_wait".into());
    w("*conv_ll_up".into());
    w(format!("if {TOP} > 0 sub {TOP},1:gosub *conv_ll_draw:print 1"));
    w("goto *conv_ll_wait".into());
    w("*conv_ll_down".into());
    w(format!("if {TOP} < {MAX_TOP} add {TOP},1:gosub *conv_ll_draw:print 1"));
    w("goto *conv_ll_wait".into());
    w("*conv_ll_release".into());
    w(format!("isdown {TMP}"));
    w(format!("if {TMP} == 1 wait 16:goto *conv_ll_release"));
    w("return".into());

    // Dragging the knob: the pointer's y, less where it took the knob,
    // sets the first row.
    w("*conv_ll_drag".into());
    w(format!("getmousepos {MOUSE_X},{MOUSE_Y}:mov {MOUSE_X},{MOUSE_Y}-{KNOB_Y}"));
    w(format!("cell {KNOB},1:print 1"));
    w("*conv_ll_drag_loop".into());
    w(format!("isdown {TMP}"));
    w(format!("if {TMP} == 0 goto *conv_ll_drag_end"));
    w(format!("getmousepos {TMP},{MOUSE_Y}"));
    if k1 > k0 {
        w(format!("mov {TMP},({MOUSE_Y}-{MOUSE_X}-{k0})*{MAX_TOP}*2+{}", k1 - k0));
        w(format!("div {TMP},{}", 2 * (k1 - k0)));
    } else {
        w(format!("mov {TMP},{MAX_TOP}"));
    }
    w(format!("if {TMP} < 0 mov {TMP},0"));
    w(format!("if {TMP} > {MAX_TOP} mov {TMP},{MAX_TOP}"));
    w(format!("if {TMP} != {TOP} mov {TOP},{TMP}:gosub *conv_ll_draw:cell {KNOB},1:print 1"));
    w("wait 16".into());
    w("goto *conv_ll_drag_loop".into());
    w("*conv_ll_drag_end".into());
    w(format!("cell {KNOB},0:print 1"));
    w("goto *conv_ll_wait".into());

    w("*conv_ll_close".into());
    w("btndown 0:btndef clear".into());
    w([UP, DOWN, CLOSE, KNOB].map(|n| format!("csp {n}")).join(":"));
    w("popscreen".into());
    // The scene first, then the window: texton drops the pending screen
    // updates but the window's.
    w(fade);
    w(format!("if {IN_GAME} == 1 texton"));
    w("return".into());

    // The rows from TOP, their voice icons, the knob.
    w("*conv_ll_draw".into());
    w(format!("for {TMP} = {ROW_VOICE} to {}:csp {TMP}:next", ROW_VOICE + last));
    w(format!("for {TMP} = {ROW_TEXT} to {}:csp {TMP}:next", ROW_TEXT + last));
    w(format!("mov {K},0"));
    w("*conv_ll_row".into());
    w(format!("if {K} > {last} goto *conv_ll_knob"));
    w(format!("mov {TMP},{TOP}+{K}"));
    w(format!("if {TMP} >= {ROWS} goto *conv_ll_knob"));
    w(format!("getlogrow {TEXT},{VOICE},{TMP}"));
    w(format!("mov {Y},{K}*{}+{}", l.row_step, l.text_y));
    // The outline is the strsp's shadow, drawn as one by strspoutline.
    let (outline_on, shadow, outline_off) = match &l.outline {
        Some(c) => (format!("strspoutline \"{c}\":"), 1, ":strspoutline \"\""),
        None => (String::new(), 0, ""),
    };
    w(format!(
        "if {TEXT} != \"\" mov {SPRITE},{ROW_TEXT}+{K}:{outline_on}strsp {SPRITE},{}{TEXT},{},{Y},{},1,{s},{s},0,0,0,{shadow},\"{}\"{outline_off}",
        if single_byte { "\"`\"+" } else { "" },
        l.text_x,
        // A row is cut at the columns, but strsp wraps a full-width
        // character one column early: a full row would lose its last.
        l.columns + 1,
        l.colour,
        s = l.font_size
    ));
    let v = &l.voice_icon;
    w(format!("if {VOICE} != \"\" mov {SPRITE},{ROW_VOICE}+{K}:lsp {SPRITE},\":a/3,0,3;{}\",{},{Y}+{}", path(&v.image), v.x, v.y));
    w(format!("add {K},1"));
    w("goto *conv_ll_row".into());
    w("*conv_ll_knob".into());
    w(format!("mov {KNOB_Y},{k1}"));
    w(format!("if {MAX_TOP} > 0 mov {KNOB_Y},{TOP}*{}/{MAX_TOP}+{k0}", k1 - k0));
    w(format!("amsp {KNOB},{},{KNOB_Y}", l.knob.x));
    w("return".into());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(name: &str, x: i32, y: i32) -> Placed {
        Placed { image: format!("system/{name}.png"), x, y, w: 30, h: 60 }
    }

    #[test]
    fn line_log_routine() {
        let l = LineLog {
            background: placed("bg", 0, 0),
            up: placed("up", 898, 168),
            down: placed("down", 898, 303),
            close: placed("close", 927, 438),
            knob: placed("knob", 860, 461),
            knob_y: (50, 461),
            voice_icon: placed("voice", 102, 3),
            text_x: 140,
            text_y: 49,
            row_step: 30,
            rows: 16,
            columns: 26,
            font_size: 20,
            colour: "#000000".into(),
            fade_ms: 300,
            outline: None,
            row_width: None,
            engine_screen: false,
        };
        let mut out = String::new();
        emit(&mut out, &l, false);
        for line in [
            "*conv_backlog\n",
            "logrows %171,26\n",
            "mov %174,%171-16:if %174 < 0 mov %174,0\nmov %170,%174\n",
            // The last row's voice button.
            "mov %105,%170+15:getlogrow $107,$108,%105:if $108 != \"\" spbtn 945,25,2\n",
            "if %9 == -3 && %170 >= %174 goto *conv_ll_close\n",
            "strsp %175,$107,140,%173,27,1,20,20,0,0,0,0,\"#000000\"\n",
            "lsp %175,\":a/3,0,3;system\\voice.png\",102,%173+3\n",
            "if %174 > 0 mov %179,%170*411/%174+50\n",
            "btndown 0:btndef clear\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
        let mut out = String::new();
        emit(&mut out, &LineLog { row_width: Some(570), ..l }, true);
        assert!(out.contains("strsp %175,\"`\"+$107,140,%173,"), "{out}");
        assert!(out.contains("logrows %171,26,570\n"), "{out}");
    }
}
