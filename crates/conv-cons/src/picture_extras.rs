//! [`PictureExtras`] as NScripter code under [`EXTRAS`], reached from the
//! title's [`TitleAction::Extras`](conv_core::ir::TitleAction::Extras)
//! button with the title still on screen (the menu cross-fades in over
//! it). Only reached from the title, so it takes the game's sprite range
//! and clears everything with `csp -1`; the buttons are in the menus'
//! range. Going back sets [`TITLE_SHOWN`] to 2, which the title reads as
//! "back from the extras": its music goes on, and a
//! [`TitleOpening::after_extras`](conv_core::ir::TitleOpening::after_extras)
//! opening plays.

use std::fmt::Write;

use conv_core::ir::{ExtrasMode, PictureExtras};

use crate::{menu_slot, menu_z, path, var_name, BUTTON_VAR, MAX_SPRITE, MENU_SLOTS, SE_CHANNELS, TITLE_SHOWN};

pub(crate) const EXTRAS: &str = "conv_pextras";
const DRAW: &str = "conv_pextras_draw";
const WAIT: &str = "conv_pextras_wait";
const BACK: &str = "conv_pextras_back";
/// The title's label that draws it again (its picture, then its openings).
pub(crate) const TITLE_BACK: &str = crate::extras::TITLE_BACK;
const BACKGROUND: u32 = MAX_SPRITE;

/// A picture for `lsp`: a JPEG as it is, another (a PNG) with its alpha.
pub(crate) fn pic(p: &str) -> String {
    let mode = format!(":{};", crate::sprite_mode(p));
    format!("{mode}{}", path(p))
}

/// Whether `mode` has a screen in this script.
fn has_mode(e: &PictureExtras, mode: ExtrasMode) -> bool {
    match mode {
        ExtrasMode::Back => true,
        ExtrasMode::Gallery => e.gallery.is_some(),
        ExtrasMode::Sound => e.sound.is_some(),
        ExtrasMode::Recollection => e.replay.is_some(),
        ExtrasMode::Turntable => e.turntable.is_some(),
    }
}

/// Label of a mode's screen.
fn mode_label(mode: ExtrasMode) -> String {
    match mode {
        ExtrasMode::Back => BACK.to_string(),
        other => format!("conv_pextras_{}", format!("{other:?}").to_lowercase()),
    }
}

pub(crate) fn emit(out: &mut String, e: &PictureExtras, border: u32, width: u32, height: u32) {
    assert!(e.items.len() <= MENU_SLOTS, "more extras buttons than menu sprites");
    let fade = if e.fade_ms > 0 { format!("print 10,{}", e.fade_ms) } else { "print 1".to_string() };
    writeln!(out, "*{EXTRAS}").unwrap();
    if let Some(r) = &e.sound {
        writeln!(out, "{}", crate::picture_sound::on_open(r)).unwrap();
    }
    writeln!(out, "*{DRAW}").unwrap();
    writeln!(out, "csp -1").unwrap();
    writeln!(out, "lsp {BACKGROUND},\"{}\",0,0", pic(&e.background)).unwrap();
    for (i, item) in e.items.iter().enumerate() {
        let b = &item.button;
        let (slot, z) = (menu_slot(i), menu_z(menu_slot(i)));
        let lsp = format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&b.image), b.x, b.y);
        match item.unlocked_by {
            Some(v) => {
                let v = var_name(v, border);
                writeln!(out, "if {v} == 0 {lsp},{}{z}", item.locked_alpha).unwrap();
                writeln!(out, "if {v} != 0 {lsp}{z}").unwrap();
            }
            None => writeln!(out, "{lsp}{z}").unwrap(),
        }
    }
    writeln!(out, "{fade}").unwrap();
    writeln!(out, "*{WAIT}").unwrap();
    if let Some(se) = &e.hover_se {
        writeln!(out, "selectvoice \"\",\"{}\",\"\"", path(se)).unwrap();
    }
    // Buttons that open something, and only while unlocked.
    let clickable: Vec<_> = e.items.iter().enumerate().filter(|(_, item)| has_mode(e, item.mode)).collect();
    for (n, (i, item)) in clickable.iter().enumerate() {
        let only = item.unlocked_by.map_or(String::new(), |v| format!("if {} != 0 ", var_name(v, border)));
        writeln!(out, "{only}spbtn {},{},2", menu_slot(*i), n + 1).unwrap();
    }
    writeln!(out, "btnwait {BUTTON_VAR}").unwrap();
    if e.hover_se.is_some() {
        writeln!(out, "selectvoice \"\",{},\"\"", crate::MENU_HOVER).unwrap();
    }
    writeln!(out, "if {BUTTON_VAR} == -1 goto *{BACK}").unwrap();
    for (n, (_, item)) in clickable.iter().enumerate() {
        let se = item.click_se.as_ref().map_or(String::new(), |se| format!("dwave {},\"{}\":", SE_CHANNELS.start(), path(se)));
        writeln!(out, "if {BUTTON_VAR} == {} {se}goto *{}", n + 1, mode_label(item.mode)).unwrap();
    }
    writeln!(out, "goto *{WAIT}").unwrap();
    writeln!(out, "*{BACK}").unwrap();
    // A channel of its own: the title's opening sound comes at once.
    if let Some(se) = &e.back_se {
        writeln!(out, "dwave {},\"{}\"", SE_CHANNELS.start() + 1, path(se)).unwrap();
    }
    if let Some(r) = &e.sound {
        writeln!(out, "{}", crate::picture_sound::back_to_title(r)).unwrap();
    }
    writeln!(out, "mov {TITLE_SHOWN},2").unwrap();
    writeln!(out, "csp -1").unwrap();
    writeln!(out, "goto *{TITLE_BACK}").unwrap();
    if let Some(g) = &e.gallery {
        crate::picture_gallery::emit(out, g, border, width, height);
    }
    if let Some(r) = &e.sound {
        crate::picture_sound::emit(out, r, border, width, height);
    }
    if let Some(r) = &e.replay {
        let music = e.sound.as_ref().map(|_| format!("gosub *{}", crate::picture_sound::PLAY));
        crate::picture_replay::emit(out, r, border, music.as_deref());
    }
    if let Some(t) = &e.turntable {
        crate::picture_turntable::emit(out, t, border);
    }
}
