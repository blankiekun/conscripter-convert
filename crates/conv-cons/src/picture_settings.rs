//! [`PictureSettings`] as an NScripter subroutine under the settings label
//! ([`menu::SETTINGS`]), entered the same way as [`menu::settings`].
//!
//! The screen takes the game's sprite range between `pushscreen` and
//! `popscreen`, like the backlog ([`crate::line_log`]): the picture, the
//! fixed pictures and texts, then the controls over it. A button or a
//! pair's side is a sprite whose first cell is empty (the picture shows it
//! at rest), its second shown under the pointer (`spbtn`); a pair's chosen
//! side is a second sprite showing that strip's third cell. A slider's
//! track is an invisible sprite whose `exbtn` shows the knob's second cell
//! while the pointer is over it; pressing there drags the knob.
//!
//! The values are globals (a slider's position 0-1000, a pair's 1 = on),
//! given their defaults on the first start and applied to the engine by
//! [`APPLY`] at startup and as they change. The game's speed is kept in
//! [`GAME_SPEED`] and applied by the click wait as it ends (menus, which
//! are opened from it, run at 100 %).
//!
//! The voice window ([`VoiceWindow`]) lays its pictures over the settings
//! (lower sprite numbers): an entry shows once its flag is set, with a
//! check while its characters' voices play (the per-character switches
//! `dwave` reads, [`menu::VOICE_OFF_BASE`]); a `btn` over its box toggles
//! them all.

use conv_core::ir::{PairOption, PictureSettings, Placed, SliderSetting, Switch, Var, VoiceWindow};

use crate::menu::{self, CONFIRM, IN_GAME, TEXT_MS, TMP};
use crate::{escape, path, var_name, BUTTON_VAR, MAX_SPRITE};

// Globals (`globalon` keeps %200 and up): 2920-2949, after the switches'
// second range (2910-2914), below the scene replay's flags (3000).
/// 1 once the values have their defaults.
const SET: &str = "%2920";
/// The `gamespeed` percent the story runs at.
pub(crate) const GAME_SPEED: &str = "%2921";
/// Slider i's position.
const SLIDER_BASE: u32 = 2922;
const MAX_SLIDERS: usize = 8;
/// Pair i's value.
const PAIR_BASE: u32 = 2930;
const MAX_PAIRS: usize = 12;
// Locals (the backlog's; the two screens are never open together).
const DRAGGED: &str = "%170";
const MOUSE_X: &str = "%177";
const MOUSE_Y: &str = "%178";
const MUTE: [&str; 3] = ["%171", "%172", "%173"];
/// The voice window's layout shown.
const LAYOUT: &str = "%174";

pub(crate) const APPLY: &str = "conv_ps_apply";
const DEFAULTS: &str = "conv_ps_defaults";

// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const KNOB: u32 = 700; // + slider
const CLOSE: u32 = 710;
const RESET: u32 = 711;
const VOICE_TEST: u32 = 712;
const VOICE_OPEN: u32 = 713;
// The voice window, over the settings.
const VOICE_BACKDROP: u32 = 689;
const VOICE_BACKGROUND: u32 = 688;
const VOICE_ENTRY: u32 = 600; // + entry
const VOICE_CHECK: u32 = 540; // + entry
const MAX_VOICE_ENTRIES: usize = 60;
const CHOSEN: u32 = 720; // + 2 × pair (+ 1 off)
const PAIR: u32 = 750; // + 2 × pair (+ 1 off)
const TRACK: u32 = 780; // + slider
const TEXT: u32 = 790; // + text
const FIXED: u32 = 800; // + picture
const MAX_FIXED: usize = 100;
// Buttons.
const BTN_CLOSE: i32 = 1;
const BTN_RESET: i32 = 2;
const BTN_VOICE_TEST: i32 = 3;
const BTN_VOICES: i32 = 4;
const BTN_VOICE_ENTRY: i32 = 100; // + entry
const BTN_TRACK: i32 = 10; // + slider
const BTN_PAIR: i32 = 30; // + 2 × pair (+ 1 off)

/// The questions [`PairOption::Confirm`] turns off.
const QUESTIONS: [Switch; 7] =
    [Switch::ConfirmSave, Switch::ConfirmLoad, Switch::ConfirmDelete, Switch::ConfirmTitle, Switch::ConfirmQuit, Switch::ConfirmBackToChoice, Switch::ConfirmReset];

fn slider_var(i: usize) -> String {
    format!("%{}", SLIDER_BASE + i as u32)
}

fn pair_var(i: usize) -> String {
    format!("%{}", PAIR_BASE + i as u32)
}

/// At `*start`: the defaults on the first start, then the values applied.
pub(crate) fn init(out: &mut String, s: &PictureSettings) {
    out.push_str(&format!("if {SET} == 0 gosub *{DEFAULTS}\ngosub *{APPLY}\n"));
    // `reset` comes back here: the title runs at the normal speed.
    if s.sliders.iter().any(|l| matches!(l.setting, SliderSetting::GameSpeed { .. })) {
        out.push_str("gamespeed 100\n");
    }
}

/// Whether skipping text not read before is a choice (`kidokuskip`).
pub(crate) fn skips_unread(s: &PictureSettings) -> bool {
    s.pairs.iter().any(|p| p.option == PairOption::SkipUnread)
}

pub(crate) fn emit(out: &mut String, s: &PictureSettings, border: u32) {
    assert!(s.sliders.len() <= MAX_SLIDERS && s.pairs.len() <= MAX_PAIRS && s.fixed.len() <= MAX_FIXED, "too many settings controls");
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let strip = |slot: u32, cells: u32, p: &Placed| format!("lsp {slot},\":a/{cells},0,3;{}\",{},{}", path(&p.image), p.x, p.y);
    let fade = if s.fade_ms > 0 { format!("print 10,{}", s.fade_ms) } else { "print 1".into() };
    let pair_fade = if s.pair_fade_ms > 0 { format!("print 10,{}", s.pair_fade_ms) } else { "print 1".into() };
    let buttons: Vec<(u32, i32, &Placed)> = [(CLOSE, BTN_CLOSE, Some(&s.close)), (RESET, BTN_RESET, s.reset.as_ref()), (VOICE_TEST, BTN_VOICE_TEST, s.voice_test.as_ref()), (VOICE_OPEN, BTN_VOICES, s.voice_window.as_ref().map(|v| &v.open))]
        .into_iter()
        .filter_map(|(slot, n, p)| Some((slot, n, p?)))
        .collect();

    w(format!("*{}", menu::SETTINGS));
    w(format!("if {IN_GAME} == 1 textoff"));
    w(format!("pushscreen {},{MAX_SPRITE}", crate::ANIMATION_SPRITE));
    // pushscreen keeps the scene's sprites up: the screen covers them.
    w(format!("for {TMP} = {} to {MAX_SPRITE}:csp {TMP}:next", crate::ANIMATION_SPRITE));
    let b = &s.background;
    w(format!("lsp {BACKGROUND},\":c;{}\",{},{}", path(&b.image), b.x, b.y));
    for (k, p) in s.fixed.iter().enumerate() {
        w(format!("lsp {},\":a;{}\",{},{}", FIXED + k as u32, path(&p.image), p.x, p.y));
    }
    for (k, t) in s.texts.iter().enumerate() {
        let columns = t.text.chars().count();
        w(format!("strsp {},\"{}\",{},{},{columns},1,{f},{f},0,0,0,0,\"{}\"", TEXT + k as u32, escape(&t.text), t.x, t.y, t.colour, f = t.font_size));
    }
    for &(slot, _, p) in &buttons {
        w(strip(slot, 3, p));
    }
    for (i, p) in s.pairs.iter().enumerate() {
        w(strip(PAIR + 2 * i as u32, 3, &p.on));
        w(strip(PAIR + 2 * i as u32 + 1, 3, &p.off));
    }
    for (i, l) in s.sliders.iter().enumerate() {
        let t = &l.track;
        w(format!("lsp {},\":c;>{},{},#000000\",{},{},0", TRACK + i as u32, t.w, t.h, t.x, t.y));
        w(strip(KNOB + i as u32, 2, &l.knob));
    }
    w("gosub *conv_ps_draw".into());
    w(fade.clone());

    w("*conv_ps_wait".into());
    w("btndef clear".into());
    let knobs_at_rest: String = (0..s.sliders.len()).map(|i| format!("P{},0", KNOB + i as u32)).collect();
    w(format!("exbtn_d \"{knobs_at_rest}\""));
    for i in 0..s.sliders.len() {
        w(format!("exbtn {},{},\"{knobs_at_rest}P{},1\"", TRACK + i as u32, BTN_TRACK + i as i32, KNOB + i as u32));
    }
    for &(slot, n, _) in &buttons {
        w(format!("spbtn {slot},{n},2"));
    }
    for i in 0..s.pairs.len() {
        for side in 0..2 {
            w(format!("spbtn {},{}", PAIR + 2 * i as u32 + side, BTN_PAIR + 2 * i as i32 + side as i32));
        }
    }
    // After btndef, which turns it off: a press on a track starts a drag.
    w("btndown 1".into());
    w("getkey".into());
    w(format!("btnwait {BUTTON_VAR}"));
    // A right-click, or the screen's key (C), leaves.
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {} goto *conv_ps_close", menu::key_code('C')));
    // A release, or a move with the button held: nothing.
    w(format!("isdown {TMP}"));
    w(format!("if {TMP} == 0 goto *conv_ps_wait"));
    if !s.sliders.is_empty() {
        w(format!(
            "if {BUTTON_VAR} >= {BTN_TRACK} && {BUTTON_VAR} < {} mov {DRAGGED},{BUTTON_VAR}-{BTN_TRACK}:goto *conv_ps_drag",
            BTN_TRACK + s.sliders.len() as i32
        ));
    }
    w("gosub *conv_ps_release".into());
    w(format!("if {BUTTON_VAR} == {BTN_CLOSE} goto *conv_ps_close"));
    if s.reset.is_some() {
        w(format!("if {BUTTON_VAR} == {BTN_RESET} goto *conv_ps_reset"));
    }
    if s.voice_window.is_some() {
        w(format!("if {BUTTON_VAR} == {BTN_VOICES} goto *conv_ps_voices"));
    }
    if let Some(v) = &s.test_voice {
        w(format!("if {BUTTON_VAR} == {BTN_VOICE_TEST} dwave 0,\"{}\"", path(v)));
    }
    for i in 0..s.pairs.len() {
        let (n, v) = (BTN_PAIR + 2 * i as i32, pair_var(i));
        w(format!("if {BUTTON_VAR} == {n} && {v} == 0 mov {v},1:goto *conv_ps_changed"));
        w(format!("if {BUTTON_VAR} == {} && {v} == 1 mov {v},0:goto *conv_ps_changed", n + 1));
    }
    w("goto *conv_ps_wait".into());
    w("*conv_ps_changed".into());
    w(format!("gosub *{APPLY}:gosub *conv_ps_draw"));
    w(pair_fade);
    w("goto *conv_ps_wait".into());
    w("*conv_ps_release".into());
    w(format!("isdown {TMP}"));
    w(format!("if {TMP} == 1 wait 16:goto *conv_ps_release"));
    w("return".into());

    if s.reset.is_some() {
        w("*conv_ps_reset".into());
        w(menu::confirm_call("", Switch::ConfirmReset));
        w(format!("if {CONFIRM} == 0 goto *conv_ps_wait"));
        w(format!("gosub *{DEFAULTS}:gosub *{APPLY}:gosub *conv_ps_draw"));
        w(fade.clone());
        w("goto *conv_ps_wait".into());
    }

    // Dragging a knob: its middle follows the pointer.
    if !s.sliders.is_empty() {
        w("*conv_ps_drag".into());
        w(format!("isdown {TMP}"));
        w(format!("if {TMP} == 0 goto *conv_ps_wait"));
        w(format!("getmousepos {MOUSE_X},{MOUSE_Y}"));
        for (i, l) in s.sliders.iter().enumerate() {
            let (k0, k1) = l.knob_y;
            let v = slider_var(i);
            w(format!("if {DRAGGED} == {i} mov {TMP},({MOUSE_Y}-{}-{k0})*1000/{}", l.knob.h / 2, (k1 - k0).max(1)));
            w(format!("if {DRAGGED} == {i} && {TMP} < 0 mov {TMP},0"));
            w(format!("if {DRAGGED} == {i} && {TMP} > 1000 mov {TMP},1000"));
            w(format!("if {DRAGGED} == {i} && {TMP} != {v} mov {v},{TMP}:gosub *{APPLY}:gosub *conv_ps_draw"));
            w(format!("if {DRAGGED} == {i} cell {},1", KNOB + i as u32));
        }
        w("print 1".into());
        w("wait 16".into());
        w("goto *conv_ps_drag".into());
    }

    w("*conv_ps_close".into());
    w("btndown 0:btndef clear:exbtn_d \"\"".into());
    let mut controls: Vec<u32> = buttons.iter().map(|b| b.0).collect();
    controls.extend((0..s.sliders.len() as u32).map(|i| KNOB + i));
    if !controls.is_empty() {
        w(controls.iter().map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    }
    w("popscreen".into());
    // The scene first, then the window: texton drops the pending screen
    // updates but the window's.
    w(fade.clone());
    w(format!("if {IN_GAME} == 1 texton"));
    w("return".into());

    // The chosen side of each pair, the knobs.
    w("*conv_ps_draw".into());
    for (i, p) in s.pairs.iter().enumerate() {
        let (on, off) = (CHOSEN + 2 * i as u32, CHOSEN + 2 * i as u32 + 1);
        let v = pair_var(i);
        w(format!("if {v} == 1 csp {off}:{}:cell {on},2", strip(on, 3, &p.on)));
        w(format!("if {v} == 0 csp {on}:{}:cell {off},2", strip(off, 3, &p.off)));
    }
    for (i, l) in s.sliders.iter().enumerate() {
        let (k0, k1) = l.knob_y;
        w(format!("mov {TMP},{}*{}/1000+{k0}", slider_var(i), k1 - k0));
        w(format!("amsp {},{},{TMP}", KNOB + i as u32, l.knob.x));
    }
    w("return".into());

    if let Some(v) = &s.voice_window {
        voice_window(&mut w, v, border, &fade);
    }

    w(format!("*{DEFAULTS}"));
    for (i, l) in s.sliders.iter().enumerate() {
        w(format!("mov {},{}", slider_var(i), l.default));
    }
    for (i, p) in s.pairs.iter().enumerate() {
        w(format!("mov {},{}", pair_var(i), u8::from(p.default_on)));
    }
    w(format!("mov {SET},1"));
    w("return".into());

    w(format!("*{APPLY}"));
    for (i, l) in s.sliders.iter().enumerate() {
        let v = slider_var(i);
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
        let v = pair_var(i);
        match p.option {
            PairOption::Confirm => {
                for q in QUESTIONS {
                    w(format!("mov {},1-{v}", menu::switch_off(q)));
                }
            }
            PairOption::MovieStop => w(format!("moviestop {v}")),
            PairOption::StopAtChoice => w(format!("mov {TMP},1-{v}:selectkeep {TMP}")),
            PairOption::SkipUnread => w(format!("if {v} == 1 kidokumode 0\nif {v} == 0 kidokumode 1")),
            PairOption::VoicePlaysOn => w(format!("mov {TMP},1-{v}:clickvoicestop {TMP}")),
            PairOption::AutoWaitsForVoice => w(format!("autovoicewait {v}")),
            PairOption::Bgm => mute[0] = Some(v),
            PairOption::Se => mute[1] = Some(v),
            PairOption::Voice => mute[2] = Some(v),
        }
    }
    if mute.iter().any(Option::is_some) {
        for (var, on) in MUTE.iter().zip(&mute) {
            match on {
                Some(v) => w(format!("mov {var},1-{v}")),
                None => w(format!("mov {var},0")),
            }
        }
        w(format!("soundmute {},{},{}", MUTE[0], MUTE[1], MUTE[2]));
    }
    w("return".into());
}

/// The voice window's routines (`*conv_ps_voices`, back to the settings'
/// wait when it closes).
fn voice_window(w: &mut impl FnMut(String), v: &VoiceWindow, border: u32, fade: &str) {
    assert!(v.layouts.iter().all(|l| l.entries.len() <= MAX_VOICE_ENTRIES), "too many voice entries");
    let fade = if v.fade_ms > 0 { format!("print 10,{}", v.fade_ms) } else { fade.to_string() };
    let off = |c: u32| format!("%{}", menu::VOICE_OFF_BASE + c);
    // The test that shows entry k of layout l.
    let shown = |l: usize, heard: Option<Var>| match heard {
        Some(h) => format!("{LAYOUT} == {l} && {} == 1", var_name(h, border)),
        None => format!("{LAYOUT} == {l}"),
    };
    let (b, opacity) = &v.backdrop;
    w("*conv_ps_voices".into());
    w(format!("lsp {VOICE_BACKDROP},\":c;{}\",{},{},{opacity}", path(&b.image), b.x, b.y));
    let g = &v.background;
    w(format!("lsp {VOICE_BACKGROUND},\":a;{}\",{},{}", path(&g.image), g.x, g.y));
    w(format!("mov {LAYOUT},{}", v.layouts.len().saturating_sub(1)));
    for (l, layout) in v.layouts.iter().enumerate().rev() {
        if let Some(when) = layout.when {
            w(format!("if {} == 1 mov {LAYOUT},{l}", var_name(when, border)));
        }
    }
    w("gosub *conv_ps_voices_draw".into());
    w(fade.clone());

    w("*conv_ps_voices_wait".into());
    w("btndef clear".into());
    for (l, layout) in v.layouts.iter().enumerate() {
        for (k, e) in layout.entries.iter().enumerate().filter(|(_, e)| !e.characters.is_empty()) {
            let p = &e.picture;
            w(format!("if {} btn {},{},{},{},{},0,0", shown(l, e.heard), BTN_VOICE_ENTRY + k as i32, p.x, p.y, e.hit.0, e.hit.1));
        }
    }
    w("getkey".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {} goto *conv_ps_voices_close", menu::key_code('C')));
    for (l, layout) in v.layouts.iter().enumerate() {
        for (k, e) in layout.entries.iter().enumerate() {
            let Some(&first) = e.characters.first() else { continue };
            let all = |to: u32| e.characters.iter().map(|&c| format!("mov {},{to}", off(c))).collect::<Vec<_>>().join(":");
            let n = BTN_VOICE_ENTRY + k as i32;
            w(format!("if {LAYOUT} == {l} && {BUTTON_VAR} == {n} && {} == 0 {}:goto *conv_ps_voices_toggled", off(first), all(1)));
            w(format!("if {LAYOUT} == {l} && {BUTTON_VAR} == {n} {}:goto *conv_ps_voices_toggled", all(0)));
        }
    }
    w("goto *conv_ps_voices_wait".into());
    w("*conv_ps_voices_toggled".into());
    w("gosub *conv_ps_voices_draw:print 1".into());
    w("goto *conv_ps_voices_wait".into());

    w("*conv_ps_voices_close".into());
    w("btndef clear".into());
    w(format!("for {TMP} = {VOICE_CHECK} to {VOICE_BACKDROP}:csp {TMP}:next"));
    w(fade);
    w("goto *conv_ps_wait".into());

    w("*conv_ps_voices_draw".into());
    w(format!("for {TMP} = {VOICE_CHECK} to {}:csp {TMP}:next", VOICE_ENTRY as usize + MAX_VOICE_ENTRIES - 1));
    let c = &v.check;
    for (l, layout) in v.layouts.iter().enumerate() {
        for (k, e) in layout.entries.iter().enumerate() {
            let p = &e.picture;
            let cond = shown(l, e.heard);
            w(format!("if {cond} lsp {},\":a;{}\",{},{}", VOICE_ENTRY + k as u32, path(&p.image), p.x, p.y));
            // An entry with no voices in the story is always heard.
            let heard = e.characters.first().map_or(String::new(), |&first| format!(" && {} == 0", off(first)));
            w(format!("if {cond}{heard} lsp {},\":a;{}\",{},{}", VOICE_CHECK + k as u32, path(&c.image), p.x, p.y));
        }
    }
    w("return".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{PicturePair, PictureSlider, PictureText, Rect, VoiceEntry, VoiceLayout};

    fn placed(name: &str, x: i32, y: i32) -> Placed {
        Placed { image: format!("system/{name}.png"), x, y, w: 24, h: 7 }
    }

    fn settings() -> PictureSettings {
        PictureSettings {
            background: placed("bg", 0, 0),
            fixed: vec![placed("aspect", 215, 51)],
            close: placed("exit", 26, 108),
            reset: Some(placed("reset", 899, 197)),
            voice_test: Some(placed("test", 287, 197)),
            test_voice: Some("voice/test.ogg".into()),
            sliders: vec![
                PictureSlider { setting: SliderSetting::VoiceVolume, knob: placed("knob", 358, 67), knob_y: (67, 206), track: Rect { x: 358, y: 67, w: 24, h: 147 }, default: 207 },
                PictureSlider {
                    setting: SliderSetting::GameSpeed { levels: [50, 75, 100, 150, 200, 300, 400, 550, 800, 1200, 1600, 3200] },
                    knob: placed("knob", 862, 67),
                    knob_y: (67, 206),
                    track: Rect { x: 862, y: 67, w: 24, h: 147 },
                    default: 750,
                },
            ],
            pairs: vec![
                PicturePair { option: PairOption::Confirm, on: placed("on", 827, 183), off: placed("off", 827, 213), default_on: true },
                PicturePair { option: PairOption::Voice, on: placed("on", 395, 183), off: placed("off", 395, 213), default_on: true },
            ],
            texts: vec![PictureText { text: "v1.00".into(), x: 453, y: 548, font_size: 11, colour: "#bb7420".into() }],
            fade_ms: 300,
            pair_fade_ms: 100,
            voice_window: Some(VoiceWindow {
                open: placed("open", 323, 151),
                backdrop: (placed("black", 0, 0), 178),
                background: placed("voices", 0, 0),
                check: placed("check", 0, 0),
                layouts: vec![
                    VoiceLayout { when: Some(Var { global: true, index: 50 }), entries: vec![VoiceEntry { picture: placed("a", 790, 82), hit: (15, 142), characters: vec![3, 7], heard: Some(Var { global: true, index: 10 }) }] },
                    VoiceLayout { when: None, entries: vec![VoiceEntry { picture: placed("b", 214, 376), hit: (15, 142), characters: vec![], heard: None }] },
                ],
                fade_ms: 300,
            }),
        }
    }

    #[test]
    fn picture_settings_routine() {
        let mut out = String::new();
        emit(&mut out, &settings(), 800);
        for line in [
            "*conv_settings\n",
            "lsp 800,\":a;system\\aspect.png\",215,51\n",
            "strsp 790,\"v1.00\",453,548,5,1,11,11,0,0,0,0,\"#bb7420\"\n",
            // The track shows its knob's hover cell.
            "exbtn_d \"P700,0P701,0\"\nexbtn 780,10,\"P700,0P701,0P700,1\"\n",
            "spbtn 710,1,2\n",
            "if %9 == 30 && %2930 == 0 mov %2930,1:goto *conv_ps_changed\n",
            "if %9 == 31 && %2930 == 1 mov %2930,0:goto *conv_ps_changed\n",
            "if %9 == 3 dwave 0,\"voice\\test.ogg\"\n",
            "if %170 == 0 mov %105,(%178-3-67)*1000/139\n",
            "if %2931 == 1 csp 723:lsp 722,\":a/3,0,3;system\\on.png\",395,183:cell 722,2\n",
            "mov %105,%2922*139/1000+67\namsp 700,358,%105\n",
            "mov %2922,207\nmov %2923,750\nmov %2930,1\nmov %2931,1\nmov %2920,1\n",
            "mov %105,(1000-%2922)/10:mov %105,(%105*%105/50+%105)/3:voicevol %105\n",
            "if %105 == 3 mov %2921,100\n",
            "mov %3642,1-%2930\n",
            "mov %2914,1-%2930\n",
            "mov %171,0\nmov %172,0\nmov %173,1-%2931\nsoundmute %171,%172,%173\n",
            // The voice window: layout 0 when its flag is set.
            "if %9 == 4 goto *conv_ps_voices\n",
            "mov %174,1\nif %850 == 1 mov %174,0\n",
            "if %174 == 0 && %810 == 1 btn 100,790,82,15,142,0,0\n",
            "if %174 == 0 && %9 == 100 && %973 == 0 mov %973,1:mov %977,1:goto *conv_ps_voices_toggled\n",
            "if %174 == 0 && %810 == 1 && %973 == 0 lsp 540,\":a;system\\check.png\",790,82\n",
            "if %174 == 1 lsp 540,\":a;system\\check.png\",214,376\n",
        ] {
            assert!(out.contains(line), "missing {line:?} in\n{out}");
        }
    }
}
