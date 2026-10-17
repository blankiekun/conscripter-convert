//! System screens built from SystemData's atlases. The title's layout is
//! the eboot's (0x81083fd8); other parts' rectangles are measured from
//! the pictures (`research/oretsuba/atlas.py`).

use conv_core::ir::{MenuSounds, TitleAction, TitleButton, TitleScreen};

use super::assets::ArchiveAssets;

/// `title_bg.png` (960×965): the title picture in the top 544 rows; the
/// menu labels below, 173 wide, normal from x 0 and highlighted from x
/// 168, one row every 36 pixels from y 545 (QUICK LOAD, LOAD, START,
/// GALLERY, CONFIG). The eboot (0x81083fd8) draws row N centred at (786,
/// 258 + 41.25 N) (its pictures are placed by their centres: "Press START
/// button", 258 wide, at x 769): we have no quick save, so its place stays
/// empty.
// src: exe 0x81083fd8 (title_menu_init: atlas cells and their centres)
pub(super) const TITLE_ROW_Y: u32 = 545;
pub(super) const TITLE_ROW_H: u32 = 36;
pub(super) const TITLE_CELL_W: u32 = 173;
pub(super) const TITLE_LIT_X: u32 = 168;
/// Where the title's buttons go: the left edge, and the top of row N.
pub(super) const TITLE_X: i32 = 786 - TITLE_CELL_W as i32 / 2;

pub(super) fn title_y(row: u32) -> i32 {
    (258.0 + 41.25 * row as f32 - TITLE_ROW_H as f32 / 2.0).round() as i32
}

/// The title screen: START, LOAD, and GALLERY opening the extras (with
/// the settings inside), or without extras CONFIG and GALLERY greyed.
pub fn title(assets: &mut ArchiveAssets, extras: bool) -> Option<TitleScreen> {
    let (background, _, _) = assets.system_cells("title_bg.png", "title.png", &[(0, 0, 960, 544)], 100)?;
    // (row in the atlas, action); conv-cons opens the extras with the
    // settings action when there are extras.
    let rows: &[(u32, TitleAction)] = if extras {
        &[(1, TitleAction::Load), (2, TitleAction::Start), (3, TitleAction::Settings)]
    } else {
        &[(1, TitleAction::Load), (2, TitleAction::Start), (3, TitleAction::Unavailable), (4, TitleAction::Settings)]
    };
    let mut buttons = Vec::new();
    for &(row, action) in rows {
        let y = TITLE_ROW_Y + row * TITLE_ROW_H;
        let mut cells = vec![(0, y, TITLE_CELL_W, TITLE_ROW_H)];
        if action != TitleAction::Unavailable {
            cells.push((TITLE_LIT_X, y, TITLE_CELL_W, TITLE_ROW_H));
        }
        let (image, _, _) = assets.system_cells("title_bg.png", &format!("title_{row}.png"), &cells, 100)?;
        buttons.push(TitleButton { image, x: TITLE_X, y: title_y(row), action, ..Default::default() });
    }
    // guess: fade_ms (the eboot's title fade not found)
    Some(TitleScreen { background, fade_ms: 500, buttons, ..Default::default() })
}

/// SystemSE.AWB's two sounds, in every menu: the eboot ticks (0) as the
/// cursor moves and confirms (1) an item (the title's at 0x8105e7a6).
pub fn menu_sounds(assets: &mut ArchiveAssets) -> Option<MenuSounds> {
    // src: exe 0x8105e7a6 (SystemSE ids)
    let (hover, confirm) = (assets.system_sound(0), assets.system_sound(1));
    (hover.is_some() || confirm.is_some()).then_some(MenuSounds { hover, confirm })
}
