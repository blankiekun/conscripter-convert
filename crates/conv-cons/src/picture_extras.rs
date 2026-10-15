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

use crate::{path, var_name, BUTTON_VAR, MAX_SPRITE, MENU_SPRITES, SE_CHANNELS, TITLE_SHOWN};

pub(crate) const EXTRAS: &str = "conv_pextras";
const DRAW: &str = "conv_pextras_draw";
const WAIT: &str = "conv_pextras_wait";
const BACK: &str = "conv_pextras_back";
/// The title's label that draws it again (its picture, then its openings).
pub(crate) const TITLE_BACK: &str = crate::extras::TITLE_BACK;
const BACKGROUND: u32 = MAX_SPRITE;
const ITEM: u32 = MENU_SPRITES.start; // + item

/// Whether `mode` has a screen in this script.
fn has_mode(_e: &PictureExtras, mode: ExtrasMode) -> bool {
    mode == ExtrasMode::Back
}

/// Label of a mode's screen.
fn mode_label(mode: ExtrasMode) -> String {
    match mode {
        ExtrasMode::Back => BACK.to_string(),
        other => format!("conv_pextras_{}", format!("{other:?}").to_lowercase()),
    }
}

pub(crate) fn emit(out: &mut String, e: &PictureExtras, border: u32) {
    assert!(e.items.len() <= MENU_SPRITES.len(), "more extras buttons than menu sprites");
    let fade = if e.fade_ms > 0 { format!("print 10,{}", e.fade_ms) } else { "print 1".to_string() };
    writeln!(out, "*{EXTRAS}").unwrap();
    writeln!(out, "*{DRAW}").unwrap();
    writeln!(out, "csp -1").unwrap();
    writeln!(out, "lsp {BACKGROUND},\":c;{}\",0,0", path(&e.background)).unwrap();
    for (i, item) in e.items.iter().enumerate() {
        let b = &item.button;
        let slot = ITEM + i as u32;
        let lsp = format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&b.image), b.x, b.y);
        match item.unlocked_by {
            Some(v) => {
                let v = var_name(v, border);
                writeln!(out, "if {v} == 0 {lsp},{}", item.locked_alpha).unwrap();
                writeln!(out, "if {v} != 0 {lsp}").unwrap();
            }
            None => writeln!(out, "{lsp}").unwrap(),
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
        writeln!(out, "{only}spbtn {},{},2", ITEM + *i as u32, n + 1).unwrap();
    }
    writeln!(out, "btnwait {BUTTON_VAR}").unwrap();
    if e.hover_se.is_some() {
        writeln!(out, "selectvoice \"\",\"\",\"\"").unwrap();
    }
    writeln!(out, "if {BUTTON_VAR} == -1 goto *{BACK}").unwrap();
    for (n, (_, item)) in clickable.iter().enumerate() {
        let se = item.click_se.as_ref().map_or(String::new(), |se| format!("dwave {},\"{}\":", SE_CHANNELS.start(), path(se)));
        writeln!(out, "if {BUTTON_VAR} == {} {se}goto *{}", n + 1, mode_label(item.mode)).unwrap();
    }
    writeln!(out, "goto *{WAIT}").unwrap();
    writeln!(out, "*{BACK}").unwrap();
    if let Some(se) = &e.back_se {
        writeln!(out, "dwave {},\"{}\"", SE_CHANNELS.start(), path(se)).unwrap();
    }
    writeln!(out, "mov {TITLE_SHOWN},2").unwrap();
    writeln!(out, "csp -1").unwrap();
    writeln!(out, "goto *{TITLE_BACK}").unwrap();
}
