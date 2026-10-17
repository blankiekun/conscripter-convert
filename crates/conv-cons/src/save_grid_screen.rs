//! [`SaveGrid`] drawn by the engine (`screen "save"`, `screen "load"`,
//! COMMANDS.md "Screens") when it has [`SaveGrid::engine_screen`]: the
//! page's picture, the frames, the slots (down the columns), each slot's
//! details shown while the pointer is on it (`hover`), previous and next
//! page (going round, cross-faded), the delete mode as a toggle and the
//! slots' actions taken by it (`when`), the game's own questions
//! (`save_ask_*`) and the notice after a save (`save_saved`).
//!
//! A save's text is "|play time|chapter|last line", made before the screen
//! opens ([`save_grid::details`]); the date is the save file's. A save of
//! the script routine's ("date|play time|…") shows the same, its parts
//! counted from the last.

use std::fmt::Write as _;

use conv_core::ir::{GridText, SaveGrid, Script, Switch};

use crate::menu::{switch_off, IN_GAME, LABEL, PAGE};
use crate::save_grid::{self, DELETING};
use crate::screens::{image, lua};

/// The questions asked: the switch, its screen.
const ASKS: [(Switch, &str); 3] = [(Switch::ConfirmSave, "save_ask_save"), (Switch::ConfirmLoad, "save_ask_load"), (Switch::ConfirmDelete, "save_ask_delete")];

/// The save grid is the engine's.
pub(crate) fn engine_grid(script: &Script) -> Option<&SaveGrid> {
    script.save_grid.as_ref().filter(|g| g.engine_screen)
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
}

/// The files under `screens/`.
pub(crate) fn files(script: &Script) -> Vec<(String, String)> {
    let Some(g) = engine_grid(script) else { return Vec::new() };
    assert!(g.popup.is_none() && g.tabs.is_empty() && !g.quick_page, "an engine save grid has no popup, tabs or quick page");
    let pictures = script.confirm.as_ref().map(|d| &d.pictures[..]).unwrap_or_default();
    let asks: Vec<(Switch, &str)> = ASKS.into_iter().filter(|(s, _)| pictures.iter().any(|(q, _)| q == s)).collect();
    let mut files = vec![("save.lua".to_string(), grid_lua(g, &asks, true)), ("load.lua".to_string(), grid_lua(g, &asks, false))];
    if let Some(d) = &script.confirm {
        for (s, name) in &asks {
            let picture = &pictures.iter().find(|(q, _)| q == s).expect("asked").1;
            files.push((format!("{name}.lua"), crate::settings_screen::question_lua(d, picture, name)));
        }
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

fn grid_lua(g: &SaveGrid, asks: &[(Switch, &str)], saving: bool) -> String {
    let kind = if saving { "save" } else { "load" };
    let ask = |s: Switch| {
        asks.iter().find(|(q, _)| *q == s).map(|(_, name)| format!(", dialog=var({}) == 0 and {} or nil", lua(&switch_off(s)), lua(name))).unwrap_or_default()
    };
    let deleting = |on: u32| format!("when={{{}, {on}}}", lua(DELETING));
    let act = if saving {
        let saved = if g.saved.is_some() { format!(", {{\"open\", \"save_saved\", {}}}", deleting(0)) } else { String::new() };
        format!("{{{{\"delete\", {}{}}}, {{\"save\", {}{}}}{saved}}}", deleting(1), ask(Switch::ConfirmDelete), deleting(0), ask(Switch::ConfirmSave))
    } else {
        format!("{{{{\"delete\", {}{}}}, {{\"load\", {}{}}}}}", deleting(1), ask(Switch::ConfirmDelete), deleting(0), ask(Switch::ConfirmLoad))
    };
    let pages = if saving { &g.save_pages } else { &g.load_pages };
    let mut w = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        w.push(format!("{{\"image\", file={}, opaque=true, x=0, y=0, page={}}}", lua(p), i + 1));
    }
    // The frames over the thumbnails, under the new mark and the hover frame.
    let over = g.frames.as_ref().map(|f| format!("\n      over={{file={}, x={}, y={}}},", lua(&f.image), f.x, f.y)).unwrap_or_default();
    let prefix = g.play_time_prefix.replace('%', "%%");
    let (tw, th) = (g.thumb_w, g.thumb_h);
    let p = g.preview;
    let (hx, hy) = g.hover_offset;
    let (nx, ny) = g.new_offset;
    w.push(format!(
        "{{\"saveslots\", x={}, y={}, dx={}, dy={}, rows={}, cols={}, down={}, pages={}, page_var={}, action=act,{over}\n      button={{file=\">{tw},{th},#000000\", opaque=true, alpha=0}},\n      thumb={{x=0, y=0, w={tw}, h={th}}}, shot={{{tw}, {th}}},\n      new={{file={}, x={nx}, y={ny}}},\n      save_text={}, split=\"|\",\n      hover={{image={{file={}, x={hx}, y={hy}}}, preview={{x={}, y={}, w={}, h={}}},\n        lines={{\n          {},\n          {},\n          {},\n          {}}}}}}}",
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
    ));
    let fade = if g.fade_ms > 0 { format!(", fade={}", g.fade_ms) } else { String::new() };
    w.push(image("button", &g.previous, &format!(", cells=4, pressed=3, action={{\"page\", \"-1\", round=true{fade}}}")));
    w.push(image("button", &g.next, &format!(", cells=4, pressed=3, action={{\"page\", \"+1\", round=true{fade}}}")));
    w.push(image(
        "toggle",
        &g.delete,
        &format!(", cells=4, pressed=3, on_file={}, on_cells=1, var={}, fade=150", lua(&g.delete_on.image), lua(DELETING)),
    ));
    w.push(image("button", &g.close, ", cells=4, pressed=3, action={\"close\"}"));

    let mut out = format!("-- The {kind} screen (conscripter-convert, from SaveGrid).\nlocal act = {act}\nreturn {{\n  kind = \"{kind}\",\n");
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

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{ConfirmDialog, Placed};

    #[test]
    fn grid_becomes_a_screen() {
        let mut g = crate::save_grid::tests::grid();
        g.engine_screen = true;
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
}
