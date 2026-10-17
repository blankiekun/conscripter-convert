//! `conscripter-convert typemoon …`: TYPE-MOON's archives (月姫 -A piece
//! of blue glass moon-, Switch): MRG (`.hed`/`.mrg`/`.nam`) and MZP
//! archives, and the BNTX pictures in them.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Subcommand;
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
    /// `<output>/stream/*.ogg`, and every wave sound of `knife1.bfsar` to
    /// `<output>/se/<name>.wav`; counts them and fails on any error
    Sounds {
        dir: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
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
    },
    /// Write a RomFS folder's media for the build into `<output>`: every
    /// `allpac` picture at full size as `pic/<name>.webp` (lossy WebP at
    /// `--quality`, the `RULE_*` masks near-lossless), each from its
    /// largest copy (a bare background's `.JPG`, else the larger `.NXGZ`);
    /// the movies at 1280 × 720 as `movie/<name>.mp4`; and the sound as
    /// `typemoon sounds` writes it. Pictures and movies already there are
    /// kept, so an interrupted run goes on where it stopped
    Media {
        dir: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 85, value_parser = clap::value_parser!(u8).range(0..=100))]
        quality: u8,
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
        TypemoonCommand::Sounds { dir, output } => sounds(&dir, &output),
        TypemoonCommand::Script { dir, output } => scripts(&dir, output.as_deref()),
        TypemoonCommand::Exe { main, romfs } => exe_tables(&main, romfs.as_deref()),
        TypemoonCommand::Media { dir, output, quality } => media(&dir, &output, quality),
        TypemoonCommand::Convert { dir, exe, title, build } => {
            let exe = exe.unwrap_or_else(|| dir.join("../exefs/main"));
            crate::build::run(build, &title, &typemoon::build::OUTPUT_DIRS, |opts| {
                if !opts.dry_run {
                    media(&dir, opts.output, 85)?;
                }
                typemoon::build::build(&dir, &exe, opts)
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


fn exe_tables(main: &Path, romfs: Option<&Path>) -> Result<()> {
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

fn sounds(dir: &Path, output: &Path) -> Result<()> {
    let mut failed = Vec::new();
    let out = output.join("stream");
    fs::create_dir_all(&out)?;
    let mut streams: Vec<PathBuf> = fs::read_dir(dir.join("stream"))?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    streams.sort();
    let (mut count, mut seconds) = (0, 0.0);
    for path in &streams {
        let name = path.file_name().unwrap().to_string_lossy();
        match nxopus::to_ogg(&map(path)?) {
            Ok((ogg, samples)) => {
                fs::write(out.join(path.with_extension("ogg").file_name().unwrap()), ogg)?;
                (count, seconds) = (count + 1, seconds + samples as f64 / 48000.0);
            }
            Err(e) => failed.push(format!("{name}: {e}")),
        }
    }
    println!("stream: {count} of {} to Ogg Opus, {:.1} hours", streams.len(), seconds / 3600.0);

    let out = output.join("se");
    fs::create_dir_all(&out)?;
    let bfsar = map(&dir.join("knife1.bfsar"))?;
    let waves = bfsar::wave_sounds(&bfsar).context("reading knife1.bfsar")?;
    let mut kinds = BTreeMap::<String, usize>::new();
    for w in &waves {
        match fwav::decode(w.fwav) {
            Ok(wave) => {
                let looped = match wave.looped {
                    None => "",
                    Some((0, _)) => ", loops from the start",
                    Some(_) => ", loops from inside",
                };
                *kinds.entry(format!("{} ch {} Hz{looped}", wave.channels, wave.rate)).or_default() += 1;
                let file = out.join(safe_relative_path(&format!("{}.wav", w.name)).with_context(|| format!("unsafe name {:?}", w.name))?);
                fs::write(file, fwav::to_wav(&wave))?;
            }
            Err(e) => failed.push(format!("{}: {e}", w.name)),
        }
    }
    let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
    println!("knife1.bfsar: {} wave sounds ({})", waves.len(), kinds.join("; "));
    for e in failed.iter().take(5) {
        println!("  {e}");
    }
    anyhow::ensure!(failed.is_empty(), "{} sounds failed", failed.len());
    Ok(())
}

fn media(dir: &Path, output: &Path, quality: u8) -> Result<()> {
    pictures(dir, &output.join("pic"), quality)?;
    movies(&dir.join("movie"), &output.join("movie"))?;
    sounds(dir, output)
}

/// `allpac`'s pictures as WebP: per name (case ignored, extension
/// dropped) the `.JPG` if there is one, else the largest entry.
fn pictures(dir: &Path, out: &Path, quality: u8) -> Result<()> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    let path = dir.join("allpac.mrg");
    let bytes = map(&path)?;
    let entries = mrg::parse(&map(&path.with_extension("hed"))?, Some(&map(&path.with_extension("nam"))?)).context("reading allpac.hed")?;
    let best = typemoon::full_copies(&entries);
    fs::create_dir_all(out)?;
    let jobs: Vec<(&String, &&mrg::Entry)> = best.iter().filter(|(stem, _)| !out.join(format!("{stem}.webp")).exists()).collect();
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
        let q = (!stem.starts_with("rule_")).then_some(quality);
        let webp = conv_cons::webp::encode_rgba(&img, q)?;
        let file = out.join(safe_relative_path(&format!("{stem}.webp")).with_context(|| format!("unsafe name {stem:?}"))?);
        let part = file.with_extension("part");
        fs::write(&part, &webp)?;
        fs::rename(&part, &file)?;
        in_bytes.fetch_add(e.size, Ordering::Relaxed);
        out_bytes.fetch_add(webp.len() as u64, Ordering::Relaxed);
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

/// The movies (H.264 1920 × 1080) at the stage's 1280 × 720.
fn movies(dir: &Path, out: &Path) -> Result<()> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    files.sort();
    let mut failed = 0;
    for src in &files {
        let dest = out.join(src.file_name().unwrap().to_string_lossy().to_lowercase());
        if dest.exists() {
            continue;
        }
        let mut args: Vec<std::ffi::OsString> = ["-i".into(), src.clone().into(), "-vf".into(), "scale=1280:-2".into()].into();
        args.extend(conv_engines::media::mp4_options(23, "slow").into_iter().map(Into::into));
        if let Err(e) = conv_engines::media::ffmpeg(&dest, args, |_| Ok(())) {
            println!("  {}: {e}", src.display());
            failed += 1;
        }
    }
    println!("movie: {} of {} at 1280 × 720", files.len() - failed, files.len());
    anyhow::ensure!(failed == 0, "{failed} movies failed");
    Ok(())
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
