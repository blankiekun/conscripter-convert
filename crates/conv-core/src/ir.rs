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
    /// `moving` travels one screen width or height in direction `dx`, `dy`
    /// (each -1, 0 or 1; +y is down).
    Slide { moving: SlideLayer, dx: i8, dy: i8 },
    /// The old picture breaks into growing blocks, then the new one
    /// sharpens from them.
    Mosaic,
    /// Zooms about the screen centre, up to `max_percent` (100 = no zoom),
    /// eased by the progress to the power `curve`.
    Zoom { style: ZoomStyle, max_percent: u32, curve: u8 },
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// rippled picture lies over the plain one at `opacity` (0-256). Each
    /// step turns the angle by `angle_step` and moves the phase by
    /// `phase_step`.
    Ripple { map: String, time_ms: u32, interval_ms: u32, opacity: (i32, i32), amplitude: (i32, i32), angle: i32, angle_step: i32, phase_step: i32 },
}

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
    /// Jump to `label` if `var == value`.
    IfEqual { var: Var, value: i32, label: String },
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
    /// Start (or with `None` stop) a moving effect on the picture behind
    /// the text window in `slot` (1-4); slot 0 with `None` stops them all.
    /// It runs by itself while the script goes on. Shown at once unless a
    /// transition is open.
    ScreenEffect { slot: u32, effect: Option<ScreenEffect> },
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
    /// A new chapter starts (shown in the top bar and save slots).
    Chapter { title: String },
    /// Something the converter can't express yet; kept for the report.
    Unsupported { what: String, at: SourceLoc },
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleScreen {
    /// Full-screen picture (logo included).
    pub background: String,
    pub fade_ms: u32,
    pub buttons: Vec<TitleButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleButton {
    /// Two cells side by side, normal and under the mouse pointer; one cell
    /// for [`TitleAction::Unavailable`].
    pub image: String,
    pub x: i32,
    pub y: i32,
    pub action: TitleAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleAction {
    /// Run the script from its first op.
    Start,
    Load,
    Settings,
    Exit,
    /// Shown but not clickable (the menu it opens isn't converted yet).
    Unavailable,
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
///
/// The bars hide until the pointer comes near: over `reveal` (transparent,
/// along the screen edge) this bar appears, over `hide` (the rest of the
/// screen) all bars go away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonBar {
    pub buttons: Vec<BarButton>,
    pub reveal: Placed,
    pub hide: Placed,
    /// Bars along the top edge, shown and hidden the same way.
    pub top: Option<TopBars>,
}

/// The current chapter and the current link ("blog") scene, along the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopBars {
    pub reveal: Placed,
    pub chapter_bar: Placed,
    pub chapter_text: TextSpot,
    pub scene_bar: Placed,
    pub scene_text: TextSpot,
    /// Shown as the scene outside any link scene.
    pub root_scene: String,
    /// Images shown as they are (frames, controls that aren't converted).
    pub decorations: Vec<Placed>,
    /// Link scene depth, "current / deepest": ten digit images 0-9 and
    /// where the two numbers go (top-left corners).
    pub digits: Vec<String>,
    pub depth_at: (i32, i32),
    pub max_depth_at: (i32, i32),
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
    pub tab_labels: [String; 2],
    pub tabs_x: i32,
    pub tabs_y: i32,
    /// Two columns at `label_x` and `voice_column_x`, from `y`.
    pub voices: Vec<VoiceToggle>,
    pub voice_column_x: i32,
    pub voice_pitch: i32,
    /// A switch under the levels: skip mode goes on through text not read
    /// before (off: it stops there). `None`: skip skips everything.
    pub skip_unread_label: Option<String>,
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
}

#[derive(Debug, Clone, Default)]
pub struct Script {
    /// Window caption.
    pub title: String,
    pub width: u32,
    pub height: u32,
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
    pub ops: Vec<Op>,
}
