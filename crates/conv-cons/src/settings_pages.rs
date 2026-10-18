//! A settings screen made of a game's own pictures ([`PagedSettings`]),
//! drawn by the engine (`screens/settings.lua`, COMMANDS.md "screen"):
//! pages with their rows' words drawn in, switched with ◀ ▶; each row an
//! option list (the current option lit) or a level bar whose knob is
//! dragged (a picture slider with a fill) or stepped with ◀ ▶. It sets the same
//! variables as the text screen (`menu/settings.rs`) and runs its
//! `*conv_settings_apply` routine at each change.

use std::fmt::Write as _;

use conv_core::ir::{ConfirmDialog, PagedSettings, Placed, RowControl, SettingValue, SettingsMenu, Switch};

use crate::menu::{hidden_while_open, setting_var, settings_defaults, switch_off, IN_GAME, SETTINGS_APPLY, SKIP_UNREAD, VOICE_OFF_BASE};
use crate::screens::{image, lua, question};

/// The variables a value lives in (set together), and whether they are 1
/// for a 0 option (`border`: the game's globals' first number).
fn vars(v: &SettingValue, border: u32) -> (Vec<String>, bool) {
    match v {
        SettingValue::Level(s) => (vec![setting_var(*s).to_string()], false),
        // The globals are 1 while off.
        SettingValue::Switch(s) => (vec![switch_off(*s)], true),
        SettingValue::SkipUnread => (vec![SKIP_UNREAD.to_string()], false),
        SettingValue::Voices(c) => (c.iter().map(|c| format!("%{}", VOICE_OFF_BASE + c)).collect(), true),
        SettingValue::Language(var) => (vec![crate::var_name(*var, border)], false),
    }
}

/// Sets `v` to `value`.
fn set(v: &SettingValue, value: i32) -> String {
    let (vars, inverted) = vars(v, 0);
    vars.iter().map(|var| format!("mov {var},{}", if inverted { 1 - value } else { value })).collect::<Vec<_>>().join(":")
}

/// `var=...` of a widget.
fn var_field(v: &SettingValue, border: u32) -> String {
    let (vars, _) = vars(v, border);
    match &vars[..] {
        [one] => lua(one),
        all => format!("{{{}}}", all.iter().map(|v| lua(v)).collect::<Vec<_>>().join(", ")),
    }
}

/// `{file=..., x=..., y=...}`, a slider's part.
fn part(p: &Placed, extra: &str) -> String {
    format!("{{file={}, x={}, y={}{extra}}}", lua(&p.image), p.x, p.y)
}

/// `screens/settings.lua`.
/// `border`: the game's globals' first number.
pub(crate) fn lua_file(p: &PagedSettings, confirm: Option<&ConfirmDialog>, leave: Option<(&str, &str)>, border: u32) -> String {
    let pages = p.pages.len().max(1);
    let apply = format!("action={{\"gosub\", \"*{SETTINGS_APPLY}\"}}");
    // A language picked in game closes the screen (SystemPage::Settings).
    let language = format!("action=in_game and {{{{\"gosub\", \"*{SETTINGS_APPLY}\"}}, {{\"close\", 3}}}} or {{\"gosub\", \"*{SETTINGS_APPLY}\"}}");
    // The pages ◀ ▶ turn between: all, or with tabs those of the tab.
    let span = |n: usize| p.tabs.iter().find(|t| (t.pages.0..=t.pages.1).contains(&n)).map_or((0, pages - 1), |t| t.pages);
    let mut w = Vec::new();
    for (n, page) in p.pages.iter().enumerate() {
        let on = format!(", page={}", n + 1);
        w.push(image("image", &page.base, &on));
        let (first, last) = span(n);
        let count = last - first + 1;
        // Round from the last to the first and back; with tabs not (the
        // first's ◀ and the last's ▶ left out).
        let at = |k: usize| first + k % count + 1;
        if count > 1 && (p.tabs.is_empty() || n > first) {
            w.push(image("button", &p.prev, &format!("{on}, cells=2, action={{\"page\", {}}}", at(n - first + count - 1))));
        }
        if count > 1 && (p.tabs.is_empty() || n < last) {
            w.push(image("button", &p.next, &format!("{on}, cells=2, action={{\"page\", {}}}", at(n - first + 1))));
        }
        for c in &page.rows {
            match c {
                RowControl::Choice { value, options } => {
                    let inverted = vars(value, border).1;
                    let act = if matches!(value, SettingValue::Language(_)) { &language } else { &apply };
                    for (v, o, current) in options {
                        let v = if inverted { 1 - v } else { *v };
                        w.push(image("radio", o, &format!("{on}, cells=2, on_file={}, var={}, value={v}, {act}", lua(current), var_field(value, border))));
                    }
                }
                RowControl::Toggle { value, on: checked_value, button, checked } => {
                    let v = if vars(value, border).1 { 1 - checked_value } else { *checked_value };
                    w.push(image("toggle", button, &format!("{on}, cells=2, on_file={}, var={}, on={v}, off={}, {apply}", lua(checked), var_field(value, border), 1 - v)));
                }
                RowControl::Slider { value, knob, travel, fill, bar, minus, plus } => {
                    let var = var_field(value, border);
                    if let Some(bar) = bar {
                        w.push(image("image", bar, &on));
                    }
                    // Presses anywhere along the fill line, between ◀ and ▶.
                    let left = knob.x + knob.w / 2 + travel.min(&0) - 8;
                    w.push(format!(
                        "{{\"slider\", var={var}, min=0, max=10, travel={travel}, knob={}, fill={}, track={{x={left}, y={}, w={}, h={}}},\n      minus={}, plus={}{on}, {apply}}}",
                        part(knob, ""),
                        part(fill, ""),
                        knob.y,
                        travel.abs() + 17,
                        knob.h,
                        part(minus, ", cells=2"),
                        part(plus, ", cells=2"),
                    ));
                }
            }
        }
    }
    for t in &p.tabs {
        let (a, b) = (t.pages.0 + 1, t.pages.1 + 1);
        w.push(image("button", &t.button, &format!(", cells=2, current={{{a}, {b}}}, selected={}, action={{\"page\", {a}}}", lua(&t.selected))));
    }
    w.push(image("button", &p.close, ", cells=2, action={\"close\"}"));
    w.push(image("button", &p.defaults, ", cells=2, action={\"gosub\", \"*conv_settings_defaults\"}"));
    let mut out = format!("-- The settings (conscripter-convert, from PagedSettings).\nlocal in_game = var({}) == 1\nlocal w = {{\n", lua(IN_GAME));
    for x in w {
        writeln!(out, "  {x},").unwrap();
    }
    // Back to the title (or, during a replay, to its list) and quit ask
    // on `screens/confirm.lua` over the screen, then close it.
    let ask = |q: Option<&String>, s: Switch| q.map(|q| format!(", confirm={}", question(q, s))).unwrap_or_default();
    let title = ask(confirm.map(|c| &c.title), Switch::ConfirmTitle);
    let button = |p: &Placed, value: u32, q: &str| image("button", p, &format!(", cells=2, action={{\"close\", {value}{q}}}"));
    writeln!(out, "}}\nif in_game then").unwrap();
    match (leave, &p.title) {
        (_, None) => {}
        (Some((_, text)), Some(t)) => {
            let leave_q = ask(confirm.map(|_| text.to_string()).as_ref(), Switch::ConfirmTitle);
            writeln!(out, "  if var({}) == 1 then\n    w[#w+1] = {}\n  else\n    w[#w+1] = {}\n  end", lua(crate::replay::REPLAYING), button(t, 1, &leave_q), button(t, 1, &title)).unwrap();
        }
        (None, Some(t)) => writeln!(out, "  w[#w+1] = {}", button(t, 1, &title)).unwrap(),
    }
    if let Some(q) = &p.quit {
        writeln!(out, "  w[#w+1] = {}", button(q, 2, &ask(confirm.map(|c| &c.quit), Switch::ConfirmQuit))).unwrap();
    }
    writeln!(out, "end").unwrap();
    let hidden: Vec<String> = hidden_while_open().iter().map(|s| if s.starts_with('%') { format!("var({})", lua(s)) } else { s.clone() }).collect();
    writeln!(
        out,
        "return {{\n  kind = \"settings\",\n  pages = {pages},\n  hide = in_game and {{window=true, sprites={{{}}}}} or nil,\n  widgets = w,\n}}",
        hidden.join(", ")
    )
    .unwrap();
    out
}

/// `*conv_settings_defaults`: every setting back to its default (the
/// screen's labels are [`crate::screens::settings_labels`]).
pub(crate) fn labels(out: &mut String, m: &SettingsMenu, p: &PagedSettings) {
    // The levels' defaults and those of the values on the pages.
    let mut defaults = vec![settings_defaults(m)];
    for c in p.pages.iter().flat_map(|page| &page.rows) {
        let (RowControl::Choice { value: v, .. } | RowControl::Slider { value: v, .. } | RowControl::Toggle { value: v, .. }) = c;
        match v {
            SettingValue::Switch(s) => defaults.push(set(v, i32::from(!m.switches_off.contains(s)))),
            SettingValue::Voices(_) => defaults.push(set(v, 1)),
            SettingValue::SkipUnread => defaults.push(set(v, 0)),
            // The language stays as picked.
            SettingValue::Level(_) | SettingValue::Language(_) => {}
        }
    }
    writeln!(out, "*conv_settings_defaults\n{}:gosub *{SETTINGS_APPLY}\nreturn", defaults.join(":")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{Setting, SettingsPage};

    fn placed(image: &str, x: i32) -> Placed {
        Placed { image: image.into(), x, y: 100, w: 20, h: 10 }
    }

    #[test]
    fn values_set() {
        assert_eq!(set(&SettingValue::Switch(Switch::SceneSkipDialog), 0), "mov %2919,1");
        assert_eq!(set(&SettingValue::Voices(vec![3, 5]), 0), "mov %973,1:mov %975,1");
        assert_eq!(var_field(&SettingValue::Voices(vec![3, 5]), 0), "{\"%973\", \"%975\"}");
    }

    #[test]
    fn rows_become_widgets() {
        let slider = RowControl::Slider {
            value: SettingValue::Level(Setting::WindowTransparency),
            knob: placed("k.png", 764),
            travel: -268,
            fill: placed("e.png", 509),
            bar: None,
            minus: placed("p.png", 785),
            plus: placed("n.png", 484),
        };
        let choice = RowControl::Choice { value: SettingValue::Switch(Switch::SceneSkipDialog), options: vec![(1, placed("on.png", 500), "on_lit.png".into())] };
        let page = |rows| SettingsPage { base: placed("base.png", 0), rows };
        let p = PagedSettings {
            pages: vec![page(vec![choice]), page(vec![slider])],
            prev: placed("prev.png", 0),
            next: placed("next.png", 900),
            close: placed("close.png", 0),
            defaults: placed("defaults.png", 0),
            title: Some(placed("title.png", 0)),
            quit: Some(placed("quit.png", 0)),
            tabs: Vec::new(),
        };
        let lua = lua_file(&p, None, None, 0);
        // The switch's global is 1 while off: option 1 is value 0.
        assert!(lua.contains("{\"radio\", file=\"on.png\", x=500, y=100, page=1, cells=2, on_file=\"on_lit.png\", var=\"%2919\", value=0,"), "{lua}");
        assert!(lua.contains("knob={file=\"k.png\", x=764, y=100}, fill={file=\"e.png\", x=509, y=100}, track="), "{lua}");
        // The track from ◀ to ▶: the knob's centre at level 10 (506) minus 8.
        assert!(lua.contains("track={x=498, y=100, w=285, h=10}"), "{lua}");
        assert!(lua.contains("action={\"page\", 1}"));
    }
}
