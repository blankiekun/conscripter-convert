//! System screens built from SystemData's atlases. The title's layout is
//! the eboot's (0x81083fd8); other parts' rectangles are measured from
//! the pictures (`research/oretsuba/atlas.py`).

use conv_core::ir::{BootStep, MenuSounds, TitleAction, TitleButton, TitleScreen};

use super::assets::ArchiveAssets;
use super::convert::Assets;

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
    // src: exe 0x81085a4c (the screen fade 0x8115c634 from black, started
    // once the title is loaded: 4 % an update, 25 updates of 2 frames)
    Some(TitleScreen { background, fade_ms: 833, buttons, ..Default::default() })
}

/// SystemSE.AWB's two sounds, in every menu: the eboot ticks (0) as the
/// cursor moves and confirms (1) an item (the title's at 0x8105e7a6).
pub fn menu_sounds(assets: &mut ArchiveAssets) -> Option<MenuSounds> {
    // src: exe 0x8105e7a6 (SystemSE ids)
    let (hover, confirm) = (assets.system_sound(0), assets.system_sound(1));
    (hover.is_some() || confirm.is_some()).then_some(MenuSounds { hover, confirm })
}

/// The boot (the eboot's scenes 7 then 1, 0x8103a7b0): the logos in
/// `media/img/logo` on white, each faded in, held and faded out (scene 7,
/// 0x81081c16), then the opening OP00.usm (scene 1, 0x8108205c). The Vita's
/// save data check between them (scene 0xf) has no counterpart here.
pub fn boot(assets: &mut ArchiveAssets, logo_dir: &std::path::Path) -> Vec<BootStep> {
    // src: exe 0x810815bc (the logo of each step 0-3, 0x8103a7b0 stops after 3)
    const LOGOS: [&str; 4] = ["makerlogo_WH_vita", "brandlogo_5pb_vita", "Navel_logo", "criware960x544sw"];
    // src: exe 0x81081c16: alpha 6 % an update at 30 updates a second (the
    // log's logos 5.13-5.22 s apart), so 17 updates; held 4000 ms from the
    // fade-in's end, circle (or input bit 0x100) ending it after 3000 ms
    let mut steps: Vec<BootStep> = LOGOS
        .iter()
        .filter_map(|name| {
            let img = image::open(logo_dir.join(format!("{name}.png"))).ok()?;
            // src: exe 0x810815bc (drawn 960×544, the 960×540 pictures stretched)
            let img = image::imageops::resize(&img.to_rgba8(), super::convert::SCREEN_W as u32, super::convert::SCREEN_H as u32, image::imageops::FilterType::Triangle);
            let image = assets.put_system_png(&name.to_ascii_lowercase(), &img);
            Some(BootStep { image, backdrop: "white".into(), fade_in_ms: 567, hold_ms: 4000, fade_out_ms: 567, skip_after_ms: Some(3000), ..Default::default() })
        })
        .collect();
    // src: exe 0x8108205c (OP00 at once, circle or up stops it), 0x8108237a
    // (black for 30 updates after it, then the title)
    if let Some(image) = assets.movie(0) {
        steps.push(BootStep { image, movie: true, backdrop: "black".into(), hold_ms: 1000, skip_after_ms: Some(0), ..Default::default() });
    }
    steps
}
