use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use memmap2::Mmap;
use conv_core::ir::{Op, Script};
use conv_engines::xuse::cd::CdScript;
use conv_engines::xuse::convert::{AssetBank, Flow, LabelNames, Needs, lower_script, presets};
use conv_engines::xuse::gaf::Gaf;
use conv_engines::xuse::symbols::{self, SymbolTable};
use conv_engines::xuse::xarc::{CrcTarget, Xarc};
use conv_engines::xuse::xsa::Xsa;

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
        #[arg(short, long)]
        output: PathBuf,
        /// Font to copy into the output as default.ttf (CONscripter needs one)
        #[arg(long)]
        font: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        /// Keep assets as loose files instead of packing them into .nsa
        #[arg(long)]
        loose: bool,
        /// Play story scripts straight through in file order: skip the
        /// engine-setup scripts (names starting with '!') and convert no
        /// jumps. Stopgap until conditions are decoded.
        #[arg(long)]
        linear: bool,
        /// Store images as near-lossless WebP (about half the size of PNG).
        /// Needs a CONscripter build that reads WebP: the web build, or one
        /// whose SDL_image has libwebp
        #[arg(long)]
        webp: bool,
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
        /// Store images as near-lossless WebP (about half the size of PNG).
        /// Needs a CONscripter build that reads WebP: the web build, or one
        /// whose SDL_image has libwebp
        #[arg(long)]
        webp: bool,
    },
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
    /// Low-level tools for Nitroplus NSS scripts (*.nss, from nss.npa)
    Nss {
        #[command(subcommand)]
        command: NssCommand,
    },
}

#[derive(Subcommand)]
enum NssCommand {
    /// Parse scripts (files, or directories of *.nss) and report every
    /// file that fails
    Check {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Check that every scene, chapter, script file and function the
    /// scripts name exists, and list the engine built-ins they call
    Refs {
        /// Directory with all the game's *.nss files (as extracted from nss.npa)
        dir: PathBuf,
    },
    /// Show how the scripts call each engine built-in: argument counts and
    /// the most common values in each position
    Builtins {
        /// Directory with all the game's *.nss files (as extracted from nss.npa)
        dir: PathBuf,
        /// Only these built-ins (default: all, most used first)
        names: Vec<String>,
        /// Values listed per argument position
        #[arg(long, default_value_t = 8)]
        top: usize,
    },
    /// Print the route map: each story file's next files and the
    /// conditions or choices that lead there
    Route {
        /// Directory with all the game's *.nss files (as extracted from nss.npa)
        dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum CdCommand {
    /// Show header fields and check the MD5 footer
    Info {
        #[arg(required = true)]
        scripts: Vec<PathBuf>,
    },
    /// Print the label table and instructions
    Dump { script: PathBuf },
    /// Print the decoded messages (text) and hyperlinks of a script
    Messages { script: PathBuf },
    /// Print the strings found in the operand pool (heuristic scan)
    Strings { script: PathBuf },
    /// Print the label symbol table (CDFiles/0.cd)
    Symbols {
        file: PathBuf,
        /// Only show this script id
        #[arg(long)]
        script: Option<usize>,
    },
    /// Count opcodes over many scripts
    Opstats {
        #[arg(required = true)]
        scripts: Vec<PathBuf>,
    },
}

#[derive(Subcommand)]
enum GafCommand {
    /// Show header fields and image count, checking that all images are found
    List {
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Print each image's index and original file name
    Names { file: PathBuf },
    /// Extract the embedded PNGs as <file>_<index>.png into a directory
    Extract {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum XsaCommand {
    /// List clips
    List { archive: PathBuf },
    /// Extract clips as <archive>_<slot>.ogg into a directory
    Extract {
        #[arg(required = true)]
        archives: Vec<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum XarcCommand {
    /// List entries
    List { archive: PathBuf },
    /// Check all stored CRCs
    Verify {
        #[arg(required = true)]
        archives: Vec<PathBuf>,
    },
    /// Extract all entries into a directory
    Extract {
        archive: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::ConvertGame { game, output, font, title, loose, linear, webp } => {
            convert_game(&game, &output, font.as_deref(), &title, loose, linear, webp)
        }
        Command::ConvertScript { game, script, output, font, title, webp } => {
            convert_script(&game, script, &output, font.as_deref(), &title, webp)
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
        Command::Nss { command } => match command {
            NssCommand::Check { paths } => nss_check(&paths),
            NssCommand::Refs { dir } => nss_refs(&dir),
            NssCommand::Route { dir } => nss_route(&dir),
            NssCommand::Builtins { dir, names, top } => nss_builtins(&dir, &names, top),
        },
    }
}

const ASSET_DIRS: [&str; 7] = ["bg", "sprite", "bgm", "se", "system", "voice", "fx"];
/// Not `fx`: its animations are strips up to 47 screens wide, and WebP
/// pictures can't be wider than 16,383 pixels.
const IMAGE_DIRS: [&str; 3] = ["bg", "sprite", "system"];

/// With `webp`, re-encodes the images in `output` as WebP and points the
/// script at them.
fn image_format(output: &Path, source: String, webp: bool) -> Result<String> {
    if !webp {
        return Ok(source);
    }
    let dirs: Vec<PathBuf> = IMAGE_DIRS.iter().map(|d| output.join(d)).collect();
    let c = conv_cons::webp::convert_dirs(&dirs).context("converting images to WebP")?;
    println!(
        "  {} images as WebP: {:.1} MB (PNG {:.1} MB)",
        c.files,
        c.webp_bytes as f64 / 1e6,
        c.png_bytes as f64 / 1e6
    );
    Ok(conv_cons::webp::retarget(&source))
}

fn convert_game(game: &Path, output: &Path, font: Option<&Path>, title: &str, loose: bool, linear: bool, webp: bool) -> Result<()> {
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
    fs::create_dir_all(output)?;
    // Packing deletes these folders afterwards, so never start with ones
    // that might hold someone's own files.
    if let Some(d) = ASSET_DIRS.iter().find(|d| !loose && output.join(d).exists()) {
        bail!("{} already exists; move it away first (it would be packed and then deleted)", output.join(d).display());
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
    let mut assets = AssetBank::new(game, output);
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
    };
    let source = image_format(output, conv_cons::emit(&ir), webp)?;
    let (encoded, lost) = conv_cons::encode(&source);
    fs::write(output.join("0.txt"), encoded)?;
    fs::write(output.join("ons_args"), conv_cons::ENGINE_ARGS)?;
    if let Some(font) = font {
        fs::copy(font, output.join("default.ttf")).with_context(|| format!("copying {}", font.display()))?;
    }

    if !loose {
        let mut files = Vec::new();
        for d in ASSET_DIRS {
            let Ok(entries) = fs::read_dir(output.join(d)) else { continue };
            let mut names: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
            names.sort();
            for path in names {
                let name = format!("{d}\\{}", path.file_name().unwrap().to_string_lossy());
                files.push(conv_cons::nsa::NsaFile { name, source: path });
            }
        }
        let archives = conv_cons::nsa::write_archives(output, &files).context("packing .nsa archives")?;
        for d in ASSET_DIRS {
            let _ = fs::remove_dir_all(output.join(d));
        }
        for a in &archives {
            let size = fs::metadata(a)?.len();
            println!("  {}: {:.1} MB", a.display(), size as f64 / 1e6);
        }
        println!("  packed {} files", files.len());
    }

    report(&ir, lost, font.is_some());
    Ok(())
}

fn report(ir: &Script, lost: usize, has_font: bool) {
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
        println!("  characters not representable in Shift-JIS: {lost}");
    }
    if !has_font {
        println!("  note: no --font given; CONscripter needs default.ttf in the game directory");
    }
}

fn convert_script(game: &Path, script: u32, output: &Path, font: Option<&Path>, title: &str, webp: bool) -> Result<()> {
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
    let ir = Script { title: title.to_string(), width: 800, height: 600, window: Some(window), title_screen: None, save_menu: None, button_bar: None, settings: None, backlog: None, confirm: None, extras: None, ops };

    let source = image_format(output, conv_cons::emit(&ir), webp)?;
    let (encoded, lost) = conv_cons::encode(&source);
    fs::write(output.join("0.txt"), encoded)?;
    fs::write(output.join("ons_args"), conv_cons::ENGINE_ARGS)?;
    if let Some(font) = font {
        fs::copy(font, output.join("default.ttf")).with_context(|| format!("copying {}", font.display()))?;
    }

    println!("{file} ({}) -> {}", cd.name, output.join("0.txt").display());
    report(&ir, lost, font.is_some());
    Ok(())
}

fn map(path: &Path) -> Result<Mmap> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    // SAFETY: the archive is opened read-only and not modified while mapped.
    unsafe { Mmap::map(&file) }.with_context(|| format!("mapping {}", path.display()))
}

fn parse<'a>(path: &Path, bytes: &'a [u8]) -> Result<Xarc<'a>> {
    Xarc::parse(bytes).with_context(|| format!("parsing {}", path.display()))
}

fn xarc_list(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let arc = parse(path, &bytes)?;
    println!("{:>10}  {:<19}  {:<8}  name", "size", "timestamp", "unknown");
    for e in &arc.entries {
        let t = e.timestamp;
        println!(
            "{:>10}  {:04}-{:02}-{:02} {:02}:{:02}:{:02}  {}  {}",
            e.data.len(),
            t.year,
            t.month,
            t.day,
            t.hour,
            t.minute,
            t.second,
            hex(&e.unknown),
            e.name
        );
    }
    println!("{} entries", arc.entries.len());
    Ok(())
}

fn xarc_verify(paths: &[PathBuf]) -> Result<()> {
    let mut failed = false;
    for path in paths {
        let bytes = map(path)?;
        let arc = parse(path, &bytes)?;
        let mismatches = arc.verify();
        if mismatches.is_empty() {
            println!("{}: OK ({} entries)", path.display(), arc.entries.len());
            continue;
        }
        failed = true;
        for m in mismatches {
            let what = match m.target {
                CrcTarget::ArchiveHeader => "archive header".to_string(),
                CrcTarget::EntryHeader(i) => format!("header of {}", arc.entries[i].name),
                CrcTarget::EntryData(i) => format!("data of {}", arc.entries[i].name),
            };
            println!(
                "{}: CRC mismatch in {what}: stored {:04x}, computed {:04x}",
                path.display(),
                m.stored,
                m.computed
            );
        }
    }
    if failed {
        bail!("CRC verification failed");
    }
    Ok(())
}

fn xarc_extract(path: &Path, output: &Path) -> Result<()> {
    let bytes = map(path)?;
    let arc = parse(path, &bytes)?;
    for e in &arc.entries {
        let rel = safe_relative_path(&e.name)
            .with_context(|| format!("refusing unsafe entry name {:?}", e.name))?;
        let dest = output.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, arc.data(e)).with_context(|| format!("writing {}", dest.display()))?;
    }
    println!("extracted {} entries to {}", arc.entries.len(), output.display());
    Ok(())
}

fn parse_cd<'a>(path: &Path, bytes: &'a [u8]) -> Result<CdScript<'a>> {
    CdScript::parse(bytes).with_context(|| format!("parsing {}", path.display()))
}

fn cd_info(paths: &[PathBuf]) -> Result<()> {
    let mut failed = false;
    for path in paths {
        let bytes = map(path)?;
        let cd = match CdScript::parse(&bytes) {
            Ok(cd) => cd,
            Err(e) => {
                failed = true;
                println!("{}: {e}", path.display());
                continue;
            }
        };
        let md5 = if cd.md5_ok() { "md5 ok" } else { "MD5 MISMATCH" };
        failed |= !cd.md5_ok();
        println!(
            "{}: id {} {:?}, {} labels, {} instructions, pool {} bytes, {md5}",
            path.display(),
            cd.script_id,
            cd.name,
            cd.labels.len(),
            cd.code.len(),
            cd.pool.len()
        );
    }
    if failed {
        bail!("some scripts failed to parse or verify");
    }
    Ok(())
}

fn cd_dump(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let cd = parse_cd(path, &bytes)?;
    println!("; {} (id {})", cd.name, cd.script_id);
    println!("; labels (script, label, instruction index)");
    for l in &cd.labels {
        println!("label {:>4} {:>5} {:>7}", l.script, l.label, l.code_offset / 12);
    }
    println!("; instructions (index, op, sub, arg, source pos)");
    for (i, ins) in cd.code.iter().enumerate() {
        println!("{i:>7}  op {:#06x}  sub {:#06x}  arg {:>11}  @{}", ins.op, ins.sub, ins.arg, ins.source_pos);
    }
    Ok(())
}

fn cd_messages(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let cd = parse_cd(path, &bytes)?;
    for (i, ins) in cd.code.iter().enumerate() {
        if let Some(link) = cd.link(ins) {
            let l = link.with_context(|| format!("instruction {i}"))?;
            // Link targets are label-table indices; index n is stored entry n - 1.
            let dest = (l.target as usize)
                .checked_sub(1)
                .and_then(|i| cd.labels.get(i))
                .map(|d| format!("script {} label {}", d.script, d.label))
                .unwrap_or_else(|| "unresolved".into());
            println!(
                "#{i} link row {} col {} len {} style {:#x} target {} ({dest})",
                l.row, l.column, l.length, l.style, l.target
            );
            continue;
        }
        let Some(msg) = cd.message(ins) else { continue };
        let msg = msg.with_context(|| format!("instruction {i}"))?;
        let voice = msg.voice_id.map(|v| format!(" voice {v}")).unwrap_or_default();
        println!("#{i} line {}{voice}", msg.line);
        for seg in &msg.segments {
            println!("    {seg}");
        }
    }
    Ok(())
}

fn cd_strings(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let cd = parse_cd(path, &bytes)?;
    for (offset, s) in cd.scan_strings() {
        println!("{offset:>8}  {s}");
    }
    Ok(())
}

fn read_symbols(path: &Path) -> Result<SymbolTable> {
    let mut bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    symbols::decrypt(&mut bytes);
    SymbolTable::parse(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Label names (chapter titles, link scene titles) from `HD/CDFiles/0.cd`.
fn label_names(game: &Path) -> Result<LabelNames> {
    Ok(LabelNames::new(&read_symbols(&game.join("HD").join("CDFiles").join("0.cd"))?))
}

fn cd_symbols(path: &Path, only: Option<usize>) -> Result<()> {
    let table = read_symbols(path)?;
    for (id, syms) in table.scripts.iter().enumerate() {
        if only.is_some_and(|o| o != id) {
            continue;
        }
        println!("script {id:04}: {} labels", syms.len());
        for s in syms {
            println!("  {:>7}  {:?}  {}", s.code_offset / 12, s.unknown, s.name);
        }
    }
    Ok(())
}

fn cd_opstats(paths: &[PathBuf]) -> Result<()> {
    let mut counts = std::collections::BTreeMap::<u16, (usize, std::collections::BTreeSet<u16>)>::new();
    let mut total = 0;
    for path in paths {
        let bytes = map(path)?;
        let Ok(cd) = CdScript::parse(&bytes) else {
            eprintln!("skipping {} (not a numbered script)", path.display());
            continue;
        };
        for ins in &cd.code {
            let e = counts.entry(ins.op).or_default();
            e.0 += 1;
            e.1.insert(ins.sub);
        }
        total += cd.code.len();
    }
    println!("{:>8}  {:>8}  subs", "op", "count");
    for (op, (n, subs)) in &counts {
        let shown: Vec<String> = subs.iter().take(8).map(|s| format!("{s:#x}")).collect();
        let more = if subs.len() > 8 { format!(" …(+{})", subs.len() - 8) } else { String::new() };
        println!("{op:#8x}  {n:>8}  {}{more}", shown.join(","));
    }
    println!("{total} instructions, {} distinct opcodes", counts.len());
    Ok(())
}

fn parse_gaf(path: &Path) -> Result<Gaf> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Gaf::parse(&bytes).with_context(|| format!("parsing {}", path.display()))
}

fn gaf_list(paths: &[PathBuf]) -> Result<()> {
    let mut failed = false;
    for path in paths {
        let gaf = parse_gaf(path)?;
        match gaf.images() {
            Ok(images) => {
                let bytes: usize = images.iter().map(|i| i.png.len()).sum();
                println!("{}: {} images, {bytes} bytes of PNG, key {:?}", path.display(), images.len(), gaf.name);
            }
            Err(e) => {
                failed = true;
                println!("{}: {e}", path.display());
            }
        }
    }
    if failed {
        bail!("some files could not be fully read");
    }
    Ok(())
}

fn gaf_names(path: &Path) -> Result<()> {
    let gaf = parse_gaf(path)?;
    for (i, img) in gaf.images()?.iter().enumerate() {
        println!("{i:>4}  {}", img.file_name());
    }
    Ok(())
}

fn gaf_extract(paths: &[PathBuf], output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    let mut total = 0;
    for path in paths {
        let gaf = parse_gaf(path)?;
        let images = gaf.images().with_context(|| format!("reading images of {}", path.display()))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .with_context(|| format!("no usable file name in {}", path.display()))?;
        for (i, img) in images.iter().enumerate() {
            let dest = output.join(format!("{stem}_{i:03}.png"));
            fs::write(&dest, img.png).with_context(|| format!("writing {}", dest.display()))?;
        }
        total += images.len();
    }
    println!("extracted {total} images from {} files to {}", paths.len(), output.display());
    Ok(())
}

fn xsa_list(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let xsa = Xsa::parse(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    println!("{:>4}  {:>10}  {:>10}  {:>10}  {:>10}  kind", "slot", "offset", "size", "param0", "param1");
    for e in &xsa.entries {
        let kind = if xsa.data(e).starts_with(b"OggS") { "ogg" } else { "unknown" };
        let [p0, p1] = e.params;
        println!("{:>4}  {:>10}  {:>10}  {p0:>10}  {p1:>10}  {kind}", e.slot, e.data.start, e.data.len());
    }
    println!("{} clips", xsa.entries.len());
    Ok(())
}

fn xsa_extract(paths: &[PathBuf], output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    let mut total = 0;
    for path in paths {
        let bytes = map(path)?;
        let xsa = Xsa::parse(&bytes).with_context(|| format!("parsing {}", path.display()))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .with_context(|| format!("no usable file name in {}", path.display()))?
            .to_lowercase();
        for e in &xsa.entries {
            let data = xsa.data(e);
            let ext = if data.starts_with(b"OggS") { "ogg" } else { "bin" };
            let dest = output.join(format!("{stem}_{:03}.{ext}", e.slot));
            fs::write(&dest, data).with_context(|| format!("writing {}", dest.display()))?;
        }
        total += xsa.entries.len();
    }
    println!("extracted {total} clips from {} archives to {}", paths.len(), output.display());
    Ok(())
}

/// Turns an archive entry name into a relative path that stays inside the
/// output directory. Backslashes are treated as directory separators.
fn safe_relative_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name.replace('\\', "/"));
    // ':' is rejected so Windows drive prefixes ("C:") can't slip through.
    let ok = !name.is_empty()
        && !name.contains(':')
        && path.components().all(|c| matches!(c, Component::Normal(_)));
    ok.then_some(path)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Every `*.nss` named, or found directly inside a named directory, sorted.
fn nss_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            for entry in fs::read_dir(path).with_context(|| format!("reading {}", path.display()))? {
                let p = entry?.path();
                if p.extension().is_some_and(|e| e == "nss") {
                    files.push(p);
                }
            }
        } else {
            files.push(path.clone());
        }
    }
    files.sort();
    Ok(files)
}

fn nss_check(paths: &[PathBuf]) -> Result<()> {
    use conv_engines::nitroplus::nss::{self, Item};
    let files = nss_files(paths)?;
    let (mut failed, mut functions, mut blocks, mut skipped) = (0, 0, 0, 0);
    for path in &files {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        match nss::parse(&nss::decode(&bytes)) {
            Ok(file) => {
                for e in &file.errors {
                    println!("{}: skipped: {e}", path.display());
                }
                skipped += file.errors.len();
                for item in &file.items {
                    match item {
                        Item::Function(_) => functions += 1,
                        Item::Chapter(_) | Item::Scene(_) => blocks += 1,
                        Item::Include(_) => {}
                    }
                }
            }
            Err(e) => {
                failed += 1;
                println!("{}: {e}", path.display());
            }
        }
    }
    println!(
        "{} files, {} parsed, {failed} failed; {blocks} chapters/scenes, {functions} functions; {skipped} source errors skipped",
        files.len(),
        files.len() - failed
    );
    if failed > 0 {
        bail!("{failed} files failed to parse");
    }
    Ok(())
}

/// All `*.nss` in `dir`, parsed, by file name.
fn nss_corpus(dir: &Path) -> Result<std::collections::BTreeMap<String, conv_engines::nitroplus::nss::NssFile>> {
    use conv_engines::nitroplus::nss;
    let mut files = std::collections::BTreeMap::new();
    for path in nss_files(&[dir.to_path_buf()])? {
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let file = nss::parse(&nss::decode(&bytes)).with_context(|| format!("parsing {}", path.display()))?;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        files.insert(name, file);
    }
    Ok(files)
}

fn nss_route(dir: &Path) -> Result<()> {
    use conv_engines::nitroplus::route::{self, Next};
    let map = route::build(&nss_corpus(dir)?);
    println!("Starts: {}", map.starts.join(", "));
    let mut from = "";
    let (mut ends, mut unknown) = (0, 0);
    for e in &map.edges {
        if e.from != from {
            from = &e.from;
            println!("{from}");
        }
        let to = match &e.to {
            Next::File(f) => format!("-> {f}"),
            Next::End(why) => {
                ends += 1;
                format!("END ({why})")
            }
            Next::Unknown(why) => {
                unknown += 1;
                format!("?? ({why})")
            }
        };
        if e.path.is_empty() {
            println!("  {to}");
        } else {
            println!("  {to}   [{}]", e.path.join("; "));
        }
    }
    println!("\nUnreachable story files ({}):", map.unreachable.len());
    for f in &map.unreachable {
        println!("  {f}");
    }
    println!(
        "\n{} reachable story files, {} edges, {ends} ends, {unknown} unresolved",
        map.reachable.len(),
        map.edges.len()
    );
    Ok(())
}

fn nss_refs(dir: &Path) -> Result<()> {
    use conv_engines::nitroplus::refs;
    let files = nss_corpus(dir)?;
    let report = refs::check(&files);
    for p in &report.problems {
        println!("{p}");
    }
    println!("\nDefined in more than one file ({}):", report.duplicates.len());
    for (name, in_files) in &report.duplicates {
        println!("  {name}: {}", in_files.join(", "));
    }
    println!("\nCalled but not defined, i.e. engine built-ins ({}):", report.builtins.len());
    let mut builtins: Vec<_> = report.builtins.iter().collect();
    builtins.sort_by(|a, b| b.1.calls.cmp(&a.1.calls).then(a.0.cmp(b.0)));
    for (name, b) in builtins {
        println!("  {:6} {name}", b.calls);
    }
    println!(
        "\n{} files; {} call targets, {} script names, {} calls checked; {} problems",
        files.len(),
        report.checked_targets,
        report.checked_names,
        report.checked_calls,
        report.problems.len()
    );
    Ok(())
}

fn nss_builtins(dir: &Path, names: &[String], top: usize) -> Result<()> {
    use conv_engines::nitroplus::refs;
    let report = refs::check(&nss_corpus(dir)?);
    let mut builtins: Vec<_> = report
        .builtins
        .iter()
        .filter(|(name, _)| names.is_empty() || names.iter().any(|n| n.eq_ignore_ascii_case(name)))
        .collect();
    if builtins.is_empty() {
        bail!("no built-in by those names");
    }
    builtins.sort_by(|a, b| b.1.calls.cmp(&a.1.calls).then(a.0.cmp(b.0)));
    for (name, b) in builtins {
        let arities: Vec<String> = b.arities.iter().map(|(n, c)| format!("{n} args ×{c}")).collect();
        println!("{name}: {} calls ({})", b.calls, arities.join(", "));
        // Array() and the like take up to 351 arguments
        const MAX_POSITIONS: usize = 12;
        for (pos, values) in b.args.iter().enumerate().take(MAX_POSITIONS) {
            let mut values: Vec<_> = values.iter().collect();
            values.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let shown: Vec<String> = values.iter().take(top).map(|(v, c)| format!("{v} ×{c}")).collect();
            let more = values.len().saturating_sub(top);
            let more = if more > 0 { format!(", … {more} more") } else { String::new() };
            println!("  {}: {}{more}", pos + 1, shown.join(", "));
        }
        if b.args.len() > MAX_POSITIONS {
            println!("  … {} more positions", b.args.len() - MAX_POSITIONS);
        }
    }
    Ok(())
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
