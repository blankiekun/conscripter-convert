//! [`Script::boot`]: what the game shows at launch before the title.

use std::fmt::Write;

use conv_core::ir::{BootStep, Op, Script};

use crate::TITLE_SHOWN;

/// A local, 1 from a [`BootStep::gosub`]'s end to the title's music: the
/// boot left it playing, so the title doesn't start it again (`reset`
/// clears it).
pub(crate) const BGM_ON: &str = "%conv_boot_bgm";
/// Its declaration, in the define section.
pub(crate) const DEFINE: &str = "defvar conv_boot_bgm";

/// A step is gosubbed once a run ([`TITLE_SHOWN`] 0 until the title is up).
pub(crate) fn once(script: &Script) -> bool {
    script.boot.iter().any(|s| s.gosub.is_some())
}

/// A [`BootStep::gosub`] leaves the title's music playing.
pub(crate) fn keeps_bgm(script: &Script) -> bool {
    let Some(bgm) = script.title_screen.as_ref().and_then(|t| t.bgm.as_ref()) else {
        return false;
    };
    let mut playing = None;
    for label in script.boot.iter().filter_map(|s| s.gosub.as_ref()) {
        let body = script.ops.iter().skip_while(|op| !matches!(op, Op::Label(l) if l == label)).take_while(|op| !matches!(op, Op::Return));
        for op in body {
            match op {
                Op::BgmPlay { file, .. } | Op::BgmCrossfade { file, .. } => playing = Some(file),
                Op::BgmStop { .. } => playing = None,
                _ => {}
            }
        }
    }
    playing == Some(bgm)
}

/// The steps in order. A click cuts a fade only where the step says
/// (`effectskip`); a skippable hold is a `wait` for its fixed part, then a
/// `delay`.
pub(crate) fn emit(out: &mut String, script: &Script, border: u32) {
    if script.boot.is_empty() {
        return;
    }
    for (i, s) in script.boot.iter().enumerate() {
        if !s.when.is_empty() {
            let cond: Vec<String> = s.when.iter().map(|(v, value)| format!("{} == {value}", crate::var_name(*v, border))).collect();
            let tmp = crate::menu::TMP;
            writeln!(out, "mov {tmp},0:if {} mov {tmp},1", cond.join(" && ")).unwrap();
            writeln!(out, "if {tmp} == 0 goto *conv_boot_skip{i}").unwrap();
        }
        match &s.gosub {
            Some(label) => gosub(out, script, label),
            None => step(out, s),
        }
        if !s.when.is_empty() {
            writeln!(out, "*conv_boot_skip{i}").unwrap();
        }
    }
    writeln!(out, "effectskip 1").unwrap();
}

/// While the title hasn't been shown in this run; the title's openings
/// then mark it shown, else this does.
fn gosub(out: &mut String, script: &Script, label: &str) {
    let mut then = format!("gosub *{label}");
    if script.title_screen.as_ref().is_none_or(|t| t.openings.is_empty()) {
        write!(then, ":mov {TITLE_SHOWN},1").unwrap();
    }
    if keeps_bgm(script) {
        write!(then, ":mov {BGM_ON},1").unwrap();
    }
    writeln!(out, "effectskip 0").unwrap();
    writeln!(out, "if {TITLE_SHOWN} == 0 {then}").unwrap();
}

fn step(out: &mut String, s: &BootStep) {
    let fade = |ms: u32| if ms == 0 { "1".to_string() } else { format!("{},{ms}", crate::FADE) };
    let image = crate::path(&s.image);
    writeln!(out, "effectskip {}", u8::from(s.click_skips_fades)).unwrap();
    writeln!(out, "bg {},1", s.backdrop).unwrap();
    if s.image.is_empty() {
        hold(out, s.hold_ms, s.skip_after_ms);
        return;
    }
    if s.movie {
        writeln!(out, "mpegplay \"{image}\",{}", u8::from(s.skip_after_ms.is_some())).unwrap();
        writeln!(out, "bg {},1", s.backdrop).unwrap();
        hold(out, s.hold_ms, None);
        return;
    }
    writeln!(out, "bg \"{image}\",{}", fade(s.fade_in_ms)).unwrap();
    if s.until_click {
        writeln!(out, "click").unwrap();
    } else {
        hold(out, s.hold_ms, s.skip_after_ms);
    }
    writeln!(out, "bg {},{}", s.fade_out_to.as_deref().unwrap_or(&s.backdrop), fade(s.fade_out_ms)).unwrap();
}

fn hold(out: &mut String, ms: u32, skip_after: Option<u32>) {
    let fixed = skip_after.map_or(ms, |s| s.min(ms));
    if fixed > 0 {
        writeln!(out, "wait {fixed}").unwrap();
    }
    if ms > fixed {
        writeln!(out, "delay {}", ms - fixed).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::Var;

    fn emitted(boot: Vec<BootStep>) -> String {
        let mut out = String::new();
        emit(&mut out, &Script { boot, ..Default::default() }, 3000);
        out
    }

    #[test]
    fn logo_then_movie() {
        let logo = BootStep { image: "system/logo.png".into(), backdrop: "white".into(), fade_in_ms: 567, hold_ms: 4000, fade_out_ms: 567, skip_after_ms: Some(3000), ..Default::default() };
        let movie = BootStep { image: "movie/op.mp4".into(), movie: true, backdrop: "black".into(), hold_ms: 1000, skip_after_ms: Some(0), ..Default::default() };
        let fade = crate::FADE;
        assert_eq!(emitted(vec![logo, movie]), format!("effectskip 0\nbg white,1\nbg \"system\\logo.png\",{fade},567\nwait 3000\ndelay 1000\nbg white,{fade},567\neffectskip 0\nbg black,1\nmpegplay \"movie\\op.mp4\",1\nbg black,1\nwait 1000\neffectskip 1\n"));
    }

    #[test]
    fn backdrop_alone_fade_out_to_and_when() {
        let pause = BootStep { backdrop: "black".into(), hold_ms: 250, ..Default::default() };
        let caution = BootStep { image: "c.png".into(), backdrop: "white".into(), fade_in_ms: 500, hold_ms: 2000, fade_out_ms: 500, fade_out_to: Some("black".into()), when: vec![(Var { index: 1, global: true }, 1)], ..Default::default() };
        let fade = crate::FADE;
        assert_eq!(
            emitted(vec![pause, caution]),
            format!("effectskip 0\nbg black,1\nwait 250\nmov %105,0:if %3001 == 1 mov %105,1\nif %105 == 0 goto *conv_boot_skip1\neffectskip 0\nbg white,1\nbg \"c.png\",{fade},500\nwait 2000\nbg black,{fade},500\n*conv_boot_skip1\neffectskip 1\n")
        );
    }

    #[test]
    fn until_click_with_skippable_fades() {
        let warning = BootStep { image: "w.png".into(), backdrop: "black".into(), fade_in_ms: 1000, fade_out_ms: 1000, until_click: true, click_skips_fades: true, ..Default::default() };
        let fade = crate::FADE;
        assert_eq!(emitted(vec![warning]), format!("effectskip 1\nbg black,1\nbg \"w.png\",{fade},1000\nclick\nbg black,{fade},1000\neffectskip 1\n"));
    }
}
