//! CONscripter output stage: turns an IR [`Script`] into `0.txt`.
//!
//! Targets CONscripter (ONScripter-Yuri fork) with Japanese text; the
//! NScripter commands used also run on ogapee's ONScripter. Things marked
//! "unverified" below follow the NScripter documentation but haven't been
//! checked in a running engine yet.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use conv_core::ir::{
    ArithOp, ConfirmDialog, Op, ScreenEffect, Switch, Operand, Script, Ruby, SlideLayer, TextStyle, SpritePlacement, TextLink, TextWindow, TitleAction, TitleScreen,
    SourceLoc, TransitionStyle, Var, ZoomStyle, RowControl, SettingValue,
};

/// A line writer onto `out` (a closure, so `out` is free again after its last use): each call appends the line and a newline.
macro_rules! lines {
    ($out:expr) => {
        |line: String| {
            $out.push_str(&line);
            $out.push('\n');
        }
    };
}

mod boot;
mod button_screen;
mod chapters;
mod choice_plates;
mod command_menu;
mod ease;
mod extras;
mod extras_screens;
mod hold_movie;
mod roll;
pub mod lint;
mod menu;
mod motion;
mod movies;
mod music;
mod particles;
mod notice;
pub mod parity;
pub mod nsa;
pub mod strtab;
mod picture_screen;
mod picture_extras;
mod picture_gallery;
mod gallery_screen;
mod picture_replay;
mod picture_sound;
mod sound_screen;
mod picture_settings;
mod picture_turntable;
mod puzzle;
mod replay;
mod save_grid;
mod save_grid_screen;
pub mod warp;
mod screens;
pub use screens::screen_files;
mod settings_pages;
mod settings_screen;
mod switches;
mod system_menu;
mod line_log_screen;
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
/// The name text and its plate: `spnew` handles in the `ui` layer (with
/// `windowback` and `humanz`, over the text window, under sprites 0-58),
/// taken in the define section ([`NAME_SPRITES_DEFINE`]), the text first so
/// it is over the plate. The UI slots (0-58) are never saved in a
/// meaningful state, so they may move; the game's (62 up) may not.
const NAME_TEXT_SPRITE: &str = "%conv_name_text";
const NAME_PLATE_SPRITE: &str = "%conv_name_plate";
/// The define section's named variables (`defvar`, `defstr`): the
/// click-wait icon's place ([`WAIT_ICON_AT`]), a runtime choice's options
/// ([`runtime_option`]) and backlog line ([`RUNTIME_LOG`]).
fn names_define() -> String {
    let mut lines = vec!["defvar conv_wait_icon_x".to_string(), "defvar conv_wait_icon_y".to_string()];
    for n in 0..RUNTIME_OPTIONS {
        lines.push(format!("defstr conv_choice_text{n}"));
        lines.push(format!("defvar conv_choice_x{n}"));
    }
    lines.push("defstr conv_log_line".to_string());
    lines.join("\n")
}
const NAME_SPRITES_DEFINE: &str = "defvar conv_name_text\ndefvar conv_name_plate\nspnew %conv_name_text,ui\nspnew %conv_name_plate,ui";
/// `humanz`: with `windowback`, sprites up to it (and `ui` handles) are
/// drawn over the text window and the others under it.
const HUMANZ: u32 = 58;
/// Clickable link words (and choice plates): `spnew` handles in the `ui`
/// layer, `%conv_link0` on top ([`LINKS_DEFINE`]; were sprites 51-58);
/// twelve: a plate and a text for each of 装甲悪鬼村正's six options.
const LINK_SPRITE_COUNT: usize = 12;
fn link_sprite(n: usize) -> String {
    format!("%conv_link{n}")
}
/// Their declarations, in the define section after the name plate's.
fn links_define() -> String {
    let names = (0..LINK_SPRITE_COUNT).map(|n| format!("defvar conv_link{n}"));
    names.chain((0..LINK_SPRITE_COUNT).map(|n| format!("spnew {},ui", link_sprite(n)))).collect::<Vec<_>>().join("\n")
}
/// The animation ([`Op::Animation`]), over the static; the static
/// ([`Op::Noise`]), over the overlay; the overlay ([`Op::Overlay`]) over
/// the character sprites: `spnew` handles in the `game` layer (were sprites
/// 59-61), taken in the define section in that order ([`overlays_define`]).
const ANIMATION_SPRITE: &str = "%conv_animation";
const NOISE_SPRITE: &str = "%conv_noise";
const OVERLAY_SPRITE: &str = "%conv_overlay";
/// Their declarations, with their state below, for the features the
/// script uses (an [`Op::SpriteInFront`] goes over the overlay's handle).
fn overlays_define(animation: bool, particles: bool, noise: bool, overlay: bool) -> String {
    let mut lines = Vec::new();
    if animation {
        lines.push("defvar conv_animation\ndefvar conv_animation_alpha\ndefstr conv_animation_image\nspnew %conv_animation".to_string());
    }
    if particles {
        lines.push(particles::define());
    }
    if noise {
        lines.push("defvar conv_noise\ndefvar conv_noise_alpha\ndefstr conv_noise_image\nspnew %conv_noise".to_string());
    }
    if overlay {
        lines.push("defvar conv_overlay\ndefvar conv_overlay_slot\ndefstr conv_overlay_image\nspnew %conv_overlay".to_string());
    }
    if animation || noise || overlay {
        lines.push("defvar conv_overlay_depth".to_string());
    }
    lines.join("\n")
}
/// Sprite layer n is NScripter sprite 62 + n.
const SPRITE_BASE: u32 = 62;
/// The `loadgosub` routine: goes on to the game's own routine.
const LOAD_CLEAR: &str = "conv_load_clear";
/// The lsp2 slot of a flash ([`Op::Flash`]), under the menus' and the top
/// bars' (0-23).
const FLASH_SPRITE2: u32 = 24;
/// The lsp2 slot of a fade to a colour ([`Op::ScreenFade`]), under the
/// flash's.
const FADE_SPRITE2: u32 = 26;
/// The click-wait icon's place ([`Script::wait_icon`]), set with each
/// window; the bar's wait routine shows it.
const WAIT_ICON_AT: [&str; 2] = ["%conv_wait_icon_x", "%conv_wait_icon_y"];
/// The highest sprite number (ONScripter has 1000); also the overlay
/// under the character sprites.
const MAX_SPRITE: u32 = 999;
/// The overlay's sprite (0 = none) and image, to show it again after
/// `csp -1`. Link scenes save them at OVERLAY_SAVED + depth.
const OVERLAY_SLOT: &str = "%conv_overlay_slot";
const OVERLAY_IMAGE: &str = "$conv_overlay_image";
const OVERLAY_DEPTH: &str = "%conv_overlay_depth";
const OVERLAY_SAVED: u32 = 182;
/// The static's opacity (0 = none, else 1-255) and image, to show it again
/// after `csp -1`. Link scenes save them at NOISE_SAVED + depth (the depth
/// is the overlay's counter).
const NOISE_ALPHA: &str = "%conv_noise_alpha";
const NOISE_IMAGE: &str = "$conv_noise_image";
const NOISE_SAVED: u32 = 130;
/// The animation's opacity (0 = none, else 1-255) and image, to show it
/// again after `csp -1`. Link scenes save them at ANIMATION_SAVED + depth
/// (free locals between the game's own and the presets').
const ANIMATION_ALPHA: &str = "%conv_animation_alpha";
const ANIMATION_IMAGE: &str = "$conv_animation_image";
const ANIMATION_SAVED: u32 = 21;
/// Scratch variable receiving the clicked button (also used by nstrace).
const BUTTON_VAR: &str = "%9";
// Every range below is also in `conv_core::alloc`, which the build checks
// the game's variables against (the `vars` tests keep the two in step).
// Ranges claimed by modules: %140-%146, the command menu and the sliding
// bar ([`command_menu`], [`menu::button_bar`]); %147, %149 and $110-$113,
// the save grid ([`save_grid`]); %190-%289 and $200-$229, the tile puzzle's
// state ([`puzzle`]; locals when the global border is 800); %170-%179 and
// $107-$108, the backlog ([`line_log`]);
// the click-wait icon's place is named ([`WAIT_ICON_AT`]).
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
/// Colours of clickable words: not yet visited, visited, each also under
/// the mouse pointer.
/// src: Saihate exe 0x492bb0 (COLORREFs by button state, the second when
/// the target's flag is set: 0x93e1ff/0xffb9dd, under the pointer
/// 0x5ecaff/0xff96cc; pressed 0x43b4ff/0xff7cc4, which ours doesn't show).
const LINK_COLOR: &str = "#FFE193";
const LINK_HOVER_COLOR: &str = "#FFCA5E";
const LINK_VISITED_COLOR: &str = "#DDB9FF";
const LINK_VISITED_HOVER_COLOR: &str = "#CC96FF";
/// Underlines of link words (and their shadows), two sprites per word. The
/// menus' slots: they are drawn again at every wait of the page, so a menu
/// opened from the bar can't leave them wrong.
const LINK_UNDERLINE_SPRITES: std::ops::Range<u32> = MENU_SPRITES.start..MENU_SPRITES.start + 2 * 8;
/// Global flags: link target n (in order of first use) was visited,
/// `?conv_link_visited[n]`, declared when the script has links.
const LINK_VISITED: &str = "conv_link_visited";
/// How many link targets there can be.
const LINK_TARGETS: usize = 200;
/// Global flags the player unlocks, kept by name (`defarr ...,global`).
pub(crate) struct Flags {
    name: &'static str,
    size: u32,
}
impl Flags {
    /// Flag `n`.
    pub(crate) fn at(&self, n: u32) -> String {
        assert!(n < self.size, "{} has {} flags, not {n}", self.name, self.size);
        format!("?{}[{n}]", self.name)
    }
    /// The declaration.
    fn define(&self) -> String {
        format!("defarr {},{},global", self.name, self.size)
    }
}
/// Scene n of the scene replay is unlocked.
const SCENE_UNLOCKED: Flags = Flags { name: "conv_scene_unlocked", size: 600 };
/// Progress flag n ([`Op::Progress`]) is set.
const PROGRESS: Flags = Flags { name: "conv_progress", size: 32 };
/// Option n of an [`Op::RuntimeChoice`] whose text is known only at run
/// time: its text and its x on a plate ([`names_define`] declares them).
fn runtime_option(n: usize) -> (String, String) {
    (format!("$conv_choice_text{n}"), format!("%conv_choice_x{n}"))
}
/// How many options of a choice can be runtime ones (NSS choices have six
/// at most, `SetChoice06`).
const RUNTIME_OPTIONS: usize = 6;
/// An [`Op::RuntimeBacklogLine`]'s text.
const RUNTIME_LOG: &str = "$conv_log_line";
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
/// CG n ([`Op::CgSeen`], up to 9 × 64) was seen.
const CG_SEEN: Flags = Flags { name: "conv_cg_seen", size: 576 };
const TEXT_COLOR: &str = "#FFFFFF";

/// Renders the script as NScripter source text (use [`encode`] for the
/// bytes CONscripter reads).
pub fn emit(script: &Script) -> String {
    emit_mapped(script, strtab::Strings::default()).script
}

/// A script emitted by [`emit_mapped`].
pub struct Emitted {
    pub script: String,
    /// Where its lines came from ([`Script::source_map`]): a line number,
    /// from 1; the source instruction of it and the lines after it up to
    /// the next entry, `None` = none.
    pub lines: Vec<(usize, Option<SourceLoc>)>,
    /// Its text table, when `strings` put the text in one.
    pub rows: Vec<strtab::Row>,
}

/// [`emit`], with the text where `strings` puts it, and where the lines
/// came from.
pub fn emit_mapped(script: &Script, strings: strtab::Strings) -> Emitted {
    let mut strings = strings;
    let effects = fade_effects(script);
    let has_ruby = script.ops.iter().any(|op| matches!(op, Op::Text { ruby, .. } if !ruby.is_empty()));
    let has_overlay = script.ops.iter().any(|op| matches!(op, Op::Overlay { .. }));
    let has_noise = script.ops.iter().any(|op| matches!(op, Op::Noise(_)));
    let has_chapters = script.button_bar.as_ref().and_then(|b| b.top.as_ref()).is_some_and(|t| t.chapter_history.is_some());
    let has_animation = script.ops.iter().any(|op| matches!(op, Op::Animation(_)));
    let has_particles = script.ops.iter().any(|op| matches!(op, Op::Particles(_)));
    let particle_layers: std::collections::BTreeSet<u32> = script.ops.iter().filter_map(|op| if let Op::ParticleLayer { layer, .. } = op { Some(*layer) } else { None }).collect();
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
    if has_particles {
        restore_overlay.push_str(&particles::restore());
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
    // A gallery CG seen by its pictures: the engine's log of every picture loaded (`fchk`).
    if script.picture_extras.as_ref().and_then(|e| e.gallery.as_ref()).is_some_and(|g| g.categories.iter().flat_map(|c| &c.slots).flat_map(|s| &s.cgs).any(|cg| cg.seen.reads_pictures())) {
        writeln!(out, "filelog").unwrap();
    }
    writeln!(out, "{}", notice::DEFINE).unwrap();
    boot::define(&mut out, script);
    if let Some(g) = script.picture_extras.as_ref().and_then(|e| e.gallery.as_ref()) {
        picture_gallery::define(&mut out, g);
    }
    writeln!(out, "{NAME_SPRITES_DEFINE}").unwrap();
    writeln!(out, "{}", names_define()).unwrap();
    writeln!(out, "{}", links_define()).unwrap();
    if let Some(s) = &script.picture_settings {
        picture_settings::define(&mut out, s, &script.language);
    }
    let in_front = script.ops.iter().any(|op| matches!(op, Op::SpriteInFront { front: true, .. }));
    if script.system_menu.is_some() {
        writeln!(out, "{}", system_menu::DEFINE).unwrap();
    }
    if has_overlay || has_noise || has_animation || has_particles || in_front {
        writeln!(out, "{}", overlays_define(has_animation, has_particles, has_noise, has_overlay || in_front)).unwrap();
    }
    for &layer in &particle_layers {
        writeln!(out, "{}", particles::define_layer(layer)).unwrap();
    }
    if script.ops.iter().any(|op| matches!(op, Op::Text { links, .. } if links.iter().any(|l| l.target.is_some()))) {
        writeln!(out, "defarr {LINK_VISITED},{LINK_TARGETS},global").unwrap();
    }
    for flags in [&CG_SEEN, &movies::SEEN, &SCENE_UNLOCKED, &PROGRESS, &replay::REPLAYED] {
        writeln!(out, "{}", flags.define()).unwrap();
    }
    if lua::uses_processes(script) {
        for c in lua::COMMANDS {
            writeln!(out, "luasub {c}").unwrap();
        }
    }
    if script.window.is_some() {
        writeln!(out, "windowback").unwrap();
        writeln!(out, "humanz {HUMANZ}").unwrap();
    }
    if let Some(k) = script.button_bar.as_ref().and_then(|_| menu::pad_hide_key(script.command_menu.as_ref())) {
        // The gamepad's Y hides the text window as the bar does.
        writeln!(out, "padmap \"y\",\"{k}\"").unwrap();
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
    if script.button_bar.is_some() {
        // Every click wait runs the bar routine instead of waiting itself.
        writeln!(out, "textgosub *{}", menu::WAIT).unwrap();
    }
    // shortcut: script.after_load replaces a bar's routine; chain the two when a game with a bar needs both.
    let after_load = script.after_load.clone().or_else(|| {
        let bar = script.button_bar.as_ref()?;
        if menu::has_back_to_choice(bar, script.command_menu.as_ref()) {
            Some(menu::LOADED.to_string())
        } else {
            bar.top.is_some().then(|| menu::SCENE_LOADED.to_string())
        }
    });
    writeln!(out, "loadgosub *{LOAD_CLEAR}").unwrap();
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
    if !script.language.is_empty() {
        let first = format!("language \"{}\"", script.language);
        if script.picture_settings.as_ref().is_some_and(picture_settings::has_language) {
            picture_settings::define_language(&mut out, LOAD_CLEAR, &first);
        } else {
            writeln!(out, "{first}").unwrap();
        }
    }
    if script.click_skips_effects {
        writeln!(out, "clickskip").unwrap();
    }
    let shake_pool = ops::shake_pool(script);
    for (n, list) in shake_pool.iter().enumerate() {
        writeln!(out, "stralias {},\"{list}\"", ops::shake_alias(n)).unwrap();
    }
    let shake_lists: HashMap<String, usize> = shake_pool.into_iter().enumerate().map(|(n, l)| (l, n)).collect();
    writeln!(out, "game").unwrap();
    writeln!(out, "*start").unwrap();
    // The text speed is a setting when there is a settings menu.
    let has_settings = script.settings.is_some() || script.picture_settings.is_some();
    let speed = if has_settings { menu::TEXT_MS.to_string() } else { script.text_ms.unwrap_or(20).to_string() };
    let speed = speed.as_str();
    let voice_switches = voice_switches(script);
    let duck = uses_switch(script, Switch::BgmFullForVoice);
    if let Some(s) = &script.settings {
        menu::settings_init(&mut out, s);
    }
    if let Some(s) = &script.picture_settings {
        picture_settings::init(&mut out, s, &script.language);
    }
    if let Some(m) = &script.menu_sounds {
        writeln!(out, "mov {MENU_HOVER},\"{}\":selectvoice \"\",{MENU_HOVER},\"\"", m.hover.as_deref().map(path).unwrap_or_default()).unwrap();
        writeln!(out, "btnse \"{}\"", m.confirm.as_deref().map(path).unwrap_or_default()).unwrap();
    }
    // setwindow redraws the screen, so it must come after `game` (ONScripter
    // crashes if it runs in the define section).
    if let Some(w) = &script.window {
        writeln!(out, "{}", strings.lines(strtab::WINDOW, &setwindow(w, speed, has_ruby, has_vertical, true, script))).unwrap();
        // Keep the window up while sprites change and pages are printed
        // (by default every print hides it until the next text, which
        // flickers); the script hides it explicitly with textoff.
        writeln!(out, "erasetextwindow 0").unwrap();
    }
    if system_menu::menu(script).is_some() {
        writeln!(out, "rgosub *{}", system_menu::RIGHT_CLICK).unwrap();
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

    let save_menu = script.save_menu.is_some() || script.save_grid.is_some() || system_menu::loads(script);
    let effect = |ms: u32| if ms == 0 { "1".to_string() } else { effects[&(ms, Effect::Fade)].clone() };
    boot::emit(&mut out, script, border);
    notice::emit(&mut out, script.width, script.height, &script.language);
    if let Some(t) = &script.title_screen {
        title_screen(&mut out, t, &effect(t.fade_ms), &effect(t.start_fade()), save_menu, has_settings, script.confirm.as_ref(), script.extras.is_some(), border, picture_replay, boot::keeps_bgm(script));
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
    let eased = BTreeMap::new();
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
    assert!(link_targets.len() <= LINK_TARGETS, "more than {LINK_TARGETS} link targets");
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
    let mut e = ops::Emitter { script, out, effects, has_ruby, has_overlay, has_noise, has_chapters, has_animation, root_returns, has_replay, has_vertical, border, voice_switches, duck, back_to_choice, speed, link_depth, clear_fade, restore_overlay, link_targets, zoomed_layers, root_resumes, puzzles, button_screens, plate_choices, picture_screens, se, se_not_skipped, noise_fade_in, move_ease, move_nowait, move_arc, eased, needs_click_wait, plate_visible, link_pages, transition, window, page_rows, page_tag, text_goes_on: false, fading, strings, shake_lists };
    // (byte offset in out, source), up to the end of the ops' lines
    let mut starts: Vec<(usize, Option<&SourceLoc>)> = Vec::new();
    let mut map = script.source_map.iter().peekable();
    for (i, op) in script.ops.iter().enumerate() {
        while let Some((_, at)) = map.next_if(|(o, _)| *o <= i) {
            starts.push((e.out.len(), Some(at)));
            e.strings.at = format!("{}:{}", at.file, at.index);
        }
        if let Op::Label(name) = op {
            e.strings.label = name.clone();
        }
        e.op(i, op);
    }
    starts.push((e.out.len(), None));
    let ops::Emitter { mut out, effects, eased, needs_click_wait, strings, .. } = e;
    let effect = |ms: u32| if ms == 0 { "1".to_string() } else { effects[&(ms, Effect::Fade)].clone() };
    writeln!(out, "end").unwrap();
    writeln!(out, "*{LOAD_CLEAR}").unwrap();
    if has_particles {
        writeln!(out, "{}", particles::after_load()).unwrap();
    }
    for &layer in &particle_layers {
        writeln!(out, "{}", particles::layer_after_load(layer)).unwrap();
    }
    match &after_load {
        Some(label) => writeln!(out, "goto *{label}").unwrap(),
        None => writeln!(out, "return").unwrap(),
    }
    if !eased.is_empty() {
        // The eased moves' curves, defined as the game starts (saves keep them).
        let defs: String = eased.iter().map(|(&(e, n), &id)| ease::curve_def(id, e, n) + "\n").collect();
        let at = out.find("\n*start\n").map(|i| i + "\n*start\n".len()).expect("*start");
        out.insert_str(at, &defs);
        starts.iter_mut().filter(|s| s.0 >= at).for_each(|s| s.0 += defs.len());
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
    if let Some(g) = save_grid_screen::engine_grid(script) {
        // shortcut: no link scene levels (scene_stack) kept with a save; add them when a game with both needs it.
        save_grid_screen::labels(&mut out, g);
    } else if screens::engine_menu(script).is_some() {
        screens::save_labels(&mut out);
    } else if let Some(m) = system_menu::menu(script) {
        system_menu::labels(&mut out, m, script.confirm.as_ref());
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
                .filter(|s| picture_settings::has_game_speed(s))
                .map(|_| picture_settings::GAME_SPEED),
            toggles: script.settings.as_ref().is_some_and(|m| menu::has_switch(m, Switch::ClickStopsAuto) || menu::has_switch(m, Switch::ClickStopsSkip))
                || script.picture_settings.as_ref().is_some_and(|s| s.pairs.iter().any(|p| p.option == conv_core::ir::PairOption::ClickStopsAuto)),
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
        picture_settings::emit(&mut out, s);
    } else if let Some(m) = screens::engine_settings(script) {
        screens::settings_labels(&mut out, has_replay);
        menu::settings_apply(&mut out, m);
    } else if let Some(m) = &script.settings {
        menu::settings_menu(&mut out, m, has_replay);
    }
    if let Some(l) = line_log_screen::engine_log(script) {
        line_log_screen::labels(&mut out, l);
    } else if script.backlog.is_some() {
        screens::backlog_labels(&mut out);
    }
    if let Some(d) = &script.confirm {
        menu::confirm(&mut out, d);
    }
    if let (Some(m), Some(t)) = (&script.extras, &script.title_screen) {
        extras::extras(&mut out, m, &effect(t.fade_ms), script.settings.is_some(), !t.openings.is_empty());
    }
    if let (Some(e), Some(_)) = (&script.picture_extras, &script.title_screen) {
        picture_extras::emit(&mut out, e, border, script.width, script.height);
    }
    let mut lines: Vec<(usize, Option<SourceLoc>)> = Vec::new();
    let (mut line, mut from) = (1, 0);
    for (offset, at) in starts {
        line += out.as_bytes()[from..offset].iter().filter(|&&b| b == b'\n').count();
        from = offset;
        // an instruction that wrote nothing gives way to the next one
        if lines.last().is_some_and(|l| l.0 == line) {
            lines.pop();
        }
        lines.push((line, at.cloned()));
    }
    if script.source_map.is_empty() {
        lines.clear();
    }
    Emitted { script: out, lines, rows: strings.rows }
}

/// The title menu: buttons are sprites over the picture, faded in together;
/// Start clears them and falls through to the script that follows. Load
/// opens the save menu's load screen if there is one, else ONScripter's.
#[allow(clippy::too_many_arguments)]
/// `replay_back`: a picture extras' replay may `reset` back to its list
/// ([`picture_replay::BACK_TO_LIST`]).
fn title_screen(out: &mut String, t: &TitleScreen, fade: &str, start_fade: &str, save_menu: bool, settings: bool, confirm: Option<&ConfirmDialog>, extras: bool, border: u32, replay_back: bool, boot_bgm: bool) {
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
    // Back from the extras: the music goes on.
    let back = picture_extras || (extras && !t.openings.is_empty());
    if back {
        writeln!(out, "if {TITLE_SHOWN} == 2 mov {TITLE_SHOWN},1:goto *{TITLE_SPRITES}").unwrap();
    }
    if !t.openings.is_empty() {
        writeln!(out, "mov {TITLE_SHOWN},1").unwrap();
    }
    writeln!(out, "*{TITLE_DRAW}").unwrap();
    if let Some(bgm) = t.bgm.as_ref().filter(|_| t.movies.is_empty()) {
        // Left playing by the boot's logos: it goes on.
        let on = boot::BGM_ON;
        let only = if boot_bgm { format!("if {on} == 0 ") } else { String::new() };
        writeln!(out, "{only}bgm \"{}\"", path(bgm)).unwrap();
        if boot_bgm {
            writeln!(out, "mov {on},0").unwrap();
        }
    }
    title_movies(out, t, true, border);
    if back {
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
    if t.hide_without_saves || t.continue_hides.is_some() {
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
            TitleAction::Continue if t.hide_without_saves || t.continue_hides.is_some() => format!("csp -1:loadgame {}", menu::HAS_DATA),
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
        if let Some(label) = &o.script {
            if let Some(bgm) = t.bgm.as_ref().filter(|_| !o.after_extras) {
                let on = boot::BGM_ON;
                let only = if boot_bgm { format!("if {on} == 0 ") } else { String::new() };
                writeln!(out, "{only}bgm \"{}\"", path(bgm)).unwrap();
                if boot_bgm {
                    writeln!(out, "mov {on},0").unwrap();
                }
            }
            // Its sprites go as the title's (the same picture) come, once
            // their motions have ended (one still running under its last
            // cross-fade); a looping one's clock goes on in the title's.
            writeln!(out, "bg black,0\ngosub *{label}\nspmotionwait -1").unwrap();
            for &(s, layer) in &o.clocks {
                writeln!(out, "spmotionclock {},{}", title_slot(s), SPRITE_BASE + layer).unwrap();
            }
            if o.clocks.is_empty() {
                writeln!(out, "csp -1").unwrap();
            } else {
                // csp -1 would drop the clocks: the opening's own sprites only.
                let tmp = menu::TMP;
                writeln!(out, "for {tmp}={SPRITE_BASE} to {MAX_SPRITE}:csp {tmp}:next").unwrap();
            }
            writeln!(out, "bg \"{}\",0", path(&t.background)).unwrap();
            title_sprites(out, t, &buttons, quick, None, true, border);
            writeln!(out, "print 1\ngoto *{TITLE_MENU}").unwrap();
            continue;
        }
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
    writeln!(out, "bg {},{start_fade}", if t.start_white { "white" } else { "black" }).unwrap();
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
    title_movies(out, t, false, border);
    for (i, p) in t.pictures.iter().enumerate() {
        // Over the movie (the last menu sprite), under the buttons.
        let slot = MENU_SPRITES.end - 2 - i as u32;
        writeln!(out, "lsp {slot},\":a;{}\",{},{}", path(&p.image), p.x, p.y).unwrap();
        if p.add {
            writeln!(out, "spblend {slot},1").unwrap();
        }
        if !p.keys.is_empty() {
            writeln!(out, "spmotion {slot},{},{}", if p.repeat { 1 } else { 2 }, glow_keys(&p.keys)).unwrap();
        }
    }
    if t.hide_without_saves || t.continue_hides.is_some() {
        writeln!(out, "{}", saves_check()).unwrap();
    }
    if quick {
        writeln!(out, "savefileexist {},{}", menu::TMP, menu::QUICK_SLOT).unwrap();
    }
    for (b, slot) in buttons {
        let image = if b.action == TitleAction::Unavailable { format!(":a;{}", path(&b.image)) } else { format!(":a/{cells},0,3;{}", path(&b.image)) };
        if let Some(dy) = t.continue_hides.filter(|_| b.action != TitleAction::Continue) {
            let only = title_button_shown(t, b, border);
            let only = only.strip_prefix("if ").map_or(String::new(), |c| format!("&& {c}"));
            writeln!(out, "if {} > 0 {only}lsp {slot},\"{image}\",{},{}", menu::HAS_DATA, b.x, b.y).unwrap();
            writeln!(out, "if {} == 0 {only}lsp {slot},\"{image}\",{},{}", menu::HAS_DATA, b.x, b.y + dy).unwrap();
            continue;
        }
        if b.action == TitleAction::Continue && !t.hide_without_saves && t.continue_hides.is_none() {
            let only = b.shown_by.map_or(String::new(), |v| format!("if {} != 0 ", var_name(v, border)));
            writeln!(out, "{only}lsp {slot},\"{image}\",{},{}", b.x, b.y).unwrap();
            writeln!(out, "if {} == 0 lsp {slot},\"{image}\",{},{},128", menu::TMP, b.x, b.y).unwrap();
            continue;
        }
        let only = title_button_shown(t, b, border);
        writeln!(out, "{only}lsp {slot},\"{image}\",{},{}", b.x, b.y).unwrap();
    }
    for m in t.motions.iter().filter(|_| moving) {
        let only = match m.sprite {
            conv_core::ir::TitleSprite::Button(i) => title_button_shown(t, &t.buttons[i], border),
            conv_core::ir::TitleSprite::Picture(_) => String::new(),
        };
        let keys: Vec<_> = m.keys.iter().map(|k| conv_core::ir::MotionKey { ease: None, ..*k }).collect();
        let end = if m.from_key > 0 { conv_core::ir::MotionEnd::LoopFrom(m.from_key) } else { conv_core::ir::MotionEnd::Loop };
        let mut line = String::new();
        motion::emit_slot(&mut line, title_slot(m.sprite), &keys, end, None, 0, &mut std::collections::BTreeMap::new());
        write!(out, "{only}{line}").unwrap();
    }
}

/// The sprite a [`conv_core::ir::TitleSprite`] is drawn on ([`title_sprites`]).
fn title_slot(s: conv_core::ir::TitleSprite) -> u32 {
    match s {
        conv_core::ir::TitleSprite::Button(i) => MENU_SPRITES.start + i as u32,
        conv_core::ir::TitleSprite::Picture(i) => MENU_SPRITES.end - 2 - i as u32,
    }
}

/// [`TitleScreen::movies`]: the first whose conditions hold, looping in
/// the last menu sprite (`bgm`: its music instead).
fn title_movies(out: &mut String, t: &TitleScreen, bgm: bool, border: u32) {
    if t.movies.is_empty() {
        return;
    }
    debug_assert!(t.glow.is_none(), "a title movie takes the glow's sprite");
    let slot = MENU_SPRITES.end - 1;
    writeln!(out, "mov {},0", menu::TMP).unwrap();
    for m in &t.movies {
        let mut cond = vec![format!("{} == 0", menu::TMP)];
        cond.extend(m.when.iter().map(|(v, value)| format!("{} == {value}", var_name(*v, border))));
        let then = if bgm {
            m.bgm.as_ref().or(t.bgm.as_ref()).map_or("bgmstop".to_string(), |b| format!("bgm \"{}\"", path(b)))
        } else {
            format!("lsp {slot},\":c/1,0,0;{}\",0,0", path(&m.movie))
        };
        writeln!(out, "if {} {then}:mov {},1", cond.join(" && "), menu::TMP).unwrap();
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
        TitleAction::Continue if t.continue_hides.is_some() => conds.push(format!("{} > 0", menu::HAS_DATA)),
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
    if let Some(layer) = conv_core::ir::particle_layer(&s.image) {
        return format!("lsp {slot},\"*{layer}\",{},{}", s.x, s.y);
    }
    if s.image == conv_core::ir::SCREEN_UNDER {
        // (Taken at (0, 0): moved to its place.)
        return match (s.x, s.y) {
            (0, 0) => format!("spgrab {slot},-3"),
            (x, y) => format!("spgrab {slot},-3:msp {slot},{x},{y},0"),
        };
    }
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

/// A half-width character: one byte in Shift-JIS, or a Latin letter it
/// lacks (CONscripter's `charCells`).
fn is_single_byte(c: char) -> bool {
    c.is_ascii() || ('\u{FF61}'..='\u{FF9F}').contains(&c) || (('\u{C0}'..='\u{24F}').contains(&c) && !matches!(c, '×' | '÷'))
}

fn starts_ascii(line: &str) -> bool {
    line.starts_with(|c: char| c.is_ascii())
}

/// The settings switch voices per character (%970 + character, `menu::VOICE_OFF_BASE`).
pub(crate) fn voice_switches(script: &Script) -> bool {
    script.settings.as_ref().is_some_and(|s| !s.voices.is_empty() || s.pictures.as_ref().is_some_and(|p| p.pages.iter().flat_map(|p| &p.rows).any(|r| matches!(r, RowControl::Choice { value: SettingValue::Voices(_), .. } | RowControl::Toggle { value: SettingValue::Voices(_), .. }))))
        || script.picture_settings.as_ref().is_some_and(|s| s.voice_window.is_some())
}

/// The settings screen has switch `s` (in its list or on a picture page).
fn uses_switch(script: &Script, s: Switch) -> bool {
    script.settings.as_ref().is_some_and(|m| menu::has_switch(m, s))
}

/// Command-line options CONscripter reads from `ons_args` in the game
/// directory: the script is UTF-8 (without `--enc:` it reads it as GBK).
/// The web text overlay stays on (the player can turn it off).
pub const ENGINE_ARGS: &str = "--enc:utf8\n";

/// `ons_args`: [`ENGINE_ARGS`], and with a button bar its sprites for
/// `--trace-ignore` (the lists add up), so `--auto-play` doesn't open the
/// menus from the bar and get lost in them.
pub fn engine_args(script: &Script) -> String {
    match script.button_bar {
        Some(_) => format!("{ENGINE_ARGS}--trace-ignore\n{}-{}\n", BAR_SPRITES.start, BAR_SPRITES.end - 1),
        None => ENGINE_ARGS.to_string(),
    }
}

/// The settings that start at the shared default level because the game
/// gives none, as "name level" (for the build report).
pub fn settings_fallbacks(script: &Script) -> Vec<String> {
    script.settings.as_ref().map(menu::fallback_defaults).unwrap_or_default()
}

/// Encodes NScripter source as UTF-8 ([`ENGINE_ARGS`]), with the count of
/// characters past U+FFFF, which CONscripter doesn't draw.
pub fn encode(source: &str) -> (Vec<u8>, usize) {
    (source.as_bytes().to_vec(), source.chars().filter(|&c| c > '\u{FFFF}').count())
}

/// Decodes a `0.txt` (the reverse of [`encode`]).
pub fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
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
        (None, style) if warp_kind(style).is_some() => {
            let k = warp_kind(style).unwrap();
            one(Effect::Dll(format!("cons_warp.dll/{}\\{},{}", warp::DIR, k.file_name(), k.cells())))
        }
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
        // Mosaic out (16: the old picture coarsens), then in (17: the new
        // one sharpens), half the time each.
        (None, TransitionStyle::Mosaic) => {
            let half = (fade_ms / 2).max(1);
            vec![(half, Effect::Builtin(16)), ((fade_ms - half).max(1), Effect::Builtin(17))]
        }
        // No style reaches this today: the moving ones (flip, melt, cube,
        // wave, zoom, slides but orthogonal pushes) have warp maps above,
        // the rest their own arms. A style added to the IR without an arm
        // here plays as a cross-fade.
        (None, _) => one(Effect::Fade),
    }
}

/// MSVC rand()'s seed for the melting edge.
// ours: fixed seed, the exe seeds rand from the clock (srand(_time64) 0x407150)
const MELT_SEED: u32 = 1;

/// The warp map that draws a transition style (with no mask picture):
/// the plugins' moving transitions. Orthogonal pushes stay ONScripter's
/// scrolls, blurs a plugin.
fn warp_kind(style: TransitionStyle) -> Option<warp::Kind> {
    Some(match style {
        TransitionStyle::Flip { horizontal, grow } => warp::Kind::Flip { axis: u8::from(!horizontal), layer: u8::from(!grow) },
        TransitionStyle::Melt { speed } => warp::Kind::Melt { speed, seed: MELT_SEED },
        TransitionStyle::Cube { dx, dy } => warp::Kind::Cube { dx, dy },
        TransitionStyle::Wave { direction, amplitude, waves } => warp::Kind::Wave { direction, amplitude, waves },
        TransitionStyle::Slide { moving: SlideLayer::Both, dx, dy } if dx == 0 || dy == 0 => return None,
        TransitionStyle::Slide { moving, dx, dy } => {
            let layer = match moving {
                SlideLayer::New => 0,
                SlideLayer::Old => 1,
                SlideLayer::Both => 2,
            };
            warp::Kind::Slide { layer, dx, dy }
        }
        TransitionStyle::Zoom { style, max_percent, curve } => {
            let mode = match style {
                ZoomStyle::NewIn => 0,
                ZoomStyle::OldOut => 1,
                ZoomStyle::Through => 2,
                ZoomStyle::Cross => 3,
            };
            warp::Kind::Zoom { mode, scale: max_percent, curve: u32::from(curve) }
        }
        _ => return None,
    })
}

/// The warp maps the script's transitions use, with their paths in the
/// output (`warp/<name>.png`), for the CLI to write.
pub fn warp_maps(script: &Script) -> Vec<(String, image::RgbImage)> {
    let kinds: std::collections::BTreeSet<warp::Kind> = script
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::BeginTransition { fade_ms, mask: None, style, .. } if *fade_ms > 0 => warp_kind(*style),
            _ => None,
        })
        .collect();
    kinds.into_iter().map(|k| (format!("{}/{}", warp::DIR, k.file_name()), warp::map(k, script.width, script.height))).collect()
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
        .chain(script.title_screen.iter().flat_map(|t| [(t.fade_ms, Effect::Fade), (t.start_fade(), Effect::Fade)]))
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
/// `@`: a language's variant of a label (CONscripter's "Label variants").
fn label(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '@' { c } else { '_' }).collect()
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

/// A single-byte line's click waits: `@`, and where text goes on after it
/// on the line, `` @`` `` (the string closes and a new one opens). A
/// textgosub comes back after the wait with a ` string's rest only when
/// no space follows the `@` (engine/src/cons/ONScripter_text.cpp
/// `textgosubReturn`, ScriptHandler.cpp `readToken`: it skips the spaces
/// before it checks `backquote_resume`), else it reads `@ what` on as the
/// command `what`; after `` @` `` it goes on at the next token, the new
/// string.
pub(crate) fn single_byte_waits(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != conv_core::ir::CLICK_WAIT_MARK {
            out.push(c);
            continue;
        }
        out.push('@');
        if chars.peek().is_some_and(|&n| n != '\n' && n != conv_core::ir::CLICK_WAIT_MARK) {
            out.push_str("``");
        }
    }
    out
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
mod tests;
