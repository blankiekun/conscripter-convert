//! The settings menu ([`SettingsMenu`]), its switches and levels.

use super::*;

/// The settings screen (see [`SettingsMenu`]): levels 0-10 in globals,
/// applied by [`SETTINGS_APPLY`] (also run at startup by
/// [`settings_init`]), and on a second page the voice switches (globals
/// from [`VOICE_OFF_BASE`], read where each voice plays).
/// `leave`: during a scene replay, "back to the title" becomes this label
/// and question and leads back to the replay's list. `version`: the
/// build's line and the screen's height, the line written small in the
/// bottom left corner.
pub(crate) fn settings_menu(
    out: &mut String,
    m: &SettingsMenu,
    confirm: Option<&ConfirmDialog>,
    leave: Option<(&str, &str)>,
    version: Option<(&str, u32)>,
) {
    if let Some(p) = &m.pictures {
        crate::settings_pages::emit(out, m, p, confirm, leave);
        settings_apply(out, m);
        return;
    }
    let rows = &m.rows[..m.rows.len().min(MAX_SETTING_ROWS)];
    let voices = &m.voices[..m.voices.len().min(MAX_VOICES)];
    let switches = &m.switches[..m.switches.len().min(MAX_SWITCHES)];
    // Pages: levels, voices, switches (only as many as there are tabs).
    let pages = m.tab_labels.len().min(1 + usize::from(!voices.is_empty()) + usize::from(!switches.is_empty())) as u32;
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let size = m.font_size;
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    // `strsp` drops a line's last character unless there is a column to
    // spare.
    let hover = |slot: u32, text: &str, x: i32, y: i32, chars: usize| {
        let columns = chars + 1;
        format!("strsp {slot},{text},{x},{y},{columns},1,{size},{size},0,0,0,1,\"#FFFFFF\",\"#F0C040\"")
    };
    // A switch's state is its first character's.
    let off = |t: &VoiceToggle| format!("%{}", VOICE_OFF_BASE + t.characters.first().copied().unwrap_or(0));
    let column_rows = voices.len().div_ceil(2);
    w(format!("*{SETTINGS}"));
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(format!("mov {SETTINGS_PAGE},0"));
    w(lsp2(BASE, &m.base, Some(m.base_opacity)));
    w(lsp2(HEADER, &m.header, None));
    w(format!("lsp {CLOSE},\":a/3,0,3;{}\",{},{}", path(&m.close.image), m.close.x, m.close.y));
    // Back to the title and quit, from the game.
    for (slot, x, label) in [(SETTING_TITLE, m.label_x, &m.title_label), (SETTING_QUIT, m.level_x, &m.quit_label)] {
        w(format!("if {IN_GAME} == 1 {}", hover(slot, &format!("\"{}\"", escape(label)), x, m.exits_y, 20)));
    }
    if let Some((label, _)) = leave {
        let text = format!("\"{}\"", escape(label));
        w(format!("if {IN_GAME} == 1 && {} == 1 {}", crate::replay::REPLAYING, hover(SETTING_TITLE, &text, m.label_x, m.exits_y, 20)));
    }
    w("*conv_settings_draw".into());
    w((M..SETTING_TITLE).map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    if let Some((line, height)) = version {
        // Half-width characters count half a column.
        let text = crate::sprite_text(line);
        let columns = text.chars().count().div_ceil(2) + 1;
        w(format!("strsp {SETTING_VERSION},\"{text}\",8,{},{columns},1,14,14,0,0,0,1,\"#B0B0B0\"", height - 22));
    }
    if pages > 1 {
        // Tabs: the current page's in the highlight colour, not clickable.
        let mut x = m.tabs_x;
        for (page, label) in m.tab_labels.iter().take(pages as usize).enumerate() {
            let slot = SETTING_TAB + page as u32;
            let text = format!("\"{}\"", escape(label));
            let chars = label.chars().count();
            w(format!("if {SETTINGS_PAGE} != {page} {}", hover(slot, &text, x, m.tabs_y, chars)));
            w(format!(
                "if {SETTINGS_PAGE} == {page} strsp {slot},{text},{x},{},{},1,{size},{size},0,0,0,1,\"#F0C040\"",
                m.tabs_y,
                chars + 1
            ));
            x += (chars as i32 + 2) * (size as i32 + 1);
        }
        w(format!("if {SETTINGS_PAGE} == 1 goto *conv_settings_draw_voices"));
        w(format!("if {SETTINGS_PAGE} == 2 goto *conv_settings_draw_switches"));
    }
    for (r, row) in rows.iter().enumerate() {
        let y = m.y + r as i32 * m.pitch;
        let level = setting_var(row.setting);
        let r = r as u32;
        for (slot, x, sign) in [(SETTING_MINUS + r, m.minus_x, "－"), (SETTING_PLUS + r, m.plus_x, "＋")] {
            w(hover(slot, &format!("\"{sign}\""), x, y, 1));
        }
        // "ＢＧＭ　…　■■■■■■■□□□　７": the label, padded with full-width
        // spaces up to the level's column (minus sits over the padding).
        let pad = ((m.level_x - m.label_x) / (size as i32 + 1)).max(row.label.chars().count() as i32 + 1) as usize;
        let label = format!("{}{}", row.label, "　".repeat(pad - row.label.chars().count()));
        w(format!("mov {STR_TMP},\"{}\"", escape(&label)));
        for i in 1..=10 {
            w(format!("if {level} >= {i} add {STR_TMP},\"■\""));
            w(format!("if {level} < {i} add {STR_TMP},\"□\""));
        }
        w(format!("add {STR_TMP},\"　\""));
        w(format!("itoa2 {LINE1},{level}"));
        w(format!("add {STR_TMP},{LINE1}"));
        w(format!("strsp {},{STR_TMP},{},{y},{},1,{size},{size},0,0,0,1", SETTING_ROW + r, m.label_x, pad + 14));
    }
    if let Some(label) = &m.skip_unread_label {
        w(format!("mov {STR_TMP},\"□　\""));
        w(format!("if {SKIP_UNREAD} == 1 mov {STR_TMP},\"■　\""));
        w(format!("add {STR_TMP},\"{}\"", escape(label)));
        w(hover(SETTING_SKIP, STR_TMP, m.label_x, m.y + rows.len() as i32 * m.pitch, label.chars().count() + 2));
    }
    w("goto *conv_settings_wait".into());
    // "■ option" (on) or "□ option", one column.
    w("*conv_settings_draw_switches".into());
    for (i, s) in switches.iter().enumerate() {
        let y = m.y + i as i32 * m.voice_pitch;
        w(format!("mov {STR_TMP},\"■　\""));
        w(format!("if {} == 1 mov {STR_TMP},\"□　\"", switch_off(s.switch)));
        w(format!("add {STR_TMP},\"{}\"", escape(&s.label)));
        w(hover(SETTING_SWITCH + i as u32, STR_TMP, m.label_x, y, s.label.chars().count() + 2));
    }
    w("goto *conv_settings_wait".into());
    // "■ name" (on) or "□ name", in two columns.
    w("*conv_settings_draw_voices".into());
    for (i, t) in voices.iter().enumerate() {
        let x = if i < column_rows { m.label_x } else { m.voice_column_x };
        let y = m.y + (i % column_rows) as i32 * m.voice_pitch;
        w(format!("mov {STR_TMP},\"■　\""));
        w(format!("if {} == 1 mov {STR_TMP},\"□　\"", off(t)));
        w(format!("add {STR_TMP},\"{}\"", escape(&t.label)));
        w(hover(SETTING_VOICE + i as u32, STR_TMP, x, y, t.label.chars().count() + 2));
    }
    w("*conv_settings_wait".into());
    // The other pages' tabs.
    for p in 0..pages {
        if pages > 1 {
            w(format!("if {SETTINGS_PAGE} != {p} spbtn {},{}", SETTING_TAB + p, BTN_TAB + 1 + p));
        }
    }
    w(format!("if {SETTINGS_PAGE} == 1 goto *conv_settings_wait_voices"));
    w(format!("if {SETTINGS_PAGE} == 2 goto *conv_settings_wait_switches"));
    for r in 0..rows.len() as u32 {
        w(format!("spbtn {},{}", SETTING_MINUS + r, BTN_MINUS + r + 1));
        w(format!("spbtn {},{}", SETTING_PLUS + r, BTN_PLUS + r + 1));
    }
    if m.skip_unread_label.is_some() {
        w(format!("spbtn {SETTING_SKIP},{BTN_SKIP}"));
    }
    w("goto *conv_settings_wait_any".into());
    w("*conv_settings_wait_voices".into());
    for i in 0..voices.len() as u32 {
        w(format!("spbtn {},{}", SETTING_VOICE + i, BTN_VOICE + i + 1));
    }
    w("goto *conv_settings_wait_any".into());
    w("*conv_settings_wait_switches".into());
    for i in 0..switches.len() as u32 {
        w(format!("spbtn {},{}", SETTING_SWITCH + i, BTN_SWITCH + i + 1));
    }
    w("*conv_settings_wait_any".into());
    w(format!("if {IN_GAME} == 1 spbtn {SETTING_TITLE},{BTN_TITLE}:spbtn {SETTING_QUIT},{BTN_QUIT}"));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *conv_settings_close"));
    w(format!("if {BUTTON_VAR} == {BTN_TITLE} goto *conv_settings_title"));
    w(format!("if {BUTTON_VAR} == {BTN_QUIT} goto *conv_settings_quit"));
    w(format!(
        "if {BUTTON_VAR} > {BTN_TAB} && {BUTTON_VAR} <= {} mov {SETTINGS_PAGE},{BUTTON_VAR}-{}:goto *conv_settings_draw",
        BTN_TAB + pages,
        BTN_TAB + 1
    ));
    for (i, s) in switches.iter().enumerate() {
        let off = switch_off(s.switch);
        // A switch the apply routine puts into effect does so at once.
        let apply = if crate::switches::apply(s.switch) != crate::switches::Apply::Read { format!("gosub *{SETTINGS_APPLY}:") } else { String::new() };
        w(format!("if {BUTTON_VAR} == {} mov {off},1-{off}:{apply}goto *conv_settings_draw", BTN_SWITCH + i as u32 + 1));
    }
    for (i, t) in voices.iter().enumerate() {
        let all = |v: u32| t.characters.iter().map(|c| format!("mov %{},{v}", VOICE_OFF_BASE + c)).collect::<Vec<_>>().join(":");
        let button = BTN_VOICE + i as u32 + 1;
        w(format!("if {BUTTON_VAR} == {button} && {} == 0 {}:goto *conv_settings_draw", off(t), all(1)));
        w(format!("if {BUTTON_VAR} == {button} {}:goto *conv_settings_draw", all(0)));
    }
    w(format!("if {BUTTON_VAR} == {BTN_SKIP} mov {SKIP_UNREAD},1-{SKIP_UNREAD}"));
    for (r, row) in rows.iter().enumerate() {
        let level = setting_var(row.setting);
        let r = r as u32;
        w(format!("if {BUTTON_VAR} == {} && {level} > 0 sub {level},1", BTN_MINUS + r + 1));
        w(format!("if {BUTTON_VAR} == {} && {level} < 10 add {level},1", BTN_PLUS + r + 1));
    }
    w(format!("gosub *{SETTINGS_APPLY}"));
    w("goto *conv_settings_draw".into());
    // `reset` starts the script over: the title screen.
    // During a replay: back to its list instead.
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
    w(format!("csp2 {BASE}:csp2 {HEADER}"));
    w(MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("print 1".into());
    w("return".into());
    settings_apply(out, m);
}

/// The routine putting the settings into effect (volumes, speeds, window,
/// skip).
pub(super) fn settings_apply(out: &mut String, m: &SettingsMenu) {
    let rows = &m.rows[..m.rows.len().min(MAX_SETTING_ROWS)];
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
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
    w(format!("mov {TEXT_MS},(10-{})*{TEXT_MS_STEP}:textspeed {TEXT_MS}", setting_var(Setting::TextSpeed)));
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
            let (conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. }) = c;
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
                conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. } => *value == conv_core::ir::SettingValue::Switch(s),
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
    let shared = [
        (Setting::BgmVolume, 8),
        (Setting::SeVolume, 8),
        (Setting::VoiceVolume, 10),
        (Setting::TextSpeed, 7),
        (Setting::AutoSpeed, 5),
        (Setting::WindowTransparency, 0),
    ];
    let mut all = shared.to_vec();
    if has_setting(m, Setting::SystemVolume) {
        all.push((Setting::SystemVolume, 8));
    }
    all.iter().map(|&(s, level)| format!("mov {},{}", setting_var(s), default_level(m, s, level))).collect::<Vec<_>>().join(":")
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
                conv_core::ir::RowControl::Choice { value, .. } | conv_core::ir::RowControl::Slider { value, .. } => *value == conv_core::ir::SettingValue::Level(s),
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
