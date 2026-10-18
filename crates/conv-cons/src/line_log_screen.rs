//! [`LineLog`] drawn by the engine (`screen "backlog"`, COMMANDS.md
//! "Screens"): the background, the
//! history's rows as `logrows` lays them out (a list of rows, each voiced
//! page's first row with its voice button), 逆/進 and the wheel a row back
//! or forward (the wheel past the newest row closes), the knob a picture
//! slider over the same variable, 退出. A row of single-byte text (the
//! English build's) the engine draws after a backquote.
//!
//! [`ROWS_BACK`] is how many rows back from the newest the list ends (0:
//! the newest at the bottom); the label lays the rows out and puts how far
//! back they go in [`BACK_MAX`] before the screen opens.

use std::fmt::Write as _;

use conv_core::ir::{LineLog, Script};

use crate::menu::IN_GAME;
use crate::screens::{image, lua};

/// Rows back from the newest.
const ROWS_BACK: &str = "%170";
const ROWS: &str = "%171";
const BACK_MAX: &str = "%174";

/// The line log is the engine's.
pub(crate) fn engine_log(script: &Script) -> Option<&LineLog> {
    script.line_log.as_ref()
}

/// `*conv_backlog`.
pub(crate) fn labels(out: &mut String, l: &LineLog) {
    let width = l.row_width.map_or(String::new(), |w| format!(",{w}"));
    writeln!(
        out,
        "*{}\nif {IN_GAME} == 1 textoff\nlogrows {ROWS},{}{width}\nmov {BACK_MAX},{ROWS}-{}:if {BACK_MAX} < 0 mov {BACK_MAX},0\nmov {ROWS_BACK},0\nscreen \"backlog\"\nif {IN_GAME} == 1 texton\nreturn",
        crate::menu::BACKLOG,
        l.columns,
        l.rows
    )
    .unwrap();
}

/// `backlog.lua`.
pub(crate) fn files(script: &Script) -> Vec<(String, String)> {
    let Some(l) = engine_log(script) else { return Vec::new() };
    let mut w = Vec::new();
    w.push(image("image", &l.background, ", opaque=true"));
    let v = &l.voice_icon;
    let outline = l.outline.as_ref().map_or(String::new(), |o| format!(", outline={}", lua(o)));
    // A voice switched off has no button (its row's logvoice tag).
    let off = if crate::voice_switches(script) { format!(", voice_off={}", crate::menu::VOICE_OFF_BASE) } else { String::new() };
    // A row is cut at the columns, but strsp wraps a full-width character
    // one column early: a full row would lose its last.
    w.push(format!(
        "{{\"list\", source=\"rows\", var={}, x={}, y={}, dy={}, rows={}, size={}, cols={}, color={}{outline}{off},\n      voice={{file={}, cells=3, pressed=2, x={}, y={}}}}}",
        lua(ROWS_BACK),
        l.text_x,
        l.text_y,
        l.row_step,
        l.rows,
        l.font_size,
        l.columns + 1,
        lua(&l.colour),
        lua(&v.image),
        v.x,
        v.y
    ));
    w.push(image("button", &l.up, ", cells=4, pressed=3, action={\"scroll\", \"+1\"}"));
    w.push(image("button", &l.down, ", cells=4, pressed=3, action={\"scroll\", \"-1\"}"));
    w.push(image("button", &l.close, ", cells=4, pressed=3, action={\"close\"}"));
    // The knob from knob_y.1 (the newest rows) up to knob_y.0 (the oldest).
    let (k0, k1) = l.knob_y;
    w.push(format!(
        "{{\"slider\", var={}, min=0, max=back_max, step=1, vertical=true, travel={}, knob={{file={}, cells=2, x={}, y={k1}}}}}",
        lua(ROWS_BACK),
        k0 - k1,
        lua(&l.knob.image),
        l.knob.x
    ));
    let mut out = format!(
        "-- The backlog (conscripter-convert, from LineLog).\nlocal back_max = var({})\nreturn {{\n  kind = \"backlog\",\n  wheel_up = {{\"scroll\", \"+1\"}},\n  wheel_down = {{\"scroll\", \"-1\", at_end = {{\"close\"}}}},\n",
        lua(BACK_MAX)
    );
    if l.fade_ms > 0 {
        writeln!(out, "  fade = {},", l.fade_ms).unwrap();
    }
    out.push_str("  widgets = {\n");
    for x in w {
        writeln!(out, "    {x},").unwrap();
    }
    out.push_str("  },\n}\n");
    vec![("backlog.lua".to_string(), out)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::Placed;

    fn placed(name: &str, x: i32, y: i32) -> Placed {
        Placed { image: format!("system/{name}.png"), x, y, w: 30, h: 60 }
    }

    fn log() -> LineLog {
        LineLog {
            background: placed("bg", 0, 0),
            up: placed("up", 898, 168),
            down: placed("down", 898, 303),
            close: placed("close", 927, 438),
            knob: placed("knob", 860, 461),
            knob_y: (50, 461),
            voice_icon: placed("voice", 102, 3),
            text_x: 140,
            text_y: 49,
            row_step: 30,
            rows: 16,
            columns: 26,
            font_size: 20,
            colour: "#000000".into(),
            fade_ms: 300,
            outline: Some("#AAAA88".into()),
            row_width: None,
        }
    }

    #[test]
    fn line_log_becomes_a_screen() {
        let l = log();
        let script = Script { line_log: Some(l.clone()), ..Default::default() };
        let files = files(&script);
        let lua = &files[0].1;
        for part in [
            "local back_max = var(\"%174\")",
            "wheel_down = {\"scroll\", \"-1\", at_end = {\"close\"}},",
            "{\"list\", source=\"rows\", var=\"%170\", x=140, y=49, dy=30, rows=16, size=20, cols=27, color=\"#000000\", outline=\"#AAAA88\",\n      voice={file=\"system/voice.png\", cells=3, pressed=2, x=102, y=3}}",
            "{\"slider\", var=\"%170\", min=0, max=back_max, step=1, vertical=true, travel=-411, knob={file=\"system/knob.png\", cells=2, x=860, y=461}}",
        ] {
            assert!(lua.contains(part), "missing {part:?} in\n{lua}");
        }
        let mut out = String::new();
        labels(&mut out, &l);
        assert!(out.contains("logrows %171,26\nmov %174,%171-16:if %174 < 0 mov %174,0\nmov %170,0\nscreen \"backlog\"\n"), "{out}");
    }

    /// With voice switches, a voice is logged with its character (played
    /// or not) and the rows list hides it while its switch is off.
    #[test]
    fn switched_off_voice_is_tagged() {
        use conv_core::ir::{Op, Setting, SettingRow, SettingsMenu, Switch, Voice, VoiceToggle};
        let settings = SettingsMenu {
            base: placed("base", 800, 600),
            base_opacity: 210,
            header: placed("header", 100, 30),
            close: placed("close", 60, 20),
            rows: vec![SettingRow { label: "ＢＧＭ".into(), setting: Setting::BgmVolume }],
            label_x: 140,
            minus_x: 340,
            level_x: 380,
            plus_x: 680,
            y: 140,
            pitch: 70,
            font_size: 22,
            title_label: "title".into(),
            quit_label: "quit".into(),
            exits_y: 540,
            tab_labels: vec!["音量".into(), "音声".into(), "確認".into()],
            tabs_x: 300,
            tabs_y: 34,
            voices: vec![VoiceToggle { label: "あずさ".into(), characters: vec![3] }],
            voice_column_x: 440,
            voice_pitch: 50,
            skip_unread_label: None,
            pictures: None,
            defaults: Vec::new(),
            switches_off: Vec::new(),
            text_ms: None,
            switches: vec![conv_core::ir::SettingSwitch { label: "セーブ確認".into(), switch: Switch::ConfirmSave }],
        };
        let ops = vec![
            Op::BacklogLine { text: "《見えた》".into(), voice: Some("voice/b.ogg".into()), character: Some(3) },
            Op::Text { speaker: None, text: "「一」".into(), voice: Some(Voice { file: "voice/a.ogg".into(), character: 3 }), links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        ];
        let script = Script { line_log: Some(log()), settings: Some(settings), ops, ..Default::default() };
        let text = crate::emit(&script);
        assert!(text.contains("logvoice \"voice\\a.ogg\",3\nif %973 == 0 dwave 0,\"voice\\a.ogg\"\n"), "{text}");
        assert!(text.contains("logadd \"《見えた》\",\"voice\\b.ogg\",3\n"), "{text}");
        assert!(files(&script)[0].1.contains("outline=\"#AAAA88\", voice_off=970,"));
    }
}
