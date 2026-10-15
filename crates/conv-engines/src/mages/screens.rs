//! System screens built from SystemData's atlases. The title's layout is
//! the eboot's (0x81083fd8); other parts' rectangles are measured from
//! the pictures (`research/oretsuba/atlas.py`).

use conv_core::ir::{TitleAction, TitleButton, TitleScreen};

use super::assets::ArchiveAssets;

/// `title_bg.png` (960×965): the title picture in the top 544 rows; the
/// menu labels below, 173 wide, normal from x 0 and highlighted from x
/// 168, one row every 36 pixels from y 545 (QUICK LOAD, LOAD, START,
/// GALLERY, CONFIG). The eboot (0x81083fd8) draws row N at (786, 258 +
/// 41.25 N): we have no quick save, so its place stays empty.
const TITLE_ROW_Y: u32 = 545;
const TITLE_ROW_H: u32 = 36;
const TITLE_CELL_W: u32 = 173;
const TITLE_LIT_X: u32 = 168;

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
        buttons.push(TitleButton { image, x: 786, y: (258.0 + 41.25 * row as f32).round() as i32, action, ..Default::default() });
    }
    Some(TitleScreen { background, fade_ms: 500, buttons, ..Default::default() })
}
