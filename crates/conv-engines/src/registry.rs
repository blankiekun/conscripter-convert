//! Every game the converter builds: how to tell it from its folder and
//! the command that builds it. `conscripter-convert games --json` prints
//! it; the GUI's list (`gui/games.py`) is checked against it.
//!
//! A new game is one entry here (and its document, `docs/games/<id>.md`).

use std::path::Path;

pub struct GameInfo {
    /// As in `docs/games/<id>.md` and `tests/snapshots/<id>.txt`.
    pub id: &'static str,
    pub name: &'static str,
    pub engine: &'static str,
    /// Paths that must all exist in the game folder (a trailing `/`: a
    /// folder).
    pub markers: &'static [&'static str],
    /// The converter's subcommand (before the game folder).
    pub command: &'static [&'static str],
    /// The recommended options for the whole game (after the game folder).
    pub options: &'static [&'static str],
}

pub const GAMES: &[GameInfo] = &[
    GameInfo {
        id: "saihate-no-ima",
        name: "最果てのイマ COMPLETE",
        engine: "Xuse",
        markers: &["Farthest2015.exe", "HD/CDFiles/", "CD/"],
        command: &["convert-game"],
        options: &[],
    },
    GameInfo {
        id: "muramasa",
        name: "装甲悪鬼村正",
        engine: "Nitroplus NSS",
        markers: &["Muramasa.exe", "nss.npa", "cg.npa", "sound.npa", "voice.npa", "system.npa"],
        command: &["convert-nss"],
        options: &["--file", "ma00_000.nss", "--chain", "2000"],
    },
    GameInfo {
        id: "django",
        name: "続・殺戮のジャンゴ",
        engine: "Nitroplus NSS",
        markers: &["Django.exe", "nss.npa", "cg.npa", "sound.npa", "voice.npa", "system.npa"],
        command: &["convert-nss"],
        options: &[],
    },
    GameInfo {
        id: "oretsuba",
        name: "俺たちに翼はない (PS Vita)",
        engine: "MAGES TaskList",
        markers: &["media/SCRIPT.cpk", "media/CG.cpk", "sce_sys/param.sfo"],
        command: &["mages", "convert"],
        options: &[],
    },
];

/// The games whose markers are all in `dir`.
pub fn detect(dir: &Path) -> Vec<&'static GameInfo> {
    GAMES
        .iter()
        .filter(|g| g.markers.iter().all(|m| match m.strip_suffix('/') {
            Some(folder) => crate::find(dir, folder).is_dir(),
            None => crate::find(dir, m).is_file(),
        }))
        .collect()
}

/// [`GAMES`] as JSON: an array of objects with the fields above.
pub fn json() -> String {
    let s = |v: &str| format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""));
    let list = |l: &[&str]| format!("[{}]", l.iter().map(|v| s(v)).collect::<Vec<_>>().join(", "));
    let games: Vec<String> = GAMES
        .iter()
        .map(|g| {
            format!(
                "  {{\"id\": {}, \"name\": {}, \"engine\": {}, \"markers\": {}, \"command\": {}, \"options\": {}}}",
                s(g.id),
                s(g.name),
                s(g.engine),
                list(g.markers),
                list(g.command),
                list(g.options)
            )
        })
        .collect();
    format!("[\n{}\n]\n", games.join(",\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The GUI keeps its own list (with its options and checks): each of
    /// its games is one of these, with the same markers, and none is
    /// missing.
    #[test]
    fn the_gui_lists_the_same_games() {
        let gui = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gui/games.py")).unwrap();
        assert_eq!(gui.matches("        key=\"").count(), GAMES.len(), "gui/games.py has another number of games");
        for g in GAMES {
            assert!(gui.contains(&format!("key=\"{}\"", g.id)), "gui/games.py lacks {}", g.id);
            let markers = g.markers.iter().map(|m| format!("\"{m}\"")).collect::<Vec<_>>().join(", ");
            assert!(gui.contains(&format!("markers=({markers})")), "gui/games.py: {}'s markers differ", g.id);
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = GAMES.iter().map(|g| g.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), GAMES.len());
    }
}
