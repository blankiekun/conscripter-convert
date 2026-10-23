//! The screens most games share: the text window, the title, the classic
//! menus (save/load, settings, backlog, extras), dialogs and button
//! screens.

use super::*;

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
    /// Where the click-wait icon ([`Script::wait_icon`]) shows, on the
    /// screen; `None`: after the text, where the engine puts it.
    pub wait_icon_at: Option<(i32, i32)>,
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
    /// Start's fade to black, when it differs from `fade_ms` (0 = `fade_ms`).
    pub start_fade_ms: u32,
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
    /// A picture over the background, under the buttons, glowing in and
    /// out by itself (装甲悪鬼村正's fire).
    pub glow: Option<TitleGlow>,
    /// How the title comes up: the first opening whose `at_launch` fits
    /// (at the first title of a run, or on coming back to it) plays before
    /// the buttons show. None fits: the title fades in.
    pub openings: Vec<TitleOpening>,
    /// Continue and Load are drawn only while there is a save (in any
    /// slot, or the quick save); Continue loads the quick save, else the
    /// save screen's newest, else the first save found (the English
    /// Muramasa port's, which loads its newest).
    pub hide_without_saves: bool,
    /// Extras is drawn only while one of these isn't 0 (a game's flags:
    /// either route cleared). Empty: as its [`TitleButton::shown_by`].
    pub extras_shown_by: Vec<Var>,
    /// Looping movies over `background`, under the rest (not with a
    /// `glow`): the first whose `when` holds plays, with its music in
    /// place of `bgm` (月姫's titlebg1–4, by the scenes read).
    pub movies: Vec<TitleMovie>,
    /// Pictures over the movie, under the buttons, the first at the bottom.
    pub pictures: Vec<TitlePicture>,
    /// Continue is drawn only while there is a save to load (found as for
    /// `hide_without_saves`, Load staying), and while it isn't the other
    /// buttons move down by this many pixels (negative: up; 月姫 centres
    /// its menu on the buttons shown).
    pub continue_hides: Option<i32>,
    /// Start fades to white instead of black.
    pub start_white: bool,
    /// Buttons and pictures moving by themselves at rest (続・殺戮のジャンゴ's
    /// buttons jolting now and then, its tumbleweed rolling by).
    pub motions: Vec<TitleMotion>,
}

impl TitleScreen {
    /// Start's fade to black.
    pub fn start_fade(&self) -> u32 {
        if self.start_fade_ms == 0 { self.fade_ms } else { self.start_fade_ms }
    }
}

/// [`TitleScreen::openings`]: a silent movie of the title coming up, ending
/// on the title as it rests (buttons included), with the title music
/// started under it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleOpening {
    pub movie: String,
    /// Only at the first title after the game starts (else only on coming
    /// back to the title).
    pub at_launch: bool,
    /// Started with the movie (a sound effect with silence before it).
    pub sound: Option<String>,
    /// [`TitleGlow::keys`] for the glow's start after this opening (it
    /// may be partway through a glow).
    pub glow_keys: Option<Vec<(u32, u32)>>,
    /// Plays only while each of these variables has its value (a game's
    /// flags: the routes cleared).
    pub when: Vec<(Var, i32)>,
    /// Set to 1 when it plays.
    pub sets: Vec<Var>,
    /// One of a pool: the openings marked so whose `when` holds are tried
    /// together, where the first of them stands, one picked at random.
    pub random: bool,    /// Only on coming back from the [`Script::picture_extras`] (instead of
    /// [`TitleOpening::at_launch`]'s choice).
    pub after_extras: bool,
    /// Drawn by this subroutine of the script instead of `movie`: from a
    /// black screen, with sprites of its own, which the title's take the
    /// place of at once as it returns (on the title as it rests); the
    /// music isn't started again when the boot left it playing.
    pub script: Option<String>,
    /// With `script`: these [`TitleScreen::motions`] go on from as far as
    /// the motion on the opening's layer (an [`Op::SpriteMotion2`] one)
    /// has run when the title takes over, not from their start.
    pub clocks: Vec<(TitleSprite, u32)>,
}

/// One of [`TitleScreen::buttons`] or [`TitleScreen::pictures`], by its index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleSprite {
    Button(usize),
    Picture(usize),
}

/// [`TitleScreen::motions`]: `keys` from the sprite's place, from its
/// first draw, looping from key `from_key` on (0: all of them; a lead-in
/// before it). The keys' `ease` is unused (straight).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleMotion {
    pub sprite: TitleSprite,
    pub keys: Vec<MotionKey>,
    pub from_key: usize,
}

/// [`TitleScreen::glow`]: a full-screen picture drawn at a changing
/// opacity, looping along `keys` (ms from the start, opacity 0-256).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleGlow {
    pub image: String,
    pub keys: Vec<(u32, u32)>,
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
    /// Drawn and clickable only while this variable isn't 0 (a game's
    /// flag: 続・殺戮のジャンゴ's EXTRA once the story is finished).
    pub shown_by: Option<Var>,
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
    /// The [`Script::picture_extras`].
    Extras,
    /// Open this web page in the browser (`openurl`): 装甲悪鬼村正's
    /// ライナー画像, its staff-comment page.
    Link(&'static str),
}

/// [`TitleScreen::movies`]: a full-screen movie, looping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleMovie {
    pub movie: String,
    /// Music while it plays, in place of [`TitleScreen::bgm`].
    pub bgm: Option<String>,
    /// Chosen only while each of these variables has its value.
    pub when: Vec<(Var, i32)>,
}

/// [`TitleScreen::pictures`]: a picture with alpha, its top-left corner
/// at `x`, `y`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitlePicture {
    pub image: String,
    pub x: i32,
    pub y: i32,
    /// Its colours added to what is under it.
    pub add: bool,
    /// Its opacity along these keys (ms from the title's first draw,
    /// opacity 0-256), held at the last; empty: shown as it is.
    pub keys: Vec<(u32, u32)>,
    /// The keys loop.
    pub repeat: bool,
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
    /// Room for a row's text lines in pixels, past which they are cut
    /// (0: 40 characters).
    pub text_width: u32,
    /// A screen filter over the scene while the menu is open in game: the
    /// `screenfilter` slot (one the game leaves free then), then kind,
    /// blend, strength and the three parameters. It is off while a save is
    /// written, since saves keep the screen filters.
    pub backdrop: Option<(u32, [i32; 6])>,
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
    /// Shown over the dimmed screen after a quick save, until a click.
    pub quick_saved: Option<Placed>,
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
    /// The title's colour on rows at or above the current level, and on
    /// the deeper ones (left by going back).
    pub row_color: String,
    pub deeper_row_color: String,
    /// Opened by clicking the chapter bar; `None`: the bar isn't a button.
    pub chapter_history: Option<ChapterHistory>,
}

/// The chapters reached since the history was last cleared
/// ([`Op::ChapterHistoryClear`]), listed in columns over the scene: each
/// [`Op::Chapter`] keeps the game as it is there, and choosing a chapter
/// (after a yes/no question) goes back to it; the chapters after it stay
/// listed until another one is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterHistory {
    /// Full-screen pictures, `base` drawn at `base_opacity` (0-255).
    pub base: Placed,
    pub base_opacity: u8,
    pub frame: Placed,
    /// Three cells: normal, under the pointer, disabled.
    pub close: Placed,
    /// A row's look under the pointer, at the row's place.
    pub hover: Placed,
    /// Marks the current chapter's row, at its offset (`x`, `y`) from the
    /// row's place.
    pub current: Placed,
    /// Rows: the first column's first row at (`x`, `y`), then `row_pitch`
    /// down and `column_pitch` across, `rows` to a column, each
    /// `row_width` wide; the title starts at `text_dx`, `text_dy` in it.
    pub x: i32,
    pub y: i32,
    pub row_pitch: i32,
    pub column_pitch: i32,
    pub rows: u32,
    pub columns: u32,
    pub row_width: i32,
    pub text_dx: i32,
    pub text_dy: i32,
    pub font_size: u32,
    /// The question asked before going back, around the chapter's title.
    pub question: (String, String),
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
    /// ([`CommandItem::label`]); the background, the game's sprites and the
    /// music are put back after.
    Call,
    /// Unpins the bars (the button's `active` look) or pins them again
    /// (the default): unpinned, the top bars stay hidden and the button
    /// bar shows only while the pointer is below `reveal_y`, fading in
    /// over `fade_ms` when the pointer comes down to it.
    Lock { reveal_y: i32, fade_ms: u32 },
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
    /// The menu's own picture when it isn't the modes' `background`
    /// (俺たちに翼はない lists the modes over the title); `None` uses
    /// `background`.
    pub menu_background: Option<String>,
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
    /// Buttons for the songs before and after, play/stop and the volume.
    pub player: Option<MusicPlayer>,
}

/// A [`MusicRoom`]'s player bar. Two cells each (at rest, under the
/// pointer): `previous` and `next` play the song before or after the one
/// playing (none past the ends, nothing while none plays), `play` stops
/// the song playing or plays the last one again (the first at first),
/// shown as `playing` while a song plays. The room has its own music
/// volume, 0-100, from the settings' (100 without settings) and put back
/// when the room is left: `lower` and `raise` change it by 10, a click on
/// `bar` sets it by where the click is along it; `knob` shows it, its
/// left edge at `knob.x` + volume × `knob_travel` / 100.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicPlayer {
    pub previous: Placed,
    pub play: Placed,
    pub playing: String,
    pub next: Placed,
    pub lower: Placed,
    pub raise: Placed,
    pub bar: Rect,
    pub knob: Placed,
    pub knob_travel: i32,
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
    /// Draw the screen from pictures instead (see [`PagedSettings`]);
    /// the fields above still give the settings' defaults and what they
    /// apply.
    pub pictures: Option<Box<PagedSettings>>,
    /// Levels (0-10) to start from, and to go back to with the defaults
    /// button, where they differ from the shared ones (music and sound
    /// effects 8, voices 10, text speed 7, auto speed 5, window 0).
    pub defaults: Vec<(Setting, u32)>,
    /// Switches that start off (and go back off with the defaults button).
    pub switches_off: Vec<Switch>,
    /// Milliseconds a character for text speed levels 0-10, where the
    /// game's speeds differ from the shared `(10 - level) × 6`.
    pub text_ms: Option<[u32; 11]>,
}

/// A settings screen made of a game's own pictures: pages switched with
/// `prev`/`next`, each a background with its rows' words drawn in and a
/// control per row. Every [`Placed`] button has two cells: normal, under
/// the pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagedSettings {
    pub pages: Vec<SettingsPage>,
    pub prev: Placed,
    pub next: Placed,
    pub close: Placed,
    /// Puts every setting back to its default.
    pub defaults: Placed,
    /// Back to the title and quit, shown from the game only (`None`: not
    /// on this screen).
    pub title: Option<Placed>,
    pub quit: Option<Placed>,
    /// Page tabs: each shows its first page and is drawn lit on its pages.
    /// With tabs, `prev` and `next` turn only between the pages of one tab,
    /// not round (`prev` left out on its first page, `next` on its last).
    pub tabs: Vec<PageTab>,
}

/// One page of [`PagedSettings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPage {
    /// Full screen, the rows' labels drawn in.
    pub base: Placed,
    pub rows: Vec<RowControl>,
}

/// A row's control on a [`SettingsPage`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowControl {
    /// A level 0-10 on the page's bar: `knob` (one cell, placed where it
    /// is at level 0) moves `travel` pixels from level 0 to 10, dragged or
    /// stepped by the buttons lowering and raising it by one. `fill` is
    /// shown from the level-0 end of the bar to the knob's centre, over
    /// `bar` (the bar as it is at level 0 where the page differs).
    Slider { value: SettingValue, knob: Placed, travel: i32, fill: Placed, bar: Option<Placed>, minus: Placed, plus: Placed },
    /// Options side by side: (the value it sets, its button, the picture
    /// shown while it is the current one: its second cell twice).
    Choice { value: SettingValue, options: Vec<(i32, Placed, String)> },
    /// A check box: `button` (two cells: normal, under the pointer) flips
    /// the value between `on` and the other (`1 - on`); `checked` (its
    /// second cell twice) is shown while the value is `on`.
    Toggle { value: SettingValue, on: i32, button: Placed, checked: String },
}

/// What a [`RowControl`] sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingValue {
    /// A level 0-10.
    Level(Setting),
    /// 1 on, 0 off.
    Switch(Switch),
    /// Skip goes on through text not read before: 1 yes.
    SkipUnread,
    /// The voices of all these characters: 1 on, 0 off.
    Voices(Vec<u32>),
    /// The language, in the game's global variable: 0 Japanese, 1 English.
    /// Picking the other in game closes the settings with 3 (a
    /// [`SystemPage::Settings`] then goes to its label, which switches).
    Language(Var),
}

/// A tab of [`PagedSettings`]: `button` (two cells: normal, under the
/// pointer) shows page `pages.0`; the picture `selected` (one cell) is
/// drawn in its place on pages `pages.0` to `pages.1` (0 up).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTab {
    pub button: Placed,
    pub selected: String,
    pub pages: (usize, usize),
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
    /// A voice goes on playing after the click that reads on, until the
    /// next voice (off: the click stops it).
    VoiceKeepsPlaying,
    /// The music, the sound effects, the voices are heard (off: muted,
    /// whatever their level).
    BgmOn,
    SeOn,
    VoiceOn,
    /// Ask before putting every setting back to its default.
    ConfirmReset,
    /// Screen effects are drawn (off: every transition shows at once).
    Effects,
    /// The save menu shows the selected save's picture at full size.
    SavePreview,
    /// The message text has its shadow.
    TextShadow,
    /// Ask before going back to a chapter of the chapter history
    /// ([`ChapterHistory`]).
    ConfirmChapter,
    /// A scene already read offers to be skipped (the MAGES scene jump
    /// dialog); off: it just plays.
    SceneSkipDialog,
    /// Skip and auto mode stop at a choice; off: they go on after it.
    StopSkipAtChoice,
    /// The music keeps its volume while a voice plays; off: it is lowered.
    BgmFullForVoice,
    /// Ask before a quick load ([`BarAction::QuickLoad`]); asked only by a
    /// [`ConfirmDialog`] with a picture for it.
    ConfirmQuickLoad,
    /// A click (or key) during auto mode ends it; off: auto mode goes on
    /// and the click reads on (the auto button still ends it).
    ClickStopsAuto,
    /// A click (or key) during skip ends it; off: only the skip button does.
    ClickStopsSkip,
    /// Auto mode stops at a choice (a select, a page of links); off: it
    /// goes on past it.
    ChoiceStopsAuto,
    /// Skip stops at a choice; off: it goes on past it.
    ChoiceStopsSkip,
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
    /// Auto mode's speed, as levels like [`Setting::AutoSpeed`] (the same
    /// level), for a game whose auto wait is a time per page plus a time
    /// per character: at level v it waits (10 - v) × `page_step` ms plus
    /// (10 - v) × `char_step` ms for each character of the page.
    AutoWait { page_step: u32, char_step: u32 },
    /// The menus' sounds ([`Script::menu_sounds`]).
    SystemVolume,
    /// Auto mode's time per character as a row of its own, beside an
    /// [`Setting::AutoWait`] row (whose `char_step` it replaces): at level
    /// v, (10 - v) × `step` ms for each character of the page.
    AutoCharWait { step: u32 },
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
    /// With `pictures`: the question and its buttons also unroll sideways.
    pub unroll: Option<Unroll>,
}

/// [`ConfirmDialog::unroll`]: the question and its buttons stretch
/// sideways out of the vertical line at `x` as they fade in (over the
/// dialog's `fade_ms`, eased out), and back into it as they go, fading out
/// over `close_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unroll {
    pub x: i32,
    pub close_ms: u32,
    /// The question and buttons stretch up and down out of the horizontal
    /// line at this y instead (`x` is then unused).
    pub y: Option<i32>,
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

/// A screen drawn over the scene and its text window until a click (left
/// or right): pictures on numbered layers, the higher drawn on top.
/// A picture with a condition is shown only while its variable compares
/// with the value; of the pictures on one layer the last one shown wins.
/// It fades in and out over `fade_ms`; the scene's own pictures stay as
/// they are under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureScreen {
    pub pictures: Vec<ScreenPicture>,
    pub fade_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenPicture {
    pub layer: u32,
    pub picture: Placed,
    pub when: Option<(Var, Compare, i32)>,
}

/// A system menu the engine draws (`screen "system1"`…): a bar of tabs
/// beside a panel. Each tab is a screen of its own with the same bar,
/// its own tab shown current; a click on another tab cross-fades to it.
/// Right-click closes it. `*conv_save` and `*conv_load` open it on its
/// save and load tabs (the title's Load, the game's keys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemMenu {
    /// Drawn on every tab, the first at the bottom (the panel, the bar).
    pub backdrop: Vec<Placed>,
    pub tabs: Vec<SystemTab>,
    /// The cross-fade on opening, closing and between tabs.
    pub fade_ms: u32,
}

/// [`SystemMenu::tabs`]: its button and what it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemTab {
    /// Three cells side by side: at rest, under the pointer, current.
    pub button: Placed,
    pub page: SystemPage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemPage {
    /// In game only: shown at rest on the title.
    Save(SaveList),
    Load(SaveList),
    /// Back to the title (asking first with the [`ConfirmDialog`]'s
    /// title question, when there is one), in game only: shown at rest
    /// on the title.
    Title,
    /// Drawn, not clickable (a page not converted yet).
    Unavailable,
    /// In game only: closes the menu and goes to the game's label (the
    /// wait the menu was opened from is left); shown at rest on the title.
    Jump(String),
    /// Opens the settings screen, back to the menu after it; when the
    /// settings close on a language pick in game ([`SettingValue::Language`])
    /// the menu closes and goes to the label, as [`SystemPage::Jump`].
    Settings(Option<String>),
}

/// A [`SystemPage`]'s save slots: one column of `rows` rows a page, slots
/// 1 to `slots`, the pages turned by the mouse wheel and Left/Right (no
/// page buttons). A click saves or loads (asking first over a save with
/// the [`ConfirmDialog`]'s questions); the slot's text is "chapter|last
/// line" (the chapter from [`Op::Chapter`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveList {
    pub slots: u32,
    pub rows: u32,
    /// The first row's top left, and the step to the next.
    pub x: i32,
    pub y: i32,
    pub row_step: i32,
    /// A row's picture at the row's place (`x`, `y` from it), and in its
    /// place while the pointer is on it; both under the thumbnail.
    pub row: Placed,
    pub row_hover: Placed,
    /// The rest relative to the row's top left.
    pub thumb: Rect,
    pub date: GridText,
    pub chapter: GridText,
    pub line: GridText,
    /// The slot under the pointer's thumbnail, larger (absolute).
    pub preview: Option<Rect>,
}

/// One step of [`Script::boot`]: a full-screen picture faded in from
/// `backdrop` and out to it, a movie, or (no `image`) the backdrop alone
/// for `hold_ms`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootStep {
    /// The picture, or the movie when `movie`; empty: the backdrop alone,
    /// held for `hold_ms` with no fades.
    pub image: String,
    pub movie: bool,
    /// The colour behind the picture (`"white"`, `"black"`, `"#RRGGBB"`);
    /// after a movie, the screen held for `hold_ms`.
    pub backdrop: String,
    pub fade_in_ms: u32,
    pub hold_ms: u32,
    pub fade_out_ms: u32,
    /// The colour the picture fades out to; `None`: the backdrop.
    pub fade_out_to: Option<String>,
    /// A click ends the hold (a movie: the movie) once this many ms of it
    /// have passed; `None`: nothing skips it.
    pub skip_after_ms: Option<u32>,
    /// Shown only while each of these variables has its value (a game's
    /// flags, its language); empty: always.
    pub when: Vec<(Var, i32)>,
    /// The hold lasts until a click (or key), not `hold_ms`.
    pub until_click: bool,
    /// A click cuts the fades short (else they always run their time).
    pub click_skips_fades: bool,
    /// A label in [`Script::ops`] gosubbed in place of a picture, once a
    /// run (not again on coming back to the title): the game's boot
    /// compiled from its own script, ending in [`Op::Return`]. The title's
    /// music, when it leaves it playing, goes on into the title.
    pub gosub: Option<String>,
}
