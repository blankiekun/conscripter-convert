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

use conv_core::ir::{PictureGallery, Placed, Seen};

use crate::menu::TMP;
use crate::picture_extras::pic;
use crate::{path, var_name, BUTTON_VAR, DIALOG_SPRITES, MAX_SPRITE, MENU_SPRITES, SE_CHANNELS};

/// Entered from the extras' menu, which [`MENU`] draws again.
pub(crate) const GALLERY: &str = "conv_pextras_gallery";
pub(crate) const MENU: &str = "conv_pextras_draw";
pub(crate) const DRAW: &str = "conv_pg_draw";
/// Back from the viewer to the page it was opened on.
const CLOSE: &str = "conv_pg_close";
const SCROLL: &str = "conv_pg_scroll";
const CLICK: &str = "conv_pg_click";
// Variables: the shown category and page (from 0), the next print's
// cross-fade (0: none), 1 while the CG before was unseen, and a scroll's
// two ends, length and time gone. Only at the title (the extras' own).
const CATEGORY: &str = "%130";
const PAGE: &str = "%131";
/// The screen's page (1 up), in place of [`PAGE`] when the engine draws it.
pub(crate) const SCREEN_PAGE: &str = PAGE;
pub(crate) const FADE: &str = "%132";
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
/// A category's hover animation, over the buttons.
const HOVER: u32 = DIALOG_SPRITES.start; // + category
const BTN_PREVIOUS: u32 = 201;
const BTN_NEXT: u32 = 202;
const BTN_EXIT: u32 = 203;
/// A compact gallery's slot buttons: 1000 × (category + 1) + slot.
const COMPACT_BUTTON: u32 = 1000;
const CELL: &str = "conv_pg_cell";
// A compact page's count of slots listed, its first and last (from 0, up
// to), the slot drawn (its button), its CGs, its place on the page and
// where, its thumbnail.
const SHOWN: &str = "%conv_pg_shown";
const FIRST: &str = "%conv_pg_first";
const LAST: &str = "%conv_pg_last";
const SLOT: &str = "%conv_pg_slot";
const CGS: &str = "%conv_pg_cgs";
const AT: &str = "%conv_pg_at";
const X: &str = "%conv_pg_x";
const Y: &str = "%conv_pg_y";
const THUMBNAIL: &str = "$conv_pg_thumb";

fn button(slot: u32, p: &Placed) -> String {
    format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&p.image), p.x, p.y)
}

pub(crate) fn emit(out: &mut String, g: &PictureGallery, border: u32, width: u32, height: u32) {
    if g.engine_screen {
        crate::gallery_screen::labels(out, g);
        viewers(out, g, border, width, height);
        return;
    }
    let per_page = g.rows * g.columns;
    assert!(per_page <= MAX_SLOTS && g.categories.len() as u32 <= MAX_CATEGORIES, "a picture gallery larger than its sprites");
    assert!(g.compact || g.categories.iter().flat_map(|c| &c.slots).all(|s| s.shown_if.is_empty()), "a slot's condition in a gallery that lists every slot");
    assert!(!g.compact || g.categories.iter().all(|c| c.slots.len() < COMPACT_BUTTON as usize), "a compact gallery's slots past its buttons");
    // A compact gallery's category is one label drawing the page asked for.
    let pages: Vec<usize> = g.categories.iter().map(|c| if g.compact { 1 } else { c.slots.len().div_ceil(per_page as usize).max(1) }).collect();
    let max_pages = pages.iter().copied().max().unwrap_or(1);
    let place = |i: u32| {
        let (column, row) = if g.across { (i % g.columns, i / g.columns) } else { (i / g.rows, i % g.rows) };
        (g.first.0 + g.column_step * column as i32, g.first.1 + g.row_step * row as i32)
    };
    let mut w = lines!(out);
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
    w(format!("lsp {BACKGROUND},\"{}\",0,0", pic(&g.background)));
    // The hover animations, hidden while the pointer is on no category
    // (`P` shows one from its first cell).
    let hover = g.category_hover.as_ref().filter(|h| !h.places.is_empty());
    if let Some(h) = hover {
        assert!(h.places.len() <= DIALOG_SPRITES.len(), "more category hovers than sprites");
        for (c, (x, y)) in h.places.iter().enumerate() {
            w(format!("lsp {},\":a/{},{},0;{}\",{x},{y}:vsp {},0", HOVER + c as u32, h.cells, h.ms, path(&h.image), HOVER + c as u32));
        }
        w(format!("exbtn_d \"C{}-{}\"", HOVER, HOVER + h.places.len() as u32 - 1));
    }
    for (c, category) in g.categories.iter().enumerate() {
        let slot = CATEGORY_BUTTON + c as u32;
        w(button(slot, &category.button));
        // The open category shows its pressed picture, and isn't a button.
        w(format!("if {CATEGORY} == {c} cell {slot},2"));
        match hover {
            Some(h) if c < h.places.len() => w(format!("if {CATEGORY} != {c} exbtn {slot},{},\"P{}\"", BTN_CATEGORY + c as u32, HOVER + c as u32)),
            _ => w(format!("if {CATEGORY} != {c} spbtn {slot},{}", BTN_CATEGORY + c as u32)),
        }
    }
    // Only in a category of more pages, if so asked.
    let paged: Vec<String> = pages.iter().enumerate().filter(|(_, n)| **n > 1).map(|(c, _)| format!("if {CATEGORY} == {c} ")).collect();
    for (slot, b, n) in [(PREVIOUS, &g.previous, BTN_PREVIOUS), (NEXT, &g.next, BTN_NEXT), (EXIT, &g.exit, BTN_EXIT)] {
        if g.compact && slot != EXIT {
            continue; // drawn by the page, which counts the slots
        }
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
    let page = if g.compact { String::new() } else { format!("+{PAGE}") }; // a compact category's label draws any page
    w(format!("mov {TMP},{CATEGORY}*{max_pages}{page}"));
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
    if g.compact {
        // SHOWN: the slots the page drawn counted in its category
        w(format!("if {PAGE} < 0 mov {PAGE},({SHOWN}-1)/{per_page}"));
        w(format!("mov {TMP},{PAGE}*{per_page}"));
        w(format!("if {TMP} >= {SHOWN} mov {PAGE},0"));
        for (c, category) in g.categories.iter().enumerate().filter(|(_, c)| !c.slots.is_empty()) {
            let base = COMPACT_BUTTON * (c as u32 + 1);
            let opens: Vec<String> = (0..category.slots.len()).map(|s| format!("*conv_pg_v{c}_{s}")).collect();
            w(format!("if {BUTTON_VAR} >= {base} && {BUTTON_VAR} < {} mov {TMP},{BUTTON_VAR}-{base}:tablegoto {TMP},{}", base + COMPACT_BUTTON, opens.join(",")));
        }
    } else {
        for (c, n) in pages.iter().enumerate() {
            w(format!("if {CATEGORY} == {c} && {PAGE} < 0 mov {PAGE},{}", n - 1));
            w(format!("if {CATEGORY} == {c} && {PAGE} >= {n} mov {PAGE},0"));
        }
    }
    let shot = g.category_hover.as_ref().and_then(|h| h.click_se.as_ref()).map_or(String::new(), |se| format!(":dwave {},\"{}\"", SE_CHANNELS.start(), path(se)));
    w(format!(
        "if {BUTTON_VAR} >= {BTN_CATEGORY} && {BUTTON_VAR} < {} mov {CATEGORY},{BUTTON_VAR}-{BTN_CATEGORY}:mov {PAGE},0:mov {FADE},{}{shot}",
        BTN_CATEGORY + MAX_CATEGORIES,
        g.page_ms
    ));
    // A slot: the page's own label goes on to its viewer.
    if !g.compact {
        w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {per_page} mov {TMP},{CATEGORY}*{max_pages}+{PAGE}:tablegoto {TMP},{}", targets.iter().map(|t| format!("{t}_open")).collect::<Vec<_>>().join(",")));
    }
    w(format!("goto *{DRAW}"));

    if g.compact {
        compact_pages(out, g, border, per_page);
        viewers(out, g, border, width, height);
        return;
    }
    let mut w = lines!(out);
    for (c, category) in g.categories.iter().enumerate() {
        for (p, slots) in category.slots.chunks(per_page as usize).enumerate() {
            w(format!("*conv_pg_c{c}_p{p}"));
            for (i, slot) in slots.iter().enumerate() {
                let (x, y) = place(i as u32);
                let n = slot.cgs.len();
                w(format!("mov {TMP},0"));
                for cg in &slot.cgs {
                    w(format!("if {} inc {TMP}", seen(&cg.seen, border).0));
                }
                w(format!("lsp {},\"{}\",{x},{y}", THUMB + i as u32, pic(&g.blank)));
                for cg in slot.cgs.iter().rev() {
                    w(format!("if {} lsp {},\"{}\",{x},{y}", seen(&cg.seen, border).0, THUMB + i as u32, pic(&cg.thumbnail)));
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

    viewers(out, g, border, width, height);
}

/// A compact gallery's pages: each category's label counts its slots with a
/// CG seen (SHOWN), drawing those on the page asked for ([`CELL`]), then
/// the arrows if they fill more than a page.
fn compact_pages(out: &mut String, g: &PictureGallery, border: u32, per_page: u32) {
    let mut w = lines!(out);
    for (c, category) in g.categories.iter().enumerate() {
        w(format!("*conv_pg_c{c}_p0"));
        w(format!("mov {SHOWN},0:mov {FIRST},{PAGE}*{per_page}:mov {LAST},{FIRST}+{per_page}"));
        for (s, slot) in category.slots.iter().enumerate() {
            w(format!("mov {TMP},0"));
            // from the last CG to the first: THUMBNAIL ends as the first seen's
            for cg in slot.cgs.iter().rev() {
                w(format!("if {} inc {TMP}:mov {THUMBNAIL},\"{}\"", seen(&cg.seen, border).0, pic(&cg.thumbnail)));
            }
            if !slot.shown_if.is_empty() {
                let unset: Vec<String> = slot.shown_if.iter().map(|v| format!("{} == 0", var_name(*v, border))).collect();
                w(format!("if {} mov {TMP},0", unset.join(" && ")));
            }
            w(format!("if {TMP} > 0 inc {SHOWN}"));
            w(format!("if {TMP} > 0 && {SHOWN} > {FIRST} && {SHOWN} <= {LAST} mov {SLOT},{}:mov {CGS},{}:gosub *{CELL}", COMPACT_BUTTON * (c as u32 + 1) + s as u32, slot.cgs.len()));
        }
        let paged = if g.arrows_when_paged { format!("if {SHOWN} > {per_page} ") } else { String::new() };
        for (slot, b, n) in [(PREVIOUS, &g.previous, BTN_PREVIOUS), (NEXT, &g.next, BTN_NEXT)] {
            w(format!("{paged}{}:spbtn {slot},{n},2", button(slot, b)));
        }
        w("goto *conv_pg_show".into());
    }
    // The slot SHOWN - FIRST of the page (from 1), TMP of its CGS seen.
    let (columns, rows) = (g.columns, g.rows);
    let (column, row) = if g.across { (format!("({AT} mod {columns})"), format!("({AT}/{columns})")) } else { (format!("({AT}/{rows})"), format!("({AT} mod {rows})")) };
    w(format!("*{CELL}"));
    w(format!("mov {AT},{SHOWN}-{FIRST}-1"));
    w(format!("mov {X},{}+{}*{column}:mov {Y},{}+{}*{row}", g.first.0, g.column_step, g.first.1, g.row_step));
    w(format!("lsp {THUMB}+{AT},{THUMBNAIL},{X},{Y}"));
    w(format!("if {TMP} < {CGS} amsp {THUMB}+{AT},{X},{Y},{}", g.partial_alpha));
    w(format!("lsp {CURSOR}+{AT},\":a/2,0,3;{}\",{X},{Y}:spbtn {CURSOR}+{AT},{SLOT}", path(&g.cursor)));
    w("return".into());
}

/// A compact gallery's variables, in the define section.
pub(crate) fn define(out: &mut String, g: &PictureGallery) {
    if g.compact && !g.engine_screen {
        for v in [SHOWN, FIRST, LAST, SLOT, CGS, AT, X, Y] {
            out.push_str(&format!("defvar {}\n", &v[1..]));
        }
        out.push_str(&format!("defstr {}\n", &THUMBNAIL[1..]));
    }
}

/// A CG's [`Seen`] as an `if` condition (seen), and as the start of an
/// `if`/`notif` line that runs while it isn't.
fn seen(s: &Seen, border: u32) -> (String, String) {
    match s {
        Seen::Var(v) => {
            let v = var_name(*v, border);
            (format!("{v} != 0"), format!("if {v} == 0"))
        }
        _ => {
            let f = tests(s, border);
            // `notif` negates each test: none seen is all of them unseen
            (f.join(" || "), format!("notif {}", f.join(" && ")))
        }
    }
}

/// The tests any of which is true once a CG is seen.
fn tests(s: &Seen, border: u32) -> Vec<String> {
    match s {
        Seen::Var(v) => vec![format!("{} != 0", var_name(*v, border))],
        Seen::Pictures(p) => {
            assert!(!p.is_empty(), "a CG seen by none of its pictures");
            p.iter().map(|p| format!("fchk \"{}\"", path(p))).collect()
        }
        Seen::Any(s) => s.iter().flat_map(|s| tests(s, border)).collect(),
    }
}

/// The viewer of each slot ([`CLOSE`] goes back to [`DRAW`] with the
/// close fade), and the click and scroll subroutines.
fn viewers(out: &mut String, g: &PictureGallery, border: u32, width: u32, height: u32) {
    let mut w = lines!(out);
    for (c, category) in g.categories.iter().enumerate() {
        for (s, slot) in category.slots.iter().enumerate() {
            let label = format!("conv_pg_v{c}_{s}");
            w(format!("*{label}"));
            w(format!("csp -1:bg black,1:mov {PREVIOUS_UNSEEN},0"));
            for (k, cg) in slot.cgs.iter().enumerate() {
                let (_, unseen) = seen(&cg.seen, border);
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
                w(format!("{unseen} goto *{label}_{k}u"));
                w(format!("lsp {VIEW},\"{}\",{},{}:print 10,{}", pic(&p.image), start.0, start.1, g.view_fade_ms));
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
                w(format!("lsp {VIEW},\"{}\",0,0:print 10,{}", pic(&g.unseen), g.view_fade_ms));
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

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::Var;

    #[test]
    fn seen_by_a_variable_or_by_pictures() {
        assert_eq!(seen(&Seen::Var(Var { global: true, index: 5 }), 200), ("%205 != 0".into(), "if %205 == 0".into()));
        let s = Seen::Pictures(vec!["pic/a.png".into(), "pic/b.png".into()]);
        assert_eq!(seen(&s, 200), (r#"fchk "pic\a.png" || fchk "pic\b.png""#.into(), r#"notif fchk "pic\a.png" && fchk "pic\b.png""#.into()));
        let any = Seen::Any(vec![Seen::Pictures(vec!["pic/a.png".into()]), Seen::Var(Var { global: true, index: 5 })]);
        assert_eq!(seen(&any, 200), (r#"fchk "pic\a.png" || %205 != 0"#.into(), r#"notif fchk "pic\a.png" && %205 != 0"#.into()));
    }

    #[test]
    fn a_compact_gallery_lists_the_slots_seen() {
        use conv_core::ir::{GalleryCategory, PictureSlot, SlotCg};
        let p = |name: &str| Placed { image: format!("sys/{name}.png"), x: 0, y: 0, w: 50, h: 50 };
        let cg = |i: u16| SlotCg { seen: Seen::Var(Var { global: true, index: i }), picture: p("cg"), thumbnail: format!("t/{i}.png") };
        let slots = vec![PictureSlot { cgs: vec![cg(1), cg(2)], reversed: false, shown_if: Vec::new() }, PictureSlot { cgs: vec![cg(3)], reversed: false, shown_if: vec![Var { global: true, index: 7 }, Var { global: true, index: 8 }] }];
        let g = PictureGallery {
            background: "sys/bg.png".into(),
            categories: vec![GalleryCategory { button: p("c"), slots }],
            previous: p("prev"),
            next: p("next"),
            exit: p("exit"),
            first: (100, 50),
            row_step: 90,
            column_step: 160,
            rows: 2,
            columns: 3,
            blank: "sys/blank.png".into(),
            cursor: "sys/cursor.png".into(),
            partial_alpha: 128,
            unseen: "sys/unseen.png".into(),
            open_ms: 300,
            page_ms: 300,
            view_fade_ms: 300,
            close_ms: 300,
            scroll_ms_per_px: 6,
            across: true,
            arrows_when_paged: true,
            engine_screen: false,
            category_hover: None,
            compact: true,
        };
        let mut out = String::new();
        emit(&mut out, &g, 200, 1280, 720);
        for part in [
            // the first seen of slot 0's CGs is its thumbnail; a slot is counted, and drawn when on the page
            "if %201 != 0 inc %105:mov $conv_pg_thumb,\":a;t\\1.png\"\nif %105 > 0 inc %conv_pg_shown\n",
            "mov %conv_pg_slot,1001:mov %conv_pg_cgs,1:gosub *conv_pg_cell\n",
            // slot 1 only while flag 7 or 8 is set
            "if %203 != 0 inc %105:mov $conv_pg_thumb,\":a;t\\3.png\"\nif %207 == 0 && %208 == 0 mov %105,0\nif %105 > 0 inc %conv_pg_shown\n",
            "if %conv_pg_shown > 6 lsp",
            "mov %conv_pg_x,100+160*(%conv_pg_at mod 3):mov %conv_pg_y,50+90*(%conv_pg_at/3)\n",
            "if %131 < 0 mov %131,(%conv_pg_shown-1)/6\n",
            "if %9 >= 1000 && %9 < 2000 mov %105,%9-1000:tablegoto %105,*conv_pg_v0_0,*conv_pg_v0_1\n",
        ] {
            assert!(out.contains(part), "missing {part:?} in\n{out}");
        }
        assert!(!out.contains("_open"), "{out}");
    }
}
