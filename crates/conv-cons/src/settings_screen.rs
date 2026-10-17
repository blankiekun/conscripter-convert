//! [`PictureSettings`] drawn by the engine (`screen "settings"`,
//! COMMANDS.md "Screens") when it has an [`engine_screen`]: the picture,
//! its fixed pictures and texts, each pair's sides as radios (the chosen
//! side's picture while on), the buttons, and each slider as a picture
//! slider (its knob dragged along an invisible track). The values are the
//! named globals [`picture_settings`] keeps, applied by its
//! [`APPLY`] as they change. The reset question is a dialog of its own
//! (`settings_confirm`: the game's question picture), the voice window a
//! screen over the settings (`settings_voices`).
//!
//! The screens fade in and out, a pair's chosen side cross-fades, and the
//! reset question unrolls as the game's dialog does.
//!
//! [`engine_screen`]: PictureSettings::engine_screen
//! [`APPLY`]: picture_settings::APPLY

use std::fmt::Write as _;

use conv_core::ir::{PictureSettings, Script, Switch, VoiceWindow};

use crate::menu::IN_GAME;
use crate::picture_settings::{self, pair_var, slider_var, APPLY, DEFAULTS};
use crate::screens::{image, lua, question};
use crate::{path, var_name};

const RESET: &str = "conv_ps_reset";
const VOICE_TEST: &str = "conv_ps_test";
/// Sets each lying slider's shown value from its position.
const SHOW: &str = "conv_ps_show";
/// Puts each lying slider's position back from its shown value, then [`APPLY`].
const LIE: &str = "conv_ps_lie";

/// A lying slider's position runs from its right end (the IR's), the
/// engine's from the left (where its lit bar starts): the screen moves
/// this, 1000 - the position.
pub(crate) fn shown_var(s: &PictureSettings, i: usize) -> String {
    format!("{}_shown", slider_var(s, i))
}

fn lying(s: &PictureSettings) -> impl Iterator<Item = usize> + '_ {
    s.sliders.iter().enumerate().filter(|(_, l)| l.horizontal).map(|(i, _)| i)
}

/// The settings' files under `screens/`, if the engine draws them.
pub(crate) fn files(script: &Script, border: u32) -> Vec<(String, String)> {
    let Some(s) = script.picture_settings.as_ref().filter(|s| s.engine_screen.is_some()) else { return Vec::new() };
    let question = script.confirm.as_ref().and_then(|d| Some((d, &d.pictures.iter().find(|(q, _)| *q == Switch::ConfirmReset)?.1)));
    let mut files = vec![("settings.lua".to_string(), settings_lua(s, question.is_some()))];
    if let Some((d, picture)) = question {
        let cells = if d.pressed { 4 } else { 3 };
        let pressed = if d.pressed { ", pressed=3" } else { "" };
        let mut out = String::from("-- The reset question (conscripter-convert, from ConfirmDialog).\nreturn {\n  kind = \"confirm\",\n");
        // It unrolls out of a line as it fades in, and back as it fades out;
        // the backdrop only fades.
        let backdrop_fx = match &d.unroll {
            Some(u) => {
                let at = match u.y {
                    Some(y) => format!("zoom_y=0, y={y}"),
                    None => format!("zoom_x=0, x={}", u.x),
                };
                writeln!(out, "  open = {{ms={}, {at}, alpha=0}},\n  close = {{ms={}, {at}, alpha=0}},", d.fade_ms, u.close_ms).unwrap();
                format!(", open={{ms={}, alpha=0}}, close={{ms={}, alpha=0}}", d.fade_ms, u.close_ms)
            }
            None if d.fade_ms > 0 => {
                writeln!(out, "  fade = {},", d.fade_ms).unwrap();
                String::new()
            }
            None => String::new(),
        };
        out.push_str("  widgets = {\n");
        if let Some((b, opacity)) = &d.backdrop {
            writeln!(out, "    {},", image("image", b, &format!(", alpha={opacity}{backdrop_fx}"))).unwrap();
        }
        writeln!(out, "    {},", image("image", picture, "")).unwrap();
        for (p, value) in [(&d.yes, 1), (&d.no, 0)] {
            writeln!(out, "    {},", image("button", p, &format!(", cells={cells}{pressed}, action={{\"close\", {value}}}"))).unwrap();
        }
        out.push_str("  },\n}\n");
        files.push(("settings_confirm.lua".to_string(), out));
    }
    if let (Some(v), Some(e)) = (&s.voice_window, &s.engine_screen) {
        files.push(("settings_voices.lua".to_string(), voices_lua(v, &e.voices_checked, border)));
    }
    files
}

/// `*conv_settings`, and what its buttons run.
pub(crate) fn labels(out: &mut String, s: &PictureSettings) {
    let lies = lying(s).next().is_some();
    let show = if lies { format!("gosub *{SHOW}\n") } else { String::new() };
    writeln!(out, "*{}\n{show}screen \"settings\"\nreturn", crate::menu::SETTINGS).unwrap();
    if s.reset.is_some() {
        writeln!(out, "*{RESET}\ngosub *{DEFAULTS}\ngosub *{APPLY}\n{show}return").unwrap();
    }
    if lies {
        writeln!(out, "*{SHOW}").unwrap();
        for i in lying(s) {
            writeln!(out, "mov {},1000-{}", shown_var(s, i), slider_var(s, i)).unwrap();
        }
        writeln!(out, "return\n*{LIE}").unwrap();
        for i in lying(s) {
            writeln!(out, "mov {},1000-{}", slider_var(s, i), shown_var(s, i)).unwrap();
        }
        writeln!(out, "goto *{APPLY}").unwrap();
    }
    if let Some(v) = &s.test_voice {
        writeln!(out, "*{VOICE_TEST}\ndwave 0,\"{}\"\nreturn", path(v)).unwrap();
    }
}

fn settings_lua(s: &PictureSettings, asks: bool) -> String {
    let e = s.engine_screen.as_ref().expect("an engine screen");
    assert!(s.sliders.iter().all(|l| l.steps.is_none() && (l.horizontal || l.fill.is_none())), "the engine's settings have knobs, a lit bar only lying");
    assert!(e.chosen.len() == s.pairs.len(), "a chosen picture pair per pair");
    let apply = format!("action={{\"gosub\", \"*{APPLY}\"}}");
    // A pair's chosen side cross-fades in.
    let pair_fade = if s.pair_fade_ms > 0 { format!(", fade={}", s.pair_fade_ms) } else { String::new() };
    let button = ", cells=3, pressed=2";
    let mut w = vec![image("image", &s.background, ", opaque=true")];
    w.extend(s.fixed.iter().map(|p| image("image", p, "")));
    for t in &s.texts {
        w.push(format!("{{\"text\", text={}, x={}, y={}, size={}, cols={}, color={}}}", lua(&t.text), t.x, t.y, t.font_size, t.text.chars().count(), lua(&t.colour)));
    }
    for (i, (p, (on, off))) in s.pairs.iter().zip(&e.chosen).enumerate() {
        let var = lua(&pair_var(s, i));
        if picture_settings::check_box(p) {
            // Only the current side shows; a click switches to the other.
            w.push(image("toggle", &p.off, &format!(", cells=3, pressed=2, on_file={}, var={var}, {apply}", lua(&p.on.image))));
            continue;
        }
        for (side, chosen, value) in [(&p.on, on, 1), (&p.off, off, 0)] {
            w.push(image("radio", side, &format!(", cells=3, on_file={}, var={var}, value={value}{pair_fade}, {apply}", lua(&chosen.image))));
        }
    }
    w.push(image("button", &s.close, &format!("{button}, action={{\"close\"}}")));
    if let Some(p) = &s.reset {
        let confirm = if asks { format!(", confirm={}", question("reset", Switch::ConfirmReset)) } else { String::new() };
        w.push(image("button", p, &format!("{button}, action={{\"gosub\", \"*{RESET}\"{confirm}}}")));
    }
    if let (Some(p), Some(_)) = (&s.voice_test, &s.test_voice) {
        w.push(image("button", p, &format!("{button}, action={{\"gosub\", \"*{VOICE_TEST}\"}}")));
    }
    if let Some(v) = &s.voice_window {
        w.push(image("button", &v.open, &format!("{button}, action={{\"open\", \"settings_voices\"}}")));
    }
    for (i, l) in s.sliders.iter().enumerate() {
        let (k0, k1) = l.knob_y;
        let t = &l.track;
        if l.horizontal {
            let fill = l.fill.as_ref().map(|(f, _)| format!(", fill={{file={}, x={}, y={}}}", lua(&f.image), f.x, f.y)).unwrap_or_default();
            w.push(format!(
                "{{\"slider\", var={}, min=0, max=1000, step=1, travel={}, knob={{file={}, cells=2, x={k0}, y={}}}, track={{x={}, y={}, w={}, h={}}}{fill}, action={{\"gosub\", \"*{LIE}\"}}}}",
                lua(&shown_var(s, i)),
                k1 - k0,
                lua(&l.knob.image),
                l.knob.y,
                t.x,
                t.y,
                t.w,
                t.h
            ));
            continue;
        }
        w.push(format!(
            "{{\"slider\", var={}, min=0, max=1000, step=1, vertical=true, travel={}, knob={{file={}, cells=2, x={}, y={k0}}}, track={{x={}, y={}, w={}, h={}}}, {apply}}}",
            lua(&slider_var(s, i)),
            k1 - k0,
            lua(&l.knob.image),
            l.knob.x,
            t.x,
            t.y,
            t.w,
            t.h
        ));
    }
    let mut out = format!("-- The settings (conscripter-convert, from PictureSettings).\nlocal in_game = var({}) == 1\nreturn {{\n  kind = \"settings\",\n", lua(IN_GAME));
    if s.fade_ms > 0 {
        writeln!(out, "  fade = {},", s.fade_ms).unwrap();
    }
    out.push_str("  hide = in_game and {window=true} or nil,\n  dialog = \"settings_confirm\",\n  widgets = {\n");
    for x in w {
        writeln!(out, "    {x},").unwrap();
    }
    out.push_str("  },\n}\n");
    out
}

/// The voice window: the first layout whose flag is set (or the last), an
/// entry shown once its flag is, a switch muting its characters' voices
/// (the check while heard); one with no voices always checked.
fn voices_lua(v: &VoiceWindow, checked: &[Vec<String>], border: u32) -> String {
    let (b, opacity) = &v.backdrop;
    let mut out = String::from("-- The voice window (conscripter-convert, from VoiceWindow).\n");
    writeln!(out, "local layout = {}", v.layouts.len()).unwrap();
    for (l, layout) in v.layouts.iter().enumerate().rev() {
        if let Some(when) = layout.when {
            writeln!(out, "if var({}) == 1 then layout = {} end", lua(&var_name(when, border)), l + 1).unwrap();
        }
    }
    writeln!(out, "local w = {{\n  {},\n  {},\n}}", image("image", b, &format!(", opaque=true, alpha={opacity}")), image("image", &v.background, "")).unwrap();
    for (l, (layout, checked)) in v.layouts.iter().zip(checked).enumerate() {
        writeln!(out, "if layout == {} then", l + 1).unwrap();
        for (e, on) in layout.entries.iter().zip(checked) {
            let p = &e.picture;
            let widget = if e.characters.is_empty() {
                format!("{{\"image\", file={}, x={}, y={}}}", lua(on), p.x, p.y)
            } else {
                let vars: Vec<String> = e.characters.iter().map(|c| lua(&format!("%{}", crate::menu::VOICE_OFF_BASE + c))).collect();
                image("toggle", p, &format!(", on_file={}, var={{{}}}, on=0, off=1", lua(on), vars.join(", ")))
            };
            match e.heard {
                Some(h) => writeln!(out, "  if var({}) == 1 then w[#w+1] = {widget} end", lua(&var_name(h, border))).unwrap(),
                None => writeln!(out, "  w[#w+1] = {widget}").unwrap(),
            }
        }
        out.push_str("end\n");
    }
    let fade = if v.fade_ms > 0 { format!("\n  fade = {},", v.fade_ms) } else { String::new() };
    writeln!(out, "return {{\n  kind = \"settings\",{fade}\n  widgets = w,\n}}").unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{ConfirmDialog, PairOption, PicturePair, PictureSlider, Placed, Rect, SettingsScreen, SliderSetting, Var, VoiceEntry, VoiceLayout};

    fn placed(name: &str, x: i32, y: i32) -> Placed {
        Placed { image: format!("system/{name}.png"), x, y, w: 24, h: 7 }
    }

    #[test]
    fn settings_become_a_screen() {
        let mut script = Script::default();
        let voice = VoiceWindow {
            open: placed("open", 323, 151),
            backdrop: (placed("black", 0, 0), 178),
            background: placed("voices", 0, 0),
            check: placed("check", 0, 0),
            layouts: vec![
                VoiceLayout { when: Some(Var { global: true, index: 50 }), entries: vec![VoiceEntry { picture: placed("a", 790, 82), hit: (15, 142), characters: vec![3, 7], heard: Some(Var { global: true, index: 10 }) }] },
                VoiceLayout { when: None, entries: vec![VoiceEntry { picture: placed("b", 214, 376), hit: (15, 142), characters: vec![], heard: None }] },
            ],
            fade_ms: 300,
        };
        let s = PictureSettings {
            background: placed("bg", 0, 0),
            fixed: vec![],
            close: placed("exit", 26, 108),
            reset: Some(placed("reset", 899, 197)),
            voice_test: Some(placed("test", 287, 197)),
            test_voice: Some("voice/test.ogg".into()),
            sliders: vec![PictureSlider { setting: SliderSetting::VoiceVolume, knob: placed("knob", 358, 67), knob_y: (67, 206), track: Rect { x: 358, y: 67, w: 24, h: 147 }, default: 207, steps: None, horizontal: false, fill: None }],
            pairs: vec![PicturePair { option: PairOption::Confirm, on: placed("on", 827, 183), off: placed("off", 827, 213), default_on: true }],
            texts: vec![],
            fade_ms: 300,
            pair_fade_ms: 100,
            voice_window: Some(voice),
            engine_screen: Some(SettingsScreen { chosen: vec![(placed("on3", 827, 183), placed("off3", 827, 213))], voices_checked: vec![vec!["system/a_c.png".into()], vec!["system/b_c.png".into()]] }),
        };
        script.picture_settings = Some(s.clone());
        let files = files(&script, 800);
        let lua_of = |name: &str| &files.iter().find(|f| f.0 == name).unwrap().1;
        let settings = lua_of("settings.lua");
        for line in [
            "{\"radio\", file=\"system/on.png\", x=827, y=183, cells=3, on_file=\"system/on3.png\", var=\"%conv_set_confirm\", value=1, fade=100, action={\"gosub\", \"*conv_ps_apply\"}}",
            "  fade = 300,\n  hide = in_game",
            "{\"slider\", var=\"%conv_set_voice_volume\", min=0, max=1000, step=1, vertical=true, travel=139, knob={file=\"system/knob.png\", cells=2, x=358, y=67}, track={x=358, y=67, w=24, h=147}",
            "action={\"open\", \"settings_voices\"}",
            "action={\"gosub\", \"*conv_ps_test\"}",
        ] {
            assert!(settings.contains(line), "missing {line:?} in\n{settings}");
        }
        // No confirm dialog in the script: the reset doesn't ask.
        assert!(settings.contains("action={\"gosub\", \"*conv_ps_reset\"}}"), "{settings}");
        let voices = lua_of("settings_voices.lua");
        for line in [
            "kind = \"settings\",\n  fade = 300,\n",
            "local layout = 2\nif var(\"%850\") == 1 then layout = 1 end\n",
            "  if var(\"%810\") == 1 then w[#w+1] = {\"toggle\", file=\"system/a.png\", x=790, y=82, on_file=\"system/a_c.png\", var={\"%973\", \"%977\"}, on=0, off=1} end\n",
            "if layout == 2 then\n  w[#w+1] = {\"image\", file=\"system/b_c.png\", x=214, y=376}\nend\n",
        ] {
            assert!(voices.contains(line), "missing {line:?} in\n{voices}");
        }
        // The game's question unrolls out of x 512, its backdrop only fades.
        let yes = placed("yes", 515, 397);
        script.confirm = Some(ConfirmDialog {
            base: placed("q", 300, 200),
            yes: yes.clone(),
            no: yes,
            yes_label: String::new(),
            no_label: String::new(),
            text_y: 0,
            font_size: 20,
            overwrite: String::new(),
            load: String::new(),
            delete: String::new(),
            title: String::new(),
            quit: String::new(),
            pictures: vec![(Switch::ConfirmReset, placed("reset_q", 300, 200))],
            backdrop: Some((placed("black", 0, 0), 204)),
            pressed: true,
            fade_ms: 150,
            unroll: Some(conv_core::ir::Unroll { x: 512, close_ms: 200, y: None }),
        });
        let files = super::files(&script, 800);
        let question = &files.iter().find(|f| f.0 == "settings_confirm.lua").unwrap().1;
        for line in [
            "  open = {ms=150, zoom_x=0, x=512, alpha=0},\n  close = {ms=200, zoom_x=0, x=512, alpha=0},\n",
            "{\"image\", file=\"system/black.png\", x=0, y=0, alpha=204, open={ms=150, alpha=0}, close={ms=200, alpha=0}}",
        ] {
            assert!(question.contains(line), "missing {line:?} in\n{question}");
        }
        let mut out = String::new();
        picture_settings::emit(&mut out, &s, 800);
        assert!(out.starts_with("*conv_settings\nscreen \"settings\"\nreturn\n*conv_ps_reset\ngosub *conv_ps_defaults\ngosub *conv_ps_apply\nreturn\n*conv_ps_test\ndwave 0,\"voice\\test.ogg\"\nreturn\n"), "{out}");
        assert!(out.contains("mov %conv_set_voice_volume,207\nmov %conv_set_confirm,1\nmov %conv_set_done,1\n"), "{out}");
    }

    #[test]
    fn lying_sliders_and_check_boxes() {
        let mut script = Script::default();
        let s = PictureSettings {
            background: placed("bg", 0, 0),
            fixed: vec![],
            close: placed("exit", 854, 555),
            reset: None,
            voice_test: None,
            test_voice: None,
            sliders: vec![PictureSlider {
                setting: SliderSetting::BgmVolume,
                knob: placed("knob", 588, 84),
                knob_y: (588, 722),
                track: Rect { x: 588, y: 65, w: 141, h: 34 },
                default: 200,
                steps: None,
                horizontal: true,
                fill: Some((placed("full", 584, 72), "system/black.png".into())),
            }],
            pairs: vec![PicturePair { option: PairOption::Bgm, on: placed("heard", 769, 57), off: placed("muted", 769, 57), default_on: true }],
            texts: vec![],
            fade_ms: 300,
            pair_fade_ms: 0,
            voice_window: None,
            engine_screen: Some(SettingsScreen { chosen: vec![(placed("heard", 769, 57), placed("muted", 769, 57))], voices_checked: vec![] }),
        };
        script.picture_settings = Some(s.clone());
        let lua = &files(&script, 800)[0].1;
        for line in [
            "{\"toggle\", file=\"system/muted.png\", x=769, y=57, cells=3, pressed=2, on_file=\"system/heard.png\", var=\"%conv_set_bgm_on\", action=",
            "{\"slider\", var=\"%conv_set_bgm_volume_shown\", min=0, max=1000, step=1, travel=134, knob={file=\"system/knob.png\", cells=2, x=588, y=84}, track={x=588, y=65, w=141, h=34}, fill={file=\"system/full.png\", x=584, y=72}, action={\"gosub\", \"*conv_ps_lie\"}}",
        ] {
            assert!(lua.contains(line), "missing {line:?} in\n{lua}");
        }
        let mut out = String::new();
        picture_settings::emit(&mut out, &s, 800);
        assert!(out.starts_with("*conv_settings\ngosub *conv_ps_show\nscreen \"settings\"\nreturn\n*conv_ps_show\nmov %conv_set_bgm_volume_shown,1000-%conv_set_bgm_volume\nreturn\n*conv_ps_lie\nmov %conv_set_bgm_volume,1000-%conv_set_bgm_volume_shown\ngoto *conv_ps_apply\n"), "{out}");
        let mut define = String::new();
        picture_settings::define(&mut define, &s);
        assert!(define.ends_with("defvar conv_set_done,global\ndefvar conv_set_bgm_volume_shown\n"), "{define}");
    }
}
