//! 俺たちに翼はない's build: the scenes of SCRIPT.cpk, the pictures and
//! sounds they use (sounds and movies converted with ffmpeg here), the
//! Vita's screens.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use conv_core::ir::Script;

use super::assets::{ArchiveAssets, Bank, PICTURE_DIRS};
use super::convert::{self, Game};
use super::cpk::Archive;
use super::tasks;
use crate::build::{Built, Opts, map};
use crate::media;

pub fn open(path: &Path, bytes: &[u8]) -> Result<Archive> {
    Archive::parse(bytes).with_context(|| format!("reading {}", path.display()))
}


/// The scripts of SCRIPT.cpk by name, decoded.
pub fn scripts(game: &Path) -> Result<BTreeMap<String, String>> {
    let path = crate::find(game, "media/SCRIPT.cpk");
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let a = open(&path, &bytes)?;
    a.entries.iter().map(|e| Ok((e.name.clone(), tasks::decode(a.read(&bytes, e)?)))).collect()
}


/// Compiles `scenes` (file names such as S01_01A, in this order; empty:
/// all, from the game's first scene) of `game` (the decrypted Vita
/// folder, with `media/`).
pub fn build(game: &Path, scenes: &[String], opts: &Opts) -> Result<Built> {
    let (output, font, title, dry_run, audio_format) = (opts.output, opts.font, opts.title, opts.dry_run, opts.audio);
    let g = Game::load(&scripts(game)?).map_err(anyhow::Error::msg)?;
    let names = ["CG.cpk", "CHR.cpk", "ETC.cpk", "SystemData.cpk", "VOICE.AWB", "SE.AWB", "BGM.AWB"];
    let maps: Vec<_> = names.iter().map(|n| map(&crate::find(game, &format!("media/{n}")))).collect::<Result<_>>()?;
    let mut banks = Vec::new();
    for (n, m) in names.iter().zip(&maps) {
        banks.push(Bank::new(m).with_context(|| format!("reading {n}"))?);
    }
    let mut b = banks.into_iter();
    let mut next = || b.next().unwrap();
    let system_se = map(&crate::find(game, "media/SystemSE.AWB")).ok();
    let mut assets = ArchiveAssets::new(next(), next(), next(), next(), next(), next(), next(), output);
    assets.system_se = system_se.as_ref().and_then(|m| Bank::new(m).ok());
    assets.out.dry_run = dry_run;
    assets.movie_dir = crate::find(game, "media/data");
    // The menus' labels and the skip dialog's summaries are drawn with the font.
    assets.font = Some(font);
    let compiled = convert::compile(&g, scenes, &mut assets);
    let window = convert::text_window(&mut assets);
    let labels = super::menus::Labels::new(font);
    let mut extras = if scenes.is_empty() { labels.as_ref().and_then(|l| super::extras::extras(&mut assets, l, &g)) } else { None };
    let title_screen = super::screens::title(&mut assets, extras.is_some());
    // The extras menu is the title's GALLERY list, over the title.
    if let (Some(e), Some(t)) = (&mut extras, &title_screen) {
        e.menu_background = Some(t.background.clone());
    }
    let menus = labels.as_ref().map(|l| {
        use super::menus;
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
        language: "ja".into(),
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
        source_map: compiled.source_map,
        menu_sounds: super::screens::menu_sounds(&mut assets),
        boot: if scenes.is_empty() { super::screens::boot(&mut assets, &crate::find(game, "media/img/logo")) } else { Vec::new() },
        ..Default::default()
    };
    // made below, after the IR (none in a dry run)
    assets.audio.values().filter_map(|j| j.output.strip_prefix(output).ok()).for_each(|rel| crate::output::record(&rel.to_string_lossy()));
    assets.movies.keys().for_each(|rel| crate::output::record(rel));
    if !dry_run {
        audio(&assets.audio, output, audio_format)?;
        movies(&assets.movies, output)?;
    }
    println!("{} scenes -> {}", if scenes.is_empty() { g.scenes.len() } else { scenes.len() }, output.join("0.txt").display());
    Ok(Built { ir, notes: compiled.notes, metrics: compiled.metrics, engine: "mages", engine_doc: "mages", picture_dirs: &PICTURE_DIRS, audio_dirs: &[], package: package(game) })
}

/// Converts the queued sounds with ffmpeg, in parallel: music to Ogg
/// Vorbis, voices and sound effects as `format` asks (Vorbis by default).
/// Sounds already made are skipped, unless a voice or sound effect was
/// made with another format.
fn audio(jobs: &BTreeMap<String, super::assets::AudioJob>, output: &Path, format: media::AudioFormat) -> Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let speech = |j: &super::assets::AudioJob| j.output.starts_with(output.join("voice")) || j.output.starts_with(output.join("se"));
    let tag = format.tag();
    let made = |j: &super::assets::AudioJob| {
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
    let mut failed = 0;
    for (rel, usm) in jobs {
        let dest = output.join(rel);
        if dest.exists() {
            continue;
        }
        let data = fs::read(usm).with_context(|| format!("reading {}", usm.display()))?;
        let Some((video, audio)) = super::usm::demux(&data) else {
            println!("  movie error: {} is not a USM movie", usm.display());
            failed += 1;
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
            Err(e) => {
                println!("  movie error: {}: {e}", usm.display());
                failed += 1;
            }
        }
    }
    anyhow::ensure!(failed == 0, "{failed} of {} movies failed (above; is ffmpeg installed?)", jobs.len());
    Ok(())
}

/// The Vita package's ID and version from `sce_sys/param.sfo`
/// (`PCSG00299 01.00`).
fn package(game: &Path) -> Option<String> {
    let sfo = fs::read(crate::find(game, "sce_sys/param.sfo")).ok()?;
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
