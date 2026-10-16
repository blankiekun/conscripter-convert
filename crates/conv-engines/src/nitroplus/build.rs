//! A Nitroplus game's build: its NSS scripts from `nss.npa` compiled (one
//! scene, a run of files or the whole story), the assets they use from
//! the other archives, and the game's own screens.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use conv_core::ir::Script;

use super::assets::{NpaAssets, PICTURE_DIRS};
use super::convert::{self, Game};
use super::npa::{Key, Npa};
use super::{english, npk, nss};
use crate::build::{Built, Opts};

/// The games, by the exe in their folder (which also gives the archives'
/// key): the compiler's additions for each.
pub const GAMES: &[(&str, &Game)] = &[("Muramasa.exe", &convert::MURAMASA), ("Django.exe", &convert::DJANGO)];

/// The voices and sound effects `--audio-codec` changes (sound/se/x.ogg;
/// music is sound/bgm).
const AUDIO_DIRS: &[&str] = &["voice", "sound/se"];

/// What to compile, as `convert-nss` takes it.
#[derive(Default)]
pub struct Run<'a> {
    /// Script file to start at; default: the game's whole story
    /// ([`Game::story`]).
    pub file: Option<&'a str>,
    /// The scene in it; default: the file's name (its content scene).
    pub scene: Option<&'a str>,
    /// Play this many files in a row from `file`.
    pub chain: Option<usize>,
    /// The movie folder; default: the game's own `movie` (続・殺戮のジャンゴ),
    /// else `ムービー` beside the game data (装甲悪鬼村正's disc).
    pub movies: Option<&'a Path>,
    /// The English release's folder (`script.npk`): its text replaces the
    /// Japanese.
    pub english: Option<&'a Path>,
}

/// Compiles `run` from `game` (the folder with the exe and the `*.npa`
/// archives).
pub fn build(game: &Path, run: &Run, opts: &Opts) -> Result<Built> {
    let exe_name = GAMES.iter().map(|(exe, _)| *exe).find(|exe| crate::find(game, exe).exists()).unwrap_or(GAMES[0].0);
    let exe = crate::find(game, exe_name);
    // Read entry by entry, not mapped: the mapped pages of cg.npa and
    // voice.npa (3 GB) would count as the converter's memory.
    let key = npa_key(&exe)?;
    let open = |name: &str| -> Result<Npa> {
        let path = crate::find(game, name);
        let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
        Npa::open(file, key.clone()).with_context(|| format!("parsing {}", path.display()))
    };
    let scripts = open("nss.npa")?;
    let mut files = BTreeMap::new();
    // 続・殺戮のジャンゴ 1.01's nss.npa also holds SourceSafe's vssver2.scc.
    for e in scripts.entries.iter().filter(|e| e.name.to_lowercase().ends_with(".nss")) {
        let parsed = nss::parse(&nss::decode(&scripts.data(e)?)).with_context(|| format!("parsing {}", e.name))?;
        let name = e.name.rsplit('\\').next().unwrap_or(&e.name).to_string();
        files.insert(name, parsed);
    }
    let open_npk = |dir: &Path, name: &str| -> Result<npk::Npk> {
        let path = crate::find(dir, name);
        let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
        npk::Npk::open(file).with_context(|| format!("parsing {}", path.display()))
    };
    if let Some(dir) = run.english {
        let report = english::English::load(&open_npk(dir, "script.npk")?)?.apply(&mut files);
        println!(
            "English: {} stretches of text replaced, {} aligned by position, {} left out by the translation, {} left in Japanese",
            report.replaced,
            report.aligned,
            report.dropped.len(),
            report.missed.len()
        );
        for (file, jp) in report.missed.iter().take(5) {
            println!("  {file}: {}", jp.chars().take(40).collect::<String>());
        }
    }
    let mut assets = NpaAssets::new(open("cg.npa")?, open("sound.npa")?, open("voice.npa")?, opts.output);
    assets.out.dry_run = opts.dry_run;
    // 続・殺戮のジャンゴ 1.01's own pictures.
    if crate::find(game, "cg2.npa").exists() {
        assets.cg2 = Some(open("cg2.npa")?);
    }
    if let Some(dir) = run.english {
        assets.cg_en = Some(open_npk(dir, "cg.npk")?);
    }
    let movies = match run.movies {
        Some(m) => Some(m.to_path_buf()).filter(|m| m.is_dir()),
        None => find_movies(game),
    };
    match movies {
        Some(m) => assets.movie_dir = Some(m),
        None => println!("no movie folder (movie, movies or ムービー, in {} or beside it): the movies are left out", game.display()),
    }
    // The game's own additions to the compiler, by its exe.
    let g = GAMES.iter().find(|(exe, _)| *exe == exe_name).map_or(&Game::PLAIN, |(_, g)| *g);
    let g = if run.english.is_some() && exe_name == "Muramasa.exe" { &convert::MURAMASA_EN } else { g };
    assets.screen = g.screen;
    let (file, scene) = match (run.file, g.story) {
        (Some(file), _) => (file, run.scene.unwrap_or(file)),
        (None, Some((file, story))) => (file, run.scene.unwrap_or(story)),
        (None, None) => anyhow::bail!("this game needs --file"),
    };
    let compiled = match run.chain {
        _ if g.story.is_some_and(|s| s.0 == file) => convert::compile_story(&files, file, scene, g, &mut assets).map_err(anyhow::Error::msg)?,
        Some(n) => {
            let (compiled, played) = convert::compile_run(&files, file, n, g, &mut assets).map_err(anyhow::Error::msg)?;
            println!("{} files: {}", played.len(), played.join(" "));
            compiled
        }
        None => convert::compile_scene(&files, file, scene, g, &mut assets).map_err(anyhow::Error::msg)?,
    };
    for e in &assets.out.errors {
        println!("  asset error: {e}");
    }
    if std::env::var_os("NSS_TRACE").is_some() {
        let mut flags: Vec<_> = compiled.flags.iter().map(|(name, &v)| (v.global, v.index, conv_core::alloc::number(v, convert::GLOBAL_BORDER), name)).collect();
        flags.sort();
        for (_, _, var, name) in flags {
            eprintln!("flag %{var} {name}");
        }
    }
    let story = convert::Story { flags: &compiled.flags, voice_classes: &compiled.voice_classes, files: &files };
    let mut ir = Script {
        title: opts.title.to_string(),
        width: g.screen.0 as u32,
        height: g.screen.1 as u32,
        global_border: Some(convert::GLOBAL_BORDER),
        window: compiled.window,
        ops: compiled.ops,
        single_byte_text: run.english.is_some(),
        ..Default::default()
    };
    g.menus(&mut assets, &mut ir, &story);
    println!("{file} -> {scene} -> {}", opts.output.join("0.txt").display());
    Ok(Built { ir, notes: compiled.notes, picture_dirs: &PICTURE_DIRS, audio_dirs: AUDIO_DIRS, package: None })
}

/// The archives' key, from the game's exe.
pub fn npa_key(exe: &Path) -> Result<Key> {
    let name = exe.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Key::for_exe(&name, &fs::read(exe).with_context(|| format!("reading {}", exe.display()))?)?)
}

/// The movie folder: `movie`, `movies` or `ムービー` (any case) in the
/// game folder or beside it (the disc's `ムービー/` next to `ゲームデータ/`).
fn find_movies(game: &Path) -> Option<PathBuf> {
    [game, &game.join("..")].into_iter().find_map(|dir| {
        fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            matches!(name.as_str(), "movie" | "movies" | "ムービー") && p.is_dir()
        })
    })
}
