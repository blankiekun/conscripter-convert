//! CONscripter output stage: turns an IR [`Script`] into `0.txt`.
//!
//! Targets CONscripter (ONScripter-Yuri fork) with Japanese text; the
//! NScripter commands used also run on ogapee's ONScripter. Things marked
//! "unverified" below follow the NScripter documentation but haven't been
//! checked in a running engine yet (see PLAN.md, Phase 0).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use conv_core::ir::{
    ArithOp, ConfirmDialog, Op, Operand, Script, Ruby, SlideLayer, TextStyle, SpritePlacement, TextLink, TextWindow, TitleAction, TitleScreen,
    TransitionStyle, Var, ZoomStyle,
};

mod menu;
pub mod nsa;
pub mod webp;

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
/// else (ONScripter draws lower numbers on top); 6-33 to the menus (title,
/// save/load, settings, backlog); 34-49 to the bars (buttons, hotspots,
/// texts).
const DIALOG_SPRITES: std::ops::Range<u32> = 0..6;
const MENU_SPRITES: std::ops::Range<u32> = 6..34;
const BAR_SPRITES: std::ops::Range<u32> = 34..50;
/// Sprite slots for the name text and its plate: the text over the plate,
/// both over the character sprites.
const NAME_TEXT_SPRITE: u32 = 50;
const NAME_PLATE_SPRITE: u32 = 51;
/// Sprite slots for clickable link words. With `windowback` and `humanz
/// 59`, slots up to 59 are drawn over the text window and the character
/// sprites under it.
const LINK_SPRITES: std::ops::RangeInclusive<u32> = 52..=59;
/// The static ([`Op::Noise`]), over the overlay.
const NOISE_SPRITE: u32 = 60;
/// The overlay ([`Op::Overlay`]) over the character sprites.
const OVERLAY_SPRITE: u32 = 61;
/// Sprite layer n is NScripter sprite 62 + n.
const SPRITE_BASE: u32 = 62;
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
/// Scratch variable receiving the clicked button (also used by nstrace).
const BUTTON_VAR: &str = "%9";
/// NScripter variables: locals from %10, globals from %200. `globalon`
/// makes %200 and up persist across games.
const LOCAL_VAR_BASE: u32 = 10;
const GLOBAL_VAR_BASE: u32 = 200;
/// Labels of the title menu loop and of its Start button's target.
/// Click wait with everything hidden ([`Op::ClickWait`]).
const CLICK_WAIT: &str = "conv_click_wait";
const TITLE_DRAW: &str = "conv_title_draw";
const TITLE_MENU: &str = "conv_title";
const TITLE_START: &str = "conv_title_start";
/// Colour for words the original game made clickable, and normal text.
const LINK_COLOR: &str = "#F0C040";
/// Link words under the mouse pointer.
const LINK_HOVER_COLOR: &str = "#FFFFA0";
const TEXT_COLOR: &str = "#FFFFFF";

/// Renders the script as NScripter source text (UTF-8; use [`encode`] to
/// produce the Shift-JIS bytes ONScripter reads).
pub fn emit(script: &Script) -> String {
    let effects = fade_effects(script);
    let has_ruby = script.ops.iter().any(|op| matches!(op, Op::Text { ruby, .. } if !ruby.is_empty()));
    let has_overlay = script.ops.iter().any(|op| matches!(op, Op::Overlay { .. }));
    let has_noise = script.ops.iter().any(|op| matches!(op, Op::Noise(_)));
    // Shows the overlay and the static again after `csp -1`.
    let mut restore_overlay = String::new();
    if has_overlay {
        writeln!(restore_overlay, "if {OVERLAY_SLOT} != 0 lsp {OVERLAY_SLOT},{OVERLAY_IMAGE},0,0").unwrap();
    }
    if has_noise {
        writeln!(restore_overlay, "if {NOISE_ALPHA} != 0 lsp {NOISE_SPRITE},{NOISE_IMAGE},0,0,{NOISE_ALPHA}").unwrap();
    }
    let mut out = String::new();

    let mode = match (script.width, script.height) {
        (800, 600) => ";mode800",
        (640, 480) => ";mode640",
        _ => ";mode800", // unverified: other sizes need a `;$S` header
    };
    writeln!(out, "{mode}").unwrap();
    writeln!(out, "*define").unwrap();
    // Read arc.nsa (without this ONScripter only looks for .sar archives;
    // loose files still work either way).
    writeln!(out, "nsa").unwrap();
    writeln!(out, "globalon").unwrap();
    if script.window.is_some() {
        writeln!(out, "windowback").unwrap();
        writeln!(out, "humanz {}", LINK_SPRITES.end()).unwrap();
    }
    if script.backlog.is_some() {
        // Wheel and arrow keys reach btnwait (backlog scrolling); keep more
        // pages of history than the default 16.
        writeln!(out, "usewheel").unwrap();
        writeln!(out, "maxkaisoupage 100").unwrap();
        // Text lines carry the speaker and voice as a tag for the backlog
        // (gettaglog); tags are only kept with a pretextgosub.
        writeln!(out, "pretextgosub *{}", menu::PRETEXT).unwrap();
    }
    if script.button_bar.is_some() {
        // Every click wait runs the bar routine instead of waiting itself.
        writeln!(out, "textgosub *{}", menu::WAIT).unwrap();
    }
    // CONscripter converts the caption to UTF-8 for the window title.
    if has_ruby {
        // Draw ruby (readings) in the line gap above the text. Text then
        // starts a line gap lower, which setwindow and the backlog undo.
        writeln!(out, "rubyon").unwrap();
    }
    let caption: String = script.title.chars().filter(|&c| c != '"').collect();
    writeln!(out, "caption \"{}\"", caption.trim()).unwrap();
    for ((ms, kind), n) in &effects {
        match kind {
            Effect::Fade => writeln!(out, "effect {n},{FADE},{ms}").unwrap(),
            Effect::Mask(m) => writeln!(out, "effect {n},{MASK_FADE},{ms},\"{}\"", path(m)).unwrap(),
            Effect::Builtin(k) => writeln!(out, "effect {n},{k},{ms}").unwrap(),
            Effect::Dll(d) => writeln!(out, "effect {n},{DLL},{ms},\"{d}\"").unwrap(),
        }
    }
    // Read-text tracking (kidoku.dat, written on save and quit), for
    // skipping read text only.
    if script.settings.as_ref().is_some_and(|s| s.skip_unread_label.is_some()) {
        writeln!(out, "kidokuskip").unwrap();
    }
    writeln!(out, "game").unwrap();
    writeln!(out, "*start").unwrap();
    // The text speed is a setting when there is a settings menu.
    let speed = if script.settings.is_some() { menu::TEXT_MS } else { "20" };
    let voice_switches = script.settings.as_ref().is_some_and(|s| !s.voices.is_empty());
    if script.settings.is_some() {
        menu::settings_init(&mut out);
    }
    // setwindow redraws the screen, so it must come after `game` (ONScripter
    // crashes if it runs in the define section).
    if let Some(w) = &script.window {
        writeln!(out, "{}", setwindow(w, speed, has_ruby)).unwrap();
        // Keep the window up while sprites change and pages are printed
        // (by default every print hides it until the next text, which
        // flickers); the script hides it explicitly with textoff.
        writeln!(out, "erasetextwindow 0").unwrap();
    }

    let effect = |ms: u32| if ms == 0 { 1 } else { effects[&(ms, Effect::Fade)] };
    if let Some(t) = &script.title_screen {
        title_screen(&mut out, t, effect(t.fade_ms), script.save_menu.is_some(), script.settings.is_some(), script.confirm.as_ref());
    }
    let mut se = SeChannels::default();
    let mut needs_bit_helpers = false;
    let mut needs_click_wait = false;
    let mut plate_visible = false;
    let mut link_pages = 0;
    // Effect numbers of an open transition, printed in turn: display
    // commands only update the buffer until then.
    let mut transition: Option<Vec<u32>> = None;
    // The active text window (file order; presets are called right before
    // the text that uses them) and the lines already on its page.
    let mut window = script.window.clone();
    let mut page_rows = 0;
    // The backlog tag of the page being written: its first speaker and voice.
    let mut page_tag: Option<String> = None;
    for op in &script.ops {
        if let Op::Text { .. } = op {
            for e in transition.take().unwrap_or_default() {
                writeln!(out, "print {e}").unwrap();
            }
        }
        match op {
            Op::Label(name) => writeln!(out, "*{}", label(name)).unwrap(),
            Op::Jump(name) => writeln!(out, "goto *{}", label(name)).unwrap(),
            Op::Gosub(name) => writeln!(out, "gosub *{}", label(name)).unwrap(),
            Op::Return => writeln!(out, "return").unwrap(),
            Op::Arith { var, op, value } => {
                let value = operand(*value);
                let target = var_name(*var);
                let cmd = match op {
                    ArithOp::Set => "mov",
                    ArithOp::Add => "add",
                    ArithOp::Sub => "sub",
                    ArithOp::Mul => "mul",
                    ArithOp::Div => "div",
                    ArithOp::BitAnd | ArithOp::BitOr => {
                        // NScripter has no bitwise operators; call a helper.
                        let routine = if *op == ArithOp::BitAnd { BITAND } else { BITOR };
                        needs_bit_helpers = true;
                        writeln!(out, "mov %1,{target}:mov %2,{value}:gosub *{routine}:mov {target},%3").unwrap();
                        continue;
                    }
                };
                writeln!(out, "{cmd} {target},{value}").unwrap();
            }
            Op::IfEqual { var, value, label: target } => {
                writeln!(out, "if {} == {value} goto *{}", var_name(*var), label(target)).unwrap()
            }
            Op::Background { image, fade_ms } => {
                // NScripter's bg keeps lsp sprites; the game removes characters
                // when the background or CG changes.
                writeln!(out, "csp -1").unwrap();
                out.push_str(&restore_overlay);
                // Effect 0 = update the buffer only (shown by the transition).
                let e = if transition.is_some() { 0 } else { effect(*fade_ms) };
                writeln!(out, "bg \"{}\",{e}", path(image)).unwrap();
                plate_visible = false;
            }
            Op::Sprites { changes, fade_ms } => {
                for c in changes {
                    let slot = SPRITE_BASE + c.layer;
                    match &c.sprite {
                        Some(sp) => {
                            if !c.keep_filters {
                                // Only reloads the old image when it has filters.
                                writeln!(out, "spfilter {slot},0").unwrap();
                            }
                            writeln!(out, "{}", lsp(slot, sp)).unwrap();
                        }
                        None => writeln!(out, "csp {slot}").unwrap(),
                    }
                }
                if transition.is_none() {
                    writeln!(out, "print {}", effect(*fade_ms)).unwrap();
                }
            }
            Op::SpriteFilters { layer, filters } => {
                let slot = SPRITE_BASE + layer;
                writeln!(out, "spfilter {slot},0").unwrap();
                for f in filters {
                    let args = [&f.kind, &f.blend, &f.strength, &f.params[0], &f.params[1], &f.params[2]].map(|o| operand(*o));
                    match f.kind {
                        Operand::Const(0) => {}
                        Operand::Const(_) => writeln!(out, "spfilter {slot},{}", args.join(",")).unwrap(),
                        // spfilter's kind 0 clears every filter; skip it instead.
                        Operand::Var(_) => writeln!(out, "if {} != 0 spfilter {slot},{}", args[0], args.join(",")).unwrap(),
                    }
                }
                if transition.is_none() {
                    writeln!(out, "print 1").unwrap();
                }
            }
            Op::ScreenFilter { slot, filter, fade_ms } => {
                match filter {
                    None if *slot == 0 => writeln!(out, "screenfilter 0").unwrap(),
                    None => writeln!(out, "screenfilter {slot},0").unwrap(),
                    Some(f) => {
                        let args = [&f.kind, &f.blend, &f.strength, &f.params[0], &f.params[1], &f.params[2]].map(|o| operand(*o));
                        writeln!(out, "screenfilter {slot},{}", args.join(",")).unwrap();
                    }
                }
                if transition.is_none() {
                    writeln!(out, "print {}", effect(*fade_ms)).unwrap();
                }
            }
            Op::HideTextWindow => writeln!(out, "textoff").unwrap(),
            Op::InstantText(on) => writeln!(out, "textspeed {}", if *on { "0" } else { speed }).unwrap(),
            Op::SetWindow(w) => {
                writeln!(out, "{}", setwindow(w, speed, has_ruby)).unwrap();
                writeln!(out, "csp {NAME_PLATE_SPRITE}:csp {NAME_TEXT_SPRITE}").unwrap();
                plate_visible = false;
                page_rows = 0;
                window = Some(w.clone());
            }
            Op::ClearSprites => {
                writeln!(out, "csp -1").unwrap();
                out.push_str(&restore_overlay);
                plate_visible = false;
                if transition.is_none() {
                    writeln!(out, "print 1").unwrap();
                }
            }
            Op::Overlay { image, over_sprites } => {
                writeln!(out, "if {OVERLAY_SLOT} != 0 csp {OVERLAY_SLOT}").unwrap();
                match image {
                    Some(image) => {
                        let slot = if *over_sprites { OVERLAY_SPRITE } else { MAX_SPRITE };
                        writeln!(out, "mov {OVERLAY_SLOT},{slot}:mov {OVERLAY_IMAGE},\":a;{}\"", path(image)).unwrap();
                        writeln!(out, "lsp {slot},{OVERLAY_IMAGE},0,0").unwrap();
                    }
                    None => writeln!(out, "mov {OVERLAY_SLOT},0").unwrap(),
                }
                if transition.is_none() {
                    writeln!(out, "print 1").unwrap();
                }
            }
            Op::Noise(noise) => {
                writeln!(out, "csp {NOISE_SPRITE}").unwrap();
                match noise {
                    Some(n) => {
                        let alpha = ((n.opacity * 255 + 128) / 256).clamp(1, 255);
                        let image = format!(":c/{},{},0;{}", n.frames, n.frame_ms, path(&n.image));
                        writeln!(out, "mov {NOISE_ALPHA},{alpha}:mov {NOISE_IMAGE},\"{image}\"").unwrap();
                        writeln!(out, "lsp {NOISE_SPRITE},{NOISE_IMAGE},0,0,{NOISE_ALPHA}").unwrap();
                    }
                    None => writeln!(out, "mov {NOISE_ALPHA},0").unwrap(),
                }
                if transition.is_none() {
                    writeln!(out, "print 1").unwrap();
                }
            }
            Op::BeginTransition { effect: name, fade_ms, mask, style } => {
                writeln!(out, "; transition: {name}").unwrap();
                let steps = transition_steps(*fade_ms, mask.as_deref(), *style);
                transition = Some(if steps.is_empty() { vec![1] } else { steps.iter().map(|s| effects[s]).collect() });
            }
            Op::EndTransition => {
                for e in transition.take().unwrap_or_default() {
                    writeln!(out, "print {e}").unwrap();
                }
            }
            Op::Text { speaker, text, voice, links, ruby, styles, hang, continues } => {
                if let Some(v) = voice {
                    if voice_switches {
                        write!(out, "if %{} == 0 ", menu::VOICE_OFF_BASE + v.character).unwrap();
                    }
                    writeln!(out, "dwave 0,\"{}\"", path(&v.file)).unwrap();
                }
                let clickable: Vec<&TextLink> = links.iter().filter(|l| l.target.is_some()).collect();
                let page = if clickable.is_empty() {
                    None
                } else {
                    // The page is drawn again after each link scene returns.
                    link_pages += 1;
                    let page = format!("conv_link{link_pages}");
                    writeln!(out, "*{page}").unwrap();
                    plate_visible = true; // unknown when entered again: always set it below
                    Some(page)
                };
                let plate = window.as_ref().and_then(|w| w.name_plate.as_ref());
                let mut body = with_markup(text, links, ruby, styles);
                // Sprite changes only show after a print, so print when the
                // plate appears, changes or goes away.
                match (speaker, plate) {
                    (Some(name), Some(p)) => {
                        writeln!(out, "lsp {NAME_PLATE_SPRITE},\":a;{}\",{},{}", path(&p.image), p.x, p.y).unwrap();
                        writeln!(
                            out,
                            "strsp {NAME_TEXT_SPRITE},\"{}\",{},{},16,1,{},{},1,0,0,1",
                            text_safe(name), p.text_x, p.text_y, p.font_size, p.font_size
                        )
                        .unwrap();
                        writeln!(out, "print 1").unwrap();
                        plate_visible = true;
                    }
                    (Some(name), None) => body = format!("【{}】{body}", text_safe(name)),
                    (None, Some(_)) if plate_visible => {
                        writeln!(out, "csp {NAME_PLATE_SPRITE}").unwrap();
                        writeln!(out, "csp {NAME_TEXT_SPRITE}").unwrap();
                        writeln!(out, "print 1").unwrap();
                        plate_visible = false;
                    }
                    (None, _) => {}
                }
                // Each line of the page is its own source line; the last one
                // ends with the page wait. A line whose text starts with ASCII
                // would be read as a command (a colour tag is fine).
                let lines: Vec<String> = body
                    .split('\n')
                    .zip(text.split('\n'))
                    .map(|(l, src)| align_single_bytes(&if starts_ascii(src) { format!("　{l}") } else { l.to_string() }))
                    .collect();
                // A name the plate shows (else it's in the text already).
                let tag_name = speaker.as_deref().filter(|_| plate.is_some()).map(escape).unwrap_or_default();
                let tag_voice = voice.as_ref().map(|v| v.file.replace('\\', "/")).unwrap_or_default();
                if script.backlog.is_some() && page_tag.is_none() && !(tag_name.is_empty() && tag_voice.is_empty()) {
                    page_tag = Some(format!("[{}|{tag_voice}]", tag_name.replace('|', "｜")));
                }
                let lines: Vec<String> = match &page_tag {
                    Some(tag) => lines.into_iter().map(|l| format!("{tag}{l}")).collect(),
                    None => lines,
                };
                if !*continues || page.is_some() {
                    page_tag = None;
                }
                let rows_before = page_rows;
                page_rows = if *continues { page_rows + lines.len() } else { 0 };
                let indent = hang.map_or(0, |n| hang_indent(text, n));
                if indent > 0 {
                    writeln!(out, "indent {indent}").unwrap();
                }
                let Some(page) = page else {
                    // `@` waits and keeps the text (the next page continues
                    // below); `\` waits and clears.
                    let end = if *continues { "@" } else { "\\" };
                    writeln!(out, "{}{end}", lines.join("\n")).unwrap();
                    if indent > 0 {
                        writeln!(out, "indent 0").unwrap();
                    }
                    continue;
                };
                page_rows = 0; // link pages always end with textclear
                // No page wait: the words become buttons and btnwait waits for
                // a click on one of them or anywhere else (= next page).
                writeln!(out, "{}", lines.join("\n")).unwrap();
                if indent > 0 {
                    writeln!(out, "indent 0").unwrap();
                }
                let clickable = &clickable[..clickable.len().min(LINK_SPRITES.count())];
                let window = window.as_ref();
                for (link, slot) in clickable.iter().zip(LINK_SPRITES) {
                    let word: String = text.chars().skip(link.start).take(link.len).collect();
                    let (x, y) = window.map_or((0, 0), |w| char_position(w, text, link.start));
                    let y = y + window.map_or(0, |w| rows_before as i32 * (w.font_size + w.line_gap) as i32);
                    let (size, gap) = window.map_or((24, 0), |w| (w.font_size, w.line_gap));
                    writeln!(
                        out,
                        "strsp {slot},\"{}\",{x},{y},{},1,{size},{size},1,{gap},0,1,\"{LINK_COLOR}\",\"{LINK_HOVER_COLOR}\"",
                        escape(&word),
                        // ONScripter wraps a full-width character one column early.
                        link.len + 1
                    )
                    .unwrap();
                }
                let bar = script.button_bar.as_ref();
                writeln!(out, "*{page}_wait").unwrap();
                if bar.is_some() {
                    // Clears the buttons: register the words after it.
                    writeln!(out, "gosub *{}", menu::BAR_DRAW).unwrap();
                }
                for (n, slot) in LINK_SPRITES.take(clickable.len()).enumerate() {
                    writeln!(out, "spbtn {slot},{}", n + 1).unwrap();
                }
                if bar.is_none() {
                    writeln!(out, "print 1").unwrap();
                }
                writeln!(out, "btnwait {BUTTON_VAR}").unwrap();
                if bar.is_some() {
                    // The pointer near a bar, a bar button or a right-click:
                    // handle it and wait again.
                    writeln!(out, "if {BUTTON_VAR} <= {} goto *{page}_b", menu::HOVER_BASE).unwrap();
                    writeln!(out, "gosub *{}", menu::BAR_HOVER).unwrap();
                    writeln!(out, "goto *{page}_wait").unwrap();
                    writeln!(out, "*{page}_b").unwrap();
                    writeln!(out, "if {BUTTON_VAR} >= 0 && {BUTTON_VAR} <= {} goto *{page}_c", menu::BAR_BUTTON_BASE)
                        .unwrap();
                    writeln!(out, "gosub *{}", menu::BAR_ACTION).unwrap();
                    writeln!(out, "goto *{page}_wait").unwrap();
                    writeln!(out, "*{page}_c").unwrap();
                }
                for slot in LINK_SPRITES.take(clickable.len()) {
                    writeln!(out, "csp {slot}").unwrap();
                }
                writeln!(out, "print 1").unwrap();
                writeln!(out, "textclear").unwrap();
                for (n, link) in clickable.iter().enumerate() {
                    let target = link.target.as_deref().unwrap();
                    writeln!(out, "if {BUTTON_VAR} != {} goto *{page}_{}", n + 1, n + 1).unwrap();
                    let scenes = bar.is_some_and(|b| b.top.is_some());
                    if scenes {
                        // Push the scene for the top bar.
                        let (depth, max) = (menu::SCENE_DEPTH, menu::SCENE_MAX_DEPTH);
                        writeln!(out, "add {depth},1").unwrap();
                        writeln!(out, "if {depth} > {max} mov {max},{depth}").unwrap();
                        writeln!(out, "mov {},{}+{depth}", menu::TMP, menu::SCENE_TITLES).unwrap();
                        writeln!(out, "mov ${},\"{}\"", menu::TMP, escape(link.scene.as_deref().unwrap_or("　"))).unwrap();
                    }
                    // The scene may change the whole screen: save the
                    // background, the game's sprites and the BGM around it.
                    writeln!(out, "pushscreen {NOISE_SPRITE},{}", MAX_SPRITE).unwrap();
                    // popscreen brings the overlay and the static back; so
                    // must the variables.
                    if has_overlay || has_noise {
                        writeln!(out, "add {OVERLAY_DEPTH},1").unwrap();
                    }
                    if has_overlay {
                        writeln!(out, "mov {},{OVERLAY_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                        writeln!(out, "mov %{},{OVERLAY_SLOT}:mov ${},{OVERLAY_IMAGE}", menu::TMP, menu::TMP).unwrap();
                    }
                    if has_noise {
                        writeln!(out, "mov {},{NOISE_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                        writeln!(out, "mov %{},{NOISE_ALPHA}:mov ${},{NOISE_IMAGE}", menu::TMP, menu::TMP).unwrap();
                    }
                    writeln!(out, "gosub *{}", label(target)).unwrap();
                    if scenes {
                        let (depth, max) = (menu::SCENE_DEPTH, menu::SCENE_MAX_DEPTH);
                        writeln!(out, "sub {depth},1").unwrap();
                        writeln!(out, "if {depth} == 0 mov {max},0").unwrap();
                    }
                    writeln!(out, "popscreen").unwrap();
                    if has_overlay {
                        writeln!(out, "mov {},{OVERLAY_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                        writeln!(out, "mov {OVERLAY_SLOT},%{}:mov {OVERLAY_IMAGE},${}", menu::TMP, menu::TMP).unwrap();
                    }
                    if has_noise {
                        writeln!(out, "mov {},{NOISE_SAVED}+{OVERLAY_DEPTH}", menu::TMP).unwrap();
                        writeln!(out, "mov {NOISE_ALPHA},%{}:mov {NOISE_IMAGE},${}", menu::TMP, menu::TMP).unwrap();
                    }
                    if has_overlay || has_noise {
                        writeln!(out, "sub {OVERLAY_DEPTH},1").unwrap();
                    }
                    if let Some(w) = window {
                        // The scene may have switched the text window.
                        writeln!(out, "{}", setwindow(w, speed, has_ruby)).unwrap();
                    }
                    writeln!(out, "print 1").unwrap();
                    writeln!(out, "goto *{page}").unwrap();
                    writeln!(out, "*{page}_{}", n + 1).unwrap();
                }
            }
            Op::BgmPlay { file, fade_in_ms } => {
                writeln!(out, "bgmfadein {fade_in_ms}").unwrap();
                writeln!(out, "bgm \"{}\"", path(file)).unwrap();
            }
            Op::BgmStop { fade_out_ms } => {
                writeln!(out, "bgmfadeout {fade_out_ms}").unwrap();
                writeln!(out, "bgmstop").unwrap();
            }
            Op::Wait { ms } => writeln!(out, "delay {ms}").unwrap(),
            Op::SePlay { file, looped } => {
                let ch = se.start(file);
                let cmd = if *looped { "dwaveloop" } else { "dwave" };
                writeln!(out, "{cmd} {ch},\"{}\"", path(file)).unwrap();
            }
            Op::SeStop { file: Some(file) } => {
                if let Some(ch) = se.stop(file) {
                    writeln!(out, "dwavestop {ch}").unwrap();
                }
            }
            Op::SeStop { file: None } => {
                for ch in se.stop_all() {
                    writeln!(out, "dwavestop {ch}").unwrap();
                }
            }
            Op::ClickWait { skippable } => {
                needs_click_wait = true;
                writeln!(out, "mov {BUTTON_VAR},{}:gosub *{CLICK_WAIT}", u8::from(*skippable)).unwrap();
            }
            Op::Movie { file, skippable } => {
                // The web build plays it in a <video> element over the game.
                writeln!(out, "bgmstop").unwrap();
                writeln!(out, "mpegplay \"{}\",{}", path(file), u8::from(*skippable)).unwrap();
            }
            Op::Chapter { title } => writeln!(out, "mov {},\"{}\"", menu::CHAPTER, escape(title)).unwrap(),
            Op::Unsupported { what, at } => {
                writeln!(out, "; unsupported: {what} ({} #{})", at.file, at.index).unwrap()
            }
        }
    }
    writeln!(out, "end").unwrap();
    if needs_bit_helpers {
        out.push_str(&bit_helpers());
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
    if let Some(m) = &script.save_menu {
        menu::save_menu(&mut out, m, script.confirm.as_ref());
    }
    if let Some(b) = &script.button_bar {
        menu::button_bar(&mut out, b, script.save_menu.is_some(), script.settings.is_some(), script.backlog.is_some());
    }
    if let Some(m) = &script.settings {
        menu::settings_menu(&mut out, m, script.confirm.as_ref());
    }
    if let Some(b) = &script.backlog {
        menu::backlog(&mut out, b, has_ruby);
    }
    if let Some(d) = &script.confirm {
        menu::confirm(&mut out, d);
    }
    out
}

/// The title menu: buttons are sprites over the picture, faded in together;
/// Start clears them and falls through to the script that follows. Load
/// opens the save menu's load screen if there is one, else ONScripter's.
fn title_screen(out: &mut String, t: &TitleScreen, fade: u32, save_menu: bool, settings: bool, confirm: Option<&ConfirmDialog>) {
    let buttons: Vec<_> = t.buttons.iter().zip(MENU_SPRITES).collect();
    // Effect 0 = update the buffer only; the print shows it all.
    writeln!(out, "bg \"{}\",0", path(&t.background)).unwrap();
    writeln!(out, "*{TITLE_DRAW}").unwrap();
    for (b, slot) in &buttons {
        let cells = if b.action == TitleAction::Unavailable { "" } else { "/2,0,3" };
        writeln!(out, "lsp {slot},\":a{cells};{}\",{},{}", path(&b.image), b.x, b.y).unwrap();
    }
    writeln!(out, "print {fade}").unwrap();
    writeln!(out, "*{TITLE_MENU}").unwrap();
    let clickable: Vec<_> = buttons.iter().filter(|(b, _)| b.action != TitleAction::Unavailable).collect();
    for (n, (_, slot)) in clickable.iter().enumerate() {
        writeln!(out, "spbtn {slot},{}", n + 1).unwrap();
    }
    writeln!(out, "btnwait {BUTTON_VAR}").unwrap();
    for (n, (b, _)) in clickable.iter().enumerate() {
        let then = match b.action {
            TitleAction::Start => format!("goto *{TITLE_START}"),
            TitleAction::Load if save_menu => {
                format!("csp -1:mov {},0:gosub *{}:goto *{TITLE_DRAW}", menu::IN_GAME, menu::LOAD)
            }
            TitleAction::Settings if settings => {
                format!("csp -1:mov {},0:gosub *{}:goto *{TITLE_DRAW}", menu::IN_GAME, menu::SETTINGS)
            }
            TitleAction::Settings => continue,
            // The built-in load menu; it comes back here when cancelled.
            TitleAction::Load => format!("systemcall load:goto *{TITLE_MENU}"),
            TitleAction::Exit if confirm.is_some() => "goto *conv_title_exit".to_string(),
            TitleAction::Exit => "end".to_string(),
            TitleAction::Unavailable => unreachable!(),
        };
        writeln!(out, "if {BUTTON_VAR} == {} {then}", n + 1).unwrap();
    }
    writeln!(out, "goto *{TITLE_MENU}").unwrap();
    if let Some(c) = confirm {
        writeln!(out, "*conv_title_exit").unwrap();
        writeln!(out, "{}", menu::confirm_call(&c.quit)).unwrap();
        writeln!(out, "if {} == 1 end", menu::CONFIRM).unwrap();
        writeln!(out, "goto *{TITLE_MENU}").unwrap();
    }
    writeln!(out, "*{TITLE_START}").unwrap();
    writeln!(out, "csp -1").unwrap();
    writeln!(out, "bg black,{fade}").unwrap();
}

/// `setwindow` for a window: text x,y, columns,rows, font w,h, char
/// spacing, line gap, ms per character, bold, shadow, window image, x,y.
/// `speed`: milliseconds per character (a number or a variable).
/// `ruby`: `rubyon` is set, which draws text a line gap lower (the ruby
/// goes above it), so the window starts a line gap higher.
fn setwindow(w: &TextWindow, speed: &str, ruby: bool) -> String {
    let text_y = if ruby { w.text_y - w.line_gap as i32 } else { w.text_y };
    format!(
        "setwindow {},{text_y},{},{},{},{},1,{},{speed},0,1,\":a;{}\",{},{}",
        w.text_x, w.columns, w.rows, w.font_size, w.font_size, w.line_gap, path(&w.image), w.x, w.y
    )
}

/// `lsp` for a sprite; ":a;" = use the PNG's alpha channel.
fn lsp(slot: u32, s: &SpritePlacement) -> String {
    format!("lsp {slot},\":a;{}\",{},{}", path(&s.image), s.x, s.y)
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
    let half: usize = align_single_bytes(&format!("{lead}{}", escape(&prefix)))
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
                '!' => out.extend(chars.by_ref().take(4)), // !zNNN
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

const BITAND: &str = "conv_bitand";
const BITOR: &str = "conv_bitor";

/// Subroutines computing `%3 = %1 & %2` and `%3 = %1 | %2` one bit at a
/// time with div/mod (NScripter has no bitwise operators). They use the
/// scratch variables %1-%8, below the converted game's variables. Works for
/// non-negative values, which is all the game uses (flags).
fn bit_helpers() -> String {
    let mut out = String::new();
    for (name, keep) in [(BITAND, "if %7 == 2 add %3,%4"), (BITOR, "if %7 > 0 add %3,%4")] {
        writeln!(out, "*{name}").unwrap();
        writeln!(out, "mov %3,0:mov %4,1:mov %5,%1:mov %6,%2").unwrap();
        writeln!(out, "*{name}_loop").unwrap();
        writeln!(out, "if %5 == 0 && %6 == 0 return").unwrap();
        writeln!(out, "mov %7,%5:mod %7,2:mov %8,%6:mod %8,2:add %7,%8").unwrap();
        writeln!(out, "{keep}").unwrap();
        writeln!(out, "div %5,2:div %6,2:mul %4,2").unwrap();
        writeln!(out, "goto *{name}_loop").unwrap();
    }
    out
}

/// Command-line options CONscripter reads from `ons_args` in the game
/// directory. Without `--enc:sjis` it reads the script as GBK.
pub const ENGINE_ARGS: &str = "--enc:sjis\n";

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

/// What an `effect` definition draws.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Effect {
    Fade,
    /// Cross-fade through a mask image.
    Mask(String),
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
        (Some(m), _) => one(Effect::Mask(m.to_string())),
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
    let mut colours: Vec<String> = vec![TEXT_COLOR.to_string(); chars.len()];
    for st in styles {
        if let Some(c) = st.colour {
            for colour in colours.iter_mut().skip(st.start).take(st.len) {
                *colour = format!("#{c:06X}");
            }
        }
    }
    for &(s, e) in &link_ranges {
        colours[s..e].fill(LINK_COLOR.to_string());
    }
    let mut sizes: Vec<u16> = vec![100; chars.len()];
    for st in styles {
        if let Some(p) = st.size_percent {
            sizes.iter_mut().skip(st.start).take(st.len).for_each(|s| *s = p.clamp(1, 999));
        }
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
        if let Some(r) = readings.next_if(|r| r.start == i) {
            let base: String = chars[i..i + r.len].iter().collect();
            write!(out, "({}/{})", escape(&base), escape(&r.text)).unwrap();
            i += r.len;
        } else {
            out.push_str(&escape(&chars[i].to_string()));
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
    out
}

fn operand(o: Operand) -> String {
    match o {
        Operand::Const(n) => n.to_string(),
        Operand::Var(v) => var_name(v),
    }
}

fn var_name(v: Var) -> String {
    let base = if v.global { GLOBAL_VAR_BASE } else { LOCAL_VAR_BASE };
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

/// Makes text safe to place on a text line: [`escape`], plus making sure the
/// line doesn't start with ASCII (which would be parsed as a command).
fn text_safe(text: &str) -> String {
    let mut out = escape(text);
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

#[derive(Default)]
struct SeChannels {
    playing: HashMap<String, u32>,
    next: u32,
}

impl SeChannels {
    fn start(&mut self, file: &str) -> u32 {
        let span = SE_CHANNELS.end() - SE_CHANNELS.start() + 1;
        let ch = SE_CHANNELS.start() + self.next % span;
        self.next += 1;
        self.playing.retain(|_, c| *c != ch);
        self.playing.insert(file.to_string(), ch);
        ch
    }

    fn stop(&mut self, file: &str) -> Option<u32> {
        self.playing.remove(file)
    }

    fn stop_all(&mut self) -> Vec<u32> {
        let mut chans: Vec<u32> = self.playing.drain().map(|(_, c)| c).collect();
        chans.sort_unstable();
        chans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{Noise, SourceLoc, SpriteChange, SpriteFilter, SpritePlacement, Voice, FILTER_BLEND_COPY, FILTER_BLEND_MULTIPLY, FILTER_BRIGHTNESS, FILTER_COLOUR};

    fn script(ops: Vec<Op>) -> Script {
        Script { title: "Test".into(), width: 800, height: 600, window: None, title_screen: None, save_menu: None, button_bar: None, settings: None, backlog: None, confirm: None, ops }
    }

    fn link(start: usize, len: usize, target: Option<&str>) -> TextLink {
        TextLink { start, len, target: target.map(String::from), scene: None }
    }

    fn window() -> TextWindow {
        use conv_core::ir::NamePlate;
        TextWindow {
            image: "system/window.png".into(),
            x: 47,
            y: 452,
            text_x: 105,
            text_y: 470,
            columns: 25,
            rows: 3,
            font_size: 24,
            line_gap: 13,
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
        assert!(text.contains("globalon\nwindowback\nhumanz 59\n"));
        // Row 2, column 1: x = 105 + 0 * 25, y = 470 + 1 * 37.
        let expected = "\
*conv_link1
csp 51
csp 50
print 1
#F0C040一#FFFFFF行目
#F0C040学校#FFFFFFに―――
strsp 52,\"学校\",105,507,3,1,24,24,1,13,0,1,\"#F0C040\",\"#FFFFA0\"
*conv_link1_wait
spbtn 52,1
print 1
btnwait %9
csp 52
print 1
textclear
if %9 != 1 goto *conv_link1_1
pushscreen 60,999
gosub *s10_l1
popscreen
setwindow 105,470,25,3,24,24,1,13,20,0,1,\":a;system\\window.png\",47,452
print 1
goto *conv_link1
*conv_link1_1
";
        assert!(text.contains(expected), "{text}");
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
            Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        ]);
        let text = emit(&s);
        let defs = "\
effect 2,16,500
effect 3,17,500
effect 4,14,1000
effect 5,99,1000,\"cons_slide.dll/0,-1,0\"
effect 6,99,1000,\"cons_zoom.dll/3,220,5\"
";
        assert!(text.contains(defs), "{text}");
        assert!(text.contains("; transition: x\nprint 5\n; transition: x\nprint 4\n; transition: x\nprint 2\nprint 3\n; transition: x\nprint 6\nあ"), "{text}");
    }

    #[test]
    fn instant_text_sets_the_text_speed() {
        let text = |t: &str| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false };
        let s = script(vec![Op::InstantText(true), text("一"), Op::InstantText(false), text("二")]);
        let out = emit(&s);
        assert!(out.contains("textspeed 0\n一\\\ntextspeed 20\n二\\\n"), "{out}");
    }

    #[test]
    fn title_screen_runs_before_the_script() {
        use conv_core::ir::TitleButton;
        let button = |image: &str, action| TitleButton { image: image.into(), x: 600, y: 200, action };
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
        });
        let out = emit(&s);
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
    fn full_screen_pages_continue() {
        let mut nvl = window();
        nvl.name_plate = None;
        nvl.image = "system/textwnd5.png".into();
        let text = |t: &str, continues| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues };
        let s = script(vec![Op::SetWindow(nvl), text("一行目", true), text("二行目", false), text("新頁", false)]);
        let out = emit(&s);
        assert!(out.contains("\":a;system\\textwnd5.png\",47,452\ncsp 51:csp 50\n一行目@\n二行目\\\n新頁\\\n"), "{out}");
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
            Op::Sprites { changes: vec![SpriteChange { layer: 3, sprite: None, keep_filters: false }], fade_ms: 0 },
            Op::Text { speaker: Some("名前".into()), text: "「こんにちは」".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
            Op::Text { speaker: None, text: "地の文。".into(), voice: Some(Voice { file: "voice/v1.ogg".into(), character: 3 }), links: vec![link(1, 2, None)], ruby: vec![], styles: vec![], hang: None, continues: false },
            Op::SePlay { file: "se/k001.ogg".into(), looped: true },
            Op::SeStop { file: Some("se/k001.ogg".into()) },
            Op::BgmStop { fade_out_ms: 2000 },
            Op::Wait { ms: 500 },
            Op::Unsupported { what: "op 0x76".into(), at: SourceLoc { file: "0003.cd".into(), index: 7 } },
        ]);
        let text = emit(&s);
        let expected = "\
;mode800
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
csp 65
print 1
【名前】「こんにちは」\\
dwave 0,\"voice\\v1.ogg\"
地#F0C040の文#FFFFFF。\\
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
            image: "system/window.png".into(),
            x: 47,
            y: 452,
            text_x: 105,
            text_y: 470,
            columns: 25,
            rows: 3,
            font_size: 24,
            line_gap: 13,
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
        assert!(text.contains("setwindow 105,470,25,3,24,24,1,13,20,0,1,\":a;system\\window.png\",47,452\nerasetextwindow 0\n"));
        assert!(text.contains("lsp 51,\":a;system\\name.png\",55,412\nstrsp 50,\"名前\",75,422,16,1,20,20,1,0,0,1\nprint 1\n「台詞」\\\n"));
        assert!(text.contains("csp 51\ncsp 50\nprint 1\n　Ghost \\\n"));
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
        assert!(emit(&s).contains("一行目\n　#F0C040abc #FFFFFF\n三行目\\\n"));
    }

    #[test]
    fn links_are_coloured() {
        assert_eq!(with_markup("あいうえお", &[link(1, 2, None), link(3, 9, None)], &[], &[]), "あ#F0C040いう#FFFFFFえお");
        assert_eq!(with_markup("a#b", &[], &[], &[]), "a＃b");
    }

    #[test]
    fn flow_and_variables() {
        let g = Var { global: true, index: 7 };
        let l = Var { global: false, index: 2 };
        let s = script(vec![
            Op::Arith { var: g, op: ArithOp::Set, value: Operand::Const(1) },
            Op::Arith { var: l, op: ArithOp::Add, value: Operand::Var(g) },
            Op::IfEqual { var: g, value: 1, label: "s3_l2".into() },
            Op::Gosub("s2_l18".into()),
            Op::Return,
        ]);
        assert!(emit(&s).contains("mov %207,1\nadd %12,%207\nif %207 == 1 goto *s3_l2\ngosub *s2_l18\nreturn\n"));
    }

    #[test]
    fn bitwise_ops_use_helpers() {
        let g = Var { global: true, index: 9 };
        let h = Var { global: true, index: 12 };
        let text = emit(&script(vec![Op::Arith { var: g, op: ArithOp::BitAnd, value: Operand::Var(h) }]));
        assert!(text.contains("mov %1,%209:mov %2,%212:gosub *conv_bitand:mov %209,%3\n"));
        assert!(text.contains("*conv_bitand\n") && text.contains("*conv_bitor\n"));
        // Helpers are only emitted when needed.
        assert!(!emit(&script(vec![])).contains("conv_bitand"));
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
    }

    #[test]
    fn single_byte_runs_keep_their_pairs() {
        // A space where a pair would start is skipped by ONScripter.
        assert_eq!(align_single_bytes("ABC続き"), "ABC 続き");
        assert_eq!(align_single_bytes("AB続き"), "AB続き");
        assert_eq!(align_single_bytes(" xyz続"), " xyz 続");
        assert_eq!(align_single_bytes("a b続"), "a b 続");
        assert_eq!(align_single_bytes("ｼｬﾄﾞｳshadow）"), "ｼｬﾄﾞｳshadow ）");
        assert_eq!(align_single_bytes("x#F0C040y#FFFFFF。"), "x #F0C040y #FFFFFF。");
        assert_eq!(align_single_bytes("ab c"), "ab c ");
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
        assert_eq!(ch.start("a"), 1);
        assert_eq!(ch.start("b"), 2);
        assert_eq!(ch.stop("a"), Some(1));
        assert_eq!(ch.stop("a"), None);
        assert_eq!(ch.stop_all(), vec![2]);
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
            Op::Noise(Some(noise)),
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
        let st = |start, len, colour| TextStyle { start, len, colour, size_percent: None };
        // Each line returns to the text colour; links win over styles.
        assert_eq!(
            with_markup("あいう\nえお", &[], &[], &[st(1, 4, Some(0xffff00))]),
            "あ#FFFF00いう#FFFFFF\n#FFFF00え#FFFFFFお"
        );
        assert_eq!(
            with_markup("あいうえ", &[link(2, 1, None)], &[], &[st(0, 4, Some(0x00ff00)), st(3, 1, Some(0x0000ff))]),
            "#00FF00あい#F0C040う#0000FFえ#FFFFFF"
        );
        assert_eq!(with_markup("あい", &[], &[], &[st(0, 2, None)]), "あい");
        let sized = |start, len, p| TextStyle { start, len, colour: None, size_percent: Some(p) };
        assert_eq!(
            with_markup("あい\nう", &[], &[], &[sized(0, 4, 150), TextStyle { colour: Some(0xffff00), ..sized(3, 1, 75) }]),
            "!z150あい!z100\n#FFFF00!z075う!z100#FFFFFF"
        );
        assert_eq!(align_single_bytes("Ghost!z100の"), "Ghost !z100の");
    }

    #[test]
    fn ruby_markup() {
        let r = |start, len, text: &str| Ruby { start, len, text: text.into() };
        // After escaping: the text's own parentheses stay full-width.
        assert_eq!(with_markup("(戦ぐ)", &[], &[r(1, 1, "そよ")], &[]), "（(戦/そよ)ぐ）");
        // Inside a link, but not across its edge or a line.
        assert_eq!(
            with_markup("あ獄い\nう", &[link(0, 3, None)], &[r(1, 1, "ひとや"), r(2, 3, "x")], &[]),
            "#F0C040あ(獄/ひとや)い#FFFFFF\nう"
        );
        assert_eq!(with_markup("あいう", &[link(1, 2, None)], &[r(0, 2, "x")], &[]), "あ#F0C040いう#FFFFFF");
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
        assert!(out.contains("setwindow 105,457,25,3,24,24,1,13,"), "{out}");
        assert!(out.contains("(獄/ひとや)\\\n"), "{out}");
    }
}
