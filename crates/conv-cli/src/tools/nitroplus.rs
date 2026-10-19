//! Low-level tools for Nitroplus' formats: NPA archives and NSS scripts.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Subcommand;

use conv_engines::nitroplus::build::npa_key;

use crate::{map, safe_relative_path};

#[derive(Subcommand)]
pub enum NpaCommand {
    /// List the files: size and name
    List {
        /// The game's exe (Muramasa.exe, Django.exe), which picks the decryption table
        exe: PathBuf,
        archive: PathBuf,
    },
    /// Extract files, decrypted and unpacked
    Extract {
        /// The game's exe (Muramasa.exe, Django.exe), which picks the decryption table
        exe: PathBuf,
        archive: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Only names starting with this (case-insensitive, `/` separators)
        #[arg(long, default_value = "")]
        prefix: String,
    },
}

#[derive(Subcommand)]
pub enum NssCommand {
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
    /// List the game's functions that chain two or more timed built-ins
    /// (a helper census), most called first; with --probe, compile each
    /// one that takes an object after the scheduler's fixed contexts with
    /// the game's own library, and show the time ledger's notes and whether
    /// the context's 16 s move is still drawn. Holds the game's own names:
    /// keep the output local (`out/`)
    Helpers {
        /// Directory with all the game's *.nss files (as extracted from nss.npa)
        dir: PathBuf,
        /// Probe with this game's compiler rules (Muramasa.exe, Django.exe)
        #[arg(long)]
        probe: Option<String>,
    },
    /// Print the route map: each story file's next files and the
    /// conditions or choices that lead there
    Route {
        /// Directory with all the game's *.nss files (as extracted from nss.npa)
        dir: PathBuf,
    },
}


pub(crate) fn open_npa<'a>(exe: &Path, path: &Path, bytes: &'a [u8]) -> Result<conv_engines::nitroplus::npa::Npa<'a>> {
    conv_engines::nitroplus::npa::Npa::parse(bytes, npa_key(exe)?).with_context(|| format!("parsing {}", path.display()))
}

pub(crate) fn npa_list(exe: &Path, path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let npa = open_npa(exe, path, &bytes)?;
    for e in &npa.entries {
        println!("{:>10}  {}", e.size, e.name);
    }
    println!("{} files", npa.entries.len());
    Ok(())
}

pub(crate) fn npa_extract(exe: &Path, path: &Path, output: &Path, prefix: &str) -> Result<()> {
    use conv_engines::nitroplus::npa::normalize;
    let bytes = map(path)?;
    let npa = open_npa(exe, path, &bytes)?;
    let prefix = normalize(prefix);
    let mut n = 0;
    for e in npa.entries.iter().filter(|e| normalize(&e.name).starts_with(&prefix)) {
        let rel = safe_relative_path(&e.name).with_context(|| format!("refusing unsafe entry name {:?}", e.name))?;
        let dest = output.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, npa.data(e)?).with_context(|| format!("writing {}", dest.display()))?;
        n += 1;
    }
    println!("extracted {n} files to {}", output.display());
    Ok(())
}

/// Every `*.nss` named, or found directly inside a named directory, sorted.
pub(crate) fn nss_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
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

pub(crate) fn nss_check(paths: &[PathBuf]) -> Result<()> {
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
pub(crate) fn nss_corpus(dir: &Path) -> Result<std::collections::BTreeMap<String, conv_engines::nitroplus::nss::NssFile>> {
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

pub(crate) fn nss_route(dir: &Path) -> Result<()> {
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

pub(crate) fn nss_refs(dir: &Path) -> Result<()> {
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

pub(crate) fn nss_builtins(dir: &Path, names: &[String], top: usize) -> Result<()> {
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

pub(crate) fn nss_helpers(dir: &Path, probe: Option<&str>) -> Result<()> {
    use conv_engines::nitroplus::{build::GAMES, convert::probe};
    let mut files = nss_corpus(dir)?;
    let game = match probe {
        Some(exe) => Some(GAMES.iter().find(|(e, _)| e.eq_ignore_ascii_case(exe)).map(|(_, g)| *g).with_context(|| format!("no NSS game {exe}"))?),
        None => None,
    };
    let helpers = probe::census(&files);
    for h in helpers {
        println!("{:6}  {} ({}): {}", h.calls, h.name, h.file, h.timed.join(", "));
        let Some(game) = game.filter(|_| h.takes_name && h.calls > 0) else { continue };
        // shortcut: every argument after the object is 1000 (a time, a value or a wait in ms); helpers whose other parameters are names or flags probe wrong
        let args: Vec<String> = std::iter::once("\"p\"".to_string()).chain((1..h.params).map(|_| "1000".to_string())).collect();
        let call = format!("{}({});", h.name, args.join(", "));
        for (what, context) in probe::CONTEXTS {
            match probe::probe(&mut files, game, context, &call) {
                Ok((notes, kept)) => {
                    let notes: Vec<String> = notes.iter().map(|(n, c)| format!("{n} ×{c}")).collect();
                    println!("          {what}: move {}{}", if kept { "kept" } else { "LOST" }, if notes.is_empty() { String::new() } else { format!("; {}", notes.join("; ")) });
                }
                Err(e) => println!("          {what}: {e}"),
            }
        }
    }
    Ok(())
}
