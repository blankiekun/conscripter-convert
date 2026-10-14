//! System screens built from SystemData's atlases. Their layout tables
//! are in the eboot, which is still encrypted, so the parts' rectangles
//! are measured from the pictures (`research/oretsuba/atlas.py`) and the
//! placement on screen is ours.

use conv_core::ir::{TitleAction, TitleButton, TitleScreen};

use super::assets::ArchiveAssets;

/// `title_bg.png` (960×965): the title picture in the top 544 rows; the
/// menu labels below, normal in x 0-170 and highlighted in x 170-340,
/// one row every 36 pixels from y 545.
const TITLE_ROW_Y: u32 = 545;
const TITLE_ROW_H: u32 = 36;
const TITLE_CELL_W: u32 = 170;

/// The title screen: START, LOAD, and GALLERY opening the extras (with
/// the settings inside), or without extras CONFIG and GALLERY greyed.
pub fn title(assets: &mut ArchiveAssets, extras: bool) -> Option<TitleScreen> {
    let (background, _, _) = assets.system_cells("title_bg.png", "title.png", &[(0, 0, 960, 544)], 100)?;
    // (row in the atlas, action); conv-cons opens the extras with the
    // settings action when there are extras.
    let rows: &[(u32, TitleAction)] = if extras {
        &[(2, TitleAction::Start), (1, TitleAction::Load), (3, TitleAction::Settings)]
    } else {
        &[(2, TitleAction::Start), (1, TitleAction::Load), (4, TitleAction::Settings), (3, TitleAction::Unavailable)]
    };
    let mut buttons = Vec::new();
    for (i, &(row, action)) in rows.iter().enumerate() {
        let y = TITLE_ROW_Y + row * TITLE_ROW_H;
        let mut cells = vec![(0, y, TITLE_CELL_W, TITLE_ROW_H)];
        if action != TitleAction::Unavailable {
            cells.push((TITLE_CELL_W, y, TITLE_CELL_W, TITLE_ROW_H));
        }
        let (image, _, _) = assets.system_cells("title_bg.png", &format!("title_{row}.png"), &cells, 100)?;
        buttons.push(TitleButton { image, x: 960 - TITLE_CELL_W as i32 - 24, y: 330 + 40 * i as i32, action });
    }
    Some(TitleScreen { background, fade_ms: 500, buttons })
}
