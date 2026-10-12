//! ONScripter output stage: turns an IR [`Script`] into `0.txt`.
//!
//! Targets ogapee's ONScripter with Japanese text. Things marked
//! "unverified" below follow the NScripter documentation but haven't been
//! checked in a running engine yet (see PLAN.md, Phase 0).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

use vn2ons_core::ir::{ArithOp, Op, Operand, Script, Var};

pub mod nsa;

/// First effect number we define; 1 is NScripter's built-in "instant".
const FIRST_EFFECT: u32 = 2;
/// NScripter effect type 10 is a cross-fade.
const FADE: u32 = 10;
/// Sound effect channels we rotate through (0 is left for voices).
const SE_CHANNELS: std::ops::RangeInclusive<u32> = 1..=9;
/// Sprite slots for the name text and its plate. ONScripter draws lower
/// numbers on top, so the text (0) is over the plate (1), and both are over
/// the character sprites (10 and up).
const NAME_TEXT_SPRITE: u32 = 0;
const NAME_PLATE_SPRITE: u32 = 1;
/// NScripter variables: locals from %10, globals from %200. `globalon`
/// makes %200 and up persist across games.
const LOCAL_VAR_BASE: u32 = 10;
const GLOBAL_VAR_BASE: u32 = 200;
/// Colour for words the original game made clickable, and normal text.
const LINK_COLOR: &str = "#F0C040";
const TEXT_COLOR: &str = "#FFFFFF";

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
    // Read arc.nsa (without this ONScripter only looks for .sar archives;
    // loose files still work either way).
    writeln!(out, "nsa").unwrap();
    writeln!(out, "globalon").unwrap();
    // The caption is passed to the window system as raw Shift-JIS, which
    // shows as mojibake on Linux, so keep it ASCII.
    let caption: String = script.title.chars().filter(|c| c.is_ascii() && *c != '"').collect();
    writeln!(out, "caption \"{}\"", caption.trim()).unwrap();
    for (ms, n) in &effects {
        writeln!(out, "effect {n},{FADE},{ms}").unwrap();
    }
    writeln!(out, "game").unwrap();
    writeln!(out, "*start").unwrap();
    // setwindow redraws the screen, so it must come after `game` (ONScripter
    // crashes if it runs in the define section).
    if let Some(w) = &script.window {
        // setwindow text x,y, columns,rows, font w,h, char spacing, line gap,
        // ms per character, bold, shadow, window image, window x,y
        writeln!(
            out,
            "setwindow {},{},{},{},{},{},1,{},20,0,1,\":a;{}\",{},{}",
            w.text_x, w.text_y, w.columns, w.rows, w.font_size, w.font_size, w.line_gap, path(&w.image), w.x, w.y
        )
        .unwrap();
    }

    let effect = |ms: u32| if ms == 0 { 1 } else { effects[&ms] };
    let mut se = SeChannels::default();
    let mut needs_bit_helpers = false;
    let mut plate_visible = false;
    for op in &script.ops {
        match op {
            Op::Label(name) => writeln!(out, "*{}", label(name)).unwrap(),
            Op::Jump(name) => writeln!(out, "goto *{}", label(name)).unwrap(),
            Op::Gosub(name) => writeln!(out, "gosub *{}", label(name)).unwrap(),
            Op::Return => writeln!(out, "return").unwrap(),
            Op::Arith { var, op, value } => {
                let value = match value {
                    Operand::Const(n) => n.to_string(),
                    Operand::Var(v) => var_name(*v),
                };
                let target = var_name(*var);
                let cmd = match op {
                    ArithOp::Set => "mov",
                    ArithOp::Add => "add",
                    ArithOp::Sub => "sub",
                    ArithOp::Mul => "mul",
                    ArithOp::Div => "div",
                    ArithOp::BitAnd | ArithOp::BitOr => {
                        // NScripter has no bitwise operators; call a helper.
                        let routine = if *op == ArithOp::BitAnd { BITAND } else { BITOR };
                        needs_bit_helpers = true;
                        writeln!(out, "mov %1,{target}:mov %2,{value}:gosub *{routine}:mov {target},%3").unwrap();
                        continue;
                    }
                };
                writeln!(out, "{cmd} {target},{value}").unwrap();
            }
            Op::IfEqual { var, value, label: target } => {
                writeln!(out, "if {} == {value} goto *{}", var_name(*var), label(target)).unwrap()
            }
            Op::Background { image, fade_ms } => {
                // NScripter's bg keeps lsp sprites; the game removes characters
                // when the background or CG changes.
                writeln!(out, "csp -1").unwrap();
                writeln!(out, "bg \"{}\",{}", path(image), effect(*fade_ms)).unwrap();
                plate_visible = false;
            }
            Op::Sprites { sprites, fade_ms } => {
                writeln!(out, "csp -1").unwrap();
                for (n, s) in sprites.iter().enumerate() {
                    // ":a;" = use the PNG's alpha channel (unverified for ogapee's build).
                    writeln!(out, "lsp {},\":a;{}\",{},{}", 10 + n, path(&s.image), s.x, s.y).unwrap();
                }
                writeln!(out, "print {}", effect(*fade_ms)).unwrap();
                plate_visible = false; // csp -1 removed it too
            }
            Op::Text { speaker, text, voice, links } => {
                if let Some(v) = voice {
                    writeln!(out, "dwave 0,\"{}\"", path(v)).unwrap();
                }
                let plate = script.window.as_ref().and_then(|w| w.name_plate.as_ref());
                let mut body = with_links(text, links);
                // Sprite changes only show after a print, so print when the
                // plate appears, changes or goes away.
                match (speaker, plate) {
                    (Some(name), Some(p)) => {
                        writeln!(out, "lsp {NAME_PLATE_SPRITE},\":a;{}\",{},{}", path(&p.image), p.x, p.y).unwrap();
                        writeln!(
                            out,
                            "strsp {NAME_TEXT_SPRITE},\"{}\",{},{},16,1,{},{},1,0,0,1",
                            text_safe(name), p.text_x, p.text_y, p.font_size, p.font_size
                        )
                        .unwrap();
                        writeln!(out, "print 1").unwrap();
                        plate_visible = true;
                    }
                    (Some(name), None) => body = format!("【{}】{body}", text_safe(name)),
                    (None, Some(_)) if plate_visible => {
                        writeln!(out, "csp {NAME_PLATE_SPRITE}").unwrap();
                        writeln!(out, "csp {NAME_TEXT_SPRITE}").unwrap();
                        writeln!(out, "print 1").unwrap();
                        plate_visible = false;
                    }
                    (None, _) => {}
                }
                // Each line of the page is its own source line; the last one
                // ends with the page wait.
                let lines: Vec<String> = body
                    .split('\n')
                    .map(|l| if l.starts_with(|c: char| c.is_ascii()) { format!("　{l}") } else { l.to_string() })
                    .collect();
                writeln!(out, "{}\\", lines.join("\n")).unwrap();
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
    if needs_bit_helpers {
        out.push_str(&bit_helpers());
    }
    out
}

const BITAND: &str = "vn2ons_bitand";
const BITOR: &str = "vn2ons_bitor";

/// Subroutines computing `%3 = %1 & %2` and `%3 = %1 | %2` one bit at a
/// time with div/mod (NScripter has no bitwise operators). They use the
/// scratch variables %1-%8, below the converted game's variables. Works for
/// non-negative values, which is all the game uses (flags).
fn bit_helpers() -> String {
    let mut out = String::new();
    for (name, keep) in [(BITAND, "if %7 == 2 add %3,%4"), (BITOR, "if %7 > 0 add %3,%4")] {
        writeln!(out, "*{name}").unwrap();
        writeln!(out, "mov %3,0:mov %4,1:mov %5,%1:mov %6,%2").unwrap();
        writeln!(out, "*{name}_loop").unwrap();
        writeln!(out, "if %5 == 0 && %6 == 0 return").unwrap();
        writeln!(out, "mov %7,%5:mod %7,2:mov %8,%6:mod %8,2:add %7,%8").unwrap();
        writeln!(out, "{keep}").unwrap();
        writeln!(out, "div %5,2:div %6,2:mul %4,2").unwrap();
        writeln!(out, "goto *{name}_loop").unwrap();
    }
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

/// Escapes the text and wraps each link range in colour tags. Ranges are in
/// characters of `text`; overlapping or out-of-range ones are skipped.
fn with_links(text: &str, links: &[(usize, usize)]) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut ranges: Vec<(usize, usize)> =
        links.iter().copied().filter(|&(s, n)| n > 0 && s + n <= chars.len()).collect();
    ranges.sort_unstable();
    let mut out = String::new();
    let mut pos = 0;
    for (start, len) in ranges {
        if start < pos {
            continue;
        }
        out.push_str(&escape(&chars[pos..start].iter().collect::<String>()));
        out.push_str(LINK_COLOR);
        out.push_str(&escape(&chars[start..start + len].iter().collect::<String>()));
        out.push_str(TEXT_COLOR);
        pos = start + len;
    }
    out.push_str(&escape(&chars[pos..].iter().collect::<String>()));
    out
}

fn var_name(v: Var) -> String {
    let base = if v.global { GLOBAL_VAR_BASE } else { LOCAL_VAR_BASE };
    format!("%{}", base + u32::from(v.index))
}

/// NScripter paths use backslashes.
fn path(p: &str) -> String {
    p.replace('/', "\\")
}

/// Labels must be ASCII identifiers.
fn label(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

/// Makes text safe to place on a text line: [`escape`], plus making sure the
/// line doesn't start with ASCII (which would be parsed as a command).
fn text_safe(text: &str) -> String {
    let mut out = escape(text);
    if out.starts_with(|c: char| c.is_ascii()) {
        out.insert(0, '　');
    }
    out
}

/// Replaces ASCII characters that NScripter treats specially in text with
/// their full-width forms.
fn escape(text: &str) -> String {
    text
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
            '\r' => '　',
            c => c,
        })
        .collect()
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
        Script { title: "Test".into(), width: 800, height: 600, window: None, ops }
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
            Op::Text { speaker: Some("名前".into()), text: "「こんにちは」".into(), voice: None, links: vec![] },
            Op::Text { speaker: None, text: "地の文。".into(), voice: Some("voice/v1.ogg".into()), links: vec![(1, 2)] },
            Op::SePlay { file: "se/k001.ogg".into(), looped: true },
            Op::SeStop { file: Some("se/k001.ogg".into()) },
            Op::BgmStop { fade_out_ms: 2000 },
            Op::Unsupported { what: "op 0x76".into(), at: SourceLoc { file: "0003.cd".into(), index: 7 } },
        ]);
        let text = emit(&s);
        let expected = "\
;mode800
*define
nsa
globalon
caption \"Test\"
effect 2,10,500
game
*start
*scene_1
bgmfadein 1000
bgm \"bgm\\track01.ogg\"
csp -1
bg \"bg\\room.png\",2
csp -1
lsp 10,\":a;sprite\\a.png\",10,-20
print 2
【名前】「こんにちは」\\
dwave 0,\"voice\\v1.ogg\"
地#F0C040の文#FFFFFF。\\
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
    fn adv_window_and_name_plate() {
        use vn2ons_core::ir::{NamePlate, TextWindow};
        let mut s = script(vec![
            Op::Text { speaker: Some("名前".into()), text: "「台詞」".into(), voice: None, links: vec![] },
            Op::Text { speaker: None, text: "Ghost".into(), voice: None, links: vec![] },
        ]);
        s.window = Some(TextWindow {
            image: "system/window.png".into(),
            x: 47,
            y: 452,
            text_x: 105,
            text_y: 470,
            columns: 25,
            rows: 3,
            font_size: 24,
            line_gap: 13,
            name_plate: Some(NamePlate {
                image: "system/name.png".into(),
                x: 55,
                y: 412,
                text_x: 75,
                text_y: 422,
                font_size: 20,
            }),
        });
        let text = emit(&s);
        assert!(text.contains("setwindow 105,470,25,3,24,24,1,13,20,0,1,\":a;system\\window.png\",47,452\n"));
        assert!(text.contains("lsp 1,\":a;system\\name.png\",55,412\nstrsp 0,\"名前\",75,422,16,1,20,20,1,0,0,1\nprint 1\n「台詞」\\\n"));
        assert!(text.contains("csp 1\ncsp 0\nprint 1\n　Ghost\\\n"));
    }

    #[test]
    fn multi_line_text() {
        let s = script(vec![Op::Text {
            speaker: None,
            text: "一行目\nabc\n三行目".into(),
            voice: None,
            links: vec![(4, 3)],
        }]);
        assert!(emit(&s).contains("一行目\n　#F0C040abc#FFFFFF\n三行目\\\n"));
    }

    #[test]
    fn links_are_coloured() {
        assert_eq!(with_links("あいうえお", &[(1, 2), (3, 9)]), "あ#F0C040いう#FFFFFFえお");
        assert_eq!(with_links("a#b", &[]), "a＃b");
    }

    #[test]
    fn flow_and_variables() {
        let g = Var { global: true, index: 7 };
        let l = Var { global: false, index: 2 };
        let s = script(vec![
            Op::Arith { var: g, op: ArithOp::Set, value: Operand::Const(1) },
            Op::Arith { var: l, op: ArithOp::Add, value: Operand::Var(g) },
            Op::IfEqual { var: g, value: 1, label: "s3_l2".into() },
            Op::Gosub("s2_l18".into()),
            Op::Return,
        ]);
        assert!(emit(&s).contains("mov %207,1\nadd %12,%207\nif %207 == 1 goto *s3_l2\ngosub *s2_l18\nreturn\n"));
    }

    #[test]
    fn bitwise_ops_use_helpers() {
        let g = Var { global: true, index: 9 };
        let h = Var { global: true, index: 12 };
        let text = emit(&script(vec![Op::Arith { var: g, op: ArithOp::BitAnd, value: Operand::Var(h) }]));
        assert!(text.contains("mov %1,%209:mov %2,%212:gosub *vn2ons_bitand:mov %209,%3\n"));
        assert!(text.contains("*vn2ons_bitand\n") && text.contains("*vn2ons_bitor\n"));
        // Helpers are only emitted when needed.
        assert!(!emit(&script(vec![])).contains("vn2ons_bitand"));
    }

    /// Runs the helper's algorithm (as emitted) in Rust to check the maths.
    #[test]
    fn bit_helper_algorithm() {
        fn run(a: i32, b: i32, and: bool) -> i32 {
            let (mut r, mut bit, mut x, mut y) = (0, 1, a, b);
            while !(x == 0 && y == 0) {
                let t = x % 2 + y % 2;
                if (and && t == 2) || (!and && t > 0) {
                    r += bit;
                }
                x /= 2;
                y /= 2;
                bit *= 2;
            }
            r
        }
        for (a, b) in [(0, 0), (1, 1), (1, 0), (6, 3), (12, 10), (255, 15), (1 << 20, (1 << 20) + 5)] {
            assert_eq!(run(a, b, true), a & b, "{a} & {b}");
            assert_eq!(run(a, b, false), a | b, "{a} | {b}");
        }
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
