//! The chapter history (see [`ChapterHistory`]): the chapters reached since
//! the history was last cleared, in columns over the scene, opened from the
//! chapter bar ([`TopBars::chapter_history`]).
//!
//! Each [`Op::Chapter`](conv_core::ir::Op::Chapter) calls [`KEEP`], which
//! keeps the game as it is there (`savepoint`, then `savegame2` with the
//! chapter's title) in save slot [`SLOT`] + its entry. [`CURRENT`] is the
//! current chapter's entry and [`COUNT`] the entries listed, both locals, so
//! a save keeps its own history. Going back to an entry loads its slot and
//! carries [`COUNT`] over the load ([`CARRY`], read by [`LOADED`]), so the
//! chapters after it stay listed until a new one is reached.
//!
//! The titles of a column are one `strsp`, each padded to a full row; a
//! timed `btnwait` (`btntime`) follows the pointer to move the highlight
//! (one sprite) to the row under it, and one invisible `btn` over the list
//! takes the click.

use conv_core::ir::{ChapterHistory, TopBars};

use crate::menu::{CONFIRM, CONFIRM_ROUTINE, CONFIRM_TEXT, IN_GAME, TMP};
use crate::{escape, path, BUTTON_VAR, MENU_SPRITES};

/// Save slot of entry n: SLOT + n (n from 1; the save menu's are 1-50).
pub(crate) const SLOT: u32 = 700;
/// The entries listed and the current chapter's (locals; 0: none).
pub(crate) const COUNT: &str = "%193";
pub(crate) const CURRENT: &str = "%194";
/// [`COUNT`] + 1 while going back to an entry (a global: it outlasts the
/// load); 0 otherwise.
const CARRY: &str = "%2909";
/// Keeps the chapter just started (after its title is set).
pub(crate) const KEEP: &str = "conv_chapter_keep";
/// Opens the screen (from the bar's action).
pub(crate) const OPEN: &str = "conv_chapters";
/// Part of the `loadgosub` routine: puts back the entries carried over.
pub(crate) const LOADED: &str = "conv_chapters_loaded";
/// The chapter bar's button number.
pub(crate) const BTN_OPEN: i32 = crate::menu::BAR_BUTTON_BASE + 310;

// Sprites (within MENU_SPRITES): the columns' titles, the current
// chapter's mark, the highlight, the close button.
const M: u32 = MENU_SPRITES.start;
const TITLES: u32 = M; // + column
const MARK: u32 = M + 4;
const HIGHLIGHT: u32 = M + 5;
const CLOSE: u32 = M + 6;
// lsp2 slots, as the other screens': the base and the frame.
const BASE2: u32 = 11;
const FRAME2: u32 = 10;
const BTN_LIST: i32 = 1;
const BTN_CLOSE: i32 = 2;
/// The entry under the pointer (0: none) and the column being built.
const HOVER: &str = "%195";
const COLUMN_TEXT: &str = "$112";
const TITLE: &str = "$101";
const MOUSE_X: &str = "%103";
const MOUSE_Y: &str = "%104";
/// A title padded to a full row: this many columns of the `strsp`, two
/// bytes each.
const ROW_CHARS: u32 = 20;

/// The routines: [`KEEP`], [`OPEN`] and [`LOADED`].
pub(crate) fn emit(out: &mut String, h: &ChapterHistory) {
    let mut w = lines!(out);
    let max = h.rows * h.columns;

    w(format!("*{KEEP}"));
    w(format!("if {CURRENT} >= {max} return"));
    w(format!("add {CURRENT},1:mov {COUNT},{CURRENT}"));
    // A load resumes here: the slot is written again, the same.
    w("savepoint".into());
    w(format!("savegame2 {SLOT}+{CURRENT},{}", crate::menu::CHAPTER));
    w("return".into());

    w(format!("*{LOADED}"));
    w(format!("if {CARRY} > 0 mov {COUNT},{CARRY}-1:mov {CARRY},0"));
    w("return".into());

    let lsp2 = |slot: u32, p: &conv_core::ir::Placed, opacity: Option<u8>| {
        let o = opacity.map_or(String::new(), |o| format!(",{o}"));
        format!("lsp2 {slot},\":a;{}\",{},{},100,100,0{o}", path(&p.image), p.x + p.w / 2, p.y + p.h / 2)
    };
    let others = crate::menu::hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    // Where entry %n's row is: column and row from the entry.
    let row_x = |n: &str| format!("{}+({n}-1)/{}*{}", h.x, h.rows, h.column_pitch);
    let row_y = |n: &str| format!("{}+({n}-1) mod {}*{}", h.y, h.rows, h.row_pitch);

    w(format!("*{OPEN}"));
    // Only opened in the game (from the chapter bar).
    w(format!("mov {IN_GAME},1:textoff:{}", vsp(0)));
    w(format!("gosub *{}", crate::menu::BAR_CLEAR));
    w(lsp2(BASE2, &h.base, Some(h.base_opacity)));
    w(lsp2(FRAME2, &h.frame, None));
    w(format!("lsp {CLOSE},\":a/3,0,3;{}\",{},{}", path(&h.close.image), h.close.x, h.close.y));
    // A column's titles, each padded to a full row.
    for c in 0..h.columns {
        let (first, last) = (c * h.rows + 1, (c + 1) * h.rows);
        w(format!("mov {COLUMN_TEXT},\"\""));
        w(format!("for {TMP}={first} to {last}"));
        w(format!("if {TMP} > {COUNT} break"));
        w(format!("getsavestr {TITLE},{SLOT}+{TMP}"));
        w(format!("*conv_chapters_pad{c}"));
        w(format!("len {MOUSE_X},{TITLE}"));
        w(format!("if {MOUSE_X} < {} add {TITLE},\"　\":goto *conv_chapters_pad{c}", ROW_CHARS * 2));
        w(format!("add {COLUMN_TEXT},{TITLE}"));
        w("next".into());
        let (x, y) = (h.x + c as i32 * h.column_pitch + h.text_dx, h.y + h.text_dy);
        let pitch = h.row_pitch - h.font_size as i32;
        w(format!(
            "strsp {},{COLUMN_TEXT},{x},{y},{},{},{},{},0,{pitch},0,1,\"#FFFFFF\"",
            TITLES + c,
            ROW_CHARS,
            h.rows,
            h.font_size,
            h.font_size
        ));
    }
    let (cur, hov) = (&h.current, &h.hover);
    w(format!(
        "if {CURRENT} > 0 && {CURRENT} <= {COUNT} lsp {MARK},\":a;{}\",{}+{},{}+{}",
        path(&cur.image),
        row_x(CURRENT),
        cur.x,
        row_y(CURRENT),
        cur.y
    ));
    w(format!("lsp {HIGHLIGHT},\":a;{}\",0,0:vsp {HIGHLIGHT},0", path(&hov.image)));
    w(format!("mov {HOVER},0"));
    // Follows the pointer every 50 ms; a click on the list takes the row.
    w("*conv_chapters_wait".into());
    w("btndef clear".into());
    w(format!(
        "btn {BTN_LIST},{},{},{},{},0,0",
        h.x,
        h.y,
        h.columns as i32 * h.column_pitch,
        h.rows as i32 * h.row_pitch
    ));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    w("btntime 50".into());
    w(format!("getmousepos {MOUSE_X},{MOUSE_Y}"));
    w(format!("mov {TMP},0"));
    w(format!(
        "if {MOUSE_X} >= {} && {MOUSE_Y} >= {} && {MOUSE_X} < {} && {MOUSE_Y} < {} mov {TMP},({MOUSE_X}-{})/{}*{}+({MOUSE_Y}-{})/{}+1",
        h.x,
        h.y,
        h.x + h.columns as i32 * h.column_pitch,
        h.y + h.rows as i32 * h.row_pitch,
        h.x,
        h.column_pitch,
        h.rows,
        h.y,
        h.row_pitch
    ));
    // Past a column's rows, or past the last entry: none.
    w(format!("if {TMP} > 0 && ({MOUSE_X}-{}) mod {} >= {} mov {TMP},0", h.x, h.column_pitch, h.row_width));
    w(format!("if {TMP} > {COUNT} mov {TMP},0"));
    w(format!("if {TMP} != {HOVER} && {TMP} == 0 vsp {HIGHLIGHT},0"));
    w(format!("if {TMP} != {HOVER} && {TMP} > 0 amsp {HIGHLIGHT},{},{}:vsp {HIGHLIGHT},1", row_x(TMP), row_y(TMP)));
    w(format!("mov {HOVER},{TMP}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w("btndef clear".into());
    w(format!("if {BUTTON_VAR} == {BTN_LIST} && {HOVER} > 0 goto *conv_chapters_go"));
    w(format!("if {BUTTON_VAR} == {BTN_CLOSE} || {BUTTON_VAR} == -1 goto *conv_chapters_close"));
    w("goto *conv_chapters_wait".into());

    w("*conv_chapters_go".into());
    w(format!("getsavestr {TITLE},{SLOT}+{HOVER}"));
    w(format!("mov {CONFIRM_TEXT},\"{}\"", escape(&h.question.0)));
    w(format!("add {CONFIRM_TEXT},{TITLE}"));
    w(format!("add {CONFIRM_TEXT},\"{}\"", escape(&h.question.1)));
    // Switched off, it goes without asking.
    w(format!("mov {CONFIRM},1:if {} == 0 mov {},0:gosub *{CONFIRM_ROUTINE}", crate::menu::switch_off(conv_core::ir::Switch::ConfirmChapter), crate::menu::CONFIRM_KIND));
    w(format!("if {CONFIRM} == 0 goto *conv_chapters_wait"));
    w(format!("mov {CARRY},{COUNT}+1"));
    w("gosub *conv_chapters_clear".into());
    w(format!("loadgame {SLOT}+{HOVER}"));
    // Still here: the load failed.
    w(format!("mov {CARRY},0"));

    w("*conv_chapters_close".into());
    w("gosub *conv_chapters_clear".into());
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    w("print 1".into());
    w("return".into());

    w("*conv_chapters_clear".into());
    w(format!("csp2 {BASE2}:csp2 {FRAME2}"));
    w((TITLES..=CLOSE).map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("return".into());
}

/// The chapter bar's button, drawn with the top bars: an invisible sprite
/// in `slot` (a bar sprite, so auto-play's ignore list keeps it out).
pub(crate) fn hotspot(top: &TopBars, slot: u32) -> String {
    let b = &top.chapter_bar;
    format!("lsp {slot},\":c;>{},{},#000000\",{},{},0:spbtn {slot},{BTN_OPEN}", b.w, b.h, b.x, b.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::Placed;

    #[test]
    fn keeps_lists_and_goes_back() {
        let p = |name: &str, w, h| Placed { image: format!("system/{name}.png"), x: 0, y: 0, w, h };
        let h = ChapterHistory {
            base: p("base", 800, 600),
            base_opacity: 140,
            frame: p("frame", 800, 600),
            close: p("close", 216, 21),
            hover: p("hover", 258, 25),
            current: Placed { x: -16, ..p("current", 271, 25) },
            x: 16,
            y: 83,
            row_pitch: 24,
            column_pitch: 258,
            rows: 19,
            columns: 3,
            row_width: 256,
            text_dx: 5,
            text_dy: 5,
            font_size: 13,
            question: ("「".into(), "」？".into()),
        };
        let top = crate::menu::tests::top_bars();
        let mut out = String::new();
        emit(&mut out, &h);
        for line in [
            // A chapter's start, kept in its slot with its title.
            "*conv_chapter_keep\nif %194 >= 57 return\nadd %194,1:mov %193,%194\nsavepoint\nsavegame2 700+%194,$150\nreturn\n",
            "*conv_chapters_loaded\nif %2909 > 0 mov %193,%2909-1:mov %2909,0\nreturn\n",
            // The second column: entries 20-38, padded to a row.
            "for %105=20 to 38\nif %105 > %193 break\ngetsavestr $101,700+%105\n*conv_chapters_pad1\nlen %103,$101\nif %103 < 40 add $101,\"　\":goto *conv_chapters_pad1\n",
            "strsp 7,$112,279,88,20,19,13,13,0,11,0,1,\"#FFFFFF\"\n",
            "if %194 > 0 && %194 <= %193 lsp 10,\":a;system\\current.png\",16+(%194-1)/19*258+-16,83+(%194-1) mod 19*24+0\n",
            "btntime 50\n",
            "mov %2909,%193+1\ngosub *conv_chapters_clear\nloadgame 700+%195\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
        assert_eq!(hotspot(&top, 48), "lsp 48,\":c;>400,28,#000000\",10,20,0:spbtn 48,1310");
    }
}
