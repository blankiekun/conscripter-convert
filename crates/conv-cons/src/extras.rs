//! The extras menu the title's option button opens, and the CG gallery
//! (see [`ExtrasMenu`]; the scene replay is in [`crate::replay`], the BGM
//! room in [`crate::music`]). All are only reached from the title, so they
//! use the game's sprite slots ([`SPRITES`]) and clear everything with
//! `csp -1`.
//!
//! The gallery redraws the whole screen after every click (ONScripter drops
//! the buttons when `btnwait` returns); the screen doesn't change, so
//! nothing flickers. A page is a generated label reached by `tablegoto`
//! on tab × [`MAX_PAGES`] + page; a slot's thumbnail shows once one of its
//! CGs is seen ([`crate::CG_SEEN_BASE`] + number). The viewer is a label
//! per slot and per variant, stepping past unseen variants.

use conv_core::ir::{CgGallery, ExtrasAction, ExtrasMenu, Placed};

use crate::{menu, music, path, replay, BUTTON_VAR, CG_SEEN_BASE};

pub(crate) const EXTRAS: &str = "conv_extras";
/// Back to the title screen (its background and buttons).
pub(crate) const TITLE_BACK: &str = "conv_title_back";
const DRAW: &str = "conv_extras_draw";
const GALLERY: &str = "conv_cg";
const GALLERY_DRAW: &str = "conv_cg_draw";
const GALLERY_WAIT: &str = "conv_cg_wait";
const VIEW: &str = "conv_cg_view";
const VIEW_SHOW: &str = "conv_cg_view_show";
/// Sprite slots (the game's, unused at the title).
const SPRITES: u32 = 100;
const MENU_ITEM: u32 = SPRITES; // + item
const HEADER: u32 = SPRITES;
const EXIT: u32 = SPRITES + 1;
const TAB: u32 = SPRITES + 2; // + tab
const TAB_TITLE: u32 = SPRITES + 12;
const PAGE_LABEL: u32 = SPRITES + 13;
const PAGE_MARKER: u32 = SPRITES + 14;
const PAGE_NUMBER: u32 = SPRITES + 15; // + page
const SLOT: u32 = SPRITES + 30; // + slot on the page
const PANEL: u32 = SPRITES + 60; // + panel picture
const NAME: u32 = SPRITES + 55;
const PREVIOUS: u32 = SPRITES + 56;
const NEXT: u32 = SPRITES + 57;
const HIDE: u32 = SPRITES + 58;
const BACK: u32 = SPRITES + 59;
const MAX_TABS: usize = 10;
const MAX_PAGES: usize = 4;
const MAX_SLOTS: usize = 25;
// Buttons.
const BTN_EXIT: i32 = 99;
const BTN_TAB: i32 = 1; // + tab
const BTN_PAGE: i32 = 20; // + page
const BTN_SLOT: i32 = 100; // + slot
const BTN_PREVIOUS: i32 = 1;
const BTN_NEXT: i32 = 2;
const BTN_HIDE: i32 = 3;
const BTN_BACK: i32 = 4;
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
    format!("%{}", CG_SEEN_BASE + number)
}

/// `fade`: the effect the title fades in with. `settings`: there is a
/// settings screen for the option heading to open.
pub(crate) fn extras(out: &mut String, m: &ExtrasMenu, fade: &str, settings: bool) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let gallery = m.gallery.as_ref();
    let clear_vars = [TAB_VAR, PAGE_VAR, TMP, SLOT_VAR, VARIANT, STEP, PAGE_FIRST, UNLOCKED, PANEL_HIDDEN]
        .iter()
        .chain(replay::VARS.iter())
        .map(|v| format!("mov {v},0"))
        .collect::<Vec<_>>()
        .join(":");
    w(format!("*{EXTRAS}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(&m.background)));
    w(format!("print {fade}"));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(&m.background)));
    // The gallery is open once one of its CGs has been seen.
    w(format!("mov {UNLOCKED},0"));
    if let Some(g) = gallery {
        let mut numbers: Vec<u32> = g.tabs.iter().flat_map(|t| t.pages.iter().flatten()).flat_map(|s| s.cgs.iter().map(|c| c.0)).collect();
        numbers.sort_unstable();
        numbers.dedup();
        for chunk in numbers.chunks(8) {
            w(chunk.iter().map(|&n| format!("if {} == 1 mov {UNLOCKED},1", seen(n))).collect::<Vec<_>>().join(":"));
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
    if let Some(g) = gallery {
        gallery_routine(out, g, &m.background);
    }
    if let Some(r) = &m.replay {
        replay::replay(out, r, &m.background, DRAW);
    }
    if let Some(music) = &m.music {
        music::music(out, music, &m.background, DRAW);
    }
}

fn gallery_routine(out: &mut String, g: &CgGallery, background: &str) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let tabs = &g.tabs[..g.tabs.len().min(MAX_TABS)];
    w(format!("*{GALLERY}"));
    w(format!("mov {TAB_VAR},0:mov {PAGE_VAR},0"));
    w(format!("*{GALLERY_DRAW}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(background)));
    w(picture(HEADER, &g.header.image, g.header.x, g.header.y));
    w(button(EXIT, &g.exit, 2));
    w(format!("spbtn {EXIT},{BTN_EXIT}"));
    w(picture(PAGE_LABEL, &g.page_label.image, g.page_label.x, g.page_label.y));
    for (i, t) in tabs.iter().enumerate() {
        let slot = TAB + i as u32;
        let (x, y) = (t.button.x, t.button.y);
        w(format!("if {TAB_VAR} == {i} {}:{}", picture(slot, &t.selected, x, y), picture(TAB_TITLE, &t.title.image, t.title.x, t.title.y)));
        w(format!("if {TAB_VAR} != {i} {}:spbtn {slot},{}", button(slot, &t.button, 2), BTN_TAB + i as i32));
        for p in 0..t.pages.len().min(MAX_PAGES).min(g.page_numbers.len()) {
            let (px, py) = (g.pages_x + g.page_dx * p as i32, g.pages_y);
            let number = format!("lsp {},\":a/2,0,3;{}\",{px},{py}", PAGE_NUMBER + p as u32, path(&g.page_numbers[p]));
            w(format!("if {TAB_VAR} == {i} {number}"));
            w(format!(
                "if {TAB_VAR} == {i} && {PAGE_VAR} == {p} {}",
                picture(PAGE_MARKER, &g.page_marker, px + g.marker_dx, py + g.marker_dy)
            ));
            w(format!("if {TAB_VAR} == {i} && {PAGE_VAR} != {p} spbtn {},{}", PAGE_NUMBER + p as u32, BTN_PAGE + p as i32));
        }
    }
    // The page's slots.
    w(format!("mov {TMP},{TAB_VAR}*{MAX_PAGES}+{PAGE_VAR}"));
    let mut targets = Vec::new();
    for (i, t) in tabs.iter().enumerate() {
        for p in 0..MAX_PAGES {
            targets.push(if p < t.pages.len() { format!("*conv_cg_page{i}_{p}") } else { format!("*{GALLERY_WAIT}") });
        }
    }
    w(format!("tablegoto {TMP},{}", targets.join(",")));
    let mut first = 0;
    for (i, t) in tabs.iter().enumerate() {
        for (p, page) in t.pages.iter().enumerate().take(MAX_PAGES) {
            w(format!("*conv_cg_page{i}_{p}"));
            w(format!("mov {PAGE_FIRST},{first}"));
            for (k, s) in page.iter().enumerate().take(MAX_SLOTS) {
                let slot = SLOT + k as u32;
                let x = g.grid_x + g.grid_dx * (k % g.columns) as i32;
                let y = g.grid_y + g.grid_dy * (k / g.columns) as i32;
                let unlock: Vec<String> = s.cgs.iter().map(|&(n, _)| format!("if {} == 1 mov {TMP},1", seen(n))).collect();
                w(format!("mov {TMP},0:{}", unlock.join(":")));
                // The thumbnail strip has its frame 4 px up and left.
                w(format!("if {TMP} == 1 lsp {slot},\":a/2,0,3;{}\",{},{}:spbtn {slot},{}", path(&s.thumbnail), x - 4, y - 4, BTN_SLOT + k as i32));
                w(format!("if {TMP} == 0 {}", picture(slot, &g.locked, x, y)));
            }
            w(format!("goto *{GALLERY_WAIT}"));
            first += page.len().min(MAX_SLOTS);
        }
    }
    w(format!("*{GALLERY_WAIT}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 csp -1:return"));
    w(format!("if {BUTTON_VAR} == {BTN_EXIT} csp -1:return"));
    w(format!("if {BUTTON_VAR} >= {BTN_TAB} && {BUTTON_VAR} < {BTN_PAGE} mov {TAB_VAR},{BUTTON_VAR}-{BTN_TAB}:mov {PAGE_VAR},0"));
    w(format!("if {BUTTON_VAR} >= {BTN_PAGE} && {BUTTON_VAR} < {BTN_EXIT} mov {PAGE_VAR},{BUTTON_VAR}-{BTN_PAGE}"));
    w(format!(
        "if {BUTTON_VAR} >= {BTN_SLOT} mov {SLOT_VAR},{PAGE_FIRST}+{BUTTON_VAR}-{BTN_SLOT}:mov {VARIANT},0:mov {STEP},1:mov {PANEL_HIDDEN},0:goto *{VIEW}"
    ));
    w(format!("goto *{GALLERY_DRAW}"));

    // The viewer.
    let slots: Vec<_> = tabs.iter().flat_map(|t| t.pages.iter().take(MAX_PAGES).flat_map(|p| p.iter().take(MAX_SLOTS))).collect();
    w(format!("*{VIEW}"));
    w(format!("tablegoto {SLOT_VAR},{}", (0..slots.len()).map(|n| format!("*conv_cg_v{n}")).collect::<Vec<_>>().join(",")));
    w(format!("goto *{GALLERY_DRAW}"));
    for (n, s) in slots.iter().enumerate() {
        let count = s.cgs.len();
        w(format!("*conv_cg_v{n}"));
        w(format!("if {VARIANT} < 0 mov {VARIANT},{}", count - 1));
        w(format!("if {VARIANT} >= {count} mov {VARIANT},0"));
        w(format!("tablegoto {VARIANT},{}", (0..count).map(|i| format!("*conv_cg_v{n}_{i}")).collect::<Vec<_>>().join(",")));
        for (i, (number, image)) in s.cgs.iter().enumerate() {
            w(format!("*conv_cg_v{n}_{i}"));
            w(format!("if {} == 0 add {VARIANT},{STEP}:goto *conv_cg_v{n}", seen(*number)));
            w(format!("bg \"{}\",0:goto *{VIEW_SHOW}", path(image)));
        }
    }
    let v = &g.viewer;
    w(format!("*{VIEW_SHOW}"));
    w("csp -1".into());
    w(format!("if {PANEL_HIDDEN} == 1 goto *conv_cg_view_wait"));
    for (k, p) in v.panel.iter().enumerate() {
        w(picture(PANEL + k as u32, &p.image, p.x, p.y));
    }
    for (i, t) in tabs.iter().enumerate() {
        w(format!("if {TAB_VAR} == {i} {}", picture(NAME, &t.viewer_name.image, v.name_x, v.name_y)));
    }
    for (slot, p, n) in [(PREVIOUS, &v.previous, BTN_PREVIOUS), (NEXT, &v.next, BTN_NEXT), (HIDE, &v.hide, BTN_HIDE), (BACK, &v.back, BTN_BACK)] {
        w(format!("{}:spbtn {slot},{n}", button(slot, p, 2)));
    }
    w("*conv_cg_view_wait".into());
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 goto *{GALLERY_DRAW}"));
    w(format!("if {BUTTON_VAR} == {BTN_BACK} goto *{GALLERY_DRAW}"));
    w(format!("if {BUTTON_VAR} == {BTN_PREVIOUS} mov {STEP},-1:add {VARIANT},-1:goto *{VIEW}"));
    w(format!("if {BUTTON_VAR} == {BTN_NEXT} mov {STEP},1:add {VARIANT},1:goto *{VIEW}"));
    w(format!("if {BUTTON_VAR} == {BTN_HIDE} mov {PANEL_HIDDEN},1:goto *{VIEW_SHOW}"));
    // A click shows the hidden panel again.
    w(format!("mov {PANEL_HIDDEN},0:goto *{VIEW_SHOW}"));
}
