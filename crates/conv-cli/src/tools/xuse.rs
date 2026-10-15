//! Low-level tools for Xuse's formats: XARC archives, compiled scripts
//! (`.cd`), GAF4 image containers, XSA sound archives.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use conv_engines::xuse::cd::CdScript;
use conv_engines::xuse::convert::LabelNames;
use conv_engines::xuse::gaf::Gaf;
use conv_engines::xuse::symbols::{self, SymbolTable};
use conv_engines::xuse::xarc::{CrcTarget, Xarc};
use conv_engines::xuse::xsa::Xsa;

use crate::{map, safe_relative_path};

#[derive(Subcommand)]
pub enum CdCommand {
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
pub enum GafCommand {
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
pub enum XsaCommand {
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
pub enum XarcCommand {
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

pub(crate) fn parse<'a>(path: &Path, bytes: &'a [u8]) -> Result<Xarc<'a>> {
    Xarc::parse(bytes).with_context(|| format!("parsing {}", path.display()))
}

pub(crate) fn xarc_list(path: &Path) -> Result<()> {
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

pub(crate) fn xarc_verify(paths: &[PathBuf]) -> Result<()> {
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

pub(crate) fn xarc_extract(path: &Path, output: &Path) -> Result<()> {
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

pub(crate) fn parse_cd<'a>(path: &Path, bytes: &'a [u8]) -> Result<CdScript<'a>> {
    CdScript::parse(bytes).with_context(|| format!("parsing {}", path.display()))
}

pub(crate) fn cd_info(paths: &[PathBuf]) -> Result<()> {
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

pub(crate) fn cd_dump(path: &Path) -> Result<()> {
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

pub(crate) fn cd_messages(path: &Path) -> Result<()> {
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

pub(crate) fn cd_strings(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let cd = parse_cd(path, &bytes)?;
    for (offset, s) in cd.scan_strings() {
        println!("{offset:>8}  {s}");
    }
    Ok(())
}

pub(crate) fn read_symbols(path: &Path) -> Result<SymbolTable> {
    let mut bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    symbols::decrypt(&mut bytes);
    SymbolTable::parse(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Label names (chapter titles, link scene titles) from `HD/CDFiles/0.cd`.
pub(crate) fn label_names(game: &Path) -> Result<LabelNames> {
    Ok(LabelNames::new(&read_symbols(&game.join("HD").join("CDFiles").join("0.cd"))?))
}

pub(crate) fn cd_symbols(path: &Path, only: Option<usize>) -> Result<()> {
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

pub(crate) fn cd_opstats(paths: &[PathBuf]) -> Result<()> {
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

pub(crate) fn parse_gaf(path: &Path) -> Result<Gaf> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Gaf::parse(&bytes).with_context(|| format!("parsing {}", path.display()))
}

pub(crate) fn gaf_list(paths: &[PathBuf]) -> Result<()> {
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

pub(crate) fn gaf_names(path: &Path) -> Result<()> {
    let gaf = parse_gaf(path)?;
    for (i, img) in gaf.images()?.iter().enumerate() {
        println!("{i:>4}  {}", img.file_name());
    }
    Ok(())
}

pub(crate) fn gaf_extract(paths: &[PathBuf], output: &Path) -> Result<()> {
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

pub(crate) fn xsa_list(path: &Path) -> Result<()> {
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

pub(crate) fn xsa_extract(paths: &[PathBuf], output: &Path) -> Result<()> {
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

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
