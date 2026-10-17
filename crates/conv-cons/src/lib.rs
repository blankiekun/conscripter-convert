//! CONscripter output stage: turns an IR [`Script`] into `0.txt`.
//!
//! Targets CONscripter (ONScripter-Yuri fork) with Japanese text; the
//! NScripter commands used also run on ogapee's ONScripter. Things marked
//! "unverified" below follow the NScripter documentation but haven't been
//! checked in a running engine yet (see PLAN.md, Phase 0).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use conv_core::ir::{
    ArithOp, ConfirmDialog, Op, ScreenEffect, SliderSetting, Switch, Operand, Script, Ruby, SlideLayer, TextStyle, SpritePlacement, TextLink, TextWindow, TitleAction, TitleScreen,
    TransitionStyle, Var, ZoomStyle,
};

mod button_screen;
mod chapters;
mod choice_plates;
mod command_menu;
mod ease;
mod extras;
mod hold_movie;
mod roll;
pub mod lint;
mod menu;
mod motion;
mod movies;
mod music;
mod notice;
pub mod nsa;
mod picture_screen;
mod picture_extras;
mod picture_gallery;
mod picture_replay;
mod picture_sound;
mod picture_settings;
mod picture_turntable;
mod puzzle;
mod replay;
mod save_grid;
mod settings_pages;
mod switches;
mod line_log;
mod lua;
pub use lua::system_lua;
mod zoom;
pub mod webp;
mod vars;
mod ops;
pub use vars::clashes;

/// First effect number we define; 1 is NScripter's built-in "instant".
const FIRST_EFFECT: u32 = 2;
/// NScripter effect type 10 is a cross-fade; 18 a cross-fade shaped by a
/// mask image (dark parts first).
const FADE: u32 = 10;
const MASK_FADE: u32 = 18;
/// ONScripter's effect 99 runs the plugin named in its image argument.
const DLL: u32 = 99;
/// Sound effect channels we rotate through (0 is left for voices).
const SE_CHANNELS: std::ops::RangeInclusive<u32> = 1..=9;
/// Sprite slots 0-5 belong to the yes/no dialog, drawn over everything
/// else (ONScripter draws lower numbers on top); 6-33 (then
/// [`MENU_MORE_SPRITES`]) to the menus (title, save/load, settings,
/// backlog, [`button_screen`]s); 34-49 to the bars
/// (buttons, hotspots, texts).
const DIALOG_SPRITES: std::ops::Range<u32> = 0..6;
const MENU_SPRITES: std::ops::Range<u32> = 6..34;
/// More menu slots for the screens with many sprites ([`menu_slot`]):
/// numbers no game takes ([`conv_core::alloc::MENU_MORE_SPRITES`]), drawn
/// with `spz` under the menus' 6-33 and over the bars.
const MENU_MORE_SPRITES: std::ops::Range<u32> = conv_core::alloc::MENU_MORE_SPRITES;
/// How many slots [`menu_slot`] has.
const MENU_SLOTS: usize = (MENU_SPRITES.end - MENU_SPRITES.start + MENU_MORE_SPRITES.end - MENU_MORE_SPRITES.start) as usize;
/// The menus' slot `i` from the top: 6-33, then [`MENU_MORE_SPRITES`].
/// Follow its `lsp` with [`menu_z`].
fn menu_slot(i: usize) -> u32 {
    assert!(i < MENU_SLOTS, "more than {MENU_SLOTS} menu sprites");
    let i = i as u32;
    let n = MENU_SPRITES.end - MENU_SPRITES.start;
    if i < n { MENU_SPRITES.start + i } else { MENU_MORE_SPRITES.start + i - n }
}
/// What draws a [`menu_slot`] in its place, after each `lsp` (`csp`
/// forgets it): nothing for 6-33.
fn menu_z(slot: u32) -> String {
    if MENU_MORE_SPRITES.contains(&slot) { format!(":spz {slot},{}", MENU_SPRITES.end) } else { String::new() }
}
/// The bars use 34-42.
const BAR_SPRITES: std::ops::Range<u32> = 34..49;
/// Sprite slots for the name text and its plate: the text over the plate,
/// both over the character sprites. The UI slots (0-58) are never saved in
/// a meaningful state, so they may move; the game's (59 up) may not.
const NAME_TEXT_SPRITE: u32 = 49;
const NAME_PLATE_SPRITE: u32 = 50;
/// Sprite slots for clickable link words. With `windowback` and `humanz
/// 58`, slots up to 58 are drawn over the text window and the others
/// under it.
const LINK_SPRITES: std::ops::RangeInclusive<u32> = 51..=58;
/// The animation ([`Op::Animation`]), over the static.
const ANIMATION_SPRITE: u32 = 59;
/// The static ([`Op::Noise`]), over the overlay.
const NOISE_SPRITE: u32 = 60;
/// The overlay ([`Op::Overlay`]) over the character sprites.
const OVERLAY_SPRITE: u32 = 61;
/// Sprite layer n is NScripter sprite 62 + n.
const SPRITE_BASE: u32 = 62;
/// The lsp2 slot of a flash ([`Op::Flash`]), under the menus' and the top
/// bars' (0-23).
const FLASH_SPRITE2: u32 = 24;
/// The lsp2 slot of a fade to a colour ([`Op::ScreenFade`]), under the
/// flash's.
const FADE_SPRITE2: u32 = 26;
/// The click-wait icon's place ([`Script::wait_icon`]), set with each
/// window; the bar's wait routine shows it.
const WAIT_ICON_AT: [&str; 2] = ["%183", "%184"];
/// The highest sprite number (ONScripter has 1000); also the overlay
/// under the character sprites.
const MAX_SPRITE: u32 = 999;
/// The overlay's sprite (0 = none) and image, to show it again after
/// `csp -1`. Link scenes save them at OVERLAY_SAVED + depth.
const OVERLAY_SLOT: &str = "%180";
const OVERLAY_IMAGE: &str = "$180";
const OVERLAY_DEPTH: &str = "%181";
const OVERLAY_SAVED: u32 = 182;
/// The static's opacity (0 = none, else 1-255) and image, to show it again
/// after `csp -1`. Link scenes save them at NOISE_SAVED + depth (the depth
/// is the overlay's counter).
const NOISE_ALPHA: &str = "%116";
const NOISE_IMAGE: &str = "$116";
const NOISE_SAVED: u32 = 130;
/// The animation's opacity (0 = none, else 1-255) and image, to show it
/// again after `csp -1`. Link scenes save them at ANIMATION_SAVED + depth
/// (free locals between the game's own and the presets').
const ANIMATION_ALPHA: &str = "%117";
const ANIMATION_IMAGE: &str = "$117";
const ANIMATION_SAVED: u32 = 21;
/// Scratch variable receiving the clicked button (also used by nstrace).
const BUTTON_VAR: &str = "%9";
// Every range below is also in `conv_core::alloc`, which the build checks
// the game's variables against (the `vars` tests keep the two in step).
// Ranges claimed by modules: %140-%146, the command menu and the sliding
// bar ([`command_menu`], [`menu::button_bar`]); %147, %149 and $110-$113,
// the save grid ([`save_grid`]); %190-%289 and $200-$229, the tile puzzle's
// state ([`puzzle`]; locals when the global border is 800); %170-%179 and
// $107-$108, the backlog ([`menu::backlog`] or [`line_log`], never both);
// %183-%184, the click-wait icon's place ([`WAIT_ICON_AT`]).
// The settings switches' flags: %3640-%3647, %2910-%2919, %2950-%2951 and
// %4116-%5095 ([`switches`]). %2960, the title's openings
// ([`TITLE_SHOWN`]). %5200 up: an NSS game's `#` flags past %969 (the
// nitroplus compiler's `GLOBAL_OVERFLOW`). %130-%139, the picture extras'
// gallery ([`picture_gallery`]; only at the title). %2961, the picture
// extras' music ([`picture_sound`]). %150, a scene replay of the picture
// extras is playing, %2962, back to its list after a reset, %151, saving
// locked meanwhile ([`picture_replay`], [`menu::NO_SAVE`]). %2963-%2964,
// the picture extras' turntable page and entry ([`picture_turntable`]).
/// The menus' hover sound ([`Script::menu_sounds`]; "" none), which a
/// screen with a hover sound of its own puts back after it.
pub(crate) const MENU_HOVER: &str = "$118";
/// NScripter variables: locals from %10, globals from %200. `globalon`
/// makes %200 and up persist across games.
const LOCAL_VAR_BASE: u32 = 10;
const GLOBAL_VAR_BASE: u32 = 200;
/// Variables the script declares (`;$V`; ONScripter's default is 4096):
/// %4096 up hold the settings switches' flags ([`switches`]).
pub(crate) const VARIABLE_RANGE: u32 = 8192;
/// Labels of the title menu loop and of its Start button's target.
/// Click wait with everything hidden ([`Op::ClickWait`]).
const CLICK_WAIT: &str = "conv_click_wait";
const TITLE_DRAW: &str = "conv_title_draw";
const TITLE_MENU: &str = "conv_title";
const TITLE_START: &str = "conv_title_start";
/// A global, 0 until the title has been shown in this run (the define
/// section clears it; `reset` keeps it): which opening plays.
const TITLE_SHOWN: &str = "%2960";
/// The title's sprites, without its music (back from the picture extras).
const TITLE_SPRITES: &str = "conv_title_sprites";
/// Colours of clickable words (measured on the original): not yet
/// visited, visited, each also under the mouse pointer.
const LINK_COLOR: &str = "#FEE092";
const LINK_HOVER_COLOR: &str = "#FEC95E";
const LINK_VISITED_COLOR: &str = "#DCB8FE";
const LINK_VISITED_HOVER_COLOR: &str = "#CB95FE";
/// Underlines of link words (and their shadows), two sprites per word. The
/// menus' slots: they are drawn again at every wait of the page, so a menu
/// opened from the bar can't leave them wrong.
const LINK_UNDERLINE_SPRITES: std::ops::Range<u32> = MENU_SPRITES.start..MENU_SPRITES.start + 2 * 8;
/// Global flags: link target n (in order of first use) was visited.
const LINK_VISITED_BASE: u32 = 2000;
/// Global flags: scene n of the scene replay is unlocked.
const SCENE_UNLOCKED_BASE: u32 = 3000;
/// Global flags: progress flag n ([`Op::Progress`]) is set.
const PROGRESS_BASE: u32 = 3600;
/// Option n of an [`Op::RuntimeChoice`] whose text is known only at run
/// time: its text in `$` n + this, its x on a plate in `%` n + this.
const RUNTIME_OPTION_BASE: u32 = 3632;
/// How many options of a choice can be runtime ones (alloc.rs reserves
/// 3632-3637; NSS choices have six at most, `SetChoice06`).
const RUNTIME_OPTIONS: usize = 6;
/// An [`Op::RuntimeBacklogLine`]'s text.
const RUNTIME_LOG: u32 = 3638;
/// Link scenes nest at most this deep: inside the deepest, link words
/// aren't buttons. The original has no limit, but link scenes that link to
/// each other (the encyclopedia) would nest without end, and each level
/// takes a `pushscreen` (32 at most) and a slot in the per-level variables
/// (OVERLAY_SAVED + depth must stay below %190). The game's own scenes
/// nest five deep.
const MAX_LINK_DEPTH: u32 = 7;
/// Link scenes open (gosubs from link pages), for [`Op::ReturnToRoot`].
const LINK_NEST: &str = "%118";
/// Set while [`Op::ReturnToRoot`] unwinds: which one to go on at.
const ROOT_RESUME: &str = "%119";
/// Returns from one more link scene, or at the root jumps to the
/// [`ROOT_RESUME`] point.
const UNWIND: &str = "conv_unwind";
/// Global flags: CG n ([`Op::CgSeen`], up to 9 × 64) was seen; above the
/// link flags, below 4096 (the variable range).
const CG_SEEN_BASE: u32 = 2200;
const TEXT_COLOR: &str = "#FFFFFF";

/// Renders the script as NScripter source text (UTF-8; use [`encode`] to
/// produce the Shift-JIS bytes ONScripter reads).
pub fn emit(script: &Script) -> String {
    let effects = fade_effects(script);
    let has_ruby = script.ops.iter().any(|op| matches!(op, Op::Text { ruby, .. } if !ruby.is_empty()));
    let has_overlay = script.ops.iter().any(|op| matches!(op, Op::Overlay { .. }));
    let has_noise = script.ops.iter().any(|op| matches!(op, Op::Noise(_)));
    let has_chapters = script.button_bar.as_ref().and_then(|b| b.top.as_ref()).is_some_and(|t| t.chapter_history.is_some());
    let has_animation = script.ops.iter().any(|op| matches!(op, Op::Animation(_)));
    // What removes a fade to a colour, after `csp -1` and `popscreen`.
    let clear_fade = if script.ops.iter().any(|op| matches!(op, Op::ScreenFade { .. })) {
        format!(":csp2 {FADE_SPRITE2}")
    } else {
        String::new()
    };
    let root_returns = script.ops.iter().filter(|op| matches!(op, Op::ReturnToRoot)).count();
    // The counter of open link scenes, where one is kept.
    let link_depth = if script.button_bar.as_ref().is_some_and(|b| b.top.is_some()) {
        Some(menu::SCENE_DEPTH)
    } else if has_overlay || has_noise || has_animation {
        Some(OVERLAY_DEPTH)
    } else if root_returns > 0 {
        Some(LINK_NEST)
    } else {
        None
    };
    // The replay is reached from the title's extras.
    let has_replay = script.title_screen.is_some() && script.extras.as_ref().is_some_and(|e| e.replay.is_some());
    let picture_replay = script.title_screen.is_some() && script.picture_extras.as_ref().is_some_and(|e| e.replay.is_some());
    let root_resumes = 0;
    let puzzles = 0;
    let button_screens = 0;
    let plate_choices = 0;
    let picture_screens = 0;
    let has_vertical = script.window.iter().chain(script.ops.iter().filter_map(|op| match op {
        Op::SetWindow(w) => Some(w),
        Op::Choice { style: Some(s), .. } | Op::RuntimeChoice { style: Some(s), .. } => Some(&s.window),
        _ => None,
    }))
    .any(|w| w.vertical);
    // Shows the overlay and the static again after `csp -1`.
    let mut restore_overlay = String::new();
    if has_overlay {
        writeln!(restore_overlay, "if {OVERLAY_SLOT} != 0 lsp {OVERLAY_SLOT},{OVERLAY_IMAGE},0,0").unwrap();
    }
    if has_noise {
        writeln!(restore_overlay, "if {NOISE_ALPHA} != 0 lsp {NOISE_SPRITE},{NOISE_IMAGE},0,0,{NOISE_ALPHA}").unwrap();
    }
    if has_animation {
        writeln!(restore_overlay, "if {ANIMATION_ALPHA} != 0 lsp {ANIMATION_SPRITE},{ANIMATION_IMAGE},0,0,{ANIMATION_ALPHA}").unwrap();
    }
    let mut out = String::new();

    let border = script.global_border.unwrap_or(GLOBAL_VAR_BASE);
    let mut mode = match (script.width, script.height) {
        (800, 600) => ";mode800".to_string(),
        (640, 480) => ";mode640".to_string(),
        (w, h) => format!(";$S{w},{h}"),
    };
    if script.global_border.is_some() {
        // Variables below the border are saved with each save slot.
        write!(mode, ",value{border}").unwrap();
    }
    write!(mode, ",V{VARIABLE_RANGE}").unwrap();
    writeln!(out, "{mode}").unwrap();
    writeln!(out, "*define").unwrap();
    // Read arc.nsa (without this ONScripter only looks for .sar archives;
    // loose files still work either way).
    writeln!(out, "nsa").unwrap();
    writeln!(out, "globalon").unwrap();
    if lua::uses_processes(script) {
        for c in lua::COMMANDS {
            writeln!(out, "luasub {c}").unwrap();
        }
    }
    if script.window.is_some() {
        writeln!(out, "windowback").unwrap();
        writeln!(out, "humanz {}", LINK_SPRITES.end()).unwrap();
    }
    if script.backlog.is_some() || script.line_log.is_some() {
        // Wheel and arrow keys reach btnwait (backlog scrolling); keep more
        // pages of history than the default 16.
        writeln!(out, "usewheel").unwrap();
        writeln!(out, "maxkaisoupage 100").unwrap();
    }
    if script.backlog.is_some() {
        // Text lines carry the speaker and voice as a tag for the backlog
        // (gettaglog); tags are only kept with a pretextgosub.
        writeln!(out, "pretextgosub *{}", menu::PRETEXT).unwrap();
    }
    if let Some(bar) = &script.button_bar {
        // Every click wait runs the bar routine instead of waiting itself.
        writeln!(out, "textgosub *{}", menu::WAIT).unwrap();
        if menu::has_back_to_choice(bar, script.command_menu.as_ref()) {
            writeln!(out, "loadgosub *{}", menu::LOADED).unwrap();
        } else if bar.top.is_some() {
            writeln!(out, "loadgosub *{}", menu::SCENE_LOADED).unwrap();
        }
    }
    // CONscripter converts the caption to UTF-8 for the window title.
    if has_ruby {
        // Draw ruby (readings) in the line gap above the text. Text then
        // starts a line gap lower, which setwindow and the backlog undo.
        writeln!(out, "rubyon").unwrap();
    }
    let caption: String = script.title.chars().filter(|&c| c != '"').collect();
    writeln!(out, "caption \"{}\"", caption.trim()).unwrap();
    // Numbered effects while they fit in NScripter's 2-255; past that every
    // print carries its effect's parameters itself (`print 10,300`).
    let numbered = effects.len() <= 254;
    let effects: BTreeMap<(u32, Effect), String> = effects
        .into_iter()
        .map(|((ms, kind), n)| {
            let params = match &kind {
                Effect::Fade => format!("{FADE},{ms}"),
                Effect::Mask(m) => format!("{MASK_FADE},{ms},\"{}\"", path(m)),
                // Effect 15: a pixel is new once the progress passes its level.
                Effect::HardMask(m) => format!("15,{ms},\"{}\"", path(m)),
                Effect::Builtin(k) => format!("{k},{ms}"),
                Effect::Dll(d) => format!("{DLL},{ms},\"{d}\""),
            };
            if numbered {
                writeln!(out, "effect {n},{params}").unwrap();
            }
            ((ms, kind), if numbered { n.to_string() } else { params })
        })
        .collect();
    // Read-text tracking (kidoku.dat, written on save and quit), for
    // skipping read text only.
    if script.settings.as_ref().is_some_and(|s| s.skip_unread_label.is_some()) || script.picture_settings.as_ref().is_some_and(picture_settings::skips_unread) {
        writeln!(out, "kidokuskip").unwrap();
    }
    if script.title_screen.as_ref().is_some_and(|t| !t.openings.is_empty()) {
        writeln!(out, "mov {TITLE_SHOWN},0").unwrap();
    }
    writeln!(out, "game").unwrap();
    writeln!(out, "*start").unwrap();
    // The text speed is a setting when there is a settings menu.
    let has_settings = script.settings.is_some() || script.picture_settings.is_some();
    let speed = if has_settings { menu::TEXT_MS } else { "20" };
    let voice_switches = script.settings.as_ref().is_some_and(|s| !s.voices.is_empty()) || script.picture_settings.as_ref().is_some_and(|s| s.voice_window.is_some());
    let duck = uses_switch(script, Switch::BgmFullForVoice);
    if let Some(s) = &script.settings {
        menu::settings_init(&mut out, s);
    }
    if let Some(s) = &script.picture_settings {
        picture_settings::init(&mut out, s);
    }
    if let Some(m) = &script.menu_sounds {
        writeln!(out, "mov {MENU_HOVER},\"{}\":selectvoice \"\",{MENU_HOVER},\"\"", m.hover.as_deref().map(path).unwrap_or_default()).unwrap();
        writeln!(out, "btnse \"{}\"", m.confirm.as_deref().map(path).unwrap_or_default()).unwrap();
    }
    // setwindow redraws the screen, so it must come after `game` (ONScripter
    // crashes if it runs in the define section).
    if let Some(w) = &script.window {
        writeln!(out, "{}", setwindow(w, speed, has_ruby, has_vertical, true, script)).unwrap();
        // Keep the window up while sprites change and pages are printed
        // (by default every print hides it until the next text, which
        // flickers); the script hides it explicitly with textoff.
        writeln!(out, "erasetextwindow 0").unwrap();
    }
    // An icon with no place of its own in any window: drawn after the text
    // by the engine, at line and page waits alike.
    // `Some(drop)`: the icon goes after the text, `drop` below the place
    // `getcursorpos` gives (the line's top; rubyon draws the text a line gap
    // lower: the first window's, as the bar has one routine).
    let icon_after_text = script.wait_icon.as_ref().and_then(|_| {
        let windows = script.window.iter().chain(script.ops.iter().filter_map(|op| match op {
            Op::SetWindow(w) => Some(w),
            _ => None,
        }));
        let first = windows.clone().next()?;
        windows.into_iter().all(|w| w.wait_icon_at.is_none()).then_some(if has_ruby { first.line_gap as i32 } else { 0 })
    });
    if let (Some(i), Some(_)) = (&script.wait_icon, icon_after_text) {
        let ms: Vec<String> = i.frame_ms.iter().map(u32::to_string).collect();
        for n in 0..2 {
            writeln!(out, "setcursor {n},\":a/{},<{}>,0;{}\",0,0", ms.len(), ms.join(","), path(&i.image)).unwrap();
        }
    }

    let save_menu = script.save_menu.is_some() || script.save_grid.is_some();
    let effect = |ms: u32| if ms == 0 { "1".to_string() } else { effects[&(ms, Effect::Fade)].clone() };
    notice::emit(&mut out, script.width, script.height);
    if let Some(t) = &script.title_screen {
        title_screen(&mut out, t, &effect(t.fade_ms), save_menu, has_settings, script.confirm.as_ref(), script.extras.is_some(), border, picture_replay);
    }
    let back_to_choice = script.button_bar.as_ref().is_some_and(|b| menu::has_back_to_choice(b, script.command_menu.as_ref()));
    if back_to_choice {
        // A new game has no choice to go back to.
        writeln!(out, "delsavegame {}", menu::BACK_SLOT).unwrap();
    }
    if script.save_grid.is_some() {
        // A new game's play time.
        writeln!(out, "{}", save_grid::NEW_GAME).unwrap();
    }
    let se = SeChannels { volumes: script.ops.iter().any(|op| matches!(op, Op::SoundVolume { .. })), ..Default::default() };
    // The next sound effect isn't played while skipping (Op::SeNotSkipped).
    let se_not_skipped = false;
    // The next static's fade in (Op::NoiseFadeIn).
    let noise_fade_in = None;
    let fading = BTreeMap::new();
    // The next move's or zoom's curve ([`Op::MoveEase`]), and the eased
    // subroutines used (zoom, ease, exponent).
    let move_ease: Option<(u8, u32)> = None;
    let move_nowait = false;
    let move_arc = None;
    let eased = std::collections::BTreeSet::new();
    let needs_click_wait = false;
    let plate_visible = false;
    let link_pages = 0;
    let mut link_targets: Vec<&str> = Vec::new();
    for op in &script.ops {
        if let Op::Text { links, .. } = op {
            for t in links.iter().filter_map(|l| l.target.as_deref()) {
                if !link_targets.contains(&t) {
                    link_targets.push(t);
                }
            }
        }
    }
    // Effect numbers of an open transition, printed in turn: display
    // commands only update the buffer until then.
    // The lines that show an armed transition (prints and what goes between).
    let transition: Option<Vec<String>> = None;
    // The active text window (file order; presets are called right before
    // the text that uses them) and the lines already on its page.
    let window = script.window.clone();
    let page_rows = 0;
    // The backlog tag of the page being written: its first speaker and voice.
    let page_tag: Option<String> = None;
    // Layers an animation zooms: spzoom stays through lsp, so a picture
    // loaded there later is drawn at 100 again.
    let zoomed_layers: std::collections::HashSet<u32> = script
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::AnimatedSprite { layer, zoom, .. } if *zoom != (100, 100) => Some(*layer),
            _ => None,
        })
        .collect();
    let mut e = ops::Emitter { script, out, effects, has_ruby, has_overlay, has_noise, has_chapters, has_animation, root_returns, has_replay, has_vertical, border, voice_switches, duck, back_to_choice, speed, link_depth, clear_fade, restore_overlay, link_targets, zoomed_layers, root_resumes, puzzles, button_screens, plate_choices, picture_screens, se, se_not_skipped, noise_fade_in, move_ease, move_nowait, move_arc, eased, needs_click_wait, plate_visible, link_pages, transition, window, page_rows, page_tag, text_goes_on: false, fading };
    for (i, op) in script.ops.iter().enumerate() {
        e.op(i, op);
    }
    let ops::Emitter { mut out, effects, eased, needs_click_wait, .. } = e;
    let effect = |ms: u32| if ms == 0 { "1".to_string() } else { effects[&(ms, Effect::Fade)].clone() };
    writeln!(out, "end").unwrap();
    if !eased.is_empty() {
        // The eased moves' curves, defined as the game starts (saves keep them).
        let defs: String = eased.iter().map(|&(e, n)| ease::curve_def(e, n) + "\n").collect();
        let at = out.find("\n*start\n").map(|i| i + "\n*start\n".len()).expect("*start");
        out.insert_str(at, &defs);
    }
    if root_returns > 0 {
        // Reached from a link page once its scene returned and the page
        // put its screen back.
        writeln!(out, "*{UNWIND}").unwrap();
        writeln!(out, "if {LINK_NEST} > 0 return").unwrap();
        writeln!(out, "mov {0},{ROOT_RESUME}:mov {ROOT_RESUME},0", menu::TMP).unwrap();
        for n in 1..=root_returns {
            writeln!(out, "if {} == {n} goto *conv_root{n}", menu::TMP).unwrap();
        }
    }
    if needs_click_wait {
        // %9 = 1: skip mode passes. Auto mode waits a while instead.
        writeln!(out, "*{CLICK_WAIT}").unwrap();
        writeln!(out, "textoff").unwrap();
        if script.button_bar.is_some() {
            writeln!(out, "gosub *{}", menu::BAR_HIDE).unwrap();
        }
        writeln!(out, "isskip %108").unwrap();
        writeln!(out, "if {BUTTON_VAR} == 1 && %108 == 1 return").unwrap();
        writeln!(out, "if %108 == 2 delay 2000:return").unwrap();
        writeln!(out, "click").unwrap();
        writeln!(out, "return").unwrap();
    }
    let scene_stack = script.button_bar.as_ref().is_some_and(|b| b.top.is_some());
    if let Some(g) = &script.save_grid {
        save_grid::emit(&mut out, g, script.confirm.as_ref(), script.width, script.height, scene_stack);
    } else if let Some(m) = &script.save_menu {
        menu::save_menu(&mut out, m, script.confirm.as_ref(), scene_stack);
    }
    if let Some(b) = &script.button_bar {
        let screens = menu::BarScreens {
            save_menu,
            settings: has_settings,
            backlog: script.backlog.is_some() || script.line_log.is_some(),
            replay: has_replay,
            picture_replay,
            command_menu: script.command_menu.as_ref(),
            confirm: script.confirm.as_ref(),
            width: script.width,
            height: script.height,
            wait_icon: script.wait_icon.as_ref().map(|i| (i, WAIT_ICON_AT)),
            icon_after_text,
            game_speed: script
                .picture_settings
                .as_ref()
                .filter(|s| s.sliders.iter().any(|l| matches!(l.setting, SliderSetting::GameSpeed { .. })))
                .map(|_| picture_settings::GAME_SPEED),
        };
        menu::button_bar(&mut out, b, screens);
        if let Some(m) = &script.command_menu {
            command_menu::emit(&mut out, m, b, picture_replay);
        }
        if let Some(h) = b.top.as_ref().and_then(|t| t.chapter_history.as_ref()) {
            chapters::emit(&mut out, h);
        }
    }
    if let Some(s) = &script.picture_settings {
        picture_settings::emit(&mut out, s, border);
    } else if let Some(m) = &script.settings {
        let replay = script.extras.as_ref().and_then(|e| e.replay.as_ref()).filter(|_| has_replay);
        menu::settings_menu(&mut out, m, script.confirm.as_ref(), replay.map(|r| (r.leave_label.as_str(), r.leave_question.as_str())), script.version.as_deref().map(|v| (v, script.height)));
    }
    if let Some(l) = &script.line_log {
        line_log::emit(&mut out, l, script.single_byte_text);
    } else if let Some(b) = &script.backlog {
        menu::backlog(&mut out, b, has_ruby);
    }
    if let Some(d) = &script.confirm {
        menu::confirm(&mut out, d);
    }
    if let (Some(m), Some(t)) = (&script.extras, &script.title_screen) {
        extras::extras(&mut out, m, &effect(t.fade_ms), script.settings.is_some());
    }
    if let (Some(e), Some(_)) = (&script.picture_extras, &script.title_screen) {
        picture_extras::emit(&mut out, e, border, script.width, script.height);
    }
    out
}

/// The title menu: buttons are sprites over the picture, faded in together;
/// Start clears them and falls through to the script that follows. Load
/// opens the save menu's load screen if there is one, else ONScripter's.
#[allow(clippy::too_many_arguments)]
/// `replay_back`: a picture extras' replay may `reset` back to its list
/// ([`picture_replay::BACK_TO_LIST`]).
fn title_screen(out: &mut String, t: &TitleScreen, fade: &str, save_menu: bool, settings: bool, confirm: Option<&ConfirmDialog>, extras: bool, border: u32, replay_back: bool) {
    if replay_back {
        writeln!(out, "{}", picture_replay::after_reset()).unwrap();
    }
    let buttons: Vec<_> = t.buttons.iter().zip(MENU_SPRITES).collect();
    // %105 = 1 while there is a quick save (for Continue).
    let quick = buttons.iter().any(|(b, _)| b.action == TitleAction::Continue);
    let quick_exists = format!("savefileexist {},{}", menu::TMP, menu::QUICK_SLOT);
    let picture_extras = t.buttons.iter().any(|b| b.action == TitleAction::Extras);
    if extras || picture_extras {
        writeln!(out, "*{}", extras::TITLE_BACK).unwrap();
    }
    // Effect 0 = update the buffer only; the print shows it all.
    writeln!(out, "bg \"{}\",0", path(&t.background)).unwrap();
    title_openings(out, &t.openings, border);
    // Back from the picture extras: the music goes on.
    if picture_extras {
        writeln!(out, "if {TITLE_SHOWN} == 2 mov {TITLE_SHOWN},1:goto *{TITLE_SPRITES}").unwrap();
    }
    if !t.openings.is_empty() {
        writeln!(out, "mov {TITLE_SHOWN},1").unwrap();
    }
    writeln!(out, "*{TITLE_DRAW}").unwrap();
    if let Some(bgm) = &t.bgm {
        writeln!(out, "bgm \"{}\"", path(bgm)).unwrap();
    }
    if picture_extras {
        writeln!(out, "*{TITLE_SPRITES}").unwrap();
    }
    title_sprites(out, t, &buttons, quick, None, true, border);
    writeln!(out, "print {fade}").unwrap();
    writeln!(out, "*{TITLE_MENU}").unwrap();
    if let Some(se) = &t.hover_se {
        writeln!(out, "selectvoice \"\",\"{}\",\"\"", path(se)).unwrap();
    }
    if quick {
        writeln!(out, "{quick_exists}").unwrap();
    }
    if t.hide_without_saves {
        writeln!(out, "{}", saves_check()).unwrap();
    }
    let press = if t.pressed { ",2" } else { "" };
    let clickable: Vec<_> = buttons.iter().filter(|(b, _)| b.action != TitleAction::Unavailable).collect();
    for (n, (b, slot)) in clickable.iter().enumerate() {
        let only = title_button_shown(t, b, border);
        writeln!(out, "{only}spbtn {slot},{}{press}", n + 1).unwrap();
    }
    writeln!(out, "btnwait {BUTTON_VAR}").unwrap();
    if t.hover_se.is_some() {
        writeln!(out, "selectvoice \"\",{MENU_HOVER},\"\"").unwrap();
    }
    for (n, (b, _)) in clickable.iter().enumerate() {
        let then = match b.action {
            TitleAction::Start => format!("goto *{TITLE_START}"),
            TitleAction::Continue if t.hide_without_saves => format!("csp -1:loadgame {}", menu::HAS_DATA),
            TitleAction::Continue => format!("csp -1:loadgame {}", menu::QUICK_SLOT),
            TitleAction::Load if save_menu => {
                format!("csp -1:mov {},0:gosub *{}:goto *{TITLE_DRAW}", menu::IN_GAME, menu::LOAD)
            }
            // The title stays up: the extras cross-fade in over it.
            TitleAction::Extras => format!("goto *{}", picture_extras::EXTRAS),
            TitleAction::Settings if extras => format!("csp -1:goto *{}", extras::EXTRAS),
            TitleAction::Settings if settings => {
                format!("csp -1:mov {},0:gosub *{}:goto *{TITLE_DRAW}", menu::IN_GAME, menu::SETTINGS)
            }
            TitleAction::Settings => continue,
            // The built-in load menu; it comes back here when cancelled.
            TitleAction::Load => format!("systemcall load:goto *{TITLE_MENU}"),
            TitleAction::Exit if confirm.is_some() && !t.exit_without_asking => "goto *conv_title_exit".to_string(),
            TitleAction::Exit => "end".to_string(),
            TitleAction::Link(url) => format!("openurl \"{url}\":goto *{TITLE_MENU}"),
            TitleAction::Unavailable => unreachable!(),
        };
        let se = b.click_se.as_ref().map_or(String::new(), |se| format!("dwave {},\"{}\":", SE_CHANNELS.start(), path(se)));
        writeln!(out, "if {BUTTON_VAR} == {} {se}{then}", n + 1).unwrap();
    }
    writeln!(out, "goto *{TITLE_MENU}").unwrap();
    // An opening: the music, then the movie (the music going on), which
    // ends on the title as the sprites show it.
    for (i, o) in t.openings.iter().enumerate() {
        writeln!(out, "*conv_title_opening{i}").unwrap();
        // Back from the extras, the music is playing.
        if let Some(bgm) = t.bgm.as_ref().filter(|_| !o.after_extras) {
            writeln!(out, "bgm \"{}\"", path(bgm)).unwrap();
        }
        if let Some(se) = &o.sound {
            writeln!(out, "dwave {},\"{}\"", SE_CHANNELS.start(), path(se)).unwrap();
        }
        // The title is loaded before the movie, so the screen redrawn as
        // it ends already has its buttons; the glow starts moving then.
        title_sprites(out, t, &buttons, quick, o.glow_keys.as_deref(), false, border);
        writeln!(out, "mpegplay \"{}\",1,1", path(&o.movie)).unwrap();
        if let Some(g) = &t.glow {
            writeln!(out, "spmotion {},1,{}", MENU_SPRITES.end - 1, glow_keys(o.glow_keys.as_deref().unwrap_or(&g.keys))).unwrap();
        }
        writeln!(out, "print 1").unwrap();
        writeln!(out, "goto *{TITLE_MENU}").unwrap();
    }
    if let Some(c) = confirm.filter(|_| !t.exit_without_asking) {
        writeln!(out, "*conv_title_exit").unwrap();
        writeln!(out, "{}", menu::confirm_call(&c.quit, Switch::ConfirmQuit)).unwrap();
        writeln!(out, "if {} == 1 end", menu::CONFIRM).unwrap();
        writeln!(out, "goto *{TITLE_MENU}").unwrap();
    }
    writeln!(out, "*{TITLE_START}").unwrap();
    writeln!(out, "csp -1").unwrap();
    if t.bgm.is_some() && t.start_bgm_fade_ms > 0 {
        writeln!(out, "bgmfadeout {}:bgmstop:bgmfadeout 0", t.start_bgm_fade_ms).unwrap();
    }
    writeln!(out, "bg black,{fade}").unwrap();
    if t.start_wait_ms > 0 {
        writeln!(out, "wait {}", t.start_wait_ms).unwrap();
    }
}

/// Which opening plays: the first whose conditions hold, a random pool
/// tried as one where its first member stands (%105 counts the pool's
/// openings that may play, %9 is the one picked).
fn title_openings(out: &mut String, openings: &[conv_core::ir::TitleOpening], border: u32) {
    let cond = |o: &conv_core::ir::TitleOpening| {
        let shown = if o.after_extras {
            2
        } else if o.at_launch {
            0
        } else {
            1
        };
        let mut c = vec![format!("{TITLE_SHOWN} == {shown}")];
        c.extend(o.when.iter().map(|(v, value)| format!("{} == {value}", var_name(*v, border))));
        c.join(" && ")
    };
    let then = |i: usize, o: &conv_core::ir::TitleOpening| {
        let sets: String = o.sets.iter().map(|v| format!("mov {},1:", var_name(*v, border))).collect();
        format!("mov {TITLE_SHOWN},1:{sets}goto *conv_title_opening{i}")
    };
    let mut pool_done = false;
    for (i, o) in openings.iter().enumerate() {
        if !o.random {
            writeln!(out, "if {} {}", cond(o), then(i, o)).unwrap();
            continue;
        }
        if pool_done {
            continue;
        }
        pool_done = true;
        let pool: Vec<(usize, &conv_core::ir::TitleOpening)> = openings.iter().enumerate().filter(|(_, o)| o.random).collect();
        writeln!(out, "mov {},0", menu::TMP).unwrap();
        for (_, o) in &pool {
            writeln!(out, "if {} inc {}", cond(o), menu::TMP).unwrap();
        }
        writeln!(out, "if {} > 0 rnd {BUTTON_VAR},{}", menu::TMP, menu::TMP).unwrap();
        writeln!(out, "mov {},0", menu::TMP).unwrap();
        for (k, o) in &pool {
            writeln!(out, "if {} && {BUTTON_VAR} == {} {}", cond(o), menu::TMP, then(*k, o)).unwrap();
            writeln!(out, "if {} inc {}", cond(o), menu::TMP).unwrap();
        }
    }
}

fn glow_keys(keys: &[(u32, u32)]) -> String {
    motion::alpha_keys(keys)
}

/// The title's glow (with `keys` in place of its own; held at its first
/// alpha unless `moving`) and buttons.
#[allow(clippy::too_many_arguments)]
fn title_sprites(out: &mut String, t: &TitleScreen, buttons: &[(&conv_core::ir::TitleButton, u32)], quick: bool, keys: Option<&[(u32, u32)]>, moving: bool, border: u32) {
    let cells = if t.pressed { 3 } else { 2 };
    if let Some(g) = &t.glow {
        // Behind the buttons, which take the first menu sprites.
        let slot = MENU_SPRITES.end - 1;
        let keys = keys.unwrap_or(&g.keys);
        if moving {
            writeln!(out, "lsp {slot},\":c;{}\",0,0:spmotion {slot},1,{}", path(&g.image), glow_keys(keys)).unwrap();
        } else {
            let alpha = keys.first().map_or(255, |&(_, a)| a.min(255));
            writeln!(out, "lsp {slot},\":c;{}\",0,0,{alpha}", path(&g.image)).unwrap();
        }
    }
    if t.hide_without_saves {
        writeln!(out, "{}", saves_check()).unwrap();
    }
    if quick {
        writeln!(out, "savefileexist {},{}", menu::TMP, menu::QUICK_SLOT).unwrap();
    }
    for (b, slot) in buttons {
        let image = if b.action == TitleAction::Unavailable { format!(":a;{}", path(&b.image)) } else { format!(":a/{cells},0,3;{}", path(&b.image)) };
        if b.action == TitleAction::Continue && !t.hide_without_saves {
            let only = b.shown_by.map_or(String::new(), |v| format!("if {} != 0 ", var_name(v, border)));
            writeln!(out, "{only}lsp {slot},\"{image}\",{},{}", b.x, b.y).unwrap();
            writeln!(out, "if {} == 0 lsp {slot},\"{image}\",{},{},128", menu::TMP, b.x, b.y).unwrap();
            continue;
        }
        let only = title_button_shown(t, b, border);
        writeln!(out, "{only}lsp {slot},\"{image}\",{},{}", b.x, b.y).unwrap();
    }
}

/// [`TitleScreen::hide_without_saves`]: the save Continue loads into
/// [`menu::HAS_DATA`], 0 when there is none (the user's slots are 1-100).
fn saves_check() -> String {
    let (has, tmp, slot) = (menu::HAS_DATA, menu::TMP, menu::SLOT);
    format!(
        "mov {has},0\nfor {slot} = 1 to 100\nif {has} == 0 savefileexist {tmp},{slot}:if {tmp} == 1 mov {has},{slot}\nnext\n\
         if {n} > 0 savefileexist {tmp},{n}:if {tmp} == 1 mov {has},{n}\n\
         savefileexist {tmp},{q}:if {tmp} == 1 mov {has},{q}",
        n = menu::NEWEST,
        q = menu::QUICK_SLOT
    )
}

/// The `if …` before a title button's `lsp` and `spbtn` (empty: always).
fn title_button_shown(t: &TitleScreen, b: &conv_core::ir::TitleButton, border: u32) -> String {
    let mut conds = Vec::new();
    match b.action {
        TitleAction::Continue | TitleAction::Load if t.hide_without_saves => conds.push(format!("{} > 0", menu::HAS_DATA)),
        TitleAction::Continue => conds.push(format!("{} == 1", menu::TMP)),
        // NScripter's `if` has no brackets: either flag, alone.
        TitleAction::Extras if !t.extras_shown_by.is_empty() => {
            return format!("if {} ", t.extras_shown_by.iter().map(|v| format!("{} != 0", var_name(*v, border))).collect::<Vec<_>>().join(" || "));
        }
        _ => {}
    }
    if let Some(v) = b.shown_by {
        conds.push(format!("{} != 0", var_name(v, border)));
    }
    if conds.is_empty() { String::new() } else { format!("if {} ", conds.join(" && ")) }
}

/// `setwindow3` for a window (`setwindow` without flushing the text
/// history, which games switching windows mid-story would lose): text x,y, columns,rows, font w,h, char
/// spacing, line gap, ms per character, bold, shadow, window image, x,y.
/// `speed`: milliseconds per character (a number or a variable).
/// `ruby`: `rubyon` is set, which draws text a line gap lower (the ruby
/// goes above it), so the window starts a line gap higher; vertical lines
/// it moves left by the line gap (the ruby goes at their right), so a
/// vertical window starts a line gap further right. The line's own gap
/// then holds the ruby, as in the original.
/// `any_vertical`: some window is vertical, so a horizontal one must switch
/// back.
/// A [`Script::single_byte_text`] window sets its letters by the font
/// alone: no spacing (CONscripter adds half of it to each half-width
/// letter of a proportional font).
fn setwindow(w: &TextWindow, speed: &str, ruby: bool, any_vertical: bool, shadow: bool, script: &Script) -> String {
    let mut line = setwindow_only(w, speed, ruby, any_vertical, shadow, u32::from(!script.single_byte_text));
    // Where the bar's wait routine shows the click-wait icon.
    if let (Some(_), Some((x, y))) = (&script.wait_icon, w.wait_icon_at) {
        line.push_str(&format!("\nmov {},{x}:mov {},{y}", WAIT_ICON_AT[0], WAIT_ICON_AT[1]));
    }
    line
}

/// [`setwindow`]'s window, without the icon.
/// `spacing`: between the letters of a horizontal window.
fn setwindow_only(w: &TextWindow, speed: &str, ruby: bool, any_vertical: bool, shadow: bool, spacing: u32) -> String {
    let shadow = shadow as u8;
    if w.vertical {
        // tateyoko first: it clears the page using the old window's size.
        // Lines are columns: x counts them, y the characters down each.
        let text_x = if ruby { w.text_x + w.line_gap as i32 } else { w.text_x };
        return format!(
            "tateyoko 1:setwindow3 {text_x},{},{},{},{},{},{},0,{speed},0,{shadow},\":a;{}\",{},{}",
            w.text_y, w.rows, w.columns, w.font_size, w.font_size, w.line_gap, path(&w.image), w.x, w.y
        );
    }
    let text_y = if ruby { w.text_y - w.line_gap as i32 } else { w.text_y };
    format!(
        "{}setwindow3 {},{text_y},{},{},{},{},{spacing},{},{speed},0,{shadow},\":a;{}\",{},{}",
        if any_vertical { "tateyoko 0:" } else { "" },
        w.text_x, w.columns, w.rows, w.font_size, w.font_size, w.line_gap, path(&w.image), w.x, w.y
    )
}

/// A sprite's transparency mode: 'a' (the PNG's alpha channel), or 'c'
/// (opaque) for a picture without one (JPEG): with 'a' NScripter takes its
/// right half for the mask. A movie (`.mp4`, played as a picture) is
/// opaque, unless named `*_alpha.mp4`: its right half is the mask.
pub(crate) fn sprite_mode(image: &str) -> char {
    let lower = image.to_ascii_lowercase();
    let opaque = [".jpg", ".jpeg", ".mp4"].iter().any(|e| lower.ends_with(e)) && !lower.ends_with("_alpha.mp4");
    if opaque { 'c' } else { 'a' }
}

/// `lsp` for a sprite ([`sprite_mode`]).
fn lsp(slot: u32, s: &SpritePlacement) -> String {
    let mode = sprite_mode(&s.image);
    format!("lsp {slot},\":{mode};{}\",{},{}", path(&s.image), s.x, s.y)
}

/// Screen position of character `index` of `text` as the text window draws
/// it: each character is one cell of font size + 1 pixel pitch, and lines
/// starting with ASCII got a full-width space in front. Assumes full-width
/// characters, which holds for this game's text.
fn char_position(w: &TextWindow, text: &str, index: usize) -> (i32, i32) {
    let before: Vec<char> = text.chars().take(index).collect();
    let row = before.iter().filter(|&&c| c == '\n').count();
    let mut col = before.iter().rev().take_while(|&&c| c != '\n').count();
    if text.split('\n').nth(row).is_some_and(starts_ascii) {
        col += 1;
    }
    let x = w.text_x + col as i32 * (w.font_size as i32 + 1);
    let y = w.text_y + row as i32 * (w.font_size + w.line_gap) as i32;
    (x, y)
}

/// `indent` (full-width characters) that puts the lines ONScripter wraps
/// below character `hang` of the first line, as this page's first line is
/// emitted: with the space before ASCII and the single-byte padding.
fn hang_indent(text: &str, hang: usize) -> usize {
    let first = text.split('\n').next().unwrap_or_default();
    let prefix: String = first.chars().take(hang).collect();
    let lead = if starts_ascii(first) { "　" } else { "" };
    let half: usize = align_single_bytes(&format!("{lead}{}", escape_text(&prefix)))
        .chars()
        .map(|c| if is_single_byte(c) { 1 } else { 2 })
        .sum();
    half.div_ceil(2)
}

/// Outside English mode ONScripter reads single-byte text (ASCII and
/// half-width katakana) two bytes at a time, skipping a space where a pair
/// would start. A run that ends half way through a pair takes the first
/// byte of what follows with it: a Japanese character, a colour tag or the
/// page wait. Pads such runs with a space.
fn align_single_bytes(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 4);
    let mut half = false; // a single byte waits for its pair
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '#' | '!' | '(' | '/' | ')') || !is_single_byte(c) {
            // Our colour and size tags and ruby markup are the only '#',
            // '!', '(', '/' and ')' left (escape() replaces the rest);
            // ONScripter reads them one byte at a time.
            if half {
                out.push(' ');
                half = false;
            }
            out.push(c);
            match c {
                '#' => out.extend(chars.by_ref().take(6)),
                // !zNNN, !i1, !c
                '!' => {
                    out.extend(chars.next());
                    while let Some(d) = chars.next_if(char::is_ascii_digit) {
                        out.push(d);
                    }
                }
                _ => {}
            }
        } else {
            if half || c != ' ' {
                half = !half;
            }
            out.push(c);
        }
    }
    if half {
        out.push(' ');
    }
    out
}

/// One byte in Shift JIS.
fn is_single_byte(c: char) -> bool {
    c.is_ascii() || ('\u{FF61}'..='\u{FF9F}').contains(&c)
}

fn starts_ascii(line: &str) -> bool {
    line.starts_with(|c: char| c.is_ascii())
}

/// The settings screen has switch `s` (in its list or on a picture page).
fn uses_switch(script: &Script, s: Switch) -> bool {
    script.settings.as_ref().is_some_and(|m| menu::has_switch(m, s))
}

/// Command-line options CONscripter reads from `ons_args` in the game
/// directory. Without `--enc:sjis` it reads the script as GBK;
/// `--no-text-overlay`: the web player draws no text overlay.
pub const ENGINE_ARGS: &str = "--enc:sjis\n--no-text-overlay\n";

/// Encodes NScripter source as Shift-JIS. Characters Shift-JIS can't
/// represent become `?`; their count is returned.
pub fn encode(source: &str) -> (Vec<u8>, usize) {
    let mut lost = 0;
    let mut out = Vec::with_capacity(source.len());
    for ch in source.chars() {
        let mut buf = [0u8; 4];
        let (bytes, _, bad) = encoding_rs::SHIFT_JIS.encode(ch.encode_utf8(&mut buf));
        if bad {
            lost += 1;
            out.push(b'?');
        } else {
            out.extend_from_slice(&bytes);
        }
    }
    (out, lost)
}

/// Decodes a Shift-JIS `0.txt` (the reverse of [`encode`]).
pub fn decode(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}

/// What an `effect` definition draws.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Effect {
    Fade,
    /// Cross-fade through a mask image.
    Mask(String),
    /// Each pixel switches when the progress passes its mask level.
    HardMask(String),
    /// Another of ONScripter's numbered effects.
    Builtin(u32),
    /// Effect 99 with this plugin name (CONscripter's `cons_*.dll`).
    Dll(String),
}

/// The effects a transition prints in turn, with their durations; empty
/// for an instant one.
fn transition_steps(fade_ms: u32, mask: Option<&str>, style: TransitionStyle) -> Vec<(u32, Effect)> {
    if fade_ms == 0 {
        return Vec::new();
    }
    let one = |e| vec![(fade_ms, e)];
    match (mask, style) {
        (Some(m), TransitionStyle::Threshold) => one(Effect::HardMask(m.to_string())),
        (Some(m), _) => one(Effect::Mask(m.to_string())),
        (None, TransitionStyle::Threshold) => one(Effect::Fade),
        (None, TransitionStyle::Flip { horizontal, grow }) => {
            one(Effect::Dll(format!("cons_flip.dll/{},{}", u8::from(!horizontal), u8::from(!grow))))
        }
        (None, TransitionStyle::Melt { stretch_percent }) => one(Effect::Dll(format!("cons_melt.dll/{stretch_percent}"))),
        (None, TransitionStyle::Cube { dx, dy }) => one(Effect::Dll(format!("cons_cube.dll/{dx},{dy}"))),
        (None, TransitionStyle::Wave { direction, amplitude, waves }) => one(Effect::Dll(format!("cons_wave.dll/{direction},{amplitude},{waves}"))),
        (None, TransitionStyle::Blur { radius, new_only }) => one(Effect::Dll(format!("cons_blur.dll/{radius},{}", u8::from(new_only)))),
        (_, TransitionStyle::ThroughWhite) => {
            let half = (fade_ms / 2).max(1);
            vec![(half, Effect::Fade), ((fade_ms - half).max(1), Effect::Fade)]
        }
        // ONScripter's curtains: 6 opens from the left, 7 from the right,
        // 8 from the top, 9 from the bottom.
        (None, TransitionStyle::Curtain { dx, dy }) => one(Effect::Builtin(match (dx, dy) {
            (1, _) => 6,
            (-1, _) => 7,
            (_, 1) => 8,
            _ => 9,
        })),
        (None, TransitionStyle::Fade) => one(Effect::Fade),
        // ONScripter's scrolls (11-14) push both pictures; 11 moves them right.
        (None, TransitionStyle::Slide { moving: SlideLayer::Both, dx, dy }) if dx == 0 || dy == 0 => {
            one(Effect::Builtin(match (dx, dy) {
                (1, _) => 11,
                (-1, _) => 12,
                (_, 1) => 13,
                _ => 14,
            }))
        }
        (None, TransitionStyle::Slide { moving, dx, dy }) => {
            let layer = match moving {
                SlideLayer::New => 0,
                SlideLayer::Old => 1,
                SlideLayer::Both => 2,
            };
            one(Effect::Dll(format!("cons_slide.dll/{layer},{dx},{dy}")))
        }
        // Mosaic out (16: the old picture coarsens), then in (17: the new
        // one sharpens), half the time each.
        (None, TransitionStyle::Mosaic) => {
            let half = (fade_ms / 2).max(1);
            vec![(half, Effect::Builtin(16)), ((fade_ms - half).max(1), Effect::Builtin(17))]
        }
        (None, TransitionStyle::Zoom { style, max_percent, curve }) => {
            let mode = match style {
                ZoomStyle::NewIn => 0,
                ZoomStyle::OldOut => 1,
                ZoomStyle::Through => 2,
                ZoomStyle::Cross => 3,
            };
            one(Effect::Dll(format!("cons_zoom.dll/{mode},{max_percent},{curve}")))
        }
    }
}

/// One effect number per distinct (duration, effect) pair, in a stable
/// order.
fn fade_effects(script: &Script) -> BTreeMap<(u32, Effect), u32> {
    let mut keys: Vec<(u32, Effect)> = script
        .ops
        .iter()
        .flat_map(|op| match op {
            Op::Background { fade_ms, .. } | Op::Sprites { fade_ms, .. } | Op::ScreenFilter { fade_ms, .. } => {
                vec![(*fade_ms, Effect::Fade)]
            }
            Op::ScreenFade { ms, .. } => vec![(*ms, Effect::Fade)],
            Op::TilePuzzle(p) => vec![(p.fade_ms, Effect::Fade)],
            Op::ButtonScreen(s) => vec![(s.fade_ms, Effect::Fade)],
            Op::PictureScreen(s) => vec![(s.fade_ms, Effect::Fade)],
            Op::BeginTransition { fade_ms, mask, style, .. } => transition_steps(*fade_ms, mask.as_deref(), *style),
            _ => Vec::new(),
        })
        .chain(script.title_screen.iter().map(|t| (t.fade_ms, Effect::Fade)))
        .filter(|(ms, _)| *ms > 0)
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter().zip(FIRST_EFFECT..).collect()
}

/// The page text escaped, with link words coloured, `styles` colours
/// (where no link is) and ruby as NScripter's `(base/reading)`. The
/// markup is added after escaping, which turns the text's own `(`, `/`
/// and `)` full-width. Overlapping links, and ruby that crosses a line or
/// a link's edge, are left out. Later styles win where they overlap.
/// Sizes use CONscripter's `!zNNN` tag.
fn with_markup(text: &str, links: &[TextLink], ruby: &[Ruby], styles: &[TextStyle]) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut link_ranges: Vec<(usize, usize)> = Vec::new();
    let mut sorted: Vec<(usize, usize)> =
        links.iter().map(|l| (l.start, l.start + l.len)).filter(|&(s, e)| s < e && e <= chars.len()).collect();
    sorted.sort_unstable();
    for (s, e) in sorted {
        if link_ranges.last().is_none_or(|&(_, end)| s >= end) {
            link_ranges.push((s, e));
        }
    }
    let inside = |i: usize| link_ranges.iter().position(|&(s, e)| s <= i && i < e);
    let (mut colours, sizes) = colours_and_sizes(chars.len(), styles);
    for &(s, e) in &link_ranges {
        colours[s..e].fill(LINK_COLOR.to_string());
    }
    let mut readings: Vec<&Ruby> = ruby
        .iter()
        .filter(|r| {
            let end = r.start + r.len;
            r.len > 0
                && end <= chars.len()
                && !chars[r.start..end].contains(&'\n')
                && (r.start..end).all(|i| inside(i) == inside(r.start))
        })
        .collect();
    readings.sort_by_key(|r| r.start);

    let marks = slant_tags(&chars, styles);
    let mut out = String::new();
    let mut readings = readings.into_iter().peekable();
    let mut current = TEXT_COLOR;
    let mut size = 100;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\n' && colours[i] != current {
            current = &colours[i];
            out.push_str(current);
        }
        if chars[i] != '\n' && sizes[i] != size {
            size = sizes[i];
            write!(out, "!z{size:03}").unwrap();
        }
        while readings.peek().is_some_and(|r| r.start < i) {
            readings.next(); // overlaps the previous one
        }
        out.push_str(&marks[i]);
        if let Some(r) = readings.next_if(|r| r.start == i) {
            let base: String = chars[i..i + r.len].iter().collect();
            write!(out, "({}/{})", escape_text(&base), escape_text(&r.text)).unwrap();
            // shortcut: a slant change inside ruby moves after it; fine while no game slants ruby.
            out.extend(marks[i + 1..i + r.len].iter().map(String::as_str));
            i += r.len;
        } else {
            out.push_str(&escape_text(&chars[i].to_string()));
            i += 1;
        }
        // Tags last past the page: end each line on the text colour and size.
        let line_end = chars.get(i).is_none_or(|&c| c == '\n');
        if size != 100 && (line_end || sizes[i] == 100) {
            size = 100;
            out.push_str("!z100");
        }
        if current != TEXT_COLOR && (line_end || colours[i] == TEXT_COLOR) {
            current = TEXT_COLOR;
            out.push_str(TEXT_COLOR);
        }
    }
    out.push_str(&marks[chars.len()]);
    out
}

/// CONscripter's tags for the styles' slant, centring and outline, to go
/// before each character (and after the last): `!c` where a centred range
/// starts a line, `!i1` and `!i0` around italics, `!o#RRGGBB` and `!o0`
/// around outlines (the last style's wins), closed at each line's end as
/// tags last past the page.
fn slant_tags(chars: &[char], styles: &[TextStyle]) -> Vec<String> {
    let covered = |i: usize, f: fn(&TextStyle) -> bool| styles.iter().any(|s| f(s) && s.start <= i && i < s.start + s.len);
    let mut marks = vec![String::new(); chars.len() + 1];
    let mut italic = false;
    let mut outline = None;
    for (i, mark) in marks.iter_mut().enumerate() {
        let line_end = chars.get(i).is_none_or(|&c| c == '\n');
        if !line_end && (i == 0 || chars[i - 1] == '\n') && covered(i, |s| s.centred) {
            mark.push_str("!c");
        }
        let want = !line_end && covered(i, |s| s.italic);
        if want != italic {
            italic = want;
            mark.push_str(if want { "!i1" } else { "!i0" });
        }
        let want = if line_end { None } else { styles.iter().rev().find(|s| s.outline.is_some() && s.start <= i && i < s.start + s.len).and_then(|s| s.outline) };
        if want != outline {
            outline = want;
            match want {
                Some(c) => write!(mark, "!o#{c:06X}").unwrap(),
                None => mark.push_str("!o0"),
            }
        }
    }
    marks
}

fn operand(o: Operand, border: u32) -> String {
    match o {
        Operand::Const(n) => n.to_string(),
        Operand::Var(v) => var_name(v, border),
    }
}

/// `border`: the first global variable ([`Script::global_border`]).
pub fn var_name(v: Var, border: u32) -> String {
    let base = if v.global { border } else { LOCAL_VAR_BASE };
    format!("%{}", base + u32::from(v.index))
}

/// NScripter paths use backslashes.
fn path(p: &str) -> String {
    p.replace('/', "\\")
}

/// Labels must be ASCII identifiers.
fn label(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

/// [`escape`] for a text line, where `?` also starts an array reference
/// (as `%` a variable): the engine exits on an undeclared one. In strings
/// it is only a character.
fn escape_text(text: &str) -> String {
    escape(text).replace('?', "？")
}

/// Makes text safe to place on a text line: [`escape_text`], plus making
/// sure the line doesn't start with ASCII (which would be parsed as a
/// command).
fn text_safe(text: &str) -> String {
    let mut out = escape_text(text);
    if out.starts_with(|c: char| c.is_ascii()) {
        out.insert(0, '　');
    }
    out
}

/// Replaces ASCII characters that NScripter treats specially in text with
/// their full-width forms.
fn escape(text: &str) -> String {
    text
        .chars()
        .map(|c| match c {
            '@' => '＠',
            '\\' => '＼',
            '/' => '／',
            '#' => '＃',
            '%' => '％',
            '$' => '＄',
            '`' => '｀',
            '!' => '！',
            '_' => '＿',
            '"' => '”',
            ';' => '；',
            '*' => '＊',
            // Ruby, text buttons and text tags.
            '(' => '（',
            ')' => '）',
            '<' => '＜',
            '>' => '＞',
            '[' => '［',
            ']' => '］',
            '\r' => '　',
            c => c,
        })
        .collect()
}

/// Each character's colour (`#RRGGBB`) and size (percent) from `styles`;
/// later styles win where they overlap.
fn colours_and_sizes(n: usize, styles: &[TextStyle]) -> (Vec<String>, Vec<u16>) {
    let mut colours: Vec<String> = vec![TEXT_COLOR.to_string(); n];
    let mut sizes: Vec<u16> = vec![100; n];
    for st in styles {
        if let Some(c) = st.colour {
            colours.iter_mut().skip(st.start).take(st.len).for_each(|colour| *colour = format!("#{c:06X}"));
        }
        if let Some(p) = st.size_percent {
            sizes.iter_mut().skip(st.start).take(st.len).for_each(|s| *s = p.clamp(1, 999));
        }
    }
    (colours, sizes)
}

/// Text for a single-byte line (after a backquote), where only the click
/// waits, the backquote itself and CONscripter's tags are special (`!i`,
/// `!c`; `!w`, `!d`, `!z`, `!v`, `!s` before a number; `!o` before a
/// number or `#`; `#` before six hex digits): those turn full-width,
/// everything else stays as written. The styles become those tags
/// (colours, sizes, slant, centring, outline), each line
/// ending on the text colour and size.
fn single_byte_text(text: &str, styles: &[TextStyle]) -> String {
    let chars: Vec<char> = text.chars().collect();
    let marks = slant_tags(&chars, styles);
    let (colours, sizes) = colours_and_sizes(chars.len(), styles);
    let mut out = String::new();
    let (mut current, mut size) = (TEXT_COLOR, 100);
    for (i, &c) in chars.iter().enumerate() {
        if c != '\n' && colours[i] != current {
            current = &colours[i];
            out.push_str(current);
        }
        if c != '\n' && sizes[i] != size {
            size = sizes[i];
            write!(out, "!z{size:03}").unwrap();
        }
        out.push_str(&marks[i]);
        let next = |k: usize| chars.get(i + k).copied().unwrap_or(' ');
        let numbered_tag = (matches!(next(1), 's' | 'w' | 'd' | 'z' | 'v') && (next(2).is_ascii_digit() || (next(1) == 's' && next(2) == 'd')))
            || (next(1) == 'o' && (next(2).is_ascii_digit() || next(2) == '#'));
        out.push(match c {
            '@' => '＠',
            '\\' => '＼',
            '`' => '｀',
            '\r' => ' ',
            '!' if matches!(next(1), 'i' | 'c') || numbered_tag => '！',
            '#' if (1..=6).all(|k| next(k).is_ascii_hexdigit()) => '＃',
            c => c,
        });
        let line_end = chars.get(i + 1).is_none_or(|&c| c == '\n');
        if size != 100 && (line_end || sizes[i + 1] == 100) {
            size = 100;
            out.push_str("!z100");
        }
        if current != TEXT_COLOR && (line_end || colours[i + 1] == TEXT_COLOR) {
            current = TEXT_COLOR;
            out.push_str(TEXT_COLOR);
        }
    }
    out.push_str(&marks[chars.len()]);
    out
}

/// The text's timed and voice waits ([`conv_core::ir::TIMED_WAIT_MARK`],
/// [`conv_core::ir::VOICE_WAIT_MARK`]) as CONscripter's `!d` and `!v`. The
/// engine skips spaces after the number: a wait before spaces goes after
/// them, and one before a digit is ended by a space.
fn wait_tags(text: &str) -> String {
    use conv_core::ir::{TIMED_WAIT_MARK, VOICE_WAIT_MARK, WAIT_END_MARK};
    let mut out = String::new();
    let mut tag: Option<String> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, &mut tag) {
            (TIMED_WAIT_MARK, _) => tag = Some("!d".into()),
            (VOICE_WAIT_MARK, _) => tag = Some("!v".into()),
            (WAIT_END_MARK, t) => {
                let t = t.take().unwrap_or_default();
                while let Some(' ') = chars.peek() {
                    out.push(chars.next().unwrap_or(' '));
                }
                out.push_str(&t);
                if chars.peek().is_some_and(char::is_ascii_digit) {
                    out.push(' ');
                }
            }
            (c, Some(t)) => t.push(c),
            (c, None) => out.push(c),
        }
    }
    out
}

/// [`escape`]s text that `strsp` (or another sprite text) will draw.
/// CONscripter no longer pairs a half-width character with a full-width
/// one after it, so an odd run needs no padding space.
fn sprite_text(text: &str) -> String {
    escape(text)
}

#[derive(Default)]
struct SeChannels {
    playing: HashMap<String, u32>,
    next: u32,
    /// The script sets volumes ([`Op::SoundVolume`]): every sound starts
    /// with one (`chvolfade`, music -1).
    volumes: bool,
    /// The op before the sound starting set its volume.
    preset: bool,
    /// Channels a [`Op::SoundPan`] panned (centred again by the next
    /// sound on them), and whether one is for the sound starting next.
    panned: HashSet<u32>,
    pan_preset: bool,
    /// Channels playing a looped sound, which another sound doesn't take.
    looping: HashSet<u32>,
}

/// Channels for looped sounds when every one of [`SE_CHANNELS`] has one
/// (続・殺戮のジャンゴ's chorus: 16 parts at once); 10 is the choices' shot.
const SE_OVERFLOW: std::ops::RangeInclusive<u32> = 11..=49;

impl SeChannels {
    /// The channel the next [`SeChannels::start`] takes: the next of
    /// [`SE_CHANNELS`] in turn without a looped sound, or else an
    /// overflow one.
    fn peek(&self) -> u32 {
        let span = SE_CHANNELS.end() - SE_CHANNELS.start() + 1;
        (0..span)
            .map(|i| SE_CHANNELS.start() + (self.next + i) % span)
            .chain(SE_OVERFLOW)
            .find(|c| !self.looping.contains(c))
            .unwrap_or(SE_CHANNELS.start() + self.next % span)
    }

    fn start(&mut self, file: &str, looped: bool) -> u32 {
        let ch = self.peek();
        if SE_CHANNELS.contains(&ch) {
            self.next = ch - SE_CHANNELS.start() + 1;
        }
        self.playing.retain(|_, c| *c != ch);
        // (The same file again: its old channel is let go.)
        if let Some(old) = self.playing.insert(file.to_string(), ch) {
            self.looping.remove(&old);
        }
        self.looping.remove(&ch);
        if looped {
            self.looping.insert(ch);
        }
        ch
    }

    fn stop(&mut self, file: &str) -> Option<u32> {
        let ch = self.playing.remove(file);
        if let Some(c) = ch {
            self.looping.remove(&c);
        }
        ch
    }

    fn stop_all(&mut self) -> Vec<u32> {
        self.looping.clear();
        let mut chans: Vec<u32> = self.playing.drain().map(|(_, c)| c).collect();
        chans.sort_unstable();
        chans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::Compare;
    use conv_core::ir::{Animation, ButtonScreen, ChoiceStyle, Noise, Placed, SourceLoc, SpriteChange, SpriteFilter, SpritePlacement, Voice, FILTER_BLEND_COPY, FILTER_BLEND_MULTIPLY, FILTER_BRIGHTNESS, FILTER_COLOUR};

    fn script(ops: Vec<Op>) -> Script {
        Script { title: "Test".into(), width: 800, height: 600, ops, ..Default::default() }
    }

    fn link(start: usize, len: usize, target: Option<&str>) -> TextLink {
        TextLink { start, len, target: target.map(String::from), scene: None }
    }

    fn window() -> TextWindow {
        use conv_core::ir::NamePlate;
        TextWindow {
            wait_icon_at: None,
            image: "system/window.png".into(),
            x: 47,
            y: 452,
            text_x: 105,
            text_y: 470,
            columns: 25,
            rows: 3,
            font_size: 24,
            line_gap: 13,
            vertical: false,
            name_plate: Some(NamePlate {
                image: "system/name.png".into(),
                x: 55,
                y: 412,
                text_x: 75,
                text_y: 422,
                font_size: 20,
            }),
        }
    }

    #[test]
    fn a_choice_of_plates_is_picture_buttons_with_centred_text() {
        let window = TextWindow { wait_icon_at: None, image: "system/w.png".into(), x: 0, y: 0, text_x: 0, text_y: 0, columns: 10, rows: 2, font_size: 22, line_gap: 0, vertical: false, name_plate: None };
        let plates = conv_core::ir::ChoicePlates { image: "system/plate.png".into(), x: 138, y: 244, step: 68, text_x: 388, text_dy: 18, font_size: 22, colour: "#FFFFFF".into(), fade_ms: 300, shot: None };
        let options = vec![("はい".to_string(), "a".to_string()), ("OK".to_string(), "b".to_string())];
        let style = Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) });
        let text = emit(&script(vec![Op::Choice { options, style }]));
        for part in [
            "lsp 52,\":a/2,0,3;system\\plate.png\",138,244\nstrsp 51,\"はい\",366,262,3,1,22,22,0,0,0,1,\"#FFFFFF\",\"#FFFFFF\"\n",
            "strsp 53,\"OK\",377,330,",
            "print 10,300\n*conv_cplate0_wait\nbtndef clear\nspbtn 52,1\nspbtn 54,2\nbtnwait %9\n",
            "if %9 == 2 goto *b\n",
        ] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
        assert!(!text.contains("select "));
    }

    #[test]
    fn a_runtime_choice_sets_its_option_texts_from_a_variable() {
        let var = Var { global: false, index: 5 };
        let rt = conv_core::ir::RuntimeText { var, texts: vec![(1, "頼む".into()), (2, "Leave".into())] };
        let options = vec![("見る".to_string(), "a".to_string()), ("頼む".to_string(), "b".to_string())];
        let text = emit(&script(vec![Op::RuntimeChoice { options: options.clone(), texts: vec![None, Some(rt.clone())], style: None }]));
        for part in ["mov $3633,\"頼む\"", "if %15 == 2 mov $3633,\"　Leave\"", "select \"見る\",*a,$3633,*b"] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
        // On plates: the text and its centred x, set together.
        let window = TextWindow { wait_icon_at: None, image: "system/w.png".into(), x: 0, y: 0, text_x: 0, text_y: 0, columns: 10, rows: 2, font_size: 22, line_gap: 0, vertical: false, name_plate: None };
        let plates = conv_core::ir::ChoicePlates { image: "system/plate.png".into(), x: 138, y: 244, step: 68, text_x: 388, text_dy: 18, font_size: 22, colour: "#FFFFFF".into(), fade_ms: 300, shot: None };
        let style = Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) });
        let text = emit(&script(vec![Op::RuntimeChoice { options, texts: vec![None, Some(rt)], style }]));
        for part in ["if %15 == 2 mov $3633,\"Leave\":mov %3633,361", "strsp 53,$3633,%3633,330,6,"] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
        let text = emit(&script(vec![Op::RuntimeBacklogLine { text: "【】".into(), runtime: conv_core::ir::RuntimeText { var, texts: vec![(2, "【Leave】".into())] } }]));
        assert!(text.contains("mov $3638,\"【】\"\nif %15 == 2 mov $3638,\"【Leave】\"\nlogadd $3638\n"), "{text}");
    }

    #[test]
    fn a_shot_plate_gets_its_hole_and_tips_over_about_its_bottom() {
        let window = TextWindow { wait_icon_at: None, image: "system/w.png".into(), x: 0, y: 0, text_x: 0, text_y: 0, columns: 10, rows: 2, font_size: 22, line_gap: 0, vertical: false, name_plate: None };
        let shot = Some(conv_core::ir::ChoiceShot { hole: "system/hole.png".into(), hole_dx: -47, hole_dy: -41, sound: "se/shot.ogg".into() });
        let plates = conv_core::ir::ChoicePlates { image: "system/plate.png".into(), x: 138, y: 244, step: 68, text_x: 388, text_dy: 18, font_size: 22, colour: "#FFFFFF".into(), fade_ms: 300, shot };
        let options = vec![("はい".to_string(), "a".to_string()), ("OK".to_string(), "b".to_string())];
        let style = Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) });
        let text = emit(&script(vec![Op::Choice { options, style }]));
        for part in [
            "getmousepos %0,%1:add %0,-47:add %1,-41\ndwave 10,\"se\\shot.ogg\"\nif %9 == 1 goto *conv_cplate0_s1\n",
            "*conv_cplate0_s2\ncell 54,1:spcut 54,\"system\\hole.png\",%0,%1:spcut 53,\"system\\hole.png\",%0,%1:print 1\ndelay 300\ncsp 51:csp 52:print 10,300\n",
            // The last step: a quarter high, its bottom still at 380.
            "spzoom 54,100,27:spzoom 53,100,27:amsp 54,138,337,64:amsp 53,377,358,64:print 1:delay 50\ncsp 53:csp 54:print 1\ngoto *b\n",
        ] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
    }

    #[test]
    fn a_held_movie_draws_its_prompt_and_plays_its_sound_on_an_effect_channel() {
        use conv_core::ir::{HoldPrompt, HoldSound};
        let prompt = Some(HoldPrompt { image: "system/push.png".into(), x: 690, y: 20, cells: 10, frame_ms: 100 });
        let sound = Some(HoldSound { file: "sound/se/fire.ogg".into(), fade_ms: 500 });
        let text = emit(&script(vec![Op::HoldMovie { file: "movie/a.mp4".into(), prompt, sound }]));
        let part = "lsp 6,\":a/10,100,0;system\\push.png\",690,20\nmpeghold \"movie\\a.mp4\",6,1,\"sound\\se\\fire.ogg\",500\ncsp 6\n";
        assert!(text.contains(part), "missing {part:?} in\n{text}");
        let text = emit(&script(vec![Op::HoldMovie { file: "movie/a.mp4".into(), prompt: None, sound: None }]));
        assert!(text.contains("mpeghold \"movie\\a.mp4\"\n"));
    }

    #[test]
    fn a_flashing_sprite_is_added_on_with_its_keys_repeated() {
        let keys = vec![(0, 76), (17, 76), (97, 0), (114, 0)];
        let text = emit(&script(vec![Op::SpriteFlashing { layer: 3, keys: keys.clone(), times: 60 }]));
        let slot = SPRITE_BASE + 3;
        assert!(text.contains(&format!("spblend {slot},1:spmotion {slot},-60,0,0,0,100,100,0,76,0,17,0,0,100,100,0,76,0,97,0,0,100,100,0,0,0,114,0,0,100,100,0,0,0\nprint 1\n")), "{text}");
        // Once: spmotion's loop 2, keeping the last key (1 would repeat it for ever).
        let text = emit(&script(vec![Op::SpriteFlashing { layer: 3, keys, times: 1 }]));
        assert!(text.contains(&format!("spmotion {slot},2,0,0,0,100,100,0,76,")), "{text}");
    }

    #[test]
    fn clickable_links_call_their_target() {
        let mut s = script(vec![
            Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
            Op::BgmPlay { file: "bgm/a.ogg".into(), fade_in_ms: 0 },
            Op::Text {
                speaker: None,
                text: "一行目\n学校に―――".into(),
                voice: None,
                links: vec![link(4, 2, Some("s10_l1")), link(0, 1, None)],
                ruby: vec![],
                styles: vec![],
                hang: None,
                continues: false,
            },
        ]);
        s.window = Some(window());
        let text = emit(&s);
        assert!(text.contains("globalon\nwindowback\nhumanz 58\n"));
        // Row 2, column 1: x = 105 + 0 * 25, y = 470 + 1 * 37.
        let expected = "\
*conv_link1
csp 50
csp 49
print 1
#FEE092一#FFFFFF行目
#FEE092学校#FFFFFFに―――
if %2000 == 0 strsp 51,\"学校\",105,507,3,1,24,24,1,13,0,1,\"#FEE092\",\"#FEC95E\"
if %2000 != 0 strsp 51,\"学校\",105,507,3,1,24,24,1,13,0,1,\"#DCB8FE\",\"#CB95FE\"
*conv_link1_wait
if %2000 == 0 lsp 6,\":c;>49,1,#FEE092\",107,532
if %2000 != 0 lsp 6,\":c;>49,1,#DCB8FE\",107,532
lsp 7,\":c;>49,1,#000000\",108,533
spbtn 51,1
print 1
btnwait %9
csp 51
csp 6
csp 7
print 1
textclear
if %9 != 1 goto *conv_link1_1
mov %2000,1
pushscreen 59,999
gosub *s10_l1
popscreen
setwindow3 105,470,25,3,24,24,1,13,20,0,1,\":a;system\\window.png\",47,452
print 1
goto *conv_link1
*conv_link1_1
";
        assert!(text.contains(expected), "{text}");
    }

    #[test]
    fn link_scenes_nest_at_most_seven_deep() {
        let mut s = script(vec![
            Op::Overlay { image: Some("bg/frame.png".into()), over_sprites: true },
            Op::Text {
                speaker: None,
                text: "学校に".into(),
                voice: None,
                links: vec![link(0, 2, Some("s10_l1"))],
                ruby: vec![],
                styles: vec![],
                hang: None,
                continues: false,
            },
        ]);
        s.window = Some(window());
        let text = emit(&s);
        assert!(text.contains("\nif %181 < 7 spbtn 51,1\n"), "{text}");
    }

    #[test]
    fn effects_past_255_are_written_inline() {
        let ops: Vec<Op> = (1..=300)
            .flat_map(|ms| [Op::BeginTransition { effect: "Fade".into(), fade_ms: ms, mask: None, style: TransitionStyle::Fade }, Op::EndTransition])
            .collect();
        let text = emit(&script(ops));
        assert!(!text.contains("\neffect "), "{text}");
        assert!(text.contains("print 10,300\n"));
        let few = emit(&script(vec![Op::BeginTransition { effect: "Fade".into(), fade_ms: 300, mask: None, style: TransitionStyle::Fade }, Op::EndTransition]));
        assert!(few.contains("effect 2,10,300\n") && few.contains("print 2\n"), "{few}");
    }

    #[test]
    fn jpeg_sprites_are_opaque() {
        let jpeg = SpritePlacement { image: "cg/bg/a.JPG".into(), x: 0, y: 0 };
        assert_eq!(lsp(70, &jpeg), "lsp 70,\":c;cg\\bg\\a.JPG\",0,0");
        let png = SpritePlacement { image: "cg/st/a.png".into(), x: 1, y: 2 };
        assert_eq!(lsp(70, &png), "lsp 70,\":a;cg\\st\\a.png\",1,2");
    }

    #[test]
    fn vertical_window_on_a_wide_screen() {
        let vertical = TextWindow { vertical: true, name_plate: None, x: 447, y: 40, text_x: 430, text_y: 55, columns: 27, rows: 5, font_size: 20, line_gap: 9, ..window() };
        let mut s = script(vec![Op::SetWindow(window())]);
        (s.width, s.height, s.window) = (1024, 576, Some(vertical));
        let text = emit(&s);
        assert!(text.starts_with(";$S1024,576,V8192\n"), "{text}");
        assert!(text.contains("tateyoko 1:setwindow3 430,55,5,27,20,20,9,0,20,0,1,\":a;system\\window.png\",447,40\n"), "{text}");
        assert!(text.contains("tateyoko 0:setwindow3 105,"), "{text}");
    }

    #[test]
    fn styled_choice_has_its_own_window_and_colours() {
        let plates = TextWindow { vertical: true, name_plate: None, x: 462, y: 68, text_x: 477, text_y: 108, columns: 18, rows: 2, font_size: 20, line_gap: 80, ..window() };
        let style = Some(ChoiceStyle { window: plates, on_color: 0xA00000, off_color: 0, plates: None });
        let options = vec![("はい".into(), "a".into()), ("いいえ".into(), "b".into())];
        let text = emit(&script(vec![Op::Choice { options, style }]));
        // No shadow; no ruby here, so text_x as given.
        assert!(text.contains("selectcolor \"#A00000\",\"#000000\"\ntateyoko 1:setwindow3 477,108,2,18,20,20,80,0,20,0,0,"), "{text}");
        assert!(text.contains("select \"はい\",*a,\"いいえ\",*b\n"), "{text}");
    }

    #[test]
    fn button_screen_waits_for_its_buttons_and_fades_out() {
        let placed = |image: &str, x, y| Placed { image: image.into(), x, y, w: 10, h: 10 };
        let s = ButtonScreen {
            pictures: vec![placed("system/bg.png", 0, 0), placed("system/text.png", 300, 319)],
            buttons: vec![(placed("system/yes.png", 385, 230), "scene_jump".into()), (placed("system/no.png", 515, 230), "scene_play".into())],
            fade_ms: 300,
        };
        let text = emit(&script(vec![Op::ButtonScreen(Box::new(s)), Op::Label("scene_jump".into()), Op::Label("scene_play".into())]));
        // Buttons on top (6, 7), then the pictures from the top down.
        assert!(text.contains("textoff\nlsp 9,\":a;system\\bg.png\",0,0\nlsp 8,\":a;system\\text.png\",300,319\nlsp 6,\":a/2,0,3;system\\yes.png\",385,230\nlsp 7,"), "{text}");
        assert!(text.contains("*conv_bscreen0\nbtndef clear\nspbtn 6,1\nspbtn 7,2\nbtnwait %9\nif %9 == 1 csp 6:csp 7:csp 8:csp 9:print 2:goto *scene_jump\nif %9 == 2 "), "{text}");
        assert!(text.contains("goto *scene_play\ngoto *conv_bscreen0\n"), "{text}");
    }

    #[test]
    fn button_screen_past_28_sprites_takes_the_extra_slots() {
        let placed = |x| Placed { image: "system/b.png".into(), x, y: 0, w: 10, h: 10 };
        let buttons = (0..40).map(|i| (placed(i), "end".to_string())).collect();
        let s = ButtonScreen { pictures: vec![placed(-1)], buttons, fade_ms: 0 };
        let text = emit(&script(vec![Op::ButtonScreen(Box::new(s)), Op::Label("end".into())]));
        // 6-33 as before, then 980 up drawn under 33 and over the bars.
        assert!(text.contains("lsp 33,\":a/2,0,3;system\\b.png\",27,0\nlsp 980,\":a/2,0,3;system\\b.png\",28,0:spz 980,34\n"), "{text}");
        assert!(text.contains("\nlsp 992,\":a;system\\b.png\",-1,0:spz 992,34\n"), "{text}");
        assert!(text.contains("spbtn 991,40\nbtnwait"), "{text}");
        assert_eq!(menu_slot(MENU_SLOTS - 1), 998);
    }

    #[test]
    fn scene_replay_unlocks_are_global_flags() {
        let text = emit(&script(vec![Op::SceneEnd, Op::UnlockScene { scene: 5 }, Op::Progress { bit: 3 }, Op::CgSeen { number: 519 }]));
        assert!(text.contains("; scene end\nmov %3005,1\nmov %3603,1\nmov %2719,1\n"), "{text}");
    }

    #[test]
    fn dissolve_passes_through_white() {
        let text = emit(&script(vec![
            Op::BeginTransition { effect: "Dissolve".into(), fade_ms: 1000, mask: None, style: TransitionStyle::ThroughWhite },
            Op::EndTransition,
        ]));
        let n = |ms: u32| text.lines().find(|l| l.starts_with("effect ") && l.ends_with(&format!(",10,{ms}"))).unwrap().split([' ', ',']).nth(1).unwrap().to_string();
        let expected = format!("lsp2 24,\":c;>800,600,#ffffff\",400,300,100,100,0,255\nprint {}\ncsp2 24\nprint {}\n", n(500), n(500));
        assert!(text.contains(&expected), "{text}");
    }

    #[test]
    fn sprite_turns_are_engine_motions() {
        let turn = Op::SpriteTurn { layer: 7, image: "sprite/a.png".into(), from: (200, 400), to: (200, 900), degrees: -90, ms: 800 };
        let text = emit(&script(vec![turn]));
        let expected = "lsp 69,\":a;sprite\\a.png\",0,0:getspsize 69,%1,%2:amsp 69,200-%1/2,400-%2/2\n\
            spmotion 69,0,0,0,0,100,100,0,256,0,800,0,500,100,100,90,256,0:spmotionwait 69\ncsp 69\nprint 1\n";
        assert!(text.contains(expected), "{text}");
    }

    #[test]
    fn return_to_root_unwinds_link_scenes() {
        let text = emit(&script(vec![Op::ReturnToRoot, Op::Wait { ms: 1 }]));
        assert!(text.contains("if %118 > 0 mov %119,1:return\n*conv_root1\ndelay 1\n"), "{text}");
        assert!(text.contains("*conv_unwind\nif %118 > 0 return\nmov %105,%119:mov %119,0\nif %105 == 1 goto *conv_root1\n"), "{text}");
    }

    #[test]
    fn sound_rates_bend_the_sound_on_its_channel() {
        let s = script(vec![
            Op::SePlay { file: "se\\a.ogg".into(), looped: false },
            Op::SoundRate { file: "se\\a.ogg".into(), rate: 1100, ms: 1000 },
            Op::SoundRate { file: "se\\a.ogg".into(), rate: 1000, ms: 0 },
            Op::SoundRate { file: "se\\b.ogg".into(), rate: 900, ms: 0 },
        ]);
        let text = emit(&s);
        let ch = text.split("dwave ").nth(1).unwrap().split(',').next().unwrap();
        assert!(text.contains(&format!("chfreq {ch},1100,1000\nchfreq {ch},1000\n")), "{text}");
        assert!(!text.contains(",900"), "{text}");
    }

    #[test]
    fn bgm_crossfades_over_the_music_playing() {
        let text = emit(&script(vec![Op::BgmPlay { file: "bgm/a.ogg".into(), fade_in_ms: 0 }, Op::BgmCrossfade { file: "bgm/b.ogg".into(), ms: 3000 }]));
        assert!(text.contains("bgm \"bgm\\b.ogg\",3000\n"), "{text}");
    }

    #[test]
    fn sprites_fading_fade_by_themselves() {
        let picture = SpritePlacement { image: "sprite/a.png".into(), x: 1, y: 2 };
        let fade = Op::SpritesFading {
            changes: vec![SpriteChange { layer: 6, sprite: None, keep_filters: false }, SpriteChange { layer: 5, sprite: Some(picture), keep_filters: false }],
            fade_ms: 300,
        };
        let hop = Op::SpriteMotion { layer: 5, keys: vec![(0, 0, 0), (150, 0, 37), (600, 0, 0)], looped: false };
        let text = emit(&script(vec![fade.clone(), hop.clone(), Op::Wait { ms: 100 }, hop]));
        assert!(text.contains("spmotion 68,3,0,0,0,100,100,0,256,0,300,0,0,100,100,0,0,0
spfilter 67,0
lsp 67,\":a;sprite\\a.png\",1,2
spmotion 67,0,0,0,0,100,100,0,0,0,300,0,0,100,100,0,256,0
print 1
"), "{text}");
        // the hop started with the fade takes it into its keys; one after a wait doesn't
        assert!(text.contains("spmotion 67,0,0,0,0,100,100,0,0,0,150,0,37,100,100,0,128,0,300,0,25,100,100,0,256,0,600,0,0,100,100,0,256,0\n"), "{text}");
        assert!(text.contains("spmotion 67,0,0,0,0,100,100,0,256,0,150,0,37,100,100,0,256,0,600,0,0,100,100,0,256,0\n"), "{text}");
    }

    #[test]
    fn a_picture_after_a_zoomed_animation_is_drawn_unzoomed() {
        let movie = SpritePlacement { image: "system/movie/smoke.png".into(), x: 1, y: 2 };
        let picture = SpritePlacement { image: "sprite/a.png".into(), x: 1, y: 2 };
        let s = script(vec![
            Op::AnimatedSprite { layer: 5, sprite: movie, cells: 4, frame_ms: 66, looped: true, zoom: (400, 400) },
            Op::Sprites { changes: vec![SpriteChange { layer: 5, sprite: Some(picture.clone()), keep_filters: false }], fade_ms: 0 },
            Op::Sprites { changes: vec![SpriteChange { layer: 6, sprite: Some(picture), keep_filters: false }], fade_ms: 0 },
        ]);
        let text = emit(&s);
        assert!(text.contains("spzoom 67,400,400\n"), "{text}");
        assert!(text.contains("lsp 67,\":a;sprite\\a.png\",1,2\nspzoom 67,100,100\n"), "{text}");
        assert!(!text.contains("spzoom 68"), "{text}");
    }

    #[test]
    fn transitions_reveal_changes_at_once() {
        let sprite = SpritePlacement { image: "sprite/a.png".into(), x: 1, y: 2 };
        let s = script(vec![
            Op::BeginTransition { effect: "CrossFade".into(), fade_ms: 2000, mask: None, style: TransitionStyle::Fade },
            Op::ClearSprites,
            Op::Background { image: "bg/a.png".into(), fade_ms: 500 },
            Op::Sprites { changes: vec![SpriteChange { layer: 5, sprite: Some(sprite), keep_filters: false }], fade_ms: 500 },
            Op::EndTransition,
            Op::BeginTransition {
                effect: "MaskBlend2".into(),
                fade_ms: 1000,
                mask: Some("system/mask0009r.png".into()),
                style: TransitionStyle::Fade,
            },
            Op::Background { image: "bg/b.png".into(), fade_ms: 500 },
            Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        ]);
        let text = emit(&s);
        assert!(text.contains("effect 2,10,500\neffect 3,18,1000,\"system\\mask0009r.png\"\neffect 4,10,2000\n"), "{text}");
        let expected = "\
; transition: CrossFade
csp -1
csp -1
bg \"bg\\a.png\",0
spfilter 67,0
lsp 67,\":a;sprite\\a.png\",1,2
print 4
; transition: MaskBlend2
csp -1
bg \"bg\\b.png\",0
print 3
あ\\
";
        assert!(text.contains(expected), "{text}");
    }

    #[test]
    fn transition_styles() {
        let begin = |style| Op::BeginTransition { effect: "x".into(), fade_ms: 1000, mask: None, style };
        let slide = |moving, dx, dy| begin(TransitionStyle::Slide { moving, dx, dy });
        let s = script(vec![
            slide(SlideLayer::New, -1, 0),
            Op::EndTransition,
            slide(SlideLayer::Both, 0, -1),
            Op::EndTransition,
            begin(TransitionStyle::Mosaic),
            Op::EndTransition,
            begin(TransitionStyle::Zoom { style: ZoomStyle::Cross, max_percent: 220, curve: 5 }),
            Op::EndTransition,
            begin(TransitionStyle::Melt { stretch_percent: 71 }),
            Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        ]);
        let text = emit(&s);
        let defs = "\
effect 2,16,500
effect 3,17,500
effect 4,14,1000
effect 5,99,1000,\"cons_melt.dll/71\"
effect 6,99,1000,\"cons_slide.dll/0,-1,0\"
effect 7,99,1000,\"cons_zoom.dll/3,220,5\"
";
        assert!(text.contains(defs), "{text}");
        assert!(text.contains("; transition: x\nprint 6\n; transition: x\nprint 4\n; transition: x\nprint 2\nprint 3\n; transition: x\nprint 7\n; transition: x\nprint 5\nあ"), "{text}");
    }

    #[test]
    fn instant_text_sets_the_text_speed() {
        let text = |t: &str| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false };
        let s = script(vec![Op::InstantText(true), text("一"), Op::InstantText(false), text("二")]);
        let out = emit(&s);
        assert!(out.contains("textspeed 0\n一\\\ntextspeed 20\n二\\\n"), "{out}");
    }

    #[test]
    fn menu_sounds_at_startup() {
        let mut s = script(vec![]);
        s.menu_sounds = Some(conv_core::ir::MenuSounds { hover: Some("se/tick.ogg".into()), confirm: Some("se/ok.ogg".into()) });
        let out = emit(&s);
        assert!(out.contains("*start\nmov $118,\"se\\tick.ogg\":selectvoice \"\",$118,\"\"\nbtnse \"se\\ok.ogg\"\n"), "{out}");
    }

    #[test]
    fn title_buttons_hidden_without_saves_and_routes() {
        use conv_core::ir::{TitleButton, Var};
        let mut s = script(vec![]);
        let button = |action| TitleButton { image: "system/b.png".into(), x: 0, y: 0, action, click_se: None, shown_by: None };
        s.title_screen = Some(TitleScreen {
            background: "system/title.png".into(),
            buttons: vec![button(TitleAction::Continue), button(TitleAction::Start), button(TitleAction::Load), button(TitleAction::Extras)],
            hide_without_saves: true,
            extras_shown_by: vec![Var { global: true, index: 1001 }, Var { global: true, index: 1002 }],
            ..Default::default()
        });
        let text = emit(&s);
        for part in [
            // The save Continue loads: any slot, the newest, the quick save.
            "mov %104,0\nfor %103 = 1 to 100\nif %104 == 0 savefileexist %105,%103:if %105 == 1 mov %104,%103\nnext\nif %901 > 0 savefileexist %105,%901:if %105 == 1 mov %104,%901\nsavefileexist %105,890:if %105 == 1 mov %104,890\n",
            "if %104 > 0 lsp 6,",
            "if %104 > 0 lsp 8,",
            "if %1201 != 0 || %1202 != 0 lsp 9,",
            "if %104 > 0 spbtn 6,1\nspbtn 7,2\nif %104 > 0 spbtn 8,3\nif %1201 != 0 || %1202 != 0 spbtn 9,4\n",
            "if %9 == 1 csp -1:loadgame %104\n",
        ] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
        assert!(!text.contains(",128\n"), "{text}");
    }

    #[test]
    fn title_screen_sounds_pressed_cell_and_continue() {
        use conv_core::ir::TitleButton;
        let mut s = script(vec![Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false }]);
        s.confirm = Some(menu::tests::dialog());
        s.title_screen = Some(TitleScreen {
            background: "system/title.png".into(),
            fade_ms: 1000,
            buttons: vec![
                TitleButton { image: "system/quick.png".into(), x: 822, y: 150, action: TitleAction::Continue, click_se: Some("sound/se/a.ogg".into()), shown_by: None },
                TitleButton { image: "system/start.png".into(), x: 762, y: 150, action: TitleAction::Start, click_se: Some("sound/se/a.ogg".into()), shown_by: None },
                // Shown only once a flag is set.
                TitleButton { image: "system/exit.png".into(), x: 162, y: 150, action: TitleAction::Exit, click_se: None, shown_by: Some(conv_core::ir::Var { global: false, index: 50 }) },
            ],
            pressed: true,
            hover_se: Some("sound/se/over.ogg".into()),
            bgm: Some("sound/bgm/title.ogg".into()),
            start_bgm_fade_ms: 3000,
            start_wait_ms: 2000,
            exit_without_asking: true,
            glow: Some(conv_core::ir::TitleGlow { image: "system/fire.jpg".into(), keys: vec![(0, 0), (600, 256), (1200, 0)] }),
            openings: vec![conv_core::ir::TitleOpening {
                movie: "movie/op.mp4".into(),
                at_launch: true,
                sound: Some("sound/se/op.ogg".into()),
                glow_keys: Some(vec![(0, 256), (100, 256), (700, 0)]),
                ..Default::default()
            }],
            ..Default::default()
        });
        let text = emit(&s);
        for part in [
            // The glow behind the buttons, redrawn with them.
            "*conv_title_draw\nbgm \"sound\\bgm\\title.ogg\"\nlsp 33,\":c;system\\fire.jpg\",0,0:spmotion 33,1,0,0,0,100,100,0,0,0,600,0,0,100,100,0,256,0,1200,0,0,100,100,0,0,0\nsavefileexist %105,890\nlsp 6,\":a/3,0,3;system\\quick.png\",822,150\nif %105 == 0 lsp 6,\":a/3,0,3;system\\quick.png\",822,150,128\n",
            "if %60 != 0 lsp 8,\":a/3,0,3;system\\exit.png\",162,150\n",
            "*conv_title\nselectvoice \"\",\"sound\\se\\over.ogg\",\"\"\nsavefileexist %105,890\nif %105 == 1 spbtn 6,1,2\nspbtn 7,2,2\nif %60 != 0 spbtn 8,3,2\nbtnwait %9\nselectvoice \"\",$118,\"\"\n",
            "if %9 == 1 dwave 1,\"sound\\se\\a.ogg\":csp -1:loadgame 890\n",
            // No question before quitting.
            "if %9 == 3 end\n",
            "*conv_title_start\ncsp -1\nbgmfadeout 3000:bgmstop:bgmfadeout 0\nbg black,",
            "wait 2000\n",
        ] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
        assert!(!text.contains("conv_title_exit"));
        // The opening plays once a run: cleared by the define section,
        // then the music, its sound, the movie and the title at once.
        for part in [
            "mov %2960,0\ngame\n",
            "if %2960 == 0 mov %2960,1:goto *conv_title_opening0\nmov %2960,1\n*conv_title_draw\n",
            "*conv_title_opening0\nbgm \"sound\\bgm\\title.ogg\"\ndwave 1,\"sound\\se\\op.ogg\"\nlsp 33,\":c;system\\fire.jpg\",0,0,255\n",
            "mpegplay \"movie\\op.mp4\",1,1\nspmotion 33,1,0,0,0,100,100,0,256,0,100,0,0,100,100,0,256,0,700,0,0,100,100,0,0,0\nprint 1\n",
        ] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
    }

    #[test]
    fn title_openings_by_flags_then_at_random() {
        use conv_core::ir::{TitleOpening, Var};
        let (done, seen) = (Var { global: true, index: 5 }, Var { global: true, index: 6 });
        let opening = |movie: &str, when: Vec<(Var, i32)>, sets: Vec<Var>, random| TitleOpening { movie: movie.into(), at_launch: true, when, sets, random, ..Default::default() };
        let openings = [
            opening("first", vec![(done, 1), (seen, 0)], vec![seen], false),
            opening("route", vec![(done, 1)], vec![], true),
            opening("plain", vec![], vec![], true),
        ];
        let mut out = String::new();
        title_openings(&mut out, &openings, 800);
        assert_eq!(
            out,
            "if %2960 == 0 && %805 == 1 && %806 == 0 mov %2960,1:mov %806,1:goto *conv_title_opening0\n\
             mov %105,0\n\
             if %2960 == 0 && %805 == 1 inc %105\n\
             if %2960 == 0 inc %105\n\
             if %105 > 0 rnd %9,%105\n\
             mov %105,0\n\
             if %2960 == 0 && %805 == 1 && %9 == %105 mov %2960,1:goto *conv_title_opening1\n\
             if %2960 == 0 && %805 == 1 inc %105\n\
             if %2960 == 0 && %9 == %105 mov %2960,1:goto *conv_title_opening2\n\
             if %2960 == 0 inc %105\n"
        );
    }

    /// The startup notice's lines, which the tests comparing a whole script leave out.
    fn notice_of(s: &Script) -> String {
        let mut out = String::new();
        notice::emit(&mut out, s.width, s.height);
        out
    }

    #[test]
    fn title_screen_runs_before_the_script() {
        use conv_core::ir::TitleButton;
        let button = |image: &str, action| TitleButton { image: image.into(), x: 600, y: 200, action, ..Default::default() };
        let mut s = script(vec![Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false }]);
        s.title_screen = Some(TitleScreen {
            background: "system/title.png".into(),
            fade_ms: 1000,
            buttons: vec![
                button("system/start.png", TitleAction::Start),
                button("system/option.png", TitleAction::Unavailable),
                button("system/load.png", TitleAction::Load),
                button("system/exit.png", TitleAction::Exit),
            ],
            ..Default::default()
        });
        let out = emit(&s).replace(&notice_of(&s), "");
        let expected = "*start\nbg \"system\\title.png\",0\n*conv_title_draw\n\
            lsp 6,\":a/2,0,3;system\\start.png\",600,200\n\
            lsp 7,\":a;system\\option.png\",600,200\n\
            lsp 8,\":a/2,0,3;system\\load.png\",600,200\n\
            lsp 9,\":a/2,0,3;system\\exit.png\",600,200\n\
            print 2\n*conv_title\nspbtn 6,1\nspbtn 8,2\nspbtn 9,3\nbtnwait %9\n\
            if %9 == 1 goto *conv_title_start\n\
            if %9 == 2 systemcall load:goto *conv_title\n\
            if %9 == 3 end\n\
            goto *conv_title\n*conv_title_start\ncsp -1\nbg black,2\nあ\\\n";
        assert!(out.contains(expected), "{out}");
    }

    #[test]
    fn extras_open_from_the_title_option() {
        use conv_core::ir::{CgGallery, CgViewer, ExtrasAction, ExtrasItem, ExtrasMenu, GallerySlot, GalleryTab, Placed, TitleButton};
        let placed = |image: &str| Placed { image: image.into(), x: 10, y: 20, w: 30, h: 40 };
        let mut s = script(vec![]);
        s.title_screen = Some(TitleScreen {
            background: "system/title.png".into(),
            fade_ms: 1000,
            buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
            ..Default::default()
        });
        let slot = GallerySlot { thumbnail: "system/t.png".into(), cgs: vec![(213, "bg/a.png".into()), (214, "bg/b.png".into())] };
        s.extras = Some(ExtrasMenu {
            background: "system/extras.png".into(),
            items: vec![
                ExtrasItem { button: placed("system/cg.png"), action: ExtrasAction::Gallery },
                ExtrasItem { button: placed("system/exit.png"), action: ExtrasAction::Back },
            ],
            gallery: Some(CgGallery {
                header: placed("h"),
                exit: placed("x"),
                tabs: vec![GalleryTab { button: placed("tab"), selected: "tab_on".into(), title: placed("title"), viewer_name: placed("name"), pages: vec![vec![slot]] }],
                grid_x: 44,
                grid_y: 142,
                grid_dx: 108,
                grid_dy: 92,
                columns: 5,
                locked: "system/locked.png".into(),
                page_label: placed("page"),
                page_numbers: vec!["p1".into()],
                pages_x: 67,
                pages_y: 521,
                page_dx: 24,
                page_marker: "on".into(),
                marker_dx: 0,
                marker_dy: -3,
                viewer: CgViewer { panel: vec![placed("panel")], name_x: 1, name_y: 2, previous: placed("p"), next: placed("n"), hide: placed("c"), back: placed("b") },
            }),
            replay: None,
            music: None,
            movies: None,
            menu_background: None,
        });
        let out = emit(&s);
        assert!(out.contains("*conv_title_back\nbg \"system\\title.png\",0\n"), "{out}");
        assert!(out.contains("if %9 == 1 csp -1:goto *conv_extras\n"), "{out}");
        // The gallery opens once one of its CGs is seen.
        assert!(out.contains("if %2413 == 1 mov %128,1:if %2414 == 1 mov %128,1\n"), "{out}");
        assert!(out.contains("if %128 == 0 cell 100,2\nif %128 == 1 spbtn 100,1\n"), "{out}");
        // A slot is a button once one of its CGs is seen, else locked.
        assert!(out.contains("mov %122,0:if %2413 == 1 mov %122,1:if %2414 == 1 mov %122,1\n\
            if %122 == 1 lsp 130,\":a/2,0,3;system\\t.png\",40,138:spbtn 130,100\n\
            if %122 == 0 lsp 130,\":a;system\\locked.png\",44,142\n"), "{out}");
        // The viewer steps past unseen variants.
        assert!(out.contains("*conv_cg_v0_1\nif %2414 == 0 add %124,%126:goto *conv_cg_v0\nbg \"bg\\b.png\",0:goto *conv_cg_view_show\n"), "{out}");
    }

    #[test]
    fn scene_replay_plays_unlocked_scenes_to_their_end() {
        use conv_core::ir::{ExtrasAction, ExtrasItem, ExtrasMenu, Placed, ReplayList, ReplayScene, SceneReplay, TitleButton};
        let placed = |image: &str| Placed { image: image.into(), x: 10, y: 20, w: 30, h: 40 };
        let mut s = script(vec![Op::Label("s4_l2".into()), Op::SceneEnd, Op::UnlockScene { scene: 5 }]);
        s.title_screen = Some(TitleScreen {
            background: "system/title.png".into(),
            fade_ms: 1000,
            buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
            ..Default::default()
        });
        let scene = |scene, title: &str, label: &str, highlight| ReplayScene { scene, title: title.into(), label: label.into(), highlight };
        s.extras = Some(ExtrasMenu {
            background: "system/extras.png".into(),
            items: vec![ExtrasItem { button: placed("system/chapter.png"), action: ExtrasAction::Replay }],
            gallery: None,
            replay: Some(SceneReplay {
                header: placed("h"),
                exit: placed("x"),
                lists: vec![ReplayList {
                    button: placed("list"),
                    selected: placed("list_on"),
                    scenes: vec![scene(5, "工場への道", "s14_l3", false), scene(7, "Ｈ", "s14_l5", true)],
                }],
                list_background: "system/back.png".into(),
                frame: placed("frame"),
                close: placed("close"),
                selector: "system/row.png".into(),
                new_mark: "system/new.png".into(),
                rows: 23,
                columns: 3,
                column_dx: 258,
                row_dy: 21,
                row_y: 58,
                selector_x: 15,
                new_x: 16,
                text_x: 21,
                text_y: 63,
                font_size: 14,
                text_color: "#FEFEFE".into(),
                highlight_color: "#FEC7C7".into(),
                leave_label: "シーン選択に戻る".into(),
                leave_question: "シーン選択に戻りますか？".into(),
            }),
            music: None,
            movies: None,
            menu_background: None,
        });
        let out = emit(&s);
        assert!(out.contains("if %9 == 1 goto *conv_replay\n"), "{out}");
        // A replay stops at the scene's end.
        assert!(out.contains("*s4_l2\nif %152 == 1 goto *conv_replay_end\nmov %3005,1\n"), "{out}");
        // A list is open once one of its scenes is unlocked.
        assert!(out.contains("mov %160,0\nif %3005 == 1 mov %160,1:if %3007 == 1 mov %160,1\n"), "{out}");
        // Unlocked scenes get a row, "new" until replayed.
        assert!(out.contains(
            "*conv_replay_rows0\n\
            if %3005 == 1 mov $130,\"工場への道\":mov %155,1:mov %156,1-%3655:mov %157,0:gosub *conv_replay_row\n\
            if %3007 == 1 mov $130,\"Ｈ\":mov %155,2:mov %156,1-%3657:mov %157,1:gosub *conv_replay_row\n"
        ), "{out}");
        assert!(out.contains("*conv_replay_go0_1\nmov %3657,1\ngosub *conv_replay_begin:goto *s14_l5\n"), "{out}");
    }

    #[test]
    fn movie_room_shows_played_movies() {
        use conv_core::ir::{ExtrasAction, ExtrasItem, ExtrasMenu, MovieRoom, Placed, RoomMovie, TitleButton};
        let placed = |image: &str| Placed { image: image.into(), x: 120, y: 71, w: 30, h: 20 };
        let mut s = script(vec![Op::Movie { file: "movie/op01.mp4".into(), skippable: true }]);
        s.title_screen = Some(TitleScreen {
            background: "system/title.png".into(),
            fade_ms: 1000,
            buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
            ..Default::default()
        });
        let movie = |image: &str, file: &str, open| RoomMovie { button: placed(image), file: file.into(), open };
        s.extras = Some(ExtrasMenu {
            background: "system/extras.png".into(),
            items: vec![ExtrasItem { button: placed("system/mv.png"), action: ExtrasAction::Movies }],
            gallery: None,
            replay: None,
            music: None,
            movies: Some(MovieRoom {
                exit: placed("x"),
                movies: vec![movie("system/m0.png", "movie/op00.mp4", true), movie("system/m1.png", "movie/op01.mp4", false)],
                background: None,
            }),
            menu_background: Some("system/title.png".into()),
        });
        let out = emit(&s);
        assert!(out.contains("if %9 == 1 goto *conv_movies\n"), "{out}");
        // The menu over its own picture, the room over the extras' one.
        assert!(out.contains("*conv_extras_draw\ncsp -1\nbg \"system\\title.png\",0\n"), "{out}");
        assert!(out.contains("bg \"system\\extras.png\",0"), "{out}");
        // A scene playing a movie of the room marks it.
        assert!(out.contains("mpegplay \"movie\\op01.mp4\",1\nmov %2801,1\n"), "{out}");
        assert!(out.contains(
            "lsp 102,\":a/2,0,3;system\\m0.png\",120,71:spbtn 102,1\n\
            if %2801 == 1 lsp 103,\":a/2,0,3;system\\m1.png\",120,71:spbtn 103,2\n"
        ), "{out}");
        assert!(out.contains("*conv_movie1\nbgmstop:mov %161,0:mpegplay \"movie\\op01.mp4\",1:goto *conv_movies_draw\n"), "{out}");
    }

    #[test]
    fn music_room_plays_a_song_and_marks_it() {
        use conv_core::ir::{ExtrasAction, ExtrasItem, ExtrasMenu, MusicRoom, MusicSong, Placed, TitleButton};
        let placed = |image: &str| Placed { image: image.into(), x: 120, y: 71, w: 30, h: 20 };
        let mut s = script(vec![]);
        s.title_screen = Some(TitleScreen {
            background: "system/title.png".into(),
            fade_ms: 1000,
            buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
            ..Default::default()
        });
        let song = |image: &str, file: &str, progress| MusicSong { button: placed(image), playing: format!("{image}_on"), file: file.into(), progress };
        s.extras = Some(ExtrasMenu {
            background: "system/extras.png".into(),
            items: vec![ExtrasItem { button: placed("system/bgm.png"), action: ExtrasAction::Music }],
            gallery: None,
            replay: None,
            music: Some(MusicRoom {
                header: placed("h"),
                exit: placed("x"),
                songs: vec![song("system/s1.png", "bgm/bgm01.ogg", None), song("system/s2.png", "bgm/bgm22.ogg", Some(0))],
                background: Some("system/music.png".into()),
                player: None,
            }),
            movies: None,
            menu_background: None,
        });
        let out = emit(&s);
        assert!(out.contains("if %9 == 1 goto *conv_music\n"), "{out}");
        assert!(out.contains("*conv_music\ncsp -1\nbg \"system\\music.png\",0\n"), "{out}");
        // The song playing is drawn marked; a locked one isn't drawn.
        assert!(out.contains(
            "if %161 != 1 lsp 102,\":a/2,0,3;system\\s1.png\",120,71:spbtn 102,1\n\
            if %161 == 1 lsp 102,\":a/2,0,3;system\\s1.png_on\",120,71:spbtn 102,1\n\
            if %3600 == 1 && %161 != 2 lsp 103,"
        ), "{out}");
        assert!(out.contains("*conv_music1\nbgmfadein 0:bgm \"bgm\\bgm22.ogg\":goto *conv_music_draw\n"), "{out}");
    }

    #[test]
    fn full_screen_pages_continue() {
        let mut nvl = window();
        nvl.name_plate = None;
        nvl.image = "system/textwnd5.png".into();
        let text = |t: &str, continues| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues };
        let s = script(vec![Op::SetWindow(nvl), text("一行目", true), text("二行目", false), text("新頁", false)]);
        let out = emit(&s);
        assert!(out.contains("\":a;system\\textwnd5.png\",47,452\ncsp 50:csp 49\n一行目@\n二行目\\\n新頁\\\n"), "{out}");
    }

    #[test]
    fn a_page_goes_on_after_code_inside_it() {
        let text = |t: &str| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false };
        let image = Op::WindowImage("system/face2.png".into());
        let s = script(vec![text("《\u{5}"), Op::Wait { ms: 50 }, image, text("興\n\u{5}"), Op::Wait { ms: 50 }, text("》")]);
        let out = emit(&s);
        assert!(out.contains("《/\ndelay 50\nsetwindow2 \":a;system\\face2.png\"\n興\ndelay 50\n》\\\n"), "{out}");
        let mut s = script(vec![text("One\u{5}"), Op::Wait { ms: 50 }, text(" two")]);
        s.single_byte_text = true;
        assert!(emit(&s).contains("`One`/\ndelay 50\n` two\\\n"));
    }

    #[test]
    fn link_positions() {
        let w = window();
        assert_eq!(char_position(&w, "あいう", 2), (105 + 2 * 25, 470));
        assert_eq!(char_position(&w, "あ\nabc", 2), (105 + 25, 507));
    }

    fn local(index: u16) -> Operand {
        Operand::Var(Var { global: false, index })
    }

    #[test]
    fn emits_a_scene() {
        let s = script(vec![
            Op::Label("scene_1".into()),
            Op::BgmPlay { file: "bgm/track01.ogg".into(), fade_in_ms: 1000 },
            Op::Background { image: "bg/room.png".into(), fade_ms: 500 },
            Op::Sprites {
                changes: vec![SpriteChange {
                    layer: 3,
                    sprite: Some(SpritePlacement { image: "sprite/a.png".into(), x: 10, y: -20 }),
                    keep_filters: true,
                }],
                fade_ms: 500,
            },
            Op::SpriteFilters {
                layer: 3,
                filters: vec![
                    SpriteFilter {
                        kind: Operand::Const(FILTER_COLOUR),
                        blend: Operand::Const(FILTER_BLEND_MULTIPLY),
                        strength: Operand::Const(128),
                        params: [255, 200, 100].map(Operand::Const),
                    },
                    SpriteFilter {
                        kind: Operand::Const(0),
                        blend: Operand::Const(0),
                        strength: Operand::Const(256),
                        params: [Operand::Const(0); 3],
                    },
                    SpriteFilter {
                        kind: local(30),
                        blend: local(31),
                        strength: local(32),
                        params: [local(33), local(34), local(35)],
                    },
                ],
            },
            Op::SpriteOpacity { layer: Some(3), opacity: 128 },
            Op::SpriteOpacity { layer: None, opacity: 256 },
            Op::SpriteMove { layer: 3, x: -400, y: 300, ms: 0 },
            Op::Sprites { changes: vec![SpriteChange { layer: 3, sprite: None, keep_filters: false }], fade_ms: 0 },
            Op::Text { speaker: Some("名前".into()), text: "「こんにちは」".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
            Op::Text { speaker: None, text: "地の文。".into(), voice: Some(Voice { file: "voice/v1.ogg".into(), character: 3 }), links: vec![link(1, 2, None)], ruby: vec![], styles: vec![], hang: None, continues: false },
            Op::SePlay { file: "se/k001.ogg".into(), looped: true },
            Op::SeStop { file: Some("se/k001.ogg".into()) },
            Op::BgmStop { fade_out_ms: 2000 },
            Op::Wait { ms: 500 },
            Op::Unsupported { what: "op 0x76".into(), at: SourceLoc { file: "0003.cd".into(), index: 7 } },
        ]);
        let text = emit(&s).replace(&notice_of(&s), "");
        let expected = "\
;mode800,V8192
*define
nsa
globalon
caption \"Test\"
effect 2,10,500
game
*start
*scene_1
bgmfadein 1000
bgm \"bgm\\track01.ogg\"
csp -1
bg \"bg\\room.png\",2
lsp 65,\":a;sprite\\a.png\",10,-20
print 2
spfilter 65,0
spfilter 65,1,1,128,255,200,100
if %40 != 0 spfilter 65,%40,%41,%42,%43,%44,%45
print 1
msp 65,0,0,-255:msp 65,0,0,127
print 1
if %180 != 0 msp %180,0,0,-255:msp %180,0,0,255
print 1
getspsize 65,%1,%2:amsp 65,-400-%1/2,300-%2/2
print 1
csp 65
print 1
【名前】「こんにちは」\\
dwave 0,\"voice\\v1.ogg\"
地#FEE092の文#FFFFFF。\\
dwaveloop 1,\"se\\k001.ogg\"
dwavestop 1
bgmfadeout 2000
bgmstop
delay 500
; unsupported: op 0x76 (0003.cd #7)
end
";
        assert_eq!(text, expected);
    }

    #[test]
    fn adv_window_and_name_plate() {
        use conv_core::ir::{NamePlate, TextWindow};
        let mut s = script(vec![
            Op::Text { speaker: Some("名前".into()), text: "「台詞」".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
            Op::Text { speaker: None, text: "Ghost".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        ]);
        s.window = Some(TextWindow {
            wait_icon_at: None,
            image: "system/window.png".into(),
            x: 47,
            y: 452,
            text_x: 105,
            text_y: 470,
            columns: 25,
            rows: 3,
            font_size: 24,
            line_gap: 13,
            vertical: false,
            name_plate: Some(NamePlate {
                image: "system/name.png".into(),
                x: 55,
                y: 412,
                text_x: 75,
                text_y: 422,
                font_size: 20,
            }),
        });
        let text = emit(&s);
        assert!(text.contains("setwindow3 105,470,25,3,24,24,1,13,20,0,1,\":a;system\\window.png\",47,452\nerasetextwindow 0\n"));
        assert!(text.contains("lsp 50,\":a;system\\name.png\",55,412\nstrsp 49,\"名前\",75,422,16,1,20,20,1,0,0,1\nprint 1\n「台詞」\\\n"));
        assert!(text.contains("csp 50\ncsp 49\nprint 1\n　Ghost \\\n"));
        // With a backlog, lines carry their page's first speaker and voice.
        use conv_core::ir::{Backlog, Placed, Rect};
        let p = Placed { image: "system/x.png".into(), x: 0, y: 0, w: 10, h: 10 };
        s.backlog = Some(Backlog {
            base: p.clone(),
            base_opacity: 200,
            frame: p.clone(),
            track: p.clone(),
            up: p.clone(),
            down: p.clone(),
            close: p,
            area: Rect { x: 30, y: 44, w: 700, h: 516 },
            font_size: 16,
            line_height: 24,
            columns: 41,
        });
        let voice = |n: &str| Some(Voice { file: format!("voice/{n}.ogg"), character: 1 });
        s.ops = vec![
            Op::Text { speaker: None, text: "地の文".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: true },
            Op::Text { speaker: Some("名前".into()), text: "「一」\n「二」".into(), voice: voice("a"), links: vec![], ruby: vec![], styles: vec![], hang: None, continues: true },
            Op::Text { speaker: Some("他".into()), text: "「三」".into(), voice: voice("b"), links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
            Op::Text { speaker: None, text: "地の文".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        ];
        let text = emit(&s);
        assert!(text.contains("pretextgosub *conv_pretext\n"));
        assert!(text.contains("\n地の文@\n"));
        assert!(text.contains("\n[名前|voice/a.ogg]「一」\n[名前|voice/a.ogg]「二」@\n"));
        assert!(text.contains("\n[名前|voice/a.ogg]「三」\\\n"));
        assert!(text.contains("\n地の文\\\n"));
        assert!(text.contains("*conv_pretext\nsaveon\nreturn\n"));
    }

    #[test]
    fn multi_line_text() {
        let s = script(vec![Op::Text {
            speaker: None,
            text: "一行目\nabc\n三行目".into(),
            voice: None,
            links: vec![link(4, 3, None)],
            ruby: vec![],
            styles: vec![],
            hang: None,
            continues: false,
        }]);
        assert!(emit(&s).contains("一行目\n　#FEE092abc #FFFFFF\n三行目\\\n"));
    }

    #[test]
    fn links_are_coloured() {
        assert_eq!(with_markup("あいうえお", &[link(1, 2, None), link(3, 9, None)], &[], &[]), "あ#FEE092いう#FFFFFFえお");
        assert_eq!(with_markup("a#b", &[], &[], &[]), "a＃b");
    }

    #[test]
    fn flow_and_variables() {
        let g = Var { global: true, index: 7 };
        let l = Var { global: false, index: 2 };
        let s = script(vec![
            Op::Arith { var: g, op: ArithOp::Set, value: Operand::Const(1) },
            Op::Arith { var: l, op: ArithOp::Add, value: Operand::Var(g) },
            Op::If { var: g, cmp: Compare::Eq, value: Operand::Const(1), label: "s3_l2".into() },
            Op::Gosub("s2_l18".into()),
            Op::Return,
        ]);
        assert!(emit(&s).contains("mov %207,1\nadd %12,%207\nif %207 == 1 goto *s3_l2\ngosub *s2_l18\nreturn\n"));
    }

    #[test]
    fn bitwise_ops_use_engine_commands() {
        let g = Var { global: true, index: 9 };
        let h = Var { global: true, index: 12 };
        let text = emit(&script(vec![Op::Arith { var: g, op: ArithOp::BitAnd, value: Operand::Var(h) }]));
        assert!(text.contains("bitand %209,%209,%212\n"), "{text}");
    }

    #[test]
    fn sprite_moves_are_engine_motions() {
        let text = emit(&script(vec![Op::SpriteMove { layer: 5, x: 550, y: 300, ms: 300 }]));
        assert!(text.contains(",300,550-%1/2-%3,300-%2/2-%4,100,100,0,256,0:spmotionwait 67\n"), "{text}");
        assert!(!text.contains("spcurve"));
        // An eased one: its curve's table at *start, the motion along it.
        let text = emit(&script(vec![Op::MoveEase { ease: 2, exponent: 3 }, Op::SpriteMove { layer: 5, x: 550, y: 300, ms: 300 }]));
        assert!(text.contains("\n*start\nspcurve 32,0,"), "{text}");
        assert!(text.contains(",256,32:spmotionwait 67\n"), "{text}");
        // One not waited for, then waited for later; the next one waits.
        let move_to = |x| Op::SpriteMove { layer: 5, x, y: 300, ms: 300 };
        let text = emit(&script(vec![Op::MoveNoWait, move_to(550), Op::MotionWait { layer: 5 }, move_to(600)]));
        assert!(text.contains("550-%1/2-%3,300-%2/2-%4,100,100,0,256,0\nspmotionwait 67\n"), "{text}");
        assert!(text.contains("600-%1/2-%3,300-%2/2-%4,100,100,0,256,0:spmotionwait 67\n"), "{text}");
    }

    /// Runs the helper's algorithm (as emitted) in Rust to check the maths.
    #[test]
    fn bit_helper_algorithm() {
        fn run(a: i32, b: i32, and: bool) -> i32 {
            let (mut r, mut bit, mut x, mut y) = (0, 1, a, b);
            while !(x == 0 && y == 0) {
                let t = x % 2 + y % 2;
                if (and && t == 2) || (!and && t > 0) {
                    r += bit;
                }
                x /= 2;
                y /= 2;
                bit *= 2;
            }
            r
        }
        for (a, b) in [(0, 0), (1, 1), (1, 0), (6, 3), (12, 10), (255, 15), (1 << 20, (1 << 20) + 5)] {
            assert_eq!(run(a, b, true), a & b, "{a} & {b}");
            assert_eq!(run(a, b, false), a | b, "{a} | {b}");
        }
    }

    #[test]
    fn text_is_made_safe() {
        assert_eq!(text_safe("Ghost: 50% @home"), "　Ghost: 50％ ＠home");
        assert_eq!(text_safe("「a/b」"), "「a／b」");
        assert_eq!(single_byte_text("\"Hey! 50% (a/b) #1 @me\\`\"", &[]), "\"Hey! 50% (a/b) #1 ＠me＼｀\"");
        let slant = |start, len, centred: bool| TextStyle { start, len, italic: !centred, centred, ..Default::default() };
        assert_eq!(single_byte_text("Oh!it\nab cd", &[slant(4, 4, false), slant(6, 5, true)]), "Oh！i!i1t!i0\n!c!i1ab!i0 cd");
        assert_eq!(single_byte_text("x\nab", &[slant(2, 2, true)]), "x\n!cab");
        // Tag-like text turns full-width; colours and sizes become tags,
        // ended at the range's or the line's end.
        assert_eq!(single_byte_text("!w5 !sd #00ff00 !wow", &[]), "！w5 ！sd ＃00ff00 !wow");
        let red = TextStyle { start: 0, len: 2, colour: Some(0xCC0000), ..Default::default() };
        let small = TextStyle { start: 3, len: 9, size_percent: Some(75), ..Default::default() };
        assert_eq!(single_byte_text("ab cd\nef", &[red, small]), "#CC0000ab#FFFFFF !z075cd!z100\n!z075ef!z100");
        // Outlines: "!o" before '#' or a digit is a tag; closed at the line's end.
        assert_eq!(single_byte_text("!o# !o1 !ok", &[]), "！o# ！o1 !ok");
        let grey = TextStyle { start: 1, len: 3, outline: Some(0x808080), ..Default::default() };
        assert_eq!(single_byte_text("ab\ncd", &[grey]), "a!o#808080b!o0\n!o#808080c!o0d");
        // Waits: a space ends a number before a digit; one before spaces
        // goes after them (the engine skips spaces after the number).
        let (t, v, e) = (conv_core::ir::TIMED_WAIT_MARK, conv_core::ir::VOICE_WAIT_MARK, conv_core::ir::WAIT_END_MARK);
        assert_eq!(wait_tags(&format!("0{t}300{e}1 a{v}5000{e}  b")), "0!d300 1 a  !v5000b");
        // 最果てのイマ, *conv_link19: the '?' stopped the engine (array).
        assert_eq!(text_safe("嘘ー！?  投稿者: x"), "嘘ー！？  投稿者: x");
        // Every ASCII character the engine's text parser reads specially:
        // variables (% ? $), waits (@ \ _), tags (! # ( / ) < > [ ]), and
        // what starts a token or a comment (` " ;). A ',' is only special
        // at the start of a line, which text_safe guards.
        let safe = text_safe("%?$@\\_!#(/)<>[]`\";*");
        assert!(!safe.contains(|c: char| c.is_ascii()), "{safe}");
    }

    #[test]
    fn single_byte_runs_keep_their_pairs() {
        // A space where a pair would start is skipped by ONScripter.
        assert_eq!(align_single_bytes("ABC続き"), "ABC 続き");
        assert_eq!(align_single_bytes("AB続き"), "AB続き");
        assert_eq!(align_single_bytes(" xyz続"), " xyz 続");
        assert_eq!(align_single_bytes("a b続"), "a b 続");
        assert_eq!(align_single_bytes("ｼｬﾄﾞｳshadow）"), "ｼｬﾄﾞｳshadow ）");
        assert_eq!(align_single_bytes("x#FEE092y#FFFFFF。"), "x #FEE092y #FFFFFF。");
        assert_eq!(align_single_bytes("ab c"), "ab c ");
    }

    #[test]
    fn sprite_texts_are_not_padded() {
        assert_eq!(sprite_text("ＳＡＴＩＥ'Ｓ・ＴＩＭＥ"), "ＳＡＴＩＥ'Ｓ・ＴＩＭＥ");
        assert_eq!(sprite_text("『TCP/IP』"), "『TCP／IP』");
    }

    #[test]
    fn encodes_shift_jis() {
        let (bytes, lost) = encode("あ\u{1F600}");
        assert_eq!(bytes, [0x82, 0xa0, b'?']);
        assert_eq!(lost, 1);
    }

    #[test]
    fn se_channels_rotate_and_stop() {
        let mut ch = SeChannels::default();
        assert_eq!(ch.start("a", false), 1);
        assert_eq!(ch.start("b", false), 2);
        assert_eq!(ch.stop("a"), Some(1));
        assert_eq!(ch.stop("a"), None);
        assert_eq!(ch.stop_all(), vec![2]);
    }

    #[test]
    fn se_channels_keep_loops() {
        let mut ch = SeChannels::default();
        // Nine loops fill 1-9; the next sounds go past them.
        let loops: Vec<u32> = (0..9).map(|i| ch.start(&format!("l{i}"), true)).collect();
        assert_eq!(loops, (1..=9).collect::<Vec<_>>());
        assert_eq!(ch.start("x", false), 11);
        assert_eq!((ch.start("y", true), ch.start("v", false)), (11, 12));
        // A stopped loop frees its channel; one-shots take turns as before.
        assert_eq!(ch.stop("l2"), Some(3));
        assert_eq!((ch.start("z", false), ch.start("w", false)), (3, 3));
    }

    #[test]
    fn movies_stop_the_music() {
        let out = emit(&script(vec![Op::Movie { file: "movie/mv0001.mp4".into(), skippable: true }]));
        assert!(out.contains("bgmstop\nmpegplay \"movie\\mv0001.mp4\",1\n"), "{out}");
    }

    #[test]
    fn japanese_caption() {
        let mut ir = script(Vec::new());
        ir.title = "最果てのイマ \"COMPLETE\"".into();
        assert!(emit(&ir).contains("caption \"最果てのイマ COMPLETE\"\n"));
    }

    #[test]
    fn screen_filters() {
        let dark = SpriteFilter {
            kind: Operand::Const(FILTER_BRIGHTNESS),
            blend: Operand::Const(FILTER_BLEND_COPY),
            strength: Operand::Const(256),
            params: [-200, 0, 0].map(Operand::Const),
        };
        let s = script(vec![
            Op::ScreenFilter { slot: 1, filter: Some(dark), fade_ms: 1000 },
            Op::ScreenFilter { slot: 1, filter: None, fade_ms: 0 },
            Op::ScreenFilter { slot: 0, filter: None, fade_ms: 0 },
        ]);
        let out = emit(&s);
        assert!(out.contains("effect 2,10,1000\n"), "{out}");
        assert!(out.contains("screenfilter 1,4,0,256,-200,0,0\nprint 2\nscreenfilter 1,0\nprint 1\nscreenfilter 0\nprint 1\n"), "{out}");
    }

    #[test]
    fn animations_play_and_outlast_the_background() {
        let a = Animation { image: "fx/fx0401.png".into(), frame_ms: vec![33, 50], looped: false, opacity: 256 };
        let s = script(vec![Op::Animation(Some(a)), Op::Background { image: "bg/room.png".into(), fade_ms: 0 }, Op::Animation(None)]);
        let out = emit(&s);
        assert!(
            out.contains(
                "csp 59\nmov %117,255:mov $117,\":c/2,<33,50>,1;fx\\fx0401.png\"\nlsp 59,$117,0,0,%117\nprint 1\n\
                 csp -1\nif %117 != 0 lsp 59,$117,0,0,%117\nbg \"bg\\room.png\",1\n\
                 csp 59\nmov %117,0\nprint 1\n"
            ),
            "{out}"
        );
    }

    #[test]
    fn overlays_outlast_the_background() {
        let s = script(vec![
            Op::Overlay { image: Some("bg/frame01.png".into()), over_sprites: true },
            Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
            Op::Overlay { image: None, over_sprites: true },
        ]);
        let out = emit(&s);
        assert!(
            out.contains(
                "if %180 != 0 csp %180\nmov %180,61:mov $180,\":a;bg\\frame01.png\"\nlsp 61,$180,0,0\nprint 1\n\
                 csp -1\nif %180 != 0 lsp %180,$180,0,0\nbg \"bg\\room.png\",1\n\
                 if %180 != 0 csp %180\nmov %180,0\nprint 1\n"
            ),
            "{out}"
        );
    }

    #[test]
    fn noise_outlasts_the_background() {
        let noise = Noise { image: "system/noise.png".into(), frames: 4, frame_ms: 100, opacity: 120 };
        let s = script(vec![
            Op::Noise(Some(noise.clone())),
            Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
            Op::Noise(None),
        ]);
        let out = emit(&s);
        assert!(
            out.contains(
                "csp 60\nmov %116,120:mov $116,\":c/4,100,0;system\\noise.png\"\nlsp 60,$116,0,0,%116\nprint 1\n\
                 csp -1\nif %116 != 0 lsp 60,$116,0,0,%116\nbg \"bg\\room.png\",1\n\
                 csp 60\nmov %116,0\nprint 1\n"
            ),
            "{out}"
        );
        let out = emit(&script(vec![Op::NoiseFadeIn { ms: 2000 }, Op::Noise(Some(noise))]));
        assert!(out.contains("lsp 60,$116,0,0,%116\nspmotion 60,0,0,0,0,100,100,0,0,0,2000,0,0,100,100,0,256,0\nprint 1\n"), "{out}");
    }

    #[test]
    fn screen_effects() {
        let shake = ScreenEffect::Shake { time_ms: 1000, interval_ms: 100, path: false, speed: (800, 800), range: [(0, 0), (30, 30)], end_ms: 0 };
        let ripple = ScreenEffect::Ripple {
            map: "system/mask1001.png".into(),
            time_ms: 3000,
            interval_ms: 100,
            opacity: (100, 150),
            amplitude: (5, 10),
            angle: 3150,
            angle_step: 20,
            phase_step: 30,
            multiply: false,
            phase: 10,
        };
        let s = script(vec![
            Op::ScreenEffect { slot: 1, effect: Some(shake) },
            Op::ScreenEffect { slot: 2, effect: Some(ripple) },
            Op::ScreenEffect { slot: 1, effect: None },
            Op::ScreenEffect { slot: 0, effect: None },
            Op::ScreenEffect {
                slot: 3,
                effect: Some(ScreenEffect::Pulse {
                    time_ms: 500,
                    interval_ms: 10,
                    zoom: (150, 150),
                    opacity: 150,
                    colour: 0xff8080,
                    tint_opacity: 256,
                    tint: 1,
                }),
            },
        ]);
        let out = emit(&s);
        assert!(
            out.contains(
                "screenshake 1,1000,100,0,800,800,0,0,30,30\nprint 1\n\
                 screenripple 2,\"system\\mask1001.png\",3000,100,100,150,5,10,3150,20,30,0,10\nprint 1\n\
                 screenshake 1\nprint 1\nscreenshake 0\nprint 1\n\
                 screenpulse 3,500,10,150,150,150,16744576,256,1\nprint 1\n"
            ),
            "{out}"
        );
    }

    #[test]
    fn sprites_in_front_of_the_overlay() {
        let out = emit(&script(vec![Op::SpriteInFront { layer: 7, front: true }, Op::SpriteInFront { layer: 7, front: false }]));
        assert!(out.contains("spz 69,61\nspz 69\n"), "{out}");
    }

    #[test]
    fn screen_flashes() {
        let out = emit(&script(vec![Op::ScreenFlash { colour: 0xff0000, opacity: 128, ms: 100 }]));
        assert!(out.contains("screenflash 128,\"#ff0000\"\nprint 1\ndelay 100\nscreenflash 0\nprint 1\n"), "{out}");
    }

    #[test]
    fn flashes() {
        let out = emit(&script(vec![Op::Flash { colour: 0xff0000, opacity: 128, ms: 100 }]));
        assert!(
            out.contains("lsp2add 24,\":c;>800,600,#ff0000\",400,300,100,100,0,127\nprint 1\ndelay 100\ncsp2 24\nprint 1\n"),
            "{out}"
        );
    }

    #[test]
    fn sound_effects_fade() {
        let out = emit(&script(vec![
            Op::SeFadeIn { ms: 2000 },
            Op::SePlay { file: "se/rain.ogg".into(), looped: true },
            Op::SeFadeOut { file: Some("se/rain.ogg".into()), ms: 1500 },
        ]));
        assert!(out.contains("dwavefadein 2000\ndwaveloop 1,\"se\\rain.ogg\"\nchvolfade 1,0,1500,1\n"), "{out}");
    }

    #[test]
    fn sound_effects_pan() {
        let shot = || "se/shot.ogg".to_string();
        let mut ops = vec![
            Op::SoundPan { file: shot(), pan: -1000, ms: 0 },
            Op::SoundVolume { file: Some(shot()), percent: 50, ms: 0 },
            Op::SePlay { file: shot(), looped: false },
            Op::SoundPan { file: shot(), pan: 1000, ms: 1500 },
        ];
        // Round the channels to the panned one again: centred.
        let span = SE_CHANNELS.end() - SE_CHANNELS.start() + 1;
        ops.extend((0..span).map(|i| Op::SePlay { file: format!("se/{i}.ogg"), looped: false }));
        let out = emit(&script(ops));
        assert!(out.contains("chpan 1,-1000\nchvolfade 1,50\ndwave 1,\"se\\shot.ogg\"\nchpan 1,1000,1500\n"), "{out}");
        assert_eq!(out.matches("chpan 1,0\n").count(), 1, "{out}");
        assert_eq!(out.matches("chpan").count(), 3, "{out}");
    }

    #[test]
    fn some_sounds_are_left_out_while_skipping() {
        let out = emit(&script(vec![
            Op::SeNotSkipped,
            Op::SeFadeIn { ms: 300 },
            Op::SePlay { file: "se/door.ogg".into(), looped: false },
            Op::SePlay { file: "se/rain.ogg".into(), looped: true },
        ]));
        assert!(
            out.contains("isskip %105\nif %105 != 1 dwavefadein 300\nif %105 != 1 dwave 1,\"se\\door.ogg\"\ndwaveloop 2,\"se\\rain.ogg\"\n"),
            "{out}"
        );
    }

    #[test]
    fn pauses_ignore_clicks() {
        let out = emit(&script(vec![Op::Wait { ms: 300 }, Op::Pause { ms: 500 }]));
        assert!(out.contains("delay 300\nwait 500\n"), "{out}");
    }

    #[test]
    fn fades_to_a_colour_until_the_next_background() {
        let out = emit(&script(vec![
            Op::ScreenFade { colour: 0xffffff, ms: 2000 },
            Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
        ]));
        assert!(out.contains("effect 2,10,2000\n"), "{out}");
        let fade = out.find("lsp2 26,\":c;>800,600,#ffffff\",400,300,100,100,0,255\nprint 2\n").expect(&out);
        let clear = out.find("csp -1:csp2 26\n").expect(&out);
        assert!(fade < clear, "{out}");
    }

    #[test]
    fn hanging_indent() {
        // "　Ghost 　": the space before ASCII, five letters and a pad.
        assert_eq!(hang_indent("Ghost　忍、いるか", 6), 5);
        assert_eq!(hang_indent("θ　すべて", 2), 2);
        let mut s = script(vec![Op::Text {
            speaker: None,
            text: "NIN-NIN　長い行".into(),
            voice: None,
            links: vec![],
            ruby: vec![],
            styles: vec![],
            hang: Some(8),
            continues: true,
        }]);
        s.window = Some(window());
        assert!(emit(&s).contains("indent 6\n　NIN-NIN 　長い行@\nindent 0\n"), "{}", emit(&s));
    }

    #[test]
    fn style_colours() {
        let st = |start, len, colour| TextStyle { start, len, colour, ..Default::default() };
        // Each line returns to the text colour; links win over styles.
        assert_eq!(
            with_markup("あいう\nえお", &[], &[], &[st(1, 4, Some(0xffff00))]),
            "あ#FFFF00いう#FFFFFF\n#FFFF00え#FFFFFFお"
        );
        assert_eq!(
            with_markup("あいうえ", &[link(2, 1, None)], &[], &[st(0, 4, Some(0x00ff00)), st(3, 1, Some(0x0000ff))]),
            "#00FF00あい#FEE092う#0000FFえ#FFFFFF"
        );
        assert_eq!(with_markup("あい", &[], &[], &[st(0, 2, None)]), "あい");
        let sized = |start, len, p| TextStyle { start, len, size_percent: Some(p), ..Default::default() };
        assert_eq!(
            with_markup("あい\nう", &[], &[], &[sized(0, 4, 150), TextStyle { colour: Some(0xffff00), ..sized(3, 1, 75) }]),
            "!z150あい!z100\n#FFFF00!z075う!z100#FFFFFF"
        );
        assert_eq!(align_single_bytes("Ghost!z100の"), "Ghost !z100の");
        assert_eq!(align_single_bytes("ab!i1cd!i0の!c"), "ab!i1cd!i0の!c");
    }

    #[test]
    fn ruby_markup() {
        let r = |start, len, text: &str| Ruby { start, len, text: text.into() };
        // After escaping: the text's own parentheses stay full-width.
        assert_eq!(with_markup("(戦ぐ)", &[], &[r(1, 1, "そよ")], &[]), "（(戦/そよ)ぐ）");
        // Inside a link, but not across its edge or a line.
        assert_eq!(
            with_markup("あ獄い\nう", &[link(0, 3, None)], &[r(1, 1, "ひとや"), r(2, 3, "x")], &[]),
            "#FEE092あ(獄/ひとや)い#FFFFFF\nう"
        );
        assert_eq!(with_markup("あいう", &[link(1, 2, None)], &[r(0, 2, "x")], &[]), "あ#FEE092いう#FFFFFF");
        // ONScripter reads the markup one byte at a time.
        assert_eq!(align_single_bytes("a(獄/ひとや)"), "a (獄/ひとや)");
    }

    #[test]
    fn rubyon_lifts_the_text() {
        let mut s = script(vec![Op::Text {
            speaker: None,
            text: "獄".into(),
            voice: None,
            links: vec![],
            ruby: vec![Ruby { start: 0, len: 1, text: "ひとや".into() }],
            styles: vec![],
            hang: None,
            continues: false,
        }]);
        s.window = Some(window());
        let out = emit(&s);
        assert!(out.contains("rubyon\n"));
        assert!(out.contains("setwindow3 105,457,25,3,24,24,1,13,"), "{out}");
        assert!(out.contains("(獄/ひとや)\\\n"), "{out}");
    }
}
