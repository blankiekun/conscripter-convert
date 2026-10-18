use std::ffi::OsString;
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
        /// Leave the pictures and sounds out of the output: CONscripter
        /// reads them from the game's own HD/ and CD/ files (copy or link
        /// them into the output), which cons_archives.txt names
        #[arg(long)]
        original_archives: bool,
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
        /// default: the bundled IPA Gothic, which also goes in as fallback.ttf
        /// when the font lacks Japanese)
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
        /// Build without the movies (a full build otherwise fails when it
        /// finds no movie folder or no ffmpeg)
        #[arg(long)]
        no_movies: bool,
        /// Window caption
        #[arg(long, default_value = "CONscripter-convert")]
        title: String,
        /// The English release's folder (Full Metal Daemon Muramasa, its
        /// *.npk): its text replaces the Japanese
        #[arg(long)]
        english: Option<PathBuf>,
        /// Leave the pictures and sounds out of the output: CONscripter
        /// reads them from the game's own *.npa, which the output's
        /// cons_archives.txt names (put them in the game folder)
        #[arg(long)]
        original_archives: bool,
        #[command(flatten)]
        build: build::BuildArgs,
    },
    /// Build the game in a folder: recognise it (as `games <dir>` does) and
    /// run its convert command with the game's title and recommended
    /// options. `build <game> -o <output> [options]`; the options are the
    /// convert command's (--dry-run, --font, --webp, …) and replace the
    /// game's own (--title, --file, …). An English edition (Full Metal
    /// Daemon Muramasa, Tsukihime's fan patch) is built on the Japanese
    /// game: `build <english folder> --japanese <japanese folder> -o …`
    Build {
        /// The game folder, then -o <output> and the convert command's options
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1.., value_name = "GAME [OPTIONS]")]
        args: Vec<OsString>,
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
    /// Low-level tools for TYPE-MOON's archives (月姫 -A piece of blue
    /// glass moon-, Switch): MRG and MZP
    Typemoon {
        #[command(subcommand)]
        command: tools::typemoon::TypemoonCommand,
    },
    /// Split a built game's 0.txt into the script with text IDs and one
    /// text table per language (docs/string-tables.md)
    Strtab {
        /// The built game's 0.txt (Japanese)
        script: PathBuf,
        /// The build's --source-map (IDs by source instruction; needed with --english)
        #[arg(long)]
        map: Option<PathBuf>,
        /// The same game's 0.txt built in English, and its --source-map: its text becomes text.en.txt
        #[arg(long, num_args = 2, value_names = ["0.txt", "MAP"])]
        english: Option<Vec<PathBuf>>,
        /// Labels starting with this begin a scene (IDs are per scene); default: every label
        #[arg(long, default_value = "")]
        scene_prefix: String,
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    run(Cli::parse().command)
}

fn run(command: Command) -> Result<()> {
    match command {
        Command::Build { args } => {
            let args = build_args(args)?;
            println!("conscripter-convert {}", args[1..].iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" "));
            run(Cli::try_parse_from(args)?.command)
        }
        Command::ConvertGame { game, title, linear, original_archives, build } => {
            build::run(build, &title, xuse::build::OUTPUT_DIRS, |opts| xuse::build::build(&game, linear, original_archives, opts))
        }
        Command::ConvertScript { game, script, output, font, title, webp, audio } => {
            convert_script(&game, script, &output, font.as_deref(), &title, webp.get(false)?, audio.format(false)?)
        }
        Command::ConvertNss { game, file, scene, chain, movies, no_movies, title, english, original_archives, build } => {
            let run = nitroplus::build::Run { file: file.as_deref(), scene: scene.as_deref(), chain, movies: movies.as_deref(), no_movies, english: english.as_deref(), original_archives };
            build::run(build, &title, &nitroplus::assets::OUTPUT_DIRS, |opts| nitroplus::build::build(&game, &run, opts))
        }
        Command::Strtab { script, map, english, scene_prefix, output } => {
            let read = |p: &PathBuf| std::fs::read_to_string(p);
            let ja = read(&script)?;
            let map = map.as_ref().map(read).transpose()?;
            let en = english.as_ref().map(|e| Ok::<_, std::io::Error>((read(&e[0])?, read(&e[1])?))).transpose()?;
            let s = conv_cons::strtab::split(&ja, map.as_deref(), en.as_ref().map(|(a, b)| (a.as_str(), b.as_str())), &scene_prefix);
            std::fs::create_dir_all(&output)?;
            std::fs::write(output.join("0.txt"), &s.script)?;
            for (lang, table) in &s.tables {
                std::fs::write(output.join(format!("text.{lang}.txt")), table)?;
            }
            let st = &s.stats;
            println!("{} scenes, {} text lines, {} choice options, {} window lines in the tables; script {} bytes (was {})",
                st.scenes, st.text_lines, st.choices, st.window_lines, s.script.len(), ja.len());
            if st.english_missing > 0 {
                println!("{} text lines keep their Japanese in text.en.txt (no English line from the same source)", st.english_missing);
            }
            Ok(())
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
                let japanese = g.japanese.map(|id| format!(" --japanese <{id}>")).unwrap_or_default();
                println!("{} ({}, {}): conscripter-convert build <game>{japanese} -o <output> [--font <ttf>]", g.name, g.id, g.engine);
            }
            if games.is_empty() {
                bail!("no game found");
            }
            Ok(())
        }
        Command::Lint { dir } => {
            let report = build::lint_dir(&dir, Some(&build::dir_files(&dir)?))?;
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
        Command::Typemoon { command } => tools::typemoon::run(command),
    }
}

/// The convert command line `build` runs: `args` are the game folder and
/// the options given (with `--japanese <folder>` for an English edition).
fn build_args(mut args: Vec<OsString>) -> Result<Vec<OsString>> {
    use conv_engines::registry::{self, GAMES};
    let folder = PathBuf::from(args.remove(0));
    let japanese = match args.iter().position(|a| a == "--japanese") {
        Some(i) if i + 1 < args.len() => Some(PathBuf::from(args.drain(i..i + 2).nth(1).unwrap())),
        Some(_) => bail!("--japanese needs the Japanese game's folder"),
        None => None,
    };
    let found = registry::detect(&folder);
    let names = |l: &[&registry::GameInfo]| l.iter().map(|g| g.name).collect::<Vec<_>>().join(", ");
    // an English edition's folder can also hold all of the Japanese game's markers
    let editions: Vec<_> = found.iter().copied().filter(|g| g.japanese.is_some()).collect();
    let game = match (&editions[..], &found[..]) {
        ([g], _) => *g,
        ([], [g]) => *g,
        ([], []) => bail!("no game found in {}; supported: {}", folder.display(), names(&GAMES.iter().collect::<Vec<_>>())),
        _ => bail!("{} holds several games: {}", folder.display(), names(&found)),
    };
    let mut out: Vec<OsString> = vec!["conscripter-convert".into()];
    out.extend(game.command.iter().map(OsString::from));
    match (game.japanese, japanese) {
        (Some(id), Some(dir)) => {
            if !registry::detect(&dir).iter().any(|g| g.id == id) {
                bail!("{} is not {}'s folder", dir.display(), GAMES.iter().find(|g| g.id == id).unwrap().name);
            }
            out.extend([dir.into(), "--english".into(), folder.into()]);
        }
        (Some(id), None) => bail!("{} is {}: build it with --japanese <{id}'s folder>", folder.display(), game.name),
        (None, Some(_)) => bail!("{} is {}, not an English edition: --japanese is not needed", folder.display(), game.name),
        (None, None) => out.push(folder.into()),
    }
    let given = |flag: &str| args.iter().any(|a| a == flag || a.to_str().is_some_and(|a| a.starts_with(&format!("{flag}="))));
    for pair in game.options.chunks(2) {
        if !given(pair[0]) {
            out.extend(pair.iter().map(OsString::from));
        }
    }
    if !given("--title") && !game.options.contains(&"--title") {
        out.extend(["--title".into(), game.title.into()]);
    }
    out.extend(args);
    Ok(out)
}

fn convert_script(game: &Path, script: u32, output: &Path, font: Option<&Path>, title: &str, webp: Option<Option<u8>>, audio: AudioFormat) -> Result<()> {
    let file = format!("{script:04}.cd");
    let path = conv_engines::find(game, &format!("HD/CDFiles/{file}"));
    let bytes = map(&path)?;
    let cd = parse_cd(&path, &bytes)?;
    fs::create_dir_all(output)?;

    let mut assets = AssetBank::new(game, output)?;
    let window = assets.text_window().context("building the text window")?;
    let presets = Default::default(); // the presets live in script 2
    let names = label_names(game)?;
    let ops =
        lower_script(&cd, &file, Flow::Jumps, &mut assets, &presets, &names, &mut Vec::new()).with_context(|| format!("converting {file}"))?;
    let ir = Script { title: title.to_string(), width: SCREEN.0, height: SCREEN.1, window: Some(window), ops, version: Some(version::line()), language: "ja".into(), ..Default::default() };

    let source = build::webp_tree(output, PICTURE_DIRS, webp, false, conv_cons::emit(&ir))?;
    build::reencode_audio(output, AUDIO_DIRS, audio)?;
    let lost = build::write_game(output, &ir, &source, Some(&build::read_font(font)?), None, &version::media(webp, audio))?;
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
    use std::ffi::OsStr;

    #[test]
    fn safe_paths() {
        assert_eq!(safe_relative_path("a.png"), Some(PathBuf::from("a.png")));
        assert_eq!(safe_relative_path("dir\\a.png"), Some(PathBuf::from("dir/a.png")));
        for bad in ["", "../a", "/etc/passwd", "a/../../b", "C:\\x", "\\\\server\\x", "./a"] {
            assert_eq!(safe_relative_path(bad), None, "{bad}");
        }
    }

    /// `build` picks the game by its markers, adds its options and title
    /// unless given, and builds an English edition on the Japanese game.
    #[test]
    fn build_command_line() {
        let tmp = std::env::temp_dir().join(format!("conv-build-args-{}", std::process::id()));
        let (jp, en) = (tmp.join("jp"), tmp.join("en"));
        for (dir, files) in [(&jp, &["Muramasa.exe", "nss.npa", "cg.npa", "sound.npa", "voice.npa", "system.npa"][..]), (&en, &["muramasa_en.exe", "script.npk", "cg.npk", "font.npk"][..])] {
            fs::create_dir_all(dir).unwrap();
            for f in files {
                fs::write(dir.join(f), b"").unwrap();
            }
        }
        let line = |args: &[&OsStr]| build_args(args.iter().map(OsString::from).collect()).map(|v| v[1..].iter().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" "));
        let (j, e) = (jp.as_os_str(), en.as_os_str());
        let o = OsStr::new;
        assert_eq!(line(&[j, o("-o"), o("x")]).unwrap(), format!("convert-nss {} --file ma00_000.nss --chain 2000 --title 装甲悪鬼村正 -o x", jp.display()));
        assert_eq!(line(&[j, o("--chain"), o("2"), o("--title=T")]).unwrap(), format!("convert-nss {} --file ma00_000.nss --chain 2 --title=T", jp.display()));
        assert_eq!(
            line(&[e, o("-o"), o("x"), o("--japanese"), j]).unwrap(),
            format!("convert-nss {} --english {} --file ma00_000.nss --chain 2000 --title Full Metal Daemon Muramasa -o x", jp.display(), en.display())
        );
        assert!(line(&[e, o("-o"), o("x")]).is_err(), "an edition needs --japanese");
        assert!(line(&[e, o("--japanese"), e]).is_err(), "--japanese must be the Japanese game");
        assert!(line(&[j, o("--japanese"), j]).is_err(), "a Japanese game takes no --japanese");
        assert!(line(&[tmp.as_os_str()]).is_err(), "no game");
        fs::remove_dir_all(&tmp).unwrap();
    }
}
