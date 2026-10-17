//! Menus the engine draws (CONscripter's `screen`, COMMANDS.md "Screens"):
//! the save and load menus, the settings and the yes/no dialog as Lua
//! tables in `screens/`, for a [`SaveMenu`], and a [`SettingsMenu`] with
//! `engine_screen`.

use std::fmt::Write as _;

use conv_core::ir::{Backlog, ConfirmDialog, Placed, SaveMenu, Script, SettingsMenu, Switch};

use crate::menu::{hidden_while_open, switch_off, CHAPTER, IN_GAME, PAGE, SCENE_LOAD_SLOT, SELECTED};

/// The save menu is the engine's.
pub(crate) fn engine_menu(script: &Script) -> Option<&SaveMenu> {
    script.save_menu.as_ref().filter(|_| script.save_grid.is_none())
}

/// The settings are the engine's.
pub(crate) fn engine_settings(script: &Script) -> Option<&SettingsMenu> {
    script.settings.as_ref().filter(|m| m.engine_screen && m.pictures.is_none()).filter(|_| script.picture_settings.is_none())
}

/// The files under `screens/` (name, UTF-8 Lua), empty when the game has
/// no menu the engine draws.
pub fn screen_files(script: &Script) -> Vec<(String, String)> {
    let mut files = Vec::new();
    if let Some(m) = engine_menu(script) {
        let scene_stack = script.button_bar.as_ref().is_some_and(|b| b.top.is_some());
        files.push(("save.lua".to_string(), save_lua(m, script.confirm.as_ref(), scene_stack, true)));
        files.push(("load.lua".to_string(), save_lua(m, script.confirm.as_ref(), scene_stack, false)));
    }
    let has_replay = script.title_screen.is_some() && script.extras.as_ref().is_some_and(|e| e.replay.is_some());
    let leave = script.extras.as_ref().and_then(|e| e.replay.as_ref()).filter(|_| has_replay).map(|r| (r.leave_label.as_str(), r.leave_question.as_str()));
    if let Some(m) = engine_settings(script) {
        let version = script.version.as_deref().map(|v| (v, script.height));
        files.push(("settings.lua".to_string(), settings_lua(m, script.confirm.as_ref(), leave, version)));
    }
    if let Some(p) = script.settings.as_ref().and_then(|m| m.pictures.as_deref()).filter(|_| script.picture_settings.is_none()) {
        files.push(("settings.lua".to_string(), crate::settings_pages::lua_file(p, script.confirm.as_ref(), leave)));
    }
    if let Some(b) = script.backlog.as_ref().filter(|_| script.line_log.is_none()) {
        files.push(("backlog.lua".to_string(), backlog_lua(b)));
    }
    if let Some(e) = script.extras.as_ref().filter(|_| script.title_screen.is_some()) {
        files.extend(crate::extras_screens::files(e));
    }
    files.extend(crate::system_menu::files(script));
    if let Some(d) = script.confirm.as_ref().filter(|_| !files.is_empty()) {
        files.push(("confirm.lua".to_string(), confirm_lua(d)));
    }
    files.extend(crate::save_grid_screen::files(script));
    files.extend(crate::line_log_screen::files(script));
    if let Some(e) = script.picture_extras.as_ref().filter(|_| script.title_screen.is_some()) {
        files.push(("extras.lua".to_string(), crate::picture_extras::lua_file(e, script.global_border.unwrap_or(crate::GLOBAL_VAR_BASE))));
    }
    if let Some(g) = script.picture_extras.as_ref().filter(|_| script.title_screen.is_some()).and_then(|e| e.gallery.as_ref()).filter(|g| g.engine_screen) {
        files.push(("gallery.lua".to_string(), crate::gallery_screen::lua_file(g, script.global_border.unwrap_or(crate::GLOBAL_VAR_BASE))));
    }
    if let Some(r) = script.picture_extras.as_ref().filter(|_| script.title_screen.is_some()).and_then(|e| e.replay.as_ref()).filter(|r| r.engine_screen) {
        files.push(("replay.lua".to_string(), crate::picture_replay::lua_file(r, script.global_border.unwrap_or(crate::GLOBAL_VAR_BASE))));
    }
    if let Some(r) = script.picture_extras.as_ref().filter(|_| script.title_screen.is_some()).and_then(|e| e.sound.as_ref()).filter(|r| r.engine_screen) {
        files.push(("sound.lua".to_string(), crate::sound_screen::lua_file(r, script.global_border.unwrap_or(crate::GLOBAL_VAR_BASE))));
    }
    files.extend(crate::settings_screen::files(script, script.global_border.unwrap_or(crate::GLOBAL_VAR_BASE)));
    files
}

/// `*conv_settings`: the screen; it closes with 1 for back to the title
/// (during a scene replay: back to its list), 2 for quit.
pub(crate) fn settings_labels(out: &mut String, replay: bool) {
    let b = crate::BUTTON_VAR;
    writeln!(out, "*{}\nscreen \"settings\",{b}", crate::menu::SETTINGS).unwrap();
    if replay {
        writeln!(out, "if {b} == 1 && {} == 1 goto *{}", crate::replay::REPLAYING, crate::replay::END).unwrap();
    }
    writeln!(out, "if {b} == 1 reset\nif {b} == 2 end\nreturn").unwrap();
}

/// `*conv_save` and `*conv_load`: the screens.
pub(crate) fn save_labels(out: &mut String) {
    for (label, name) in [(crate::menu::SAVE, "save"), (crate::menu::LOAD, "load")] {
        writeln!(out, "*{label}\nscreen \"{name}\"\nreturn").unwrap();
    }
}

/// A Lua string.
pub(crate) fn lua(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

pub(crate) fn image(kind: &str, p: &Placed, extra: &str) -> String {
    format!("{{\"{kind}\", file={}, x={}, y={}{extra}}}", lua(&p.image), p.x, p.y)
}

/// A question when the switch is on: `var(off) == 0 and "..." or nil`.
pub(crate) fn question(text: &str, s: Switch) -> String {
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

/// The settings screen, as the script one (`menu::settings_menu`) draws
/// it: text rows with －/＋ and a bar of ■□, switches as "■　label", the
/// voices in two columns, tabs for the pages, and in game back to the
/// title and quit.
fn settings_lua(m: &SettingsMenu, confirm: Option<&ConfirmDialog>, leave: Option<(&str, &str)>, version: Option<(&str, u32)>) -> String {
    use crate::menu::{setting_var, SETTINGS_APPLY, SKIP_UNREAD, VOICE_OFF_BASE};
    let rows = &m.rows[..m.rows.len().min(6)];
    let voices = &m.voices[..m.voices.len().min(15)];
    let switches = &m.switches[..m.switches.len().min(15)];
    let pages = m.tab_labels.len().min(1 + usize::from(!voices.is_empty()) + usize::from(!switches.is_empty()));
    let size = m.font_size;
    let apply = format!("{{\"gosub\", \"*{SETTINGS_APPLY}\"}}");
    // "strsp" drops a line's last character unless there is a column to spare.
    let text = |chars: usize| format!("size={size}, cols={}, shadow=true", chars + 1);
    let hover = "color=\"#FFFFFF\", hover=\"#F0C040\"";
    let mut w = Vec::new();
    w.push(image("image", &m.base, &format!(", alpha={}", m.base_opacity)));
    w.push(image("image", &m.header, ""));
    w.push(image("button", &m.close, ", cells=3, action={\"close\"}"));
    if let Some((line, height)) = version {
        let t = crate::sprite_text(line);
        w.push(format!("{{\"text\", text={}, x=8, y={}, size=14, cols={}, shadow=true, color=\"#B0B0B0\"}}", lua(&t), height - 22, t.chars().count().div_ceil(2) + 1));
    }
    if pages > 1 {
        let mut x = m.tabs_x;
        for (page, label) in m.tab_labels.iter().take(pages).enumerate() {
            let chars = label.chars().count();
            w.push(format!("{{\"button\", text={}, x={x}, y={}, {}, {hover}, disabled_color=\"#F0C040\", action={{\"page\", {}}}}}", lua(label), m.tabs_y, text(chars), page + 1));
            x += (chars as i32 + 2) * (size as i32 + 1);
        }
    }
    let on_page = |p: usize| if pages > 1 { format!(", page={p}") } else { String::new() };
    for (r, row) in rows.iter().enumerate() {
        let y = m.y + r as i32 * m.pitch;
        // "ＢＧＭ　…　■■■■■■■□□□　７": the label padded with full-width
        // spaces up to the level's column (minus sits over the padding).
        let pad = ((m.level_x - m.label_x) / (size as i32 + 1)).max(row.label.chars().count() as i32 + 1) as usize;
        let label = format!("{}{}%B　%V", row.label, "　".repeat(pad - row.label.chars().count()));
        w.push(format!(
            "{{\"slider\", var={}, min=0, max=10, text={}, x={}, y={y}, size={size}, cols={}, shadow=true{},\n      minus={{text=\"－\", x={}, y={y}, {}, {hover}}}, plus={{text=\"＋\", x={}, y={y}, {}, {hover}}}, action={apply}}}",
            lua(setting_var(row.setting)),
            lua(&label),
            m.label_x,
            pad + 14,
            on_page(1),
            m.minus_x,
            text(1),
            m.plus_x,
            text(1),
        ));
    }
    if let Some(label) = &m.skip_unread_label {
        w.push(format!(
            "{{\"toggle\", var={}, text={}, x={}, y={}, {}, {hover}{}, action={apply}}}",
            lua(SKIP_UNREAD),
            lua(&format!("%C　{label}")),
            m.label_x,
            m.y + rows.len() as i32 * m.pitch,
            text(label.chars().count() + 2),
            on_page(1)
        ));
    }
    let column_rows = voices.len().div_ceil(2);
    for (i, t) in voices.iter().enumerate() {
        let x = if i < column_rows { m.label_x } else { m.voice_column_x };
        let y = m.y + (i % column_rows) as i32 * m.voice_pitch;
        let vars: Vec<String> = t.characters.iter().map(|c| lua(&format!("%{}", VOICE_OFF_BASE + c))).collect();
        w.push(format!(
            "{{\"toggle\", var={{{}}}, on=0, off=1, text={}, x={x}, y={y}, {}, {hover}{}}}",
            vars.join(", "),
            lua(&format!("%C　{}", t.label)),
            text(t.label.chars().count() + 2),
            on_page(2)
        ));
    }
    for (i, sw) in switches.iter().enumerate() {
        let y = m.y + i as i32 * m.voice_pitch;
        // A switch the apply routine puts into effect does so at once.
        let then = if crate::switches::apply(sw.switch) != crate::switches::Apply::Read { format!(", action={apply}") } else { String::new() };
        w.push(format!(
            "{{\"toggle\", var={}, on=0, off=1, text={}, x={}, y={y}, {}, {hover}{}{then}}}",
            lua(&switch_off(sw.switch)),
            lua(&format!("%C　{}", sw.label)),
            m.label_x,
            text(sw.label.chars().count() + 2),
            on_page(if voices.is_empty() { 2 } else { 3 })
        ));
    }
    let ask = |q: Option<&String>, s: Switch| q.map(|q| format!(", confirm={}", question(q, s))).unwrap_or_default();
    let title_q = ask(confirm.map(|c| &c.title), Switch::ConfirmTitle);
    let quit_q = ask(confirm.map(|c| &c.quit), Switch::ConfirmQuit);
    let mut out = format!("-- The settings (conscripter-convert, from SettingsMenu).\nlocal in_game = var({}) == 1\nlocal w = {{\n", lua(IN_GAME));
    for x in w {
        writeln!(out, "  {x},").unwrap();
    }
    out.push_str("}\nif in_game then\n");
    let exit = |label: &str, x: i32, value: u32, q: &str| {
        format!("{{\"button\", text={}, x={x}, y={}, size={size}, cols=21, shadow=true, {hover}, action={{\"close\", {value}{q}}}}}", lua(label), m.exits_y)
    };
    match leave {
        Some((label, question_text)) => {
            let leave_q = confirm.map(|_| format!(", confirm={}", question(question_text, Switch::ConfirmTitle))).unwrap_or_default();
            writeln!(out, "  if var({}) == 1 then\n    w[#w+1] = {}\n  else\n    w[#w+1] = {}\n  end", lua(crate::replay::REPLAYING), exit(label, m.label_x, 1, &leave_q), exit(&m.title_label, m.label_x, 1, &title_q)).unwrap();
        }
        None => writeln!(out, "  w[#w+1] = {}", exit(&m.title_label, m.label_x, 1, &title_q)).unwrap(),
    }
    writeln!(out, "  w[#w+1] = {}", exit(&m.quit_label, m.level_x, 2, &quit_q)).unwrap();
    let hidden: Vec<String> = hidden_while_open().iter().map(|s| if s.starts_with('%') { format!("var({})", lua(s)) } else { s.clone() }).collect();
    writeln!(out, "end\nreturn {{\n  kind = \"settings\",{}\n  hide = in_game and {{window=true, sprites={{{}}}}} or nil,\n  widgets = w,\n}}", if pages > 1 { format!("\n  pages = {pages},") } else { String::new() }, hidden.join(", ")).unwrap();
    out
}

/// `*conv_backlog`: the screen; `*conv_pretext`, the pretextgosub that
/// keeps each page's `[name|voice]` tag (it only allows the save point).
pub(crate) fn backlog_labels(out: &mut String) {
    writeln!(out, "*{}\nsaveon\nreturn\n*{}\nscreen \"backlog\"\nreturn", crate::menu::PRETEXT, crate::menu::BACKLOG).unwrap();
}

/// The backlog: the log's
/// pages from the newest up in `area`, the speaker's name over each (with
/// a voice, a button replaying it), up and down a page, the wheel too
/// (down past the newest page closes, unless switched off).
fn backlog_lua(b: &Backlog) -> String {
    let a = b.area;
    let hidden: Vec<String> = hidden_while_open().iter().map(|s| if s.starts_with('%') { format!("var({})", lua(s)) } else { s.clone() }).collect();
    let mut out = format!("-- The backlog (conscripter-convert, from Backlog).\nlocal in_game = var({}) == 1\nreturn {{\n  kind = \"backlog\",\n", lua(IN_GAME));
    writeln!(out, "  hide = in_game and {{window=true, sprites={{{}}}}} or nil,", hidden.join(", ")).unwrap();
    writeln!(out, "  wheel_up = {{\"scroll\", \"+1\"}},").unwrap();
    writeln!(out, "  wheel_down = {{\"scroll\", \"-1\", at_end = var({}) == 0 and {{\"close\"}} or nil}},", lua(&switch_off(Switch::WheelClosesLog))).unwrap();
    out.push_str("  widgets = {\n");
    for w in [
        image("image", &b.base, &format!(", alpha={}", b.base_opacity)),
        image("image", &b.frame, ""),
        image("image", &b.track, ""),
        format!(
            "{{\"list\", source=\"log\", x={}, y={}, w={}, h={}, size={}, line_height={}, cols={}, shadow=true, name_color={}, hover=\"#FFFFFF\"}}",
            a.x,
            a.y,
            a.w,
            a.h,
            b.font_size,
            b.line_height,
            b.columns,
            lua(crate::menu::LOG_NAME_COLOR)
        ),
        image("button", &b.up, ", cells=3, disabled=2, action={\"scroll\", \"+1\"}"),
        image("button", &b.down, ", cells=3, disabled=2, action={\"scroll\", \"-1\"}"),
        image("button", &b.close, ", cells=3, action={\"close\"}"),
    ] {
        writeln!(out, "    {w},").unwrap();
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
    fn settings_become_widgets() {
        use conv_core::ir::{Setting, SettingRow, SettingSwitch, VoiceToggle};
        let placed = |name: &str| Placed { image: format!("system/{name}.png"), x: 10, y: 20, w: 60, h: 20 };
        let m = SettingsMenu {
            base: placed("base"),
            base_opacity: 210,
            header: placed("header"),
            close: placed("close"),
            rows: vec![SettingRow { label: "ＢＧＭ".into(), setting: Setting::BgmVolume }],
            label_x: 140,
            minus_x: 340,
            level_x: 380,
            plus_x: 680,
            y: 140,
            pitch: 70,
            font_size: 22,
            title_label: "title".into(),
            quit_label: "quit".into(),
            exits_y: 540,
            tab_labels: vec!["音量".into(), "音声".into(), "確認".into()],
            tabs_x: 300,
            tabs_y: 34,
            voices: vec![VoiceToggle { label: "その他".into(), characters: vec![0, 2] }],
            voice_column_x: 440,
            voice_pitch: 50,
            skip_unread_label: Some("未読".into()),
            switches: vec![SettingSwitch { label: "セーブ確認".into(), switch: Switch::ConfirmSave }],
            pictures: None,
            defaults: Vec::new(),
            switches_off: Vec::new(),
            engine_screen: true,
            text_ms: None,
        };
        let lua = settings_lua(&m, None, None, None);
        for part in [
            "pages = 3,",
            "{\"slider\", var=\"%902\", min=0, max=10, text=\"ＢＧＭ　　　　　　　%B　%V\", x=140, y=140,",
            "action={\"gosub\", \"*conv_settings_apply\"}",
            "{\"toggle\", var={\"%970\", \"%972\"}, on=0, off=1, text=\"%C　その他\"",
            "page=3",
            "{\"button\", text=\"quit\", x=380, y=540",
        ] {
            assert!(lua.contains(part), "{part}\n{lua}");
        }
        let mut out = String::new();
        settings_labels(&mut out, true);
        assert_eq!(out, "*conv_settings\nscreen \"settings\",%9\nif %9 == 1 && %152 == 1 goto *conv_replay_end\nif %9 == 1 reset\nif %9 == 2 end\nreturn\n");
    }

    #[test]
    fn backlog_is_a_list() {
        use conv_core::ir::{Backlog, Rect};
        let p = Placed { image: "system/x.png".into(), x: 760, y: 44, w: 30, h: 33 };
        let b = Backlog {
            base: p.clone(),
            base_opacity: 210,
            frame: p.clone(),
            track: p.clone(),
            up: p.clone(),
            down: p.clone(),
            close: p,
            area: Rect { x: 30, y: 44, w: 700, h: 516 },
            font_size: 16,
            line_height: 24,
            columns: 41,
        };
        let lua = backlog_lua(&b);
        for part in [
            "{\"list\", source=\"log\", x=30, y=44, w=700, h=516, size=16, line_height=24, cols=41,",
            "action={\"scroll\", \"+1\"}",
            "wheel_down = {\"scroll\", \"-1\", at_end = var(\"%3641\") == 0 and {\"close\"} or nil},",
        ] {
            assert!(lua.contains(part), "{part}\n{lua}");
        }
        let mut out = String::new();
        backlog_labels(&mut out);
        assert_eq!(out, "*conv_pretext\nsaveon\nreturn\n*conv_backlog\nscreen \"backlog\"\nreturn\n");
    }

    #[test]
    fn lua_strings_escape() {
        assert_eq!(lua("a\\b\"c"), "\"a\\\\b\\\"c\"");
    }
}
