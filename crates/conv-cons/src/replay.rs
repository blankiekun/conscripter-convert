//! The scene replay (see [`SceneReplay`]), reached from the extras menu:
//! its menu of lists and an open list are screens the engine draws
//! (`screens/replay.lua`, `replay_list.lua`, written by
//! [`crate::extras_screens`]); only unlocked scenes are listed, each with a
//! "new" mark until it has been replayed ([`REPLAYED`]).
//!
//! A title jumps to its scene's label with [`REPLAYING`] set; the scene's
//! end ([`conv_core::ir::Op::SceneEnd`]) and the settings screen's
//! "back to the list" jump to [`END`], which stops the sound, clears the
//! screen and shows the list again. Both leave the scene's subroutine
//! calls unfinished: the replay never returns into them, and loading or
//! going back to the title starts over.

use conv_core::ir::SceneReplay;

use crate::{label, Flags, BUTTON_VAR};

pub(crate) const REPLAY: &str = "conv_replay";
/// Where a replayed scene ends.
pub(crate) const END: &str = "conv_replay_end";
/// 1 while a scene is replayed (saving is off, the settings screen leads
/// back to the list).
pub(crate) const REPLAYING: &str = "%152";
/// Scene n has been replayed (no "new" mark), n up to 445 (the size the
/// numbered flags had).
pub(crate) const REPLAYED: Flags = Flags { name: "conv_scene_replayed", size: 446 };
const LIST: &str = "conv_replay_list";
const BEGIN: &str = "conv_replay_begin";
pub(crate) const MAX_LISTS: usize = 98;
pub(crate) const MAX_ROWS: usize = 69;
// Variables (cleared when the extras close).
/// The open list (from 0), read by `replay_list.lua`.
pub(crate) const LIST_VAR: &str = "%153";
const TMP: &str = "%160";
/// The replay's variables are %152-%160.
const FIRST_VAR: u32 = 152;
const LAST_VAR: u32 = 160;
pub(crate) const VARS: [&str; 3] = [REPLAYING, LIST_VAR, TMP];

/// `back`: the extras menu's label.
pub(crate) fn replay(out: &mut String, r: &SceneReplay, back: &str) {
    let mut w = lines!(out);
    let lists = &r.lists[..r.lists.len().min(MAX_LISTS)];
    let max_rows = (r.rows * r.columns).min(MAX_ROWS);
    let b = BUTTON_VAR;

    // The menu closes with a list's number, an open list with a title's.
    w(format!("*{REPLAY}"));
    w(format!("csp -1:screen \"replay\",{b}"));
    w(format!("if {b} == 0 csp -1:goto *{back}"));
    w(format!("mov {LIST_VAR},{b}-1"));
    w(format!("*{LIST}"));
    w(format!("csp -1:screen \"replay_list\",{b}"));
    w(format!("if {b} == 0 goto *{REPLAY}"));
    w(format!("mov {TMP},{b}-1"));
    w(format!("tablegoto {LIST_VAR},{}", (0..lists.len()).map(|i| format!("*conv_replay_play{i}")).collect::<Vec<_>>().join(",")));
    w(format!("goto *{LIST}"));
    for (i, l) in lists.iter().enumerate() {
        let scenes = &l.scenes[..l.scenes.len().min(max_rows)];
        w(format!("*conv_replay_play{i}"));
        w(format!("tablegoto {TMP},{}", (0..scenes.len()).map(|k| format!("*conv_replay_go{i}_{k}")).collect::<Vec<_>>().join(",")));
        w(format!("goto *{LIST}"));
        for (k, s) in scenes.iter().enumerate() {
            w(format!("*conv_replay_go{i}_{k}"));
            if s.scene < REPLAYED.size {
                w(format!("mov {},1", REPLAYED.at(s.scene)));
            }
            w(format!("gosub *{BEGIN}:goto *{}", label(&s.label)));
        }
    }

    // A clean screen and story state for the scene.
    w(format!("*{BEGIN}"));
    w("csp -1:csp2 -1:bgmstop".into());
    w("bg black,1".into());
    // The playthrough's variables as a new game has them, but the replay's.
    w(format!("for {TMP}=10 to {}:mov %{TMP},0:next", FIRST_VAR - 1));
    w(format!("for {TMP}={} to 199:mov %{TMP},0:next", LAST_VAR + 1));
    w(format!("mov {},\"\"", crate::menu::CHAPTER));
    w(format!("mov {REPLAYING},1"));
    w("return".into());
    w(format!("*{END}"));
    w(format!("mov {REPLAYING},0"));
    w(format!("bgmstop:{}", (0..10).map(|c| format!("dwavestop {c}")).collect::<Vec<_>>().join(":")));
    w("textclear:textoff".into());
    w("csp -1:csp2 -1".into());
    w(format!("goto *{LIST}"));
}
