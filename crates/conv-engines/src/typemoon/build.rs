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
pub const OUTPUT_DIRS: [&str; 5] = ["pic", "movie", "stream", "se", "sys"];
/// The text window's picture, written by the build.
const WINDOW: &str = "sys/txtwindow.png";

/// Builds the game from `romfs` (1.0.2's or the fan patch's RomFS) and
/// `main`, 1.0.2's exe.
pub fn build(romfs: &Path, main: &Path, opts: &Opts) -> Result<Built> {
    let allscr = map(&romfs.join("allscr.mrg"))?;
    let entries = mzp::parse(&allscr).context("reading allscr.mrg")?;
    let names: Vec<String> = mrg::names(entries.first().context("allscr.mrg is empty")?).into_iter().filter(|n| !n.is_empty()).collect();
    let data: Vec<Vec<u8>> = (0..names.len())
        .map(|i| Ok(super::unpack(entries.get(i + 3).with_context(|| format!("no entry for script {}", names[i]))?)?.into_owned()))
        .collect::<Result<_>>()?;
    let mut scripts = Vec::new();
    for (name, d) in names.iter().zip(&data) {
        let text = std::str::from_utf8(d).with_context(|| format!("script {name}"))?;
        // The one malformed command of each copy is left out.
        scripts.push((name.clone(), script::parse(text).into_iter().filter_map(Result::ok).collect()));
    }
    let script_text = map(&romfs.join("script_text.mrg"))?;
    let pairs = mzp::parse(&script_text).context("reading script_text.mrg")?;
    anyhow::ensure!(pairs.len() >= 2, "script_text.mrg has {} entries", pairs.len());
    let lines = script::text(pairs[0], pairs[1]).context("script_text.mrg")?;
    let exe = exe::Exe::parse(&map(main)?).with_context(|| format!("reading {}", main.display()))?;
    let pictures: HashMap<String, exe::Picture> = exe.pictures()?.into_iter().map(|p| (p.name.to_lowercase(), p)).collect();
    let textures = textures(romfs, &pictures)?;
    let bfsar = map(&romfs.join("knife1.bfsar"))?;
    let waves = bfsar::wave_sounds(&bfsar)?.into_iter().map(|w| (w.name.trim_start_matches("WSD_").to_uppercase(), w.name)).collect::<HashMap<_, _>>();
    let game = Game { scripts, scenes: exe.scenes()?, lines, pictures, textures, sounds: bfsar::sound_names(&bfsar)?, waves, motions: exe.motions()? };
    let compiled = convert::compile(&game);
    if !opts.dry_run {
        window(romfs, &opts.output.join(WINDOW))?;
    }
    let ir = Script {
        title: opts.title.to_string(),
        width: convert::SCREEN_W,
        height: convert::SCREEN_H,
        global_border: Some(convert::GLOBAL_BORDER),
        // shortcut: the text's place and size are guessed from the window picture (a full-screen panel); measure the original's.
        window: Some(TextWindow {
            image: WINDOW.into(),
            x: 0,
            y: 0,
            text_x: 176,
            text_y: 64,
            columns: 32,
            rows: 14,
            font_size: 28,
            line_gap: 16,
            vertical: false,
            name_plate: None,
            wait_icon_at: None,
        }),
        ops: compiled.ops,
        ..Default::default()
    };
    println!("{} scripts -> {}", game.scripts.len(), opts.output.join("0.txt").display());
    Ok(Built { ir, notes: compiled.notes, metrics: compiled.metrics, engine: "typemoon", picture_dirs: &["sys"], audio_dirs: &[], package: None })
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
