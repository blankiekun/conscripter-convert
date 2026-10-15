//! `conscripter-convert mages …`: tools for 5pb./MAGES' TaskList engine
//! (俺たちに翼はない, PS Vita).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use conv_engines::mages::build::{open, scripts};
use conv_engines::mages::tasks::{Block, Scene};
use conv_engines::mages;

use crate::{build, map, safe_relative_path};

#[derive(Subcommand)]
pub enum MagesCommand {
    /// List a CPK or AFS2 archive (*.cpk, *.AWB): ID, offset, size, name
    List { archive: PathBuf },
    /// Extract files from a CPK or AFS2 archive
    Extract {
        archive: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Only names starting with this (case-insensitive)
        #[arg(long, default_value = "")]
        prefix: String,
    },
    /// Convert scenes into a CONscripter game directory: 0.txt plus arc.nsa
    /// with the assets they use (sounds converted with ffmpeg)
    Convert {
        /// The decrypted game folder (contains media/)
        game: PathBuf,
        /// Only these scenes (file names such as S01_01A), in this order;
        /// default: all, starting at the game's first scene
        #[arg(long, value_delimiter = ',')]
        scenes: Vec<String>,
        /// Window caption
        #[arg(long, default_value = "俺たちに翼はない")]
        title: String,
        #[command(flatten)]
        build: build::BuildArgs,
    },
    /// Parse every script in <game>/media/SCRIPT.cpk, count the task types
    /// and check that every file ID they use exists
    Check {
        /// The decrypted game folder (contains media/)
        game: PathBuf,
    },
}

pub fn run(command: MagesCommand) -> Result<()> {
    match command {
        MagesCommand::List { archive } => list(&archive),
        MagesCommand::Extract { archive, output, prefix } => extract(&archive, &output, &prefix),
        MagesCommand::Check { game } => check(&game),
        MagesCommand::Convert { game, scenes, title, build } => {
            build::run(build, &title, &mages::assets::OUTPUT_DIRS, |opts| mages::build::build(&game, &scenes, opts))
        }
    }
}

fn list(path: &Path) -> Result<()> {
    let bytes = map(path)?;
    let a = open(path, &bytes)?;
    for e in &a.entries {
        println!("{:>6} {:>10} {:>10}  {}", e.id, e.offset, e.size, e.name);
    }
    println!("{} files", a.entries.len());
    Ok(())
}

fn extract(path: &Path, output: &Path, prefix: &str) -> Result<()> {
    let bytes = map(path)?;
    let a = open(path, &bytes)?;
    let prefix = prefix.to_lowercase();
    let mut n = 0;
    for e in a.entries.iter().filter(|e| e.name.to_lowercase().starts_with(&prefix)) {
        let rel = safe_relative_path(&e.name).with_context(|| format!("refusing unsafe entry name {:?}", e.name))?;
        let dest = output.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, a.read(&bytes, e)?).with_context(|| format!("writing {}", dest.display()))?;
        n += 1;
    }
    println!("extracted {n} files to {}", output.display());
    Ok(())
}

/// Calls `f` on every block under `b`, depth first.
fn walk<'a>(b: &'a Block, f: &mut impl FnMut(&'a Block)) {
    f(b);
    for c in &b.children {
        walk(c, f);
    }
}

fn check(game: &Path) -> Result<()> {
    let scripts = scripts(game)?;
    let ids = |name: &str| -> Result<Vec<u32>> {
        let path = game.join("media").join(name);
        let bytes = map(&path)?;
        Ok(open(&path, &bytes)?.entries.iter().map(|e| e.id).collect())
    };
    let cg = ids("CG.cpk")?;
    let voice = ids("VOICE.AWB")?;
    let se = ids("SE.AWB")?;
    let bgm = ids("BGM.AWB")?;

    let mut failed = 0;
    let mut types: BTreeMap<String, usize> = BTreeMap::new();
    let mut missing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut scenes = 0;
    for (name, src) in &scripts {
        let scene = match Scene::parse(src) {
            Ok(s) => s,
            Err(e) => {
                println!("{name}: {e}");
                failed += 1;
                continue;
            }
        };
        if scene.tasks.is_empty() {
            continue;
        }
        scenes += 1;
        for t in &scene.tasks {
            let ty = t.task_type().unwrap_or("?").to_string();
            *types.entry(ty.clone()).or_default() += 1;
            let mut test = |key: &str, list: &[u32], b: &Block| {
                if let Some(id) = b.int(key)
                    && id >= 0
                    && list.binary_search(&(id as u32)).is_err()
                {
                    missing.entry(format!("{ty} :{key}")).or_default().push(format!("{name}:{} {id}", b.line));
                }
            };
            walk(t, &mut |b| {
                if matches!(ty.as_str(), "Sprite" | "SpriteBG" | "SpriteBS") {
                    test("Texture", &cg, b);
                }
                test("Voice", &voice, b);
                test("SE", &se, b);
                test("BGM", &bgm, b);
            });
        }
    }
    let mut types: Vec<_> = types.into_iter().collect();
    types.sort_by_key(|t| std::cmp::Reverse(t.1));
    println!("{scenes} scenes; task types:");
    for (t, n) in types {
        println!("{n:>7}  {t}");
    }
    for (what, at) in &missing {
        println!("missing file for {what}: {} uses, e.g. {}", at.len(), at[..at.len().min(3)].join(", "));
    }
    if failed > 0 || !missing.is_empty() {
        bail!("{failed} scripts failed to parse, {} kinds of missing reference", missing.len());
    }
    Ok(())
}
