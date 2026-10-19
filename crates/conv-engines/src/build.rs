//! What an engine's whole-game build takes and hands back. Each engine
//! has a `build` function (`xuse::build`, `nitroplus::build`,
//! `mages::build`) that reads the game, writes its assets and returns the
//! IR; the CLI then does what every build shares: emit the script,
//! convert pictures and sounds, write, pack and report.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use conv_core::ir::Script;
use memmap2::Mmap;

use crate::media::AudioFormat;

/// The options every build takes besides the game's own.
pub struct Opts<'a> {
    pub output: &'a Path,
    /// Window caption.
    pub title: &'a str,
    /// The font the game's menus may draw labels with (also copied into
    /// the output by the CLI): the `--font`, else the bundled IPA Gothic.
    pub font: &'a [u8],
    /// Compile and write the script only, no assets.
    pub dry_run: bool,
    /// For voices and sound effects an engine converts itself.
    pub audio: AudioFormat,
    /// For pictures an engine writes itself: None for PNG, else WebP
    /// (`--webp`) with its lossy quality, if any (`--webp-quality`).
    pub webp: Option<Option<u8>>,
}

/// A compiled game, its assets written.
pub struct Built {
    pub ir: Script,
    /// What the compiler left out or approximated, with counts.
    pub notes: BTreeMap<String, usize>,
    /// The source's calls, converted fully or with gaps (`metrics.json`).
    pub metrics: crate::metrics::Metrics,
    /// The engine's name in `metrics.json` (`xuse`, `nitroplus`, `mages`).
    pub engine: &'static str,
    /// Its command reference, `docs/engines/<engine_doc>.md` (`metrics.json`).
    pub engine_doc: &'static str,
    /// Folders of the output whose PNG pictures `--webp` converts.
    pub picture_dirs: &'static [&'static str],
    /// Folders of the output whose sounds `--audio-codec` re-encodes
    /// (empty when the engine converts its sounds itself).
    pub audio_dirs: &'static [&'static str],
    /// The source's own version, for `version.txt`.
    pub package: Option<String>,
    /// The same game compiled in another language (its `language`): its
    /// text fills that language's table by place (conv_cons::strtab).
    pub translation: Option<Script>,
    /// How many files couldn't be read or written (`Output::errors`, each
    /// printed): the build then exits with an error.
    pub asset_errors: usize,
}

/// Maps a file read-only.
pub fn map(path: &Path) -> Result<Mmap> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    // SAFETY: the archive is opened read-only and not modified while mapped.
    unsafe { Mmap::map(&file) }.with_context(|| format!("mapping {}", path.display()))
}
