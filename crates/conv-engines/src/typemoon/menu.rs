//! 月姫's system menu (`FUN_0014c370` draws its bar, `FUN_0014f240` the
//! save list), from `MENU_PARTS` and `SAVE_PARTS` in `allui`: the bar's
//! Save, Load, Config, FlowChart and Title, beside the save list of 4
//! rows a page.
//!
//! Coordinates: the exe draws both on a 640 × 448 stage, the parts at
//! their 1920 × 1080 size; here × 2 and × 720 / 448. Left out: the
//! header's chapter title (`system_phasetitle`, by the phase) and scene
//! name, the cursor's line and light (`systemcursor_*`), the route
//! picture of the slot under the pointer (`saveinfobg_arc`/`_ciel`, by
//! the save's phase: the unknown one is drawn), the chapter label of each
//! save (`savetitle`, by its phase), the date and time drawn in the
//! picture digits, the 39 autosaves (`saveauto`).

use std::path::Path;

use anyhow::Result;
use conv_core::ir::{GridText, Placed, Rect, SaveList, SystemMenu, SystemPage, SystemTab};
use image::RgbaImage;

use super::title::{Parts, px};

const DIR: &str = "sys/menu";

/// A point of the 640 × 448 stage on the screen.
fn stage(x: f32, y: f32) -> (i32, i32) {
    (px(x * 3.0), px(y * 1080.0 / 448.0))
}

/// The system menu, and (unless `dry_run`) its pictures in `<output>/sys/menu`.
/// `languages`: the build has both (the settings' 言語 switches between them).
pub fn system_menu(romfs: &Path, output: &Path, dry_run: bool, languages: bool) -> Result<SystemMenu> {
    let (menu, save) = if dry_run { (None, None) } else { (Some(Parts::read(romfs, "MENU_PARTS")?), Some(Parts::read(romfs, "SAVE_PARTS")?)) };
    let write = |name: &str, img: Option<RgbaImage>| -> Result<(String, i32, i32)> {
        let file = format!("{DIR}/{name}.png");
        let (w, h) = img.as_ref().map_or((0, 0), |i| (i.width() as i32, i.height() as i32));
        if let Some(img) = img {
            let dest = output.join(&file);
            std::fs::create_dir_all(dest.parent().unwrap())?;
            img.save(&dest)?;
        }
        Ok((file, w, h))
    };
    let placed = |name: &str, img: Option<RgbaImage>, (x, y): (i32, i32)| -> Result<Placed> {
        let (image, w, h) = write(name, img)?;
        Ok(Placed { image, x, y, w, h })
    };
    let part = |p: &Option<Parts>, name: &str| p.as_ref().map(|p| p.scaled(name, None)).transpose();

    // src: FUN_0014c370 (the bar) and FUN_0014f240 (the list):
    // config_innerbg (0x22A) at (120, 0), system_titlebg (0x1F7) at (0, 0),
    // system_scenebg (0x1F8) at (250.67, 0), underline (0x1FC) at
    // (0, 29.87), menu_window (0x1F6) at (18.67, 1); the route panel
    // (saveinfobg, 0x224-0x226) at (453.33, 39.41)
    // guess: the game dimmed under it (the original's ground not traced)
    let dim = (!dry_run).then(|| RgbaImage::from_pixel(1280, 720, image::Rgba([0, 0, 0, 160])));
    let mut backdrop = vec![placed("dim", dim, (0, 0))?];
    backdrop.push(placed("inner", part(&save, "config_innerbg")?, stage(120.0, 0.0))?);
    backdrop.push(placed("panel", part(&save, "saveinfobg_unknown")?, stage(453.333, 39.407))?);
    backdrop.push(placed("titlebg", part(&menu, "system_titlebg")?, stage(0.0, 0.0))?);
    backdrop.push(placed("scenebg", part(&menu, "system_scenebg")?, stage(250.667, 0.0))?);
    backdrop.push(placed("underline", part(&menu, "underline")?, stage(0.0, 29.867))?);
    backdrop.push(placed("window", part(&menu, "menu_window")?, stage(18.667, 1.0))?);

    // src: FUN_0014f240: 4 rows at (133.67, 36.92 + 93.75 a row), save_win's
    // (32, 32)-(992, 304) at rest and (31, 336)-(991, 608) chosen; the
    // thumbnail at (151, row + 23.23); the slot's text (FUN_001ab710) at
    // (274.67, row + 79.64)
    let (x, y) = stage(133.667, 36.919);
    let row = placed("row", save.as_ref().map(|p| p.scaled("save_win", Some((32, 32, 960, 272)))).transpose()?, (0, 0))?;
    // The size without the picture too (a dry run): the hit box.
    let row = Placed { w: px(960.0), h: px(272.0), ..row };
    let row_hover = placed("row_on", save.as_ref().map(|p| p.scaled("save_win", Some((31, 336, 960, 272)))).transpose()?, (0, 0))?;
    let rel = |sx: f32, sy: f32| {
        let (a, b) = stage(sx, sy);
        (a - x, b)
    };
    let (tx, ty) = rel(151.0, 23.230);
    let text = |(x, y): (i32, i32), size: u32, chars: u32, lines: u32| GridText { x, y, font_size: size, lines, chars, line_gap: 4, char_gap: 0, colour: "#202838".into(), vertical: false };
    let (lx, ly) = rel(274.667, 79.644);
    let list = SaveList {
        slots: 100,
        rows: 4,
        x,
        y,
        row_step: px(93.748 * 1080.0 / 448.0),
        row,
        row_hover,
        // guess: the thumbnail's size (16:9 in the window's left part)
        thumb: Rect { x: tx, y: ty, w: 192, h: 108 },
        // guess: the date where the picture digits are, in the font
        date: text((lx, ty), 20, 20, 1),
        chapter: text((lx, ty + 28), 22, 18, 1),
        line: text((lx, ly - 10), 18, 18, 2),
        preview: None,
    };

    // src: FUN_0014c370: the buttons (widgets 0xC5-0xC9, hit box 74 × 29 at
    // table 0x691438) at (26.67, 43.56 + 31.11 a row)
    // ours: menu_btn's cells cut 240 × 104 from (24, 26) so the hit box is
    // centred on each label, as measured from its alpha; its columns at
    // rest, under the pointer, lit
    // The settings (settings.rs); a language picked there in game switches.
    let config = SystemPage::Settings(languages.then(|| super::language::SWITCH.into()));
    let mut tabs = Vec::new();
    for (r, page) in [SystemPage::Save(list.clone()), SystemPage::Load(list), config, SystemPage::Unavailable, SystemPage::Title].into_iter().enumerate() {
        let r = r as u32;
        let img = menu.as_ref().map(|p| p.cells("menu_btn_ja", &[0, 1, 2].map(|c| (24 + 240 * c, 26 + 104 * r, 240, 104)))).transpose()?;
        let button = placed(&format!("tab{r}"), img, stage(26.667, 43.556 + 31.111 * r as f32))?;
        tabs.push(SystemTab { button, page });
    }
    // guess: the fade (the bar's opacity DAT_00c359f4 steps not traced)
    Ok(SystemMenu { backdrop, tabs, fade_ms: 200 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_from_the_stage() {
        let m = system_menu(Path::new("/nonexistent"), Path::new("/nonexistent"), true, false).unwrap();
        // The bar's first button at (80, 105) of 1080p, the rows 226 apart from y 89.
        assert_eq!((m.tabs[0].button.x, m.tabs[0].button.y), (53, 70));
        let SystemPage::Save(l) = &m.tabs[0].page else { panic!() };
        assert_eq!((l.x, l.y, l.row_step), (267, 59, 151));
    }
}
