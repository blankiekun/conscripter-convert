use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use memmap2::Mmap;
use vn2ons_core::ir::{Op, Script};
use vn2ons_engines::xuse::cd::CdScript;
use vn2ons_engines::xuse::convert::{AssetBank, Flow, Needs, lower_script, window_presets};
use vn2ons_engines::xuse::gaf::Gaf;
use vn2ons_engines::xuse::symbols::{self, SymbolTable};
use vn2ons_engines::xuse::xarc::{CrcTarget, Xarc};
use vn2ons_engines::xuse::xsa::Xsa;

#[derive(Parser)]
#[command(name = "vn2ons", about = "Convert Japanese visual novels to ONScripter")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert the whole game into an ONScripter directory: 0.txt plus
    /// arc.nsa (arc1.nsa, …) with every asset the scripts use
    ConvertGame {
        /// Installed game directory (contains HD/ and CD/)
        game: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Font to copy into the output as default.ttf (ONScripter needs one)
        #[arg(long)]
        font: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "vn2ons")]
        title: String,
        /// Keep assets as loose files instead of packing them into .nsa
        #[arg(long)]
        loose: bool,
        /// Play story scripts straight through in file order: skip the
        /// engine-setup scripts (names starting with '!') and convert no
        /// jumps. Stopgap until conditions are decoded.
        #[arg(long)]
        linear: bool,
    },
    /// Convert one Xuse script (CDFiles/NNNN.cd) into an ONScripter game
    /// directory: 0.txt plus the assets it uses
    ConvertScript {
        /// Installed game directory (contains HD/ and CD/)
        game: PathBuf,
        /// Script number (the NNNN in CDFiles/NNNN.cd)
        #[arg(long)]
        script: u32,
        #[arg(short, long)]
        output: PathBuf,
        /// Font to copy into the output as default.ttf (ONScripter needs one)
        #[arg(long)]
        font: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "vn2ons")]
        title: String,
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
        Command::ConvertGame { game, output, font, title, loose, linear } => {
            convert_game(&game, &output, font.as_deref(), &title, loose, linear)
        }
        Command::ConvertScript { game, script, output, font, title } => {
            convert_script(&game, script, &output, font.as_deref(), &title)
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
    }
}

const ASSET_DIRS: [&str; 6] = ["bg", "sprite", "bgm", "se", "system", "voice"];

fn convert_game(game: &Path, output: &Path, font: Option<&Path>, title: &str, loose: bool, linear: bool) -> Result<()> {
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

    let presets = window_presets(&scripts);
    let mut ops = Vec::new();
    for (file, cd) in &scripts {
        ops.extend(lower_script(cd, file, flow, &mut assets, &presets).with_context(|| format!("converting {file}"))?);
    }
    // The linear build plays the story straight through, without a menu.
    let (title_screen, save_menu, button_bar) = if linear {
        (None, None, None)
    } else {
        (
            Some(assets.title_screen().context("building the title screen")?),
            Some(assets.save_menu().context("building the save menu")?),
            Some(assets.button_bar().context("building the button bar")?),
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
        ops,
    };
    let (encoded, lost) = vn2ons_ons::encode(&vn2ons_ons::emit(&ir));
    fs::write(output.join("0.txt"), encoded)?;
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
                files.push(vn2ons_ons::nsa::NsaFile { name, source: path });
            }
        }
        let archives = vn2ons_ons::nsa::write_archives(output, &files).context("packing .nsa archives")?;
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
        println!("  note: no --font given; ONScripter needs default.ttf in the game directory");
    }
}

fn convert_script(game: &Path, script: u32, output: &Path, font: Option<&Path>, title: &str) -> Result<()> {
    let file = format!("{script:04}.cd");
    let path = game.join("HD").join("CDFiles").join(&file);
    let bytes = map(&path)?;
    let cd = parse_cd(&path, &bytes)?;
    fs::create_dir_all(output)?;

    let mut assets = AssetBank::new(game, output);
    let window = assets.text_window().context("building the text window")?;
    let presets = Default::default(); // the presets live in script 2
    let ops =
        lower_script(&cd, &file, Flow::Jumps, &mut assets, &presets).with_context(|| format!("converting {file}"))?;
    let ir = Script { title: title.to_string(), width: 800, height: 600, window: Some(window), title_screen: None, save_menu: None, button_bar: None, ops };

    let source = vn2ons_ons::emit(&ir);
    let (encoded, lost) = vn2ons_ons::encode(&source);
    fs::write(output.join("0.txt"), encoded)?;
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

fn cd_symbols(path: &Path, only: Option<usize>) -> Result<()> {
    let mut bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    symbols::decrypt(&mut bytes);
    let table = SymbolTable::parse(&bytes).with_context(|| format!("parsing {}", path.display()))?;
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
