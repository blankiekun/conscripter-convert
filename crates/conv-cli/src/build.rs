//! What every build does once its IR is compiled, whatever the engine:
//! write the game's files, pack the assets, report.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use conv_core::ir::{Op, Script};

use crate::version;

/// Fails if one of the asset folders `dirs` is already in `output`:
/// packing deletes them afterwards, so a build never starts with folders
/// that might hold someone's own files. Loose builds and dry runs don't
/// pack (`packs` false).
pub fn check_output(output: &Path, dirs: &[&str], packs: bool) -> Result<()> {
    fs::create_dir_all(output)?;
    if let Some(d) = dirs.iter().find(|d| packs && output.join(d).exists()) {
        bail!("{} already exists; move it away first (it would be packed and then deleted)", output.join(d).display());
    }
    Ok(())
}

/// Writes the game into `output`: `0.txt` from `source` (the emitted
/// script), `version.txt` (with `package`, the source's own version, when
/// known), `ons_args` and, when given, the font as `default.ttf`. Returns
/// the number of characters Shift-JIS can't show.
pub fn write_game(output: &Path, ir: &Script, source: &str, font: Option<&Path>, package: Option<&str>) -> Result<usize> {
    let (encoded, lost) = conv_cons::encode(source);
    fs::write(output.join("0.txt"), encoded)?;
    if let Some(v) = &ir.version {
        version::write(output, &ir.title, v, package)?;
    }
    fs::write(output.join("ons_args"), conv_cons::ENGINE_ARGS)?;
    if let Some(font) = font {
        fs::copy(font, output.join("default.ttf")).with_context(|| format!("copying {}", font.display()))?;
    }
    Ok(lost)
}

/// Prints the build's report: [`report`], then what the compiler left out
/// or approximated (`notes`, with counts), then [`lint_dir`]'s findings
/// (`Some(files)`; `files`: also look for the files the script names).
/// A build of part of a game skips the lint (`None`): its jumps out of
/// the part have no labels.
pub fn summary(output: &Path, ir: &Script, lost: usize, has_font: bool, notes: BTreeMap<String, usize>, lint: Option<bool>) -> Result<()> {
    report(ir, lost, has_font);
    let mut notes: Vec<_> = notes.into_iter().collect();
    notes.sort_by_key(|n| std::cmp::Reverse(n.1));
    if !notes.is_empty() {
        println!("  not converted or approximated:");
    }
    for (what, n) in notes {
        println!("    {n:>6}  {what}");
    }
    let Some(files) = lint else { return Ok(()) };
    let lint = lint_dir(output, files)?;
    if !lint.is_empty() {
        println!("  lint:");
        print!("{lint}");
    }
    Ok(())
}

/// Packs the files under `dirs` (subfolders included) into .nsa archives,
/// then deletes the folders.
pub fn pack(output: &Path, dirs: &[&str]) -> Result<()> {
    fn walk(dir: &Path, name: &str, files: &mut Vec<conv_cons::nsa::NsaFile>) -> Result<()> {
        let Ok(entries) = fs::read_dir(dir) else { return Ok(()) };
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            let name = format!("{name}\\{}", path.file_name().unwrap().to_string_lossy());
            if path.is_dir() {
                walk(&path, &name, files)?;
            } else {
                files.push(conv_cons::nsa::NsaFile { name, source: path });
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    for d in dirs {
        walk(&output.join(d), d, &mut files)?;
    }
    let archives = conv_cons::nsa::write_archives(output, &files).context("packing .nsa archives")?;
    for d in dirs {
        let _ = fs::remove_dir_all(output.join(d));
    }
    for a in &archives {
        let size = fs::metadata(a)?.len();
        println!("  {}: {:.1} MB", a.display(), size as f64 / 1e6);
    }
    println!("  packed {} files", files.len());
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

/// Lints `dir/0.txt` ([`conv_cons::lint`]). With `files`, also checks that
/// the files it names are in `dir`'s .nsa archives, or loose with an ASCII
/// name (CONscripter doesn't find loose files with Japanese names).
pub fn lint_dir(dir: &Path, files: bool) -> Result<conv_cons::lint::Report> {
    let path = dir.join("0.txt");
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let script = conv_cons::decode(&bytes);
    let mut have = std::collections::HashSet::new();
    if files {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
            if name.starts_with("arc") && name.ends_with(".nsa") {
                for n in conv_cons::nsa::read_names(&path).with_context(|| format!("reading {}", path.display()))? {
                    have.insert(n.to_lowercase());
                }
            }
        }
        fn walk(dir: &Path, name: &str, have: &mut std::collections::HashSet<String>) {
            let Ok(entries) = fs::read_dir(dir) else { return };
            for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
                let file = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                let name = if name.is_empty() { file } else { format!("{name}\\{file}") };
                if path.is_dir() {
                    walk(&path, &name, have);
                } else if name.is_ascii() {
                    have.insert(name);
                }
            }
        }
        walk(dir, "", &mut have);
    }
    Ok(conv_cons::lint::lint(&script, &|n| !files || have.contains(&n.replace('/', "\\").to_lowercase())))
}
