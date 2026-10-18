//! [`PictureExtras`] as NScripter code under [`EXTRAS`], reached from the
//! title's [`TitleAction::Extras`](conv_core::ir::TitleAction::Extras)
//! button with the title still on screen (the menu cross-fades in over
//! it). The menu is the engine's (`screen "extras"`, [`lua_file`]); only
//! reached from the title, so it clears everything with `csp -1`. Going back sets [`TITLE_SHOWN`] to 2, which the title reads as
//! "back from the extras": its music goes on, and a
//! [`TitleOpening::after_extras`](conv_core::ir::TitleOpening::after_extras)
//! opening plays.

use std::fmt::Write;

use conv_core::ir::{ExtrasMode, ExtrasPassage, PictureExtras, PictureExtrasItem};

use crate::{path, var_name, BUTTON_VAR, DIALOG_SPRITES, MAX_SPRITE, SE_CHANNELS, TITLE_SHOWN};

pub(crate) const EXTRAS: &str = "conv_pextras";
const DRAW: &str = "conv_pextras_draw";
const BACK: &str = "conv_pextras_back";
/// The menu's buttons acted on (after the screen).
const PICK: &str = "conv_pextras_pick";
/// Through the doors into a mode (a subroutine).
const IN: &str = "conv_pextras_in";
// The passage's sprites (lower numbers drawn on top).
const PASSAGE_BG: u32 = MAX_SPRITE;
const PASSAGE_DOORS: u32 = MAX_SPRITE - 1;
const PASSAGE_BLACK: u32 = DIALOG_SPRITES.start;
/// The title's label that draws it again (its picture, then its openings).
pub(crate) const TITLE_BACK: &str = crate::extras::TITLE_BACK;

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
    writeln!(out, "*{EXTRAS}").unwrap();
    if let Some(r) = &e.sound {
        writeln!(out, "{}", crate::picture_sound::on_open(r)).unwrap();
    }
    if let Some(p) = &e.passage {
        // From the title, then back from a mode through the doors.
        writeln!(out, "csp -1").unwrap();
        screen_call(out, e, "");
        writeln!(out, "goto *{PICK}").unwrap();
        writeln!(out, "*{DRAW}").unwrap();
        passage_back(out, p, width, height);
        screen_call(out, e, ",\"back\"");
        writeln!(out, "*{PICK}").unwrap();
    } else {
        writeln!(out, "*{DRAW}").unwrap();
        writeln!(out, "csp -1").unwrap();
        screen_call(out, e, "");
    }
    writeln!(out, "if {BUTTON_VAR} == 0 goto *{BACK}").unwrap();
    dispatch(out, e);
    writeln!(out, "goto *{DRAW}").unwrap();
    if let Some(p) = &e.passage {
        passage_in(out, p, width, height);
    }
    writeln!(out, "*{BACK}").unwrap();
    back(out, e);
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

/// The buttons that open something (with their value, 1 up).
fn clickable(e: &PictureExtras) -> impl Iterator<Item = (usize, &PictureExtrasItem)> {
    e.items.iter().filter(|item| has_mode(e, item.mode)).enumerate().map(|(n, item)| (n + 1, item))
}

/// What a click on button `n` runs: its sound, the passage, then its mode.
fn dispatch(out: &mut String, e: &PictureExtras) {
    for (n, item) in clickable(e) {
        let se = item.click_se.as_ref().map_or(String::new(), |se| format!("dwave {},\"{}\":", SE_CHANNELS.start(), path(se)));
        let through = if e.passage.is_some() && item.mode != ExtrasMode::Back { format!("gosub *{IN}:") } else { String::new() };
        writeln!(out, "if {BUTTON_VAR} == {n} {se}{through}goto *{}", mode_label(item.mode)).unwrap();
    }
}

/// The menu as `screens/extras.lua` (`arg`: the command's third argument,
/// with its comma); a right-click closes it with 0.
fn screen_call(out: &mut String, e: &PictureExtras, arg: &str) {
    if let Some(se) = &e.hover_se {
        writeln!(out, "selectvoice \"\",\"{}\",\"\"", path(se)).unwrap();
    }
    writeln!(out, "screen \"extras\",{BUTTON_VAR}{arg}").unwrap();
    if e.hover_se.is_some() {
        writeln!(out, "selectvoice \"\",{},\"\"", crate::MENU_HOVER).unwrap();
    }
}

/// The background and the doors at `cell` over it, and black over them
/// (`width` × `height`) when `black`.
fn passage_draw(out: &mut String, p: &ExtrasPassage, cell: u32, black: Option<(u32, u32)>) {
    let d = &p.doors;
    writeln!(out, "csp -1:lsp {PASSAGE_BG},\"{}\",0,0", pic(&p.background)).unwrap();
    writeln!(out, "lsp {PASSAGE_DOORS},\":a/{},0,3;{}\",{},{}:cell {PASSAGE_DOORS},{cell}", p.cells, path(&d.image), d.x, d.y).unwrap();
    if let Some((w, h)) = black {
        writeln!(out, "lsp {PASSAGE_BLACK},\":c;>{w},{h},#000000\",0,0").unwrap();
    }
}

/// `spmotion` keys of a zoom from `from` to `to` percent over `ms` along
/// `curve`, at full opacity.
fn zoom_keys(from: u32, to: u32, ms: u32, curve: u32) -> String {
    format!("0,0,0,{from},{from},0,256,0,{ms},0,0,{to},{to},0,256,{curve}")
}

/// [`IN`]: from the menu (held on the display) through the doors to black.
fn passage_in(out: &mut String, p: &ExtrasPassage, width: u32, height: u32) {
    let last = p.cells.saturating_sub(1);
    writeln!(out, "*{IN}").unwrap();
    passage_draw(out, p, 0, None);
    writeln!(out, "print 10,{}", p.buttons_ms).unwrap();
    if let Some(se) = &p.open_se {
        writeln!(out, "dwave {},\"{}\"", SE_CHANNELS.start(), path(se)).unwrap();
    }
    writeln!(out, "wait {}", p.step_ms).unwrap();
    for cell in 1..=last {
        writeln!(out, "cell {PASSAGE_DOORS},{cell}:print 10,{}", p.step_ms).unwrap();
    }
    writeln!(out, "wait {}", p.hold_ms.saturating_sub(p.step_ms)).unwrap();
    let keys = zoom_keys(100, p.zoom, p.zoom_ms, 1);
    writeln!(out, "spmotion {PASSAGE_BG},2,{keys}:spmotion {PASSAGE_DOORS},2,{keys}").unwrap();
    writeln!(out, "lsp {PASSAGE_BLACK},\":c;>{width},{height},#000000\",0,0").unwrap();
    writeln!(out, "spmotion {PASSAGE_BLACK},2,0,0,0,100,100,0,0,0,{},0,0,100,100,0,256,0", p.zoom_ms).unwrap();
    writeln!(out, "spmotionwait -1").unwrap();
    writeln!(out, "return").unwrap();
}

/// Back from a mode through the doors to the menu without its buttons
/// (the screen fades them in).
fn passage_back(out: &mut String, p: &ExtrasPassage, width: u32, height: u32) {
    passage_draw(out, p, p.cells.saturating_sub(1), Some((width, height)));
    writeln!(out, "print 10,{}", p.out_ms).unwrap();
    let keys = zoom_keys(p.zoom, 100, p.back_ms, 2);
    writeln!(out, "spmotion {PASSAGE_BG},0,{keys}:spmotion {PASSAGE_DOORS},0,{keys}").unwrap();
    writeln!(out, "spmotion {PASSAGE_BLACK},3,0,0,0,100,100,0,256,0,{},0,0,100,100,0,0,0", p.back_ms).unwrap();
    writeln!(out, "spmotionwait -1").unwrap();
    if let Some(se) = &p.close_se {
        writeln!(out, "dwave {},\"{}\"", SE_CHANNELS.start(), path(se)).unwrap();
    }
    writeln!(out, "cell {PASSAGE_DOORS},0:print 1").unwrap();
}

/// `screens/extras.lua`. Every close holds the menu on the display, so the
/// mode (or the title) cross-fades straight from it, as the routine's
/// menu did.
pub(crate) fn lua_file(e: &PictureExtras, border: u32) -> String {
    use crate::screens::lua;
    let mut out = String::from("-- The extras' menu (conscripter-convert, from PictureExtras).\nreturn {\n  kind = \"extras\",\n  cancel = {\"close\", 0, hold=true},\n");
    // Back through the doors: the menu is drawn already, its buttons fade in.
    let back = "arg == \"back\"";
    if e.passage.is_some() {
        writeln!(out, "  fade = {back} and 0 or {},", e.fade_ms).unwrap();
    } else if e.fade_ms > 0 {
        writeln!(out, "  fade = {},", e.fade_ms).unwrap();
    }
    let open = e.passage.as_ref().map_or(String::new(), |p| format!(", open={back} and {{ms={}, alpha=0}} or false", p.buttons_ms));
    let opaque = if crate::sprite_mode(&e.background) == 'c' { ", opaque=true" } else { "" };
    writeln!(out, "  widgets = {{\n    {{\"image\", file={}{opaque}, x=0, y=0}},", lua(&e.background)).unwrap();
    let values: Vec<usize> = e.items.iter().map(|item| clickable(e).find(|(_, c)| std::ptr::eq(*c, item)).map_or(0, |(n, _)| n)).collect();
    for (item, n) in e.items.iter().zip(values) {
        let b = &item.button;
        // Locked, or a mode the script lacks (a build without the movies): faint and not a
        // button, so it doesn't look like one that does nothing.
        let state = match (n, item.unlocked_by) {
            (0, _) => format!(", enabled=false, alpha={}", item.locked_alpha),
            (_, Some(v)) => {
                let v = lua(&var_name(v, border));
                format!(", enabled=var({v}) ~= 0, alpha=var({v}) ~= 0 and 255 or {}", item.locked_alpha)
            }
            (_, None) => String::new(),
        };
        writeln!(out, "    {{\"button\", file={}, x={}, y={}, cells=3, pressed=2{state}{open}, action={{\"close\", {n}, hold=true}}}},", lua(&b.image), b.x, b.y).unwrap();
    }
    out.push_str("  },\n}\n");
    out
}

/// Back to the title.
fn back(out: &mut String, e: &PictureExtras) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{Placed, Var};

    #[test]
    fn menu_becomes_a_screen() {
        let item = |mode, unlocked_by| PictureExtrasItem { button: Placed { image: "sys/b.png".into(), x: 10, y: 20, w: 30, h: 40 }, mode, unlocked_by, locked_alpha: 51, click_se: None };
        let e = PictureExtras {
            background: "sys/menu.jpg".into(),
            items: vec![item(ExtrasMode::Gallery, None), item(ExtrasMode::Sound, Some(Var { global: true, index: 7 })), item(ExtrasMode::Back, None)],
            hover_se: None,
            back_se: None,
            fade_ms: 500,
            gallery: None,
            sound: None,
            replay: None,
            turntable: None,
            passage: None,
        };
        let lua = lua_file(&e, 4000);
        for part in [
            "cancel = {\"close\", 0, hold=true},\n  fade = 500,",
            "{\"image\", file=\"sys/menu.jpg\", opaque=true, x=0, y=0},",
            // No gallery or sound room in this script: drawn faint, not buttons.
            "cells=3, pressed=2, enabled=false, alpha=51, action={\"close\", 0, hold=true}}",
            // Back is the only mode here: value 1.
            "cells=3, pressed=2, action={\"close\", 1, hold=true}}",
        ] {
            assert!(lua.contains(part), "missing {part:?} in\n{lua}");
        }
        let mut out = String::new();
        emit(&mut out, &e, 4000, 800, 600);
        assert!(out.contains("screen \"extras\",%9\nif %9 == 0 goto *conv_pextras_back\nif %9 == 1 goto *conv_pextras_back\n"), "{out}");

        // Through doors: the first open, then back from a mode with "back".
        let mut e = e;
        e.items.push(item(ExtrasMode::Gallery, None));
        e.gallery = None;
        e.passage = Some(ExtrasPassage {
            background: "sys/bg.jpg".into(),
            doors: Placed { image: "sys/doors.png".into(), x: 89, y: 58, w: 636, h: 478 },
            cells: 3,
            buttons_ms: 200,
            step_ms: 30,
            hold_ms: 100,
            zoom: 200,
            zoom_ms: 500,
            out_ms: 300,
            back_ms: 200,
            open_se: Some("se/open.ogg".into()),
            close_se: None,
        });
        let mut out = String::new();
        emit(&mut out, &e, 4000, 800, 600);
        for part in [
            "csp -1\nscreen \"extras\",%9\ngoto *conv_pextras_pick\n*conv_pextras_draw\ncsp -1:lsp 999,",
            "cell 998,2\nlsp 0,\":c;>800,600,#000000\",0,0\nprint 10,300\nspmotion 999,0,0,0,0,200,200,0,256,0,200,0,0,100,100,0,256,2:",
            "cell 998,0:print 1\nscreen \"extras\",%9,\"back\"\n*conv_pextras_pick\n",
            "*conv_pextras_in\ncsp -1:lsp 999,\":c;sys\\bg.jpg\",0,0\nlsp 998,\":a/3,0,3;sys\\doors.png\",89,58:cell 998,0\nprint 10,200\ndwave 1,\"se\\open.ogg\"\nwait 30\ncell 998,1:print 10,30\ncell 998,2:print 10,30\nwait 70\n",
        ] {
            assert!(out.contains(part), "missing {part:?} in\n{out}");
        }
        let lua = lua_file(&e, 4000);
        assert!(lua.contains("fade = arg == \"back\" and 0 or 500,") && lua.contains("open=arg == \"back\" and {ms=200, alpha=0} or false"), "{lua}");
    }
}
