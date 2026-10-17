//! 月姫's scripts lowered to the IR: the flow (labels, jumps, flags), the
//! text, the picture layers and the sound. What isn't lowered yet is
//! counted in the notes (`docs/games/tsukihime.md`, "Status").
//!
//! Pictures: `_STCH` stages a picture on one of 40 layers and `_FADS`
//! shows the staged layers. A picture is drawn at its record's size on a
//! 1280 × 724 stage (the texture at half size), the record's centre at
//! the script's x, y × 8/3 ("The stage"). The screen is 1280 × 720.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use conv_core::ir::{ArithOp, Compare, MotionEnd, MotionKey, Op, Operand, Ruby, SpriteChange, SpritePlacement, TransitionStyle, Var, Voice};

use super::exe::Picture;
use super::script::Command;
use crate::metrics::Metrics;

pub const SCREEN_W: u32 = 1280;
pub const SCREEN_H: u32 = 720;
/// Locals (saved with each slot) below, globals from here.
pub const GLOBAL_BORDER: u32 = 800;
/// The stage's height in record units (its width is the screen's).
const STAGE_H: f32 = 724.0;
const LAYERS: usize = 40;
/// The `F` and `C` flags are locals from %196 (NScripter number 10 +
/// index), one each in name order; the `G` flags globals %800 + n.
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
    alpha: u32,
    /// `_STGS`'s fade, from the next `_FADS` on: (from, to, ms); `alpha` is
    /// already `to`.
    fade: Option<(u32, u32, u32)>,
    /// `_STMA` on a layer not shown yet: the motion, from the `_FADS` that shows it.
    motion: Option<usize>,
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

/// A backquote option `` `001:4000`` among `args`: option 1's value.
fn option(args: &[&str], n: u32) -> Option<i32> {
    args.iter().find_map(|a| {
        let (k, v) = a.strip_prefix('`')?.split_once(':')?;
        (k.parse::<u32>().ok()? == n).then(|| number(v)).flatten()
    })
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
fn colour(name: &str) -> Option<u32> {
    Some(match name {
        "black" => 0x000000,
        "white" => 0xFFFFFF,
        "red" => 0xFF0000,
        "green" => 0x00FF00,
        "blue" => 0x0000FF,
        "yellow" => 0xFFFF00,
        "cyan" => 0x00FFFF,
        "magenta" => 0xFF00FF,
        "gray" | "grey" => 0x808080,
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
    /// `_WNTY(1)`: the next line ends its page.
    page_end: bool,
    voice: Option<Voice>,
    /// A line ended with `@n`: the next one goes on after it, no click.
    held: Option<(String, Vec<Ruby>)>,
    se_channels: HashMap<i32, String>,
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

    /// Lowers one command; false when it isn't (yet).
    fn command(&mut self, script: &str, c: &Command) -> bool {
        let a = &c.args;
        let arg = |i: usize| a.get(i).copied().unwrap_or("").trim();
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
            "_STCH" => return self.stch(a),
            "_STCC" => {
                let n = number(arg(0)).unwrap_or(0);
                if n < 0 {
                    self.staged.iter_mut().for_each(|l| *l = None);
                } else if let Some(l) = self.staged.get_mut(n as usize) {
                    *l = None;
                }
            }
            // `_STZ4(layer, x0, y0, x1, y1, ms, …)` (`FUN_00162d20`): the zoom
            // in ‰ goes to x1, y1 (`_STCH` resets it to 1000).
            // shortcut: a timed zoom (2 uses) jumps to its end.
            "_STZ4" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                let Some(l) = self.staged[n as usize].as_mut() else { return true };
                let z = |i: usize, old: f32| if arg(i).is_empty() { Some(old) } else { number(arg(i)).map(|v| v as f32 / 1000.0) };
                let (Some(zx), Some(zy)) = (z(3, l.stretch.0), z(4, l.stretch.1)) else { return false };
                l.stretch = (zx, zy);
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
                l.fade = (ms > 16 && from != to).then_some((from, to, ms));
            }
            "_STMA" => {
                let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)) else { return false };
                return self.stma(n as usize, number(arg(1)).unwrap_or(0));
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

    fn stch(&mut self, a: &[&str]) -> bool {
        let arg = |i: usize| a.get(i).copied().unwrap_or("").trim();
        let Some(n) = number(arg(0)).filter(|n| (0..LAYERS as i32).contains(n)).map(|n| n as usize) else { return false };
        if arg(1).is_empty() {
            self.staged[n] = None;
            return true;
        }
        let old = self.staged[n].clone();
        let base = arg(1).to_lowercase();
        let num = |i: usize, default: f32| if arg(i).is_empty() { Some(default) } else { number(arg(i)).map(|v| v as f32) };
        let (Some(x), Some(y), Some(zoom), Some(alpha)) = (num(2, 240.0), num(3, 136.0), num(4, 1000.0), num(5, 255.0)) else { return false };
        let expression = if arg(9).is_empty() && arg(10).is_empty() {
            None
        } else {
            let (oa, ob) = old.as_ref().and_then(|l| l.expression).unwrap_or((0, 0));
            let e = |i: usize, d: u32| number(arg(i)).map_or(d, |v| v as u32);
            Some((e(9, oa), e(10, ob)))
        };
        self.staged[n] = Some(Layer { base, expression, x, y, zoom: zoom / 1000.0, stretch: (1.0, 1.0), alpha: alpha.clamp(0.0, 255.0) as u32, fade: None, motion: None });
        // Argument 6, else layer × 10 (`FUN_00160840`).
        self.priority[n] = if arg(6).is_empty() { n as i32 * 10 } else { number(arg(6)).unwrap_or(n as i32 * 10) };
        if !arg(11).is_empty() && arg(11) != "-1" {
            self.note("_STCH with a parent layer (placed on its own)");
        }
        true
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
            want[slot] = self.shown[i].clone();
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
        let fade_ms = if kind == 1 { 0 } else { ms.max(0) as u32 };
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
                (None, _) => self.ops.push(Op::Sprites { changes: vec![SpriteChange { layer, sprite: None, keep_filters: false }], fade_ms: 0 }),
            }
            if let Some(m) = motions[slot] {
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
    /// layer's place), and moves the layer's state to its last key.
    // shortcut: the keys' easing, priority (0x3C) and expression (0x32, 0x34) are not followed; straight lines.
    fn run_motion(&mut self, slot: usize, m: usize) {
        let (Some(start), Some(i)) = (self.slots[slot].clone(), self.slot_layer[slot]) else { return };
        let Ok((_, (x0, y0), _)) = self.look(&start) else { return };
        let mut at = start.clone();
        let mut keys = Vec::new();
        for k in &self.g.motions[m - 1].keys {
            if let Some(x) = k.x() {
                at.x = f32::from(x);
            }
            if let Some(y) = k.y() {
                at.y = f32::from(y);
            }
            if let Some(a) = k.alpha() {
                at.alpha = a.clamp(0, 255) as u32;
            }
            let (zx, zy) = k.zoom();
            at.stretch = (zx.map_or(at.stretch.0, |z| f32::from(z) / 1000.0), zy.map_or(at.stretch.1, |z| f32::from(z) / 1000.0));
            let Ok((_, (x, y), _)) = self.look(&at) else { return };
            // spmotion's zoom multiplies the place's (spzoom), about its centre.
            let rel = |z: f32, z0: f32| if z0 == 0.0 { 100 } else { (100.0 * z / z0).round().max(0.0) as u32 };
            keys.push(MotionKey {
                ms: k.time_ms,
                x: (x - x0).round() as i32,
                y: (y - y0).round() as i32,
                zoom: rel(at.stretch.0.abs(), start.stretch.0.abs()),
                angle: 0,
                opacity: at.alpha * 256 / 255,
                zoom_y: rel(at.stretch.1.abs(), start.stretch.1.abs()),
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
        // shortcut: the layer's state jumps to the motion's end; a picture drawn again mid-motion starts there.
        let end = |l: &mut Layer| (l.x, l.y, l.alpha, l.stretch) = (at.x, at.y, at.alpha, at.stretch);
        for l in [self.slots[slot].as_mut(), self.shown[i].as_mut(), self.staged[i].as_mut()].into_iter().flatten() {
            end(l);
        }
    }

    /// Where and how `l` is drawn: the image, its middle on the screen and
    /// its zoom (%); or the name when the picture is unknown.
    fn look(&self, l: &Layer) -> Result<Look, String> {
        // An expression the picture doesn't have (`_00_00` on a background): the picture.
        let name = match l.expression {
            Some((a, b)) => Some(format!("{}_{a:02}_{b:02}", l.base)).filter(|n| self.g.pictures.contains_key(n)).unwrap_or_else(|| l.base.clone()),
            None => l.base.clone(),
        };
        let picture = self.g.pictures.get(&name);
        let fill = colour(&name).filter(|_| picture.is_none_or(|p| p.flags & 3 == 3));
        Ok(match (picture, fill) {
            (_, Some(rgb)) => (format!(">{SCREEN_W},{SCREEN_H},#{rgb:06x}"), (SCREEN_W as f32 / 2.0, SCREEN_H as f32 / 2.0), (100, 100)),
            (Some(p), None) => {
                // The record's centre goes to the script's x, y (480 × 272
                // for the screen); zoom about it. Records are 1280 × 724
                // for the screen.
                let (sx, sy) = (SCREEN_W as f32 / 480.0, SCREEN_H as f32 / 272.0);
                let (zx, zy) = (l.zoom * l.stretch.0, l.zoom * l.stretch.1);
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
        self.ops.push(Op::SpriteAngle { layer, degrees: 0, zoom });
        if l.alpha < 255 {
            self.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: l.alpha * 256 / 255 });
        }
    }
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
        priority: (0..LAYERS as i32).map(|i| i * 10).collect(),
        shown_priority: (0..LAYERS as i32).map(|i| i * 10).collect(),
        slots: vec![None; LAYERS],
        slot_layer: vec![None; LAYERS],
        page_end: false,
        voice: None,
        held: None,
        se_channels: HashMap::new(),
    };
    for (name, commands) in &g.scripts {
        l.ops.push(Op::Label(label(name, None)));
        for c in commands {
            let done = l.command(name, c);
            if !done {
                l.note(format!("not converted: {}", c.opcode()));
            }
            l.metrics.call(c.opcode(), if done { vec![] } else { vec!["not converted".into()] });
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
    fn numbers_and_markup() {
        assert_eq!(number("258-18"), Some(240));
        assert_eq!(number("-5"), Some(-5));
        assert_eq!(number("F1"), None);
        assert_eq!(option(&["", "`001:4000", "`017:50"], 17), Some(50));
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
}
