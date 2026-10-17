//! 月姫's scripts lowered to the IR: the flow (labels, jumps, flags), the
//! text, the picture layers and the sound. What isn't lowered yet is
//! counted in the notes (`docs/games/tsukihime.md`, "Status").
//!
//! Pictures: `_STCH` stages a picture on one of 40 layers and `_FADS`
//! shows the staged layers. A picture is drawn at its record's size on a
//! 1280 × 724 stage (the texture at half size), the record's centre at
//! the script's x, y × 8/3 ("The stage"). The screen is 1280 × 720.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use conv_core::ir::{ArithOp, Compare, MotionEnd, MotionKey, Op, Operand, Ruby, SpriteChange, SpriteFilter, SpritePlacement, TransitionStyle, Var, Voice, BLEND_ADD, BLEND_MULTIPLY, BLEND_NORMAL, BLEND_SUBTRACT, FILTER_BLEND_COPY, FILTER_BLEND_MULTIPLY, FILTER_BLUR, FILTER_COLOUR, FILTER_GREYSCALE, FILTER_NEGATIVE};

use super::exe::Picture;
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
    /// The width and height of each picture's file (`pic/<name>.webp`),
    /// by lowercase name: the record's size is drawn from it.
    pub textures: HashMap<String, (u32, u32)>,
    /// The sound archive's sound names: `_MPLY(n)` plays sound `n − 1`.
    pub sounds: Vec<String>,
    /// The wave sounds' names (`WSD_moon_se02093`) by the scripts' name
    /// for them (uppercase, without `WSD_`).
    pub waves: HashMap<String, String>,
    /// The exe's motions: `_STMA(layer, m)` runs `motions[m - 1]`.
    pub motions: Vec<super::exe::Motion>,
}

pub struct Compiled {
    pub ops: Vec<Op>,
    pub notes: BTreeMap<String, usize>,
    pub metrics: Metrics,
}

/// How a layer is drawn: the image, its middle on the screen, its zoom (%).
type Look = (String, (f32, f32), (i32, i32));

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
    /// `_STGS`'s fade, from the next `_FADS` on: (from, to, ms); `alpha` is
    /// already `to`.
    fade: Option<(u32, u32, u32)>,
    /// `_STMA` on a layer not shown yet: the motion, from the `_FADS` that shows it.
    motion: Option<usize>,
    /// `_STCH` argument 11: the layer this one is placed in.
    parent: Option<usize>,
    /// Turned by this many 1/100 degrees, counter-clockwise (`_STRT`,
    /// `_STCH` argument 19).
    angle: i32,
    /// `_STBR`: the shader it is drawn with (0 or 255: the normal one).
    shader: u8,
    /// `_STEC(layer, 1, colour)`: its colours multiplied by this one.
    tint: Option<u32>,
    /// `_STCH` arguments 12–16: the screen rectangle it is drawn in (x, y,
    /// w, h on a 1920 × 1080 screen; none if w or h is 0 or it is the
    /// screen) and its frame's width; a child is drawn in its parent's.
    clip: [i32; 5],
}

/// CONscripter's `spblend` screen mode (lighten, never past white).
const BLEND_SCREEN: u8 = 4;

/// How a `_STBR` shader is drawn here: an `spblend` mode and a filter.
/// The shader numbers are the exe's shader table's (`FUN_00132d20`
/// maps the layer's byte through a table at 0x44DD70, the same number
/// for 1–39; the shaders are listed at 0xBD2308, 16 bytes each).
// src: exe 0x44DD70 (the shader table), 0xBD2308 (its names)
// shortcut: darken, burn and lighten, dodge are drawn as multiply and screen; overlay, soft and hard light and the rest as normal.
fn shader(n: u8) -> (u8, Option<SpriteFilter>) {
    let filter = |kind, p: i32| Some(SpriteFilter { kind: Operand::Const(kind), blend: Operand::Const(FILTER_BLEND_COPY), strength: Operand::Const(256), params: [Operand::Const(p); 3] });
    match n {
        3 | 18 | 25 | 27 => (BLEND_ADD, None),
        19 | 21 | 22 | 29 | 36 | 38 => (BLEND_MULTIPLY, None),
        26 | 28 => (BLEND_SUBTRACT, None),
        20 | 23 | 24 | 30 | 34 | 35 | 37 => (BLEND_SCREEN, None),
        7 => (BLEND_NORMAL, filter(FILTER_GREYSCALE, 0)),
        8 => (BLEND_NORMAL, filter(FILTER_NEGATIVE, 0)),
        // guess: gauss's radius, not read from its shader
        11 => (BLEND_NORMAL, filter(FILTER_BLUR, 8)),
        _ => (BLEND_NORMAL, None),
    }
}

fn label(script: &str, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("tm_{script}__{n}"),
        None => format!("tm_{script}"),
    }
}

/// A whole-number argument: digits with `+`/`-` between them (`258-18`).
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
];

/// Commands left out on purpose (command, why): done for the completion
/// figure (`metrics.json`'s `left_out`), not gaps.
const LEFT_OUT: &[(&str, &str)] = &[
    // `script_command` (0x177790) has no case for it: the Switch exe does nothing.
    ("_SNX4", "the Switch exe ignores it"),
    // `script_command` stores the page in the state (+0x8c4) for the save; its
    // only reader is the load (`FUN_00182330` → `FUN_00182c60`), which replays
    // the script until `_PGST` reaches the saved page. CONscripter's saves
    // restore the state themselves.
    ("_PGST", "the page a load replays to; CONscripter's saves restore the state"),
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
            None => format!("{op} argument {i} ignored"),
        })
        .collect()
}

/// A text line's markup: `<base|ruby>` readings, `^` line breaks, `@x`
/// codes and `[ber00]` inline pictures (both dropped).
pub fn text(line: &str) -> (String, Vec<Ruby>) {
    let (mut out, mut ruby) = (String::new(), Vec::new());
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
                    ruby.push(Ruby { start: len, len: n, text: reading.to_string() });
                }
                out.push_str(base);
                len += n;
            }
            _ => {
                // shortcut: Shift-JIS has no hearts; a note stands in until the engine reads UTF-8 text.
                out.push(if matches!(c, '♡' | '♥') { '♪' } else { c });
                len += 1;
            }
        }
    }
    (out, ruby)
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
    clips: Vec<Option<(i32, i32, i32, i32)>>,
    /// `_WNTY(1)`: the next line ends its page.
    page_end: bool,
    voice: Option<Voice>,
    /// A line ended with `@n`: the next one goes on after it, no click.
    held: Option<(String, Vec<Ruby>)>,
    se_channels: HashMap<i32, String>,
    /// `_SELR`'s options so far: (index, text).
    options: Vec<(i32, String)>,
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
            "_END_" => self.ops.push(Op::ReturnToTitle),
            "_GOTO" => self.ops.push(Op::Jump(label(script, Some(arg(0))))),
            // The Switch's and the PS4's own lines: this is the Switch.
            "_IFNX" => {}
            "_IFP4" | "_IFSC" => self.ops.push(Op::Jump(label(script, Some(arg(1))))),
            // `_SELR(index, $text[, 1])`: a choice's option (1: greyed, still
            // offered here).
            "_SELR" => {
                let Some(n) = arg(1).split_once('$').and_then(|(_, s)| s.get(..6)?.parse::<usize>().ok()) else { return false };
                let Some(line) = self.g.lines.get(n) else { return false };
                // An option is written once per branch of the `_IF__`s around it
                // (shown or greyed): the converter keeps the first.
                // shortcut: an option offered only under a flag is always offered.
                let i = number(arg(0)).unwrap_or(0);
                if !self.options.iter().any(|o| o.0 == i) {
                    self.options.push((i, text(line).0));
                }
            }
            // `_SELD`/`_SELA`/`_SELS(_D000, …)` show the choice; `_GOTS(_D000,
            // label, …)` goes to the label of the option picked.
            "_SELD" | "_SELA" | "_SELS" => {}
            "_GOTS" => {
                let options = std::mem::take(&mut self.options);
                (1..a.len()).for_each(|i| mark(read, i));
                let labels = &a[1..];
                let choice: Vec<(String, String)> = options
                    .into_iter()
                    .filter_map(|(i, t)| Some((t, label(script, Some(labels.get(usize::try_from(i).ok()?)?.trim())))))
                    .collect();
                if choice.is_empty() {
                    return false;
                }
                self.ops.push(Op::Choice { options: choice, style: None });
            }
            // `script_command` (0x177790): `_CLO2` hides the text window at once
            // (alpha 0), `_CLO3` lets `FUN_00138d40` fade it out in 4 frames,
            // `_CLO4(ms)` fades window and text out over `ms` (visible over the
            // first half: drawn at 2 × (fade − 0.5)). The next text fades it back
            // in (`FUN_0016e0d0`).
            // shortcut: `_CLO4`'s fade hides at once (its time is a gap); a fade that doesn't block needs the engine.
            "_CLO2" | "_CLO3" | "_CLO4" => self.ops.push(Op::HideTextWindow),
            "_WNTY" => self.page_end = arg(0) == "1",
            "_VPLY" => self.voice = Some(Voice { file: format!("stream/{}.ogg", arg(0)), character: 0 }),
            "_ZM" | "_MSAD" => {
                // `$000107`, with codes around it (`@x$001549@n`, `^$042767`).
                let Some(n) = arg(0).split_once('$').and_then(|(_, s)| s.get(..6)?.parse::<usize>().ok()) else { return false };
                let Some(line) = self.g.lines.get(n) else { return false };
                let (mut body, mut ruby) = text(line);
                if let Some((before, mut r)) = self.held.take() {
                    let shift = before.chars().count();
                    ruby.iter_mut().for_each(|x| x.start += shift);
                    r.append(&mut ruby);
                    (body, ruby) = (before + &body, r);
                }
                // shortcut: `@n` joins the next line to this one, no click; what happens between them is shown before both.
                if arg(0).contains("@n") {
                    self.held = Some((body, ruby));
                    return true;
                }
                let continues = !std::mem::take(&mut self.page_end);
                self.ops.push(Op::Text { speaker: None, text: body, voice: self.voice.take(), links: vec![], ruby, styles: vec![], hang: None, continues });
            }
            "_WTTM" => self.ops.push(Op::Wait { ms: number(arg(0)).unwrap_or(0).max(0) as u32 }),
            "_SEPL" => {
                let Some(file) = self.g.waves.get(&arg(2).to_uppercase()).map(|w| format!("se/{w}.wav")) else { return false };
                self.se_channels.insert(number(arg(0)).unwrap_or(0), file.clone());
                self.ops.push(Op::SePlay { file, looped: arg(1) == "1" });
            }
            "_SEFD" => {
                let ms = option(a, 1).unwrap_or(0).max(0) as u32;
                let ch = number(arg(0)).unwrap_or(-1);
                let file = if ch < 0 {
                    self.se_channels.clear();
                    None
                } else {
                    let Some(f) = self.se_channels.remove(&ch) else { return true };
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
                self.ops.push(Op::BgmPlay { file: format!("stream/{name}.ogg"), fade_in_ms: option(a, 1).unwrap_or(0).max(0) as u32 });
            }
            "_MFAD" => {
                if option(a, 11).is_some_and(|v| v != 0) {
                    return false;
                }
                self.ops.push(Op::BgmStop { fade_out_ms: option(a, 1).unwrap_or(0).max(0) as u32 });
            }
            "_STCH" => return self.stch(a, read),
            "_STCC" => {
                let n = number(arg(0)).unwrap_or(0);
                if n < 0 {
                    self.staged.iter_mut().for_each(|l| *l = None);
                } else if let Some(l) = self.staged.get_mut(n as usize) {
                    *l = None;
                }
            }
            // `_STZ4(layer, x0, y0, x1, y1, ms, ease, off)` (`FUN_00162d20`): the
            // zoom in ‰ goes to x1, y1 (`_STCH` resets it to 1000) over ms · 60
            // / 1000 frames; at 0 frames `FUN_00131f30` leaves out x0, y0 and the
            // easing. `off` (empty: the layer's, `_STCH` clears it) keeps the
            // zoom from the draw (`FUN_00132e30`).
            // shortcut: a timed zoom (2 uses) jumps to its end; its x0, y0, ms and easing stay gaps.
            "_STZ4" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                let Some(l) = self.staged[n as usize].as_mut() else { return true };
                let z = |i: usize, old: f32| if arg(i).is_empty() { Some(old) } else { number(arg(i)).map(|v| v as f32 / 1000.0) };
                let (Some(zx), Some(zy)) = (z(3, l.stretch.0), z(4, l.stretch.1)) else { return false };
                l.stretch = (zx, zy);
                if a.get(5).map_or(Some(0), |v| number(v.trim())).is_some_and(|ms| (0..17).contains(&ms)) {
                    for i in [1, 2, 5, 6] {
                        arg(i);
                    }
                }
                if let Some(off) = number(arg(7)) {
                    l.stretch_off = off as u8 != 0; // src: a byte at 0x874 of the state
                }
            }
            // `_STGS(layer, from, to, ms, ease)` (`FUN_001637a0`): the alpha
            // from `from` (default: the layer's) to `to` (default 0) over `ms`,
            // running from the next `_FADS` while the script goes on.
            // shortcut: the easing (argument 4) is not followed; straight.
            "_STGS" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                let Some(l) = self.staged[n as usize].as_mut() else { return true };
                let from = if arg(1).is_empty() { l.alpha } else { number(arg(1)).unwrap_or(0).clamp(0, 255) as u32 };
                let to = number(arg(2)).unwrap_or(0).clamp(0, 255) as u32;
                let ms = number(arg(3)).unwrap_or(0).max(0) as u32;
                l.alpha = to;
                l.fade = (ms > 16 && from != to).then_some((from, to, ms)); // ours: under a frame is at once
            }
            "_STMA" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                return self.stma(n as usize, number(arg(1)).unwrap_or(0));
            }
            // `_STRT(layer, from, to, ms, ease)` (`FUN_00162b20`): the turn
            // in 1/100 degrees, to `to` (default 0; `_STCH` resets it).
            // shortcut: timed turns (none in the scripts) jump to the end.
            "_STRT" => {
                let Some(l) = self.staged_layer(arg(0)) else { return arg(0).parse::<i32>().is_ok() };
                l.angle = number(arg(2)).unwrap_or(0);
            }
            // `_STBR(layer, n)` (`FUN_00162490`): the shader (0: the normal one).
            "_STBR" => {
                let Some(l) = self.staged_layer(arg(0)) else { return arg(0).parse::<i32>().is_ok() };
                l.shader = number(arg(1)).unwrap_or(0).clamp(0, 255) as u8;
            }
            // `_STEC(layer, effect, colour)` (`FUN_00103de0`): effect 1 tints
            // the layer (white: none), 0 ends it; the others are not known.
            "_STEC" => {
                let effect = number(arg(1)).unwrap_or(0);
                let colour = arg(2).strip_prefix("0x").and_then(|h| u32::from_str_radix(h, 16).ok());
                let Some(l) = self.staged_layer(arg(0)) else { return arg(0).parse::<i32>().is_ok() };
                match (effect, colour) {
                    (0, _) => l.tint = None,
                    (1, Some(c)) => l.tint = (c != 0xFFFFFF).then_some(c),
                    _ => return false,
                }
            }
            "_STCP" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                self.priority[n as usize] = number(arg(1)).unwrap_or(0);
            }
            "_FADS" => self.fads(number(arg(0)).unwrap_or(1), number(arg(1)).unwrap_or(0), number(arg(2)).unwrap_or(0)),
            _ => return false,
        }
        true
    }

    fn stch(&mut self, a: &[&str], read: &Cell<u64>) -> bool {
        let arg = |i: usize| {
            mark(read, i);
            a.get(i).copied().unwrap_or("").trim()
        };
        let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)).map(|n| n as usize) else { return false };
        if arg(1).is_empty() {
            self.staged[n] = None;
            return true;
        }
        let old = self.staged[n].clone();
        let base = arg(1).to_lowercase();
        let num = |i: usize, default: f32| if arg(i).is_empty() { Some(default) } else { number(arg(i)).map(|v| v as f32) };
        // src: exe 0x160840: with no x and y the layer keeps its place (0, 0 after `_STCC`), else a missing one is 0;
        // a zoom left out is the layer's (1000 after `_STCC`); alpha 255 by default.
        let (x0, y0) = old.as_ref().filter(|_| arg(2).is_empty() && arg(3).is_empty()).map_or((0.0, 0.0), |o| (o.x, o.y));
        let zoom0 = old.as_ref().map_or(1000.0, |o| o.zoom * 1000.0);
        let (Some(x), Some(y), Some(zoom), Some(alpha)) = (num(2, x0), num(3, y0), num(4, zoom0), num(5, 255.0)) else { return false };
        let expression = if arg(9).is_empty() && arg(10).is_empty() {
            None
        } else {
            let (oa, ob) = old.as_ref().and_then(|l| l.expression).unwrap_or((0, 0));
            let e = |i: usize, d: u32| number(arg(i)).map_or(d, |v| v as u32);
            Some((e(9, oa), e(10, ob)))
        };
        // An argument left out keeps the layer's rectangle (`FUN_00160840`).
        let mut clip = old.as_ref().map_or([0; 5], |l| l.clip);
        for (i, c) in clip.iter_mut().enumerate() {
            if let Some(v) = number(arg(12 + i)) {
                *c = v;
            }
        }
        let parent = number(arg(11)).filter(|&p| (0..LAYERS as i32).contains(&p) && p as usize != n).map(|p| p as usize);
        self.staged[n] = Some(Layer { base, expression, x, y, zoom: zoom / 1000.0, stretch: (1.0, 1.0), stretch_off: false, alpha: alpha.clamp(0.0, 255.0) as u32, fade: None, motion: None, parent, angle: number(arg(19)).unwrap_or(0), shader: 255, tint: None, clip });
        // Argument 6, else layer × 10 (`FUN_00160840`).
        // src: exe 0x160840 (priority layer × 10 by default)
        self.priority[n] = if arg(6).is_empty() { n as i32 * 10 } else { number(arg(6)).unwrap_or(n as i32 * 10) };
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
        let changed: Vec<usize> = (0..LAYERS).filter(|&s| want[s] != self.slots[s] || fades[s].is_some() || motions[s].is_some()).collect();
        let fade_ms = if kind == 1 { 0 } else { ms.max(0) as u32 }; // src: exe script_command (type 1 returns at once)
        if changed.is_empty() && fade_ms == 0 {
            return;
        }
        let mask = (kind == 2).then(|| format!("pic/rule_{rule:02}.webp"));
        self.ops.push(Op::BeginTransition { effect: format!("_FADS({kind},{ms},{rule})"), fade_ms, mask, style: TransitionStyle::Fade });
        for slot in changed {
            self.slots[slot] = want[slot].clone();
            let layer = slot as u32;
            match (want[slot].take(), fades[slot]) {
                (Some(l), Some((from, to, ms))) => {
                    // Opaque, the motion's keys giving the alpha (they multiply it).
                    self.place(layer, &Layer { alpha: 255, ..l });
                    let key = |ms, alpha: u32| MotionKey { ms, x: 0, y: 0, zoom: 100, angle: 0, opacity: alpha * 256 / 255, zoom_y: 100 };
                    self.ops.push(Op::SpriteMotion2 { layer, keys: vec![key(0, from), key(ms, to)], end: MotionEnd::Keep });
                }
                (Some(l), None) => self.place(layer, &l),
                (None, _) => {
                    self.ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: None, keep_filters: false }], fade_ms: 0 });
                    self.blends[slot] = BLEND_NORMAL;
                    self.clips[slot] = None;
                }
            }
        }
        // After every place: a parent's motion moves its children too.
        for (slot, m) in motions.into_iter().enumerate() {
            if let Some(m) = m {
                self.run_motion(slot, m);
            }
        }
        self.ops.push(Op::EndTransition);
    }

    /// `_STMA(layer, m)` (`FUN_00199620`): motion `m` of the exe runs on the
    /// layer at once (the shown one and the staged copy), while the script
    /// goes on; 0 stops it. Its keys hold absolute values (x, y in script
    /// units, alpha, the `_STZ4` zoom in ‰); a field a key leaves out
    /// keeps the one before. The layer stays at the last key's.
    fn stma(&mut self, n: usize, m: i32) -> bool {
        if m <= 0 || m as usize > self.g.motions.len() {
            return m == 0;
        }
        match self.slot_layer.iter().position(|&l| l == Some(n)) {
            Some(slot) if self.shown[n].is_some() => self.run_motion(slot, m as usize),
            _ => match self.staged[n].as_mut() {
                Some(l) => l.motion = Some(m as usize),
                None => return false,
            },
        }
        true
    }

    /// Emits motion `m` on IR layer `slot` (an `spmotion` from the
    /// layer's place), and moves the layer's state to its last key. Its
    /// children (and theirs) follow it, each with an `spmotion` of its
    /// own from where the parent's keys place it.
    // shortcut: the keys' easing, priority (0x3C) and expression (0x32, 0x34) are not followed; straight lines.
    fn run_motion(&mut self, slot: usize, m: usize) {
        let Some(i) = self.slot_layer[slot] else { return };
        let (Some(start), Some(shown)) = (self.slots[slot].clone(), self.shown[i].clone()) else { return };
        // The layer as drawn and as shown (its own place) at each key.
        let (mut at, mut own) = (start.clone(), shown.clone());
        let mut states = Vec::new();
        for k in &self.g.motions[m - 1].keys {
            for l in [&mut at, &mut own] {
                if let Some(x) = k.x() {
                    l.x = f32::from(x);
                }
                if let Some(y) = k.y() {
                    l.y = f32::from(y);
                }
                if let Some(a) = k.alpha() {
                    l.alpha = a.clamp(0, 255) as u32;
                }
                let (zx, zy) = k.zoom();
                l.stretch = (zx.map_or(l.stretch.0, |z| f32::from(z) / 1000.0), zy.map_or(l.stretch.1, |z| f32::from(z) / 1000.0));
            }
            states.push((k.time_ms, at.clone(), own.clone()));
        }
        if states.is_empty() {
            return;
        }
        let mut moves = vec![(slot, start, states.iter().map(|(ms, l, _)| (*ms, l.clone())).collect::<Vec<_>>())];
        for c in (0..LAYERS).filter(|&c| c != slot) {
            let (Some(j), Some(child)) = (self.slot_layer[c], self.slots[c].clone()) else { continue };
            if !self.descends(j, i) {
                continue;
            }
            let keys = states
                .iter()
                .filter_map(|(ms, _, own)| {
                    self.shown[i] = Some(own.clone());
                    Some((*ms, self.resolved(j, 0)?))
                })
                .collect();
            moves.push((c, child, keys));
        }
        self.shown[i] = Some(shown);
        for (c, start, keys) in moves {
            self.emit_motion(c, &start, &keys);
        }
        // shortcut: the layer's state jumps to the motion's end; a picture drawn again mid-motion starts there.
        let end = |l: &mut Layer| (l.x, l.y, l.alpha, l.stretch) = (at.x, at.y, at.alpha, at.stretch);
        for l in [self.slots[slot].as_mut(), self.shown[i].as_mut(), self.staged[i].as_mut()].into_iter().flatten() {
            end(l);
        }
    }

    /// An `spmotion` on IR layer `slot` from `start` (as it is drawn)
    /// through each key's state.
    fn emit_motion(&mut self, slot: usize, start: &Layer, states: &[(u32, Layer)]) {
        let Ok((_, (x0, y0), _)) = self.look(start) else { return };
        let mut keys = Vec::new();
        for (ms, at) in states {
            let Ok((_, (x, y), _)) = self.look(at) else { return };
            // spmotion's zoom multiplies the place's (spzoom), about its centre.
            let rel = |z: f32, z0: f32| if z0 == 0.0 { 100 } else { (100.0 * z / z0).round().max(0.0) as u32 };
            keys.push(MotionKey {
                ms: *ms,
                x: (x - x0).round() as i32,
                y: (y - y0).round() as i32,
                zoom: rel(at.drawn_stretch().0.abs(), start.drawn_stretch().0.abs()),
                angle: 0,
                opacity: at.alpha * 256 / 255,
                zoom_y: rel(at.drawn_stretch().1.abs(), start.drawn_stretch().1.abs()),
            });
        }
        if keys.is_empty() {
            return;
        }
        // Opaque, the keys giving the alpha (they multiply it).
        if start.alpha < 255 {
            self.ops.push(Op::SpriteOpacity { layer: Some(slot as u32), opacity: 256 });
        }
        self.ops.push(Op::SpriteMotion2 { layer: slot as u32, keys, end: MotionEnd::Keep });
    }

    /// Whether layer `j` is placed in layer `i`, or in one placed in it.
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
    /// its zoom (%); or the name when the picture is unknown.
    fn look(&self, l: &Layer) -> Result<Look, String> {
        let name = self.name(l);
        let picture = self.g.pictures.get(&name);
        let fill = colour(&name).filter(|_| picture.is_none_or(|p| p.flags & 3 == 3));
        Ok(match (picture, fill) {
            (_, Some(rgb)) => (format!(">{SCREEN_W},{SCREEN_H},#{rgb:06x}"), (SCREEN_W as f32 / 2.0, SCREEN_H as f32 / 2.0), (100, 100)),
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
                let zoom = |rec: f32, tex: f32, z: f32| (100.0 * rec / tex * z).round() as i32;
                (format!("pic/{name}.webp"), (mx, my), (zoom(p.width, tw, zx), zoom(p.height * SCREEN_H as f32 / STAGE_H, th, zy)))
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
        let (image, (cx, cy), zoom) = match self.look(l) {
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
        self.ops.push(Op::SpriteAngle { layer, degrees: (-l.angle as f32 / 100.0).round() as i32, zoom });
        if l.alpha < 255 {
            self.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: l.alpha * 256 / 255 });
        }
        let (blend, filter) = shader(l.shader);
        // spblend stays through lsp and goes with csp.
        if blend != self.blends[layer as usize] {
            self.ops.push(Op::SpriteBlend { layer, mode: blend });
            self.blends[layer as usize] = blend;
        }
        // spclip stays through lsp: a full-screen one draws everywhere again.
        let clip = clip(l.clip);
        if clip != self.clips[layer as usize] {
            let rect = clip.unwrap_or((0, 0, SCREEN_W as i32, SCREEN_H as i32));
            self.ops.push(Op::SpriteClip { layer, rect });
            self.clips[layer as usize] = clip;
        }
        let tint = l.tint.map(|c| SpriteFilter {
            kind: Operand::Const(FILTER_COLOUR),
            blend: Operand::Const(FILTER_BLEND_MULTIPLY),
            strength: Operand::Const(256),
            params: [c >> 16, (c >> 8) & 0xFF, c & 0xFF].map(|v| Operand::Const(v as i32)),
        });
        let filters: Vec<SpriteFilter> = filter.into_iter().chain(tint).collect();
        if !filters.is_empty() {
            self.ops.push(Op::SpriteFilters { layer, filters });
        }
    }
}

/// A layer's `_STCH` rectangle on this screen (`FUN_00189ad0`): a
/// scissor on the 1920 × 1080 screen, inside its frame (the frame's width
/// off each side, across if narrower than the screen and down if lower).
// shortcut: the frame (argument 17's colour: 1, 7 black, 2 white, …) is not drawn, and a turned layer's rectangle isn't turned.
fn clip([x, y, w, h, frame]: [i32; 5]) -> Option<(i32, i32, i32, i32)> {
    // src: exe 0x189ad0 (a scissor in 1920 × 1080 pixels)
    if w == 0 || h == 0 || (w, h) == (1920, 1080) {
        return None;
    }
    let (fx, fy) = (if w < 1920 { frame.max(0) } else { 0 }, if h < 1080 { frame.max(0) } else { 0 });
    let px = |v: i32| (v as f32 * SCREEN_W as f32 / 1920.0).round() as i32;
    Some((px(x + fx), px(y + fy), px(w - 2 * fx).max(0), px(h - 2 * fy).max(0)))
}

/// The `F`, `G` and `C` flags the scripts name, as IR variables.
fn variables(g: &Game) -> HashMap<String, Var> {
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
        .collect()
}

pub fn compile(g: &Game) -> Compiled {
    let scene_labels = g.scenes.iter().zip(&g.scripts).map(|(&n, (name, _))| (n, label(name, None))).collect();
    let mut l = Lowering {
        g,
        ops: Vec::new(),
        notes: BTreeMap::new(),
        metrics: Metrics::default(),
        vars: variables(g),
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
        voice: None,
        held: None,
        se_channels: HashMap::new(),
        options: Vec::new(),
    };
    for (name, commands) in &g.scripts {
        l.ops.push(Op::Label(label(name, None)));
        for c in commands {
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
            l.metrics.call(c.opcode(), reasons);
        }
    }
    Compiled { ops: l.ops, notes: l.notes, metrics: l.metrics }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typemoon::script;

    fn picture(name: &str, w: f32, h: f32) -> (String, Picture) {
        (name.into(), Picture { name: name.into(), width: w, height: h, centre: (w / 2.0, h / 2.0), flags: 0, expression: false, shot: 0, pose_value: 0 })
    }

    #[test]
    fn layers_are_drawn_in_priority_order() {
        let src = "_STCH(1,a,240,136);_STCH(2,b,240,136);_FADS(1,0,0);_STCP(1,50);_FADS(1,0,0);_STCP(2,0);_FADS(1,0,0);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("a", 10.0, 10.0), picture("b", 10.0, 10.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
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
    fn stz4_zooms_the_staged_picture() {
        let src = "_STCH(1,img1,240,136);_STZ4(1,1000,1000,2000,-500,0,0,0);_FADS(1,0,0);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: [("img1".into(), (2560, 1440))].into(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let ops = compile(&g).ops;
        assert!(ops.contains(&Op::SpriteAngle { layer: 39, degrees: 0, zoom: (100, -25) }), "{ops:?}");
        // Argument 7 keeps the zoom from the draw; static ones read every argument.
        let src = "_STCH(1,img1,240,136);_STZ4(1,1000,1000,2000,-500,0,0,1);_FADS(1,0,0);";
        let g = Game { scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())], ..g };
        let c = compile(&g);
        assert!(c.ops.contains(&Op::SpriteAngle { layer: 39, degrees: 0, zoom: (50, 50) }), "{:?}", c.ops);
        assert!(!c.metrics.reasons().keys().any(|r| r.contains("_STZ4")), "{:?}", c.metrics.reasons());
    }

    #[test]
    fn stgs_fades_from_the_next_fads() {
        let src = "_STCH(1,img1,240,136,1000,0);_STGS(1,,255,1000,0);_FADS(1,0,0);_FADS(1,0,0);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let ops = compile(&g).ops;
        let motions: Vec<_> = ops.iter().filter(|o| matches!(o, Op::SpriteMotion2 { .. })).collect();
        assert_eq!(motions.len(), 1, "{ops:?}");
        let Op::SpriteMotion2 { layer: 39, keys, end: MotionEnd::Keep } = motions[0] else { panic!("{:?}", motions[0]) };
        assert_eq!(keys.iter().map(|k| (k.ms, k.opacity)).collect::<Vec<_>>(), [(0, 0), (1000, 256)]);
        assert!(!ops.iter().any(|o| matches!(o, Op::SpriteOpacity { .. })), "{ops:?}");
    }

    #[test]
    fn stma_runs_the_exe_motion_from_the_layer() {
        use crate::typemoon::exe::{Key, Motion};
        let key = |ms, set: &[(usize, i16)]| {
            let mut fields = [Key::UNSET; 30];
            for &(at, v) in set {
                fields[(at - 4) / 2] = v;
            }
            Key { time_ms: ms, fields }
        };
        // y 136 -> 146 (10 script units = 26.5 px), zoom 1000 -> 2000 ‰, alpha 255 -> 0.
        let motion = Motion { length_ms: 500, keys: vec![key(0, &[(0x0A, 136), (0x0C, 255)]), key(500, &[(0x0A, 146), (0x0C, 0), (0x18, 2000), (0x1A, 2000)])] };
        let src = "_STCH(1,img1,240,136);_FADS(1,0,0);_STMA(1,1);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![motion],
        };
        let ops = compile(&g).ops;
        let Some(Op::SpriteMotion2 { layer: 39, keys, end: MotionEnd::Keep }) = ops.last() else { panic!("{ops:?}") };
        assert_eq!(keys.iter().map(|k| (k.ms, k.y, k.zoom, k.zoom_y, k.opacity)).collect::<Vec<_>>(), [(0, 0, 100, 100, 256), (500, 26, 200, 200, 0)]);
    }

    #[test]
    fn a_child_is_placed_in_its_parent() {
        // Parent img1 (the screen's size) at half size about the middle; the
        // child at its parent's top-left quarter point.
        let src = "_STCH(1,img1,240,136,1000,255,10);_STZ4(1,500,500,500,500,0,0,0);_STCH(2,dot,120,68,1000,255,20,,CC,,,1);_FADS(1,0,0);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            textures: [("img1".into(), (2560, 1440)), ("dot".into(), (20, 20))].into(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let ops = compile(&g).ops;
        // 240 + 0.5 × (120 − 240) = 180 script units: 480 px; 136 + 0.5 × (68 − 135.75) ≈ 102.1: 270 px.
        assert!(ops.contains(&Op::SpriteMove { layer: 38, x: 480, y: 270, ms: 0 }), "{ops:?}");
        assert!(ops.contains(&Op::SpriteAngle { layer: 38, degrees: 0, zoom: (25, 25) }), "{ops:?}");
    }

    #[test]
    fn a_panel_is_clipped_to_its_rectangle() {
        // img1 in a 960 × 540 panel framed 60 wide; the child in it, then
        // img1 again without one.
        let src = "_STCH(1,img1,240,136,1000,255,10,,,,,,480,270,960,540,60);_STCH(2,dot,10,10,1000,255,20,,,,,1);_FADS(1,0,0);_STCH(1,img1,240,136,1000,255,10,,,,,,0,0,0,0,0);_STCH(2,);_FADS(1,0,0);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let clips: Vec<_> = compile(&g).ops.into_iter().filter(|o| matches!(o, Op::SpriteClip { .. })).collect();
        let panel = (360, 220, 560, 280);
        assert_eq!(clips, [Op::SpriteClip { layer: 38, rect: panel }, Op::SpriteClip { layer: 39, rect: panel }, Op::SpriteClip { layer: 39, rect: (0, 0, 1280, 720) }]);
    }

    #[test]
    fn a_child_follows_its_parents_motion() {
        use crate::typemoon::exe::{Key, Motion};
        let key = |ms, set: &[(usize, i16)]| {
            let mut fields = [Key::UNSET; 30];
            for &(at, v) in set {
                fields[(at - 4) / 2] = v;
            }
            Key { time_ms: ms, fields }
        };
        // The parent goes 60 script units right (160 px) and fades out.
        let motion = Motion { length_ms: 500, keys: vec![key(0, &[(0x08, 240), (0x0C, 255)]), key(500, &[(0x08, 300), (0x0C, 0)])] };
        let src = "_STCH(1,img1,240,136,1000,255,10);_STCH(2,dot,300,136,1000,128,20,,,,,1);_FADS(1,0,0);_STMA(1,1);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![motion],
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
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0), picture("dot", 10.0, 10.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let ops = compile(&g).ops;
        assert!(ops.contains(&Op::SpriteMove { layer: 38, x: 639, y: 519, ms: 0 }), "{ops:?}");
        assert!(ops.iter().any(|o| matches!(o, Op::SpriteAngle { layer: 38, degrees: 0, .. })), "{ops:?}");
    }

    #[test]
    fn turn_shader_and_tint() {
        let src = "_STCH(1,img1,240,136);_STRT(1,-9000,-9000,0,0);_STBR(1,18);_STEC(1,1,0xFF8000);_FADS(1,0,0);_STCH(1,img1,240,136);_FADS(1,0,0);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: [("img1".into(), (2560, 1440))].into(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        let ops = c.ops;
        assert!(ops.contains(&Op::SpriteAngle { layer: 39, degrees: 90, zoom: (50, 50) }), "{ops:?}");
        let blends: Vec<_> = ops.iter().filter_map(|o| if let Op::SpriteBlend { mode, .. } = o { Some(*mode) } else { None }).collect();
        // Added on, then normal again: `_STCH` resets the shader.
        assert_eq!(blends, [BLEND_ADD, BLEND_NORMAL]);
        let Some(Op::SpriteFilters { filters, .. }) = ops.iter().find(|o| matches!(o, Op::SpriteFilters { .. })) else { panic!("{ops:?}") };
        assert_eq!(filters[0].params, [255, 128, 0].map(Operand::Const));
    }

    #[test]
    fn a_choice_goes_to_its_label() {
        let src = "_SELR(0,$000000);_SELR(1,$000001,1);_SELR(1,$000001);_SELD(_D000,);_GOTS(_D000,SEL0_0,SEL0_1);_ZZ05a01(SEL0_0);_IFP4(,LV00100);_ZY12800(LV00100);_IFNX(,LV00200);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec!["父の<事|こと>だ。", "妹の事だ。"],
            pictures: HashMap::new(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        assert_eq!(
            c.ops,
            [
                Op::Label("tm_S".into()),
                Op::Choice { options: vec![("父の事だ。".into(), "tm_S__SEL0_0".into()), ("妹の事だ。".into(), "tm_S__SEL0_1".into())], style: None },
                Op::Label("tm_S__SEL0_0".into()),
                Op::Jump("tm_S__LV00100".into()),
                Op::Label("tm_S__LV00100".into()),
            ]
        );
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
        assert_eq!(number("F1"), None);
        assert_eq!(option_at(&["", "`001:4000", "`017:50"], 17), Some((2, 50)));
        let (t, r) = text("@g　開け<肌寒|はだざむ>い^[ber00]風@k");
        assert_eq!(t, "　開け肌寒い\n風");
        assert_eq!(r, vec![Ruby { start: 3, len: 2, text: "はだざむ".into() }]);
    }

    #[test]
    fn a_picture_is_placed_by_its_centre_at_half_size() {
        let src = "_ZZ01501(START);_STCH(1,img1,240,136,1000,255,2,,CC);_FADS(0,500,0);_WNTY(1);_VPLY(A00,1);_ZM01801($000001@n);_MSAD($000000);_JUMP(110);";
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![110],
            lines: vec!["と。", "「<肌|はだ>"],
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: [("img1".into(), (2560, 1440))].into(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let c = compile(&g);
        assert!(c.notes.is_empty(), "{:?}", c.notes);
        let layer = 39;
        assert_eq!(
            c.ops,
            vec![
                Op::Label("tm_S".into()),
                Op::Label("tm_S__START".into()),
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
        let g = Game {
            scripts: vec![("S".into(), script::parse(src).into_iter().map(Result::unwrap).collect())],
            scenes: vec![],
            lines: vec![],
            pictures: [picture("img1", 1280.0, 724.0)].into(),
            textures: HashMap::new(),
            sounds: vec![],
            waves: HashMap::new(),
            motions: vec![],
        };
        let ops = compile(&g).ops;
        let moves: Vec<(i32, i32)> = ops.iter().filter_map(|o| if let Op::SpriteMove { x, y, .. } = o { Some((*x, *y)) } else { None }).collect();
        let zooms: Vec<_> = ops.iter().filter_map(|o| if let Op::SpriteAngle { zoom, .. } = o { Some(*zoom) } else { None }).collect();
        // Kept at 120, 68 and zoom 500; after `_STCC` a missing y is 0 and the zoom 1000.
        assert_eq!(moves, [(320, 180), (320, 0)]);
        assert_eq!(zooms, [(25, 25), (50, 50)]);
    }
}
