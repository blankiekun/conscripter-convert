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
    self, switch_index, CONFIRM, QUICK_SLOT, CONFIRM_KIND, CONFIRM_ROUTINE, CONFIRM_TEXT, DATE, FILE, FIRST, HAS_DATA, IN_GAME, LABEL, MODE, NEWEST, PAGE, SCENE_LOAD_SLOT, SELECTED, SLOT, STR_TMP,
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
// The popup ([`SaveGrid::popup`]): its buttons over its window.
const POPUP_BUTTONS: [u32; 4] = [M + 5, M + 6, M + 7, M + 8]; // save, load, delete, lock
const POPUP_WINDOW: u32 = M + 9;
const BTN_POPUP: [u32; 4] = [105, 106, 107, 108];
/// Tab t's sprite and button.
const TABS: u32 = M + 10;
const BTN_TAB: u32 = 110;
const MAX_TABS: usize = 10;
/// Locked marks: + slot.
const LOCKED_MARKS: u32 = 880;
/// The slot the popup is open on (1 up), 0 while it is shut.
const POPUP_AT: &str = "%164";
/// 1 while that slot is locked.
const LOCKED: &str = "%165";
/// The popup's top left (its slot's).
const POPUP_XY: [&str; 2] = ["%166", "%167"];
/// Slot n's lock is the global %(LOCK_BASE + n), n 1-100 (the user's
/// slots only).
const LOCK_BASE: u32 = 1010;

/// `confirm`: the script's yes/no dialog (none: act without asking).
pub(crate) fn emit(out: &mut String, g: &SaveGrid, confirm: Option<&ConfirmDialog>, width: u32, height: u32, scene_stack: bool) {
    let slots = g.rows * g.columns;
    assert!(slots <= MAX_SLOTS, "a save grid has more slots a page than its sprites");
    let pages = g.save_pages.len().min(g.load_pages.len()).max(1);
    assert!(g.tabs.len() <= MAX_TABS, "a save grid has more tabs than its sprites");
    assert!(g.popup.is_none() || pages as u32 * slots <= 100, "save grid locks past their range");
    // Popup buttons on, and off: save, load, delete, lock (`notif` gets
    // `||` wrong, so both are spelled out).
    let (quick_on, quick_off) = if g.quick_page { (format!(" && {PAGE} != {pages}"), format!(" || {PAGE} == {pages}")) } else { Default::default() };
    let on = [
        format!("{LOCKED} == 0 && {IN_GAME} == 1{quick_on}"),
        format!("{HAS_DATA} == 1"),
        format!("{HAS_DATA} == 1 && {LOCKED} == 0"),
        format!("{HAS_DATA} == 1{quick_on}"),
    ];
    let off = [
        format!("{LOCKED} == 1 || {IN_GAME} == 0{quick_off}"),
        format!("{HAS_DATA} == 0"),
        format!("{HAS_DATA} == 0 || {LOCKED} == 1"),
        format!("{HAS_DATA} == 0{quick_off}"),
    ];
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let place = |i: u32| {
        let (column, row) = if g.across { (i % g.columns, i / g.columns) } else { (i / g.rows, i % g.rows) };
        (g.first.0 + g.column_step * column as i32, g.first.1 + g.row_step * row as i32)
    };
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
    // With the popup the screen saves whichever way it was opened.
    let saving = if g.popup.is_some() { format!("{IN_GAME} == 1") } else { format!("{MODE} == 1") };
    w(format!("if {saving} getscreenshot {},{}", g.thumb_w, g.thumb_h));
    w(format!("pushscreen {},{MAX_SPRITE}", crate::ANIMATION_SPRITE));
    w(format!("mov {DELETING},0"));
    if g.popup.is_some() {
        w(format!("mov {POPUP_AT},0"));
    }
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
        // Not under the popup's own buttons.
        let unless = if g.popup.is_some() { format!("if {POPUP_AT} != {} ", i + 1) } else { String::new() };
        w(format!("{unless}exbtn {},{},\"{all_details}P{HOVER}M{HOVER},{hx},{hy}{shown}\"", HITS + i, i + 1));
    }
    for t in 0..g.tabs.len() as u32 {
        w(format!("if {PAGE} != {} spbtn {},{}", t + 1, TABS + t, BTN_TAB + t));
    }
    if g.popup.is_some() {
        w(format!("if {POPUP_AT} > 0 gosub *conv_sg_popup_state"));
        for ((on, slot), n) in on.iter().zip(POPUP_BUTTONS).zip(BTN_POPUP) {
            w(format!("if {POPUP_AT} > 0 && {on} spbtn {slot},{n}"));
        }
    }
    for (slot, n) in [(PREVIOUS, BTN_PREVIOUS), (NEXT, BTN_NEXT), (DELETE, BTN_DELETE), (CLOSE, BTN_CLOSE)] {
        w(format!("spbtn {slot},{n},3"));
    }
    w("getkey".into());
    w(format!("btnwait {BUTTON_VAR}"));
    // A right-click, or the screen's key (S saving, L loading), leaves.
    if g.popup.is_some() {
        w(format!("if {BUTTON_VAR} == -1 && {POPUP_AT} > 0 gosub *conv_sg_popup_close:print 1:goto *conv_sg_wait"));
        w(format!("if {BUTTON_VAR} >= {} && {BUTTON_VAR} <= {} goto *conv_sg_popup_click", BTN_POPUP[0], BTN_POPUP[3]));
    }
    if !g.tabs.is_empty() {
        w(format!(
            "if {BUTTON_VAR} >= {BTN_TAB} && {BUTTON_VAR} < {} mov {PAGE},{BUTTON_VAR}-{}:gosub *conv_sg_draw:{fade}:goto *conv_sg_wait",
            BTN_TAB + g.tabs.len() as u32,
            BTN_TAB - 1
        ));
    }
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
    if let Some(p) = &g.popup {
        if p.empty_saves {
            w(format!("if {HAS_DATA} == 0 && {IN_GAME} == 0 goto *conv_sg_wait"));
            w(format!("if {HAS_DATA} == 0 && {SLOT} < {QUICK_SLOT} gosub *conv_sg_popup_close:goto *conv_sg_save"));
        }
        // Open (or move) the popup onto the slot.
        w(format!("mov {POPUP_AT},{BUTTON_VAR}"));
        w("gosub *conv_sg_popup_state".into());
        let i = format!("({POPUP_AT}-1)");
        let (column, row) = if g.across { (format!("{i} mod {}", g.columns), format!("{i}/{}", g.columns)) } else { (format!("{i}/{}", g.rows), format!("{i} mod {}", g.rows)) };
        w(format!("mov {},{}+{}*({column})", POPUP_XY[0], g.first.0, g.column_step));
        w(format!("mov {},{}+{}*({row})", POPUP_XY[1], g.first.1, g.row_step));
        let at = |q: &Placed| format!("{}+{},{}+{}", POPUP_XY[0], q.x, POPUP_XY[1], q.y);
        w(format!("lsp {POPUP_WINDOW},\":a;{}\",{}", path(&p.window.image), at(&p.window)));
        for ((b, slot), off) in [&p.save, &p.load, &p.delete].into_iter().zip(POPUP_BUTTONS).zip(&off) {
            w(format!("lsp {slot},\":a/4,0,3;{}\",{}:if {off} cell {slot},2", path(&b.image), at(b)));
        }
        let lock = POPUP_BUTTONS[3];
        w(format!("if {LOCKED} == 0 lsp {lock},\":a/4,0,3;{}\",{}", path(&p.lock.image), at(&p.lock)));
        w(format!("if {LOCKED} == 1 lsp {lock},\":a/4,0,3;{}\",{}", path(&p.unlock.image), at(&p.unlock)));
        w(format!("if {} cell {lock},2", off[3]));
        w("print 10,150:goto *conv_sg_wait".into());

        // SLOT, HAS_DATA and LOCKED of the popup's slot.
        w("*conv_sg_popup_state".into());
        w(format!("mov {SLOT},{FIRST}+{POPUP_AT}-1:gosub *conv_sg_data"));
        w(format!("mov {LOCKED},0:if {SLOT} < {QUICK_SLOT} mov {TMP},{LOCK_BASE}+{SLOT}:mov {LOCKED},%%{}", &TMP[1..]));
        w("return".into());
        w("*conv_sg_popup_close".into());
        w(format!("mov {POPUP_AT},0:csp {POPUP_WINDOW}:{}", POPUP_BUTTONS.map(|n| format!("csp {n}")).join(":")));
        w("return".into());

        w("*conv_sg_popup_click".into());
        w("gosub *conv_sg_popup_state:gosub *conv_sg_popup_close:print 1".into());
        w(format!("if {BUTTON_VAR} == {} goto *conv_sg_load", BTN_POPUP[1]));
        w(format!("if {BUTTON_VAR} == {} goto *conv_sg_delete", BTN_POPUP[2]));
        w(format!("if {BUTTON_VAR} == {} mov {TMP},{LOCK_BASE}+{SLOT}:mov %%{t},1-%%{t}:gosub *conv_sg_draw:print 10,150:goto *conv_sg_wait", BTN_POPUP[3], t = &TMP[1..]));
        w(format!("if {HAS_DATA} == 0 goto *conv_sg_save"));
        w(ask(Switch::ConfirmSave));
        w("goto *conv_sg_save".into());
    }
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
    let parts = if g.play_time_hm {
        vec![(format!("{PLAY}/3600"), 2), (format!("{PLAY}/60 mod 60"), 2)]
    } else {
        vec![(format!("{PLAY}/3600"), 3), (format!("{PLAY}/60 mod 60"), 2), (format!("{PLAY} mod 60"), 2)]
    };
    for (n, (part, digits)) in parts.into_iter().enumerate() {
        if n > 0 {
            w(format!("add {LABEL},\":\""));
        }
        w(format!("mov {TMP},{part}"));
        // Hours stop at 99 (the English Muramasa port's `GetDataInfoPlayTime`).
        if g.play_time_hm && n == 0 {
            w(format!("if {TMP} > 99 mov {TMP},99"));
        }
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
    let popup_sprites: &[u32] = if g.popup.is_some() { &[POPUP_WINDOW, POPUP_BUTTONS[0], POPUP_BUTTONS[1], POPUP_BUTTONS[2], POPUP_BUTTONS[3]] } else { &[] };
    let shown = [PREVIOUS, NEXT, DELETE_ON, DELETE, CLOSE].into_iter().chain(popup_sprites.iter().copied()).chain(TABS..TABS + g.tabs.len() as u32);
    w(shown.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    if g.popup.is_some() {
        w(format!("mov {POPUP_AT},0"));
    }
    w("popscreen".into());
    // The scene first, then the window: texton drops the pending screen
    // updates but the window's.
    w(fade.clone());
    w(format!("if {IN_GAME} == 1 texton"));
    w("return".into());

    // The page: its picture, thumbnails and each slot's hidden details.
    w("*conv_sg_draw".into());
    // The whole game range: the scene's sprites (saved by pushscreen)
    // would be drawn over the page.
    w(format!("for {TMP} = {} to {MAX_SPRITE}:csp {TMP}:next", crate::ANIMATION_SPRITE));
    for (p, (save, load)) in g.save_pages.iter().zip(&g.load_pages).enumerate() {
        w(format!("if {PAGE} == {} && {MODE} == 1 lsp {BACKGROUND},\":c;{}\",0,0", p + 1, path(save)));
        w(format!("if {PAGE} == {} && {MODE} == 2 lsp {BACKGROUND},\":c;{}\",0,0", p + 1, path(load)));
    }
    if let Some(f) = &g.frames {
        w(format!("lsp {FRAMES},\":a;{}\",{},{}", path(&f.image), f.x, f.y));
    }
    w(format!("lsp {HOVER},\":a;{}\",0,0:vsp {HOVER},0", path(&g.hover.image)));
    w(format!("mov {FIRST},{PAGE}*{slots}-{}", slots - 1));
    if g.quick_page {
        w(format!("if {PAGE} == {pages} mov {FIRST},{QUICK_SLOT}"));
    }
    if g.popup.is_some() {
        w(format!("mov {POPUP_AT},0:csp {POPUP_WINDOW}:{}", POPUP_BUTTONS.map(|n| format!("csp {n}")).join(":")));
    }
    for (t, tab) in g.tabs.iter().enumerate() {
        let n = TABS + t as u32;
        w(format!("lsp {n},\":a/4,0,3;{}\",{},{}:if {PAGE} == {} cell {n},2", path(&tab.image), tab.x, tab.y, t + 1));
    }
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
        if let Some(e) = &g.empty {
            w(format!("if {HAS_DATA} == 0 lsp {},\":a;{}\",{},{}", THUMBS + i, path(&e.image), x + e.x, y + e.y));
        }
        if let Some(p) = &g.popup {
            let l = &p.locked;
            w(format!(
                "if {HAS_DATA} == 1 && {SLOT} < {QUICK_SLOT} mov {TMP},{LOCK_BASE}+{SLOT}:if %%{} == 1 lsp {},\":a;{}\",{},{}",
                &TMP[1..],
                LOCKED_MARKS + i,
                path(&l.image),
                x + l.x,
                y + l.y
            ));
        }
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
    if g.quick_page {
        // The quick page's other slots stay empty.
        w(format!("if {SLOT} > {QUICK_SLOT} && {SLOT} < {} mov {HAS_DATA},0", QUICK_SLOT + slots));
    }
    w(format!("if {HAS_DATA} == 0 mov {},\"\":mov {},\"\":mov {},\"\":mov {},\"　データ無し\":return", TEXTS[0], TEXTS[1], TEXTS[2], TEXTS[3]));
    w(format!("getsavestr {LABEL},{SLOT}"));
    w(format!("split {LABEL},\"|\",{}", TEXTS.join(",")));
    if !g.play_time_prefix.is_empty() {
        w(format!("mov {STR_TMP},\"{}\":add {STR_TMP},{t}:mov {t},{STR_TMP}", escape(&g.play_time_prefix), t = TEXTS[1]));
    }
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
    use conv_core::ir::{Rect, SlotPopup};

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
            across: false,
            tabs: Vec::new(),
            popup: None,
            empty: None,
            quick_page: false,
            play_time_prefix: String::new(),
            play_time_hm: false,
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
        assert!(out.contains("mov %2906,%103\nloadgame %103\n"), "{out}");
        assert!(out.contains("delsavegame %103,$103\nmov %102,%103:gosub *conv_scene_drop\n"), "{out}");
    }

    #[test]
    fn popup_tabs_locks_and_quick_page() {
        let mut g = grid();
        g.tabs = vec![placed("tab1", 157, 0), placed("tab2", 299, 0)];
        g.quick_page = true;
        g.empty = Some(placed("empty", 0, 0));
        g.play_time_prefix = "PLAY TIME ".into();
        g.popup = Some(SlotPopup {
            window: placed("window", 0, 0),
            save: placed("save", 0, 1),
            load: placed("load", 82, 0),
            delete: placed("delete", 82, 47),
            lock: placed("lock", 0, 47),
            unlock: placed("unlock", 0, 47),
            locked: placed("locked", -2, -2),
            empty_saves: true,
        });
        g.play_time_hm = true;
        let mut out = String::new();
        emit(&mut out, &g, None, 1024, 576, false);
        // The current page's tab is no button; the other one picks its page.
        assert!(out.contains("if %900 != 2 spbtn 17,111\n"), "{out}");
        assert!(out.contains("lsp 17,\":a/4,0,3;sys\\tab2.png\",299,0:if %900 == 2 cell 17,2\n"), "{out}");
        // The second page is the quick page: the quick save first.
        assert!(out.contains("if %900 == 2 mov %106,890\n"), "{out}");
        // Save is off on a locked slot, outside the game and on the quick page.
        assert!(out.contains("if %164 > 0 && %165 == 0 && %101 == 1 && %900 != 2 spbtn 11,105\n"), "{out}");
        // The lock is a global per slot, toggled by its button.
        assert!(out.contains("mov %165,0:if %103 < 890 mov %105,1010+%103:mov %165,%%105\n"), "{out}");
        assert!(out.contains("if %9 == 108 mov %105,1010+%103:mov %%105,1-%%105:"), "{out}");
        assert!(out.contains("mov $104,\"PLAY TIME \":add $104,$111:mov $111,$104\n"), "{out}");
        // An empty slot saves at once in the game, does nothing at the title.
        assert!(out.contains("if %104 == 0 && %101 == 0 goto *conv_sg_wait\nif %104 == 0 && %103 < 890 gosub *conv_sg_popup_close:goto *conv_sg_save\n"), "{out}");
        // Hours and minutes, the hours up to 99.
        assert!(out.contains("mov %105,%147/3600\nif %105 > 99 mov %105,99\n"), "{out}");
        assert!(!out.contains("%147 mod 60"), "{out}");
    }
}
