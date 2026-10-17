//! [`SystemMenu`] drawn by the engine (COMMANDS.md "Screens"): tab n is
//! `screens/system<n>.lua`. A screen closes with 10 + n to show tab n
//! (held, so the next tab cross-fades in from it), 2 to go back to the
//! title, 0 to close; `*conv_system` opens the tab in
//! [`TAB`] and follows what it closed with.

use std::fmt::Write as _;

use conv_core::ir::{ConfirmDialog, GridText, SaveList, Script, Switch, SystemMenu, SystemPage};

use crate::menu::{CHAPTER, CONFIRM, IN_GAME, LABEL, LOAD, SAVE, STR_TMP};
use crate::screens::{image, lua, question};

/// The tab shown (1 up).
pub(crate) const TAB: &str = "%conv_system_tab";
/// The declaration of [`TAB`].
pub(crate) const DEFINE: &str = "defvar conv_system_tab";
const LABEL_SYSTEM: &str = "conv_system";

/// The script's system menu.
pub(crate) fn menu(script: &Script) -> Option<&SystemMenu> {
    script.system_menu.as_ref()
}

/// Its first save or load tab (1 up).
fn first(m: &SystemMenu, saving: bool) -> Option<usize> {
    m.tabs.iter().position(|t| matches!((&t.page, saving), (SystemPage::Save(_), true) | (SystemPage::Load(_), false))).map(|i| i + 1)
}

/// The menu has a load tab (the title's Load opens it).
pub(crate) fn loads(script: &Script) -> bool {
    menu(script).is_some_and(|m| first(m, false).is_some())
}

/// `*conv_save`, `*conv_load` (for the tabs there are) and `*conv_system`.
pub(crate) fn labels(out: &mut String, m: &SystemMenu, confirm: Option<&ConfirmDialog>) {
    for (label, saving) in [(SAVE, true), (LOAD, false)] {
        if let Some(n) = first(m, saving) {
            writeln!(out, "*{label}\nmov {TAB},{n}:goto *{LABEL_SYSTEM}").unwrap();
        }
    }
    let b = crate::BUTTON_VAR;
    writeln!(out, "*{LABEL_SYSTEM}").unwrap();
    // The save's text: "chapter|last line".
    writeln!(out, "mov {LABEL},{CHAPTER}:getlog {STR_TMP},0:add {LABEL},\"|\":add {LABEL},{STR_TMP}").unwrap();
    writeln!(out, "if {IN_GAME} == 1 textoff\n*{LABEL_SYSTEM}_tab").unwrap();
    for n in 1..=m.tabs.len() {
        writeln!(out, "if {TAB} == {n} screen \"system{n}\",{b}").unwrap();
    }
    writeln!(out, "if {b} >= 10 mov {TAB},{b}:sub {TAB},10:goto *{LABEL_SYSTEM}_tab").unwrap();
    if m.tabs.iter().any(|t| t.page == SystemPage::Title) {
        if let Some(c) = confirm {
            writeln!(out, "if {b} == 2 {}", crate::menu::confirm_call(&c.title, Switch::ConfirmTitle).replace('\n', ":")).unwrap();
            writeln!(out, "if {b} == 2 && {CONFIRM} == 0 goto *{LABEL_SYSTEM}_tab").unwrap();
        }
        writeln!(out, "if {b} == 2 reset").unwrap();
    }
    writeln!(out, "if {IN_GAME} == 1 texton\nreturn").unwrap();
}

/// The files under `screens/`.
pub(crate) fn files(script: &Script) -> Vec<(String, String)> {
    let Some(m) = menu(script) else { return Vec::new() };
    (1..=m.tabs.len()).map(|n| (format!("system{n}.lua"), tab_lua(m, n, script.confirm.as_ref()))).collect()
}

/// A [`GridText`] as a line of the slots' (`text` for a save, `empty`
/// without one).
fn line(t: &GridText, text: &str, empty: &str) -> String {
    format!(
        "{{x={}, y={}, size={}, cols={}, rows={}, pitch={}, gap={}, color={}, text={}, empty={}}}",
        t.x,
        t.y,
        t.font_size,
        t.chars,
        t.lines,
        t.line_gap,
        t.char_gap,
        lua(&t.colour),
        lua(text),
        lua(empty)
    )
}

fn slots(l: &SaveList, saving: bool, confirm: Option<&ConfirmDialog>) -> String {
    let ask = |text: Option<&str>, s: Switch| text.map(|t| format!(", confirm={}", question(&format!("No.%N {t}"), s))).unwrap_or_default();
    let act = if saving {
        format!("{{\"save\"{}}}", ask(confirm.map(|c| c.overwrite.as_str()), Switch::ConfirmSave))
    } else {
        format!("{{\"load\"{}}}", ask(confirm.map(|c| c.load.as_str()), Switch::ConfirmLoad))
    };
    let t = l.thumb;
    let preview = l.preview.map(|p| format!(" preview={{x={}, y={}, w={}, h={}}},", p.x, p.y, p.w, p.h)).unwrap_or_default();
    format!(
        "{{\"saveslots\", x={}, y={}, dy={}, rows={}, cols=1, pages={}, page_var={}, action={act},\n      button={{file={}, cells=2, x={}, y={}}},\n      thumb={{x={}, y={}, w={}, h={}}},{preview} shot={{{}, {}}},\n      save_text={}, split=\"|\",\n      lines={{{}, {}, {}}}}}",
        l.x,
        l.y,
        l.row_step,
        l.rows,
        l.slots.div_ceil(l.rows.max(1)),
        lua(crate::menu::PAGE),
        lua(&l.row.image),
        l.row.x,
        l.row.y,
        t.x,
        t.y,
        t.w,
        t.h,
        t.w,
        t.h,
        lua(LABEL),
        line(&l.date, "%Y/%m/%d %H:%M", ""),
        line(&l.chapter, "%T1", ""),
        line(&l.line, "%T2", ""),
    )
}

fn tab_lua(m: &SystemMenu, n: usize, confirm: Option<&ConfirmDialog>) -> String {
    let page = &m.tabs[n - 1].page;
    let kind = match page {
        SystemPage::Save(_) => "save",
        SystemPage::Load(_) => "load",
        _ => "other",
    };
    let mut w: Vec<String> = m.backdrop.iter().map(|p| image("image", p, "")).collect();
    for (i, t) in m.tabs.iter().enumerate() {
        let extra = if i + 1 == n {
            ", cells=3, enabled=0, disabled=2".to_string()
        } else {
            match t.page {
                SystemPage::Unavailable => ", cells=3, enabled=0, disabled=0".to_string(),
                SystemPage::Title => ", cells=3, enabled=in_game, disabled=0, action={\"close\", 2}".to_string(),
                // Saving from the title would save the title.
                SystemPage::Save(_) => format!(", cells=3, enabled=in_game, disabled=0, action={{\"close\", {}, hold=true}}", 10 + i + 1),
                _ => format!(", cells=3, action={{\"close\", {}, hold=true}}", 10 + i + 1),
            }
        };
        w.push(image("button", &t.button, &extra));
    }
    let paged = match page {
        SystemPage::Save(l) => Some(slots(l, true, confirm)),
        SystemPage::Load(l) => Some(slots(l, false, confirm)),
        _ => None,
    };
    let mut out = format!("-- The system menu's tab {n} (conscripter-convert, from SystemMenu).\nlocal in_game = var({}) == 1\nreturn {{\n  kind = \"{kind}\",\n  hide = in_game and {{window=true}} or nil,\n", lua(IN_GAME));
    if m.fade_ms > 0 {
        writeln!(out, "  fade = {},", m.fade_ms).unwrap();
    }
    if let Some(s) = paged {
        out.push_str("  wheel_up = {\"page\", \"-1\"},\n  wheel_down = {\"page\", \"+1\"},\n");
        w.push(s);
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
    use conv_core::ir::{Placed, Rect, SystemTab};

    fn placed(image: &str) -> Placed {
        Placed { image: image.into(), x: 10, y: 20, w: 30, h: 40 }
    }

    pub(crate) fn sample() -> SystemMenu {
        let text = GridText { x: 1, y: 2, font_size: 20, lines: 1, chars: 30, line_gap: 0, char_gap: 0, colour: "#FFFFFF".into(), vertical: false };
        let list = SaveList { slots: 10, rows: 4, x: 100, y: 50, row_step: 120, row: placed("sys/row.png"), thumb: Rect { x: 5, y: 6, w: 160, h: 90 }, date: text.clone(), chapter: text.clone(), line: text, preview: None };
        let tab = |i: i32, page| SystemTab { button: Placed { y: 20 + 50 * i, ..placed("sys/tab.png") }, page };
        let panel = Placed { x: 0, y: 0, ..placed("sys/panel.png") };
        SystemMenu { backdrop: vec![panel], tabs: vec![tab(0, SystemPage::Save(list.clone())), tab(1, SystemPage::Load(list)), tab(2, SystemPage::Unavailable), tab(3, SystemPage::Title)], fade_ms: 200 }
    }

    #[test]
    fn tabs_become_screens() {
        let m = sample();
        let save = tab_lua(&m, 1, None);
        assert!(save.contains("kind = \"save\""), "{save}");
        // Its own tab current, the others close to theirs.
        assert!(save.contains("cells=3, enabled=0, disabled=2}"), "{save}");
        assert!(save.contains("action={\"close\", 12, hold=true}"), "{save}");
        assert!(save.contains("rows=4, cols=1, pages=3"), "{save}");
        assert!(save.contains("wheel_down = {\"page\", \"+1\"}"), "{save}");
        let title = tab_lua(&m, 4, None);
        assert!(!title.contains("saveslots") && title.contains("kind = \"other\""), "{title}");
        let mut out = String::new();
        labels(&mut out, &m, None);
        assert!(out.contains("*conv_save\nmov %conv_system_tab,1:goto *conv_system\n*conv_load\nmov %conv_system_tab,2:"), "{out}");
        assert!(out.contains("if %9 >= 10 mov %conv_system_tab,%9:sub %conv_system_tab,10:goto *conv_system_tab"), "{out}");
        assert!(out.contains("if %9 == 2 reset"), "{out}");
    }
}

