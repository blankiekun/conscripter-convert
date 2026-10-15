//! [`SaveGrid`] as NScripter subroutines under the save menu's labels
//! ([`menu::SAVE`], [`menu::LOAD`]), entered the same way.
//!
//! The screen takes the game's sprite range for itself between `pushscreen`
//! and `popscreen` (the scene comes back as it was): the page's picture at
//! the bottom, the thumbnails, their frames, then per slot an invisible hit
//! sprite and its details, hidden. Each hit sprite is an `exbtn` whose
//! hover command shows that slot's details and moves the hover frame onto
//! it; `exbtn_d` hides them again, so `btnwait` returns only for clicks.
//! The buttons are `lsp` sprites in the menus' range, over all of it.
//!
//! A save's label (`savegame2`, read back with `getsavestr`) holds its date,
//! play time, chapter and last line, `|` between them; its thumbnail is the
//! screen as the screen opened, saved as `savethumbN.bmp` like the save
//! menu's. The play time is CONscripter's (`getplaytime`), which saves
//! keep; a new game sets it to 0 ([`NEW_GAME`]).
//!
//! With link scene levels (`scene_stack`), a save keeps the levels' slots
//! and a load puts them back, as the save menu does.

use conv_core::ir::{ConfirmDialog, GridText, Placed, SaveGrid, Switch};

use crate::menu::{
    self, switch_index, CONFIRM, CONFIRM_KIND, CONFIRM_ROUTINE, CONFIRM_TEXT, DATE, FILE, FIRST, HAS_DATA, IN_GAME, LABEL, MODE, NEWEST, PAGE, SCENE_LOAD_SLOT, SELECTED, SLOT, STR_TMP,
    THUMB_IMAGE, TMP,
};
use crate::{escape, path, BUTTON_VAR, DIALOG_SPRITES, MAX_SPRITE, MENU_SPRITES};

/// The play time in seconds, read for a save's label.
const PLAY: &str = "%147";
/// What a new game runs.
pub(crate) const NEW_GAME: &str = "setplaytime 0";
/// 1 in delete mode.
const DELETING: &str = "%149";
/// A slot's details: date, play time, chapter, last line.
const TEXTS: [&str; 4] = ["$110", "$111", "$112", "$113"];
// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const THUMBS: u32 = 960; // + slot
const FRAMES: u32 = 950;
const HITS: u32 = 920; // + slot
const NEW: u32 = 919;
const HOVER: u32 = 918;
/// Details of slot i: DETAILS + 5i + date, play time, chapter, line,
/// preview.
const DETAILS: u32 = 700;
const MAX_SLOTS: u32 = 30;
// Buttons in the menus' range.
const M: u32 = MENU_SPRITES.start;
const PREVIOUS: u32 = M;
const NEXT: u32 = M + 1;
const DELETE_ON: u32 = M + 2;
const DELETE: u32 = M + 3;
const CLOSE: u32 = M + 4;
const BTN_PREVIOUS: u32 = 101;
const BTN_NEXT: u32 = 102;
const BTN_DELETE: u32 = 103;
const BTN_CLOSE: u32 = 104;

/// `confirm`: the script's yes/no dialog (none: act without asking).
pub(crate) fn emit(out: &mut String, g: &SaveGrid, confirm: Option<&ConfirmDialog>, width: u32, height: u32, scene_stack: bool) {
    let slots = g.rows * g.columns;
    assert!(slots <= MAX_SLOTS, "a save grid has more slots a page than its sprites");
    let pages = g.save_pages.len().min(g.load_pages.len()).max(1);
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let place = |i: u32| (g.first.0 + g.column_step * (i / g.rows) as i32, g.first.1 + g.row_step * (i % g.rows) as i32);
    let button = |slot: u32, p: &Placed| format!("lsp {slot},\":a/4,0,3;{}\",{},{}", path(&p.image), p.x, p.y);
    let fade = if g.fade_ms > 0 { format!("print 10,{}", g.fade_ms) } else { "print 1".into() };
    // Asks; no: back to waiting.
    let ask = |kind: Switch| match confirm {
        Some(c) => {
            let question = match kind {
                Switch::ConfirmSave => &c.overwrite,
                Switch::ConfirmLoad => &c.load,
                _ => &c.delete,
            };
            format!(
                "mov {CONFIRM_KIND},{}:mov {CONFIRM_TEXT},\"{}\"\ngosub *{CONFIRM_ROUTINE}\nif {CONFIRM} == 0 goto *conv_sg_wait",
                switch_index(kind) + 1,
                escape(question)
            )
        }
        None => String::new(),
    };

    w(format!("*{}", menu::SAVE));
    w(format!("mov {MODE},1:goto *conv_sg"));
    w(format!("*{}", menu::LOAD));
    w(format!("mov {MODE},2"));
    w("*conv_sg".into());
    // The thumbnail: the scene as it is before the menu covers it, without
    // the text window.
    w(format!("if {IN_GAME} == 1 textoff"));
    w(format!("if {MODE} == 1 getscreenshot {},{}", g.thumb_w, g.thumb_h));
    w(format!("pushscreen {},{MAX_SPRITE}", crate::ANIMATION_SPRITE));
    w(format!("mov {DELETING},0"));
    w(format!("if {PAGE} < 1 || {PAGE} > {pages} mov {PAGE},1"));
    w("gosub *conv_sg_draw".into());
    w(button(PREVIOUS, &g.previous));
    w(button(NEXT, &g.next));
    w(button(DELETE, &g.delete));
    w(button(CLOSE, &g.close));
    w(fade.clone());

    w("*conv_sg_wait".into());
    w("btndef clear".into());
    let all_details = format!("C{HOVER}C{}-{}", DETAILS, DETAILS + 5 * slots - 1);
    w(format!("exbtn_d \"{all_details}\""));
    for i in 0..slots {
        let (x, y) = place(i);
        let shown: String = (0..5).map(|k| format!("P{}", DETAILS + 5 * i + k)).collect();
        let (hx, hy) = (x + g.hover_offset.0, y + g.hover_offset.1);
        w(format!("exbtn {},{},\"{all_details}P{HOVER}M{HOVER},{hx},{hy}{shown}\"", HITS + i, i + 1));
    }
    for (slot, n) in [(PREVIOUS, BTN_PREVIOUS), (NEXT, BTN_NEXT), (DELETE, BTN_DELETE), (CLOSE, BTN_CLOSE)] {
        w(format!("spbtn {slot},{n},3"));
    }
    w("getkey".into());
    w(format!("btnwait {BUTTON_VAR}"));
    // A right-click, or the screen's key (S saving, L loading), leaves.
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *conv_sg_close"));
    w(format!("if {MODE} == 1 && {BUTTON_VAR} == {} goto *conv_sg_close", menu::key_code('S')));
    w(format!("if {MODE} == 2 && {BUTTON_VAR} == {} goto *conv_sg_close", menu::key_code('L')));
    w(format!("if {BUTTON_VAR} == {BTN_PREVIOUS} sub {PAGE},1:if {PAGE} < 1 mov {PAGE},{pages}"));
    w(format!("if {BUTTON_VAR} == {BTN_NEXT} add {PAGE},1:if {PAGE} > {pages} mov {PAGE},1"));
    w(format!("if {BUTTON_VAR} == {BTN_PREVIOUS} || {BUTTON_VAR} == {BTN_NEXT} gosub *conv_sg_draw:{fade}:goto *conv_sg_wait"));
    w(format!("if {BUTTON_VAR} == {BTN_DELETE} mov {DELETING},1-{DELETING}:gosub *conv_sg_delete_mode:print 10,150:goto *conv_sg_wait"));
    w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {slots} goto *conv_sg_slot"));
    w("goto *conv_sg_wait".into());

    w("*conv_sg_delete_mode".into());
    w(format!("csp {DELETE_ON}"));
    w(format!("if {DELETING} == 1 lsp {DELETE_ON},\":a;{}\",{},{}", path(&g.delete_on.image), g.delete_on.x, g.delete_on.y));
    w("return".into());

    // A slot was clicked.
    w("*conv_sg_slot".into());
    w(format!("mov {SLOT},{FIRST}+{BUTTON_VAR}-1"));
    w("gosub *conv_sg_data".into());
    w(format!("if {DELETING} == 1 goto *conv_sg_delete"));
    w(format!("if {MODE} == 2 goto *conv_sg_load"));
    w(format!("if {HAS_DATA} == 0 goto *conv_sg_save"));
    w(ask(Switch::ConfirmSave));
    w("*conv_sg_save".into());
    w(format!("savescreenshot2 {FILE}"));
    // Label: date|play time|chapter|last line.
    w(format!("date {},{},{}", DATE[0], DATE[1], DATE[2]));
    w(format!("time {},{},{}", DATE[3], DATE[4], DATE[5]));
    w(format!("mov {LABEL},\"20\""));
    for (i, v) in DATE.iter().enumerate() {
        if i > 0 {
            w(format!("add {LABEL},\"{}\"", ["", "-", "-", " ", ":", ":"][i]));
        }
        w(format!("if {v} < 10 add {LABEL},\"0\""));
        w(format!("itoa {STR_TMP},{v}"));
        w(format!("add {LABEL},{STR_TMP}"));
    }
    w(format!("getplaytime {PLAY}"));
    w(format!("add {LABEL},\"|\""));
    for (part, digits) in [(format!("{PLAY}/3600"), 3), (format!("{PLAY}/60 mod 60"), 2), (format!("{PLAY} mod 60"), 2)] {
        if digits == 2 {
            w(format!("add {LABEL},\":\""));
        }
        w(format!("mov {TMP},{part}"));
        if digits == 3 {
            w(format!("if {TMP} < 100 add {LABEL},\"0\""));
        }
        w(format!("if {TMP} < 10 add {LABEL},\"0\""));
        w(format!("itoa {STR_TMP},{TMP}"));
        w(format!("add {LABEL},{STR_TMP}"));
    }
    w(format!("add {LABEL},\"|\":add {LABEL},{}", menu::CHAPTER));
    // With its line breaks, as the original shows it.
    w(format!("getlog {STR_TMP},0"));
    w(format!("add {LABEL},\"|\":add {LABEL},{STR_TMP}"));
    w(format!("savegame2 {SLOT},{LABEL}"));
    if scene_stack {
        w(format!("mov {SELECTED},{SLOT}:gosub *conv_scene_keep"));
    }
    w(format!("mov {NEWEST},{SLOT}"));
    w("gosub *conv_sg_draw".into());
    if let Some(n) = &g.saved {
        // The notice over the dimmed page, until a click.
        let (back, notice) = (DIALOG_SPRITES.start + 1, DIALOG_SPRITES.start);
        w("exbtn_d \"\":btndef clear".into());
        w(format!("lsp {back},\":c;>{width},{height},#000000\",0,0,204"));
        w(format!("lsp {notice},\":a;{}\",{},{}", path(&n.image), n.x, n.y));
        w("print 1".into());
        w(format!("btnwait {BUTTON_VAR}"));
        w(format!("csp {back}:csp {notice}"));
    }
    w("print 1:goto *conv_sg_wait".into());

    w("*conv_sg_load".into());
    w(format!("if {HAS_DATA} == 0 goto *conv_sg_wait"));
    w(ask(Switch::ConfirmLoad));
    w("exbtn_d \"\":btndef clear".into());
    if scene_stack {
        w(format!("mov {SCENE_LOAD_SLOT},{SLOT}"));
    }
    w(format!("loadgame {SLOT}"));
    // Still here: the load failed.
    w("goto *conv_sg_wait".into());

    w("*conv_sg_delete".into());
    w(format!("if {HAS_DATA} == 0 goto *conv_sg_wait"));
    w(ask(Switch::ConfirmDelete));
    w(format!("delsavegame {SLOT},{FILE}"));
    if scene_stack {
        w(format!("mov {SELECTED},{SLOT}:gosub *conv_scene_drop"));
    }
    w(format!("if {SLOT} == {NEWEST} mov {NEWEST},0"));
    w("gosub *conv_sg_draw:print 1:goto *conv_sg_wait".into());

    w("*conv_sg_close".into());
    w("exbtn_d \"\":btndef clear".into());
    w([PREVIOUS, NEXT, DELETE_ON, DELETE, CLOSE].map(|n| format!("csp {n}")).join(":"));
    w("popscreen".into());
    // The scene first, then the window: texton drops the pending screen
    // updates but the window's.
    w(fade.clone());
    w(format!("if {IN_GAME} == 1 texton"));
    w("return".into());

    // The page: its picture, thumbnails and each slot's hidden details.
    w("*conv_sg_draw".into());
    w(format!("for {TMP} = {} to {MAX_SPRITE}:csp {TMP}:next", DETAILS));
    for (p, (save, load)) in g.save_pages.iter().zip(&g.load_pages).enumerate() {
        w(format!("if {PAGE} == {} && {MODE} == 1 lsp {BACKGROUND},\":c;{}\",0,0", p + 1, path(save)));
        w(format!("if {PAGE} == {} && {MODE} == 2 lsp {BACKGROUND},\":c;{}\",0,0", p + 1, path(load)));
    }
    if let Some(f) = &g.frames {
        w(format!("lsp {FRAMES},\":a;{}\",{},{}", path(&f.image), f.x, f.y));
    }
    w(format!("lsp {HOVER},\":a;{}\",0,0:vsp {HOVER},0", path(&g.hover.image)));
    w(format!("mov {FIRST},{PAGE}*{slots}-{}", slots - 1));
    let (pw, ph) = (g.preview.w, g.preview.h);
    let zoom = pw * 100 / g.thumb_w as i32;
    // The preview sprite is zoomed about its centre.
    let (px, py) = (g.preview.x + (pw - g.thumb_w as i32) / 2, g.preview.y + (ph - g.thumb_h as i32) / 2);
    for i in 0..slots {
        let (x, y) = place(i);
        let d = DETAILS + 5 * i;
        w(format!("mov {SLOT},{FIRST}+{i}:gosub *conv_sg_data"));
        w(format!("lsp {},\":c;>{},{},#000000\",{x},{y},0", HITS + i, g.thumb_w, g.thumb_h));
        w(format!("if {HAS_DATA} == 1 lsp {},{THUMB_IMAGE},{x},{y}", THUMBS + i));
        w(format!("if {HAS_DATA} == 1 lsp {},{THUMB_IMAGE},{px},{py}:spzoom {},{zoom}:vsp {},0", d + 4, d + 4, d + 4));
        for (k, (t, text)) in [&g.date, &g.play_time, &g.chapter, &g.line].iter().zip(TEXTS).enumerate() {
            w(format!("{}:vsp {},0", grid_text(d + k as u32, t, text), d + k as u32));
        }
        let (nx, ny) = (x + g.new_offset.0, y + g.new_offset.1);
        w(format!("if {HAS_DATA} == 1 && {SLOT} == {NEWEST} lsp {NEW},\":a;{}\",{nx},{ny}", path(&g.new_mark.image)));
    }
    w("gosub *conv_sg_delete_mode".into());
    w("return".into());

    // Slot SLOT: HAS_DATA, its details and thumbnail (FILE, THUMB_IMAGE).
    w("*conv_sg_data".into());
    w("gosub *conv_sl_file".into());
    w(format!("savefileexist {HAS_DATA},{SLOT}"));
    w(format!("if {HAS_DATA} == 0 mov {},\"\":mov {},\"\":mov {},\"\":mov {},\"　データ無し\":return", TEXTS[0], TEXTS[1], TEXTS[2], TEXTS[3]));
    w(format!("getsavestr {LABEL},{SLOT}"));
    w(format!("split {LABEL},\"|\",{}", TEXTS.join(",")));
    w("return".into());
    // The thumbnail's file name (as the save menu names it).
    w("*conv_sl_file".into());
    w(format!("itoa {STR_TMP},{SLOT}"));
    w(format!("mov {FILE},\"savethumb\""));
    w(format!("add {FILE},{STR_TMP}"));
    w(format!("add {FILE},\".bmp\""));
    w(format!("mov {THUMB_IMAGE},\":c;\""));
    w(format!("add {THUMB_IMAGE},{FILE}"));
    w("return".into());
}

/// A [`GridText`] as a `strsp` (or `vstrsp`) of `text`.
fn grid_text(slot: u32, t: &GridText, text: &str) -> String {
    let pitch = t.font_size + t.line_gap;
    if t.vertical {
        // x is the first (rightmost) column's left edge.
        let x = t.x - (t.lines.saturating_sub(1) * pitch) as i32;
        format!("vstrsp {slot},{text},{x},{},{},{},{},{},{},{},0,0,\"{}\"", t.y, t.lines, t.chars, t.font_size, t.font_size, t.line_gap, t.char_gap, t.colour)
    } else {
        format!("strsp {slot},{text},{},{},{},{},{},{},{},{},0,0,\"{}\"", t.x, t.y, t.chars, t.lines, t.font_size, t.font_size, t.char_gap, t.line_gap, t.colour)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::Rect;

    fn placed(name: &str, x: i32, y: i32) -> Placed {
        Placed { image: format!("sys/{name}.png"), x, y, w: 60, h: 80 }
    }

    fn text(x: i32, y: i32, lines: u32, vertical: bool) -> GridText {
        GridText { x, y, font_size: 18, lines, chars: 20, line_gap: 0, char_gap: -1, colour: "#000000".into(), vertical }
    }

    fn grid() -> SaveGrid {
        SaveGrid {
            save_pages: vec!["sys/save1.png".into(), "sys/save2.png".into()],
            load_pages: vec!["sys/load1.png".into(), "sys/load2.png".into()],
            frames: Some(placed("frames", 0, 0)),
            first: (600, 34),
            row_step: 100,
            column_step: -190,
            rows: 5,
            columns: 4,
            thumb_w: 160,
            thumb_h: 90,
            hover: placed("hover", 0, 0),
            hover_offset: (-3, -3),
            new_mark: placed("new", 0, 0),
            new_offset: (164, 64),
            previous: placed("previous", 790, 34),
            next: placed("next", 790, 130),
            delete: placed("delete", 790, 226),
            delete_on: placed("delete_on", 790, 226),
            close: placed("close", 810, 322),
            preview: Rect { x: 810, y: 416, w: 192, h: 108 },
            date: text(821, 516, 1, false),
            play_time: text(68, 528, 1, false),
            chapter: text(929, 41, 1, true),
            line: text(898, 45, 3, true),
            saved: Some(placed("saved", 462, 88)),
            fade_ms: 300,
        }
    }

    #[test]
    fn slots_down_the_columns_with_their_details_on_hover() {
        let mut out = String::new();
        emit(&mut out, &grid(), None, 1024, 576, false);
        // Slot 6 is the second column's first: 190 to the left. Its hover
        // shows the frame around it and its five details.
        assert!(out.contains("exbtn 925,6,\"C918C700-799P918M918,407,31P725P726P727P728P729\"\n"), "{out}");
        // The last line: three columns from the right, the first at 898.
        assert!(out.contains("vstrsp 703,$113,862,45,3,20,18,18,0,-1,0,0,\"#000000\":vsp 703,0\n"), "{out}");
        assert!(out.contains("strsp 701,$111,68,528,20,1,18,18,-1,0,0,0,\"#000000\":vsp 701,0\n"), "{out}");
        // The preview zoomed to 120 % about the box's centre.
        assert!(out.contains("lsp 704,$105,826,425:spzoom 704,120:vsp 704,0\n"), "{out}");
        // Play time from the engine, the line with its breaks.
        assert!(out.contains("getplaytime %147\n"), "{out}");
        assert!(out.contains("getlog $104,0\n"), "{out}");
        assert!(!out.contains("conv_scene_keep"), "{out}");
    }

    #[test]
    fn link_scene_levels_kept_with_a_save() {
        let mut out = String::new();
        emit(&mut out, &grid(), None, 1024, 576, true);
        assert!(out.contains("savegame2 %103,$100\nmov %102,%103:gosub *conv_scene_keep\n"), "{out}");
        assert!(out.contains("mov %2904,%103\nloadgame %103\n"), "{out}");
        assert!(out.contains("delsavegame %103,$103\nmov %102,%103:gosub *conv_scene_drop\n"), "{out}");
    }
}
