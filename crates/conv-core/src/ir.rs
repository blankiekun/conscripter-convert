//! The intermediate representation: what a scene does, independent of the
//! source engine and of NScripter syntax.
//!
//! Assets are referred to by their path inside the output directory (forward
//! slashes, e.g. `bg/bg20d1.png`); the engine side is responsible for writing
//! those files.

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
    /// moving on when `speed.0` × ms reaches 65536.
    Shake { time_ms: u32, interval_ms: u32, path: bool, speed: (i32, i32), range: [(i32, i32); 2] },
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
}

/// In [`Op::Text`] text: wait for a click here and go on typing on the
/// same page, at the same place (NScripter's `@`).
pub const CLICK_WAIT_MARK: char = '\u{1}';

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

/// Size and colour for characters of a text page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    /// Character range in the page text.
    pub start: usize,
    pub len: usize,
    /// 0xRRGGBB; `None` keeps the colour.
    pub colour: Option<u32>,
    /// Percent of the window's font size; `None` keeps the size.
    pub size_percent: Option<u16>,
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
}

/// An ADV-style text box (text at the bottom over the scene).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextWindow {
    /// Window background image and its top-left corner.
    pub image: String,
    pub x: i32,
    pub y: i32,
    /// Top-left of the first character.
    pub text_x: i32,
    pub text_y: i32,
    pub columns: u32,
    pub rows: u32,
    pub font_size: u32,
    /// Extra space between lines, in pixels.
    pub line_gap: u32,
    /// Vertical writing (tategaki): `columns` characters run down each
    /// line, the `rows` lines are columns read right to left, and
    /// (`text_x`, `text_y`) is the top-left of the leftmost one.
    pub vertical: bool,
    pub name_plate: Option<NamePlate>,
}

/// A separate box showing the speaker's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePlate {
    pub image: String,
    pub x: i32,
    pub y: i32,
    pub text_x: i32,
    pub text_y: i32,
    pub font_size: u32,
}

/// The game's title screen: image buttons over a full-screen picture, shown
/// before the first op runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleScreen {
    /// Full-screen picture (logo included).
    pub background: String,
    pub fade_ms: u32,
    pub buttons: Vec<TitleButton>,
    /// Button strips have a third cell, shown while the mouse button is
    /// held on the button.
    pub pressed: bool,
    /// Played when the pointer comes onto a button.
    pub hover_se: Option<String>,
    /// Music while the title is up (looping). Start fades it out over
    /// `start_bgm_fade_ms` while the screen fades to black.
    pub bgm: Option<String>,
    pub start_bgm_fade_ms: u32,
    /// After Start's fade to black, wait this long before the game.
    pub start_wait_ms: u32,
    /// Exit quits at once, without the [`ConfirmDialog`]'s question.
    pub exit_without_asking: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleButton {
    /// Two cells side by side, normal and under the mouse pointer (a third,
    /// pressed, with [`TitleScreen::pressed`]); one cell for
    /// [`TitleAction::Unavailable`].
    pub image: String,
    pub x: i32,
    pub y: i32,
    pub action: TitleAction,
    /// Played when the button is clicked.
    pub click_se: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TitleAction {
    /// Run the script from its first op.
    #[default]
    Start,
    Load,
    Settings,
    Exit,
    /// Shown but not clickable (the menu it opens isn't converted yet).
    Unavailable,
    /// Load the quick save; drawn at half opacity and not clickable while
    /// there is none.
    Continue,
}

/// An image with its top-left corner on screen, and its size (all cells).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub image: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A rectangle on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The save/load screen: `pages` pages of `rows` slots, each slot a row
/// with a thumbnail and a two-line label; clicking a row selects it, and
/// the save/load/delete buttons act on the selection.
///
/// Buttons are strips of three cells side by side: normal, under the mouse
/// pointer, and disabled (for page buttons: the current page).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveMenu {
    /// Full-screen layers over the game screen; `base` is drawn with
    /// `base_opacity` (0-255).
    pub base: Placed,
    pub base_opacity: u8,
    pub frame: Placed,
    pub save_header: Placed,
    pub load_header: Placed,
    pub delete: Placed,
    pub save: Placed,
    pub load: Placed,
    pub close: Placed,
    pub pages: Vec<PageButton>,
    /// Marks the current page's button (placed by [`PageButton`]).
    pub page_marker: Placed,
    pub rows: u32,
    pub row_y: i32,
    pub row_pitch: i32,
    /// Positions below are relative to the row's top-left corner (x = 0).
    /// Shown under the pointer: two cells, the first empty.
    pub row_hover: Placed,
    /// Added (additive blend) to the selected row.
    pub row_selected: Placed,
    pub thumbnail: Rect,
    /// On the thumbnail of the most recent save.
    pub new_icon: Placed,
    pub text_x: i32,
    pub text_y: i32,
    pub font_size: u32,
    pub line_height: i32,
    /// The selected slot's thumbnail at full size (absolute position).
    pub preview: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageButton {
    pub button: Placed,
    /// Top-left of [`SaveMenu::page_marker`] when this page is current.
    pub marker_x: i32,
    pub marker_y: i32,
}

/// Buttons shown during the game's click waits: the way into the menus.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ButtonBar {
    pub buttons: Vec<BarButton>,
    /// Bars along the top edge.
    pub top: Option<TopBars>,
    /// A picture over the buttons, moving with them (a slide-in bar): the
    /// buttons show through where it is see-through.
    pub base: Option<Placed>,
    /// The bar waits off screen and slides in when the pointer comes near
    /// (see [`BarSlide`]); `None`: always shown where placed.
    pub slide: Option<BarSlide>,
    /// Button strips have a fourth cell, shown while the mouse button is
    /// held on them.
    pub pressed: bool,
    /// Played by [`BarAction::QuickSave`] once it has saved.
    pub quick_save_se: Option<String>,
    /// [`BarAction::QuickLoad`] and [`BarAction::BackToChoice`] fade to
    /// black over this long before they load.
    pub load_fade_ms: u32,
}

/// How a [`ButtonBar`] along the right edge slides in and out: it waits
/// `hidden_dx` pixels right of its place (off screen), slides in over
/// `ms` (easing out, cubic) when the pointer goes right of `edge_x`
/// during a click wait, and back out when it goes back left. While it is
/// out a click that misses its buttons doesn't advance the text; while
/// the text window is hidden it stays out of the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarSlide {
    pub hidden_dx: i32,
    pub edge_x: i32,
    pub ms: u32,
}

/// The current chapter and the current link ("blog") scene, along the top.
///
/// The link scenes entered form a stack of levels (0 is the root) that
/// can be browsed like a web browser's history: back to the scene that
/// linked here, forward again, out to the root, or to any level from a
/// drop-down list. Each level is resumed where it was left. Going on
/// from a link page, or a scene ending, drops the levels below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopBars {
    pub chapter_bar: Placed,
    pub chapter_text: TextSpot,
    pub scene_bar: Placed,
    pub scene_text: TextSpot,
    /// Shown as the scene outside any link scene.
    pub root_scene: String,
    /// The box behind the level numbers. It, the numbers and the back,
    /// forward and exit buttons move down below the list while it is open.
    pub level_box: Placed,
    /// Link scene depth, "current / deepest": ten digit images 0-9 and
    /// where the two numbers go (top-left corners).
    pub digits: Vec<String>,
    pub depth_at: (i32, i32),
    pub max_depth_at: (i32, i32),
    /// Buttons, four cells side by side: normal, under the pointer,
    /// pressed, disabled. Back one level, forward one level, to the root.
    pub back: Placed,
    pub forward: Placed,
    pub exit: Placed,
    /// Opens and closes the list.
    pub list: Placed,
    /// List rows, cells as for the buttons: one per level from 1 to the
    /// deepest, the first at `row`'s position, `row_pitch` apart.
    /// `current_row` marks the current level.
    pub row: Placed,
    pub current_row: Placed,
    pub row_pitch: i32,
    /// The first row's title.
    pub row_text: TextSpot,
    pub row_color: String,
    pub current_row_color: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpot {
    pub x: i32,
    pub y: i32,
    pub font_size: u32,
    /// Width in full-width characters.
    pub columns: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarButton {
    /// Three cells side by side: normal, under the mouse pointer,
    /// disabled. One cell for [`BarAction::Unavailable`].
    pub image: Placed,
    /// The same while the mode it toggles (auto, skip) is on.
    pub active: Option<Placed>,
    pub action: BarAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarAction {
    Auto,
    Skip,
    /// The text history.
    Log,
    Save,
    Load,
    Settings,
    /// Hide the text window until the next click.
    HideWindow,
    /// Shown but not clickable (the menu it opens isn't converted yet).
    Unavailable,
    /// Save to the quick slot without asking (the title's
    /// [`TitleAction::Continue`] loads it).
    QuickSave,
    /// Load the quick save without asking; disabled while there is none.
    QuickLoad,
    /// Back to the title (asks [`Switch::ConfirmTitle`]).
    Title,
    /// Quit the game (asks [`Switch::ConfirmQuit`]).
    Quit,
    /// Switch between a window and full screen.
    Fullscreen,
    /// Close the [`CommandMenu`] it is in.
    Close,
    /// Skip to the next choice or unread text without showing the pages.
    FastSkip,
    /// Load the game saved right before the last choice (asks
    /// [`Switch::ConfirmBackToChoice`]); disabled while there is none.
    BackToChoice,
    /// Run the script's subroutine at the item's label
    /// ([`CommandItem::label`]), the text window hidden; the background,
    /// the game's sprites and the music are put back after.
    Call,
}

/// The extras the title's option button opens: a menu of modes over its
/// own background. Buttons are strips of cells side by side: normal and
/// under the pointer, then disabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtrasMenu {
    /// Full-screen picture (shared with the modes' screens).
    pub background: String,
    pub items: Vec<ExtrasItem>,
    pub gallery: Option<CgGallery>,
    pub replay: Option<SceneReplay>,
    pub music: Option<MusicRoom>,
    pub movies: Option<MovieRoom>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtrasItem {
    pub button: Placed,
    pub action: ExtrasAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtrasAction {
    /// The settings screen (when there is one).
    Settings,
    /// The CG gallery; disabled until a CG in it has been seen.
    Gallery,
    /// The scene replay.
    Replay,
    /// The BGM room.
    Music,
    /// Back to the title.
    Back,
    /// Drawn disabled (the mode isn't converted yet).
    Unavailable,
    /// The movie room.
    Movies,
}

/// The CG gallery: tabs (one per heroine), each with pages of thumbnails
/// in a grid; a thumbnail opens its CG and the variants seen of it.
/// Buttons are strips of two cells side by side: normal, under the
/// pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgGallery {
    pub header: Placed,
    pub exit: Placed,
    pub tabs: Vec<GalleryTab>,
    /// The grid: top-left of the first thumbnail, the pitch, columns.
    pub grid_x: i32,
    pub grid_y: i32,
    pub grid_dx: i32,
    pub grid_dy: i32,
    pub columns: usize,
    /// A slot whose CGs are all unseen, at the thumbnail's place.
    pub locked: String,
    /// "PAGE", then page numbers (one strip per page, `page_dx` apart from
    /// `pages_x`, `pages_y`) and the current page's marker (at the
    /// number's place plus `marker_dx`, `marker_dy`).
    pub page_label: Placed,
    pub page_numbers: Vec<String>,
    pub pages_x: i32,
    pub pages_y: i32,
    pub page_dx: i32,
    pub page_marker: String,
    pub marker_dx: i32,
    pub marker_dy: i32,
    pub viewer: CgViewer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryTab {
    /// The tab's button; `selected` replaces it while the tab is shown.
    pub button: Placed,
    pub selected: String,
    /// Shown over the grid.
    pub title: Placed,
    /// Shown in the viewer's panel.
    pub viewer_name: Placed,
    pub pages: Vec<Vec<GallerySlot>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GallerySlot {
    /// Two cells: normal, under the pointer.
    pub thumbnail: String,
    /// The CGs in viewing order: [`Op::CgSeen`] number and picture.
    pub cgs: Vec<(u32, String)>,
}

/// The full-screen CG with a control panel: previous / next variant, hide
/// the panel (a click shows it again), back. Buttons as in [`CgGallery`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgViewer {
    /// Panel pictures, drawn in order.
    pub panel: Vec<Placed>,
    /// Where the tab's name goes.
    pub name_x: i32,
    pub name_y: i32,
    pub previous: Placed,
    pub next: Placed,
    pub hide: Placed,
    pub back: Placed,
}

/// The scene replay: a menu of lists (one per route) over the extras'
/// background; a list opens a window of the unlocked scenes' titles in
/// columns, and a title plays its scene from its label to the scene's end
/// ([`Op::SceneEnd`]), then shows the list again. During a replay the
/// settings screen's "back to the title" goes back to the list instead
/// ([`SceneReplay::leave_label`]) and the button bar has no save or load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneReplay {
    pub header: Placed,
    /// Back to the extras menu; two cells: normal, under the pointer.
    pub exit: Placed,
    pub lists: Vec<ReplayList>,
    /// Full-screen picture behind an open list: the menu as the window
    /// darkens it, with the lists' buttons in their normal state.
    pub list_background: String,
    pub frame: Placed,
    /// Two cells: normal, under the pointer.
    pub close: Placed,
    /// A row's button: two cells, nothing and the row under the pointer.
    pub selector: String,
    /// Marks a scene not replayed yet.
    pub new_mark: String,
    /// Rows per column (columns as many as the window holds).
    pub rows: usize,
    pub columns: usize,
    pub column_dx: i32,
    pub row_dy: i32,
    /// The first row's top: selector and "new" mark at their x.
    pub row_y: i32,
    pub selector_x: i32,
    pub new_x: i32,
    /// The first title's top left.
    pub text_x: i32,
    pub text_y: i32,
    pub font_size: u32,
    pub text_color: String,
    /// Colour of the titles of [`ReplayScene::highlight`] scenes.
    pub highlight_color: String,
    /// The settings screen's "back to the title" during a replay, and the
    /// question it asks.
    pub leave_label: String,
    pub leave_question: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayList {
    /// Three cells: normal, under the pointer, disabled.
    pub button: Placed,
    /// Drawn over [`SceneReplay::list_background`] while the list is open.
    pub selected: Placed,
    pub scenes: Vec<ReplayScene>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayScene {
    /// Unlocked by [`Op::UnlockScene`] with this number.
    pub scene: u32,
    pub title: String,
    /// Where the replay starts.
    pub label: String,
    /// Written in [`SceneReplay::highlight_color`].
    pub highlight: bool,
}

/// The BGM room: a column of song titles over the extras' background; a
/// title plays its song (looping), marked while it plays. The song goes
/// on after the room is left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicRoom {
    pub header: Placed,
    /// Back to the extras menu; two cells: normal, under the pointer.
    pub exit: Placed,
    pub songs: Vec<MusicSong>,
    /// The room's own background picture; `None` keeps the extras menu's.
    pub background: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicSong {
    /// Two cells: normal, under the pointer.
    pub button: Placed,
    /// The same while the song plays (same place).
    pub playing: String,
    pub file: String,
    /// Left out until [`Op::Progress`] with this bit was reached.
    pub progress: Option<u32>,
}

/// The movie room: a thumbnail per movie, which plays it. A movie is
/// shown once an [`Op::Movie`] has played its file (or always, when
/// `open`); until then the background shows through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovieRoom {
    /// Back to the extras menu; two cells: normal, under the pointer.
    pub exit: Placed,
    pub movies: Vec<RoomMovie>,
    /// The room's own background picture; `None` keeps the extras menu's.
    pub background: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMovie {
    /// Two cells: normal, under the pointer.
    pub button: Placed,
    pub file: String,
    pub open: bool,
}

/// A settings screen: rows of levels 0-10 changed with minus/plus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsMenu {
    /// Full-screen layer drawn with `base_opacity` over the screen.
    pub base: Placed,
    pub base_opacity: u8,
    pub header: Placed,
    /// Three cells: normal, under the pointer, disabled.
    pub close: Placed,
    pub rows: Vec<SettingRow>,
    pub label_x: i32,
    pub minus_x: i32,
    pub level_x: i32,
    pub plus_x: i32,
    pub y: i32,
    pub pitch: i32,
    pub font_size: u32,
    /// "Back to the title" and "quit" (from the game only), their labels
    /// at this height.
    pub title_label: String,
    pub quit_label: String,
    pub exits_y: i32,
    /// Page tabs (levels, voices) at `tabs_y`, from `tabs_x`; the voice
    /// page is left out without `voices`.
    /// Page tabs: levels, voices, switches (a page without content is
    /// left out at the end).
    pub tab_labels: Vec<String>,
    pub tabs_x: i32,
    pub tabs_y: i32,
    /// Two columns at `label_x` and `voice_column_x`, from `y`.
    pub voices: Vec<VoiceToggle>,
    pub voice_column_x: i32,
    pub voice_pitch: i32,
    /// A switch under the levels: skip mode goes on through text not read
    /// before (off: it stops there). `None`: skip skips everything.
    pub skip_unread_label: Option<String>,
    /// On/off options on their own page (all on by default).
    pub switches: Vec<SettingSwitch>,
}

/// One on/off option of [`SettingsMenu::switches`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingSwitch {
    pub label: String,
    pub switch: Switch,
}

/// What a [`SettingSwitch`] turns on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// The mouse wheel (down) reads on at a text wait.
    WheelReadsOn,
    /// Wheel down on the newest backlog page closes the backlog.
    WheelClosesLog,
    /// Ask before saving over a slot.
    ConfirmSave,
    ConfirmLoad,
    ConfirmDelete,
    /// Ask before going back to the title.
    ConfirmTitle,
    ConfirmQuit,
    /// Ask before going back to the last choice ([`BarAction::BackToChoice`]).
    ConfirmBackToChoice,
}

/// A voiced line's clip and whose voice it is (for the per-character
/// voice switches, [`VoiceToggle`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    pub file: String,
    pub character: u32,
}

/// A switch on the settings screen's voice page: voices of `characters`
/// play only while it is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceToggle {
    pub label: String,
    pub characters: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingRow {
    pub label: String,
    pub setting: Setting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    BgmVolume,
    SeVolume,
    VoiceVolume,
    TextSpeed,
    AutoSpeed,
    /// The text window's transparency (0 = as drawn, 10 = nearly gone).
    WindowTransparency,
}

/// The text history: past pages stacked from the bottom of `area` up,
/// scrolled a page at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlog {
    pub base: Placed,
    pub base_opacity: u8,
    pub frame: Placed,
    pub track: Placed,
    /// Three cells each: normal, under the pointer, disabled.
    pub up: Placed,
    pub down: Placed,
    pub close: Placed,
    pub area: Rect,
    pub font_size: u32,
    pub line_height: i32,
    pub columns: u32,
}

/// A yes/no question over whatever is on screen, asked before saving over
/// a save, loading, deleting, going back to the title and quitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmDialog {
    /// Base and frame, centred on the screen.
    pub base: Placed,
    /// Three cells each: normal, under the pointer, disabled.
    pub yes: Placed,
    pub no: Placed,
    pub yes_label: String,
    pub no_label: String,
    /// Message baseline offset from the top of `base`.
    pub text_y: i32,
    pub font_size: u32,
    /// After "No.NN " (the slot).
    pub overwrite: String,
    pub load: String,
    pub delete: String,
    pub title: String,
    pub quit: String,
    /// Questions drawn as pictures (frame and text in one, at their
    /// place), each shown in place of `base` and the text. A dialog with
    /// pictures draws no button labels (the buttons carry their own).
    pub pictures: Vec<(Switch, Placed)>,
    /// A picture over the whole screen under the dialog (black, to dim
    /// it), and its opacity (0-255); with `pictures` only.
    pub backdrop: Option<(Placed, u8)>,
    /// `yes` and `no` have a fourth cell, shown while the mouse button is
    /// held on them.
    pub pressed: bool,
    /// The dialog fades in over this long (0 = at once).
    pub fade_ms: u32,
}

/// A screen of its own drawn over the scene, the text window hidden:
/// `pictures` from the bottom up, then `buttons` (two-cell strips:
/// normal, under the mouse), each with the label to continue at. It fades
/// in and, once a button is clicked, out over `fade_ms`, showing the scene
/// again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonScreen {
    pub pictures: Vec<Placed>,
    pub buttons: Vec<(Placed, String)>,
    pub fade_ms: u32,
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
}

/// A menu of commands opened over the game at a click wait, by a
/// right-click or its key: a column of picture buttons over a full-screen
/// backdrop. Needs a [`ButtonBar`] (it runs the bar's actions).
///
/// Opening, the backdrop fades in over `fade_ms`, then the items appear
/// one by one from the top, `step_ms` apart; closing, they go one by one
/// from the bottom, then the backdrop fades out. It closes on a
/// right-click, its key, [`BarAction::Close`], or after an action that
/// goes on with the game (auto, skip, fast skip, hide the window);
/// screens it opens come back to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMenu {
    pub backdrop: Placed,
    pub items: Vec<CommandItem>,
    pub fade_ms: u32,
    pub step_ms: u32,
    /// Upper-case letter that opens and closes it at a click wait.
    pub key: Option<char>,
    /// Items have a fourth cell, shown while the mouse button is held on
    /// them.
    pub pressed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandItem {
    /// Three cells side by side: normal, under the pointer, disabled (a
    /// fourth, pressed, with [`CommandMenu::pressed`]).
    pub button: Placed,
    pub action: BarAction,
    /// Upper-case letter that runs the action at a click wait, the menu
    /// open or not.
    pub key: Option<char>,
    /// The subroutine [`BarAction::Call`] runs.
    pub label: Option<String>,
}
