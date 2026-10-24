//! [`PictureSettings`] values: the engine's screen
//! ([`settings_screen`](crate::settings_screen)) binds them by name
//! (`defvar ...,global`); this writes their defaults, the routine applying
//! them to the engine ([`APPLY`]), and the start-up code.
//!
//! The values are globals (a slider's position 0-1000, a pair's 1 = on),
//! given their defaults on the first start and applied by [`APPLY`] at
//! startup and as they change. The game's speed is kept in [`GAME_SPEED`]
//! and applied by the click wait as it ends (menus, which are opened from
//! it, run at 100 %).

use conv_core::ir::{PairOption, PictureSettings, SliderSetting, Switch};

use crate::menu::{self, TEXT_MS, TMP};

// Globals (`globalon` keeps %200 and up): 2921, after the switches'
// second range (2910-2919; the third is 2950 up), below the scene replay's flags (3000).
/// The `gamespeed` setting's percent the story runs at.
pub(crate) const GAME_SPEED: &str = "%2921";

// Locals.
const MUTE: [&str; 3] = ["%171", "%172", "%173"];

pub(crate) const APPLY: &str = "conv_ps_apply";
pub(crate) const DEFAULTS: &str = "conv_ps_defaults";

/// The questions [`PairOption::Confirm`] turns off.
const QUESTIONS: [Switch; 8] = [
    Switch::ConfirmSave,
    Switch::ConfirmLoad,
    Switch::ConfirmDelete,
    Switch::ConfirmTitle,
    Switch::ConfirmQuit,
    Switch::ConfirmBackToChoice,
    Switch::ConfirmReset,
    Switch::ConfirmQuickLoad,
];

/// A pair at one place is a check box.
pub(crate) fn check_box(p: &conv_core::ir::PicturePair) -> bool {
    (p.on.x, p.on.y) == (p.off.x, p.off.y)
}

pub(crate) fn slider_var(s: &PictureSettings, i: usize) -> String {
    format!("%{}", named(slider_name(s.sliders[i].setting)))
}

/// The engine's screen binds its values by name (`defvar ...,global`).
fn named(what: &str) -> String {
    format!("conv_set_{what}")
}

fn slider_name(l: SliderSetting) -> &'static str {
    match l {
        SliderSetting::BgmVolume => "bgm_volume",
        SliderSetting::SeVolume => "se_volume",
        SliderSetting::VoiceVolume => "voice_volume",
        SliderSetting::TextSpeed { .. } | SliderSetting::TextSpeedCurve { .. } => "text_speed",
        SliderSetting::AutoWait { .. } => "auto_wait",
        SliderSetting::GameSpeed { .. } => "game_speed",
    }
}

fn pair_name(p: PairOption) -> &'static str {
    match p {
        PairOption::Confirm => "confirm",
        PairOption::MovieStop => "movie_stop",
        PairOption::StopAtChoice => "stop_at_choice",
        PairOption::SkipUnread => "skip_unread",
        PairOption::VoicePlaysOn => "voice_plays_on",
        PairOption::AutoWaitsForVoice => "auto_waits_for_voice",
        PairOption::Bgm => "bgm_on",
        PairOption::Se => "se_on",
        PairOption::Voice => "voice_on",
        PairOption::ConfirmSave => "confirm_save",
        PairOption::ConfirmLoad => "confirm_load",
        PairOption::ClickStopsAuto => "click_stops_auto",
        PairOption::English => "english",
        PairOption::Uncensored => "uncensored",
    }
}

/// The define section's names, for the engine's screen.
pub(crate) fn define(out: &mut String, s: &PictureSettings, language: &str) {
    let names = s.sliders.iter().map(|l| slider_name(l.setting)).chain(s.pairs.iter().map(|p| pair_name(p.option))).chain(["done"]);
    for n in names {
        out.push_str(&format!("defvar {},global\n", named(n)));
    }
    if has_language(s) {
        out.push_str(&format!("defvar {},global\n", named(&format!("seen_{}", language))));
    }
}

pub(crate) fn pair_var(s: &PictureSettings, i: usize) -> String {
    format!("%{}", named(pair_name(s.pairs[i].option)))
}

/// What `done` holds once the settings are set: the newest of the pairs'
/// versions (see [`added_in`]).
fn done(s: &PictureSettings) -> u8 {
    s.pairs.iter().map(|p| added_in(p.option)).max().unwrap_or(1)
}

/// The version a pair came in (`done` older than it: the save lacks it).
fn added_in(o: PairOption) -> u8 {
    match o {
        PairOption::ClickStopsAuto => 2,
        PairOption::English | PairOption::Uncensored => 3,
        _ => 1,
    }
}

/// The language a save was made in (1 Japanese, 2 English, 0 a save from
/// before): not global, so a save keeps it.
/// The running language is the save's after a load ([`LOAD_LANGUAGE`]), so
/// it is also the language that runs.
pub(crate) const SAVE_LANGUAGE: &str = "%conv_save_lang";
/// The `loadgosub` routine with a Language setting: the save's language
/// first, then [`crate::LOAD_CLEAR`].
const LOAD_LANGUAGE: &str = "conv_load_lang";

/// Whether there is a Language setting.
pub(crate) fn has_language(s: &PictureSettings) -> bool {
    s.pairs.iter().any(|p| p.option == PairOption::English)
}

/// The define section's language: the Language setting's from the second
/// start on, else `first` (the script's `language` line, which the engine
/// reads its first table from when it remembers none: `language.txt`); and
/// a load going on in the save's language. Both editions write these same
/// lines, so they stay shared and a switch moves no code under the
/// engine's feet. The setting is the language that runs.
pub(crate) fn define_language(out: &mut String, load_clear: &str, first: &str) {
    let (save, l) = (SAVE_LANGUAGE, LOAD_LANGUAGE);
    let (english, done) = (format!("%{}", named("english")), format!("%{}", named("done")));
    for line in [
        format!("defvar {}", &save[1..]),
        // `first` before the others: a build of both editions puts the other's code there (conv-cli)
        format!("if {done} > 0 goto *conv_lang_set"),
        first.to_string(),
        "*conv_lang_set".to_string(),
        format!("if {done} > 0 && {english} == 1 language \"en\""),
        format!("if {done} > 0 && {english} == 0 language \"ja\""),
        format!("loadgosub *{l}:goto *{l}_end"),
        format!("*{l}"),
        format!("if {save} == 2 && {english} == 0 mov {english},1:language \"en\""),
        format!("if {save} == 1 && {english} == 1 mov {english},0:language \"ja\""),
        format!("goto *{load_clear}"),
        format!("*{l}_end"),
    ] {
        out.push_str(&line);
        out.push('\n');
    }
}

/// At `*start`: the defaults on the first start, then the values applied.
/// `language`: the script's; with a Language setting each edition's
/// `*start` marks the saves with it and sets its defaults on its own first
/// start (the editions' settings differ: Japanese has a game speed).
pub(crate) fn init(out: &mut String, s: &PictureSettings, language: &str) {
    let set = format!("%{}", named("done"));
    // A global save from before a pair was added: that pair takes its default.
    for v in 2..=done(s) {
        let fresh: String = s.pairs.iter().enumerate().filter(|(_, p)| added_in(p.option) == v)
            .map(|(i, p)| format!(":mov {},{}", pair_var(s, i), u8::from(p.default_on))).collect();
        out.push_str(&format!("if {set} == {} mov {set},{v}{fresh}\n", v - 1));
    }
    if has_language(s) {
        let seen = named(&format!("seen_{language}"));
        out.push_str(&format!("mov {SAVE_LANGUAGE},{}\nif %{seen} == 0 mov %{seen},1:gosub *{DEFAULTS}\ngosub *{APPLY}\n", if language == "en" { 2 } else { 1 }));
    } else {
        out.push_str(&format!("if {set} == 0 gosub *{DEFAULTS}\ngosub *{APPLY}\n"));
    }
    // `reset` comes back here: the title runs at the normal speed.
    if s.sliders.iter().any(|l| matches!(l.setting, SliderSetting::GameSpeed { .. })) {
        out.push_str("consset \"gamespeed\",100\n");
    }
}

/// Whether skipping text not read before is a choice (`kidokuskip`).
pub(crate) fn skips_unread(s: &PictureSettings) -> bool {
    s.pairs.iter().any(|p| p.option == PairOption::SkipUnread)
}

/// The settings label (the engine's screen), and the routines that put the
/// defaults back and apply the values.
pub(crate) fn emit(out: &mut String, s: &PictureSettings) {
    crate::settings_screen::labels(out, s);
    values(out, s);
}

/// `*conv_ps_defaults` and [`APPLY`].
fn values(out: &mut String, s: &PictureSettings) {
    let mut w = lines!(out);
    w(format!("*{DEFAULTS}"));
    for (i, l) in s.sliders.iter().enumerate() {
        w(format!("mov {},{}", slider_var(s, i), l.default));
    }
    for (i, p) in s.pairs.iter().enumerate() {
        w(format!("mov {},{}", pair_var(s, i), u8::from(p.default_on)));
    }
    w(format!("mov %{},{}", named("done"), done(s)));
    w("return".into());

    w(format!("*{APPLY}"));
    for (i, l) in s.sliders.iter().enumerate() {
        let v = slider_var(s, i);
        let volume = |w: &mut dyn FnMut(String), cmd: &str| {
            // 100 × (p/100)^1.661, p the position 0-100: Nitroplus's volume setting.
            // src: Muramasa.exe FUN_00476730 (setting/1000 to 005d3b10-18), FUN_00475980 (copies
            // it to a sound's +0x118), sound_set_volume 00475f50 (DirectSound volume
            // 3322·log10(f·v/1000) hundredths of a dB, 3322.0 at 0x514fe8); Django.exe 1.01 the
            // same at 0x457ed1 (fmuls +0x118, fmull 0x4d56e0 = 3322.0, fyl2x with fldlg2).
            // English: Diesel's own mixer (sqApp_setSoundTypeVolume 0x4d0c60 keeps 0-1; the
            // curve is in Mware.dll), the Japanese curve kept (muramasa-en.md, Scope).
            // ours: the cubic, NScripter having no pow; within 1 of the exe's percent at every
            // p (slider_volume_follows_the_exe_curve).
            w(format!(
                "mov {TMP},(1000-{v})/10:mov {TMP},({TMP}*(138300+{TMP}*(11017-24*{TMP}))+500000)/1000000:{cmd} {TMP}"
            ));
        };
        match l.setting {
            SliderSetting::BgmVolume => volume(&mut w, "bgmvol"),
            SliderSetting::SeVolume => volume(&mut w, "sevol"),
            SliderSetting::VoiceVolume => volume(&mut w, "voicevol"),
            SliderSetting::TextSpeed { ms } => w(format!("mov {TEXT_MS},{v}*{ms}/1000:textspeed {TEXT_MS}")),
            SliderSetting::AutoWait { ms } => w(format!("mov {TMP},{v}*{ms}/1000:automode_time 0,{TMP}")),
            // TMP = u in 1/100000 (each product under 2^31 for slow up to 2000);
            // text_ms(v) in the tests is the same sum.
            SliderSetting::TextSpeedCurve { fast, slow } => w(format!(
                "mov {TMP},(1000-{v})*(1000-{v})/100:mov {TMP},{TMP}*{TMP}/1000:mov {TEXT_MS},{}/({TMP}*{slow}+(100000-{TMP})*{fast}):if {v} == 0 mov {TEXT_MS},0:textspeed {TEXT_MS}",
                10000 * fast as u64 * slow as u64
            )),
            SliderSetting::GameSpeed { levels } => {
                w(format!("mov {TMP},12-{v}*12/1000:if {TMP} < 1 mov {TMP},1"));
                for (k, percent) in levels.iter().enumerate() {
                    w(format!("if {TMP} == {} mov {GAME_SPEED},{percent}", k + 1));
                }
            }
        }
    }
    let mut mute = [None, None, None];
    for (i, p) in s.pairs.iter().enumerate() {
        let v = pair_var(s, i);
        match p.option {
            PairOption::Confirm => {
                for q in QUESTIONS {
                    w(format!("mov {},1-{v}", menu::switch_off(q)));
                }
            }
            // Off, a movie is skippable as the script says (the title's openings).
            PairOption::MovieStop => w(format!("mov {TMP},{v}*2-1:consset \"moviestop\",{TMP}")),
            PairOption::StopAtChoice => w(format!("mov {TMP},1-{v}:consset \"selectkeep\",{TMP}")),
            PairOption::SkipUnread => w(format!("if {v} == 1 kidokumode 0\nif {v} == 0 kidokumode 1")),
            PairOption::VoicePlaysOn => w(format!("mov {TMP},1-{v}:consset \"clickvoicestop\",{TMP}")),
            PairOption::AutoWaitsForVoice => w(format!("consset \"autovoicewait\",{v}")),
            PairOption::Bgm => mute[0] = Some(v),
            PairOption::Se => mute[1] = Some(v),
            PairOption::Voice => mute[2] = Some(v),
            PairOption::ConfirmSave => w(format!("mov {},1-{v}", menu::switch_off(Switch::ConfirmSave))),
            PairOption::ConfirmLoad => w(format!("mov {},1-{v}", menu::switch_off(Switch::ConfirmLoad))),
            PairOption::ClickStopsAuto => w(format!("consset \"clickautostop\",{v}")),
            // switched in the define section only (the settings screen restarts for it)
            PairOption::English => {}
            PairOption::Uncensored => w(format!("assetlayer \"uncensored\",{v}")),
        }
    }
    if mute.iter().any(Option::is_some) {
        for (var, on) in MUTE.iter().zip(&mute) {
            match on {
                Some(v) => w(format!("mov {var},1-{v}")),
                None => w(format!("mov {var},0")),
            }
        }
        w(format!("consset \"mutebgm\",{}:consset \"mutese\",{}:consset \"mutevoice\",{}", MUTE[0], MUTE[1], MUTE[2]));
    }
    w("return".into());
}

#[cfg(test)]
mod tests {
    /// The integer sum the volume sliders write, for position p (0-100).
    fn slider_volume(p: i64) -> i64 {
        (p * (138300 + p * (11017 - 24 * p)) + 500000) / 1000000
    }

    #[test]
    fn slider_volume_follows_the_exe_curve() {
        // the amplitude of 3322·log10(p/100) hundredths of a dB (exe sound_set_volume)
        for p in 0..=100 {
            let exe = (100.0 * 10f64.powf(3322.0 * (p as f64 / 100.0).log10() / 2000.0)).round() as i64;
            let exe = if p == 0 { 0 } else { exe };
            assert!((slider_volume(p) - exe).abs() <= 1, "p {p}: {} vs {exe}", slider_volume(p));
        }
    }

    #[test]
    fn text_speed_curve_follows_the_rate() {
        // The integer sum TextSpeedCurve writes, against 1 / (u/fast + (1-u)/slow).
        let (fast, slow) = (38i64, 1506i64);
        let text_ms = |v: i64| {
            let u = (1000 - v) * (1000 - v) / 100;
            let u = u * u / 1000;
            let (num, den) = (10000 * fast * slow, u * slow + (100000 - u) * fast);
            assert!(num < 1 << 31 && den < 1 << 31);
            if v == 0 { 0 } else { num / den }
        };
        for v in 1..=1000 {
            let u = (1.0 - v as f64 / 1000.0).powi(4);
            let want = 1.0 / (u / (fast as f64 / 10.0) + (1.0 - u) / (slow as f64 / 10.0));
            assert!((text_ms(v) as f64 - want).abs() <= 1.0, "v {v}: {} vs {want}", text_ms(v));
        }
    }
}
