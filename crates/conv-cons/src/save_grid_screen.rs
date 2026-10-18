//! [`SaveGrid`] drawn by the engine (`screen "save"`, `screen "load"`,
//! COMMANDS.md "Screens"): the
//! page's picture, the frames, the slots (down the columns), each slot's
//! details shown while the pointer is on it (`hover`), previous and next
//! page (going round, cross-faded), the delete mode as a toggle and the
//! slots' actions taken by it (`when`), the game's own questions
//! (`save_ask_*`) and the notice after a save (`save_saved`).
//!
//! With [`SaveGrid::popup`] a click on a slot runs `*conv_sg_slot` (is it
//! saved, locked) and opens the slot's window over the screen
//! (`save_popup`), whose buttons act on that slot; the locks' marks are
//! images shown while their variable is 1. [`SaveGrid::tabs`] pick the
//! pages, and [`SaveGrid::quick_page`] is a last page of its own slots
//! widget holding only the quick save.
//!
//! A save's text is "|play time|chapter|last line", made before the screen
//! opens ([`save_grid::details`]); the date is the save file's. Its parts
//! are counted from the last.

use std::fmt::Write as _;

use conv_core::ir::{GridText, Placed, SaveGrid, Script, SlotPopup, Switch};

use crate::menu::{switch_off, IN_GAME, LABEL, PAGE};
use crate::menu::{HAS_DATA, QUICK_SLOT, SLOT, TMP};
use crate::save_grid::{self, DELETING, LOCKED, LOCK_BASE};
use crate::screens::{image, lua};

/// The questions asked: the switch, its screen.
const ASKS: [(Switch, &str); 3] = [(Switch::ConfirmSave, "save_ask_save"), (Switch::ConfirmLoad, "save_ask_load"), (Switch::ConfirmDelete, "save_ask_delete")];

/// The save grid is the engine's.
pub(crate) fn engine_grid(script: &Script) -> Option<&SaveGrid> {
    script.save_grid.as_ref()
}

/// `*conv_save` and `*conv_load`. The thumbnail is the scene without the
/// text window, as the routine takes it.
pub(crate) fn labels(out: &mut String, g: &SaveGrid) {
    for (label, name) in [(crate::menu::SAVE, "save"), (crate::menu::LOAD, "load")] {
        writeln!(out, "*{label}").unwrap();
        if name == "save" {
            writeln!(out, "mov {LABEL},\"\"").unwrap();
            save_grid::details(&mut |l: String| writeln!(out, "{l}").unwrap(), g);
        }
        writeln!(out, "mov {DELETING},0\nif {IN_GAME} == 1 textoff\nscreen \"{name}\"\nif {IN_GAME} == 1 texton\nreturn").unwrap();
    }
    if let Some(p) = &g.popup {
        // The slot clicked (SLOT): HAS_DATA, LOCKED, and in TMP what to do:
        // 1 save at once, 2 open its window, 0 nothing.
        writeln!(out, "*conv_sg_slot\nsavefileexist {HAS_DATA},{SLOT}").unwrap();
        writeln!(out, "mov {LOCKED},0:if {SLOT} < {QUICK_SLOT} mov {TMP},{LOCK_BASE}+{SLOT}:mov {LOCKED},%%{}", &TMP[1..]).unwrap();
        writeln!(out, "mov {TMP},2").unwrap();
        if p.empty_saves {
            writeln!(out, "if {HAS_DATA} == 0 mov {TMP},0\nif {HAS_DATA} == 0 && {IN_GAME} == 1 && {SLOT} < {QUICK_SLOT} mov {TMP},1").unwrap();
        }
        writeln!(out, "return").unwrap();
    }
}

/// The files under `screens/`.
pub(crate) fn files(script: &Script) -> Vec<(String, String)> {
    let Some(g) = engine_grid(script) else { return Vec::new() };
    assert!(g.popup.is_none() || g.save_pages.len() as u32 * g.rows * g.columns <= 100, "save grid locks past their range");
    let pictures = script.confirm.as_ref().map(|d| &d.pictures[..]).unwrap_or_default();
    let asks: Vec<(Switch, &str)> = ASKS.into_iter().filter(|(s, _)| pictures.iter().any(|(q, _)| q == s)).collect();
    let mut files = vec![("save.lua".to_string(), grid_lua(g, &asks, true)), ("load.lua".to_string(), grid_lua(g, &asks, false))];
    if let Some(d) = &script.confirm {
        for (s, name) in &asks {
            let picture = &pictures.iter().find(|(q, _)| q == s).expect("asked").1;
            files.push((format!("{name}.lua"), crate::settings_screen::question_lua(d, picture, name)));
        }
    }
    if let Some(p) = &g.popup {
        files.push(("save_popup.lua".to_string(), popup_lua(g, p, &asks, script.width, script.height)));
    }
    if let Some(n) = &g.saved {
        // Over the dimmed screen until a click.
        let (w, h) = (script.width, script.height);
        let out = format!(
            "-- After a save (conscripter-convert, from SaveGrid).\nreturn {{\n  kind = \"other\",\n  widgets = {{\n    {{\"image\", file=\">{w},{h},#000000\", opaque=true, x=0, y=0, alpha=204}},\n    {},\n    {{\"button\", file=\">{w},{h},#000000\", opaque=true, x=0, y=0, alpha=0, action={{\"close\"}}}},\n  }},\n}}\n",
            image("image", n, "")
        );
        files.push(("save_saved.lua".to_string(), out));
    }
    files
}

/// A [`GridText`] as a line of the slots' (`text` for a save, `empty`
/// without one).
fn line(t: &GridText, text: &str, empty: &str) -> String {
    let (cols, rows, x) = if t.vertical {
        // x is the first (rightmost) column's left edge; the box's is the last's.
        (t.lines, t.chars, t.x - (t.lines.saturating_sub(1) * (t.font_size + t.line_gap)) as i32)
    } else {
        (t.chars, t.lines, t.x)
    };
    let vertical = if t.vertical { ", vertical=true" } else { "" };
    format!(
        "{{x={x}, y={}, size={}, cols={cols}, rows={rows}, pitch={}, gap={}, color={}{vertical}, text={}, empty={}}}",
        t.y,
        t.font_size,
        t.line_gap,
        t.char_gap,
        lua(&t.colour),
        lua(text),
        lua(empty)
    )
}

/// Slot i's top left on a page.
fn place(g: &SaveGrid, i: u32) -> (i32, i32) {
    let (column, row) = if g.across { (i % g.columns, i / g.columns) } else { (i / g.rows, i % g.rows) };
    (g.first.0 + g.column_step * column as i32, g.first.1 + g.row_step * row as i32)
}

fn grid_lua(g: &SaveGrid, asks: &[(Switch, &str)], saving: bool) -> String {
    let kind = if saving { "save" } else { "load" };
    let ask = |s: Switch| {
        asks.iter().find(|(q, _)| *q == s).map(|(_, name)| format!(", dialog=var({}) == 0 and {} or nil", lua(&switch_off(s)), lua(name))).unwrap_or_default()
    };
    let deleting = |on: u32| format!("when={{{}, {on}}}", lua(DELETING));
    let act = if g.popup.is_some() {
        // *conv_sg_slot says what to do (TMP): save at once, or the slot's window.
        let to = |n: u32| format!("when={{{}, {n}}}", lua(TMP));
        let saved = if g.saved.is_some() { format!(", {{\"open\", \"save_saved\", {}}}", to(1)) } else { String::new() };
        format!("{{{{\"set\", {}, slot=true}}, {{\"gosub\", \"*conv_sg_slot\"}}, {{\"save\", {}}}{saved}, {{\"open\", \"save_popup\", {}}}}}", lua(SLOT), to(1), to(2))
    } else if saving {
        let saved = if g.saved.is_some() { format!(", {{\"open\", \"save_saved\", {}}}", deleting(0)) } else { String::new() };
        format!("{{{{\"delete\", {}{}}}, {{\"save\", {}{}}}{saved}}}", deleting(1), ask(Switch::ConfirmDelete), deleting(0), ask(Switch::ConfirmSave))
    } else {
        format!("{{{{\"delete\", {}{}}}, {{\"load\", {}{}}}}}", deleting(1), ask(Switch::ConfirmDelete), deleting(0), ask(Switch::ConfirmLoad))
    };
    let pages = if saving { &g.save_pages } else { &g.load_pages };
    // With a quick page, the last page is a slots widget of its own.
    let slot_pages = if g.quick_page { pages.len() - 1 } else { pages.len() };
    let per = g.rows * g.columns;
    let mut w = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        w.push(format!("{{\"image\", file={}, opaque=true, x=0, y=0, page={}}}", lua(p), i + 1));
    }
    // Under the thumbnails, which cover it.
    if let Some(e) = &g.empty {
        for i in 0..per {
            let (x, y) = place(g, i);
            w.push(format!("{{\"image\", file={}, x={}, y={}}}", lua(&e.image), x + e.x, y + e.y));
        }
    }
    // The frames over the thumbnails, under the new mark and the hover frame.
    // Inline (indented as a widget), or a `local` the pages share.
    let i = if g.quick_page { "  " } else { "      " };
    let over = g.frames.as_ref().map(|f| format!("\n{i}over={{file={}, x={}, y={}}},", lua(&f.image), f.x, f.y)).unwrap_or_default();
    let prefix = g.play_time_prefix.replace('%', "%%");
    let (tw, th) = (g.thumb_w, g.thumb_h);
    let p = g.preview;
    let (hx, hy) = g.hover_offset;
    let (nx, ny) = g.new_offset;
    let slots = format!(
        "{{\"saveslots\", x={}, y={}, dx={}, dy={}, rows={}, cols={}, down={}, pages={}, page_var={}, action=act,{over}\n{i}button={{file=\">{tw},{th},#000000\", opaque=true, alpha=0}},\n{i}thumb={{x=0, y=0, w={tw}, h={th}}}, shot={{{tw}, {th}}},\n{i}new={{file={}, x={nx}, y={ny}}},\n{i}save_text={}, split=\"|\",\n{i}hover={{image={{file={}, x={hx}, y={hy}}}, preview={{x={}, y={}, w={}, h={}}},\n{i}  lines={{\n{i}    {},\n{i}    {},\n{i}    {},\n{i}    {}}}}}}}",
        g.first.0,
        g.first.1,
        g.column_step,
        g.row_step,
        g.rows,
        g.columns,
        !g.across,
        pages.len(),
        lua(PAGE),
        lua(&g.new_mark.image),
        lua(LABEL),
        lua(&g.hover.image),
        p.x,
        p.y,
        p.w,
        p.h,
        line(&g.date, "%Y-%m-%d %H:%M:%S", ""),
        line(&g.play_time, &format!("{prefix}%T-3"), ""),
        line(&g.chapter, "%T-2", ""),
        line(&g.line, "%T-1", "　データ無し"),
    );
    if g.quick_page {
        for page in 1..=slot_pages {
            w.push(format!("with(slots, {{page={page}}})"));
        }
        // One slot, the quick save: its slot number on the last page.
        w.push(format!("with(slots, {{page={}, first={}, rows=1, cols=1, new=false}})", pages.len(), QUICK_SLOT as usize + 1 - pages.len()));
    } else {
        w.push(slots.clone());
    }
    if let Some(l) = g.popup.as_ref().map(|p| &p.locked) {
        for n in 0..slot_pages as u32 * per {
            let (x, y) = place(g, n % per);
            w.push(format!("{{\"image\", file={}, x={}, y={}, page={}, when={{\"%{}\", 1}}}}", lua(&l.image), x + l.x, y + l.y, n / per + 1, LOCK_BASE + n + 1));
        }
    }
    let fade = if g.fade_ms > 0 { format!(", fade={}", g.fade_ms) } else { String::new() };
    if g.tabs.is_empty() {
        w.push(image("button", &g.previous, &format!(", cells=4, pressed=3, action={{\"page\", \"-1\", round=true{fade}}}")));
        w.push(image("button", &g.next, &format!(", cells=4, pressed=3, action={{\"page\", \"+1\", round=true{fade}}}")));
    }
    // The current page's tab shows its third cell (its action can't be taken).
    for (t, tab) in g.tabs.iter().enumerate() {
        w.push(image("button", tab, &format!(", cells=4, pressed=3, disabled=2, action={{\"page\", \"{}\"{fade}}}", t + 1)));
    }
    if g.popup.is_none() {
        w.push(image(
            "toggle",
            &g.delete,
            &format!(", cells=4, pressed=3, on_file={}, on_cells=1, var={}, fade=150", lua(&g.delete_on.image), lua(DELETING)),
        ));
    }
    w.push(image("button", &g.close, ", cells=4, pressed=3, action={\"close\"}"));

    let mut out = format!("-- The {kind} screen (conscripter-convert, from SaveGrid).\nlocal act = {act}\n");
    if g.quick_page {
        writeln!(out, "local slots = {slots}").unwrap();
        out.push_str("local function with(t, more)\n  local w = {}\n  for k, v in pairs(t) do w[k] = v end\n  for k, v in pairs(more) do w[k] = v end\n  return w\nend\n");
    }
    write!(out, "return {{\n  kind = \"{kind}\",\n").unwrap();
    if g.fade_ms > 0 {
        writeln!(out, "  fade = {},", g.fade_ms).unwrap();
    }
    out.push_str("  widgets = {\n");
    for x in w {
        writeln!(out, "    {x},").unwrap();
    }
    out.push_str("  },\n}\n");
    out
}

/// The slot's window ([`SaveGrid::popup`]), over the screen at the slot
/// clicked (`SLOT`, with `LOCKED` from `*conv_sg_slot`); a click outside it
/// or a right-click closes it.
fn popup_lua(g: &SaveGrid, p: &SlotPopup, asks: &[(Switch, &str)], width: u32, height: u32) -> String {
    let ask = |s: Switch| {
        asks.iter().find(|(q, _)| *q == s).map(|(_, name)| format!(", dialog=var({}) == 0 and {} or nil", lua(&switch_off(s)), lua(name))).unwrap_or_default()
    };
    let per = g.rows * g.columns;
    let (column, row) = if g.across { (format!("i % {}", g.columns), format!("i // {}", g.columns)) } else { (format!("i // {}", g.rows), format!("i % {}", g.rows)) };
    let at = |q: &Placed| format!("x=x+{}, y=y+{}", q.x, q.y);
    let button = |q: &Placed, more: &str| format!("{{\"button\", file={}, cells=4, pressed=3, disabled=2, {}{more}}}", lua(&q.image), at(q));
    let saved = if g.saved.is_some() { ", {\"open\", \"save_saved\"}" } else { "" };
    let lock = |q: &Placed| button(q, &format!(", enabled=quick and 0 or 1, action={{{{\"set\", \"%\" .. ({LOCK_BASE} + slot), 1 - locked}}, {{\"close\"}}}}"));
    format!(
        "-- A save slot's window (conscripter-convert, from SaveGrid.popup).\n\
local slot, locked, in_game = var({}), var({}), var({})\n\
local quick = slot >= {QUICK_SLOT}\n\
local i = quick and 0 or (slot - 1) % {per}\n\
local x, y = {} + {} * ({column}), {} + {} * ({row})\n\
return {{\n  kind = \"other\",\n  fade = 150,\n  widgets = {{\n    \
{{\"button\", file=\">{width},{height},#000000\", opaque=true, alpha=0, x=0, y=0, action={{\"close\"}}}},\n    \
{{\"image\", file={}, {}}},\n    \
{},\n    {},\n    {},\n    locked == 1 and {} or {},\n  }},\n}}\n",
        lua(SLOT),
        lua(LOCKED),
        lua(IN_GAME),
        g.first.0,
        g.column_step,
        g.first.1,
        g.row_step,
        lua(&p.window.image),
        at(&p.window),
        button(&p.save, &format!(", enabled=(locked == 0 and in_game == 1 and not quick) and 1 or 0, action={{{{\"save\", slot{}}}{saved}, {{\"close\"}}}}", ask(Switch::ConfirmSave))),
        button(&p.load, &format!(", action={{\"load\", slot{}}}", ask(Switch::ConfirmLoad))),
        button(&p.delete, &format!(", enabled=locked == 0 and 1 or 0, action={{{{\"delete\", slot{}}}, {{\"close\"}}}}", ask(Switch::ConfirmDelete))),
        lock(&p.unlock),
        lock(&p.lock),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::ConfirmDialog;

    #[test]
    fn grid_becomes_a_screen() {
        let g = crate::save_grid::tests::grid();
        let mut script = Script { width: 1024, height: 576, ..Default::default() };
        script.save_grid = Some(g.clone());
        let p = |name: &str| Placed { image: format!("sys/{name}.png"), x: 412, y: 88, w: 100, h: 400 };
        script.confirm = Some(ConfirmDialog {
            base: p("q"),
            yes: p("yes"),
            no: p("no"),
            yes_label: String::new(),
            no_label: String::new(),
            text_y: 0,
            font_size: 20,
            overwrite: String::new(),
            load: String::new(),
            delete: String::new(),
            title: String::new(),
            quit: String::new(),
            pictures: vec![(Switch::ConfirmSave, p("over")), (Switch::ConfirmDelete, p("del"))],
            backdrop: None,
            pressed: true,
            fade_ms: 150,
            unroll: None,
        });
        let files = files(&script);
        let names: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, ["save.lua", "load.lua", "save_ask_save.lua", "save_ask_delete.lua", "save_saved.lua"]);
        let save = &files[0].1;
        for part in [
            "{\"image\", file=\"sys/save2.png\", opaque=true, x=0, y=0, page=2}",
            "x=600, y=34, dx=-190, dy=100, rows=5, cols=4, down=true, pages=2, page_var=\"%900\"",
            "local act = {{\"delete\", when={\"%149\", 1}, dialog=var(\"%",
            "{\"save\", when={\"%149\", 0}, dialog=var(",
            "{\"open\", \"save_saved\", when={\"%149\", 0}}}",
            "hover={image={file=\"sys/hover.png\", x=-3, y=-3}",
            "action=act,\n      over={file=\"sys/frames.png\", x=0, y=0},",
            "vertical=true, text=\"%T-2\"",
            "action={\"page\", \"+1\", round=true, fade=300}",
            "{\"toggle\", file=\"sys/delete.png\"",
            "on_cells=1, var=\"%149\", fade=150",
        ] {
            assert!(save.contains(part), "missing {part:?} in\n{save}");
        }
        // No picture for the load question: loading doesn't ask.
        assert!(files[1].1.contains("{\"load\", when={\"%149\", 0}}"), "{}", files[1].1);
        let mut out = String::new();
        labels(&mut out, &g);
        assert!(out.contains("*conv_save\nmov $100,\"\"\ngetplaytime"), "{out}");
        assert!(out.contains("mov %149,0\nif %101 == 1 textoff\nscreen \"load\"\nif %101 == 1 texton\nreturn"), "{out}");
    }

    #[test]
    fn popup_tabs_and_quick_page_as_screens() {
        let mut g = crate::save_grid::tests::grid();
        let p = |name: &str, x: i32, y: i32| Placed { image: format!("sys/{name}.png"), x, y, w: 10, h: 10 };
        g.tabs = vec![p("tab1", 157, 0), p("tab2", 299, 0)];
        g.quick_page = true;
        g.empty = Some(p("empty", 0, 0));
        g.popup = Some(SlotPopup {
            window: p("window", 0, 0),
            save: p("save", 0, 1),
            load: p("load", 82, 0),
            delete: p("delete", 82, 47),
            lock: p("lock", 0, 47),
            unlock: p("unlock", 0, 47),
            locked: p("locked", -2, -2),
            empty_saves: true,
        });
        let script = Script { width: 1024, height: 576, save_grid: Some(g.clone()), ..Default::default() };
        let files = files(&script);
        let names: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, ["save.lua", "load.lua", "save_popup.lua", "save_saved.lua"]);
        let save = &files[0].1;
        for part in [
            // A click asks *conv_sg_slot, then saves at once or opens the slot's window.
            "local act = {{\"set\", \"%103\", slot=true}, {\"gosub\", \"*conv_sg_slot\"}, {\"save\", when={\"%105\", 1}}, {\"open\", \"save_saved\", when={\"%105\", 1}}, {\"open\", \"save_popup\", when={\"%105\", 2}}}",
            // Page 1 its slots; page 2 the quick save alone (890 = first + 1).
            "with(slots, {page=1}),",
            "with(slots, {page=2, first=889, rows=1, cols=1, new=false}),",
            // The lock marks: slot 20 (page 1's last, down the columns).
            "{\"image\", file=\"sys/locked.png\", x=28, y=432, page=1, when={\"%1030\", 1}},",
            "action={\"page\", \"2\", fade=300}",
        ] {
            assert!(save.contains(part), "missing {part:?} in\n{save}");
        }
        assert!(!save.contains("\"toggle\"") && !save.contains("\"-1\""), "no delete mode or previous page: {save}");
        let popup = &files[2].1;
        assert!(popup.contains("local i = quick and 0 or (slot - 1) % 20\nlocal x, y = 600 + -190 * (i // 5), 34 + 100 * (i % 5)"), "{popup}");
        assert!(popup.contains("{{\"set\", \"%\" .. (1010 + slot), 1 - locked}, {\"close\"}}"), "{popup}");
        let mut out = String::new();
        labels(&mut out, &g);
        assert!(out.contains("*conv_sg_slot\nsavefileexist %104,%103\n"), "{out}");
        assert!(out.contains("if %104 == 0 && %101 == 1 && %103 < 890 mov %105,1\n"), "{out}");
    }
}
