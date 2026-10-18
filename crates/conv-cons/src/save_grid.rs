//! What the engine's save grid ([`save_grid_screen`](crate::save_grid_screen))
//! shares with the script: the play time and lock variables, and the
//! details a save's label keeps.
//!
//! A save's label (`savegame2`, read back with `getsavestr`) holds its date,
//! play time, chapter and last line, `|` between them. The play time is
//! CONscripter's (`getplaytime`), which saves keep; a new game sets it to 0
//! ([`NEW_GAME`]).

use conv_core::ir::SaveGrid;

use crate::menu::{self, LABEL, STR_TMP, TMP};

/// The play time in seconds, read for a save's label.
const PLAY: &str = "%147";
/// What a new game runs.
pub(crate) const NEW_GAME: &str = "setplaytime 0";
/// 1 in delete mode.
pub(crate) const DELETING: &str = "%149";

/// 1 while that slot is locked.
pub(crate) const LOCKED: &str = "%165";
/// Slot n's lock is the global %(LOCK_BASE + n), n 1-100 (the user's
/// slots only).
pub(crate) const LOCK_BASE: u32 = 1010;

/// Adds "|play time|chapter|last line" to [`LABEL`] (a save's label after
/// its date).
pub(crate) fn details(w: &mut impl FnMut(String), g: &SaveGrid) {
    w(format!("getplaytime {PLAY}"));
    w(format!("add {LABEL},\"|\""));
    let parts = if g.play_time_hm {
        vec![(format!("{PLAY}/3600"), 2), (format!("{PLAY}/60 mod 60"), 2)]
    } else {
        vec![(format!("{PLAY}/3600"), 3), (format!("{PLAY}/60 mod 60"), 2), (format!("{PLAY} mod 60"), 2)]
    };
    for (n, (part, digits)) in parts.into_iter().enumerate() {
        if n > 0 {
            w(format!("add {LABEL},\":\""));
        }
        w(format!("mov {TMP},{part}"));
        // Hours stop at 99 (the English Muramasa port's `GetDataInfoPlayTime`).
        if g.play_time_hm && n == 0 {
            w(format!("if {TMP} > 99 mov {TMP},99"));
        }
        if digits == 3 {
            w(format!("if {TMP} < 100 add {LABEL},\"0\""));
        }
        w(format!("if {TMP} < 10 add {LABEL},\"0\""));
        w(format!("itoa {STR_TMP},{TMP}"));
        w(format!("add {LABEL},{STR_TMP}"));
    }
    w(format!("add {LABEL},\"|\":add {LABEL},{}", menu::CHAPTER));
    // With its line breaks, as the original shows it.
    w(format!("getlog {STR_TMP},0"));
    w(format!("add {LABEL},\"|\":add {LABEL},{STR_TMP}"));
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use conv_core::ir::{GridText, Placed, Rect};

    fn placed(name: &str, x: i32, y: i32) -> Placed {
        Placed { image: format!("sys/{name}.png"), x, y, w: 60, h: 80 }
    }

    fn text(x: i32, y: i32, lines: u32, vertical: bool) -> GridText {
        GridText { x, y, font_size: 18, lines, chars: 20, line_gap: 0, char_gap: -1, colour: "#000000".into(), vertical }
    }

    pub(crate) fn grid() -> SaveGrid {
        SaveGrid {
            save_pages: vec!["sys/save1.png".into(), "sys/save2.png".into()],
            load_pages: vec!["sys/load1.png".into(), "sys/load2.png".into()],
            frames: Some(placed("frames", 0, 0)),
            first: (600, 34),
            row_step: 100,
            column_step: -190,
            rows: 5,
            columns: 4,
            thumb_w: 160,
            thumb_h: 90,
            hover: placed("hover", 0, 0),
            hover_offset: (-3, -3),
            new_mark: placed("new", 0, 0),
            new_offset: (164, 64),
            previous: placed("previous", 790, 34),
            next: placed("next", 790, 130),
            delete: placed("delete", 790, 226),
            delete_on: placed("delete_on", 790, 226),
            close: placed("close", 810, 322),
            preview: Rect { x: 810, y: 416, w: 192, h: 108 },
            date: text(821, 516, 1, false),
            play_time: text(68, 528, 1, false),
            chapter: text(929, 41, 1, true),
            line: text(898, 45, 3, true),
            saved: Some(placed("saved", 462, 88)),
            fade_ms: 300,
            across: false,
            tabs: Vec::new(),
            popup: None,
            empty: None,
            quick_page: false,
            play_time_prefix: String::new(),
            play_time_hm: false,
        }
    }
}
