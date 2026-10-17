//! Menus the engine draws (CONscripter's `screen`, COMMANDS.md "Screens"):
//! the save and load menus and the yes/no dialog as Lua tables in
//! `screens/`, for a [`SaveMenu`] with `engine_screen`.

use std::fmt::Write as _;

use conv_core::ir::{ConfirmDialog, Placed, SaveMenu, Script, Switch};

use crate::menu::{hidden_while_open, switch_off, CHAPTER, IN_GAME, PAGE, SCENE_LOAD_SLOT, SELECTED};

/// The save menu is the engine's.
pub(crate) fn engine_menu(script: &Script) -> Option<&SaveMenu> {
    script.save_menu.as_ref().filter(|m| m.engine_screen).filter(|_| script.save_grid.is_none())
}

/// The files under `screens/` (name, UTF-8 Lua), empty when the game has
/// no menu the engine draws.
pub fn screen_files(script: &Script) -> Vec<(String, String)> {
    let Some(m) = engine_menu(script) else { return Vec::new() };
    let scene_stack = script.button_bar.as_ref().is_some_and(|b| b.top.is_some());
    let mut files = vec![
        ("save.lua".to_string(), save_lua(m, script.confirm.as_ref(), scene_stack, true)),
        ("load.lua".to_string(), save_lua(m, script.confirm.as_ref(), scene_stack, false)),
    ];
    if let Some(d) = &script.confirm {
        files.push(("confirm.lua".to_string(), confirm_lua(d)));
    }
    files
}

/// `*conv_save` and `*conv_load`: the screens. Slots that builds before
/// `delsavegame` marked deleted (a global flag a slot) are deleted first.
pub(crate) fn save_labels(out: &mut String, m: &SaveMenu) {
    let slots = m.rows as usize * m.pages.len();
    let (slot, tmp, num, file) = (crate::menu::SLOT, crate::menu::TMP, crate::menu::STR_TMP, crate::menu::FILE);
    for (label, name) in [(crate::menu::SAVE, "save"), (crate::menu::LOAD, "load")] {
        writeln!(out, "*{label}\ngosub *conv_sl_flags\nscreen \"{name}\"\nreturn").unwrap();
    }
    writeln!(out, "*conv_sl_flags\nfor {slot}=1 to {slots}").unwrap();
    writeln!(out, "mov {tmp},{slot}+{}", crate::menu::DELETED_BASE).unwrap();
    writeln!(
        out,
        // %%105: the variable whose number %105 holds
        "if %{tmp} == 1 itoa {num},{slot}:mov {file},\"savethumb\":add {file},{num}:add {file},\".bmp\":delsavegame {slot},{file}:mov %{tmp},0"
    )
    .unwrap();
    writeln!(out, "next\nreturn").unwrap();
}

/// A Lua string.
fn lua(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn image(kind: &str, p: &Placed, extra: &str) -> String {
    format!("{{\"{kind}\", file={}, x={}, y={}{extra}}}", lua(&p.image), p.x, p.y)
}

/// A question when the switch is on: `var(off) == 0 and "..." or nil`.
fn question(text: &str, s: Switch) -> String {
    format!("var({}) == 0 and {} or nil", lua(&switch_off(s)), lua(text))
}

fn save_lua(m: &SaveMenu, confirm: Option<&ConfirmDialog>, scene_stack: bool, saving: bool) -> String {
    let kind = if saving { "save" } else { "load" };
    let pages = &m.pages[..m.pages.len().min(10)];
    let mut w = Vec::new();
    w.push(image("image", &m.base, &format!(", alpha={}", m.base_opacity)));
    w.push(image("image", &m.frame, ""));
    w.push(image("image", if saving { &m.save_header } else { &m.load_header }, ""));
    for (i, p) in pages.iter().enumerate() {
        w.push(image("image", &Placed { x: p.marker_x, y: p.marker_y, ..m.page_marker.clone() }, &format!(", page={}", i + 1)));
    }
    let ask = |text: &dyn Fn(&ConfirmDialog) -> &str, s: Switch| confirm.map(|d| format!(", confirm={}", question(&format!("No.%N {}", text(d)), s))).unwrap_or_default();
    let act = if saving {
        let keep = if scene_stack { ", {\"gosub\", \"*conv_scene_keep\"}" } else { "" };
        format!("{{{{\"set\", {}, slot=true}}, {{\"save\"{}}}{keep}}}", lua(SELECTED), ask(&|d| &d.overwrite, Switch::ConfirmSave))
    } else {
        let at = if scene_stack { format!("{{\"set\", {}, slot=true}}, ", lua(SCENE_LOAD_SLOT)) } else { String::new() };
        format!("{{{at}{{\"load\"{}}}}}", ask(&|d| &d.load, Switch::ConfirmLoad))
    };
    let size = m.font_size;
    let cols = if m.text_width > 0 { (m.text_width / size.max(1)).max(1) } else { 40 };
    let line = |dy: i32, text: &str, empty: &str| format!("{{x={}, y={}, size={size}, cols={cols}, shadow=true, text={}, empty={}}}", m.text_x, m.text_y + dy, lua(text), lua(empty));
    let preview = format!("var({}) == 0 and {{x={}, y={}, w={}, h={}}} or nil", lua(&switch_off(Switch::SavePreview)), m.preview.x, m.preview.y, m.preview.w, m.preview.h);
    let t = m.thumbnail;
    // The date is the save file's; the text is "chapter" (older builds:
    // "No.NN - date|chapter"), the chapter its last part. Spaces at even
    // places are full-width: ONScripter drops a half-width one starting a cell.
    w.push(format!(
        "{{\"saveslots\", x=0, y={}, rows={}, dy={}, first=1, pages={}, page_var={}, select=true, action=act,\n    button={{file={}, cells=2, x={}, y={}}},\n    selected={{file={}, opaque=true, blend=\"add\", x={}, y={}}},\n    thumb={{x={}, y={}, w={}, h={}}}, preview={preview}, shot={{{}, {}}},\n    new={},\n    save_text={}, split=\"|\",\n    lines={{{}, {}}}}}",
        m.row_y,
        m.rows.min(5),
        m.row_pitch,
        pages.len(),
        lua(PAGE),
        lua(&m.row_hover.image),
        m.row_hover.x,
        m.row_hover.y,
        lua(&m.row_selected.image),
        m.row_selected.x,
        m.row_selected.y,
        t.x,
        t.y,
        t.w,
        t.h,
        m.preview.w,
        m.preview.h,
        image("image", &m.new_icon, "").replacen("{\"image\", ", "{", 1),
        lua(CHAPTER),
        line(0, "No.%N - %Y/%m/%d　%H:%M:%S", "No.%N - no　data"),
        line(m.line_height, "%T-1", ""),
    ));
    for (i, p) in pages.iter().enumerate() {
        w.push(image("button", &p.button, &format!(", cells=3, disabled=2, action={{\"page\", {}}}", i + 1)));
    }
    let drop = if scene_stack { ", {\"gosub\", \"*conv_scene_drop\"}" } else { "" };
    w.push(image(
        "button",
        &m.delete,
        &format!(", cells=3, disabled=2, action={{{{\"set\", {}, slot=true}}, {{\"delete\"{}}}{drop}}}", lua(SELECTED), ask(&|d| &d.delete, Switch::ConfirmDelete)),
    ));
    w.push(image("button", if saving { &m.save } else { &m.load }, ", cells=3, disabled=2, action=act"));
    w.push(image("button", &m.close, ", cells=3, disabled=2, action={\"close\"}"));

    // Hidden while open in game: the bars, the name plate (a handle) and the link words.
    let hidden: Vec<String> = hidden_while_open().iter().map(|s| if s.starts_with('%') { format!("var({})", lua(s)) } else { s.clone() }).collect();
    let filter = m.backdrop.map(|(slot, f)| format!("{slot}, {}", f.map(|n| n.to_string()).join(", ")));
    let mut out = format!("-- The {kind} menu (conscripter-convert, from SaveMenu).\nlocal in_game = var({}) == 1\nlocal act = {act}\nreturn {{\n  kind = \"{kind}\",\n", lua(IN_GAME));
    writeln!(out, "  hide = in_game and {{window=true, sprites={{{}}}}} or nil,", hidden.join(", ")).unwrap();
    if let Some(f) = filter {
        writeln!(out, "  filter = in_game and {{{f}}} or nil,").unwrap();
    }
    out.push_str("  widgets = {\n");
    for x in w {
        writeln!(out, "    {x},").unwrap();
    }
    out.push_str("  },\n}\n");
    out
}

/// The yes/no dialog: its base, the question (`arg`) centred, the buttons
/// and their labels. Close gives 1 for yes.
// shortcut: the dialog with a picture per question, its fade and unroll
// aren't drawn (no game with them uses the engine's menu yet).
fn confirm_lua(d: &ConfirmDialog) -> String {
    let cells = if d.pressed { 4 } else { 3 };
    let pressed = if d.pressed { ", pressed=3" } else { "" };
    let label_size = d.yes.h as u32 * 2 / 3;
    let mut out = String::from("-- The yes/no dialog (conscripter-convert, from ConfirmDialog); arg: the question.\nreturn {\n  kind = \"confirm\",\n  widgets = {\n");
    writeln!(out, "    {},", image("image", &d.base, "")).unwrap();
    writeln!(out, "    {{\"text\", text=arg, x={}, y={}, size={}, align=\"centre\", shadow=true}},", d.base.x + d.base.w / 2, d.base.y + d.text_y, d.font_size).unwrap();
    for (p, label, value) in [(&d.yes, &d.yes_label, 1), (&d.no, &d.no_label, 0)] {
        writeln!(out, "    {},", image("button", p, &format!(", cells={cells}{pressed}, action={{\"close\", {value}}}"))).unwrap();
        let chars = label.chars().count() as i32;
        let x = p.x + p.w / (2 * cells) - chars * label_size as i32 / 2;
        let y = p.y + (p.h - label_size as i32) / 2;
        writeln!(out, "    {{\"text\", text={}, x={x}, y={y}, size={label_size}, cols=10, shadow=true}},", lua(label)).unwrap();
    }
    out.push_str("  },\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_strings_escape() {
        assert_eq!(lua("a\\b\"c"), "\"a\\\\b\\\"c\"");
    }
}
