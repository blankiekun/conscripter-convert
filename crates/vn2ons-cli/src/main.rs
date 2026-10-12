use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use memmap2::Mmap;
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
    /// Low-level tools for Xuse voice archives (*.xsa)
    Xsa {
        #[command(subcommand)]
        command: XsaCommand,
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
