//! [`PictureGallery`] as NScripter code, a mode of
//! [`picture_extras`](crate::picture_extras) entered at [`GALLERY`].
//!
//! Each page of each category is a generated label that draws the whole
//! screen ([`DRAW`] picks it with `tablegoto`): a slot's thumbnail is its
//! first CG seen, written as `if` lines from the last CG to the first; the
//! cursor over it is a two-cell `spbtn` sprite (nothing, the cursor), put
//! up only while one of the slot's CGs has been seen. Any click redraws the
//! page (`btnwait` drops its buttons); the screen doesn't change, so
//! nothing flickers.
//!
//! The viewer is a generated label per slot that shows its CGs in turn on
//! a black screen. The click waits are one full-screen `btn` ([`CLICK`]); a
//! picture larger than the screen scrolls in [`SCROLL`], a loop of `amsp`
//! and `print 1` timed by `gettimer`, which a timed `btnwait` (`btntime`)
//! lets clicks pause and a right-click end.

use conv_core::ir::{PictureGallery, Placed};

use crate::menu::TMP;
use crate::{path, var_name, BUTTON_VAR, MAX_SPRITE, MENU_SPRITES};

/// Entered from the extras' menu, which [`MENU`] draws again.
pub(crate) const GALLERY: &str = "conv_pextras_gallery";
const MENU: &str = "conv_pextras_draw";
const DRAW: &str = "conv_pg_draw";
/// Back from the viewer to the page it was opened on.
const CLOSE: &str = "conv_pg_close";
const SCROLL: &str = "conv_pg_scroll";
const CLICK: &str = "conv_pg_click";
// Variables: the shown category and page (from 0), the next print's
// cross-fade (0: none), 1 while the CG before was unseen, and a scroll's
// two ends, length and time gone. Only at the title (the extras' own).
const CATEGORY: &str = "%130";
const PAGE: &str = "%131";
const FADE: &str = "%132";
const PREVIOUS_UNSEEN: &str = "%133";
const FROM: [&str; 2] = ["%134", "%135"];
const TO: [&str; 2] = ["%136", "%137"];
const DURATION: &str = "%138";
const ELAPSED: &str = "%139";
// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const THUMB: u32 = 900; // + slot
const CURSOR: u32 = 860; // + slot
const VIEW: u32 = 850;
const MAX_SLOTS: u32 = 40;
// Buttons in the menus' range.
const CATEGORY_BUTTON: u32 = MENU_SPRITES.start; // + category
const MAX_CATEGORIES: u32 = 8;
const PREVIOUS: u32 = MENU_SPRITES.start + MAX_CATEGORIES;
const NEXT: u32 = PREVIOUS + 1;
const EXIT: u32 = PREVIOUS + 2;
const BTN_CATEGORY: u32 = 100; // + category
const BTN_PREVIOUS: u32 = 201;
const BTN_NEXT: u32 = 202;
const BTN_EXIT: u32 = 203;

fn button(slot: u32, p: &Placed) -> String {
    format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&p.image), p.x, p.y)
}

pub(crate) fn emit(out: &mut String, g: &PictureGallery, border: u32, width: u32, height: u32) {
    let per_page = g.rows * g.columns;
    assert!(per_page <= MAX_SLOTS && g.categories.len() as u32 <= MAX_CATEGORIES, "a picture gallery larger than its sprites");
    let pages: Vec<usize> = g.categories.iter().map(|c| c.slots.len().div_ceil(per_page as usize).max(1)).collect();
    let max_pages = pages.iter().copied().max().unwrap_or(1);
    let place = |i: u32| {
        let (column, row) = if g.across { (i % g.columns, i / g.columns) } else { (i / g.rows, i % g.rows) };
        (g.first.0 + g.column_step * column as i32, g.first.1 + g.row_step * row as i32)
    };
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let prints: Vec<u32> = {
        let mut p = vec![g.open_ms, g.page_ms, g.close_ms];
        p.retain(|&ms| ms > 0);
        p.sort_unstable();
        p.dedup();
        p
    };
    w(format!("*{GALLERY}"));
    w(format!("mov {CATEGORY},0:mov {PAGE},0:mov {FADE},{}", g.open_ms));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("lsp {BACKGROUND},\":c;{}\",0,0", path(&g.background)));
    for (c, category) in g.categories.iter().enumerate() {
        let slot = CATEGORY_BUTTON + c as u32;
        w(button(slot, &category.button));
        // The open category shows its pressed picture, and isn't a button.
        w(format!("if {CATEGORY} == {c} cell {slot},2"));
        w(format!("if {CATEGORY} != {c} spbtn {slot},{}", BTN_CATEGORY + c as u32));
    }
    // Only in a category of more pages, if so asked.
    let paged: Vec<String> = pages.iter().enumerate().filter(|(_, n)| **n > 1).map(|(c, _)| format!("if {CATEGORY} == {c} ")).collect();
    for (slot, b, n) in [(PREVIOUS, &g.previous, BTN_PREVIOUS), (NEXT, &g.next, BTN_NEXT), (EXIT, &g.exit, BTN_EXIT)] {
        let only = if g.arrows_when_paged && slot != EXIT { paged.clone() } else { vec![String::new()] };
        for only in only {
            w(format!("{only}{}", button(slot, b)));
            w(format!("{only}spbtn {slot},{n},2"));
        }
    }
    let counts = &pages;
    let targets: Vec<String> = (0..g.categories.len())
        .flat_map(|c| (0..max_pages).map(move |p| format!("*conv_pg_c{c}_p{}", if p < counts[c] { p } else { 0 })))
        .collect();
    w(format!("mov {TMP},{CATEGORY}*{max_pages}+{PAGE}"));
    w(format!("tablegoto {TMP},{}", targets.join(",")));
    // Shows the page drawn, then waits for a button.
    w("*conv_pg_show".into());
    for ms in &prints {
        w(format!("if {FADE} == {ms} print 10,{ms}"));
    }
    w(format!("if {FADE} == 0 print 1"));
    w(format!("mov {FADE},0"));
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} goto *{MENU}"));
    w(format!("if {BUTTON_VAR} == {BTN_PREVIOUS} mov {FADE},{}:dec {PAGE}", g.page_ms));
    w(format!("if {BUTTON_VAR} == {BTN_NEXT} mov {FADE},{}:inc {PAGE}", g.page_ms));
    for (c, n) in pages.iter().enumerate() {
        w(format!("if {CATEGORY} == {c} && {PAGE} < 0 mov {PAGE},{}", n - 1));
        w(format!("if {CATEGORY} == {c} && {PAGE} >= {n} mov {PAGE},0"));
    }
    w(format!(
        "if {BUTTON_VAR} >= {BTN_CATEGORY} && {BUTTON_VAR} < {} mov {CATEGORY},{BUTTON_VAR}-{BTN_CATEGORY}:mov {PAGE},0:mov {FADE},{}",
        BTN_CATEGORY + MAX_CATEGORIES,
        g.page_ms
    ));
    // A slot: the page's own label goes on to its viewer.
    w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {per_page} mov {TMP},{CATEGORY}*{max_pages}+{PAGE}:tablegoto {TMP},{}", targets.iter().map(|t| format!("{t}_open")).collect::<Vec<_>>().join(",")));
    w(format!("goto *{DRAW}"));

    for (c, category) in g.categories.iter().enumerate() {
        for (p, slots) in category.slots.chunks(per_page as usize).enumerate() {
            w(format!("*conv_pg_c{c}_p{p}"));
            for (i, slot) in slots.iter().enumerate() {
                let (x, y) = place(i as u32);
                let n = slot.cgs.len();
                w(format!("mov {TMP},0"));
                for cg in &slot.cgs {
                    w(format!("if {} != 0 inc {TMP}", var_name(cg.seen, border)));
                }
                w(format!("lsp {},\":c;{}\",{x},{y}", THUMB + i as u32, path(&g.blank)));
                for cg in slot.cgs.iter().rev() {
                    w(format!("if {} != 0 lsp {},\":c;{}\",{x},{y}", var_name(cg.seen, border), THUMB + i as u32, path(&cg.thumbnail)));
                }
                w(format!("if {TMP} > 0 && {TMP} < {n} amsp {},{x},{y},{}", THUMB + i as u32, g.partial_alpha));
                w(format!("if {TMP} > 0 lsp {},\":a/2,0,3;{}\",{x},{y}:spbtn {},{}", CURSOR + i as u32, path(&g.cursor), CURSOR + i as u32, i + 1));
            }
            w("goto *conv_pg_show".into());
            w(format!("*conv_pg_c{c}_p{p}_open"));
            let first = p * per_page as usize;
            let opens: Vec<String> = (0..slots.len()).map(|i| format!("*conv_pg_v{c}_{}", first + i)).collect();
            w(format!("mov {TMP},{BUTTON_VAR}-1:tablegoto {TMP},{}", opens.join(",")));
        }
    }

    // The viewers.
    for (c, category) in g.categories.iter().enumerate() {
        for (s, slot) in category.slots.iter().enumerate() {
            let label = format!("conv_pg_v{c}_{s}");
            w(format!("*{label}"));
            w(format!("csp -1:bg black,1:mov {PREVIOUS_UNSEEN},0"));
            for (k, cg) in slot.cgs.iter().enumerate() {
                let seen = var_name(cg.seen, border);
                let p = &cg.picture;
                let (over_x, over_y) = ((p.w - width as i32).max(0), (p.h - height as i32).max(0));
                // The far end (bottom or right) and the top left.
                let far = (-over_x, -over_y);
                let (mut start, mut end) = if slot.reversed { (far, (0, 0)) } else { ((0, 0), far) };
                // Along an axis that doesn't scroll, the picture's own place.
                if over_x == 0 {
                    (start.0, end.0) = (p.x, p.x);
                }
                if over_y == 0 {
                    (start.1, end.1) = (p.y, p.y);
                }
                w(format!("if {seen} == 0 goto *{label}_{k}u"));
                w(format!("lsp {VIEW},\":c;{}\",{},{}:print 10,{}", path(&p.image), start.0, start.1, g.view_fade_ms));
                w(format!("mov {PREVIOUS_UNSEEN},0:gosub *{CLICK}"));
                w(format!("if {BUTTON_VAR} == -1 goto *{CLOSE}"));
                if over_x > 0 || over_y > 0 {
                    let ms = (over_x.max(over_y) as u32) * g.scroll_ms_per_px;
                    w(format!("mov {},{}:mov {},{}:mov {},{}:mov {},{}:mov {DURATION},{ms}", FROM[0], start.0, FROM[1], start.1, TO[0], end.0, TO[1], end.1));
                    w(format!("gosub *{SCROLL}"));
                    w(format!("if {BUTTON_VAR} == -1 goto *{CLOSE}"));
                }
                w(format!("goto *{label}_{k}n"));
                w(format!("*{label}_{k}u"));
                w(format!("if {PREVIOUS_UNSEEN} == 1 csp {VIEW}:print 10,150"));
                w(format!("lsp {VIEW},\":c;{}\",0,0:print 10,{}", path(&g.unseen), g.view_fade_ms));
                w(format!("mov {PREVIOUS_UNSEEN},1:gosub *{CLICK}"));
                w(format!("if {BUTTON_VAR} == -1 goto *{CLOSE}"));
                w(format!("*{label}_{k}n"));
            }
            w(format!("goto *{CLOSE}"));
        }
    }
    w(format!("*{CLOSE}"));
    w(format!("mov {FADE},{}:goto *{DRAW}", g.close_ms));
    // A click anywhere (1) or a right-click (-1).
    w(format!("*{CLICK}"));
    w(format!("btndef clear:btn 1,0,0,{width},{height},0,0"));
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} != -1 && {BUTTON_VAR} < 1 goto *{CLICK}"));
    w("return".into());
    // Scrolls VIEW from FROM to TO over DURATION ms; a click pauses and
    // the next goes on, a right-click jumps to the end. Then waits at the
    // end for a click ([`CLICK`]'s result).
    w(format!("*{SCROLL}"));
    w(format!("mov {ELAPSED},0"));
    w(format!("*{SCROLL}_go"));
    w(format!("resettimer:btndef clear:btn 1,0,0,{width},{height},0,0"));
    w(format!("*{SCROLL}_loop"));
    w(format!("gettimer {TMP}:add {TMP},{ELAPSED}"));
    w(format!("if {TMP} >= {DURATION} goto *{SCROLL}_end"));
    w(format!(
        "amsp {VIEW},{}+({}-{})*{TMP}/{DURATION},{}+({}-{})*{TMP}/{DURATION}",
        FROM[0], TO[0], FROM[0], FROM[1], TO[1], FROM[1]
    ));
    w("print 1".into());
    w(format!("btntime 16:btnwait {BUTTON_VAR}:btntime -1"));
    w(format!("if {BUTTON_VAR} == -1 goto *{SCROLL}_end"));
    // The time ran out (-2, or -5 with `usewheel`), the wheel, a click on
    // nothing.
    w(format!("if {BUTTON_VAR} != -1 && {BUTTON_VAR} < 1 goto *{SCROLL}_loop"));
    w(format!("mov {ELAPSED},{TMP}"));
    w(format!("gosub *{CLICK}"));
    w(format!("if {BUTTON_VAR} == -1 goto *{SCROLL}_end"));
    w(format!("goto *{SCROLL}_go"));
    w(format!("*{SCROLL}_end"));
    w(format!("amsp {VIEW},{},{}:print 1", TO[0], TO[1]));
    w(format!("gosub *{CLICK}"));
    w("return".into());
}
