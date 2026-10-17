//! Screens drawn from a game's own pictures (Nitroplus's): the save grid,
//! the line log, the command menu, picture settings and picture extras;
//! the menus' sounds.

use super::*;

/// The menus' sounds: `hover` as the pointer comes onto a button,
/// `confirm` when one is clicked. Played at their own volume,
/// [`Setting::SystemVolume`] when the settings have it, else the sound
/// effects'.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuSounds {
    pub hover: Option<String>,
    pub confirm: Option<String>,
}

/// The save and load screens as pages of thumbnails over a picture per
/// page (one set for saving, one for loading). Slots run down the columns
/// from `first`, `row_step` apart, the next column `column_step` away. The
/// slot under the pointer is framed and its details show (the date, the
/// play time, the chapter, the last line, a larger thumbnail); a click
/// saves (asking before overwriting, then a notice until a click), loads
/// (asking), or in delete mode deletes (asking). Previous and next page go
/// round; a right-click or `close` leaves. It cross-fades in and out, and
/// from page to page, over `fade_ms`.
///
/// Buttons are strips of four cells: normal, under the pointer, disabled,
/// pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveGrid {
    pub save_pages: Vec<String>,
    pub load_pages: Vec<String>,
    /// Over the thumbnails (their frames), full screen.
    pub frames: Option<Placed>,
    pub first: (i32, i32),
    pub row_step: i32,
    pub column_step: i32,
    pub rows: u32,
    pub columns: u32,
    /// The thumbnails' size (the screen scaled down).
    pub thumb_w: u32,
    pub thumb_h: u32,
    /// Around the slot under the pointer, `offset` from its top left.
    pub hover: Placed,
    pub hover_offset: (i32, i32),
    /// On the newest save, `offset` from its top left.
    pub new_mark: Placed,
    pub new_offset: (i32, i32),
    pub previous: Placed,
    pub next: Placed,
    /// Toggles delete mode; `delete_on` is shown over it while it is on.
    pub delete: Placed,
    pub delete_on: Placed,
    pub close: Placed,
    /// The slot under the pointer's thumbnail, larger, filling this box.
    pub preview: Rect,
    pub date: GridText,
    pub play_time: GridText,
    pub chapter: GridText,
    pub line: GridText,
    /// Shown over a dimmed screen after saving, until a click.
    pub saved: Option<Placed>,
    pub fade_ms: u32,
    /// Slots run along the rows instead, left to right then down:
    /// `column_step` apart in a row, the next row `row_step` below.
    pub across: bool,
    /// Tabs that pick a page each (strips: at rest, under the pointer,
    /// current, pressed); the current page's tab shows its third cell and
    /// is not a button. Empty: previous and next only.
    pub tabs: Vec<Placed>,
    /// A click on a slot opens this window by it instead of acting at
    /// once: one screen for saving and loading.
    pub popup: Option<SlotPopup>,
    /// In a slot with no save, `x`,`y` from its top left.
    pub empty: Option<Placed>,
    /// The last page shows the quick save in its first slot (the rest
    /// empty), loadable but not saved over or locked.
    pub quick_page: bool,
    /// Put before the play time.
    pub play_time_prefix: String,
    /// The play time as hours and minutes, two digits each (up to 99:59),
    /// instead of hours (three digits), minutes and seconds.
    pub play_time_hm: bool,
    /// Drawn by the engine (CONscripter's `screen`, `screens/save.lua` and
    /// `load.lua`) rather than by script loops.
    pub engine_screen: bool,
}

/// A text of [`SaveGrid`]: its box's top left (for vertical text, its top
/// right column's left edge), the size of its characters, how many lines
/// of how many characters, the gaps, the colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridText {
    pub x: i32,
    pub y: i32,
    pub font_size: u32,
    pub lines: u32,
    pub chars: u32,
    pub line_gap: u32,
    /// Between characters (may be negative).
    pub char_gap: i32,
    pub colour: String,
    /// In columns, top to bottom, from the right.
    pub vertical: bool,
}

/// A menu of commands opened over the game at a click wait, by a
/// right-click or its key: a column of picture buttons over a full-screen
/// backdrop. Needs a [`ButtonBar`] (it runs the bar's actions).
///
/// Opening, the backdrop fades in over `fade_ms`, then the items appear
/// one by one from the top, `step_ms` apart; closing, they go one by one
/// from the bottom, then the backdrop fades out. With `step_ms` 0 the
/// items fade in and out with the backdrop. It closes on a
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

/// The text history as rows scrolled one at a time ([`Script::line_log`]),
/// over `background`: up to `rows` rows from (`text_x`, `text_y`),
/// `row_step` apart, each page's lines cut at `columns` characters, ruby
/// as its base text; the newest row at the bottom once they fill the
/// screen. The first row of a voiced page has `voice_icon` (a button
/// replaying the voice) at its x and the row's y plus its y. The wheel and
/// `up`/`down` scroll a row; wheel-down at the newest row, `close` and a
/// right-click close it. `knob` shows where the rows shown are, from
/// `knob_y.0` (the oldest) to `knob_y.1` (the newest), and can be dragged.
/// It fades in and out over `fade_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineLog {
    pub background: Placed,
    /// Four cells each: normal, under the pointer, disabled, pressed.
    pub up: Placed,
    pub down: Placed,
    pub close: Placed,
    /// Two cells: normal, under the pointer (or dragged).
    pub knob: Placed,
    pub knob_y: (i32, i32),
    /// Three cells: normal, under the pointer, pressed.
    pub voice_icon: Placed,
    pub text_x: i32,
    pub text_y: i32,
    pub row_step: i32,
    pub rows: u32,
    pub columns: u32,
    pub font_size: u32,
    pub colour: String,
    pub fade_ms: u32,
    /// A 1 px outline of this colour (`#RRGGBB`) around the text's letters.
    pub outline: Option<String>,
    /// Rows this many pixels wide instead of `columns` cells (proportional
    /// text in a field of its own width).
    pub row_width: Option<u32>,
    /// Drawn by the engine (CONscripter's `screen`, `screens/backlog.lua`)
    /// rather than by script loops.
    pub engine_screen: bool,
}

/// An animated click-wait icon ([`Script::wait_icon`]): pictures of `w`×`h`
/// side by side in `image`, picture k shown `frame_ms[k]`, looped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitIcon {
    pub image: String,
    pub frame_ms: Vec<u32>,
    pub w: u32,
    pub h: u32,
}

/// A settings screen drawn as one picture with its controls laid over it
/// (Nitroplus's コンフィグ): vertical sliders, pairs of on/off pictures and
/// a few buttons. The picture shows every control at rest; a control shows
/// its own picture only under the pointer, and a pair its chosen side.
/// Values are globals, applied to the engine at startup and as they change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureSettings {
    pub background: Placed,
    /// Pictures shown over the background as they are (options left out,
    /// drawn as they stand by default).
    pub fixed: Vec<Placed>,
    /// Three cells each: at rest (usually empty), under the pointer,
    /// pressed. `close` leaves; `reset` puts every setting back to its
    /// default after asking ([`Switch::ConfirmReset`]); `voice_test` plays
    /// [`PictureSettings::test_voice`].
    pub close: Placed,
    pub reset: Option<Placed>,
    pub voice_test: Option<Placed>,
    pub test_voice: Option<String>,
    pub sliders: Vec<PictureSlider>,
    pub pairs: Vec<PicturePair>,
    /// Lines of text shown as they are (a version line).
    pub texts: Vec<PictureText>,
    pub fade_ms: u32,
    /// A pair's chosen picture fades in (and the other out) over this.
    pub pair_fade_ms: u32,
    /// Opened by its own button: a switch for each character's voice.
    pub voice_window: Option<VoiceWindow>,
    /// Drawn by the engine (`screen "settings"`) instead of a script
    /// routine, with the pictures it needs besides.
    pub engine_screen: Option<SettingsScreen>,
}

/// The pictures [`PictureSettings`] drawn by the engine needs besides its
/// own: an engine radio shows a picture of its own while on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsScreen {
    /// Per pair, its on and off sides' chosen pictures (the third cell) in
    /// all three cells.
    pub chosen: Vec<(Placed, Placed)>,
    /// Per [`VoiceLayout`], per entry: its picture with the check over it
    /// (at the picture's place).
    pub voices_checked: Vec<Vec<String>>,
    /// Per slider, per step of a stepped one ([`SliderSteps`]): its
    /// button's chosen picture (the third cell) in all three cells; empty
    /// for a slider with a knob.
    pub steps_chosen: Vec<Vec<Placed>>,
}

/// A window of [`PictureSettings`] with a switch for each character's
/// voice (Nitroplus's 別窓を開く): the characters' pictures over a
/// background, a check mark on those heard. Its layout is the first of
/// `layouts` whose flag is set, or the last. Right-click or C closes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceWindow {
    /// Three cells: at rest (usually empty), under the pointer, pressed.
    pub open: Placed,
    /// A full-screen colour drawn at the opacity (0-255) under the background.
    pub backdrop: (Placed, u8),
    pub background: Placed,
    /// Drawn at an entry's top-left while its voices are heard.
    pub check: Placed,
    pub layouts: Vec<VoiceLayout>,
    pub fade_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceLayout {
    pub when: Option<Var>,
    pub entries: Vec<VoiceEntry>,
}

/// One character of a [`VoiceWindow`]: shown (and clickable in `hit`, at
/// the picture's top-left) once its flag `heard` is set; mutes and unmutes
/// the voices of `characters` ([`Voice::character`]) together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceEntry {
    pub picture: Placed,
    pub hit: (i32, i32),
    pub characters: Vec<u32>,
    pub heard: Option<Var>,
}

/// A vertical slider of [`PictureSettings`]: the knob runs from `knob_y.0`
/// (position 0) to `knob_y.1` (position 1000) at `knob.x`; pressing in
/// `track` puts it under the pointer and drags it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureSlider {
    pub setting: SliderSetting,
    /// Two cells: at rest, under the pointer (or dragged).
    pub knob: Placed,
    pub knob_y: (i32, i32),
    pub track: Rect,
    /// The position (0-1000) a first start and the reset give it.
    pub default: u32,
    /// Drawn as a row of buttons instead of the knob (`knob`, `knob_y`
    /// and `track` unused): see [`SliderSteps`].
    pub steps: Option<SliderSteps>,
    /// Lying: `knob_y` is the knob's x range, position 0 at its right end
    /// (what is at the top standing), the knob at `knob.y`.
    pub horizontal: bool,
    /// Lying: a bar shown from its left end to the knob's middle, and a
    /// black picture at least as big as it (cutting the rest away).
    pub fill: Option<(Placed, String)>,
}

/// A [`PictureSlider`] set by clicking one of a row of buttons (続・殺戮の
/// ジャンゴ's ten bullets). Each button has three cells (at rest, under
/// the pointer, chosen) and the position it gives. With `filled`, every
/// button up to the chosen one shows its chosen cell (a gauge); otherwise
/// only the chosen one (presets). The default must be one of the
/// positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliderSteps {
    pub buttons: Vec<(Placed, u32)>,
    pub filled: bool,
}

/// What a [`PictureSlider`] sets, at position p (0 at the top, 1000 at the
/// bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderSetting {
    /// Full at the top, silent at the bottom (the Nitroplus exe's curve,
    /// percent = 100 × (v/1000)^1.661 of v = 1000 - p, approximated).
    BgmVolume,
    SeVolume,
    VoiceVolume,
    /// `ms` per character at the bottom, none at the top.
    TextSpeed { ms: u32 },
    /// Auto mode waits `ms` per character of the page at the bottom.
    AutoWait { ms: u32 },
    /// The game's speed: level 12 - p × 12 / 1000 (1-12), each level a
    /// `consset "gamespeed"` percent (`levels[level - 1]`).
    GameSpeed { levels: [u32; 12] },
    /// Per character, with s = p/1000 and u = (1 - s)^4:
    /// 1 / (u / `fast` + (1 - u) / `slow`) ms (`fast` and `slow` in 1/10
    /// ms); none at the top. Nitroplus Diesel's typing: a rate between
    /// its fastest and slowest weighted by (1 - s)^4.
    TextSpeedCurve { fast: u32, slow: u32 },
}

/// Two pictures for one on/off option of [`PictureSettings`]: three cells
/// each (at rest, under the pointer, chosen); the chosen side shows its
/// third cell. Both at one place make a check box: only the current
/// side's picture shows (cells: at rest, under the pointer, pressed), and
/// a click on it switches to the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PicturePair {
    pub option: PairOption,
    pub on: Placed,
    pub off: Placed,
    pub default_on: bool,
}

/// What a [`PicturePair`] turns on (the "on" side).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairOption {
    /// The yes/no questions are asked (off: every [`ConfirmDialog`] question
    /// answers yes).
    Confirm,
    /// A click stops any movie (`consset "moviestop",1`; off, -1: as
    /// the script's flag says).
    MovieStop,
    /// Auto mode and skip stop at a choice (off: `consset "selectkeep",1`).
    StopAtChoice,
    /// Skip goes on through text not read before.
    SkipUnread,
    /// The voice plays on after a click (off: `consset "clickvoicestop",1`).
    VoicePlaysOn,
    /// Auto mode waits for the voice (off: `consset "autovoicewait",0`).
    AutoWaitsForVoice,
    /// The music, the sound effects, the voices are heard (`consset "mutebgm"` and the like).
    Bgm,
    Se,
    Voice,
}

/// A line of text on a picture screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureText {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub font_size: u32,
    pub colour: String,
}

/// Extras drawn from the game's own pictures (装甲悪鬼村正's 特別付加目録):
/// buttons over a full-screen picture, each opening a mode or going back
/// to the title (as does a right-click). It cross-fades in from the title
/// and back to it over `fade_ms`. Buttons are strips of three cells: at
/// rest, under the pointer, pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureExtras {
    pub background: String,
    pub items: Vec<PictureExtrasItem>,
    /// Played when the pointer comes onto a button.
    pub hover_se: Option<String>,
    /// Played on going back to the title.
    pub back_se: Option<String>,
    pub fade_ms: u32,
    pub gallery: Option<PictureGallery>,
    pub sound: Option<SoundRoom>,
    pub replay: Option<PictureReplay>,
    pub turntable: Option<Turntable>,
    /// The way into a mode and back (`None`: the menu cross-fades).
    pub passage: Option<ExtrasPassage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureExtrasItem {
    pub button: Placed,
    pub mode: ExtrasMode,
    /// Locked while this variable is 0: drawn at `locked_alpha` (0-255),
    /// not clickable.
    pub unlocked_by: Option<Var>,
    pub locked_alpha: u8,
    /// Played when it is clicked.
    pub click_se: Option<String>,
}

/// What a [`PictureExtrasItem`] opens. A mode the script lacks is drawn
/// but not clickable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtrasMode {
    Gallery,
    Recollection,
    Sound,
    /// A viewer of looping movies with their names and details (装甲悪鬼村正's
    /// 劒冑回転目録).
    Turntable,
    /// Back to the title.
    Back,
}

/// The CG gallery of [`PictureExtras`]: categories, each pages of slots
/// over its own picture; a slot shows the thumbnail of its first CG seen
/// (at `partial_alpha` while some are unseen, `blank` while none is) and
/// opens the viewer on its CGs. Slots run down the columns from `first`,
/// `row_step` apart, the next column `column_step` away. The pointer's
/// slot gets `cursor` over it. Previous and next page go round; `exit`
/// or a right-click goes back to the extras' menu. Buttons are strips of
/// three cells: at rest, under the pointer, pressed; a category's shows
/// its third cell while it is open.
///
/// The viewer shows a slot's CGs in turn, each cross-faded in over
/// `view_fade_ms` (`unseen` for one not seen, with black between two
/// unseen); a picture larger than the screen starts at one end and a click
/// scrolls it to the other at `scroll_ms_per_px` (a click pauses and goes
/// on, a right-click jumps to the end). A click past the last, or a
/// right-click, closes it over `close_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureGallery {
    pub background: String,
    pub categories: Vec<GalleryCategory>,
    pub previous: Placed,
    pub next: Placed,
    pub exit: Placed,
    pub first: (i32, i32),
    pub row_step: i32,
    pub column_step: i32,
    pub rows: u32,
    pub columns: u32,
    pub blank: String,
    pub cursor: String,
    pub partial_alpha: u8,
    pub unseen: String,
    /// From the menu, and between pages and categories.
    pub open_ms: u32,
    pub page_ms: u32,
    pub view_fade_ms: u32,
    pub close_ms: u32,
    pub scroll_ms_per_px: u32,
    /// Slots run along the rows instead (`column_step` apart, the next row
    /// `row_step` below): 続・殺戮のジャンゴ's five across.
    pub across: bool,
    /// Previous and next show only in a category of more than one page.
    pub arrows_when_paged: bool,
    /// The pages are drawn by the engine (CONscripter's `screen`,
    /// `screens/gallery.lua`); the viewer stays a script routine.
    pub engine_screen: bool,
    /// Over a category's button under the pointer.
    pub category_hover: Option<CategoryHover>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryCategory {
    pub button: Placed,
    pub slots: Vec<PictureSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureSlot {
    pub cgs: Vec<SlotCg>,
    /// A picture larger than the screen starts at its far end (bottom or
    /// right) and scrolls back to the top left.
    pub reversed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotCg {
    /// Non-zero once the CG has been seen.
    pub seen: Var,
    /// The full picture and its size; one no larger than the screen is
    /// shown at its x, y.
    pub picture: Placed,
    pub thumbnail: String,
}

/// The music and movie room of [`PictureExtras`]: a button per track and
/// per movie over its own picture. A track plays looping (the one playing
/// clicked again stops) and stays the extras' music, kept across games:
/// the extras play it when they open, and the title starts its own music
/// again if it differs. `marker` (an animation's cells, looping) shows
/// beside the track playing, `marker_dx` left of its button. A movie
/// fades the music down and the screen to black over `movie_fade_ms`,
/// plays (a click stops it), then the music comes back over that time and
/// the black fades out over `movie_back_ms`. `exit` or a right-click goes
/// back to the extras' menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundRoom {
    pub background: String,
    /// Three cells: at rest, under the pointer, pressed.
    pub exit: Placed,
    pub tracks: Vec<RoomTrack>,
    pub movies: Vec<RoomVideo>,
    /// The track playing at first: the title's music.
    pub title_track: usize,
    /// The track the extras play each time they open from the title,
    /// fading in over `menu_fade_ms` (続・殺戮のジャンゴ's `dj_INS12`); `None`:
    /// the extras' music from before (kept across games).
    pub menu_track: Option<usize>,
    pub menu_fade_ms: u32,
    pub marker: Placed,
    pub marker_cells: u32,
    pub marker_ms: u32,
    pub marker_dx: i32,
    pub open_ms: u32,
    pub stop_fade_ms: u32,
    pub movie_fade_ms: u32,
    pub movie_back_ms: u32,
    /// Drawn by the engine (CONscripter's `screen`, `screens/sound.lua`)
    /// rather than by a script loop.
    pub engine_screen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomTrack {
    /// Two cells: at rest, under the pointer.
    pub button: Placed,
    pub file: String,
    /// Locked while this variable is 0: `locked` drawn instead (nothing
    /// when `None`), not clickable.
    pub unlocked_by: Option<Var>,
    pub locked: Option<String>,
    /// Drawn in the button's place while the track plays (still the
    /// button): 続・殺戮のジャンゴ's title in white.
    pub playing: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomVideo {
    pub thumbnail: Placed,
    /// Over the thumbnail: two cells, nothing and the pointer's cursor.
    pub cursor: String,
    pub file: String,
    /// Locked while 0: `locked` drawn instead, not clickable.
    pub unlocked_by: Option<Var>,
    pub locked: String,
}

/// A key of [`Op::SpriteMotion2`], `ms` from the motion's start: the offset
/// from the sprite's place (pixels), its zoom (percent, 100 = its size;
/// `zoom` across, `zoom_y` up and down) and angle (degrees, counterclockwise) about the centre of its place, and its
/// opacity (0-256 of its own); straight from key to key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionKey {
    pub ms: u32,
    pub x: i32,
    pub y: i32,
    pub zoom: u32,
    pub angle: i32,
    pub opacity: u32,
    pub zoom_y: u32,
}

/// After the last key of an [`Op::SpriteMotion2`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionEnd {
    /// The sprite is back at its place, size and opacity.
    Back,
    /// The motion starts again.
    Loop,
    /// The sprite stays at the last key's place, zoom and opacity (its
    /// angle goes back to 0). A later motion on it (also stopping one)
    /// first takes a motion still running to its end.
    Keep,
    /// After the first pass the motion starts again from key `k` (a
    /// lead-in, then a loop; the last key should match key `k`).
    LoopFrom(usize),
}

/// The scene replay of [`PictureExtras`]: a slot per scene over its own
/// picture, down the columns from `first` as [`PictureGallery`]'s; an
/// unlocked scene shows its thumbnail, a locked one `blank`, with `cursor`
/// over the pointer's. A click fades the music out over `start_ms` and
/// plays the scene (a gosub to its label); at its end the music fades out
/// and the screen holds over `end_ms`, then the list cross-fades back in
/// over `open_ms` with the extras' music. `exit` or a right-click goes
/// back to the extras' menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureReplay {
    pub background: String,
    /// Three cells: at rest, under the pointer, pressed.
    pub exit: Placed,
    pub first: (i32, i32),
    pub row_step: i32,
    pub column_step: i32,
    pub rows: u32,
    pub blank: String,
    /// Two cells: nothing, the cursor.
    pub cursor: String,
    pub scenes: Vec<ReplayEntry>,
    pub open_ms: u32,
    pub start_ms: u32,
    pub end_ms: u32,
    /// Slots run along rows of this many instead (`column_step` apart, the
    /// next row `row_step` below): 続・殺戮のジャンゴ's five across.
    pub across: Option<u32>,
    /// The list stays frozen over the scene's first pictures, in this
    /// screen-effect slot (a [`ScreenEffect::Zoom`] at 100 % about
    /// `.1`, the screen's centre), until the scene clears the slot
    /// (装甲悪鬼村正's `RecoIn` fades it out).
    pub hold: Option<(u32, (i32, i32))>,
    /// The list is drawn by the engine (CONscripter's `screen`,
    /// `screens/replay.lua`) rather than by a script loop.
    pub engine_screen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEntry {
    pub thumbnail: String,
    /// Unlocked once non-zero.
    pub unlocked_by: Var,
    /// The scene, a subroutine.
    pub label: String,
}

/// The [`ExtrasMode::Turntable`] screen of [`PictureExtras`]: pages of
/// name plates (`per_page` each, in their places on the page), the chosen
/// entry's plate drawn pressed and its movie looping, name and details
/// beside it. A plate is `locked` until its entry's flag is set. A click
/// on a plate chooses it, `previous`/`next` turn the page (round), the
/// screen cross-fading over `change_ms`; `exit` or a right-click goes back
/// to the extras' menu. The page and the entry chosen are kept across
/// games; `first` is chosen at first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turntable {
    pub background: String,
    /// Three cells each: at rest, under the pointer, pressed.
    pub previous: Placed,
    pub next: Placed,
    pub exit: Placed,
    pub locked: String,
    pub entries: Vec<TurntableEntry>,
    pub per_page: usize,
    pub first: usize,
    pub open_ms: u32,
    pub change_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurntableEntry {
    /// Three cells: at rest, under the pointer, pressed (chosen).
    pub plate: Placed,
    /// Locked while 0; `None`: always open.
    pub unlocked_by: Option<Var>,
    pub name: Placed,
    pub details: Placed,
    /// Drawn in place of `details` once the variable is non-zero.
    pub details_after: Option<(Var, String)>,
    /// The movie's cells side by side (`w`×`h` each), shown `frame_ms`
    /// apart, or a movie file (one cell, 0 ms: the engine plays it,
    /// looping), zoomed `zoom` percent, the zoomed movie's top left at
    /// `x`,`y`.
    pub movie: Placed,
    pub cells: u32,
    pub frame_ms: u32,
    pub zoom: u32,
}

/// [`SaveGrid::popup`]: a window and its buttons, each `x`,`y` from the
/// clicked slot's top left. Buttons are strips: at rest, under the
/// pointer, disabled, pressed. Save is disabled on a locked slot and
/// outside the game, Load and Lock on an empty slot, Delete on an empty or
/// locked one. Lock (`unlock` while the slot is locked) toggles the lock,
/// a global per slot, shown by `locked` over the slot. A right-click
/// closes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotPopup {
    pub window: Placed,
    pub save: Placed,
    pub load: Placed,
    pub delete: Placed,
    pub lock: Placed,
    pub unlock: Placed,
    pub locked: Placed,
    /// A click on an empty slot saves at once in the game (asking nothing)
    /// and does nothing outside it, instead of opening the window.
    pub empty_saves: bool,
}

/// How [`PictureExtras`]' menu goes into a mode and back (続・殺戮のジャンゴ's
/// saloon doors). Into a mode: the buttons fade out over `buttons_ms`,
/// `open_se` plays and `doors` (a strip of `cells` cells over
/// `background`, the first as the menu shows them) step to their last
/// cell, each cross-fading in over `step_ms`, the first after `step_ms`,
/// the last held `hold_ms` from its start; then background and doors zoom
/// to `zoom` percent about their centres over `zoom_ms` (easing in, t²)
/// while the screen goes to black, and the mode comes in from black. Back
/// from a mode: it goes to black over `out_ms`; background and open doors
/// zoom back from `zoom` over `back_ms` (easing out) as the black fades,
/// `close_se` plays, the doors shut (their first cell) and the buttons
/// fade in over `buttons_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtrasPassage {
    /// The menu's background without the doors.
    pub background: String,
    pub doors: Placed,
    pub cells: u32,
    pub buttons_ms: u32,
    pub step_ms: u32,
    pub hold_ms: u32,
    pub zoom: u32,
    pub zoom_ms: u32,
    pub out_ms: u32,
    pub back_ms: u32,
    pub open_se: Option<String>,
    pub close_se: Option<String>,
}

/// A looping animation over a [`GalleryCategory`]'s button while the
/// pointer is on it, not on the open category's (続・殺戮のジャンゴ's rifle);
/// drawn by the script routine (not with `engine_screen`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryHover {
    /// A strip of `cells` cells, shown in turn `ms` each from the first.
    pub image: String,
    pub cells: u32,
    pub ms: u32,
    /// Its place over each category, in order.
    pub places: Vec<(i32, i32)>,
    /// Played on opening another category.
    pub click_se: Option<String>,
}
