use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use conv_core::ir::Script;
pub(crate) use conv_engines::build::map;
use conv_engines::media::AudioFormat;
use conv_engines::xuse::build::{AUDIO_DIRS, PICTURE_DIRS, SCREEN, label_names, parse_cd};
use conv_engines::xuse::convert::{AssetBank, Flow, lower_script};
use conv_engines::{nitroplus, xuse};

mod build;
mod mages;
mod tools;
mod version;

use tools::nitroplus::*;
use tools::xuse::*;

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
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        /// Play story scripts straight through in file order: skip the
        /// engine-setup scripts (names starting with '!') and convert no
        /// jumps. Stopgap until conditions are decoded.
        #[arg(long)]
        linear: bool,
        #[command(flatten)]
        build: build::BuildArgs,
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
        /// Font to copy into the output as default.ttf (CONscripter needs one;
        /// default: the bundled IPA Gothic)
        #[arg(long)]
        font: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        #[command(flatten)]
        webp: build::WebpArgs,
        #[command(flatten)]
        audio: build::AudioArgs,
    },
    /// Convert one Nitroplus story scene (装甲悪鬼村正) into a CONscripter
    /// game directory: 0.txt plus the assets it uses
    ConvertNss {
        /// Game data directory (Muramasa.exe or Django.exe and the *.npa archives)
        game: PathBuf,
        /// Script file and scene; the scene defaults to the file's name
        /// (its content scene). Without it, the game's whole story
        /// (続・殺戮のジャンゴ; 装甲悪鬼村正 needs --file and --chain)
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        scene: Option<String>,
        /// Play this many story files in a row from --file, following
        /// $GameName (from each file's _MAIN scene, as the game does).
        /// A new game starts at ma00_000.nss (the boot script's start; it
        /// also unlocks 魔王編 once both other routes are cleared)
        #[arg(long)]
        chain: Option<usize>,
        /// The movie folder (default: the game's own `movie`, as
        /// 続・殺戮のジャンゴ has, else `ムービー` beside the game data, as on
        /// 装甲悪鬼村正's disc)
        #[arg(long)]
        movies: Option<PathBuf>,
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        /// The English release's folder (Full Metal Daemon Muramasa, its
        /// *.npk): its text replaces the Japanese
        #[arg(long)]
        english: Option<PathBuf>,
        #[command(flatten)]
        build: build::BuildArgs,
    },
    /// Check a converted game directory's 0.txt: missing labels, effect
    /// numbers past 255, half-width characters in vertical text, files
    /// that aren't in the archives
    Lint { dir: PathBuf },
    /// List the games the converter builds (as JSON with --json), or with a
    /// folder, the ones found in it
    Games {
        dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
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
    /// Low-level tools for Nitroplus NPA archives (*.npa)
    Npa {
        #[command(subcommand)]
        command: NpaCommand,
    },
    /// Low-level tools for Nitroplus NSS scripts (*.nss, from nss.npa)
    Nss {
        #[command(subcommand)]
        command: NssCommand,
    },
    /// Tools for 5pb./MAGES' TaskList engine (俺たちに翼はない, PS Vita):
    /// CRI archives and scripts
    Mages {
        #[command(subcommand)]
        command: mages::MagesCommand,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::ConvertGame { game, title, linear, build } => {
            build::run(build, &title, xuse::build::OUTPUT_DIRS, |opts| xuse::build::build(&game, linear, opts))
        }
        Command::ConvertScript { game, script, output, font, title, webp, audio } => {
            convert_script(&game, script, &output, font.as_deref(), &title, webp.get(), audio.format()?)
        }
        Command::ConvertNss { game, file, scene, chain, movies, title, english, build } => {
            let run = nitroplus::build::Run { file: file.as_deref(), scene: scene.as_deref(), chain, movies: movies.as_deref(), english: english.as_deref() };
            build::run(build, &title, &nitroplus::assets::OUTPUT_DIRS, |opts| nitroplus::build::build(&game, &run, opts))
        }
        Command::Games { dir, json } => {
            use conv_engines::registry;
            if json {
                print!("{}", registry::json());
                return Ok(());
            }
            let games = match &dir {
                Some(d) => registry::detect(d),
                None => registry::GAMES.iter().collect(),
            };
            for g in &games {
                let options = if g.options.is_empty() { String::new() } else { format!(" {}", g.options.join(" ")) };
                println!("{} ({}, {}): conscripter-convert {} <game> -o <output> [--font <ttf>]{options}", g.name, g.id, g.engine, g.command.join(" "));
            }
            if games.is_empty() {
                bail!("no game found");
            }
            Ok(())
        }
        Command::Lint { dir } => {
            let report = build::lint_dir(&dir, true)?;
            print!("{report}");
            if !report.is_empty() {
                bail!("problems found");
            }
            Ok(())
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
        Command::Npa { command } => match command {
            NpaCommand::List { exe, archive } => npa_list(&exe, &archive),
            NpaCommand::Extract { exe, archive, output, prefix } => npa_extract(&exe, &archive, &output, &prefix),
        },
        Command::Nss { command } => match command {
            NssCommand::Check { paths } => nss_check(&paths),
            NssCommand::Refs { dir } => nss_refs(&dir),
            NssCommand::Route { dir } => nss_route(&dir),
            NssCommand::Builtins { dir, names, top } => nss_builtins(&dir, &names, top),
        },
        Command::Mages { command } => mages::run(command),
    }
}

fn convert_script(game: &Path, script: u32, output: &Path, font: Option<&Path>, title: &str, webp: Option<Option<u8>>, audio: AudioFormat) -> Result<()> {
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
    let ir = Script { title: title.to_string(), width: SCREEN.0, height: SCREEN.1, window: Some(window), ops, version: Some(version::line()), ..Default::default() };

    let source = build::webp_tree(output, PICTURE_DIRS, webp, conv_cons::emit(&ir))?;
    build::reencode_audio(output, AUDIO_DIRS, audio)?;
    let lost = build::write_game(output, &ir, &source, Some(&build::read_font(font)?), None)?;
    println!("{file} ({}) -> {}", cd.name, output.join("0.txt").display());
    build::summary(output, &ir, lost, Default::default(), None)
}


/// Turns an archive entry name into a relative path that stays inside the
/// output directory. Backslashes are treated as directory separators.
pub(crate) fn safe_relative_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name.replace('\\', "/"));
    // ':' is rejected so Windows drive prefixes ("C:") can't slip through.
    let ok = !name.is_empty()
        && !name.contains(':')
        && path.components().all(|c| matches!(c, Component::Normal(_)));
    ok.then_some(path)
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
