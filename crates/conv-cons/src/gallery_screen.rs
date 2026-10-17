//! [`PictureGallery`]'s pages drawn by the engine (`screen "gallery"`,
//! COMMANDS.md "Screens") when it has [`PictureGallery::engine_screen`]:
//! every page of every category is a page of the screen; a slot is its
//! first seen CG's thumbnail (faint while some are unseen, `blank` while
//! none is) under the two-cell cursor button; the categories are tabs
//! (their pressed cell while open), previous and next go round inside the
//! category, all cross-fading over `page_ms`. A slot closes the screen
//! holding its frame with its value (`category * 1000 + slot + 1`) and the
//! script's viewer ([`picture_gallery`]) runs, then the screen opens again
//! on the same page, fading over [`FADE`]'s value.
//!
//! [`picture_gallery`]: crate::picture_gallery

use std::fmt::Write as _;

use conv_core::ir::{PictureGallery, Placed};

use crate::menu::TMP;
use crate::picture_gallery::{DRAW, FADE, GALLERY, MENU, SCREEN_PAGE};
use crate::screens::lua;
use crate::{var_name, BUTTON_VAR};

/// A slot's value: its category and place in it.
fn value(c: usize, s: usize) -> usize {
    c * 1000 + s + 1
}

/// Each category's pages.
fn pages(g: &PictureGallery) -> Vec<usize> {
    let per_page = (g.rows * g.columns) as usize;
    g.categories.iter().map(|c| c.slots.len().div_ceil(per_page).max(1)).collect()
}

/// [`GALLERY`] and [`DRAW`], and what a slot's value opens.
pub(crate) fn labels(out: &mut String, g: &PictureGallery) {
    writeln!(out, "*{GALLERY}\nmov {SCREEN_PAGE},1:mov {FADE},{}\n*{DRAW}\ncsp -1\nscreen \"gallery\",{BUTTON_VAR}\nif {BUTTON_VAR} == 0 goto *{MENU}", g.open_ms).unwrap();
    for (c, category) in g.categories.iter().enumerate().filter(|(_, c)| !c.slots.is_empty()) {
        let opens: Vec<String> = (0..category.slots.len()).map(|s| format!("*conv_pg_v{c}_{s}")).collect();
        writeln!(out, "if {BUTTON_VAR} >= {} && {BUTTON_VAR} <= {} mov {TMP},{BUTTON_VAR}-{}:tablegoto {TMP},{}", value(c, 0), value(c, category.slots.len() - 1), value(c, 0), opens.join(",")).unwrap();
    }
    writeln!(out, "goto *{DRAW}").unwrap();
}

/// A picture as a Lua file and whether it is opaque.
fn file(p: &str) -> String {
    format!("{}, {}", lua(p), crate::sprite_mode(p) == 'c')
}

fn button(p: &Placed, extra: &str) -> String {
    format!("{{\"button\", file={}, x={}, y={}, cells=3, pressed=2{extra}}}", lua(&p.image), p.x, p.y)
}

/// `screens/gallery.lua`.
pub(crate) fn lua_file(g: &PictureGallery, border: u32) -> String {
    let per_page = (g.rows * g.columns) as usize;
    let pages = pages(g);
    let total: usize = pages.iter().sum();
    let mut out = String::from("-- The CG gallery's pages (conscripter-convert, from PictureGallery).\n");
    let bg = crate::sprite_mode(&g.background) == 'c';
    writeln!(out, "local widgets = {{{{\"image\", file={}, opaque={bg}, x=0, y=0}}}}", lua(&g.background)).unwrap();
    writeln!(out, "local blank = {{nil, {}}}", file(&g.blank)).unwrap();
    // A slot: the first seen CG's thumbnail ({seen variable, thumbnail, opaque} each), the cursor button.
    writeln!(
        out,
        "local function slot(page, x, y, value, cgs)\n  local seen, first = 0, nil\n  for _, c in ipairs(cgs) do\n    if var(c[1]) ~= 0 then seen = seen + 1; first = first or c end\n  end\n  local f = first or blank\n  widgets[#widgets + 1] = {{\"image\", file=f[2], opaque=f[3], x=x, y=y, page=page, alpha=(seen > 0 and seen < #cgs) and {} or 255}}\n  widgets[#widgets + 1] = {{\"button\", file={}, cells=2, x=x, y=y, page=page, enabled=seen > 0, action={{\"close\", value, hold=true}}}}\nend",
        g.partial_alpha,
        lua(&g.cursor)
    )
    .unwrap();
    let place = |i: usize| {
        let i = i as i32;
        let (columns, rows) = (g.columns as i32, g.rows as i32);
        let (column, row) = if g.across { (i % columns, i / columns) } else { (i / rows, i % rows) };
        (g.first.0 + g.column_step * column, g.first.1 + g.row_step * row)
    };
    let fade = format!(", fade={}", g.page_ms);
    let mut first_page = 1;
    let mut rest = Vec::new();
    for (c, category) in g.categories.iter().enumerate() {
        let (a, b) = (first_page, first_page + pages[c] - 1);
        for (s, slot) in category.slots.iter().enumerate() {
            let (x, y) = place(s % per_page);
            let cgs: Vec<String> = slot.cgs.iter().map(|cg| format!("{{{}, {}}}", lua(&var_name(cg.seen, border)), file(&cg.thumbnail))).collect();
            writeln!(out, "slot({}, {x}, {y}, {}, {{{}}})", a + s / per_page, value(c, s), cgs.join(", ")).unwrap();
        }
        // The open category's tab shows its pressed cell and is no button.
        rest.push(button(&category.button, &format!(", disabled=2, enabled=false, pages={{{a}, {b}}}")));
        for (from, to) in [(1, a - 1), (b + 1, total)] {
            if from <= to {
                rest.push(button(&category.button, &format!(", pages={{{from}, {to}}}, action={{\"page\", {a}{fade}}}")));
            }
        }
        if !(g.arrows_when_paged && a == b) {
            for p in a..=b {
                let previous = if p == a { b } else { p - 1 };
                let next = if p == b { a } else { p + 1 };
                rest.push(button(&g.previous, &format!(", page={p}, action={{\"page\", {previous}{fade}}}")));
                rest.push(button(&g.next, &format!(", page={p}, action={{\"page\", {next}{fade}}}")));
            }
        }
        first_page = b + 1;
    }
    rest.push(button(&g.exit, ", action={\"close\", 0, hold=true}"));
    for r in rest {
        writeln!(out, "widgets[#widgets + 1] = {r}").unwrap();
    }
    writeln!(
        out,
        "return {{\n  kind = \"extras\",\n  fade = var({}),\n  pages = {total},\n  page_var = {},\n  cancel = {{\"close\", 0, hold=true}},\n  widgets = widgets,\n}}",
        lua(FADE),
        lua(SCREEN_PAGE)
    )
    .unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{GalleryCategory, PictureSlot, SlotCg, Var};

    #[test]
    fn gallery_pages_become_a_screen() {
        let p = |name: &str, x, y| Placed { image: format!("sys/{name}.png"), x, y, w: 50, h: 50 };
        let slot = |i: u16| PictureSlot { cgs: vec![SlotCg { seen: Var { global: true, index: i }, picture: p("cg", 0, 0), thumbnail: format!("t/{i}.jpg") }], reversed: false };
        let g = PictureGallery {
            background: "sys/bg.png".into(),
            categories: vec![GalleryCategory { button: p("c1", 862, 136), slots: (0..25).map(slot).collect() }, GalleryCategory { button: p("c2", 862, 240), slots: vec![slot(30)] }],
            previous: p("prev", 912, 438),
            next: p("next", 862, 438),
            exit: p("exit", 0, 438),
            first: (672, 38),
            row_step: 100,
            column_step: -100,
            rows: 5,
            columns: 4,
            blank: "sys/blank.png".into(),
            cursor: "sys/cursor.png".into(),
            partial_alpha: 128,
            unseen: "sys/unseen.png".into(),
            open_ms: 500,
            page_ms: 300,
            view_fade_ms: 500,
            close_ms: 300,
            scroll_ms_per_px: 12,
            across: false,
            arrows_when_paged: true,
            engine_screen: true,
        };
        let lua = lua_file(&g, 4000);
        for part in [
            // Slot 21 is page 2's first, at the first place; the second category starts at page 3.
            "slot(2, 672, 38, 21, {{\"%4020\", \"t/20.jpg\", true}})",
            "slot(3, 672, 38, 1001, {{\"%4030\", \"t/30.jpg\", true}})",
            "x=862, y=136, cells=3, pressed=2, disabled=2, enabled=false, pages={1, 2}}",
            "x=862, y=136, cells=3, pressed=2, pages={3, 3}, action={\"page\", 1, fade=300}}",
            // Round inside the category; none in the one-page category.
            "x=912, y=438, cells=3, pressed=2, page=1, action={\"page\", 2, fade=300}}",
            "pages = 3,\n  page_var = \"%131\",",
        ] {
            assert!(lua.contains(part), "missing {part:?} in\n{lua}");
        }
        assert!(!lua.contains("page=3, action"), "{lua}");
        let mut out = String::new();
        labels(&mut out, &g);
        assert!(out.contains("if %9 >= 1001 && %9 <= 1001 mov %105,%9-1001:tablegoto %105,*conv_pg_v1_0\n"), "{out}");
    }
}
