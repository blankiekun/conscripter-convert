//! The NScripter variables the output stage (conv-cons) keeps for itself.
//! A game's own variables ([`crate::ir::Var`], numbered with
//! [`number`]) must stay off them: an engine places its flags in the gaps
//! (see the table), and the build refuses a game variable that falls in
//! one of these ranges.
//!
//! Free for games: %10-%20 and %29-%99 (locals), %196-%899 (%290 up in a
//! script with a tile puzzle, [`PUZZLE`]), %1111-%1999,
//! %5096 up (to the script's 8,192). Engines today: Xuse %10-%12 and its
//! presets from %40, globals %200 up; Nitroplus fork variables %50-%80,
//! story locals from %300, `#` globals %800-%899 then %5200 up; MAGES
//! locals %60-%92, globals %410-%439, scene-read flags %1200 up.
//!
//! A new range in conv-cons goes here first; conv-cons' tests check that
//! every variable its code names is in this table.

use crate::ir::Var;
use std::ops::RangeInclusive;

/// The ranges conv-cons uses, with who uses them.
pub const RESERVED: &[(RangeInclusive<u32>, &str)] = &[
    (0..=9, "scratch, %9 the button clicked"),
    // Kept numbered on purpose: the overlay's, static's and animation's
    // state per link-scene level, %/$ 21-28, 130-137 and 182-189 (`defarr`
    // holds no strings, and an engine string array isn't worth one use).
    (21..=28, "the animation's state per link-scene level (ANIMATION_SAVED + depth)"),
    (100..=195, "menus and screens: scratch, backlog, bars, replays, galleries, overlay and static per link level, scene stack"),
    (900..=1010, "the classic menus' globals: save page, volumes, text speed, settings; deleted slots %910 + slot; voice switches %970 + character"),
    (1011..=1110, "save grid locks: %1010 + slot (1-100)"),
    (2000..=2199, "link targets visited, before ?conv_link_visited: read at start from an older gloval.sav"),
    (2200..=2775, "CGs seen (9 × 64), before ?conv_cg_seen: read at start from an older gloval.sav"),
    (2800..=2899, "movies seen, before ?conv_movie_seen: read at start from an older gloval.sav"),
    (2900..=2999, "scene stack across loads, bar state, settings sliders and pairs, title openings, picture extras"),
    (3000..=3599, "scene replay unlocks, before ?conv_scene_unlocked: read at start from an older gloval.sav"),
    (3600..=3631, "progress flags, before ?conv_progress: read at start from an older gloval.sav"),
    (3639..=3639, "the startup notice dismissed, before it was %conv_notice_seen: read once at start from an older gloval.sav"),
    (3640..=3647, "settings switches 0-7"),
    (3650..=4095, "scene replays watched, before ?conv_scene_replayed: read at start from an older gloval.sav"),
    (4096..=5095, "settings switches 20 up"),
];

/// NScripter sprites 980-998: more of the menus' slots, drawn over the
/// game with `spz`. A game's sprites (conv-cons' sprite 62 + IR layer) and
/// the menus' full screens (900-979, 999) stay off them; no game used them
/// when they were taken (2026-10), so old saves hold nothing there.
pub const MENU_MORE_SPRITES: std::ops::Range<u32> = 980..999;

/// The tile puzzle's state, kept only in a script that has one (a Xuse
/// game's globals are there).
pub const PUZZLE: RangeInclusive<u32> = 190..=289;

/// A variable's NScripter number: locals from %10, globals from `border`
/// (the script's global border).
pub fn number(v: Var, border: u32) -> u32 {
    let base = if v.global { border } else { 10 };
    base + u32::from(v.index)
}

/// The range `n` is reserved for, if any.
pub fn reserved(n: u32) -> Option<&'static str> {
    RESERVED.iter().find(|(r, _)| r.contains(&n)).map(|(_, who)| *who)
}
