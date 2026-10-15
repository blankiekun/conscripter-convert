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

use conv_core::ir::{BarAction, Backlog, ButtonBar, CommandMenu, ConfirmDialog, Placed, SaveMenu, Setting, SettingsMenu, Switch, TextSpot, TopBars, VoiceToggle, WaitIcon};

use crate::{escape, path, BAR_SPRITES, BUTTON_VAR, DIALOG_SPRITES, LINK_SPRITES, MENU_SPRITES, NAME_PLATE_SPRITE, NAME_TEXT_SPRITE};

pub(crate) const SAVE: &str = "conv_save";
/// The `textgosub` routine.
pub(crate) const WAIT: &str = "conv_wait";
/// Shows the button bar and makes its buttons clickable.
pub(crate) const BAR_DRAW: &str = "conv_bar_draw";
/// Runs the bar button in %9 (or -1, a right-click).
pub(crate) const BAR_ACTION: &str = "conv_bar_action";
pub(crate) const SETTINGS: &str = "conv_settings";
pub(crate) const SETTINGS_APPLY: &str = "conv_settings_apply";
pub(crate) const BACKLOG: &str = "conv_backlog";
/// The `pretextgosub` routine (see [`backlog`]).
pub(crate) const PRETEXT: &str = "conv_pretext";
/// Settings levels are %902-%906 (globals); %909 = 1 once they are set.
pub(crate) const SETTINGS_SET: &str = "%909";
/// 1: skip was on at a choice and goes on at the next text
/// ([`Switch::StopSkipAtChoice`] off).
pub(crate) const KEEP_SKIP: &str = "%131";
/// 1: skip goes on through unread text (`kidokumode 0`).
pub(crate) const SKIP_UNREAD: &str = "%908";
/// Milliseconds per character, for `setwindow`.
pub(crate) const TEXT_MS: &str = "%907";
const TEXT_MS_STEP: u32 = 6;
// Settings and backlog sprites (lsp, within MENU_SPRITES).
/// A level row's label and level, one text sprite.
const SETTING_ROW: u32 = M; // + row
const SETTING_TAB: u32 = M + 21; // + page
const SETTING_SKIP: u32 = M + 8;
const SETTING_MINUS: u32 = M + 9; // + row
const SETTING_PLUS: u32 = M + 15; // + row
const SETTING_TITLE: u32 = M + 25;
const SETTING_QUIT: u32 = M + 26;
/// The build's version line, bottom left.
const SETTING_VERSION: u32 = M + 24;
const MAX_SETTING_ROWS: usize = 6;
/// Voice switches, on the voice page (in place of the level rows), and the
/// switches page's options.
const SETTING_VOICE: u32 = M + 6; // + switch
const MAX_VOICES: usize = 15;
const SETTING_SWITCH: u32 = M + 6; // + switch
const MAX_SWITCHES: usize = 15;
const BTN_SWITCH: u32 = 70; // + 1 + switch
// The switches' flags: see `switches.rs`.
/// The confirmation being asked (a [`Switch`] + 1; 0 = always ask).
pub(crate) const CONFIRM_KIND: &str = "%125";
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
pub(crate) const SETTINGS_PAGE: &str = "%176";
/// Voice character c is muted when the global %(VOICE_OFF_BASE + c) is 1.
pub(crate) const VOICE_OFF_BASE: u32 = 970;
/// Asks the question in [`CONFIRM_TEXT`]; [`CONFIRM`] = 1 for yes.
pub(crate) const CONFIRM_ROUTINE: &str = "conv_confirm";
pub(crate) const CONFIRM: &str = "%109";
pub(crate) const CONFIRM_TEXT: &str = "$106";
/// Bar buttons return numbers above this.
pub(crate) const BAR_BUTTON_BASE: i32 = 1000;
pub(crate) const BAR_CLEAR: &str = "conv_bar_clear";
/// Sets [`NO_QUICK_SAVE`] and [`NO_BACK_SAVE`].
pub(crate) const BAR_STATE: &str = "conv_bar_state";
pub(crate) const NO_QUICK_SAVE: &str = "%141";
pub(crate) const NO_BACK_SAVE: &str = "%142";
/// A sliding bar ([`BarSlide`]): 1 while it is out; its x offset now; the
/// slide's progress (1000 to 0); the pointer's y.
pub(crate) const BAR_OUT: &str = "%143";
const BAR_OFFSET: &str = "%144";
const SLIDE_K: &str = "%145";
const MOUSE_Y: &str = "%146";
pub(crate) const BAR_SLIDE_IN: &str = "conv_bar_slide_in";
pub(crate) const BAR_SLIDE_OUT: &str = "conv_bar_slide_out";
/// The sliding bar's hotspot (a `btn`, see [`button_bar`]), also the
/// unpinned bar's ([`BarAction::Lock`]).
const BTN_EDGE: i32 = BAR_BUTTON_BASE + 300;
/// 1 while the bars are unpinned ([`BarAction::Lock`]; global).
const BAR_UNPINNED: &str = "%2908";
/// 1 while the unpinned bar is shown: it fades in only when it was hidden.
const BAR_SHOWN: &str = "%148";
/// The game saved before the last choice ([`BarAction::BackToChoice`]);
/// a global that is 1 while it is being loaded (see [`LOADED`]).
pub(crate) const BACK_SLOT: u32 = 891;
const BACK_LOADING: &str = "%2904";
/// The `loadgosub` routine when the game can go back to a choice.
pub(crate) const LOADED: &str = "conv_loaded";
/// Puts the bars away (the next click wait draws them again).
pub(crate) const BAR_HIDE: &str = "conv_bar_hide_all";
/// The link scene level (0 = the root); the deepest level that can be
/// gone forward to; titles at $(SCENE_TITLES + level).
pub(crate) const SCENE_DEPTH: &str = "%150";
pub(crate) const SCENE_MAX_DEPTH: &str = "%151";
pub(crate) const SCENE_TITLES: u32 = 160;
const SCENE_TMP: &str = "$151";
/// Levels below this one can't be gone back to: their snapshots belong to
/// another playthrough (it is the level a save was loaded at).
pub(crate) const SCENE_FLOOR: &str = "%190";
/// Leaving a level saves the game to save slot SCENE_SLOT + level; going to
/// a level loads its slot (see [`scene_stack`]). The user's slots are 1-50.
pub(crate) const SCENE_SLOT: u32 = 900;
/// The deepest level the list shows (the game nests five).
const SCENE_ROWS: u32 = 6;
const SCENE_SHIFT: &str = "%191"; // how far the list pushes the level row down
const SCENE_TARGET: &str = "%192";
/// Globals carrying the history over a snapshot load: 1 while one is
/// loaded, the deepest level, the floor, the titles from $SCENE_CARRY + 1.
const SCENE_LOADING: &str = "%2900";
const SCENE_CARRY_MAX: &str = "%2901";
const SCENE_CARRY_FLOOR: &str = "%2902";
const SCENE_CARRY: u32 = 2900;
/// 1 while the list is open (global: it stays open).
const SCENE_LIST_OPEN: &str = "%2903";
/// The save menu's slot being loaded (global: it outlasts the load).
pub(crate) const SCENE_LOAD_SLOT: &str = "%2906";
/// 1 once [`SCENE_LOADED`] has put a save's levels back.
const SCENE_RESTORED: &str = "%2907";
/// A save from the save menu keeps the levels' slots with it, level k of
/// slot n in save slot SCENE_KEPT + 10 × n + k (levels 0-7: link scenes
/// nest at most 7 deep), so a load can still go back and forward.
const SCENE_KEPT: u32 = 1000;
const SCENE_LEVELS: u32 = 8;
/// The `loadgosub` routine.
pub(crate) const SCENE_LOADED: &str = "conv_scene_loaded";
/// List rows: title sprites at SCENE_ROW_SPRITE + 2 × row, their buttons
/// one above (menu sprites the link underlines leave free; the bars are
/// cleared before a menu uses them).
const SCENE_ROW_SPRITE: u32 = M + 16;
const BTN_SCENE_BACK: i32 = BAR_BUTTON_BASE + 101;
const BTN_SCENE_FORWARD: i32 = BAR_BUTTON_BASE + 102;
const BTN_SCENE_EXIT: i32 = BAR_BUTTON_BASE + 103;
const BTN_SCENE_LIST: i32 = BAR_BUTTON_BASE + 104;
const BTN_SCENE_ROW: i32 = BAR_BUTTON_BASE + 110; // + level
/// lsp2 slots of the top bars (below the save menu's).
const TOP_SPRITES2: std::ops::Range<u32> = 12..24;
pub(crate) const LOAD: &str = "conv_load";
/// The quick save's slot (the user's slots are 1-100, the link scenes'
/// from [`SCENE_SLOT`]).
pub(crate) const QUICK_SLOT: u32 = 890;
/// 1 when entered from the game (hide and restore the text window).
pub(crate) const IN_GAME: &str = "%101";

// Local scratch variables (below the game's locals at %10-%12 would be too
// close; these are free).
pub(crate) const MODE: &str = "%100"; // 1 save, 2 load
pub(crate) const SELECTED: &str = "%102"; // selected slot, 0 = none
pub(crate) const SLOT: &str = "%103";
pub(crate) const HAS_DATA: &str = "%104";
pub(crate) const TMP: &str = "%105";
pub(crate) const FIRST: &str = "%106"; // first slot on the page
const SELECTED_HAS_DATA: &str = "%107";
const MODE_STATE: &str = "%108"; // isskip: 1 skip, 2 auto
pub(crate) const DATE: [&str; 6] = ["%110", "%111", "%112", "%113", "%114", "%115"];
pub(crate) const LABEL: &str = "$100";
const LINE1: &str = "$101";
const LINE2: &str = "$102";
pub(crate) const FILE: &str = "$103"; // thumbnail file
pub(crate) const THUMB_IMAGE: &str = "$105"; // the same with its sprite tag
pub(crate) const STR_TMP: &str = "$104";
// Globals (`globalon` keeps %200 and up; the game uses up to %220).
pub(crate) const PAGE: &str = "%900";
pub(crate) const NEWEST: &str = "%901";
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
pub(crate) const BASE: u32 = 11;
// Button numbers returned by btnwait: rows 1.., pages 101..
const BTN_PAGE: u32 = 100;
const BTN_DELETE: u32 = 200;
const BTN_ACTION: u32 = 201;
const BTN_CLOSE: u32 = 202;

/// `scene_stack`: the game has link scene levels ([`scene_stack`]), whose
/// slots a save keeps.
pub(crate) fn save_menu(out: &mut String, m: &SaveMenu, confirm: Option<&ConfirmDialog>, scene_stack: bool) {
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
    let backdrop_on = m.backdrop.map(|(slot, f)| format!("screenfilter {slot},{}", f.map(|n| n.to_string()).join(",")));
    let backdrop_off = m.backdrop.map(|(slot, _)| format!("screenfilter {slot},0"));
    if let Some(on) = &backdrop_on {
        w(format!("if {IN_GAME} == 1 {on}"));
    }
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
        let cols = if m.text_width > 0 { (m.text_width / size.max(1)).max(1) } else { 40 };
        for (slot, line, dy) in [(TEXT1, LINE1, 0), (TEXT2, LINE2, m.line_height)] {
            w(format!(
                "strsp {},{line},{},{},{cols},1,{size},{size},0,0,0,1",
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
    // (Switched off, no picture: the flag is 0 in a game without the switch.)
    w(format!("if {HAS_DATA} == 1 && {} == 0 lsp2 {PREVIEW},{THUMB_IMAGE},{px},{py},100,100,0", switch_off(Switch::SavePreview)));
    w(format!("if {HAS_DATA} == 0 csp2 {PREVIEW}"));
    w("goto *conv_sl_buttons".into());

    // Asks "No.NN {question}" about the selected slot; no: back to waiting.
    let ask = |question: &str, kind: Switch| match confirm {
        Some(_) => format!(
            "mov {SLOT},{SELECTED}\ngosub *conv_sl_number\nmov {CONFIRM_KIND},{}:mov {CONFIRM_TEXT},{LINE1}\nadd {CONFIRM_TEXT},\" {}\"\ngosub *{CONFIRM_ROUTINE}\nif {CONFIRM} == 0 goto *conv_sl_buttons",
            switch_index(kind) + 1,
            escape(question)
        ),
        None => String::new(),
    };
    w("*conv_sl_act".into());
    w(format!("if {SELECTED} == 0 goto *conv_sl_wait"));
    w(format!("if {MODE} == 2 goto *conv_sl_load"));
    if let Some(c) = confirm {
        w(format!("if {SELECTED_HAS_DATA} == 0 goto *conv_sl_save"));
        w(ask(&c.overwrite, Switch::ConfirmSave));
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
    match (&backdrop_off, &backdrop_on) {
        (Some(off), Some(on)) => w(format!("{off}:savegame2 {SELECTED},{LABEL}:{on}")),
        _ => w(format!("savegame2 {SELECTED},{LABEL}")),
    }
    if scene_stack {
        w("gosub *conv_scene_keep".into());
    }
    w(format!("mov {NEWEST},{SELECTED}"));
    w("goto *conv_sl_page".into());

    w("*conv_sl_load".into());
    w(format!("if {SELECTED_HAS_DATA} == 0 goto *conv_sl_wait"));
    if let Some(c) = confirm {
        w(ask(&c.load, Switch::ConfirmLoad));
    }
    w("gosub *conv_sl_clear".into());
    if scene_stack {
        w(format!("mov {SCENE_LOAD_SLOT},{SELECTED}"));
    }
    w(format!("loadgame {SELECTED}"));
    // Still here: the load failed.
    w("goto *conv_sl".into());

    w("*conv_sl_delete".into());
    if let Some(c) = confirm {
        w(ask(&c.delete, Switch::ConfirmDelete));
    }
    w(format!("mov {SLOT},{SELECTED}"));
    w("gosub *conv_sl_file".into());
    w(format!("delsavegame {SELECTED},{FILE}"));
    if scene_stack {
        w("gosub *conv_scene_drop".into());
    }
    w(format!("if {SELECTED} == {NEWEST} mov {NEWEST},0"));
    w("goto *conv_sl_page".into());

    w("*conv_sl_close".into());
    // texton first: it drops the pending screen updates except the window's.
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    if let Some(off) = &backdrop_off {
        w(format!("if {IN_GAME} == 1 {off}"));
    }
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

/// What the bar's actions can open, and the screen's size.
#[derive(Clone, Copy, Default)]
pub(crate) struct BarScreens<'a> {
    pub save_menu: bool,
    pub settings: bool,
    pub backlog: bool,
    /// There is a scene replay, during which save and load are left out.
    pub replay: bool,
    pub command_menu: Option<&'a CommandMenu>,
    pub confirm: Option<&'a ConfirmDialog>,
    pub width: u32,
    pub height: u32,
    /// The settings have [`Switch::VoiceKeepsPlaying`].
    pub voice_stops: bool,
    /// The click-wait icon and the variables holding its top-left corner.
    pub wait_icon: Option<(&'a WaitIcon, [&'static str; 2])>,
    /// The variable holding the story's `gamespeed` percent: the click wait
    /// (and the menus opened from it) run at 100, the story at that.
    pub game_speed: Option<&'static str>,
}

/// The routine that runs a bar action (`gosub` it).
pub(crate) fn bar_label(a: BarAction) -> &'static str {
    match a {
        BarAction::Auto => "conv_bar_auto",
        BarAction::Skip => "conv_bar_skip",
        BarAction::Log => "conv_bar_log",
        BarAction::Save => "conv_bar_save",
        BarAction::Load => "conv_bar_load",
        BarAction::Settings => "conv_bar_settings",
        BarAction::HideWindow | BarAction::Unavailable => "conv_bar_hide",
        BarAction::QuickSave => "conv_bar_qsave",
        BarAction::QuickLoad => "conv_bar_qload",
        BarAction::Title => "conv_bar_title",
        BarAction::Quit => "conv_bar_quit",
        BarAction::Fullscreen => "conv_bar_full",
        BarAction::FastSkip => "conv_bar_fskip",
        BarAction::BackToChoice => "conv_bar_back",
        BarAction::Lock { .. } => "conv_bar_lock",
        // The command menu's own.
        BarAction::Close | BarAction::Call => "",
    }
}

/// Actions after which the click wait goes on to the next page.
pub(crate) fn goes_on(a: BarAction) -> bool {
    matches!(a, BarAction::Auto | BarAction::Skip | BarAction::FastSkip)
}

/// The variable that is 1 while `a` can't be run (no quick save, no save
/// before a choice), set by [`BAR_STATE`]; `None`: always available.
pub(crate) fn disabled_while(a: BarAction) -> Option<&'static str> {
    match a {
        BarAction::QuickLoad => Some(NO_QUICK_SAVE),
        BarAction::BackToChoice => Some(NO_BACK_SAVE),
        _ => None,
    }
}

/// The value CONscripter's `getkey` makes `btnwait` return for a letter.
pub(crate) fn key_code(c: char) -> i32 {
    -1000 - c.to_ascii_uppercase() as i32
}

/// The click-wait routine (`textgosub`): shows the bars the pointer has
/// revealed, waits, runs the clicked button's action, and ends the wait
/// with `texec`. Link pages call [`BAR_DRAW`], [`BAR_HOVER`] and
/// [`BAR_ACTION`] around their own `btnwait`.
///
/// Bar buttons return [`BAR_BUTTON_BASE`] + 1 + index; a right-click (-1)
/// hides the text window, or opens the [`CommandMenu`] when there is one
/// (as does its key; the items' keys run their actions). `textbtnwait`
/// returns at once in skip mode and after the delay in auto mode; a
/// click turns auto mode off.
///
/// Bars without a [`BarSlide`] are always up during a click wait. A bar
/// with one is drawn off screen; an invisible hotspot (`btn`) beyond its
/// edge, with `getmouseover`, ends the wait when the pointer comes onto
/// it, and the bar slides in; the hotspot then covers the rest of the
/// screen, and the bar slides out when the pointer goes back.
pub(crate) fn button_bar(out: &mut String, bar: &ButtonBar, s: BarScreens) {
    let mut slots = BAR_SPRITES;
    // The base is drawn over the buttons (a lower number).
    let base_slot = if bar.base.is_some() { slots.next().unwrap() } else { 0 };
    let buttons: Vec<_> = bar.buttons.iter().zip(slots.by_ref()).collect();
    let (chapter_text, scene_text) = (slots.next().unwrap(), slots.next().unwrap());
    // Back, forward, exit, list.
    let scene_buttons: [u32; 4] = std::array::from_fn(|_| slots.next().unwrap());
    // An invisible button over the chapter bar (see [`ChapterHistory`]).
    let chapters_slot = slots.next();
    // The click-wait icon: over the text window (below humanz), cleared
    // with the bar.
    let icon_slot = s.wait_icon.map(|_| slots.next().expect("no bar sprite left for the click-wait icon"));
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let slide = bar.slide;
    // Where an unpinned bar shows ([`BarAction::Lock`]).
    let reveal = bar.buttons.iter().find_map(|b| match b.action {
        BarAction::Lock { reveal_y, fade_ms } => Some((reveal_y, fade_ms)),
        _ => None,
    });
    // A sliding bar's x is offset by BAR_OFFSET.
    let x = |p: &Placed| if slide.is_some() { format!("{}+{BAR_OFFSET}", p.x) } else { p.x.to_string() };
    let lsp = |slot: u32, p: &Placed, cells: &str| format!("lsp {slot},\":a{cells};{}\",{},{}", path(&p.image), x(p), p.y);
    let cells = if bar.pressed { "/4,0,3" } else { "/3,0,3" };
    let press = if bar.pressed { ",3" } else { "" };
    let number = |i: usize| BAR_BUTTON_BASE + 1 + i as i32;
    let menu = s.command_menu;
    let menu_items = menu.map_or(&[][..], |m| &m.items[..]);
    let keys = menu.is_some_and(|m| m.key.is_some() || m.items.iter().any(|i| i.key.is_some()));

    w(format!("*{WAIT}"));
    if s.game_speed.is_some() {
        w("gamespeed 100".into());
    }
    if menu.is_some() {
        w(format!("mov {},0", crate::command_menu::GO_ON));
    }
    w(format!("gosub *{BAR_DRAW}"));
    // The icon (drawing the bar cleared it).
    if let (Some((i, [x, y])), Some(slot)) = (s.wait_icon, icon_slot) {
        let ms: Vec<String> = i.frame_ms.iter().map(u32::to_string).collect();
        w(format!("if {x} > 0 || {y} > 0 lsp {slot},\":a/{},<{}>,0;{}\",{x},{y}:print 1", ms.len(), ms.join(","), path(&i.image)));
    }
    if let Some(sl) = slide {
        // The pointer may have crossed the edge since the bar was drawn. Off
        // the screen (no pointer: auto-play reports the far corner) is away.
        w(format!("getmousepos {TMP},{MOUSE_Y}"));
        w(format!("if {TMP} >= {} || {MOUSE_Y} >= {} mov {TMP},0", s.width, s.height));
        w(format!("if {BAR_OUT} == 0 && {TMP} > {} gosub *{BAR_SLIDE_IN}:goto *{WAIT}", sl.edge_x));
        w(format!("if {BAR_OUT} == 1 && {TMP} <= {} gosub *{BAR_SLIDE_OUT}:goto *{WAIT}", sl.edge_x));
        w(format!("getmouseover {BTN_EDGE},{BTN_EDGE}"));
    }
    if reveal.is_some() {
        // Unpinned: the pointer crossing the line redraws the bar.
        w(format!("if {BAR_UNPINNED} == 1 getmouseover {BTN_EDGE},{BTN_EDGE}"));
    }
    if keys {
        w("getkey".into());
    }
    w(format!("textbtnwait {BUTTON_VAR}"));
    if slide.is_some() || reveal.is_some() {
        w(format!("if {BUTTON_VAR} == {BTN_EDGE} goto *{WAIT}"));
    }
    // Wheel down (-3) reads on, like a click; wheel up (-2) opens the log.
    w(format!("if {BUTTON_VAR} == -3 && {} == 0 goto *conv_wait_end", switch_off(Switch::WheelReadsOn)));
    w(format!("if {BUTTON_VAR} == -3 goto *{WAIT}"));
    if slide.is_some() {
        // While the bar is out a click (not auto mode's time) does nothing.
        w(format!("isskip {MODE_STATE}"));
        w(format!("if {BUTTON_VAR} == 0 && {BAR_OUT} == 1 && {MODE_STATE} == 0 goto *{WAIT}"));
    }
    w(format!("if {BUTTON_VAR} >= 0 && {BUTTON_VAR} <= {BAR_BUTTON_BASE} goto *conv_wait_end"));
    w(format!("gosub *{BAR_ACTION}"));
    if menu.is_some() {
        w(format!("if {} == 1 goto *conv_wait_end", crate::command_menu::GO_ON));
    }
    // Auto and skip go on to the next page; the rest come back here.
    for (i, (b, _)) in buttons.iter().enumerate() {
        if goes_on(b.action) {
            w(format!("if {BUTTON_VAR} == {} goto *conv_wait_end", number(i)));
        }
    }
    w(format!("goto *{WAIT}"));
    w("*conv_wait_end".into());
    if let Some(slot) = icon_slot {
        w(format!("csp {slot}"));
    }
    if s.voice_stops {
        // Voices play on channel 0.
        w(format!("if {} == 1 dwavestop 0", switch_off(Switch::VoiceKeepsPlaying)));
    }
    if let Some(v) = s.game_speed {
        w(format!("gamespeed {v}"));
    }
    w("texec".into());
    w("return".into());

    // Removes every bar sprite.
    w(format!("*{BAR_HIDE}"));
    w(format!("gosub *{BAR_CLEAR}"));
    w("print 1".into());
    w("return".into());
    w(format!("*{BAR_CLEAR}"));
    w(BAR_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));

    if bar.top.is_some() {
        w(scene_row_sprites().map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    }
    w(TOP_SPRITES2.map(|n| format!("csp2 {n}")).collect::<Vec<_>>().join(":"));
    w("return".into());

    let all_actions: Vec<BarAction> = bar.buttons.iter().map(|b| b.action).chain(menu_items.iter().map(|i| i.action)).collect();
    let state = all_actions.iter().any(|&a| disabled_while(a).is_some());
    if state {
        w(format!("*{BAR_STATE}"));
        w(format!("savefileexist {TMP},{QUICK_SLOT}:mov {NO_QUICK_SAVE},1-{TMP}"));
        w(format!("savefileexist {TMP},{BACK_SLOT}:mov {NO_BACK_SAVE},1-{TMP}"));
        w("return".into());
    }

    // Draws the bars and registers their buttons; link words are
    // registered after this.
    w(format!("*{BAR_DRAW}"));
    w("btndef clear".into());
    w(format!("gosub *{BAR_CLEAR}"));
    w(format!("isskip {MODE_STATE}"));
    if let Some(sl) = slide {
        w(format!("mov {BAR_OFFSET},{}:if {BAR_OUT} == 1 mov {BAR_OFFSET},0", sl.hidden_dx));
    }
    if let Some((y, _)) = reveal {
        // Unpinned and the pointer above the line: only the hotspot below
        // it. Off the screen (auto-play) is below.
        w(format!("getmousepos {TMP},{MOUSE_Y}"));
        w(format!("if {BAR_UNPINNED} == 1 && {MOUSE_Y} < {y} btn {BTN_EDGE},0,{y},{},{},0,0:mov {BAR_SHOWN},0:print 1:return", s.width, s.height as i32 - y));
    }
    if state {
        w(format!("gosub *{BAR_STATE}"));
    }
    // Only a bar that is out has buttons.
    let only_out = if slide.is_some() { format!("if {BAR_OUT} == 1 ") } else { String::new() };
    for (i, (b, slot)) in buttons.iter().enumerate() {
        if b.action == BarAction::Unavailable {
            w(lsp(*slot, &b.image, ""));
            continue;
        }
        if s.replay && matches!(b.action, BarAction::Save | BarAction::Load) {
            w(format!("if {} == 0 {}:spbtn {slot},{}", crate::replay::REPLAYING, lsp(*slot, &b.image, cells), number(i)));
            continue;
        }
        w(lsp(*slot, &b.image, cells));
        if let Some(active) = &b.active {
            let on = match b.action {
                BarAction::Auto => format!("{MODE_STATE} == 2"),
                BarAction::Lock { .. } => format!("{BAR_UNPINNED} == 1"),
                _ => format!("{MODE_STATE} == 1"),
            };
            w(format!("if {on} {}", lsp(*slot, active, cells)));
        }
        match disabled_while(b.action) {
            Some(off) => w(format!("if {off} == 1 cell {slot},2\n{only_out}if {off} == 0 spbtn {slot},{}{press}", number(i))),
            None => w(format!("{only_out}spbtn {slot},{}{press}", number(i))),
        }
    }
    if let Some(base) = &bar.base {
        w(lsp(base_slot, base, ""));
    }
    if let Some(sl) = slide {
        // The hotspot: past the edge while the bar is in, the rest while out.
        let (edge, width, height) = (sl.edge_x + 1, s.width as i32, s.height as i32);
        w(format!("if {BAR_OUT} == 0 btn {BTN_EDGE},{edge},0,{},{height},0,0", width - edge));
        w(format!("if {BAR_OUT} == 1 btn {BTN_EDGE},0,0,{edge},{height},0,0"));
    }
    if let Some((y, fade_ms)) = reveal {
        // The hotspot above the bar ends the wait when the pointer leaves.
        w(format!("if {BAR_UNPINNED} == 1 btn {BTN_EDGE},0,0,{},{y},0,0", s.width));
        // Coming down to a hidden bar, it fades in.
        let fades: Vec<String> = buttons.iter().map(|(_, slot)| *slot).chain(bar.base.as_ref().map(|_| base_slot)).map(|n| format!("spfade {n},{fade_ms}")).collect();
        if fade_ms > 0 && !fades.is_empty() {
            w(format!("if {BAR_UNPINNED} == 1 && {BAR_SHOWN} == 0 {}", fades.join(":")));
        }
        w(format!("mov {BAR_SHOWN},1"));
    }
    if let Some(top) = &bar.top {
        // Unpinned, the top bars stay hidden.
        if reveal.is_some() {
            w(format!("if {BAR_UNPINNED} == 1 goto *conv_bar_draw_end"));
        }
        scene_bar_draw(&mut w, top, chapter_text, scene_text, scene_buttons);
        if let (Some(_), Some(slot)) = (&top.chapter_history, chapters_slot) {
            w(crate::chapters::hotspot(top, slot));
        }
    }
    if reveal.is_some() {
        w("*conv_bar_draw_end".into());
    }
    w("print 1".into());
    w("return".into());

    if let Some(sl) = slide {
        // Eases out (cubic): the offset left is dx·(1 - t)³.
        let (dx, ms) = (sl.hidden_dx, sl.ms.max(1));
        w(format!("*{BAR_SLIDE_IN}"));
        w(format!("mov {BAR_OUT},1:goto *conv_bar_slide"));
        w(format!("*{BAR_SLIDE_OUT}"));
        w(format!("mov {BAR_OUT},0"));
        w("*conv_bar_slide".into());
        w("btndef clear".into());
        w("resettimer".into());
        w("*conv_bar_slide_loop".into());
        w(format!("gettimer {TMP}:if {TMP} > {ms} mov {TMP},{ms}"));
        w(format!("mov {SLIDE_K},({ms}-{TMP})*1000/{ms}"));
        w(format!("mov {BAR_OFFSET},{dx}*{SLIDE_K}*{SLIDE_K}/1000*{SLIDE_K}/1000000"));
        w(format!("if {BAR_OUT} == 0 mov {BAR_OFFSET},{dx}-{BAR_OFFSET}"));
        for (b, slot) in &buttons {
            w(format!("amsp {slot},{},{}", x(&b.image), b.image.y));
        }
        if let Some(base) = &bar.base {
            w(format!("amsp {base_slot},{},{}", x(base), base.y));
        }
        w("print 1".into());
        w(format!("if {TMP} < {ms} wait 1:goto *conv_bar_slide_loop"));
        w("return".into());
    }

    w(format!("*{BAR_ACTION}"));
    let right_click = if menu.is_some() { crate::command_menu::MENU } else { "conv_bar_hide" };
    w(format!("if {BUTTON_VAR} == -1 goto *{right_click}"));
    if let Some(k) = menu.and_then(|m| m.key) {
        w(format!("if {BUTTON_VAR} == {} goto *{right_click}", key_code(k)));
    }
    for (i, item) in menu_items.iter().enumerate() {
        let Some(k) = item.key else { continue };
        let go = if goes_on(item.action) { format!("mov {},1:", crate::command_menu::GO_ON) } else { String::new() };
        let call;
        let target = match item.action {
            BarAction::Call if item.label.is_some() => {
                call = crate::command_menu::call_label(i);
                call.as_str()
            }
            BarAction::Call | BarAction::Close => continue,
            a => bar_label(a),
        };
        w(format!("if {BUTTON_VAR} == {} {go}goto *{target}", key_code(k)));
    }
    w(format!("if {BUTTON_VAR} == -2 goto *conv_bar_log"));
    if bar.top.is_some() {
        let last = BTN_SCENE_ROW + SCENE_ROWS as i32;
        w(format!("if {BUTTON_VAR} >= {BTN_SCENE_BACK} && {BUTTON_VAR} <= {last} goto *conv_scene_action"));
        if bar.top.as_ref().is_some_and(|t| t.chapter_history.is_some()) {
            w(format!("if {BUTTON_VAR} == {} goto *{}", crate::chapters::BTN_OPEN, crate::chapters::OPEN));
        }
    }
    for (i, (b, _)) in buttons.iter().enumerate() {
        if b.action != BarAction::Unavailable {
            w(format!("if {BUTTON_VAR} == {} goto *{}", number(i), bar_label(b.action)));
        }
    }
    w("return".into());
    let mut actions = all_actions;
    actions.push(BarAction::HideWindow);
    actions.push(BarAction::Log);
    actions.retain(|&a| !bar_label(a).is_empty());
    actions.sort_by_key(|a| bar_label(*a));
    actions.dedup_by_key(|a| bar_label(*a));
    // The save menu covers the screen: put the bars away first.
    let menu_entry = |entry: &str| format!("gosub *{BAR_CLEAR}\nmov {IN_GAME},1\ngosub *{entry}");
    // Fades to black (the dialog's top sprite) before a load.
    let black = format!("gosub *{BAR_CLEAR}\nlsp {},\":c;>{},{},#000000\",0,0\nprint 10,{}", DIALOG_SPRITES.start, s.width, s.height, bar.load_fade_ms.max(1));
    for a in actions {
        w(format!("*{}", bar_label(a)));
        match a {
            BarAction::Auto => w("systemcall automode".into()),
            BarAction::Skip | BarAction::FastSkip => w("systemcall skip".into()),
            BarAction::Log if s.backlog => w(menu_entry(BACKLOG)),
            BarAction::Log => w("systemcall lookback".into()),
            BarAction::Settings if s.settings => w(menu_entry(SETTINGS)),
            BarAction::Settings => w("return".into()),
            BarAction::Save if s.save_menu => w(menu_entry(SAVE)),
            BarAction::Load if s.save_menu => w(menu_entry(LOAD)),
            BarAction::Save => w("systemcall save".into()),
            BarAction::Load => w("systemcall load".into()),
            BarAction::HideWindow | BarAction::Unavailable => {
                if slide.is_some() {
                    w(format!("if {BAR_OUT} == 1 gosub *{BAR_SLIDE_OUT}"));
                }
                w("systemcall windowerase".into())
            }
            BarAction::QuickSave => {
                w(format!("savegame {QUICK_SLOT}"));
                if let Some(se) = &bar.quick_save_se {
                    w(format!("dwave {},\"{}\"", crate::SE_CHANNELS.start(), path(se)));
                }
            }
            BarAction::QuickLoad => {
                w(format!("savefileexist {TMP},{QUICK_SLOT}:if {TMP} == 0 return"));
                w(black.clone());
                w(format!("loadgame {QUICK_SLOT}"));
            }
            BarAction::BackToChoice => {
                w(format!("savefileexist {TMP},{BACK_SLOT}:if {TMP} == 0 return"));
                if let Some(c) = s.confirm {
                    w(confirm_call(&c.load, Switch::ConfirmBackToChoice));
                    w(format!("if {CONFIRM} == 0 return"));
                }
                w(black.clone());
                w(format!("mov {BACK_LOADING},1"));
                w(format!("loadgame {BACK_SLOT}"));
            }
            BarAction::Title | BarAction::Quit => {
                let (kind, then) = if a == BarAction::Title { (Switch::ConfirmTitle, "reset") } else { (Switch::ConfirmQuit, "end") };
                if let Some(c) = s.confirm {
                    w(confirm_call(if a == BarAction::Title { &c.title } else { &c.quit }, kind));
                    w(format!("if {CONFIRM} == 0 return"));
                }
                w(then.into());
            }
            BarAction::Fullscreen => w(format!("isfull {TMP}\nif {TMP} == 1 menu_window:return\nmenu_full")),
            BarAction::Lock { .. } => w(format!("mov {BAR_UNPINNED},1-{BAR_UNPINNED}")),
            BarAction::Close | BarAction::Call => unreachable!(),
        }
        w("return".into());
    }
    if has_back_to_choice(bar, menu) {
        // Every load but the one going back to the choice drops that save
        // (it belongs to the game left), as a new game does.
        w(format!("*{LOADED}"));
        w(format!("if {BACK_LOADING} == 1 mov {BACK_LOADING},0:goto *conv_loaded_keep"));
        w(format!("delsavegame {BACK_SLOT}"));
        w("*conv_loaded_keep".into());
        if bar.top.is_some() {
            w(format!("goto *{SCENE_LOADED}"));
        }
        w("return".into());
    }
    if bar.top.is_some() {
        scene_stack(&mut w, bar.top.as_ref().is_some_and(|t| t.chapter_history.is_some()));
    }
}

/// The bar or the command menu can go back to the last choice.
pub(crate) fn has_back_to_choice(bar: &ButtonBar, menu: Option<&CommandMenu>) -> bool {
    bar.buttons.iter().any(|b| b.action == BarAction::BackToChoice) || menu.is_some_and(|m| m.items.iter().any(|i| i.action == BarAction::BackToChoice))
}

/// Sprites of the scene list's rows.
fn scene_row_sprites() -> std::ops::Range<u32> {
    SCENE_ROW_SPRITE..SCENE_ROW_SPRITE + 2 * SCENE_ROWS
}

/// Draws the top bars: chapter and scene titles, the scene list (if
/// open), the level numbers and the buttons that browse the levels.
fn scene_bar_draw(w: &mut impl FnMut(String), top: &TopBars, chapter_text: u32, scene_text: u32, buttons: [u32; 4]) {
    let [back, forward, exit, list] = buttons;
    let mut sprites2 = TOP_SPRITES2;
    for p in [&top.chapter_bar, &top.scene_bar] {
        w(lsp2(sprites2.next().unwrap(), p, None));
    }
    let text = |slot: u32, t: &TextSpot, y: i32, var: &str, colour: &str| {
        format!("strsp {slot},{var},{},{y},{},1,{},{},0,0,0,1{colour}", t.x, t.columns, t.font_size, t.font_size)
    };
    // Four cells (normal, pointer, pressed, disabled); `y` may be an
    // expression.
    let lsp4 = |slot: u32, p: &Placed, y: &str| format!("lsp {slot},\":a/4,0,3;{}\",{},{y}", path(&p.image), p.x);
    w(text(chapter_text, &top.chapter_text, top.chapter_text.y, CHAPTER, ""));
    // The scene: its title at the current depth, or the root's.
    w(format!("mov {SCENE_TMP},\"{}\"", top.root_scene));
    w(format!("mov {TMP},{SCENE_TITLES}+{SCENE_DEPTH}"));
    w(format!("if {SCENE_DEPTH} > 0 mov {SCENE_TMP},${TMP}"));
    w(text(scene_text, &top.scene_text, top.scene_text.y, SCENE_TMP, ""));

    w(lsp4(list, &top.list, &top.list.y.to_string()));
    w(format!("spbtn {list},{BTN_SCENE_LIST}"));
    // The list: a row per level from 1 to the deepest; the current one
    // marked, those that can't be gone to disabled.
    let pitch = top.row_pitch;
    w(format!("mov {SCENE_SHIFT},0"));
    w(format!("if {SCENE_LIST_OPEN} == 0 goto *conv_scene_rows_end"));
    w(format!("mov {SCENE_SHIFT},{SCENE_MAX_DEPTH}*{pitch}"));
    w(format!("if {SCENE_MAX_DEPTH} > {SCENE_ROWS} mov {SCENE_SHIFT},{}", SCENE_ROWS as i32 * pitch));
    for k in 1..=SCENE_ROWS {
        let r = k - 1;
        let (label, button) = (SCENE_ROW_SPRITE + 2 * r, SCENE_ROW_SPRITE + 2 * r + 1);
        let (y, text_y) = (top.row.y + r as i32 * pitch, top.row_text.y + r as i32 * pitch);
        let title = format!("${}", SCENE_TITLES + k);
        let colour = |c: &str| format!(",\"{c}\"");
        w(format!("if {SCENE_MAX_DEPTH} < {k} goto *conv_scene_rows_end"));
        w(format!(
            "if {SCENE_DEPTH} == {k} {}:{}",
            lsp4(button, &top.current_row, &y.to_string()),
            text(label, &top.row_text, text_y, &title, &colour(&top.current_row_color))
        ));
        w(format!(
            "if {SCENE_DEPTH} != {k} {}:{}",
            lsp4(button, &top.row, &y.to_string()),
            text(label, &top.row_text, text_y, &title, &colour(&top.row_color))
        ));
        w(format!("if {SCENE_DEPTH} != {k} && {SCENE_FLOOR} <= {k} spbtn {button},{}", BTN_SCENE_ROW + k as i32));
        w(format!("if {SCENE_FLOOR} > {k} cell {button},3"));
    }
    w("*conv_scene_rows_end".into());

    // The level row, below the list.
    let shifted = |y: i32| format!("{y}+{SCENE_SHIFT}");
    let b = &top.level_box;
    w(format!("lsp2 {},\":a;{}\",{},{},100,100,0", sprites2.next().unwrap(), path(&b.image), b.x + b.w / 2, shifted(b.y + b.h / 2)));
    for (var, (x, y), slot) in [(SCENE_DEPTH, top.depth_at, sprites2.next().unwrap()), (SCENE_MAX_DEPTH, top.max_depth_at, sprites2.next().unwrap())] {
        for (d, image) in top.digits.iter().enumerate() {
            w(format!("if {var} == {d} lsp2 {slot},\":a;{}\",{},{},100,100,0", path(image), x + 4, shifted(y + 6)));
        }
    }
    // Disabled unless: back, a level above that can be gone to; forward, a
    // level below; exit, the root can be gone to.
    let enabled = [
        (back, &top.back, format!("{SCENE_DEPTH} > {SCENE_FLOOR}"), BTN_SCENE_BACK),
        (forward, &top.forward, format!("{SCENE_DEPTH} < {SCENE_MAX_DEPTH}"), BTN_SCENE_FORWARD),
        (exit, &top.exit, format!("{SCENE_DEPTH} > 0 && {SCENE_FLOOR} == 0"), BTN_SCENE_EXIT),
    ];
    for (slot, p, condition, number) in enabled {
        w(format!("{}:cell {slot},3", lsp4(slot, p, &shifted(p.y))));
        w(format!("if {condition} cell {slot},0:spbtn {slot},{number}"));
    }
}

/// The routines behind the scene buttons, and the `loadgosub` routine.
///
/// Going to another level saves the game to the current level's slot
/// (the page the player is on, as a save from the menu would) and loads
/// the target level's. Entering a link scene saves the link page to the
/// current level's slot; the link pages in `lib.rs` keep the rest of the
/// bookkeeping (see [`SCENE_DEPTH`]). The deepest level and the titles are
/// locals and go back with a load, so they are carried over in globals.
///
/// A save from the save menu keeps the slots of the levels that can be
/// gone to (`conv_scene_keep`, with CONscripter's `copysavegame`), and a
/// load puts them back, as the original keeps its scene stack in a save.
/// A save without them (made by an older build) loads at the level saved,
/// whose levels below are dropped and above can't be gone back to.
/// `chapters`: there is a [`ChapterHistory`](conv_core::ir::ChapterHistory),
/// whose entries a load may carry over.
fn scene_stack(w: &mut impl FnMut(String), chapters: bool) {
    let titles = || (1..=SCENE_ROWS).map(|k| (SCENE_TITLES + k, SCENE_CARRY + k));
    w("*conv_scene_action".into());
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_LIST} goto *conv_scene_list"));
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_BACK} mov {SCENE_TARGET},{SCENE_DEPTH}-1"));
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_FORWARD} mov {SCENE_TARGET},{SCENE_DEPTH}+1"));
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_EXIT} mov {SCENE_TARGET},0"));
    w(format!("if {BUTTON_VAR} > {BTN_SCENE_ROW} mov {SCENE_TARGET},{BUTTON_VAR}-{BTN_SCENE_ROW}"));
    // Only levels that can be gone to.
    w(format!("if {SCENE_TARGET} < {SCENE_FLOOR} return"));
    w(format!("if {SCENE_TARGET} > {SCENE_MAX_DEPTH} return"));
    w(format!("if {SCENE_TARGET} == {SCENE_DEPTH} return"));
    w(format!("savegame {SCENE_SLOT}+{SCENE_DEPTH}"));
    w(format!("mov {SCENE_LOADING},1:mov {SCENE_CARRY_MAX},{SCENE_MAX_DEPTH}:mov {SCENE_CARRY_FLOOR},{SCENE_FLOOR}"));
    w(titles().map(|(local, carry)| format!("mov ${carry},${local}")).collect::<Vec<_>>().join(":"));
    w(format!("loadgame {SCENE_SLOT}+{SCENE_TARGET}"));
    // Only reached if the slot couldn't be loaded.
    w(format!("mov {SCENE_LOADING},0"));
    w("return".into());

    w("*conv_scene_list".into());
    w(format!("if {SCENE_LIST_OPEN} == 1 mov {SCENE_LIST_OPEN},0:return"));
    w(format!("mov {SCENE_LIST_OPEN},1"));
    w("return".into());

    // SELECTED: the save slot.
    let kept = |slot: &str| format!("{SCENE_KEPT}+{slot}*10+{SLOT}");
    w("*conv_scene_keep".into());
    w(format!("for {SLOT}=0 to {}", SCENE_LEVELS - 1));
    w(format!("delsavegame {}", kept(SELECTED)));
    w(format!(
        "if {SLOT} >= {SCENE_FLOOR} && {SLOT} <= {SCENE_MAX_DEPTH} && {SLOT} != {SCENE_DEPTH} copysavegame {SCENE_SLOT}+{SLOT},{}",
        kept(SELECTED)
    ));
    w("next".into());
    w("return".into());
    w("*conv_scene_drop".into());
    w(format!("for {SLOT}=0 to {}", SCENE_LEVELS - 1));
    w(format!("delsavegame {}", kept(SELECTED)));
    w("next".into());
    w("return".into());

    w(format!("*{SCENE_LOADED}"));
    if chapters {
        w(format!("gosub *{}", crate::chapters::LOADED));
    }
    w(format!("if {SCENE_LOADING} == 1 goto *conv_scene_switched"));
    // A load from the save menu: its levels back, if it kept them.
    w(format!("mov {SCENE_RESTORED},0"));
    w(format!("for {SLOT}=0 to {}", SCENE_LEVELS - 1));
    w(format!("savefileexist {HAS_DATA},{}", kept(SCENE_LOAD_SLOT)));
    w(format!("if {HAS_DATA} == 1 copysavegame {},{SCENE_SLOT}+{SLOT}:mov {SCENE_RESTORED},1", kept(SCENE_LOAD_SLOT)));
    w("next".into());
    w(format!("if {SCENE_RESTORED} == 0 mov {SCENE_MAX_DEPTH},{SCENE_DEPTH}:mov {SCENE_FLOOR},{SCENE_DEPTH}"));
    w("return".into());
    w("*conv_scene_switched".into());
    w(format!("mov {SCENE_LOADING},0:mov {SCENE_MAX_DEPTH},{SCENE_CARRY_MAX}:mov {SCENE_FLOOR},{SCENE_CARRY_FLOOR}"));
    w(titles().map(|(local, carry)| format!("mov ${local},${carry}")).collect::<Vec<_>>().join(":"));
    w("return".into());
}

/// The settings screen (see [`SettingsMenu`]): levels 0-10 in globals,
/// applied by [`SETTINGS_APPLY`] (also run at startup by
/// [`settings_init`]), and on a second page the voice switches (globals
/// from [`VOICE_OFF_BASE`], read where each voice plays).
/// `leave`: during a scene replay, "back to the title" becomes this label
/// and question and leads back to the replay's list. `version`: the
/// build's line and the screen's height, the line written small in the
/// bottom left corner.
pub(crate) fn settings_menu(
    out: &mut String,
    m: &SettingsMenu,
    confirm: Option<&ConfirmDialog>,
    leave: Option<(&str, &str)>,
    version: Option<(&str, u32)>,
) {
    if let Some(p) = &m.pictures {
        crate::settings_pages::emit(out, m, p, confirm, leave);
        settings_apply(out, m);
        return;
    }
    let rows = &m.rows[..m.rows.len().min(MAX_SETTING_ROWS)];
    let voices = &m.voices[..m.voices.len().min(MAX_VOICES)];
    let switches = &m.switches[..m.switches.len().min(MAX_SWITCHES)];
    // Pages: levels, voices, switches (only as many as there are tabs).
    let pages = m.tab_labels.len().min(1 + usize::from(!voices.is_empty()) + usize::from(!switches.is_empty())) as u32;
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
    if let Some((label, _)) = leave {
        let text = format!("\"{}\"", escape(label));
        w(format!("if {IN_GAME} == 1 && {} == 1 {}", crate::replay::REPLAYING, hover(SETTING_TITLE, &text, m.label_x, m.exits_y, 20)));
    }
    w("*conv_settings_draw".into());
    w((M..SETTING_TITLE).map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    if let Some((line, height)) = version {
        // Half-width characters count half a column.
        let text = crate::sprite_text(line);
        let columns = text.chars().count().div_ceil(2) + 1;
        w(format!("strsp {SETTING_VERSION},\"{text}\",8,{},{columns},1,14,14,0,0,0,1,\"#B0B0B0\"", height - 22));
    }
    if pages > 1 {
        // Tabs: the current page's in the highlight colour, not clickable.
        let mut x = m.tabs_x;
        for (page, label) in m.tab_labels.iter().take(pages as usize).enumerate() {
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
        w(format!("if {SETTINGS_PAGE} == 2 goto *conv_settings_draw_switches"));
    }
    for (r, row) in rows.iter().enumerate() {
        let y = m.y + r as i32 * m.pitch;
        let level = setting_var(row.setting);
        let r = r as u32;
        for (slot, x, sign) in [(SETTING_MINUS + r, m.minus_x, "－"), (SETTING_PLUS + r, m.plus_x, "＋")] {
            w(hover(slot, &format!("\"{sign}\""), x, y, 1));
        }
        // "ＢＧＭ　…　■■■■■■■□□□　７": the label, padded with full-width
        // spaces up to the level's column (minus sits over the padding).
        let pad = ((m.level_x - m.label_x) / (size as i32 + 1)).max(row.label.chars().count() as i32 + 1) as usize;
        let label = format!("{}{}", row.label, "　".repeat(pad - row.label.chars().count()));
        w(format!("mov {STR_TMP},\"{}\"", escape(&label)));
        for i in 1..=10 {
            w(format!("if {level} >= {i} add {STR_TMP},\"■\""));
            w(format!("if {level} < {i} add {STR_TMP},\"□\""));
        }
        w(format!("add {STR_TMP},\"　\""));
        w(format!("itoa2 {LINE1},{level}"));
        w(format!("add {STR_TMP},{LINE1}"));
        w(format!("strsp {},{STR_TMP},{},{y},{},1,{size},{size},0,0,0,1", SETTING_ROW + r, m.label_x, pad + 14));
    }
    if let Some(label) = &m.skip_unread_label {
        w(format!("mov {STR_TMP},\"□　\""));
        w(format!("if {SKIP_UNREAD} == 1 mov {STR_TMP},\"■　\""));
        w(format!("add {STR_TMP},\"{}\"", escape(label)));
        w(hover(SETTING_SKIP, STR_TMP, m.label_x, m.y + rows.len() as i32 * m.pitch, label.chars().count() + 2));
    }
    w("goto *conv_settings_wait".into());
    // "■ option" (on) or "□ option", one column.
    w("*conv_settings_draw_switches".into());
    for (i, s) in switches.iter().enumerate() {
        let y = m.y + i as i32 * m.voice_pitch;
        w(format!("mov {STR_TMP},\"■　\""));
        w(format!("if {} == 1 mov {STR_TMP},\"□　\"", switch_off(s.switch)));
        w(format!("add {STR_TMP},\"{}\"", escape(&s.label)));
        w(hover(SETTING_SWITCH + i as u32, STR_TMP, m.label_x, y, s.label.chars().count() + 2));
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
    // The other pages' tabs.
    for p in 0..pages {
        if pages > 1 {
            w(format!("if {SETTINGS_PAGE} != {p} spbtn {},{}", SETTING_TAB + p, BTN_TAB + 1 + p));
        }
    }
    w(format!("if {SETTINGS_PAGE} == 1 goto *conv_settings_wait_voices"));
    w(format!("if {SETTINGS_PAGE} == 2 goto *conv_settings_wait_switches"));
    for r in 0..rows.len() as u32 {
        w(format!("spbtn {},{}", SETTING_MINUS + r, BTN_MINUS + r + 1));
        w(format!("spbtn {},{}", SETTING_PLUS + r, BTN_PLUS + r + 1));
    }
    if m.skip_unread_label.is_some() {
        w(format!("spbtn {SETTING_SKIP},{BTN_SKIP}"));
    }
    w("goto *conv_settings_wait_any".into());
    w("*conv_settings_wait_voices".into());
    for i in 0..voices.len() as u32 {
        w(format!("spbtn {},{}", SETTING_VOICE + i, BTN_VOICE + i + 1));
    }
    w("goto *conv_settings_wait_any".into());
    w("*conv_settings_wait_switches".into());
    for i in 0..switches.len() as u32 {
        w(format!("spbtn {},{}", SETTING_SWITCH + i, BTN_SWITCH + i + 1));
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
        BTN_TAB + pages,
        BTN_TAB + 1
    ));
    for (i, s) in switches.iter().enumerate() {
        let off = switch_off(s.switch);
        // A switch the apply routine puts into effect does so at once.
        let apply = if crate::switches::apply(s.switch) != crate::switches::Apply::Read { format!("gosub *{SETTINGS_APPLY}:") } else { String::new() };
        w(format!("if {BUTTON_VAR} == {} mov {off},1-{off}:{apply}goto *conv_settings_draw", BTN_SWITCH + i as u32 + 1));
    }
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
    // During a replay: back to its list instead.
    if let Some((_, question)) = leave {
        w("*conv_settings_title".into());
        w(format!("if {} == 0 goto *conv_settings_title_go", crate::replay::REPLAYING));
        if confirm.is_some() {
            w(confirm_call(question, Switch::ConfirmTitle));
            w(format!("if {CONFIRM} == 0 goto *conv_settings_wait"));
        }
        w(format!("goto *{}", crate::replay::END));
    }
    let title = if leave.is_some() { "conv_settings_title_go" } else { "conv_settings_title" };
    for (label, question, then, kind) in [
        (title, confirm.map(|c| &c.title), "reset", Switch::ConfirmTitle),
        ("conv_settings_quit", confirm.map(|c| &c.quit), "end", Switch::ConfirmQuit),
    ] {
        w(format!("*{label}"));
        if let Some(q) = question {
            w(confirm_call(q, kind));
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
    settings_apply(out, m);
}

/// The routine putting the settings into effect (volumes, speeds, window,
/// skip).
fn settings_apply(out: &mut String, m: &SettingsMenu) {
    let rows = &m.rows[..m.rows.len().min(MAX_SETTING_ROWS)];
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    w(format!("*{SETTINGS_APPLY}"));
    let used = used_switches(m);
    for (setting, cmd) in [(Setting::BgmVolume, "bgmvol"), (Setting::SeVolume, "sevol"), (Setting::VoiceVolume, "voicevol")] {
        w(format!("mov {TMP},{}*10", setting_var(setting)));
        // A switch muting it: volume 0, the level kept.
        for &s in used.iter().filter(|&&s| crate::switches::apply(s) == crate::switches::Apply::Mutes(setting)) {
            w(format!("if {} == 1 mov {TMP},0", switch_off(s)));
        }
        w(format!("{cmd} {TMP}"));
    }
    w(format!("mov {TEXT_MS},(10-{})*{TEXT_MS_STEP}:textspeed {TEXT_MS}", setting_var(Setting::TextSpeed)));
    let auto = setting_var(Setting::AutoSpeed);
    match rows.iter().find_map(|r| match r.setting {
        Setting::AutoWait { page_step, char_step } => Some((page_step, char_step)),
        _ => None,
    }) {
        Some((page, char)) => w(format!("automode_time (10-{auto})*{page},(10-{auto})*{char}")),
        None => w(format!("mov {TMP},(10-{auto})*400+600:automode_time {TMP}")),
    }
    if rows.iter().any(|r| r.setting == Setting::WindowTransparency) {
        w(format!("mov {TMP},255-{}*25:windowtrans {TMP}", setting_var(Setting::WindowTransparency)));
    }
    for &s in &used {
        if let crate::switches::Apply::Commands { on, off } = crate::switches::apply(s) {
            let flag = switch_off(s);
            w(format!("if {flag} == 0 {on}"));
            w(format!("if {flag} == 1 {off}"));
        }
    }
    if m.skip_unread_label.is_some() {
        w(format!("if {SKIP_UNREAD} == 0 kidokumode 1"));
        w(format!("if {SKIP_UNREAD} == 1 kidokumode 0"));
    }
    w("return".into());
}

/// The switches the settings screen has (in its list or on picture
/// pages), each once.
fn used_switches(m: &SettingsMenu) -> Vec<Switch> {
    let mut all: Vec<Switch> = m.switches.iter().map(|s| s.switch).collect();
    if let Some(p) = &m.pictures {
        for c in p.pages.iter().flat_map(|page| &page.rows) {
            let (conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. }) = c;
            if let conv_core::ir::SettingValue::Switch(s) = value {
                all.push(*s);
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    all.retain(|&s| seen.insert(switch_index(s)));
    all
}

/// The settings screen has switch `s` (in its list or on a picture page).
pub(crate) fn has_switch(m: &SettingsMenu, s: Switch) -> bool {
    m.switches.iter().any(|x| x.switch == s)
        || m.pictures.as_ref().is_some_and(|p| {
            p.pages.iter().flat_map(|page| &page.rows).any(|c| match c {
                conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. } => *value == conv_core::ir::SettingValue::Switch(s),
            })
        })
}

/// Sets the question and asks it (see [`confirm`]).
pub(crate) fn confirm_call(question: &str, kind: Switch) -> String {
    format!("mov {CONFIRM_KIND},{}:mov {CONFIRM_TEXT},\"{}\"\ngosub *{CONFIRM_ROUTINE}", switch_index(kind) + 1, escape(question))
}

pub(crate) fn switch_index(s: Switch) -> u32 {
    crate::switches::index(s)
}

/// The global that is 1 while `s` is switched off.
pub(crate) fn switch_off(s: Switch) -> String {
    format!("%{}", crate::switches::off_var(switch_index(s)))
}

/// The yes/no dialog (see [`ConfirmDialog`]) over whatever is on screen:
/// asks [`CONFIRM_TEXT`], sets [`CONFIRM`] (1 yes, 0 no or right-click)
/// and takes itself away. The message is centred by its length in bytes
/// (Shift-JIS: half-width characters are one byte, half as wide).
///
/// A dialog with pictures shows the question's picture (or, for a question
/// without one, the base and the text) over the backdrop; its sprites are
/// yes, no, text, picture, backdrop from the top.
pub(crate) fn confirm(out: &mut String, d: &ConfirmDialog) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let size = d.font_size;
    let label_size = d.yes.h as u32 * 2 / 3;
    let centre = d.base.x + d.base.w / 2;
    let pictures = !d.pictures.is_empty();
    let (yes_slot, no_slot, text_slot, base_slot) = if pictures { (DIALOG_TEXT, DIALOG_YES_LABEL, DIALOG_NO_LABEL, DIALOG_NO) } else { (DIALOG_YES, DIALOG_NO, DIALOG_TEXT, DIALOG_BASE) };
    let cells = if d.pressed { 4 } else { 3 };
    w(format!("*{CONFIRM_ROUTINE}"));
    // A confirmation switched off in the settings answers yes at once.
    w(format!("mov {CONFIRM},1"));
    // The switch's flag (switch_off).
    for line in crate::switches::off_var_lines(CONFIRM_KIND, TMP).lines() {
        w(line.to_string());
    }
    w(format!("if {CONFIRM_KIND} > 0 if %%{} == 1 mov {CONFIRM_KIND},0:return", &TMP[1..]));
    if let Some((b, opacity)) = d.backdrop.as_ref().filter(|_| pictures) {
        w(format!("lsp {DIALOG_BASE},\":c;{}\",{},{},{opacity}", path(&b.image), b.x, b.y));
    }
    let base_and_text = [
        format!("lsp {base_slot},\":a;{}\",{},{}", path(&d.base.image), d.base.x, d.base.y),
        format!("len {TMP},{CONFIRM_TEXT}"),
        format!("mov {TMP},{centre}-{TMP}*{size}/4"),
        format!("strsp {text_slot},{CONFIRM_TEXT},{TMP},{},40,1,{size},{size},0,0,0,1", d.base.y + d.text_y),
    ];
    if pictures {
        for (kind, p) in &d.pictures {
            w(format!("if {CONFIRM_KIND} == {} lsp {base_slot},\":a;{}\",{},{}:goto *conv_confirm_drawn", switch_index(*kind) + 1, path(&p.image), p.x, p.y));
        }
        for line in &base_and_text {
            w(line.clone());
        }
        w("*conv_confirm_drawn".into());
    } else {
        for line in &base_and_text {
            w(line.clone());
        }
    }
    w(format!("mov {CONFIRM_KIND},0"));
    for (button, label_slot, p, label) in [(yes_slot, DIALOG_YES_LABEL, &d.yes, &d.yes_label), (no_slot, DIALOG_NO_LABEL, &d.no, &d.no_label)] {
        w(format!("lsp {button},\":a/{cells},0,3;{}\",{},{}", path(&p.image), p.x, p.y));
        if pictures {
            continue;
        }
        let chars = label.chars().count() as i32;
        let x = p.x + p.w / (2 * cells) - chars * label_size as i32 / 2;
        let y = p.y + (p.h - label_size as i32) / 2;
        w(format!("strsp {label_slot},\"{}\",{x},{y},10,1,{label_size},{label_size},0,0,0,1", escape(label)));
    }
    // The question and buttons unrolling: the sprites at each step (slot,
    // left x, y, one cell's width), the backdrop's opacity.
    let unroll = d.unroll.filter(|_| pictures && d.fade_ms > 0).zip(d.pictures.first()).map(|(u, (_, p))| {
        let sprites = [(base_slot, p.x, p.y, p.w), (yes_slot, d.yes.x, d.yes.y, d.yes.w / cells), (no_slot, d.no.x, d.no.y, d.no.w / cells)];
        (u, sprites, d.backdrop.as_ref().map(|(b, o)| (b.x, b.y, *o)))
    });
    if let Some((u, sprites, backdrop)) = &unroll {
        unroll_steps(&mut w, u.x, sprites, *backdrop, d.fade_ms, d.fade_ms, true);
    } else if d.fade_ms > 0 {
        w(format!("print 10,{}", d.fade_ms));
    }
    w("*conv_confirm_wait".into());
    let press = if d.pressed { ",3" } else { "" };
    w(format!("spbtn {yes_slot},1{press}:spbtn {no_slot},2{press}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("mov {CONFIRM},0"));
    w(format!("if {BUTTON_VAR} == 1 mov {CONFIRM},1"));
    w(format!("if {BUTTON_VAR} != 1 && {BUTTON_VAR} != 2 && {BUTTON_VAR} != -1 goto *conv_confirm_wait"));
    if let Some((u, sprites, backdrop)) = &unroll {
        unroll_steps(&mut w, u.x, sprites, *backdrop, d.fade_ms, u.close_ms, false);
    }
    w(DIALOG_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w(if d.fade_ms > 0 && unroll.is_none() { format!("print 10,{}", d.fade_ms) } else { "print 1".into() });
    w("return".into());
}

/// The unroll's frames (`ConfirmDialog::unroll`), about every 16 ms: the sprites (slot, x, y, width)
/// stretched sideways about the line `x` (eased out over `zoom_ms`) and
/// faded (linearly over `fade_ms`), in from nothing (`opening`) or out to
/// it. The backdrop (x, y, opacity) fades with them.
fn unroll_steps(w: &mut impl FnMut(String), x: i32, sprites: &[(u32, i32, i32, i32)], backdrop: Option<(i32, i32, u8)>, zoom_ms: u32, fade_ms: u32, opening: bool) {
    let total = zoom_ms.max(fade_ms);
    let steps = (total / 16).max(1);
    w("resettimer".into());
    for k in 0..=steps {
        let t = (total * k / steps) as f64;
        let p = (t / zoom_ms.max(1) as f64).min(1.0);
        let eased = 1.0 - (1.0 - p) * (1.0 - p);
        let f = (t / fade_ms.max(1) as f64).min(1.0);
        let (s, a) = if opening { (eased, f) } else { (1.0 - eased, 1.0 - f) };
        let mut line = Vec::new();
        for &(slot, sx, sy, sw) in sprites {
            // The sprite's centre moves toward the line as it narrows.
            let centre = x as f64 + (sx as f64 + sw as f64 / 2.0 - x as f64) * s;
            let left = (centre - sw as f64 / 2.0).round() as i32;
            line.push(format!("amsp {slot},{left},{sy},{}", (255.0 * a).round() as u32));
            line.push(format!("spzoom {slot},{},100", ((s * 100.0).round() as u32).max(1)));
        }
        if let Some((bx, by, o)) = backdrop {
            line.push(format!("amsp {DIALOG_BASE},{bx},{by},{}", (o as f64 * a).round() as u32));
        }
        if k > 0 {
            line.push(format!("waittimer {}", total * k / steps));
        }
        line.push("print 1".into());
        w(line.join(":"));
    }
}

/// First run: default levels; every run: apply them (before `setwindow`,
/// which takes the text speed from [`TEXT_MS`]).
pub(crate) fn settings_init(out: &mut String, m: &SettingsMenu) {
    let mut first = settings_defaults(m);
    for &s in &m.switches_off {
        first.push_str(&format!(":mov {},1", switch_off(s)));
    }
    out.push_str(&format!("if {SETTINGS_SET} == 0 {first}:mov {SETTINGS_SET},1\ngosub *{SETTINGS_APPLY}\n"));
}

/// The levels' defaults (as [`settings_init`] sets them, the game's
/// [`SettingsMenu::defaults`] over the shared ones), as commands.
pub(crate) fn settings_defaults(m: &SettingsMenu) -> String {
    let shared = [
        (Setting::BgmVolume, 8),
        (Setting::SeVolume, 8),
        (Setting::VoiceVolume, 10),
        (Setting::TextSpeed, 7),
        (Setting::AutoSpeed, 5),
        (Setting::WindowTransparency, 0),
    ];
    shared
        .iter()
        .map(|&(s, level)| {
            let level = m.defaults.iter().find(|(d, _)| setting_var(*d) == setting_var(s)).map_or(level, |&(_, l)| l.min(10));
            format!("mov {},{level}", setting_var(s))
        })
        .collect::<Vec<_>>()
        .join(":")
}

pub(crate) fn setting_var(s: Setting) -> &'static str {
    match s {
        Setting::BgmVolume => "%902",
        Setting::SeVolume => "%903",
        Setting::VoiceVolume => "%904",
        Setting::TextSpeed => "%905",
        Setting::AutoSpeed | Setting::AutoWait { .. } => "%906",
        // Added later: 0 (the default for older saves) is the plain window.
        Setting::WindowTransparency => "%910",
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
/// `ruby`: `rubyon` is set, so `strsp` draws text with a line pitch a line
/// gap lower (the ruby goes above it); the log is drawn that much higher.
pub(crate) fn backlog(out: &mut String, b: &Backlog, ruby: bool, keep_skip: bool) {
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
    let lift = if ruby { format!("-{gap}") } else { String::new() };
    w(format!("*{PRETEXT}"));
    if keep_skip {
        w(format!("if {KEEP_SKIP} == 1 mov {KEEP_SKIP},0:systemcall skip"));
    }
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
    w(format!("strsp {LOG_SPRITE},{LINE1},{},{LOG_TEXT_Y}{lift},{},{LOG_LINES},{size},{size},0,{gap},0,1", a.x, b.columns));
    w(format!("if {LOG_NAMED} == 1 gosub *conv_log_name"));
    w(format!("mov {LOG_Y},{TMP}"));
    w(format!("add {LOG_K},1"));
    w("goto *conv_log_page".into());
    // The name of page LOG_K at TMP; with a voice it's a button replaying it.
    w("*conv_log_name".into());
    w(format!("mov {LOG_SPRITE},{LOG_K}+{LOG_NAME_SPRITE}"));
    let name = |text: &str, colors: &str| {
        format!("strsp {LOG_SPRITE},{text},{},{TMP}{lift},{},1,{size},{size},0,{gap},0,1,{colors}", a.x, b.columns)
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
    w(format!("if {BUTTON_VAR} == -3 && {LOG_OFFSET} == 0 && {} == 1 goto *conv_log_wait", switch_off(Switch::WheelClosesLog)));
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
pub(crate) fn hidden_while_open() -> Vec<u32> {
    BAR_SPRITES.chain([NAME_TEXT_SPRITE, NAME_PLATE_SPRITE]).chain(LINK_SPRITES).collect()
}

/// `lsp2` places the image by its centre.
pub(crate) fn lsp2(slot: u32, p: &Placed, opacity: Option<u8>) -> String {
    let (w, h) = (p.w, p.h);
    let opacity = opacity.map_or(String::new(), |o| format!(",{o}"));
    format!("lsp2 {slot},\":a;{}\",{},{},100,100,0{opacity}", path(&p.image), p.x + w / 2, p.y + h / 2)
}

#[cfg(test)]
pub(crate) mod tests {
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
            text_width: 0,
            backdrop: Some((4, [6, 0, 256, 1, 1, 0])),
        };
        let d = dialog();
        let mut out = String::new();
        save_menu(&mut out, &m, Some(&d), false);
        for line in [
            "if %100 == 1 getscreenshot 400,300\n",
            // lsp2 takes the centre.
            "lsp2 11,\":a;system\\base.png\",410,320,100,100,0,140\n",
            // Row 2 (slot FIRST + 1): thumbnail at 25%, centred in its box.
            "if %104 == 1 lsp2 2,$105,64,183,25,25,0\n",
            "if %900 == 2 cell 22,2:lsp2 7,\":a;system\\marker.png\",214,484,100,100,0\n",
            "if %9 > 100 && %9 <= 102 mov %900,%9-100:goto *conv_sl_page\n",
            "savescreenshot2 $103\n",
            "screenfilter 4,0:savegame2 %102,$100:screenfilter 4,6,0,256,1,1,0\n",
            "if %101 == 1 screenfilter 4,6,0,256,1,1,0\n",
            "if %101 == 1 screenfilter 4,0\n",
            "loadgame %102\n",
            // Delete removes the save and its thumbnail; a slot an older
            // build hid with a flag is deleted when listed.
            "mov %103,%102\ngosub *conv_sl_file\ndelsavegame %102,$103\n",
            "if %%105 == 1 delsavegame %103,$103:mov %%105,0\nsavefileexist %104,%103\n",
            // Saving over a save asks first.
            "if %107 == 0 goto *conv_sl_save\nmov %103,%102\ngosub *conv_sl_number\nmov %125,3:mov $106,$101\nadd $106,\" over?\"\ngosub *conv_confirm\nif %109 == 0 goto *conv_sl_buttons\n*conv_sl_save\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    pub(crate) fn dialog() -> ConfirmDialog {
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
            pictures: Vec::new(),
            backdrop: None,
            pressed: false,
            fade_ms: 0,
            unroll: None,
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
    fn confirm_with_pictures() {
        let mut d = dialog();
        d.pictures = vec![(Switch::ConfirmQuit, placed("quit_question", 100, 400))];
        d.backdrop = Some((placed("black", 800, 600), 204));
        d.pressed = true;
        d.fade_ms = 150;
        let mut out = String::new();
        confirm(&mut out, &d);
        for line in [
            "lsp 5,\":c;system\\black.png\",10,20,204\n",
            // The question's picture in place of the base and text; others get both.
            "if %125 == 7 lsp 4,\":a;system\\quit_question.png\",10,20:goto *conv_confirm_drawn\nlsp 4,\":a;system\\dialog_base.png\",140,254\n",
            "strsp 2,$106,",
            // No labels; four cells, the fourth while pressed.
            "lsp 0,\":a/4,0,3;system\\dialog_button.png\",242,305\nlsp 1,",
            "print 10,150\n*conv_confirm_wait\nspbtn 0,1,3:spbtn 1,2,3\nprint 1\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
        assert!(!out.contains("はい"));
    }

    #[test]
    fn confirm_unrolls() {
        let mut d = dialog();
        d.pictures = vec![(Switch::ConfirmQuit, placed("quit_question", 100, 400))];
        d.backdrop = Some((placed("black", 800, 600), 204));
        d.fade_ms = 150;
        d.unroll = Some(conv_core::ir::Unroll { x: 60, close_ms: 200 });
        let mut out = String::new();
        confirm(&mut out, &d);
        for line in [
            // From a line at x 60, unseen, the backdrop with it…
            "resettimer\namsp 4,10,20,0:spzoom 4,1,100:",
            "amsp 5,10,20,0:print 1\n",
            // …to whole after 150 ms; closing takes 200.
            "spzoom 4,100,100:",
            "amsp 5,10,20,204:waittimer 150:print 1\n*conv_confirm_wait\n",
            "amsp 5,10,20,0:waittimer 200:print 1\ncsp 0:",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
        assert!(!out.contains("print 10,150"));
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
            tab_labels: vec!["音量".into(), "音声".into(), "確認".into()],
            tabs_x: 300,
            tabs_y: 34,
            voices: vec![
                VoiceToggle { label: "あずさ".into(), characters: vec![3] },
                VoiceToggle { label: "その他".into(), characters: vec![0, 2] },
            ],
            voice_column_x: 440,
            voice_pitch: 50,
            skip_unread_label: Some("未読".into()),
            pictures: None,
            defaults: Vec::new(),
            switches_off: Vec::new(),
            switches: vec![conv_core::ir::SettingSwitch { label: "セーブ確認".into(), switch: Switch::ConfirmSave }],
        };
        let mut out = String::new();
        settings_menu(&mut out, &m, None, None, None);
        for line in [
            // The current tab can't be clicked; the other one can.
            "if %176 != 1 strsp 28,\"音声\",392,34,3,1,22,22,0,0,0,1,\"#FFFFFF\",\"#F0C040\"\n",
            "if %176 == 0 strsp 27,\"音量\",300,34,3,1,22,22,0,0,0,1,\"#F0C040\"\n",
            "if %176 == 1 goto *conv_settings_draw_voices\n",
            // One per column: the second switch is in the second column.
            "mov $104,\"■　\"\nif %973 == 1 mov $104,\"□　\"\nadd $104,\"あずさ\"\nstrsp 12,$104,140,140,6,1,22,22,0,0,0,1,\"#FFFFFF\",\"#F0C040\"\n",
            "strsp 13,$104,440,140,",
            "*conv_settings_wait_voices\nspbtn 12,51\nspbtn 13,52\ngoto *conv_settings_wait_any\n*conv_settings_wait_switches\nspbtn 12,71\n",
            "if %176 != 0 spbtn 27,41\nif %176 != 1 spbtn 28,42\nif %176 != 2 spbtn 29,43\n",
            // A confirmation switch: its off flag, and the dialog answers yes when off.
            "if %9 == 71 mov %3642,1-%3642:goto *conv_settings_draw\n",
            "if %9 > 40 && %9 <= 43 mov %176,%9-41:goto *conv_settings_draw\n",
            // A level row: label padded to the level's column, one sprite.
            "mov $104,\"ＢＧＭ　　　　　　　\"\n",
            "strsp 6,$104,140,140,24,1,22,22,0,0,0,1\n",
            "spbtn 15,11\nspbtn 21,21\n",
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
    fn sliding_bar_waits_off_screen_and_comes_in_at_the_edge() {
        use conv_core::ir::{BarButton, BarSlide};
        let bar = ButtonBar {
            buttons: vec![
                BarButton { image: placed("qsave", 104, 130), active: None, action: BarAction::QuickSave },
                BarButton { image: placed("qload", 104, 106), active: None, action: BarAction::QuickLoad },
            ],
            base: Some(placed("base", 32, 524)),
            slide: Some(BarSlide { hidden_dx: 32, edge_x: 968, ms: 200 }),
            pressed: true,
            quick_save_se: Some("sound/se/bookmark.ogg".into()),
            load_fade_ms: 300,
            ..Default::default()
        };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { width: 1024, height: 576, ..Default::default() });
        for line in [
            "getmousepos %105,%146\nif %105 >= 1024 || %146 >= 576 mov %105,0\n",
            "if %143 == 0 && %105 > 968 gosub *conv_bar_slide_in:goto *conv_wait\n",
            "getmouseover 1300,1300\ntextbtnwait %9\nif %9 == 1300 goto *conv_wait\n",
            "if %9 == 0 && %143 == 1 && %108 == 0 goto *conv_wait\n",
            // The base over the buttons; buttons only while the bar is out.
            "lsp 35,\":a/4,0,3;system\\qsave.png\",10+%144,20\nif %143 == 1 spbtn 35,1001,3\n",
            "if %141 == 1 cell 36,2\nif %143 == 1 if %141 == 0 spbtn 36,1002,3\n",
            "lsp 34,\":a;system\\base.png\",10+%144,20\n",
            "if %143 == 0 btn 1300,969,0,55,576,0,0\nif %143 == 1 btn 1300,0,0,969,576,0,0\n",
            "mov %144,32*%145*%145/1000*%145/1000000\nif %143 == 0 mov %144,32-%144\n",
            "*conv_bar_qsave\nsavegame 890\ndwave 1,\"sound\\se\\bookmark.ogg\"\nreturn\n",
            "*conv_bar_qload\nsavefileexist %105,890:if %105 == 0 return\ngosub *conv_bar_clear\nlsp 0,\":c;>1024,576,#000000\",0,0\nprint 10,300\nloadgame 890\n",
        ] {
            assert!(out.contains(line), "missing:\n{line}\nin:\n{out}");
        }
    }

    #[test]
    fn wait_icon_shows_at_the_bar_wait() {
        use conv_core::ir::BarButton;
        let bar = ButtonBar {
            buttons: vec![BarButton { image: placed("save", 132, 19), active: None, action: BarAction::Save }],
            top: None,
            ..Default::default()
        };
        let icon = WaitIcon { image: "system/icon.png".into(), frame_ms: vec![50, 50, 500], w: 20, h: 20 };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { wait_icon: Some((&icon, ["%183", "%184"])), ..Default::default() });
        for line in [
            // After the bar (which cleared it), in the bar's next free slot.
            "gosub *conv_bar_draw\nif %183 > 0 || %184 > 0 lsp 42,\":a/3,<50,50,500>,0;system\\icon.png\",%183,%184:print 1\n",
            "*conv_wait_end\ncsp 42\ntexec\n",
        ] {
            assert!(out.contains(line), "missing:\n{line}\nin:\n{out}");
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
            top: None,
            ..Default::default()
        };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { save_menu: true, ..Default::default() });
        for line in [
            "*conv_wait\ngosub *conv_bar_draw\ntextbtnwait %9\nif %9 == -3 && %3640 == 0 goto *conv_wait_end\nif %9 == -3 goto *conv_wait\n",
            "if %9 >= 0 && %9 <= 1000 goto *conv_wait_end\n",
            "if %9 == 1001 goto *conv_wait_end\ngoto *conv_wait\n",
            "*conv_wait_end\ntexec\nreturn\n",
            "btndef clear\ngosub *conv_bar_clear\nisskip %108\n",
            "lsp 34,\":a/3,0,3;system\\auto.png\",10,20\nif %108 == 2 lsp 34,\":a/3,0,3;system\\auto_on.png\",10,20\nspbtn 34,1001\n",
            "lsp 36,\":a;system\\setting.png\",10,20\nprint 1\nreturn\n",
            "if %9 == -1 goto *conv_bar_hide\nif %9 == -2 goto *conv_bar_log\nif %9 == 1001 goto *conv_bar_auto\nif %9 == 1002 goto *conv_bar_save\nreturn\n",
            "*conv_bar_log\nsystemcall lookback\nreturn\n",
            "*conv_bar_save\ngosub *conv_bar_clear\nmov %101,1\ngosub *conv_save\nreturn\n",
            "*conv_bar_hide\nsystemcall windowerase\nreturn\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
        assert!(!out.contains("conv_scene"), "scene routines without top bars:\n{out}");
    }

    #[test]
    fn unpinned_bars_show_under_the_pointer() {
        use conv_core::ir::BarButton;
        let bar = ButtonBar {
            buttons: vec![BarButton { image: placed("lock", 87, 19), active: Some(placed("lock_on", 87, 19)), action: BarAction::Lock { reveal_y: 573, fade_ms: 100 } }],
            top: None,
            ..Default::default()
        };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { width: 800, height: 600, ..Default::default() });
        for line in [
            "if %2908 == 1 getmouseover 1300,1300\n",
            "textbtnwait %9\nif %9 == 1300 goto *conv_wait\n",
            "getmousepos %105,%146\nif %2908 == 1 && %146 < 573 btn 1300,0,573,800,27,0,0:mov %148,0:print 1:return\n",
            "if %2908 == 1 lsp 34,\":a/3,0,3;system\\lock_on.png\",10,20\n",
            "if %2908 == 1 btn 1300,0,0,800,573,0,0\nif %2908 == 1 && %148 == 0 spfade 34,100\nmov %148,1\n",
            "*conv_bar_lock\nmov %2908,1-%2908\nreturn\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    #[test]
    fn voices_stop_on_a_click_when_switched_off() {
        let bar = ButtonBar { buttons: Vec::new(), top: None, ..Default::default() };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { save_menu: true, voice_stops: true, ..Default::default() });
        assert!(out.contains("*conv_wait_end\nif %2910 == 1 dwavestop 0\ntexec\n"), "{out}");
        assert_eq!(switch_off(Switch::ConfirmQuit), "%3646");
    }

    pub(crate) fn top_bars() -> TopBars {
        let spot = TextSpot { x: 5, y: 6, font_size: 14, columns: 17 };
        TopBars {
            chapter_bar: placed("chapter_bar", 400, 28),
            chapter_text: spot,
            scene_bar: placed("scene_bar", 384, 28),
            scene_text: spot,
            root_scene: "root scene".into(),
            level_box: placed("scene_depth", 94, 20),
            digits: (0..10).map(|d| format!("system/digit{d}.png")).collect(),
            depth_at: (664, 34),
            max_depth_at: (685, 34),
            back: placed("scene_prev", 26, 23),
            forward: placed("scene_next", 26, 23),
            exit: placed("scene_exit", 38, 23),
            list: placed("scene_list", 20, 18),
            row: Placed { image: "system/scene_row.png".into(), x: 531, y: 30, w: 269, h: 26 },
            current_row: Placed { image: "system/scene_row_current.png".into(), x: 531, y: 30, w: 269, h: 26 },
            row_pitch: 26,
            row_text: TextSpot { x: 550, y: 34, font_size: 14, columns: 17 },
            row_color: "#A0A0A0".into(),
            current_row_color: "#FFFFFF".into(),
            chapter_history: None,
        }
    }

    #[test]
    fn scene_stack_routines() {
        let top = top_bars();
        let bar = ButtonBar { buttons: vec![], top: Some(top), ..Default::default() };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { save_menu: true, ..Default::default() });
        for line in [
            // The rows clear with the bars.
            "csp 22:csp 23:csp 24:",
            // Buttons: back, forward, exit (disabled unless they lead somewhere), list.
            "lsp 39,\":a/4,0,3;system\\scene_list.png\",10,20\nspbtn 39,1104\n",
            "lsp 36,\":a/4,0,3;system\\scene_prev.png\",10,20+%191:cell 36,3\nif %150 > %190 cell 36,0:spbtn 36,1101\n",
            "if %150 < %151 cell 37,0:spbtn 37,1102\n",
            "if %150 > 0 && %190 == 0 cell 38,0:spbtn 38,1103\n",
            // The list pushes the level row down.
            "if %2903 == 0 goto *conv_scene_rows_end\nmov %191,%151*26\n",
            "if %151 < 2 goto *conv_scene_rows_end\n",
            "if %150 == 2 lsp 25,\":a/4,0,3;system\\scene_row_current.png\",531,56:strsp 24,$162,550,60,17,1,14,14,0,0,0,1,\"#FFFFFF\"\n",
            "if %150 != 2 && %190 <= 2 spbtn 25,1112\nif %190 > 2 cell 25,3\n",
            "if %9 >= 1101 && %9 <= 1116 goto *conv_scene_action\n",
            // Going to a level: save this one, carry the history over, load.
            "if %9 > 1110 mov %192,%9-1110\nif %192 < %190 return\nif %192 > %151 return\nif %192 == %150 return\nsavegame 900+%150\nmov %2900,1:mov %2901,%151:mov %2902,%190\nmov $2901,$161:",
            "loadgame 900+%192\nmov %2900,0\nreturn\n",
            // A save keeps its levels' slots; a load puts them back.
            "*conv_scene_keep\nfor %103=0 to 7\ndelsavegame 1000+%102*10+%103\nif %103 >= %190 && %103 <= %151 && %103 != %150 copysavegame 900+%103,1000+%102*10+%103\nnext\nreturn\n",
            "*conv_scene_loaded\nif %2900 == 1 goto *conv_scene_switched\nmov %2907,0\nfor %103=0 to 7\nsavefileexist %104,1000+%2906*10+%103\nif %104 == 1 copysavegame 1000+%2906*10+%103,900+%103:mov %2907,1\nnext\nif %2907 == 0 mov %151,%150:mov %190,%150\nreturn\n*conv_scene_switched\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    #[test]
    fn auto_wait_per_page_and_character() {
        use conv_core::ir::SettingRow;
        let mut m = SettingsMenu {
            base: placed("base", 800, 600),
            base_opacity: 210,
            header: placed("header", 100, 30),
            close: placed("close", 60, 20),
            rows: vec![SettingRow { label: "オート".into(), setting: Setting::AutoSpeed }],
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
            tab_labels: Vec::new(),
            tabs_x: 300,
            tabs_y: 34,
            voices: Vec::new(),
            voice_column_x: 440,
            voice_pitch: 50,
            skip_unread_label: None,
            pictures: None,
            defaults: Vec::new(),
            switches_off: Vec::new(),
            switches: Vec::new(),
        };
        let mut out = String::new();
        settings_menu(&mut out, &m, None, None, None);
        assert!(out.contains("mov %105,(10-%906)*400+600:automode_time %105\n"), "{out}");
        m.rows[0].setting = Setting::AutoWait { page_step: 500, char_step: 4 };
        let mut out = String::new();
        settings_menu(&mut out, &m, None, None, None);
        assert!(out.contains("automode_time (10-%906)*500,(10-%906)*4\n"), "{out}");
        // A mute switch: volume 0, the level kept; applied when toggled.
        m.switches = vec![conv_core::ir::SettingSwitch { label: "ＢＧＭ".into(), switch: Switch::BgmOn }];
        m.tab_labels = vec!["a".into(), "b".into()];
        let mut out = String::new();
        settings_menu(&mut out, &m, None, None, None);
        assert!(out.contains("mov %105,%902*10\nif %2911 == 1 mov %105,0\nbgmvol %105\n"), "{out}");
        assert!(out.contains("if %9 == 71 mov %2911,1-%2911:gosub *conv_settings_apply:goto *conv_settings_draw\n"), "{out}");
        assert!(out.contains("itoa2 $101,%906\n"), "{out}");
    }
}
