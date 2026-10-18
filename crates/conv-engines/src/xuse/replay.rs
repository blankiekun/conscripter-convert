//! The scene replay ("chapter" in the extras; `SubModeCommon.004`,
//! `SceneList.004` in `HD/gd.arc`), seen under wine with a 100% save.
//!
//! The extras' chapter item opens a menu of lists (one per route, from
//! `SubModeCommon.004`) under a "chapter" heading (#18); a list opens a
//! window over the menu (`SceneList.004`: #0 a black band the screen is
//! darkened through, by about 80 per channel; #1 the frame; #2 close;
//! #3 the row under the pointer, added to the screen; #4 "new") with up
//! to 3 columns of 23 titles. Only unlocked scenes are listed, in table
//! order, closing up the gaps. "new" marks a scene not replayed yet;
//! scenes flagged as H scenes are written in pink.
//!
//! [`super::exe::Tables::lists`] is the exe's table (0x4fe200: 16 × (u32 used, pointer to
//! (u32 count, pointer to entries)); an entry is 16 bytes: u32 scene bit,
//! u32 H flag, pointer to the script name, pointer to the label name).
//! Entries 4, 7 and 13-15 are empty; the menu lists the others in table
//! order. The replay list code is at 0x451090.
//!
//! The scene bit is the one `0x93` sets (unlocked; 0x51eaa0, the first
//! "RMSN" chunk of the global save); a second set (0x51eaec) records the
//! scenes replayed, set when the replay starts. A label (`rm<NNN><title>`
//! in `09_回想*.rio`, scripts 0014-0018) calls the replay set-up
//! (`09_回想初期化.rio`, 0019) and jumps into the story, which plays
//! until the scene's end (`0x94`). In a replay the right-click menu has
//! "シーン選択に戻る" (asked with "シーン選択に戻りますか？") and the button
//! tray has no save or load.

/// A list of the scene replay.
pub struct List {
    /// Its button in `SubModeCommon.004`, a column of four states (normal,
    /// under the pointer, pressed, disabled); the disabled one is empty
    /// in some.
    pub picture: usize,
    pub disabled: bool,
    /// The script with the scenes' labels.
    pub file: String,
    /// Scene bit, H scene, label (`rm<NNN><title>`).
    pub scenes: Vec<(u16, bool, String)>,
}

/// Each list's button (`SubModeCommon.004` #, disabled state drawn), by
/// table entry; they are not in the exe's table.
pub const BUTTONS: [Option<(usize, bool)>; 13] = [
    Some((5, true)),   // プロローグ
    Some((0, true)),   // 紅緒あずさ
    Some((1, true)),   // 本堂沙也加
    Some((4, true)),   // 塚本葉子
    None,
    Some((53, false)), // 伊月笛子-1
    Some((54, false)), // 伊月笛子-2
    None,
    Some((7, true)),   // 戦争編
    Some((6, true)),   // エピローグ
    Some((8, true)),   // BLOG-1
    Some((9, true)),   // BLOG-2
    Some((43, false)), // BLOG-3
];

/// Where the list window's parts go. src: exe 0x4529c0 (rows of 21 px,
/// 23 to a column, columns 258 px apart; a row's hit box from (17, 60),
/// its title at (21, 64) in 14 px, white or 0xc8c8ff for H scenes),
/// 0x453130 (the row under the pointer at the hit box - (3, 2), "new"
/// at the title - (5, 6)).
pub const ROWS: usize = 23;
pub const COLUMNS: usize = 3;
pub const COLUMN_DX: i32 = 258;
pub const ROW_DY: i32 = 21;
/// The row under the pointer, the "new" mark (both at a row's top left).
pub const SELECTOR_X: i32 = 14;
pub const NEW_X: i32 = 16;
pub const ROW_Y: i32 = 58;
/// The title.
pub const TEXT_X: i32 = 21;
pub const TEXT_Y: i32 = 64;
pub const FONT_SIZE: u32 = 14;
pub const TEXT_COLOR: &str = "#FFFFFF";
pub const H_COLOR: &str = "#FFC8C8";
/// The menu: lists from (624, 160) every 28 px (src: exe 0x4552c0, 33 px
/// with fewer than 6 items).
pub const MENU_X: i32 = 624;
pub const MENU_Y: i32 = 160;
pub const MENU_DY: i32 = 28;
/// The window's black band darkens the screen by this much per channel
/// (src: shot under wine, see the module doc).
pub const DARKEN: u8 = 80;

/// The title shown for a label: `rm<NNN><title>` → title.
pub fn title(label: &str) -> &str {
    let rest = label.strip_prefix("rm").unwrap_or(label);
    rest.trim_start_matches(|c: char| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_drop_the_label_number() {
        assert_eq!(title("rm101お茶会－０"), "お茶会－０");
        assert_eq!(title("rm510『ダーリン的行為（TypeCGC）』"), "『ダーリン的行為（TypeCGC）』");
    }
}
