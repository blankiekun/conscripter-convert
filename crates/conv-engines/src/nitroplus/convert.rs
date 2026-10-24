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
//!   scene. While a cover is opaque nothing under it changes on screen
//!   (what lies above it does), so the cover itself is never drawn: its
//!   fade-out is the cross-fade.
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

use std::collections::{BTreeMap, HashMap, HashSet};

use conv_core::ir::{ArithOp, SpriteFilter, BLEND_ADD, BLEND_MULTIPLY, BLEND_NORMAL, BLEND_SUBTRACT, ChoiceStyle, SCREEN_UNDER, CLICK_WAIT_MARK, MotionEnd, MotionKey, Op, Operand, ScreenEffect, Ruby, Script, TextStyle, SpriteChange, SpritePlacement, TextWindow, TransitionStyle, Voice};

use super::nss::{AssignOp, BinaryOp, Expr, Function, Inline, Item, NssFile, Param, Stmt, StmtKind, Target, Text, UnaryOp};

/// 装甲悪鬼村正's screen ([`Game::screen`]).
// src: sys_config.nss ConfigSet (CreateWindow 1024×576)
pub const SCREEN_W: i64 = 1024;
/// The longest cross-fade non-blocking actions get where the script doesn't
/// wait for them (a text page, a blocking action): the original goes on
/// while they run, up to minutes (a background scrolling through a
/// battle).
// ours: a cap so an unwaited fade does not stall our blocking engine command
// (the exe runs it in the background)
const UNWAITED_MS: u32 = 1000;
pub const SCREEN_H: i64 = 576;
/// 装甲悪鬼村正's story driver ([`Game::boot`]); the helper library is what
/// it `#include`s.
const BOOT: &str = "0_boot_開始スクリプト.nss";
/// Highest IR sprite layer used.
const MAX_LAYER: u32 = 900;
// The game's sprites (62 + layer) stay under the menus' extra slots.
const _: () = assert!(62 + MAX_LAYER < conv_core::alloc::MENU_MORE_SPRITES.start);
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
/// Story flags read only by a choice ([`choice_flags`]): the last locals
/// below the global border, past the helpers made runtime ones.
const CHOICE_FLAGS: std::ops::Range<u16> = (GLOBAL_BORDER - 10 - 1 - 16) as u16..(GLOBAL_BORDER - 10 - 1) as u16;
/// Codes of the strings first held by a choice's flags.
const CHOICE_CODES: i32 = 1_000_000;
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
pub use builtin::channel;
pub mod probe;
mod sync;
mod muramasa;
pub use muramasa::{MURAMASA, MURAMASA_EN};
mod django;
pub use django::DJANGO;
mod process;
mod loops;
mod one_shot;
mod array;
mod fast_skip;
mod text_object;
mod sounds;
mod shake;
mod rain;
mod turn;
mod random;
mod story;
pub use story::{compile_run, compile_scene, compile_story, library, story_flags, BOOT_LABEL, OPENING_BACK_LABEL, OPENING_LABEL};
use story::*;
mod value;
use value::*;
mod branches;
mod objects;
mod drawing;
mod text;
use text::*;
pub(super) use fast_skip::read_flag;

/// A window picture's text place ([`Game::box_text`]): picture, text x, y,
/// width, the click-wait icon's x, y.
pub type BoxText = (&'static str, i64, i64, i64, (i32, i32));

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
    /// Off for 装甲悪鬼村正, whose children of cameras weren't checked
    /// against it (a window's follow `CreateWindow`'s last argument in
    /// both games).
    relative_children: bool,
    /// `CreateTexture` from a `LoadImage` picture draws it (続・殺戮のジャンゴ's
    /// face-window frames, 装甲悪鬼村正's pictures made from them).
    loaded_textures: bool,
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
    /// ruby space above it): 続・殺戮のジャンゴ's baseline sits 8 + 15 px
    /// below its box (ruby space and ＭＳ ゴシック's ascent, from the
    /// exe), CONscripter's 16 below the text's top with ipag. 0 for
    /// 装甲悪鬼村正, whose text is vertical; 2 for its English port, whose
    /// lines start 2 px lower than CONscripter's from the same place
    /// ([`Game::box_text`] too; measured in the original).
    text_drop: i64,
    /// Children follow their parents: `@移動窓/立絵40` names a child of
    /// the object `移動窓` is an alias of (`窓/移動窓`), and deleting an
    /// object deletes its children (続・殺戮のジャンゴ: pictures under a
    /// deleted window stayed up). Off for 装甲悪鬼村正 until checked: it
    /// changes 621 places of its script.
    family: bool,
    /// A variable's name in quotes added to a string means its value
    /// (装甲悪鬼村正's `CreateSEEX`: `"sound/se/" + "$音楽データ"`;
    /// 続・殺戮のジャンゴ's face windows, `"@frame03" + "/" + "$顔ナット名"`,
    /// one object per face as under wine).
    string_vars: bool,
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
    /// port, whose `text0` is 280×150 (`function_textbox.nut`). The pitch
    /// is also that of the [`Game::box_text`] boxes that were across.
    horizontal_box: Option<(i64, i64, i64)>,
    /// Where the text starts and how wide it is while the named window
    /// picture is shown: the first of these shown (Full Metal Daemon
    /// Muramasa's English port, `BoxInfo` in `val.nut`): picture, text x,
    /// y, width, and the click-wait icon's x, y (`icon_pos`). In pairs
    /// (a box with a face window, then the box without one) when the
    /// game's text has `<FW img=''>`.
    box_text: &'static [BoxText],
    /// How `Fade` and `DrawTransition` ease by their tempo
    /// ([`Op::TransitionEase`]): 装甲悪鬼村正's and 続・殺戮のジャンゴ's exes
    /// as [`tempo`]; `None`, straight.
    fade_tempo: Option<Tempo>,
    /// Full Metal Daemon Muramasa's English port's
    /// `Shake` (a Diesel `CEffectShake`), turning every (1000 ÷ `n`) × 20
    /// ms, the size straight (its tempo is the swings' own).
    diesel_shake: bool,
    /// Full Metal Daemon Muramasa's English port's `DrawTransition` edge
    /// (Mware's `psUpdateTransitionRule`, `spfadeto`'s form 1).
    diesel_transition: bool,
    /// A script file and the NSS source of a function played once each
    /// launch before the first title, as if in that file (its functions
    /// callable): the game's logos and warnings ([`compile_boot`]). An
    /// empty source: none.
    boot_logos: (&'static str, &'static str),
    /// A function of [`Game::boot_logos`]' file that draws the first title
    /// of a run coming up, run after the logos as the boot script does:
    /// compiled at [`OPENING_LABEL`] for the title's
    /// [`conv_core::ir::TitleOpening::script`]; and a flag it reads, then
    /// compiled once with it 0 (there) and once with it 1 (at
    /// [`OPENING_LABEL`] `1`), so that no runtime `if` on it has to be
    /// merged (TitleSet's `#ClearG` buttons). Empty: none. The third, a
    /// variable that is 1 on coming back to the title (TitleSet's `$Logo`):
    /// both compiled again with it 1, at [`OPENING_BACK_LABEL`] (and `1`).
    title_opening: (&'static str, &'static str, &'static str),
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
        chapter_objects: false,
        text_drop: 0,
        family: false,
        string_vars: false,
        choice_style: |c, n| c.vertical_choice_style(n),
        story_end: &[],
        horizontal_box: None,
        box_text: &[],
        diesel_shake: false,
        diesel_transition: false,
        fade_tempo: None,
        boot_logos: ("", ""),
        title_opening: ("", "", ""),
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
    /// An output image (or `.mp4` movie) mirrored left to right (`x`)
    /// and/or upside down (`y`).
    fn mirrored(&mut self, image: &str, x: bool, y: bool) -> Option<String>;
    /// Images the scripts name, as cells side by side in one picture (a
    /// button's states): its path and size (all cells). Each cell is as
    /// large as the largest part, the part at its top left; a part `""` is
    /// an empty cell.
    fn strip(&mut self, parts: &[&str]) -> Option<(String, u32, u32)>;
    /// A movie the scripts name (`dx/mvOP.ngs`), or `None` if the game has
    /// no such file.
    fn movie_info(&mut self, path: &str) -> Option<MovieInfo>;
    /// Whether this build has the movie folder (the dry run may not).
    fn has_movies(&self) -> bool {
        true
    }
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
    /// The output picture `image` with its opacity multiplied by that of
    /// the output picture `mask`, 0 outside it (`CreateStencil`); with
    /// `colour`, its colour by the mask's too, and 0 where the mask is
    /// clear (`CreateMask`): the
    /// mask's top left at `at` from the picture's, on the screen, where
    /// the picture is drawn at `zoom` (per mille) about its centre. Its
    /// path.
    fn stenciled(&mut self, image: &str, mask: &str, colour: bool, at: (i64, i64), zoom: (i64, i64)) -> Option<String>;
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
    /// A sound's length (`sound/bgm/x`, `voice/ma04/x`), in a dry run too.
    fn sound_ms(&mut self, path: &str) -> Option<u32>;
    /// The English release's text font (`font.npk`), when it is given.
    fn english_font(&mut self) -> Option<Vec<u8>> {
        None
    }
    /// The font the game is built with (`--font`), for `CreateText`.
    fn font(&self) -> &[u8] {
        &[]
    }
    /// A copy of the sound that loops from `start_ms` to `end_ms`
    /// (`SetLoopPoint`; `LOOPSTART`/`LOOPEND` tags): its output path.
    fn looped_sound(&mut self, path: &str, _start_ms: u32, _end_ms: u32) -> Option<String> {
        self.sound(path)
    }
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
// ours: cells are a size/smoothness trade-off for our strips (the exe
// plays the movie file)
pub const MOVIE_FPS: u32 = 15;

/// The result of compiling one scene.
#[derive(Debug, Default)]
pub struct Compiled {
    pub ops: Vec<Op>,
    /// The text window of the first page (the script's initial window).
    pub window: Option<TextWindow>,
    /// What was left out or approximated, with counts.
    pub notes: BTreeMap<String, usize>,
    /// The built-in calls, converted fully or with gaps.
    pub metrics: crate::metrics::Metrics,
    /// The story flags and their variables.
    pub flags: HashMap<String, conv_core::ir::Var>,
    /// The voice classes, in the order of their character numbers
    /// ([`Voice::character`]).
    pub voice_classes: Vec<String>,
    /// Where the ops came from: the statement (file, line) each run of ops
    /// was lowered from ([`Script::source_map`]).
    pub source_map: Vec<(usize, conv_core::ir::SourceLoc)>,
}

/// What the system screens may need from the compiled story.
pub struct Story<'a> {
    pub flags: &'a HashMap<String, conv_core::ir::Var>,
    pub voice_classes: &'a [String],
    /// The game's scripts (the system screens' own, `sys_*` and `extra_*`,
    /// hold their lists).
    pub files: &'a BTreeMap<String, NssFile>,
    /// A build of both editions (`--english`): the settings get the
    /// language and pictures rows.
    pub both_editions: bool,
}

/// A story file played on its own from a gosub to `label`, its
/// `X.nss_MAIN` scene with `set` variables at convert time (`$PLACE_reco`
/// = 1) and `$GameName` its name ([`Game::replays`]).
pub struct Replay {
    pub label: String,
    pub file: String,
    pub set: Vec<(String, i64)>,
}





#[derive(Debug, Clone)]
enum Kind<'a> {
    /// A texture or a colour: `image` is the output picture.
    Image { image: String },
    /// `CreateEffect("Plain")`: the screen frozen as it was. `froze`: the
    /// pictures it froze, top first; `again`: it has been faded out to 0
    /// and the screen under it may have changed, so while shown it draws
    /// `froze` (装甲悪鬼村正's ma00_000 shows each fighter's cover again).
    Cover { froze: Vec<(String, Shown)>, again: bool },
    /// `CreateEffect("Sepia")` and the like: the pictures below its
    /// priority drawn through the filters (strength: its opacity).
    Tint(Vec<SpriteFilter>),
    Text(TextObject<'a>),
    Sound(SoundObject),
    Movie(MovieObject),
    /// Names, windows and other objects that draw nothing themselves.
    Other,
    /// `CreateProcess` of a function that only plays music on a timer or
    /// only runs timed actions ([`Compiler::start_process`]).
    Process { file: &'a str, function: &'a Function },
    /// Pictures drawn by a game's own code ([`Assets::frames_strip`]),
    /// shown once in turn while the script goes on, the last one staying.
    Strip(StripObject),
    /// `CreateEffect` Rain or Drop: a live particle layer of its own
    /// ([`Op::ParticleLayer`]) drawn as a `w`×`h` picture at its place.
    Particles(ParticleObject),
    /// `CreateCamera`: what its children show. Its place is the point the
    /// screen looks at (they are drawn that much the other way); its zoom
    /// `z` and a child's priority `p` (its depth) magnify the child
    /// `z / (1000 - p)` times about the screen's centre; one of priority
    /// 1000 or more isn't drawn. Measured under wine (続・殺戮のジャンゴ,
    /// boxes at four priorities and six camera settings); 装甲悪鬼村正's
    /// exe projects the same ([`drawing::through_camera`]'s test).
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
    /// Its cells repeat (else it stops on the last).
    looped: bool,
}

/// A [`Kind::Particles`]: layer `layer` (2 to 31, one a live object)
/// running `particles`.
#[derive(Debug, Clone)]
struct ParticleObject {
    layer: u32,
    particles: conv_core::ir::Particles,
    w: i64,
    h: i64,
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
    /// `SetBlur(obj, true, passes, alpha, lag, wait)`: not a blur but a
    /// trail. The exe (`FUN_00460230`, `FUN_00460530`, `FUN_00456810`)
    /// draws `passes` (1-128) copies under the picture at its opacity ×
    /// `alpha`/1000 (1-1000), copy i showing where the picture was i ×
    /// `lag` ms before on a move, zoom or turn. A still picture's copies
    /// lie under it and don't show: drawn by the engine
    /// ([`Op::SpriteTrail`]) behind its motions.
    trail: Option<Trail>,
    /// `Request` `AddRender`/`MulRender`/`SubRender`: one of the IR's
    /// `BLEND_*` ([`Op::SpriteBlend`]).
    blend: u8,
    /// `SetVertex`: the point (from the top left) a zoom or a turn keeps
    /// in place; `None` = the centre.
    vertex: Option<(i64, i64)>,
    /// `Rotate`: degrees about the x, y and z axes. A picture turned past
    /// 90° about y shows mirrored left to right, about x upside down; edge-on
    /// (90°, 270°) it isn't drawn, nor are its children
    /// ([`Compiler::drawn_opacity`]). About z (clockwise, as `Rotate(text,
    /// 90)` turns a line top to bottom) the engine turns it ([`Op::SpriteAngle`]).
    turn: (i64, i64, i64),
    /// A motion the engine loops while it is shown, and the loop process
    /// running it ([`loops`]: 続・殺戮のジャンゴ's `ShakeVLoop`, 装甲悪鬼村正's
    /// `FlyMoving`).
    shake: Option<ShakeLoop>,
    /// The start and length of the first fade in [`Object::shake`]'s
    /// lead-in (key times), for a fade at once before it is drawn.
    loop_fade: Option<(u32, u32)>,
    /// `CreateWindow`'s size (w, h): its children are drawn only inside
    /// it, at its place.
    clip: Option<(i64, i64)>,
    /// A [`Kind::Strip`] drawn over the text window ([`Op::SpriteOverText`]),
    /// as the English port draws its subtitles. Only strips: a layer a
    /// strip leaves is cleared, which ends it.
    over_text: bool,
    /// Made by `CreateWindow`, `CreateName` or `CreateCamera`: a wildcard
    /// that matches it doesn't reach its children (the exe's object flag
    /// at +0x9d4, 0 for these, 1 for pictures; [`Compiler::matching`]).
    container: bool,
    /// A `CreateText` picture taken away by `Request(…, Erase)` until its
    /// `Request(…, Enter)` (続・殺戮のジャンゴ's song, dja026).
    erased: bool,
    /// `SetTone`: the exe rewrites the object's pixels in that tone
    /// (`FUN_00452000`); drawn through these filters ([`Op::SpriteFilters`]).
    /// On a `Plain` cover, the pictures under it are.
    tone: Option<Vec<SpriteFilter>>,
    /// `SetShade`: how many passes of the exe's blur (`FUN_00465b10`) its
    /// pixels went through; they add up until `NULL` takes them away.
    shade: Option<i64>,
    /// `DrawEffect`'s wave ([`Wave`]), while it runs.
    wave: Option<Wave>,
    /// `CreateStencil`'s picture (output path): its children are drawn
    /// only where it is opaque, as much as it is ([`Compiler::stenciled`]);
    /// `true` for `CreateMask`'s, which multiplies their colour too.
    stencil: Option<(String, bool)>,
    /// A picture under stencils: its picture, place and zoom and their
    /// pictures and places, and the cut made of them, kept while these
    /// stay the same.
    cut: Option<(String, String)>,
    /// A container's children's places count from the screen, not from it
    /// (`CreateWindow`'s last argument not `true`; a name without
    /// [`Game::relative_children`]).
    screen_children: bool,
    /// Its last `DrawTransition` (the engine's `spfadeto`), which it is
    /// drawn through from then on.
    fade: Option<FadeTo>,
}

/// `DrawEffect(obj, ms, wave, from, to, tempo)`: the exe's wave
/// (`FUN_00430a70`, `FUN_00430de0`), drawn by the engine ([`Op::SpriteWave`]).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Wave {
    /// The sound clock ([`Compiler::sound_clock`]) it started at.
    start: u64,
    ms: u32,
    /// The rows' phase over the picture's height, in 1/256 turns.
    period: u32,
    from: i32,
    to: i32,
    ease: Option<(u8, u32)>,
}

impl Wave {
    /// Its op on `layer` at sound clock `now` for `shown` (a picture cut to
    /// the screen has the phase of its part's height); `None` once over
    /// (ramped to 0: the exe's `cb_00431230` removes it, else it stays).
    fn op(&self, layer: u32, now: u64, shown: &Shown) -> Option<Op> {
        let gone = now.saturating_sub(self.start).min(u64::from(self.ms)) as u32;
        let ms = self.ms - gone;
        if ms == 0 && self.to == 0 {
            return None;
        }
        // shortcut: past its start the rest goes straight from where it got to.
        let from = self.from + ((i64::from(self.to - self.from) * i64::from(gone)) / i64::from(self.ms.max(1))) as i32;
        let period = match shown.whole {
            Some((_, _, _, h)) if h > 0 => (i64::from(self.period) * shown.h / h) as u32,
            _ => self.period,
        };
        Some(Op::SpriteWave { layer, ms, period, from, to: self.to, ease: if gone == 0 { self.ease } else { None } })
    }
}

/// [`Object::shake`]: the process, the keys.
type ShakeLoop = (String, Vec<MotionKey>, usize);

impl Object<'_> {
    /// Mirrored (left to right, upside down) as its turn shows it.
    fn mirror(&self) -> (bool, bool) {
        let back = |deg: i64| (91..270).contains(&deg.rem_euclid(360));
        (back(self.turn.1), back(self.turn.0))
    }

    /// Its `SetTone` and `SetShade` filters, for its image drawn at
    /// `scale` (per mille, x and y) of the pixels the blur is counted in.
    fn filters(&self, scale: (i64, i64)) -> Vec<SpriteFilter> {
        let mut fs = self.tone.clone().unwrap_or_default();
        fs.extend(self.shade.map(|n| builtin::shade_blur(n, scale)).unwrap_or_default());
        fs
    }

    /// Its turn about z, counterclockwise (spzoom's angle).
    fn angle(&self) -> i64 {
        -self.turn.2
    }
}

#[derive(Debug, Clone)]
struct TextObject<'a> {
    block: &'a Text,
    /// The `<PRE>` statement (file, line): its pages' source.
    at: Option<(&'a str, u32)>,
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
    /// After the English port's `<FW img=''>` (`SwitchFw`): shown in the
    /// window's box without a face, the face and name plate gone.
    face_off: bool,
    /// Goes on in the page before, which stopped for `code` (code inside
    /// a page: the text before it shows first).
    goes_on: bool,
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
    /// `SetLoopPoint`: looped, it loops from the first to the second ms.
    loop_ms: Option<(u32, u32)>,
    /// When it started ([`Compiler::sound_clock`]) and its length, a
    /// one-shot's (`WaitPlay` waits for the rest; disused, it is
    /// forgotten after it).
    started_at: u64,
    len_ms: Option<u32>,
    /// `SetPan`: -1000 (left) .. 1000 (right), for its next `Play` too.
    pan: i64,
    /// `SetFrequency` before it started: its rate (per mille), ramp ms and tempo.
    rate: Option<Ramp>,
    /// The rate it plays at (per mille; 1000 its own).
    freq: u32,
    /// Faded to 0 by `SetVolume`: when it stops ([`Compiler::sound_clock`]),
    /// as the exe's fade thread (`0x459980`) stops it at the fade's end.
    stops_at: Option<u64>,
}

/// A sound's ramp: to (per mille), over ms, along a tempo ([`Op::SoundEase`]).
type Ramp = (u32, u32, Option<(u8, u32)>);

/// A game's reading of a tempo as [`Op::MoveEase`]'s (ease, exponent).
type Tempo = fn(&Value) -> Option<(u8, u32)>;

/// An NSS tempo as [`Op::MoveEase`]'s (ease, exponent), the exe's
/// curves (apply_tempo 00449f40, run by research/nitroplus/tempo_exe.py; test `tempos_are_the_exes`): `Axl1`-`3`
/// t^2-t^4, `Dxl1`-`3` 1-(1-t)^2-^4, the sine ones as eases 4-7; any
/// other (`null` too) straight: `None`.
fn tempo(v: &Value) -> Option<(u8, u32)> {
    match v.string().as_str() {
        "Axl1" => Some((1, 2)),
        "Axl2" => Some((1, 3)),
        "Axl3" => Some((1, 4)),
        "Dxl1" => Some((2, 2)),
        "Dxl2" => Some((2, 3)),
        "Dxl3" => Some((2, 4)),
        "AxlAuto" => Some((4, 1)),
        "DxlAuto" => Some((5, 1)),
        "AxlDxl" => Some((6, 1)),
        "DxlAxl" => Some((7, 1)),
        _ => None,
    }
}

/// Calls left out on purpose (a note's start, why): done for the
/// completion figure (`metrics.json`'s `left_out`), not gaps.
const LEFT_OUT: &[(&str, &str)] = &[
    ("skipped: no function ", "no function of this name: the exe skips the call too"),
    (builtin::CONQUEST, "whole-scene read marks for the skip: the engine keeps its own per page"),
    (builtin::CHOICE_SCREEN, "the choice screen's own code: the engine's choice stands in"),
];

/// Why a call whose notes are all [`LEFT_OUT`]'s is left out.
fn left_out(reasons: &[String]) -> Option<&'static str> {
    let why = |r: &String| LEFT_OUT.iter().find(|(start, _)| r.starts_with(start)).map(|&(_, why)| why);
    let first = why(reasons.first()?)?;
    reasons.iter().all(|r| why(r).is_some()).then_some(first)
}

/// `old` moved and zoomed into `now` over `ms` by the engine: the same
/// picture, zoomed alike both ways. A fade goes into the keys' opacity,
/// times the sprite's, which must first be set to the larger of the two
/// ([`fade_base`]).
fn motion(old: &Shown, now: &Shown, ms: u32) -> Option<Op> {
    // (A `DrawTransition` is the engine's spfadeto, after a placement.)
    if old.layer != now.layer || old.source != now.source || old.fade != now.fade {
        return None;
    }
    // A movie (one cell, the same file or frame) keeps playing as it moves:
    // its zoom is the cell's, its size the movie's.
    let (zx, zy) = match (old.cells, now.cells) {
        (None, None) => (now.zoom.0 * 1000 / old.zoom.0.max(1), now.zoom.1 * 1000 / old.zoom.1.max(1)),
        (Some((1, _, (ox, oy), _)), Some((1, _, (nx, ny), _))) if old.image == now.image && (old.w, old.h) == (now.w, now.h) => {
            (i64::from(nx) * 1000 / i64::from(ox).max(1), i64::from(ny) * 1000 / i64::from(oy).max(1))
        }
        _ => return None,
    };
    // (A picture cut to the screen, see `Assets::scaled`, isn't the whole.)
    let sized = |a: i64, b: i64, z: i64| now.cells.is_some() || (b * 1000 - a * z).abs() <= a * 10 + 1000;
    let (o, n) = (old.whole.unwrap_or((old.x, old.y, old.w, old.h)), now.whole.unwrap_or((now.x, now.y, now.w, now.h)));
    // (The engine zooms the old sprite: a cut one would show only its part.)
    if (zx - zy).abs() > 5 || !sized(o.2, n.2, zx) || !sized(o.3, n.3, zy) || old.whole.is_some() {
        return None;
    }
    let centre = |s: &Shown| s.whole.map_or((s.x + s.w / 2, s.y + s.h / 2), |(x, y, w, h)| (x + w / 2, y + h / 2));
    let ((x0, y0), (x1, y1)) = (centre(old), centre(now));
    let zoom = ((zx + 5) / 10).clamp(1, 10000) as u32;
    let base = fade_base(old, now);
    let alpha = |s: &Shown| (s.opacity.min(1000) * 256 / base) as u32;
    let key = |ms, x, y, zoom, angle, opacity| MotionKey { ms, x, y, zoom, angle, opacity, zoom_y: zoom, ease: None };
    let keys = vec![key(0, 0, 0, 100, 0, alpha(old)), key(ms, (x1 - x0) as i32, (y1 - y0) as i32, zoom, (now.angle - old.angle) as i32, alpha(now))];
    Some(Op::SpriteMotion2 { layer: old.layer, keys, end: MotionEnd::Keep })
}

/// A zoom over time more along one axis than the other (dja023: a
/// picture unfolding from a line): the keys for the new sprite, placed
/// where it ends, from the old place and zoom ([`motion`] takes even
/// zooms only).
fn uneven_motion(old: &Shown, now: &Shown, ms: u32) -> Option<Vec<MotionKey>> {
    let (o, n) = (old.zoom, now.zoom);
    if (n.0 * o.1 - n.1 * o.0).abs() * 200 <= o.0 * o.1 || n.0 <= 0 || n.1 <= 0 || old.whole.is_some() || now.whole.is_some() {
        return None;
    }
    let centre = |s: &Shown| (s.x + s.w / 2, s.y + s.h / 2);
    let ((x0, y0), (x1, y1)) = (centre(old), centre(now));
    let key = |ms, x, y, zoom, zoom_y, angle| MotionKey { ms, x, y, zoom, zoom_y, angle, opacity: 256, ease: None };
    Some(vec![key(0, (x0 - x1) as i32, (y0 - y1) as i32, (o.0 * 100 / n.0) as u32, (o.1 * 100 / n.1) as u32, (old.angle - now.angle) as i32), key(ms, 0, 0, 100, 100, 0)])
}

/// The opacity (per mille) a sprite fading from `old` to `now` in a
/// [`motion`] is set to: its keys' opacity is times it.
fn fade_base(old: &Shown, now: &Shown) -> i64 {
    old.opacity.max(now.opacity).clamp(1, 1000)
}

/// A picture's move over time, as [`Compiler::sync`] gave it to the engine.
#[derive(Debug, Clone)]
struct Run {
    from: Shown,
    to: Shown,
    ms: u32,
    /// When it started ([`Compiler::sound_clock`] ms).
    at: u64,
    ease: Option<(u8, u32)>,
}

/// `SetBlur`'s copies ([`Object::trail`]).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Trail {
    passes: u32,
    alpha: u32,
    lag: u32,
    /// The copies drawn over the picture (`SetBlur`'s `wait` not `true`).
    over: bool,
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
    /// The picture before its zoom (mirrored), and the zoom (per
    /// mille): a picture that only moves or zooms over time is moved by
    /// the engine ([`Op::SpriteMotion2`]).
    source: String,
    zoom: (i64, i64),
    /// Turned (degrees counterclockwise) by the engine ([`Op::SpriteAngle`]).
    angle: i64,
    /// Drawn only inside its windows' rectangle ([`Op::SpriteClip`]).
    clip: Option<(i64, i64, i64, i64)>,
    engine: Engine,
    /// A [`Kind::Flash`]'s `seq` (0 for any other picture): a new flash
    /// starts again.
    flash: u64,
    /// How it is drawn on what is under it ([`Op::SpriteBlend`]).
    blend: u8,
    /// Drawn through a [`Kind::Tint`] above it ([`Op::SpriteFilters`]).
    tint: Option<Vec<SpriteFilter>>,
    /// Drawn over the text window ([`Compiler::over_text`]).
    over: bool,
    /// Cut to the screen at a big zoom (`Assets::scaled`): the place and
    /// size of the whole picture (x, y, w, h), which a motion goes by.
    whole: Option<(i64, i64, i64, i64)>,
    /// Its `DrawTransition` ([`Op::SpriteFadeTo`]).
    fade: Option<FadeTo>,
}

/// `DrawTransition(obj, ms, from, to, softness, tempo, mask, wait)` on a
/// picture: through `mask` (an output picture) from level `from` to `to`
/// over `ms`, eased by `ease`, ending at `until` on the sound clock (which
/// tells one call from the next, so each starts again).
// src: Muramasa.exe transition_start 0044fe00 starts at `from` whatever the
// picture showed, and leaves its opacity; each frame (00457bf0) draws it
// times the table.
#[derive(Debug, Clone, PartialEq)]
struct FadeTo {
    mask: String,
    ms: u32,
    from: i64,
    to: i64,
    softness: i64,
    ease: Option<(u8, u32)>,
    until: u64,
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
    /// The particle layers running ([`Kind::Particles`]), as last synced.
    particle_layers: BTreeMap<u32, conv_core::ir::Particles>,
    /// The longest non-blocking action since the last sync.
    pending_ms: u32,
    /// The same per object, for a `WaitAction` on one of them.
    pending_by: HashMap<String, u32>,
    /// The tempo of a pending `Move` or `Zoom` per object, as
    /// [`Op::MoveEase`] (see [`tempo`]).
    pending_tempo: HashMap<String, (u8, u32)>,
    /// The tempo of a pending `Fade` or `DrawTransition` over time: the
    /// next sync's transition goes along it ([`Op::TransitionEase`]).
    // shortcut: one curve a transition (the last fade's) when fades with different tempos meet in one sync
    fade_tempo: Option<(u8, u32)>,
    /// A pending full `DrawTransition`'s softness and whether it fades out
    /// ([`Op::TransitionSoft`]).
    transition_soft: Option<(u32, bool)>,
    /// A pending `Rotate` about x or y over time per object it turns.
    pending_turn: HashMap<String, turn::Turning>,
    /// The objects with a `Fade` over time pending (its ms).
    pending_fade: HashMap<String, u32>,
    /// Pictures moving as they fade out to 0, drawn till then
    /// ([`Compiler::sound_clock`] ms). Ordered: sync.rs emits its keys.
    vanishing: BTreeMap<String, u64>,
    /// When each picture's motion started at a sync ends
    /// ([`Compiler::sound_clock`] ms): its layer stays when the layers
    /// spread out again, so the engine's motion goes on.
    motion_until: HashMap<String, u64>,
    /// Pictures whose motion the script went on from (longer than the
    /// sync waited): a later `WaitAction` on one waits for it in the engine.
    unwaited_motions: std::collections::HashSet<String>,
    /// Pictures disused as an unwaited fade starts (`FadeDelete(…, false)`):
    /// deleted at the first sync after its fade, if still the same object
    /// (by its `seq`).
    disused_fading: Vec<(String, u64)>,
    /// Pictures faded since the last sync from 0 (hidden): a new one is
    /// faded in by the engine ([`Compiler::sync`]).
    fade_from_zero: std::collections::HashSet<String>,
    /// Objects the script made that the compiler doesn't keep (a `Create*`
    /// it leaves out, as the choice screen's), until deleted: an action on
    /// them may still wait in the original.
    untracked: std::collections::BTreeSet<String>,
    /// The cameras a `MoveCamera` over time moves at the next sync (its ms).
    pending_camera: HashMap<String, u32>,
    /// The pictures' moves running: a new picture beside them in a moving
    /// camera moves as they do.
    runs: BTreeMap<String, Run>,
    /// A `DrawTransition` mask for the next sync.
    pending_mask: Option<String>,
    /// The pictures the running screen shake shakes: it stops when they go.
    shaken: Vec<String>,
    font_size: i64,
    /// The last `SetFont`'s arguments, for `CreateText`.
    font: Vec<Value>,
    /// The box the last `SetTextEX…` names (`L`, `CH`, …), for a game's
    /// [`Game::box_text`] when two boxes are up.
    text_box: Option<String>,
    window: Option<TextWindow>,
    first_window: Option<TextWindow>,
    window_visible: bool,
    /// The box faded in from hidden (ms, waited): the next page shows the
    /// window fading in, and a waited fade holds its text until it is in.
    window_fade_in: Option<(u32, bool)>,
    texts_shown: usize,
    /// `Request("@text131", Enter)` before the text block `[text131]`
    /// exists (続・殺戮のジャンゴ's `TypeBegin`): the block shows its
    /// pages as soon as it runs.
    pending_enter: Option<String>,
    /// While a music process runs ([`Compiler::start_process`]): its time
    /// so far, ms.
    process_clock: Option<u32>,
    /// The last `SetVolume`'s ms in a music process.
    process_fade: u32,
    /// The music process running does what only the run could (a sound
    /// effect after a wait, or its fade): it is noted instead.
    music_refused: bool,
    /// [`Compiler::sound_clock`]: the time of the ops up to `clock_scanned`.
    clock_ms: u64,
    clock_scanned: usize,
    /// `Shake`s of pictures not shown yet, as `spmotion` keys: started
    /// when the next sync shows them, dropped otherwise.
    shake_once: HashMap<String, Vec<(u32, i32, i32)>>,
    /// `BezierMove`s since the last sync: the pictures' places along the
    /// way, as `spmotion` keys from where they start ([`loops`]).
    pending_path: HashMap<String, Vec<MotionKey>>,
    /// Pictures that `Move`s since the last sync: where they start, from
    /// where they end (a new one shown there and moved, [`Compiler::sync`]).
    pending_from: HashMap<String, (i64, i64)>,
    /// Hidden pictures whose unwaited move the last sync left for the next.
    kept_moves: std::collections::HashSet<String>,
    /// Of those, the ones that `Rotate` about z over time: the angle they
    /// had (shown at it and turned as they move).
    pending_angle_from: HashMap<String, i64>,
    /// Pictures that `Zoom` over time since the last sync: the zoom they
    /// had (for a turn's motion, [`turn::Turning::zoom`]).
    pending_size_from: HashMap<String, (i64, i64)>,
    /// Pictures that `Zoom`s since the last sync zoom from or to 0 along
    /// an axis over time: the zoom they had (squashed away or grown back,
    /// [`Compiler::sync`]).
    pending_zoom_from: HashMap<String, (i64, i64)>,
    /// The action process running ([`Compiler::start_process`]): its
    /// actions don't block, and their time counts as its own.
    action_process: Option<String>,
    /// How long the loop process starting has waited before its loop
    /// ([`loops`]): its waits hold only the process, so its loop starts
    /// this much later (a lead-in) while the script goes on.
    process_lead: u32,
    /// Pictures whose loop starts after the move they are in ([`loops`]),
    /// and the lead-in's time: the next sync makes the move the loop's
    /// lead-in, so that neither motion replaces the other.
    lead_ins: BTreeMap<String, u32>,
    /// Processes run in line ([`Compiler::inline_process`]): their waits
    /// hold the script.
    inline_processes: Vec<String>,
    /// Pictures moved by a one-shot process ([`one_shot`]), and their
    /// motions.
    one_shots: BTreeMap<String, one_shot::OneShot>,
    /// The next runtime `if` is the `SkipOut` test of the scene with this
    /// read flag ([`Compiler::fast_skip_test`]).
    fast_skip_read: Option<conv_core::ir::Var>,
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
    metrics: crate::metrics::Metrics,
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
    /// The variables choices read ([`choice_flags`]).
    choice_reads: std::collections::BTreeSet<String>,
    /// The strings each story flag has been given, by code: a choice's
    /// option texts known only at run time.
    flag_texts: HashMap<conv_core::ir::Var, BTreeMap<i32, String>>,
    /// Codes of the strings story flags hold ("" = 0).
    codes: HashMap<String, i32>,
    /// Helper variables of the current file that became runtime ones at a
    /// merge, the next variable for one, and the strings each can hold.
    dynamic: HashMap<String, conv_core::ir::Var>,
    next_dynamic: u16,
    dynamic_values: HashMap<conv_core::ir::Var, std::collections::BTreeSet<String>>,
    /// The `Random` picks being compiled ([`random`]), whether each
    /// function draws one, and processes being played on paper (which
    /// draw when converting).
    random: Option<random::Draws>,
    random_users: HashMap<String, bool>,
    paper: u32,
    random_sites: usize,
    /// (op index, statement) marks for [`Compiled::source_map`], and the
    /// lines of the statements being run, innermost last.
    source_map: Vec<(usize, conv_core::ir::SourceLoc)>,
    running: Vec<(&'a str, u32)>,
    /// A sound's last `Play`: the statement that called the helper it is in
    /// (`SoundPlay`), whose mark its ops take when [`Self::flush_sounds`] starts it.
    played_at: HashMap<String, (&'a str, u32)>,
    /// The file of a page's code being run (the `<PRE>`'s), for the marks.
    code_file: Option<&'a str>,
    /// The scene file being run (not a function's): where its calls'
    /// motion time goes (`metrics.json`'s timing parity).
    scene_file: &'a str,
    /// The (picture, ms) moved, zoomed or turned since the last wait:
    /// [`Self::parity_motion`].
    parity_moving: HashSet<(String, i64)>,
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
    particle_layers: BTreeMap<u32, conv_core::ir::Particles>,
    font_size: i64,
    window: Option<TextWindow>,
    window_visible: bool,
}


impl<'a, 'b> Compiler<'a, 'b> {

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
            particle_layers: BTreeMap::new(),
            pending_ms: 0,
            pending_by: HashMap::new(),
            pending_tempo: HashMap::new(),
            fade_tempo: None,
            transition_soft: None,
            pending_turn: HashMap::new(),
            pending_fade: HashMap::new(),
            vanishing: BTreeMap::new(),
            motion_until: HashMap::new(),
            unwaited_motions: Default::default(),
            disused_fading: Vec::new(),
            fade_from_zero: Default::default(),
            untracked: Default::default(),
            pending_camera: HashMap::new(),
            runs: BTreeMap::new(),
            pending_mask: None,
            shaken: Vec::new(),
            // src: system.ini [フォント] size (18 in both 装甲悪鬼村正's and 続・殺戮のジャンゴ's)
            font_size: 18,
            font: Vec::new(),
            text_box: None,
            window: None,
            first_window: None,
            window_visible: false,
            window_fade_in: None,
            texts_shown: 0,
            pending_enter: None,
            process_clock: None,
            process_fade: 0,
            music_refused: false,
            clock_ms: 0,
            clock_scanned: 0,
            shake_once: HashMap::new(),
            pending_path: HashMap::new(),
            pending_from: HashMap::new(),
            kept_moves: std::collections::HashSet::new(),
            pending_angle_from: HashMap::new(),
            pending_size_from: HashMap::new(),
            pending_zoom_from: HashMap::new(),
            action_process: None,
            process_lead: 0,
            lead_ins: BTreeMap::new(),
            inline_processes: Vec::new(),
            one_shots: BTreeMap::new(),
            fast_skip_read: None,
            chapter_mark: None,
            box_layouts: HashMap::new(),
            whole_story: false,
            loaded_images: HashMap::new(),
            voice_classes: Vec::new(),
            ops: Vec::new(),
            notes: BTreeMap::new(),
            metrics: Default::default(),
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
            choice_reads: choice_flags(files),
            flag_texts: HashMap::new(),
            codes: HashMap::new(),
            dynamic: HashMap::new(),
            next_dynamic: 0,
            dynamic_values: HashMap::new(),
            random: None,
            random_users: HashMap::new(),
            paper: 0,
            random_sites: 0,
            source_map: Vec::new(),
            running: Vec::new(),
            played_at: HashMap::new(),
            code_file: None,
            scene_file: file,
            parity_moving: HashSet::new(),
        }
    }

    /// SetBacklog's speaker as a voice class seen so far: the class itself,
    /// or one it starts (村正 for 村正さん: the exe looks for the speaker in
    /// the list of classes switched off).
    /// shortcut: a class first voiced after the line has no switch there
    /// (1 of 90 lines, ma01_021's 鈴川); a pre-pass would number it.
    fn backlog_character(&self, speaker: &str) -> Option<u32> {
        let c = &self.voice_classes;
        c.iter().position(|c| c == speaker).or_else(|| c.iter().position(|c| !speaker.is_empty() && c.starts_with(speaker))).map(|i| i as u32)
    }

    fn note(&mut self, what: impl Into<String>) {
        let what = what.into();
        self.metrics.note(&what);
        *self.notes.entry(what).or_default() += 1;
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
        let (mut skip, mut checkpoint) = (0, None);
        for (i, s) in stmts.iter().enumerate() {
            if i < skip {
                continue;
            }
            if let Some((end, _)) = self.exec_drawing(stmts, i, &mut checkpoint) {
                skip = end + 1;
            } else {
                checkpoint = self.checkpoint(stmts, i).or(checkpoint.filter(|_| random::simple(s)));
                self.exec(s);
            }
            self.flush_sounds_after(s.line);
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
            let mut skip = 0;
            let mut checkpoint = None;
            for (i, s) in part.iter().enumerate() {
                if i < skip {
                    continue;
                }
                if let Some((end, flow)) = self.exec_drawing(part, i, &mut checkpoint) {
                    skip = end + 1;
                    if self.in_function == 0 {
                        self.flush_sounds_after(s.line);
                    }
                    match flow {
                        Flow::Normal => continue,
                        flow => return flow,
                    }
                }
                if let StmtKind::If { cond, then, otherwise } = &s.kind
                    && (ends_in_return(then) || otherwise.as_deref().is_some_and(ends_in_return))
                    && self.is_runtime(cond)
                {
                    let mut rest = vec![&part[i + 1..]];
                    rest.extend_from_slice(&parts[pi + 1..]);
                    self.runtime_if_then(cond, then, otherwise.as_deref(), &rest);
                    if self.in_function == 0 {
                        self.flush_sounds_after(s.line);
                    }
                    return Flow::Normal;
                }
                checkpoint = self.checkpoint(part, i).or(checkpoint.filter(|_| random::simple(s)));
                let flow = self.exec(s);
                // A statement of a scene (not of a helper) has finished
                // setting up the sounds it started.
                if self.in_function == 0 {
                    self.flush_sounds_after(s.line);
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
        let file = self.code_file.take().unwrap_or(self.file);
        self.running.push((file, stmt.line));
        self.mark();
        let flow = self.exec_statement(stmt);
        self.running.pop();
        self.mark(); // what follows a call comes from the calling statement
        flow
    }

    /// Marks the ops from here on as lowered from the statement being run.
    fn mark(&mut self) {
        let Some(&(file, line)) = self.running.last() else { return };
        let n = self.ops.len();
        let map = &mut self.source_map;
        // ops taken back (a pick compiled again) and marks with no op
        while map.last().is_some_and(|m| m.0 >= n) {
            map.pop();
        }
        if map.last().is_none_or(|m| m.1.index != line as usize || m.1.file != file) {
            map.push((n, conv_core::ir::SourceLoc { file: file.to_string(), index: line as usize }));
        }
    }

    /// [`Self::flush_sounds`] after the statement at `line`, its ops marked as
    /// that statement's.
    fn flush_sounds_after(&mut self, line: u32) {
        self.running.push((self.file, line));
        self.mark();
        self.flush_sounds();
        self.running.pop();
        self.mark();
    }

    /// `ops.insert`, keeping the source marks after `at` on their ops.
    fn insert_op(&mut self, at: usize, op: Op) {
        self.ops.insert(at, op);
        self.source_map.iter_mut().filter(|m| m.0 >= at).for_each(|m| m.0 += 1);
    }

    fn exec_statement(&mut self, stmt: &'a Stmt) -> Flow {
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
                    let i = self.eval(i);
                    key = self.element_key(&key, &i);
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
                    AssignOp::Add => self.add_texts(&old, &value),
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
                // The player's auto mode and skip are engine state: stopped
                // where the script stops them (the ending roll).
                let stops = (key == "$SYSTEM_text_auto" && new.int() == 0) || (key == "$SYSTEM_menu_lock" && new.int() != 0);
                if stops && self.ops.last() != Some(&Op::StopAutoSkip) {
                    self.ops.push(Op::StopAutoSkip);
                }
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
            StmtKind::If { cond, then, otherwise } if let Some(read) = self.fast_skip_test(cond) => {
                self.fast_skip_read = Some(read);
                self.runtime_if(cond, then, otherwise.as_deref());
                self.ops.push(Op::Arith { var: read, op: ArithOp::Set, value: Operand::Const(1) });
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

    /// Timing parity (`metrics.json`): the motion time builtin `name`
    /// asks for. The timed builtins are all (obj, ms, …); a move, zoom or
    /// turn of a picture already moving over the same time since the last
    /// wait is the same motion (one `spmotion`), counted once.
    fn parity_motion(&mut self, name: &str, args: &[Value]) {
        let ms = args.get(1).map_or(0, |v| v.int().max(0));
        let ms = match name {
            "Move" | "Zoom" | "Rotate" | "BezierMove" => {
                let obj = args.first().map(Value::string).unwrap_or_default();
                if self.parity_moving.insert((obj, ms)) { ms } else { 0 }
            }
            "Fade" | "DrawTransition" | "Shake" | "MoveCamera" => ms,
            _ => {
                if name == "Draw" || name.starts_with("Wait") {
                    self.parity_moving.clear();
                }
                0
            }
        };
        self.metrics.motion(self.scene_file, ms as u64);
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
        let saved_scene = std::mem::replace(&mut self.scene_file, file);
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
        self.scene_file = saved_scene;
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
                if let Some(v) = self.variable_call(name, args) {
                    return v;
                }
                let exprs = args;
                let args: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
                let given = args.len();
                // A call of anything but the scripts' own functions is one
                // source command call (`metrics.json`).
                let command = self.function(name).is_none();
                if command {
                    self.metrics.begin(name);
                    self.parity_motion(name, &args);
                }
                let value = if (self.game.call)(self, name, &args) {
                    Value::Null
                } else {
                    match self.function(name) {
                        Some((file, f)) => self.call_function(file, f, args),
                        // (Its path is no value: read from the call.)
                        None if name == "BezierMove" => self.bezier_move(exprs, &args),
                        None => self.builtin(name, &args),
                    }
                };
                if command && let Some((kind, reasons)) = self.metrics.pop() {
                    match left_out(&reasons) {
                        Some(why) => self.metrics.left_out(&kind, why),
                        None => self.metrics.call(&kind, given, reasons),
                    }
                }
                value
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr);
                match op {
                    UnaryOp::Neg => Value::Int(-v.int()),
                    UnaryOp::Not => Value::Int(i64::from(!v.truthy())),
                    UnaryOp::Relative => Value::Rel(v.int()),
                }
            }
            // A variable's name in quotes added to a string means its value
            // (CreateSEEX: `"sound/se/" + "$音楽データ"`).
            // (Unless a parameter of that name binds the string.)
            Expr::Binary { op: BinaryOp::Add, left, right } if [left, right].iter().any(|e| matches!(&***e, Expr::Str(s) if self.names_variable(s))) => {
                let value = |c: &mut Self, e: &'a Expr| match e {
                    Expr::Str(s) if c.names_variable(s) => c.vars[s].clone(),
                    e => c.eval(e),
                };
                let (l, r) = (value(self, left), value(self, right));
                add(&l, &r)
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
                    BinaryOp::Add => self.add_texts(&l, &r),
                    BinaryOp::Sub => Value::Int(l.int() - r.int()),
                    BinaryOp::Mul => Value::Int(l.int() * r.int()),
                    BinaryOp::Div => Value::Int(l.int().checked_div(r.int()).unwrap_or(0)),
                    BinaryOp::Rem => Value::Int(l.int().checked_rem(r.int()).unwrap_or(0)),
                }
            }
            Expr::Point { .. } | Expr::Path(_) => Value::Null,
        }
    }

    /// Whether the string literal `s` names a variable that has a value
    /// (and no parameter binds the string).
    fn names_variable(&self, s: &str) -> bool {
        s.starts_with(['$', '#']) && self.vars.contains_key(s) && self.bound(&format!("s:{s}")).is_none() && self.game.string_vars
    }

    fn index_key(&mut self, e: &'a Expr) -> String {
        match e {
            Expr::Var(v) => v.to_string(),
            Expr::Index { base, index } => {
                let base = self.index_key(base);
                let index = self.eval(index);
                self.element_key(&base, &index)
            }
            other => other.to_string(),
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

#[cfg(test)]
mod tests;
