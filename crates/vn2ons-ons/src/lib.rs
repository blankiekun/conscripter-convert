//! ONScripter output stage: turns an IR [`Script`] into `0.txt`.
//!
//! Targets ogapee's ONScripter with Japanese text. Things marked
//! "unverified" below follow the NScripter documentation but haven't been
//! checked in a running engine yet (see PLAN.md, Phase 0).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use vn2ons_core::ir::{Op, Script};

/// First effect number we define; 1 is NScripter's built-in "instant".
const FIRST_EFFECT: u32 = 2;
/// NScripter effect type 10 is a cross-fade.
const FADE: u32 = 10;
/// Sound effect channels we rotate through (0 is left for voices).
const SE_CHANNELS: std::ops::RangeInclusive<u32> = 1..=9;

/// Renders the script as NScripter source text (UTF-8; use [`encode`] to
/// produce the Shift-JIS bytes ONScripter reads).
pub fn emit(script: &Script) -> String {
    let effects = fade_effects(script);
    let mut out = String::new();

    let mode = match (script.width, script.height) {
        (800, 600) => ";mode800",
        (640, 480) => ";mode640",
        _ => ";mode800", // unverified: other sizes need a `;$S` header
    };
    writeln!(out, "{mode}").unwrap();
    writeln!(out, "*define").unwrap();
    writeln!(out, "caption \"{}\"", script.title.replace('"', "'")).unwrap();
    for (ms, n) in &effects {
        writeln!(out, "effect {n},{FADE},{ms}").unwrap();
    }
    writeln!(out, "game").unwrap();
    writeln!(out, "*start").unwrap();

    let effect = |ms: u32| if ms == 0 { 1 } else { effects[&ms] };
    let mut se = SeChannels::default();
    for op in &script.ops {
        match op {
            Op::Label(name) => writeln!(out, "*{}", label(name)).unwrap(),
            Op::Background { image, fade_ms } => {
                writeln!(out, "bg \"{}\",{}", path(image), effect(*fade_ms)).unwrap()
            }
            Op::Sprites { sprites, fade_ms } => {
                writeln!(out, "csp -1").unwrap();
                for (n, s) in sprites.iter().enumerate() {
                    // ":a;" = use the PNG's alpha channel (unverified for ogapee's build).
                    writeln!(out, "lsp {},\":a;{}\",{},{}", 10 + n, path(&s.image), s.x, s.y).unwrap();
                }
                writeln!(out, "print {}", effect(*fade_ms)).unwrap();
            }
            Op::Text { speaker, text, voice } => {
                if let Some(v) = voice {
                    writeln!(out, "dwave 0,\"{}\"", path(v)).unwrap();
                }
                let body = match speaker {
                    Some(name) => format!("【{}】{}", text_safe(name), text_safe(text)),
                    None => text_safe(text),
                };
                writeln!(out, "{body}\\").unwrap();
            }
            Op::BgmPlay { file, fade_in_ms } => {
                writeln!(out, "bgmfadein {fade_in_ms}").unwrap();
                writeln!(out, "bgm \"{}\"", path(file)).unwrap();
            }
            Op::BgmStop { fade_out_ms } => {
                writeln!(out, "bgmfadeout {fade_out_ms}").unwrap();
                writeln!(out, "bgmstop").unwrap();
            }
            Op::SePlay { file, looped } => {
                let ch = se.start(file);
                let cmd = if *looped { "dwaveloop" } else { "dwave" };
                writeln!(out, "{cmd} {ch},\"{}\"", path(file)).unwrap();
            }
            Op::SeStop { file: Some(file) } => {
                if let Some(ch) = se.stop(file) {
                    writeln!(out, "dwavestop {ch}").unwrap();
                }
            }
            Op::SeStop { file: None } => {
                for ch in se.stop_all() {
                    writeln!(out, "dwavestop {ch}").unwrap();
                }
            }
            Op::Unsupported { what, at } => {
                writeln!(out, "; unsupported: {what} ({} #{})", at.file, at.index).unwrap()
            }
        }
    }
    writeln!(out, "end").unwrap();
    out
}

/// Encodes NScripter source as Shift-JIS. Characters Shift-JIS can't
/// represent become `?`; their count is returned.
pub fn encode(source: &str) -> (Vec<u8>, usize) {
    let mut lost = 0;
    let mut out = Vec::with_capacity(source.len());
    for ch in source.chars() {
        let mut buf = [0u8; 4];
        let (bytes, _, bad) = encoding_rs::SHIFT_JIS.encode(ch.encode_utf8(&mut buf));
        if bad {
            lost += 1;
            out.push(b'?');
        } else {
            out.extend_from_slice(&bytes);
        }
    }
    (out, lost)
}

/// One effect number per distinct non-zero fade duration, in a stable order.
fn fade_effects(script: &Script) -> BTreeMap<u32, u32> {
    let mut durations: Vec<u32> = script
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Background { fade_ms, .. } | Op::Sprites { fade_ms, .. } => Some(*fade_ms),
            _ => None,
        })
        .filter(|&ms| ms > 0)
        .collect();
    durations.sort_unstable();
    durations.dedup();
    durations.into_iter().zip(FIRST_EFFECT..).collect()
}

/// NScripter paths use backslashes.
fn path(p: &str) -> String {
    p.replace('/', "\\")
}

/// Labels must be ASCII identifiers.
fn label(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

/// Makes message text safe to place on a text line: replaces ASCII
/// characters that NScripter treats as commands with full-width forms, and
/// makes sure the line doesn't start with ASCII (which would be parsed as a
/// command).
fn text_safe(text: &str) -> String {
    let mut out: String = text
        .chars()
        .map(|c| match c {
            '@' => '＠',
            '\\' => '＼',
            '/' => '／',
            '#' => '＃',
            '%' => '％',
            '$' => '＄',
            '`' => '｀',
            '!' => '！',
            '_' => '＿',
            '"' => '”',
            ';' => '；',
            '*' => '＊',
            '\n' | '\r' => '　',
            c => c,
        })
        .collect();
    if out.starts_with(|c: char| c.is_ascii()) {
        out.insert(0, '　');
    }
    out
}

#[derive(Default)]
struct SeChannels {
    playing: HashMap<String, u32>,
    next: u32,
}

impl SeChannels {
    fn start(&mut self, file: &str) -> u32 {
        let span = SE_CHANNELS.end() - SE_CHANNELS.start() + 1;
        let ch = SE_CHANNELS.start() + self.next % span;
        self.next += 1;
        self.playing.retain(|_, c| *c != ch);
        self.playing.insert(file.to_string(), ch);
        ch
    }

    fn stop(&mut self, file: &str) -> Option<u32> {
        self.playing.remove(file)
    }

    fn stop_all(&mut self) -> Vec<u32> {
        let mut chans: Vec<u32> = self.playing.drain().map(|(_, c)| c).collect();
        chans.sort_unstable();
        chans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vn2ons_core::ir::{SourceLoc, SpritePlacement};

    fn script(ops: Vec<Op>) -> Script {
        Script { title: "Test".into(), width: 800, height: 600, ops }
    }

    #[test]
    fn emits_a_scene() {
        let s = script(vec![
            Op::Label("scene_1".into()),
            Op::BgmPlay { file: "bgm/track01.ogg".into(), fade_in_ms: 1000 },
            Op::Background { image: "bg/room.png".into(), fade_ms: 500 },
            Op::Sprites {
                sprites: vec![SpritePlacement { image: "sprite/a.png".into(), x: 10, y: -20 }],
                fade_ms: 500,
            },
            Op::Text { speaker: Some("名前".into()), text: "「こんにちは」".into(), voice: None },
            Op::Text { speaker: None, text: "地の文。".into(), voice: Some("voice/v1.ogg".into()) },
            Op::SePlay { file: "se/k001.ogg".into(), looped: true },
            Op::SeStop { file: Some("se/k001.ogg".into()) },
            Op::BgmStop { fade_out_ms: 2000 },
            Op::Unsupported { what: "op 0x76".into(), at: SourceLoc { file: "0003.cd".into(), index: 7 } },
        ]);
        let text = emit(&s);
        let expected = "\
;mode800
*define
caption \"Test\"
effect 2,10,500
game
*start
*scene_1
bgmfadein 1000
bgm \"bgm\\track01.ogg\"
bg \"bg\\room.png\",2
csp -1
lsp 10,\":a;sprite\\a.png\",10,-20
print 2
【名前】「こんにちは」\\
dwave 0,\"voice\\v1.ogg\"
地の文。\\
dwaveloop 1,\"se\\k001.ogg\"
dwavestop 1
bgmfadeout 2000
bgmstop
; unsupported: op 0x76 (0003.cd #7)
end
";
        assert_eq!(text, expected);
    }

    #[test]
    fn text_is_made_safe() {
        assert_eq!(text_safe("Ghost: 50% @home"), "　Ghost: 50％ ＠home");
        assert_eq!(text_safe("「a/b」"), "「a／b」");
    }

    #[test]
    fn encodes_shift_jis() {
        let (bytes, lost) = encode("あ\u{1F600}");
        assert_eq!(bytes, [0x82, 0xa0, b'?']);
        assert_eq!(lost, 1);
    }

    #[test]
    fn se_channels_rotate_and_stop() {
        let mut ch = SeChannels::default();
        assert_eq!(ch.start("a"), 1);
        assert_eq!(ch.start("b"), 2);
        assert_eq!(ch.stop("a"), Some(1));
        assert_eq!(ch.stop("a"), None);
        assert_eq!(ch.stop_all(), vec![2]);
    }
}
