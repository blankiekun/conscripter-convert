//! The settings' on/off switches ([`Switch`]): each one's index, which
//! gives its global "off" flag, and what putting it into effect does.
//! A new switch is one entry in [`def`], in the block of the game that
//! adds it, and nothing in the settings screens' code.
//!
//! Index blocks (claim the next free number in yours; a new game takes
//! the next hundred and says so here):
//!
//! | Indexes | Who |
//! |---|---|
//! | 0-19 | given before the blocks (shared 0-8; xuse 9-11, 13-16; 装甲悪鬼村正 12; 俺たちに翼はない 17-19) |
//! | 20-99 | switches every game may use |
//! | 100-199 | xuse (最果てのイマ) |
//! | 200-299 | mages (俺たちに翼はない) |
//! | 300-399 | nitroplus (装甲悪鬼村正) |
//! | 400-999 | new games, 100 each |
//!
//! The flags (1 = off; 0, the default, is on): indexes 0-7 at %3640,
//! 8-17 at %2910, 18-19 at %2950, 20 up at %4096 + index (the script
//! declares 8192 variables, [`crate::VARIABLE_RANGE`]).

use conv_core::ir::{Setting, Switch};

/// What putting a switch into effect does, in the settings' apply routine
/// (at startup, after a load and when it changes on the settings screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Apply {
    /// Nothing there: the switch is read where it matters (a click, a
    /// confirmation, a choice).
    Read,
    /// Off: this level's volume is 0, the level kept.
    Mutes(Setting),
    /// A command for on and one for off.
    Commands { on: &'static str, off: &'static str },
}

/// A switch's index (its block, see the module) and what applying it does.
pub(crate) fn def(s: Switch) -> (u32, Apply) {
    use Apply::*;
    match s {
        // Given before the blocks.
        Switch::WheelReadsOn => (0, Read),
        Switch::WheelClosesLog => (1, Read),
        Switch::ConfirmSave => (2, Read),
        Switch::ConfirmLoad => (3, Read),
        Switch::ConfirmDelete => (4, Read),
        Switch::ConfirmTitle => (5, Read),
        Switch::ConfirmQuit => (6, Read),
        Switch::ConfirmBackToChoice => (7, Read),
        Switch::VoiceKeepsPlaying => (8, Commands { on: "clickvoicestop 0", off: "clickvoicestop 1" }),
        Switch::BgmOn => (9, Mutes(Setting::BgmVolume)),
        Switch::SeOn => (10, Mutes(Setting::SeVolume)),
        Switch::VoiceOn => (11, Mutes(Setting::VoiceVolume)),
        Switch::ConfirmReset => (12, Read),
        Switch::Effects => (13, Commands { on: "effectoff 0", off: "effectoff 1" }),
        Switch::SavePreview => (14, Read),
        Switch::TextShadow => (15, Commands { on: "textshadow 1", off: "textshadow 0" }),
        Switch::ConfirmChapter => (16, Read),
        Switch::SceneSkipDialog => (17, Read),
        Switch::StopSkipAtChoice => (18, Commands { on: "selectkeep 0", off: "selectkeep 1" }),
        Switch::BgmFullForVoice => (19, Read),
        // 20-99: every game's.
        // 100-199: xuse's.
        // 200-299: mages's.
        // 300-399: nitroplus's.
    }
}

/// The switch's index.
pub(crate) fn index(s: Switch) -> u32 {
    def(s).0
}

/// What applying the switch does.
pub(crate) fn apply(s: Switch) -> Apply {
    def(s).1
}

/// Indexes end before this.
pub(crate) const MAX_INDEX: u32 = 1000;

/// The global that is 1 while the switch with index `i` is off.
pub(crate) fn off_var(i: u32) -> u32 {
    debug_assert!(i < MAX_INDEX);
    match i {
        0..=7 => 3640 + i,
        8..=17 => 2910 + i - 8,
        18..=19 => 2950 + i - 18,
        _ => 4096 + i,
    }
}

/// Lines setting `tmp` to the number of the "off" flag of switch
/// `kind` - 1 (a variable, `kind` > 0), as [`off_var`] does.
pub(crate) fn off_var_lines(kind: &str, tmp: &str) -> String {
    // From each range's first kind: the flag is kind plus this.
    [(0, off_var(0)), (8, off_var(8)), (18, off_var(18)), (20, off_var(20))]
        .iter()
        .map(|&(from, first)| format!("if {kind} > {from} mov {tmp},{}+{kind}\n", first as i64 - from as i64 - 1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every [`Switch`]. The match makes a new one fail to compile until
    /// it is listed here too.
    pub(crate) fn all() -> Vec<Switch> {
        use Switch::*;
        let all = vec![
            WheelReadsOn, WheelClosesLog, ConfirmSave, ConfirmLoad, ConfirmDelete, ConfirmTitle, ConfirmQuit, ConfirmBackToChoice, VoiceKeepsPlaying, BgmOn, SeOn,
            VoiceOn, ConfirmReset, Effects, SavePreview, TextShadow, ConfirmChapter, SceneSkipDialog, StopSkipAtChoice, BgmFullForVoice,
        ];
        for s in &all {
            match s {
                WheelReadsOn | WheelClosesLog | ConfirmSave | ConfirmLoad | ConfirmDelete | ConfirmTitle | ConfirmQuit | ConfirmBackToChoice | VoiceKeepsPlaying | BgmOn | SeOn | VoiceOn
                | ConfirmReset | Effects | SavePreview | TextShadow | ConfirmChapter | SceneSkipDialog | StopSkipAtChoice | BgmFullForVoice => {}
            }
        }
        all
    }

    /// Two switches with one index would share their flag.
    #[test]
    fn each_switch_has_its_own_index_and_flag() {
        let mut seen = std::collections::BTreeMap::new();
        let mut flags = std::collections::BTreeSet::new();
        for s in all() {
            let i = index(s);
            assert!(i < MAX_INDEX, "{s:?}: index {i} past MAX_INDEX");
            if let Some(other) = seen.insert(i, s) {
                panic!("{s:?} and {other:?} both have index {i}");
            }
            assert!(flags.insert(off_var(i)));
            assert!(off_var(i) < crate::VARIABLE_RANGE);
        }
    }

    /// The flags keep their numbers (saves hold them) and the script's
    /// lookup finds the same ones.
    #[test]
    fn flags_stay_where_saves_have_them() {
        assert_eq!([0, 7, 8, 17, 18, 19, 20, 205].map(off_var), [3640, 3647, 2910, 2919, 2950, 2951, 4116, 4301]);
        let lines = off_var_lines("%125", "%5");
        for i in [0u32, 7, 8, 17, 18, 19, 20, 205] {
            let kind = i as i64 + 1;
            let mut tmp = 0;
            for l in lines.lines() {
                let (from, add) = l.trim_start_matches("if %125 > ").split_once(" mov %5,").unwrap();
                if kind > from.parse::<i64>().unwrap() {
                    tmp = add.trim_end_matches("+%125").parse::<i64>().unwrap() + kind;
                }
            }
            assert_eq!(tmp, off_var(i) as i64, "{i}");
        }
    }
}
