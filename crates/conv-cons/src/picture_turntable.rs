//! [`Turntable`] as NScripter code, a mode of
//! [`picture_extras`](crate::picture_extras) entered at [`TURNTABLE`].
//!
//! [`PAGE`] (from 0) and [`CHOSEN`] (the entry from 1, 0 before the first
//! visit) are globals, kept across games. `btnwait` drops its buttons, so
//! the plates are drawn again after every click ([`PLATES`]); the chosen
//! entry's movie is drawn only when it changes ([`SHOW`]), so it doesn't
//! start over.

use conv_core::ir::Turntable;

use crate::picture_extras::pic;
use crate::{path, var_name, BUTTON_VAR, MAX_SPRITE, MENU_SPRITES};

pub(crate) const TURNTABLE: &str = "conv_pextras_turntable";
const MENU: &str = "conv_pextras_draw";
const WAIT: &str = "conv_ptt_wait";
const PLATES: &str = "conv_ptt_plates";
const SHOW: &str = "conv_ptt_show";
const PAGE: &str = "%2963";
const CHOSEN: &str = "%2964";
// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const PLATE: u32 = 900; // + place on the page
const MAX_PER_PAGE: usize = 60;
const CHOSEN_PLATE: u32 = 899;
const NAME: u32 = 890;
const DETAILS: u32 = 891;
const MOVIE: u32 = 892;
const PREVIOUS: u32 = MENU_SPRITES.start;
const NEXT: u32 = MENU_SPRITES.start + 1;
const EXIT: u32 = MENU_SPRITES.start + 2;
const BTN_PREVIOUS: usize = 1001;
const BTN_NEXT: usize = 1002;
const BTN_EXIT: usize = 1003;

pub(crate) fn emit(out: &mut String, t: &Turntable, border: u32) {
    assert!(t.per_page > 0 && t.per_page <= MAX_PER_PAGE && t.entries.len() < BTN_PREVIOUS, "a turntable larger than its sprites");
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let n = t.entries.len();
    let pages = n.div_ceil(t.per_page).max(1);
    w(format!("*{TURNTABLE}"));
    w(format!("if {CHOSEN} == 0 mov {CHOSEN},{}", t.first + 1));
    w(format!("csp -1:lsp {BACKGROUND},\"{}\",0,0", pic(&t.background)));
    w(format!("gosub *{SHOW}:gosub *{PLATES}:print 10,{}", t.open_ms));
    w(format!("*{WAIT}"));
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} goto *{MENU}"));
    w(format!("if {BUTTON_VAR} == {BTN_PREVIOUS} add {PAGE},{}:mod {PAGE},{pages}", pages - 1));
    w(format!("if {BUTTON_VAR} == {BTN_NEXT} add {PAGE},1:mod {PAGE},{pages}"));
    w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {n} mov {CHOSEN},{BUTTON_VAR}:gosub *{SHOW}"));
    w(format!("if {BUTTON_VAR} == 0 gosub *{PLATES}:print 1:goto *{WAIT}"));
    w(format!("gosub *{PLATES}:print 10,{}:goto *{WAIT}", t.change_ms));

    w(format!("*{SHOW}"));
    for (i, e) in t.entries.iter().enumerate() {
        let k = i + 1;
        let m = &e.movie;
        // spzoom zooms about the centre: the sprite's place for the zoomed
        // movie's top left at m.x, m.y.
        let (x, y) = (m.x + (m.w * e.zoom as i32 / 100 - m.w) / 2, m.y + (m.h * e.zoom as i32 / 100 - m.h) / 2);
        w(format!(
            "if {CHOSEN} == {k} lsp {NAME},\"{}\",{},{}:lsp {DETAILS},\"{}\",{},{}:lsp {MOVIE},\":c/{},{},0;{}\",{x},{y}:spzoom {MOVIE},{}",
            pic(&e.name.image),
            e.name.x,
            e.name.y,
            pic(&e.details.image),
            e.details.x,
            e.details.y,
            e.cells,
            e.frame_ms,
            path(&m.image),
            e.zoom
        ));
        if let Some((v, image)) = &e.details_after {
            w(format!("if {CHOSEN} == {k} && {} != 0 lsp {DETAILS},\"{}\",{},{}", var_name(*v, border), pic(image), e.details.x, e.details.y));
        }
    }
    w("return".into());

    w(format!("*{PLATES}"));
    w((0..t.per_page as u32).map(|k| format!("csp {}", PLATE + k)).chain([format!("csp {CHOSEN_PLATE}")]).collect::<Vec<_>>().join(":"));
    for (b, sprite, number) in [(&t.previous, PREVIOUS, BTN_PREVIOUS), (&t.next, NEXT, BTN_NEXT), (&t.exit, EXIT, BTN_EXIT)] {
        w(format!("lsp {sprite},\":a/3,0,3;{}\",{},{}:spbtn {sprite},{number}", path(&b.image), b.x, b.y));
    }
    for (i, e) in t.entries.iter().enumerate() {
        let (k, p) = (i + 1, i / t.per_page);
        let (slot, b) = (PLATE + (i % t.per_page) as u32, &e.plate);
        let open = format!("lsp {slot},\":a/3,0,3;{}\",{},{}:spbtn {slot},{k}", path(&b.image), b.x, b.y);
        match e.unlocked_by.map(|v| var_name(v, border)) {
            Some(v) => {
                w(format!("if {PAGE} == {p} && {v} != 0 {open}"));
                w(format!("if {PAGE} == {p} && {v} == 0 lsp {slot},\"{}\",{},{}", pic(&t.locked), b.x, b.y));
            }
            None => w(format!("if {PAGE} == {p} {open}")),
        }
        w(format!("if {PAGE} == {p} && {CHOSEN} == {k} lsp {CHOSEN_PLATE},\":a/3,0,3;{}\",{},{}:cell {CHOSEN_PLATE},2", path(&b.image), b.x, b.y));
    }
    w("return".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{Placed, TurntableEntry, Var};

    #[test]
    fn pages_plates_and_the_chosen_movie() {
        let placed = |image: &str, x, y, w, h| Placed { image: image.into(), x, y, w, h };
        let entry = |n: i32, unlocked_by| TurntableEntry {
            plate: placed(&format!("system/p{n}.png"), 910 - 30 * n, 28, 28, 188),
            unlocked_by,
            name: placed("system/name.png", 480, 28, 80, 400),
            details: placed("system/spec.png", 60, 436, 900, 100),
            details_after: Some((Var { global: true, index: 5 }, "system/spec2.png".into())),
            movie: placed("system/movie/ex.jpg", 80, 28, 200, 200),
            cells: 75,
            frame_ms: 66,
            zoom: 200,
        };
        let t = Turntable {
            background: "system/bg.png".into(),
            previous: placed("system/prev.png", 948, 438, 76, 36),
            next: placed("system/next.png", 948, 474, 76, 36),
            exit: placed("system/exit.png", 948, 510, 76, 36),
            locked: "system/no.png".into(),
            entries: vec![entry(0, None), entry(1, Some(Var { global: true, index: 4 })), entry(2, None)],
            per_page: 2,
            first: 1,
            open_ms: 500,
            change_ms: 300,
        };
        let mut out = String::new();
        emit(&mut out, &t, 800);
        assert!(out.contains("if %2964 == 0 mov %2964,2\n"), "{out}");
        assert!(out.contains("if %9 == 1001 add %2963,1:mod %2963,2\n"), "{out}");
        // A 200×200 movie zoomed 200% with its top left at 80,28.
        assert!(out.contains("lsp 892,\":c/75,66,0;system\\movie\\ex.jpg\",180,128:spzoom 892,200\n"), "{out}");
        assert!(out.contains("if %2964 == 1 && %805 != 0 lsp 891,"), "{out}");
        assert!(out.contains("if %2963 == 0 && %804 == 0 lsp 901,\":a;system\\no.png\",880,28\n"), "{out}");
        // The third entry is first on the second page.
        assert!(out.contains("if %2963 == 1 lsp 900,\":a/3,0,3;system\\p2.png\",850,28:spbtn 900,3\n"), "{out}");
    }
}
