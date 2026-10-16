//! The intermediate representation: what a scene does, independent of the
//! source engine and of NScripter syntax.
//!
//! Assets are referred to by their path inside the output directory (forward
//! slashes, e.g. `bg/bg20d1.png`); the engine side is responsible for writing
//! those files.

mod picture;
mod screens;
pub use picture::*;
pub use screens::*;

/// How a transition gets from the old picture to the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransitionStyle {
    /// A cross-fade (shaped by the transition's mask, if it has one).
    Fade,
    /// With a mask: each pixel switches to the new picture at once when
    /// the progress passes its grey level (dark first).
    Threshold,
    /// The old picture fades to white, then the new one fades in from it.
    ThroughWhite,
    /// A picture turns edge-on about the screen centre (squeezed in width
    /// if `horizontal`, else in height): the new one grows from a line over
    /// the old (`grow`), or the old one shrinks to a line over the new.
    Flip { horizontal: bool, grow: bool },
    /// `moving` travels one screen width or height in direction `dx`, `dy`
    /// (each -1, 0 or 1; +y is down).
    Slide { moving: SlideLayer, dx: i8, dy: i8 },
    /// The old picture breaks into growing blocks, then the new one
    /// sharpens from them.
    Mosaic,
    /// A curtain: stripes across the screen open one after another, the
    /// edge travelling in direction `dx`, `dy` (one of them ±1).
    Curtain { dx: i8, dy: i8 },
    /// Zooms about the screen centre, up to `max_percent` (100 = no zoom),
    /// eased by the progress to the power `curve`.
    Zoom { style: ZoomStyle, max_percent: u32, curve: u8 },
    /// The old picture melts down off the screen behind a wavy edge, each
    /// column falling faster and faster; a falling column is stretched by
    /// up to `stretch_percent` more.
    Melt { stretch_percent: u32 },
    /// The pictures are two faces of a cube as large as the screen,
    /// turning a quarter turn, seen straight on: the new face comes in
    /// moving in direction `dx`, `dy` (one of them ±1), both squeezed.
    Cube { dx: i8, dy: i8 },
    /// A cross-fade with the rows waving sideways (`direction` 0 or 2; 1
    /// or 3 the columns up and down; 2 and 3 the other way), each shifted
    /// by up to `amplitude`/1000 of the screen, `waves`/10 waves down it,
    /// the most halfway.
    Wave { direction: u8, amplitude: u32, waves: u32 },
    /// A cross-fade with the pictures blurred sideways by up to `radius`
    /// pixels either side halfway; `new_only`: only the new picture.
    Blur { radius: u32, new_only: bool },
}

/// What moves in a [`TransitionStyle::Slide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlideLayer {
    /// The new picture slides in over the old one.
    New,
    /// The old picture slides out, uncovering the new one.
    Old,
    /// Both move together (a push).
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ZoomStyle {
    /// The new picture shrinks into place while the old one fades out.
    NewIn,
    /// The old picture grows and fades out over the new one.
    OldOut,
    /// Into the old picture for the first half, out of the new one after.
    Through,
    /// The new picture shrinks into place while the old one grows and
    /// fades out.
    Cross,
}

/// Where an instruction came from, for reports and comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoc {
    pub file: String,
    pub index: usize,
}

/// A script variable. Global variables survive across playthroughs and
/// saves (flags like "route cleared"); local ones belong to one game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Var {
    pub global: bool,
    pub index: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Const(i32),
    Var(Var),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    BitAnd,
    BitOr,
}

/// Comparisons for [`Op::If`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl Compare {
    /// The NScripter operator.
    pub fn operator(self) -> &'static str {
        match self {
            Compare::Eq => "==",
            Compare::Ne => "!=",
            Compare::Lt => "<",
            Compare::Gt => ">",
            Compare::Le => "<=",
            Compare::Ge => ">=",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpritePlacement {
    pub image: String,
    /// Top-left corner on screen, in pixels.
    pub x: i32,
    pub y: i32,
}

/// Shows (`Some`) or removes (`None`) the sprite on one layer. Each layer
/// holds one sprite; layers not mentioned keep theirs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteChange {
    pub layer: u32,
    pub sprite: Option<SpritePlacement>,
    /// The layer's colour filters ([`Op::SpriteFilters`]) stay on the new
    /// image; otherwise they are removed. Removing the sprite always
    /// removes them.
    pub keep_filters: bool,
}

/// Filter kinds for [`SpriteFilter::kind`] (the numbers of CONscripter's
/// `spfilter`). Each makes a colour for every pixel; 0 means no filter.
/// The colour in `params` (r, g, b).
pub const FILTER_COLOUR: i32 = 1;
/// The pixel in greyscale.
pub const FILTER_GREYSCALE: i32 = 2;
/// The pixel's negative.
pub const FILTER_NEGATIVE: i32 = 3;
/// The pixel plus `params[0]` (-255..255).
pub const FILTER_BRIGHTNESS: i32 = 4;
/// The pixel in greyscale plus the offsets in `params` (r, g, b,
/// -255..255): a monotone such as sepia.
pub const FILTER_MONOTONE: i32 = 5;
/// A blur: the mean of the pixels up to `params[0]`, `params[1]` pixels
/// away (x, y).
pub const FILTER_BLUR: i32 = 6;
/// The pixel with its hue turned by `params[0]`/256 of a turn and its
/// saturation moved by `params[1]` (-256 grey, 0 kept, 256 full).
pub const FILTER_HUE_SATURATION: i32 = 7;
/// The pixel with its lightness inverted (a negative that keeps the hue).
pub const FILTER_LIGHTNESS_INVERT: i32 = 8;
/// How a filter's colour is combined with the pixel's
/// ([`SpriteFilter::blend`]).
pub const FILTER_BLEND_COPY: i32 = 0;
pub const FILTER_BLEND_MULTIPLY: i32 = 1;
pub const FILTER_BLEND_ADD: i32 = 2;
pub const FILTER_BLEND_SCREEN: i32 = 3;

/// A colour filter on a sprite layer. The fields are operands because a
/// game can keep filter presets in variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteFilter {
    /// One of the `FILTER_*` kinds; 0 = none (skipped).
    pub kind: Operand,
    /// One of the `FILTER_BLEND_*` modes.
    pub blend: Operand,
    /// 0-256: how much of the blended colour replaces the pixel's.
    pub strength: Operand,
    pub params: [Operand; 3],
}

/// Static over the whole picture ([`Op::Noise`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Noise {
    /// `frames` pictures of static side by side, each as large as the
    /// screen, shown in turn for `frame_ms` each.
    pub image: String,
    pub frames: u32,
    pub frame_ms: u32,
    /// 0-256 (256 = only static).
    pub opacity: u32,
}

/// A full-screen animation ([`Op::Animation`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    /// The frames side by side, each as large as the screen, in the order
    /// they play (a picture may appear more than once).
    pub image: String,
    /// How long each frame shows.
    pub frame_ms: Vec<u32>,
    /// Starts over after the last frame; otherwise the last frame stays.
    pub looped: bool,
    /// 0-256 (256 = opaque).
    pub opacity: u32,
}

/// A moving effect on the whole picture ([`Op::ScreenEffect`]). Every
/// `interval_ms` it takes a step; over `time_ms` its sizes go from their
/// first to their last values, then stay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenEffect {
    /// Shaking by up to `range` pixels (x, y; first and last). `path`
    /// false: each axis swings on a sine wave advancing `speed`/65536 of a
    /// turn per ms; true: the picture jumps along a fixed 7-point path,
    /// moving on when `speed.0` × ms reaches 65536. `end_ms` > 0: it stops
    /// by itself after that long (0: until its slot is cleared).
    Shake { time_ms: u32, interval_ms: u32, path: bool, speed: (i32, i32), range: [(i32, i32); 2], end_ms: u32 },
    /// Rippling through `map` (a greyscale picture as large as the screen):
    /// each pixel comes from `amplitude` × sin(2π(grey/256 + phase/1000))
    /// pixels away in direction `angle` (0.1°, 0 = right, 900 = down). The
    /// rippled picture lies over the plain one at `opacity` (0-256), or with
    /// `multiply` the plain one is multiplied by it at that opacity. Each
    /// step turns the angle by `angle_step` and moves the phase by
    /// `phase_step`.
    Ripple {
        map: String,
        time_ms: u32,
        interval_ms: u32,
        opacity: (i32, i32),
        amplitude: (i32, i32),
        angle: i32,
        angle_step: i32,
        phase_step: i32,
        multiply: bool,
    },
    /// A copy of the picture taken when it starts, tinted (`tint` 0 none,
    /// 1 multiplied by `colour`, 2 `colour` laid over it, at
    /// `tint_opacity` 0-256), over the live picture scaled by
    /// 1 + f × `zoom`/1000 around the centre at f × `opacity` (0-256),
    /// f = sin(π × elapsed / `time_ms`). `colour` is 0xRRGGBB.
    Pulse { time_ms: u32, interval_ms: u32, zoom: (i32, i32), opacity: i32, colour: u32, tint_opacity: i32, tint: u8 },
    /// A copy of the picture frozen when it starts, over the live picture:
    /// over `zoom_ms` its scale goes from 1 to `zoom`/1000 and its centre
    /// from the screen's centre to `centre` (pixels), eased by `ease` (0
    /// straight, 1 in, 2 out, 3 in and out, with `power`/1000); it fades
    /// in over `fade_ms`, then stays until the slot is cleared. With
    /// `zoom` 1000 a freeze frame.
    Zoom { interval_ms: u32, zoom: i32, centre: (i32, i32), zoom_ms: u32, fade_ms: u32, ease: u8, power: i32 },
}

/// In [`Op::Text`] text: wait for a click here and go on typing on the
/// same page, at the same place (NScripter's `@`).
pub const CLICK_WAIT_MARK: char = '\u{1}';

/// In [`Op::Text`] text: wait here the milliseconds written after it (ASCII
/// digits, up to [`WAIT_END_MARK`]), or until a click, and go on typing.
pub const TIMED_WAIT_MARK: char = '\u{2}';

/// In [`Op::Text`] text: wait here until the page's voice has played the
/// milliseconds written after it (as [`TIMED_WAIT_MARK`]'s), has ended, or
/// a click.
pub const VOICE_WAIT_MARK: char = '\u{3}';

/// Ends the number after [`TIMED_WAIT_MARK`] or [`VOICE_WAIT_MARK`].
pub const WAIT_END_MARK: char = '\u{4}';

/// A clickable word in a text page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLink {
    /// Character range in the page text.
    pub start: usize,
    pub len: usize,
    /// Label run as a subroutine when the word is clicked; it ends with
    /// [`Op::Return`], and the page is shown again. `None` = colour only.
    pub target: Option<String>,
    /// Title of the scene the link opens (shown in the top bar).
    pub scene: Option<String>,
}

/// A reading (furigana) shown in small type above characters of a text
/// page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ruby {
    /// Character range in the page text.
    pub start: usize,
    pub len: usize,
    pub text: String,
}

/// Size, colour and slant for characters of a text page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextStyle {
    /// Character range in the page text.
    pub start: usize,
    pub len: usize,
    /// 0xRRGGBB; `None` keeps the colour.
    pub colour: Option<u32>,
    /// Percent of the window's font size; `None` keeps the size.
    pub size_percent: Option<u16>,
    /// Italic (CONscripter's `!i1` … `!i0`).
    pub italic: bool,
    /// The lines this range starts are centred in the window (`!c`).
    pub centred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Label(String),
    /// Continue at a label (possibly from another source script).
    Jump(String),
    /// Call a label as a subroutine; [`Op::Return`] comes back.
    Gosub(String),
    Return,
    /// `var = var <op> value`.
    Arith { var: Var, op: ArithOp, value: Operand },
    /// Jump to `label` if `var <cmp> value`.
    If { var: Var, cmp: Compare, value: Operand, label: String },
    /// Replace the background. Clears sprites, as most engines do.
    Background { image: String, fade_ms: u32 },
    /// Remove every sprite layer.
    ClearSprites,
    /// Show (or with `None` remove) a picture laid over the background:
    /// over the sprites if `over_sprites`, else between the background and
    /// the sprites. Unlike sprites, it stays when the background changes
    /// or the sprites are cleared. Shown at once unless a transition is
    /// open.
    Overlay { image: Option<String>, over_sprites: bool },
    /// Show (or with `None` remove) moving static over the picture: over
    /// the sprites and the overlay, under the text window. Like the
    /// overlay, it stays when the background changes or the sprites are
    /// cleared. Shown at once unless a transition is open.
    Noise(Option<Noise>),
    /// Play (or with `None` remove) a full-screen animation over the
    /// sprites and the overlay, under the static and the text window. Like
    /// the overlay, it stays when the background changes or the sprites
    /// are cleared. Shown at once unless a transition is open.
    Animation(Option<Animation>),
    /// Start (or with `None` stop) a moving effect on the picture behind
    /// the text window in `slot` (1-4); slot 0 with `None` stops them all.
    /// It runs by itself while the script goes on. Shown at once unless a
    /// transition is open.
    ScreenEffect { slot: u32, effect: Option<ScreenEffect> },
    /// Add `colour` (0xRRGGBB) at `opacity` (0-256) to the whole picture
    /// for `ms`, then take it away; the script waits, and a click ends it
    /// early. Drawn at once, even while a transition is open.
    Flash { colour: u32, opacity: u32, ms: u32 },
    /// Hide the text window; the next text page shows it again.
    HideTextWindow,
    /// Show the following pages whole (`true`) or typed out at the text
    /// speed (`false`). Setting the text window resets it to typed.
    InstantText(bool),
    /// Start collecting screen changes (background, sprites) without showing
    /// them; [`Op::EndTransition`] (or the next text page) reveals them all
    /// at once with a `fade_ms` cross-fade, shaped by `mask` (a greyscale
    /// image; dark parts change first) if given, or moving as `style` says.
    /// `effect` names the original transition.
    BeginTransition { effect: String, fade_ms: u32, mask: Option<String>, style: TransitionStyle },
    /// Reveal the changes since [`Op::BeginTransition`].
    EndTransition,
    /// Show or remove sprites on some layers, then show the result.
    Sprites { changes: Vec<SpriteChange>, fade_ms: u32 },
    /// Replace the colour filters on a layer's sprite (applied in order;
    /// empty = none), then show the result. A layer without a sprite
    /// ignores this.
    SpriteFilters { layer: u32, filters: Vec<SpriteFilter> },
    /// Set the opacity (0-256) of a layer's sprite (`Some(layer)`) or of
    /// the overlay (`None`). A layer without a sprite ignores it; a new
    /// picture on the layer is opaque again. Shown at once unless a
    /// transition is open.
    SpriteOpacity { layer: Option<u32>, opacity: u32 },
    /// Move a layer's sprite so that its centre is at (x, y), in a
    /// straight line over `ms` (0 = at once). A layer without a sprite
    /// ignores it. A jump is shown at once unless a transition is open; a
    /// motion is shown as it runs, so it must not be inside one.
    SpriteMove { layer: u32, x: i32, y: i32, ms: u32 },
    /// Move a layer's sprite (showing `image`) from centre `from` to `to`
    /// while it turns `degrees` clockwise, in straight lines over `ms`;
    /// it ends off screen, so the layer is empty afterwards. Shown as it
    /// runs: not inside a transition.
    SpriteTurn { layer: u32, image: String, from: (i32, i32), to: (i32, i32), degrees: i32, ms: u32 },
    /// Set (or with `None` clear) one slot of the filter on the whole
    /// picture behind the text window; slot 0 with `None` clears them
    /// all. Shown with a fade of `fade_ms` unless a transition is open.
    ScreenFilter { slot: u32, filter: Option<SpriteFilter>, fade_ms: u32 },
    /// One page of text, shown and then waited on. `text` may contain '\n'
    /// for line breaks. `links` are the words the original game made
    /// clickable. `ruby`: readings shown above some of the words. `styles`:
    /// sizes and colours of some of the characters. `hang`: lines the
    /// engine wraps start below this character of the first line.
    /// `continues`: the next page is added below this one after a click
    /// instead of replacing it (full-screen text modes).
    Text { speaker: Option<String>, text: String, voice: Option<Voice>, links: Vec<TextLink>, ruby: Vec<Ruby>, styles: Vec<TextStyle>, hang: Option<usize>, continues: bool },
    /// Switch to another text window layout; the page starts empty.
    SetWindow(TextWindow),
    BgmPlay { file: String, fade_in_ms: u32 },
    BgmStop { fade_out_ms: u32 },
    /// Pause; a click ends it early.
    Wait { ms: u32 },
    SePlay { file: String, looped: bool },
    /// `file: None` stops every sound effect.
    SeStop { file: Option<String> },
    /// Wait for a click with the text window and every bar hidden, to look
    /// at the picture. `skippable`: skip mode passes it (otherwise it
    /// stops skip mode).
    ClickWait { skippable: bool },
    /// Play a movie full screen and wait for its end, with the music
    /// stopped. `skippable`: a click ends it early.
    Movie { file: String, skippable: bool },
    /// A new chapter starts (shown in the top bar and save slots); an
    /// empty title: the chapter ended.
    Chapter { title: String },
    /// Scene `scene` of the scene replay was reached: unlock it there (a
    /// global flag, kept across playthroughs).
    UnlockScene { scene: u32 },
    /// Progress flag `bit` (0-31) of the playthrough was reached (a global
    /// flag, kept across playthroughs; the extras read them).
    Progress { bit: u32 },
    /// Leave every open link scene, as if each returned (their screens are
    /// put back), then go on here at the root. Nothing if none is open.
    ReturnToRoot,
    /// End the game and go back to the title screen (global flags stay).
    ReturnToTitle,
    /// CG `number` (1-based) counts as seen in the CG gallery without being
    /// shown (a global flag).
    CgSeen { number: u32 },
    /// The end of a scene the scene replay can play on its own: a replay
    /// stops here; normal play goes on.
    SceneEnd,
    /// Ask the player to pick one of `options` (text, label) in the text
    /// window (or in `style`'s own window) and continue at the label
    /// picked.
    Choice { options: Vec<(String, String)>, style: Option<ChoiceStyle> },
    /// A puzzle the player solves with the mouse on a screen of its own,
    /// then the screen goes white and clears to black (see [`TilePuzzle`]).
    TilePuzzle(Box<TilePuzzle>),
    /// Something the converter can't express yet; kept for the report.
    Unsupported { what: String, at: SourceLoc },
    /// A screen of pictures and image buttons over everything (see
    /// [`ButtonScreen`]); continues at the label of the button clicked.
    ButtonScreen(Box<ButtonScreen>),
    /// Move a layer's sprite so that its centre is at (x, y) and zoom it
    /// about that centre from `from` to `to` percent (100 = its size), in
    /// a straight line over `ms` (0 = at once), like [`Op::SpriteMove`]:
    /// a camera zoom. The zoom stays on the layer until it changes again
    /// or the sprite is removed (a new picture on the layer keeps it).
    SpriteZoom { layer: u32, x: i32, y: i32, from: u32, to: u32, ms: u32 },
    /// Fade the picture (under the text window and the bars) to `colour`
    /// (0xRRGGBB) over `ms`. The colour stays until the next background, a
    /// clear of the scene's sprites or a link scene's return.
    ScreenFade { colour: u32, ms: u32 },
    /// Wait `ms` milliseconds; unlike [`Op::Wait`], a click doesn't end it.
    Pause { ms: u32 },
    /// The next [`Op::SePlay`] fades its sound in over `ms`.
    SeFadeIn { ms: u32 },
    /// Fade a sound effect (`None`: every one) out over `ms`, then stop it,
    /// going on at once; [`Op::SeStop`] stops it at once.
    SeFadeOut { file: Option<String>, ms: u32 },
    /// The next [`Op::SePlay`] (and an [`Op::SeFadeIn`] before it) isn't
    /// played while the player skips (a sound the original leaves out in
    /// skip mode).
    SeNotSkipped,
    /// Like [`Op::Flash`], but over everything on the screen: the text
    /// window, the bars and the menus' buttons too.
    ScreenFlash { colour: u32, opacity: u32, ms: u32 },
    /// The next [`Op::Noise`]'s static rises from nothing to its opacity
    /// over `ms`, while the script goes on.
    NoiseFadeIn { ms: u32 },
    /// A screen of pictures over the scene until a click (see
    /// [`PictureScreen`]).
    PictureScreen(Box<PictureScreen>),
    /// Draw a layer's sprite in front of an [`Op::Overlay`] over the sprites
    /// (but under [`Op::Noise`] and [`Op::Animation`]), or (`front` false)
    /// back at its place. It stays until the sprite is removed.
    SpriteInFront { layer: u32, front: bool },
    /// The [`ChapterHistory`] starts over, empty.
    ChapterHistoryClear,
    /// Move a layer's sprite by itself along `keys` (ms from now, x, y
    /// offset from its place in pixels), straight from key to key, while
    /// the game goes on (text and click waits too): `looped` starts again
    /// after the last key, otherwise the sprite ends back at its place. No
    /// keys: stop the motion. A character trembling while it talks, a
    /// background shaking.
    SpriteMotion { layer: u32, keys: Vec<(u32, i32, i32)>, looped: bool },
    /// Jump to `label` if `switch` (a setting, [`Switch`]) is `on`.
    IfSwitch { switch: Switch, on: bool, label: String },
    /// The next timed [`Op::SpriteMove`] or [`Op::SpriteZoom`] eases (the
    /// MAGES engine's curves): with t the progress 0-1 and p =
    /// `exponent`, `ease` 1 goes as t^p, 2 as 1 - (1-t)^p, 3 as (2t)^p / 2
    /// for the first half and 1 - (2(1-t))^p / 2 for the second; 0 is
    /// straight.
    MoveEase { ease: u8, exponent: u32 },
    /// As [`Op::Movie`], but the music and the sound effects play on: a
    /// movie without sound of its own cut into a scene (装甲悪鬼村正's
    /// battle cuts, with their sound effect started beside them).
    SceneMovie { file: String, skippable: bool },
    /// Shows (as [`Op::Sprites`] does) a sprite whose picture is `cells`
    /// pictures side by side, shown in turn for `frame_ms` each: once,
    /// stopping on the last, or `looped`. `zoom` (x, y percent) draws it
    /// zoomed about the centre of its place. A movie played as frames
    /// while the script goes on.
    AnimatedSprite { layer: u32, sprite: SpritePlacement, cells: u32, frame_ms: u32, looped: bool, zoom: (i32, i32) },
    /// [`Op::SpriteMotion`] with a zoom, an angle and an opacity in each
    /// key ([`MotionKey`]), and what happens after the last key
    /// ([`MotionEnd`]). A character jumping toward the viewer, spinning,
    /// or walking off and fading out (俺たちに翼はない's CharacterAction).
    SpriteMotion2 { layer: u32, keys: Vec<MotionKey>, end: MotionEnd },
    /// Set the volume of the sound effect `file` (as [`Op::SePlay`] names
    /// it), or of the music (`None`), to `percent` of the player's setting,
    /// over `ms` (0 = at once) while the script goes on; at 0 it keeps
    /// playing. Right before the sound's `SePlay` or [`Op::BgmPlay`], it is
    /// the volume the sound starts at. In a script that has any, a sound
    /// started without one starts at 100.
    SoundVolume { file: Option<String>, percent: u32, ms: u32 },
    /// Plays `file` as the music `after_ms` from now (0: at once) while
    /// the script goes on, `looped` or once; a music op before then
    /// cancels it. Music a script starts on a timer: a loop a set time
    /// after its intro, a song some seconds after a line (続・殺戮のジャンゴ's
    /// processes).
    BgmLater { file: String, after_ms: u32, looped: bool },
    /// Plays the movie `file` only while the player holds the button (or
    /// a touch, Enter or Space) down, stopping on its frame when let go,
    /// with `prompt` animated over it; `sound` plays with it: paused and
    /// resumed with the movie when `fade_ms` is 0, else started again
    /// (looped) at each press and faded out over `fade_ms` when let go.
    /// The music goes on. 続・殺戮のジャンゴ's 400-vs-3 battle, where the
    /// player holds the trigger.
    HoldMovie { file: String, prompt: Option<HoldPrompt>, sound: Option<HoldSound> },
    /// Draws a layer's sprite added onto what is under it (a light), its
    /// opacity (0-256) following `keys` (ms from now, opacity; straight
    /// from key to key) `times` times while the script goes on, then
    /// staying at the last key's (0: until the layer is cleared).
    /// 続・殺戮のジャンゴ's muzzle flashes and strobe: white added on,
    /// flashing.
    SpriteFlashing { layer: u32, keys: Vec<(u32, u32)>, times: u32 },
    /// Draws a layer's sprite turned `degrees` counterclockwise about the
    /// centre of its place, from now until the layer is cleared (a motion's
    /// angle adds to it). 装甲悪鬼村正's battle HUD: the roll bar and the
    /// sky tilted as the craft rolls. `zoom`: its zoom percent (x, y),
    /// set with the angle.
    SpriteAngle { layer: u32, degrees: i32, zoom: (i32, i32) },
    /// Jumps to `label` while the player skips (not in auto mode): past
    /// what the original leaves out then. 最果てのイマ's quakes, flashes,
    /// pulses and script fades.
    IfSkipping { label: String },
    /// Auto mode and skip stop, as a click would (Nitroplus: the script
    /// sets `$SYSTEM_text_auto` false or `$SYSTEM_menu_lock`, before the
    /// ending roll).
    StopAutoSkip,
    /// Adds `text` to the text history as a page of its own, without
    /// showing it; `voice` (a sound file) is played from the history.
    /// Lines the story shows as pictures (装甲悪鬼村正's SetBacklog).
    BacklogLine { text: String, voice: Option<String> },
    /// Jumps to `label` while 超速スキップ is on: the skip that
    /// `BarAction::FastSkip` started still runs (装甲悪鬼村正's
    /// `SkipOut`, which jumps over a scene read before).
    IfFastSkip { label: String },
    /// [`Op::Sprites`] whose pictures fade in and out over `fade_ms` by
    /// themselves while the script goes on (the text, click waits). A new
    /// picture on a layer that had one replaces it at once, so a picture
    /// changing goes to another layer. Inside a transition, as
    /// [`Op::Sprites`]. 俺たちに翼はない's `:FadeWait 0`.
    SpritesFading { changes: Vec<SpriteChange>, fade_ms: u32 },
    /// The picture on `layer` rolls `dy` px (up when negative) in a
    /// straight line over the rest of the music playing now and `after_ms`
    /// more: the music is `song` (the file, `song_ms` long; how far
    /// through it is is read when the roll starts, and it starts from the
    /// beginning, once, when no music plays yet), and `next` plays once
    /// when it ends. The roll runs at the normal speed, whatever the
    /// player's speed setting (the next click wait sets it again). Then
    /// the script waits for a click; `skippable`: a click during the roll
    /// is that click.
    /// 続・殺戮のジャンゴ's ending credits (ProcessRoll), timed to the
    /// insert song left and the closing song.
    MusicRoll { layer: u32, dy: i32, song: String, song_ms: u32, next: Option<String>, after_ms: u32, skippable: bool },
    /// Draws a layer's sprite only inside the screen's rectangle (x, y, w,
    /// h), wherever it moves, until the layer is cleared (`spclip`). NSS's
    /// `CreateWindow` children: 装甲悪鬼村正's cockpit tapes.
    SpriteClip { layer: u32, rect: (i32, i32, i32, i32) },
    /// The picture on `layer` is drawn over the text window and its text
    /// until the layer is cleared (CONscripter's `spovertext`): the English
    /// Muramasa port's subtitles. Inside a transition, as [`Op::Sprites`].
    SpriteOverText { layer: u32 },
    /// An [`Op::Choice`] some of whose option texts are known only at run
    /// time: `texts[i]`, when given, picks option i's text by a variable
    /// (the option's own text when no value matches). NSS choices whose
    /// options a runtime if sets (装甲悪鬼村正's mc02_031innsyoku).
    RuntimeChoice { options: Vec<(String, String)>, texts: Vec<Option<RuntimeText>>, style: Option<ChoiceStyle> },
    /// [`Op::BacklogLine`] (no voice) whose text is picked at run time by
    /// `runtime` (`text` when no value matches): the log line of an
    /// [`Op::RuntimeChoice`]'s option.
    RuntimeBacklogLine { text: String, runtime: RuntimeText },
}

/// [`Op::HoldMovie`]'s prompt: `image` is `cells` pictures side by side,
/// each shown `frame_ms` in turn, drawn at (`x`, `y`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldPrompt {
    pub image: String,
    pub x: i32,
    pub y: i32,
    pub cells: u32,
    pub frame_ms: u32,
}

/// [`Op::HoldMovie`]'s sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldSound {
    pub file: String,
    pub fade_ms: u32,
}

/// A puzzle of numbered tiles (装甲悪鬼村正's 立方陣): the player clicks a
/// tile in the tray, then an empty cell to put it in (a right-click puts
/// it back); clicking a filled cell empties it and the tile goes back to
/// the tray. "Reset" empties every cell, "help" shows the rules until a
/// click, "answer" ends the puzzle: `solved` is set to 1 when every cell
/// holds its tile of the solution, else 0 (empty cells count as wrong).
///
/// Button images are two-cell strips: normal, under the mouse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilePuzzle {
    /// Pictures under everything, bottom first; the first fills the screen.
    pub backdrop: Vec<Placed>,
    pub tiles: Vec<PuzzleTile>,
    pub cells: Vec<PuzzleCell>,
    /// Size of a cell, and the strips drawn over an empty cell while a
    /// tile is picked, and over a filled one (its first cell all but
    /// invisible, the tile's number showing through). Buttons are hit by
    /// the alpha of their pictures, since cells may overlap.
    pub cell_w: i32,
    pub cell_h: i32,
    pub empty_cell: String,
    pub filled_cell: String,
    pub answer: Placed,
    pub reset: Placed,
    pub help: Placed,
    /// The help screen: lines (text, top-left x, y, colour 0xRRGGBB) over
    /// the screen dimmed by half, in this font size.
    pub help_lines: Vec<(String, i32, i32, u32)>,
    pub help_font_size: u32,
    /// When this variable is 1 at the start, the cells marked `easy` start
    /// with their solution's tile, and stay filled.
    pub easy: Option<Var>,
    pub solved: Var,
    /// Played while the puzzle is on screen; faded out at the answer.
    pub bgm: Option<String>,
    /// Fade-in of the puzzle's screen, and of each half of the white-out.
    pub fade_ms: u32,
}

/// A tile: its button in the tray (a strip), the picture drawn over it
/// while it is picked (an animation strip of `frames` cells, `frame_ms`
/// each, looping), and the picture of its number inside a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleTile {
    pub button: Placed,
    pub picked: String,
    pub frames: u32,
    pub frame_ms: u32,
    pub in_cell: String,
}

/// A cell's top-left, the tile of the solution (index into
/// [`TilePuzzle::tiles`]), and whether the easy mode fills it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuzzleCell {
    pub x: i32,
    pub y: i32,
    pub solution: usize,
    pub easy: bool,
}

/// How a choice is drawn when the game has its own look for it: the
/// window the options are written in (one line each, the window image
/// holds their plates) and the text colours of the option under the
/// mouse and of the others (0xRRGGBB). The text window after the choice
/// is set again by the next `SetWindow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceStyle {
    pub window: TextWindow,
    pub on_color: u32,
    pub off_color: u32,
    /// The options as picture buttons instead ([`ChoicePlates`]); then
    /// the window and colours are unused.
    pub plates: Option<ChoicePlates>,
}

/// Choice options as picture buttons (続・殺戮のジャンゴ's wooden plates):
/// a plate per option, `step` below the one before from (`x`, `y`), its
/// text centred about `text_x`, `text_dy` below the plate's top, in
/// `font_size` px `colour` (#RRGGBB) text. The plates and texts fade in
/// and out over `fade_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoicePlates {
    /// Two cells: at rest, under the pointer.
    pub image: String,
    pub x: i32,
    pub y: i32,
    pub step: i32,
    pub text_x: i32,
    pub text_dy: i32,
    pub font_size: u32,
    pub colour: String,
    pub fade_ms: u32,
    /// The chosen plate is shot instead of fading out.
    pub shot: Option<ChoiceShot>,
}

/// [`ChoicePlates`]' shot (続・殺戮のジャンゴ's `ChoiceA02`): `sound`
/// plays and `hole` (dark: see-through) is cut into the chosen plate and
/// its text, its top left (`hole_dx`, `hole_dy`) from the pointer; 300 ms
/// later the other options fade out over 300 ms, and 500 ms after that
/// the chosen one tips over backwards about its bottom edge in 800 ms
/// (cubic ease-in) as it fades out over 1000 ms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceShot {
    pub hole: String,
    pub hole_dx: i32,
    pub hole_dy: i32,
    pub sound: String,
}

/// A text chosen at run time ([`Op::RuntimeChoice`]): `texts` (value,
/// text) by the value of `var`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeText {
    pub var: Var,
    pub texts: Vec<(i32, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Script {
    /// Window caption.
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// The first global variable (NScripter `value`): local variables (saved
    /// with each save slot) can use more than the default 200. `None` =
    /// NScripter's default (200).
    pub global_border: Option<u32>,
    /// `None` = the engine's default full-screen (NVL) text window.
    pub window: Option<TextWindow>,
    /// `None` = start the script right away.
    pub title_screen: Option<TitleScreen>,
    /// `None` = ONScripter's built-in save and load menus.
    pub save_menu: Option<SaveMenu>,
    /// `None` = plain click waits (right-click opens ONScripter's menu).
    pub button_bar: Option<ButtonBar>,
    pub settings: Option<SettingsMenu>,
    /// `None` = ONScripter's own text history.
    pub backlog: Option<Backlog>,
    /// `None` = act without asking.
    pub confirm: Option<ConfirmDialog>,
    /// `None` = the title's option button opens the settings.
    pub extras: Option<ExtrasMenu>,
    pub ops: Vec<Op>,
    /// A line naming the build (its date, the converter's commit, the
    /// engine's version), shown small on the settings screen. `None` = not
    /// shown.
    pub version: Option<String>,
    /// `None` = right-click at a click wait does what the button bar does.
    pub command_menu: Option<CommandMenu>,
    /// The save and load screens as a grid of thumbnails, in place of
    /// [`Script::save_menu`].
    pub save_grid: Option<SaveGrid>,
    /// The text history scrolled a row at a time, in place of
    /// [`Script::backlog`].
    pub line_log: Option<LineLog>,
    /// The icon shown at the button bar's click waits (at each window's
    /// [`TextWindow::wait_icon_at`]); `None` = none.
    pub wait_icon: Option<WaitIcon>,
    /// The settings screen drawn as one picture, in place of
    /// [`Script::settings`].
    pub picture_settings: Option<PictureSettings>,
    /// The extras drawn from the game's pictures, opened by the title's
    /// [`TitleAction::Extras`].
    pub picture_extras: Option<PictureExtras>,
    /// Sounds of every button, in every menu and on the button bar; `None`
    /// = only the screens' own (`hover_se`, `click_se`).
    pub menu_sounds: Option<MenuSounds>,
    /// The text is single-byte (English): each text line is written as it
    /// is, ASCII punctuation and spaces kept, for a proportional font; not
    /// turned full-width, no links, ruby or styles.
    pub single_byte_text: bool,
}
