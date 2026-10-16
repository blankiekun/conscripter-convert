//! 最果てのイマ's whole build: every script in `HD/CDFiles`, the assets
//! they use, the menus.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use conv_core::ir::Script;
use memmap2::Mmap;

use super::cd::CdScript;
use super::convert::{AssetBank, Flow, LabelNames, Needs, lower_script, presets};
use super::symbols::{self, SymbolTable};
use crate::build::{Built, Opts, map};

/// The output's asset folders (packed into `arc.nsa`).
pub const OUTPUT_DIRS: &[&str] = &["bg", "sprite", "bgm", "se", "system", "voice", "fx"];
/// Not `fx`: its animations are strips up to 47 screens wide, and WebP
/// pictures can't be wider than 16,383 pixels.
pub const PICTURE_DIRS: &[&str] = &["bg", "sprite", "system"];
/// The voices and sound effects `--audio-codec` and `--audio-quality` change.
pub const AUDIO_DIRS: &[&str] = &["voice", "se"];
/// The game's screen.
pub const SCREEN: (u32, u32) = (800, 600);

/// Compiles every `NNNN.cd` script of `game` (the installed game, with
/// `HD/` and `CD/`). `linear`: play the story scripts straight through in
/// file order, skipping the engine-setup ones (names starting with '!'),
/// with no jumps and no menus.
pub fn build(game: &Path, linear: bool, opts: &Opts) -> Result<Built> {
    let dir = crate::find(game, "HD/CDFiles");
    let mut ids: Vec<u32> = fs::read_dir(&dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let stem = name.strip_suffix(".cd")?;
            (stem.len() == 4).then(|| stem.parse().ok()).flatten()
        })
        .collect();
    ids.sort_unstable();
    if ids.is_empty() {
        bail!("no NNNN.cd scripts in {}", dir.display());
    }

    let maps: Vec<(String, Mmap)> = ids
        .iter()
        .map(|id| {
            let file = format!("{id:04}.cd");
            Ok((file.clone(), map(&dir.join(&file))?))
        })
        .collect::<Result<_>>()?;
    let mut scripts: Vec<(String, CdScript)> = maps
        .iter()
        .map(|(file, bytes)| Ok((file.clone(), parse_cd(&dir.join(file), bytes)?)))
        .collect::<Result<_>>()?;
    let flow = if linear { Flow::Linear } else { Flow::Jumps };
    if linear {
        let before = scripts.len();
        scripts.retain(|(_, cd)| !cd.name.starts_with('!'));
        println!("linear mode: skipping {} engine-setup scripts", before - scripts.len());
    }

    let mut needs = Needs::default();
    for (_, cd) in &scripts {
        needs.add_script(cd);
    }
    println!(
        "{} scripts; extracting {} backgrounds, {} sprites, {} BGM tracks, {} sound effects",
        scripts.len(),
        needs.backgrounds.len(),
        needs.sprites.len(),
        needs.bgm.len(),
        needs.se.len()
    );
    let mut assets = AssetBank::new(game, opts.output);
    assets.dry_run = opts.dry_run;
    assets.prefetch(&needs).context("extracting assets")?;
    let window = assets.text_window().context("building the text window")?;

    let presets = presets(&scripts).context("reading the preset subroutines")?;
    let names = label_names(game)?;
    let mut ops = Vec::new();
    let mut metrics = crate::metrics::Metrics::default();
    for (file, cd) in &scripts {
        let lowered = lower_script(cd, file, flow, &mut assets, &presets, &names).with_context(|| format!("converting {file}"))?;
        // Each instruction is a call; its gaps are the unsupported ops it
        // lowered to (their record bytes left out of the reason).
        let mut gaps: std::collections::HashMap<usize, Vec<String>> = Default::default();
        for op in &lowered {
            if let conv_core::ir::Op::Unsupported { what, at } = op {
                gaps.entry(at.index).or_default().push(what.split(" [").next().unwrap_or(what).to_string());
            }
        }
        for (index, ins) in cd.code.iter().enumerate() {
            metrics.call(&format!("opcode {:#04x}", ins.op), gaps.remove(&index).unwrap_or_default());
        }
        ops.extend(lowered);
    }
    // The linear build plays the story straight through, without a menu.
    let (title_screen, save_menu, button_bar, settings, backlog, confirm, extras) = if linear {
        (None, None, None, None, None, None, None)
    } else {
        (
            Some(assets.title_screen().context("building the title screen")?),
            Some(assets.save_menu().context("building the save menu")?),
            Some(assets.button_bar().context("building the button bar")?),
            Some(assets.settings_menu().context("building the settings menu")?),
            Some(assets.backlog().context("building the backlog")?),
            Some(assets.confirm_dialog().context("building the confirmation dialog")?),
            Some(assets.extras_menu(|file, name| names.find(&scripts, file, name)).context("building the extras")?),
        )
    };
    let ir = Script {
        title: opts.title.to_string(),
        width: SCREEN.0,
        height: SCREEN.1,
        window: Some(window),
        title_screen,
        save_menu,
        button_bar,
        settings,
        backlog,
        confirm,
        extras,
        ops,
        ..Default::default()
    };
    Ok(Built { ir, notes: Default::default(), metrics, engine: "xuse", picture_dirs: PICTURE_DIRS, audio_dirs: AUDIO_DIRS, package: None })
}

pub fn parse_cd<'a>(path: &Path, bytes: &'a [u8]) -> Result<CdScript<'a>> {
    CdScript::parse(bytes).with_context(|| format!("parsing {}", path.display()))
}

pub fn read_symbols(path: &Path) -> Result<SymbolTable> {
    let mut bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    symbols::decrypt(&mut bytes);
    SymbolTable::parse(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Label names (chapter titles, link scene titles) from `HD/CDFiles/0.cd`.
pub fn label_names(game: &Path) -> Result<LabelNames> {
    Ok(LabelNames::new(&read_symbols(&crate::find(game, "HD/CDFiles/0.cd"))?))
}
