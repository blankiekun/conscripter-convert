//! The BGM room ("bgm" in the extras; `BgmMode.cpp`, `BgmMode.004` in
//! `HD/gd.arc`), seen under wine with a 100% save.
//!
//! A column of 22 song titles under a "bgm" heading (`SubModeCommon.004`
//! #17) with an exit (#15). The exe's table (0x5001d0, 40 bytes a song)
//! places song i at (120, 71 + 20 i) with a 261 × 20 hit box and picture
//! #i, a grid of 2 × 4 cells of 21 px: normal, under the pointer, pressed
//! (the fourth row empty), on the right the same while the song plays.
//! The last song is left out until progress bit 0 (0x51eb3c) is set.
//! A click plays the song (again: nothing changes); it goes on after the
//! room is left. 0x500598 maps a BGM track to its song, to mark the one
//! playing.

/// The BGM track of each song, in list order (inverse of 0x500598).
pub const TRACKS: [usize; 22] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 17, 22];
/// The song shown only once progress bit [`LOCK_BIT`] is set.
pub const LOCKED: usize = 21;
pub const LOCK_BIT: u32 = 0;
pub const X: i32 = 120;
pub const Y: i32 = 71;
pub const DY: i32 = 20;
