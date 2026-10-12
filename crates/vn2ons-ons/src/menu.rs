//! The in-game menus as NScripter subroutines: the save/load screen (see
//! [`SaveMenu`]) and the button bar shown at click waits ([`ButtonBar`]).
//!
//! Enter with `mov %101,1:gosub *vn2ons_save` (or `vn2ons_load`) from a
//! click wait in the game, or with `%101` = 0 from the title screen. It
//! returns when closed; a successful load doesn't return.
//!
//! How it maps onto ONScripter:
//! - ONScripter keeps a snapshot of the game state from the start of the
//!   current text page (`storeSaveFile`); `savegame` writes that snapshot,
//!   so saving from inside this subroutine saves the page, not the menu.
//! - `savegame2` stores the slot's label inside the save file; `getsavestr`
//!   reads it back. The label is "No.NN - date time|chapter".
//! - Thumbnails: `getscreenshot` when the menu opens (before anything is
//!   drawn), `savescreenshot2` to `savethumbN.bmp` next to the save files,
//!   shown with `lsp2` (scaled down in the rows, full size as the preview).
//! - ONScripter can't delete a save, so a global flag per slot hides it.
//! - Layers: `lsp2` sprites are drawn under the text window and under
//!   sprites 0-49 (`windowback`, `humanz 49`), so the base, frame,
//!   thumbnails and the (additive) selection bar are `lsp2` sprites, and
//!   buttons and labels `lsp` sprites over them. The text window, name
//!   plate and link words are hidden while the menu is open.

use vn2ons_core::ir::{BarAction, Backlog, ButtonBar, Placed, SaveMenu, Setting, SettingsMenu, TextSpot};

use crate::{path, BAR_SPRITES, BUTTON_VAR, LINK_SPRITES, MENU_SPRITES, NAME_PLATE_SPRITE, NAME_TEXT_SPRITE};

pub(crate) const SAVE: &str = "vn2ons_save";
/// The `textgosub` routine.
pub(crate) const WAIT: &str = "vn2ons_wait";
/// Shows the button bar and makes its buttons clickable.
pub(crate) const BAR_DRAW: &str = "vn2ons_bar_draw";
/// Runs the bar button in %9 (or -1, a right-click).
pub(crate) const BAR_ACTION: &str = "vn2ons_bar_action";
pub(crate) const SETTINGS: &str = "vn2ons_settings";
const SETTINGS_APPLY: &str = "vn2ons_settings_apply";
pub(crate) const BACKLOG: &str = "vn2ons_backlog";
/// Settings levels are %902-%906 (globals); %909 = 1 once they are set.
const SETTINGS_SET: &str = "%909";
/// Milliseconds per character, for `setwindow`.
pub(crate) const TEXT_MS: &str = "%907";
const TEXT_MS_STEP: u32 = 6;
// Settings and backlog sprites (lsp, within MENU_SPRITES).
const SETTING_MINUS: u32 = 10; // + row
const SETTING_PLUS: u32 = 15; // + row
const SETTING_LEVEL: u32 = 20; // + row
const BTN_MINUS: u32 = 10;
const BTN_PLUS: u32 = 20;
const LOG_PAGES: u32 = 10; // text sprites 0..LOG_PAGES
const LOG_UP: u32 = 25;
const LOG_DOWN: u32 = 26;
const LOG_OFFSET: &str = "%170";
const LOG_Y: &str = "%171";
const LOG_K: &str = "%172";
const LOG_LINES: &str = "%173";
const LOG_OLDER: &str = "%174";
/// Bar buttons return numbers above this.
pub(crate) const BAR_BUTTON_BASE: i32 = 1000;
/// Sets which bars are up after a hotspot reported the pointer (%9).
pub(crate) const BAR_HOVER: &str = "vn2ons_bar_hover";
const BAR_CLEAR: &str = "vn2ons_bar_clear";
/// Puts the bars away (they come back when the pointer nears an edge).
pub(crate) const BAR_HIDE: &str = "vn2ons_bar_hide_all";
/// Hotspots report the pointer with numbers above this.
pub(crate) const HOVER_BASE: i32 = 2000;
// Which bars are up (locals, so a save keeps them as they were).
const TOP_SHOWN: &str = "%160";
const BOTTOM_SHOWN: &str = "%161";
/// Link scenes entered and not yet left; the deepest since the last time
/// at the root; titles at $(SCENE_TITLES + depth).
pub(crate) const SCENE_DEPTH: &str = "%150";
pub(crate) const SCENE_MAX_DEPTH: &str = "%151";
pub(crate) const SCENE_TITLES: u32 = 160;
const SCENE_TMP: &str = "$151";
/// lsp2 slots of the top bars (below the save menu's).
const TOP_SPRITES2: std::ops::Range<u32> = 12..24;
pub(crate) const LOAD: &str = "vn2ons_load";
/// 1 when entered from the game (hide and restore the text window).
pub(crate) const IN_GAME: &str = "%101";

// Local scratch variables (below the game's locals at %10-%12 would be too
// close; these are free).
const MODE: &str = "%100"; // 1 save, 2 load
const SELECTED: &str = "%102"; // selected slot, 0 = none
const SLOT: &str = "%103";
const HAS_DATA: &str = "%104";
pub(crate) const TMP: &str = "%105";
const FIRST: &str = "%106"; // first slot on the page
const SELECTED_HAS_DATA: &str = "%107";
const MODE_STATE: &str = "%108"; // isskip: 1 skip, 2 auto
const DATE: [&str; 6] = ["%110", "%111", "%112", "%113", "%114", "%115"];
const LABEL: &str = "$100";
const LINE1: &str = "$101";
const LINE2: &str = "$102";
const FILE: &str = "$103"; // thumbnail file
const THUMB_IMAGE: &str = "$105"; // the same with its sprite tag
const STR_TMP: &str = "$104";
// Globals (`globalon` keeps %200 and up; the game uses up to %220).
const PAGE: &str = "%900";
const NEWEST: &str = "%901";
/// Slot n is deleted when %(DELETED_BASE + n) is 1.
const DELETED_BASE: u32 = 910;
/// The current chapter's title (a local: saves keep it), shown in the top
/// bar and under a save's date.
pub(crate) const CHAPTER: &str = "$150";

// lsp slots (all within MENU_SPRITES).
const TEXT1: u32 = 0; // + row
const TEXT2: u32 = 5; // + row
const ROW_BUTTON: u32 = 10; // + row
const PAGE_BUTTON: u32 = 15; // + page index
const DELETE: u32 = 30;
const ACTION: u32 = 31;
const CLOSE: u32 = 32;
const MAX_ROWS: usize = 5;
const MAX_PAGES: usize = 15;
// lsp2 slots (higher numbers are drawn further down).
const NEW: u32 = 0;
const THUMB: u32 = 1; // + row
const PREVIEW: u32 = 6;
const MARKER: u32 = 7;
const SELECTION: u32 = 8;
const HEADER: u32 = 9;
const FRAME: u32 = 10;
const BASE: u32 = 11;
// Button numbers returned by btnwait: rows 1.., pages 101..
const BTN_PAGE: u32 = 100;
const BTN_DELETE: u32 = 200;
const BTN_ACTION: u32 = 201;
const BTN_CLOSE: u32 = 202;

pub(crate) fn save_menu(out: &mut String, m: &SaveMenu) {
    let rows = (m.rows as usize).min(MAX_ROWS);
    let pages = &m.pages[..m.pages.len().min(MAX_PAGES)];
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let button = |slot: u32, p: &Placed| format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&p.image), p.x, p.y);
    let row_y = |row: usize| m.row_y + row as i32 * m.row_pitch;
    let thumb_centre = |row: usize| (m.thumbnail.x + m.thumbnail.w / 2, row_y(row) + m.thumbnail.y + m.thumbnail.h / 2);
    let scale = m.thumbnail.w * 100 / m.preview.w;

    w(format!("*{SAVE}"));
    w(format!("mov {MODE},1"));
    w("goto *vn2ons_sl".into());
    w(format!("*{LOAD}"));
    w(format!("mov {MODE},2"));
    w("*vn2ons_sl".into());
    // The thumbnail: the screen as it is before the menu covers it.
    w(format!("if {MODE} == 1 getscreenshot {},{}", m.preview.w, m.preview.h));
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(lsp2(BASE, &m.base, Some(m.base_opacity)));
    w(lsp2(FRAME, &m.frame, None));
    w(format!("if {MODE} == 1 {}", lsp2(HEADER, &m.save_header, None)));
    w(format!("if {MODE} == 2 {}", lsp2(HEADER, &m.load_header, None)));
    w(button(DELETE, &m.delete));
    w(format!("if {MODE} == 1 {}", button(ACTION, &m.save)));
    w(format!("if {MODE} == 2 {}", button(ACTION, &m.load)));
    w(button(CLOSE, &m.close));
    w(format!("if {PAGE} < 1 mov {PAGE},1"));
    w(format!("if {PAGE} > {} mov {PAGE},1", pages.len()));

    // A page: its rows and the page buttons.
    w("*vn2ons_sl_page".into());
    w(format!("mov {SELECTED},0"));
    w(format!("csp2 {SELECTION}:csp2 {PREVIEW}:csp2 {NEW}:csp2 {MARKER}"));
    w(format!("mov {FIRST},{PAGE}*{rows}-{}", rows - 1));
    for row in 0..rows {
        let y = row_y(row);
        w(format!("mov {SLOT},{FIRST}+{row}"));
        w("gosub *vn2ons_sl_slot".into());
        let size = m.font_size;
        for (slot, line, dy) in [(TEXT1, LINE1, 0), (TEXT2, LINE2, m.line_height)] {
            w(format!(
                "strsp {},{line},{},{},40,1,{size},{size},0,0,0,1",
                slot + row as u32,
                m.text_x,
                y + m.text_y + dy
            ));
        }
        w(format!("lsp {},\":a/2,0,3;{}\",{},{}", ROW_BUTTON + row as u32, path(&m.row_hover.image), m.row_hover.x, y + m.row_hover.y));
        let (cx, cy) = thumb_centre(row);
        w(format!("if {HAS_DATA} == 0 csp2 {}", THUMB + row as u32));
        w(format!("if {HAS_DATA} == 1 lsp2 {},{THUMB_IMAGE},{cx},{cy},{scale},{scale},0", THUMB + row as u32));
        let icon = &m.new_icon;
        w(format!(
            "if {HAS_DATA} == 1 && {SLOT} == {NEWEST} {}",
            lsp2(NEW, &Placed { y: y + icon.y, ..icon.clone() }, None)
        ));
    }
    for (i, p) in pages.iter().enumerate() {
        let slot = PAGE_BUTTON + i as u32;
        w(button(slot, &p.button));
        let marker = Placed { x: p.marker_x, y: p.marker_y, ..m.page_marker.clone() };
        w(format!("if {PAGE} == {} cell {slot},2:{}", i + 1, lsp2(MARKER, &marker, None)));
    }

    // Enable the buttons that apply to the selection.
    w("*vn2ons_sl_buttons".into());
    w(format!("cell {DELETE},2:cell {ACTION},2"));
    w(format!("if {SELECTED} > 0 && {SELECTED_HAS_DATA} == 1 cell {DELETE},0:cell {ACTION},0"));
    w(format!("if {SELECTED} > 0 && {MODE} == 1 cell {ACTION},0"));
    w("*vn2ons_sl_wait".into());
    for row in 0..rows {
        w(format!("spbtn {},{}", ROW_BUTTON + row as u32, row + 1));
    }
    for i in 0..pages.len() {
        w(format!("if {PAGE} != {} spbtn {},{}", i + 1, PAGE_BUTTON + i as u32, BTN_PAGE + i as u32 + 1));
    }
    w(format!("if {SELECTED} > 0 && {SELECTED_HAS_DATA} == 1 spbtn {DELETE},{BTN_DELETE}:spbtn {ACTION},{BTN_ACTION}"));
    w(format!("if {SELECTED} > 0 && {MODE} == 1 spbtn {ACTION},{BTN_ACTION}"));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    // Right-click closes, as in the original.
    w(format!("if {BUTTON_VAR} == -1 goto *vn2ons_sl_close"));
    w(format!("if {BUTTON_VAR} == {BTN_CLOSE} goto *vn2ons_sl_close"));
    w(format!("if {BUTTON_VAR} == {BTN_DELETE} goto *vn2ons_sl_delete"));
    w(format!("if {BUTTON_VAR} == {BTN_ACTION} goto *vn2ons_sl_act"));
    w(format!(
        "if {BUTTON_VAR} > {BTN_PAGE} && {BUTTON_VAR} <= {} mov {PAGE},{BUTTON_VAR}-{BTN_PAGE}:goto *vn2ons_sl_page",
        BTN_PAGE + pages.len() as u32
    ));
    w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {rows} goto *vn2ons_sl_row"));
    w("goto *vn2ons_sl_wait".into());

    // A row was clicked: select it, or act on it if it already was.
    w("*vn2ons_sl_row".into());
    w(format!("mov {SLOT},{FIRST}+{BUTTON_VAR}-1"));
    w(format!("if {SLOT} == {SELECTED} goto *vn2ons_sl_act"));
    w(format!("mov {SELECTED},{SLOT}"));
    w("gosub *vn2ons_sl_slot".into());
    w(format!("mov {SELECTED_HAS_DATA},{HAS_DATA}"));
    let sel = &m.row_selected;
    w(format!("mov {TMP},({BUTTON_VAR}-1)*{}+{}", m.row_pitch, m.row_y + sel.y + sel.h / 2));
    w(format!("lsp2add {SELECTION},\":c;{}\",{},{TMP},100,100,0", path(&sel.image), sel.x + sel.w / 2));
    let (px, py) = (m.preview.x + m.preview.w / 2, m.preview.y + m.preview.h / 2);
    w(format!("if {HAS_DATA} == 1 lsp2 {PREVIEW},{THUMB_IMAGE},{px},{py},100,100,0"));
    w(format!("if {HAS_DATA} == 0 csp2 {PREVIEW}"));
    w("goto *vn2ons_sl_buttons".into());

    w("*vn2ons_sl_act".into());
    w(format!("if {SELECTED} == 0 goto *vn2ons_sl_wait"));
    w(format!("if {MODE} == 2 goto *vn2ons_sl_load"));
    w(format!("mov {SLOT},{SELECTED}"));
    w("gosub *vn2ons_sl_file".into());
    w(format!("savescreenshot2 {FILE}"));
    // Label: "No.NN - 20YY/MM/DD hh:mm:ss|chapter". ONScripter (Shift-JIS)
    // draws half-width characters two per cell and drops a space that
    // starts a cell, so spaces at even positions are full-width.
    w("gosub *vn2ons_sl_number".into());
    w(format!("date {},{},{}", DATE[0], DATE[1], DATE[2]));
    w(format!("time {},{},{}", DATE[3], DATE[4], DATE[5]));
    w(format!("add {LINE1},\" - 20\""));
    for (i, v) in DATE.iter().enumerate() {
        if i > 0 {
            w(format!("add {LINE1},\"{}\"", ["", "/", "/", "　", ":", ":"][i]));
        }
        w(format!("if {v} < 10 add {LINE1},\"0\""));
        w(format!("itoa {STR_TMP},{v}"));
        w(format!("add {LINE1},{STR_TMP}"));
    }
    w(format!("mov {LABEL},{LINE1}"));
    w(format!("add {LABEL},\"|\""));
    w(format!("add {LABEL},{CHAPTER}"));
    w(format!("savegame2 {SELECTED},{LABEL}"));
    w(format!("mov {TMP},{SELECTED}+{DELETED_BASE}"));
    w(format!("mov %{TMP},0"));
    w(format!("mov {NEWEST},{SELECTED}"));
    w("goto *vn2ons_sl_page".into());

    w("*vn2ons_sl_load".into());
    w(format!("if {SELECTED_HAS_DATA} == 0 goto *vn2ons_sl_wait"));
    w("gosub *vn2ons_sl_clear".into());
    w(format!("loadgame {SELECTED}"));
    // Still here: the load failed.
    w("goto *vn2ons_sl".into());

    w("*vn2ons_sl_delete".into());
    w(format!("mov {TMP},{SELECTED}+{DELETED_BASE}"));
    w(format!("mov %{TMP},1"));
    w(format!("if {SELECTED} == {NEWEST} mov {NEWEST},0"));
    w("goto *vn2ons_sl_page".into());

    w("*vn2ons_sl_close".into());
    // texton first: it drops the pending screen updates except the window's.
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    w("gosub *vn2ons_sl_clear".into());
    w("print 1".into());
    w("return".into());

    w("*vn2ons_sl_clear".into());
    w((0..=BASE).map(|n| format!("csp2 {n}")).collect::<Vec<_>>().join(":"));
    w(MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("return".into());

    // Slot SLOT: HAS_DATA, LINE1 and LINE2 (its label), FILE (thumbnail).
    w("*vn2ons_sl_slot".into());
    w(format!("savefileexist {HAS_DATA},{SLOT}"));
    w(format!("mov {TMP},{SLOT}+{DELETED_BASE}"));
    w(format!("if %{TMP} == 1 mov {HAS_DATA},0"));
    w("gosub *vn2ons_sl_file".into());
    w(format!("if {HAS_DATA} == 0 goto *vn2ons_sl_empty"));
    w(format!("getsavestr {LABEL},{SLOT}"));
    w(format!("split {LABEL},\"|\",{LINE1},{LINE2}"));
    w(format!("if {LINE2} == \"\" mov {LINE2},\"　\""));
    w("return".into());
    w("*vn2ons_sl_empty".into());
    w("gosub *vn2ons_sl_number".into());
    w(format!("add {LINE1},\" - no　data\""));
    w(format!("mov {LINE2},\"　\""));
    w("return".into());

    w("*vn2ons_sl_number".into());
    w(format!("mov {LINE1},\"No.\""));
    w(format!("if {SLOT} < 10 add {LINE1},\"0\""));
    w(format!("itoa {STR_TMP},{SLOT}"));
    w(format!("add {LINE1},{STR_TMP}"));
    w("return".into());

    w("*vn2ons_sl_file".into());
    w(format!("itoa {STR_TMP},{SLOT}"));
    w(format!("mov {FILE},\"savethumb\""));
    w(format!("add {FILE},{STR_TMP}"));
    w(format!("add {FILE},\".bmp\""));
    w(format!("mov {THUMB_IMAGE},\":c;\""));
    w(format!("add {THUMB_IMAGE},{FILE}"));
    w("return".into());
}

/// The click-wait routine (`textgosub`): shows the bars the pointer has
/// revealed, waits, runs the clicked button's action, and ends the wait
/// with `texec`. Link pages call [`BAR_DRAW`], [`BAR_HOVER`] and
/// [`BAR_ACTION`] around their own `btnwait`.
///
/// Bar buttons return [`BAR_BUTTON_BASE`] + 1 + index; a right-click (-1)
/// hides the text window. `textbtnwait` returns at once in skip mode and
/// after the delay in auto mode; a click turns auto mode off.
///
/// The bars hide until the pointer comes near, as in the original:
/// transparent hotspots along the edges and over the middle report the
/// pointer through `getmouseover` (button numbers above [`HOVER_BASE`]),
/// which ends the wait; the routine records which bars are up
/// ([`TOP_SHOWN`], [`BOTTOM_SHOWN`]) and waits again.
pub(crate) fn button_bar(out: &mut String, bar: &ButtonBar, save_menu: bool, settings: bool, backlog: bool) {
    let mut slots = BAR_SPRITES;
    let buttons: Vec<_> = bar.buttons.iter().zip(slots.by_ref()).collect();
    let (reveal, hide, reveal_top, chapter_text, scene_text) =
        (slots.next().unwrap(), slots.next().unwrap(), slots.next().unwrap(), slots.next().unwrap(), slots.next().unwrap());
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let lsp = |slot: u32, p: &Placed, cells: &str| format!("lsp {slot},\":a{cells};{}\",{},{}", path(&p.image), p.x, p.y);
    let number = |i: usize| BAR_BUTTON_BASE + 1 + i as i32;
    let label = |a: BarAction| match a {
        BarAction::Auto => "vn2ons_bar_auto",
        BarAction::Skip => "vn2ons_bar_skip",
        BarAction::Log => "vn2ons_bar_log",
        BarAction::Save => "vn2ons_bar_save",
        BarAction::Load => "vn2ons_bar_load",
        BarAction::Settings => "vn2ons_bar_settings",
        BarAction::HideWindow | BarAction::Unavailable => "vn2ons_bar_hide",
    };
    let (hover_top, hover_bottom, hover_hide) = (HOVER_BASE + 1, HOVER_BASE + 2, HOVER_BASE + 3);

    w(format!("*{WAIT}"));
    w(format!("gosub *{BAR_DRAW}"));
    w(format!("textbtnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} > {HOVER_BASE} goto *vn2ons_wait_hover"));
    // Wheel down (-3) reads on, like a click; wheel up (-2) opens the log.
    w(format!("if {BUTTON_VAR} == -3 goto *vn2ons_wait_end"));
    w(format!("if {BUTTON_VAR} >= 0 && {BUTTON_VAR} <= {BAR_BUTTON_BASE} goto *vn2ons_wait_end"));
    w(format!("gosub *{BAR_ACTION}"));
    // Auto and skip go on to the next page; the rest come back here.
    for (i, (b, _)) in buttons.iter().enumerate() {
        if matches!(b.action, BarAction::Auto | BarAction::Skip) {
            w(format!("if {BUTTON_VAR} == {} goto *vn2ons_wait_end", number(i)));
        }
    }
    w(format!("goto *{WAIT}"));
    w("*vn2ons_wait_hover".into());
    w(format!("gosub *{BAR_HOVER}"));
    w(format!("goto *{WAIT}"));
    w("*vn2ons_wait_end".into());
    w("texec".into());
    w("return".into());

    w(format!("*{BAR_HOVER}"));
    w(format!("if {BUTTON_VAR} == {hover_top} mov {TOP_SHOWN},1"));
    w(format!("if {BUTTON_VAR} == {hover_bottom} mov {BOTTOM_SHOWN},1"));
    w(format!("if {BUTTON_VAR} == {hover_hide} mov {TOP_SHOWN},0:mov {BOTTOM_SHOWN},0"));
    w("return".into());

    // Removes every bar sprite.
    w(format!("*{BAR_HIDE}"));
    w(format!("mov {TOP_SHOWN},0:mov {BOTTOM_SHOWN},0"));
    w(format!("gosub *{BAR_CLEAR}"));
    w("print 1".into());
    w("return".into());
    w(format!("*{BAR_CLEAR}"));
    w(BAR_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w(TOP_SPRITES2.map(|n| format!("csp2 {n}")).collect::<Vec<_>>().join(":"));
    w("return".into());

    // Draws the bars that are up, or the hotspots that bring them up.
    // Buttons registered later are tested first, so the hide hotspot comes
    // first and link words (registered after this) win over it.
    w(format!("*{BAR_DRAW}"));
    w("btndef clear".into());
    w(format!("gosub *{BAR_CLEAR}"));
    w(format!("if {TOP_SHOWN} == 1 || {BOTTOM_SHOWN} == 1 {}:spbtn {hide},{hover_hide}", lsp(hide, &bar.hide, "")));
    w(format!("if {BOTTOM_SHOWN} == 0 {}:spbtn {reveal},{hover_bottom}:goto *vn2ons_bar_draw_top", lsp(reveal, &bar.reveal, "")));
    w(format!("isskip {MODE_STATE}"));
    for (i, (b, slot)) in buttons.iter().enumerate() {
        if b.action == BarAction::Unavailable {
            w(lsp(*slot, &b.image, ""));
            continue;
        }
        w(lsp(*slot, &b.image, "/3,0,3"));
        if let Some(active) = &b.active {
            let state = if b.action == BarAction::Auto { 2 } else { 1 };
            w(format!("if {MODE_STATE} == {state} {}", lsp(*slot, active, "/3,0,3")));
        }
        w(format!("spbtn {slot},{}", number(i)));
    }
    w("*vn2ons_bar_draw_top".into());
    if let Some(top) = &bar.top {
        w(format!("if {TOP_SHOWN} == 0 {}:spbtn {reveal_top},{hover_top}:goto *vn2ons_bar_draw_end", lsp(reveal_top, &top.reveal, "")));
        let mut sprites2 = TOP_SPRITES2;
        for p in [&top.chapter_bar, &top.scene_bar].into_iter().chain(&top.decorations) {
            w(lsp2(sprites2.next().unwrap(), p, None));
        }
        let text = |slot: u32, t: &TextSpot, var: &str| {
            format!("strsp {slot},{var},{},{},{},1,{},{},0,0,0,1", t.x, t.y, t.columns, t.font_size, t.font_size)
        };
        w(text(chapter_text, &top.chapter_text, CHAPTER));
        // The scene: its title at the current depth, or the root's.
        w(format!("mov {SCENE_TMP},\"{}\"", top.root_scene));
        w(format!("mov {TMP},{SCENE_TITLES}+{SCENE_DEPTH}"));
        w(format!("if {SCENE_DEPTH} > 0 mov {SCENE_TMP},${TMP}"));
        w(text(scene_text, &top.scene_text, SCENE_TMP));
        for (var, (x, y), slot) in [(SCENE_DEPTH, top.depth_at, sprites2.next().unwrap()), (SCENE_MAX_DEPTH, top.max_depth_at, sprites2.next().unwrap())] {
            for (d, image) in top.digits.iter().enumerate() {
                w(format!("if {var} == {d} lsp2 {slot},\":a;{}\",{},{},100,100,0", path(image), x + 4, y + 6));
            }
        }
    }
    w("*vn2ons_bar_draw_end".into());
    w(format!("getmouseover {},{}", HOVER_BASE + 1, HOVER_BASE + 3));
    w("print 1".into());
    w("return".into());

    w(format!("*{BAR_ACTION}"));
    w(format!("if {BUTTON_VAR} == -1 goto *vn2ons_bar_hide"));
    w(format!("if {BUTTON_VAR} == -2 goto *vn2ons_bar_log"));
    for (i, (b, _)) in buttons.iter().enumerate() {
        if b.action != BarAction::Unavailable {
            w(format!("if {BUTTON_VAR} == {} goto *{}", number(i), label(b.action)));
        }
    }
    w("return".into());
    let mut actions: Vec<BarAction> = bar.buttons.iter().map(|b| b.action).collect();
    actions.push(BarAction::HideWindow);
    actions.push(BarAction::Log);
    actions.sort_by_key(|a| label(*a));
    actions.dedup_by_key(|a| label(*a));
    // The save menu covers the screen: put the bars away first.
    let menu = |entry: &str| format!("mov {TOP_SHOWN},0:mov {BOTTOM_SHOWN},0\ngosub *{BAR_CLEAR}\nmov {IN_GAME},1\ngosub *{entry}");
    for a in actions {
        w(format!("*{}", label(a)));
        match a {
            BarAction::Auto => w("systemcall automode".into()),
            BarAction::Skip => w("systemcall skip".into()),
            BarAction::Log if backlog => w(menu(BACKLOG)),
            BarAction::Log => w("systemcall lookback".into()),
            BarAction::Settings if settings => w(menu(SETTINGS)),
            BarAction::Settings => w("return".into()),
            BarAction::Save if save_menu => w(menu(SAVE)),
            BarAction::Load if save_menu => w(menu(LOAD)),
            BarAction::Save => w("systemcall save".into()),
            BarAction::Load => w("systemcall load".into()),
            BarAction::HideWindow | BarAction::Unavailable => w("systemcall windowerase".into()),
        }
        w("return".into());
    }
}

/// The settings screen (see [`SettingsMenu`]): levels 0-10 in globals,
/// applied by [`SETTINGS_APPLY`] (also run at startup by
/// [`settings_init`]).
pub(crate) fn settings_menu(out: &mut String, m: &SettingsMenu) {
    let rows = &m.rows[..m.rows.len().min(MAX_ROWS)];
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let size = m.font_size;
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    w(format!("*{SETTINGS}"));
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(lsp2(BASE, &m.base, Some(m.base_opacity)));
    w(lsp2(HEADER, &m.header, None));
    w(format!("lsp {CLOSE},\":a/3,0,3;{}\",{},{}", path(&m.close.image), m.close.x, m.close.y));
    w("*vn2ons_settings_draw".into());
    for (r, row) in rows.iter().enumerate() {
        let y = m.y + r as i32 * m.pitch;
        let level = setting_var(row.setting);
        let r = r as u32;
        w(format!("strsp {},\"{}\",{},{y},10,1,{size},{size},0,0,0,1", TEXT1 + r, row.label, m.label_x));
        for (slot, x, sign) in [(SETTING_MINUS + r, m.minus_x, "－"), (SETTING_PLUS + r, m.plus_x, "＋")] {
            w(format!("strsp {slot},\"{sign}\",{x},{y},1,1,{size},{size},0,0,0,1,\"#FFFFFF\",\"#F0C040\""));
        }
        // "■■■■■■■□□□　７"
        w(format!("mov {STR_TMP},\"\""));
        for i in 1..=10 {
            w(format!("if {level} >= {i} add {STR_TMP},\"■\""));
            w(format!("if {level} < {i} add {STR_TMP},\"□\""));
        }
        w(format!("add {STR_TMP},\"　\""));
        w(format!("itoa2 {LINE1},{level}"));
        w(format!("add {STR_TMP},{LINE1}"));
        w(format!("strsp {},{STR_TMP},{},{y},14,1,{size},{size},0,0,0,1", SETTING_LEVEL + r, m.level_x));
    }
    w("*vn2ons_settings_wait".into());
    for r in 0..rows.len() as u32 {
        w(format!("spbtn {},{}", SETTING_MINUS + r, BTN_MINUS + r + 1));
        w(format!("spbtn {},{}", SETTING_PLUS + r, BTN_PLUS + r + 1));
    }
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *vn2ons_settings_close"));
    for (r, row) in rows.iter().enumerate() {
        let level = setting_var(row.setting);
        let r = r as u32;
        w(format!("if {BUTTON_VAR} == {} && {level} > 0 sub {level},1", BTN_MINUS + r + 1));
        w(format!("if {BUTTON_VAR} == {} && {level} < 10 add {level},1", BTN_PLUS + r + 1));
    }
    w(format!("gosub *{SETTINGS_APPLY}"));
    w("goto *vn2ons_settings_draw".into());
    w("*vn2ons_settings_close".into());
    // texton first: it drops the pending screen updates except the window's.
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    w(format!("csp2 {BASE}:csp2 {HEADER}"));
    w(MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("print 1".into());
    w("return".into());

    w(format!("*{SETTINGS_APPLY}"));
    for (setting, cmd) in [(Setting::BgmVolume, "bgmvol"), (Setting::SeVolume, "sevol"), (Setting::VoiceVolume, "voicevol")] {
        w(format!("mov {TMP},{}*10:{cmd} {TMP}", setting_var(setting)));
    }
    w(format!("mov {TEXT_MS},(10-{})*{TEXT_MS_STEP}:textspeed {TEXT_MS}", setting_var(Setting::TextSpeed)));
    w(format!("mov {TMP},(10-{})*400+600:automode_time {TMP}", setting_var(Setting::AutoSpeed)));
    w("return".into());
}

/// First run: default levels; every run: apply them (before `setwindow`,
/// which takes the text speed from [`TEXT_MS`]).
pub(crate) fn settings_init(out: &mut String) {
    let v = setting_var;
    out.push_str(&format!(
        "if {SETTINGS_SET} == 0 mov {},8:mov {},8:mov {},10:mov {},7:mov {},5:mov {SETTINGS_SET},1\ngosub *{SETTINGS_APPLY}\n",
        v(Setting::BgmVolume),
        v(Setting::SeVolume),
        v(Setting::VoiceVolume),
        v(Setting::TextSpeed),
        v(Setting::AutoSpeed)
    ));
}

fn setting_var(s: Setting) -> &'static str {
    match s {
        Setting::BgmVolume => "%902",
        Setting::SeVolume => "%903",
        Setting::VoiceVolume => "%904",
        Setting::TextSpeed => "%905",
        Setting::AutoSpeed => "%906",
    }
}

/// The text history (see [`Backlog`]). Pages come from `getlog` (0 = the
/// page on screen); a page's line count is its length with line breaks
/// (`getlog`) minus without (`getlogtext`), plus one. [`LOG_OFFSET`] is
/// the newest page shown; up/wheel-up go back one page, down/wheel-down
/// forward (past the newest: close).
pub(crate) fn backlog(out: &mut String, b: &Backlog) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    let button = |slot: u32, p: &Placed| format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&p.image), p.x, p.y);
    let a = &b.area;
    let size = b.font_size;
    let gap = b.line_height - size as i32;
    w(format!("*{BACKLOG}"));
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(lsp2(BASE, &b.base, Some(b.base_opacity)));
    w(lsp2(FRAME, &b.frame, None));
    w(lsp2(HEADER, &b.track, None));
    w(button(LOG_UP, &b.up));
    w(button(LOG_DOWN, &b.down));
    w(button(CLOSE, &b.close));
    w(format!("mov {LOG_OFFSET},0"));
    w("*vn2ons_log_draw".into());
    w((0..LOG_PAGES).map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w(format!("mov {LOG_Y},{}", a.y + a.h));
    w(format!("mov {LOG_K},0"));
    w("*vn2ons_log_page".into());
    w(format!("if {LOG_K} >= {LOG_PAGES} goto *vn2ons_log_shown"));
    w(format!("mov {TMP},{LOG_OFFSET}+{LOG_K}"));
    w(format!("getlog {LINE1},{TMP}"));
    w(format!("if {LINE1} == \"\" goto *vn2ons_log_shown"));
    w(format!("getlogtext {LINE2},{TMP}"));
    w(format!("len {LOG_LINES},{LINE1}"));
    w(format!("len {TMP},{LINE2}"));
    w(format!("mov {LOG_LINES},{LOG_LINES}-{TMP}+1"));
    w(format!("mov {TMP},{LOG_Y}-{LOG_LINES}*{}-{}", b.line_height, b.line_height / 2));
    // Doesn't fit: stop (but always show at least one page).
    w(format!("if {TMP} < {} && {LOG_K} > 0 goto *vn2ons_log_shown", a.y));
    w(format!("if {TMP} < {} mov {TMP},{}", a.y, a.y));
    w(format!("strsp {LOG_K},{LINE1},{},{TMP},{},{LOG_LINES},{size},{size},0,{gap},0,1", a.x, b.columns));
    w(format!("mov {LOG_Y},{TMP}"));
    w(format!("add {LOG_K},1"));
    w("goto *vn2ons_log_page".into());
    w("*vn2ons_log_shown".into());
    w(format!("cell {LOG_DOWN},0:cell {LOG_UP},0"));
    w(format!("if {LOG_OFFSET} == 0 cell {LOG_DOWN},2"));
    // LOG_OLDER: there is an older page to scroll to.
    w(format!("mov {TMP},{LOG_OFFSET}+1"));
    w(format!("getlog {LINE2},{TMP}"));
    w(format!("mov {LOG_OLDER},1"));
    w(format!("if {LINE2} == \"\" mov {LOG_OLDER},0:cell {LOG_UP},2"));
    w("*vn2ons_log_wait".into());
    w(format!("if {LOG_OLDER} == 1 spbtn {LOG_UP},1"));
    w(format!("if {LOG_OFFSET} > 0 spbtn {LOG_DOWN},2"));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *vn2ons_log_close"));
    w(format!("if {BUTTON_VAR} == -2 mov {BUTTON_VAR},1"));
    w(format!("if {BUTTON_VAR} == -3 mov {BUTTON_VAR},2"));
    w(format!("if {BUTTON_VAR} == 1 && {LOG_OLDER} == 1 add {LOG_OFFSET},1:goto *vn2ons_log_draw"));
    w(format!("if {BUTTON_VAR} == 2 && {LOG_OFFSET} == 0 goto *vn2ons_log_close"));
    w(format!("if {BUTTON_VAR} == 2 sub {LOG_OFFSET},1:goto *vn2ons_log_draw"));
    w("goto *vn2ons_log_wait".into());
    w("*vn2ons_log_close".into());
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    w(format!("csp2 {BASE}:csp2 {FRAME}:csp2 {HEADER}"));
    w(MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("print 1".into());
    w("return".into());
}

/// Sprites hidden while a full-screen menu is open: the bars, the name
/// plate and the link words.
fn hidden_while_open() -> Vec<u32> {
    BAR_SPRITES.chain([NAME_TEXT_SPRITE, NAME_PLATE_SPRITE]).chain(LINK_SPRITES).collect()
}

/// `lsp2` places the image by its centre.
fn lsp2(slot: u32, p: &Placed, opacity: Option<u8>) -> String {
    let (w, h) = (p.w, p.h);
    let opacity = opacity.map_or(String::new(), |o| format!(",{o}"));
    format!("lsp2 {slot},\":a;{}\",{},{},100,100,0{opacity}", path(&p.image), p.x + w / 2, p.y + h / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vn2ons_core::ir::{PageButton, Rect};

    fn placed(name: &str, w: i32, h: i32) -> Placed {
        Placed { image: format!("system/{name}.png"), x: 10, y: 20, w, h }
    }

    #[test]
    fn save_menu_routine() {
        let page = |n: i32| PageButton { button: placed(&format!("page{n}"), 30, 27), marker_x: 100 * n, marker_y: 480 };
        let m = SaveMenu {
            base: placed("base", 800, 600),
            base_opacity: 140,
            frame: placed("frame", 800, 600),
            save_header: placed("save_header", 173, 74),
            load_header: placed("load_header", 173, 74),
            delete: placed("delete", 285, 33),
            save: placed("save", 237, 33),
            load: placed("load", 237, 33),
            close: placed("close", 267, 33),
            pages: vec![page(1), page(2)],
            page_marker: placed("marker", 28, 9),
            rows: 5,
            row_y: 44,
            row_pitch: 92,
            row_hover: placed("hover", 758, 92),
            row_selected: placed("selected", 379, 92),
            thumbnail: Rect { x: 14, y: 10, w: 100, h: 75 },
            new_icon: placed("new", 66, 20),
            text_x: 124,
            text_y: 11,
            font_size: 14,
            line_height: 15,
            preview: Rect { x: 390, y: 137, w: 400, h: 300 },
        };
        let mut out = String::new();
        save_menu(&mut out, &m);
        for line in [
            "if %100 == 1 getscreenshot 400,300\n",
            // lsp2 takes the centre.
            "lsp2 11,\":a;system\\base.png\",410,320,100,100,0,140\n",
            // Row 2 (slot FIRST + 1): thumbnail at 25%, centred in its box.
            "if %104 == 1 lsp2 2,$105,64,183,25,25,0\n",
            "if %900 == 2 cell 16,2:lsp2 7,\":a;system\\marker.png\",214,484,100,100,0\n",
            "if %9 > 100 && %9 <= 102 mov %900,%9-100:goto *vn2ons_sl_page\n",
            "savescreenshot2 $103\n",
            "savegame2 %102,$100\n",
            "loadgame %102\n",
            "mov %%105,1\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    #[test]
    fn button_bar_routine() {
        use vn2ons_core::ir::BarButton;
        let bar = ButtonBar {
            buttons: vec![
                BarButton { image: placed("auto", 132, 19), active: Some(placed("auto_on", 132, 19)), action: BarAction::Auto },
                BarButton { image: placed("save", 132, 19), active: None, action: BarAction::Save },
                BarButton { image: placed("setting", 56, 19), active: None, action: BarAction::Unavailable },
            ],
            reveal: placed("reveal", 800, 25),
            hide: placed("hide", 800, 500),
            top: None,
        };
        let mut out = String::new();
        button_bar(&mut out, &bar, true, false, false);
        for line in [
            "*vn2ons_wait\ngosub *vn2ons_bar_draw\ntextbtnwait %9\nif %9 > 2000 goto *vn2ons_wait_hover\n",
            "if %9 >= 0 && %9 <= 1000 goto *vn2ons_wait_end\n",
            "if %9 == 1001 goto *vn2ons_wait_end\ngoto *vn2ons_wait\n",
            "*vn2ons_wait_end\ntexec\nreturn\n",
            // Hotspots: the hide one first (tested last), the bottom one
            // while the bar is down.
            "btndef clear\ngosub *vn2ons_bar_clear\n\
             if %160 == 1 || %161 == 1 lsp 37,\":a;system\\hide.png\",10,20:spbtn 37,2003\n\
             if %161 == 0 lsp 36,\":a;system\\reveal.png\",10,20:spbtn 36,2002:goto *vn2ons_bar_draw_top\n",
            "lsp 33,\":a/3,0,3;system\\auto.png\",10,20\nif %108 == 2 lsp 33,\":a/3,0,3;system\\auto_on.png\",10,20\nspbtn 33,1001\n",
            "lsp 35,\":a;system\\setting.png\",10,20\n*vn2ons_bar_draw_top\n*vn2ons_bar_draw_end\ngetmouseover 2001,2003\nprint 1\n",
            "*vn2ons_bar_hover\nif %9 == 2001 mov %160,1\nif %9 == 2002 mov %161,1\nif %9 == 2003 mov %160,0:mov %161,0\nreturn\n",
            "if %9 == -1 goto *vn2ons_bar_hide\nif %9 == -2 goto *vn2ons_bar_log\nif %9 == 1001 goto *vn2ons_bar_auto\nif %9 == 1002 goto *vn2ons_bar_save\nreturn\n",
            "*vn2ons_bar_log\nsystemcall lookback\nreturn\n",
            "*vn2ons_bar_save\nmov %160,0:mov %161,0\ngosub *vn2ons_bar_clear\nmov %101,1\ngosub *vn2ons_save\nreturn\n",
            "*vn2ons_bar_hide\nsystemcall windowerase\nreturn\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }
}
