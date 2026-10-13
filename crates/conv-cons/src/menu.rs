//! The in-game menus as NScripter subroutines: the save/load screen (see
//! [`SaveMenu`]) and the button bar shown at click waits ([`ButtonBar`]).
//!
//! Enter with `mov %101,1:gosub *conv_save` (or `conv_load`) from a
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
//! - Delete removes the save and its thumbnail with CONscripter's
//!   `delsavegame`. Builds before it hid deleted slots with a global flag
//!   per slot; a slot still flagged is deleted when the menu lists it.
//! - Layers: `lsp2` sprites are drawn under the text window and under
//!   sprites 0-49 (`windowback`, `humanz 49`), so the base, frame,
//!   thumbnails and the (additive) selection bar are `lsp2` sprites, and
//!   buttons and labels `lsp` sprites over them. The text window, name
//!   plate and link words are hidden while the menu is open.

use conv_core::ir::{BarAction, Backlog, ButtonBar, ConfirmDialog, Placed, SaveMenu, Setting, SettingsMenu, TextSpot, VoiceToggle};

use crate::{escape, path, BAR_SPRITES, BUTTON_VAR, DIALOG_SPRITES, LINK_SPRITES, MENU_SPRITES, NAME_PLATE_SPRITE, NAME_TEXT_SPRITE};

pub(crate) const SAVE: &str = "conv_save";
/// The `textgosub` routine.
pub(crate) const WAIT: &str = "conv_wait";
/// Shows the button bar and makes its buttons clickable.
pub(crate) const BAR_DRAW: &str = "conv_bar_draw";
/// Runs the bar button in %9 (or -1, a right-click).
pub(crate) const BAR_ACTION: &str = "conv_bar_action";
pub(crate) const SETTINGS: &str = "conv_settings";
const SETTINGS_APPLY: &str = "conv_settings_apply";
pub(crate) const BACKLOG: &str = "conv_backlog";
/// The `pretextgosub` routine (see [`backlog`]).
pub(crate) const PRETEXT: &str = "conv_pretext";
/// Settings levels are %902-%906 (globals); %909 = 1 once they are set.
const SETTINGS_SET: &str = "%909";
/// 1: skip goes on through unread text (`kidokumode 0`).
const SKIP_UNREAD: &str = "%908";
/// Milliseconds per character, for `setwindow`.
pub(crate) const TEXT_MS: &str = "%907";
const TEXT_MS_STEP: u32 = 6;
// Settings and backlog sprites (lsp, within MENU_SPRITES).
const SETTING_MINUS: u32 = M + 10; // + row
const SETTING_PLUS: u32 = M + 15; // + row
const SETTING_LEVEL: u32 = M + 20; // + row
const SETTING_TITLE: u32 = M + 25;
const SETTING_QUIT: u32 = M + 26;
const SETTING_TAB: u32 = M + 5; // + page
const SETTING_SKIP: u32 = M + 8;
/// Voice switches, on the voice page (in place of the level rows).
const SETTING_VOICE: u32 = M + 7; // + switch
const MAX_VOICES: usize = 18;
const BTN_MINUS: u32 = 10;
const BTN_PLUS: u32 = 20;
const BTN_TITLE: u32 = 30;
const BTN_QUIT: u32 = 31;
const BTN_TAB: u32 = 40; // + 1 + page
const BTN_SKIP: u32 = 32;
const BTN_VOICE: u32 = 50; // + 1 + switch
const LOG_PAGES: u32 = 10; // text sprites M..M + LOG_PAGES, names after
const LOG_UP: u32 = M + 20;
const LOG_DOWN: u32 = M + 21;
const LOG_OFFSET: &str = "%170";
const LOG_Y: &str = "%171";
const LOG_K: &str = "%172";
const LOG_LINES: &str = "%173";
const LOG_OLDER: &str = "%174";
const LOG_SPRITE: &str = "%175";
const LOG_TEXT_Y: &str = "%177";
const LOG_NAMED: &str = "%178";
const LOG_TAG: &str = "$107";
const LOG_NAME: &str = "$108";
const LOG_VOICE: &str = "$109";
/// The voice of shown page k is in $(LOG_VOICES + k).
const LOG_VOICES: u32 = 120;
/// Speaker names: sprites M + 10 + k.
const LOG_NAME_SPRITE: u32 = M + 10;
const LOG_NAME_COLOR: &str = "#D8C088";
const BTN_LOG_VOICE: u32 = 210; // + k
/// The settings page shown: 0 levels, 1 voices.
const SETTINGS_PAGE: &str = "%176";
/// Voice character c is muted when the global %(VOICE_OFF_BASE + c) is 1.
pub(crate) const VOICE_OFF_BASE: u32 = 970;
/// Asks the question in [`CONFIRM_TEXT`]; [`CONFIRM`] = 1 for yes.
pub(crate) const CONFIRM_ROUTINE: &str = "conv_confirm";
pub(crate) const CONFIRM: &str = "%109";
const CONFIRM_TEXT: &str = "$106";
/// Bar buttons return numbers above this.
pub(crate) const BAR_BUTTON_BASE: i32 = 1000;
/// Sets which bars are up after a hotspot reported the pointer (%9).
pub(crate) const BAR_HOVER: &str = "conv_bar_hover";
const BAR_CLEAR: &str = "conv_bar_clear";
/// Puts the bars away (they come back when the pointer nears an edge).
pub(crate) const BAR_HIDE: &str = "conv_bar_hide_all";
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
pub(crate) const LOAD: &str = "conv_load";
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
/// Slot n was deleted by an older build when %(DELETED_BASE + n) is 1.
const DELETED_BASE: u32 = 910;
/// The current chapter's title (a local: saves keep it), shown in the top
/// bar and under a save's date.
pub(crate) const CHAPTER: &str = "$150";

// lsp slots (all within MENU_SPRITES).
const M: u32 = MENU_SPRITES.start;
const TEXT1: u32 = M; // + row
const TEXT2: u32 = M + 5; // + row
const ROW_BUTTON: u32 = M + 10; // + row
const PAGE_BUTTON: u32 = M + 15; // + page index
const DELETE: u32 = M + 25;
const ACTION: u32 = M + 26;
const CLOSE: u32 = M + 27;
const MAX_ROWS: usize = 5;
const MAX_PAGES: usize = 10;
// Dialog lsp slots (DIALOG_SPRITES).
const DIALOG_TEXT: u32 = DIALOG_SPRITES.start;
const DIALOG_YES_LABEL: u32 = DIALOG_TEXT + 1;
const DIALOG_NO_LABEL: u32 = DIALOG_TEXT + 2;
const DIALOG_YES: u32 = DIALOG_TEXT + 3;
const DIALOG_NO: u32 = DIALOG_TEXT + 4;
const DIALOG_BASE: u32 = DIALOG_TEXT + 5;
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

pub(crate) fn save_menu(out: &mut String, m: &SaveMenu, confirm: Option<&ConfirmDialog>) {
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
    w("goto *conv_sl".into());
    w(format!("*{LOAD}"));
    w(format!("mov {MODE},2"));
    w("*conv_sl".into());
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
    w("*conv_sl_page".into());
    w(format!("mov {SELECTED},0"));
    w(format!("csp2 {SELECTION}:csp2 {PREVIEW}:csp2 {NEW}:csp2 {MARKER}"));
    w(format!("mov {FIRST},{PAGE}*{rows}-{}", rows - 1));
    for row in 0..rows {
        let y = row_y(row);
        w(format!("mov {SLOT},{FIRST}+{row}"));
        w("gosub *conv_sl_slot".into());
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
    w("*conv_sl_buttons".into());
    w(format!("cell {DELETE},2:cell {ACTION},2"));
    w(format!("if {SELECTED} > 0 && {SELECTED_HAS_DATA} == 1 cell {DELETE},0:cell {ACTION},0"));
    w(format!("if {SELECTED} > 0 && {MODE} == 1 cell {ACTION},0"));
    w("*conv_sl_wait".into());
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
    w(format!("if {BUTTON_VAR} == -1 goto *conv_sl_close"));
    w(format!("if {BUTTON_VAR} == {BTN_CLOSE} goto *conv_sl_close"));
    w(format!("if {BUTTON_VAR} == {BTN_DELETE} goto *conv_sl_delete"));
    w(format!("if {BUTTON_VAR} == {BTN_ACTION} goto *conv_sl_act"));
    w(format!(
        "if {BUTTON_VAR} > {BTN_PAGE} && {BUTTON_VAR} <= {} mov {PAGE},{BUTTON_VAR}-{BTN_PAGE}:goto *conv_sl_page",
        BTN_PAGE + pages.len() as u32
    ));
    w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {rows} goto *conv_sl_row"));
    w("goto *conv_sl_wait".into());

    // A row was clicked: select it, or act on it if it already was.
    w("*conv_sl_row".into());
    w(format!("mov {SLOT},{FIRST}+{BUTTON_VAR}-1"));
    w(format!("if {SLOT} == {SELECTED} goto *conv_sl_act"));
    w(format!("mov {SELECTED},{SLOT}"));
    w("gosub *conv_sl_slot".into());
    w(format!("mov {SELECTED_HAS_DATA},{HAS_DATA}"));
    let sel = &m.row_selected;
    w(format!("mov {TMP},({BUTTON_VAR}-1)*{}+{}", m.row_pitch, m.row_y + sel.y + sel.h / 2));
    w(format!("lsp2add {SELECTION},\":c;{}\",{},{TMP},100,100,0", path(&sel.image), sel.x + sel.w / 2));
    let (px, py) = (m.preview.x + m.preview.w / 2, m.preview.y + m.preview.h / 2);
    w(format!("if {HAS_DATA} == 1 lsp2 {PREVIEW},{THUMB_IMAGE},{px},{py},100,100,0"));
    w(format!("if {HAS_DATA} == 0 csp2 {PREVIEW}"));
    w("goto *conv_sl_buttons".into());

    // Asks "No.NN {question}" about the selected slot; no: back to waiting.
    let ask = |question: &str| match confirm {
        Some(_) => format!(
            "mov {SLOT},{SELECTED}\ngosub *conv_sl_number\nmov {CONFIRM_TEXT},{LINE1}\nadd {CONFIRM_TEXT},\" {}\"\ngosub *{CONFIRM_ROUTINE}\nif {CONFIRM} == 0 goto *conv_sl_buttons",
            escape(question)
        ),
        None => String::new(),
    };
    w("*conv_sl_act".into());
    w(format!("if {SELECTED} == 0 goto *conv_sl_wait"));
    w(format!("if {MODE} == 2 goto *conv_sl_load"));
    if let Some(c) = confirm {
        w(format!("if {SELECTED_HAS_DATA} == 0 goto *conv_sl_save"));
        w(ask(&c.overwrite));
    }
    w("*conv_sl_save".into());
    w(format!("mov {SLOT},{SELECTED}"));
    w("gosub *conv_sl_file".into());
    w(format!("savescreenshot2 {FILE}"));
    // Label: "No.NN - 20YY/MM/DD hh:mm:ss|chapter". ONScripter (Shift-JIS)
    // draws half-width characters two per cell and drops a space that
    // starts a cell, so spaces at even positions are full-width.
    w("gosub *conv_sl_number".into());
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
    w(format!("mov {NEWEST},{SELECTED}"));
    w("goto *conv_sl_page".into());

    w("*conv_sl_load".into());
    w(format!("if {SELECTED_HAS_DATA} == 0 goto *conv_sl_wait"));
    if let Some(c) = confirm {
        w(ask(&c.load));
    }
    w("gosub *conv_sl_clear".into());
    w(format!("loadgame {SELECTED}"));
    // Still here: the load failed.
    w("goto *conv_sl".into());

    w("*conv_sl_delete".into());
    if let Some(c) = confirm {
        w(ask(&c.delete));
    }
    w(format!("mov {SLOT},{SELECTED}"));
    w("gosub *conv_sl_file".into());
    w(format!("delsavegame {SELECTED},{FILE}"));
    w(format!("if {SELECTED} == {NEWEST} mov {NEWEST},0"));
    w("goto *conv_sl_page".into());

    w("*conv_sl_close".into());
    // texton first: it drops the pending screen updates except the window's.
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    w("gosub *conv_sl_clear".into());
    w("print 1".into());
    w("return".into());

    w("*conv_sl_clear".into());
    w((0..=BASE).map(|n| format!("csp2 {n}")).collect::<Vec<_>>().join(":"));
    w(MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("return".into());

    // Slot SLOT: HAS_DATA, LINE1 and LINE2 (its label), FILE (thumbnail).
    w("*conv_sl_slot".into());
    w("gosub *conv_sl_file".into());
    w(format!("mov {TMP},{SLOT}+{DELETED_BASE}"));
    w(format!("if %{TMP} == 1 delsavegame {SLOT},{FILE}:mov %{TMP},0"));
    w(format!("savefileexist {HAS_DATA},{SLOT}"));
    w(format!("if {HAS_DATA} == 0 goto *conv_sl_empty"));
    w(format!("getsavestr {LABEL},{SLOT}"));
    w(format!("split {LABEL},\"|\",{LINE1},{LINE2}"));
    w(format!("if {LINE2} == \"\" mov {LINE2},\"　\""));
    w("return".into());
    w("*conv_sl_empty".into());
    w("gosub *conv_sl_number".into());
    w(format!("add {LINE1},\" - no　data\""));
    w(format!("mov {LINE2},\"　\""));
    w("return".into());

    w("*conv_sl_number".into());
    w(format!("mov {LINE1},\"No.\""));
    w(format!("if {SLOT} < 10 add {LINE1},\"0\""));
    w(format!("itoa {STR_TMP},{SLOT}"));
    w(format!("add {LINE1},{STR_TMP}"));
    w("return".into());

    w("*conv_sl_file".into());
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
        BarAction::Auto => "conv_bar_auto",
        BarAction::Skip => "conv_bar_skip",
        BarAction::Log => "conv_bar_log",
        BarAction::Save => "conv_bar_save",
        BarAction::Load => "conv_bar_load",
        BarAction::Settings => "conv_bar_settings",
        BarAction::HideWindow | BarAction::Unavailable => "conv_bar_hide",
    };
    let (hover_top, hover_bottom, hover_hide) = (HOVER_BASE + 1, HOVER_BASE + 2, HOVER_BASE + 3);

    w(format!("*{WAIT}"));
    w(format!("gosub *{BAR_DRAW}"));
    w(format!("textbtnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} > {HOVER_BASE} goto *conv_wait_hover"));
    // Wheel down (-3) reads on, like a click; wheel up (-2) opens the log.
    w(format!("if {BUTTON_VAR} == -3 goto *conv_wait_end"));
    w(format!("if {BUTTON_VAR} >= 0 && {BUTTON_VAR} <= {BAR_BUTTON_BASE} goto *conv_wait_end"));
    w(format!("gosub *{BAR_ACTION}"));
    // Auto and skip go on to the next page; the rest come back here.
    for (i, (b, _)) in buttons.iter().enumerate() {
        if matches!(b.action, BarAction::Auto | BarAction::Skip) {
            w(format!("if {BUTTON_VAR} == {} goto *conv_wait_end", number(i)));
        }
    }
    w(format!("goto *{WAIT}"));
    w("*conv_wait_hover".into());
    w(format!("gosub *{BAR_HOVER}"));
    w(format!("goto *{WAIT}"));
    w("*conv_wait_end".into());
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
    w(format!("if {BOTTOM_SHOWN} == 0 {}:spbtn {reveal},{hover_bottom}:goto *conv_bar_draw_top", lsp(reveal, &bar.reveal, "")));
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
    w("*conv_bar_draw_top".into());
    if let Some(top) = &bar.top {
        w(format!("if {TOP_SHOWN} == 0 {}:spbtn {reveal_top},{hover_top}:goto *conv_bar_draw_end", lsp(reveal_top, &top.reveal, "")));
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
    w("*conv_bar_draw_end".into());
    w(format!("getmouseover {},{}", HOVER_BASE + 1, HOVER_BASE + 3));
    w("print 1".into());
    w("return".into());

    w(format!("*{BAR_ACTION}"));
    w(format!("if {BUTTON_VAR} == -1 goto *conv_bar_hide"));
    w(format!("if {BUTTON_VAR} == -2 goto *conv_bar_log"));
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
/// [`settings_init`]), and on a second page the voice switches (globals
/// from [`VOICE_OFF_BASE`], read where each voice plays).
pub(crate) fn settings_menu(out: &mut String, m: &SettingsMenu, confirm: Option<&ConfirmDialog>) {
    let rows = &m.rows[..m.rows.len().min(MAX_ROWS)];
    let voices = &m.voices[..m.voices.len().min(MAX_VOICES)];
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let size = m.font_size;
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    // `strsp` drops a line's last character unless there is a column to
    // spare.
    let hover = |slot: u32, text: &str, x: i32, y: i32, chars: usize| {
        let columns = chars + 1;
        format!("strsp {slot},{text},{x},{y},{columns},1,{size},{size},0,0,0,1,\"#FFFFFF\",\"#F0C040\"")
    };
    // A switch's state is its first character's.
    let off = |t: &VoiceToggle| format!("%{}", VOICE_OFF_BASE + t.characters.first().copied().unwrap_or(0));
    let column_rows = voices.len().div_ceil(2);
    w(format!("*{SETTINGS}"));
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(format!("mov {SETTINGS_PAGE},0"));
    w(lsp2(BASE, &m.base, Some(m.base_opacity)));
    w(lsp2(HEADER, &m.header, None));
    w(format!("lsp {CLOSE},\":a/3,0,3;{}\",{},{}", path(&m.close.image), m.close.x, m.close.y));
    // Back to the title and quit, from the game.
    for (slot, x, label) in [(SETTING_TITLE, m.label_x, &m.title_label), (SETTING_QUIT, m.level_x, &m.quit_label)] {
        w(format!("if {IN_GAME} == 1 {}", hover(slot, &format!("\"{}\"", escape(label)), x, m.exits_y, 20)));
    }
    w("*conv_settings_draw".into());
    w((M..SETTING_TITLE).map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    if !voices.is_empty() {
        // Tabs: the current page's in the highlight colour, not clickable.
        let mut x = m.tabs_x;
        for (page, label) in m.tab_labels.iter().enumerate() {
            let slot = SETTING_TAB + page as u32;
            let text = format!("\"{}\"", escape(label));
            let chars = label.chars().count();
            w(format!("if {SETTINGS_PAGE} != {page} {}", hover(slot, &text, x, m.tabs_y, chars)));
            w(format!(
                "if {SETTINGS_PAGE} == {page} strsp {slot},{text},{x},{},{},1,{size},{size},0,0,0,1,\"#F0C040\"",
                m.tabs_y,
                chars + 1
            ));
            x += (chars as i32 + 2) * (size as i32 + 1);
        }
        w(format!("if {SETTINGS_PAGE} == 1 goto *conv_settings_draw_voices"));
    }
    for (r, row) in rows.iter().enumerate() {
        let y = m.y + r as i32 * m.pitch;
        let level = setting_var(row.setting);
        let r = r as u32;
        w(format!("strsp {},\"{}\",{},{y},10,1,{size},{size},0,0,0,1", TEXT1 + r, row.label, m.label_x));
        for (slot, x, sign) in [(SETTING_MINUS + r, m.minus_x, "－"), (SETTING_PLUS + r, m.plus_x, "＋")] {
            w(hover(slot, &format!("\"{sign}\""), x, y, 1));
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
    if let Some(label) = &m.skip_unread_label {
        w(format!("mov {STR_TMP},\"□　\""));
        w(format!("if {SKIP_UNREAD} == 1 mov {STR_TMP},\"■　\""));
        w(format!("add {STR_TMP},\"{}\"", escape(label)));
        w(hover(SETTING_SKIP, STR_TMP, m.label_x, m.y + rows.len() as i32 * m.pitch, label.chars().count() + 2));
    }
    w("goto *conv_settings_wait".into());
    // "■ name" (on) or "□ name", in two columns.
    w("*conv_settings_draw_voices".into());
    for (i, t) in voices.iter().enumerate() {
        let x = if i < column_rows { m.label_x } else { m.voice_column_x };
        let y = m.y + (i % column_rows) as i32 * m.voice_pitch;
        w(format!("mov {STR_TMP},\"■　\""));
        w(format!("if {} == 1 mov {STR_TMP},\"□　\"", off(t)));
        w(format!("add {STR_TMP},\"{}\"", escape(&t.label)));
        w(hover(SETTING_VOICE + i as u32, STR_TMP, x, y, t.label.chars().count() + 2));
    }
    w("*conv_settings_wait".into());
    w(format!("if {SETTINGS_PAGE} == 1 goto *conv_settings_wait_voices"));
    for r in 0..rows.len() as u32 {
        w(format!("spbtn {},{}", SETTING_MINUS + r, BTN_MINUS + r + 1));
        w(format!("spbtn {},{}", SETTING_PLUS + r, BTN_PLUS + r + 1));
    }
    if m.skip_unread_label.is_some() {
        w(format!("spbtn {SETTING_SKIP},{BTN_SKIP}"));
    }
    if !voices.is_empty() {
        w(format!("spbtn {},{}", SETTING_TAB + 1, BTN_TAB + 2));
        w("goto *conv_settings_wait_any".into());
        w("*conv_settings_wait_voices".into());
        for i in 0..voices.len() as u32 {
            w(format!("spbtn {},{}", SETTING_VOICE + i, BTN_VOICE + i + 1));
        }
        w(format!("spbtn {SETTING_TAB},{}", BTN_TAB + 1));
    } else {
        w("*conv_settings_wait_voices".into());
    }
    w("*conv_settings_wait_any".into());
    w(format!("if {IN_GAME} == 1 spbtn {SETTING_TITLE},{BTN_TITLE}:spbtn {SETTING_QUIT},{BTN_QUIT}"));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *conv_settings_close"));
    w(format!("if {BUTTON_VAR} == {BTN_TITLE} goto *conv_settings_title"));
    w(format!("if {BUTTON_VAR} == {BTN_QUIT} goto *conv_settings_quit"));
    w(format!(
        "if {BUTTON_VAR} > {BTN_TAB} && {BUTTON_VAR} <= {} mov {SETTINGS_PAGE},{BUTTON_VAR}-{}:goto *conv_settings_draw",
        BTN_TAB + 2,
        BTN_TAB + 1
    ));
    for (i, t) in voices.iter().enumerate() {
        let all = |v: u32| t.characters.iter().map(|c| format!("mov %{},{v}", VOICE_OFF_BASE + c)).collect::<Vec<_>>().join(":");
        let button = BTN_VOICE + i as u32 + 1;
        w(format!("if {BUTTON_VAR} == {button} && {} == 0 {}:goto *conv_settings_draw", off(t), all(1)));
        w(format!("if {BUTTON_VAR} == {button} {}:goto *conv_settings_draw", all(0)));
    }
    w(format!("if {BUTTON_VAR} == {BTN_SKIP} mov {SKIP_UNREAD},1-{SKIP_UNREAD}"));
    for (r, row) in rows.iter().enumerate() {
        let level = setting_var(row.setting);
        let r = r as u32;
        w(format!("if {BUTTON_VAR} == {} && {level} > 0 sub {level},1", BTN_MINUS + r + 1));
        w(format!("if {BUTTON_VAR} == {} && {level} < 10 add {level},1", BTN_PLUS + r + 1));
    }
    w(format!("gosub *{SETTINGS_APPLY}"));
    w("goto *conv_settings_draw".into());
    // `reset` starts the script over: the title screen.
    for (label, question, then) in [("conv_settings_title", confirm.map(|c| &c.title), "reset"), ("conv_settings_quit", confirm.map(|c| &c.quit), "end")] {
        w(format!("*{label}"));
        if let Some(q) = question {
            w(confirm_call(q));
            w(format!("if {CONFIRM} == 0 goto *conv_settings_wait"));
        }
        w(then.into());
    }
    w("*conv_settings_close".into());
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
    if m.skip_unread_label.is_some() {
        w(format!("if {SKIP_UNREAD} == 0 kidokumode 1"));
        w(format!("if {SKIP_UNREAD} == 1 kidokumode 0"));
    }
    w("return".into());
}

/// Sets the question and asks it (see [`confirm`]).
pub(crate) fn confirm_call(question: &str) -> String {
    format!("mov {CONFIRM_TEXT},\"{}\"\ngosub *{CONFIRM_ROUTINE}", escape(question))
}

/// The yes/no dialog (see [`ConfirmDialog`]) over whatever is on screen:
/// asks [`CONFIRM_TEXT`], sets [`CONFIRM`] (1 yes, 0 no or right-click)
/// and takes itself away. The message is centred by its length in bytes
/// (Shift-JIS: half-width characters are one byte, half as wide).
pub(crate) fn confirm(out: &mut String, d: &ConfirmDialog) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let size = d.font_size;
    let label_size = d.yes.h as u32 * 2 / 3;
    let centre = d.base.x + d.base.w / 2;
    w(format!("*{CONFIRM_ROUTINE}"));
    w(format!("lsp {DIALOG_BASE},\":a;{}\",{},{}", path(&d.base.image), d.base.x, d.base.y));
    w(format!("len {TMP},{CONFIRM_TEXT}"));
    w(format!("mov {TMP},{centre}-{TMP}*{size}/4"));
    w(format!("strsp {DIALOG_TEXT},{CONFIRM_TEXT},{TMP},{},40,1,{size},{size},0,0,0,1", d.base.y + d.text_y));
    for (button, label_slot, p, label) in [(DIALOG_YES, DIALOG_YES_LABEL, &d.yes, &d.yes_label), (DIALOG_NO, DIALOG_NO_LABEL, &d.no, &d.no_label)] {
        w(format!("lsp {button},\":a/3,0,3;{}\",{},{}", path(&p.image), p.x, p.y));
        let chars = label.chars().count() as i32;
        let x = p.x + p.w / 6 - chars * label_size as i32 / 2;
        let y = p.y + (p.h - label_size as i32) / 2;
        w(format!("strsp {label_slot},\"{}\",{x},{y},10,1,{label_size},{label_size},0,0,0,1", escape(label)));
    }
    w("*conv_confirm_wait".into());
    w(format!("spbtn {DIALOG_YES},1:spbtn {DIALOG_NO},2"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("mov {CONFIRM},0"));
    w(format!("if {BUTTON_VAR} == 1 mov {CONFIRM},1"));
    w(format!("if {BUTTON_VAR} != 1 && {BUTTON_VAR} != 2 && {BUTTON_VAR} != -1 goto *conv_confirm_wait"));
    w(DIALOG_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("print 1".into());
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
///
/// Each text line is tagged `[name|voice]` with its page's speaker (when a
/// name plate shows it) and voice clip; the log reads the tags back with
/// `gettaglog`, shows the name above the page and replays the voice when
/// the name is clicked. A tag is only kept when there is a pretextgosub,
/// so [`PRETEXT`] is one that does nothing but allow the save point
/// (pretextgosub makes the text wait for `saveon`).
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
    w(format!("*{PRETEXT}"));
    w("saveon".into());
    w("return".into());
    w(format!("*{BACKLOG}"));
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(lsp2(BASE, &b.base, Some(b.base_opacity)));
    w(lsp2(FRAME, &b.frame, None));
    w(lsp2(HEADER, &b.track, None));
    w(button(LOG_UP, &b.up));
    w(button(LOG_DOWN, &b.down));
    w(button(CLOSE, &b.close));
    w(format!("mov {LOG_OFFSET},0"));
    w("*conv_log_draw".into());
    w((M..LOG_NAME_SPRITE + LOG_PAGES).map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w((0..LOG_PAGES).map(|k| format!("mov ${},\"\"", LOG_VOICES + k)).collect::<Vec<_>>().join(":"));
    w(format!("mov {LOG_Y},{}", a.y + a.h));
    w(format!("mov {LOG_K},0"));
    w("*conv_log_page".into());
    w(format!("if {LOG_K} >= {LOG_PAGES} goto *conv_log_shown"));
    w(format!("mov {TMP},{LOG_OFFSET}+{LOG_K}"));
    w(format!("getlog {LINE1},{TMP}"));
    w(format!("if {LINE1} == \"\" goto *conv_log_shown"));
    w(format!("getlogtext {LINE2},{TMP}"));
    w(format!("gettaglog {LOG_TAG},{TMP}"));
    w(format!("split {LOG_TAG},\"|\",{LOG_NAME},{LOG_VOICE}"));
    w(format!("mov {LOG_NAMED},1"));
    w(format!("if {LOG_NAME} == \"\" && {LOG_VOICE} == \"\" mov {LOG_NAMED},0"));
    w(format!("len {LOG_LINES},{LINE1}"));
    w(format!("len {TMP},{LINE2}"));
    w(format!("mov {LOG_LINES},{LOG_LINES}-{TMP}+1"));
    w(format!("mov {TMP},{LOG_Y}-({LOG_LINES}+{LOG_NAMED})*{}-{}", b.line_height, b.line_height / 2));
    // Doesn't fit: stop (but always show at least one page).
    w(format!("if {TMP} < {} && {LOG_K} > 0 goto *conv_log_shown", a.y));
    w(format!("if {TMP} < {} mov {TMP},{}", a.y, a.y));
    w(format!("mov {LOG_SPRITE},{LOG_K}+{M}"));
    w(format!("mov {LOG_TEXT_Y},{TMP}+{LOG_NAMED}*{}", b.line_height));
    w(format!("strsp {LOG_SPRITE},{LINE1},{},{LOG_TEXT_Y},{},{LOG_LINES},{size},{size},0,{gap},0,1", a.x, b.columns));
    w(format!("if {LOG_NAMED} == 1 gosub *conv_log_name"));
    w(format!("mov {LOG_Y},{TMP}"));
    w(format!("add {LOG_K},1"));
    w("goto *conv_log_page".into());
    // The name of page LOG_K at TMP; with a voice it's a button replaying it.
    w("*conv_log_name".into());
    w(format!("mov {LOG_SPRITE},{LOG_K}+{LOG_NAME_SPRITE}"));
    let name = |text: &str, colors: &str| {
        format!("strsp {LOG_SPRITE},{text},{},{TMP},{},1,{size},{size},0,{gap},0,1,{colors}", a.x, b.columns)
    };
    w(format!("if {LOG_VOICE} == \"\" {}:return", name(LOG_NAME, &format!("\"{LOG_NAME_COLOR}\""))));
    w(format!("add {LOG_NAME},\"　♪\""));
    w(name(LOG_NAME, &format!("\"{LOG_NAME_COLOR}\",\"#FFFFFF\"")));
    w(format!("mov {LOG_TEXT_Y},{LOG_VOICES}+{LOG_K}"));
    w(format!("mov ${LOG_TEXT_Y},{LOG_VOICE}"));
    w("return".into());
    w("*conv_log_shown".into());
    w(format!("cell {LOG_DOWN},0:cell {LOG_UP},0"));
    w(format!("if {LOG_OFFSET} == 0 cell {LOG_DOWN},2"));
    // LOG_OLDER: there is an older page to scroll to.
    w(format!("mov {TMP},{LOG_OFFSET}+1"));
    w(format!("getlog {LINE2},{TMP}"));
    w(format!("mov {LOG_OLDER},1"));
    w(format!("if {LINE2} == \"\" mov {LOG_OLDER},0:cell {LOG_UP},2"));
    w("*conv_log_wait".into());
    w(format!("if {LOG_OLDER} == 1 spbtn {LOG_UP},1"));
    w(format!("if {LOG_OFFSET} > 0 spbtn {LOG_DOWN},2"));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    for k in 0..LOG_PAGES {
        w(format!("if ${} != \"\" spbtn {},{}", LOG_VOICES + k, LOG_NAME_SPRITE + k, BTN_LOG_VOICE + k));
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *conv_log_close"));
    w(format!(
        "if {BUTTON_VAR} >= {BTN_LOG_VOICE} && {BUTTON_VAR} < {} mov {TMP},{BUTTON_VAR}-{BTN_LOG_VOICE}+{LOG_VOICES}:dwave 0,${TMP}:goto *conv_log_wait",
        BTN_LOG_VOICE + LOG_PAGES
    ));
    w(format!("if {BUTTON_VAR} == -2 mov {BUTTON_VAR},1"));
    w(format!("if {BUTTON_VAR} == -3 mov {BUTTON_VAR},2"));
    w(format!("if {BUTTON_VAR} == 1 && {LOG_OLDER} == 1 add {LOG_OFFSET},1:goto *conv_log_draw"));
    w(format!("if {BUTTON_VAR} == 2 && {LOG_OFFSET} == 0 goto *conv_log_close"));
    w(format!("if {BUTTON_VAR} == 2 sub {LOG_OFFSET},1:goto *conv_log_draw"));
    w("goto *conv_log_wait".into());
    w("*conv_log_close".into());
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
    use conv_core::ir::{PageButton, Rect};

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
        let d = dialog();
        let mut out = String::new();
        save_menu(&mut out, &m, Some(&d));
        for line in [
            "if %100 == 1 getscreenshot 400,300\n",
            // lsp2 takes the centre.
            "lsp2 11,\":a;system\\base.png\",410,320,100,100,0,140\n",
            // Row 2 (slot FIRST + 1): thumbnail at 25%, centred in its box.
            "if %104 == 1 lsp2 2,$105,64,183,25,25,0\n",
            "if %900 == 2 cell 22,2:lsp2 7,\":a;system\\marker.png\",214,484,100,100,0\n",
            "if %9 > 100 && %9 <= 102 mov %900,%9-100:goto *conv_sl_page\n",
            "savescreenshot2 $103\n",
            "savegame2 %102,$100\n",
            "loadgame %102\n",
            // Delete removes the save and its thumbnail; a slot an older
            // build hid with a flag is deleted when listed.
            "mov %103,%102\ngosub *conv_sl_file\ndelsavegame %102,$103\n",
            "if %%105 == 1 delsavegame %103,$103:mov %%105,0\nsavefileexist %104,%103\n",
            // Saving over a save asks first.
            "if %107 == 0 goto *conv_sl_save\nmov %103,%102\ngosub *conv_sl_number\nmov $106,$101\nadd $106,\" over?\"\ngosub *conv_confirm\nif %109 == 0 goto *conv_sl_buttons\n*conv_sl_save\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    fn dialog() -> ConfirmDialog {
        ConfirmDialog {
            base: Placed { x: 140, y: 254, ..placed("dialog_base", 520, 91) },
            yes: Placed { x: 242, y: 305, ..placed("dialog_button", 426, 28) },
            no: Placed { x: 416, y: 305, ..placed("dialog_button", 426, 28) },
            yes_label: "はい".into(),
            no_label: "いいえ".into(),
            text_y: 16,
            font_size: 20,
            overwrite: "over?".into(),
            load: "load?".into(),
            delete: "delete?".into(),
            title: "title?".into(),
            quit: "quit?".into(),
        }
    }

    #[test]
    fn confirm_routine() {
        let mut out = String::new();
        confirm(&mut out, &dialog());
        for line in [
            "lsp 5,\":a;system\\dialog_base.png\",140,254\n",
            // Centred by length: 400 - bytes * 20 / 4.
            "len %105,$106\nmov %105,400-%105*20/4\nstrsp 0,$106,%105,270,40,1,20,20,0,0,0,1\n",
            "lsp 3,\":a/3,0,3;system\\dialog_button.png\",242,305\nstrsp 1,\"はい\",295,310,10,1,18,18,0,0,0,1\n",
            "spbtn 3,1:spbtn 4,2\nprint 1\nbtnwait %9\nmov %109,0\nif %9 == 1 mov %109,1\n",
            "csp 0:csp 1:csp 2:csp 3:csp 4:csp 5\nprint 1\nreturn\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    #[test]
    fn settings_voice_page() {
        use conv_core::ir::SettingRow;
        let m = SettingsMenu {
            base: placed("base", 800, 600),
            base_opacity: 210,
            header: placed("header", 100, 30),
            close: placed("close", 60, 20),
            rows: vec![SettingRow { label: "ＢＧＭ".into(), setting: Setting::BgmVolume }],
            label_x: 140,
            minus_x: 340,
            level_x: 380,
            plus_x: 680,
            y: 140,
            pitch: 70,
            font_size: 22,
            title_label: "title".into(),
            quit_label: "quit".into(),
            exits_y: 540,
            tab_labels: ["音量".into(), "音声".into()],
            tabs_x: 300,
            tabs_y: 34,
            voices: vec![
                VoiceToggle { label: "あずさ".into(), characters: vec![3] },
                VoiceToggle { label: "その他".into(), characters: vec![0, 2] },
            ],
            voice_column_x: 440,
            voice_pitch: 50,
            skip_unread_label: Some("未読".into()),
        };
        let mut out = String::new();
        settings_menu(&mut out, &m, None);
        for line in [
            // The current tab can't be clicked; the other one can.
            "if %176 != 1 strsp 12,\"音声\",392,34,3,1,22,22,0,0,0,1,\"#FFFFFF\",\"#F0C040\"\n",
            "if %176 == 0 strsp 11,\"音量\",300,34,3,1,22,22,0,0,0,1,\"#F0C040\"\n",
            "if %176 == 1 goto *conv_settings_draw_voices\n",
            // One per column: the second switch is in the second column.
            "mov $104,\"■　\"\nif %973 == 1 mov $104,\"□　\"\nadd $104,\"あずさ\"\nstrsp 13,$104,140,140,6,1,22,22,0,0,0,1,\"#FFFFFF\",\"#F0C040\"\n",
            "strsp 14,$104,440,140,",
            "*conv_settings_wait_voices\nspbtn 13,51\nspbtn 14,52\nspbtn 11,41\n",
            "if %9 > 40 && %9 <= 42 mov %176,%9-41:goto *conv_settings_draw\n",
            // Skip unread text: under the levels, applied as kidokumode.
            "if %908 == 1 mov $104,\"■　\"\nadd $104,\"未読\"\nstrsp 14,$104,140,210,5,1,",
            "if %9 == 32 mov %908,1-%908\n",
            "if %908 == 0 kidokumode 1\nif %908 == 1 kidokumode 0\nreturn\n",
            // A switch sets all its characters.
            "if %9 == 52 && %970 == 0 mov %970,1:mov %972,1:goto *conv_settings_draw\nif %9 == 52 mov %970,0:mov %972,0:goto *conv_settings_draw\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    #[test]
    fn button_bar_routine() {
        use conv_core::ir::BarButton;
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
            "*conv_wait\ngosub *conv_bar_draw\ntextbtnwait %9\nif %9 > 2000 goto *conv_wait_hover\n",
            "if %9 >= 0 && %9 <= 1000 goto *conv_wait_end\n",
            "if %9 == 1001 goto *conv_wait_end\ngoto *conv_wait\n",
            "*conv_wait_end\ntexec\nreturn\n",
            // Hotspots: the hide one first (tested last), the bottom one
            // while the bar is down.
            "btndef clear\ngosub *conv_bar_clear\n\
             if %160 == 1 || %161 == 1 lsp 38,\":a;system\\hide.png\",10,20:spbtn 38,2003\n\
             if %161 == 0 lsp 37,\":a;system\\reveal.png\",10,20:spbtn 37,2002:goto *conv_bar_draw_top\n",
            "lsp 34,\":a/3,0,3;system\\auto.png\",10,20\nif %108 == 2 lsp 34,\":a/3,0,3;system\\auto_on.png\",10,20\nspbtn 34,1001\n",
            "lsp 36,\":a;system\\setting.png\",10,20\n*conv_bar_draw_top\n*conv_bar_draw_end\ngetmouseover 2001,2003\nprint 1\n",
            "*conv_bar_hover\nif %9 == 2001 mov %160,1\nif %9 == 2002 mov %161,1\nif %9 == 2003 mov %160,0:mov %161,0\nreturn\n",
            "if %9 == -1 goto *conv_bar_hide\nif %9 == -2 goto *conv_bar_log\nif %9 == 1001 goto *conv_bar_auto\nif %9 == 1002 goto *conv_bar_save\nreturn\n",
            "*conv_bar_log\nsystemcall lookback\nreturn\n",
            "*conv_bar_save\nmov %160,0:mov %161,0\ngosub *conv_bar_clear\nmov %101,1\ngosub *conv_save\nreturn\n",
            "*conv_bar_hide\nsystemcall windowerase\nreturn\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }
}
