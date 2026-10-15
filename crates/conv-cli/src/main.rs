use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use memmap2::Mmap;
use conv_core::ir::Script;
use conv_engines::media::AudioFormat;
use conv_engines::xuse::cd::CdScript;
use conv_engines::xuse::convert::{AssetBank, Flow, Needs, lower_script, presets};

mod build;
mod mages;
mod tools;
mod version;

use tools::nitroplus::*;
use tools::xuse::*;

#[derive(Parser)]
#[command(name = "conscripter-convert", about = "Convert Japanese visual novels to CONscripter")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert the whole game into a CONscripter game directory: 0.txt plus
    /// arc.nsa (arc1.nsa, …) with every asset the scripts use
    ConvertGame {
        /// Installed game directory (contains HD/ and CD/)
        game: PathBuf,
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        /// Play story scripts straight through in file order: skip the
        /// engine-setup scripts (names starting with '!') and convert no
        /// jumps. Stopgap until conditions are decoded.
        #[arg(long)]
        linear: bool,
        #[command(flatten)]
        build: build::BuildArgs,
    },
    /// Convert one Xuse script (CDFiles/NNNN.cd) into a CONscripter game
    /// directory: 0.txt plus the assets it uses
    ConvertScript {
        /// Installed game directory (contains HD/ and CD/)
        game: PathBuf,
        /// Script number (the NNNN in CDFiles/NNNN.cd)
        #[arg(long)]
        script: u32,
        #[arg(short, long)]
        output: PathBuf,
        /// Font to copy into the output as default.ttf (CONscripter needs one)
        #[arg(long)]
        font: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        #[command(flatten)]
        webp: build::WebpArgs,
        #[command(flatten)]
        audio: build::AudioArgs,
    },
    /// Convert one Nitroplus story scene (装甲悪鬼村正) into a CONscripter
    /// game directory: 0.txt plus the assets it uses
    ConvertNss {
        /// Game data directory (Muramasa.exe or Django.exe and the *.npa archives)
        game: PathBuf,
        /// Script file and scene; the scene defaults to the file's name
        /// (its content scene). Without it, the game's whole story
        /// (続・殺戮のジャンゴ; 装甲悪鬼村正 needs --file and --chain)
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        scene: Option<String>,
        /// Play this many story files in a row from --file, following
        /// $GameName (from each file's _MAIN scene, as the game does).
        /// A new game starts at ma00_000.nss (the boot script's start; it
        /// also unlocks 魔王編 once both other routes are cleared)
        #[arg(long)]
        chain: Option<usize>,
        /// The movie folder (default: the game's own `movie`, as
        /// 続・殺戮のジャンゴ has, else `ムービー` beside the game data, as on
        /// 装甲悪鬼村正's disc)
        #[arg(long)]
        movies: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        /// The English release's folder (Full Metal Daemon Muramasa, its
        /// *.npk): its text replaces the Japanese
        #[arg(long)]
        english: Option<PathBuf>,
        #[command(flatten)]
        build: build::BuildArgs,
    },
    /// Check a converted game directory's 0.txt: missing labels, effect
    /// numbers past 255, half-width characters in vertical text, files
    /// that aren't in the archives
    Lint { dir: PathBuf },
    /// Low-level tools for Xuse XARC archives (*.arc)
    Xarc {
        #[command(subcommand)]
        command: XarcCommand,
    },
    /// Low-level tools for Xuse compiled scripts (CDFiles/*.cd)
    Cd {
        #[command(subcommand)]
        command: CdCommand,
    },
    /// Low-level tools for Xuse GAF4 image containers (*.004)
    Gaf {
        #[command(subcommand)]
        command: GafCommand,
    },
    /// Low-level tools for Xuse sound archives (*.xsa, HD/*.bin sound banks)
    Xsa {
        #[command(subcommand)]
        command: XsaCommand,
    },
    /// Low-level tools for Nitroplus NPA archives (*.npa)
    Npa {
        #[command(subcommand)]
        command: NpaCommand,
    },
    /// Low-level tools for Nitroplus NSS scripts (*.nss, from nss.npa)
    Nss {
        #[command(subcommand)]
        command: NssCommand,
    },
    /// Tools for 5pb./MAGES' TaskList engine (俺たちに翼はない, PS Vita):
    /// CRI archives and scripts
    Mages {
        #[command(subcommand)]
        command: mages::MagesCommand,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::ConvertGame { game, title, linear, build: build::BuildArgs { output, font, loose, dry_run, webp, audio } } => {
            convert_game(&game, &output, font.as_deref(), &title, loose, linear, dry_run, webp.get(), audio.format()?)
        }
        Command::ConvertScript { game, script, output, font, title, webp, audio } => {
            convert_script(&game, script, &output, font.as_deref(), &title, webp.get(), audio.format()?)
        }
        Command::ConvertNss { game, file, scene, chain, movies, title, english, build: build::BuildArgs { output, font, loose, dry_run, webp, audio } } => {
            // 続・殺戮のジャンゴ's movie/ is in the game folder; 装甲悪鬼村正's
            // ムービー beside it.
            let movies = movies.unwrap_or_else(|| if game.join("movie").is_dir() { game.join("movie") } else { game.join("../ムービー") });
            convert_nss(&game, file.as_deref(), scene.as_deref(), chain, &movies, &output, font.as_deref(), &title, loose, english.as_deref(), dry_run, webp.get(), audio.format()?)
        }
        Command::Lint { dir } => {
            let report = build::lint_dir(&dir, true)?;
            print!("{report}");
            if !report.is_empty() {
                bail!("problems found");
            }
            Ok(())
        }
        Command::Xarc { command } => match command {
            XarcCommand::List { archive } => xarc_list(&archive),
            XarcCommand::Verify { archives } => xarc_verify(&archives),
            XarcCommand::Extract { archive, output } => xarc_extract(&archive, &output),
        },
        Command::Cd { command } => match command {
            CdCommand::Info { scripts } => cd_info(&scripts),
            CdCommand::Dump { script } => cd_dump(&script),
            CdCommand::Messages { script } => cd_messages(&script),
            CdCommand::Strings { script } => cd_strings(&script),
            CdCommand::Opstats { scripts } => cd_opstats(&scripts),
            CdCommand::Symbols { file, script } => cd_symbols(&file, script),
        },
        Command::Gaf { command } => match command {
            GafCommand::List { files } => gaf_list(&files),
            GafCommand::Names { file } => gaf_names(&file),
            GafCommand::Extract { files, output } => gaf_extract(&files, &output),
        },
        Command::Xsa { command } => match command {
            XsaCommand::List { archive } => xsa_list(&archive),
            XsaCommand::Extract { archives, output } => xsa_extract(&archives, &output),
        },
        Command::Npa { command } => match command {
            NpaCommand::List { exe, archive } => npa_list(&exe, &archive),
            NpaCommand::Extract { exe, archive, output, prefix } => npa_extract(&exe, &archive, &output, &prefix),
        },
        Command::Nss { command } => match command {
            NssCommand::Check { paths } => nss_check(&paths),
            NssCommand::Refs { dir } => nss_refs(&dir),
            NssCommand::Route { dir } => nss_route(&dir),
            NssCommand::Builtins { dir, names, top } => nss_builtins(&dir, &names, top),
        },
        Command::Mages { command } => mages::run(command),
    }
}

const ASSET_DIRS: [&str; 7] = ["bg", "sprite", "bgm", "se", "system", "voice", "fx"];
/// Not `fx`: its animations are strips up to 47 screens wide, and WebP
/// pictures can't be wider than 16,383 pixels.
const IMAGE_DIRS: [&str; 3] = ["bg", "sprite", "system"];
/// The voices and sound effects `--audio-codec` and `--audio-quality` change.
const AUDIO_DIRS: [&str; 2] = ["voice", "se"];

/// With `webp` (Some: the lossy quality, if any), re-encodes the images in
/// `output` as WebP and points the script at them.
fn image_format(output: &Path, source: String, webp: Option<Option<u8>>) -> Result<String> {
    let Some(quality) = webp else {
        return Ok(source);
    };
    let dirs: Vec<PathBuf> = IMAGE_DIRS.iter().map(|d| output.join(d)).collect();
    let c = conv_cons::webp::convert_dirs(&dirs, quality).context("converting images to WebP")?;
    println!(
        "  {} images as WebP: {:.1} MB (PNG {:.1} MB)",
        c.files,
        c.webp_bytes as f64 / 1e6,
        c.png_bytes as f64 / 1e6
    );
    Ok(conv_cons::webp::retarget(&source))
}

#[allow(clippy::too_many_arguments)]
fn convert_game(
    game: &Path,
    output: &Path,
    font: Option<&Path>,
    title: &str,
    loose: bool,
    linear: bool,
    dry_run: bool,
    webp: Option<Option<u8>>,
    audio: AudioFormat,
) -> Result<()> {
    let dir = game.join("HD").join("CDFiles");
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
    build::check_output(output, &ASSET_DIRS, !loose && !dry_run)?;

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
    let mut assets = AssetBank::new(game, output);
    assets.dry_run = dry_run;
    assets.prefetch(&needs).context("extracting assets")?;
    let window = assets.text_window().context("building the text window")?;

    let presets = presets(&scripts).context("reading the preset subroutines")?;
    let names = label_names(game)?;
    let mut ops = Vec::new();
    for (file, cd) in &scripts {
        ops.extend(lower_script(cd, file, flow, &mut assets, &presets, &names).with_context(|| format!("converting {file}"))?);
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
        title: title.to_string(),
        width: 800,
        height: 600,
        window: Some(window),
        title_screen,
        save_menu,
        button_bar,
        settings,
        backlog,
        confirm,
        extras,
        ops,
        version: Some(version::line()),
        ..Default::default()
    };
    if dry_run {
        // The script only: no assets, so nothing to convert, pack or find.
        let lost = build::write_game(output, &ir, &conv_cons::emit(&ir), None, None)?;
        return build::summary(output, &ir, lost, font.is_some(), Default::default(), Some(false));
    }
    let source = image_format(output, conv_cons::emit(&ir), webp)?;
    build::reencode_audio(output, &AUDIO_DIRS, audio)?;
    let lost = build::write_game(output, &ir, &source, font, None)?;
    if !loose {
        build::pack(output, &ASSET_DIRS)?;
    }
    build::summary(output, &ir, lost, font.is_some(), Default::default(), Some(true))
}

fn convert_script(game: &Path, script: u32, output: &Path, font: Option<&Path>, title: &str, webp: Option<Option<u8>>, audio: AudioFormat) -> Result<()> {
    let file = format!("{script:04}.cd");
    let path = game.join("HD").join("CDFiles").join(&file);
    let bytes = map(&path)?;
    let cd = parse_cd(&path, &bytes)?;
    fs::create_dir_all(output)?;

    let mut assets = AssetBank::new(game, output);
    let window = assets.text_window().context("building the text window")?;
    let presets = Default::default(); // the presets live in script 2
    let names = label_names(game)?;
    let ops =
        lower_script(&cd, &file, Flow::Jumps, &mut assets, &presets, &names).with_context(|| format!("converting {file}"))?;
    let ir = Script { title: title.to_string(), width: 800, height: 600, window: Some(window), ops, version: Some(version::line()), ..Default::default() };

    let source = image_format(output, conv_cons::emit(&ir), webp)?;
    build::reencode_audio(output, &AUDIO_DIRS, audio)?;
    let lost = build::write_game(output, &ir, &source, font, None)?;
    println!("{file} ({}) -> {}", cd.name, output.join("0.txt").display());
    build::summary(output, &ir, lost, font.is_some(), Default::default(), None)
}

pub(crate) fn map(path: &Path) -> Result<Mmap> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    // SAFETY: the archive is opened read-only and not modified while mapped.
    unsafe { Mmap::map(&file) }.with_context(|| format!("mapping {}", path.display()))
}

/// Turns an archive entry name into a relative path that stays inside the
/// output directory. Backslashes are treated as directory separators.
pub(crate) fn safe_relative_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name.replace('\\', "/"));
    // ':' is rejected so Windows drive prefixes ("C:") can't slip through.
    let ok = !name.is_empty()
        && !name.contains(':')
        && path.components().all(|c| matches!(c, Component::Normal(_)));
    ok.then_some(path)
}

#[allow(clippy::too_many_arguments)]
fn convert_nss(
    game: &Path,
    file: Option<&str>,
    scene: Option<&str>,
    chain: Option<usize>,
    movies: &Path,
    output: &Path,
    font: Option<&Path>,
    title: &str,
    loose: bool,
    english: Option<&Path>,
    dry_run: bool,
    webp: Option<Option<u8>>,
    audio: AudioFormat,
) -> Result<()> {
    use conv_engines::nitroplus::assets::{NpaAssets, OUTPUT_DIRS, PICTURE_DIRS};
    use conv_engines::nitroplus::{convert, nss};
    // The first Nitroplus exe found names the game.
    let exe = ["Muramasa.exe", "Django.exe"].iter().map(|n| game.join(n)).find(|p| p.exists()).unwrap_or_else(|| game.join("Muramasa.exe"));
    build::check_output(output, &OUTPUT_DIRS, !loose && !dry_run)?;
    // Read entry by entry, not mapped: the mapped pages of cg.npa and
    // voice.npa (3 GB) would count as the converter's memory.
    let key = npa_key(&exe)?;
    let mut npas = Vec::new();
    for n in ["nss", "cg", "sound", "voice"] {
        let path = game.join(format!("{n}.npa"));
        let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
        npas.push(conv_engines::nitroplus::npa::Npa::open(file, key.clone()).with_context(|| format!("parsing {}", path.display()))?);
    }
    let mut npas = npas.into_iter();
    let scripts = npas.next().unwrap();
    let mut files = std::collections::BTreeMap::new();
    // 続・殺戮のジャンゴ 1.01's nss.npa also holds SourceSafe's vssver2.scc.
    for e in scripts.entries.iter().filter(|e| e.name.to_lowercase().ends_with(".nss")) {
        let parsed = nss::parse(&nss::decode(&scripts.data(e)?)).with_context(|| format!("parsing {}", e.name))?;
        let name = e.name.rsplit('\\').next().unwrap_or(&e.name).to_string();
        files.insert(name, parsed);
    }
    if let Some(dir) = english {
        let path = dir.join("script.npk");
        let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
        let npk = conv_engines::nitroplus::npk::Npk::open(file).with_context(|| format!("parsing {}", path.display()))?;
        let report = conv_engines::nitroplus::english::English::load(&npk)?.apply(&mut files);
        println!("English: {} stretches of text replaced, {} left in Japanese", report.replaced, report.missed.len());
        for (file, jp) in report.missed.iter().take(5) {
            println!("  {file}: {}", jp.chars().take(40).collect::<String>());
        }
    }
    let (cg, sound, voice) = (npas.next().unwrap(), npas.next().unwrap(), npas.next().unwrap());
    let mut assets = NpaAssets::new(cg, sound, voice, output);
    assets.out.dry_run = dry_run;
    // 続・殺戮のジャンゴ 1.01's own pictures.
    let cg2 = game.join("cg2.npa");
    if cg2.exists() {
        let file = File::open(&cg2).with_context(|| format!("opening {}", cg2.display()))?;
        assets.cg2 = Some(conv_engines::nitroplus::npa::Npa::open(file, key.clone()).with_context(|| format!("parsing {}", cg2.display()))?);
    }
    if movies.is_dir() {
        assets.movie_dir = Some(movies.to_path_buf());
    } else {
        println!("no movie folder at {}: the movies are left out", movies.display());
    }
    // The game's own additions to the compiler, by its exe.
    let game = match exe.file_name().and_then(|n| n.to_str()) {
        Some("Muramasa.exe") => &convert::MURAMASA,
        Some("Django.exe") => &convert::DJANGO,
        _ => &convert::Game::PLAIN,
    };
    assets.screen = game.screen;
    let (file, scene) = match (file, game.story) {
        (Some(file), _) => (file, scene.unwrap_or(file)),
        (None, Some((file, story))) => (file, scene.unwrap_or(story)),
        (None, None) => anyhow::bail!("this game needs --file"),
    };
    let compiled = match chain {
        _ if game.story.is_some_and(|s| s.0 == file) => convert::compile_story(&files, file, scene, game, &mut assets).map_err(anyhow::Error::msg)?,
        Some(n) => {
            let (compiled, played) = convert::compile_run(&files, file, n, game, &mut assets).map_err(anyhow::Error::msg)?;
            println!("{} files: {}", played.len(), played.join(" "));
            compiled
        }
        None => convert::compile_scene(&files, file, scene, game, &mut assets).map_err(anyhow::Error::msg)?,
    };
    for e in &assets.out.errors {
        println!("  asset error: {e}");
    }
    if std::env::var_os("NSS_TRACE").is_some() {
        let mut flags: Vec<_> = compiled.flags.iter().map(|(name, &v)| (v.global, v.index, conv_cons::var_name(v, convert::GLOBAL_BORDER), name)).collect();
        flags.sort();
        for (_, _, var, name) in flags {
            eprintln!("flag {var} {name}");
        }
    }
    let story = convert::Story { flags: &compiled.flags, voice_classes: &compiled.voice_classes, files: &files };
    let mut ir = Script {
        title: title.to_string(),
        width: game.screen.0 as u32,
        height: game.screen.1 as u32,
        global_border: Some(convert::GLOBAL_BORDER),
        window: compiled.window,
        ops: compiled.ops,
        version: Some(version::line()),
        ..Default::default()
    };
    game.menus(&mut assets, &mut ir, &story);
    // A dry run writes the script but no assets (and no WebP).
    let source = build::webp_tree(output, &PICTURE_DIRS, webp.filter(|_| !dry_run), conv_cons::emit(&ir))?;
    let lost = build::write_game(output, &ir, &source, font.filter(|_| !dry_run), None)?;
    if !dry_run {
        // the voices and the sound effects (sound/se/x.ogg; music is sound/bgm)
        build::reencode_audio(output, &["voice", "sound/se"], audio)?;
    }
    if !dry_run && !loose {
        build::pack(output, &OUTPUT_DIRS)?;
    }
    println!("{file} -> {scene} -> {}", output.join("0.txt").display());
    // A dry run has no assets to look for.
    build::summary(output, &ir, lost, font.is_some(), compiled.notes, Some(!dry_run))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_paths() {
        assert_eq!(safe_relative_path("a.png"), Some(PathBuf::from("a.png")));
        assert_eq!(safe_relative_path("dir\\a.png"), Some(PathBuf::from("dir/a.png")));
        for bad in ["", "../a", "/etc/passwd", "a/../../b", "C:\\x", "\\\\server\\x", "./a"] {
            assert_eq!(safe_relative_path(bad), None, "{bad}");
        }
    }
}
