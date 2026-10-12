use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use memmap2::Mmap;
use vn2ons_engines::xuse::cd::CdScript;
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
    /// Low-level tools for Xuse voice archives (*.xsa)
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
    /// Print the decoded messages (text) of a script
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
        Command::Xsa { command } => match command {
            XsaCommand::List { archive } => xsa_list(&archive),
            XsaCommand::Extract { archives, output } => xsa_extract(&archives, &output),
        },
    }
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

fn xsa_list(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let xsa = Xsa::parse(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    println!("{:>4}  {:>10}  {:>10}  kind", "slot", "offset", "size");
    for e in &xsa.entries {
        let kind = if xsa.data(e).starts_with(b"OggS") { "ogg" } else { "unknown" };
        println!("{:>4}  {:>10}  {:>10}  {kind}", e.slot, e.data.start, e.data.len());
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
