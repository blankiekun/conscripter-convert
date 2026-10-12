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
    /// Replace the background. Clears sprites, as most engines do.
    Background { image: String, fade_ms: u32 },
    /// Replace the set of visible sprites (an empty list clears them).
    Sprites { sprites: Vec<SpritePlacement>, fade_ms: u32 },
    /// One page of text, shown and then waited on.
    Text { speaker: Option<String>, text: String, voice: Option<String> },
    BgmPlay { file: String, fade_in_ms: u32 },
    BgmStop { fade_out_ms: u32 },
    SePlay { file: String, looped: bool },
    /// `file: None` stops every sound effect.
    SeStop { file: Option<String> },
    /// Something the converter can't express yet; kept for the report.
    Unsupported { what: String, at: SourceLoc },
}

#[derive(Debug, Clone, Default)]
pub struct Script {
    /// Window caption.
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub ops: Vec<Op>,
}
