//! Compiles NSS story scenes into the IR.
//!
//! A story scene is a straight run of calls to the NSS helpers in
//! `function*.nss` (`CreateTextureEX`, `FadeDelete`, `SetTextEXC`,
//! `TypeBeginCIO`, `OnSE`, …), which in turn call the engine's built-ins.
//! The compiler runs the scene at convert time: helpers are inlined with
//! their parameters bound by token (see [`super::nss`]), variables and the
//! engine's named objects (textures, colours, sounds, text) are tracked, and
//! each built-in updates that model. What the player would see is turned
//! into IR ops at the points where the original waits:
//!
//! - **Pictures.** Every visible texture or colour object is a sprite layer,
//!   ordered by draw priority (higher priority = lower layer = on top;
//!   equal priority: the newer one on top). A blocking `Fade`/`Move`, a
//!   `Wait` or a text page shows the changes since the last such point as
//!   one cross-fade over the action's time: fading one object from a to b
//!   is exactly a cross-fade of the whole picture. Non-blocking actions
//!   (wait = false) join the next blocking point.
//! - **`PrintBG`** freezes the screen in a `CreateEffect("Plain")` cover and
//!   rebuilds the scene under it; fading the cover out reveals the new
//!   scene. While a cover is opaque nothing changes on screen, so the cover
//!   itself is never drawn: its fade-out is the cross-fade.
//! - **Text.** A `<PRE>` block becomes a text object; `Request(text, Enter)`
//!   (in the `TypeBegin*` helpers) shows its pages. The text window is
//!   taken from the layout the `SetText*` helper gave the text
//!   (`LoadText` size and line pitch, `Rotate` 90° = vertical, `Move`) and
//!   from the visible `Hideable` texture behind it (`boxCC`, at its
//!   opacity). A blank line ends a page; `<k>` waits without clearing;
//!   `{code}` runs before the page it starts (seen under wine).
//! - **Sound.** `Request(snd, Play)` with the `SetVolume`/`SetLoop` calls
//!   after it (the `SoundPlay`/`MusicStart`/`OnSE` helpers) starts BGM or a
//!   sound effect; `SetVolume(…, 0)` on a playing sound stops it.
//!
//! Conditions are evaluated with the values known at convert time (a fresh
//! game: unset variables are empty), which holds for linear scenes. What
//! isn't converted is counted in [`Compiled::notes`].

use std::collections::{BTreeMap, HashMap};

use conv_core::ir::{CLICK_WAIT_MARK, Op, Ruby, SpriteChange, SpritePlacement, TextWindow, TransitionStyle, Voice};

use super::nss::{AssignOp, BinaryOp, Expr, Function, Inline, Item, NssFile, Param, Stmt, StmtKind, Target, Text, UnaryOp};

pub const SCREEN_W: i64 = 1024;
pub const SCREEN_H: i64 = 576;
/// The story driver; the helper library is what it `#include`s.
const BOOT: &str = "0_boot_開始スクリプト.nss";
/// Highest IR sprite layer used.
const MAX_LAYER: u32 = 900;
/// Guard against scripts that loop forever at convert time.
const MAX_ITERATIONS: usize = 10_000;
const MAX_DEPTH: usize = 64;

/// Where the compiler gets the game's files from and where it puts them.
/// Paths returned are inside the output directory, `/`-separated.
pub trait Assets {
    /// An image the scripts name (`cg/bg/bg001_空a_01.jpg`): its output path
    /// and size, or `None` if the game has no such file.
    fn image(&mut self, path: &str) -> Option<(String, u32, u32)>;
    /// A sound the scripts name without extension (`sound/se/x`,
    /// `voice/ma01/x`): its output path.
    fn sound(&mut self, path: &str) -> Option<String>;
    /// A `w`×`h` picture of one colour (0xRRGGBB).
    fn color(&mut self, rgb: u32, w: u32, h: u32) -> String;
    /// Output images (path, top-left x, y, opacity 0-1000) drawn in order
    /// into one picture over their bounding box: its path and top-left.
    fn composite(&mut self, parts: &[(String, i64, i64, i64)]) -> Option<(String, i64, i64)>;
    /// A fully transparent picture.
    fn transparent(&mut self) -> String;
}

/// The result of compiling one scene.
#[derive(Debug, Default)]
pub struct Compiled {
    pub ops: Vec<Op>,
    /// The text window of the first page (the script's initial window).
    pub window: Option<TextWindow>,
    /// What was left out or approximated, with counts.
    pub notes: BTreeMap<String, usize>,
}

/// Compiles scene `scene` of `file` (`ma01_001.nss`, `ma01_001.nss`).
pub fn compile_scene(files: &BTreeMap<String, NssFile>, file: &str, scene: &str, assets: &mut dyn Assets) -> Result<Compiled, String> {
    let (file, _) = files.get_key_value(file).ok_or_else(|| format!("no script {file}"))?;
    let mut c = Compiler::new(files, file, assets);
    let body = c.scene(file, scene).ok_or_else(|| format!("no scene {scene} in {file}"))?;
    c.run_top(body);
    c.sync(0);
    Ok(Compiled { ops: c.ops, window: c.first_window, notes: c.notes })
}

/// Files the story driver includes, directly or not, in include order.
pub fn library(files: &BTreeMap<String, NssFile>) -> Vec<&str> {
    fn visit<'a>(files: &'a BTreeMap<String, NssFile>, name: &str, out: &mut Vec<&'a str>) {
        let Some((key, file)) = files.get_key_value(name) else { return };
        if out.contains(&key.as_str()) {
            return;
        }
        out.push(key);
        for item in &file.items {
            if let Item::Include(path) = item {
                let name = path.rsplit('/').next().unwrap_or(path);
                visit(files, name, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(files, BOOT, &mut out);
    out.retain(|&f| f != BOOT);
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Int(i64),
    Str(String),
    /// `@n`: relative to the current value.
    Rel(i64),
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Int(n) | Value::Rel(n) => *n != 0,
            Value::Str(s) => !s.is_empty(),
        }
    }

    fn int(&self) -> i64 {
        match self {
            Value::Null => 0,
            Value::Int(n) | Value::Rel(n) => *n,
            Value::Str(s) => s.trim().parse().unwrap_or(0),
        }
    }

    fn string(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Int(n) | Value::Rel(n) => n.to_string(),
            Value::Str(s) => s.clone(),
        }
    }

    fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Null, Value::Str(s)) | (Value::Str(s), Value::Null) => s.is_empty(),
            (Value::Str(s), n) | (n, Value::Str(s)) => s.trim().parse::<i64>().is_ok_and(|v| v == n.int()),
            (a, b) => a.int() == b.int(),
        }
    }

    /// A position argument: a number, `@n`, or a name such as `Center` for
    /// an object `size` px wide on a `screen` px axis.
    fn position(&self, current: i64, size: i64, screen: i64) -> i64 {
        match self {
            Value::Int(n) => *n,
            Value::Rel(d) => current + d,
            Value::Null => current,
            Value::Str(s) => match s.to_ascii_lowercase().as_str() {
                "center" | "middle" => (screen - size) / 2,
                "inleft" | "left" | "intop" | "top" => 0,
                "inright" | "right" | "inbottom" | "bottom" => screen - size,
                "outleft" | "outtop" => -size,
                "outright" | "outbottom" => screen,
                "onleft" | "ontop" => -size / 2,
                "onright" | "onbottom" => screen - size / 2,
                _ => s.trim().parse().unwrap_or(current),
            },
        }
    }
}

fn word(w: &str) -> Value {
    match w.to_ascii_lowercase().as_str() {
        "true" => Value::Int(1),
        "false" => Value::Int(0),
        "null" => Value::Null,
        _ => Value::Str(w.to_string()),
    }
}

fn color(v: &Value) -> Option<u32> {
    match v {
        Value::Int(n) => Some(*n as u32 & 0xFF_FFFF),
        Value::Str(s) => Some(match s.to_ascii_lowercase().as_str() {
            "black" => 0x000000,
            "white" => 0xFFFFFF,
            "red" => 0xFF0000,
            "green" => 0x00FF00,
            "blue" => 0x0000FF,
            other => u32::from_str_radix(other.trim_start_matches('#'), 16).ok()?,
        }),
        _ => None,
    }
}

#[derive(Debug, Clone)]
enum Kind<'a> {
    /// A texture or a colour: `image` is the output picture.
    Image { image: String },
    /// `CreateEffect("Plain")`: the screen frozen as it was.
    Cover,
    Text(TextObject<'a>),
    Sound(SoundObject),
    /// Names, windows and other objects that draw nothing themselves.
    Other,
}

#[derive(Debug, Clone)]
struct Object<'a> {
    kind: Kind<'a>,
    priority: i64,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
    /// 0-1000
    opacity: i64,
    locked: bool,
    /// Part of the text window (hidden with it).
    hideable: bool,
    /// Creation order: among equal priorities the newer draws on top.
    seq: u64,
}

#[derive(Debug, Clone)]
struct TextObject<'a> {
    block: &'a Text,
    /// `LoadText`: box size, letter spacing, line pitch, font size.
    layout: Option<(i64, i64, i64, i64, i64)>,
    rotation: i64,
}

#[derive(Debug, Clone, Default)]
struct Page<'a> {
    text: String,
    ruby: Vec<Ruby>,
    /// Speaker name and voice file (`<voice name= src=>`).
    voice: Option<(String, String)>,
    /// Inline code to run before the page.
    code: Vec<&'a Stmt>,
    /// Ended by a `<k>` with code after it: the next page goes on below
    /// without clearing.
    continues: bool,
}

#[derive(Debug, Clone)]
struct SoundObject {
    /// As the script names it (`sound/bgm/mbgm29`).
    path: String,
    bgm: bool,
    volume: i64,
    looped: bool,
    /// Started since the last flush: the next calls set its volume and loop.
    starting: bool,
    fade_ms: u32,
    /// The output file while playing.
    playing: Option<String>,
    /// `Request(snd, Disused)`: forgotten once it has started (a one-shot
    /// ends by itself).
    disused: bool,
}

/// A sprite layer as the output currently shows it.
#[derive(Debug, Clone, PartialEq)]
struct Shown {
    layer: u32,
    image: String,
    x: i64,
    y: i64,
    opacity: i64,
}

enum Flow {
    Normal,
    Return,
    Break,
}

struct Compiler<'a, 'b> {
    files: &'a BTreeMap<String, NssFile>,
    library: Vec<&'a str>,
    assets: &'b mut dyn Assets,
    /// Keyed `$name` / `#name` (`$a[1]` for array elements).
    vars: HashMap<String, Value>,
    /// Token bindings of the functions being run, innermost last
    /// (`w:` + bare word, `s:` + string literal).
    frames: Vec<HashMap<String, Value>>,
    file: &'a str,
    objects: BTreeMap<String, Object<'a>>,
    aliases: HashMap<String, String>,
    seq: u64,
    shown: BTreeMap<String, Shown>,
    /// The longest non-blocking action since the last sync.
    pending_ms: u32,
    font_size: i64,
    window: Option<TextWindow>,
    first_window: Option<TextWindow>,
    window_visible: bool,
    texts_shown: usize,
    voice_classes: Vec<String>,
    ops: Vec<Op>,
    notes: BTreeMap<String, usize>,
    depth: usize,
}

impl<'a, 'b> Compiler<'a, 'b> {
    fn new(files: &'a BTreeMap<String, NssFile>, file: &'a str, assets: &'b mut dyn Assets) -> Self {
        Compiler {
            files,
            library: library(files),
            assets,
            vars: HashMap::new(),
            frames: Vec::new(),
            file,
            objects: BTreeMap::new(),
            aliases: HashMap::new(),
            seq: 0,
            shown: BTreeMap::new(),
            pending_ms: 0,
            font_size: 20,
            window: None,
            first_window: None,
            window_visible: false,
            texts_shown: 0,
            voice_classes: Vec::new(),
            ops: Vec::new(),
            notes: BTreeMap::new(),
            depth: 0,
        }
    }

    fn note(&mut self, what: impl Into<String>) {
        *self.notes.entry(what.into()).or_default() += 1;
    }

    // -- program structure --------------------------------------------------

    fn scene(&self, file: &str, name: &str) -> Option<&'a [Stmt]> {
        self.files.get(file)?.items.iter().find_map(|item| match item {
            Item::Scene(b) | Item::Chapter(b) if b.name == name => Some(b.body.as_slice()),
            _ => None,
        })
    }

    /// Current file first, then the library; later definitions win.
    fn function(&self, name: &str) -> Option<(&'a str, &'a Function)> {
        let find = |file: &'a str, exact: bool| {
            self.files.get(file)?.items.iter().rev().find_map(|item| match item {
                Item::Function(f) if f.name == name || (!exact && f.name.eq_ignore_ascii_case(name)) => Some((file, f)),
                _ => None,
            })
        };
        let order: Vec<&'a str> = std::iter::once(self.file).chain(self.library.iter().rev().copied()).collect();
        order.iter().find_map(|&f| find(f, true)).or_else(|| order.iter().find_map(|&f| find(f, false)))
    }

    /// Statements of the scene itself: sounds started by a statement are
    /// flushed after it (their volume and loop calls come right after the
    /// `Play`).
    fn run_top(&mut self, stmts: &'a [Stmt]) {
        for s in stmts {
            self.exec(s);
            self.flush_sounds();
        }
    }

    fn exec_block(&mut self, stmts: &'a [Stmt]) -> Flow {
        for s in stmts {
            match self.exec(s) {
                Flow::Normal => {}
                flow => return flow,
            }
        }
        Flow::Normal
    }

    fn exec(&mut self, stmt: &'a Stmt) -> Flow {
        match &stmt.kind {
            StmtKind::Expr(e) => {
                self.eval(e);
            }
            StmtKind::Assign { target, index, op, value } => {
                let mut key = target.to_string();
                for i in index {
                    key = format!("{key}[{}]", self.eval(i).string());
                }
                let value = value.as_ref().map(|v| self.eval(v)).unwrap_or(Value::Null);
                let old = self.vars.get(&key).cloned().unwrap_or(Value::Null);
                let new = match op {
                    AssignOp::Set => value,
                    AssignOp::Add => add(&old, &value),
                    AssignOp::Sub => Value::Int(old.int() - value.int()),
                    AssignOp::Mul => Value::Int(old.int() * value.int()),
                    AssignOp::Div => Value::Int(old.int().checked_div(value.int()).unwrap_or(0)),
                    AssignOp::Inc => Value::Int(old.int() + 1),
                    AssignOp::Dec => Value::Int(old.int() - 1),
                };
                self.vars.insert(key, new);
            }
            StmtKind::If { cond, then, otherwise } => {
                if self.eval(cond).truthy() {
                    return self.exec_block(then);
                } else if let Some(o) = otherwise {
                    return self.exec_block(o);
                }
            }
            StmtKind::While { cond, body } => {
                let mut n = 0;
                while self.eval(cond).truthy() {
                    n += 1;
                    if n > MAX_ITERATIONS {
                        self.note("while loop stopped after 10000 rounds");
                        break;
                    }
                    match self.exec_block(body) {
                        Flow::Normal => {}
                        Flow::Break => break,
                        Flow::Return => return Flow::Return,
                    }
                }
            }
            StmtKind::Select(_) => self.note("select (choices)"),
            StmtKind::Case { .. } => {}
            StmtKind::CallScene(t) | StmtKind::CallChapter(t) => {
                let chapter = matches!(stmt.kind, StmtKind::CallChapter(_));
                let target = match t {
                    Target::Name(n) => n.clone(),
                    Target::Var(v) => self.vars.get(&v.to_string()).map(Value::string).unwrap_or_default(),
                };
                self.call_scene(&target, chapter);
            }
            StmtKind::Return => return Flow::Return,
            StmtKind::Break => return Flow::Break,
            StmtKind::Label(_) => {}
            StmtKind::Block(b) => return self.exec_block(b),
            StmtKind::Text(t) => self.text_block(t),
            StmtKind::Tag(_) => self.note("voice outside a text block"),
            StmtKind::Invalid(_) => self.note("statement that doesn't parse"),
        }
        Flow::Normal
    }

    /// `@->scene`, `nss/file.nss->scene`, `nss/file.nss` (its chapter main).
    fn call_scene(&mut self, target: &str, chapter: bool) {
        let (file, scene) = match target.split_once("->") {
            Some(("@", scene)) => (self.file.to_string(), scene.to_string()),
            Some((file, scene)) => (file.trim_start_matches("nss/").to_string(), scene.to_string()),
            None if chapter || target.ends_with(".nss") => (target.trim_start_matches("nss/").to_string(), "main".to_string()),
            None => (self.file.to_string(), target.to_string()),
        };
        let Some((file, _)) = self.files.get_key_value(&file) else {
            self.note(format!("call_scene to a missing file {file}"));
            return;
        };
        let Some(body) = self.scene(file, &scene) else {
            self.note(format!("call_scene to a missing scene {scene}"));
            return;
        };
        if self.depth >= MAX_DEPTH {
            self.note("calls nested too deep");
            return;
        }
        let saved = std::mem::replace(&mut self.file, file);
        self.depth += 1;
        self.frames.push(HashMap::new());
        self.exec_block(body);
        self.frames.pop();
        self.depth -= 1;
        self.file = saved;
    }

    fn call_function(&mut self, file: &'a str, f: &'a Function, args: Vec<Value>) -> Value {
        if self.depth >= MAX_DEPTH {
            self.note("calls nested too deep");
            return Value::Null;
        }
        let mut frame = HashMap::new();
        let mut args = args.into_iter();
        for p in &f.params {
            let v = args.next().unwrap_or(Value::Null);
            match p {
                Param::Var(var) => {
                    self.vars.insert(var.to_string(), v);
                }
                Param::Word(w) => {
                    frame.insert(format!("w:{w}"), v);
                }
                Param::Str(s) => {
                    frame.insert(format!("s:{s}"), v);
                }
            }
        }
        let saved = std::mem::replace(&mut self.file, file);
        self.depth += 1;
        self.frames.push(frame);
        self.exec_block(&f.body);
        self.frames.pop();
        self.depth -= 1;
        self.file = saved;
        Value::Null
    }

    fn bound(&self, key: &str) -> Option<Value> {
        self.frames.last()?.get(key).cloned()
    }

    fn eval(&mut self, e: &'a Expr) -> Value {
        match e {
            Expr::Int(n) => Value::Int(*n),
            Expr::Float(f) => Value::Int(*f as i64),
            Expr::Str(s) => self.bound(&format!("s:{s}")).unwrap_or_else(|| Value::Str(s.clone())),
            Expr::Color(c) => Value::Int(*c as i64),
            Expr::Word(w) => self.bound(&format!("w:{w}")).unwrap_or_else(|| word(w)),
            Expr::Var(v) => self.vars.get(&v.to_string()).cloned().unwrap_or(Value::Null),
            Expr::Index { .. } => {
                let key = self.index_key(e);
                self.vars.get(&key).cloned().unwrap_or(Value::Null)
            }
            Expr::Call { name, args } => {
                let args: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
                match self.function(name) {
                    Some((file, f)) => self.call_function(file, f, args),
                    None => self.builtin(name, &args),
                }
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr);
                match op {
                    UnaryOp::Neg => Value::Int(-v.int()),
                    UnaryOp::Not => Value::Int(i64::from(!v.truthy())),
                    UnaryOp::Relative => Value::Rel(v.int()),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval(left);
                match op {
                    BinaryOp::Or if l.truthy() => return Value::Int(1),
                    BinaryOp::And if !l.truthy() => return Value::Int(0),
                    _ => {}
                }
                let r = self.eval(right);
                let b = |x: bool| Value::Int(i64::from(x));
                match op {
                    BinaryOp::Or | BinaryOp::And => b(r.truthy()),
                    BinaryOp::Eq => b(l.equals(&r)),
                    BinaryOp::Ne => b(!l.equals(&r)),
                    BinaryOp::Lt => b(l.int() < r.int()),
                    BinaryOp::Gt => b(l.int() > r.int()),
                    BinaryOp::Le => b(l.int() <= r.int()),
                    BinaryOp::Ge => b(l.int() >= r.int()),
                    BinaryOp::Add => add(&l, &r),
                    BinaryOp::Sub => Value::Int(l.int() - r.int()),
                    BinaryOp::Mul => Value::Int(l.int() * r.int()),
                    BinaryOp::Div => Value::Int(l.int().checked_div(r.int()).unwrap_or(0)),
                    BinaryOp::Rem => Value::Int(l.int().checked_rem(r.int()).unwrap_or(0)),
                }
            }
            Expr::Point { .. } | Expr::Path(_) => Value::Null,
        }
    }

    fn index_key(&mut self, e: &'a Expr) -> String {
        match e {
            Expr::Var(v) => v.to_string(),
            Expr::Index { base, index } => {
                let base = self.index_key(base);
                format!("{base}[{}]", self.eval(index).string())
            }
            other => other.to_string(),
        }
    }

    // -- objects ------------------------------------------------------------

    /// The object a name argument means: `$var` names the variable holding
    /// it, `@x` goes through the alias table.
    fn object_name(&self, v: &Value) -> String {
        let mut s = v.string();
        for _ in 0..4 {
            match self.vars.get(&s) {
                Some(v) if s.starts_with(['$', '#']) => s = v.string(),
                _ => break,
            }
        }
        while let Some(rest) = s.strip_prefix('@') {
            s = self.aliases.get(rest).cloned().unwrap_or_else(|| rest.to_string());
        }
        s
    }

    /// Objects matched by a name argument (a trailing `*` is a prefix
    /// wildcard).
    fn matching(&self, v: &Value) -> Vec<String> {
        let name = self.object_name(v);
        match name.strip_suffix('*') {
            Some(prefix) => self.objects.keys().filter(|k| k.starts_with(prefix)).cloned().collect(),
            None if self.objects.contains_key(&name) => vec![name],
            None => Vec::new(),
        }
    }

    fn create(&mut self, name: String, kind: Kind<'a>, priority: i64, w: i64, h: i64) -> &mut Object<'a> {
        self.seq += 1;
        let o = Object { kind, priority, x: 0, y: 0, w, h, opacity: 1000, locked: false, hideable: false, seq: self.seq };
        self.objects.insert(name.clone(), o);
        self.objects.get_mut(&name).unwrap()
    }

    fn delete(&mut self, name: &str) {
        let Some(o) = self.objects.remove(name) else { return };
        if let Kind::Sound(s) = o.kind
            && let Some(file) = s.playing
        {
            self.ops.push(if s.bgm { Op::BgmStop { fade_out_ms: 0 } } else { Op::SeStop { file: Some(file) } });
        }
    }

    fn builtin(&mut self, name: &str, a: &[Value]) -> Value {
        let arg = |i: usize| a.get(i).cloned().unwrap_or(Value::Null);
        match name.to_ascii_lowercase().as_str() {
            "createtexture" => {
                let name = self.object_name(&arg(0));
                let src = arg(4).string();
                let Some((image, w, h)) = self.texture_source(&src) else {
                    self.note(format!("CreateTexture from {}", if src.contains('/') { "a missing file" } else { "an object or the screen" }));
                    self.create(name, Kind::Other, arg(1).int(), 0, 0);
                    return Value::Null;
                };
                let (w, h) = (w as i64, h as i64);
                let o = self.create(name, Kind::Image { image }, arg(1).int(), w, h);
                o.x = arg(2).position(0, w, SCREEN_W);
                o.y = arg(3).position(0, h, SCREEN_H);
            }
            "createcolor" => {
                let name = self.object_name(&arg(0));
                let (w, h) = (arg(4).int().max(1), arg(5).int().max(1));
                let Some(rgb) = color(&arg(6)) else {
                    self.note("CreateColor with an unknown colour");
                    return Value::Null;
                };
                let image = self.assets.color(rgb, w as u32, h as u32);
                let o = self.create(name, Kind::Image { image }, arg(1).int(), w, h);
                o.x = arg(2).position(0, w, SCREEN_W);
                o.y = arg(3).position(0, h, SCREEN_H);
            }
            "createeffect" => {
                let name = self.object_name(&arg(0));
                let kind = if arg(6).string().eq_ignore_ascii_case("plain") {
                    Kind::Cover
                } else {
                    self.note(format!("CreateEffect {}", arg(6).string()));
                    Kind::Other
                };
                self.create(name, kind, arg(1).int(), SCREEN_W, SCREEN_H);
            }
            "createwindow" | "createname" => {
                let name = self.object_name(&arg(0));
                self.create(name, Kind::Other, arg(1).int(), 0, 0);
            }
            "createsound" => {
                let name = self.object_name(&arg(0));
                let bgm = arg(1).string().eq_ignore_ascii_case("bgm");
                let s = SoundObject { path: arg(2).string(), bgm, volume: 1000, looped: false, starting: false, fade_ms: 0, playing: None, disused: false };
                self.create(name, Kind::Sound(s), 0, 0, 0);
            }
            "fade" => {
                // Four arguments: no tempo.
                let wait = if a.len() == 4 { arg(3) } else { arg(4) };
                let targets = self.matching(&arg(0));
                let mut window_only = !targets.is_empty();
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    o.opacity = arg(2).position(o.opacity, 0, 0).clamp(0, 1000);
                    window_only &= o.hideable;
                }
                // The text window's own fades aren't waited for: the window
                // just shows or hides.
                if !window_only {
                    self.timed(arg(1).int(), &wait);
                } else if !matches!(wait, Value::Null | Value::Int(0)) {
                    self.check_window();
                }
            }
            "move" => {
                let targets = self.matching(&arg(0));
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    o.x = arg(2).position(o.x, o.w, SCREEN_W);
                    o.y = arg(3).position(o.y, o.h, SCREEN_H);
                }
                if arg(1).int() > 0 && targets.iter().any(|t| self.shown.contains_key(t)) {
                    self.note("Move over time drawn as a cross-fade");
                }
                self.timed(arg(1).int(), &arg(5));
            }
            "delete" => {
                for t in self.matching(&arg(0)) {
                    if !self.objects[&t].locked {
                        self.delete(&t);
                    }
                }
            }
            "request" => {
                let action = arg(1).string().to_ascii_lowercase();
                for t in self.matching(&arg(0)) {
                    self.request(&t, &action);
                }
            }
            "setalias" => {
                let (obj, alias) = (self.object_name(&arg(0)), arg(1).string());
                if obj != alias {
                    self.aliases.insert(alias, obj);
                }
            }
            "rotate" => {
                for t in self.matching(&arg(0)) {
                    match &mut self.objects.get_mut(&t).unwrap().kind {
                        Kind::Text(text) => text.rotation = arg(4).position(text.rotation, 0, 0),
                        _ if arg(4).int() != 0 => self.note("Rotate"),
                        _ => {}
                    }
                }
            }
            "zoom" if !(arg(2).int() == 1000 && arg(3).int() == 1000) => self.note("Zoom"),
            "imagehorizon" => return Value::Int(self.objects.get(&self.object_name(&arg(0))).map_or(0, |o| o.w)),
            "imagevertical" => return Value::Int(self.objects.get(&self.object_name(&arg(0))).map_or(0, |o| o.h)),
            "setfont" => self.font_size = arg(1).int().max(1),
            "loadtext" => {
                let name = self.object_name(&arg(2));
                let font = self.font_size;
                if let Some(Object { kind: Kind::Text(t), .. }) = self.objects.get_mut(&name) {
                    t.layout = Some((arg(3).int(), arg(4).int(), arg(5).int(), arg(6).int(), font));
                }
            }
            "wait" => {
                let ms = arg(0).int().max(0) as u32;
                let shown = self.pending_ms.min(ms);
                self.sync(0);
                if ms > shown {
                    self.ops.push(Op::Wait { ms: ms - shown });
                }
            }
            "waitkey" => {
                self.sync(0);
                match arg(0) {
                    Value::Null => self.ops.push(Op::ClickWait { skippable: true }),
                    ms => self.ops.push(Op::Wait { ms: ms.int().max(0) as u32 }),
                }
            }
            "waitaction" | "waitfade" | "waitmove" | "waitzoom" | "draw" => self.sync(0),
            "setvolume" => self.set_volume(&arg(0), arg(1).int(), arg(2).int()),
            "setloop" => {
                for t in self.matching(&arg(0)) {
                    if let Kind::Sound(s) = &mut self.objects.get_mut(&t).unwrap().kind {
                        s.looped = arg(1).truthy();
                    }
                }
            }
            "setfrequency" if arg(2).int() != 1000 => self.note("SetFrequency"),
            "passagetime" | "remaintime" | "durationtime" | "conquest" | "random" => return Value::Int(0),
            "modulefilename" => return Value::Str(self.file.to_string()),
            "strstr" => return Value::Int(i64::from(arg(0).string().contains(&arg(1).string()))),
            "integer" => return Value::Int(arg(0).int()),
            // Drawing and bookkeeping that changes nothing on screen.
            "setvertex" | "zoom" | "setfrequency" | "setpan" | "waittext" | "waitplay" | "lockvideo" | "reset" | "setshortcut" => {}
            other => self.note(format!("built-in {other}")),
        }
        Value::Null
    }

    fn texture_source(&mut self, src: &str) -> Option<(String, u32, u32)> {
        if src.contains('/') {
            return self.assets.image(src);
        }
        let name = self.object_name(&Value::Str(src.to_string()));
        match self.objects.get(&name) {
            Some(Object { kind: Kind::Image { image }, w, h, .. }) => Some((image.clone(), *w as u32, *h as u32)),
            _ => None,
        }
    }

    fn request(&mut self, name: &str, action: &str) {
        let Some(o) = self.objects.get_mut(name) else { return };
        match action {
            "lock" => o.locked = true,
            "unlock" => o.locked = false,
            "hideable" => o.hideable = true,
            "play" => {
                if let Kind::Sound(s) = &mut o.kind {
                    s.starting = true;
                    s.fade_ms = 0;
                }
            }
            "disused" => match &mut o.kind {
                // Looping sounds play on until stopped; one-shots end by
                // themselves.
                Kind::Sound(s) if s.looped || s.starting => s.disused = true,
                Kind::Sound(_) => {
                    self.objects.remove(name);
                }
                _ if !o.locked => self.delete(name),
                _ => {}
            },
            "enter" => self.show_text(name),
            "smoothing" | "passive" | "erase" => {}
            other => self.note(format!("Request {other}")),
        }
    }

    // -- timing and drawing -------------------------------------------------

    /// An action of `ms` with the `wait` argument of the timed built-ins:
    /// true blocks for the action, false lets the script go on.
    fn timed(&mut self, ms: i64, wait: &Value) {
        let ms = ms.max(0) as u32;
        match wait {
            Value::Null | Value::Int(0) => self.pending_ms = self.pending_ms.max(ms),
            Value::Int(n) if *n > 1 => {
                self.note("timed action that waits only part of its time");
                self.pending_ms = self.pending_ms.max(ms);
                self.sync(0);
            }
            _ => {
                self.pending_ms = self.pending_ms.max(ms);
                self.sync(0);
            }
        }
    }

    /// The screen is frozen while an opaque `Plain` cover is up.
    fn frozen(&self) -> bool {
        self.objects.values().any(|o| matches!(o.kind, Kind::Cover) && o.opacity >= 1000)
    }

    /// Shows the changes since the last sync, as a cross-fade over the
    /// pending actions' time (or `ms`, if longer).
    fn sync(&mut self, ms: u32) {
        let ms = ms.max(std::mem::take(&mut self.pending_ms));
        self.flush_sounds();
        self.check_window();
        if self.frozen() {
            if ms > 0 {
                self.ops.push(Op::Wait { ms });
            }
            return;
        }
        // Wanted: visible pictures, by (priority, seq), top first.
        let mut want: Vec<(String, Shown, (i64, u64))> = self
            .objects
            .iter()
            .filter_map(|(n, o)| match &o.kind {
                Kind::Image { image } if o.opacity > 0 && !o.hideable => {
                    let shown = Shown { layer: 0, image: image.clone(), x: o.x, y: o.y, opacity: o.opacity };
                    Some((n.clone(), shown, (o.priority, o.seq)))
                }
                _ => None,
            })
            .collect();
        want.sort_by_key(|w| std::cmp::Reverse(w.2));

        let mut changes = Vec::new();
        let mut opacity = Vec::new();
        let gone: Vec<String> = self.shown.keys().filter(|n| !want.iter().any(|w| &w.0 == *n)).cloned().collect();
        for n in gone {
            let s = self.shown.remove(&n).unwrap();
            changes.push(SpriteChange { layer: s.layer, sprite: None, keep_filters: false });
        }
        // Layers of the wanted pictures in drawing order, top first: each
        // must be above (lower than) the next one.
        let mut layers: Vec<Option<u32>> = want.iter().map(|w| self.shown.get(&w.0).map(|s| s.layer)).collect();
        let mut crowded = false;
        for i in 0..want.len() {
            if layers[i].is_some() {
                continue;
            }
            let above = layers[..i].iter().flatten().max().map_or(0, |l| l + 1);
            let below = layers[i + 1..].iter().flatten().min().copied().unwrap_or(MAX_LAYER + 1);
            let ideal = ((30000 - want[i].2.0).clamp(0, 30000) * MAX_LAYER as i64 / 30000) as u32;
            if above >= below {
                crowded = true;
                break;
            }
            layers[i] = Some(ideal.clamp(above, below - 1));
        }
        // No gap left between two pictures: spread them all out again, in
        // the same order (pictures that move layer are drawn anew).
        if crowded {
            let n = want.len() as u32;
            layers = (0..n).map(|i| Some((i + 1) * MAX_LAYER / (n + 1))).collect();
        }
        let mut placed = Vec::new();
        let mut new_shown = Vec::new();
        for ((name, now, _), layer) in want.into_iter().zip(&layers) {
            let Some(layer) = *layer else { continue };
            let now = Shown { layer, ..now };
            match self.shown.get(&name) {
                Some(old) if *old == now => continue,
                Some(old) if old.layer == now.layer && old.image == now.image && old.x == now.x && old.y == now.y => {
                    opacity.push(Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                }
                old => {
                    if let Some(old) = old.filter(|o| o.layer != layer) {
                        changes.push(SpriteChange { layer: old.layer, sprite: None, keep_filters: false });
                    }
                    let sprite = SpritePlacement { image: now.image.clone(), x: now.x as i32, y: now.y as i32 };
                    placed.push(SpriteChange { layer, sprite: Some(sprite), keep_filters: false });
                    if now.opacity < 1000 {
                        opacity.push(Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                    }
                }
            }
            new_shown.push((name, now));
        }
        // Removals first: a placement may reuse a freed layer.
        changes.retain(|c| !placed.iter().any(|p| p.layer == c.layer));
        changes.extend(placed);
        self.shown.extend(new_shown);
        if changes.is_empty() && opacity.is_empty() {
            if ms > 0 {
                self.ops.push(Op::Wait { ms });
            }
            return;
        }
        self.ops.push(Op::BeginTransition { effect: "Fade".into(), fade_ms: ms, mask: None, style: TransitionStyle::Fade });
        if !changes.is_empty() {
            self.ops.push(Op::Sprites { changes, fade_ms: 0 });
        }
        self.ops.extend(opacity);
        self.ops.push(Op::EndTransition);
    }

    /// Hides the text window once no picture of it is visible.
    fn check_window(&mut self) {
        let visible = self.objects.values().any(|o| o.hideable && o.opacity > 0 && matches!(o.kind, Kind::Image { .. }));
        if self.window_visible && !visible {
            self.ops.push(Op::HideTextWindow);
            self.window_visible = false;
        }
    }

    // -- sound --------------------------------------------------------------

    fn set_volume(&mut self, target: &Value, ms: i64, volume: i64) {
        let ms = ms.max(0) as u32;
        for t in self.matching(target) {
            let Some(Object { kind: Kind::Sound(s), .. }) = self.objects.get_mut(&t) else { continue };
            if s.starting {
                s.volume = volume;
                s.fade_ms = ms;
            } else if s.playing.is_some() && volume == 0 {
                let file = s.playing.take().unwrap();
                self.ops.push(if s.bgm { Op::BgmStop { fade_out_ms: ms } } else { Op::SeStop { file: Some(file) } });
            } else if s.playing.is_some() {
                s.volume = volume;
                self.note("volume change of a playing sound");
            } else {
                s.volume = volume;
            }
        }
    }

    /// Starts the sounds whose `Play` came since the last flush.
    fn flush_sounds(&mut self) {
        let starting: Vec<String> =
            self.objects.iter().filter(|(_, o)| matches!(&o.kind, Kind::Sound(s) if s.starting)).map(|(n, _)| n.clone()).collect();
        for name in starting {
            let Kind::Sound(s) = &mut self.objects.get_mut(&name).unwrap().kind else { continue };
            s.starting = false;
            if s.volume <= 0 && s.fade_ms == 0 {
                continue;
            }
            let (path, bgm, looped, fade_ms) = (s.path.clone(), s.bgm, s.looped, s.fade_ms);
            let Some(file) = self.assets.sound(&path) else {
                self.note("sound missing from the archives");
                continue;
            };
            if bgm {
                // One BGM at a time.
                for o in self.objects.values_mut() {
                    if let Kind::Sound(other) = &mut o.kind
                        && other.bgm
                    {
                        other.playing = None;
                    }
                }
                if !looped {
                    self.note("BGM without loop played looping");
                }
                self.ops.push(Op::BgmPlay { file: file.clone(), fade_in_ms: fade_ms });
            } else {
                self.ops.push(Op::SePlay { file: file.clone(), looped });
            }
            if let Some(Object { kind: Kind::Sound(s), .. }) = self.objects.get_mut(&name) {
                if s.disused && !s.looped {
                    self.objects.remove(&name);
                } else {
                    s.playing = Some(file);
                }
            }
        }
    }

    // -- text ---------------------------------------------------------------

    /// A `<PRE @box>` block: replaces the current text object.
    fn text_block(&mut self, t: &'a Text) {
        let window = t.window.as_deref().unwrap_or("@box0").trim_start_matches('@').to_string();
        let marker = t.items.iter().find_map(|i| match i {
            Inline::Marker(m) => Some(m.clone()),
            _ => None,
        });
        let name = format!("{window}/{}", marker.unwrap_or_else(|| format!("text{}", self.seq)));
        if let Some(prev) = self.vars.get("$SYSTEM_present_text").map(Value::string) {
            let prev = prev.trim_start_matches('@').to_string();
            self.objects.remove(&prev);
        }
        let text = TextObject { block: t, layout: None, rotation: 0 };
        let priority = self.objects.get(&window).map_or(20001, |o| o.priority);
        self.create(name.clone(), Kind::Text(text), priority, 0, 0);
        self.vars.insert("$SYSTEM_present_preprocess".into(), Value::Str(format!("@{window}")));
        self.vars.insert("$SYSTEM_present_text".into(), Value::Str(format!("@{name}")));
    }

    /// Splits a text block into pages: a blank line ends a page, `<k>`
    /// waits mid-page. Lines holding only comments, markers, tags or code
    /// aren't lines of the page.
    fn pages(&mut self, t: &'a Text) -> Vec<Page<'a>> {
        let mut pages = Vec::new();
        let mut page = Page::default();
        let mut line = String::new();
        // The line had something other than text (so it isn't blank).
        let mut other = false;
        let mut ruby: Option<(usize, String)> = None;
        let page_len = |page: &Page<'a>, line: &str| {
            let n = page.text.chars().count();
            n + usize::from(n > 0) + line.chars().count()
        };
        let end_line = |page: &mut Page<'a>, line: &mut String| {
            if !page.text.is_empty() {
                page.text.push('\n');
            }
            page.text.push_str(&std::mem::take(line));
        };
        for item in &t.items {
            match item {
                Inline::Marker(_) | Inline::Comment(_) => other = true,
                Inline::Str(s) => line.push_str(s),
                Inline::Code(stmts) => {
                    // Code after a `<k>` runs after that click: end the page
                    // there and go on below it.
                    let mut text = page.text.clone();
                    if !line.is_empty() {
                        text = if text.is_empty() { line.clone() } else { format!("{text}\n{line}") };
                    }
                    if let Some(before) = text.strip_suffix(CLICK_WAIT_MARK) {
                        page.text = before.to_string();
                        line.clear();
                        page.continues = true;
                        pages.push(std::mem::take(&mut page));
                    } else if !page.text.is_empty() || !line.is_empty() {
                        self.note("code inside a page runs before it");
                    }
                    page.code.extend(stmts.iter());
                    other = true;
                }
                Inline::Tag(tag) => {
                    other = true;
                    match (tag.name.to_ascii_lowercase().as_str(), tag.close) {
                        ("voice", false) => {
                            let name = tag.attr("name").unwrap_or("").to_string();
                            let src = tag.attr("src").unwrap_or("").to_string();
                            page.voice = Some((name, src));
                        }
                        ("ruby", false) => ruby = Some((page_len(&page, &line), tag.attr("text").unwrap_or("").to_string())),
                        ("ruby", true) => {
                            if let Some((start, text)) = ruby.take() {
                                let len = page_len(&page, &line) - start;
                                page.ruby.push(Ruby { start, len, text });
                            }
                        }
                        ("br", _) => end_line(&mut page, &mut line),
                        // A click wait where it stands, mid-line or not.
                        ("k", _) => line.push(CLICK_WAIT_MARK),
                        ("pre", _) | ("font", _) => {}
                        (other, _) => self.note(format!("text tag <{other}>")),
                    }
                }
                Inline::Newline => {
                    if !line.is_empty() {
                        end_line(&mut page, &mut line);
                    } else if !other && !page.text.is_empty() {
                        pages.push(std::mem::take(&mut page));
                    }
                    other = false;
                }
            }
        }
        if !line.is_empty() {
            end_line(&mut page, &mut line);
        }
        if !page.text.is_empty() || !page.code.is_empty() {
            pages.push(page);
        }
        pages
    }

    /// `Request(text, Enter)`: shows the text object's pages.
    fn show_text(&mut self, name: &str) {
        let Some(Object { kind: Kind::Text(t), x, y, .. }) = self.objects.get(name) else { return };
        let (t, x, y) = (t.clone(), *x, *y);
        self.sync(0);
        let pages = self.pages(t.block);
        for page in &pages {
            for &s in &page.code {
                self.exec(s);
            }
            self.flush_sounds();
            if page.text.is_empty() {
                continue;
            }
            self.sync(0);
            // Code before the page may have changed the face window.
            let window = self.text_window(&t, x, y);
            if self.texts_shown == 0 {
                self.first_window = Some(window.clone());
            } else if self.window.as_ref() != Some(&window) {
                self.ops.push(Op::SetWindow(window.clone()));
            }
            self.window = Some(window);
            let voice = page.voice.as_ref().and_then(|(_, src)| {
                let file = self.assets.sound(src)?;
                Some(file)
            });
            let speaker = page.voice.as_ref().map(|(n, _)| n.clone()).filter(|n| !n.is_empty());
            let voice = voice.map(|file| {
                let class = speaker.clone().unwrap_or_default();
                let character = match self.voice_classes.iter().position(|c| *c == class) {
                    Some(i) => i,
                    None => {
                        self.voice_classes.push(class);
                        self.voice_classes.len() - 1
                    }
                } as u32;
                Voice { file, character }
            });
            // The face window or name plate (part of the window image)
            // shows who speaks; the name is kept for the voice switches.
            self.ops.push(Op::Text {
                speaker: None,
                text: page.text.clone(),
                voice,
                links: Vec::new(),
                ruby: page.ruby.clone(),
                styles: Vec::new(),
                hang: None,
                continues: page.continues,
            });
            self.texts_shown += 1;
        }
        self.window_visible = true;
    }

    /// The text window for a text object at (x, y): its `LoadText` box,
    /// turned vertical by a 90° `Rotate`, over the `Hideable` picture
    /// that is visible behind it.
    fn text_window(&mut self, t: &TextObject<'a>, x: i64, y: i64) -> TextWindow {
        let (w, h, letter, line, font) = t.layout.unwrap_or((550, 150, 0, 29, self.font_size));
        let columns = (w / (font + letter).max(1)).max(1) as u32;
        let rows = (h / line.max(1)).max(1) as u32;
        let vertical = t.rotation.rem_euclid(360) == 90;
        let (text_x, text_y) = if vertical { (x - rows as i64 * line, y) } else { (x, y) };
        // The window's pictures (box, face window, name plate), bottom
        // first, drawn into one window image.
        let mut parts: Vec<(i64, u64, (String, i64, i64, i64))> = self
            .objects
            .values()
            .filter(|o| o.hideable && o.opacity > 0)
            .filter_map(|o| match &o.kind {
                Kind::Image { image } => Some((o.priority, o.seq, (image.clone(), o.x, o.y, o.opacity))),
                _ => None,
            })
            .collect();
        parts.sort_by_key(|p| (p.0, p.1));
        let parts: Vec<_> = parts.into_iter().map(|p| p.2).collect();
        let (image, wx, wy) = match self.assets.composite(&parts) {
            Some(c) => c,
            None => (self.assets.transparent(), 0, 0),
        };
        TextWindow {
            image,
            x: wx as i32,
            y: wy as i32,
            text_x: text_x as i32,
            text_y: text_y as i32,
            columns,
            rows,
            font_size: font as u32,
            line_gap: (line - font).max(0) as u32,
            vertical,
            name_plate: None,
        }
    }
}

fn add(l: &Value, r: &Value) -> Value {
    match (l, r) {
        (Value::Str(_), _) | (_, Value::Str(_)) => Value::Str(l.string() + &r.string()),
        _ => Value::Int(l.int() + r.int()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nitroplus::nss;

    #[derive(Default)]
    struct Fake;

    impl Assets for Fake {
        fn image(&mut self, path: &str) -> Option<(String, u32, u32)> {
            Some((path.to_lowercase(), 1024, 576))
        }
        fn sound(&mut self, path: &str) -> Option<String> {
            Some(format!("{path}.ogg"))
        }
        fn color(&mut self, rgb: u32, w: u32, h: u32) -> String {
            format!("color/{rgb:06x}_{w}x{h}.png")
        }
        fn composite(&mut self, parts: &[(String, i64, i64, i64)]) -> Option<(String, i64, i64)> {
            let (_, x, y, _) = parts.first()?;
            let names: Vec<String> = parts.iter().map(|(i, _, _, o)| format!("{i}@{o}")).collect();
            Some((names.join("+"), *x, *y))
        }
        fn transparent(&mut self) -> String {
            "none.png".into()
        }
    }

    fn compile(story: &str, library: &str) -> Compiled {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse(library).unwrap());
        files.insert("a.nss".to_string(), nss::parse(story).unwrap());
        compile_scene(&files, "a.nss", "a.nss", &mut Fake).unwrap()
    }

    const LIBRARY: &str = r#"
function CreateTextureEX("ナット名", 描画優先度, $x, $y, $画像)
{
	CreateTexture("ナット名", 描画優先度, $x, $y, $画像);
	Fade("ナット名", 0, 0, null, true);
}
function FadeDelete("ナット名", 所要時間, 待ち)
{
	Fade("ナット名", 所要時間, 0, null, 待ち);
	Request("ナット名", UnLock);
	Request("ナット名", Disused);
}
function SetText()
{
	SetFont("@ＭＳ 明朝", 20, #FFFFFF, #222222, 500, RIGHTDOWN);
	LoadText("$構文名","$SYSTEM_present_preprocess","$SYSTEM_present_text",550,150,0,29);
	Rotate("$SYSTEM_present_text", 0, @0, @0, 90, null,true);
	Move("$SYSTEM_present_text", 0, 575, 55, null, true);
}
function TypeBegin()
{
	Fade("@box", 150, 800, null, true);
	Request($SYSTEM_present_text, Enter);
	Fade("@box", 150, 0, null, true);
}
"#;

    #[test]
    fn helpers_are_inlined_and_fades_become_transitions() {
        let story = r#"
scene a.nss
{
	CreateTexture("box", 20000, 447, 40, "cg/sys/adv/TextWindow.png");
	SetAlias("box", "box");
	Fade("box", 0, 0, null, false);
	Request("box", Hideable);
	CreateTextureEX("絵背景100", 10000, Center, Middle, "cg/bg/a.jpg");
	SoundPlay("@mbgm29", 0, 1000, true);
	Fade("絵背景100", 2000, 1000, null, true);
<PRE @box0>
[text0010]
　一行目
　二行目

　次のページ

{	OnSE("se01");}
　三ページ目
</PRE>
	SetText();
	TypeBegin();
	FadeDelete("絵背景100", 500, true);
}
"#;
        let c = compile(story, LIBRARY);
        let ops = &c.ops;
        let fade = ops.iter().position(|op| matches!(op, Op::BeginTransition { fade_ms: 2000, .. })).expect("2 s fade");
        assert!(matches!(&ops[fade + 1], Op::Sprites { changes, .. } if changes[0].sprite.as_ref().unwrap().image == "cg/bg/a.jpg"));
        let texts: Vec<&str> = ops.iter().filter_map(|op| if let Op::Text { text, .. } = op { Some(text.as_str()) } else { None }).collect();
        assert_eq!(texts, ["　一行目\n　二行目", "　次のページ", "　三ページ目"]);
        let w = c.window.unwrap();
        assert!(w.vertical);
        assert_eq!((w.x, w.y, w.text_x, w.text_y, w.rows, w.columns, w.line_gap), (447, 40, 430, 55, 5, 27, 9));
        assert_eq!(w.image, "cg/sys/adv/textwindow.png@800");
        assert!(ops.iter().any(|op| matches!(op, Op::HideTextWindow)));
        let last = ops.iter().rposition(|op| matches!(op, Op::BeginTransition { fade_ms: 500, .. })).expect("fade out");
        assert!(matches!(&ops[last + 1], Op::Sprites { changes, .. } if changes[0].sprite.is_none()));
        // SoundPlay and OnSE aren't defined here: built-ins we don't know.
        assert!(c.notes.contains_key("built-in soundplay"));
    }

    #[test]
    fn k_waits_in_place_and_code_after_it_runs_after_the_click() {
        let story = r#"
scene a.nss
{
<PRE @box0>
[text0010]
　怪我をする。<k>痛い。<k>
もがき苦しみ<k>
{	Wait(5);}
　兵士
</PRE>
	SetText();
	TypeBegin();
}
"#;
        let c = compile(story, LIBRARY);
        let texts: Vec<(&str, bool)> =
            c.ops.iter().filter_map(|op| if let Op::Text { text, continues, .. } = op { Some((text.as_str(), *continues)) } else { None }).collect();
        assert_eq!(texts, [("　怪我をする。\u{1}痛い。\u{1}\nもがき苦しみ", true), ("　兵士", false)]);
        let wait = c.ops.iter().position(|op| matches!(op, Op::Wait { ms: 5 })).unwrap();
        assert!(matches!(c.ops[wait - 1], Op::Text { continues: true, .. }));
    }

    #[test]
    fn a_plain_cover_turns_the_scene_change_into_one_fade() {
        let story = r#"
scene a.nss
{
	CreateTexture("絵背景100", 10000, 0, 0, "cg/bg/a.jpg");
	Wait(10);
	CreateEffect("上背景", 30000, 0, 0, 1024, 576, "Plain");
	Request("上背景", Lock);
	Delete("*");
	Request("上背景", UnLock);
	CreateColor("絵色100", 1500, 0, 0, 1024, 576, "Black");
	FadeDelete("上背景", 300, true);
}
"#;
        let c = compile(story, LIBRARY);
        let fades: Vec<u32> = c.ops.iter().filter_map(|op| if let Op::BeginTransition { fade_ms, .. } = op { Some(*fade_ms) } else { None }).collect();
        assert_eq!(fades, [0, 300]);
        let Some(Op::Sprites { changes, .. }) = c.ops.iter().rev().find(|op| matches!(op, Op::Sprites { .. })) else { panic!() };
        assert_eq!(changes.len(), 2, "{changes:?}");
        assert!(changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "color/000000_1024x576.png")));
    }
}
