//! `conscripter-convert mages …`: tools for 5pb./MAGES' TaskList engine
//! (俺たちに翼はない, PS Vita).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use conv_engines::mages::cpk::Archive;
use conv_engines::mages::tasks::{self, Block, Scene};

use crate::{lint_dir, map, pack, report, safe_relative_path};

#[derive(Subcommand)]
pub enum MagesCommand {
    /// List a CPK or AFS2 archive (*.cpk, *.AWB): ID, offset, size, name
    List { archive: PathBuf },
    /// Extract files from a CPK or AFS2 archive
    Extract {
        archive: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Only names starting with this (case-insensitive)
        #[arg(long, default_value = "")]
        prefix: String,
    },
    /// Convert scenes into a CONscripter game directory: 0.txt plus arc.nsa
    /// with the assets they use (sounds converted with ffmpeg)
    Convert {
        /// The decrypted game folder (contains media/)
        game: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Only these scenes (file names such as S01_01A), in this order;
        /// default: all, starting at the game's first scene
        #[arg(long, value_delimiter = ',')]
        scenes: Vec<String>,
        /// Font to copy into the output as default.ttf (CONscripter needs one)
        #[arg(long)]
        font: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "俺たちに翼はない")]
        title: String,
        /// Keep assets as loose files instead of packing them into .nsa
        #[arg(long)]
        loose: bool,
        /// Write 0.txt only, no assets
        #[arg(long)]
        dry_run: bool,
    },
    /// Parse every script in <game>/media/SCRIPT.cpk, count the task types
    /// and check that every file ID they use exists
    Check {
        /// The decrypted game folder (contains media/)
        game: PathBuf,
    },
}

pub fn run(command: MagesCommand) -> Result<()> {
    match command {
        MagesCommand::List { archive } => list(&archive),
        MagesCommand::Extract { archive, output, prefix } => extract(&archive, &output, &prefix),
        MagesCommand::Check { game } => check(&game),
        MagesCommand::Convert { game, output, scenes, font, title, loose, dry_run } => {
            convert(&game, &output, &scenes, font.as_deref(), &title, loose, dry_run)
        }
    }
}

fn open(path: &Path, bytes: &[u8]) -> Result<Archive> {
    Archive::parse(bytes).with_context(|| format!("reading {}", path.display()))
}

fn list(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let a = open(path, &bytes)?;
    for e in &a.entries {
        println!("{:>6} {:>10} {:>10}  {}", e.id, e.offset, e.size, e.name);
    }
    println!("{} files", a.entries.len());
    Ok(())
}

fn extract(path: &Path, output: &Path, prefix: &str) -> Result<()> {
    let bytes = map(path)?;
    let a = open(path, &bytes)?;
    let prefix = prefix.to_lowercase();
    let mut n = 0;
    for e in a.entries.iter().filter(|e| e.name.to_lowercase().starts_with(&prefix)) {
        let rel = safe_relative_path(&e.name).with_context(|| format!("refusing unsafe entry name {:?}", e.name))?;
        let dest = output.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, a.read(&bytes, e)?).with_context(|| format!("writing {}", dest.display()))?;
        n += 1;
    }
    println!("extracted {n} files to {}", output.display());
    Ok(())
}

/// The scripts of SCRIPT.cpk by name, decoded.
pub fn scripts(game: &Path) -> Result<BTreeMap<String, String>> {
    let path = game.join("media").join("SCRIPT.cpk");
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let a = open(&path, &bytes)?;
    a.entries.iter().map(|e| Ok((e.name.clone(), tasks::decode(a.read(&bytes, e)?)))).collect()
}

/// Calls `f` on every block under `b`, depth first.
fn walk<'a>(b: &'a Block, f: &mut impl FnMut(&'a Block)) {
    f(b);
    for c in &b.children {
        walk(c, f);
    }
}

fn check(game: &Path) -> Result<()> {
    let scripts = scripts(game)?;
    let ids = |name: &str| -> Result<Vec<u32>> {
        let path = game.join("media").join(name);
        let bytes = map(&path)?;
        Ok(open(&path, &bytes)?.entries.iter().map(|e| e.id).collect())
    };
    let cg = ids("CG.cpk")?;
    let voice = ids("VOICE.AWB")?;
    let se = ids("SE.AWB")?;
    let bgm = ids("BGM.AWB")?;

    let mut failed = 0;
    let mut types: BTreeMap<String, usize> = BTreeMap::new();
    let mut missing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut scenes = 0;
    for (name, src) in &scripts {
        let scene = match Scene::parse(src) {
            Ok(s) => s,
            Err(e) => {
                println!("{name}: {e}");
                failed += 1;
                continue;
            }
        };
        if scene.tasks.is_empty() {
            continue;
        }
        scenes += 1;
        for t in &scene.tasks {
            let ty = t.task_type().unwrap_or("?").to_string();
            *types.entry(ty.clone()).or_default() += 1;
            let mut test = |key: &str, list: &[u32], b: &Block| {
                if let Some(id) = b.int(key)
                    && id >= 0
                    && list.binary_search(&(id as u32)).is_err()
                {
                    missing.entry(format!("{ty} :{key}")).or_default().push(format!("{name}:{} {id}", b.line));
                }
            };
            walk(t, &mut |b| {
                if matches!(ty.as_str(), "Sprite" | "SpriteBG" | "SpriteBS") {
                    test("Texture", &cg, b);
                }
                test("Voice", &voice, b);
                test("SE", &se, b);
                test("BGM", &bgm, b);
            });
        }
    }
    let mut types: Vec<_> = types.into_iter().collect();
    types.sort_by_key(|t| std::cmp::Reverse(t.1));
    println!("{scenes} scenes; task types:");
    for (t, n) in types {
        println!("{n:>7}  {t}");
    }
    for (what, at) in &missing {
        println!("missing file for {what}: {} uses, e.g. {}", at.len(), at[..at.len().min(3)].join(", "));
    }
    if failed > 0 || !missing.is_empty() {
        bail!("{failed} scripts failed to parse, {} kinds of missing reference", missing.len());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn convert(game: &Path, output: &Path, scenes: &[String], font: Option<&Path>, title: &str, loose: bool, dry_run: bool) -> Result<()> {
    use conv_core::ir::Script;
    use conv_engines::mages::assets::{ArchiveAssets, Bank, OUTPUT_DIRS};
    use conv_engines::mages::convert::{self, Game};
    fs::create_dir_all(output)?;
    if let Some(d) = OUTPUT_DIRS.iter().find(|d| !loose && !dry_run && output.join(d).exists()) {
        bail!("{} already exists; move it away first (it would be packed and then deleted)", output.join(d).display());
    }
    let g = Game::load(&scripts(game)?).map_err(anyhow::Error::msg)?;
    let names = ["CG.cpk", "CHR.cpk", "ETC.cpk", "SystemData.cpk", "VOICE.AWB", "SE.AWB", "BGM.AWB"];
    let maps: Vec<_> = names.iter().map(|n| map(&game.join("media").join(n))).collect::<Result<_>>()?;
    let mut banks = Vec::new();
    for (n, m) in names.iter().zip(&maps) {
        banks.push(Bank::new(m).with_context(|| format!("reading {n}"))?);
    }
    let mut b = banks.into_iter();
    let mut next = || b.next().unwrap();
    let mut assets = ArchiveAssets::new(next(), next(), next(), next(), next(), next(), next(), output);
    assets.dry_run = dry_run;
    assets.movie_dir = game.join("media").join("data");
    let compiled = convert::compile(&g, scenes, &mut assets);
    let window = convert::text_window(&mut assets);
    // The menus' labels are drawn with the font.
    let font_bytes = font.map(fs::read).transpose().context("reading the font")?;
    let labels = font_bytes.as_deref().and_then(conv_engines::mages::menus::Labels::new);
    let extras = if scenes.is_empty() { labels.as_ref().and_then(|l| conv_engines::mages::extras::extras(&mut assets, l, &g)) } else { None };
    let title_screen = conv_engines::mages::screens::title(&mut assets, extras.is_some());
    let menus = labels.as_ref().map(|l| {
        use conv_engines::mages::menus;
        (menus::button_bar(&mut assets, l), menus::save_menu(&mut assets, l), menus::settings_menu(&mut assets, l), menus::backlog(&mut assets, l), menus::confirm_dialog(&mut assets, l))
    });
    let (button_bar, save_menu, settings, backlog, confirm) = match menus {
        Some((a, b, c, d, e)) => (Some(a), b, c, d, e),
        None => (None, None, None, None, None),
    };
    for e in &assets.errors {
        println!("  asset error: {e}");
    }
    let ir = Script {
        title: title.to_string(),
        width: convert::SCREEN_W as u32,
        height: convert::SCREEN_H as u32,
        global_border: Some(convert::GLOBAL_BORDER),
        window: Some(window),
        title_screen,
        button_bar,
        save_menu,
        settings,
        backlog,
        confirm,
        extras,
        ops: compiled.ops,
        ..Script::default()
    };
    let (encoded, lost) = conv_cons::encode(&conv_cons::emit(&ir));
    fs::write(output.join("0.txt"), encoded)?;
    fs::write(output.join("ons_args"), conv_cons::ENGINE_ARGS)?;
    if !dry_run {
        audio(&assets.audio)?;
        movies(&assets.movies, output)?;
        if let Some(font) = font {
            fs::copy(font, output.join("default.ttf")).with_context(|| format!("copying {}", font.display()))?;
        }
        if !loose {
            pack(output, &OUTPUT_DIRS)?;
        }
    }
    println!("{} scenes -> {}", if scenes.is_empty() { g.scenes.len() } else { scenes.len() }, output.join("0.txt").display());
    report(&ir, lost, font.is_some());
    let mut notes: Vec<_> = compiled.notes.into_iter().collect();
    notes.sort_by_key(|n| std::cmp::Reverse(n.1));
    if !notes.is_empty() {
        println!("  not converted or approximated:");
    }
    for (what, n) in notes {
        println!("    {n:>6}  {what}");
    }
    let lint = lint_dir(output, !dry_run)?;
    if !lint.is_empty() {
        println!("  lint:");
        print!("{lint}");
    }
    Ok(())
}

/// Converts the queued sounds to Ogg Vorbis with ffmpeg, in parallel.
fn audio(jobs: &BTreeMap<String, conv_engines::mages::assets::AudioJob>) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let jobs: Vec<_> = jobs.values().filter(|j| !j.output.exists()).collect();
    let next = AtomicUsize::new(0);
    let failed = std::sync::Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                while let Some(job) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Some(dir) = job.output.parent() {
                        let _ = fs::create_dir_all(dir);
                    }
                    let mut cmd = Command::new("ffmpeg");
                    cmd.args(["-v", "error", "-y", "-i", "pipe:0", "-c:a", "libvorbis", "-q:a", "4"]);
                    if let Some((start, end)) = job.looped {
                        cmd.args(["-metadata", &format!("LOOPSTART={start}"), "-metadata", &format!("LOOPEND={end}")]);
                    }
                    cmd.arg(&job.output).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
                    let ok = cmd.spawn().ok().and_then(|mut child| {
                        let mut stdin = child.stdin.take()?;
                        // ffmpeg may stop reading early; its exit status tells.
                        let _ = stdin.write_all(job.source);
                        drop(stdin);
                        child.wait_with_output().ok()
                    });
                    match ok {
                        Some(o) if o.status.success() => {}
                        Some(o) => failed.lock().unwrap().push(format!("{}: {}", job.output.display(), String::from_utf8_lossy(&o.stderr).trim())),
                        None => failed.lock().unwrap().push(format!("{}: could not run ffmpeg", job.output.display())),
                    }
                }
            });
        }
    });
    let failed = failed.into_inner().unwrap();
    for f in &failed {
        println!("  audio error: {f}");
    }
    println!("  {} sounds converted, {} failed", jobs.len() - failed.len(), failed.len());
    Ok(())
}

/// Converts the USM movies to H.264 + AAC MP4 with ffmpeg (loose files:
/// CONscripter plays movies from the folder, not from archives).
fn movies(jobs: &BTreeMap<String, PathBuf>, output: &Path) -> Result<()> {
    use std::process::Command;
    for (rel, usm) in jobs {
        let dest = output.join(rel);
        if dest.exists() {
            continue;
        }
        let data = fs::read(usm).with_context(|| format!("reading {}", usm.display()))?;
        let Some((video, audio)) = conv_engines::mages::usm::demux(&data) else {
            println!("  movie error: {} is not a USM movie", usm.display());
            continue;
        };
        let dir = dest.parent().unwrap();
        fs::create_dir_all(dir)?;
        let (v, a) = (dir.join("demux.m2v"), dir.join("demux.adx"));
        fs::write(&v, video)?;
        fs::write(&a, audio)?;
        let status = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-f", "mpegvideo", "-i"])
            .arg(&v)
            .args(["-f", "adx", "-i"])
            .arg(&a)
            .args(["-map", "0:v", "-map", "1:a", "-c:v", "libx264", "-crf", "20", "-preset", "medium", "-pix_fmt", "yuv420p"])
            .args(["-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart"])
            .arg(&dest)
            .status();
        let _ = fs::remove_file(&v);
        let _ = fs::remove_file(&a);
        match status {
            Ok(s) if s.success() => println!("  movie {rel}"),
            _ => println!("  movie error: ffmpeg failed on {}", usm.display()),
        }
    }
    Ok(())
}
