//! 月姫's scripts lowered to the IR: the flow (labels, jumps, flags), the
//! text, the picture layers and the sound. What isn't lowered yet is
//! counted in the notes (`docs/games/tsukihime.md`, "Status").
//!
//! Pictures: `_STCH` stages a picture on one of 40 layers and `_FADS`
//! shows the staged layers. A picture is drawn at its record's size on a
//! 1280 × 724 stage (the texture at half size), the record's centre at
//! the script's x, y × 8/3 ("The stage"). The screen is 1280 × 720.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use conv_core::ir::{ArithOp, Compare, MotionEnd, MotionKey, Op, Operand, Ruby, ScreenEffect, SpriteChange, TextStyle, SpriteFilter, SpritePlacement, TransitionStyle, Var, Voice, BLEND_ADD, BLEND_MULTIPLY, BLEND_NORMAL, BLEND_SUBTRACT, FILTER_BLEND_COPY, FILTER_GREYSCALE, FILTER_NEGATIVE};

use super::exe::{Key, Motion, Picture};
use super::quake::{self, Kind, Quake};
use super::script::Command;
use crate::metrics::Metrics;

// ours: the output size (the Switch draws at 1920 × 1080 docked, 1280 × 720 handheld).
pub const SCREEN_W: u32 = 1280;
pub const SCREEN_H: u32 = 720;
/// Locals (saved with each slot) below, globals from here.
// ours
pub const GLOBAL_BORDER: u32 = 800;
/// The stage's height in record units (its width is the screen's).
const STAGE_H: f32 = 724.0; // src: exe 0x56F830 (a full-screen background's record is 1280 × 724)
const LAYERS: usize = 40; // src: exe 0x162140 (`_STCC` with a negative layer clears 40)
/// The `F` and `C` flags are locals from %196 (NScripter number 10 +
/// index), one each in name order; the `G` flags globals %800 + n.
// ours
const FIRST_LOCAL: u16 = 186;
/// `_RDST(n)` sets the exe's scene-read flag n (`FUN_00192630`, n below
/// 0x240), kept as global %1200 + n.
// ours: index 400 from the global border, in alloc.rs's free %1111-%1999
const READ_FLAGS: u16 = 400;

/// `_STCG(name)` sets the CG flag of a picture it doesn't show
/// (`FUN_001774f0`); here a global per name, in name order from
/// [`CG_FLAGS`], which the gallery reads ([`cg_flags`]).
// ours: after gallery::FLAG_557 (1003), in alloc.rs's free %1111-%1999
const CG_FLAGS: u16 = 1004;
/// The movie-seen bits (`FUN_001923c0`, set as the movie ends): a click
/// skips the openings, the prologue and the endings once they were seen.
// ours: one global a bit, after the CG flags (fewer than 100)
const MOVIE_SEEN: u16 = 1104;

/// `_TOMP(n)`'s seen bit (`FUN_00140480`), for the movies a click skips
/// only once seen; every other movie a click skips.
fn movie_seen(n: usize) -> Option<Var> {
    let bit = match n {
        4 => 2,
        5 => 0,
        6 => 1,
        0x23..=0x25 => n as u16 - 0x20,
        _ => return None,
    };
    Some(Var { global: true, index: MOVIE_SEEN + bit })
}

/// The globals `_STCG` sets, by lowercase picture name.
pub fn cg_flags(scripts: &[(String, Vec<Command>)]) -> BTreeMap<String, Var> {
    let names: BTreeSet<String> = scripts.iter().flat_map(|(_, c)| c).filter(|c| c.opcode() == "_STCG").filter_map(|c| c.args.first()).map(|a| a.trim().to_lowercase()).collect();
    assert!(names.len() < 100, "more _STCG pictures than their globals");
    names.into_iter().enumerate().map(|(i, n)| (n, Var { global: true, index: CG_FLAGS + i as u16 })).collect()
}

/// The global that holds scene-read flag `n` (`_RDST(n)`).
pub fn read_flag(n: u16) -> Var {
    Var { global: true, index: READ_FLAGS + n }
}

/// The global that holds lesson `n`'s read flag (`_RDED(n)`, n 533-557):
/// 557, the last, is [`super::gallery::FLAG_557`].
// ours: 976-999, between the read flags' end (975) and language.rs's 1000
pub fn lesson_flag(n: i32) -> Option<Var> {
    match n {
        533..=556 => Some(Var { global: true, index: (n + 443) as u16 }),
        557 => Some(super::gallery::FLAG_557),
        _ => None,
    }
}

/// What the lowering reads from the game.
pub struct Game<'a> {
    /// Every script by name with its commands, in `allscr` order (the first
    /// is the game's start).
    pub scripts: Vec<(String, Vec<Command<'a>>)>,
    /// Each script's scene number (`_JUMP(n)`), in the same order.
    pub scenes: Vec<i16>,
    /// The text lines (`$n`).
    pub lines: Vec<&'a str>,
    /// The exe's picture records by lowercase name.
    pub pictures: HashMap<String, Picture>,
    /// The pictures' extension in `pic/` (`png`, or `webp` with `--webp`;
    /// the `rule_*` masks are always PNG).
    pub pic_ext: &'static str,
    /// The width and height of each picture's file (`pic/<name>.<ext>`),
    /// by lowercase name: the record's size is drawn from it.
    pub textures: HashMap<String, (u32, u32)>,
    /// The sound archive's sound names: `_MPLY(n)` plays sound `n − 1`.
    pub sounds: Vec<String>,
    /// The wave sounds' files (`WSD_moon_se02093.wav`, `.ogg` when the
    /// build re-encodes them) by the scripts' name for them (uppercase,
    /// without `WSD_`).
    pub waves: HashMap<String, String>,
    /// The exe's motions: `_STMA(layer, m)` runs `motions[m - 1]`.
    pub motions: Vec<super::exe::Motion>,
    /// The exe's movie table ([`super::exe::Exe::movies`]), the English
    /// copy's as `movie/en/<name>` where the fan patch has its own.
    pub movies: Vec<String>,
    /// Each `_STBR` byte's shader ([`super::exe::Exe::shaders`]); none:
    /// the normal one.
    pub shaders: Vec<i32>,
}

pub struct Compiled {
    pub ops: Vec<Op>,
    pub notes: BTreeMap<String, usize>,
    pub metrics: Metrics,
    /// Where each script's ops start in `ops` (its label).
    pub starts: Vec<usize>,
    /// (op index, command) marks: each command's ops start there
    /// ([`conv_core::ir::Script::source_map`]); the command is its index
    /// in the script's [`script::parse`] list, the malformed one left out.
    pub source_map: Vec<(usize, conv_core::ir::SourceLoc)>,
}

/// How a layer is drawn: the image, its middle on the screen, its zoom (%).
type Look = (String, (f32, f32), (f32, f32));

impl Layer {
    /// The `_STZ4` zoom the draw uses.
    fn drawn_stretch(&self) -> (f32, f32) {
        if self.stretch_off { (1.0, 1.0) } else { self.stretch }
    }
}

/// A picture on a layer, as `_STCH` left it.
#[derive(Debug, Clone, PartialEq)]
struct Layer {
    /// Lowercase, without the expression.
    base: String,
    expression: Option<(u32, u32)>,
    x: f32,
    y: f32,
    zoom: f32,
    /// `_STZ4`'s zoom across and down (1 = none; negative flips), times `zoom`.
    stretch: (f32, f32),
    /// `_STZ4` argument 7 set: the draw leaves `stretch` out (`FUN_00132e30`
    /// reads the layer's byte 0xA2), a child's place still uses it.
    stretch_off: bool,
    alpha: u32,
    /// `_STGS`'s fade, from the next `_FADS` on: (from, to, ms, its
    /// curve); `alpha` is already `to`.
    fade: Option<(u32, u32, u32, Curve)>,
    /// `_STMA` on a layer not shown yet: the motion, from the `_FADS` that shows it.
    motion: Option<usize>,
    /// `_STCH` argument 11: the layer this one is placed in.
    parent: Option<usize>,
    /// Turned by this many 1/100 degrees, counter-clockwise (`_STRT`,
    /// `_STCH` argument 19).
    angle: i32,
    /// `_STBR`'s byte: its blend id, so its shader ([`Game::shaders`]).
    shader: u8,
    /// `_STEC(layer, effect, colour)`: the effect (1-82) and its colour.
    effect: Option<(i32, u32)>,
    /// `_STCH` arguments 12–16: the screen rectangle it is drawn in (x, y,
    /// w, h on a 1920 × 1080 screen; none if w or h is 0 or it is the
    /// screen) and its frame's width; a child is drawn in its parent's.
    clip: [i32; 6],
}

/// CONscripter's `spblend` screen mode (lighten, never past white).
const BLEND_SCREEN: u8 = 4;

/// How a shader is drawn here: an `spblend` mode and a filter. The
/// numbers index the exe's shader names at 0xBD2308 (16 bytes each); a
/// `_STBR` byte reaches one through two tables ([`super::exe::Exe::shaders`]).
// src: exe 0xBD2308 (the shader names)
// shortcut: darken, burn and lighten, dodge are drawn as multiply and screen; overlay, soft and hard light and the rest as normal.
fn shader(n: i32) -> (u8, Option<SpriteFilter>) {
    let filter = |kind, p: i32| Some(SpriteFilter { kind: Operand::Const(kind), blend: Operand::Const(FILTER_BLEND_COPY), strength: Operand::Const(256), params: [Operand::Const(p); 3] });
    match n {
        3 | 18 | 25 | 27 => (BLEND_ADD, None),
        19 | 21 | 22 | 29 | 36 | 38 => (BLEND_MULTIPLY, None),
        26 | 28 => (BLEND_SUBTRACT, None),
        20 | 23 | 24 | 30 | 34 | 35 | 37 => (BLEND_SCREEN, None),
        31 | 32 => (BLEND_MULTIPLY, None), // ours: no engine overlay/hardlight; nearest mode
        33 => (BLEND_SCREEN, None), // ours: no engine softlight; nearest mode
        39 | 40 => (BLEND_SUBTRACT, None), // ours: no engine difference; nearest mode
        41 => (BLEND_ADD, None), // ours: no engine exclusion; nearest mode
        7 => (BLEND_NORMAL, filter(FILTER_GREYSCALE, 0)),
        8 => (BLEND_NORMAL, filter(FILTER_NEGATIVE, 0)),
        _ => (BLEND_NORMAL, None),
    }
}

/// A curve, as [`Op::MoveEase`]'s `(ease, exponent)`; `None` straight.
type Curve = Option<(u8, u32)>;

/// The exe's easing `code` (`FUN_00130c80`, checked against it in
/// `ease_as_the_exe`: 2-41, Penner's quad, cubic,
/// quart, quint, sine, circ, expo, elastic, back and bounce, each in,
/// out, in-out, out-in) as [`Op::MoveEase`]'s; 0, 1 and the rest are
/// straight (`None`).
pub(super) fn ease(code: i32) -> Curve {
    let (family, kind) = ((code - 2) / 4, (code - 2) % 4);
    if code < 2 {
        return None;
    }
    match family {
        0..=3 => Some(([1, 2, 3, 8][kind as usize], family as u32 + 2)),
        4 => Some(([4, 5, 6, 9][kind as usize], 0)),
        5..=9 => Some((10 + (family as u8 - 5) * 4 + kind as u8, 0)),
        _ => None,
    }
}

fn label(script: &str, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("tm_{script}__{n}"),
        None => format!("tm_{script}"),
    }
}

/// The motion time command `c` asks for, in ms (timing parity,
/// `metrics.json`): `_FADS(type, ms, …)`, `_STGS`/`_STRT(layer, from, to,
/// ms, …)`, `_STZ4`'s ms (argument 5), `_SHOK`'s period × turns.
/// shortcut: `_STMA` (the exe's motion table) counts 0, read its table when a parity gap points there.
fn motion_ms(c: &Command) -> u64 {
    let n = |i: usize, default: i32| c.args.get(i).filter(|a| !a.trim().is_empty()).map_or(Some(default), |a| number(a)).unwrap_or(0).max(0) as u64;
    match c.opcode() {
        "_FADS" => n(1, 0),
        "_STGS" | "_STRT" => n(3, 0),
        "_STZ4" => n(5, 0),
        "_SHOK" => n(2, 300).max(17) * n(3, 3).max(1),
        _ => 0,
    }
}

/// A whole-number argument: digits with `+`/`-` between them (`258-18`).
/// A `.` ends it (`242.25` is 242): the exe's expression reader
/// (`FUN_00103160`) takes the digits with `strtol` and stops at a `.`.
fn number(arg: &str) -> Option<i32> {
    let arg = arg.trim();
    let mut total = 0i32;
    let mut sign = 1;
    let mut digits = String::new();
    let flush = |digits: &mut String, sign: i32, total: &mut i32| -> Option<()> {
        *total += sign * digits.parse::<i32>().ok()?;
        digits.clear();
        Some(())
    };
    for (i, c) in arg.char_indices() {
        match c {
            '0'..='9' => digits.push(c),
            '.' if !digits.is_empty() => break,
            '+' | '-' if i == 0 => sign = if c == '-' { -1 } else { 1 },
            '+' | '-' => {
                flush(&mut digits, sign, &mut total)?;
                sign = if c == '-' { -1 } else { 1 };
            }
            _ => return None,
        }
    }
    flush(&mut digits, sign, &mut total)?;
    Some(total)
}

/// A backquote option `` `001:4000`` among `args`: the argument it is in
/// and option 1's value.
fn option_at(args: &[&str], n: u32) -> Option<(usize, i32)> {
    args.iter().enumerate().find_map(|(i, a)| {
        let (k, v) = a.strip_prefix('`')?.split_once(':')?;
        (k.parse::<u32>().ok()? == n).then(|| number(v).map(|v| (i, v))).flatten()
    })
}

/// Marks argument `i` read in `read`.
fn mark(read: &Cell<u64>, i: usize) {
    read.set(read.get() | 1u64.checked_shl(i as u32).unwrap_or(0));
}

/// Arguments a converted command passes and the conversion doesn't need
/// to read: (command, argument, why).
const UNREAD_HARMLESS: &[(&str, usize, &str)] = &[
    ("_IFNX", 1, "the label a non-Switch build goes to; this is the Switch"),
    ("_SELD", 0, "the choice's name: one choice is open at a time"),
    ("_SELA", 0, "the choice's name: one choice is open at a time"),
    ("_SELS", 0, "the choice's name: one choice is open at a time"),
    ("_GOTS", 0, "the choice's name: one choice is open at a time"),
    // `FUN_00160840` reads arguments 0–6 and 9–19 only; 8 is always `CC`.
    // `script_command` (0x177790, case "PLY" under V) reads the name and argument 2 only.
    ("_VPLY", 1, "the Switch exe never reads it"),
    ("_STCH", 7, "the Switch exe never reads it"),
    ("_STCH", 8, "the Switch exe never reads it"),
    ("_SCH2", 7, "the Switch exe never reads it"),
    ("_SCH2", 8, "the Switch exe never reads it"),
    ("_SQK2", 8, "the Switch exe never reads it (cmd_SQK2 0x191540 reads 0-7)"),
];

/// Commands left out on purpose (command, why): done for the completion
/// figure (`metrics.json`'s `left_out`), not gaps.
const LEFT_OUT: &[(&str, &str)] = &[
    // `script_command` (0x177790) has no case for it: the Switch exe does nothing.
    ("_SNX4", "the Switch exe ignores it"),
    ("_FANX", "the Switch exe ignores it"),
    ("_STQK", "the Switch exe ignores it"),
    ("_TRPY", "the Switch exe ignores it"),
    // Its case does nothing.
    ("_NOP_", "the Switch exe ignores it"),
    // A cache of pictures loaded ahead (FUN_001b6a20) and its release:
    // nothing drawn; CONscripter loads a picture when it shows it.
    ("_SPFC", "a picture cache: the engine loads pictures as they show"),
    ("_SPRC", "a picture cache: the engine loads pictures as they show"),
    // `script_command` stores the page in the state (+0x8c4) for the save; its
    // only reader is the load (`FUN_00182330` → `FUN_00182c60`), which replays
    // the script until `_PGST` reaches the saved page. CONscripter's saves
    // restore the state themselves.
    ("_PGST", "the page a load replays to; CONscripter's saves restore the state"),
];

/// An opcode's channel in `metrics.json`'s score (the findings rows say
/// what each does).
pub fn channel(kind: &str) -> crate::metrics::Channel {
    use crate::metrics::Channel::*;
    match kind {
        "_ZM" | "_MSAD" | "_WNTY" | "_CLO2" | "_CLO3" | "_CLO4" | "_CLO5" | "_SELR" | "_SELD" | "_SELA" | "_SELS" | "_GOTS" => Text,
        "_SEPL" | "_SEFD" | "_SESP" | "_SEVL" | "_MPLY" | "_MFAD" | "_MVOL" | "_VPLY" | "_VPL2" => Sound,
        "_STCH" | "_SCH2" | "_STCC" | "_STCP" | "_STBR" | "_STEC" | "_SACL" | "_SBAS" | "_SNX2" | "_SNX3" | "_SOXY" | "_SPRD" | "_STNS" | "_STPV" | "_STRR" | "_STTR"
        // an overlay of a kind not known yet (findings rows: open)
        | "_XRSM" | "_XSTP" => Picture,
        "_FADS" | "_FADE" | "_STMA" | "_STZ4" | "_STGS" | "_STRT" | "_STRA" | "_STCL" | "_SHOK" | "_SHAF" | "_SQK2" | "_SHK2" | "_SSW2" | "_SQS2" | "_SHS2" | "_SSS2" | "_MSQK" | "_FCLN" | "_FCSP" | "_SGMA"
        | "_SPMA" => Motion,
        "_WTTM" | "_VCWT" | "_STCF" | "_TOMW" | "_WTSV" => Timing,
        "_TOMP" => Movie,
        "_IF__" | "_IFNX" | "_IFP4" | "_IFSC" | "_IFUR" | "_WKST" | "_WKAD" | "_WKOR" | "_GOTO" | "_ZZ" | "_ZY" | "_JUMP" | "_RDST" | "_RDAS" | "_RDED" | "_RTST" | "_RTM_" | "_RTM2" | "_RTMN" | "_END_"
        | "_ATSV" | "_PGST" | "_NOP_" | "_SNX4" | "_FANX" | "_STQK" | "_TRPY" | "_SPFC" | "_SPRC" | "_STCG" => None,
        _ => Unclassed,
    }
}

/// What an opcode's argument controls (the findings rows), for the
/// score's credit (`metrics::credit`): an unread one loses that much of
/// its call, not one argument's part.
const ROLES: &[(&str, usize, &str)] = &[
    // _STGS(layer, from, to, ms, easing): an opacity fade
    ("_STGS", 1, "picture"),
    ("_STGS", 2, "picture"),
    ("_STGS", 3, "tempo"),
    ("_STGS", 4, "tempo"),
    // _STMA(layer, m, mode, p): p the motion's start time
    ("_STMA", 3, "tempo"),
    // _STRT(layer, to, from, ms, cx, cy): a turn
    ("_STRT", 3, "tempo"),
];

/// Why a converted call is only in part: each argument passed and not
/// read (`<command> argument <i> ignored`, or `option <n>` for a backquote
/// option).
fn unread(c: &Command, read: u64) -> Vec<String> {
    let op = c.opcode();
    c.args
        .iter()
        .enumerate()
        .filter(|&(i, a)| !a.trim().is_empty() && (i >= 64 || read & (1 << i) == 0) && !UNREAD_HARMLESS.iter().any(|&(o, k, _)| o == op && k == i))
        .map(|(i, a)| match a.trim().strip_prefix('`').and_then(|o| o.split_once(':')) {
            Some((k, _)) => format!("{op} option {} ignored", k.parse::<u32>().unwrap_or(0)),
            None => match ROLES.iter().find(|&&(o, k, _)| o == op && k == i) {
                Some((_, _, role)) => format!("{op} argument {i} ignored ({role})"),
                None => format!("{op} argument {i} ignored"),
            },
        })
        .collect()
}

/// A text line's markup: `<base|ruby>` readings, `^` line breaks, `@x`
/// codes and `[ber00]` inline pictures (both dropped), and the fan
/// patch's styled letters (see [`styled`]).
pub fn text(line: &str) -> (String, Vec<Ruby>, Vec<TextStyle>) {
    let (mut out, mut ruby, mut styles) = (String::new(), Vec::new(), Vec::<TextStyle>::new());
    let line = unmirror(line);
    let mut chars = line.chars().peekable();
    let mut len = 0;
    while let Some(c) = chars.next() {
        match c {
            '^' => {
                out.push('\n');
                len += 1;
            }
            '@' => {
                chars.next();
            }
            '[' if line.contains(']') => {
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            '<' => {
                let inner: String = chars.by_ref().take_while(|&c| c != '>').collect();
                let (base, reading) = inner.split_once('|').unwrap_or((&inner, ""));
                let n = base.chars().count();
                if !reading.is_empty() {
                    ruby.push(Ruby { start: len, len: n, text: reading.chars().map(|c| styled(c).map_or(c.to_string(), |s| s.0)).collect() });
                }
                out.push_str(base);
                len += n;
            }
            _ if let Some((plain, italic)) = styled(c) => {
                if italic {
                    match styles.last_mut() {
                        // over a plain space between two italic words too
                        Some(s) if s.start + s.len == len || (s.start + s.len + 1 == len && out.ends_with(' ')) => s.len = len - s.start + plain.chars().count(),
                        _ => styles.push(TextStyle { start: len, len: plain.chars().count(), italic: true, ..Default::default() }),
                    }
                }
                out.push_str(&plain);
                len += plain.chars().count();
            }
            _ => {
                out.push(c);
                len += 1;
            }
        }
    }
    (out, ruby, styles)
}

/// What [`text`] drops from a line, as notes: its `@x` codes and its
/// `[ber00]` inline pictures.
fn dropped(line: &str) -> Vec<String> {
    let mut notes: Vec<String> = line.match_indices('@').filter_map(|(i, _)| line[i + 1..].chars().next()).map(|c| format!("text code @{c} dropped")).collect();
    if line.contains('[') && line.contains(']') {
        notes.push("text inline picture dropped".into());
    }
    notes
}

/// The fan patch's letters in private-use code points, which its font
/// (`ja/TT_TsukuMin-D.ttf`, glyphs `uniE000`–`uniE2FE`) draws styled: the
/// ASCII letter and whether it is italic. U+E0xx italic, U+E1xx mirrored,
/// U+E1(80+xx) a serif italic, U+E2(80+xx) upside down, U+E200 a raised
/// "-35" (`10^-35 m`). Mirrored and upside-down letters are drawn plain
/// (mirrored ones put back in reading order by [`unmirror`]).
/// The fan patch's mirrored letters (U+E120–U+E17E) are stored right to
/// left, so its font draws a mirror image of the line: on each line (`^`)
/// the stretch from the first to the last of them (with what lies between,
/// `■■` or spaces) is put back in reading order.
fn unmirror(line: &str) -> String {
    let mirrored = |c: char| ('\u{E120}'..='\u{E17E}').contains(&c);
    let lines: Vec<String> = line
        .split('^')
        .map(|l| {
            let c: Vec<char> = l.chars().collect();
            let (Some(a), Some(b)) = (c.iter().position(|&x| mirrored(x)), c.iter().rposition(|&x| mirrored(x))) else { return l.to_string() };
            if c[a..=b].iter().any(|x| "<>[]@".contains(*x)) {
                return l.to_string();
            }
            c[..a].iter().chain(c[a..=b].iter().rev()).chain(&c[b + 1..]).collect()
        })
        .collect();
    lines.join("^")
}

fn styled(c: char) -> Option<(String, bool)> {
    let (hi, lo, high) = (c as u32 >> 8, c as u32 & 0x7F, c as u32 & 0x80 != 0);
    match hi {
        0xE2 if c == '\u{E200}' => Some(("^-35".into(), false)),
        0xE0..=0xE2 if (0x20..0x7F).contains(&lo) => Some(((lo as u8 as char).to_string(), hi == 0xE0 || (hi == 0xE1 && high))),
        _ => None,
    }
}

/// The plain colours' fills (`BLACK` …), by name.
// src: exe 0x6E02A4 (the seven plain colours, first in the picture table); allpac's textures (their colours)
fn colour(name: &str) -> Option<u32> {
    Some(match name {
        "black" => 0x000000,
        "blue" => 0x0000FF,
        "green" => 0x00FF00,
        "orange" => 0xFF7F00,
        "red" => 0xFF0000,
        "white" => 0xFFFFFF,
        "yellow" => 0xFFFF00,
        _ => return None,
    })
}

struct Lowering<'g, 'a> {
    g: &'g Game<'a>,
    ops: Vec<Op>,
    notes: BTreeMap<String, usize>,
    metrics: Metrics,
    vars: HashMap<String, Var>,
    scene_labels: HashMap<i16, String>,
    shown: Vec<Option<Layer>>,
    staged: Vec<Option<Layer>>,
    /// Each layer's priority (`_STCP`, `_STCH` argument 6), staged and shown.
    priority: Vec<i32>,
    shown_priority: Vec<i32>,
    /// What each IR layer shows, and the layer it shows.
    slots: Vec<Option<Layer>>,
    slot_layer: Vec<Option<usize>>,
    /// Each IR layer's `spblend` mode.
    blends: Vec<u8>,
    /// Each IR layer's `spclip` rectangle.
    clips: Vec<Option<Clip>>,
    /// `_WNTY(1)`: the next line ends its page.
    page_end: bool,
    /// The text window was hidden or faded (`_CLO2`/`_CLO3`/`_CLO4`): the
    /// next text fades it in.
    window_faded: bool,
    voice: Option<Voice>,
    /// A line ended with `@n`: the next one goes on after it, no click.
    held: Option<(String, Vec<Ruby>, Vec<TextStyle>)>,
    /// The markup [`text`] drops in the lines of the page being built
    /// (`metrics.json`'s "text").
    page_notes: Vec<String>,
    /// `_SEPL`'s channels: the sound on each, and whether it loops.
    se_channels: HashMap<i32, (String, bool)>,
    /// `_SELR`'s options so far: (index, text, greyed, under an `_IF__`).
    options: Vec<(i32, String, bool, bool)>,
    /// The `_IF__`s whose label hasn't come yet in this script.
    open_ifs: Vec<String>,
    /// Bit i: option i, under an `_IF__`, is offered (`_SELR`'s run-time
    /// part); the choice picks its option set by it and clears it.
    choice_mask: Var,
    /// Choices with options under an `_IF__` so far (their labels).
    choices: usize,
    /// The pictures drawn from `pic/en/` (the fan patch's own).
    own: &'g HashSet<String>,
    /// The English copy of a build with both languages.
    english: bool,
    /// `_SELS` ran: the next `_GOTS` asks yes or no.
    question: bool,
    /// The looping motion each shown layer runs (`FUN_001999a0`).
    looping: Vec<Option<usize>>,
    /// The looping `spmotion`s running: the layer whose motion it is, and
    /// the IR layers it runs on with the layer each shows (its own, its
    /// children's).
    loop_runs: Vec<(usize, Vec<(usize, usize)>)>,
    /// Where each command's ops start ([`Compiled::source_map`]).
    source_map: Vec<(usize, conv_core::ir::SourceLoc)>,
    /// The `_STMA` that gave a staged layer its `Layer::motion`: its
    /// `spmotion` at the `_FADS` is marked as that command's.
    motion_from: Vec<Option<conv_core::ir::SourceLoc>>,
    /// What the last `spmotion` on each IR layer is, as `_STCF`'s mask
    /// bits (`FUN_001329f0`): 8 an `_STGS` fade, 0x20 an `_STMA` motion.
    motion_bits: Vec<u8>,
    /// Each layer's quakes in the exe's two slots (0: `_SQK2`, 1: `_SHK2`/`_SSW2`).
    quakes: Vec<[Option<Placed>; 2]>,
    /// The quake (its id) each IR layer's `spshake` slots 1, 2 run.
    shaking: Vec<[Option<u32>; 2]>,
    /// The last quake's id (its random seed).
    quake_id: u32,
}

/// A layer's quake: its id, its `spshake` list (`None`: it moves no
/// pixel), whether it runs for ever, and `_SQK2`'s follow (layers 2 up
/// take layer 0's or 1's offsets).
#[derive(Debug, Clone)]
struct Placed {
    id: u32,
    effect: Option<ScreenEffect>,
    forever: bool,
    follow: bool,
}

impl Lowering<'_, '_> {
    fn note(&mut self, what: impl Into<String>) {
        *self.notes.entry(what.into()).or_default() += 1;
    }

    fn var(&self, arg: &str) -> Option<Var> {
        self.vars.get(arg).copied()
    }

    fn operand(&self, arg: &str) -> Option<Operand> {
        number(arg).map(Operand::Const).or_else(|| self.var(arg).map(Operand::Var))
    }

    /// `_FADS`'s and `_STMA`'s `mode` (`FUN_00160610`, `FUN_00199620`): 1
    /// hides the text window until the next text (the flag at 0x89E that
    /// `_CLO2` sets), 2 hides it too (for `_FADS` over a fade of 2 ms or
    /// more) under the settings' byte 99 at its default 1; none (or −1)
    /// would under byte 99 = 0, and 0 never does. Not while the window
    /// fades (`_CLO4`, `_CLO5`): left out here.
    fn hide_window_for(&mut self, mode: Option<i32>, ms: Option<i32>) {
        if mode == Some(1) || (mode == Some(2) && ms.is_none_or(|ms| ms > 1)) {
            self.ops.push(Op::HideTextWindow);
            self.window_faded = true;
        }
    }

    /// Lowers one command; false when it isn't (yet). The arguments it
    /// reads are marked in `read`.
    fn command(&mut self, script: &str, c: &Command, read: &Cell<u64>) -> bool {
        let a = &c.args;
        let arg = |i: usize| {
            mark(read, i);
            a.get(i).copied().unwrap_or("").trim()
        };
        let option = |args: &[&str], n: u32| option_at(args, n).map(|(i, v)| {
            mark(read, i);
            v
        });
        match c.opcode() {
            "_ZZ" | "_ZY" => {
                self.open_ifs.retain(|l| l != arg(0));
                if !arg(0).is_empty() {
                    self.ops.push(Op::Label(label(script, Some(arg(0)))));
                }
            }
            "_JUMP" => {
                let Some(target) = number(arg(0)).and_then(|n| self.scene_labels.get(&(n as i16))) else { return false };
                let target = if arg(1).is_empty() { target.clone() } else { format!("{target}__{}", arg(1)) };
                self.ops.push(Op::Jump(target));
            }
            "_IF__" => {
                let cmp = match arg(1) {
                    "eq" => Compare::Ne,
                    "ne" => Compare::Eq,
                    "lt" => Compare::Ge,
                    "le" => Compare::Gt,
                    "gt" => Compare::Le,
                    "ge" => Compare::Lt,
                    _ => return false,
                };
                let (Some(var), Some(value)) = (self.var(arg(0)), self.operand(arg(2))) else { return false };
                // Runs on when it holds, else skips to the local label.
                self.open_ifs.push(arg(3).to_string());
                self.ops.push(Op::If { var, cmp, value, label: label(script, Some(arg(3))) });
            }
            "_WKST" | "_WKAD" => {
                let (op, value) = if c.opcode() == "_WKST" { (ArithOp::Set, arg(2)) } else { (ArithOp::Add, arg(1)) };
                if c.opcode() == "_WKST" && arg(1) != "eq" {
                    return false;
                }
                let (Some(var), Some(value)) = (self.var(arg(0)), self.operand(value)) else { return false };
                self.ops.push(Op::Arith { var, op, value });
            }
            "_RDST" => {
                let Some(n) = number(arg(0)).filter(|n| (0..0x240).contains(n)) else { return false };
                self.ops.push(Op::Arith { var: read_flag(n as u16), op: ArithOp::Set, value: Operand::Const(1) });
            }
            // `cmd_RTST(n)` sets per-play bit n and, for a lesson (0x215-0x22d),
            // the current scene +0xdc = n: only `_RDED` reads them, and every
            // lesson runs `_RTST(n)` before its `_RDED(n)`, so nothing to keep.
            "_RTST" => {
                arg(0);
            }
            // `cmd_RDED(n)`: global read flag n (words 300 up) when n is the
            // current scene with its per-play bit set, which holds for the
            // lessons (above). shortcut: only the lessons 533-557, the flags
            // `_IFUR` and the gallery read; the scenes' 1-532 with the Flowchart
            "_RDED" => {
                let Some(var) = number(arg(0)).and_then(lesson_flag) else { return false };
                self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(1) });
                // the Ending List's last entry, and the end of a lesson it replays
                self.ops.extend([Op::Gosub(super::gallery::ALL_LESSONS_CHECK.into()), Op::SceneEnd]);
            }
            // `_IFUR(1, L, LV)` (`all_read_check(1)`): to L when lessons
            // 533-556 are all read; else on to the `_RTM_` after it (`QA_25`,
            // its only use), which ends the script: here back to the title.
            "_IFUR" if arg(0) == "1" => {
                arg(2); // the exe reads up to the label: LV, the else label, is the compiler's
                let not_all = label(script, Some(&format!("{}_not_all", arg(1))));
                for n in 533..557 {
                    self.ops.push(Op::If { var: lesson_flag(n).unwrap(), cmp: Compare::Eq, value: Operand::Const(0), label: not_all.clone() });
                }
                self.ops.push(Op::Jump(label(script, Some(arg(1)))));
                self.ops.push(Op::Label(not_all));
                self.ops.push(Op::ReturnToTitle);
            }
            "_STCG" => {
                let Some(&var) = self.vars.get(&format!("_STCG {}", arg(0).to_lowercase())) else { return false };
                self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(1) });
            }
            // `_RTM_` ends the script (back to the caller: the title here; the
            // Ending List's replays end at `_RDED` first). Live where no
            // `_END_` follows: `QACALL`'s "no" to a lesson, `STAFFROLL`.
            "_END_" | "_RTM_" | "_RTMN" => self.ops.push(Op::ReturnToTitle),
            // `_TOMP(n)` (`FUN_0016ef30`) plays movie-table entry n in
            // overlay 0 and `_TOMW` waits for it: one blocking movie here.
            // It stops skip and auto mode (but n = 7, which no script
            // plays); a press fades it out (`FUN_00140480`). Left out: the
            // 48-frame fade in, the 1 s fade out on a press, and the hold
            // to the exe's end frame (day cards 420, credits 618).
            "_TOMP" => {
                let n: usize = arg(0).parse().unwrap_or(usize::MAX);
                let Some(file) = self.g.movies.get(n).cloned() else { return false };
                self.ops.push(Op::StopAutoSkip);
                match movie_seen(n) {
                    None => self.ops.push(Op::Movie { file, skippable: true }),
                    Some(var) => {
                        let (seen, done) = (label(script, Some(&format!("movie{n}_seen"))), label(script, Some(&format!("movie{n}_done"))));
                        self.ops.extend([
                            Op::If { var, cmp: Compare::Eq, value: Operand::Const(1), label: seen.clone() },
                            Op::Movie { file: file.clone(), skippable: false },
                            Op::Jump(done.clone()),
                            Op::Label(seen),
                            Op::Movie { file, skippable: true },
                            Op::Label(done),
                            Op::Arith { var, op: ArithOp::Set, value: Operand::Const(1) },
                        ]);
                    }
                }
            }
            "_TOMW" => {}
            "_GOTO" => self.ops.push(Op::Jump(label(script, Some(arg(0))))),
            // `cmd_SHOK(x, y, period = 300, turns = 3, delay = 0)` (`FUN_0012ee70`
            // kind 1, drawn by `FUN_0012ef30`): both axes on one sine, a turn
            // every `period` ms, x and y in 1280 × 720 units (it scales them by
            // 640/1280 and 480/720), the size falling straight to 0 over the
            // turns, then it ends, `delay` ms after the command (`FUN_0012ef30`
            // counts it down). shortcut: shaken while skipping too (the exe
            // doesn't; it ends by itself within ~1 s).
            "_SHOK" => {
                let n = |i: usize, default: i32| if arg(i).is_empty() { Some(default) } else { number(arg(i)) };
                let (Some(x), Some(y), Some(period), Some(turns), Some(delay)) = (n(0, 0), n(1, 0), n(2, 300), n(3, 3), n(4, 0)) else { return false };
                let (period, turns) = (period.max(17), turns.max(1));
                let time_ms = (period * turns) as u32;
                let speed = (65536 + period / 2) / period;
                let range = [(x * SCREEN_W as i32 / 1280, y * SCREEN_H as i32 / 720), (0, 0)];
                self.ops.push(Op::ScreenEffect { slot: 1, effect: Some(ScreenEffect::Shake { delay_ms: delay.max(0) as u32, time_ms, interval_ms: 16, path: false, speed: (speed, speed), range, end_ms: time_ms }) });
            }
            "_SQK2" | "_SHK2" | "_SSW2" => {
                let n = |i: usize, default: i32| if arg(i).is_empty() { Some(default) } else { number(arg(i)) };
                let kind = match c.opcode() {
                    "_SQK2" => 1,
                    "_SHK2" => 2,
                    _ => 3,
                };
                return self.quake_start(kind, n);
            }
            "_SQS2" | "_SHS2" | "_SSS2" => {
                let Some(n) = number(arg(0)) else { return false };
                self.quake_stop(n, usize::from(c.opcode() != "_SQS2"));
            }
            // `shake_stop`: `_SHAF()` ends it at once (`_SHAF(ms)`, unused, decays it).
            "_SHAF" if arg(0).is_empty() => self.ops.push(Op::ScreenEffect { slot: 1, effect: None }),
            // The Switch's and the PS4's own lines: this is the Switch.
            "_IFNX" => {}
            "_IFP4" | "_IFSC" => self.ops.push(Op::Jump(label(script, Some(arg(1))))),
            // `_SELR(index, text[, 1])` (`script_command` 0x177790, case "ELR"):
            // adds a slot to the choice, in the order run, with the index
            // `_GOTS` picks its label by; the text is a line (`$042569`) or
            // written as it is (the chapter-jump menus' script names);
            // 1: shown greyed, which the cursor skips (`FUN_00183e50`).
            "_SELR" => {
                let t = arg(1);
                let text = match t.strip_prefix('$') {
                    Some(n) => {
                        let Some(line) = n.get(..6).and_then(|n| n.parse::<usize>().ok()).and_then(|n| self.g.lines.get(n)) else { return false };
                        // a `^` break: one line, as `select` shows it
                        text(line).0.split('\n').map(str::trim).collect::<Vec<_>>().join(" ")
                    }
                    None if !t.is_empty() => t.to_string(),
                    None => return false,
                };
                let i = number(arg(0)).unwrap_or(0);
                if !(0..16).contains(&i) {
                    return false;
                }
                let grey = arg(2) == "1";
                let cond = !self.open_ifs.is_empty();
                if cond {
                    self.ops.push(Op::Arith { var: self.choice_mask, op: ArithOp::BitOr, value: Operand::Const(choice_bit(i, grey)) });
                }
                self.options.push((i, text, grey, cond));
            }
            // `_SELD`/`_SELA(_D000, …)` show the choice; `_GOTS(_D000,
            // label, …)` goes to the label of the option picked. `_SELS` asks
            // yes or no (`question.rs`).
            "_SELD" | "_SELA" => {}
            "_SELS" => self.question = true,
            "_GOTS" if self.question => {
                self.question = false;
                (1..a.len()).for_each(|i| mark(read, i));
                let Some(lesson) = self.var("F027") else { return false };
                let (no, yes) = (label(script, Some(arg(1))), label(script, Some(arg(2))));
                self.choices += 1;
                let k = self.choices;
                let tag = |n: i32| label(script, Some(&format!("question{k}_{n}")));
                self.ops.extend(super::question::ops(lesson, no, yes, self.english, tag));
            }
            "_GOTS" => {
                let options = std::mem::take(&mut self.options);
                (1..a.len()).for_each(|i| mark(read, i));
                let labels = &a[1..];
                // Each index once, where it is first run. Its two bits
                // (offered, greyed) are set for sure when run outside an
                // `_IF__`, else by the mask; offered wins over greyed.
                struct Slot {
                    i: i32,
                    text: String,
                    sure: i32,
                    maybe: i32,
                }
                let mut order: Vec<Slot> = Vec::new();
                for (i, text, grey, cond) in options {
                    let at = match order.iter().position(|o| o.i == i) {
                        Some(at) => at,
                        None => {
                            order.push(Slot { i, text, sure: 0, maybe: 0 });
                            order.len() - 1
                        }
                    };
                    let o = &mut order[at];
                    *(if cond { &mut o.maybe } else { &mut o.sure }) |= choice_bit(i, grey);
                }
                let target = |i: i32| Some(label(script, Some(labels.get(usize::try_from(i).ok()?)?.trim())));
                let pick = |mask: i32| -> Option<Op> {
                    let (mut options, mut disabled) = (Vec::new(), Vec::new());
                    for o in &order {
                        let set = o.sure | (o.maybe & mask);
                        let offered = set & choice_bit(o.i, false) != 0;
                        let Some(to) = target(o.i).filter(|_| offered || set & choice_bit(o.i, true) != 0) else { continue };
                        if !offered {
                            disabled.push(options.len());
                        }
                        options.push((o.text.clone(), to));
                    }
                    // none to pick: no choice
                    (disabled.len() < options.len()).then_some(Op::Choice { options, style: None, disabled })
                };
                // The mask bits that change what is shown.
                let mut bits = Vec::new();
                for o in &order {
                    let (on, grey) = (choice_bit(o.i, false), choice_bit(o.i, true));
                    if o.sure & on != 0 {
                        continue;
                    }
                    if o.maybe & on != 0 {
                        bits.push(on);
                    }
                    if o.maybe & grey != 0 && o.sure & grey == 0 {
                        bits.push(grey);
                    }
                }
                if bits.is_empty() {
                    let Some(choice) = pick(0) else { return false };
                    self.ops.push(choice);
                    return true;
                }
                // One choice per set of the options under an `_IF__`.
                self.choices += 1;
                let masks: Vec<i32> = (0..1 << bits.len()).map(|m: i32| bits.iter().enumerate().filter(|(j, _)| m & (1 << j) != 0).map(|(_, b)| b).sum()).collect();
                let leaf = |m: i32| label(script, Some(&format!("choice{}_{m}", self.choices)));
                for &m in &masks[1..] {
                    self.ops.push(Op::If { var: self.choice_mask, cmp: Compare::Eq, value: Operand::Const(m), label: leaf(m) });
                }
                for (k, &m) in masks.iter().enumerate() {
                    if k > 0 {
                        self.ops.push(Op::Label(leaf(m)));
                    }
                    self.ops.push(Op::Arith { var: self.choice_mask, op: ArithOp::Set, value: Operand::Const(0) });
                    // shortcut: a set with no option offered (none in the
                    // game) goes on to the next set's choice.
                    if let Some(choice) = pick(m) {
                        self.ops.push(choice);
                    }
                }
            }
            // `script_command` (0x177790): `_CLO2` hides the text window at once
            // (alpha 0), `_CLO3` lets `FUN_00138d40` fade it out in 4 frames,
            // `_CLO4(ms)` fades window and text out over `ms` (visible over the
            // first half: drawn at 2 × (fade − 0.5)), `_CLO5(ms)` shows the
            // window and fades it in the same way. None of them blocks. The
            // next text fades it back in over 18 frames (`FUN_0016e0d0`).
            "_CLO2" => {
                self.ops.push(Op::HideTextWindow);
                self.window_faded = true;
            }
            "_CLO3" => {
                self.ops.push(Op::WindowFade { alpha: 0, ms: 67, from: None, delay: 0, show: false });
                self.window_faded = true;
            }
            "_CLO4" | "_CLO5" => {
                let Some(ms) = number(arg(0)) else { return false };
                let half = (ms.max(0) / 2) as u32;
                if c.opcode() == "_CLO4" {
                    self.ops.push(Op::WindowFade { alpha: 0, ms: half, from: None, delay: 0, show: false });
                    self.window_faded = true;
                } else {
                    self.ops.push(Op::WindowFade { alpha: 256, ms: half, from: Some(0), delay: half, show: true });
                    self.window_faded = false;
                }
            }
            "_WNTY" => self.page_end = arg(0) == "1",
            "_VPLY" => self.voice = Some(Voice { file: format!("stream/{}.ogg", arg(0)), character: 0 }),
            "_ZM" | "_MSAD" => {
                // `$000107`, with codes around it (`@x$001549@n`, `^$042767`).
                let Some(n) = arg(0).split_once('$').and_then(|(_, s)| s.get(..6)?.parse::<usize>().ok()) else { return false };
                let Some(line) = self.g.lines.get(n) else { return false };
                let (mut body, mut ruby, mut styles) = text(line);
                self.page_notes.extend(dropped(line));
                if let Some((before, mut r, mut s)) = self.held.take() {
                    let shift = before.chars().count();
                    ruby.iter_mut().for_each(|x| x.start += shift);
                    styles.iter_mut().for_each(|x| x.start += shift);
                    r.append(&mut ruby);
                    s.append(&mut styles);
                    (body, ruby, styles) = (before + &body, r, s);
                }
                // shortcut: `@n` joins the next line to this one, no click; what happens between them is shown before both.
                if arg(0).contains("@n") {
                    self.held = Some((body, ruby, styles));
                    return true;
                }
                let continues = !std::mem::take(&mut self.page_end);
                if std::mem::take(&mut self.window_faded) {
                    // 18 frames, drawn at 2 × (fade − 0.5): hidden 150 ms, then in over 150
                    self.ops.push(Op::WindowFade { alpha: 256, ms: 150, from: Some(0), delay: 150, show: false });
                }
                self.ops.push(Op::Text { speaker: None, text: body, voice: self.voice.take(), links: vec![], ruby, styles, hang: None, continues });
                self.metrics.page(std::mem::take(&mut self.page_notes));
            }
            // `_WTTM(ms[, 1])`: ms (at least a frame). The decompile seems to take
            // off a frame counter (0x1473cde) with a second argument, but nothing
            // resets it (two code references: here and the tick adding to it),
            // and the original's trace waits ms (33 of 46 within 2 frames, the
            // rest shorter with `0` as with `1`): the 1 changes nothing seen.
            "_WTTM" => {
                arg(1);
                self.ops.push(Op::Wait { ms: number(arg(0)).unwrap_or(0).max(0) as u32 });
            }
            // `_SEPL(channel, loop, sound, …)` (`FUN_0016a340`): the sound
            // plays on its channel, in place of the one there. (The same
            // sound looping there already at a volume above 0 goes on.)
            // shortcut: a loop on the channel is stopped first (the same one starts again; the channels are
            // followed down the script, not its jumps); a one-shot cut short by the next is let play.
            "_SEPL" => {
                let Some(file) = self.g.waves.get(&arg(2).to_uppercase()).map(|w| format!("se/{w}")) else { return false };
                let (ch, looped) = (number(arg(0)).unwrap_or(0), arg(1) == "1");
                if let Some((old, true)) = self.se_channels.insert(ch, (file.clone(), looped)) {
                    self.ops.push(Op::SeFadeOut { file: Some(old), ms: 0 });
                }
                // Option 17: the volume (0–100, default 100), reached over option 1's ms.
                let fade = option(a, 1).unwrap_or(0).max(0) as u32;
                if fade > 1 {
                    self.ops.push(Op::SeFadeIn { ms: fade });
                }
                let percent = option(a, 17).unwrap_or(100).clamp(0, 100) as u32;
                if percent < 100 {
                    self.ops.push(Op::SoundVolume { file: Some(file.clone()), percent, ms: 0 });
                }
                self.ops.push(Op::SePlay { file, looped });
            }
            // `_SEVL(channel, …)` (`FUN_0016ae10`): the sound on channel n % 20
            // to option 17's volume (0–100, none: 0) over option 1's ms (none: at once).
            "_SEVL" => {
                let ch = number(arg(0)).unwrap_or(0) % 20;
                let percent = option(a, 17).unwrap_or(0).clamp(0, 100) as u32;
                let ms = option(a, 1).unwrap_or(0).max(0) as u32;
                if let Some((file, _)) = self.se_channels.get(&ch) {
                    self.ops.push(Op::SoundVolume { file: Some(file.clone()), percent, ms });
                }
            }
            // `_MVOL(…)` (`cmd_MVOL`): the music to option 17's volume (none: 0) over option 1's ms.
            "_MVOL" => {
                let percent = option(a, 17).unwrap_or(0).clamp(0, 100) as u32;
                self.ops.push(Op::SoundVolume { file: None, percent, ms: option(a, 1).unwrap_or(0).max(0) as u32 });
            }
            "_SEFD" => {
                let ms = option(a, 1).unwrap_or(0).max(0) as u32;
                let ch = number(arg(0)).unwrap_or(-1);
                let file = if ch < 0 {
                    self.se_channels.clear();
                    None
                } else {
                    let Some((f, _)) = self.se_channels.remove(&ch) else { return true };
                    Some(f)
                };
                self.ops.push(Op::SeFadeOut { file, ms });
            }
            "_MPLY" => {
                let n = number(arg(0)).unwrap_or(-1);
                if n < 0 {
                    self.ops.push(Op::BgmStop { fade_out_ms: 0 });
                    return true;
                }
                let Some(name) = self.g.sounds.get(n as usize - 1).and_then(|s| s.strip_prefix("STRM_")) else { return false };
                let percent = option(a, 17).unwrap_or(100).clamp(0, 100) as u32;
                if percent < 100 {
                    self.ops.push(Op::SoundVolume { file: None, percent, ms: 0 });
                }
                self.ops.push(Op::BgmPlay { file: format!("stream/{name}.ogg"), fade_in_ms: option(a, 1).unwrap_or(0).max(0) as u32 });
            }
            "_MFAD" => {
                if option(a, 11).is_some_and(|v| v != 0) {
                    return false;
                }
                self.ops.push(Op::BgmStop { fade_out_ms: option(a, 1).unwrap_or(0).max(0) as u32 });
            }
            "_STCH" => return self.stch(a, read, true),
            "_SCH2" => return self.stch(a, read, false),
            // `_STCC(n)` (`FUN_00162140`): clears layer n (all for n < 0), its
            // priority to 0 too.
            "_STCC" => {
                let n = number(arg(0)).unwrap_or(0);
                for i in (0..LAYERS).filter(|&i| n < 0 || i as i32 == n) {
                    self.staged[i] = None;
                    self.priority[i] = 0;
                }
            }
            // `_STZ4(layer, x0, y0, x1, y1, ms, ease, off)` (`FUN_00162d20`): the
            // zoom in ‰ goes from x0, y0 (default: the layer's) to x1, y1
            // (`_STCH` resets it to 1000) over ms · 60 / 1000 frames along the
            // easing (`FUN_00132030`, drawn by `FUN_00138a70` with
            // `FUN_00130c80`), on the shown layer at once (`FUN_00131f30`); at
            // 0 frames it is x1, y1 at once. `_STCF` doesn't wait for it
            // (`FUN_001329f0` reads no zoom count). `off` (empty: the layer's,
            // `_STCH` clears it) keeps the zoom from the draw (`FUN_00132e30`).
            "_STZ4" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                let Some(l) = self.staged[n as usize].as_mut() else { return true };
                let z = |i: usize, old: f32| if arg(i).is_empty() { Some(old) } else { number(arg(i)).map(|v| v as f32 / 1000.0) };
                let (Some(zx), Some(zy)) = (z(3, l.stretch.0), z(4, l.stretch.1)) else { return false };
                let raw = |i: usize| a.get(i).map_or("", |v| v.trim());
                // src: exe FUN_00131f30 (_STZ4: x0, y0 = the starting zoom, arguments 1 and 2)
                let from = (if raw(1).is_empty() { Some(l.stretch.0) } else { number(raw(1)).map(|v| v as f32 / 1000.0) }, if raw(2).is_empty() { Some(l.stretch.1) } else { number(raw(2)).map(|v| v as f32 / 1000.0) });
                l.stretch = (zx, zy);
                if let Some(off) = number(arg(7)) {
                    l.stretch_off = off as u8 != 0; // src: a byte at 0x874 of the state
                }
                // src: exe FUN_00131f30 (_STZ4 duration, argument 5)
                let ms = if raw(5).is_empty() { Some(0) } else { number(raw(5)) };
                // src: exe 0x162d20 (ms × 60 / 1000 frames; under 17 ms none)
                let timed = match (ms, from) {
                    // src: exe 0x132820 (under 17 ms is at once)
                    (Some(ms), _) if ms < 17 => true,
                    // From where it ends: the same at once.
                    (Some(_), (Some(x0), Some(y0))) if (x0, y0) == (zx, zy) => true,
                    (Some(ms), (Some(x0), Some(y0))) => {
                        let start = Layer { stretch: (x0, y0), ..l.clone() };
                        // src: exe FUN_00130c80 (easing) drawn by FUN_00138a70; no bit of _STCF waits for a zoom (FUN_001329f0)
                        self.timed_now(n as usize, start, ms as u32, ease(number(raw(6)).unwrap_or(0)), 0)
                    }
                    _ => false,
                };
                if timed {
                    for i in [1, 2, 5, 6] {
                        arg(i);
                    }
                }
            }
            // `_STGS(layer, from, to, ms, ease)` (`FUN_001637a0`): the alpha
            // from `from` (default: the layer's) to `to` (default 0) over `ms`,
            // while the script goes on. The fade starts at once on the shown
            // layer (`FUN_00132820` sets the live layer's); ours does that when
            // nothing else of the layer waits for a `_FADS`, else from the next
            // `_FADS` (that comes right after in the script).
            "_STGS" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                let Some(l) = self.staged[n as usize].as_mut() else { return true };
                let from = if arg(1).is_empty() { l.alpha } else { number(arg(1)).unwrap_or(0).clamp(0, 255) as u32 };
                let to = number(arg(2)).unwrap_or(0).clamp(0, 255) as u32;
                let ms = number(arg(3)).unwrap_or(0).max(0) as u32;
                l.alpha = to;
                // src: exe 0x130c80 (easing 0, 1 and any it doesn't know: straight)
                let curve = ease(number(arg(4)).unwrap_or(0));
                l.fade = (ms > 16 && from != to).then_some((from, to, ms, curve)); // src: exe 0x1637a0 (ms × 60 / 1000 frames; 0, under 17 ms, is at once in 0x132820)
                // Else from the `_FADS`, whose ops these are (marking them the `_STGS`'s puts the
                // `_FADS`'s own frames before it in the trace diff).
                self.fade_now(n as usize);
            }
            "_STMA" => {
                self.hide_window_for(number(arg(2)), None);
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                return self.stma(n as usize, number(arg(1)).unwrap_or(0));
            }
            // `_STRT(layer, from, to, ms, ease, cx, cy)` (`FUN_00162b20`): the
            // turn in 1/100 degrees goes to `to` (default 0; `_STCH` resets it)
            // from `from` (default: the layer's) over ms · 60 / 1000 frames
            // on the shown layer at once (`FUN_001326e0`: at 0 frames straight
            // to `to`), along the easing `ease` (`FUN_001325d0`, drawn by
            // `FUN_00137f50` with `FUN_00130c80`); `_STCF`'s bit 4 waits for
            // it (`FUN_001329f0`). `cx`, `cy` two shorts beside the turn (the
            // live layer's 0x72, 0x74).
            "_STRT" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                let Some(l) = self.staged[n as usize].as_mut() else { return true };
                let raw = |i: usize| a.get(i).map_or("", |v| v.trim());
                let from = if raw(1).is_empty() { Some(l.angle) } else { number(raw(1)) };
                l.angle = number(arg(2)).unwrap_or(0);
                // src: exe FUN_001326e0 (_STRT handler; an empty from keeps the current angle)
                let ms = if raw(3).is_empty() { Some(0) } else { number(raw(3)) };
                let zero = |i: usize| raw(i).is_empty() || number(raw(i)) == Some(0);
                // At once, `from` and the easing mean nothing (checked only where from = to); a centre of 0 is none.
                let done = match (ms, from) {
                    (Some(0), Some(f)) => f == l.angle,
                    // ours: a timed turn runs only without a centre; the exe's turn handler is FUN_001326e0
                    (Some(ms), Some(f)) if ms >= 17 && zero(5) && zero(6) => {
                        let start = Layer { angle: f, ..l.clone() };
                        // src: exe FUN_00130c80 (easing) drawn by FUN_00137f50; _STCF bit 4 waits for a turn (FUN_001329f0)
                        self.timed_now(n as usize, start, ms as u32, ease(number(raw(4)).unwrap_or(0)), 4)
                    }
                    _ => false,
                };
                if done {
                    for i in [1, 3, 4].into_iter().chain([5, 6].into_iter().filter(|&i| zero(i))) {
                        arg(i);
                    }
                }
            }
            // `_STBR(layer, n)` (`FUN_00162490`): the blend id (0: the normal one).
            "_STBR" => {
                let Some(l) = self.staged_layer(arg(0)) else { return arg(0).parse::<i32>().is_ok() };
                l.shader = number(arg(1)).unwrap_or(0).clamp(0, 255) as u8;
            }
            // `_STEC(layer, effect, colour)` (`FUN_00103de0`): 0 ends it, 1
            // fills the layer with `colour`, the others are `stec::EFFECTS`.
            "_STEC" => {
                let effect = number(arg(1)).unwrap_or(0);
                let colour = arg(2).strip_prefix("0x").and_then(|h| u32::from_str_radix(h, 16).ok());
                let Some(l) = self.staged_layer(arg(0)) else { return arg(0).parse::<i32>().is_ok() };
                match (effect, colour) {
                    (0, _) => l.effect = None,
                    (1, None) => return false,
                    (e, c) if super::stec::filters(e, 0).is_some() => l.effect = Some((e, c.unwrap_or(0))),
                    _ => return false,
                }
            }
            // `_STCF(layer, mask)` (`FUN_00162f90`): the script waits until the
            // shown layer's changes under `mask` (default 0x3f) have ended
            // (`FUN_001802c0`, each frame `FUN_001329f0`): 8 its `_STGS` fade,
            // 0x20 its motion unless it loops; 1, 2, 4 timed changes ours
            // doesn't make, 0x10 a field not found yet. It always returns 1
            // (wait), and the wait is checked from the next frame: a frame
            // more than the change (trace: a 1 ms fade 17 ms, 1000 ms 1016).
            "_STCF" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                let mask = number(arg(1)).unwrap_or(0x3f);
                if let Some(slot) = self.slot_layer.iter().position(|&l| l == Some(n as usize))
                    && i32::from(self.motion_bits[slot]) & mask != 0
                {
                    self.ops.push(Op::MotionWait { layer: slot as u32 });
                }
                self.ops.push(Op::Pause { ms: 17 }); // src: exe 0x162f90, a frame at 60 fps
            }
            "_STCP" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                self.priority[n as usize] = number(arg(1)).unwrap_or(0);
            }
            // `_FADS(type, ms, rule, mode)` (`FUN_00160610`): `mode` may hide the
            // text window first (`hides_window`).
            "_FADS" => {
                let ms = number(arg(1)).unwrap_or(0);
                self.hide_window_for(number(arg(3)), Some(ms));
                self.fads(number(arg(0)).unwrap_or(1), ms, number(arg(2)).unwrap_or(0));
            }
            _ => return false,
        }
        true
    }

    /// `_STCH` (`fresh`) and `_SCH2`: both `FUN_00160840`, `_SCH2` with
    /// its first parameter 0, which keeps the layer's alpha (0 on a layer
    /// with no picture), priority and turn where `_STCH` sets 255, layer
    /// × 10 and 0. Argument 18 copies another layer's settings first (its
    /// place, zoom, alpha, priority, turn, parent, clip, effect), and its
    /// children become this layer's.
    fn stch(&mut self, a: &[&str], read: &Cell<u64>, fresh: bool) -> bool {
        let arg = |i: usize| {
            mark(read, i);
            a.get(i).copied().unwrap_or("").trim()
        };
        let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)).map(|n| n as usize) else { return false };
        // src: exe 0x160840: the layer's motion stops (the shown one at once, unless argument 18
        // copies another's settings: then at the `_FADS`, which shows the copy's).
        let from = number(arg(18)).filter(|&s| (0..LAYERS as i32).contains(&s) && s as usize != n).map(|s| s as usize);
        self.motion_from[n] = None;
        if arg(1).is_empty() || from.is_none() {
            self.stop_loop(n, &[], false);
        } else {
            self.looping[n] = None;
        }
        if arg(1).is_empty() {
            self.staged[n] = None;
            return true;
        }
        // shortcut: a copy from a layer with no picture copies nothing (the exe copies its blank settings).
        let copy = from.and_then(|s| self.staged[s].clone());
        let old = copy.clone().or_else(|| self.staged[n].clone());
        let base = arg(1).to_lowercase();
        let num = |i: usize, default: f32| if arg(i).is_empty() { Some(default) } else { number(arg(i)).map(|v| v as f32) };
        // src: exe 0x160840: an x or y left out is the layer's (0 after `_STCC`), or the copied one's;
        // a zoom left out is the layer's (1000 after `_STCC`).
        let (x0, y0) = old.as_ref().map_or((0.0, 0.0), |o| (o.x, o.y));
        let zoom0 = old.as_ref().map_or(1000.0, |o| o.zoom * 1000.0);
        // src: exe 0x160840: alpha 255 by default (`_STCH`), else the layer's, 0 with no picture.
        let alpha0 = if fresh { 255.0 } else { old.as_ref().map_or(0.0, |o| o.alpha as f32) };
        let (Some(x), Some(y), Some(zoom), Some(alpha)) = (num(2, x0), num(3, y0), num(4, zoom0), num(5, alpha0)) else { return false };
        // src: exe 0x160840: arguments 9 and 10 left out are the copied layer's numbers (argument
        // 18; parsed from its picture's name), else 0; with both 0 the bare picture.
        let (oa, ob) = copy.as_ref().and_then(|l| l.expression).unwrap_or((0, 0));
        let e = |i: usize, d: u32| number(arg(i)).map_or(d, |v| v as u32);
        let expression = Some((e(9, oa), e(10, ob))).filter(|&ab| ab != (0, 0));
        // An argument left out keeps the layer's rectangle (`FUN_00160840`).
        let mut clip = old.as_ref().map_or([0; 6], |l| l.clip);
        for (i, c) in clip.iter_mut().enumerate().take(5) {
            if let Some(v) = number(arg(12 + i)) {
                *c = v;
            }
        }
        // Argument 17, the frame's colour (0 none).
        if let Some(v) = number(arg(17)) {
            clip[5] = v;
        }
        let parent = number(arg(11)).filter(|&p| (0..LAYERS as i32).contains(&p) && p as usize != n).map(|p| p as usize).or(copy.as_ref().and_then(|c| c.parent));
        let angle = number(arg(19)).unwrap_or(if fresh { 0 } else { old.as_ref().map_or(0, |o| o.angle) });
        let (stretch, stretch_off, effect) = copy.as_ref().map_or(((1.0, 1.0), false, None), |c| (c.stretch, c.stretch_off, c.effect));
        self.staged[n] = Some(Layer { base, expression, x, y, zoom: zoom / 1000.0, stretch, stretch_off, alpha: alpha.clamp(0.0, 255.0) as u32, fade: None, motion: None, parent, angle, shader: 255, effect, clip });
        if let Some(s) = from.filter(|_| copy.is_some()) {
            // A looping motion goes on in the copy (here from its start).
            if let Some(l) = self.staged[n].as_mut() {
                l.motion = self.looping[s];
            }
            self.priority[n] = self.priority[s];
            for l in self.staged.iter_mut().flatten().filter(|l| l.parent == Some(s)) {
                l.parent = Some(n);
            }
        }
        // Argument 6, else layer × 10 (`_STCH`) or the layer's (`_SCH2`).
        // src: exe 0x160840 (priority layer × 10 by default)
        if let Some(p) = number(arg(6)) {
            self.priority[n] = p;
        } else if fresh {
            self.priority[n] = n as i32 * 10;
        }
        true
    }

    /// Starts layer `n`'s staged `_STGS` fade (or sets its alpha) at once
    /// when the layer is shown and differs from what is shown only by its
    /// alpha; false when it waits for the `_FADS`.
    // shortcut: a layer with a parent or shown children (their alpha multiplies) or a loop waits for the `_FADS`.
    fn fade_now(&mut self, n: usize) -> bool {
        let Some(Layer { fade, alpha, .. }) = self.staged[n] else { return false };
        let Some(slot) = self.slot_layer.iter().position(|&l| l == Some(n)) else { return false };
        let same = |a: &Option<Layer>| a.clone().map(|l| Layer { alpha: 0, fade: None, motion: None, ..l });
        if same(&self.staged[n]) != same(&self.shown[n])
            || self.shown[n].as_ref().is_some_and(|l| l.parent.is_some() || l.motion.is_some())
            || self.shown.iter().flatten().any(|c| c.parent == Some(n))
            || self.loop_runs.iter().any(|(_, runs)| runs.iter().any(|&(c, _)| c == slot))
        {
            return false;
        }
        if let Some(l) = self.staged[n].as_mut() {
            l.fade = None;
        }
        self.shown[n].clone_from(&self.staged[n]);
        self.slots[slot] = self.resolved(n, 0);
        let layer = slot as u32;
        let Some((from, to, ms, curve)) = fade else {
            self.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: alpha * 256 / 255 });
            return true;
        };
        // src: exe 0x1637a0 (the fade, unchanged here; the lines only moved)
        let key = |ms, alpha: u32| MotionKey { ms, x: 0, y: 0, zoom: 100, angle: 0, opacity: alpha * 256 / 255, zoom_y: 100, ease: None };
        // src: exe 0x1637a0 (as above)
        self.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: 256 });
        self.ops.push(Op::SpriteMotion2 { layer, keys: vec![key(0, from), MotionKey { ease: curve, ..key(ms, to) }], end: MotionEnd::Keep });
        // src: exe 0x1637a0 (as above)
        self.motion_bits[slot] = 8;
        true
    }

    /// Runs a timed `_STZ4` or `_STRT` on layer `n` at once: from `start`
    /// (the staged layer with the change's first value) to the staged layer
    /// over `ms`, its children following; `bits` is what `_STCF` waits on.
    /// False (the change then jumps to its end at the next `_FADS`) when the
    /// layer isn't drawn as staged.
    // shortcut: a timed change on a layer not drawn as staged (none in either script), or one whose
    // fade, motion or loop (or a child's loop) has started since it was drawn, jumps to its end.
    fn timed_now(&mut self, n: usize, start: Layer, ms: u32, curve: Curve, bits: u8) -> bool {
        let Some(to) = self.staged[n].clone() else { return false };
        let Some(slot) = self.slot_layer.iter().position(|&l| l == Some(n)) else { return false };
        // src: exe FUN_00131f30 / FUN_001326e0 set the live layer, FUN_00130c80 eases it (see the commit message); the rest is ours: the layer must be shown as staged.
        let same = |l: &Layer| Layer { stretch: (0.0, 0.0), angle: 0, ..l.clone() };
        if self.shown[n].as_ref().map(same) != Some(same(&to))
            || to.fade.is_some()
            || to.motion.is_some()
            || self.motion_bits[slot] != 0
            || self.loop_runs.iter().any(|(_, runs)| runs.iter().any(|&(c, _)| self.slot_layer[c].is_some_and(|j| j == n || self.descends(j, n))))
        {
            return false;
        }
        let shown = self.shown[n].replace(start.clone());
        let at_start = self.resolved(n, 0);
        self.shown[n] = Some(to.clone());
        let at_end = self.resolved(n, 0);
        self.shown[n] = shown;
        let (Some(at_start), Some(at_end)) = (at_start, at_end) else { return false };
        let runs = self.move_states(slot, &[(0, None, at_start, start), (ms, curve, at_end, to.clone())], 0);
        if runs.iter().any(|&(c, _)| c == slot) {
            self.motion_bits[slot] = bits;
        }
        self.shown[n] = Some(to);
        for c in 0..LAYERS {
            if let Some(j) = self.slot_layer[c].filter(|&j| j == n || self.descends(j, n)) {
                self.slots[c] = self.resolved(j, 0);
            }
        }
        true
    }

    /// The staged picture on layer `arg`, if any.
    fn staged_layer(&mut self, arg: &str) -> Option<&mut Layer> {
        let n = number(arg).filter(|n| (0..LAYERS as i32).contains(n))?;
        self.staged[n as usize].as_mut()
    }

    /// Shows the staged layers. The original draws the layers with a
    /// picture and a priority above 0 in order of priority, the lower
    /// first (a stable sort, so the lower layer first on a tie;
    /// `FUN_001884f0`): the IR's layers take them in that order from the
    /// bottom (layer 39) up, and each IR layer whose picture changes is
    /// drawn again.
    fn fads(&mut self, kind: i32, ms: i32, rule: i32) {
        self.shown.clone_from(&self.staged);
        self.shown_priority.clone_from(&self.priority);
        let mut order: Vec<usize> = (0..LAYERS).filter(|&i| self.shown[i].is_some() && self.shown_priority[i] > 0).collect();
        order.sort_by_key(|&i| self.shown_priority[i]);
        let mut want = vec![None; LAYERS];
        let (mut fades, mut motions) = (vec![None; LAYERS], vec![None; LAYERS]);
        self.slot_layer = vec![None; LAYERS];
        for (rank, &i) in order.iter().enumerate() {
            let slot = LAYERS - 1 - rank;
            want[slot] = self.resolved(i, 0);
            self.slot_layer[slot] = Some(i);
            if let Some(l) = want[slot].as_mut() {
                fades[slot] = l.fade.take();
                motions[slot] = l.motion.take();
            }
        }
        // A fade or motion runs once.
        for l in self.staged.iter_mut().chain(self.shown.iter_mut()).flatten() {
            (l.fade, l.motion) = (None, None);
        }
        let mut changed: Vec<usize> = (0..LAYERS).filter(|&s| want[s] != self.slots[s] || fades[s].is_some() || motions[s].is_some()).collect();
        // A loop goes on while its layer and children keep their IR layers, unchanged; else it
        // stops, and starts again from the layer's new IR layer while the layer still runs it,
        // with the old run's clock (CONscripter's `spmotionclock`).
        for (i, runs) in std::mem::take(&mut self.loop_runs) {
            let now: Vec<(usize, usize)> = (0..LAYERS).filter_map(|c| self.slot_layer[c].filter(|&j| j == i || self.descends(j, i)).map(|j| (c, j))).collect();
            let mut was = runs.clone();
            was.sort_unstable();
            if self.looping[i].is_some() && was == now && !runs.iter().any(|(c, _)| changed.contains(c)) {
                self.loop_runs.push((i, runs));
                continue;
            }
            let restart = self.looping[i].and(self.slot_layer.iter().position(|&l| l == Some(i))).is_some_and(|c| motions[c].is_none());
            if let Some(&(from, _)) = runs.first().filter(|_| restart) {
                // Only where the loop's spmotion comes next (a fade's would take the clock).
                let to: Vec<usize> = now.iter().map(|&(d, _)| d).filter(|&d| fades[d].is_none() && want[d].as_ref().is_some_and(|l| self.look(l).is_ok())).collect();
                self.ops.extend(to.into_iter().map(|d| Op::MotionClock { layer: d as u32, from: from as u32 }));
            }
            for (c, _) in runs {
                self.ops.push(Op::SpriteMotion { layer: c as u32, keys: vec![], looped: false });
            }
            if let (Some(m), Some(c)) = (self.looping[i].take(), self.slot_layer.iter().position(|&l| l == Some(i)))
                && motions[c].is_none()
            {
                motions[c] = Some(m);
                if !changed.contains(&c) {
                    changed.push(c);
                    changed.sort_unstable();
                }
            }
        }
        let fade_ms = if kind == 1 { 0 } else { ms.max(0) as u32 }; // src: exe script_command (type 1 returns at once)
        // src: exe script_command `_FADS`: 3 frames besides the fade's own, even when nothing changed
        // (FUN_0012e0b0 asks for the pictures, a frame's countdown, the fade started once FUN_001b6920
        // has them all, a frame after it), here before it. Frames spent loading (the trace: 2–10 more
        // where a picture is new) are left out.
        self.ops.push(Op::Pause { ms: 50 });
        let from: Vec<_> = std::mem::replace(&mut self.motion_from, vec![None; LAYERS]);
        let mask = (kind == 2).then(|| format!("pic/rule_{rule:02}.png"));
        self.ops.push(Op::BeginTransition { effect: format!("_FADS({kind},{ms},{rule})"), fade_ms, mask, style: TransitionStyle::Fade });
        for slot in changed {
            self.motion_bits[slot] = 0;
            self.slots[slot] = want[slot].clone();
            let layer = slot as u32;
            match (want[slot].take(), fades[slot]) {
                (Some(l), Some((from, to, ms, curve))) => {
                    // Opaque, the motion's keys giving the alpha (they multiply it).
                    self.place(layer, &Layer { alpha: 255, ..l });
                    let key = |ms, alpha: u32| MotionKey { ms, x: 0, y: 0, zoom: 100, angle: 0, opacity: alpha * 256 / 255, zoom_y: 100, ease: None };
                    self.ops.push(Op::SpriteMotion2 { layer, keys: vec![key(0, from), MotionKey { ease: curve, ..key(ms, to) }], end: MotionEnd::Keep });
                    self.motion_bits[slot] = 8;
                }
                (Some(l), None) => self.place(layer, &l),
                (None, _) => {
                    self.ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: None, keep_filters: false }], fade_ms: 0 });
                    self.blends[slot] = BLEND_NORMAL;
                    self.clips[slot] = None;
                    self.shaking[slot] = [None; 2]; // csp stops its spshake
                }
            }

        }
        // After every place: a parent's motion moves its children too.
        let here = self.source_map.last().cloned();
        for (slot, m) in motions.into_iter().enumerate() {
            if let Some(m) = m {
                let stma = self.slot_layer[slot].and_then(|i| from[i].clone());
                if let Some(at) = stma.clone() {
                    self.source_map.push((self.ops.len(), at));
                }
                self.run_motion(slot, m);
                if let (Some(_), Some((_, at))) = (stma, here.clone()) {
                    self.source_map.push((self.ops.len(), at));
                }
            }
        }
        self.sync_quakes(u32::MAX);
        self.ops.push(Op::EndTransition);
    }

    /// `_STMA(layer, m)` (`FUN_00199620`): motion `m` of the exe runs on the
    /// layer at once (the shown one and the staged copy), while the script
    /// goes on; 0 stops it. Its keys hold absolute values (x, y in script
    /// units, alpha, the `_STZ4` zoom in ‰); a field a key leaves out
    /// keeps the one before. The layer stays at the last key's.
    fn stma(&mut self, n: usize, m: i32) -> bool {
        if m <= 0 {
            // src: exe 0x199620: motion 0 on both copies; the layer stays as the motion left it.
            // shortcut: a motion that doesn't loop runs on to its end (it has mostly ended by then).
            self.stop_loop(n, &[], true);
            if let Some(l) = self.staged[n].as_mut() {
                l.motion = None;
            }
            self.motion_from[n] = None;
            return true;
        }
        if m as usize > self.g.motions.len() {
            return false;
        }
        match self.slot_layer.iter().position(|&l| l == Some(n)) {
            Some(slot) if self.shown[n].is_some() => self.run_motion(slot, m as usize),
            _ => match self.staged[n].as_mut() {
                Some(l) => {
                    l.motion = Some(m as usize);
                    self.motion_from[n] = self.source_map.last().map(|(_, at)| at.clone());
                }
                None => return false,
            },
        }
        true
    }

    /// Emits motion `m` on IR layer `slot` (an `spmotion` from the
    /// layer's place), and moves the layer's state to its last key. Its
    /// children (and theirs) follow it, each with an `spmotion` of its
    /// own from where the parent's keys place it.
    // Each key is reached along the easing (0x04) of the last key before it that sets one (`FUN_00197710`).
    // A field a key passes through (-32766) is eased once across it: `Motion::through_resolved` cuts those keys' ways.
    // Mode 1's cubic (0x06 = 1) comes cut into straight steps by `through_resolved` too.
    // shortcut: priority (0x3C), expression (0x32, 0x34) and
    // the other fields are not followed. Mode 2 holds: a key 1 ms early. A loop (`FUN_001999a0`) runs until stopped.
    fn run_motion(&mut self, slot: usize, m: usize) {
        let Some(i) = self.slot_layer[slot] else { return };
        let (Some(start), Some(shown)) = (self.slots[slot].clone(), self.shown[i].clone()) else { return };
        // The layer as drawn and as shown (its own place) at each key.
        let (mut at, mut own) = (start.clone(), shown.clone());
        let mut states = Vec::new();
        let motion = &self.g.motions[m - 1];
        // A field drives the layer only under its bits of the motion's mask.
        let on = |bits: u64, v: Option<i16>| v.filter(|_| motion.mask & bits != 0);
        // The way to each key is the mode and easing last set before it (`FUN_00197710`); before the first key the layer stays.
        let (mut mode, mut easing) = (None, None);
        for (j, k) in motion.keys.iter().enumerate() {
            let hold = if j == 0 { k.time_ms > 0 } else { mode == Some(2) && k.time_ms > motion.keys[j - 1].time_ms + 1 };
            if hold {
                states.push((k.time_ms - 1, None, at.clone(), own.clone()));
            }
            let curve = if hold { None } else { easing.flatten() };
            mode = k.mode().or(mode);
            easing = k.easing().filter(|&e| e != Key::THROUGH).map(|e| ease(e.into())).or(easing);
            for l in [&mut at, &mut own] {
                if let Some(x) = on(Motion::X, k.x()) {
                    l.x = f32::from(x);
                }
                if let Some(y) = on(Motion::Y, k.y()) {
                    l.y = f32::from(y);
                }
                if let Some(a) = on(Motion::ALPHA, k.alpha()) {
                    l.alpha = a.clamp(0, 255) as u32;
                }
                let (zx, zy) = (on(Motion::ZOOM_X, k.zoom().0), on(Motion::ZOOM_Y, k.zoom().1));
                // ours: moved unchanged from run_motion into move_states (the motion keys set the zoom)
                l.stretch = (zx.map_or(l.stretch.0, |z| f32::from(z) / 1000.0), zy.map_or(l.stretch.1, |z| f32::from(z) / 1000.0));
                if let Some(t) = k.turn().filter(|_| motion.mask & Motion::TURN != 0) {
                    l.angle = t;
                }
            }
            states.push((k.time_ms, curve, at.clone(), own.clone()));
        }
        if states.is_empty() {
            return;
        }
        let loop_from = motion.loop_from;
        let runs = self.move_states(slot, &states, loop_from);
        if runs.iter().any(|&(c, _)| c == slot) {
            self.motion_bits[slot] = 0x20; // a child's own motion stays as it was
        }
        if loop_from != 0 {
            self.looping[i] = Some(m);
            self.loop_runs.push((i, runs));
        }
    }

    /// Moves layer `slot_layer[slot]` (shown as `slots[slot]`) and the
    /// children that follow it through `states` (ms, the easing to it, the
    /// layer as drawn and as shown) with an `spmotion` each, from what is
    /// drawn; its state then jumps to the last one. The IR layers written,
    /// with the layer each draws.
    fn move_states(&mut self, slot: usize, states: &[(u32, Curve, Layer, Layer)], loop_from: u32) -> Vec<(usize, usize)> {
        let Some(i) = self.slot_layer[slot] else { return vec![] };
        let (Some(start), Some(shown), Some((_, _, at, _))) = (self.slots[slot].clone(), self.shown[i].clone(), states.last().cloned()) else { return vec![] };
        let mut moves = vec![(slot, start, states.iter().map(|(ms, e, l, _)| (*ms, *e, l.clone())).collect::<Vec<_>>())];
        for c in (0..LAYERS).filter(|&c| c != slot) {
            let (Some(j), Some(child)) = (self.slot_layer[c], self.slots[c].clone()) else { continue };
            if !self.descends(j, i) {
                continue;
            }
            let keys = states
                .iter()
                .filter_map(|(ms, e, _, own)| {
                    self.shown[i] = Some(own.clone());
                    Some((*ms, *e, self.resolved(j, 0)?))
                })
                .collect();
            moves.push((c, child, keys));
        }
        self.shown[i] = Some(shown);
        // A motion replaces the layer's loop: stopped where the new one doesn't run.
        let ran: Vec<usize> = moves.iter().map(|m| m.0).collect();
        self.stop_loop(i, &ran, false);
        let mut runs = Vec::new();
        for (c, start, keys) in moves {
            if self.emit_motion(c, &start, &keys, loop_from) {
                runs.push((c, self.slot_layer[c].unwrap_or(i)));
            }
        }
        // shortcut: the layer's state jumps to the motion's end; a picture drawn again mid-motion starts there.
        let end = |l: &mut Layer| (l.x, l.y, l.alpha, l.stretch, l.angle) = (at.x, at.y, at.alpha, at.stretch, at.angle);
        for l in [self.slots[slot].as_mut(), self.shown[i].as_mut(), self.staged[i].as_mut()].into_iter().flatten() {
            end(l);
        }
        runs
    }

    /// An `spmotion` on IR layer `slot` from `start` (as it is drawn)
    /// through each key's state.
    /// `loop_from` as `Motion`'s: from it on the motion starts again after
    /// its last key. Whether it was written.
    fn emit_motion(&mut self, slot: usize, start: &Layer, states: &[(u32, Curve, Layer)], loop_from: u32) -> bool {
        let Ok((_, (x0, y0), z0)) = self.look(start) else { return false };
        // Under 10 % the zoom placed is far from the exact one (1‰ → 1 %, not 0.05): from what was placed.
        let base = |z: f32| if z.abs() < 10.0 { percent(z) as f32 } else { z };
        let z0 = (base(z0.0), base(z0.1));
        let mut keys = Vec::new();
        for (ms, curve, at) in states {
            let Ok((_, (x, y), z)) = self.look(at) else { return false };
            // spmotion's zoom multiplies the place's (spzoom), about its centre.
            let rel = |z: f32, z0: f32| if z0 == 0.0 { 100 } else { (100.0 * z.abs() / z0.abs()).round() as u32 };
            keys.push(MotionKey {
                ms: *ms,
                x: (x - x0).round() as i32,
                y: (y - y0).round() as i32,
                zoom: rel(z.0, z0.0),
                // Counter-clockwise, added to the place's (clockwise 1/100 degrees).
                angle: ((start.angle - at.angle) as f32 / 100.0).round() as i32,
                opacity: at.alpha * 256 / 255,
                zoom_y: rel(z.1, z0.1),
                ease: *curve,
            });
        }
        if keys.is_empty() {
            return false;
        }
        // shortcut: spmotion takes 64 keys; past that the middle ones go (7 calls), the last stays.
        if keys.len() > 64 {
            keys.drain(63..keys.len() - 1);
        }
        // src: exe 0x197b80: past its length the time goes back to loop_from (1, or one at or
        // past the length: the start) and on to the end again.
        // shortcut: from the key nearest loop_from (16 of the 222 loops from a time between keys).
        let last = keys.last().map_or(0, |k| k.ms);
        let end = match loop_from {
            0 => MotionEnd::Keep,
            1 => MotionEnd::Loop,
            t if t >= last => MotionEnd::Loop,
            t => match keys.iter().enumerate().min_by_key(|(_, k)| k.ms.abs_diff(t)).map_or(0, |(j, _)| j) {
                0 => MotionEnd::Loop,
                j if j + 1 >= keys.len() => MotionEnd::Loop,
                j => MotionEnd::LoopFrom(j),
            },
        };
        // Opaque, the keys giving the alpha (they multiply it).
        if start.alpha < 255 {
            self.ops.push(Op::SpriteOpacity { layer: Some(slot as u32), opacity: 256 });
        }
        self.ops.push(Op::SpriteMotion2 { layer: slot as u32, keys, end });
        true
    }

    /// Stops the loop layer `i`'s motion runs (`_STMA(i, 0)`, `_STCH`, a
    /// new motion), but on the IR layers in `keep`. The exe leaves the
    /// layer where the motion has it.
    /// `here` stops it where it is (`_STMA(n, 0)`); else back at its place.
    // shortcut: `_STCH` and a new motion put the sprite back at its place (it is placed or moved anew next).
    fn stop_loop(&mut self, i: usize, keep: &[usize], here: bool) {
        self.looping[i] = None;
        let Some(at) = self.loop_runs.iter().position(|r| r.0 == i) else { return };
        for (slot, _) in self.loop_runs.remove(at).1 {
            if !keep.contains(&slot) {
                self.ops.push(match here {
                    true => Op::SpriteMotion2 { layer: slot as u32, keys: vec![], end: MotionEnd::Keep },
                    false => Op::SpriteMotion { layer: slot as u32, keys: vec![], looped: false },
                });
            }
        }
    }

    /// Whether layer `j` is placed in layer `i`, or in one placed in it.
    /// `_SQK2` (kind 1, `cmd_SQK2` 0x191540), `_SHK2` (kind 2, 0x191900),
    /// `_SSW2` (kind 3, 0x191cf0): a quake on layer `a[0]` (< 0: each shown
    /// one; `_SSW2`: nothing) in the exe's slot 0 (kind 1) or 1 (0x4514f0).
    /// Amplitudes 0 clear the slot. shortcut: a finite quake starts while
    /// skipping too (the exe's `_SQK2` doesn't, FUN_0015afb0).
    fn quake_start(&mut self, kind: u8, n: impl Fn(usize, i32) -> Option<i32>) -> bool {
        let Some(layer) = n(0, 0) else { return false };
        if layer >= LAYERS as i32 || (kind == 3 && layer < 0) {
            return true; // src: exe 0x191540, 0x191cf0 return
        }
        let (slot, quake, follow) = match kind {
            1 => {
                let (Some(x), Some(y), Some(hold), Some(ms), Some(delay), Some(follow), Some(fade_in)) = (n(1, 15), n(2, 15), n(3, 0), n(4, -1), n(5, 0), n(6, 0), n(7, 0)) else { return false };
                // src: exe 0x191540: hold ms < 0: 1 frame, < 17: 2, else ms*3/50 + 1
                let hold = if hold < 0 { 1 } else if hold < 17 { 2 } else { quake::frames(hold) + 1 };
                let q = (x != 0 || y != 0).then(|| Quake { kind: Kind::Quake { x, y, hold, fade_in: quake::frames(fade_in) }, frames: quake::length(ms), delay: quake::frames(delay) });
                (0, q, follow != 0)
            }
            2 => {
                let (Some(x), Some(y), Some(period), Some(ms), Some(delay), Some(decay), Some(fade_in), Some(flicker)) = (n(1, 0), n(2, 15), n(3, 300), n(4, -1), n(5, 0), n(6, 0), n(7, 0), n(8, 1)) else { return false };
                let period = Some(quake::frames(period)).filter(|&p| p > 0).unwrap_or(60); // src: exe 0x13c970 (period 0: 60 frames)
                let kind = Kind::Sway { x, y, period, decay: decay != 0, fade_in: quake::frames(fade_in), flicker };
                (1, (x != 0 || y != 0).then(|| Quake { kind, frames: quake::length(ms), delay: quake::frames(delay) }), false)
            }
            _ => {
                let (Some(amin), Some(amax), Some(pmin), Some(pmax), Some(gain), Some(ms), Some(delay)) = (n(1, 7), n(2, 15), n(3, 1000), n(4, 2000), n(5, 1), n(6, -1), n(7, 0)) else { return false };
                let kind = Kind::Drift { amp: (amin, amax), period: (quake::frames(pmin), quake::frames(pmax)), gain };
                (1, (amax != 0 && gain != 0).then(|| Quake { kind, frames: quake::length(ms), delay: quake::frames(delay) }), false)
            }
        };
        let first = self.quake_id + 1;
        // src: exe 0x191540: < 0 is every shown layer
        for i in (0..LAYERS).filter(|&i| if layer < 0 { self.shown[i].is_some() } else { i as i32 == layer }) {
            self.quakes[i][slot] = quake.as_ref().map(|q| {
                self.quake_id += 1;
                let id = self.quake_id;
                Placed { id, effect: quake::effect(q, id), forever: q.frames.is_none(), follow: follow && i >= 2 }
            });
        }
        self.sync_quakes(first);
        true
    }

    /// `_SQS2` (slot 0), `_SHS2`/`_SSS2` (slot 1) (0x163680): the slot's
    /// quake on layer `n` (< 0: all 40) stops, its offset 0 at once.
    fn quake_stop(&mut self, n: i32, slot: usize) {
        for q in self.quakes.iter_mut().enumerate().filter(|&(i, _)| n < 0 || i as i32 == n) {
            q.1[slot] = None;
        }
        self.sync_quakes(u32::MAX);
    }

    /// The quake layer `j`'s exe slot `s` draws: its own, or in slot 0 its
    /// topmost quaking ancestor's (0x134440: a layer with a quake of its
    /// own shakes, a child by its ancestor's), and with follow layer 0's
    /// (1's when 0 isn't shown; neither: none).
    fn quake_of(&self, j: usize, s: usize) -> Option<Placed> {
        let mut q = self.quakes[j][s].as_ref()?;
        if s == 0 {
            let mut a = j;
            for _ in 0..LAYERS {
                let Some(p) = self.shown[a].as_ref().and_then(|l| l.parent) else { break };
                if let Some(pq) = self.quakes[p][0].as_ref() {
                    q = pq;
                }
                a = p;
            }
            if q.follow {
                let lead = (0..2).find(|&l| self.shown[l].is_some())?;
                q = self.quakes[lead][0].as_ref()?;
            }
        }
        Some(q.clone())
    }

    /// Brings each IR layer's `spshake` slots to its layer's quakes: the
    /// quakes from id `fresh` up start (with their delay); one that moves
    /// to another IR layer at a `_FADS` starts there again if it runs for
    /// ever (ours: from its start, no delay), else stops.
    fn sync_quakes(&mut self, fresh: u32) {
        for c in 0..LAYERS {
            for s in 0..2 {
                let want = self.slot_layer[c].and_then(|j| self.quake_of(j, s));
                if self.shaking[c][s] == want.as_ref().map(|q| q.id) {
                    continue;
                }
                let effect = want.filter(|q| q.id >= fresh || q.forever).and_then(|q| {
                    let mut e = q.effect?;
                    if q.id < fresh
                        && let ScreenEffect::ShakeList { delay_ms, .. } = &mut e
                    {
                        *delay_ms = 0;
                    }
                    Some((q.id, e))
                });
                if effect.is_none() && self.shaking[c][s].is_none() {
                    continue;
                }
                self.shaking[c][s] = effect.as_ref().map(|(id, _)| *id);
                self.ops.push(Op::SpriteShake { layer: c as u32, slot: s as u32 + 1, effect: effect.map(|(_, e)| e) });
            }
        }
    }

    fn descends(&self, mut j: usize, i: usize) -> bool {
        for _ in 0..LAYERS {
            match self.shown[j].as_ref().and_then(|l| l.parent) {
                Some(p) if p == i => return true,
                Some(p) => j = p,
                None => return false,
            }
        }
        false
    }

    /// Where and how `l` is drawn: the image, its middle on the screen and
    /// its zoom (%, as [`percent`] rounds it for `spzoom`); or the name when
    /// the picture is unknown.
    fn look(&self, l: &Layer) -> Result<Look, String> {
        let name = self.name(l);
        let picture = self.g.pictures.get(&name);
        let fill = colour(&name).filter(|_| picture.is_none_or(|p| p.flags & 3 == 3));
        Ok(match (picture, fill) {
            (_, Some(rgb)) => (format!(">{SCREEN_W},{SCREEN_H},#{rgb:06x}"), (SCREEN_W as f32 / 2.0, SCREEN_H as f32 / 2.0), (100.0, 100.0)),
            (Some(p), None) => {
                // The record's centre goes to the script's x, y (480 × 272
                // for the screen); zoom about it. Records are 1280 × 724
                // for the screen.
                let (sx, sy) = (SCREEN_W as f32 / 480.0, SCREEN_H as f32 / 272.0); // src: exe 0x189ad0
                let (zx, zy) = (l.zoom * l.drawn_stretch().0, l.zoom * l.drawn_stretch().1);
                let mx = l.x * sx + (p.width / 2.0 - p.centre.0) * zx;
                let my = l.y * sy + (p.height / 2.0 - p.centre.1) * zy * SCREEN_H as f32 / STAGE_H;
                // The file at the record's size (its art is mostly 2×).
                let (tw, th) = self.g.textures.get(&name).map_or((p.width * 2.0, p.height * 2.0), |&(w, h)| (w as f32, h as f32));
                let zoom = |rec: f32, tex: f32, z: f32| 100.0 * rec / tex * z;
                (format!("pic/{}{name}.{}", if self.own.contains(&name) { "en/" } else { "" }, self.g.pic_ext), (mx, my), (zoom(p.width, tw, zx), zoom(p.height * SCREEN_H as f32 / STAGE_H, th, zy)))
            }
            (None, None) => return Err(name),
        })
    }

    /// Layer `i` as shown, a child placed in its parent (`FUN_001892b0`,
    /// `FUN_00189190`): its x, y are in the parent's picture, from its
    /// record's top-left, zoomed with the parent (and by ½, ¼, ⅛ for a
    /// parent sprite's `_L_`, `_M_`, `_S_` shot) and turned with it about
    /// the parent's place by the angles summed up the chain. Its own picture
    /// turns by its own angle only (`FUN_001884f0` copies the layer's angle
    /// alone into what `FUN_00189ad0` draws).
    fn resolved(&self, i: usize, depth: usize) -> Option<Layer> {
        let mut l = self.shown[i].clone()?;
        let Some(p) = l.parent.filter(|_| depth < LAYERS) else { return Some(l) };
        let Some(parent) = self.resolved(p, depth + 1) else { return Some(l) };
        let Some(record) = self.g.pictures.get(&self.name(&parent)) else { return Some(l) };
        let shot = [1.0, 0.5, 0.25, 0.125].get(usize::from(record.shot)).copied().unwrap_or(1.0);
        let (zx, zy) = (parent.zoom * parent.stretch.0 * shot, parent.zoom * parent.stretch.1 * shot);
        // Script units: a record unit is 0.375 of one (1280 for the screen's 480).
        let (u, v) = (zx * (l.x - record.centre.0 * 0.375 / shot), zy * (l.y - record.centre.1 * 0.375 / shot));
        // Turned with the parent, y down (`FUN_0018c480` sums the angles).
        let (sin, cos) = (self.turn(p, depth + 1) as f32 * std::f32::consts::PI / 18000.0).sin_cos();
        l.x = parent.x + u * cos - v * sin;
        l.y = parent.y + u * sin + v * cos;
        // Its alpha times its parent's own (`FUN_001884f0`).
        l.alpha = l.alpha * self.shown[p].as_ref().map_or(255, |q| q.alpha) / 255;
        // shortcut: a parent that is a child with `stretch_off` places its children without its own zoom; the exe (`FUN_00138a70`) keeps it.
        l.stretch = (l.drawn_stretch().0 * zx, l.drawn_stretch().1 * zy);
        l.stretch_off = false;
        l.clip = parent.clip;
        l.parent = None;
        Some(l)
    }

    /// Layer `i`'s angle summed up its parents (`FUN_0018c480`).
    fn turn(&self, i: usize, depth: usize) -> i32 {
        let Some(l) = &self.shown[i] else { return 0 };
        l.angle + l.parent.filter(|_| depth < LAYERS).map_or(0, |p| self.turn(p, depth + 1))
    }

    /// The picture `l` draws: with its expression, or the bare picture if
    /// it has none (`_00_00` on a background).
    fn name(&self, l: &Layer) -> String {
        match l.expression {
            Some((a, b)) => Some(format!("{}_{a:02}_{b:02}", l.base)).filter(|n| self.g.pictures.contains_key(n)).unwrap_or_else(|| l.base.clone()),
            None => l.base.clone(),
        }
    }

    /// Draws `l` on IR layer `layer`.
    fn place(&mut self, layer: u32, l: &Layer) {
        let (image, (cx, cy), (zx, zy)) = match self.look(l) {
            Ok(v) => v,
            Err(name) => {
                self.note(format!("picture not in the exe's table: {name}"));
                self.ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: None, keep_filters: false }], fade_ms: 0 });
                return;
            }
        };
        self.ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: Some(SpritePlacement { image, x: 0, y: 0 }), keep_filters: false }], fade_ms: 0 });
        self.ops.push(Op::SpriteMove { layer, x: cx.round() as i32, y: cy.round() as i32, ms: 0 });
        // Clockwise, as the child-layer math turns (spzoom's angle is counter-clockwise).
        self.ops.push(Op::SpriteAngle { layer, degrees: (-l.angle as f32 / 100.0).round() as i32, zoom: (percent(zx), percent(zy)) });
        if l.alpha < 255 {
            self.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: l.alpha * 256 / 255 });
        }
        let (blend, filter) = shader(self.g.shaders.get(l.shader as usize).copied().unwrap_or(0));
        // spblend stays through lsp and goes with csp.
        if blend != self.blends[layer as usize] {
            self.ops.push(Op::SpriteBlend { layer, mode: blend });
            self.blends[layer as usize] = blend;
        }
        // spclip stays through lsp: a full-screen one draws everywhere again.
        let clip = clip(l.clip);
        if clip != self.clips[layer as usize] {
            let (rect, feather, colour) = clip.unwrap_or(((0, 0, SCREEN_W as i32, SCREEN_H as i32), 0, None));
            self.ops.push(if feather > 0 { Op::SpriteClipFeather { layer, rect, feather, colour } } else { Op::SpriteClip { layer, rect } });
            self.clips[layer as usize] = clip;
        }
        // The effect makes the texture the shader then draws.
        let effect = l.effect.and_then(|(e, c)| super::stec::filters(e, c)).unwrap_or_default();
        let filters: Vec<SpriteFilter> = effect.into_iter().chain(filter).collect();
        if !filters.is_empty() {
            self.ops.push(Op::SpriteFilters { layer, filters });
        }
    }
}

/// A zoom in % as `spzoom` takes it: whole, and not 0 unless it is (a
/// line drawn at 1‰ across, `_STZ4(28,1,584,…)`, grows by a motion).
// shortcut: under 10 % the rounding is coarse (3 % may be 2.5 or 3.49); a finer spzoom would need an engine change.
fn percent(z: f32) -> i32 {
    let r = z.round();
    (if r == 0.0 && z != 0.0 { z.signum() } else { r }) as i32
}

/// A layer's `_STCH` rectangle on this screen (`FUN_00189ad0`): a
/// scissor on the 1920 × 1080 screen, inside its frame (the frame's width
/// off each side, across if narrower than the screen and down if lower).
// shortcut: the frame (argument 17's colour: 1, 7 black, 2 white, …) is not drawn, and a turned layer's rectangle isn't turned.
/// A screen rectangle (x, y, w, h), its feather in px and the colour it
/// fades from.
type Clip = ((i32, i32, i32, i32), u32, Option<u32>);

/// A panel's screen rectangle and feather. With no frame colour
/// (argument 17 0) the frame is a soft edge: the draw (`FUN_00189ad0`)
/// makes the layer's mask (blend 0 into its target) from the inner
/// rectangle and `BORDER_PARTS`' `alphagradation*` pieces, a smooth ramp
/// over the frame width from the rectangle's edge in. With a colour the
/// picture is cut at the frame's inner edge (the scissor) and the `_inv`
/// pieces are drawn over it in the colour, inside the cut: the same ramp
/// from the colour. An axis as wide as the screen has no frame; its sides
/// are pushed out of the feather.
fn clip([x, y, w, h, frame, colour]: [i32; 6]) -> Option<Clip> {
    // src: exe 0x189ad0 (a scissor in 1920 × 1080 pixels)
    if w == 0 || h == 0 || (w, h) == (1920, 1080) {
        return None;
    }
    let (fx, fy) = (if w < 1920 { frame.max(0) } else { 0 }, if h < 1080 { frame.max(0) } else { 0 });
    let px = |v: i32| (v as f32 * SCREEN_W as f32 / 1920.0).round() as i32;
    if frame <= 0 {
        return Some(((px(x), px(y), px(w), px(h)), 0, None));
    }
    if colour != 0 {
        let inner = (px(x + fx), px(y + fy), px(w - 2 * fx).max(0), px(h - 2 * fy).max(0));
        return Some((inner, px(frame).max(1) as u32, Some(frame_colour(colour))));
    }
    let (ex, ey) = (if fx == 0 { frame } else { 0 }, if fy == 0 { frame } else { 0 });
    Some(((px(x - ex), px(y - ey), px(w + 2 * ex), px(h + 2 * ey)), px(frame).max(1) as u32, None))
}

/// A frame colour (`_STCH` argument 17) as 0xRRGGBB.
// src: exe 0x189ad0 (the switch before the frame pieces: alpha, r, g, b to FUN_001b03e0; case 1's r and g read as 0)
fn frame_colour(n: i32) -> u32 {
    let rgb = |r: f32, g: f32, b: f32| ((r * 255.0) as u32) << 16 | ((g * 255.0) as u32) << 8 | (b * 255.0) as u32;
    match n {
        2 => 0xffffff,
        3 => rgb(1.0, 0.0, 0.0),
        4 => rgb(f32::from_bits(0x3e34b4b5), f32::from_bits(0x3d989899), 0.0),
        5 => rgb(f32::from_bits(0x3f76f6f7), f32::from_bits(0x3f74f4f5), f32::from_bits(0x3f68e8e9)),
        6 => rgb(1.0, f32::from_bits(0x3f68e8e9), f32::from_bits(0x3f52d2d3)),
        _ => 0, // 1, and the default (7)
    }
}

/// The `F`, `G` and `C` flags the scripts name, as IR variables.
pub(super) fn variables(g: &Game) -> HashMap<String, Var> {
    let mut names = BTreeSet::new();
    for (_, commands) in &g.scripts {
        for c in commands {
            for a in &c.args {
                let a = a.trim();
                let digits = a.get(1..).unwrap_or("");
                if matches!(a.as_bytes().first(), Some(b'F' | b'G' | b'C')) && !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                    names.insert(a.to_string());
                }
            }
        }
    }
    let mut next = FIRST_LOCAL;
    let cg = cg_flags(&g.scripts).into_iter().map(|(n, v)| (format!("_STCG {n}"), v));
    names
        .into_iter()
        .map(|n| {
            let var = if let Some(g) = n.strip_prefix('G') {
                Var { global: true, index: g.parse().unwrap() }
            } else {
                next += 1;
                Var { global: false, index: next - 1 }
            };
            (n, var)
        })
        .chain(cg)
        .collect()
}

/// `_SELR` option `i`'s bit in the choice mask: offered, or shown greyed
/// (16 up; options are 0-15).
fn choice_bit(i: i32, grey: bool) -> i32 {
    1 << (i + if grey { 16 } else { 0 })
}

pub fn compile(g: &Game) -> Compiled {
    let vars = variables(g);
    let mask = choice_mask(&vars);
    compile_copy(g, "", vars, mask, &HashSet::new())
}

/// The local after the game's flags: [`Lowering::choice_mask`].
// ours
pub(super) fn choice_mask(vars: &HashMap<String, Var>) -> Var {
    Var { global: false, index: vars.values().filter(|v| !v.global).map(|v| v.index + 1).max().unwrap_or(FIRST_LOCAL) }
}

/// One copy of the scripts: its labels `tm_<prefix><script>…`, the
/// pictures in `own` from `pic/en/`.
pub(super) fn compile_copy(g: &Game, prefix: &str, vars: HashMap<String, Var>, choice_mask: Var, own: &HashSet<String>) -> Compiled {
    let scene_labels = g.scenes.iter().zip(&g.scripts).map(|(&n, (name, _))| (n, label(&format!("{prefix}{name}"), None))).collect();
    let mut l = Lowering {
        g,
        ops: Vec::new(),
        notes: BTreeMap::new(),
        metrics: Metrics::default(),
        vars,
        scene_labels,
        shown: vec![None; LAYERS],
        staged: vec![None; LAYERS],
        // src: exe 0x160840, 0x162a40 (layer × 10)
        priority: (0..LAYERS as i32).map(|i| i * 10).collect(),
        shown_priority: (0..LAYERS as i32).map(|i| i * 10).collect(),
        slots: vec![None; LAYERS],
        slot_layer: vec![None; LAYERS],
        blends: vec![BLEND_NORMAL; LAYERS],
        clips: vec![None; LAYERS],
        page_end: false,
        window_faded: false,
        voice: None,
        held: None,
        page_notes: Vec::new(),
        se_channels: HashMap::new(),
        options: Vec::new(),
        open_ifs: Vec::new(),
        choice_mask,
        choices: 0,
        own,
        english: !prefix.is_empty(),
        question: false,
        looping: vec![None; LAYERS],
        loop_runs: Vec::new(),
        source_map: Vec::new(),
        motion_from: vec![None; LAYERS],
        motion_bits: vec![0; LAYERS],
        quakes: vec![[None, None]; LAYERS],
        shaking: vec![[None; 2]; LAYERS],
        quake_id: 0,
    };
    let mut starts = Vec::new();
    for (name, commands) in &g.scripts {
        let name = &format!("{prefix}{name}");
        starts.push(l.ops.len());
        l.ops.push(Op::Label(label(name, None)));
        l.open_ifs.clear();
        for (index, c) in commands.iter().enumerate() {
            l.source_map.push((l.ops.len(), conv_core::ir::SourceLoc { file: name.clone(), index }));
            l.metrics.motion(name, motion_ms(c));
            if let Some((_, why)) = LEFT_OUT.iter().find(|(k, _)| *k == c.opcode()) {
                l.metrics.left_out(c.opcode(), why);
                continue;
            }
            let read = Cell::new(0);
            let done = l.command(name, c, &read);
            if !done {
                l.note(format!("not converted: {}", c.opcode()));
            }
            let reasons = if done { unread(c, read.get()) } else { vec!["not converted".into()] };
            l.metrics.call(c.opcode(), c.args.iter().filter(|a| !a.trim().is_empty()).count(), reasons);
        }
    }
    Compiled { ops: l.ops, notes: l.notes, metrics: l.metrics, starts, source_map: l.source_map }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typemoon::script;

    /// FUN_00130c80(t, 0, 1, 16, code) for t 0-16, run in the exe:
    /// `research/typemoon/ease_exe.py`.
    #[allow(clippy::approx_constant)] // the exe's numbers, as it gives them
    const EXE_EASES: &[(i32, [f64; 17])] = &[
    (0, [0.0, 0.0625, 0.125, 0.1875, 0.25, 0.3125, 0.375, 0.4375, 0.5, 0.5625, 0.625, 0.6875, 0.75, 0.8125, 0.875, 0.9375, 1.0]),
    (1, [0.0, 0.0625, 0.125, 0.1875, 0.25, 0.3125, 0.375, 0.4375, 0.5, 0.5625, 0.625, 0.6875, 0.75, 0.8125, 0.875, 0.9375, 1.0]),
    (2, [0.0, 0.00390625, 0.015625, 0.03515625, 0.0625, 0.09765625, 0.140625, 0.1914062, 0.25, 0.3164062, 0.390625, 0.4726562, 0.5625, 0.6601562, 0.765625, 0.8789062, 1.0]),
    (3, [0.0, 0.1210938, 0.234375, 0.3398438, 0.4375, 0.5273438, 0.609375, 0.6835938, 0.75, 0.8085938, 0.859375, 0.9023438, 0.9375, 0.9648438, 0.984375, 0.9960938, 1.0]),
    (4, [0.0, 0.0078125, 0.03125, 0.0703125, 0.125, 0.1953125, 0.28125, 0.3828125, 0.5, 0.6171875, 0.71875, 0.8046875, 0.875, 0.9296875, 0.96875, 0.9921875, 1.0]),
    (5, [0.0, 0.1171875, 0.21875, 0.3046875, 0.375, 0.4296875, 0.46875, 0.4921875, 0.5, 0.5078125, 0.53125, 0.5703125, 0.625, 0.6953125, 0.78125, 0.8828125, 1.0]),
    (6, [0.0, 0.0002441406, 0.001953125, 0.006591797, 0.015625, 0.03051758, 0.05273438, 0.08374023, 0.125, 0.1779785, 0.2441406, 0.3249512, 0.421875, 0.536377, 0.6699219, 0.8239746, 1.0]),
    (7, [0.0, 0.1760254, 0.3300781, 0.463623, 0.578125, 0.6750488, 0.7558594, 0.8220215, 0.875, 0.9162598, 0.9472656, 0.9694824, 0.984375, 0.9934082, 0.9980469, 0.9997559, 1.0]),
    (8, [0.0, 0.0009765625, 0.0078125, 0.02636719, 0.0625, 0.1220703, 0.2109375, 0.3349609, 0.5, 0.6650391, 0.7890625, 0.8779297, 0.9375, 0.9736328, 0.9921875, 0.9990234, 1.0]),
    (9, [0.0, 0.1650391, 0.2890625, 0.3779297, 0.4375, 0.4736328, 0.4921875, 0.4990234, 0.5, 0.5009766, 0.5078125, 0.5263672, 0.5625, 0.6220703, 0.7109375, 0.8349609, 1.0]),
    (10, [0.0, 1.525879e-05, 0.0002441406, 0.001235962, 0.00390625, 0.009536743, 0.01977539, 0.03663635, 0.0625, 0.1001129, 0.1525879, 0.2234039, 0.3164062, 0.4358063, 0.5861816, 0.7724762, 1.0]),
    (11, [0.0, 0.2275238, 0.4138184, 0.5641937, 0.6835938, 0.7765961, 0.8474121, 0.8998871, 0.9375, 0.9633636, 0.9802246, 0.9904633, 0.9960938, 0.998764, 0.9997559, 0.9999847, 1.0]),
    (12, [0.0, 0.0001220703, 0.001953125, 0.009887695, 0.03125, 0.07629395, 0.1582031, 0.2930908, 0.5, 0.7069092, 0.8417969, 0.9237061, 0.96875, 0.9901123, 0.9980469, 0.9998779, 1.0]),
    (13, [0.0, 0.2069092, 0.3417969, 0.4237061, 0.46875, 0.4901123, 0.4980469, 0.4998779, 0.5, 0.5001221, 0.5019531, 0.5098877, 0.53125, 0.5762939, 0.6582031, 0.7930908, 1.0]),
    (14, [0.0, 9.536743e-07, 3.051758e-05, 0.0002317429, 0.0009765625, 0.002980232, 0.007415771, 0.0160284, 0.03125, 0.05631351, 0.09536743, 0.1535902, 0.2373047, 0.3540926, 0.5129089, 0.7241964, 1.0]),
    (15, [0.0, 0.2758036, 0.4870911, 0.6459074, 0.7626953, 0.8464098, 0.9046326, 0.9436865, 0.96875, 0.9839716, 0.9925842, 0.9970198, 0.9990234, 0.9997683, 0.9999695, 0.999999, 1.0]),
    (16, [0.0, 1.525879e-05, 0.0004882812, 0.003707886, 0.015625, 0.04768372, 0.1186523, 0.2564545, 0.5, 0.7435455, 0.8813477, 0.9523163, 0.984375, 0.9962921, 0.9995117, 0.9999847, 1.0]),
    (17, [0.0, 0.2435455, 0.3813477, 0.4523163, 0.484375, 0.4962921, 0.4995117, 0.4999847, 0.5, 0.5000153, 0.5004883, 0.5037079, 0.515625, 0.5476837, 0.6186523, 0.7564545, 1.0]),
    (18, [0.0, 0.004815273, 0.01921472, 0.04305967, 0.07612047, 0.1180787, 0.1685304, 0.2269896, 0.2928932, 0.3656067, 0.4444298, 0.5286033, 0.6173165, 0.7097153, 0.8049097, 0.9019828, 1.0]),
    (19, [0.0, 0.09801714, 0.1950903, 0.2902847, 0.3826834, 0.4713967, 0.5555702, 0.6343933, 0.7071068, 0.7730104, 0.8314696, 0.8819213, 0.9238795, 0.9569404, 0.9807853, 0.9951847, 1.0]),
    (20, [0.0, 0.00960736, 0.03806023, 0.08426519, 0.1464466, 0.2222149, 0.3086583, 0.4024549, 0.5, 0.5975451, 0.6913417, 0.7777851, 0.8535534, 0.9157348, 0.9619398, 0.9903926, 1.0]),
    (21, [0.0, 0.09754516, 0.1913417, 0.2777851, 0.3535534, 0.4157348, 0.4619398, 0.4903926, 0.5, 0.5096074, 0.5380602, 0.5842652, 0.6464466, 0.7222149, 0.8086583, 0.9024549, 1.0]),
    (22, [0.0, 0.001955032, 0.007843256, 0.01773542, 0.03175414, 0.05008227, 0.07297522, 0.1007816, 0.1339746, 0.1732027, 0.2193753, 0.2738156, 0.3385622, 0.4170388, 0.5158771, 0.6520147, 1.0]),
    (23, [0.0, 0.3479853, 0.4841229, 0.5829612, 0.6614378, 0.7261844, 0.7806247, 0.8267973, 0.8660254, 0.8992184, 0.9270248, 0.9499177, 0.9682459, 0.9822646, 0.9921567, 0.998045, 1.0]),
    (24, [0.0, 0.003921629, 0.01587708, 0.03648759, 0.0669873, 0.1096876, 0.1692811, 0.2579385, 0.5, 0.7420615, 0.8307189, 0.8903124, 0.9330127, 0.9635124, 0.9841229, 0.9960784, 1.0]),
    (25, [0.0, 0.2420615, 0.3307189, 0.3903124, 0.4330127, 0.4635124, 0.4841229, 0.4960784, 0.5, 0.5039216, 0.5158771, 0.5364876, 0.5669873, 0.6096876, 0.6692811, 0.7579385, 1.0]),
    (26, [0.0, 0.0005060653, 0.00132267, 0.002582047, 0.004524272, 0.007519592, 0.01213901, 0.01926312, 0.03025, 0.04719409, 0.07332545, 0.1136255, 0.1757767, 0.2716269, 0.4194482, 0.6474198, 0.999]),
    (27, [0.0, 0.3519318, 0.5801314, 0.7281004, 0.8240466, 0.8862599, 0.9266002, 0.9527577, 0.9697188, 0.9807166, 0.9878479, 0.9924719, 0.9954702, 0.9974144, 0.998675, 0.9994924, 1.0]),
    (28, [0.0, 0.0006613351, 0.002262136, 0.006069503, 0.015125, 0.03666272, 0.08788835, 0.2097241, 0.50025, 0.7901708, 0.9120675, 0.9633187, 0.9848672, 0.9939272, 0.9977365, 0.9993381, 1.0]),
    (29, [0.0, 0.2900657, 0.4120233, 0.4633001, 0.4848594, 0.4939239, 0.4977351, 0.4993375, 0.5, 0.5006613, 0.5022621, 0.5060695, 0.515125, 0.5366627, 0.5878884, 0.7097241, 0.9995]),
    (30, [0.0, 0.001064949, 0.00201149, -0.0009271054, -0.005524272, -0.002205025, 0.01137872, 0.01432818, -0.01562502, -0.04655191, 4.639233e-08, 0.1107198, 0.08838827, -0.1927764, -0.3641188, 0.1678234, 1.0]),
    (31, [0.0, 0.8321766, 1.364119, 1.192776, 0.9116117, 0.8892803, 1.0, 1.046552, 1.015625, 0.9856718, 0.9886213, 1.002205, 1.005524, 1.000927, 0.9979885, 0.998935, 1.0]),
    (32, [0.0, 0.001091298, -0.001381067, -0.005032533, 0.01196944, 0.01858137, -0.08305787, -0.03650504, 0.5, 1.036505, 1.083058, 0.9814187, 0.9880306, 1.005033, 1.001381, 0.9989087, 1.0]),
    (33, [0.0, 0.6820594, 0.4558058, 0.5, 0.5078125, 0.4943106, 0.5027621, 0.4989943, 0.5, 0.5010058, 0.4972379, 0.5056894, 0.4921875, 0.5, 0.5441942, 0.3179406, 1.0]),
    (34, [0.0, -0.005987232, -0.02131066, -0.04201291, -0.06413656, -0.08372425, -0.09681856, -0.09946211, -0.08769751, -0.05756735, -0.005114265, 0.07361916, 0.1825903, 0.3257566, 0.5070754, 0.730504, 1.0]),
    (35, [0.0, 0.269496, 0.4929246, 0.6742435, 0.8174097, 0.9263809, 1.005114, 1.057567, 1.087698, 1.099462, 1.096819, 1.083724, 1.064137, 1.042013, 1.021311, 1.005987, 1.0]),
    (36, [0.0, -0.01676208, -0.0530057, -0.08766693, -0.09968185, -0.06798655, 0.02848291, 0.2107905, 0.5, 0.7892095, 0.9715171, 1.067986, 1.099682, 1.087667, 1.053006, 1.016762, 1.0]),
    (37, [0.0, 0.2464623, 0.4087048, 0.5025572, 0.5438488, 0.5484093, 0.5320683, 0.5106553, 0.5, 0.4893447, 0.4679317, 0.4515907, 0.4561512, 0.4974429, 0.5912951, 0.7535377, 1.0]),
    (38, [0.0, 0.01342773, 0.03808594, 0.06225586, 0.02734375, 0.09741211, 0.2021484, 0.2478027, 0.234375, 0.1618652, 0.03027344, 0.2614746, 0.5273438, 0.7341309, 0.8818359, 0.970459, 1.0]),
    (39, [0.0, 0.02954102, 0.1181641, 0.2658691, 0.4726562, 0.7385254, 0.9697266, 0.8381348, 0.765625, 0.7521973, 0.7978516, 0.9025879, 0.9726562, 0.9377441, 0.9619141, 0.9865723, 1.0]),
    (40, [0.0, 0.01904297, 0.01367188, 0.1010742, 0.1171875, 0.01513672, 0.2636719, 0.440918, 0.5, 0.559082, 0.7363281, 0.9848633, 0.8828125, 0.8989258, 0.9863281, 0.980957, 1.0]),
    (41, [0.0, 0.05908203, 0.2363281, 0.4848633, 0.3828125, 0.3989258, 0.4863281, 0.480957, 0.5, 0.519043, 0.5136719, 0.6010742, 0.6171875, 0.5151367, 0.7636719, 0.940918, 1.0]),
    ];

    #[test]
    fn ease_as_the_exe() {
        let mut bad = vec![];
        for &(code, want) in EXE_EASES {
            // the exe's callers (FUN_00137f50) ease only while time is left and
            // land on the target at the end: expo's 0.999 at t = 1 never shows
            for (k, w) in want.iter().enumerate().take(16) {
                let t = k as f64 / 16.0;
                let got = ease(code).map_or(t, |(e, n)| conv_core::ease::eased(e, n, t));
                if (got - w).abs() > 1e-5 {
                    bad.push(format!("{code} {k}: {got} {w}"));
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    fn picture(name: &str, w: f32, h: f32) -> (String, Picture) {
        (name.into(), Picture { name: name.into(), width: w, height: h, centre: (w / 2.0, h / 2.0), flags: 0, expression: false, shot: 0, pose_value: 0 })
    }

    fn scripts(src: &str) -> Vec<(String, Vec<Command<'_>>)> {
        vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())]
    }

    /// A game with one script and nothing else; tests override fields with `..game(src)`.
    fn game(src: &str) -> Game<'_> {
        Game { scripts: scripts(src), scenes: vec![], lines: vec![], pictures: HashMap::new(), textures: HashMap::new(), sounds: vec![], waves: HashMap::new(), pic_ext: "webp", motions: vec![], movies: vec![], shaders: vec![] }
    }

    #[test]
    fn tomp_plays_the_movie_table_entry_skippable_once_seen() {
        let movies: Vec<String> = (0..9).map(|i| format!("movie/m{i}.mp4")).collect();
        let g = Game { movies, ..game("_TOMP(8);_TOMW();_TOMP(4);_TOMW();") };
        let played: Vec<(String, bool)> = compile(&g).ops.iter().filter_map(|o| match o { Op::Movie { file, skippable } => Some((file.clone(), *skippable)), _ => None }).collect();
        let m = |f: &str, s| (f.to_string(), s);
        // a day card always; the prologue unskippable until its seen bit (2)
        assert_eq!(played, [m("movie/m8.mp4", true), m("movie/m4.mp4", false), m("movie/m4.mp4", true)]);
        assert_eq!(movie_seen(4), Some(Var { global: true, index: MOVIE_SEEN + 2 }));
        assert_eq!((movie_seen(0x25).unwrap().index, movie_seen(8)), (MOVIE_SEEN + 5, None));
    }

    #[test]
    fn layers_are_drawn_in_priority_order() {
        let src = "_STCH(1,a,240,136);_STCH(2,b,240,136);_FADS(1,0,0);_STCP(1,50);_FADS(1,0,0);_STCP(2,0);_FADS(1,0,0);";
        let g = Game { pictures: [picture("a", 10.0, 10.0), picture("b", 10.0, 10.0)].into(), ..game(src) };
        // Each IR layer as it is drawn or cleared, in order.
        let drawn: Vec<(u32, Option<String>)> = compile(&g)
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Sprites { changes, .. } => Some((changes[0].layer, changes[0].sprite.as_ref().map(|s| s.image.clone()))),
                _ => None,
            })
            .collect();
        let (a, b) = (Some("pic/a.webp".to_string()), Some("pic/b.webp".to_string()));
        // a (10) under b (20); a raised to 50 goes over b; b at 0 isn't drawn.
        assert_eq!(drawn, [(38, b.clone()), (39, a.clone()), (38, a.clone()), (39, b), (38, None), (39, a)]);
    }

    #[test]
    fn opcodes_the_exe_skips_are_left_out() {
        let c = compile(&game("_FANX(1,0,0);_STQK();_SPFC(1,img1);_SPRC();"));
        assert_eq!((c.metrics.totals(), c.metrics.left_out_total()), ((4, 0), 4));
    }

    #[test]
    fn stz4_zooms_the_staged_picture() {
        let src = "_STCH(1,img1,240,136);_STZ4(1,1000,1000,2000,-500,0,0,0);_FADS(1,0,0);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: [("img1".into(), (2560, 1440))].into(),
            ..game(src)
        };
        let ops = compile(&g).ops;
        assert!(ops.contains(&Op::SpriteAngle { layer: 39, degrees: 0, zoom: (100, -25) }), "{ops:?}");
        // Argument 7 keeps the zoom from the draw; static ones read every argument.
        let src = "_STCH(1,img1,240,136);_STZ4(1,1000,1000,2000,-500,0,0,1);_FADS(1,0,0);";
        let g = Game { scripts: scripts(src), ..g };
        let c = compile(&g);
        assert!(c.ops.contains(&Op::SpriteAngle { layer: 39, degrees: 0, zoom: (50, 50) }), "{:?}", c.ops);
        assert!(!c.metrics.reasons().keys().any(|r| r.contains("_STZ4")), "{:?}", c.metrics.reasons());
    }

    #[test]
    fn stgs_fades_from_the_next_fads() {
        let src = "_STCH(1,img1,240,136,1000,0);_STGS(1,,255,1000,0);_FADS(1,0,0);_FADS(1,0,0);";
        let g = Game { pictures: [picture("img1", 1280.0, 724.0)].into(), ..game(src) };
        let ops = compile(&g).ops;
        let motions: Vec<_> = ops.iter().filter(|o| matches!(o, Op::SpriteMotion2 { .. })).collect();
        assert_eq!(motions.len(), 1, "{ops:?}");
        let Op::SpriteMotion2 { layer: 39, keys, end: MotionEnd::Keep } = motions[0] else { panic!("{:?}", motions[0]) };
        assert_eq!(keys.iter().map(|k| (k.ms, k.opacity, k.ease)).collect::<Vec<_>>(), [(0, 0, None), (1000, 256, None)]);
        assert!(!ops.iter().any(|o| matches!(o, Op::SpriteOpacity { .. })), "{ops:?}");
        // easing 35 (back out)
        let src = "_STCH(1,img1,240,136,1000,0);_STGS(1,,255,1000,35);_FADS(1,0,0);";
        let g = Game { scripts: scripts(src), ..g };
        let ops = compile(&g).ops;
        let Some(Op::SpriteMotion2 { keys, .. }) = ops.iter().find(|o| matches!(o, Op::SpriteMotion2 { .. })) else { panic!("{ops:?}") };
        assert_eq!(keys[1].ease, Some((23, 0)));
    }

    #[test]
    fn timed_zoom_and_turn_run_on_the_shown_layer() {
        // zoom 1000 -> 2000 ‰ from 1500 over 1000 ms, easing 3 (quad out); a turn 0 -> 90° over 500 ms.
        let src = "_STCH(1,img1,240,136);_FADS(1,0,0);_STZ4(1,1500,1500,2000,2000,1000,3);_STRT(1,0,9000,500,0);_STCF(1,4);";
        let g = Game { pictures: [picture("img1", 1280.0, 724.0)].into(), ..game(src) };
        let ops = compile(&g).ops;
        let motions: Vec<_> = ops.iter().filter_map(|o| if let Op::SpriteMotion2 { layer: 39, keys, end: MotionEnd::Keep } = o { Some(keys) } else { None }).collect();
        assert_eq!(motions.len(), 2, "{ops:?}");
        assert_eq!(motions[0].iter().map(|k| (k.ms, k.zoom, k.zoom_y, k.angle, k.ease)).collect::<Vec<_>>(), [(0, 150, 150, 0, None), (1000, 200, 200, 0, Some((2, 2)))]);
        // spmotion turns counter-clockwise from the place's angle (2000 ‰)
        assert_eq!(motions[1].iter().map(|k| (k.ms, k.zoom, k.angle)).collect::<Vec<_>>(), [(0, 100, 0), (500, 100, -90)]);
        assert!(ops.iter().any(|o| matches!(o, Op::MotionWait { layer: 39 })), "{ops:?}");
        // From where it ends (or not shown yet): at once.
        let src = "_STCH(1,img1,240,136);_STZ4(1,800,800,800,800,2000,0,0);_FADS(1,0,0);";
        let g = Game { scripts: scripts(src), ..g };
        assert!(!compile(&g).ops.iter().any(|o| matches!(o, Op::SpriteMotion2 { .. })));
    }

    #[test]
    fn stma_runs_the_exe_motion_from_the_layer() {
        use crate::typemoon::exe::{Key, Motion};
        let key = |ms, set: &[(usize, i16)]| {
            let mut fields = [Key::UNSET; 30];
            for &(at, v) in set {
                fields[(at - 4) / 2] = v;
            }
            let mut k = Key { time_ms: ms, fields };
            if !set.iter().any(|&(at, _)| at == 0x14) {
                k.set_turn(Key::TURN_UNSET);
            }
            k
        };
        // y 136 -> 146 (10 script units = 26.5 px), zoom 1000 -> 2000 ‰, alpha 255 -> 0.
        let motion = Motion { mask: u64::MAX, length_ms: 500, keys: vec![key(0, &[(0x0A, 136), (0x0C, 255)]), key(500, &[(0x0A, 146), (0x0C, 0), (0x18, 2000), (0x1A, 2000)])], loop_from: 0 };
        let src = "_STCH(1,img1,240,136);_FADS(1,0,0);_STMA(1,1);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            motions: vec![motion],
            ..game(src)
        };
        let ops = compile(&g).ops;
        let Some(Op::SpriteMotion2 { layer: 39, keys, end: MotionEnd::Keep }) = ops.last() else { panic!("{ops:?}") };
        assert_eq!(keys.iter().map(|k| (k.ms, k.y, k.zoom, k.zoom_y, k.opacity)).collect::<Vec<_>>(), [(0, 0, 100, 100, 256), (500, 26, 200, 200, 0)]);
        // On a layer shown only by the next `_FADS`, the motion is written there, marked as the `_STMA`'s.
        let g = Game { scripts: scripts("_STCH(1,img1,240,136);_STMA(1,1);_FADS(1,0,0);"), ..g };
        let c = compile(&g);
        let at = c.ops.iter().position(|o| matches!(o, Op::SpriteMotion2 { .. })).unwrap();
        let mark = |i: usize| c.source_map.iter().rev().find(|(o, _)| *o <= i).map(|(_, l)| l.index);
        assert_eq!((mark(at), mark(at + 1)), (Some(1), Some(2)));
        // the exe's 3 frames, then the fade
        let begin = c.ops.iter().position(|o| matches!(o, Op::BeginTransition { .. })).unwrap();
        assert_eq!(c.ops[begin - 1], Op::Pause { ms: 50 });
    }

    #[test]
    fn a_motion_turns_holds_and_follows_its_mask() {
        use crate::typemoon::exe::Key;
        let key = |ms: u32, set: &[(usize, i16)]| {
            let mut fields = [Key::UNSET; 30];
            for &(at, v) in set {
                fields[(at - 4) / 2] = v;
            }
            let mut k = Key { time_ms: ms, fields };
            if !set.iter().any(|&(at, _)| at == 0x14) {
                k.set_turn(Key::TURN_UNSET);
            }
            k
        };
        // From key 1 on, mode 2: x jumps at key 2 (a key at 999 ms holds key 1's).
        // The turn 9000 (90° clockwise) at key 2; y is set but not in the mask.
        let mut turned = key(1000, &[(0x08, 260), (0x0A, 200)]);
        turned.set_turn(9000);
        // Key 0's easing 3 (quad out) to key 1; the hold and the jump straight.
        let motion = Motion { mask: Motion::X | Motion::TURN, length_ms: 1000, keys: vec![key(0, &[(0x04, 3), (0x08, 240)]), key(500, &[(0x06, 2), (0x08, 250)]), turned], loop_from: 0 };
        let src = "_STCH(1,img1,240,136);_FADS(1,0,0);_STMA(1,1);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            motions: vec![motion],
            ..game(src)
        };
        let ops = compile(&g).ops;
        let Some(Op::SpriteMotion2 { keys, .. }) = ops.last() else { panic!("{ops:?}") };
        // x in px: 10 script units = 10 · 1280/480.
        assert_eq!(keys.iter().map(|k| (k.ms, k.x, k.y, k.angle)).collect::<Vec<_>>(), [(0, 0, 0, 0), (500, 27, 0, 0), (999, 27, 0, 0), (1000, 53, 0, -90)]);
        assert_eq!(keys.iter().map(|k| k.ease).collect::<Vec<_>>(), [None, Some((2, 2)), None, None]);
    }

    #[test]
    fn loops_run_until_stopped() {
        use crate::typemoon::exe::{Key, Motion};
        let key = |ms: u32, y: i16| {
            let mut fields = [Key::UNSET; 30];
            fields[3] = y;
            let mut k = Key { time_ms: ms, fields };
            k.set_turn(Key::TURN_UNSET);
            k
        };
        let keys = vec![key(0, 136), key(500, 146), key(1000, 140), key(1500, 146)];
        let g = |src: &'static str, loop_from| Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0), picture("img2", 1280.0, 724.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            pic_ext: "webp",
            motions: vec![Motion { mask: u64::MAX, length_ms: 1500, keys: keys.clone(), loop_from }],
            movies: vec![],
            shaders: vec![],
        };
        // (None: a stop; `_STMA(n, 0)` stops it where it is, an empty Keep)
        let ends = |ops: &[Op]| ops.iter().filter_map(|o| match o {
            Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } if keys.is_empty() => Some(None),
            Op::SpriteMotion2 { end, .. } => Some(Some(*end)),
            Op::SpriteMotion { keys, .. } if keys.is_empty() => Some(None),
            _ => None,
        }).collect::<Vec<_>>();
        let src = "_STCH(1,img1,240,136);_FADS(1,0,0);_STMA(1,1);_STMA(1,0);";
        assert_eq!(ends(&compile(&g(src, 1)).ops), [Some(MotionEnd::Loop), None]);
        // from the key at 500 ms; a motion that doesn't loop isn't stopped
        assert_eq!(ends(&compile(&g(src, 500)).ops), [Some(MotionEnd::LoopFrom(1)), None]);
        assert_eq!(ends(&compile(&g(src, 0)).ops), [Some(MotionEnd::Keep)]);
        // a new picture stops it; a fade of another layer leaves it running
        let src = "_STCH(1,img1,240,136);_FADS(1,0,0);_STMA(1,1);_STCH(2,img2,240,136);_FADS(1,0,0);_STCH(1,img2,240,136);_FADS(1,0,0);";
        assert_eq!(ends(&compile(&g(src, 1)).ops), [Some(MotionEnd::Loop), None]);
        // a layer drawn below it moves it to another IR layer: started again there
        let src = "_STCH(1,img1,240,136);_FADS(1,0,0);_STMA(1,1);_STCH(0,img2,240,136,1000,255,5);_FADS(1,0,0);";
        let ops = compile(&g(src, 1)).ops;
        let layers: Vec<_> = ops.iter().filter_map(|o| match o {
            Op::SpriteMotion2 { layer, .. } => Some((*layer, 2)),
            Op::SpriteMotion { layer, .. } => Some((*layer, 0)),
            Op::MotionClock { layer, from: 39 } => Some((*layer, 1)),
            _ => None,
        }).collect();
        // with the clock of the run it leaves
        assert_eq!(layers, [(39, 2), (38, 1), (39, 0), (38, 2)]);
    }

    #[test]
    fn easings_as_the_exe_numbers_them() {
        // FUN_00130c80: 2-5 quad in, out, in-out, out-in … 38-41 bounce.
        let e = [0, 1, 2, 3, 4, 5, 7, 16, 18, 20, 21, 22, 27, 33, 35, 41, 42].map(ease);
        let want = [None, None, Some((1, 2)), Some((2, 2)), Some((3, 2)), Some((8, 2)), Some((2, 3)), Some((3, 5)), Some((4, 0)), Some((6, 0)), Some((9, 0)), Some((10, 0)), Some((15, 0)), Some((21, 0)), Some((23, 0)), Some((29, 0)), None];
        assert_eq!(e, want);
    }


    #[test]
    fn a_tiny_zoom_is_placed_and_grows() {
        use crate::typemoon::exe::{Key, Motion};
        // 1‰ across (0.05 % of the 2× texture) is placed at 1 %, not 0, and a motion to 1000‰ goes to 50 %.
        let key = |ms: u32, set: &[(usize, i16)]| {
            let mut fields = [Key::UNSET; 30];
            for &(at, v) in set {
                fields[(at - 4) / 2] = v;
            }
            let mut k = Key { time_ms: ms, fields };
            k.set_turn(Key::TURN_UNSET);
            k
        };
        let motion = Motion { mask: u64::MAX, length_ms: 500, keys: vec![key(0, &[]), key(500, &[(0x18, 1000)])], loop_from: 0 };
        let src = "_STCH(1,img1,240,136);_STZ4(1,1,1000,1,1000,0,0,0);_FADS(1,0,0);_STMA(1,1);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            motions: vec![motion],
            ..game(src)
        };
        let ops = compile(&g).ops;
        assert!(ops.contains(&Op::SpriteAngle { layer: 39, degrees: 0, zoom: (1, 50) }), "{ops:?}");
        let Some(Op::SpriteMotion2 { keys, .. }) = ops.last() else { panic!("{ops:?}") };
        assert_eq!((keys[1].zoom, keys[1].zoom_y), (5000, 100));
    }

    #[test]
    fn a_child_is_placed_in_its_parent() {
        // Parent img1 (the screen's size) at half size about the middle; the
        // child at its parent's top-left quarter point.
        let src = "_STCH(1,img1,240,136,1000,255,10);_STZ4(1,500,500,500,500,0,0,0);_STCH(2,dot,120,68,1000,255,20,,CC,,,1);_FADS(1,0,0);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            textures: [("img1".into(), (2560, 1440)), ("dot".into(), (20, 20))].into(),
            ..game(src)
        };
        let ops = compile(&g).ops;
        // 240 + 0.5 × (120 − 240) = 180 script units: 480 px; 136 + 0.5 × (68 − 135.75) ≈ 102.1: 270 px.
        assert!(ops.contains(&Op::SpriteMove { layer: 38, x: 480, y: 270, ms: 0 }), "{ops:?}");
        assert!(ops.contains(&Op::SpriteAngle { layer: 38, degrees: 0, zoom: (25, 25) }), "{ops:?}");
    }

    #[test]
    fn quakes_shake_the_layers_and_follow_them() {
        // A child shakes with its parent; a for-ever quake moves with its layer to another IR layer.
        let src = "_STCH(1,img1,240,136,1000,255,10);_STCH(2,dot,120,68,1000,255,20,,CC,,,1);_FADS(1,0,0);\
                   _SQK2(-1,15,30,0,-1,0,0,0,0);_SHK2(2,0,15,300,600,0,1);\
                   _STCH(3,dot,0,0,1000,255,5);_FADS(1,0,0);_SQS2(-1);_SSW2(-1);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            textures: [("img1".into(), (2560, 1440)), ("dot".into(), (20, 20))].into(),
            ..game(src)
        };
        let shakes: Vec<(u32, u32, Option<ScreenEffect>)> = compile(&g)
            .ops
            .into_iter()
            .filter_map(|op| match op {
                Op::SpriteShake { layer, slot, effect } => Some((layer, slot, effect)),
                _ => None,
            })
            .collect();
        let delay = |e: &Option<ScreenEffect>| match e {
            Some(ScreenEffect::ShakeList { delay_ms, looped, .. }) => Some((*delay_ms, *looped)),
            _ => None,
        };
        let at: Vec<(u32, u32, Option<(u32, bool)>)> = shakes.iter().map(|(l, s, e)| (*l, *s, delay(e))).collect();
        assert_eq!(
            at,
            [
                (38, 1, Some((33, true))),
                (39, 1, Some((33, true))),
                (38, 2, Some((0, false))),
                // the move: layer 2 to 37 (its parent's list again, no delay); 38 already runs layer 1's
                (37, 1, Some((0, true))),
                (38, 2, None),
                (39, 1, None),
                (37, 1, None),
                (38, 1, None),
            ]
        );
        // the child runs its parent's list
        assert_eq!(shakes[0].2, shakes[1].2);
    }

    #[test]
    fn stch_defaults_follow_the_exe() {
        // Layer 1's expression 3, 4 goes with a copy (layer 2); a lone argument 9 means 0 for 10
        // (layer 3, its old 1, 1 not kept); y alone keeps the copied x (layer 2 at 100).
        let src = "_STCH(1,a,100,50,1000,255,10,,CC,3,4);_STCH(3,a,10,10,1000,255,30,,CC,1,1);_SCH2(2,b,,70,,,,,CC,,,,,,,,,,1);_STCH(3,a,10,10,1000,255,30,,CC,5,);_FADS(1,0,0);";
        let p = |n: &str| picture(n, 10.0, 10.0);
        let g = Game { pictures: [p("a"), p("b"), p("a_03_04"), p("b_03_04"), p("a_01_01"), p("a_05_00"), p("a_05_01")].into(), ..game(src) };
        let ops = compile(&g).ops;
        let shown: Vec<&str> = ops.iter().filter_map(|o| match o {
            Op::Sprites { changes, .. } => changes[0].sprite.as_ref().map(|s| s.image.as_str()),
            _ => None,
        }).collect();
        // Drawn from the highest priority down: layers 3, 2, 1.
        assert_eq!(shown, ["pic/a_05_00.webp", "pic/b_03_04.webp", "pic/a_03_04.webp"]);
        let moves: Vec<i32> = ops.iter().filter_map(|o| match o { Op::SpriteMove { x, .. } => Some(*x), _ => None }).collect();
        assert_eq!(moves[1], moves[2]);
    }

    #[test]
    fn stcc_clears_the_priority() {
        // After `_STCC` a `_SCH2` keeps priority 0: not drawn.
        let src = "_STCH(1,a,0,0,1000,255,10);_STCC(1);_SCH2(1,a,0,0);_FADS(1,0,0);";
        let g = Game { pictures: [picture("a", 10.0, 10.0)].into(), ..game(src) };
        assert!(!compile(&g).ops.iter().any(|o| matches!(o, Op::Sprites { changes, .. } if changes[0].sprite.is_some())));
    }

    #[test]
    fn sch2_keeps_the_layer_and_copies_from_argument_18() {
        // Layer 1 zoomed and placed with a child on it; `_SCH2` on layer 3
        // copies it (its child moves over) and keeps its own alpha 0.
        let src = "_STCH(1,img1,120,68,1000,200,10);_STZ4(1,500,500,500,500,0,0,0);_STCH(2,dot,,,,,20,,CC,,,1);_SCH2(3,img1,,,,,,,CC,,,,,,,,,,1);_SCH2(4,dot);_FADS(1,0,0);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            textures: [("img1".into(), (2560, 1440)), ("dot".into(), (20, 20))].into(),
            ..game(src)
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        let ops = c.ops;
        // Layers 1 and 3 (priority 10) on IR layers 39, 38; the child 2 on 37, layer 4 on 36.
        for o in [Op::SpriteAngle { layer: 38, degrees: 0, zoom: (25, 25) }, Op::SpriteOpacity { layer: Some(38), opacity: 200 }, Op::SpriteOpacity { layer: Some(36), opacity: 0 }] {
            assert!(ops.contains(&o), "{o:?} in {ops:?}");
        }
    }

    #[test]
    fn a_panel_is_clipped_to_its_rectangle() {
        // img1 in a 960 × 540 panel with a soft frame 60 wide; the child in
        // it; then a black frame (cut at its inner edge, the black fading in
        // from there), then none.
        let src = "_STCH(1,img1,240,136,1000,255,10,,,,,,480,270,960,540,60);_STCH(2,dot,10,10,1000,255,20,,,,,1);_FADS(1,0,0);_STCH(1,img1,240,136,1000,255,10,,,,,,,,,,,7);_STCH(2,);_FADS(1,0,0);_STCH(1,img1,240,136,1000,255,10,,,,,,0,0,0,0,0,0);_FADS(1,0,0);";
        let g = Game { pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(), ..game(src) };
        let c = compile(&g);
        let clips: Vec<_> = c.ops.into_iter().filter(|o| matches!(o, Op::SpriteClip { .. } | Op::SpriteClipFeather { .. })).collect();
        let (outer, inner) = ((320, 180, 640, 360), (360, 220, 560, 280));
        assert_eq!(
            clips,
            [
                Op::SpriteClipFeather { layer: 38, rect: outer, feather: 40, colour: None },
                Op::SpriteClipFeather { layer: 39, rect: outer, feather: 40, colour: None },
                Op::SpriteClipFeather { layer: 39, rect: inner, feather: 40, colour: Some(0) },
                Op::SpriteClip { layer: 39, rect: (0, 0, 1280, 720) },
            ]
        );
        assert!(!c.metrics.reasons().keys().any(|r| r.contains("argument 17")), "{:?}", c.metrics.reasons());
    }

    #[test]
    fn a_child_follows_its_parents_motion() {
        use crate::typemoon::exe::{Key, Motion};
        let key = |ms, set: &[(usize, i16)]| {
            let mut fields = [Key::UNSET; 30];
            for &(at, v) in set {
                fields[(at - 4) / 2] = v;
            }
            let mut k = Key { time_ms: ms, fields };
            if !set.iter().any(|&(at, _)| at == 0x14) {
                k.set_turn(Key::TURN_UNSET);
            }
            k
        };
        // The parent goes 60 script units right (160 px) and fades out.
        let motion = Motion { mask: u64::MAX, length_ms: 500, keys: vec![key(0, &[(0x08, 240), (0x0C, 255)]), key(500, &[(0x08, 300), (0x0C, 0)])], loop_from: 0 };
        let src = "_STCH(1,img1,240,136,1000,255,10);_STCH(2,dot,300,136,1000,128,20,,,,,1);_FADS(1,0,0);_STMA(1,1);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            motions: vec![motion],
            ..game(src)
        };
        let ops = compile(&g).ops;
        let moves: Vec<_> = ops.iter().filter_map(|o| if let Op::SpriteMotion2 { layer, keys, .. } = o { Some((*layer, keys.iter().map(|k| (k.ms, k.x, k.opacity)).collect::<Vec<_>>())) } else { None }).collect();
        assert_eq!(moves, [(39, vec![(0, 0, 256), (500, 160, 0)]), (38, vec![(0, 0, 128), (500, 160, 0)])]);
    }

    #[test]
    fn a_child_turns_with_its_parent() {
        // The parent turned 90° clockwise: the child 60 script units right
        // of its middle goes 60 below it; its own picture isn't turned.
        let src = "_STCH(1,img1,240,136,1000,255,10,,,,,,,,,,,,,9000);_STCH(2,dot,300,136,1000,255,20,,,,,1);_FADS(1,0,0);";
        let g = Game { pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(), ..game(src) };
        let ops = compile(&g).ops;
        assert!(ops.contains(&Op::SpriteMove { layer: 38, x: 639, y: 519, ms: 0 }), "{ops:?}");
        assert!(ops.iter().any(|o| matches!(o, Op::SpriteAngle { layer: 38, degrees: 0, .. })), "{ops:?}");
    }

    #[test]
    fn turn_shader_and_tint() {
        let src = "_STCH(1,img1,240,136);_STRT(1,-9000,-9000,0,0);_STBR(1,1);_STEC(1,1,0xFF8000);_FADS(1,0,0);_STCH(1,img1,240,136);_FADS(1,0,0);";
        let g = Game {
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: [("img1".into(), (2560, 1440))].into(),
            // `_STBR` 1: blend id 1, shader 27 (ps_add)
            shaders: vec![0, 27],
            ..game(src)
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        // An untimed turn from where it goes reads every argument.
        assert!(!c.metrics.reasons().keys().any(|r| r.contains("_STRT")), "{:?}", c.metrics.reasons());
        let ops = c.ops;
        assert!(ops.contains(&Op::SpriteAngle { layer: 39, degrees: 90, zoom: (50, 50) }), "{ops:?}");
        let blends: Vec<_> = ops.iter().filter_map(|o| if let Op::SpriteBlend { mode, .. } = o { Some(*mode) } else { None }).collect();
        // Added on, then normal again: `_STCH` resets the shader.
        assert_eq!(blends, [BLEND_ADD, BLEND_NORMAL]);
        let Some(Op::SpriteFilters { filters, .. }) = ops.iter().find(|o| matches!(o, Op::SpriteFilters { .. })) else { panic!("{ops:?}") };
        // Filled with the colour, drawn in it again.
        assert_eq!(filters[0].params, [255, 64, 0].map(Operand::Const));
    }

    #[test]
    fn the_last_lesson_needs_every_other_read() {
        let c = compile(&game("_RTST(533);_RDED(533);_RDED(12);_IFUR(1,OK,LV0);_RTM_();_ZZbbc01(OK);"));
        let flag = lesson_flag(533).unwrap();
        assert_eq!(c.ops[1..4], [Op::Arith { var: flag, op: ArithOp::Set, value: Operand::Const(1) }, Op::Gosub("tm_all_lessons".into()), Op::SceneEnd]);
        // `_RDED(12)` (a scene's flag, no reader yet) is left out.
        assert_eq!(c.ops[4], Op::If { var: flag, cmp: Compare::Eq, value: Operand::Const(0), label: "tm_S__OK_not_all".into() });
        assert_eq!(c.ops[4 + 24..4 + 27], [Op::Jump("tm_S__OK".into()), Op::Label("tm_S__OK_not_all".into()), Op::ReturnToTitle]);
        // `_RTM_` ends the script.
        assert_eq!(c.ops[4 + 27], Op::ReturnToTitle);
        assert_eq!(lesson_flag(556).unwrap().index, 999);
    }

    #[test]
    fn a_choice_goes_to_its_label() {
        let src = "_SELR(0,$000000);_SELR(1,$000001,1);_SELR(1,$000001);_SELD(_D000,);_GOTS(_D000,SEL0_0,SEL0_1);_ZZ05a01(SEL0_0);_IFP4(,LV00100);_ZY12800(LV00100);_IFNX(,LV00200);";
        let g = Game {
            lines: vec!["父の<事|こと>だ。", "妹の事だ。"],
            pictures: HashMap::new(),
            ..game(src)
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        assert_eq!(
            c.ops,
            [
                Op::Label("tm_S".into()),
                Op::Choice { options: vec![("父の事だ。".into(), "tm_S__SEL0_0".into()), ("妹の事だ。".into(), "tm_S__SEL0_1".into())], style: None, disabled: vec![] },
                Op::Label("tm_S__SEL0_0".into()),
                Op::Jump("tm_S__LV00100".into()),
                Op::Label("tm_S__LV00100".into()),
            ]
        );
    }

    #[test]
    fn an_option_under_a_flag_is_offered_by_it() {
        let src = "_SELR(0,$000000);_IF__(F056,eq,1,LV00100);_SELR(1,$000001);_ZY10500(LV00100);_IF__(F056,eq,0,LV00200);_SELR(2,$000001,1);_ZY10700(LV00200);_SELR(3,next);_SELD(_D000,);_GOTS(_D000,A,B,C,D);";
        let g = Game {
            lines: vec!["父の事だ。", "妹の事だ。"],
            pictures: HashMap::new(),
            ..game(src)
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        let f = Var { global: false, index: FIRST_LOCAL };
        let m = Var { global: false, index: FIRST_LOCAL + 1 };
        let opt = |t: &str, l: &str| (t.to_string(), format!("tm_S__{l}"));
        assert_eq!(
            c.ops,
            [
                Op::Label("tm_S".into()),
                Op::If { var: f, cmp: Compare::Ne, value: Operand::Const(1), label: "tm_S__LV00100".into() },
                Op::Arith { var: m, op: ArithOp::BitOr, value: Operand::Const(2) },
                Op::Label("tm_S__LV00100".into()),
                Op::If { var: f, cmp: Compare::Ne, value: Operand::Const(0), label: "tm_S__LV00200".into() },
                // option 2 greyed: its bit 16 up
                Op::Arith { var: m, op: ArithOp::BitOr, value: Operand::Const(1 << 18) },
                Op::Label("tm_S__LV00200".into()),
                Op::If { var: m, cmp: Compare::Eq, value: Operand::Const(2), label: "tm_S__choice1_2".into() },
                Op::If { var: m, cmp: Compare::Eq, value: Operand::Const(1 << 18), label: "tm_S__choice1_262144".into() },
                Op::If { var: m, cmp: Compare::Eq, value: Operand::Const(2 | 1 << 18), label: "tm_S__choice1_262146".into() },
                Op::Arith { var: m, op: ArithOp::Set, value: Operand::Const(0) },
                Op::Choice { options: vec![opt("父の事だ。", "A"), opt("next", "D")], style: None, disabled: vec![] },
                Op::Label("tm_S__choice1_2".into()),
                Op::Arith { var: m, op: ArithOp::Set, value: Operand::Const(0) },
                Op::Choice { options: vec![opt("父の事だ。", "A"), opt("妹の事だ。", "B"), opt("next", "D")], style: None, disabled: vec![] },
                Op::Label("tm_S__choice1_262144".into()),
                Op::Arith { var: m, op: ArithOp::Set, value: Operand::Const(0) },
                Op::Choice { options: vec![opt("父の事だ。", "A"), opt("妹の事だ。", "C"), opt("next", "D")], style: None, disabled: vec![1] },
                Op::Label("tm_S__choice1_262146".into()),
                Op::Arith { var: m, op: ArithOp::Set, value: Operand::Const(0) },
                Op::Choice { options: vec![opt("父の事だ。", "A"), opt("妹の事だ。", "B"), opt("妹の事だ。", "C"), opt("next", "D")], style: None, disabled: vec![2] },
            ]
        );
    }

    #[test]
    fn stcf_waits_for_the_layers_fade_or_motion() {
        // A fade waits under mask 8 and the default, not under 0x20; a layer not shown doesn't wait.
        let src = "_STCH(1,img1,240,136,1000,0);_STGS(1,,255,1000,0);_FADS(1,0,0);_STCF(1,8);_STCF(1,32);_STCF(1);_STCF(2);";
        let g = Game { pictures: [picture("img1", 1280.0, 724.0)].into(), ..game(src) };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        let waits: Vec<_> = c.ops.iter().filter(|o| matches!(o, Op::MotionWait { .. })).collect();
        assert_eq!(waits, [&Op::MotionWait { layer: 39 }, &Op::MotionWait { layer: 39 }]);
        // and each `_STCF` a frame more, waiting or not
        assert_eq!(c.ops.iter().filter(|o| **o == Op::Pause { ms: 17 }).count(), 4);
        // On a layer already shown the fade starts at the `_STGS`, and `_STCF` waits for it.
        let src = "_STCH(1,img1,240,136,1000,0);_FADS(1,0,0);_STGS(1,,255,1000,0);_STCF(1,8);_FADS(1,0,0);";
        let g = Game { scripts: scripts(src), ..g };
        let ops = compile(&g).ops;
        let at = |f: fn(&Op) -> bool| ops.iter().position(f).unwrap();
        let (fade, wait) = (at(|o| matches!(o, Op::SpriteMotion2 { layer: 39, .. })), at(|o| matches!(o, Op::MotionWait { layer: 39 })));
        let second = ops.iter().rposition(|o| matches!(o, Op::BeginTransition { .. })).unwrap();
        assert!(fade < wait && wait < second, "{ops:?}");
        // the second `_FADS` doesn't place it again
        assert!(!ops[second..].iter().any(|o| matches!(o, Op::Sprites { .. } | Op::SpriteMotion2 { .. })), "{ops:?}");
    }

    #[test]
    fn fads_and_stma_mode_hide_the_window() {
        // 1 hides; 2 over a fade of 2 ms or more (always for `_STMA`); 0, none and a 1 ms 2 don't.
        let src = "_FADS(0,500,,0);_FADS(0,500,,);_FADS(1,0,,2);_STMA(0,0,0);_FADS(0,500,,2);_ZM($000000);_STMA(0,0,2);_ZM($000000);_FADS(1,0,,1);";
        let g = Game {
            lines: vec!["父だ。"],
            pictures: HashMap::new(),
            ..game(src)
        };
        let ops: Vec<&str> = compile(&g).ops.iter().filter_map(|o| match o {
            Op::HideTextWindow => Some("hide"),
            Op::BeginTransition { .. } => Some("fade"),
            Op::WindowFade { .. } => Some("in"),
            Op::Text { .. } => Some("text"),
            _ => None,
        }).collect();
        assert_eq!(ops, ["fade", "fade", "fade", "hide", "fade", "in", "text", "hide", "in", "text", "hide", "fade"]);
    }

    #[test]
    fn a_loop_gives_its_channel_up() {
        // The same loop again on channel 0 starts once more, not twice over; a one-shot after it stops it too.
        let src = "_SEPL(0,1,A);_SEPL(0,1,A,,`017:40);_SEPL(0,0,B);_SEPL(0,0,A);";
        let g = Game {
            pictures: HashMap::new(),
            waves: [("A".into(), "a.wav".into()), ("B".into(), "b.wav".into())].into(),
            ..game(src)
        };
        let ops: Vec<String> = compile(&g).ops.iter().filter_map(|o| match o {
            Op::SePlay { file, looped } => Some(format!("{file}{}", if *looped { " loop" } else { "" })),
            Op::SeFadeOut { file: Some(f), ms: 0 } => Some(format!("stop {f}")),
            _ => None,
        }).collect();
        assert_eq!(ops, ["se/a.wav loop", "stop se/a.wav", "se/a.wav loop", "stop se/a.wav", "se/b.wav", "se/a.wav"]);
    }

    #[test]
    fn volumes_and_their_fades() {
        let src = "_SEPL(3,1,A,,`001:4000,,`017:50);_SEVL(23,,,,`001:3000,,`017:30);_SEVL(4,,,,`017:30);_MPLY(2,,`017:80);_MVOL(,,`001:5000);";
        let g = Game { pictures: HashMap::new(), waves: [("A".into(), "a.wav".into())].into(), sounds: vec!["STRM_X".into(), "STRM_Y".into()], ..game(src) };
        let ops: Vec<String> = compile(&g).ops.iter().filter_map(|o| match o {
            Op::SeFadeIn { ms } => Some(format!("in {ms}")),
            Op::SoundVolume { file, percent, ms } => Some(format!("{file:?} {percent} {ms}")),
            _ => None,
        }).collect();
        assert_eq!(ops, ["in 4000", "Some(\"se/a.wav\") 50 0", "Some(\"se/a.wav\") 30 3000", "None 80 0", "None 0 5000"]);
    }

    #[test]
    fn window_fades_go_on_and_the_next_text_fades_in() {
        let src = "_CLO4(1000);_ZM($000000);_ZM($000001);_CLO3();_CLO5(2000);_ZM($000000);";
        let g = Game {
            lines: vec!["父だ。", "妹だ。"],
            pictures: HashMap::new(),
            ..game(src)
        };
        let fades: Vec<_> = compile(&g).ops.into_iter().filter_map(|o| match o {
            Op::WindowFade { alpha, ms, from, delay, show } => Some((alpha, ms, from, delay, show)),
            Op::Text { .. } => Some((1, 0, None, 0, false)),
            _ => None,
        }).collect();
        let text = (1, 0, None, 0, false);
        assert_eq!(fades, [(0, 500, None, 0, false), (256, 150, Some(0), 150, false), text, text, (0, 67, None, 0, false), (256, 1000, Some(0), 1000, true), text]);
    }

    #[test]
    fn unread_arguments_are_gaps() {
        let c = &script::parse("_MPLY(5,7,`001:4000,`017:50);").into_iter().map(Result::unwrap).collect::<Vec<_>>()[0];
        let read = Cell::new(0b101);
        assert_eq!(unread(c, read.get()), ["_MPLY argument 1 ignored", "_MPLY option 17 ignored"]);
    }

    #[test]
    fn numbers_and_markup() {
        assert_eq!(number("258-18"), Some(240));
        assert_eq!(number("-5"), Some(-5));
        assert_eq!(number("242.25"), Some(242));
        assert_eq!(number("F1"), None);
        assert_eq!(option_at(&["", "`001:4000", "`017:50"], 17), Some((2, 50)));
        let (t, r, _) = text("@g　開け<肌寒|はだざむ>い^[ber00]風@k");
        assert_eq!(t, "　開け肌寒い\n風");
        assert_eq!(r, vec![Ruby { start: 3, len: 2, text: "はだざむ".into() }]);
        // the fan patch's italic (U+E0xx, U+E1(80+xx)) and upside-down (U+E2(80+xx)) letters
        let (t, _, s) = text("A\u{E061}\u{E062}\u{E1E3} \u{E2E4}\u{E03F} \u{E041}");
        assert_eq!(t, "Aabc d? A");
        // mirrored letters, stored right to left
        let m = |s: &str| s.chars().map(|c| if c == ' ' { c } else { char::from_u32(0xE100 + c as u32).unwrap() }).collect::<String>();
        assert_eq!(text(&format!("「{}■■{}^{} 」", m(" ti yas"), m("I "), m(".on"))).0, "「 I■■say it \nno. 」");
        assert_eq!(s, vec![TextStyle { start: 1, len: 3, italic: true, ..Default::default() }, TextStyle { start: 6, len: 3, italic: true, ..Default::default() }]);
    }

    #[test]
    fn a_picture_is_placed_by_its_centre_at_half_size() {
        let src = "_ZZ01501(START);_STCH(1,img1,240,136,1000,255,2,,CC);_FADS(0,500,0);_WNTY(1);_VPLY(A00,1);_ZM01801($000001@n);_MSAD($000000);_JUMP(110);";
        let g = Game {
            scenes: vec![110],
            lines: vec!["と。", "「<肌|はだ>"],
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: [("img1".into(), (2560, 1440))].into(),
            ..game(src)
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        // each command marks where its ops start, after the script's label
        let marks: Vec<(usize, usize)> = c.source_map.iter().map(|(at, l)| (*at, l.index)).collect();
        assert_eq!(marks[..3], [(1, 0), (2, 1), (2, 2)]);
        assert!(c.source_map.iter().all(|(at, l)| l.file == "S" && *at <= c.ops.len()));
        let layer = 39;
        assert_eq!(
            c.ops,
            vec![
                Op::Label("tm_S".into()),
                Op::Label("tm_S__START".into()),
                Op::Pause { ms: 50 },
                Op::BeginTransition { effect: "_FADS(0,500,0)".into(), fade_ms: 500, mask: None, style: TransitionStyle::Fade },
                Op::Sprites { changes: vec![SpriteChange { layer, sprite: Some(SpritePlacement { image: "pic/img1.webp".into(), x: 0, y: 0 }), keep_filters: false }], fade_ms: 0 },
                Op::SpriteMove { layer, x: 640, y: 360, ms: 0 },
                Op::SpriteAngle { layer, degrees: 0, zoom: (50, 50) },
                Op::EndTransition,
                Op::Text {
                    speaker: None,
                    text: "「肌と。".into(),
                    voice: Some(Voice { file: "stream/A00.ogg".into(), character: 0 }),
                    links: vec![],
                    ruby: vec![Ruby { start: 1, len: 1, text: "はだ".into() }],
                    styles: vec![],
                    hang: None,
                    continues: false
                },
                Op::Jump("tm_S".into()),
            ]
        );
    }

    #[test]
    fn stch_without_a_place_keeps_the_layers() {
        let src = "_STCH(1,img1,120,68,500);_STCH(1,img1);_FADS(1,0,0);_STCC(1);_STCH(1,img1,120);_FADS(1,0,0);";
        let g = Game { pictures: [picture("img1", 1280.0, 724.0)].into(), ..game(src) };
        let ops = compile(&g).ops;
        let moves: Vec<(i32, i32)> = ops.iter().filter_map(|o| if let Op::SpriteMove { x, y, .. } = o { Some((*x, *y)) } else { None }).collect();
        let zooms: Vec<_> = ops.iter().filter_map(|o| if let Op::SpriteAngle { zoom, .. } = o { Some(*zoom) } else { None }).collect();
        // Kept at 120, 68 and zoom 500; after `_STCC` a missing y is 0 and the zoom 1000.
        assert_eq!(moves, [(320, 180), (320, 0)]);
        assert_eq!(zooms, [(25, 25), (50, 50)]);
    }

    #[test]
    fn a_shake_dies_down_over_its_turns() {
        let c = compile(&game("_SHOK(0,3,100,10,50);_SHOK(4);_SHAF();"));
        let shake = |time_ms, speed, range, delay_ms| Op::ScreenEffect { slot: 1, effect: Some(ScreenEffect::Shake { delay_ms, time_ms, interval_ms: 16, path: false, speed: (speed, speed), range: [range, (0, 0)], end_ms: time_ms }) };
        // a turn every 100 ms, 10 turns, 50 ms late; the defaults 300 ms and 3 turns
        assert_eq!(c.ops[1..], [shake(1000, 655, (0, 3), 50), shake(900, 218, (4, 0), 0), Op::ScreenEffect { slot: 1, effect: None }]);
    }
}
