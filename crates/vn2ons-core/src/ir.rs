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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpritePlacement {
    pub image: String,
    /// Top-left corner on screen, in pixels.
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Label(String),
    /// Continue at a label (possibly from another source script).
    Jump(String),
    /// Replace the background. Clears sprites, as most engines do.
    Background { image: String, fade_ms: u32 },
    /// Replace the set of visible sprites (an empty list clears them).
    Sprites { sprites: Vec<SpritePlacement>, fade_ms: u32 },
    /// One page of text, shown and then waited on. `text` may contain '\n'
    /// for line breaks. `links` are character ranges in `text` (start,
    /// length) that the original game made clickable.
    Text { speaker: Option<String>, text: String, voice: Option<String>, links: Vec<(usize, usize)> },
    BgmPlay { file: String, fade_in_ms: u32 },
    BgmStop { fade_out_ms: u32 },
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

#[derive(Debug, Clone, Default)]
pub struct Script {
    /// Window caption.
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// `None` = the engine's default full-screen (NVL) text window.
    pub window: Option<TextWindow>,
    pub ops: Vec<Op>,
}
