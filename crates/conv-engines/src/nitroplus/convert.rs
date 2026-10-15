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

use conv_core::ir::{ArithOp, ChoiceStyle, CLICK_WAIT_MARK, Op, Operand, ScreenEffect, Ruby, Script, SpriteChange, SpritePlacement, TextWindow, TransitionStyle, Voice};

use super::nss::{AssignOp, BinaryOp, Expr, Function, Inline, Item, NssFile, Param, Stmt, StmtKind, Target, Text, UnaryOp};

/// 装甲悪鬼村正's screen ([`Game::screen`]).
pub const SCREEN_W: i64 = 1024;
/// The longest cross-fade non-blocking actions get where the script doesn't
/// wait for them (a text page, a blocking action): the original goes on
/// while they run, up to minutes (a background scrolling through a
/// battle).
const UNWAITED_MS: u32 = 1000;
pub const SCREEN_H: i64 = 576;
/// 装甲悪鬼村正's story driver ([`Game::boot`]); the helper library is what
/// it `#include`s.
const BOOT: &str = "0_boot_開始スクリプト.nss";
/// Highest IR sprite layer used.
const MAX_LAYER: u32 = 900;
/// The NScripter variables that hold which branch of a choice or of a
/// runtime `if` was taken (one per fork of a file, in turn).
const FORK_VARS: std::ops::Range<u16> = 40..70;
/// Which way through a fork the run took, as a number unique in the file
/// (set at the end of every branch; fork variables are renumbered when an
/// outer fork completes, this one isn't).
const LEAF_VAR: u16 = 70;
/// Story flags: locals from here (NScripter %300 up, past the variables
/// the backend and its menus use), globals (`#`) from the global border.
const FLAG_LOCAL_BASE: u16 = 290;
/// The first global NScripter variable (locals below it are saved with each
/// save slot, and the story flags need more than the default 200).
pub const GLOBAL_BORDER: u32 = 800;
/// Where a run of files ends.
const RUN_END: &str = "conv_run_end";
/// Set (in the convert-time variables) when a file assigns `$GameName`,
/// so that a file naming itself as the next one (a retry) is told apart
/// from one that names no next file.
const GAME_NAME_SET: &str = "$GameName (set)";
mod muramasa;
pub use muramasa::MURAMASA;
mod django;
pub use django::DJANGO;

/// What one game adds to the compiler, in its own module under `convert/`
/// ([`MURAMASA`]): library functions it compiles itself (written as
/// NScripter code instead of running their loops), scenes it replaces with
/// a port (a minigame), and the story flags its own code sets and tests.
/// The CLI picks the game when it builds.
pub struct Game {
    /// A call of a function: true if the game compiled it.
    call: for<'a, 'b> fn(&mut Compiler<'a, 'b>, &str) -> bool,
    /// A call of scene (file, scene): true if the game replaced it.
    scene: for<'a, 'b> fn(&mut Compiler<'a, 'b>, &str, &str) -> bool,
    /// Story flags the game adds.
    flags: fn() -> Vec<String>,
    /// Library files the story may name as its next file (ported).
    files: &'static [&'static str],
    /// The game's system screens (title, menus, dialogs), built from its
    /// own pictures into the script.
    menus: fn(&mut dyn Assets, &mut Script, &Story),
    /// Chapters compiled once more as subroutines (label, `nss/file.nss`)
    /// for the system screens to call, such as a screen the menu opens.
    subroutines: &'static [(&'static str, &'static str)],
    /// The screen's width and height.
    pub screen: (i64, i64),
    /// The boot script: the files it `#include`s are the helper library.
    boot: &'static str,
    /// The scene (file, scene) that plays the whole story by calling the
    /// chapter files ([`compile_story`]), for a game without
    /// 装甲悪鬼村正's `$GameName` chain.
    pub story: Option<(&'static str, &'static str)>,
    /// A child object (`frame03/顔窓31`) is placed from its parent's
    /// place, not the screen's corner (続・殺戮のジャンゴ's face windows).
    /// Off for 装甲悪鬼村正, whose children of cameras and windows weren't
    /// checked against it.
    relative_children: bool,
    /// `CreateTexture` from a `LoadImage` picture draws it (続・殺戮のジャンゴ's
    /// face-window frames). Off for 装甲悪鬼村正, whose choice plates come
    /// from `SetChoiceBase` and would be drawn twice.
    loaded_textures: bool,
    /// The objects a chapter file made are deleted when it ends, except
    /// those hung under an object made before it (続・殺戮のジャンゴ: its
    /// `窓` and `音` windows are made for pictures and sounds to "hang on
    /// between files"; a scene's fade to black `黒幕` is never deleted
    /// otherwise). Inferred from the scripts, not checked in the exe.
    chapter_objects: bool,
}

impl Game {
    /// A game with nothing of its own: NSS as it is.
    /// Its screen and boot script are 装甲悪鬼村正's.
    pub const PLAIN: Game = Game {
        call: |_, _| false,
        scene: |_, _, _| false,
        flags: Vec::new,
        files: &[],
        menus: |_, _, _| {},
        subroutines: &[],
        screen: (SCREEN_W, SCREEN_H),
        boot: BOOT,
        story: None,
        relative_children: false,
        loaded_textures: false,
        chapter_objects: false,
    };

    /// Fills `script`'s system screens.
    pub fn menus(&self, assets: &mut dyn Assets, script: &mut Script, story: &Story) {
        (self.menus)(assets, script, story)
    }
}

/// Guard against scripts that loop forever at convert time.
const MAX_ITERATIONS: usize = 10_000;
const MAX_DEPTH: usize = 64;
/// Statements one story file may run at convert time.
const MAX_STEPS: u64 = 2_000_000;

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
    /// An output image scaled to `zx`, `zy` per mille about the centre of
    /// the box `at` (x, y, w, h) it fills unscaled: the picture to draw
    /// (perhaps only its part on the screen), its top-left and size.
    fn scaled(&mut self, image: &str, zx: i64, zy: i64, at: (i64, i64, i64, i64)) -> Option<(String, i64, i64, u32, u32)>;
    /// An output image blurred with a Gaussian of `sigma`/10 px.
    fn blurred(&mut self, image: &str, sigma: u32) -> Option<String>;
    /// Images the scripts name, as cells side by side in one picture (a
    /// button's states): its path and size (all cells). Each cell is as
    /// large as the largest part, the part at its top left; a part `""` is
    /// an empty cell.
    fn strip(&mut self, parts: &[&str]) -> Option<(String, u32, u32)>;
    /// A movie the scripts name (`dx/mvOP.ngs`), or `None` if the game has
    /// no such file.
    fn movie_info(&mut self, path: &str) -> Option<MovieInfo>;
    /// The movie as a file the engine plays full screen, with its sound or
    /// without: its output path.
    fn movie_file(&mut self, path: &str, sound: bool) -> Option<String>;
    /// A movie's frames at [`MOVIE_FPS`] as cells side by side in one
    /// picture, or (`at`, ms) the one frame shown then. `alpha`: the
    /// movie's top half is the colour, its bottom half the opacity;
    /// `dark`: the colour is drawn black (`SubRender`). Pictures may be
    /// smaller than the movie: the picture, the number of cells and a
    /// cell's size.
    fn movie_frames(&mut self, path: &str, at: Option<u32>, alpha: bool, dark: bool) -> Option<(String, u32, u32, u32)>;
}

/// A movie's size and length, and whether its sound can be heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovieInfo {
    pub w: u32,
    pub h: u32,
    pub ms: u32,
    pub sound: bool,
}

/// Frames a second of a movie drawn as cells ([`Assets::movie_frames`]).
pub const MOVIE_FPS: u32 = 15;

/// The result of compiling one scene.
#[derive(Debug, Default)]
pub struct Compiled {
    pub ops: Vec<Op>,
    /// The text window of the first page (the script's initial window).
    pub window: Option<TextWindow>,
    /// What was left out or approximated, with counts.
    pub notes: BTreeMap<String, usize>,
    /// The story flags and their variables.
    pub flags: HashMap<String, conv_core::ir::Var>,
    /// The voice classes, in the order of their character numbers
    /// ([`Voice::character`]).
    pub voice_classes: Vec<String>,
}

/// What the system screens may need from the compiled story.
pub struct Story<'a> {
    pub flags: &'a HashMap<String, conv_core::ir::Var>,
    pub voice_classes: &'a [String],
}

/// Compiles scene `scene` of `file` (`ma01_001.nss`, `ma01_001.nss`).
pub fn compile_scene(files: &BTreeMap<String, NssFile>, file: &str, scene: &str, game: &'static Game, assets: &mut dyn Assets) -> Result<Compiled, String> {
    let (file, _) = files.get_key_value(file).ok_or_else(|| format!("no script {file}"))?;
    let mut c = Compiler::new(files, file, game, assets);
    let body = c.scene(file, scene).ok_or_else(|| format!("no scene {scene} in {file}"))?;
    let initial = c.snapshot();
    c.run_top(body);
    c.sync(0);
    let end = "conv_scene_end";
    c.ops.push(Op::Jump(end.to_string()));
    let base = c.next_dynamic.max(FLAG_LOCAL_BASE);
    c.subroutines(&initial, base);
    c.ops.push(Op::Label(end.to_string()));
    Ok(Compiled { ops: c.ops, window: c.first_window, notes: c.notes, flags: c.flags, voice_classes: c.voice_classes })
}

/// Compiles a whole story that one scene drives by calling the chapter
/// files in turn (続・殺戮のジャンゴ's `dja000_開始スクリプト.nss`,
/// `main`): [`compile_scene`] with the story flags kept at run time, so a
/// choice's flag decides what follows it when the game is played.
pub fn compile_story(files: &BTreeMap<String, NssFile>, file: &str, scene: &str, game: &'static Game, assets: &mut dyn Assets) -> Result<Compiled, String> {
    let (file, _) = files.get_key_value(file).ok_or_else(|| format!("no script {file}"))?;
    let mut c = Compiler::new(files, file, game, assets);
    c.whole_story = true;
    c.flags = story_flags(files, game);
    for (k, &v) in &c.flags {
        c.vars.insert(k.clone(), Value::Runtime(v));
    }
    c.next_dynamic = c.flags.values().filter(|v| !v.global).map(|v| v.index + 1).max().unwrap_or(FLAG_LOCAL_BASE);
    let body = c.scene(file, scene).ok_or_else(|| format!("no scene {scene} in {file}"))?;
    let initial = c.snapshot();
    c.run_top(body);
    c.sync(0);
    c.ops.push(Op::Jump(RUN_END.to_string()));
    let base = c.next_dynamic;
    c.subroutines(&initial, base);
    c.ops.push(Op::Label(RUN_END.to_string()));
    Ok(Compiled { ops: c.ops, window: c.first_window, notes: c.notes, flags: c.flags, voice_classes: c.voice_classes })
}

/// Plays story files from `start` (`ma01_001.nss`), as the story
/// driver's loop does: each file's `X.nss_MAIN` scene runs its content
/// scene and sets `$GameName` to the next file. A choice forks the run:
/// the next file of each branch is compiled from that branch's state, and
/// the end of the file jumps to it by the choice the player made. Each
/// file starts at a label named after it and is compiled once (from the
/// first state that reaches it). Stops after `max_files` files. Returns
/// the files compiled.
pub fn compile_run(files: &BTreeMap<String, NssFile>, start: &str, max_files: usize, game: &'static Game, assets: &mut dyn Assets) -> Result<(Compiled, Vec<String>), String> {
    let (first, _) = files.get_key_value(start).ok_or_else(|| format!("no script {start}"))?;
    let mut c = Compiler::new(files, first, game, assets);
    c.flags = story_flags(files, game);
    let dynamic_base = c.flags.values().filter(|v| !v.global).map(|v| v.index + 1).max().unwrap_or(FLAG_LOCAL_BASE);
    c.vars.insert("$GameName".into(), Value::Str(start.to_string()));
    c.vars.insert("$GameStart".into(), Value::Int(1));
    c.vars.insert("$GameCircle".into(), Value::Int(1));
    let initial = c.snapshot();
    let mut max_dynamic = dynamic_base;
    let mut queue = std::collections::VecDeque::from([(start.to_string(), c.snapshot())]);
    let mut done = std::collections::HashSet::new();
    let mut not_compiled = std::collections::BTreeSet::new();
    let mut played = Vec::new();
    // Ends of files whose next file is in a story flag: label, flag, state,
    // file names dispatched so far.
    let mut dispatches: Vec<(String, conv_core::ir::Var, State, std::collections::BTreeSet<String>)> = Vec::new();
    // The variables each file was compiled with, to report merges of paths
    // that differ.
    let mut entry_vars: HashMap<String, HashMap<String, Value>> = HashMap::new();
    loop {
        let Some((name, state)) = queue.pop_front() else {
            // Queue the file names the dispatched flags can hold by now, in
            // the order they were met (the output stays the same each run).
            let mut names: Vec<(i32, String)> = c.codes.iter().filter(|(k, _)| k.ends_with(".nss") && files.contains_key(*k)).map(|(k, &v)| (v, k.clone())).collect();
            names.sort();
            let names: Vec<String> = names.into_iter().map(|(_, k)| k).collect();
            for (_, _, state, seen) in &mut dispatches {
                for n in &names {
                    if seen.insert(n.clone()) {
                        let mut s = state.clone();
                        s.vars.insert("$GameName".into(), Value::Str(n.clone()));
                        queue.push_back((n.clone(), s));
                    }
                }
            }
            if queue.is_empty() {
                break;
            }
            continue;
        };
        if let Some(first) = entry_vars.get(&name) {
            let mut differ: Vec<&String> = first
                .keys()
                .chain(state.vars.keys())
                .filter(|k| !k.starts_with("$SYSTEM") && !k.contains("GameName") && first.get(*k) != state.vars.get(*k))
                .collect();
            differ.sort();
            differ.dedup();
            for k in differ {
                c.note(format!("paths merge with different {k}"));
            }
            continue;
        }
        if done.contains(&name) {
            continue;
        }
        let Some((file, _)) = files.get_key_value(&name) else {
            c.note(format!("next file {name} doesn't exist"));
            not_compiled.insert(name);
            continue;
        };
        if played.len() >= max_files {
            not_compiled.insert(name);
            continue;
        }
        // The helper library isn't a story file, unless the game ports it
        // (a minigame).
        if (c.library.contains(&file.as_str()) || file.starts_with("function")) && !game.files.contains(&file.as_str()) {
            c.note(format!("run reaches {name}: a minigame, not converted"));
            not_compiled.insert(name);
            continue;
        }
        done.insert(name.clone());
        not_compiled.remove(&name);
        entry_vars.insert(name.clone(), state.vars.clone());
        c.restore(state);
        // Story flags may hold anything when another path reaches the file.
        for (k, &v) in &c.flags {
            c.vars.insert(k.clone(), Value::Runtime(v));
        }
        // Helper variables kept at run time belong to the file that made
        // them; the next file sets them again before use.
        c.vars.retain(|k, v| !matches!(v, Value::Runtime(_)) || c.flags.contains_key(k));
        c.dynamic.clear();
        c.dynamic_values.clear();
        c.next_dynamic = dynamic_base;
        c.vars.remove(GAME_NAME_SET);
        c.forks_in_file = 0;
        c.leaves_in_file = 0;
        c.steps = 0;
        c.ops.push(Op::Label(file_label(&name)));
        c.file = file;
        if std::env::var_os("NSS_TRACE").is_some() {
            eprintln!("compiling {name}");
        }
        c.call_scene(&format!("nss/{name}->{name}_MAIN"), false);
        c.sync(0);
        max_dynamic = max_dynamic.max(c.next_dynamic);
        played.push(name.clone());
        // Branch states: the end of the file, with the variables each
        // branch of a choice set differently.
        let end = c.snapshot();
        let (fork_var, branches) = match c.pending_fork.take() {
            None => (None, vec![end]),
            Some((var, leaves, first)) => (Some(var), leaves.iter().map(|l| with_branch_vars(&end, &l.state, &first)).collect()),
        };
        let fork = branches.len() > 1;
        for (i, b) in branches.into_iter().enumerate() {
            let next = b.vars.get("$GameName").map(Value::string).unwrap_or_default();
            let target = if let Some(&Value::Runtime(var)) = b.vars.get("$GameName") {
                // The next file is in a story flag: decided by the dispatch
                // written at the end, over every file name the flag holds.
                let label = format!("dispatch{}", dispatches.len());
                dispatches.push((label.clone(), var, b, std::collections::BTreeSet::new()));
                label
            } else if !b.vars.get("$GameCircle").is_some_and(Value::truthy) {
                RUN_END.to_string()
            } else if (next == name && !b.vars.contains_key(GAME_NAME_SET)) || next.is_empty() {
                c.note("file didn't name a next file");
                if std::env::var_os("NSS_TRACE").is_some() {
                    eprintln!("{name}: no next file");
                }
                RUN_END.to_string()
            } else {
                let l = file_label(&next);
                if !done.contains(&next) {
                    not_compiled.insert(next.clone());
                }
                queue.push_back((next, b));
                l
            };
            if let (true, Some(var)) = (fork, fork_var) {
                c.ops.push(Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(i as i32), label: target });
            } else {
                c.ops.push(Op::Jump(target));
            }
        }
        if fork {
            c.ops.push(Op::Jump(RUN_END.to_string()));
        }
    }
    c.ops.push(Op::Jump(RUN_END.to_string()));
    c.subroutines(&initial, max_dynamic);
    for (label, var, _, seen) in &dispatches {
        c.ops.push(Op::Label(label.clone()));
        for n in seen {
            if let Some(&code) = c.codes.get(n) {
                let value = Operand::Const(code);
                c.ops.push(Op::If { var: *var, cmp: conv_core::ir::Compare::Eq, value, label: file_label(n) });
            }
        }
        c.ops.push(Op::Jump(RUN_END.to_string()));
    }
    // Files the run reaches but doesn't include end the game.
    for name in not_compiled {
        c.ops.push(Op::Label(file_label(&name)));
    }
    c.ops.push(Op::Label(RUN_END.to_string()));
    Ok((Compiled { ops: c.ops, window: c.first_window, notes: c.notes, flags: c.flags, voice_classes: c.voice_classes }, played))
}

/// Whether a block's last statement is `return`.
fn ends_in_return(stmts: &[Stmt]) -> bool {
    stmts.last().is_some_and(|s| matches!(s.kind, StmtKind::Return))
}

fn negate(c: conv_core::ir::Compare) -> conv_core::ir::Compare {
    use conv_core::ir::Compare::*;
    match c {
        Eq => Ne,
        Ne => Eq,
        Lt => Ge,
        Ge => Lt,
        Gt => Le,
        Le => Gt,
    }
}

/// `end` (the state after a fork's first branch went on) with the
/// variables `branch` set differently from that first branch, `first`,
/// unless the code after the fork set them again.
fn with_branch_vars<'a>(end: &State<'a>, branch: &State<'a>, first: &State<'a>) -> State<'a> {
    let mut s = end.clone();
    let untouched = |k: &String| end.vars.get(k) == first.vars.get(k);
    for (k, v) in branch.vars.iter().filter(|(k, _)| untouched(k)) {
        if first.vars.get(k) != Some(v) {
            s.vars.insert(k.clone(), v.clone());
        }
    }
    for k in first.vars.keys().filter(|k| !branch.vars.contains_key(*k) && untouched(k)) {
        s.vars.remove(k);
    }
    s
}

/// Story flags: variables the story files (not the helper library) both
/// set and test in a condition, such as route counters (`$Ichizyou_Flag`)
/// and unlocks after a clear (`#魔王編解放`). Their values depend on the
/// path through the game, so they are kept in NScripter variables. The
/// engine's own (`$SYSTEM_*`) and the file chaining (`$GameName`) are left
/// to the convert-time run.
/// The game adds the variables its own code sets and tests ([`Game`]).
pub fn story_flags(files: &BTreeMap<String, NssFile>, game: &Game) -> HashMap<String, conv_core::ir::Var> {
    use std::collections::BTreeSet;
    /// `$a["x"]` with a constant index: its key (as the compiler builds it).
    fn element(base: &Expr, index: &Expr) -> Option<String> {
        let i = match index {
            Expr::Str(s) => s.clone(),
            Expr::Int(n) => n.to_string(),
            _ => return None,
        };
        match base {
            Expr::Var(v) => Some(format!("{v}[{i}]")),
            _ => None,
        }
    }
    fn expr_vars(e: &Expr, out: &mut BTreeSet<String>) {
        match e {
            Expr::Var(v) => {
                out.insert(v.to_string());
            }
            Expr::Index { base, index } => {
                out.extend(element(base, index));
            }
            Expr::Unary { expr, .. } => expr_vars(expr, out),
            Expr::Binary { left, right, .. } => {
                expr_vars(left, out);
                expr_vars(right, out);
            }
            _ => {}
        }
    }
    fn walk(stmts: &[Stmt], set: &mut BTreeSet<String>, tested: &mut BTreeSet<String>) {
        for s in stmts {
            match &s.kind {
                StmtKind::Assign { target, index, .. } if index.is_empty() => {
                    set.insert(target.to_string());
                }
                StmtKind::Assign { target, index, .. } if index.len() == 1 => {
                    set.extend(element(&Expr::Var(target.clone()), &index[0]));
                }
                StmtKind::If { cond, then, otherwise } => {
                    expr_vars(cond, tested);
                    walk(then, set, tested);
                    if let Some(o) = otherwise {
                        walk(o, set, tested);
                    }
                }
                StmtKind::While { cond, body } => {
                    expr_vars(cond, tested);
                    walk(body, set, tested);
                }
                StmtKind::Select(b) | StmtKind::Block(b) | StmtKind::Case { body: b, .. } => walk(b, set, tested),
                StmtKind::Text(t) => {
                    for i in &t.items {
                        if let Inline::Code(c) = i {
                            walk(c, set, tested);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let library: Vec<&str> = library(files, game);
    let (mut set, mut tested) = (BTreeSet::new(), BTreeSet::new());
    for (name, file) in files {
        let story = !library.contains(&name.as_str()) && !["0_", "sys_", "extra_", "function"].iter().any(|p| name.starts_with(p));
        if !story {
            continue;
        }
        for item in &file.items {
            if let Item::Scene(b) | Item::Chapter(b) = item {
                walk(&b.body, &mut set, &mut tested);
            }
        }
    }
    for key in (game.flags)() {
        set.insert(key.clone());
        tested.insert(key);
    }
    let mut flags = HashMap::new();
    let (mut local, mut global) = (FLAG_LOCAL_BASE, 0u16);
    for name in set.intersection(&tested) {
        if name.starts_with("$SYSTEM") || name.starts_with("#SYSTEM") || name == "$GameName" || name == "$GameCircle" || name == "$AllRead" {
            continue;
        }
        let var = if name.starts_with('#') {
            global += 1;
            conv_core::ir::Var { global: true, index: global - 1 }
        } else {
            local += 1;
            conv_core::ir::Var { global: false, index: local - 1 }
        };
        flags.insert(name.clone(), var);
    }
    flags
}

/// Label of a story file's start.
fn file_label(name: &str) -> String {
    format!("f_{}", name.trim_end_matches(".nss"))
}

/// Files the story driver includes, directly or not, in include order.
pub fn library<'a>(files: &'a BTreeMap<String, NssFile>, game: &Game) -> Vec<&'a str> {
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
    visit(files, game.boot, &mut out);
    out.retain(|&f| f != game.boot);
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Int(i64),
    Str(String),
    /// `@n`: relative to the current value.
    Rel(i64),
    /// A story flag whose value is only known when the game runs: it lives
    /// in this NScripter variable.
    Runtime(conv_core::ir::Var),
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Null | Value::Runtime(_) => false,
            Value::Int(n) | Value::Rel(n) => *n != 0,
            Value::Str(s) => !s.is_empty(),
        }
    }

    fn int(&self) -> i64 {
        match self {
            Value::Null | Value::Runtime(_) => 0,
            Value::Int(n) | Value::Rel(n) => *n,
            Value::Str(s) => s.trim().parse().unwrap_or(0),
        }
    }

    fn string(&self) -> String {
        match self {
            Value::Null | Value::Runtime(_) => String::new(),
            Value::Int(n) | Value::Rel(n) => n.to_string(),
            Value::Str(s) => s.clone(),
        }
    }

    fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Runtime(_), _) | (_, Value::Runtime(_)) => false,
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
            Value::Null | Value::Runtime(_) => current,
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

/// The built-ins of Muramasa.exe, as `dispatch_builtin` (`00405250`,
/// statements) and `eval_primary` (`0043a780`, values) name them
/// (`research/nitroplus/builtins.md`, "In the exe"). The exe matches them
/// case-sensitively; any other name is an NSS function, and a call to a
/// function that doesn't exist is skipped.
const BUILTINS: [&str; 122] = [
    "CreateName", "CreateAnimation", "CreateProcess", "CreateColor", "CreateEffect",
    "CreateFrame", "CreateTexture", "CreateClipTexture", "CreateDifferenceTexture",
    "CreateMask", "CreateStencil", "CreateSurface", "CreateMovie", "CreateCube", "CreateSound",
    "CreateText", "CreateWindow", "CreateCamera", "CreateChoice", "CreateBacklog",
    "CreateScrollbar", "LoadImage", "LoadClipImage", "LoadColor", "LoadFont", "LoadText",
    "LoadFile", "UnloadFile", "LoadNPT", "Move", "BezierMove", "Request", "Rotate", "Shake",
    "MoveCamera", "MoveCube", "Delete", "Disable", "Fade", "SetChip", "SetAlias", "SetVertex",
    "SetShade", "SetTone", "SetImage", "SetSurface", "SetComposite", "SetBlur", "SetStream",
    "SetFov", "SetNextFocus", "SetScrollbar", "SetScrollSpeed", "SetBacklog", "Zoom", "Draw",
    "DrawEffect", "DrawTransition", "Wait", "WaitKey", "WaitMove", "WaitFade", "WaitZoom",
    "WaitRotate", "WaitShake", "WaitPlay", "WaitText", "WaitAction", "SetVolume", "SetPan",
    "SetFrequency", "SetLoop", "SetLoopPoint", "MoviePlay", "ClearScore", "ClearBacklog",
    "SetBaseTextInterval", "SetFont", "Save", "Load", "DeleteSaveFile", "Position",
    "CursorPosition", "MoveCursor", "DateTime", "WriteFile", "SetShortcut", "Escape", "Reset",
    "Exit", "System",
    "Random", "RemainTime", "DurationTime", "PassageTime", "ImageHorizon", "ImageVertical",
    "Strlen", "Strstr", "String", "EnableBacklog", "ExistSave", "ReadFile", "ModuleFileName",
    "BacklogRow", "BacklogLine", "ScrollbarValue", "Time", "MilliTime", "Integer",
    "SoundAmplitude", "Conquest", "Message", "Array", "AssocArray", "Count", "VariableValue",
    "EnableDirect3DHAL", "Platform", "AvailableFile", "MountSavedata", "LockVideo",
];

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
    Movie(MovieObject),
    /// Names, windows and other objects that draw nothing themselves.
    Other,
}

/// `CreateMovie`: a movie in the scene. One the script waits for that
/// fills the screen plays as a movie (`mpegplay`, the music going on) and
/// then shows its last frame; any other is drawn as its frames (a loop of
/// smoke, a cut under the text), or as one frame while paused.
#[derive(Debug, Clone)]
struct MovieObject {
    path: String,
    looped: bool,
    /// The top half is the colour, the bottom half the opacity.
    alpha: bool,
    /// `SubRender`: drawn darkening (as black).
    dark: bool,
    info: MovieInfo,
    state: MovieState,
    /// Time of the movie already played as a movie that the script's own
    /// waits for it still count (it waits for `RemainTime` after starting
    /// it).
    credit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MovieState {
    /// Playing, `at` ms in (as far as the script's waits tell).
    Playing { at: u32 },
    Paused { at: u32 },
    /// Played through as a movie: its last frame shows.
    Played,
}

impl MovieObject {
    /// Fills the screen and plays once, with nothing to blend: it can play
    /// as a movie.
    fn full_screen(&self, (w, h): (i64, i64)) -> bool {
        !self.looped && !self.alpha && self.info.w as i64 >= w && self.info.h as i64 >= h
    }
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
    /// `Zoom`, per mille (x, y), about the centre.
    zoom: (i64, i64),
    /// `SetBlur`: Gaussian sigma in tenths of a pixel (0 = none).
    blur: u32,
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
    /// Size as drawn (after `Zoom`).
    w: i64,
    h: i64,
    opacity: i64,
    /// Movie frames ([`Op::AnimatedSprite`]): cells, looped, zoom percent.
    cells: Option<(u32, bool, (i32, i32))>,
}

enum Flow {
    Normal,
    Return,
    Break,
}

struct Compiler<'a, 'b> {
    files: &'a BTreeMap<String, NssFile>,
    library: Vec<&'a str>,
    game: &'static Game,
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
    /// The same per object, for a `WaitAction` on one of them.
    pending_by: HashMap<String, u32>,
    /// A `DrawTransition` mask for the next sync.
    pending_mask: Option<String>,
    /// The pictures the running screen shake shakes: it stops when they go.
    shaken: Vec<String>,
    font_size: i64,
    window: Option<TextWindow>,
    first_window: Option<TextWindow>,
    window_visible: bool,
    texts_shown: usize,
    /// `Request("@text131", Enter)` before the text block `[text131]`
    /// exists (続・殺戮のジャンゴ's `TypeBegin`): the block shows its
    /// pages as soon as it runs.
    pending_enter: Option<String>,
    /// `LoadText(unit, box, w, h, letter, line)` (続・殺戮のジャンゴ): the
    /// layout of every text in that box, as [`TextObject::layout`].
    box_layouts: HashMap<String, (i64, i64, i64, i64, i64)>,
    /// Compiling a whole story in one stream ([`compile_story`]).
    whole_story: bool,
    /// `LoadImage` pictures by object name: (output path, w, h).
    loaded_images: HashMap<String, (String, u32, u32)>,
    voice_classes: Vec<String>,
    ops: Vec<Op>,
    notes: BTreeMap<String, usize>,
    depth: usize,
    /// Helper functions being run.
    in_function: usize,
    /// Statements run in the current file (see [`MAX_STEPS`]).
    steps: u64,
    /// Choices compiled so far (for unique labels).
    choices: usize,
    /// The branches of the file's last fork (a choice, or a runtime `if`
    /// that sets the next file differently) and the variable that tells
    /// them apart, and the state the code after the fork started from.
    pending_fork: Option<(conv_core::ir::Var, Vec<Leaf<'a>>, State<'a>)>,
    /// The fork being compiled (its branches share one variable).
    fork_var: Option<conv_core::ir::Var>,
    /// Forks in the current file (picks the next of [`FORK_VARS`]).
    forks_in_file: u16,
    /// Leaves of forks in the current file (see [`LEAF_VAR`]).
    leaves_in_file: i32,
    /// Story flags kept in NScripter variables (see [`story_flags`]).
    flags: HashMap<String, conv_core::ir::Var>,
    /// Codes of the strings story flags hold ("" = 0).
    codes: HashMap<String, i32>,
    /// Helper variables of the current file that became runtime ones at a
    /// merge, the next variable for one, and the strings each can hold.
    dynamic: HashMap<String, conv_core::ir::Var>,
    next_dynamic: u16,
    dynamic_values: HashMap<conv_core::ir::Var, std::collections::BTreeSet<String>>,
}

/// One way through a fork: its state at the end, and the op that marks it
/// (a `mov` to the fork variable, numbered when the fork is complete).
#[derive(Clone)]
struct Leaf<'a> {
    state: State<'a>,
    marker: usize,
    /// Its value of [`LEAF_VAR`].
    id: i32,
}

/// What a branch of a choice changes at convert time.
#[derive(Clone)]
struct State<'a> {
    vars: HashMap<String, Value>,
    objects: BTreeMap<String, Object<'a>>,
    aliases: HashMap<String, String>,
    seq: u64,
    shown: BTreeMap<String, Shown>,
    font_size: i64,
    window: Option<TextWindow>,
    window_visible: bool,
}

/// Branches that end with different pictures on screen (the favour
/// screen adds a picture in one branch, and every sprite after it moves
/// up a layer): the code after them runs on all of them, so the next sync
/// removes every sprite a branch left that the merged state doesn't use,
/// and draws anew each picture some branch doesn't have where the merged
/// state has it.
fn reconcile_shown(merged: &mut BTreeMap<String, Shown>, leaves: &[Leaf<'_>]) {
    let mut stale = BTreeMap::new();
    for l in leaves {
        for s in l.state.shown.values() {
            if !merged.values().any(|m| m.layer == s.layer) {
                stale.entry(s.layer).or_insert_with(|| s.clone());
            }
        }
    }
    for m in merged.values_mut() {
        if leaves.iter().any(|l| !l.state.shown.values().any(|s| s == m)) {
            // Not equal to any wanted picture: placed again.
            m.image.clear();
        }
    }
    for (layer, s) in stale {
        merged.insert(format!(" branch layer {layer}"), s);
    }
}

impl<'a, 'b> Compiler<'a, 'b> {
    /// The game's subroutines ([`Game`]), each from `initial` (an empty
    /// screen) with the story flags as they are at run time; only reached
    /// by a gosub. They run in the middle of a file, so their helper
    /// variables come after every file's (`dynamic_base`), and the fork
    /// variables the file may still need are kept aside and put back.
    fn subroutines(&mut self, initial: &State<'a>, dynamic_base: u16) {
        let local = |index| conv_core::ir::Var { global: false, index };
        let kept: Vec<u16> = (FORK_VARS.start..FORK_VARS.end).chain([LEAF_VAR]).collect();
        for &(label, target) in self.game.subroutines {
            self.restore(initial.clone());
            for (k, &v) in &self.flags {
                self.vars.insert(k.clone(), Value::Runtime(v));
            }
            self.dynamic.clear();
            self.dynamic_values.clear();
            self.next_dynamic = dynamic_base + kept.len() as u16;
            self.steps = 0;
            self.forks_in_file = 0;
            self.leaves_in_file = 0;
            self.ops.push(Op::Label(label.to_string()));
            for (i, &v) in kept.iter().enumerate() {
                self.ops.push(Op::Arith { var: local(dynamic_base + i as u16), op: ArithOp::Set, value: Operand::Var(local(v)) });
            }
            self.call_scene(target, true);
            self.sync(0);
            for (i, &v) in kept.iter().enumerate() {
                self.ops.push(Op::Arith { var: local(v), op: ArithOp::Set, value: Operand::Var(local(dynamic_base + i as u16)) });
            }
            self.ops.push(Op::Return);
        }
    }

    fn snapshot(&self) -> State<'a> {
        State {
            vars: self.vars.clone(),
            objects: self.objects.clone(),
            aliases: self.aliases.clone(),
            seq: self.seq,
            shown: self.shown.clone(),
            font_size: self.font_size,
            window: self.window.clone(),
            window_visible: self.window_visible,
        }
    }

    fn restore(&mut self, s: State<'a>) {
        self.vars = s.vars;
        self.objects = s.objects;
        self.aliases = s.aliases;
        self.seq = s.seq;
        self.shown = s.shown;
        self.font_size = s.font_size;
        self.window = s.window;
        self.window_visible = s.window_visible;
        self.pending_ms = 0;
        self.pending_mask = None;
    }

    /// The `case`s of a select body, also inside `if`s (options shown
    /// only after a clear: the condition is taken as it is now).
    /// `decided`: runtime `if`s already split on (by statement), and the
    /// branch taken. Returns the first runtime `if` holding cases that
    /// isn't decided yet.
    fn collect_cases(&mut self, stmts: &'a [Stmt], out: &mut Vec<&'a [Stmt]>, decided: &[(*const Stmt, bool)]) -> Option<&'a Stmt> {
        let mut open = None;
        for s in stmts {
            match &s.kind {
                StmtKind::Case { body, .. } => out.push(body),
                StmtKind::Block(b) => open = open.or(self.collect_cases(b, out, decided)),
                StmtKind::If { cond, then, otherwise } => {
                    let has_cases = |b: &[Stmt]| b.iter().any(|s| matches!(s.kind, StmtKind::Case { .. }));
                    let taken = match decided.iter().find(|d| std::ptr::eq(d.0, s)) {
                        Some(&(_, t)) => t,
                        None if self.is_runtime(cond) && (has_cases(then) || otherwise.as_deref().is_some_and(has_cases)) => {
                            open = open.or(Some(s));
                            continue;
                        }
                        None => self.eval(cond).truthy(),
                    };
                    if taken {
                        open = open.or(self.collect_cases(then, out, decided));
                    } else if let Some(o) = otherwise {
                        open = open.or(self.collect_cases(o, out, decided));
                    }
                }
                _ => {}
            }
        }
        open
    }

    /// `select { … case @x { … } … }`: the player picks a case. Each case
    /// is compiled from the state before the choice; the code after the
    /// choice goes on in the first case's state, and the run forks at the
    /// end of the file (see [`compile_run`]). The statements before the
    /// first `case` are the menu's input loop and are left out.
    fn select(&mut self, body: &'a [Stmt]) {
        self.select_variant(body, &mut Vec::new());
    }

    /// A select whose options depend on runtime flags (an option shown
    /// only after a clear): one NScripter select per variant, behind the
    /// runtime test.
    fn select_variant(&mut self, body: &'a [Stmt], decided: &mut Vec<(*const Stmt, bool)>) {
        let mut cases = Vec::new();
        if let Some(split) = self.collect_cases(body, &mut cases, decided) {
            let StmtKind::If { cond, .. } = &split.kind else { unreachable!() };
            self.sync(0);
            let own = self.begin_fork();
            self.choices += 1;
            let (else_label, end_label) = (format!("sel{}_else", self.choices), format!("sel{}_end", self.choices));
            self.jump_if(cond, false, &else_label);
            let base = self.snapshot();
            let mut leaves = Vec::new();
            for taken in [true, false] {
                if !taken {
                    self.ops.push(Op::Jump(end_label.clone()));
                    self.ops.push(Op::Label(else_label.clone()));
                }
                decided.push((split as *const Stmt, taken));
                leaves.extend(self.branch(&base, |c| c.select_variant(body, decided)));
                decided.pop();
            }
            self.ops.push(Op::Label(end_label));
            self.finish_fork(leaves, own, true);
            return;
        }
        if cases.is_empty() {
            self.note("select without cases");
            return;
        }
        self.sync(0);
        let own = self.begin_fork();
        let n = self.choices;
        self.choices += 1;
        // SetChoice02…06 keep the button texts in these parameters.
        let texts: Vec<String> = (1..=cases.len())
            .map(|i| {
                let digit = char::from_u32('０' as u32 + i as u32).unwrap_or('?');
                self.vars.get(&format!("$テキストデータ{digit}")).map(Value::string).filter(|t| !t.is_empty()).unwrap_or_else(|| format!("選択肢{i}"))
            })
            .collect();
        let labels: Vec<String> = (0..cases.len()).map(|i| format!("choice{n}_{i}")).collect();
        let after = format!("choice{n}_end");
        let style = self.choice_style(texts.len() as i64);
        let style_shown = style.is_some();
        self.ops.push(Op::Choice { options: texts.into_iter().zip(labels.iter().cloned()).collect(), style });
        if style_shown {
            // The next text sets its own window again.
            self.window = None;
        }
        let base = self.snapshot();
        let mut leaves = Vec::new();
        for (case, label) in cases.iter().zip(&labels) {
            self.ops.push(Op::Label(label.clone()));
            leaves.extend(self.branch(&base, |c| {
                c.exec_block(case);
            }));
            self.ops.push(Op::Jump(after.clone()));
        }
        self.ops.push(Op::Label(after));
        self.finish_fork(leaves, own, true);
    }

    /// The choice plates of `SetChoiceBase` (function_select.nss): `n`
    /// vertical plates 100 px apart, the first at the right, centred on
    /// the screen; the options in black, vertical, one per plate.
    fn choice_style(&mut self, n: i64) -> Option<ChoiceStyle> {
        const PITCH: i64 = 100;
        const FONT: i64 = 20;
        const MARGIN: i64 = 40;
        let (plate, w, h) = self.assets.image("cg/sys/select/select001.png")?;
        let (w, h) = (w as i64, h as i64);
        let first = PITCH * (n - 2) + 50 + self.game.screen.0 / 2;
        let y = (self.game.screen.1 - h) / 2;
        let parts: Vec<_> = (0..n).map(|i| (plate.clone(), first - PITCH * i, y, 1000)).collect();
        let (image, wx, wy) = self.assets.composite(&parts)?;
        let window = TextWindow {
            wait_icon_at: None,
            image,
            x: wx as i32,
            y: wy as i32,
            text_x: (first - PITCH * (n - 1) + (w - FONT) / 2) as i32,
            text_y: (y + MARGIN) as i32,
            columns: ((h - 2 * MARGIN) / FONT) as u32,
            rows: n as u32,
            font_size: FONT as u32,
            line_gap: (PITCH - FONT) as u32,
            vertical: true,
            name_plate: None,
        };
        Some(ChoiceStyle { window, on_color: 0xA00000, off_color: 0x000000 })
    }

    /// Starts a fork; true if it's the outermost one (nested forks share
    /// its variable).
    fn begin_fork(&mut self) -> bool {
        if self.fork_var.is_some() {
            return false;
        }
        let index = FORK_VARS.start + self.forks_in_file % (FORK_VARS.end - FORK_VARS.start);
        self.forks_in_file += 1;
        self.fork_var = Some(conv_core::ir::Var { global: false, index });
        true
    }

    /// Compiles one branch of a fork from `base`: its leaves (one, or those
    /// of the forks inside it, each with the branch's later changes).
    fn branch(&mut self, base: &State<'a>, body: impl FnOnce(&mut Self)) -> Vec<Leaf<'a>> {
        self.restore(base.clone());
        let outer = self.pending_fork.take();
        body(self);
        self.sync(0);
        let end = self.snapshot();
        let leaves = match self.pending_fork.take() {
            None => {
                let marker = self.ops.len();
                let var = self.fork_var.unwrap();
                self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(0) });
                self.leaves_in_file += 1;
                let id = self.leaves_in_file;
                self.ops.push(Op::Arith { var: conv_core::ir::Var { global: false, index: LEAF_VAR }, op: ArithOp::Set, value: Operand::Const(id) });
                vec![Leaf { state: end, marker, id }]
            }
            Some((_, inner, first)) => {
                inner.into_iter().map(|l| Leaf { state: with_branch_vars(&end, &l.state, &first), marker: l.marker, id: l.id }).collect()
            }
        };
        self.pending_fork = outer;
        leaves
    }

    /// Numbers a fork's leaves. Where they lead to different files (or it's
    /// a choice), the file's end dispatches on them; otherwise the branches
    /// merge here, and story flags that differ become unknown.
    fn finish_fork(&mut self, leaves: Vec<Leaf<'a>>, own: bool, choice: bool) {
        let var = self.fork_var.unwrap();
        for (i, l) in leaves.iter().enumerate() {
            self.ops[l.marker] = Op::Arith { var, op: ArithOp::Set, value: Operand::Const(i as i32) };
        }
        if own {
            self.fork_var = None;
        }
        let differ = |k: &str| leaves.iter().any(|l| l.state.vars.get(k) != leaves[0].state.vars.get(k));
        if !own || choice || differ("$GameName") || differ("$GameCircle") {
            if own && self.pending_fork.is_some() {
                self.note("second fork in one file (the run forks on the last one)");
            }
            self.restore(leaves[0].state.clone());
            let mut first = leaves[0].state.clone();
            // The code after the fork runs on every branch: story flags the
            // branches set differently are tested at run time, and so are
            // names of scenes or files they set differently (the code after
            // a choice may copy `$GameName` into a story flag). Where that
            // code leaves them alone, each branch still goes on with its own
            // value (`first` holds the runtime ones). In a whole story
            // ([`compile_story`]) a fork inside another too: its choices
            // are inside the runtime ifs of the deaths before them.
            if own || self.whole_story {
                for (k, &v) in &self.flags {
                    if differ(k) {
                        self.vars.insert(k.clone(), Value::Runtime(v));
                    }
                }
            }
            let keys: std::collections::BTreeSet<String> = leaves.iter().flat_map(|l| l.state.vars.keys().cloned()).collect();
            let helpers: Vec<&String> = keys.iter().filter(|k| differ(k) && !self.flags.contains_key(*k) && !k.starts_with("$SYSTEM")).collect();
            for k in helpers {
                let Some((var, codes)) = self.dynamic_names(k, &leaves) else { continue };
                // Set here from the way the run came (inner forks aren't
                // numbered for good yet, so not at the markers).
                let leaf = conv_core::ir::Var { global: false, index: LEAF_VAR };
                for (i, code) in codes {
                    self.choices += 1;
                    let skip = format!("leaf{}", self.choices);
                    self.ops.push(Op::If { var: leaf, cmp: conv_core::ir::Compare::Ne, value: Operand::Const(leaves[i].id), label: skip.clone() });
                    self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(code) });
                    self.ops.push(Op::Label(skip));
                }
                self.vars.insert(k.clone(), Value::Runtime(var));
                first.vars.insert(k.clone(), Value::Runtime(var));
            }
            self.pending_fork = Some((var, leaves, first));
            return;
        }
        let mut merged = leaves[0].state.clone();
        let keys: std::collections::BTreeSet<String> = leaves.iter().flat_map(|l| l.state.vars.keys().cloned()).collect();
        let mut inserts = Vec::new();
        for k in keys.iter().filter(|k| differ(k)) {
            match self.flags.get(k) {
                Some(&v) => {
                    merged.vars.insert(k.clone(), Value::Runtime(v));
                }
                None if k.starts_with("$SYSTEM") => {}
                // Different names (of a scene to call, say): kept at run time
                // from here on, set at the end of each branch.
                None => match self.dynamic_names(k, &leaves) {
                    Some((var, codes)) => {
                        inserts.extend(codes.into_iter().map(|(i, code)| (leaves[i].marker, Op::Arith { var, op: ArithOp::Set, value: Operand::Const(code) })));
                        merged.vars.insert(k.clone(), Value::Runtime(var));
                    }
                    None => self.note("branches of a runtime if end with different helper variables"),
                },
            }
        }
        // After each marker, last first so the earlier positions hold.
        inserts.sort_by_key(|i| std::cmp::Reverse(i.0));
        for (at, op) in inserts {
            self.ops.insert(at + 1, op);
        }
        if leaves.iter().any(|l| l.state.shown != leaves[0].state.shown) {
            self.note("branches of a runtime if end with different pictures");
            reconcile_shown(&mut merged.shown, &leaves);
        }
        self.restore(merged);
    }

    /// A helper variable the leaves of a fork set to different names of
    /// scenes or files to go to (`@->x_SELECT02`, `y.nss`): a runtime
    /// variable from here on, with the code each leaf sets it to (leaves
    /// that already hold it at run time left out). None for other values.
    fn dynamic_names(&mut self, k: &str, leaves: &[Leaf<'a>]) -> Option<(conv_core::ir::Var, Vec<(usize, i32)>)> {
        let known = self.dynamic.get(k).copied();
        let scene = |v: Option<&Value>| match v {
            Some(Value::Str(t)) => t.starts_with("@->") || t.starts_with("nss/") || t.contains(".nss"),
            Some(Value::Null) | None => true,
            Some(&Value::Runtime(r)) => Some(r) == known,
            _ => false,
        };
        let strings = leaves.iter().all(|l| scene(l.state.vars.get(k))) && leaves.iter().any(|l| matches!(l.state.vars.get(k), Some(Value::Str(_))));
        if !strings || (known.is_none() && (self.next_dynamic as u32) >= GLOBAL_BORDER - 11) {
            return None;
        }
        let var = known.unwrap_or_else(|| {
            let var = conv_core::ir::Var { global: false, index: self.next_dynamic };
            self.next_dynamic += 1;
            self.dynamic.insert(k.to_string(), var);
            var
        });
        let mut codes = Vec::new();
        for (i, l) in leaves.iter().enumerate() {
            let v = l.state.vars.get(k).cloned().unwrap_or(Value::Null);
            if matches!(v, Value::Runtime(_)) {
                continue;
            }
            codes.push((i, self.code(&v).unwrap_or(0)));
            self.dynamic_values.entry(var).or_default().insert(v.string());
        }
        Some((var, codes))
    }

    /// `if` on a story flag only the running game knows: both branches are
    /// compiled, behind a runtime test.
    fn runtime_if(&mut self, cond: &'a Expr, then: &'a [Stmt], otherwise: Option<&'a [Stmt]>) {
        self.runtime_if_then(cond, then, otherwise, &[]);
    }

    /// [`Compiler::runtime_if`] followed by `rest`, which runs at the end of
    /// each branch that doesn't end in `return`.
    fn runtime_if_then(&mut self, cond: &'a Expr, then: &'a [Stmt], otherwise: Option<&'a [Stmt]>, rest: &[&'a [Stmt]]) {
        let with_rest = |branch: &'a [Stmt]| {
            let mut parts = vec![branch];
            if !ends_in_return(branch) {
                parts.extend_from_slice(rest);
            }
            parts
        };
        let (then_parts, else_parts) = (with_rest(then), with_rest(otherwise.unwrap_or(&[])));
        self.sync(0);
        let own = self.begin_fork();
        let n = self.choices;
        self.choices += 1;
        let (else_label, end_label) = (format!("if{n}_else"), format!("if{n}_end"));
        self.jump_if(cond, false, &else_label);
        let base = self.snapshot();
        let mut leaves = self.branch(&base, |c| {
            c.exec_seq(&then_parts);
        });
        self.ops.push(Op::Jump(end_label.clone()));
        self.ops.push(Op::Label(else_label));
        let else_leaves = self.branch(&base, |c| {
            c.exec_seq(&else_parts);
        });
        // A branch that returned left the scene: the code after the if
        // goes on from the other one (its pictures, its variables).
        match (ends_in_return(then), otherwise.is_some_and(ends_in_return)) {
            (true, false) => leaves = else_leaves,
            (false, true) => {}
            _ => leaves.extend(else_leaves),
        }
        self.ops.push(Op::Label(end_label));
        self.finish_fork(leaves, own, false);
    }

    /// `call_scene $x` where `$x` is only known at run time: each scene it
    /// can name, behind a runtime test.
    fn call_runtime_scene(&mut self, var: conv_core::ir::Var, chapter: bool) {
        let values: Vec<String> = self.dynamic_values.get(&var).map(|v| v.iter().cloned().collect()).unwrap_or_default();
        if values.is_empty() {
            self.note("call_scene to a runtime value with no known names");
            return;
        }
        self.sync(0);
        let own = self.begin_fork();
        self.choices += 1;
        let n = self.choices;
        let end = format!("callrt{n}_end");
        let base = self.snapshot();
        let mut leaves = Vec::new();
        for (i, v) in values.iter().enumerate() {
            let next = format!("callrt{n}_{i}");
            let code = self.code(&Value::Str(v.clone())).unwrap_or(0);
            self.ops.push(Op::If { var, cmp: conv_core::ir::Compare::Ne, value: Operand::Const(code), label: next.clone() });
            leaves.extend(self.branch(&base, |c| c.call_scene(v, chapter)));
            self.ops.push(Op::Jump(end.clone()));
            self.ops.push(Op::Label(next));
        }
        self.ops.push(Op::Label(end));
        self.finish_fork(leaves, own, false);
    }

    /// Whether `e` reads a story flag whose value isn't known now.
    fn is_runtime(&mut self, e: &'a Expr) -> bool {
        match e {
            Expr::Var(v) => matches!(self.vars.get(&v.to_string()), Some(Value::Runtime(_))),
            Expr::Index { .. } => {
                let key = self.index_key(e);
                matches!(self.vars.get(&key), Some(Value::Runtime(_)))
            }
            Expr::Unary { expr, .. } => self.is_runtime(expr),
            Expr::Binary { left, right, .. } => self.is_runtime(left) || self.is_runtime(right),
            _ => false,
        }
    }

    /// The number a known value is stored as in a flag variable.
    fn code(&mut self, v: &Value) -> Option<i32> {
        Some(match v {
            Value::Null => 0,
            Value::Int(n) | Value::Rel(n) => *n as i32,
            Value::Str(s) if s.is_empty() => 0,
            Value::Str(s) => match s.trim().parse::<i32>() {
                Ok(n) => n,
                Err(_) => {
                    let next = self.codes.len() as i32 + 1;
                    *self.codes.entry(s.clone()).or_insert(next)
                }
            },
            Value::Runtime(_) => return None,
        })
    }

    /// Emits a jump to `label` taken when `e` is `want` (true or false).
    fn jump_if(&mut self, e: &'a Expr, want: bool, label: &str) {
        use conv_core::ir::Compare;
        let fresh = |c: &mut Self| {
            c.choices += 1;
            format!("cond{}", c.choices)
        };
        match e {
            Expr::Binary { op: BinaryOp::And, left, right } | Expr::Binary { op: BinaryOp::Or, left, right } => {
                let and = matches!(e, Expr::Binary { op: BinaryOp::And, .. });
                if and != want {
                    // a && b false: either is false; a || b true: either is true.
                    self.jump_if(left, want, label);
                    self.jump_if(right, want, label);
                } else {
                    let skip = fresh(self);
                    self.jump_if(left, !want, &skip);
                    self.jump_if(right, want, label);
                    self.ops.push(Op::Label(skip));
                }
            }
            Expr::Unary { op: UnaryOp::Not, expr } => self.jump_if(expr, !want, label),
            Expr::Binary { op, left, right } if matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge) => {
                let (l, r) = (self.eval(left), self.eval(right));
                let cmp = match op {
                    BinaryOp::Eq => Compare::Eq,
                    BinaryOp::Ne => Compare::Ne,
                    BinaryOp::Lt => Compare::Lt,
                    BinaryOp::Gt => Compare::Gt,
                    BinaryOp::Le => Compare::Le,
                    _ => Compare::Ge,
                };
                let (var, cmp, value) = match (&l, &r) {
                    (Value::Runtime(v), k) if !matches!(k, Value::Runtime(_)) => (*v, cmp, k.clone()),
                    // k < var is var > k.
                    (k, Value::Runtime(v)) if !matches!(k, Value::Runtime(_)) => {
                        let mirrored = match cmp {
                            Compare::Lt => Compare::Gt,
                            Compare::Gt => Compare::Lt,
                            Compare::Le => Compare::Ge,
                            Compare::Ge => Compare::Le,
                            c => c,
                        };
                        (*v, mirrored, k.clone())
                    }
                    (Value::Runtime(a), Value::Runtime(b)) => {
                        let cmp = if want { cmp } else { negate(cmp) };
                        self.ops.push(Op::If { var: *a, cmp, value: Operand::Var(*b), label: label.to_string() });
                        return;
                    }
                    _ => {
                        let truth = self.eval(e).truthy();
                        if truth == want {
                            self.ops.push(Op::Jump(label.to_string()));
                        }
                        return;
                    }
                };
                let cmp = if want { cmp } else { negate(cmp) };
                let value = Operand::Const(self.code(&value).unwrap_or(0));
                self.ops.push(Op::If { var, cmp, value, label: label.to_string() });
            }
            _ => match self.eval(e) {
                Value::Runtime(var) => {
                    let cmp = if want { Compare::Ne } else { Compare::Eq };
                    self.ops.push(Op::If { var, cmp, value: Operand::Const(0), label: label.to_string() });
                }
                v => {
                    if self.is_runtime(e) {
                        self.note("runtime if on an expression (taken as false)");
                    }
                    if v.truthy() == want {
                        self.ops.push(Op::Jump(label.to_string()));
                    }
                }
            },
        }
    }

    /// An assignment to a story flag: also done at run time.
    fn assign_flag(&mut self, var: conv_core::ir::Var, op: AssignOp, old: &Value, value: &Value, new: &Value) {
        match (op, old, value) {
            (AssignOp::Add | AssignOp::Sub, Value::Runtime(_), v) | (AssignOp::Inc | AssignOp::Dec, Value::Runtime(_), v) if !matches!(v, Value::Runtime(_)) => {
                let n = match op {
                    AssignOp::Inc | AssignOp::Dec => 1,
                    _ => v.int() as i32,
                };
                let ir_op = if matches!(op, AssignOp::Add | AssignOp::Inc) { ArithOp::Add } else { ArithOp::Sub };
                self.ops.push(Op::Arith { var, op: ir_op, value: Operand::Const(n) });
            }
            (AssignOp::Set, _, Value::Runtime(other)) => {
                self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Var(*other) });
            }
            _ => match self.code(new) {
                Some(n) => self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(n) }),
                None => self.note("story flag set from a runtime expression"),
            },
        }
    }

    fn new(files: &'a BTreeMap<String, NssFile>, file: &'a str, game: &'static Game, assets: &'b mut dyn Assets) -> Self {
        // Settings the scripts read, at the values a first start gives them
        // (`sys_config.nss`): the voices of lines spoken in unison
        // (`CreateVOICE`) play only with "other voices" on.
        let vars = HashMap::from([("#SYSTEM_voice_enable_another".to_string(), Value::Int(1))]);
        Compiler {
            files,
            library: library(files, game),
            game,
            assets,
            vars,
            frames: Vec::new(),
            file,
            objects: BTreeMap::new(),
            aliases: HashMap::new(),
            seq: 0,
            shown: BTreeMap::new(),
            pending_ms: 0,
            pending_by: HashMap::new(),
            pending_mask: None,
            shaken: Vec::new(),
            font_size: 20,
            window: None,
            first_window: None,
            window_visible: false,
            texts_shown: 0,
            pending_enter: None,
            box_layouts: HashMap::new(),
            whole_story: false,
            loaded_images: HashMap::new(),
            voice_classes: Vec::new(),
            ops: Vec::new(),
            notes: BTreeMap::new(),
            depth: 0,
            in_function: 0,
            steps: 0,
            choices: 0,
            pending_fork: None,
            fork_var: None,
            forks_in_file: 0,
            leaves_in_file: 0,
            flags: HashMap::new(),
            codes: HashMap::new(),
            dynamic: HashMap::new(),
            next_dynamic: 0,
            dynamic_values: HashMap::new(),
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

    /// Current file first, then the library, then what the current file
    /// `#include`s besides (続・殺戮のジャンゴ's chapters include
    /// `function_select.nss`, its boot script doesn't); later definitions
    /// win. Names are case-sensitive, as in the exe (`call_nss_function`).
    fn function(&self, name: &str) -> Option<(&'a str, &'a Function)> {
        let find = |file: &'a str| {
            self.files.get(file)?.items.iter().rev().find_map(|item| match item {
                Item::Function(f) if f.name == name => Some((file, f)),
                _ => None,
            })
        };
        let files = self.files;
        let includes = files.get(self.file).into_iter().flat_map(|f| f.items.iter().rev()).filter_map(move |item| match item {
            Item::Include(path) => files.get_key_value(path.rsplit('/').next().unwrap_or(path)).map(|(k, _)| k.as_str()),
            _ => None,
        });
        std::iter::once(self.file).chain(self.library.iter().rev().copied()).chain(includes).find_map(find)
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
        self.exec_seq(&[stmts])
    }

    /// Runs `parts` one after the other. A runtime `if` with a branch that
    /// ends in `return` (`if($死亡01 == 1){ …; return; }`) runs what follows
    /// it only in its other branch: the player who died doesn't go on.
    fn exec_seq(&mut self, parts: &[&'a [Stmt]]) -> Flow {
        for (pi, part) in parts.iter().enumerate() {
            for (i, s) in part.iter().enumerate() {
                if let StmtKind::If { cond, then, otherwise } = &s.kind
                    && (ends_in_return(then) || otherwise.as_deref().is_some_and(ends_in_return))
                    && self.is_runtime(cond)
                {
                    let mut rest = vec![&part[i + 1..]];
                    rest.extend_from_slice(&parts[pi + 1..]);
                    self.runtime_if_then(cond, then, otherwise.as_deref(), &rest);
                    if self.in_function == 0 {
                        self.flush_sounds();
                    }
                    return Flow::Normal;
                }
                let flow = self.exec(s);
                // A statement of a scene (not of a helper) has finished
                // setting up the sounds it started.
                if self.in_function == 0 {
                    self.flush_sounds();
                }
                match flow {
                    Flow::Normal => {}
                    flow => return flow,
                }
            }
        }
        Flow::Normal
    }

    fn exec(&mut self, stmt: &'a Stmt) -> Flow {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            if self.steps == MAX_STEPS + 1 {
                self.note("file stopped after 2,000,000 statements at convert time");
            }
            return Flow::Return;
        }
        match &stmt.kind {
            StmtKind::Expr(e) => {
                self.eval(e);
            }
            StmtKind::Assign { target, index, op, value } => {
                let mut key = target.to_string();
                for i in index {
                    key = format!("{key}[{}]", self.eval(i).string());
                }
                // `$x = $x + k` on a story flag is `$x += k`.
                let (op, value) = match (op, value, self.flags.contains_key(&key)) {
                    (AssignOp::Set, Some(Expr::Binary { op: o @ (BinaryOp::Add | BinaryOp::Sub), left, right }), true)
                        if matches!(&**left, Expr::Var(v) if v == target) && index.is_empty() =>
                    {
                        (if *o == BinaryOp::Add { &AssignOp::Add } else { &AssignOp::Sub }, Some(&**right))
                    }
                    (op, value, _) => (op, value.as_ref()),
                };
                let value = value.map(|v| self.eval(v)).unwrap_or(Value::Null);
                let old = self.vars.get(&key).cloned().unwrap_or(Value::Null);
                let new = match op {
                    AssignOp::Set => value.clone(),
                    AssignOp::Add => add(&old, &value),
                    AssignOp::Sub => Value::Int(old.int() - value.int()),
                    AssignOp::Mul => Value::Int(old.int() * value.int()),
                    AssignOp::Div => Value::Int(old.int().checked_div(value.int()).unwrap_or(0)),
                    AssignOp::Inc => Value::Int(old.int() + 1),
                    AssignOp::Dec => Value::Int(old.int() - 1),
                };
                let new = match self.flags.get(&key).copied() {
                    Some(var) => {
                        self.assign_flag(var, *op, &old, &value, &new);
                        // Added to an unknown value: still unknown.
                        if matches!(old, Value::Runtime(_)) && *op != AssignOp::Set { Value::Runtime(var) } else { new }
                    }
                    None => new,
                };
                if key == "$GameName" {
                    self.vars.insert(GAME_NAME_SET.into(), Value::Int(1));
                }
                // The chapter title the save screen shows.
                if key == "$TITLE_NOW"
                    && let Value::Str(t) = &new
                {
                    self.ops.push(Op::Chapter { title: t.trim_matches(' ').to_string() });
                }
                self.vars.insert(key, new);
            }
            StmtKind::If { cond, then, otherwise } if self.is_runtime(cond) => {
                self.runtime_if(cond, then, otherwise.as_deref());
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
            StmtKind::Select(body) => self.select(body),
            StmtKind::Case { .. } => {}
            StmtKind::CallScene(t) | StmtKind::CallChapter(t) => {
                let chapter = matches!(stmt.kind, StmtKind::CallChapter(_));
                let target = match t {
                    Target::Name(n) => n.clone(),
                    Target::Var(v) => match self.vars.get(&v.to_string()) {
                        Some(&Value::Runtime(var)) => {
                            self.call_runtime_scene(var, chapter);
                            return Flow::Normal;
                        }
                        v => v.map(Value::string).unwrap_or_default(),
                    },
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
        if (self.game.scene)(self, file, &scene) {
            return;
        }
        let Some(body) = self.scene(file, &scene) else {
            self.note(format!("call_scene to a missing scene {scene}"));
            return;
        };
        if self.depth >= MAX_DEPTH {
            self.note("calls nested too deep");
            return;
        }
        let saved = std::mem::replace(&mut self.file, file);
        let mark = self.seq;
        self.depth += 1;
        self.frames.push(HashMap::new());
        self.exec_block(body);
        self.frames.pop();
        self.depth -= 1;
        self.file = saved;
        if chapter && self.game.chapter_objects {
            self.delete_chapter_objects(mark);
        }
    }

    /// Deletes the objects made since `mark` ([`Game::chapter_objects`])
    /// whose first name part is one of them too.
    fn delete_chapter_objects(&mut self, mark: u64) {
        let new = |c: &Self, n: &str| c.objects.get(n).is_some_and(|o| o.seq > mark);
        let gone: Vec<String> = self
            .objects
            .keys()
            .filter(|n| new(self, n) && new(self, n.split('/').next().unwrap_or(n)))
            .cloned()
            .collect();
        for n in gone {
            self.delete(&n);
        }
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
        self.in_function += 1;
        self.frames.push(frame);
        self.exec_block(&f.body);
        self.frames.pop();
        self.in_function -= 1;
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
                if (self.game.call)(self, name) {
                    return Value::Null;
                }
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

    /// A string argument that names a variable (`"$Transition"`) means
    /// that variable's value.
    fn deref(&self, v: &Value) -> String {
        let mut s = v.string();
        for _ in 0..4 {
            match self.vars.get(&s) {
                Some(v) if s.starts_with(['$', '#']) => s = v.string(),
                _ => break,
            }
        }
        s
    }

    /// The object a name argument means: `$var` names the variable holding
    /// it, `@x` goes through the alias table.
    /// A string argument, or the value of the variable it names
    /// (`CreateSound("x", SE, "$場所指定")`, 続・殺戮のジャンゴ's
    /// `CreateSE`).
    fn named_string(&self, v: &Value) -> String {
        let s = v.string();
        match self.vars.get(&s) {
            Some(v) if s.starts_with(['$', '#']) => v.string(),
            _ => s,
        }
    }

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
    /// wildcard, over names and aliases: `@StMR*` finds `遠４/StMR01`,
    /// alias `StMR01`).
    fn matching(&self, v: &Value) -> Vec<String> {
        let name = self.object_name(v);
        match name.strip_suffix('*') {
            Some(prefix) => {
                let mut found: std::collections::BTreeSet<String> = self.objects.keys().filter(|k| k.starts_with(prefix)).cloned().collect();
                if !prefix.is_empty() {
                    found.extend(self.aliases.iter().filter(|(a, o)| a.starts_with(prefix) && self.objects.contains_key(*o)).map(|(_, o)| o.clone()));
                }
                found.into_iter().collect()
            }
            None if self.objects.contains_key(&name) => vec![name],
            None => Vec::new(),
        }
    }

    fn create(&mut self, name: String, kind: Kind<'a>, priority: i64, w: i64, h: i64) -> &mut Object<'a> {
        self.seq += 1;
        let o = Object { kind, priority, x: 0, y: 0, w, h, opacity: 1000, locked: false, hideable: false, seq: self.seq, zoom: (1000, 1000), blur: 0 };
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

    /// Movie `name`, if it is one that fills the screen and is playing:
    /// plays it as a movie (the sounds started with it play beside it),
    /// then shows its last frame. The script's waits for its remaining
    /// time are counted as played ([`MovieObject::credit`]).
    fn play_movie(&mut self, name: &str) {
        let Some(Object { kind: Kind::Movie(m), opacity, .. }) = self.objects.get(name) else { return };
        let MovieState::Playing { at } = m.state else { return };
        if !m.full_screen(self.game.screen) {
            return;
        }
        let (path, opacity, left) = (m.path.clone(), *opacity, m.info.ms.saturating_sub(at));
        let Some(file) = self.assets.movie_file(&path, false) else { return };
        // Hidden while the screen catches up, then the last frame shows.
        self.objects.get_mut(name).unwrap().opacity = 0;
        self.sync(0);
        self.ops.push(Op::SceneMovie { file, skippable: true });
        let o = self.objects.get_mut(name).unwrap();
        o.opacity = opacity;
        if let Kind::Movie(m) = &mut o.kind {
            m.state = MovieState::Played;
            m.credit = left;
        }
        self.sync(0);
    }

    /// A wait of `ms`: the time movies already played as movies take off
    /// it (what is left is returned), a movie on screen that it mostly
    /// waits for plays as a movie, and the others go on by that much.
    fn movie_wait(&mut self, mut ms: u32) -> u32 {
        let screen = self.game.screen;
        for o in self.objects.values_mut() {
            if let Kind::Movie(m) = &mut o.kind {
                let used = m.credit.min(ms);
                m.credit -= used;
                ms -= used;
            }
        }
        if ms == 0 {
            return 0;
        }
        let waited: Vec<String> = self
            .objects
            .iter()
            .filter(|(_, o)| {
                matches!(&o.kind, Kind::Movie(m) if m.full_screen(screen) && o.opacity > 0
                    && matches!(m.state, MovieState::Playing { at } if ms * 2 >= m.info.ms.saturating_sub(at)))
            })
            .map(|(n, _)| n.clone())
            .collect();
        if let Some(name) = waited.first() {
            self.play_movie(name);
            if let Some(Object { kind: Kind::Movie(m), .. }) = self.objects.get_mut(name) {
                let used = m.credit.min(ms);
                m.credit -= used;
                ms -= used;
            }
        }
        for o in self.objects.values_mut() {
            if let Kind::Movie(m) = &mut o.kind
                && let MovieState::Playing { at } = m.state
            {
                let at = at + ms;
                m.state = MovieState::Playing { at: if m.looped { at % m.info.ms.max(1) } else { at.min(m.info.ms) } };
            }
        }
        ms
    }

    fn builtin(&mut self, name: &str, a: &[Value]) -> Value {
        let (screen_w, screen_h) = self.game.screen;
        let arg = |i: usize| a.get(i).cloned().unwrap_or(Value::Null);
        match name {
            "LoadImage" if self.game.loaded_textures => {
                // (name, path): a picture other objects are made from
                // (`CreateTexture("frame03", …, "@LoadFrame")`), not shown.
                let name = self.object_name(&arg(0));
                match self.assets.image(&self.deref(&arg(1))) {
                    Some(img) => {
                        self.loaded_images.insert(name, img);
                    }
                    None => self.note("LoadImage of a missing file"),
                }
            }
            "CreateTexture" => {
                let name = self.object_name(&arg(0));
                let src = self.deref(&arg(4));
                let Some((image, w, h)) = self.texture_source(&src) else {
                    self.note(format!("CreateTexture from {}", if src.contains('/') { "a missing file" } else { "an object or the screen" }));
                    self.create(name, Kind::Other, arg(1).int(), 0, 0);
                    return Value::Null;
                };
                let (w, h) = (w as i64, h as i64);
                let o = self.create(name, Kind::Image { image }, arg(1).int(), w, h);
                o.x = arg(2).position(0, w, screen_w);
                o.y = arg(3).position(0, h, screen_h);
            }
            "CreateColor" => {
                let name = self.object_name(&arg(0));
                let (w, h) = (arg(4).int().max(1), arg(5).int().max(1));
                let Some(rgb) = color(&arg(6)) else {
                    if std::env::var_os("NSS_TRACE").is_some() {
                        eprintln!("unknown colour {:?}", arg(6));
                    }
                    self.note("CreateColor with an unknown colour");
                    return Value::Null;
                };
                let image = self.assets.color(rgb, w as u32, h as u32);
                let o = self.create(name, Kind::Image { image }, arg(1).int(), w, h);
                o.x = arg(2).position(0, w, screen_w);
                o.y = arg(3).position(0, h, screen_h);
            }
            "CreateEffect" => {
                let name = self.object_name(&arg(0));
                let kind = if arg(6).string().eq_ignore_ascii_case("plain") {
                    Kind::Cover
                } else {
                    self.note(format!("CreateEffect {}", arg(6).string()));
                    Kind::Other
                };
                self.create(name, kind, arg(1).int(), screen_w, screen_h);
            }
            "CreateWindow" | "CreateName" => {
                let name = self.object_name(&arg(0));
                let o = self.create(name, Kind::Other, arg(1).int(), 0, 0);
                // (name, priority, x, y, w, h, ?): a text in the window
                // starts at its corner (続・殺戮のジャンゴ's boxes).
                if a.len() >= 6 {
                    (o.x, o.y) = (arg(2).int(), arg(3).int());
                }
            }
            "CreateMovie" => {
                let name = self.object_name(&arg(0));
                let path = self.deref(&arg(6));
                let Some(info) = self.assets.movie_info(&path) else {
                    self.note("CreateMovie of a missing movie");
                    self.create(name, Kind::Other, arg(1).int(), 0, 0);
                    return Value::Null;
                };
                let alpha = arg(5).truthy();
                let (w, h) = (info.w as i64, if alpha { info.h as i64 / 2 } else { info.h as i64 });
                let m = MovieObject { path, looped: arg(4).truthy(), alpha, dark: false, info, state: MovieState::Playing { at: 0 }, credit: 0 };
                let o = self.create(name, Kind::Movie(m), arg(1).int(), w, h);
                o.x = arg(2).position(0, w, screen_w);
                o.y = arg(3).position(0, h, screen_h);
            }
            "MoviePlay" => {
                // Full screen, the script waiting. A movie with sound stops
                // the music; a silent cut lets it play on.
                let path = self.deref(&arg(0));
                self.sync(0);
                let Some(info) = self.assets.movie_info(&path) else {
                    // `mv_smg_PV` (the title's): no such file on the disc.
                    self.note("MoviePlay of a missing movie");
                    return Value::Null;
                };
                let skippable = arg(1).truthy();
                if let Some(file) = self.assets.movie_file(&path, info.sound) {
                    self.ops.push(if info.sound { Op::Movie { file, skippable } } else { Op::SceneMovie { file, skippable } });
                }
            }
            "CreateSound" => {
                let name = self.object_name(&arg(0));
                let bgm = arg(1).string().eq_ignore_ascii_case("bgm");
                let s = SoundObject { path: self.named_string(&arg(2)), bgm, volume: 1000, looped: false, starting: false, fade_ms: 0, playing: None, disused: false };
                self.create(name, Kind::Sound(s), 0, 0, 0);
            }
            "Fade" => {
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
                    self.timed(arg(1).int(), &wait, &targets);
                } else if !matches!(wait, Value::Null | Value::Int(0)) {
                    self.check_window();
                }
            }
            "Move" => {
                let targets = self.matching(&arg(0));
                let ms = arg(1).int();
                let blocking = !matches!(arg(5), Value::Null | Value::Int(0));
                // One picture on screen moving while the script waits: a real
                // motion (after showing what changed before it).
                if ms > 0 && blocking && targets.len() == 1 && self.shown.contains_key(&targets[0]) {
                    self.sync(0);
                    let o = self.objects.get_mut(&targets[0]).unwrap();
                    let (x, y) = (arg(2).position(o.x, o.w, screen_w), arg(3).position(o.y, o.h, screen_h));
                    let (dx, dy) = (x - o.x, y - o.y);
                    (o.x, o.y) = (x, y);
                    if let Some(sh) = self.shown.get_mut(&targets[0]) {
                        sh.x += dx;
                        sh.y += dy;
                        let (cx, cy) = ((sh.x + sh.w / 2) as i32, (sh.y + sh.h / 2) as i32);
                        self.ops.push(Op::SpriteMove { layer: sh.layer, x: cx, y: cy, ms: ms as u32 });
                    }
                    return Value::Null;
                }
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    o.x = arg(2).position(o.x, o.w, screen_w);
                    o.y = arg(3).position(o.y, o.h, screen_h);
                }
                if arg(1).int() > 0 && targets.iter().any(|t| self.shown.contains_key(t)) {
                    self.note("Move over time drawn as a cross-fade");
                }
                self.timed(arg(1).int(), &arg(5), &targets);
            }
            "Delete" => {
                for t in self.matching(&arg(0)) {
                    if !self.objects[&t].locked {
                        self.delete(&t);
                    }
                }
            }
            "Request" => {
                let action = arg(1).string().to_ascii_lowercase();
                let targets = self.matching(&arg(0));
                if targets.is_empty() && action == "enter" {
                    self.pending_enter = Some(arg(0).string().trim_start_matches('@').to_string());
                }
                for t in targets {
                    self.request(&t, &action);
                }
            }
            "SetAlias" => {
                let (obj, alias) = (self.object_name(&arg(0)), arg(1).string());
                if obj != alias {
                    self.aliases.insert(alias, obj);
                }
            }
            "Rotate" => {
                for t in self.matching(&arg(0)) {
                    match &mut self.objects.get_mut(&t).unwrap().kind {
                        Kind::Text(text) => text.rotation = arg(4).position(text.rotation, 0, 0),
                        _ if arg(4).int() != 0 => self.note("Rotate"),
                        _ => {}
                    }
                }
            }
            "Zoom" => {
                let targets = self.matching(&arg(0));
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    o.zoom = (arg(2).position(o.zoom.0, 0, 0).max(1), arg(3).position(o.zoom.1, 0, 0).max(1));
                }
                if arg(1).int() > 0 && targets.iter().any(|t| self.shown.contains_key(t)) {
                    self.note("Zoom over time drawn as a cross-fade");
                }
                self.timed(arg(1).int(), &arg(5), &targets);
            }
            "SetBlur" => {
                // (obj, on, passes 1-3, ?, strength, wait). Unverified:
                // drawn as a Gaussian of 1.5 px per pass.
                let sigma = if arg(1).truthy() { (arg(2).int().clamp(1, 5) * 15) as u32 } else { 0 };
                let targets = self.matching(&arg(0));
                for t in &targets {
                    self.objects.get_mut(t).unwrap().blur = sigma;
                }
                self.timed(arg(3).int(), &arg(5), &targets);
            }
            "DrawTransition" => {
                // (obj, time, from, to, softness, tempo, mask, wait): a fade
                // to `to` shaped by the mask.
                let targets = self.matching(&arg(0));
                for t in &targets {
                    self.objects.get_mut(t).unwrap().opacity = arg(3).int().clamp(0, 1000);
                }
                match self.assets.image(&self.deref(&arg(6))) {
                    Some((mask, _, _)) => self.pending_mask = Some(mask),
                    None => self.note(format!("DrawTransition mask missing: {}", arg(6).string())),
                }
                self.timed(arg(1).int(), &arg(7), &targets);
            }
            "Shake" => {
                // (obj, time, x, y, …, wait). A full-screen picture (or all
                // of them): the whole picture shakes.
                let targets = self.matching(&arg(0));
                if !targets.iter().any(|t| self.objects[t].w >= screen_w && self.objects[t].h >= screen_h) {
                    self.note("Shake of a picture smaller than the screen");
                    self.timed(arg(1).int(), a.last().unwrap_or(&Value::Null), &targets);
                    return Value::Null;
                }
                // It lasts `ms` (or until the picture is deleted: shakes of
                // 36 minutes end that way); 0 ms or 0 pixels stops it.
                let ms = arg(1).int().max(0) as u32;
                let (x, y) = (arg(2).int().abs() as i32, arg(3).int().abs() as i32);
                self.sync(0);
                if ms > 0 && (x > 0 || y > 0) {
                    let shake =
                        ScreenEffect::Shake { time_ms: ms, interval_ms: 16, path: false, speed: (1300, 1700), range: [(x, y), (x, y)], end_ms: ms };
                    self.ops.push(Op::ScreenEffect { slot: 1, effect: Some(shake) });
                    self.shaken = targets;
                    if !matches!(a.last(), Some(Value::Null | Value::Int(0))) {
                        self.ops.push(Op::Wait { ms });
                    }
                } else if !self.shaken.is_empty() {
                    self.shaken.clear();
                    self.ops.push(Op::ScreenEffect { slot: 1, effect: None });
                }
            }
            "ImageHorizon" => return Value::Int(self.objects.get(&self.object_name(&arg(0))).map_or(0, |o| o.w)),
            "ImageVertical" => return Value::Int(self.objects.get(&self.object_name(&arg(0))).map_or(0, |o| o.h)),
            "SetFont" => self.font_size = arg(1).int().max(1),
            "LoadText" if a.len() == 6 => {
                let layout = (arg(2).int(), arg(3).int(), arg(4).int(), arg(5).int(), self.font_size);
                self.box_layouts.insert(self.object_name(&arg(1)), layout);
            }
            "LoadText" => {
                let name = self.object_name(&arg(2));
                let font = self.font_size;
                if let Some(Object { kind: Kind::Text(t), .. }) = self.objects.get_mut(&name) {
                    t.layout = Some((arg(3).int(), arg(4).int(), arg(5).int(), arg(6).int(), font));
                }
            }
            "Wait" => {
                // The actions still running are shown over the wait, at
                // most.
                let ms = self.movie_wait(arg(0).int().max(0) as u32);
                let shown = std::mem::take(&mut self.pending_ms).min(ms);
                self.sync(shown);
                if ms > shown {
                    self.ops.push(Op::Wait { ms: ms - shown });
                }
            }
            "WaitKey" => {
                self.sync(0);
                match arg(0) {
                    Value::Null => self.ops.push(Op::ClickWait { skippable: true }),
                    ms => {
                        let ms = self.movie_wait(ms.int().max(0) as u32);
                        if ms > 0 {
                            self.ops.push(Op::Wait { ms });
                        }
                    }
                }
            }
            "WaitPlay" => {
                // Until the movie's end: what played as a movie has ended.
                for t in self.matching(&arg(0)) {
                    self.play_movie(&t);
                    if let Some(Object { kind: Kind::Movie(m), .. }) = self.objects.get_mut(&t) {
                        m.credit = 0;
                    }
                }
            }
            "WaitAction" if self.matching(&arg(0)).iter().any(|t| matches!(self.objects[t].kind, Kind::Movie(_))) => {
                for t in self.matching(&arg(0)) {
                    self.play_movie(&t);
                }
            }
            "WaitAction" | "WaitFade" | "WaitMove" | "WaitZoom" | "Draw" => {
                // Waits for the actions: their whole time. Given an object,
                // only for that object's (`CP_EHPChange` waits on its own
                // process, not on the 15 s zoom started before it).
                let ms = if name == "Draw" || matches!(arg(0), Value::Null) {
                    std::mem::take(&mut self.pending_ms)
                } else {
                    self.matching(&arg(0)).iter().filter_map(|t| self.pending_by.get(t)).copied().max().unwrap_or(0)
                };
                self.sync(ms);
            }
            "SetVolume" => self.set_volume(&arg(0), arg(1).int(), arg(2).int()),
            "SetLoop" => {
                for t in self.matching(&arg(0)) {
                    if let Kind::Sound(s) = &mut self.objects.get_mut(&t).unwrap().kind {
                        s.looped = arg(1).truthy();
                    }
                }
            }
            "SetFrequency" if arg(2).int() != 1000 => self.note("SetFrequency"),
            "RemainTime" | "DurationTime" => {
                let movie = self.matching(&arg(0)).iter().find_map(|t| match &self.objects[t].kind {
                    Kind::Movie(m) => Some(m.clone()),
                    _ => None,
                });
                return Value::Int(movie.map_or(0, |m| match (name, m.state) {
                    ("DurationTime", _) | (_, MovieState::Playing { at: 0 } | MovieState::Paused { at: 0 }) => m.info.ms,
                    (_, MovieState::Playing { at } | MovieState::Paused { at }) => m.info.ms.saturating_sub(at),
                    (_, MovieState::Played) => 0,
                }) as i64);
            }
            "PassageTime" | "Conquest" | "Random" => return Value::Int(0),
            "ModuleFileName" => return Value::Str(self.file.to_string()),
            "Strstr" => return Value::Int(i64::from(arg(0).string().contains(&arg(1).string()))),
            "Integer" => return Value::Int(arg(0).int()),
            // Drawing and bookkeeping that changes nothing on screen.
            "SetVertex" | "SetFrequency" | "SetPan" | "WaitText" | "LockVideo" | "Reset" | "SetShortcut" => {}
            "CreateProcess" => self.note(format!("CreateProcess {}", arg(4).string())),
            other if BUILTINS.contains(&other) => self.note(format!("built-in {other}")),
            // No built-in and no NSS function has this name: the exe skips
            // the call (`Waitkey(500)`, `SL_Rightdown(…)`).
            other => self.note(format!("skipped: no function {other}")),
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
            _ => self.loaded_images.get(&name).cloned(),
        }
    }

    fn request(&mut self, name: &str, action: &str) {
        let Some(o) = self.objects.get_mut(name) else { return };
        if let Kind::Movie(m) = &mut o.kind {
            match action {
                "play" => {
                    m.state = match m.state {
                        MovieState::Paused { at } => MovieState::Playing { at },
                        _ => MovieState::Playing { at: 0 },
                    };
                    return;
                }
                "stop" => {
                    m.state = MovieState::Paused { at: 0 };
                    return;
                }
                "pause" => {
                    if let MovieState::Playing { at } = m.state {
                        m.state = MovieState::Paused { at };
                    }
                    return;
                }
                "subrender" => {
                    m.dark = true;
                    return;
                }
                // White smoke added: drawn over the scene as it is.
                "addrender" => return,
                _ => {}
            }
        }
        match action {
            "lock" => o.locked = true,
            "unlock" => o.locked = false,
            "hideable" => o.hideable = true,
            "play" => {
                // `Play` on a loop that is still playing goes on with it
                // (`md04_048vs` calls MusicStart again on its loop to change
                // the volume and speed); it isn't started a second time.
                if let Kind::Sound(s) = &mut o.kind
                    && !(s.looped && s.playing.is_some())
                {
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
    fn timed(&mut self, ms: i64, wait: &Value, targets: &[String]) {
        let ms = ms.max(0) as u32;
        match wait {
            Value::Null | Value::Int(0) => {
                self.pending_ms = self.pending_ms.max(ms);
                for t in targets {
                    let p = self.pending_by.entry(t.clone()).or_default();
                    *p = (*p).max(ms);
                }
            }
            Value::Int(n) if *n > 1 => {
                self.note("timed action that waits only part of its time");
                self.sync(ms);
            }
            _ => self.sync(ms),
        }
    }

    /// Where a child object's place counts from: the sum of the places of
    /// the objects its name runs through (`@frame03/顔窓31` at 25,25 in
    /// `frame03` at 25,80 is at 50,105).
    fn parent_offset(&self, name: &str) -> (i64, i64) {
        let (mut x, mut y) = (0, 0);
        if !self.game.relative_children {
            return (x, y);
        }
        let mut end = 0;
        while let Some(i) = name[end..].find('/') {
            end += i;
            if let Some(p) = self.objects.get(&name[..end]) {
                (x, y) = (x + p.x, y + p.y);
            }
            end += 1;
        }
        (x, y)
    }

    /// The screen is frozen while an opaque `Plain` cover is up.
    fn frozen(&self) -> bool {
        self.objects.values().any(|o| matches!(o.kind, Kind::Cover) && o.opacity >= 1000)
    }

    /// Shows the changes since the last sync, as a cross-fade over `ms`,
    /// or the pending actions' time up to [`UNWAITED_MS`] if longer.
    fn sync(&mut self, ms: u32) {
        let ms = ms.max(std::mem::take(&mut self.pending_ms).min(UNWAITED_MS));
        self.pending_by.clear();
        let mask = self.pending_mask.take();
        self.flush_sounds();
        self.check_window();
        if !self.shaken.is_empty() && !self.shaken.iter().any(|t| self.objects.get(t).is_some_and(|o| o.opacity > 0)) {
            self.shaken.clear();
            self.ops.push(Op::ScreenEffect { slot: 1, effect: None });
        }
        if self.frozen() {
            if ms > 0 {
                self.ops.push(Op::Wait { ms });
            }
            return;
        }
        // Wanted: visible pictures, by (priority, seq), top first.
        let want: Vec<_> = self
            .objects
            .iter()
            .filter_map(|(n, o)| match &o.kind {
                Kind::Image { image } if o.opacity > 0 && !o.hideable => {
                    let (px, py) = self.parent_offset(n);
                    let shown = Shown { layer: 0, image: image.clone(), x: o.x + px, y: o.y + py, w: o.w, h: o.h, opacity: o.opacity, cells: None };
                    Some((n.clone(), shown, (o.priority, o.seq), o.zoom, (o.w, o.h), o.blur))
                }
                _ => None,
            })
            .collect();
        let movies: Vec<(String, MovieObject, Object)> = self
            .objects
            .iter()
            .filter_map(|(n, o)| match &o.kind {
                Kind::Movie(m) if o.opacity > 0 && !o.hideable => Some((n.clone(), m.clone(), Object { kind: Kind::Other, ..o.clone() })),
                _ => None,
            })
            .collect();
        let mut want = want;
        for (n, m, o) in movies {
            // Playing: all its frames; else the one showing.
            let (at, looped) = match m.state {
                MovieState::Playing { .. } => (None, m.looped),
                MovieState::Paused { at } => (Some(at), false),
                MovieState::Played => (Some(m.info.ms.saturating_sub(1000 / MOVIE_FPS)), false),
            };
            let Some((image, cells, cw, ch)) = self.assets.movie_frames(&m.path, at, m.alpha, m.dark) else {
                self.note("movie frames missing");
                continue;
            };
            // The cells are drawn zoomed to the movie's size and its Zoom,
            // about the centre of its place.
            let zoom = ((o.zoom.0 * o.w / cw as i64 / 10) as i32, (o.zoom.1 * o.h / ch as i64 / 10) as i32);
            let (x, y) = (o.x + (o.w - cw as i64) / 2, o.y + (o.h - ch as i64) / 2);
            let shown = Shown { layer: 0, image, x, y, w: cw as i64, h: ch as i64, opacity: o.opacity, cells: Some((cells, looped, zoom)) };
            want.push((n, shown, (o.priority, o.seq), (1000, 1000), (o.w, o.h), 0));
        }
        // Zoomed pictures: scaled copies, centred where the original was.
        let mut want: Vec<(String, Shown, (i64, u64))> = want
            .into_iter()
            .map(|(n, mut shown, key, zoom, (w, h), blur)| {
                if zoom != (1000, 1000)
                    && let Some((image, x, y, sw, sh)) = self.assets.scaled(&shown.image, zoom.0, zoom.1, (shown.x, shown.y, w, h))
                {
                    (shown.x, shown.y) = (x, y);
                    (shown.w, shown.h) = (sw as i64, sh as i64);
                    shown.image = image;
                }
                if blur > 0
                    && let Some(image) = self.assets.blurred(&shown.image, blur)
                {
                    shown.image = image;
                }
                (n, shown, key)
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
        let mut animated = Vec::new();
        // Layers whose movie (zoomed with spzoom) goes: removed even when a
        // picture takes the layer, which would keep the zoom.
        let mut unzoom: Vec<u32> = changes.iter().filter(|c| self.shown.values().any(|s| s.layer == c.layer && s.cells.is_some())).map(|c| c.layer).collect();
        let mut new_shown = Vec::new();
        for ((name, now, _), layer) in want.into_iter().zip(&layers) {
            let Some(layer) = *layer else { continue };
            let now = Shown { layer, ..now };
            match self.shown.get(&name) {
                Some(old) if *old == now => continue,
                Some(old) if old.layer == now.layer && old.image == now.image && old.x == now.x && old.y == now.y && old.cells == now.cells => {
                    opacity.push(Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                }
                old => {
                    if let Some(old) = old.filter(|o| o.layer != layer || (o.cells.is_some() && now.cells.is_none())) {
                        changes.push(SpriteChange { layer: old.layer, sprite: None, keep_filters: false });
                        if old.cells.is_some() {
                            unzoom.push(old.layer);
                        }
                    }
                    let sprite = SpritePlacement { image: now.image.clone(), x: now.x as i32, y: now.y as i32 };
                    match now.cells {
                        Some((cells, looped, zoom)) => {
                            animated.push(Op::AnimatedSprite { layer, sprite, cells, frame_ms: 1000 / MOVIE_FPS, looped, zoom });
                        }
                        None => placed.push(SpriteChange { layer, sprite: Some(sprite), keep_filters: false }),
                    }
                    if now.opacity < 1000 {
                        opacity.push(Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                    }
                }
            }
            new_shown.push((name, now));
        }
        // Removals first: a placement may reuse a freed layer.
        changes.retain(|c| unzoom.contains(&c.layer) || !placed.iter().any(|p| p.layer == c.layer));
        changes.extend(placed);
        self.shown.extend(new_shown);
        if changes.is_empty() && animated.is_empty() && opacity.is_empty() {
            if ms > 0 {
                self.ops.push(Op::Wait { ms });
            }
            return;
        }
        let effect = if mask.is_some() { "DrawTransition" } else { "Fade" }.to_string();
        self.ops.push(Op::BeginTransition { effect, fade_ms: ms, mask, style: TransitionStyle::Fade });
        if !changes.is_empty() {
            self.ops.push(Op::Sprites { changes, fade_ms: 0 });
        }
        self.ops.extend(animated);
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
                if std::env::var_os("NSS_TRACE").is_some() {
                    eprintln!("missing sound {path}");
                }
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
        let text = TextObject { block: t, layout: self.box_layouts.get(&window).copied(), rotation: 0 };
        let (priority, x, y) = self.objects.get(&window).map_or((20001, 0, 0), |o| (o.priority, o.x, o.y));
        let o = self.create(name.clone(), Kind::Text(text), priority, 0, 0);
        (o.x, o.y) = (x, y);
        self.vars.insert("$SYSTEM_present_preprocess".into(), Value::Str(format!("@{window}")));
        self.vars.insert("$SYSTEM_present_text".into(), Value::Str(format!("@{name}")));
        if let Some(pending) = self.pending_enter.take() {
            if name.rsplit('/').next() == Some(pending.as_str()) {
                self.show_text(&name);
            } else {
                self.note("Request Enter for another text than the next");
            }
        }
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
        // The line ended in `<?>`: the next text line goes on in it.
        let mut joined = false;
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
                // `<?>` (not parsed as a tag): the text goes on after the
                // code that follows, which runs before the page anyway, on
                // the same line (a typewriter writing one character at a
                // time, `ma01_026`).
                Inline::Str(s) => {
                    joined = s.trim_end().ends_with("<?>");
                    line.push_str(&s.replace("<?>", ""));
                }
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
                    if joined && !line.is_empty() {
                        continue;
                    }
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
            // 続・殺戮のジャンゴ's src leaves out the voice folder
            // (`dja002/0100010ri`; system.ini: `path = voice/`).
            let voice = page.voice.as_ref().and_then(|(_, src)| self.assets.sound(src).or_else(|| self.assets.sound(&format!("voice/{src}"))));
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
            // Vertical text is full-width (CONscripter also exits on a
            // half-width character in vertical mode).
            let vertical = self.window.as_ref().is_some_and(|w| w.vertical);
            let text = if vertical { full_width(&page.text) } else { page.text.clone() };
            let mut ruby = page.ruby.clone();
            if vertical {
                for r in &mut ruby {
                    r.text = full_width(&r.text);
                }
            }
            self.ops.push(Op::Text {
                speaker: None,
                text,
                voice,
                links: Vec::new(),
                ruby,
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
        // The click-wait icon's place, set by each TypeBegin helper.
        let icon = |k: &str| match self.vars.get(k) {
            Some(Value::Int(v)) => Some(*v as i32),
            _ => None,
        };
        let wait_icon_at = icon("$SYSTEM_position_x_text_icon").zip(icon("$SYSTEM_position_y_text_icon"));
        TextWindow {
            wait_icon_at,
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

/// Half-width katakana and signs U+FF61-FF9F as full-width ones.
const HALF_KANA: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// ASCII letters, digits and signs, and half-width katakana, as their
/// full-width forms, one character for one (ruby positions stay valid).
fn full_width(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => '　',
            '!'..='~' => char::from_u32(c as u32 - 0x21 + 0xFF01).unwrap_or(c),
            '\u{FF61}'..='\u{FF9F}' => HALF_KANA.chars().nth(c as usize - 0xFF61).unwrap_or(c),
            _ => c,
        })
        .collect()
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
        fn scaled(&mut self, image: &str, zx: i64, zy: i64, at: (i64, i64, i64, i64)) -> Option<(String, i64, i64, u32, u32)> {
            let (w, h) = (1024 * zx / 1000, 576 * zy / 1000);
            Some((format!("{image}@{zx}x{zy}"), at.0 + (at.2 - w) / 2, at.1 + (at.3 - h) / 2, w as u32, h as u32))
        }
        fn blurred(&mut self, image: &str, sigma: u32) -> Option<String> {
            Some(format!("{image}~{sigma}"))
        }
        fn strip(&mut self, parts: &[&str]) -> Option<(String, u32, u32)> {
            Some((parts.join("|"), 100 * parts.len() as u32, 100))
        }
        fn movie_info(&mut self, path: &str) -> Option<MovieInfo> {
            let small = path.contains("k100");
            Some(MovieInfo { w: if small { 400 } else { 1024 }, h: if small { 432 } else { 576 }, ms: 2000, sound: path.contains("OP") })
        }
        fn movie_file(&mut self, path: &str, sound: bool) -> Option<String> {
            Some(format!("movie/{path}{}", if sound { "" } else { "-mute" }))
        }
        fn movie_frames(&mut self, path: &str, at: Option<u32>, alpha: bool, dark: bool) -> Option<(String, u32, u32, u32)> {
            let (w, h) = if alpha { (200, 108) } else { (1024, 576) };
            let cells = if at.is_some() { 1 } else { 30 };
            Some((format!("{path}@{at:?}{}", if dark { "d" } else { "" }), cells, w, h))
        }
    }

    fn compile(story: &str, library: &str) -> Compiled {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse(library).unwrap());
        files.insert("a.nss".to_string(), nss::parse(story).unwrap());
        compile_scene(&files, "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap()
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
        // SoundPlay and OnSE aren't defined here, and aren't built-ins.
        assert!(c.notes.contains_key("skipped: no function SoundPlay"));
    }

    #[test]
    fn names_are_case_sensitive_as_in_the_exe() {
        let story = r#"
scene a.nss
{
	Waitkey(500);
	WaitKey(300);
	sl_rightdown(1000);
}
"#;
        let library = "function SL_rightdown($t)\n{\n\tWait($t);\n}\n";
        let c = compile(story, library);
        // `Waitkey` and `sl_rightdown` are neither built-ins nor functions.
        let waits: Vec<u32> = c.ops.iter().filter_map(|op| match op { Op::Wait { ms } => Some(*ms), _ => None }).collect();
        assert_eq!(waits, [300], "{:?}", c.ops);
        assert!(c.notes.contains_key("skipped: no function Waitkey"));
    }

    #[test]
    fn a_cut_the_script_waits_for_plays_as_a_movie_then_shows_its_last_frame() {
        // MovieSEStart: wait for the cut, then for RemainTime + 500.
        let story = r#"
scene a.nss
{
	CreateMovie("cut", 100, 0, 0, false, false, "dx/mv剣戟.ngs");
	$t = RemainTime("cut");
	WaitAction("cut", 100);
	WaitKey($t + 500);
	Delete("cut");
}
"#;
        let c = compile(story, LIBRARY);
        let movie = c.ops.iter().position(|op| matches!(op, Op::SceneMovie { file, .. } if file == "movie/dx/mv剣戟.ngs-mute"));
        let last = c.ops.iter().position(|op| matches!(op, Op::AnimatedSprite { cells: 1, sprite, .. } if sprite.image.contains("Some(1934)")));
        assert!(movie.is_some() && last > movie, "{:?}", c.ops);
        // The movie's 2 s count as waited: 500 ms are left.
        assert!(c.ops.iter().any(|op| matches!(op, Op::Wait { ms: 500 })), "{:?}", c.ops);
        assert!(!c.ops.iter().any(|op| matches!(op, Op::Wait { ms } if *ms > 500)), "{:?}", c.ops);
    }

    #[test]
    fn a_smoke_loop_plays_as_frames_zoomed_and_a_paused_cut_as_one_frame() {
        let story = r#"
scene a.nss
{
	CreateMovie("smoke", 300, 0, 0, true, true, "dx/mvk100.ngs");
	Zoom("smoke", 0, 3000, 5000, null, true);
	CreateMovie("cut", 400, 0, 0, false, false, "dx/mv剣戟.ngs");
	Wait(300);
	Request("cut", Pause);
	Wait(100);
}
"#;
        let c = compile(story, LIBRARY);
        // 400x216 shown from 200x108 cells: 2x, then the zoom.
        let smoke = c.ops.iter().any(|op| matches!(op, Op::AnimatedSprite { cells: 30, looped: true, zoom: (600, 1000), .. }));
        let paused = c.ops.iter().any(|op| matches!(op, Op::AnimatedSprite { cells: 1, sprite, .. } if sprite.image.contains("Some(300)")));
        assert!(smoke && paused, "{:?}", c.ops);
        assert!(!c.ops.iter().any(|op| matches!(op, Op::SceneMovie { .. })), "{:?}", c.ops);
    }

    #[test]
    fn a_wildcard_matches_aliases() {
        let story = r#"
scene a.nss
{
	CreateTexture("@遠４/StMR01", 100, 1236, 0, "cg/st/a.png");
	SetAlias("@遠４/StMR01", "StMR01");
	Fade("@遠４/StMR01", 0, 1000, null, true);
	Move("@StMR*", 600, @-540, @0, null, true);
	Wait(100);
}
"#;
        let c = compile(story, LIBRARY);
        // The centre of the 1024-wide picture at x 696.
        let moved = c.ops.iter().any(|op| matches!(op, Op::SpriteMove { x: 1208, ms: 600, .. }));
        assert!(moved, "{:?}", c.ops);
    }

    #[test]
    fn wait_action_on_one_object_waits_only_for_its_actions() {
        let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Fade("bg", 0, 1000, null, true);
	CreateTexture("hp", 200, 0, 0, "cg/cp/hp.png");
	Zoom("bg", 15000, 1200, 1200, null, false);
	Fade("hp", 300, 500, null, false);
	WaitAction("hp", null);
	Wait(500);
}
"#;
        let c = compile(story, LIBRARY);
        let longest = c.ops.iter().filter_map(|op| match op {
            Op::BeginTransition { fade_ms, .. } => Some(*fade_ms),
            Op::Wait { ms } => Some(*ms),
            _ => None,
        }).max();
        assert!(longest.is_some_and(|ms| ms < 15000), "{:?}", c.ops);
    }

    #[test]
    fn play_on_a_playing_loop_doesnt_start_it_again() {
        let story = r#"
scene a.nss
{
	CreateSound("SE01", SE, "sound/se/loop_L");
	Request("SE01", Play);
	SetLoop("SE01", true);
	Wait(100);
	Request("SE01", Play);
	SetLoop("SE01", true);
	Wait(100);
}
"#;
        let c = compile(story, LIBRARY);
        let plays = c.ops.iter().filter(|op| matches!(op, Op::SePlay { .. })).count();
        assert_eq!(plays, 1);
    }

    #[test]
    fn a_line_ending_in_question_tag_goes_on_after_the_code() {
        let story = r#"
scene a.nss
{
<PRE @box0>
[text0030]
《<?>
{OnSE("se01");}
興<?>
{OnSE("se01");
WaitKey(50);}
隆<?>
{OnSE("se02");}
》

次<?>
</PRE>
	SetText();
	TypeBegin();
}
"#;
        let c = compile(story, LIBRARY);
        let texts: Vec<&str> = c.ops.iter().filter_map(|op| if let Op::Text { text, .. } = op { Some(text.as_str()) } else { None }).collect();
        assert_eq!(texts, ["《興隆》", "次"]);
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
    fn a_choice_forks_the_run_to_each_branch_file() {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function SetChoice02($テキストデータ１,$テキストデータ２){}").unwrap());
        let main = |name: &str, body: &str| format!("scene {name}_MAIN\n{{\n{body}\n}}\n");
        files.insert(
            "a.nss".into(),
            nss::parse(&main(
                "a.nss",
                r#"SetChoice02("はい","いいえ");
	select { if($x){$x=0;} case @選択肢１ { $GameName = "b.nss"; $flag = 1; } case @選択肢２ { $GameName = "c.nss"; } }"#,
            ))
            .unwrap(),
        );
        files.insert("b.nss".into(), nss::parse(&main("b.nss", r#"$GameName = "d.nss";"#)).unwrap());
        files.insert("c.nss".into(), nss::parse(&main("c.nss", r#"if($flag){$GameName = "b.nss";}else{$GameName = "d.nss";}"#)).unwrap());
        files.insert("d.nss".into(), nss::parse(&main("d.nss", "$GameCircle = 0;")).unwrap());
        let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
        assert_eq!(played, ["a.nss", "b.nss", "c.nss", "d.nss"]);
        let Some(Op::Choice { options, style }) = c.ops.iter().find(|op| matches!(op, Op::Choice { .. })) else { panic!() };
        assert_eq!(options.iter().map(|o| o.0.as_str()).collect::<Vec<_>>(), ["はい", "いいえ"]);
        // Two plates, the first at x 562; one vertical line on each.
        let w = &style.as_ref().unwrap().window;
        assert_eq!((w.x, w.rows, w.vertical, w.font_size + w.line_gap), (562, 2, true, 100));
        let var = conv_core::ir::Var { global: false, index: FORK_VARS.start };
        assert!(c.ops.contains(&Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(0), label: "f_b".into() }));
        assert!(c.ops.contains(&Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(1), label: "f_c".into() }));
        // $flag is a story flag (set and tested by story files): set in the
        // first branch at run time, tested in c.nss at run time.
        let flag = story_flags(&files, &Game::PLAIN)["$flag"];
        assert!(c.ops.contains(&Op::Arith { var: flag, op: ArithOp::Set, value: Operand::Const(1) }));
        let c_start = c.ops.iter().position(|op| *op == Op::Label("f_c".into())).unwrap();
        assert!(matches!(&c.ops[c_start + 1], Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(0), .. } if *var == flag), "{:?}", &c.ops[c_start..]);
    }

    #[test]
    fn a_choice_that_names_its_own_file_replays_it() {
        // As md05_010vsaa: option 1 shows some text and asks again.
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function SetChoice02($テキストデータ１,$テキストデータ２){}").unwrap());
        let main = |name: &str, body: &str| format!("scene {name}_MAIN\n{{\n{body}\n}}\n");
        let a = r#"SetChoice02("もう一度","終わる");
	select { case @選択肢１ { $GameName = "a.nss"; } case @選択肢２ { $GameName = "b.nss"; } }"#;
        files.insert("a.nss".into(), nss::parse(&main("a.nss", a)).unwrap());
        files.insert("b.nss".into(), nss::parse(&main("b.nss", "$GameCircle = 0;")).unwrap());
        let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
        assert_eq!(played, ["a.nss", "b.nss"]);
        let var = conv_core::ir::Var { global: false, index: FORK_VARS.start };
        assert!(c.ops.contains(&Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(0), label: "f_a".into() }), "{:?}", c.ops);
    }

    /// Runs the variable and jump ops of `ops` (everything else is skipped).
    fn run_ir(ops: &[Op]) -> HashMap<conv_core::ir::Var, i32> {
        run_ir_picking(ops, &[])
    }

    /// [`run_ir`], taking option `picks[n]` (0-based) at the nth choice.
    fn run_ir_picking(ops: &[Op], picks: &[usize]) -> HashMap<conv_core::ir::Var, i32> {
        use conv_core::ir::Compare;
        let mut picks = picks.iter();
        let mut vars: HashMap<conv_core::ir::Var, i32> = HashMap::new();
        let labels: HashMap<&str, usize> = ops.iter().enumerate().filter_map(|(i, op)| if let Op::Label(l) = op { Some((l.as_str(), i)) } else { None }).collect();
        let mut pc = 0;
        let mut steps = 0;
        while pc < ops.len() && steps < 100_000 {
            steps += 1;
            let value = |o: &Operand, vars: &HashMap<_, i32>| match o {
                Operand::Const(n) => *n,
                Operand::Var(v) => vars.get(v).copied().unwrap_or(0),
            };
            match &ops[pc] {
                Op::Jump(l) => {
                    pc = labels[l.as_str()];
                    continue;
                }
                Op::Choice { options, .. } => {
                    pc = labels[options[*picks.next().unwrap_or(&0)].1.as_str()];
                    continue;
                }
                Op::If { var, cmp, value: v, label } => {
                    let (a, b) = (vars.get(var).copied().unwrap_or(0), value(v, &vars));
                    let hit = match cmp {
                        Compare::Eq => a == b,
                        Compare::Ne => a != b,
                        Compare::Lt => a < b,
                        Compare::Gt => a > b,
                        Compare::Le => a <= b,
                        Compare::Ge => a >= b,
                    };
                    if hit {
                        pc = labels[label.as_str()];
                        continue;
                    }
                }
                Op::Arith { var, op, value: v } => {
                    let n = value(v, &vars);
                    let e = vars.entry(*var).or_insert(0);
                    match op {
                        ArithOp::Set => *e = n,
                        ArithOp::Add => *e += n,
                        ArithOp::Sub => *e -= n,
                        _ => {}
                    }
                }
                _ => {}
            }
            pc += 1;
        }
        vars
    }

    #[test]
    fn a_scene_name_set_by_a_runtime_if_is_called_at_run_time_in_each_file() {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function F(){}").unwrap());
        // As md05_004*: the scene to call is picked by a story flag, and
        // every file uses the same helper variable for it.
        let file = |name: &str, x: i32, next: &str| {
            format!(
                r#"scene {name}_MAIN
{{
	if($f == true){{$S = "@->"+$GameName+"_X";}}
	else{{$S = "@->"+$GameName+"_Y";}}
	call_scene $S;
	$GameName = "{next}";
}}
scene {name}_X
{{
	$g = {x};
}}
scene {name}_Y
{{
	$g = {};
}}"#,
                x + 1
            )
        };
        files.insert("a.nss".into(), nss::parse(&file("a.nss", 1, "b.nss")).unwrap());
        files.insert("b.nss".into(), nss::parse(&file("b.nss", 3, "d.nss")).unwrap());
        files.insert("d.nss".into(), nss::parse("scene d.nss_MAIN\n{\n\tif($g){$f = 1;}\n\t$GameCircle = 0;\n}").unwrap());
        let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
        assert_eq!(played, ["a.nss", "b.nss", "d.nss"]);
        let flags = story_flags(&files, &Game::PLAIN);
        // $f is 0: each file calls its own _Y scene, b.nss's last.
        assert_eq!(run_ir(&c.ops).get(&flags["$g"]), Some(&4));
        assert!(!c.notes.contains_key("call_scene to a runtime value with no known names"), "{:?}", c.notes);
    }

    #[test]
    fn flags_a_choice_sets_are_tested_at_run_time_after_it() {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function F(){}").unwrap());
        // As mc01_010vs: the cases set flags, the next file is picked after
        // the select.
        let a = r#"scene a.nss_MAIN
{
	select { case @選択肢１ { $x = true; } case @選択肢２ { $y = true; } }
	if($x == true){ $GameName = "b.nss"; } else if($y){ $GameName = "c.nss"; }
}"#;
        files.insert("a.nss".into(), nss::parse(a).unwrap());
        for n in ["b.nss", "c.nss"] {
            files.insert(n.into(), nss::parse(&format!("scene {n}_MAIN\n{{\n\t$GameCircle = 0;\n}}")).unwrap());
        }
        let (_, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
        assert_eq!(played, ["a.nss", "b.nss", "c.nss"]);
    }

    #[test]
    fn a_choice_inside_a_runtime_if_hands_each_option_s_file_to_the_code_after_it() {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function F(){}").unwrap());
        // As the airship rooms (mc02_031souko2): the select sets $GameName,
        // the code after it hands that to a story flag and goes through the
        // clock file, which dispatches on the flag.
        let a = r#"scene a.nss_MAIN
{
	if($joined == true){ $GameName = "a.nss"; }
	else{
		select { case @選択肢１ { $GameName = "b.nss"; } case @選択肢２ { $GameName = "c.nss"; } }
	}
	$Next = $GameName;
	$GameName = "t.nss";
}"#;
        files.insert("a.nss".into(), nss::parse(a).unwrap());
        files.insert("t.nss".into(), nss::parse("scene t.nss_MAIN
{
	$GameName = $Next;
}").unwrap());
        let t_reads = "scene x.nss_MAIN
{
	if($Next == \"b.nss\"){$joined = true;}
	$GameCircle = 0;
}";
        for n in ["b.nss", "c.nss"] {
            files.insert(n.into(), nss::parse(&t_reads.replace("x.nss", n)).unwrap());
        }
        let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
        assert!(played.contains(&"c.nss".to_string()), "{played:?}");
        let flags = story_flags(&files, &Game::PLAIN);
        let next = flags["$Next"];
        let code = |name: &str| c.ops.iter().find_map(|op| match op {
            Op::If { var, value: Operand::Const(n), label, .. } if *var == next && label == &file_label(name) => Some(*n),
            _ => None,
        });
        for (pick, file) in [(0, "b.nss"), (1, "c.nss")] {
            let vars = run_ir_picking(&c.ops, &[pick]);
            assert_eq!(Some(vars[&next]), code(file), "option {} should hand on {file}", pick + 1);
        }
    }

    #[test]
    fn full_width_maps_ascii_and_half_width_kana_one_for_one() {
        assert_eq!(full_width("A!? ･ｶﾞｰ｡"), "Ａ！？　・カ゛ー。");
        assert_eq!(HALF_KANA.chars().count(), 0xFF9F - 0xFF61 + 1);
    }

    #[test]
    fn route_ranking_runs_as_runtime_comparisons() {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function RouteChicker(){}").unwrap());
        let story = r#"scene a.nss_MAIN
{
	$Others_Flag = 1; $Ichizyou_Flag = 3; $Kanae_Flag = 5; $Muramasa_Flag = 5;
	RouteChicker();
	$GameCircle = 0;
}"#;
        files.insert("a.nss".into(), nss::parse(story).unwrap());
        let (c, _) = compile_run(&files, "a.nss", 1, &MURAMASA, &mut Fake).unwrap();
        let vars = run_ir(&c.ops);
        let flags = story_flags(&files, &MURAMASA);
        let rank = |n: &str| vars.get(&flags[&format!("$RC_N[{n}]")]).copied();
        // 香奈枝 and 村正 tie at 5: 香奈枝 is checked first.
        assert_eq!([rank("香奈枝"), rank("村正"), rank("一条"), rank("アナザー")], [Some(1), Some(2), Some(3), Some(4)]);
    }

    #[test]
    fn the_heroine_with_the_highest_counter_dies_and_set_hex_clears_her_counter() {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function SatugaiSystem(){}\nfunction SetHex(){}").unwrap());
        let story = r#"scene a.nss_MAIN
{
	$Others_Flag = 1; $Ichizyou_Flag = 4; $Kanae_Flag = 4; $Muramasa_Flag = 2;
	SatugaiSystem();
	SetHex();
	if($一条死亡 == true){ $x = 1; }
	$GameCircle = 0;
}"#;
        files.insert("a.nss".into(), nss::parse(story).unwrap());
        let (c, _) = compile_run(&files, "a.nss", 1, &MURAMASA, &mut Fake).unwrap();
        let vars = run_ir(&c.ops);
        let flags = story_flags(&files, &MURAMASA);
        let get = |k: &str| vars.get(&flags[k]).copied().unwrap_or(0);
        // 一条 and 香奈枝 tie at 4: 一条 comes first in SatugaiSystem's order.
        assert_eq!([get("$一条死亡"), get("$香奈枝死亡"), get("$その他死亡"), get("$村正死亡")], [1, 0, 0, 0]);
        assert_eq!([get("$Ichizyou_Flag"), get("$Kanae_Flag")], [0, 4]);
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
