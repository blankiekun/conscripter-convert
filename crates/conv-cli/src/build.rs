//! What every build does once its IR is compiled, whatever the engine:
//! write the game's files, pack the assets, report.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use conv_core::ir::{Op, Script};
use conv_engines::build::{Built, Opts};
use conv_engines::media::{self, AudioCodec, AudioFormat};
use conv_engines::output::record;

use crate::version;

/// What every whole-game convert command takes besides its source.
#[derive(clap::Args, Debug)]
pub struct BuildArgs {
    #[arg(short, long)]
    pub output: PathBuf,
    /// Font to copy into the output as default.ttf (CONscripter needs one;
    /// default: the bundled IPA Gothic, which also goes in as fallback.ttf
    /// when the font lacks Japanese)
    #[arg(long)]
    pub font: Option<PathBuf>,
    /// Keep assets as loose files instead of packing them into .nsa
    #[arg(long)]
    pub loose: bool,
    /// Compile and write 0.txt only, no assets
    #[arg(long)]
    pub dry_run: bool,
    /// Also write where each 0.txt line came from in the game's scripts:
    /// one line per run of lines, `<0.txt line>\t<script>:<instruction>`
    /// (`-` = no source from that line on); engines that record it only
    #[arg(long, value_name = "FILE")]
    pub source_map: Option<PathBuf>,
    /// With --dry-run: also write the timing parity per scene, the scenes
    /// most short first: `<scene>\t<source ms>\t<our ms>\t<short ms>`, the
    /// motion time the source asks for against the time our 0.txt animates
    /// there (totals in metrics.json)
    #[arg(long, value_name = "FILE")]
    pub parity_report: Option<PathBuf>,
    #[command(flatten)]
    pub webp: WebpArgs,
    #[command(flatten)]
    pub audio: AudioArgs,
    /// A smaller download: --webp and --audio-codec opus together (an
    /// explicit --audio-codec or --webp-quality still applies)
    #[arg(long)]
    pub small: bool,
}

/// Where an edition's own files go, by language (CONscripter's asset
/// layers: `layer\\en\\cg\\…` is read for `cg\\…` while the text is English).
const LAYERS: &str = "layer";

/// Deletes the files under `layer` that are the same as the base's at the
/// same path (a layer holds only what differs). Returns how many.
fn dedupe_layer(output: &Path, layer: &Path) -> Result<usize> {
    let mut n = 0;
    let mut dirs = vec![layer.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                dirs.push(path);
            } else if let Ok(rel) = path.strip_prefix(layer) {
                let base = output.join(rel);
                if base.is_file() && fs::metadata(&base)?.len() == e.metadata()?.len() && fs::read(&base)? == fs::read(&path)? {
                    fs::remove_file(&path)?;
                    n += 1;
                }
            }
        }
    }
    Ok(n)
}

/// A whole-game build: checks the output folder (`output_dirs`: the
/// asset folders, packed into `arc.nsa`), lets the engine compile the game
/// and write its assets (`compile`), then emits the script, converts
/// pictures and sounds, writes, packs and reports.
/// Where [`conv_cons::screen_files`] go: packed like the asset folders.
const SCREENS: &str = "screens";

pub fn run(args: BuildArgs, title: &str, output_dirs: &[&str], compile: impl FnOnce(&Opts) -> Result<Built>) -> Result<()> {
    let BuildArgs { output, font, loose, dry_run, source_map, parity_report, webp, audio, small } = args;
    let webp = webp.get(small)?;
    // every engine's folders, the warp maps of its transitions and the screens
    let output_dirs: Vec<&str> = output_dirs.iter().copied().chain([conv_cons::warp::DIR, SCREENS, LAYERS]).collect();
    let output_dirs = &output_dirs[..];
    check_output(&output, output_dirs, loose, dry_run)?;
    let font = read_font(font.as_deref())?;
    let opts = Opts { output: &output, title, font: &font, dry_run, audio: audio.format(small)?, webp };
    let Built { mut ir, notes, mut metrics, engine, engine_doc, picture_dirs, audio_dirs, package, translation, asset_errors } = compile(&opts)?;
    ir.version = Some(version::line());
    // the engine fills the text from the table of the script's first `language`
    if ir.language.is_empty() {
        ir.language = "ja".into();
    }
    // A dry run writes the script but no assets (no WebP, no sound).
    let conv_cons::Emitted { script: mut source, lines, rows } = conv_cons::emit_mapped(&ir, conv_cons::strtab::Strings::tables(translation.is_some()));
    let mut tables = vec![(ir.language.clone(), conv_cons::strtab::table(&rows))];
    let other = translation.as_ref().map(|t| conv_cons::emit_mapped(t, conv_cons::strtab::Strings::tables(true)));
    // the timing parity is of the edition whose counts the build's are (the other's when there is one)
    let parity = if dry_run { other.as_ref().map_or_else(|| (source.clone(), lines.clone()), |o| (o.script.clone(), o.lines.clone())) } else { Default::default() };
    if let (Some(t), Some(other)) = (&translation, &other) {
        use conv_cons::strtab::{merge, translated, Edition};
        let (table, kept) = if lines.is_empty() || other.lines.is_empty() {
            translated(&rows, &other.rows)
        } else {
            // shortcut: --source-map stays the base's lines before the merge; map the merge's padding when a tool needs it
            let m = merge(Edition { script: &source, lines: &lines, rows: &rows }, Edition { script: &other.script, lines: &other.lines, rows: &other.rows });
            println!("  {} runs of code differ from the {} build: {} lines per language", m.blocks, t.language, m.lines);
            // a build given the other edition starts in it: the edition the player gave
            source = m.script.replacen(&format!("language \"{}\"", ir.language), &format!("language \"{}\"", t.language), 1);
            tables[0].1 = m.tables.0;
            (m.tables.1, m.kept)
        };
        println!("  text.{}.txt: {} of {} text lines from the {} build, {kept} kept as in text.{}.txt", t.language, rows.len() - kept, rows.len(), t.language, ir.language);
        tables.push((t.language.clone(), table));
    }
    if dry_run {
        // NSS fades each picture on its own: a print counts once per picture
        let ours = conv_cons::parity::animated_ms(&parity.0, &parity.1, |f| metrics.source_ms.contains_key(f), engine == "nitroplus");
        metrics.ours_ms = ours;
    }
    if lines.is_empty() {
        // no source map (a build that records none, such as TYPE-MOON's two-language one): no parity
        metrics.source_ms.clear();
    }
    if let Some(path) = source_map {
        let text: String = lines.iter().map(|(n, at)| match at {
            Some(at) => format!("{n}\t{}:{}\n", at.file, at.index),
            None => format!("{n}\t-\n"),
        }).collect();
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    // the tables name pictures too (window lines, choice plates): retargeted with the script
    let joined = std::iter::once(source).chain(tables.iter().map(|t| t.1.clone())).collect::<Vec<_>>().join("\0");
    // the other edition's own pictures, in its layer
    let layer_dirs: Vec<String> = translation.iter().flat_map(|t| picture_dirs.iter().map(move |d| format!("{LAYERS}/{}/{d}", t.language))).collect();
    let all_dirs: Vec<&str> = picture_dirs.iter().copied().chain(layer_dirs.iter().map(String::as_str)).collect();
    let joined = webp_tree(&output, &all_dirs, webp, dry_run, joined)?;
    let mut parts = joined.split('\0').map(str::to_string);
    let source = parts.next().unwrap_or_default();
    tables.iter_mut().zip(parts).for_each(|(t, p)| t.1 = p);
    if !dry_run {
        reencode_audio(&output, audio_dirs, opts.audio)?;
    }
    // lossless PNGs (outside picture_dirs: --webp leaves them alone)
    for (rel, map) in conv_cons::warp_maps(&ir) {
        record(&rel);
        if !dry_run {
            let path = output.join(&rel);
            fs::create_dir_all(path.parent().unwrap())?;
            map.save_with_format(&path, image::ImageFormat::Png).with_context(|| format!("writing {}", path.display()))?;
        }
    }
    let media = version::media(webp, opts.audio);
    let lost = write_game(&output, &ir, &source, &tables, (!dry_run).then_some(&*font), package.as_deref(), &media)?;
    if let Some(t) = &translation {
        let layer = output.join(LAYERS).join(&t.language);
        // its menus, where they differ
        let base: BTreeMap<String, String> = conv_cons::screen_files(&ir).into_iter().collect();
        for (name, lua) in conv_cons::screen_files(t) {
            if base.get(&name) != Some(&lua) {
                record(&format!("{LAYERS}/{}/{SCREENS}/{name}", t.language));
                fs::create_dir_all(layer.join(SCREENS))?;
                fs::write(layer.join(SCREENS).join(name), lua)?;
            }
        }
        if !dry_run {
            let n = dedupe_layer(&output, &layer)?;
            println!("  {LAYERS}/{}: {n} files the same as the base left out", t.language);
        }
    }
    if !dry_run && !loose {
        pack(&output, output_dirs)?;
    }
    // The completion counts (tests/snapshot.sh keeps them): dry runs only,
    // so they stay out of a game's folder.
    if dry_run {
        let game = output.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let json = metrics.json(&game, engine, engine_doc, env!("CONV_COMMIT"), env!("CONV_COMMIT_DATE"));
        fs::write(output.join("metrics.json"), json)?;
        if let Some(path) = parity_report {
            let text: String = metrics.parity().iter().map(|(s, src, ours, short)| format!("{s}\t{src}\t{ours}\t{short}\n")).collect();
            fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        }
        let (calls, gaps) = metrics.totals();
        let left_out = metrics.left_out_total();
        let pct = |n: f64| 100.0 * n / (calls - left_out).max(1) as f64;
        let full = calls - gaps - left_out;
        let weighted = metrics.weighted_total();
        println!(
            "  source calls: {calls}, fully converted {full} ({:.1}%), weighted by arguments read {weighted:.1} ({:.1}%), with gaps {gaps}, left out on purpose {left_out} (metrics.json)",
            pct(full as f64),
            pct(weighted)
        );
        if !metrics.source_ms.is_empty() {
            let p = metrics.parity();
            let s = |f: fn(&(&str, u64, u64, u64)) -> u64| p.iter().map(f).sum::<u64>() as f64 / 1000.0;
            let worst = p.first().filter(|r| r.3 > 0).map_or(String::new(), |r| format!(", most in {} ({:.1} s)", r.0, r.3 as f64 / 1000.0));
            println!(
                "  timing parity: the source asks for {:.1} s of motion, 0.txt animates {:.1} s; short {:.1} s in {} scenes{worst} (metrics.json)",
                s(|r| r.1),
                s(|r| r.2),
                s(|r| r.3),
                p.iter().filter(|r| r.3 > 0).count()
            );
        }
    }
    let files = if dry_run { dry_run_files(picture_dirs, webp.is_some()) } else { dir_files(&output)? };
    summary(&output, &ir, lost, notes, Some(Some(&files)))?;
    if asset_errors > 0 {
        bail!("{asset_errors} asset errors (\"asset error:\" above)");
    }
    Ok(())
}

/// `--audio-codec` and `--audio-quality`, for every convert command.
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct AudioArgs {
    /// Voices and sound effects as vorbis (the game's own files are kept
    /// as they are; the default, but opus with --small) or opus
    /// (re-encoded with ffmpeg, about half the size; CONscripter 0.21 or
    /// later plays it). Music stays Vorbis
    #[arg(long, value_enum)]
    audio_codec: Option<Codec>,
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
    /// `small`: Opus unless --audio-codec says otherwise.
    pub fn format(&self, small: bool) -> Result<AudioFormat> {
        let codec = match self.audio_codec {
            Some(Codec::Vorbis) => AudioCodec::Vorbis,
            Some(Codec::Opus) => AudioCodec::Opus,
            None if small => AudioCodec::Opus,
            None => AudioCodec::Vorbis,
        };
        if let Some(q) = self.audio_quality {
            let (low, high) = if codec == AudioCodec::Opus { (6.0, 256.0) } else { (-1.0, 10.0) };
            if !(low..=high).contains(&q) {
                bail!("--audio-quality {q}: must be from {low} to {high} for {codec:?}");
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
    /// With --webp (or --small): lossy WebP at this quality (0-100, as cwebp -q),
    /// smaller than the near-lossless default; transparency stays
    /// lossless
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    webp_quality: Option<u8>,
}

impl WebpArgs {
    /// None for PNG; with --webp (or `small`), the lossy quality, if any.
    pub fn get(&self, small: bool) -> Result<Option<Option<u8>>> {
        if !(self.webp || small) && self.webp_quality.is_some() {
            bail!("--webp-quality needs --webp or --small");
        }
        Ok((self.webp || small).then_some(self.webp_quality))
    }
}

/// The files a dry run would have written ([`record`]), named as the
/// script names them: with `webp`, the pictures under `picture_dirs` as
/// [`webp_tree`] retargets them.
fn dry_run_files(picture_dirs: &[&str], webp: bool) -> BTreeSet<String> {
    let files = conv_engines::output::written();
    if !webp {
        return files;
    }
    files.into_iter().map(|f| conv_cons::webp::retarget_dirs(&format!("\"{f}\""), picture_dirs).trim_matches('"').to_string()).collect()
}

/// With `webp` (Some: the lossy quality, if any), re-encodes the pictures
/// under `dirs` in `output` (subfolders included) as WebP and points the
/// script `source` at them. Pictures too large for WebP stay PNG.
pub fn webp_tree(output: &Path, dirs: &[&str], webp: Option<Option<u8>>, dry_run: bool, source: String) -> Result<String> {
    let Some(quality) = webp else {
        return Ok(source);
    };
    if dry_run {
        return Ok(conv_cons::webp::retarget_dirs(&source, dirs));
    }
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
/// pack. A dry run fails on a full build's folder (its `default.ttf`): its
/// script and loose menus would replace the build's.
pub fn check_output(output: &Path, dirs: &[&str], loose: bool, dry_run: bool) -> Result<()> {
    fs::create_dir_all(output)?;
    if dry_run && output.join("default.ttf").exists() {
        bail!("{} holds a full build; dry-run into another folder", output.display());
    }
    if let Some(d) = dirs.iter().find(|d| !loose && !dry_run && output.join(d).exists()) {
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

/// True when `font` lacks common Japanese (a Latin font such as the
/// English Muramasa's, which has a few kana and `卍` for the game's art
/// but no `「」`): the engine then draws what it lacks from `fallback.ttf`
/// instead of boxes.
pub fn needs_fallback(font: &[u8]) -> bool {
    use ab_glyph::Font;
    ab_glyph::FontRef::try_from_slice(font).map_or(true, |f| "、。「」あア日本".chars().any(|c| f.glyph_id(c).0 == 0))
}

/// Writes the game into `output`: `0.txt` from `source` (the emitted
/// script), `text.<code>.txt` from `tables` (language code, table), `system.lua` when the script has Lua, `version.txt` (with
/// `package`, the source's own version, when known, and `media`, the
/// media flags: [`version::media`]), `ons_args` and, when
/// given, the font as `default.ttf` (with [`DEFAULT_FONT`] as
/// `fallback.ttf` when it lacks Japanese: [`needs_fallback`]). Returns
/// the number of characters the engine can't draw. Fails when a game
/// variable is one conv-cons keeps for itself ([`conv_cons::clashes`]).
pub fn write_game(output: &Path, ir: &Script, source: &str, tables: &[(String, String)], font: Option<&[u8]>, package: Option<&str>, media: &str) -> Result<usize> {
    let clashes = conv_cons::clashes(ir);
    if !clashes.is_empty() {
        anyhow::bail!("the game's variables overlap conv-cons' (conv_core::alloc):\n{}", clashes.join("\n"));
    }
    let (encoded, mut lost) = conv_cons::encode(source);
    fs::write(output.join("0.txt"), encoded)?;
    for (code, table) in tables {
        let (encoded, l) = conv_cons::encode(table);
        lost += l;
        fs::write(output.join(format!("text.{code}.txt")), encoded)?;
    }
    if let Some(lua) = conv_cons::system_lua(ir) {
        record("system.lua");
        fs::write(output.join("system.lua"), conv_cons::encode(&lua).0)?;
    }
    // Menus the engine draws, UTF-8 (packed with the pictures)
    for (name, lua) in conv_cons::screen_files(ir) {
        record(&format!("{SCREENS}/{name}"));
        fs::create_dir_all(output.join(SCREENS))?;
        fs::write(output.join(SCREENS).join(name), lua)?;
    }
    if let Some(v) = &ir.version {
        version::write(output, &ir.title, v, package, media)?;
    }
    fs::write(output.join("ons_args"), conv_cons::engine_args(ir))?;
    if let Some(font) = font {
        fs::write(output.join("default.ttf"), font)?;
        // the engine draws the characters default.ttf lacks from fallback.ttf
        let fallback = output.join("fallback.ttf");
        // a language's own font (default.en.ttf, written by the engine's build) too
        let own = fs::read_dir(output)?.flatten().filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("default.") && n.ends_with(".ttf") && n != "default.ttf"));
        if needs_fallback(font) || own.filter_map(|e| fs::read(e.path()).ok()).any(|f| needs_fallback(&f)) {
            fs::write(fallback, DEFAULT_FONT)?;
        } else if fallback.exists() {
            fs::remove_file(fallback)?;
        }
    }
    Ok(lost)
}

/// Prints the build's report: [`report`], then what the compiler left out
/// or approximated (`notes`, with counts), then [`lint_dir`]'s findings
/// (`Some(files)`: also look for the files the script names among `files`,
/// as [`lint_dir`]). A build of part of a game skips the lint (`None`): its jumps out of
/// the part have no labels.
pub fn summary(output: &Path, ir: &Script, lost: usize, notes: BTreeMap<String, usize>, lint: Option<Option<&BTreeSet<String>>>) -> Result<()> {
    report(ir, lost);
    let fallbacks = conv_cons::settings_fallbacks(ir);
    if !fallbacks.is_empty() {
        println!("  settings at the shared default (none from the game): {}", fallbacks.join(", "));
    }
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
    let hash = output.join("script.hash");
    fs::write(&hash, conv_cons::nsa::script_hash(&fs::read(output.join("0.txt"))?))?;
    files.push(conv_cons::nsa::NsaFile { name: conv_cons::nsa::SCRIPT_HASH.into(), source: hash.clone() });
    let archives = conv_cons::nsa::write_archives(output, &files).context("packing .nsa archives")?;
    let _ = fs::remove_file(hash);
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
        println!("  characters past U+FFFF (not drawn): {lost}");
    }
}

/// The files in `dir` that CONscripter finds: those in its .nsa archives,
/// and loose ones with an ASCII name (it doesn't find loose files with
/// Japanese names). Lowercase, `\`-separated, as [`lint_dir`] compares.
pub fn dir_files(dir: &Path) -> Result<BTreeSet<String>> {
    let mut have = BTreeSet::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
        if name.starts_with("arc") && name.ends_with(".nsa") {
            for n in conv_cons::nsa::read_names(&path).with_context(|| format!("reading {}", path.display()))? {
                have.insert(n.to_lowercase());
            }
        }
    }
    fn walk(dir: &Path, name: &str, have: &mut BTreeSet<String>) {
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
    Ok(have)
}

/// Lints `dir/0.txt` ([`conv_cons::lint`]), its text filled from the
/// table of its first language when it has one. With `files`, also checks that
/// the files it names are among them: [`dir_files`] after a build, or the
/// names a dry run would have written ([`conv_engines::output::written`]).
pub fn lint_dir(dir: &Path, files: Option<&BTreeSet<String>>) -> Result<conv_cons::lint::Report> {
    let path = dir.join("0.txt");
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut script = conv_cons::decode(&bytes);
    let lang = conv_cons::strtab::first_language(&script).unwrap_or_default().to_string();
    if let Ok(table) = fs::read(dir.join(format!("text.{lang}.txt"))) {
        script = conv_cons::strtab::fill(&script, &conv_cons::decode(&table));
    }
    // a file of the language's layer counts too (the engine reads it first)
    let layer = format!("{LAYERS}\\{lang}\\");
    Ok(conv_cons::lint::lint(&script, &|n| {
        let n = n.replace('/', "\\").to_lowercase();
        files.is_none_or(|f| f.contains(&n) || f.contains(&format!("{layer}{n}")))
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn dry_run_lint_finds_a_file_not_written() {
        let dir = std::env::temp_dir().join(format!("conv-lint-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("0.txt"), "*start\nlsp 1,\":a;cg\\a.png\",0,0\nlsp 2,\"cg/missing.png\",0,0\nend\n").unwrap();
        conv_engines::output::Output { root: dir.clone(), dry_run: true, errors: Vec::new() }.write("cg/A.png", b"x");
        let written = conv_engines::output::written();
        let report = super::lint_dir(&dir, Some(&written)).unwrap().to_string();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(report.contains("cg\\missing.png") || report.contains("cg/missing.png"), "{report}");
        assert!(!report.contains("a.png"), "{report}");
    }

    #[test]
    fn check_output_guards_folders_and_full_builds() {
        let dir = std::env::temp_dir().join(format!("conv-check-output-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("cg")).unwrap();
        // a packing build refuses an asset folder already there; loose builds and dry runs don't pack
        assert!(super::check_output(&dir, &["cg"], false, false).is_err());
        assert!(super::check_output(&dir, &["cg"], true, false).is_ok());
        assert!(super::check_output(&dir, &["cg"], false, true).is_ok());
        // a dry run refuses a full build's folder
        std::fs::write(dir.join("default.ttf"), b"").unwrap();
        assert!(super::check_output(&dir, &["cg"], false, true).is_err());
        assert!(super::check_output(&dir, &["cg"], true, false).is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pack_archives_the_folders_and_deletes_them() {
        let dir = std::env::temp_dir().join(format!("conv-pack-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("cg/sub")).unwrap();
        std::fs::write(dir.join("cg/sub/a.png"), b"A").unwrap();
        std::fs::write(dir.join("0.txt"), b"*start\nend\n").unwrap();
        super::pack(&dir, &["cg", "absent"]).unwrap();
        assert!(!dir.join("cg").exists() && !dir.join("script.hash").exists());
        let names = conv_cons::nsa::read_names(&dir.join("arc.nsa")).unwrap();
        assert_eq!(names, ["cg\\sub\\a.png", conv_cons::nsa::SCRIPT_HASH]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fallback_only_without_japanese() {
        assert!(!super::needs_fallback(super::DEFAULT_FONT));
        assert!(super::needs_fallback(b"not a font"));
    }
}
