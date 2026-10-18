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

// Globals (`globalon` keeps %200 and up): 2920-2949, after the switches'
// second range (2910-2919; the third is 2950 up), below the scene replay's flags (3000).
// The first two and the slider and pair ranges are where builds before the
// named globals kept the values (read once by [`init`]).
/// 1 once builds before the named globals had set the values.
const SET: &str = "%2920";
/// The `gamespeed` setting's percent the story runs at.
pub(crate) const GAME_SPEED: &str = "%2921";
/// Slider i's position (before the named globals).
const SLIDER_BASE: u32 = 2922;
/// Pair i's value (before the named globals).
const PAIR_BASE: u32 = 2930;

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
    }
}

/// The define section's names, for the engine's screen.
pub(crate) fn define(out: &mut String, s: &PictureSettings) {
    let names = s.sliders.iter().map(|l| slider_name(l.setting)).chain(s.pairs.iter().map(|p| pair_name(p.option))).chain(["done"]);
    for n in names {
        out.push_str(&format!("defvar {},global\n", named(n)));
    }
}

pub(crate) fn pair_var(s: &PictureSettings, i: usize) -> String {
    format!("%{}", named(pair_name(s.pairs[i].option)))
}

/// At `*start`: the defaults on the first start, then the values applied.
pub(crate) fn init(out: &mut String, s: &PictureSettings) {
    let set = format!("%{}", named("done"));
    // shortcut: takes the values from where builds before the names kept them; drop once no such gloval.sav is played
    let copies: String = (0..s.sliders.len()).map(|i| format!(":mov {},%{}", slider_var(s, i), SLIDER_BASE + i as u32)).chain((0..s.pairs.len()).map(|i| format!(":mov {},%{}", pair_var(s, i), PAIR_BASE + i as u32))).collect();
    out.push_str(&format!("if {set} == 0 && {SET} == 1 mov {set},1{copies}\n"));
    out.push_str(&format!("if {set} == 0 gosub *{DEFAULTS}\ngosub *{APPLY}\n"));
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
    w(format!("mov %{},1", named("done")));
    w("return".into());

    w(format!("*{APPLY}"));
    for (i, l) in s.sliders.iter().enumerate() {
        let v = slider_var(s, i);
        let volume = |w: &mut dyn FnMut(String), cmd: &str| {
            // 100 × (v/1000)^1.661 ≈ (p² / 50 + p) / 3 with p = v / 10.
            w(format!("mov {TMP},(1000-{v})/10:mov {TMP},({TMP}*{TMP}/50+{TMP})/3:{cmd} {TMP}"));
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
