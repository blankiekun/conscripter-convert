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

mod backlog;
mod bar;
mod confirm;
mod save;
mod scenes;
mod settings;
pub(crate) use backlog::*;
pub(crate) use bar::*;
pub(crate) use confirm::*;
pub(crate) use save::*;
use scenes::*;
pub(crate) use settings::*;

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
/// 1 once the system volume (added later than %909) has its default.
const SYSTEM_VOLUME_SET: &str = "%912";
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
/// 1 while saving and loading are locked (a picture extras' scene replay
/// plays, [`BarScreens::picture_replay`]).
pub(crate) const NO_SAVE: &str = "%151";
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
/// 1 while the skip [`BarAction::FastSkip`] started runs (global; cleared
/// when a test finds skip off, [`Op::IfFastSkip`]).
pub(crate) const FAST_SKIP: &str = "%2905";
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

/// Sprites hidden while a full-screen menu is open: the bars, the name
/// plate and the link words.
pub(crate) fn hidden_while_open() -> Vec<String> {
    let numbered = |n: u32| n.to_string();
    BAR_SPRITES.map(numbered).chain([NAME_TEXT_SPRITE, NAME_PLATE_SPRITE].map(String::from)).chain(LINK_SPRITES.map(numbered)).collect()
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
            engine_screen: false,
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
        d.unroll = Some(conv_core::ir::Unroll { x: 60, close_ms: 200, y: None });
        let mut out = String::new();
        confirm(&mut out, &d);
        for line in [
            // From a line at x 60 (the picture's centre already), unseen, the backdrop with it,
            // a key a frame…
            "amsp 4,10,20,255:spzoom 4,100:spmotion 4,0,0,0,0,1,100,0,0,0,16,0,0,20,100,0,27,0,",
            // …the yes button's centre (242 + its half width) moving onto the line…
            "spmotion 0,0,0,-253,0,1,100,0,0,0,",
            // …to whole after 150 ms; closing takes 200, then the sprites go.
            ",150,0,0,100,100,0,256,0\n",
            "amsp 5,10,20,204:spmotion 5,0,0,0,0,100,100,0,0,0,",
            "print 1:spmotionwait 4:spmotionwait 0:spmotionwait 1:spmotionwait 5\n*conv_confirm_wait\n",
            "spmotion 4,3,0,0,0,100,100,0,256,0,",
            ",200,0,0,100,100,0,0,0\nprint 1:spmotionwait 4:spmotionwait 0:spmotionwait 1:spmotionwait 5\ncsp 0:",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
        assert!(!out.contains("print 10,150") && !out.contains("waittimer"));
        // Up and down about y 30 (the picture's 20-420 narrowed onto it: its centre 220 moved -190).
        d.unroll = Some(conv_core::ir::Unroll { x: 60, close_ms: 200, y: Some(30) });
        let mut out = String::new();
        confirm(&mut out, &d);
        assert!(out.contains("spmotion 4,0,0,0,-190,100,1,0,0,0,"), "{out}");
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
    fn quick_save_notice_until_a_click() {
        use conv_core::ir::BarButton;
        let bar = ButtonBar {
            buttons: vec![BarButton { image: placed("qsave", 104, 130), active: None, action: BarAction::QuickSave }],
            quick_saved: Some(placed("saved", 312, 213)),
            ..Default::default()
        };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { width: 1024, height: 576, ..Default::default() });
        let line = "*conv_bar_qsave\nsavegame 890\nlsp 1,\":c;>1024,576,#000000\",0,0,204\nlsp 0,\":a;system\\saved.png\",10,20\nprint 1:click:csp 1:csp 0:print 1\nreturn\n";
        assert!(out.contains(line), "missing:\n{line}\nin:\n{out}");
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
        // With no place of its own: after the text, below the ruby gap.
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { wait_icon: Some((&icon, ["%183", "%184"])), icon_after_text: Some(9), ..Default::default() });
        let line = "getcursorpos %105,%146\nlsp 42,\":a/3,<50,50,500>,0;system\\icon.png\",%105,%146 + 9:print 1\n";
        assert!(out.contains(line), "missing:\n{line}\nin:\n{out}");
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
            "*conv_wait\nerasetextwindow 0\ngosub *conv_bar_draw\ntextbtnwait %9\nif %9 == -3 && %3640 == 0 goto *conv_wait_end\nif %9 == -3 goto *conv_wait\n",
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
            "if %2908 == 1 btn 1300,0,0,800,573,0,0\nif %2908 == 1 && %148 == 0 spmotion 34,0,0,0,0,100,100,0,0,0,100,0,0,100,100,0,256,0\nmov %148,1\n",
            "*conv_bar_lock\nmov %2908,1-%2908\nreturn\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }

    #[test]
    fn voices_stop_on_a_click_by_the_engine() {
        // The engine stops the voice (clickvoicestop); the click wait doesn't.
        let bar = ButtonBar { buttons: Vec::new(), top: None, ..Default::default() };
        let mut out = String::new();
        button_bar(&mut out, &bar, BarScreens { save_menu: true, ..Default::default() });
        assert!(out.contains("*conv_wait_end\ntexec\n"), "{out}");
        assert_eq!(
            crate::switches::apply(Switch::VoiceKeepsPlaying),
            crate::switches::Apply::Commands { on: "consset \"clickvoicestop\",0", off: "consset \"clickvoicestop\",1" }
        );
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
