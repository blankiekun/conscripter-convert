//! `conscripter-convert mages …`: tools for 5pb./MAGES' TaskList engine
//! (俺たちに翼はない, PS Vita).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use conv_engines::mages::cpk::Archive;
use conv_engines::media;
use conv_engines::mages::tasks::{self, Block, Scene};

use crate::{build, map, safe_relative_path};

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
        #[command(flatten)]
        webp: build::WebpArgs,
        #[command(flatten)]
        audio: build::AudioArgs,
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
        MagesCommand::Convert { game, output, scenes, font, title, loose, dry_run, webp, audio } => {
            convert(&game, &output, &scenes, font.as_deref(), &title, loose, dry_run, webp.get(), audio.format()?)
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
fn convert(
    game: &Path,
    output: &Path,
    scenes: &[String],
    font: Option<&Path>,
    title: &str,
    loose: bool,
    dry_run: bool,
    webp: Option<Option<u8>>,
    audio_format: media::AudioFormat,
) -> Result<()> {
    use conv_core::ir::Script;
    use conv_engines::mages::assets::{ArchiveAssets, Bank, OUTPUT_DIRS, PICTURE_DIRS};
    use conv_engines::mages::convert::{self, Game};
    build::check_output(output, &OUTPUT_DIRS, !loose && !dry_run)?;
    let g = Game::load(&scripts(game)?).map_err(anyhow::Error::msg)?;
    // The menus' labels and the skip dialog's summaries are drawn with the
    // font.
    let font_bytes = font.map(fs::read).transpose().context("reading the font")?;
    let names = ["CG.cpk", "CHR.cpk", "ETC.cpk", "SystemData.cpk", "VOICE.AWB", "SE.AWB", "BGM.AWB"];
    let maps: Vec<_> = names.iter().map(|n| map(&game.join("media").join(n))).collect::<Result<_>>()?;
    let mut banks = Vec::new();
    for (n, m) in names.iter().zip(&maps) {
        banks.push(Bank::new(m).with_context(|| format!("reading {n}"))?);
    }
    let mut b = banks.into_iter();
    let mut next = || b.next().unwrap();
    let mut assets = ArchiveAssets::new(next(), next(), next(), next(), next(), next(), next(), output);
    assets.out.dry_run = dry_run;
    assets.movie_dir = game.join("media").join("data");
    assets.font = font_bytes.as_deref();
    let compiled = convert::compile(&g, scenes, &mut assets);
    let window = convert::text_window(&mut assets);
    let labels = font_bytes.as_deref().and_then(conv_engines::mages::menus::Labels::new);
    let mut extras = if scenes.is_empty() { labels.as_ref().and_then(|l| conv_engines::mages::extras::extras(&mut assets, l, &g)) } else { None };
    let title_screen = conv_engines::mages::screens::title(&mut assets, extras.is_some());
    // The extras menu is the title's GALLERY list, over the title.
    if let (Some(e), Some(t)) = (&mut extras, &title_screen) {
        e.menu_background = Some(t.background.clone());
    }
    let menus = labels.as_ref().map(|l| {
        use conv_engines::mages::menus;
        (menus::button_bar(&mut assets, l), menus::save_menu(&mut assets, l), menus::settings_menu(&mut assets, l), menus::backlog(&mut assets, l), menus::confirm_dialog(&mut assets, l))
    });
    let (button_bar, save_menu, settings, backlog, confirm) = match menus {
        Some((a, b, c, d, e)) => (Some(a), b, c, d, e),
        None => (None, None, None, None, None),
    };
    for e in &assets.out.errors {
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
        version: Some(crate::version::line()),
        ..Default::default()
    };
    // A dry run writes no pictures to convert.
    let source = build::webp_tree(output, &PICTURE_DIRS, webp.filter(|_| !dry_run), conv_cons::emit(&ir))?;
    let lost = build::write_game(output, &ir, &source, font.filter(|_| !dry_run), package(game).as_deref())?;
    if !dry_run {
        audio(&assets.audio, output, audio_format)?;
        movies(&assets.movies, output)?;
        if !loose {
            build::pack(output, &OUTPUT_DIRS)?;
        }
    }
    println!("{} scenes -> {}", if scenes.is_empty() { g.scenes.len() } else { scenes.len() }, output.join("0.txt").display());
    build::summary(output, &ir, lost, font.is_some(), compiled.notes, Some(!dry_run))
}

/// Converts the queued sounds with ffmpeg, in parallel: music to Ogg
/// Vorbis, voices and sound effects as `format` asks (Vorbis by default).
/// Sounds already made are skipped, unless a voice or sound effect was
/// made with another format.
fn audio(jobs: &BTreeMap<String, conv_engines::mages::assets::AudioJob>, output: &Path, format: media::AudioFormat) -> Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let speech = |j: &conv_engines::mages::assets::AudioJob| j.output.starts_with(output.join("voice")) || j.output.starts_with(output.join("se"));
    let tag = format.tag();
    let made = |j: &conv_engines::mages::assets::AudioJob| {
        use std::io::Read;
        let Ok(file) = fs::File::open(&j.output) else { return false };
        let mut head = Vec::new();
        file.take(8192).read_to_end(&mut head).is_ok() && (!speech(j) || head.windows(tag.len()).any(|w| w == tag.as_bytes()))
    };
    let jobs: Vec<_> = jobs.values().filter(|j| !made(j)).collect();
    let next = AtomicUsize::new(0);
    let failed = std::sync::Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                while let Some(job) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let mut args = vec!["-i".to_string(), "pipe:0".into()];
                    if speech(job) {
                        args.extend(format.ffmpeg_options(media::channels(job.source).unwrap_or(2)));
                    } else {
                        args.extend(["-c:a".into(), "libvorbis".into(), "-q:a".into(), "4".into()]);
                    }
                    if let Some((start, end)) = job.looped {
                        args.extend(["-metadata".into(), format!("LOOPSTART={start}"), "-metadata".into(), format!("LOOPEND={end}")]);
                    }
                    if let Err(e) = media::ffmpeg(&job.output, args, |stdin| stdin.write_all(job.source)) {
                        failed.lock().unwrap().push(format!("{}: {e}", job.output.display()));
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
        let mut args: Vec<std::ffi::OsString> = vec!["-f".into(), "mpegvideo".into(), "-i".into(), v.clone().into(), "-f".into(), "adx".into(), "-i".into(), a.clone().into()];
        args.extend(["-map", "0:v", "-map", "1:a"].map(Into::into));
        args.extend(media::mp4_options(20, "medium").into_iter().map(Into::into));
        let result = media::ffmpeg(&dest, args, |_| Ok(()));
        let _ = fs::remove_file(&v);
        let _ = fs::remove_file(&a);
        match result {
            Ok(()) => println!("  movie {rel}"),
            Err(e) => println!("  movie error: {}: {e}", usm.display()),
        }
    }
    Ok(())
}

/// The Vita package's ID and version from `sce_sys/param.sfo`
/// (`PCSG00299 01.00`).
fn package(game: &Path) -> Option<String> {
    let sfo = fs::read(game.join("sce_sys").join("param.sfo")).ok()?;
    let u16_at = |o: usize| Some(u16::from_le_bytes(sfo.get(o..o + 2)?.try_into().ok()?) as usize);
    let u32_at = |o: usize| Some(u32::from_le_bytes(sfo.get(o..o + 4)?.try_into().ok()?) as usize);
    if sfo.get(..4)? != b"\0PSF" {
        return None;
    }
    let (keys, data, count) = (u32_at(8)?, u32_at(12)?, u32_at(16)?);
    let value = |want: &str| {
        (0..count).find_map(|i| {
            let e = 20 + 16 * i;
            let key = sfo.get(keys + u16_at(e)?..)?;
            let key = &key[..key.iter().position(|&b| b == 0)?];
            (key == want.as_bytes()).then(|| {
                let v = sfo.get(data + u32_at(e + 12)?..data + u32_at(e + 12)? + u32_at(e + 4)?)?;
                Some(String::from_utf8_lossy(v).trim_end_matches('\0').to_string())
            })?
        })
    };
    Some(format!("{} {}", value("TITLE_ID")?, value("APP_VER")?))
}
