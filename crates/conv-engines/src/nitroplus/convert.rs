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
//!   sound effect at that volume; `SetVolume` on a playing sound changes
//!   its volume, and reaching 0 stops it.
//!
//! Conditions are evaluated with the values known at convert time (a fresh
//! game: unset variables are empty), which holds for linear scenes. What
//! isn't converted is counted in [`Compiled::notes`].

use std::collections::{BTreeMap, HashMap};

use conv_core::ir::{ArithOp, ChoiceStyle, CLICK_WAIT_MARK, MotionEnd, MotionKey, Op, Operand, ScreenEffect, Ruby, Script, TextStyle, SpriteChange, SpritePlacement, TextWindow, TransitionStyle, Voice};

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
/// Globals (`#`) past the first [`GLOBAL_ROOM`] go from here (NScripter
/// %5200 up, past the settings switches at %970-%5095).
const GLOBAL_OVERFLOW: u16 = 4400;
/// Globals from [`GLOBAL_BORDER`] up to the menus' globals (%900-%912:
/// the save page, volumes, text speed; then the voice switches at %970).
const GLOBAL_ROOM: u16 = 100;
/// Where a run of files ends.
const RUN_END: &str = "conv_run_end";
/// Set (in the convert-time variables) when a file assigns `$GameName`,
/// so that a file naming itself as the next one (a retry) is told apart
/// from one that names no next file.
const GAME_NAME_SET: &str = "$GameName (set)";
mod builtin;
mod sync;
mod muramasa;
pub use muramasa::{MURAMASA, MURAMASA_EN};
mod django;
pub use django::DJANGO;
mod process;

/// What one game adds to the compiler, in its own module under `convert/`
/// ([`MURAMASA`]): library functions it compiles itself (written as
/// NScripter code instead of running their loops), scenes it replaces with
/// a port (a minigame), and the story flags its own code sets and tests.
/// The CLI picks the game when it builds.
pub struct Game {
    /// A call of a function with its arguments: true if the game
    /// compiled it.
    call: for<'a, 'b> fn(&mut Compiler<'a, 'b>, &str, &[Value]) -> bool,
    /// A call of scene (file, scene): true if the game replaced it.
    scene: for<'a, 'b> fn(&mut Compiler<'a, 'b>, &str, &str) -> bool,
    /// Story flags the game adds (it may read them from the scripts).
    flags: fn(&BTreeMap<String, NssFile>) -> Vec<String>,
    /// Library files the story may name as its next file (ported).
    files: &'static [&'static str],
    /// The game's system screens (title, menus, dialogs), built from its
    /// own pictures into the script.
    menus: fn(&mut dyn Assets, &mut Script, &Story),
    /// Chapters compiled once more as subroutines (label, `nss/file.nss`)
    /// for the system screens to call, such as a screen the menu opens.
    subroutines: &'static [(&'static str, &'static str)],
    /// Story files compiled once more as subroutines with variables set, for
    /// a replay of a scene (装甲悪鬼村正's 回想映写機).
    replays: fn(&BTreeMap<String, NssFile>) -> Vec<Replay>,
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
    /// A `Zoom` keeps the object's `SetVertex` point in place (続・殺戮の
    /// ジャンゴ's standing pictures zoom about their bottom centre). Off
    /// for 装甲悪鬼村正: not checked against it.
    zoom_vertex: bool,
    /// The objects a chapter file made are deleted when it ends, except
    /// those hung under an object made before it (続・殺戮のジャンゴ: its
    /// `窓` and `音` windows are made for pictures and sounds to "hang on
    /// between files"; a scene's fade to black `黒幕` is never deleted
    /// otherwise). Inferred from the scripts, not checked in the exe.
    /// `Delete("*")` in a chapter deletes the same objects: dja009 waits
    /// for its music (`WaitPlay("@dj*")`) right after one, and its music
    /// objects, made once before the first chapter, play in the later
    /// chapters.
    chapter_objects: bool,
    /// Horizontal text starts this far below its box's top (the line's
    /// ruby space above it): 続・殺戮のジャンゴ's first row of ＭＳ Ｐ
    /// ゴシック is drawn from y 480 in `box00` at 472 (under wine). 0 for
    /// 装甲悪鬼村正, whose text is vertical.
    text_drop: i64,
    /// Children follow their parents: `@移動窓/立絵40` names a child of
    /// the object `移動窓` is an alias of (`窓/移動窓`), and deleting an
    /// object deletes its children (続・殺戮のジャンゴ: pictures under a
    /// deleted window stayed up). Off for 装甲悪鬼村正 until checked: it
    /// changes 621 places of its script.
    family: bool,
    /// How a choice of `n` options looks.
    choice_style: for<'a, 'b> fn(&mut Compiler<'a, 'b>, i64) -> Option<ChoiceStyle>,
    /// Functions (of the boot script or its library) the title runs when
    /// it comes up, played in turn at the end of [`compile_story`]'s story,
    /// before it goes back to the title (続・殺戮のジャンゴ's `InitBGM`,
    /// `TitleCongratulations`: the latter reads the flags the ending just
    /// set).
    story_end: &'static [&'static str],
    /// A vertical (90° `Rotate`d) text box is written horizontally in a
    /// box of this width and height, with this line pitch, from the top
    /// left of the vertical text: Full Metal Daemon Muramasa's English
    /// port, whose `text0` is 280×150 (`function_textbox.nut`).
    horizontal_box: Option<(i64, i64, i64)>,
    /// Where the text starts and how wide it is while the named window
    /// picture is shown: the first of these shown (Full Metal Daemon
    /// Muramasa's English port, `BoxInfo` in `val.nut`).
    box_text: &'static [(&'static str, i64, i64, i64)],
}

impl Game {
    /// A game with nothing of its own: NSS as it is.
    /// Its screen and boot script are 装甲悪鬼村正's.
    pub const PLAIN: Game = Game {
        call: |_, _, _| false,
        scene: |_, _, _| false,
        flags: |_| Vec::new(),
        files: &[],
        menus: |_, _, _| {},
        subroutines: &[],
        replays: |_| Vec::new(),
        screen: (SCREEN_W, SCREEN_H),
        boot: BOOT,
        story: None,
        relative_children: false,
        loaded_textures: false,
        zoom_vertex: false,
        chapter_objects: false,
        text_drop: 0,
        family: false,
        choice_style: |c, n| c.vertical_choice_style(n),
        story_end: &[],
        horizontal_box: None,
        box_text: &[],
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
    /// [`composite`](Self::composite) cut to the screen (parts that reach
    /// past it, such as the English title's logo 40 px up).
    fn screen(&mut self, parts: &[(String, i64, i64, i64)]) -> Option<String> {
        self.composite(parts).map(|c| c.0)
    }
    /// A fully transparent picture.
    fn transparent(&mut self) -> String;
    /// An output image scaled to `zx`, `zy` per mille about the centre of
    /// the box `at` (x, y, w, h) it fills unscaled: the picture to draw
    /// (perhaps only its part on the screen), its top-left and size.
    fn scaled(&mut self, image: &str, zx: i64, zy: i64, at: (i64, i64, i64, i64)) -> Option<(String, i64, i64, u32, u32)>;
    /// An output image blurred with a Gaussian of `sigma`/10 px.
    fn blurred(&mut self, image: &str, sigma: u32) -> Option<String>;
    /// An output image mirrored left to right (`x`) and/or upside down
    /// (`y`).
    fn mirrored(&mut self, image: &str, x: bool, y: bool) -> Option<String>;
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
    /// A movie the engine plays as a picture (`lsp` of a movie file),
    /// silent: its output path and the size it is drawn at. `alpha`: the
    /// movie's top half is the colour, its bottom half the opacity, made
    /// into the right half that `:a` takes (named `*_alpha.mp4`); `dark`:
    /// the colour is drawn black (`SubRender`).
    fn movie_sprite(&mut self, path: &str, alpha: bool, dark: bool) -> Option<(String, u32, u32)>;
    /// The output picture `base` (filling the screen) with the picture the
    /// scripts name `top` drawn over it in overlay (`OverlayRender`), at
    /// (0,0) zoomed `zoom` per mille about its centre, at `opacity`
    /// (0-1000): the full-screen picture's path.
    fn overlaid(&mut self, base: &str, top: &str, zoom: i64, opacity: i64) -> Option<String>;
    /// A picture decoded: one the scripts name (`cg/…`) or one in the
    /// output (`system/…`). `None` in a dry run.
    fn rgba(&mut self, path: &str) -> Option<image::RgbaImage>;
    /// A silent movie, `movie/<name>.mp4`, of `frames` screen-sized
    /// pictures at `fps`, each drawn by `draw` (frame number, picture). An
    /// existing one is kept (change the name to draw it again); a dry run
    /// draws nothing. Its output path.
    fn frames_movie(&mut self, name: &str, fps: u32, frames: u32, draw: &mut dyn FnMut(u32, &mut image::RgbImage)) -> Option<String>;
    /// A strip of `frames` pictures of `w`×`h` side by side,
    /// `system/strip/<name>.png`, each drawn by `draw` (frame number, a
    /// clear picture). An existing one is kept (change the name to draw it
    /// again); a dry run draws nothing. Its output path.
    fn frames_strip(&mut self, name: &str, w: u32, h: u32, frames: u32, draw: &mut dyn FnMut(u32, &mut image::RgbaImage)) -> Option<String>;
    /// A sound the scripts name (`sound/se/x`), starting after `delay_ms`
    /// of silence: its output path.
    fn delayed_sound(&mut self, path: &str, delay_ms: u32) -> Option<String>;
    /// A sound's length (`sound/bgm/x`), in a dry run too.
    fn sound_ms(&mut self, path: &str) -> Option<u32>;
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
    /// The game's scripts (the system screens' own, `sys_*` and `extra_*`,
    /// hold their lists).
    pub files: &'a BTreeMap<String, NssFile>,
}

/// A story file played on its own from a gosub to `label`, its
/// `X.nss_MAIN` scene with `set` variables at convert time (`$PLACE_reco`
/// = 1) and `$GameName` its name ([`Game::replays`]).
pub struct Replay {
    pub label: String,
    pub file: String,
    pub set: Vec<(String, i64)>,
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
    // The story ends back at the title, as the boot script's loop does.
    c.ops.push(Op::Label(RUN_END.to_string()));
    if !game.story_end.is_empty() {
        c.restore(initial.clone());
        for (k, &v) in &c.flags {
            c.vars.insert(k.clone(), Value::Runtime(v));
        }
    }
    for name in game.story_end {
        let in_boot = files.get_key_value(game.boot).and_then(|(boot, nss)| {
            nss.items.iter().find_map(|item| match item {
                Item::Function(f) if f.name == *name => Some((boot.as_str(), f)),
                _ => None,
            })
        });
        match in_boot.or_else(|| c.function(name)) {
            Some((file, f)) => {
                c.call_function(file, f, Vec::new());
            }
            None => c.note(format!("story_end: no function {name}")),
        }
    }
    c.sync(0);
    c.ops.push(Op::ReturnToTitle);
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
        // Other paths jump to the file's label with another window up
        // (a vertical choice's): its first page sets its own.
        c.window = None;
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
    for key in (game.flags)(files) {
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
            let index = if global <= GLOBAL_ROOM { global - 1 } else { GLOBAL_OVERFLOW + global - 1 - GLOBAL_ROOM };
            conv_core::ir::Var { global: true, index }
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
    /// `CreateProcess` of a function that only plays music on a timer
    /// ([`Compiler::start_process`]).
    Process { file: &'a str, function: &'a Function },
    /// Pictures drawn by a game's own code ([`Assets::frames_strip`]),
    /// shown once in turn while the script goes on, the last one staying.
    Strip(StripObject),
    /// `CreateCamera`: what its children show. Its place is the point the
    /// screen looks at (they are drawn that much the other way); its zoom
    /// `z` and a child's priority `p` (its depth) magnify the child
    /// `z / (1000 - p)` times about the screen's centre; one of priority
    /// 1000 or more isn't drawn. Measured under wine (続・殺戮のジャンゴ,
    /// boxes at four priorities and six camera settings).
    Camera,
    /// A colour added onto the screen, flashing while the script goes on
    /// (its own code, as 続・殺戮のジャンゴ's `GunFire`).
    Flash(FlashObject),
}

/// A [`Kind::Flash`]: `image` (a colour) added on at opacity (0-256)
/// following `keys` (ms from its start, opacity) `times` times, then
/// staying at the last key's ([`Op::SpriteFlashing`]). `started` once
/// placed.
#[derive(Debug, Clone)]
struct FlashObject {
    image: String,
    keys: Vec<(u32, u32)>,
    times: u32,
    started: bool,
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

/// A [`Kind::Strip`]: `cells` pictures of `w`×`h` side by side in
/// `image`, `frame_ms` each, drawn zoomed `zoom` percent about the centre
/// of their place (the object's place, `w`×`h`). Placed again (on another
/// layer), it shows `last`, its last picture, at once.
#[derive(Debug, Clone)]
struct StripObject {
    image: String,
    cells: u32,
    frame_ms: u32,
    w: i64,
    h: i64,
    zoom: (i32, i32),
    last: String,
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
    /// `SetVertex`: the point (from the top left) a zoom keeps in place,
    /// with [`Game::zoom_vertex`]; `None` = the centre.
    vertex: Option<(i64, i64)>,
    /// `Rotate`: degrees about the x, y and z axes. A picture turned past
    /// 90° about y shows mirrored left to right, about x upside down, and
    /// about z (a half turn) both; the turns combine. Other turns aren't
    /// drawn.
    turn: (i64, i64, i64),
}

impl Object<'_> {
    /// Mirrored (left to right, upside down) as its turn shows it.
    fn mirror(&self) -> (bool, bool) {
        let back = |deg: i64| (91..270).contains(&deg.rem_euclid(360));
        let half = back(self.turn.2);
        (back(self.turn.1) != half, back(self.turn.0) != half)
    }
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
    /// `<I>` and `<center>` (the English port's); an `<I>` not closed yet
    /// runs to the page's end.
    styles: Vec<TextStyle>,
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

/// `old` moved and zoomed into `now` over `ms` by the engine: the same
/// picture at the same opacity, zoomed alike both ways.
fn motion(old: &Shown, now: &Shown, ms: u32) -> Option<Op> {
    if old.layer != now.layer || old.source != now.source || old.opacity != now.opacity || old.cells.is_some() || now.cells.is_some() {
        return None;
    }
    let (zx, zy) = (now.zoom.0 * 1000 / old.zoom.0.max(1), now.zoom.1 * 1000 / old.zoom.1.max(1));
    // (A picture cut to the screen, see `Assets::scaled`, isn't the whole.)
    let sized = |a: i64, b: i64, z: i64| (b * 1000 - a * z).abs() <= a * 10 + 1000;
    if (zx - zy).abs() > 5 || !sized(old.w, now.w, zx) || !sized(old.h, now.h, zy) {
        return None;
    }
    let centre = |s: &Shown| (s.x + s.w / 2, s.y + s.h / 2);
    let ((x0, y0), (x1, y1)) = (centre(old), centre(now));
    let zoom = ((zx + 5) / 10).clamp(1, 10000) as u32;
    let key = |ms, x, y, zoom| MotionKey { ms, x, y, zoom, angle: 0, opacity: 256 };
    let keys = vec![key(0, 0, 0, 100), key(ms, (x1 - x0) as i32, (y1 - y0) as i32, zoom)];
    Some(Op::SpriteMotion2 { layer: old.layer, keys, end: MotionEnd::Keep })
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
    /// Movie frames ([`Op::AnimatedSprite`]): cells, looped, zoom percent,
    /// ms a cell.
    cells: Option<(u32, bool, (i32, i32), u32)>,
    /// The picture before its zoom (mirrored, blurred), and the zoom (per
    /// mille): a picture that only moves or zooms over time is moved by
    /// the engine ([`Op::SpriteMotion2`]).
    source: String,
    zoom: (i64, i64),
    engine: Engine,
    /// A [`Kind::Flash`]'s `seq` (0 for any other picture): a new flash
    /// starts again.
    flash: u64,
}

/// What the engine's motions did to a [`Shown`] picture.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Engine {
    /// It draws an earlier zoom's picture, zoomed: another picture on the
    /// layer is loaded on a fresh sprite.
    zoomed: bool,
    /// It moved (and may still be moving): not counted as covering what
    /// is under it.
    moved: bool,
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
    /// While a music process runs ([`Compiler::start_process`]): its time
    /// so far, ms.
    process_clock: Option<u32>,
    /// [`Compiler::seq`] when the chapter being run began.
    chapter_mark: Option<u64>,
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
    /// The last runtime `if` whose branch returned from the chapter
    /// (`if($死亡10 == 1) { return; }` after a choice): its test, the
    /// state the run left with, and what the chapter's end did to it (its
    /// sounds stopped). An `if` on the same test right after the chapter
    /// (the caller's `if($死亡10 == 1) { call_chapter …be.nss; }`) starts
    /// that branch from it, not from the other branch's end.
    returned: Option<(&'a Expr, State<'a>, Vec<Op>)>,
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
        // Label, target, variables set.
        type Entry = (String, String, Vec<(String, Value)>);
        let mut list: Vec<Entry> = self.game.subroutines.iter().map(|&(l, t)| (l.to_string(), t.to_string(), Vec::new())).collect();
        for r in (self.game.replays)(self.files) {
            let mut set: Vec<(String, Value)> = r.set.into_iter().map(|(k, n)| (k, Value::Int(n))).collect();
            set.push(("$GameName".to_string(), Value::Str(r.file.clone())));
            list.push((r.label, format!("nss/{}->{}_MAIN", r.file, r.file), set));
        }
        for (label, target, set) in list {
            self.restore(initial.clone());
            for (k, &v) in &self.flags {
                self.vars.insert(k.clone(), Value::Runtime(v));
            }
            for (k, v) in set {
                self.vars.insert(k, v);
            }
            self.dynamic.clear();
            self.dynamic_values.clear();
            self.next_dynamic = dynamic_base + kept.len() as u16;
            self.steps = 0;
            self.forks_in_file = 0;
            self.leaves_in_file = 0;
            self.ops.push(Op::Label(label));
            for (i, &v) in kept.iter().enumerate() {
                self.ops.push(Op::Arith { var: local(dynamic_base + i as u16), op: ArithOp::Set, value: Operand::Var(local(v)) });
            }
            self.call_scene(&target, true);
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
        let style = (self.game.choice_style)(self, texts.len() as i64);
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

    /// 装甲悪鬼村正's choice plates of `SetChoiceBase` (function_select.nss):
    /// `n` vertical plates 100 px apart, the first at the right, centred on
    /// the screen; the options in black, vertical, one per plate.
    fn vertical_choice_style(&mut self, n: i64) -> Option<ChoiceStyle> {
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
        Some(ChoiceStyle { window, on_color: 0xA00000, off_color: 0x000000, plates: None })
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
        let returned = self.returned.take().filter(|r| r.0 == cond);
        self.sync(0);
        let own = self.begin_fork();
        let n = self.choices;
        self.choices += 1;
        let (else_label, end_label) = (format!("if{n}_else"), format!("if{n}_end"));
        self.jump_if(cond, false, &else_label);
        let base = self.snapshot();
        let (then_base, stops) = match returned {
            Some((_, state, stops)) => (state, stops),
            None => (base.clone(), Vec::new()),
        };
        let mut leaves = self.branch(&then_base, |c| {
            c.ops.extend(stops);
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
            (true, false) => {
                if let [leaf] = &leaves[..] {
                    self.returned = Some((cond, leaf.state.clone(), Vec::new()));
                }
                leaves = else_leaves;
            }
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
            process_clock: None,
            chapter_mark: None,
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
            returned: None,
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
        let saved_mark = self.chapter_mark;
        if chapter {
            self.returned = None;
            self.chapter_mark = Some(mark);
        }
        self.depth += 1;
        self.frames.push(HashMap::new());
        self.exec_block(body);
        self.frames.pop();
        self.depth -= 1;
        self.file = saved;
        self.chapter_mark = saved_mark;
        if chapter && self.game.chapter_objects {
            self.delete_chapter_objects(mark);
            // The way that returned early leaves the chapter too.
            if let Some((cond, state, mut stops)) = self.returned.take() {
                let (now, at) = (self.snapshot(), self.ops.len());
                let pending = (self.pending_ms, self.pending_mask.take());
                self.restore(state);
                self.delete_chapter_objects(mark);
                stops.extend(self.ops.drain(at..));
                self.returned = Some((cond, self.snapshot(), stops));
                self.restore(now);
                (self.pending_ms, self.pending_mask) = pending;
            }
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
                if (self.game.call)(self, name, &args) {
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
            s = match self.aliases.get(rest) {
                Some(o) => o.clone(),
                // A child named from its parent's alias (`@移動窓/立絵40`,
                // `移動窓` being `窓/移動窓`).
                None => match rest.split_once('/') {
                    Some((parent, child)) if self.game.family && self.aliases.contains_key(parent) => format!("{}/{child}", self.aliases[parent]),
                    _ => rest.to_string(),
                },
            };
        }
        s
    }

    /// Objects matched by a name argument (a trailing `*` is a prefix
    /// wildcard, over names and aliases: `@StMR*` finds `遠４/StMR01`,
    /// alias `StMR01`).
    fn matching(&self, v: &Value) -> Vec<String> {
        let name = self.object_name(v);
        // A `*` before the end (`@選択肢*/MouseUsual/*`): any characters;
        // the first part may name aliases.
        if self.game.family && name.trim_end_matches('*').contains('*') {
            let mut found: std::collections::BTreeSet<String> = self.objects.keys().filter(|k| glob(&name, k)).cloned().collect();
            if let Some((first, rest)) = name.split_once('/') {
                for (a, o) in &self.aliases {
                    if glob(first, a) {
                        let pattern = format!("{o}/{rest}");
                        found.extend(self.objects.keys().filter(|k| glob(&pattern, k)).cloned());
                    }
                }
            }
            return found.into_iter().collect();
        }
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
        let o = Object { kind, priority, x: 0, y: 0, w, h, opacity: 1000, locked: false, hideable: false, seq: self.seq, zoom: (1000, 1000), blur: 0, vertex: None, turn: (0, 0, 0) };
        self.objects.insert(name.clone(), o);
        self.objects.get_mut(&name).unwrap()
    }

    fn delete(&mut self, name: &str) {
        // Its children go with it.
        let children: Vec<String> = self.objects.keys().filter(|_| self.game.family).filter(|k| k.len() > name.len() && k.starts_with(name) && k.as_bytes()[name.len()] == b'/').cloned().collect();
        for c in children {
            self.delete(&c);
        }
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
            "start" if matches!(o.kind, Kind::Process { .. }) => self.start_process(name),
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
            _ => {
                if matches!(wait, Value::Int(n) if *n > 1) {
                    self.note("timed action that waits only part of its time");
                }
                for t in targets {
                    let p = self.pending_by.entry(t.clone()).or_default();
                    *p = (*p).max(ms);
                }
                self.sync(ms);
            }
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
            match self.objects.get(&name[..end]) {
                // Seen through a camera: the other way.
                Some(p) if matches!(p.kind, Kind::Camera) => (x, y) = (x - p.x, y - p.y),
                Some(p) => (x, y) = (x + p.x, y + p.y),
                None => {}
            }
            end += 1;
        }
        (x, y)
    }

    /// How much (per mille) the cameras `name` is seen through magnify an
    /// object of `priority` ([`Kind::Camera`]); `None`: at or before the
    /// camera, not drawn.
    fn camera_zoom(&self, name: &str, priority: i64) -> Option<i64> {
        let mut zoom = 1000;
        let mut end = 0;
        while let Some(i) = name[end..].find('/') {
            end += i;
            if let Some(p) = self.objects.get(&name[..end]).filter(|p| matches!(p.kind, Kind::Camera)) {
                if priority >= 1000 {
                    return None;
                }
                zoom = zoom * p.zoom.0 / (1000 - priority);
            }
            end += 1;
        }
        Some(zoom)
    }

    /// The screen is frozen while an opaque `Plain` cover is up.
    fn frozen(&self) -> bool {
        self.objects.values().any(|o| matches!(o.kind, Kind::Cover) && o.opacity >= 1000)
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
                // Reaching 0 stops it.
                let file = s.playing.take().unwrap();
                self.ops.push(match (s.bgm, ms) {
                    (true, _) => Op::BgmStop { fade_out_ms: ms },
                    (false, 0) => Op::SeStop { file: Some(file) },
                    (false, ms) => Op::SeFadeOut { file: Some(file), ms },
                });
            } else if let Some(file) = &s.playing {
                s.volume = volume;
                let file = (!s.bgm).then(|| file.clone());
                self.ops.push(Op::SoundVolume { file, percent: percent(volume), ms });
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
            let (path, bgm, looped, fade_ms, volume) = (s.path.clone(), s.bgm, s.looped, s.fade_ms, s.volume);
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
                if let Some(after_ms) = self.process_clock {
                    // Started by a music process, on its clock.
                    self.ops.push(Op::BgmLater { file: file.clone(), after_ms, looped });
                } else {
                    if !looped {
                        self.note("BGM without loop played looping");
                    }
                    if percent(volume) < 100 {
                        self.ops.push(Op::SoundVolume { file: None, percent: percent(volume), ms: 0 });
                    }
                    self.ops.push(Op::BgmPlay { file: file.clone(), fade_in_ms: fade_ms });
                }
            } else {
                if fade_ms > 0 {
                    self.ops.push(Op::SeFadeIn { ms: fade_ms });
                }
                if percent(volume) < 100 {
                    self.ops.push(Op::SoundVolume { file: Some(file.clone()), percent: percent(volume), ms: 0 });
                }
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
            // A `<center>` the line doesn't close ends with it.
            let n = page.text.chars().count();
            for s in page.styles.iter_mut().filter(|s| s.centred && s.start + s.len == usize::MAX) {
                s.len = n - s.start;
            }
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
                        ("i", false) | ("center", false) => {
                            let centred = tag.name.eq_ignore_ascii_case("center");
                            let start = page_len(&page, &line);
                            page.styles.push(TextStyle { start, len: usize::MAX - start, italic: !centred, centred, ..Default::default() });
                        }
                        ("i", true) | ("center", true) => {
                            let centred = tag.name.eq_ignore_ascii_case("center");
                            let end = page_len(&page, &line);
                            if let Some(s) = page.styles.iter_mut().rev().find(|s| s.centred == centred && s.start + s.len == usize::MAX) {
                                s.len = end - s.start;
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
                styles: page.styles.clone(),
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
        let vertical = t.rotation.rem_euclid(360) == 90;
        // Across: as many lines as end inside the box (box02: 130 px at a
        // 27 px pitch holds five 18 px lines, the last ending at 126).
        let rows = if vertical { h / line.max(1) } else { (h - font) / line.max(1) + 1 }.max(1) as u32;
        let (mut text_x, mut text_y) = if vertical { (x - rows as i64 * line, y) } else { (x, y + self.game.text_drop) };
        let (mut columns, mut rows, mut line, mut vertical) = (columns, rows, line, vertical);
        if let (true, Some((w, h, pitch))) = (vertical, self.game.horizontal_box) {
            (columns, rows, line, vertical) = ((w / font.max(1)).max(1) as u32, ((h - font) / pitch.max(1) + 1).max(1) as u32, pitch, false);
        }
        // The window's pictures (box, face window, name plate), bottom
        // first, drawn into one window image.
        // (priority, creation order, (picture, x, y, opacity))
        type Part = (i64, u64, (String, i64, i64, i64));
        let mut parts: Vec<Part> = self
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
        let shown = |name: &str| self.objects.get(name).is_some_and(|o| o.hideable && o.opacity > 0);
        if let Some(&(_, x, y, w)) = self.game.box_text.iter().find(|b| shown(b.0)) {
            (text_x, text_y, columns) = (x, y, (w / font.max(1)).max(1) as u32);
        }
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

/// Whether `name` fits `pattern`, whose `*`s stand for any characters.
fn glob(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    let Some(mut rest) = name.strip_prefix(first) else { return false };
    let parts: Vec<&str> = parts.collect();
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.is_empty()
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

/// An NSS volume (0-1000) as a percent of the player's setting: the exe
/// sends 3322·log10(volume/1000) hundredths of a dB, an amplitude of
/// (volume/1000)^1.661 (`research/nitroplus/builtins.md`, `SetVolume`).
fn percent(volume: i64) -> u32 {
    (100.0 * (volume.clamp(0, 1000) as f64 / 1000.0).powf(1.661)).round() as u32
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
        fn mirrored(&mut self, image: &str, x: bool, y: bool) -> Option<String> {
            Some(format!("{image}~{}{}", if x { "x" } else { "" }, if y { "y" } else { "" }))
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
        fn rgba(&mut self, _path: &str) -> Option<image::RgbaImage> {
            None
        }
        fn frames_movie(&mut self, name: &str, _fps: u32, _frames: u32, _draw: &mut dyn FnMut(u32, &mut image::RgbImage)) -> Option<String> {
            Some(format!("movie/{name}.mp4"))
        }
        fn frames_strip(&mut self, name: &str, _w: u32, _h: u32, _frames: u32, _draw: &mut dyn FnMut(u32, &mut image::RgbaImage)) -> Option<String> {
            Some(format!("system/strip/{name}.png"))
        }
        fn delayed_sound(&mut self, path: &str, delay_ms: u32) -> Option<String> {
            Some(format!("{path}+{delay_ms}.ogg"))
        }
        fn sound_ms(&mut self, _path: &str) -> Option<u32> {
            Some(4000)
        }
        fn overlaid(&mut self, base: &str, top: &str, zoom: i64, opacity: i64) -> Option<String> {
            Some(format!("{base}+{top}@{zoom}x{opacity}"))
        }
        fn movie_frames(&mut self, path: &str, at: Option<u32>, alpha: bool, dark: bool) -> Option<(String, u32, u32, u32)> {
            let (w, h) = if alpha { (200, 108) } else { (1024, 576) };
            let cells = if at.is_some() { 1 } else { 30 };
            Some((format!("{path}@{at:?}{}", if dark { "d" } else { "" }), cells, w, h))
        }
        fn movie_sprite(&mut self, path: &str, alpha: bool, dark: bool) -> Option<(String, u32, u32)> {
            let (w, h) = if alpha { (400, 216) } else { (1024, 576) };
            Some((format!("{path}{}{}.mp4", if dark { "_dark" } else { "" }, if alpha { "_alpha" } else { "" }), w, h))
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
    fn the_english_port_writes_vertical_boxes_across_in_its_own_window() {
        let story = r#"
scene a.nss
{
	CreateTexture("boxCC", 20000, Center, 40, "cg/sys/adv/TextWindow.png");
	SetAlias("boxCC", "boxCC");
	Request("boxCC", Hideable);
	Move("boxCC", 0, @+2, @0, null, true);
	Fade("boxCC", 0, 1000, null, true);
<PRE @box0>
[text0010]
　一行目
</PRE>
	SetText();
	TypeBegin();
}
"#;
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse(LIBRARY).unwrap());
        files.insert("a.nss".to_string(), nss::parse(story).unwrap());
        let w = compile_scene(&files, "a.nss", "a.nss", &MURAMASA_EN, &mut Fake).unwrap().window.unwrap();
        assert!(!w.vertical);
        assert_eq!(w.image, "cg/sys/adv/en/textwindow.png@1000");
        assert_eq!((w.x, w.y, w.text_x, w.text_y, w.columns, w.rows), (360, 51, 375, 190, 14, 14));
    }

    #[test]
    fn the_english_port_puts_horizontal_text_where_its_box_info_says() {
        let story = r#"
scene a.nss
{
	CreateTexture("boxTH", 20000, 224, InBottom, "cg/sys/adv/THTextWindow_Fw.png");
	Request("boxTH", Hideable);
	Move("boxTH", 0, @0, @+4, null, true);
	Fade("boxTH", 0, 1000, null, true);
<PRE @box0>
[text0010]
　一行目
</PRE>
	SetText();
	TypeBegin();
}
"#;
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse(LIBRARY).unwrap());
        files.insert("a.nss".to_string(), nss::parse(story).unwrap());
        let w = compile_scene(&files, "a.nss", "a.nss", &MURAMASA_EN, &mut Fake).unwrap().window.unwrap();
        assert!(!w.vertical);
        assert_eq!(w.image, "cg/sys/adv/en/thtextwindow_fw.png@1000");
        assert_eq!((w.text_x, w.text_y), (238, 426));
    }

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
    fn a_smoke_loop_plays_as_a_movie_zoomed_and_a_paused_cut_as_one_frame() {
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
        // The movie at its own size (400x216), zoomed 3x.
        let smoke = c.ops.iter().any(|op| matches!(op, Op::AnimatedSprite { cells: 1, frame_ms: 0, looped: true, zoom: (300, 500), sprite, .. } if sprite.image.ends_with("_alpha.mp4")));
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
    fn a_picture_turned_about_y_shows_mirrored() {
        let story = r#"
scene a.nss
{
	CreateTexture("st", 100, 0, 0, "cg/st/a.png");
	Rotate("st", 0, @0, @180, @0, null, true);
	Fade("st", 0, 1000, null, true);
	Wait(100);
	Rotate("st", 0, @0, @180, @0, null, true);
	Wait(100);
}
"#;
        let c = compile(story, LIBRARY);
        let images: Vec<&str> = c.ops.iter().flat_map(|op| match op {
            Op::Sprites { changes, .. } => changes.iter().filter_map(|c| c.sprite.as_ref().map(|s| s.image.as_str())).collect(),
            _ => Vec::new(),
        }).collect();
        // Turned once: mirrored; twice: back as it was.
        assert_eq!(images.first(), Some(&"cg/st/a.png~x"), "{:?}", c.ops);
        assert_eq!(images.last(), Some(&"cg/st/a.png"), "{:?}", c.ops);
        // A half turn in its plane is both mirrors; with x 180 as well, left
        // to right only.
        for (turn, image) in [("@0, @0, @180", "cg/st/a.png~xy"), ("180, 0, 180", "cg/st/a.png~x")] {
            let story = format!("scene a.nss\n{{\n\tCreateTexture(\"st\", 100, 0, 0, \"cg/st/a.png\");\n\tRotate(\"st\", 0, {turn}, null, true);\n\tFade(\"st\", 0, 1000, null, true);\n\tWait(100);\n}}\n");
            let c = compile(&story, LIBRARY);
            let shown = c.ops.iter().any(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == image))));
            assert!(shown, "{turn}: {:?}", c.ops);
        }
    }

    #[test]
    fn moves_and_zooms_over_time_are_engine_motions_during_the_other_fades() {
        let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	CreateColor("white", 200, 0, 0, 1024, 576, "White");
	Fade("white", 0, 1000, null, true);
	Wait(100);
	Move("bg", 1500, @0, @-150, null, false);
	Fade("white", 300, 0, null, true);
	Zoom("bg", 1000, 2000, 2000, null, true);
	Wait(100);
}
"#;
        let c = compile(story, LIBRARY);
        let motions: Vec<_> = c.ops.iter().filter_map(|op| match op {
            Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some((keys[1].ms, keys[1].x, keys[1].y, keys[1].zoom)),
            _ => None,
        }).collect();
        // The pan goes on through the white's 300 ms fade, which takes only
        // its own time; the zoom is about the centre.
        assert_eq!(motions, [(1500, 0, -150, 100), (1000, 0, 0, 200)], "{:?}", c.ops);
        let pan = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap();
        assert!(matches!(c.ops[pan + 1], Op::BeginTransition { fade_ms: 300, .. }), "{:?}", c.ops);
        assert!(c.ops.iter().any(|op| matches!(op, Op::Wait { ms: 1000 })), "{:?}", c.ops);
    }

    #[test]
    fn a_picture_under_an_opaque_full_screen_one_isnt_drawn() {
        let story = r#"
scene a.nss
{
	CreateTexture("old", 100, 0, 0, "cg/bg/a.jpg");
	Fade("old", 0, 1000, null, true);
	Wait(100);
	CreateTexture("new", 200, 0, 0, "cg/bg/b.jpg");
	Fade("new", 500, 1000, null, true);
	Wait(100);
	Fade("new", 500, 0, null, true);
	Wait(100);
}
"#;
        let c = compile(story, LIBRARY);
        // Shown, covered (removed once the cover is up), shown again.
        let old: Vec<_> = c.ops.iter().flat_map(|op| match op {
            Op::Sprites { changes, .. } => changes.iter().filter(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/bg/a.jpg")).collect(),
            _ => Vec::new(),
        }).collect();
        assert_eq!(old.len(), 2, "{:?}", c.ops);
        let removals = c.ops.iter().filter(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.is_none()))).count();
        assert_eq!(removals, 2, "{:?}", c.ops);
    }

    #[test]
    fn a_branch_that_left_the_chapter_early_goes_on_from_where_it_left() {
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse(LIBRARY).unwrap());
        // As 続・殺戮のジャンゴ's deaths: the chapter returns right after the
        // deadly answer, the caller then plays the bad end.
        let a = r#"scene a.nss
{
	call_chapter nss/b.nss;
	if($x == 1)
	{
		CreateTexture("bad", 200, 0, 0, "cg/bad.png");
		Fade("bad", 0, 1000, null, true);
		Wait(100);
		return;
	}
}"#;
        let b = r#"chapter main
{
	CreateTexture("one", 100, 0, 0, "cg/one.png");
	Fade("one", 0, 1000, null, true);
	select { case @選択肢１ { $x = 1; } case @選択肢２ { $x = 0; } }
	if($x == 1)
	{
		return;
	}
	CreateTexture("two", 300, 0, 0, "cg/two.jpg");
	Fade("two", 0, 1000, null, true);
	Wait(100);
}"#;
        files.insert("a.nss".into(), nss::parse(a).unwrap());
        files.insert("b.nss".into(), nss::parse(b).unwrap());
        let c = compile_story(&files, "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap();
        // The bad end is drawn over "one", not under the other answer's
        // "two", which it never saw.
        let bad = c.ops.iter().position(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/bad.png"))));
        assert!(bad.is_some(), "{:?}", c.ops);
        let two_shown = c.ops[..bad.unwrap()].iter().rposition(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/two.jpg"))));
        let label = c.ops[..bad.unwrap()].iter().rposition(|op| matches!(op, Op::Label(_)));
        assert!(two_shown < label, "{:?}", c.ops);
    }

    #[test]
    fn a_music_process_plays_its_sounds_on_its_clock() {
        // 続・殺戮のジャンゴ's Play_DJ24x and PlayOWA: an intro, then its loop
        // a set time later; a jingle's second part when the first ends.
        let story = r#"
scene a.nss
{
	CreateSound("intro", BGM, "sound/bgm/intro");
	CreateSound("loop", BGM, "sound/bgm/loop");
	CreateProcess("P", 0, 0, 0, "Music");
	Request("P", Start);
	Wait(100);
}
function Music()
{
	begin:
	SoundPlay("intro", 0, 1000, false);
	WaitPlay("intro", null);
	Wait(500);
	SoundPlay("loop", 0, 1000, true);
}
function SoundPlay("ナット名", 秒数, ボリウム, ループ設定)
{
	Request("ナット名", Play);
	SetLoop("ナット名", ループ設定);
	SetVolume("ナット名", 秒数, ボリウム, null);
}
"#;
        let c = compile(story, LIBRARY);
        let later: Vec<_> = c.ops.iter().filter_map(|op| match op {
            Op::BgmLater { file, after_ms, looped } => Some((file.as_str(), *after_ms, *looped)),
            _ => None,
        }).collect();
        // (Fake's sounds last 4 s.)
        assert_eq!(later, [("sound/bgm/intro.ogg", 0, false), ("sound/bgm/loop.ogg", 4500, true)], "{:?}", c.ops);
        assert!(!c.notes.keys().any(|n| n.starts_with("CreateProcess")));
    }

    #[test]
    fn delete_all_in_a_chapter_keeps_what_was_made_before_it() {
        static GAME: Game = Game { chapter_objects: true, ..Game::PLAIN };
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse(LIBRARY).unwrap());
        let a = r#"scene a.nss
{
	CreateSound("bgm1", BGM, "sound/bgm/one");
	call_chapter nss/b.nss;
	Request("bgm1", Play);
	SetLoop("bgm1", true);
	Wait(100);
}"#;
        let b = r#"chapter main
{
	CreateTexture("pic", 100, 0, 0, "cg/one.png");
	Fade("pic", 0, 1000, null, true);
	Wait(100);
	Delete("*");
	Wait(100);
}"#;
        files.insert("a.nss".into(), nss::parse(a).unwrap());
        files.insert("b.nss".into(), nss::parse(b).unwrap());
        let c = compile_story(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
        assert!(c.ops.iter().any(|op| matches!(op, Op::BgmPlay { file, .. } if file == "sound/bgm/one.ogg")), "{:?}", c.ops);
        let removed = c.ops.iter().any(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.is_none())));
        assert!(removed, "{:?}", c.ops);
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
    fn sound_volumes_follow_the_exe_curve() {
        let story = r#"
scene a.nss
{
	CreateSound("SE01", SE, "sound/se/rain_L");
	SoundPlay("SE01", 1000, 500, true);
	Wait(100);
	SetVolume("SE01", 2000, 1, null);
	Wait(100);
	SetVolume("SE01", 500, 0, null);
	Wait(100);
}
"#;
        // function.nss's own.
        let library = r#"
function SoundPlay($ナット名,秒数,ボリウム,ループ設定)
{
	SetVolume($ナット名, 0, 1, null);
	Request($ナット名, Play);
	SetVolume($ナット名, 秒数, ボリウム, null);
	SetLoop($ナット名, ループ設定);
	Request($ナット名, Disused);
}
"#;
        let c = compile(story, library);
        let file = Some("sound/se/rain_L.ogg".to_string());
        let sound: Vec<_> = c.ops.iter().filter(|op| matches!(op, Op::SeFadeIn { .. } | Op::SoundVolume { .. } | Op::SePlay { .. } | Op::SeFadeOut { .. })).collect();
        assert_eq!(
            sound,
            [
                &Op::SeFadeIn { ms: 1000 },
                &Op::SoundVolume { file: file.clone(), percent: 32, ms: 0 },
                &Op::SePlay { file: "sound/se/rain_L.ogg".into(), looped: true },
                &Op::SoundVolume { file: file.clone(), percent: 0, ms: 2000 },
                &Op::SeFadeOut { file, ms: 500 },
            ]
        );
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
    fn a_replay_compiles_a_file_again_with_its_variables_set() {
        const GAME: Game = Game {
            replays: |_| vec![Replay { label: "conv_r".into(), file: "r.nss".into(), set: vec![("$PLACE_reco".into(), 1)] }],
            ..Game::PLAIN
        };
        let mut files = BTreeMap::new();
        files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
        files.insert("function.nss".to_string(), nss::parse("function F(){}").unwrap());
        files.insert("a.nss".into(), nss::parse("scene a.nss_MAIN\n{\n\tif($f){$g = 1;}\n\t$GameCircle = 0;\n}").unwrap());
        files.insert("r.nss".into(), nss::parse("scene r.nss_MAIN\n{\n\tif($PLACE_reco){$f = 7;}else{$f = 3;}\n\t$GameCircle = 0;\n}").unwrap());
        let (c, _) = compile_run(&files, "a.nss", 10, &GAME, &mut Fake).unwrap();
        let f = story_flags(&files, &GAME)["$f"];
        let at = c.ops.iter().position(|op| *op == Op::Label("conv_r".into())).expect("the replay's label");
        let sets: Vec<_> = c.ops[at..].iter().filter_map(|op| match op {
            Op::Arith { var, op: ArithOp::Set, value: Operand::Const(n) } if *var == f => Some(*n),
            _ => None,
        }).collect();
        assert_eq!(sets.first(), Some(&7), "{:?}", &c.ops[at..]);
        assert!(c.ops[at..].iter().any(|op| matches!(op, Op::Return)));
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
    fn glob_matches_stars_anywhere() {
        assert!(glob("選択肢*/MouseUsual/*", "選択肢１/MouseUsual/選択肢１板１"));
        assert!(!glob("選択肢*/MouseUsual/*", "選択肢１/MouseOver/選択肢１板２"));
        assert!(glob("a*", "a/b"));
        assert!(glob("*b", "ab") && !glob("*b", "ba"));
        assert!(!glob("a*c", "ab"));
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
