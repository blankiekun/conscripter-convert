//! `conscripter-convert typemoon …`: TYPE-MOON's archives (月姫 -A piece
//! of blue glass moon-, Switch): MRG (`.hed`/`.mrg`/`.nam`) and MZP
//! archives, and the BNTX pictures in them.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Subcommand;
use conv_engines::media::AudioFormat;
use conv_engines::typemoon::{self, bfsar, bntx, exe, fwav, mrg, mzp, nxopus, script};

use crate::{map, safe_relative_path};

#[derive(Subcommand)]
pub enum TypemoonCommand {
    /// List an archive: an MRG (the .mrg, with its .hed and .nam beside
    /// it) or an MZP (allscr.mrg, script_text.mrg): index, size, name
    List { archive: PathBuf },
    /// Extract an archive's entries, MZX/NXGX/NXCX unpacked (pictures come
    /// out as .bntx)
    Extract {
        archive: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Only names starting with this (case-insensitive)
        #[arg(long, default_value = "")]
        prefix: String,
        /// Pictures as PNG instead of .bntx: `<entry>/<texture>.png`
        #[arg(long)]
        png: bool,
        /// Only entries of at most this many bytes in the archive (e.g. a
        /// picture's third-size copy, not the full one of the same name)
        #[arg(long)]
        max_bytes: Option<usize>,
    },
    /// Read every archive in a RomFS folder, unpack every entry (and the
    /// entries of MZPs inside), decode every picture, and count what is
    /// inside and what fails
    Check { dir: PathBuf },
    /// Convert a RomFS folder's sound: `stream/*.opus` (Nintendo Opus) to
    /// `<output>/stream/*.ogg` (not re-encoded), and every wave sound of
    /// `knife1.bfsar` to `<output>/se/<name>.wav`, or `.ogg` re-encoded as
    /// `--audio-codec`/`--audio-quality` ask; each with its loop; counts
    /// them and fails on any error
    Sounds {
        dir: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[command(flatten)]
        audio: crate::build::AudioArgs,
    },
    /// Read every script of a RomFS folder (`allscr.mrg`) with its text
    /// (`script_text.mrg`): rank the commands by use, check the text and
    /// sound references, and with `-o` write each script as
    /// `<name>.txt`, one command a line, its text after it
    Script {
        dir: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Read the exe's tables (1.0.2's ExeFS `main`): pictures, motions,
    /// scene numbers; with `--romfs`, check each picture's size against
    /// its texture in `allpac`
    Exe {
        main: PathBuf,
        #[arg(long)]
        romfs: Option<PathBuf>,
        /// Print these motions' keys (`_STMA`'s numbers, from 1)
        #[arg(long, value_delimiter = ',')]
        motion: Vec<usize>,
    },
    /// Write a RomFS folder's media for the build into `<output>`: every
    /// `allpac` picture at full size as `pic/<name>.png` (`.webp` with
    /// `--webp`/`--webp-quality`; the `RULE_*` masks always PNG), each
    /// from its largest copy (a bare background's `.JPG`, else the larger
    /// `.NXGZ`); the movies as they are (H.264, remuxed to start playing
    /// before they have loaded) as `movie/<name>.mp4`; and the sound as
    /// `typemoon sounds` writes it. Pictures and movies already there are
    /// kept, so an interrupted run goes on where it stopped. With
    /// `--english`, the fan patch's pictures and movies that differ from
    /// `dir`'s as `pic/en/` and `movie/en/`
    Media {
        dir: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// The fan patch's RomFS (English), beside `dir` (1.0.2's)
        #[arg(long)]
        english: Option<PathBuf>,
        #[command(flatten)]
        webp: crate::build::WebpArgs,
        #[command(flatten)]
        audio: crate::build::AudioArgs,
    },
    /// Convert the game into a CONscripter folder: 0.txt from every script,
    /// and (unless --dry-run) the media as `typemoon media` writes them
    Convert {
        /// The RomFS folder (1.0.2's for Japanese, the fan patch's for English)
        dir: PathBuf,
        /// 1.0.2's exe (its ExeFS `main`); default: `exefs/main` beside the RomFS
        #[arg(long)]
        exe: Option<PathBuf>,
        #[arg(long, default_value = "月姫 -A piece of blue glass moon-")]
        title: String,
        /// The fan patch's RomFS: the English script beside the Japanese (one 0.txt,
        /// a language switch), and its media that differ (`typemoon media --english`)
        #[arg(long)]
        english: Option<PathBuf>,
        #[command(flatten)]
        build: crate::build::BuildArgs,
    },
}

pub fn run(command: TypemoonCommand) -> Result<()> {
    match command {
        TypemoonCommand::List { archive } => each(&archive, |i, name, data, _| {
            println!("{i:>6} {:>10}  {name}", data.len());
            Ok(())
        }),
        TypemoonCommand::Extract { archive, output, prefix, png, max_bytes } => extract(&archive, &output, &prefix.to_lowercase(), png, max_bytes),
        TypemoonCommand::Check { dir } => check(&dir),
        TypemoonCommand::Sounds { dir, output, audio } => sounds(&dir, &output, audio.format()?),
        TypemoonCommand::Script { dir, output } => scripts(&dir, output.as_deref()),
        TypemoonCommand::Exe { main, romfs, motion } => exe_tables(&main, romfs.as_deref(), &motion),
        TypemoonCommand::Media { dir, output, english, webp, audio } => media(&dir, english.as_deref(), &output, webp.get(), audio.format()?),
        TypemoonCommand::Convert { dir, exe, title, english, build } => {
            let exe = exe.unwrap_or_else(|| dir.join("../exefs/main"));
            crate::build::run(build, &title, &typemoon::build::OUTPUT_DIRS, |opts| {
                if !opts.dry_run {
                    media(&dir, english.as_deref(), opts.output, opts.webp, opts.audio)?;
                }
                typemoon::build::build(&dir, &exe, english.as_deref(), opts)
            })
        }
    }
}

/// Calls `f` with each entry of an MRG or MZP: index, name (the index if
/// it has none), data, and the `.hed`'s unpacked size in sectors.
fn each(path: &Path, mut f: impl FnMut(usize, &str, &[u8], Option<u16>) -> Result<()>) -> Result<()> {
    let bytes = map(path)?;
    let hed = path.with_extension("hed");
    if hed.exists() {
        let nam = path.with_extension("nam");
        let nam = nam.exists().then(|| map(&nam)).transpose()?;
        let entries = mrg::parse(&map(&hed)?, nam.as_deref()).with_context(|| format!("reading {}", hed.display()))?;
        for (i, e) in entries.iter().enumerate() {
            let data = mrg::data(&bytes, e).with_context(|| format!("{} entry {i}", path.display()))?;
            f(i, e.name.as_deref().unwrap_or(&format!("{i:05}")), data, Some(e.unpacked_sectors))?;
        }
    } else {
        let entries = mzp::parse(&bytes).with_context(|| format!("reading {}", path.display()))?;
        for (i, data) in entries.into_iter().enumerate() {
            f(i, &format!("{i:05}"), data, None)?;
        }
    }
    Ok(())
}

fn extract(archive: &Path, output: &Path, prefix: &str, png: bool, max_bytes: Option<usize>) -> Result<()> {
    let mut count = 0;
    each(archive, |i, name, data, _| {
        if !name.to_lowercase().starts_with(prefix) || max_bytes.is_some_and(|m| data.len() > m) {
            return Ok(());
        }
        let unpacked = typemoon::unpack(data).with_context(|| format!("entry {i} ({name})"))?;
        let mut path = output.join(safe_relative_path(name).with_context(|| format!("unsafe name {name:?}"))?);
        if matches!(unpacked, std::borrow::Cow::Owned(_)) || path.extension().is_none() {
            path.set_extension(typemoon::extension(&unpacked));
        }
        if png && unpacked.starts_with(b"BNTX") {
            path.set_extension("");
            fs::create_dir_all(&path)?;
            for t in bntx::parse(&unpacked).with_context(|| format!("entry {i} ({name})"))? {
                // Names hold dots ("(x1.5)"): add the extension, don't replace one.
                let file = path.join(safe_relative_path(&format!("{}.png", t.name)).with_context(|| format!("unsafe name {:?}", t.name))?);
                bntx::decode(&t)?.save(&file).with_context(|| format!("writing {}", file.display()))?;
            }
            count += 1;
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, &unpacked).with_context(|| format!("writing {}", path.display()))?;
        count += 1;
        Ok(())
    })?;
    println!("{count} files");
    Ok(())
}

fn check(dir: &Path) -> Result<()> {
    let mut archives: Vec<PathBuf> = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    archives.retain(|p| p.extension().is_some_and(|x| x == "mrg"));
    archives.sort();
    let (mut failed, mut formats) = (0, BTreeMap::<u32, usize>::new());
    // Every picture decodes; counts textures by format.
    let mut pictures = |data: &[u8], label: String, errors: &mut Vec<String>| match bntx::parse(data) {
        Ok(textures) => {
            for t in textures {
                *formats.entry(t.format).or_default() += 1;
                if let Err(e) = bntx::decode(&t) {
                    errors.push(format!("{label} / {}: {e}", t.name));
                }
            }
        }
        Err(e) => errors.push(format!("{label}: {e}")),
    };
    for path in &archives {
        let mut kinds = BTreeMap::<String, usize>::new();
        let (mut entries, mut errors, mut sizes_off) = (0, Vec::new(), 0);
        each(path, |i, name, data, unpacked_sectors| {
            entries += 1;
            let unpacked = match typemoon::unpack(data) {
                Ok(u) => u,
                Err(e) => {
                    errors.push(format!("{i} {name}: {e}"));
                    return Ok(());
                }
            };
            // The .hed's unpacked size, for wrapped entries; raw ones are padded.
            if matches!(unpacked, std::borrow::Cow::Owned(_)) && unpacked_sectors.is_some_and(|s| u64::from(s) != (unpacked.len() as u64).div_ceil(mrg::SECTOR)) {
                sizes_off += 1;
            }
            let kind = typemoon::extension(&unpacked);
            *kinds.entry(kind.to_string()).or_default() += 1;
            if kind == "bntx" {
                pictures(&unpacked, format!("{i} {name}"), &mut errors);
            }
            if kind == "mzp" {
                match mzp::parse(&unpacked) {
                    Ok(inner) => {
                        for (j, d) in inner.iter().enumerate() {
                            match typemoon::unpack(d) {
                                Ok(u) => {
                                    *kinds.entry(format!("{} in mzp", typemoon::extension(&u))).or_default() += 1;
                                    if u.starts_with(b"BNTX") {
                                        pictures(&u, format!("{i} {name} / {j}"), &mut errors);
                                    }
                                }
                                Err(e) => errors.push(format!("{i} {name} / {j}: {e}")),
                            }
                        }
                    }
                    Err(e) => errors.push(format!("{i} {name}: {e}")),
                }
            }
            Ok(())
        })?;
        let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
        println!("{}: {entries} entries ({}); {} failed; {sizes_off} unpacked sizes differ from the .hed", path.file_name().unwrap().to_string_lossy(), kinds.join(", "), errors.len());
        for e in errors.iter().take(5) {
            println!("  {e}");
        }
        failed += errors.len();
    }
    let formats: Vec<String> = formats.iter().map(|(f, n)| format!("{n} {f:#06x}")).collect();
    println!("pictures by BNTX format: {}", formats.join(", "));
    anyhow::ensure!(failed == 0, "{failed} entries failed");
    Ok(())
}


fn exe_tables(main: &Path, romfs: Option<&Path>, show: &[usize]) -> Result<()> {
    let exe = exe::Exe::parse(&map(main)?).with_context(|| format!("reading {}", main.display()))?;
    let pictures = exe.pictures()?;
    let motions = exe.motions()?;
    let keys: usize = motions.iter().map(|m| m.keys.len()).sum();
    println!(
        "{} pictures ({} expressions), {} motions ({keys} keys), {} scene numbers",
        pictures.len(),
        pictures.iter().filter(|p| p.expression).count(),
        motions.len(),
        exe.scenes()?.len()
    );
    for &m in show {
        let motion = motions.get(m.wrapping_sub(1)).with_context(|| format!("no motion {m}"))?;
        let looped = match motion.loop_from { 0 => String::new(), 1 => ", loops".into(), ms => format!(", loops from {ms} ms") };
        println!("motion {m}: {} ms{looped}", motion.length_ms);
        for k in &motion.keys {
            let fields: Vec<String> = k.fields.iter().enumerate().filter(|(_, v)| **v != exe::Key::UNSET).map(|(i, v)| format!("{:#04x}={v}", 4 + 2 * i)).collect();
            println!("  {:>6} ms  {}", k.time_ms, fields.join(" "));
        }
    }
    let Some(romfs) = romfs else { return Ok(()) };
    let by_name: BTreeMap<String, &exe::Picture> = pictures.iter().map(|p| (p.name.to_uppercase(), p)).collect();
    // Texture sizes per picture: a name can have two entries (a third-size copy and the full one).
    let mut sizes = BTreeMap::<&str, Vec<(f32, f32)>>::new();
    each(&romfs.join("allpac.mrg"), |i, name, data, _| {
        let Some((key, _)) = name.strip_suffix(".NXGZ").and_then(|n| by_name.get_key_value(&n.to_uppercase())) else { return Ok(()) };
        let unpacked = typemoon::unpack(data).with_context(|| format!("entry {i} ({name})"))?;
        let t = bntx::parse(&unpacked).with_context(|| format!("entry {i} ({name})"))?;
        sizes.entry(key.as_str()).or_default().push((t[0].width as f32, t[0].height as f32));
        Ok(())
    })?;
    // The full texture is 2× art for a 1280 × 724 stage: half its width, 724/1440 of its height
    // (to half a stage pixel: tall sprites' heights are 0.2 short). Bare names' sizes are a few
    // pixels off (about three times their third-size copy's).
    let height = |p: &exe::Picture, h: f32, slack: f32| (p.height - h * 724.0 / 1440.0).abs() <= slack;
    let (mut exact, mut near, mut off, mut missing, mut two) = (0, 0, Vec::new(), Vec::new(), 0);
    for (name, p) in &by_name {
        let Some(s) = sizes.get(name.as_str()) else {
            missing.push(name.as_str());
            continue;
        };
        two += usize::from(s.len() > 1);
        if s.iter().any(|&(w, h)| height(p, h, 0.5) && (p.width - w / 2.0).abs() <= 0.5) {
            exact += 1;
        } else if s.iter().any(|&(w, h)| height(p, h, 3.0) && (p.width - w / 2.0).abs() <= 3.0) {
            near += 1;
        } else {
            off.push((p.expression, format!("{}: {} x {} for {s:?}", p.name, p.width, p.height)));
        }
    }
    println!(
        "{} pictures have a texture ({two} two): {exact} sizes within half a pixel, {near} within 3 px, {} neither ({} of them expressions); {} have no allpac entry",
        sizes.len(),
        off.len(),
        off.iter().filter(|o| o.0).count(),
        missing.len()
    );
    // A few of each kind.
    for (_, line) in off.iter().filter(|o| o.0).take(5).chain(off.iter().filter(|o| !o.0).take(5)) {
        println!("  {line}");
    }
    if !missing.is_empty() {
        println!("  no entry: {}", missing[..missing.len().min(10)].join(", "));
    }
    Ok(())
}

/// The wave sounds as WAV (`smpl` loop) unless `audio` re-encodes them:
/// then Opus or Vorbis with `LOOPSTART`/`LOOPEND`, which CONscripter's
/// `dwaveloop` loops by for both. The streams stay the game's Opus.
fn sounds(dir: &Path, output: &Path, audio: AudioFormat) -> Result<()> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    let mut failed = Vec::new();
    let out = output.join("stream");
    fs::create_dir_all(&out)?;
    let mut streams: Vec<PathBuf> = fs::read_dir(dir.join("stream"))?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    streams.sort();
    let bfsar = map(&dir.join("knife1.bfsar"))?;
    let loops = bfsar::stream_loops(&bfsar).context("reading knife1.bfsar")?;
    let (mut count, mut seconds, mut looped) = (0, 0.0, 0);
    for path in &streams {
        let name = path.file_name().unwrap().to_string_lossy();
        // The archive's name of `arcstaffroll(short).opus` is STRM_ARCSTAFFROLL_SHORT_.
        let key: String = path.file_stem().unwrap().to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect();
        let lp = loops.get(&key).copied();
        looped += usize::from(lp.is_some());
        match nxopus::to_ogg(&map(path)?, lp) {
            Ok((ogg, samples)) => {
                fs::write(out.join(path.with_extension("ogg").file_name().unwrap()), ogg)?;
                (count, seconds) = (count + 1, seconds + samples as f64 / 48000.0);
            }
            Err(e) => failed.push(format!("{name}: {e}")),
        }
    }
    println!("stream: {count} of {} to Ogg Opus, {:.1} hours, {looped} looping (of {} in knife1.bfsar)", streams.len(), seconds / 3600.0, loops.len());

    let out = output.join("se");
    fs::create_dir_all(&out)?;
    let waves = bfsar::wave_sounds(&bfsar).context("reading knife1.bfsar")?;
    let (kinds, failed_waves) = (Mutex::new(BTreeMap::<String, usize>::new()), Mutex::new(Vec::new()));
    let (wav_bytes, written_bytes, next) = (AtomicU64::new(0), AtomicU64::new(0), AtomicUsize::new(0));
    let one = |w: &bfsar::WaveSound| -> Result<()> {
        let wave = fwav::decode(w.fwav)?;
        let looped = match wave.looped {
            None => "",
            Some((0, _)) => ", loops from the start",
            Some(_) => ", loops from inside",
        };
        *kinds.lock().unwrap().entry(format!("{} ch {} Hz{looped}", wave.channels, wave.rate)).or_default() += 1;
        let ext = if audio.reencodes() { "ogg" } else { "wav" };
        let file = out.join(safe_relative_path(&format!("{}.{ext}", w.name)).with_context(|| format!("unsafe name {:?}", w.name))?);
        let wav = fwav::to_wav(&wave);
        wav_bytes.fetch_add(wav.len() as u64, Ordering::Relaxed);
        if !audio.reencodes() {
            fs::write(&file, &wav)?;
            written_bytes.fetch_add(wav.len() as u64, Ordering::Relaxed);
            return Ok(());
        }
        let mut args: Vec<String> = ["-f", "wav", "-i", "pipe:0"].map(Into::into).into();
        args.extend(audio.ffmpeg_options(wave.channels as u8));
        if let Some((start, end)) = wave.looped {
            args.extend(["-metadata".into(), format!("LOOPSTART={start}"), "-metadata".into(), format!("LOOPEND={end}")]);
        }
        conv_engines::media::ffmpeg(&file, args, |stdin| stdin.write_all(&wav)).map_err(anyhow::Error::msg)?;
        written_bytes.fetch_add(fs::metadata(&file)?.len(), Ordering::Relaxed);
        Ok(())
    };
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map_or(4, |n| n.get()) {
            s.spawn(|| {
                while let Some(w) = waves.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Err(e) = one(w) {
                        failed_waves.lock().unwrap().push(format!("{}: {e:#}", w.name));
                    }
                }
            });
        }
    });
    failed.extend(failed_waves.into_inner().unwrap());
    let kinds = kinds.into_inner().unwrap();
    let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
    let mb = |a: &AtomicU64| a.load(Ordering::Relaxed) as f64 / 1e6;
    println!("knife1.bfsar: {} wave sounds ({}): {:.1} MB written ({:.1} MB as WAV)", waves.len(), kinds.join("; "), mb(&written_bytes), mb(&wav_bytes));
    for e in failed.iter().take(5) {
        println!("  {e}");
    }
    anyhow::ensure!(failed.is_empty(), "{} sounds failed", failed.len());
    Ok(())
}

fn media(dir: &Path, english: Option<&Path>, output: &Path, webp: Option<Option<u8>>, audio: AudioFormat) -> Result<()> {
    pictures(dir, &output.join("pic"), webp, None)?;
    movies(&dir.join("movie"), &output.join("movie"), None)?;
    if let Some(en) = english {
        pictures(en, &output.join("pic/en"), webp, Some(dir))?;
        movies(&en.join("movie"), &output.join("movie/en"), Some(&dir.join("movie")))?;
        // The sound comes from `dir` alone: the fan patch's is the same.
        let same = |rel: &str| -> Result<bool> { Ok(map(&dir.join(rel))?[..] == map(&en.join(rel))?[..]) };
        anyhow::ensure!(same("knife1.bfsar")?, "the fan patch's knife1.bfsar differs from {}'s", dir.display());
        let names = |d: &Path| -> Result<Vec<(String, u64)>> {
            let mut v = fs::read_dir(d.join("stream"))?.map(|e| e.and_then(|e| Ok((e.file_name().to_string_lossy().into_owned(), e.metadata()?.len())))).collect::<Result<Vec<_>, _>>()?;
            v.sort();
            Ok(v)
        };
        anyhow::ensure!(names(dir)? == names(en)?, "the fan patch's stream/ differs from {}'s", dir.display());
    }
    sounds(dir, output, audio)
}

/// `allpac`'s pictures as PNG, or WebP (`webp`: its lossy quality, if
/// any); the `rule_*` masks always PNG, exact. Per name (case ignored,
/// extension dropped) the `.JPG` if there is one, else the largest entry.
/// With `base` (another RomFS), only the pictures whose data differ from
/// its own.
fn pictures(dir: &Path, out: &Path, webp: Option<Option<u8>>, base: Option<&Path>) -> Result<()> {
    let ext = |stem: &str| if webp.is_none() || stem.starts_with("rule_") { "png" } else { "webp" };
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    let path = dir.join("allpac.mrg");
    let bytes = map(&path)?;
    let entries = mrg::parse(&map(&path.with_extension("hed"))?, Some(&map(&path.with_extension("nam"))?)).context("reading allpac.hed")?;
    let mut best = typemoon::full_copies(&entries);
    if let Some(base) = base {
        let path = base.join("allpac.mrg");
        let base_bytes = map(&path)?;
        let base_entries = mrg::parse(&map(&path.with_extension("hed"))?, Some(&map(&path.with_extension("nam"))?)).context("reading allpac.hed")?;
        let keep: std::collections::BTreeSet<String> = typemoon::differing(&typemoon::full_copies(&base_entries), &base_bytes, &best, &bytes)?.into_iter().collect();
        best.retain(|stem, _| keep.contains(stem));
    }
    fs::create_dir_all(out)?;
    let jobs: Vec<(&String, &&mrg::Entry)> = best.iter().filter(|(stem, _)| !out.join(format!("{stem}.{}", ext(stem))).exists()).collect();
    println!("allpac: {} pictures, {} to convert", best.len(), jobs.len());
    let (next, done, in_bytes, out_bytes) = (AtomicUsize::new(0), AtomicUsize::new(0), AtomicU64::new(0), AtomicU64::new(0));
    let failed = Mutex::new(Vec::new());
    let one = |stem: &str, e: &mrg::Entry| -> Result<()> {
        let img = {
            let data = typemoon::unpack(mrg::data(&bytes, e)?)?;
            if data.starts_with(b"BNTX") {
                let textures = bntx::parse(&data)?;
                anyhow::ensure!(textures.len() == 1, "{} textures", textures.len());
                bntx::decode(&textures[0])?
            } else {
                image::load_from_memory(&data)?.to_rgba8()
            }
        };
        let data = match webp.filter(|_| ext(stem) == "webp") {
            Some(q) => conv_cons::webp::encode_rgba(&img, q)?,
            None => {
                let mut png = std::io::Cursor::new(Vec::new());
                img.write_to(&mut png, image::ImageFormat::Png)?;
                png.into_inner()
            }
        };
        let file = out.join(safe_relative_path(&format!("{stem}.{}", ext(stem))).with_context(|| format!("unsafe name {stem:?}"))?);
        let part = file.with_extension("part");
        fs::write(&part, &data)?;
        fs::rename(&part, &file)?;
        in_bytes.fetch_add(e.size, Ordering::Relaxed);
        out_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
        Ok(())
    };
    // 32 at a time went over 10 GB (the tall sprites); 8 peak at 5.7 GB.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                while let Some(&(stem, e)) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Err(err) = one(stem, e) {
                        failed.lock().unwrap().push(format!("{}: {err:#}", e.name.as_deref().unwrap_or(stem)));
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n % 1000 == 0 {
                        println!("  {n} of {}", jobs.len());
                    }
                }
            });
        }
    });
    let mb = |a: &AtomicU64| a.load(Ordering::Relaxed) as f64 / 1e6;
    println!("pic: {} written, {:.1} MB from {:.1} MB in the archive", jobs.len() - failed.lock().unwrap().len(), mb(&out_bytes), mb(&in_bytes));
    let failed = failed.into_inner().unwrap();
    for e in failed.iter().take(5) {
        println!("  {e}");
    }
    anyhow::ensure!(failed.is_empty(), "{} pictures failed", failed.len());
    Ok(())
}

/// The movies as they are (H.264 yuv420p 1920 × 1080, some with AAC):
/// remuxed, not re-encoded, with the index first so the web build's
/// `<video>` starts before the file has loaded. Like the other games'
/// movies, at their own size.
/// With `base` (another movie folder), only those that differ from its own
/// in what they show ([`looks_same`]), not merely in their encoding.
fn movies(dir: &Path, out: &Path, base: Option<&Path>) -> Result<()> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    files.sort();
    if let Some(base) = base {
        let mut kept = Vec::new();
        for f in files {
            let other = base.join(f.file_name().unwrap());
            let same_bytes = other.is_file() && fs::metadata(&other)?.len() == fs::metadata(&f)?.len() && map(&other)?[..] == map(&f)?[..];
            if !same_bytes && !(other.is_file() && looks_same(&other, &f)?) {
                kept.push(f);
            }
        }
        files = kept;
    }
    fs::create_dir_all(out)?;
    let mut failed = 0;
    for src in &files {
        let dest = out.join(src.file_name().unwrap().to_string_lossy().to_lowercase());
        if dest.exists() {
            continue;
        }
        let args: Vec<std::ffi::OsString> = ["-i".into(), src.clone().into(), "-c".into(), "copy".into(), "-movflags".into(), "+faststart".into()].into();
        if let Err(e) = conv_engines::media::ffmpeg(&dest, args, |_| Ok(())) {
            println!("  {}: {e}", src.display());
            failed += 1;
        }
    }
    println!("movie: {} of {}", files.len() - failed, files.len());
    anyhow::ensure!(failed == 0, "{failed} movies failed");
    Ok(())
}

/// Two movies show the same: at 200 times spread over them, each frame
/// (grey, 480 × 270) against the other's nearest (one sample either side),
/// the worst 16 × 16 block's mean difference stays within re-encoding
/// noise.
// ours: the threshold, 4 of 255. Measured (the fan patch against 1.0.2):
// the re-encoded titlebg1-4 at most 0.9; a translated phasetitle_* 54-95,
// the subtitled endings 42-62, op1/op2/prologue 143-241, TM_CI (an added
// logo) 183.
fn looks_same(a: &Path, b: &Path) -> Result<bool> {
    const W: usize = 480;
    const H: usize = 270;
    const N: usize = 200;
    let duration = |p: &Path| -> Result<f64> {
        let o = std::process::Command::new("ffprobe").args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"]).arg(p).output()?;
        Ok(String::from_utf8_lossy(&o.stdout).trim().parse()?)
    };
    let fps = N as f64 / duration(a)?.min(duration(b)?);
    let frames = |p: &Path| -> Result<Vec<u8>> {
        let o = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(p)
            .args(["-vf", &format!("fps={fps},scale={W}:{H},format=gray"), "-frames:v", &N.to_string(), "-f", "rawvideo", "-"])
            .output()?;
        anyhow::ensure!(o.status.success(), "ffmpeg: {}", String::from_utf8_lossy(&o.stderr));
        Ok(o.stdout)
    };
    let (fa, fb) = (frames(a)?, frames(b)?);
    let (ca, cb) = (fa.as_chunks::<{ W * H }>().0, fb.as_chunks::<{ W * H }>().0);
    let worst_block = |x: &[u8], y: &[u8]| -> f64 {
        let mut worst = 0u32;
        for by in 0..H / 16 {
            for bx in 0..W / 16 {
                let mut sum = 0;
                for r in 0..16 {
                    let i = (by * 16 + r) * W + bx * 16;
                    sum += x[i..i + 16].iter().zip(&y[i..i + 16]).map(|(p, q)| u32::from(p.abs_diff(*q))).sum::<u32>();
                }
                worst = worst.max(sum);
            }
        }
        f64::from(worst) / 256.0
    };
    for (i, x) in ca.iter().enumerate() {
        let best = (i.saturating_sub(1)..=i + 1).filter_map(|j| cb.get(j)).map(|y| worst_block(x, y)).fold(f64::MAX, f64::min);
        if best > 4.0 {
            return Ok(false);
        }
    }
    Ok(!ca.is_empty())
}

fn scripts(dir: &Path, output: Option<&Path>) -> Result<()> {
    let allscr = map(&dir.join("allscr.mrg"))?;
    let entries = mzp::parse(&allscr).context("reading allscr.mrg")?;
    let names: Vec<String> = mrg::names(entries.first().context("allscr.mrg is empty")?).into_iter().filter(|n| !n.is_empty()).collect();
    let script_text = map(&dir.join("script_text.mrg"))?;
    let pairs = mzp::parse(&script_text).context("reading script_text.mrg")?;
    anyhow::ensure!(pairs.len() >= 2, "script_text.mrg has {} entries", pairs.len());
    let lines = script::text(pairs[0], pairs[1]).context("script_text.mrg")?;
    if let Some(out) = output {
        fs::create_dir_all(out)?;
    }
    // Sound names the scripts can use: wave sounds without `WSD_`, any case; stream files.
    let bfsar = map(&dir.join("knife1.bfsar"))?;
    let waves: std::collections::HashSet<String> = bfsar::wave_sounds(&bfsar)?.iter().map(|w| w.name.trim_start_matches("WSD_").to_uppercase()).collect();
    let streams: std::collections::HashSet<String> = fs::read_dir(dir.join("stream"))?
        .map(|e| Ok(e?.path().file_stem().unwrap().to_string_lossy().into_owned()))
        .collect::<Result<_>>()?;

    // opcode: uses, scripts, first use
    let mut ops = BTreeMap::<String, (usize, usize, String)>::new();
    let (mut odd, mut total, mut used_lines, mut bad_refs) = (Vec::new(), 0, vec![false; lines.len()], 0);
    let mut sounds = BTreeMap::<String, (usize, usize)>::new(); // opcode: (named, found)
    for (i, name) in names.iter().enumerate() {
        let data = typemoon::unpack(entries.get(i + 3).with_context(|| format!("no entry for script {name}"))?).with_context(|| format!("script {name}"))?;
        let text = std::str::from_utf8(&data).with_context(|| format!("script {name}"))?;
        let mut dump = String::new();
        let mut seen = std::collections::HashSet::new();
        for c in script::parse(text) {
            total += 1;
            let c = match c {
                Ok(c) => c,
                Err(raw) => {
                    odd.push(format!("{name}: {raw}"));
                    dump.push_str(&format!("{raw}  # malformed\n"));
                    continue;
                }
            };
            let op = c.opcode().to_string();
            let e = ops.entry(op.clone()).or_insert_with(|| (0, 0, format!("{}({})", c.name, c.args.join(","))));
            e.0 += 1;
            if seen.insert(op.clone()) {
                e.1 += 1;
            }
            let found = match (op.as_str(), c.args.as_slice()) {
                ("_SEPL", [_, _, se, ..]) => Some(waves.contains(&se.to_uppercase())),
                ("_VPLY" | "_VPL2", [voice, ..]) => Some(streams.contains(*voice)),
                _ => None,
            };
            if let Some(found) = found {
                let s = sounds.entry(op.clone()).or_default();
                (s.0, s.1) = (s.0 + 1, s.1 + usize::from(found));
            }
            dump.push_str(&format!("{}({})", c.name, c.args.join(",")));
            for r in c.text_refs() {
                match lines.get(r) {
                    Some(line) => {
                        used_lines[r] = true;
                        dump.push_str(&format!("  « {} »", line.replace("\r\n", "⏎").replace('\n', "⏎")));
                    }
                    None => bad_refs += 1,
                }
            }
            dump.push('\n');
        }
        if let Some(out) = output {
            let file = out.join(safe_relative_path(&format!("{name}.txt")).with_context(|| format!("unsafe name {name:?}"))?);
            fs::write(&file, dump).with_context(|| format!("writing {}", file.display()))?;
        }
    }
    println!(
        "{} scripts, {total} commands, {} opcodes, {} malformed; {} text lines, {} used, {bad_refs} references past the end",
        names.len(),
        ops.len(),
        odd.len(),
        lines.len(),
        used_lines.iter().filter(|&&u| u).count()
    );
    for o in odd.iter().take(5) {
        println!("  malformed: {o}");
    }
    for (op, (named, found)) in &sounds {
        println!("{op}: {named} uses, {found} name a sound in the archive or stream/");
    }
    let mut ranked: Vec<_> = ops.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    println!("| Command | Uses | Scripts | First use |\n|---|---|---|---|");
    for (op, (uses, in_scripts, example)) in ranked {
        let example: String = example.chars().take(60).collect();
        println!("| `{op}` | {uses} | {in_scripts} | `{example}` |");
    }
    Ok(())
}
