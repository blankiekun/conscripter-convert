//! ONScripter output stage: turns an IR [`Script`] into `0.txt`.
//!
//! Targets ogapee's ONScripter with Japanese text. Things marked
//! "unverified" below follow the NScripter documentation but haven't been
//! checked in a running engine yet (see PLAN.md, Phase 0).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use vn2ons_core::ir::{
    ArithOp, ConfirmDialog, Op, Operand, Script, SpritePlacement, TextLink, TextWindow, TitleAction, TitleScreen, Var,
};

mod menu;
pub mod nsa;

/// First effect number we define; 1 is NScripter's built-in "instant".
const FIRST_EFFECT: u32 = 2;
/// NScripter effect type 10 is a cross-fade; 18 a cross-fade shaped by a
/// mask image (dark parts first).
const FADE: u32 = 10;
const MASK_FADE: u32 = 18;
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
/// Sprite layer n is NScripter sprite 60 + n.
const SPRITE_BASE: u32 = 60;
/// Scratch variable receiving the clicked button (also used by nstrace).
const BUTTON_VAR: &str = "%9";
/// NScripter variables: locals from %10, globals from %200. `globalon`
/// makes %200 and up persist across games.
const LOCAL_VAR_BASE: u32 = 10;
const GLOBAL_VAR_BASE: u32 = 200;
/// Labels of the title menu loop and of its Start button's target.
/// Click wait with everything hidden ([`Op::ClickWait`]).
const CLICK_WAIT: &str = "vn2ons_click_wait";
const TITLE_DRAW: &str = "vn2ons_title_draw";
const TITLE_MENU: &str = "vn2ons_title";
const TITLE_START: &str = "vn2ons_title_start";
/// Colour for words the original game made clickable, and normal text.
const LINK_COLOR: &str = "#F0C040";
/// Link words under the mouse pointer.
const LINK_HOVER_COLOR: &str = "#FFFFA0";
const TEXT_COLOR: &str = "#FFFFFF";

/// Renders the script as NScripter source text (UTF-8; use [`encode`] to
/// produce the Shift-JIS bytes ONScripter reads).
pub fn emit(script: &Script) -> String {
    let effects = fade_effects(script);
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
    // The caption is passed to the window system as raw Shift-JIS, which
    // shows as mojibake on Linux, so keep it ASCII.
    if script.backlog.is_some() {
        // Wheel and arrow keys reach btnwait (backlog scrolling); keep more
        // pages of history than the default 16.
        writeln!(out, "usewheel").unwrap();
        writeln!(out, "maxkaisoupage 100").unwrap();
    }
    if script.button_bar.is_some() {
        // Every click wait runs the bar routine instead of waiting itself.
        writeln!(out, "textgosub *{}", menu::WAIT).unwrap();
    }
    let caption: String = script.title.chars().filter(|c| c.is_ascii() && *c != '"').collect();
    writeln!(out, "caption \"{}\"", caption.trim()).unwrap();
    for ((ms, mask), n) in &effects {
        match mask {
            Some(m) => writeln!(out, "effect {n},{MASK_FADE},{ms},\"{}\"", path(m)).unwrap(),
            None => writeln!(out, "effect {n},{FADE},{ms}").unwrap(),
        }
    }
    writeln!(out, "game").unwrap();
    writeln!(out, "*start").unwrap();
    // The text speed is a setting when there is a settings menu.
    let speed = if script.settings.is_some() { menu::TEXT_MS } else { "20" };
    if script.settings.is_some() {
        menu::settings_init(&mut out);
    }
    // setwindow redraws the screen, so it must come after `game` (ONScripter
    // crashes if it runs in the define section).
    if let Some(w) = &script.window {
        writeln!(out, "{}", setwindow(w, speed)).unwrap();
        // Keep the window up while sprites change and pages are printed
        // (by default every print hides it until the next text, which
        // flickers); the script hides it explicitly with textoff.
        writeln!(out, "erasetextwindow 0").unwrap();
    }

    let effect = |ms: u32| if ms == 0 { 1 } else { effects[&(ms, None)] };
    if let Some(t) = &script.title_screen {
        title_screen(&mut out, t, effect(t.fade_ms), script.save_menu.is_some(), script.settings.is_some(), script.confirm.as_ref());
    }
    let mut se = SeChannels::default();
    let mut needs_bit_helpers = false;
    let mut needs_click_wait = false;
    let mut plate_visible = false;
    // What is on screen at this point in file order, to restore after a link
    // scene returns (an approximation when control flow jumps around).
    let mut screen = Screen::default();
    let mut link_pages = 0;
    // Effect number of an open transition: display commands only update the
    // buffer until it is printed.
    let mut transition: Option<u32> = None;
    // The active text window (file order; presets are called right before
    // the text that uses them) and the lines already on its page.
    let mut window = script.window.clone();
    let mut page_rows = 0;
    for op in &script.ops {
        if let (Some(e), Op::Text { .. }) = (transition, op) {
            writeln!(out, "print {e}").unwrap();
            transition = None;
        }
        match op {
            Op::Label(name) => writeln!(out, "*{}", label(name)).unwrap(),
            Op::Jump(name) => writeln!(out, "goto *{}", label(name)).unwrap(),
            Op::Gosub(name) => writeln!(out, "gosub *{}", label(name)).unwrap(),
            Op::Return => writeln!(out, "return").unwrap(),
            Op::Arith { var, op, value } => {
                let value = match value {
                    Operand::Const(n) => n.to_string(),
                    Operand::Var(v) => var_name(*v),
                };
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
                // Effect 0 = update the buffer only (shown by the transition).
                let e = if transition.is_some() { 0 } else { effect(*fade_ms) };
                writeln!(out, "bg \"{}\",{e}", path(image)).unwrap();
                plate_visible = false;
                screen.bg = Some(image.clone());
                screen.sprites.clear();
            }
            Op::Sprites { changes, fade_ms } => {
                for c in changes {
                    let slot = SPRITE_BASE + c.layer;
                    match &c.sprite {
                        Some(sp) => {
                            writeln!(out, "{}", lsp(slot, sp)).unwrap();
                            screen.sprites.insert(c.layer, sp.clone());
                        }
                        None => {
                            writeln!(out, "csp {slot}").unwrap();
                            screen.sprites.remove(&c.layer);
                        }
                    }
                }
                if transition.is_none() {
                    writeln!(out, "print {}", effect(*fade_ms)).unwrap();
                }
            }
            Op::HideTextWindow => writeln!(out, "textoff").unwrap(),
            Op::SetWindow(w) => {
                writeln!(out, "{}", setwindow(w, speed)).unwrap();
                writeln!(out, "csp {NAME_PLATE_SPRITE}:csp {NAME_TEXT_SPRITE}").unwrap();
                plate_visible = false;
                page_rows = 0;
                window = Some(w.clone());
            }
            Op::ClearSprites => {
                writeln!(out, "csp -1").unwrap();
                plate_visible = false;
                screen.sprites.clear();
                if transition.is_none() {
                    writeln!(out, "print 1").unwrap();
                }
            }
            Op::BeginTransition { effect: name, fade_ms, mask } => {
                writeln!(out, "; transition: {name}").unwrap();
                transition = Some(match mask {
                    Some(_) if *fade_ms == 0 => 1,
                    Some(m) => effects[&(*fade_ms, Some(m.clone()))],
                    None => effect(*fade_ms),
                });
            }
            Op::EndTransition => {
                if let Some(e) = transition.take() {
                    writeln!(out, "print {e}").unwrap();
                }
            }
            Op::Text { speaker, text, voice, links, continues } => {
                if let Some(v) = voice {
                    writeln!(out, "dwave 0,\"{}\"", path(v)).unwrap();
                }
                let clickable: Vec<&TextLink> = links.iter().filter(|l| l.target.is_some()).collect();
                let page = if clickable.is_empty() {
                    None
                } else {
                    // The page is drawn again after each link scene returns.
                    link_pages += 1;
                    let page = format!("vn2ons_link{link_pages}");
                    writeln!(out, "*{page}").unwrap();
                    plate_visible = true; // unknown when entered again: always set it below
                    Some(page)
                };
                let plate = window.as_ref().and_then(|w| w.name_plate.as_ref());
                let mut body = with_links(text, links);
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
                    .map(|(l, src)| if starts_ascii(src) { format!("　{l}") } else { l.to_string() })
                    .collect();
                let rows_before = page_rows;
                page_rows = if *continues { page_rows + lines.len() } else { 0 };
                let Some(page) = page else {
                    // `@` waits and keeps the text (the next page continues
                    // below); `\` waits and clears.
                    let end = if *continues { "@" } else { "\\" };
                    writeln!(out, "{}{end}", lines.join("\n")).unwrap();
                    continue;
                };
                page_rows = 0; // link pages always end with textclear
                // No page wait: the words become buttons and btnwait waits for
                // a click on one of them or anywhere else (= next page).
                writeln!(out, "{}", lines.join("\n")).unwrap();
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
                    writeln!(out, "gosub *{}", label(target)).unwrap();
                    if scenes {
                        let (depth, max) = (menu::SCENE_DEPTH, menu::SCENE_MAX_DEPTH);
                        writeln!(out, "sub {depth},1").unwrap();
                        writeln!(out, "if {depth} == 0 mov {max},0").unwrap();
                    }
                    screen.restore(&mut out);
                    writeln!(out, "goto *{page}").unwrap();
                    writeln!(out, "*{page}_{}", n + 1).unwrap();
                }
            }
            Op::BgmPlay { file, fade_in_ms } => {
                writeln!(out, "bgmfadein {fade_in_ms}").unwrap();
                writeln!(out, "bgm \"{}\"", path(file)).unwrap();
                screen.bgm = Some(file.clone());
            }
            Op::BgmStop { fade_out_ms } => {
                writeln!(out, "bgmfadeout {fade_out_ms}").unwrap();
                writeln!(out, "bgmstop").unwrap();
                screen.bgm = None;
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
        menu::backlog(&mut out, b);
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
            TitleAction::Exit if confirm.is_some() => "goto *vn2ons_title_exit".to_string(),
            TitleAction::Exit => "end".to_string(),
            TitleAction::Unavailable => unreachable!(),
        };
        writeln!(out, "if {BUTTON_VAR} == {} {then}", n + 1).unwrap();
    }
    writeln!(out, "goto *{TITLE_MENU}").unwrap();
    if let Some(c) = confirm {
        writeln!(out, "*vn2ons_title_exit").unwrap();
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
fn setwindow(w: &TextWindow, speed: &str) -> String {
    format!(
        "setwindow {},{},{},{},{},{},1,{},{speed},0,1,\":a;{}\",{},{}",
        w.text_x, w.text_y, w.columns, w.rows, w.font_size, w.font_size, w.line_gap, path(&w.image), w.x, w.y
    )
}

/// `lsp` for a sprite; ":a;" = use the PNG's alpha channel.
fn lsp(slot: u32, s: &SpritePlacement) -> String {
    format!("lsp {slot},\":a;{}\",{},{}", path(&s.image), s.x, s.y)
}

/// Background, sprites and BGM as last set, in file order.
#[derive(Default)]
struct Screen {
    bg: Option<String>,
    sprites: BTreeMap<u32, SpritePlacement>,
    bgm: Option<String>,
}

impl Screen {
    /// Puts the screen back after a link scene (which may change all of it).
    fn restore(&self, out: &mut String) {
        writeln!(out, "csp -1").unwrap();
        match &self.bg {
            Some(image) => writeln!(out, "bg \"{}\",1", path(image)).unwrap(),
            None => writeln!(out, "bg black,1").unwrap(),
        }
        for (layer, s) in &self.sprites {
            writeln!(out, "{}", lsp(SPRITE_BASE + layer, s)).unwrap();
        }
        writeln!(out, "print 1").unwrap();
        match &self.bgm {
            Some(file) => writeln!(out, "bgm \"{}\"", path(file)).unwrap(),
            None => writeln!(out, "bgmstop").unwrap(),
        }
    }
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

fn starts_ascii(line: &str) -> bool {
    line.starts_with(|c: char| c.is_ascii())
}

const BITAND: &str = "vn2ons_bitand";
const BITOR: &str = "vn2ons_bitor";

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

/// One effect number per distinct non-zero (duration, mask) pair, in a
/// stable order: plain cross-fades and mask cross-fades.
fn fade_effects(script: &Script) -> BTreeMap<(u32, Option<String>), u32> {
    let mut keys: Vec<(u32, Option<String>)> = script
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Background { fade_ms, .. } | Op::Sprites { fade_ms, .. } => Some((*fade_ms, None)),
            Op::BeginTransition { fade_ms, mask, .. } => Some((*fade_ms, mask.clone())),
            _ => None,
        })
        .chain(script.title_screen.iter().map(|t| (t.fade_ms, None)))
        .filter(|(ms, _)| *ms > 0)
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter().zip(FIRST_EFFECT..).collect()
}

/// Escapes the text and wraps each link range in colour tags. Ranges are in
/// characters of `text`; overlapping or out-of-range ones are skipped.
fn with_links(text: &str, links: &[TextLink]) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut ranges: Vec<(usize, usize)> =
        links.iter().map(|l| (l.start, l.len)).filter(|&(s, n)| n > 0 && s + n <= chars.len()).collect();
    ranges.sort_unstable();
    let mut out = String::new();
    let mut pos = 0;
    for (start, len) in ranges {
        if start < pos {
            continue;
        }
        out.push_str(&escape(&chars[pos..start].iter().collect::<String>()));
        out.push_str(LINK_COLOR);
        out.push_str(&escape(&chars[start..start + len].iter().collect::<String>()));
        out.push_str(TEXT_COLOR);
        pos = start + len;
    }
    out.push_str(&escape(&chars[pos..].iter().collect::<String>()));
    out
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
    use vn2ons_core::ir::{SourceLoc, SpriteChange, SpritePlacement};

    fn script(ops: Vec<Op>) -> Script {
        Script { title: "Test".into(), width: 800, height: 600, window: None, title_screen: None, save_menu: None, button_bar: None, settings: None, backlog: None, confirm: None, ops }
    }

    fn link(start: usize, len: usize, target: Option<&str>) -> TextLink {
        TextLink { start, len, target: target.map(String::from), scene: None }
    }

    fn window() -> TextWindow {
        use vn2ons_core::ir::NamePlate;
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
                continues: false,
            },
        ]);
        s.window = Some(window());
        let text = emit(&s);
        assert!(text.contains("globalon\nwindowback\nhumanz 59\n"));
        // Row 2, column 1: x = 105 + 0 * 25, y = 470 + 1 * 37.
        let expected = "\
*vn2ons_link1
csp 51
csp 50
print 1
#F0C040一#FFFFFF行目
#F0C040学校#FFFFFFに―――
strsp 52,\"学校\",105,507,3,1,24,24,1,13,0,1,\"#F0C040\",\"#FFFFA0\"
*vn2ons_link1_wait
spbtn 52,1
print 1
btnwait %9
csp 52
print 1
textclear
if %9 != 1 goto *vn2ons_link1_1
gosub *s10_l1
csp -1
bg \"bg\\room.png\",1
print 1
bgm \"bgm\\a.ogg\"
goto *vn2ons_link1
*vn2ons_link1_1
";
        assert!(text.contains(expected), "{text}");
    }

    #[test]
    fn transitions_reveal_changes_at_once() {
        let sprite = SpritePlacement { image: "sprite/a.png".into(), x: 1, y: 2 };
        let s = script(vec![
            Op::BeginTransition { effect: "CrossFade".into(), fade_ms: 2000, mask: None },
            Op::ClearSprites,
            Op::Background { image: "bg/a.png".into(), fade_ms: 500 },
            Op::Sprites { changes: vec![SpriteChange { layer: 5, sprite: Some(sprite) }], fade_ms: 500 },
            Op::EndTransition,
            Op::BeginTransition { effect: "MaskBlend2".into(), fade_ms: 1000, mask: Some("system/mask0009r.png".into()) },
            Op::Background { image: "bg/b.png".into(), fade_ms: 500 },
            Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], continues: false },
        ]);
        let text = emit(&s);
        assert!(text.contains("effect 2,10,500\neffect 3,18,1000,\"system\\mask0009r.png\"\neffect 4,10,2000\n"), "{text}");
        let expected = "\
; transition: CrossFade
csp -1
csp -1
bg \"bg\\a.png\",0
lsp 65,\":a;sprite\\a.png\",1,2
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
    fn title_screen_runs_before_the_script() {
        use vn2ons_core::ir::TitleButton;
        let button = |image: &str, action| TitleButton { image: image.into(), x: 600, y: 200, action };
        let mut s = script(vec![Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], continues: false }]);
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
        let expected = "*start\nbg \"system\\title.png\",0\n*vn2ons_title_draw\n\
            lsp 6,\":a/2,0,3;system\\start.png\",600,200\n\
            lsp 7,\":a;system\\option.png\",600,200\n\
            lsp 8,\":a/2,0,3;system\\load.png\",600,200\n\
            lsp 9,\":a/2,0,3;system\\exit.png\",600,200\n\
            print 2\n*vn2ons_title\nspbtn 6,1\nspbtn 8,2\nspbtn 9,3\nbtnwait %9\n\
            if %9 == 1 goto *vn2ons_title_start\n\
            if %9 == 2 systemcall load:goto *vn2ons_title\n\
            if %9 == 3 end\n\
            goto *vn2ons_title\n*vn2ons_title_start\ncsp -1\nbg black,2\nあ\\\n";
        assert!(out.contains(expected), "{out}");
    }

    #[test]
    fn full_screen_pages_continue() {
        let mut nvl = window();
        nvl.name_plate = None;
        nvl.image = "system/textwnd5.png".into();
        let text = |t: &str, continues| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], continues };
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
                }],
                fade_ms: 500,
            },
            Op::Sprites { changes: vec![SpriteChange { layer: 3, sprite: None }], fade_ms: 0 },
            Op::Text { speaker: Some("名前".into()), text: "「こんにちは」".into(), voice: None, links: vec![], continues: false },
            Op::Text { speaker: None, text: "地の文。".into(), voice: Some("voice/v1.ogg".into()), links: vec![link(1, 2, None)], continues: false },
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
lsp 63,\":a;sprite\\a.png\",10,-20
print 2
csp 63
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
        use vn2ons_core::ir::{NamePlate, TextWindow};
        let mut s = script(vec![
            Op::Text { speaker: Some("名前".into()), text: "「台詞」".into(), voice: None, links: vec![], continues: false },
            Op::Text { speaker: None, text: "Ghost".into(), voice: None, links: vec![], continues: false },
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
        assert!(text.contains("csp 51\ncsp 50\nprint 1\n　Ghost\\\n"));
    }

    #[test]
    fn multi_line_text() {
        let s = script(vec![Op::Text {
            speaker: None,
            text: "一行目\nabc\n三行目".into(),
            voice: None,
            links: vec![link(4, 3, None)],
            continues: false,
        }]);
        assert!(emit(&s).contains("一行目\n　#F0C040abc#FFFFFF\n三行目\\\n"));
    }

    #[test]
    fn links_are_coloured() {
        assert_eq!(with_links("あいうえお", &[link(1, 2, None), link(3, 9, None)]), "あ#F0C040いう#FFFFFFえお");
        assert_eq!(with_links("a#b", &[]), "a＃b");
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
        assert!(text.contains("mov %1,%209:mov %2,%212:gosub *vn2ons_bitand:mov %209,%3\n"));
        assert!(text.contains("*vn2ons_bitand\n") && text.contains("*vn2ons_bitor\n"));
        // Helpers are only emitted when needed.
        assert!(!emit(&script(vec![])).contains("vn2ons_bitand"));
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
}
