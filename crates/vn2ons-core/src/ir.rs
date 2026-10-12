//! The intermediate representation: what a scene does, independent of the
//! source engine and of NScripter syntax.
//!
//! Assets are referred to by their path inside the output directory (forward
//! slashes, e.g. `bg/bg20d1.png`); the engine side is responsible for writing
//! those files.

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
    /// Hide the text window; the next text page shows it again.
    HideTextWindow,
    /// Start collecting screen changes (background, sprites) without showing
    /// them; [`Op::EndTransition`] (or the next text page) reveals them all
    /// at once with a `fade_ms` cross-fade, shaped by `mask` (a greyscale
    /// image; dark parts change first) if given. `effect` names the original
    /// transition.
    BeginTransition { effect: String, fade_ms: u32, mask: Option<String> },
    /// Reveal the changes since [`Op::BeginTransition`].
    EndTransition,
    /// Show or remove sprites on some layers, then show the result.
    Sprites { changes: Vec<SpriteChange>, fade_ms: u32 },
    /// One page of text, shown and then waited on. `text` may contain '\n'
    /// for line breaks. `links` are the words the original game made
    /// clickable.
    /// `continues`: the next page is added below this one after a click
    /// instead of replacing it (full-screen text modes).
    Text { speaker: Option<String>, text: String, voice: Option<String>, links: Vec<TextLink>, continues: bool },
    /// Switch to another text window layout; the page starts empty.
    SetWindow(TextWindow),
    BgmPlay { file: String, fade_in_ms: u32 },
    BgmStop { fade_out_ms: u32 },
    /// Pause; a click ends it early.
    Wait { ms: u32 },
    SePlay { file: String, looped: bool },
    /// `file: None` stops every sound effect.
    SeStop { file: Option<String> },
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonBar {
    pub buttons: Vec<BarButton>,
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
    /// Hide the text window until the next click.
    HideWindow,
    /// Shown but not clickable (the menu it opens isn't converted yet).
    Unavailable,
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
    pub ops: Vec<Op>,
}
