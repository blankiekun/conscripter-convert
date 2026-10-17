//! A settings screen made of a game's own pictures ([`PagedSettings`]),
//! drawn by the engine (`screens/settings.lua`, COMMANDS.md "screen"):
//! pages with their rows' words drawn in, switched with ◀ ▶; each row an
//! option list (the current option lit) or a level bar whose knob is
//! dragged (a picture slider with a fill) or stepped with ◀ ▶. It sets the same
//! variables as the text screen (`menu/settings.rs`) and runs its
//! `*conv_settings_apply` routine at each change.

use std::fmt::Write as _;

use conv_core::ir::{ConfirmDialog, PagedSettings, Placed, RowControl, SettingValue, SettingsMenu, Switch};

use crate::menu::{confirm_call, hidden_while_open, setting_var, settings_defaults, switch_off, CONFIRM, IN_GAME, SETTINGS_APPLY, SETTINGS_PAGE, SKIP_UNREAD, VOICE_OFF_BASE};
use crate::screens::{image, lua};
use crate::BUTTON_VAR;

/// The variables a value lives in (set together), and whether they are 1
/// for a 0 option.
fn vars(v: &SettingValue) -> (Vec<String>, bool) {
    match v {
        SettingValue::Level(s) => (vec![setting_var(*s).to_string()], false),
        // The globals are 1 while off.
        SettingValue::Switch(s) => (vec![switch_off(*s)], true),
        SettingValue::SkipUnread => (vec![SKIP_UNREAD.to_string()], false),
        SettingValue::Voices(c) => (c.iter().map(|c| format!("%{}", VOICE_OFF_BASE + c)).collect(), true),
    }
}

/// Sets `v` to `value`.
fn set(v: &SettingValue, value: i32) -> String {
    let (vars, inverted) = vars(v);
    vars.iter().map(|var| format!("mov {var},{}", if inverted { 1 - value } else { value })).collect::<Vec<_>>().join(":")
}

/// `var=...` of a widget.
fn var_field(v: &SettingValue) -> String {
    let (vars, _) = vars(v);
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
pub(crate) fn lua_file(p: &PagedSettings) -> String {
    let pages = p.pages.len().max(1);
    let apply = format!("action={{\"gosub\", \"*{SETTINGS_APPLY}\"}}");
    let mut w = Vec::new();
    for (n, page) in p.pages.iter().enumerate() {
        let on = format!(", page={}", n + 1);
        w.push(image("image", &page.base, &on));
        if pages > 1 {
            w.push(image("button", &p.prev, &format!("{on}, cells=2, action={{\"page\", {}}}", (n + pages - 1) % pages + 1)));
            w.push(image("button", &p.next, &format!("{on}, cells=2, action={{\"page\", {}}}", (n + 1) % pages + 1)));
        }
        for c in &page.rows {
            match c {
                RowControl::Choice { value, options } => {
                    let inverted = vars(value).1;
                    for (v, o, current) in options {
                        let v = if inverted { 1 - v } else { *v };
                        w.push(image("radio", o, &format!("{on}, cells=2, on_file={}, var={}, value={v}, {apply}", lua(current), var_field(value))));
                    }
                }
                RowControl::Slider { value, knob, travel, fill, bar, minus, plus } => {
                    let var = var_field(value);
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
    w.push(image("button", &p.close, ", cells=2, action={\"close\"}"));
    w.push(image("button", &p.defaults, ", cells=2, action={\"gosub\", \"*conv_settings_defaults\"}"));
    let mut out = format!("-- The settings (conscripter-convert, from PagedSettings).\nlocal in_game = var({}) == 1\nlocal w = {{\n", lua(IN_GAME));
    for x in w {
        writeln!(out, "  {x},").unwrap();
    }
    // Back to the title and quit close the screen: `*conv_settings` asks.
    writeln!(out, "}}\nif in_game then\n  w[#w+1] = {}\n  w[#w+1] = {}\nend", image("button", &p.title, ", cells=2, action={\"close\", 1}"), image("button", &p.quit, ", cells=2, action={\"close\", 2}")).unwrap();
    let hidden: Vec<String> = hidden_while_open().iter().map(|s| if s.starts_with('%') { format!("var({})", lua(s)) } else { s.clone() }).collect();
    writeln!(
        out,
        "return {{\n  kind = \"settings\",\n  pages = {pages},\n  page_var = {},\n  hide = in_game and {{window=true, sprites={{{}}}}} or nil,\n  widgets = w,\n}}",
        lua(SETTINGS_PAGE),
        hidden.join(", ")
    )
    .unwrap();
    out
}

/// `*conv_settings`: the screen from its first page; back to the title
/// and quit ask first (the game's own dialog) and open it again on no.
/// `*conv_settings_defaults`: every setting back to its default.
pub(crate) fn labels(out: &mut String, m: &SettingsMenu, p: &PagedSettings, confirm: Option<&ConfirmDialog>, leave: Option<(&str, &str)>) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let b = BUTTON_VAR;
    w(format!("*{}", crate::menu::SETTINGS));
    w(format!("mov {SETTINGS_PAGE},1"));
    w("*conv_settings_open".into());
    w(format!("screen \"settings\",{b}"));
    w(format!("if {b} == 1 goto *conv_settings_title"));
    w(format!("if {b} == 2 goto *conv_settings_quit"));
    w("return".into());
    // The levels' defaults and those of the values on the pages.
    let mut defaults = vec![settings_defaults(m)];
    for c in p.pages.iter().flat_map(|page| &page.rows) {
        let (RowControl::Choice { value: v, .. } | RowControl::Slider { value: v, .. }) = c;
        match v {
            SettingValue::Switch(s) => defaults.push(set(v, i32::from(!m.switches_off.contains(s)))),
            SettingValue::Voices(_) => defaults.push(set(v, 1)),
            SettingValue::SkipUnread => defaults.push(set(v, 0)),
            SettingValue::Level(_) => {}
        }
    }
    w("*conv_settings_defaults".into());
    w(format!("{}:gosub *{SETTINGS_APPLY}", defaults.join(":")));
    w("return".into());
    // `reset` starts the script over: the title screen. During a replay:
    // back to its list instead.
    if let Some((_, question)) = leave {
        w("*conv_settings_title".into());
        w(format!("if {} == 0 goto *conv_settings_title_go", crate::replay::REPLAYING));
        if confirm.is_some() {
            w(confirm_call(question, Switch::ConfirmTitle));
            w(format!("if {CONFIRM} == 0 goto *conv_settings_open"));
        }
        w(format!("goto *{}", crate::replay::END));
    }
    let title = if leave.is_some() { "conv_settings_title_go" } else { "conv_settings_title" };
    for (label, question, then, kind) in [
        (title, confirm.map(|c| &c.title), "reset", Switch::ConfirmTitle),
        ("conv_settings_quit", confirm.map(|c| &c.quit), "end", Switch::ConfirmQuit),
    ] {
        w(format!("*{label}"));
        if let Some(q) = question {
            w(confirm_call(q, kind));
            w(format!("if {CONFIRM} == 0 goto *conv_settings_open"));
        }
        w(then.into());
    }
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
        assert_eq!(var_field(&SettingValue::Voices(vec![3, 5])), "{\"%973\", \"%975\"}");
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
            title: placed("title.png", 0),
            quit: placed("quit.png", 0),
        };
        let lua = lua_file(&p);
        // The switch's global is 1 while off: option 1 is value 0.
        assert!(lua.contains("{\"radio\", file=\"on.png\", x=500, y=100, page=1, cells=2, on_file=\"on_lit.png\", var=\"%2919\", value=0,"), "{lua}");
        assert!(lua.contains("knob={file=\"k.png\", x=764, y=100}, fill={file=\"e.png\", x=509, y=100}, track="), "{lua}");
        // The track from ◀ to ▶: the knob's centre at level 10 (506) minus 8.
        assert!(lua.contains("track={x=498, y=100, w=285, h=10}"), "{lua}");
        assert!(lua.contains("action={\"page\", 1}"));
    }
}
