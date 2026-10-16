//! `conscripter-convert typemoon …`: TYPE-MOON's archives (月姫 -A piece
//! of blue glass moon-, Switch): MRG (`.hed`/`.mrg`/`.nam`) and MZP.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Subcommand;
use conv_engines::typemoon::{self, mrg, mzp};

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
    },
    /// Read every archive in a RomFS folder, unpack every entry (and the
    /// entries of MZPs inside), and count what is inside and what fails
    Check { dir: PathBuf },
}

pub fn run(command: TypemoonCommand) -> Result<()> {
    match command {
        TypemoonCommand::List { archive } => each(&archive, |i, name, data, _| {
            println!("{i:>6} {:>10}  {name}", data.len());
            Ok(())
        }),
        TypemoonCommand::Extract { archive, output, prefix } => extract(&archive, &output, &prefix.to_lowercase()),
        TypemoonCommand::Check { dir } => check(&dir),
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

fn extract(archive: &Path, output: &Path, prefix: &str) -> Result<()> {
    let mut count = 0;
    each(archive, |i, name, data, _| {
        if !name.to_lowercase().starts_with(prefix) {
            return Ok(());
        }
        let unpacked = typemoon::unpack(data).with_context(|| format!("entry {i} ({name})"))?;
        let mut path = output.join(safe_relative_path(name).with_context(|| format!("unsafe name {name:?}"))?);
        if matches!(unpacked, std::borrow::Cow::Owned(_)) || path.extension().is_none() {
            path.set_extension(typemoon::extension(&unpacked));
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
    let mut failed = 0;
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
            if kind == "mzp" {
                match mzp::parse(&unpacked) {
                    Ok(inner) => {
                        for (j, d) in inner.iter().enumerate() {
                            match typemoon::unpack(d) {
                                Ok(u) => *kinds.entry(format!("{} in mzp", typemoon::extension(&u))).or_default() += 1,
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
    anyhow::ensure!(failed == 0, "{failed} entries failed");
    Ok(())
}

