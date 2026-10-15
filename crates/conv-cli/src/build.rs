//! What every build does once its IR is compiled, whatever the engine:
//! write the game's files, pack the assets, report.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use conv_core::ir::{Op, Script};
use conv_engines::build::{Built, Opts};
use conv_engines::media::{self, AudioCodec, AudioFormat};

use crate::version;

/// What every whole-game convert command takes besides its source.
#[derive(clap::Args, Debug)]
pub struct BuildArgs {
    #[arg(short, long)]
    pub output: PathBuf,
    /// Font to copy into the output as default.ttf (CONscripter needs one;
    /// default: the bundled IPA Gothic)
    #[arg(long)]
    pub font: Option<PathBuf>,
    /// Keep assets as loose files instead of packing them into .nsa
    #[arg(long)]
    pub loose: bool,
    /// Compile and write 0.txt only, no assets
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub webp: WebpArgs,
    #[command(flatten)]
    pub audio: AudioArgs,
}

/// A whole-game build: checks the output folder (`output_dirs`: the
/// asset folders, packed into `arc.nsa`), lets the engine compile the game
/// and write its assets (`compile`), then emits the script, converts
/// pictures and sounds, writes, packs and reports.
pub fn run(args: BuildArgs, title: &str, output_dirs: &[&str], compile: impl FnOnce(&Opts) -> Result<Built>) -> Result<()> {
    let BuildArgs { output, font, loose, dry_run, webp, audio } = args;
    check_output(&output, output_dirs, !loose && !dry_run)?;
    let font = read_font(font.as_deref())?;
    let opts = Opts { output: &output, title, font: &font, dry_run, audio: audio.format()? };
    let Built { mut ir, notes, picture_dirs, audio_dirs, package } = compile(&opts)?;
    ir.version = Some(version::line());
    // A dry run writes the script but no assets (no WebP, no sound).
    let source = webp_tree(&output, picture_dirs, webp.get().filter(|_| !dry_run), conv_cons::emit(&ir))?;
    if !dry_run {
        reencode_audio(&output, audio_dirs, opts.audio)?;
    }
    let lost = write_game(&output, &ir, &source, (!dry_run).then_some(&*font), package.as_deref())?;
    if !dry_run && !loose {
        pack(&output, output_dirs)?;
    }
    summary(&output, &ir, lost, notes, Some(!dry_run))
}

/// `--audio-codec` and `--audio-quality`, for every convert command.
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct AudioArgs {
    /// Voices and sound effects as vorbis (the game's own files are kept
    /// as they are) or opus (re-encoded with ffmpeg, about half the size;
    /// CONscripter 0.21 or later plays it). Music stays Vorbis
    #[arg(long, value_enum, default_value_t = Codec::Vorbis)]
    audio_codec: Codec,
    /// Their quality: for vorbis ffmpeg's -q:a, -1 to 10 (re-encodes the
    /// game's own files); for opus kbit/s per channel, 6 to 256 (default 32)
    #[arg(long, allow_negative_numbers = true)]
    audio_quality: Option<f32>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq)]
enum Codec {
    Vorbis,
    Opus,
}

impl AudioArgs {
    pub fn format(&self) -> Result<AudioFormat> {
        let codec = match self.audio_codec {
            Codec::Vorbis => AudioCodec::Vorbis,
            Codec::Opus => AudioCodec::Opus,
        };
        if let Some(q) = self.audio_quality {
            let (low, high) = if codec == AudioCodec::Opus { (6.0, 256.0) } else { (-1.0, 10.0) };
            if !(low..=high).contains(&q) {
                bail!("--audio-quality {q}: must be from {low} to {high} for {:?}", self.audio_codec);
            }
        }
        Ok(AudioFormat { codec, quality: self.audio_quality })
    }
}

/// `--webp` and `--webp-quality`, for every convert command.
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct WebpArgs {
    /// Store images as near-lossless WebP (about half the size of PNG).
    /// Needs a CONscripter build that reads WebP: the web build, or one
    /// whose SDL_image has libwebp
    #[arg(long)]
    webp: bool,
    /// With --webp: lossy WebP at this quality (0-100, as cwebp -q),
    /// smaller than the near-lossless default; transparency stays
    /// lossless
    #[arg(long, requires = "webp", value_parser = clap::value_parser!(u8).range(0..=100))]
    webp_quality: Option<u8>,
}

impl WebpArgs {
    /// None for PNG; with --webp, the lossy quality, if any.
    pub fn get(&self) -> Option<Option<u8>> {
        self.webp.then_some(self.webp_quality)
    }
}

/// With `webp` (Some: the lossy quality, if any), re-encodes the pictures
/// under `dirs` in `output` (subfolders included) as WebP and points the
/// script `source` at them. Pictures too large for WebP stay PNG.
pub fn webp_tree(output: &Path, dirs: &[&str], webp: Option<Option<u8>>, source: String) -> Result<String> {
    let Some(quality) = webp else {
        return Ok(source);
    };
    let t = conv_cons::webp::convert_tree(output, dirs, quality).context("converting images to WebP")?;
    println!(
        "  {} images as WebP: {:.1} MB (PNG {:.1} MB); {} too large for WebP kept as PNG",
        t.totals.files,
        t.totals.webp_bytes as f64 / 1e6,
        t.totals.png_bytes as f64 / 1e6,
        t.too_large
    );
    Ok(conv_cons::webp::retarget_names(&source, &t.names))
}

/// Re-encodes the voices and sound effects under `dirs` (in `output`) as
/// `format` asks, before they are packed. Nothing to do for the default.
pub fn reencode_audio(output: &Path, dirs: &[&str], format: AudioFormat) -> Result<()> {
    if !format.reencodes() {
        return Ok(());
    }
    let paths: Vec<_> = dirs.iter().map(|d| output.join(d)).collect();
    let r = media::reencode_dirs(&paths, format);
    for f in r.failed.iter().take(10) {
        println!("  audio error: {f}");
    }
    if r.files == 0 && !r.failed.is_empty() {
        bail!("{} sounds couldn't be re-encoded (is ffmpeg installed, with {}?)", r.failed.len(), if format.codec == AudioCodec::Opus { "libopus" } else { "libvorbis" });
    }
    println!(
        "  {} voices and sound effects as {}: {:.1} MB ({:.1} MB before), {} failed",
        r.files,
        format.tag().trim_start_matches("CONSCRIPTER_AUDIO="),
        r.bytes_after as f64 / 1e6,
        r.bytes_before as f64 / 1e6,
        r.failed.len()
    );
    Ok(())
}

/// Fails if one of the asset folders `dirs` is already in `output`:
/// packing deletes them afterwards, so a build never starts with folders
/// that might hold someone's own files. Loose builds and dry runs don't
/// pack (`packs` false).
pub fn check_output(output: &Path, dirs: &[&str], packs: bool) -> Result<()> {
    fs::create_dir_all(output)?;
    if let Some(d) = dirs.iter().find(|d| packs && output.join(d).exists()) {
        bail!("{} already exists; move it away first (it would be packed and then deleted)", output.join(d).display());
    }
    Ok(())
}

/// IPA Gothic (IPAfont 003.03; its license in `fonts/`), the font when no
/// `--font` is given: the games bring none we can use (the Vita draws with
/// the console's own font, the PC games with Windows').
pub const DEFAULT_FONT: &[u8] = include_bytes!("../fonts/ipag.ttf");

/// The `--font` file's bytes, else [`DEFAULT_FONT`].
pub fn read_font(path: Option<&Path>) -> Result<Cow<'static, [u8]>> {
    Ok(match path {
        Some(p) => fs::read(p).with_context(|| format!("reading {}", p.display()))?.into(),
        None => DEFAULT_FONT.into(),
    })
}

/// Writes the game into `output`: `0.txt` from `source` (the emitted
/// script), `version.txt` (with `package`, the source's own version, when
/// known), `ons_args` and, when given, the font as `default.ttf`. Returns
/// the number of characters Shift-JIS can't show. Fails when a game
/// variable is one conv-cons keeps for itself ([`conv_cons::clashes`]).
pub fn write_game(output: &Path, ir: &Script, source: &str, font: Option<&[u8]>, package: Option<&str>) -> Result<usize> {
    let clashes = conv_cons::clashes(ir);
    if !clashes.is_empty() {
        anyhow::bail!("the game's variables overlap conv-cons' (conv_core::alloc):\n{}", clashes.join("\n"));
    }
    let (encoded, lost) = conv_cons::encode(source);
    fs::write(output.join("0.txt"), encoded)?;
    if let Some(v) = &ir.version {
        version::write(output, &ir.title, v, package)?;
    }
    fs::write(output.join("ons_args"), conv_cons::ENGINE_ARGS)?;
    if let Some(font) = font {
        fs::write(output.join("default.ttf"), font)?;
    }
    Ok(lost)
}

/// Prints the build's report: [`report`], then what the compiler left out
/// or approximated (`notes`, with counts), then [`lint_dir`]'s findings
/// (`Some(files)`; `files`: also look for the files the script names).
/// A build of part of a game skips the lint (`None`): its jumps out of
/// the part have no labels.
pub fn summary(output: &Path, ir: &Script, lost: usize, notes: BTreeMap<String, usize>, lint: Option<bool>) -> Result<()> {
    report(ir, lost);
    let mut notes: Vec<_> = notes.into_iter().collect();
    notes.sort_by_key(|n| std::cmp::Reverse(n.1));
    if !notes.is_empty() {
        println!("  not converted or approximated:");
    }
    for (what, n) in notes {
        println!("    {n:>6}  {what}");
    }
    let Some(files) = lint else { return Ok(()) };
    let lint = lint_dir(output, files)?;
    if !lint.is_empty() {
        println!("  lint:");
        print!("{lint}");
    }
    Ok(())
}

/// Packs the files under `dirs` (subfolders included) into .nsa archives,
/// then deletes the folders.
pub fn pack(output: &Path, dirs: &[&str]) -> Result<()> {
    fn walk(dir: &Path, name: &str, files: &mut Vec<conv_cons::nsa::NsaFile>) -> Result<()> {
        let Ok(entries) = fs::read_dir(dir) else { return Ok(()) };
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            let name = format!("{name}\\{}", path.file_name().unwrap().to_string_lossy());
            if path.is_dir() {
                walk(&path, &name, files)?;
            } else {
                files.push(conv_cons::nsa::NsaFile { name, source: path });
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    for d in dirs {
        walk(&output.join(d), d, &mut files)?;
    }
    let archives = conv_cons::nsa::write_archives(output, &files).context("packing .nsa archives")?;
    for d in dirs {
        let _ = fs::remove_dir_all(output.join(d));
    }
    for a in &archives {
        let size = fs::metadata(a)?.len();
        println!("  {}: {:.1} MB", a.display(), size as f64 / 1e6);
    }
    println!("  packed {} files", files.len());
    Ok(())
}

fn report(ir: &Script, lost: usize) {
    let mut unsupported = std::collections::BTreeMap::<String, usize>::new();
    let mut converted = 0;
    for op in &ir.ops {
        match op {
            Op::Unsupported { what, .. } => {
                let key = what.split(" sub").next().unwrap_or(what).to_string();
                *unsupported.entry(key).or_default() += 1;
            }
            Op::Label(_) => {}
            _ => converted += 1,
        }
    }
    println!("  converted operations: {converted}");
    let total: usize = unsupported.values().sum();
    println!("  unsupported instructions: {total}");
    let mut by_count: Vec<_> = unsupported.into_iter().collect();
    by_count.sort_by_key(|a| std::cmp::Reverse(a.1));
    for (what, n) in by_count.iter().take(12) {
        println!("    {n:>6}  {what}");
    }
    if lost > 0 {
        println!("  characters not representable in Shift-JIS: {lost}");
    }
}

/// Lints `dir/0.txt` ([`conv_cons::lint`]). With `files`, also checks that
/// the files it names are in `dir`'s .nsa archives, or loose with an ASCII
/// name (CONscripter doesn't find loose files with Japanese names).
pub fn lint_dir(dir: &Path, files: bool) -> Result<conv_cons::lint::Report> {
    let path = dir.join("0.txt");
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let script = conv_cons::decode(&bytes);
    let mut have = std::collections::HashSet::new();
    if files {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
            if name.starts_with("arc") && name.ends_with(".nsa") {
                for n in conv_cons::nsa::read_names(&path).with_context(|| format!("reading {}", path.display()))? {
                    have.insert(n.to_lowercase());
                }
            }
        }
        fn walk(dir: &Path, name: &str, have: &mut std::collections::HashSet<String>) {
            let Ok(entries) = fs::read_dir(dir) else { return };
            for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
                let file = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                let name = if name.is_empty() { file } else { format!("{name}\\{file}") };
                if path.is_dir() {
                    walk(&path, &name, have);
                } else {
                    have.insert(name);
                }
            }
        }
        walk(dir, "", &mut have);
    }
    Ok(conv_cons::lint::lint(&script, &|n| !files || have.contains(&n.replace('/', "\\").to_lowercase())))
}
