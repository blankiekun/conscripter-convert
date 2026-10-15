//! A settings screen made of a game's own pictures ([`PictureSettings`]):
//! pages with their rows' words drawn in, switched with ◀ ▶; each row an
//! option list (the current option lit) or a level bar with ◀ ▶ buttons.
//! It sets the same variables as the text screen (`menu.rs`) and ends in
//! its `*conv_settings_apply` routine.


use conv_core::ir::{ConfirmDialog, Placed, PictureSettings, RowControl, SettingValue, SettingsMenu, Switch};

use crate::menu::{
    confirm_call, hidden_while_open, lsp2, setting_var, settings_defaults, switch_off, BASE, CONFIRM, IN_GAME, SETTINGS_APPLY,
    SETTINGS_PAGE, SKIP_UNREAD, VOICE_OFF_BASE,
};
use crate::{path, BUTTON_VAR, MENU_SPRITES};

const M: u32 = MENU_SPRITES.start;
const PREV: u32 = M;
const NEXT: u32 = M + 1;
const CLOSE: u32 = M + 2;
const DEFAULTS: u32 = M + 3;
const TITLE: u32 = M + 4;
const QUIT: u32 = M + 5;
/// A row's sprites from here, in order: an option each, or bar, ◀, ▶.
const CONTROLS: u32 = M + 6;
const BTN_PREV: u32 = 1;
const BTN_NEXT: u32 = 2;
const BTN_CLOSE: u32 = 3;
const BTN_DEFAULTS: u32 = 4;
const BTN_TITLE: u32 = 5;
const BTN_QUIT: u32 = 6;
/// A control's button: this plus its sprite.
const BTN_CONTROL: u32 = 100;

/// The variable a value lives in, and how a 0/1 option compares with it.
fn var(v: &SettingValue) -> (String, bool) {
    match v {
        SettingValue::Level(s) => (setting_var(*s).to_string(), false),
        // The globals are 1 while off.
        SettingValue::Switch(s) => (switch_off(*s), true),
        SettingValue::SkipUnread => (SKIP_UNREAD.to_string(), false),
        SettingValue::Voices(c) => (format!("%{}", VOICE_OFF_BASE + c.first().copied().unwrap_or(0)), true),
    }
}

/// `value == v` as an NScripter condition.
fn is(v: &SettingValue, value: i32) -> String {
    let (var, inverted) = var(v);
    format!("{var} == {}", if inverted { 1 - value } else { value })
}

/// `value != v` as an NScripter condition.
fn isnt(v: &SettingValue, value: i32) -> String {
    let (var, inverted) = var(v);
    format!("{var} != {}", if inverted { 1 - value } else { value })
}

/// Sets `v` to `value`.
fn set(v: &SettingValue, value: i32) -> String {
    match v {
        SettingValue::Voices(chars) => chars.iter().map(|c| format!("mov %{},{}", VOICE_OFF_BASE + c, 1 - value)).collect::<Vec<_>>().join(":"),
        _ => {
            let (var, inverted) = var(v);
            format!("mov {var},{}", if inverted { 1 - value } else { value })
        }
    }
}

fn button(slot: u32, p: &Placed) -> String {
    format!("lsp {slot},\":a/2,0,3;{}\",{},{}", path(&p.image), p.x, p.y)
}

/// The sprites of a page's controls: (control, sprite of its first part).
fn sprites(rows: &[RowControl]) -> Vec<u32> {
    let mut next = CONTROLS;
    rows.iter()
        .map(|c| {
            let first = next;
            next += match c {
                RowControl::Choice { options, .. } => options.len() as u32,
                RowControl::Slider { .. } => 3,
            };
            first
        })
        .collect()
}

pub(crate) fn emit(out: &mut String, m: &SettingsMenu, p: &PictureSettings, confirm: Option<&ConfirmDialog>, leave: Option<(&str, &str)>) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let pages = p.pages.len().max(1) as u32;
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    let menu_sprites = MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":");
    w(format!("*{}", crate::menu::SETTINGS));
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(format!("mov {SETTINGS_PAGE},0"));
    w("*conv_settings_draw".into());
    w(menu_sprites.clone());
    for (n, page) in p.pages.iter().enumerate() {
        w(format!("if {SETTINGS_PAGE} == {n} {}", lsp2(BASE, &page.base, None)));
    }
    if pages > 1 {
        w(button(PREV, &p.prev));
        w(button(NEXT, &p.next));
    }
    w(button(CLOSE, &p.close));
    w(button(DEFAULTS, &p.defaults));
    w(format!("if {IN_GAME} == 1 {}:{}", button(TITLE, &p.title), button(QUIT, &p.quit)));
    for (n, page) in p.pages.iter().enumerate() {
        w(format!("if {SETTINGS_PAGE} != {n} goto *conv_settings_draw_{}", n + 1));
        for (c, first) in page.rows.iter().zip(sprites(&page.rows)) {
            match c {
                RowControl::Choice { value, options } => {
                    for (k, (v, o)) in options.iter().enumerate() {
                        let slot = first + k as u32;
                        // The current option lit (and not a button).
                        w(button(slot, o));
                        w(format!("if {} cell {slot},1", is(value, *v)));
                    }
                }
                RowControl::Slider { value, bar, bars, minus, plus } => {
                    let (level, _) = var(value);
                    for (k, image) in bars.iter().enumerate() {
                        w(format!("if {level} == {k} lsp {first},\":a;{}\",{},{}", path(image), bar.x, bar.y));
                    }
                    w(button(first + 1, minus));
                    w(button(first + 2, plus));
                }
            }
        }
        w("goto *conv_settings_wait".into());
        w(format!("*conv_settings_draw_{}", n + 1));
    }
    w("*conv_settings_wait".into());
    if pages > 1 {
        w(format!("spbtn {PREV},{BTN_PREV}:spbtn {NEXT},{BTN_NEXT}"));
    }
    w(format!("spbtn {CLOSE},{BTN_CLOSE}:spbtn {DEFAULTS},{BTN_DEFAULTS}"));
    w(format!("if {IN_GAME} == 1 spbtn {TITLE},{BTN_TITLE}:spbtn {QUIT},{BTN_QUIT}"));
    for (n, page) in p.pages.iter().enumerate() {
        for (c, first) in page.rows.iter().zip(sprites(&page.rows)) {
            match c {
                RowControl::Choice { value, options } => {
                    for (k, (v, _)) in options.iter().enumerate() {
                        let slot = first + k as u32;
                        w(format!("if {SETTINGS_PAGE} == {n} && {} spbtn {slot},{}", isnt(value, *v), BTN_CONTROL + slot));
                    }
                }
                RowControl::Slider { .. } => {
                    w(format!("if {SETTINGS_PAGE} == {n} spbtn {},{}:spbtn {},{}", first + 1, BTN_CONTROL + first + 1, first + 2, BTN_CONTROL + first + 2));
                }
            }
        }
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *conv_settings_close"));
    w(format!("if {BUTTON_VAR} == {BTN_TITLE} goto *conv_settings_title"));
    w(format!("if {BUTTON_VAR} == {BTN_QUIT} goto *conv_settings_quit"));
    w(format!("if {BUTTON_VAR} == {BTN_PREV} mov {SETTINGS_PAGE},({SETTINGS_PAGE}+{})%{pages}:goto *conv_settings_draw", pages - 1));
    w(format!("if {BUTTON_VAR} == {BTN_NEXT} mov {SETTINGS_PAGE},({SETTINGS_PAGE}+1)%{pages}:goto *conv_settings_draw"));
    // Every setting back to its default: the levels' and those of the
    // values on the pages.
    let mut defaults = vec![settings_defaults(m)];
    for page in &p.pages {
        for c in &page.rows {
            let v = match c {
                RowControl::Choice { value, .. } | RowControl::Slider { value, .. } => value,
            };
            match v {
                SettingValue::Switch(_) | SettingValue::Voices(_) => defaults.push(set(v, 1)),
                SettingValue::SkipUnread => defaults.push(set(v, 0)),
                SettingValue::Level(_) => {}
            }
        }
    }
    w(format!("if {BUTTON_VAR} == {BTN_DEFAULTS} {}:gosub *{SETTINGS_APPLY}:goto *conv_settings_draw", defaults.join(":")));
    for (n, page) in p.pages.iter().enumerate() {
        for (c, first) in page.rows.iter().zip(sprites(&page.rows)) {
            match c {
                RowControl::Choice { value, options } => {
                    for (k, (v, _)) in options.iter().enumerate() {
                        w(format!("if {SETTINGS_PAGE} == {n} && {BUTTON_VAR} == {} {}", BTN_CONTROL + first + k as u32, set(value, *v)));
                    }
                }
                RowControl::Slider { value, .. } => {
                    let (level, _) = var(value);
                    w(format!("if {SETTINGS_PAGE} == {n} && {BUTTON_VAR} == {} && {level} > 0 sub {level},1", BTN_CONTROL + first + 1));
                    w(format!("if {SETTINGS_PAGE} == {n} && {BUTTON_VAR} == {} && {level} < 10 add {level},1", BTN_CONTROL + first + 2));
                }
            }
        }
    }
    w(format!("gosub *{SETTINGS_APPLY}"));
    w("goto *conv_settings_draw".into());
    // `reset` starts the script over: the title screen. During a replay:
    // back to its list instead.
    if let Some((_, question)) = leave {
        w("*conv_settings_title".into());
        w(format!("if {} == 0 goto *conv_settings_title_go", crate::replay::REPLAYING));
        if confirm.is_some() {
            w(confirm_call(question, Switch::ConfirmTitle));
            w(format!("if {CONFIRM} == 0 goto *conv_settings_wait"));
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
            w(format!("if {CONFIRM} == 0 goto *conv_settings_wait"));
        }
        w(then.into());
    }
    w("*conv_settings_close".into());
    // texton first: it drops the pending screen updates except the window's.
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    w(format!("csp2 {BASE}"));
    w(menu_sprites);
    w("print 1".into());
    w("return".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::Setting;

    #[test]
    fn values_compare_and_set() {
        assert_eq!(is(&SettingValue::Level(Setting::TextSpeed), 7), "%905 == 7");
        assert_eq!(is(&SettingValue::Switch(Switch::SceneSkipDialog), 1), "%2914 == 0");
        assert_eq!(set(&SettingValue::Switch(Switch::SceneSkipDialog), 0), "mov %2914,1");
        assert_eq!(set(&SettingValue::Voices(vec![3, 5]), 0), "mov %973,1:mov %975,1");
    }
}
