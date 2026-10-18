//! The settings menu ([`SettingsMenu`]), its switches and levels.

use super::*;

/// The settings of a picture-page screen (see [`SettingsMenu::pictures`]):
/// the engine draws it; this writes its labels and [`SETTINGS_APPLY`].
/// `leave`: during a scene replay, "back to the title" leads to the replay.
pub(crate) fn settings_menu(out: &mut String, m: &SettingsMenu, leave: bool) {
    if let Some(p) = &m.pictures {
        crate::screens::settings_labels(out, leave);
        crate::settings_pages::labels(out, m, p);
        settings_apply(out, m);
    }
}

/// The routine putting the settings into effect (volumes, speeds, window,
/// skip).
pub(crate) fn settings_apply(out: &mut String, m: &SettingsMenu) {
    let rows = &m.rows[..m.rows.len().min(MAX_SETTING_ROWS)];
    let mut w = lines!(out);
    w(format!("*{SETTINGS_APPLY}"));
    let used = used_switches(m);
    for (setting, cmd) in [(Setting::BgmVolume, "bgmvol"), (Setting::SeVolume, "sevol"), (Setting::VoiceVolume, "voicevol")] {
        w(format!("mov {TMP},{}*10", setting_var(setting)));
        // A switch muting it: volume 0, the level kept.
        for &s in used.iter().filter(|&&s| crate::switches::apply(s) == crate::switches::Apply::Mutes(setting)) {
            w(format!("if {} == 1 mov {TMP},0", switch_off(s)));
        }
        w(format!("{cmd} {TMP}"));
    }
    if has_setting(m, Setting::SystemVolume) {
        w(format!("mov {TMP},{}*10:consset \"sysevol\",{TMP}", setting_var(Setting::SystemVolume)));
    }
    match &m.text_ms {
        Some(table) => {
            for (level, ms) in table.iter().enumerate() {
                w(format!("if {} == {level} mov {TEXT_MS},{ms}", setting_var(Setting::TextSpeed)));
            }
            w(format!("textspeed {TEXT_MS}"));
        }
        None => w(format!("mov {TEXT_MS},(10-{})*{TEXT_MS_STEP}:textspeed {TEXT_MS}", setting_var(Setting::TextSpeed))),
    }
    let auto = setting_var(Setting::AutoSpeed);
    match rows.iter().find_map(|r| match r.setting {
        Setting::AutoWait { page_step, char_step } => Some((page_step, char_step)),
        _ => None,
    }) {
        Some((page, char)) => w(format!("automode_time (10-{auto})*{page},(10-{auto})*{char}")),
        None => w(format!("mov {TMP},(10-{auto})*400+600:automode_time {TMP}")),
    }
    if rows.iter().any(|r| r.setting == Setting::WindowTransparency) {
        w(format!("mov {TMP},255-{}*25:windowtrans {TMP}", setting_var(Setting::WindowTransparency)));
    }
    for &s in &used {
        if let crate::switches::Apply::Commands { on, off } = crate::switches::apply(s) {
            let flag = switch_off(s);
            w(format!("if {flag} == 0 {on}"));
            w(format!("if {flag} == 1 {off}"));
        }
    }
    if m.skip_unread_label.is_some() {
        w(format!("if {SKIP_UNREAD} == 0 kidokumode 1"));
        w(format!("if {SKIP_UNREAD} == 1 kidokumode 0"));
    }
    w("return".into());
}

/// The switches the settings screen has (in its list or on picture
/// pages), each once.
pub(super) fn used_switches(m: &SettingsMenu) -> Vec<Switch> {
    let mut all: Vec<Switch> = m.switches.iter().map(|s| s.switch).collect();
    if let Some(p) = &m.pictures {
        for c in p.pages.iter().flat_map(|page| &page.rows) {
            let (conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. } | conv_core::ir::RowControl::Toggle { value, .. }) = c;
            if let conv_core::ir::SettingValue::Switch(s) = value {
                all.push(*s);
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    all.retain(|&s| seen.insert(switch_index(s)));
    all
}

/// The settings screen has switch `s` (in its list or on a picture page).
pub(crate) fn has_switch(m: &SettingsMenu, s: Switch) -> bool {
    m.switches.iter().any(|x| x.switch == s)
        || m.pictures.as_ref().is_some_and(|p| {
            p.pages.iter().flat_map(|page| &page.rows).any(|c| match c {
                conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. } | conv_core::ir::RowControl::Toggle { value, .. } => *value == conv_core::ir::SettingValue::Switch(s),
            })
        })
}

/// Sets the question and asks it (see [`confirm`]).
pub(crate) fn confirm_call(question: &str, kind: Switch) -> String {
    format!("mov {CONFIRM_KIND},{}:mov {CONFIRM_TEXT},\"{}\"\ngosub *{CONFIRM_ROUTINE}", switch_index(kind) + 1, escape(question))
}

pub(crate) fn switch_index(s: Switch) -> u32 {
    crate::switches::index(s)
}

/// The global that is 1 while `s` is switched off.
pub(crate) fn switch_off(s: Switch) -> String {
    format!("%{}", crate::switches::off_var(switch_index(s)))
}

/// First run: default levels; every run: apply them (before `setwindow`,
/// which takes the text speed from [`TEXT_MS`]).
pub(crate) fn settings_init(out: &mut String, m: &SettingsMenu) {
    let mut first = settings_defaults(m);
    for &s in &m.switches_off {
        first.push_str(&format!(":mov {},1", switch_off(s)));
    }
    out.push_str(&format!("if {SETTINGS_SET} == 0 {first}:mov {SETTINGS_SET},1\n"));
    if has_setting(m, Setting::SystemVolume) {
        let v = setting_var(Setting::SystemVolume);
        out.push_str(&format!("if {SYSTEM_VOLUME_SET} == 0 mov {v},{}:mov {SYSTEM_VOLUME_SET},1\n", default_level(m, Setting::SystemVolume, 8)));
    }
    out.push_str(&format!("gosub *{SETTINGS_APPLY}\n"));
}

/// The levels' defaults (as [`settings_init`] sets them, the game's
/// [`SettingsMenu::defaults`] over the shared ones), as commands.
pub(crate) fn settings_defaults(m: &SettingsMenu) -> String {
    shared_defaults(m).iter().map(|&(s, level)| format!("mov {},{}", setting_var(s), default_level(m, s, level))).collect::<Vec<_>>().join(":")
}

/// The shared default levels for `m`'s settings.
fn shared_defaults(m: &SettingsMenu) -> Vec<(Setting, u32)> {
    let mut all = vec![
        (Setting::BgmVolume, 8),
        (Setting::SeVolume, 8),
        (Setting::VoiceVolume, 10),
        (Setting::TextSpeed, 7),
        (Setting::AutoSpeed, 5),
        (Setting::WindowTransparency, 0),
    ];
    if has_setting(m, Setting::SystemVolume) {
        all.push((Setting::SystemVolume, 8));
    }
    all
}

/// The settings whose default level `m` doesn't give, so they start at the
/// shared one, as "name level" (for the build report). The window level
/// counts only where the screen has it (nothing applies it otherwise).
pub(crate) fn fallback_defaults(m: &SettingsMenu) -> Vec<String> {
    shared_defaults(m)
        .into_iter()
        .filter(|&(s, _)| s != Setting::WindowTransparency || has_setting(m, s))
        .filter(|&(s, _)| !m.defaults.iter().any(|(d, _)| setting_var(*d) == setting_var(s)))
        .map(|(s, level)| format!("{s:?} {level}"))
        .collect()
}

/// Setting `s`'s default level: the game's, else `level`.
pub(super) fn default_level(m: &SettingsMenu, s: Setting, level: u32) -> u32 {
    m.defaults.iter().find(|(d, _)| setting_var(*d) == setting_var(s)).map_or(level, |&(_, l)| l.min(10))
}

/// The settings screen has level `s` (in its rows or on a picture page).
pub(super) fn has_setting(m: &SettingsMenu, s: Setting) -> bool {
    m.rows.iter().any(|r| r.setting == s)
        || m.pictures.as_ref().is_some_and(|p| {
            p.pages.iter().flat_map(|page| &page.rows).any(|c| match c {
                conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. } | conv_core::ir::RowControl::Toggle { value, .. } => *value == conv_core::ir::SettingValue::Level(s),
            })
        })
}

pub(crate) fn setting_var(s: Setting) -> &'static str {
    match s {
        Setting::BgmVolume => "%902",
        Setting::SeVolume => "%903",
        Setting::VoiceVolume => "%904",
        Setting::TextSpeed => "%905",
        Setting::AutoSpeed | Setting::AutoWait { .. } => "%906",
        // Added later: 0 (the default for older saves) is the plain window.
        Setting::WindowTransparency => "%910",
        // Added later: set to its default once [`SYSTEM_VOLUME_SET`] is 0.
        Setting::SystemVolume => "%911",
    }
}
