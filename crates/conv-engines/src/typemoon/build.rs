//! 月姫's build from a RomFS folder: every script (`allscr.mrg`) with its
//! text, the exe's picture records, and the text window. The pictures,
//! movies and sounds are the media folder `typemoon media` writes into the
//! same output (`pic/`, `movie/`, `stream/`, `se/`).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use conv_core::ir::{Script, TextWindow};

use super::convert::{self, Game};
use super::{bfsar, bntx, exe, mrg, mzp, script};
use crate::build::{Built, Opts, map};

/// The output's asset folders (packed into `arc.nsa`).
/// Not `movie`: CONscripter plays movies from loose files only.
pub const OUTPUT_DIRS: [&str; 5] = ["pic", "cg", "stream", "se", "sys"];
/// The text window's picture, written by the build.
const WINDOW: &str = "sys/txtwindow.png";

/// Every script by name with its commands ([`Game::scripts`]).
type Scripts<'a> = Vec<(String, Vec<script::Command<'a>>)>;

/// One RomFS copy's scripts and text, read.
struct Source {
    names: Vec<String>,
    data: Vec<Vec<u8>>,
    script_text: memmap2::Mmap,
}

impl Source {
    fn read(romfs: &Path) -> Result<Source> {
        let allscr = map(&romfs.join("allscr.mrg"))?;
        let entries = mzp::parse(&allscr).context("reading allscr.mrg")?;
        let names: Vec<String> = mrg::names(entries.first().context("allscr.mrg is empty")?).into_iter().filter(|n| !n.is_empty()).collect();
        let data: Vec<Vec<u8>> = (0..names.len())
            .map(|i| Ok(super::unpack(entries.get(i + 3).with_context(|| format!("no entry for script {}", names[i]))?)?.into_owned()))
            .collect::<Result<_>>()?;
        Ok(Source { names, data, script_text: map(&romfs.join("script_text.mrg"))? })
    }

    /// Every script with its commands, and the text lines.
    fn scripts(&self) -> Result<(Scripts<'_>, Vec<&str>)> {
        let mut scripts = Vec::new();
        for (name, d) in self.names.iter().zip(&self.data) {
            let text = std::str::from_utf8(d).with_context(|| format!("script {name}"))?;
            // The one malformed command of each copy is left out.
            scripts.push((name.clone(), script::parse(text).into_iter().filter_map(Result::ok).collect()));
        }
        let pairs = mzp::parse(&self.script_text).context("reading script_text.mrg")?;
        anyhow::ensure!(pairs.len() >= 2, "script_text.mrg has {} entries", pairs.len());
        Ok((scripts, script::text(pairs[0], pairs[1]).context("script_text.mrg")?))
    }
}

/// The text window: the text drawer's layout (`FUN_001391a0`, on a 640 ×
/// 448 stage): in Japanese lines from x 88 (the panel's flat middle,
/// 176–1104 here), 464 wide, the first at y 65; glyphs of the screen's
/// width / 32 × 0.76 (`FUN_001a7790`, the text size setting at its
/// default). In English from x 30, 584 wide, the first line at y 55,
/// glyphs × 0.9. Lines are the TextWriter's line feed apart: its PairFont
/// takes the larger feed of exfont_nx01.bffnt (47) and the ScalableFont
/// (50: Seurat's win ascent + descent, 1.246 em × 40, `FUN_00243ed0`), ×
/// the glyph scale, + a line space of 16 px.
fn text_window(english: bool) -> TextWindow {
    TextWindow {
        image: WINDOW.into(),
        x: 0, // src: txtwindow01-1920 (a full-screen panel)
        y: 0,
        text_x: if english { 60 } else { 176 },  // src: exe 0x1391a0 (30 or 88, × 2)
        text_y: if english { 88 } else { 104 },  // src: exe 0x1391a0 ((short)(65 − 9.88) = 55 or 65, × 720 / 448); FUN_001a7790's (x + 4, y + 14) pair is the TextWriter's char/line space (writer +0x40/+0x44), not an origin
        columns: if english { 41 } else { 30 },  // src: exe 0x1391a0 (584 × 2 = 1168 px / 28, or 928 px / 30.4); the window types ≥ 3 (table 0x44DEFC: 299.8, 405.6, 509.8) are the same in both languages and not drawn
        rows: if english { 13 } else { 11 }, // ours: the exe has 16 line slots (0x1391a0, 0xBFECCC…0xBFED08), the last ones below the screen; these are the slots that fit 720 px
        font_size: if english { 27 } else { 30 }, // src: exe 0x1a7790 (1280 / 32 × 0.76 = 30.4, × 0.9 in English = 27.4)
        line_gap: if english { 23 } else { 24 }, // src: exe 0x1391a0's slot pitch, the TextWriter cursor y after " ^ " ('\n', FUN_00240cc8) = FUN_00245170: line feed 50 × scaleY + line space 16 = 54.0 or 50.2 px (research/typemoon/linefeed_exe.py), less font_size
        vertical: false,
        name_plate: None,
        wait_icon_at: None,
    }
}

/// Builds the game from `romfs` (1.0.2's or the fan patch's RomFS) and
/// `main`, 1.0.2's exe. With `english` (the fan patch's RomFS beside
/// 1.0.2's), both languages in one script (`language.rs`).
pub fn build(romfs: &Path, main: &Path, english: Option<&Path>, opts: &Opts) -> Result<Built> {
    let source = Source::read(romfs)?;
    let (scripts, lines) = source.scripts()?;
    // The fan patch's text is English (the exe's language check, `FUN_0012df20`).
    let is_english = lines.iter().filter(|l| !l.chars().any(|c| ('\u{3040}'..='\u{30ff}').contains(&c))).count() * 2 > lines.len();
    let exe = exe::Exe::parse(&map(main)?).with_context(|| format!("reading {}", main.display()))?;
    let picture_list = exe.pictures()?;
    let pictures: HashMap<String, exe::Picture> = picture_list.iter().map(|p| (p.name.to_lowercase(), p.clone())).collect();
    let bfsar = map(&romfs.join("knife1.bfsar"))?;
    let ext = if opts.audio.reencodes() { "ogg" } else { "wav" }; // as `typemoon sounds` writes them
    let waves = bfsar::wave_sounds(&bfsar)?.into_iter().map(|w| (w.name.trim_start_matches("WSD_").to_uppercase(), format!("{}.{ext}", w.name))).collect::<HashMap<_, _>>();
    let sounds = stream_names(romfs, bfsar::sound_names(&bfsar)?)?;
    let pic_ext = if opts.webp.is_some() { "webp" } else { "png" }; // as `typemoon media` writes them
    let game = Game { scripts, scenes: exe.scenes()?, lines, textures: textures(romfs, &pictures)?, pictures, sounds, waves, pic_ext, motions: exe.motions()?, movies: exe.movies()? };
    let en_source = english.map(Source::read).transpose()?;
    let own = english.map(|en| own_pictures(romfs, en)).transpose()?.unwrap_or_default();
    let mut compiled = match (english, &en_source) {
        (Some(en_dir), Some(en)) => {
            let (scripts, lines) = en.scripts()?;
            let en_game = Game { scripts, lines, textures: textures(en_dir, &game.pictures)?, pictures: game.pictures.clone(), scenes: game.scenes.clone(), sounds: game.sounds.clone(), waves: game.waves.clone(), motions: game.motions.clone(), movies: en_movies(romfs, en_dir, &game.movies)?, pic_ext };
            super::language::compile(&game, &en_game, &own, text_window(false), text_window(true))
        }
        _ => convert::compile(&game),
    };
    // src: script_command, case "PLY": a voice's character from its
    // name's first 3 letters; its row of the settings' voices is the
    // character a voice switch is kept for.
    let (rows, other) = exe.voice_rows()?;
    for op in &mut compiled.ops {
        if let conv_core::ir::Op::Text { voice: Some(v), .. } = op {
            v.character = v.file.trim_start_matches("stream/").get(..3).and_then(|p| rows.get(p)).copied().unwrap_or(other);
        }
    }
    compiled.ops.extend(super::gallery::ending_ops(en_source.is_some()));
    crate::output::record(WINDOW);
    if !opts.dry_run {
        window(romfs, &opts.output.join(WINDOW))?;
    }
    super::question::write(romfs, opts.output, opts.font, false, opts.dry_run)?;
    if let Some(en) = english {
        super::question::write(en, opts.output, opts.font, true, opts.dry_run)?;
    }
    let ir = Script {
        title: opts.title.to_string(),
        width: convert::SCREEN_W,
        height: convert::SCREEN_H,
        global_border: Some(convert::GLOBAL_BORDER),
        window: Some(text_window(is_english)),
        ops: compiled.ops,
        source_map: compiled.source_map,
        // src: exe 0x56b080 (the settings' defaults, also the reset in
        // FUN_0012c810): unread text 35 ms a character (+0xe), read text
        // 15 ms (+0x10); FUN_0013b2d0 shows 16.67 / ms characters a frame.
        // One speed here: the unread one.
        text_ms: Some(35),
        title_screen: Some(super::title::title(romfs, opts.output, opts.dry_run, ext)?),
        picture_extras: Some(super::gallery::extras(&exe, &picture_list, &super::gallery::Sources { romfs, output: opts.output, font: opts.font, cgs: &gallery_cgs(romfs)?, own: &own, pic_ext, dry_run: opts.dry_run }, super::gallery::FLAG_557, &convert::cg_flags(&game.scripts))?),
        system_menu: Some(super::menu::system_menu(romfs, opts.output, opts.dry_run, english.is_some())?),
        settings: Some(super::settings::settings(romfs, opts.output, opts.font, opts.dry_run, english.map(|_| super::language::PREF))?),
        boot: super::boot::steps(romfs, english.map(|en| (en, super::language::PREF)), opts.output, opts.dry_run)?,
        language: if is_english { "en" } else { "ja" }.into(),
        after_load: english.map(|_| super::language::LOADED.to_string()),
        ..Default::default()
    };
    println!("{} scripts -> {}", game.scripts.len(), opts.output.join("0.txt").display());
    Ok(Built { ir, notes: compiled.notes, metrics: compiled.metrics, engine: "typemoon", engine_doc: "typemoon", picture_dirs: &["sys"], audio_dirs: &[], package: None, translation: None, asset_errors: 0 })
}

/// `allpaccg`'s CGs (`typemoon media` writes them to `cg/`), lowercase.
fn gallery_cgs(romfs: &Path) -> Result<std::collections::HashSet<String>> {
    let path = romfs.join("allpaccg.mrg");
    let entries = mrg::parse(&map(&path.with_extension("hed"))?, Some(&map(&path.with_extension("nam"))?)).context("reading allpaccg.hed")?;
    Ok(super::full_copies(&entries).into_keys().filter(|k| k.starts_with("img")).collect())
}

/// The pictures the fan patch has its own of (`pic/en/`, as `typemoon
/// media --english` writes them), lowercase.
fn own_pictures(ja: &Path, en: &Path) -> Result<std::collections::HashSet<String>> {
    let read = |dir: &Path| -> Result<(memmap2::Mmap, memmap2::Mmap, memmap2::Mmap)> {
        let path = dir.join("allpac.mrg");
        Ok((map(&path)?, map(&path.with_extension("hed"))?, map(&path.with_extension("nam"))?))
    };
    let (jb, jh, jn) = read(ja)?;
    let (eb, eh, en_) = read(en)?;
    let je = mrg::parse(&jh, Some(&jn)).context("reading allpac.hed")?;
    let ee = mrg::parse(&eh, Some(&en_)).context("reading allpac.hed")?;
    Ok(super::differing(&super::full_copies(&je), &jb, &super::full_copies(&ee), &eb)?.into_iter().collect())
}

/// `movies` (the exe's table) as the English copy plays them: from
/// `movie/en/` where the fan patch's file differs from 1.0.2's (as
/// `typemoon media --english` keeps them; the four title backgrounds
/// it drops as looking the same are not `_TOMP`'s).
fn en_movies(ja: &Path, en: &Path, movies: &[String]) -> Result<Vec<String>> {
    movies
        .iter()
        .map(|m| {
            let (a, b) = (ja.join(m), en.join(m));
            let same = !b.is_file() || a.is_file() && fs::metadata(&a)?.len() == fs::metadata(&b)?.len() && map(&a)?[..] == map(&b)?[..];
            Ok(if same { m.clone() } else { m.replacen("movie/", "movie/en/", 1) })
        })
        .collect()
}

/// The size of each picture's full art in `allpac` (what `typemoon media`
/// writes), for the pictures with a record.
fn textures(romfs: &Path, pictures: &HashMap<String, exe::Picture>) -> Result<HashMap<String, (u32, u32)>> {
    let path = romfs.join("allpac.mrg");
    let bytes = map(&path)?;
    let entries = mrg::parse(&map(&path.with_extension("hed"))?, Some(&map(&path.with_extension("nam"))?)).context("reading allpac.hed")?;
    let mut sizes = HashMap::new();
    for (name, e) in super::full_copies(&entries) {
        if pictures.contains_key(&name) {
            let size = super::picture_size(mrg::data(&bytes, e)?).with_context(|| format!("allpac: {name}"))?;
            sizes.insert(name, size);
        }
    }
    Ok(sizes)
}

/// The text window (`txtwindow01-1920` in `allui`'s `DISP_PARTS_JA`, a
/// 1920 × 1080 panel) at the screen's size.
fn window(romfs: &Path, dest: &Path) -> Result<()> {
    let path = romfs.join("allui.mrg");
    let hed = map(&path.with_extension("hed"))?;
    let nam = map(&path.with_extension("nam"))?;
    let bytes = map(&path)?;
    let entry = mrg::parse(&hed, Some(&nam))?.into_iter().find(|e| e.name.as_deref().is_some_and(|n| n.starts_with("DISP_PARTS"))).context("allui has no DISP_PARTS")?;
    let data = super::unpack(mrg::data(&bytes, &entry)?)?;
    let texture = bntx::parse(&data)?.into_iter().find(|t| t.name == "txtwindow01-1920").context("no txtwindow01-1920 in DISP_PARTS")?;
    let img = bntx::decode(&texture)?;
    let img = image::imageops::resize(&img, convert::SCREEN_W, convert::SCREEN_H, image::imageops::FilterType::Triangle);
    fs::create_dir_all(dest.parent().unwrap())?;
    img.save(dest).with_context(|| format!("writing {}", dest.display()))
}

/// The bfsar names a stream by its file's name in capitals with every
/// other character than a letter or digit as `_` (`STRM_ARCSTAFFROLL_SHORT_`
/// for `stream/arcstaffroll(short).opus`): each `STRM_` name with the
/// file's own name, as the media folder writes it.
fn stream_names(romfs: &Path, names: Vec<String>) -> Result<Vec<String>> {
    let key = |n: &str| n.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect::<String>();
    let mut files = HashMap::new();
    for e in fs::read_dir(romfs.join("stream")).context("reading stream/")? {
        let path = e?.path();
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()).filter(|_| path.extension().is_some_and(|x| x == "opus")) {
            files.insert(key(stem), stem.to_string());
        }
    }
    Ok(names
        .into_iter()
        .map(|n| match n.strip_prefix("STRM_").and_then(|k| files.get(k)) {
            Some(file) => format!("STRM_{file}"),
            None => n,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_stream_is_named_as_its_file() {
        let dir = std::env::temp_dir().join(format!("tm-stream-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("stream")).unwrap();
        for f in ["arcstaffroll(short).opus", "Believer_Tape_FinalMix_Take1-2.opus", "MOON01_BGM01.opus"] {
            std::fs::write(dir.join("stream").join(f), b"").unwrap();
        }
        let names = ["STRM_ARCSTAFFROLL_SHORT_", "STRM_BELIEVER_TAPE_FINALMIX_TAKE1_2", "STRM_MOON01_BGM01", ""].map(String::from).to_vec();
        let got = super::stream_names(&dir, names).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(got, ["STRM_arcstaffroll(short)", "STRM_Believer_Tape_FinalMix_Take1-2", "STRM_MOON01_BGM01", ""]);
    }
}
