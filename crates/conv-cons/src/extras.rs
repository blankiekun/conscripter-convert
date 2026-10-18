//! The extras menu the title's option button opens, a script loop; the
//! CG gallery, BGM room and movie room it opens are screens the engine
//! draws ([`crate::extras_screens`]; their scripts in [`crate::music`] and
//! [`crate::movies`]), the scene replay is in [`crate::replay`]. All are
//! only reached from the title, so they use the game's sprite slots
//! ([`SPRITES`]) and clear everything with `csp -1`.

use conv_core::ir::{ExtrasAction, ExtrasMenu, Placed};

use crate::{extras_screens, menu, movies, music, path, replay, BUTTON_VAR, CG_SEEN};

pub(crate) const EXTRAS: &str = "conv_extras";
/// Back to the title screen (its background and buttons).
pub(crate) const TITLE_BACK: &str = "conv_title_back";
const DRAW: &str = "conv_extras_draw";
const GALLERY: &str = "conv_cg";
/// Sprite slots (the game's, unused at the title).
const SPRITES: u32 = 100;
const MENU_ITEM: u32 = SPRITES; // + item
// Buttons.
// Variables (cleared when the extras close).
const TAB_VAR: &str = "%120";
const PAGE_VAR: &str = "%121";
/// Scratch: tab × MAX_PAGES + page; whether a slot is unlocked.
const TMP: &str = "%122";
/// The slot shown in the viewer (across all pages), its variant, and the
/// direction to step past unseen variants.
const SLOT_VAR: &str = "%123";
const VARIANT: &str = "%124";
const STEP: &str = "%126";
/// The shown page's first slot number.
const PAGE_FIRST: &str = "%127";
/// 1 while the viewer's panel is hidden.
const PANEL_HIDDEN: &str = "%129";
const UNLOCKED: &str = "%128";

fn button(slot: u32, p: &Placed, cells: u32) -> String {
    format!("lsp {slot},\":a/{cells},0,3;{}\",{},{}", path(&p.image), p.x, p.y)
}

fn picture(slot: u32, image: &str, x: i32, y: i32) -> String {
    format!("lsp {slot},\":a;{}\",{x},{y}", path(image))
}

fn seen(number: u32) -> String {
    CG_SEEN.at(number)
}

/// `fade`: the effect the title fades in with. `settings`: there is a
/// settings screen for the option heading to open.
pub(crate) fn extras(out: &mut String, m: &ExtrasMenu, fade: &str, settings: bool) {
    let mut w = lines!(out);
    let gallery = m.gallery.as_ref();
    let clear_vars = [TAB_VAR, PAGE_VAR, TMP, SLOT_VAR, VARIANT, STEP, PAGE_FIRST, UNLOCKED, PANEL_HIDDEN]
        .iter()
        .chain(replay::VARS.iter())
        .map(|v| format!("mov {v},0"))
        .collect::<Vec<_>>()
        .join(":");
    let menu_background = m.menu_background.as_deref().unwrap_or(&m.background);
    w(format!("*{EXTRAS}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(menu_background)));
    w(format!("print {fade}"));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(menu_background)));
    // The gallery is open once one of its CGs has been seen.
    w(format!("mov {UNLOCKED},0"));
    if let Some(g) = gallery {
        let mut numbers: Vec<u32> = g.tabs.iter().flat_map(|t| t.pages.iter().flatten()).flat_map(|s| s.cgs.iter().map(|c| c.0)).collect();
        numbers.sort_unstable();
        numbers.dedup();
        // One `if` a line: a false one skips the rest of its line.
        for n in numbers {
            w(format!("if {} == 1 mov {UNLOCKED},1", seen(n)));
        }
    }
    let mut buttons = Vec::new();
    for (i, item) in m.items.iter().enumerate() {
        let slot = MENU_ITEM + i as u32;
        let n = i as i32 + 1;
        match item.action {
            ExtrasAction::Unavailable => w(picture(slot, &item.button.image, item.button.x, item.button.y)),
            ExtrasAction::Settings if !settings => {
                w(format!("lsp {slot},\":a/3,0,3;{}\",{},{}:cell {slot},2", path(&item.button.image), item.button.x, item.button.y))
            }
            ExtrasAction::Gallery if gallery.is_none() => {
                w(format!("lsp {slot},\":a/3,0,3;{}\",{},{}:cell {slot},2", path(&item.button.image), item.button.x, item.button.y))
            }
            ExtrasAction::Gallery => {
                w(button(slot, &item.button, 3));
                w(format!("if {UNLOCKED} == 0 cell {slot},2"));
                w(format!("if {UNLOCKED} == 1 spbtn {slot},{n}"));
                buttons.push((n, format!("gosub *{GALLERY}:goto *{DRAW}")));
            }
            ExtrasAction::Music if m.music.is_none() => {
                w(format!("lsp {slot},\":a/3,0,3;{}\",{},{}:cell {slot},2", path(&item.button.image), item.button.x, item.button.y))
            }
            ExtrasAction::Music => {
                w(button(slot, &item.button, 3));
                w(format!("spbtn {slot},{n}"));
                buttons.push((n, format!("goto *{}", music::MUSIC)));
            }
            ExtrasAction::Movies if m.movies.is_none() => {
                w(format!("lsp {slot},\":a/3,0,3;{}\",{},{}:cell {slot},2", path(&item.button.image), item.button.x, item.button.y))
            }
            ExtrasAction::Movies => {
                w(button(slot, &item.button, 3));
                w(format!("spbtn {slot},{n}"));
                buttons.push((n, format!("goto *{}", movies::MOVIES)));
            }
            ExtrasAction::Replay if m.replay.is_none() => {
                w(format!("lsp {slot},\":a/3,0,3;{}\",{},{}:cell {slot},2", path(&item.button.image), item.button.x, item.button.y))
            }
            ExtrasAction::Replay => {
                w(button(slot, &item.button, 3));
                w(format!("spbtn {slot},{n}"));
                buttons.push((n, format!("goto *{}", replay::REPLAY)));
            }
            ExtrasAction::Settings => {
                w(button(slot, &item.button, 3));
                w(format!("spbtn {slot},{n}"));
                buttons.push((n, format!("csp -1:mov {},0:gosub *{}:goto *{DRAW}", menu::IN_GAME, menu::SETTINGS)));
            }
            ExtrasAction::Back => {
                w(button(slot, &item.button, 3));
                w(format!("spbtn {slot},{n}"));
                buttons.push((n, "goto *conv_extras_close".to_string()));
            }
        }
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    for (n, then) in buttons {
        w(format!("if {BUTTON_VAR} == {n} {then}"));
    }
    w(format!("if {BUTTON_VAR} == -1 goto *conv_extras_close"));
    w(format!("goto *{DRAW}"));
    w("*conv_extras_close".into());
    w(format!("csp -1:{clear_vars}"));
    w(format!("goto *{TITLE_BACK}"));
    if gallery.is_some() {
        // The screen's page from the first each time.
        out.push_str(&format!("*{GALLERY}\ncsp -1:mov {},0:screen \"gallery\"\nreturn\n", extras_screens::GALLERY_PAGE));
    }
    if let Some(r) = &m.replay {
        replay::replay(out, r, DRAW);
    }
    if let Some(music) = &m.music {
        music::screen_labels(out, music, DRAW, settings);
    }
    if let Some(room) = &m.movies {
        movies::screen_labels(out, room, DRAW);
    }
}

