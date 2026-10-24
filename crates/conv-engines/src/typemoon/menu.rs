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
//! save (`savetitle`, by its phase), the 39 autosaves (`saveauto`).

use std::path::Path;

use anyhow::Result;
use conv_core::ir::{GridText, Placed, Rect, SaveList, SystemMenu, SystemPage, SystemTab};
use image::{RgbaImage, imageops};

use super::title::{Parts, px};

const DIR: &str = "sys/menu";

/// A point of the 640 × 448 stage on the screen.
fn stage(x: f32, y: f32) -> (i32, i32) {
    (px(x * 3.0), px(y * 1080.0 / 448.0))
}

/// The menu's ground at 1920 × 1080, at rest (also under Config).
///
/// src: FUN_0014b880, drawn while the menu is open (FUN_0014cba0 starts it,
/// FUN_00149630(1/18)): part 0x52 full-screen, then 0x53-0x5A centred
/// (flags 0x14) at x 320 of the 640 × 448 stage (0x4E at 283.25, 0x50 at
/// 276.5), 0x53-0x56 at y 720 - (132, 107, 85, 55), 0x57-0x5A at (103.5,
/// 78.5, 56.5, 26.5) of a 720 high screen: each band's edge on the screen's
/// bottom or top. Their opacities (64, 96, 160, 224) and (64, 96, 128, 160)
/// / 255. The ids: FUN_001a90c0 numbers textures in load order; 0x53-0x5A
/// fit only DISP_PARTS' menuBG04_0..3 and menuBG05_0..3 (half heights 198,
/// 160.5, 127.5, 82.5 and 155.5, 118, 85, 40 at 1080, the offsets × 1.5), so
/// 0x52 is the texture before them, menuBG01_upscale (1920 × 1080).
/// ours: no animation (the bands ease in over 60 frames, the ground fades
/// in by 1/18 and out by 1/30 a frame); the menu's own fade covers it.
pub(super) fn ground(romfs: &Path) -> Result<RgbaImage> {
    let disp = Parts::read(romfs, "DISP_PARTS")?;
    let mut out = disp.get("menuBG01_upscale")?.clone();
    // src: FUN_0014b880: (part, stage x, y of 720, opacity of 255)
    let bands = [("menuBG04_0_upscale", 320.0, 720.0 - 132.0, 64), ("menuBG04_1_upscale", 320.0, 720.0 - 107.0, 96), ("menuBG04_2_upscale", 320.0, 720.0 - 85.0, 160), ("menuBG04_3_upscale", 320.0, 720.0 - 55.0, 224), ("menuBG05_0_upscale", 320.0, 103.5, 64), ("menuBG05_1_upscale", 283.25, 78.5, 96), ("menuBG05_2_upscale", 320.0, 56.5, 128), ("menuBG05_3_upscale", 276.5, 26.5, 160)];
    for (name, x, y, alpha) in bands {
        let mut band = disp.get(name)?.clone();
        for p in band.pixels_mut() {
            p[3] = (u32::from(p[3]) * alpha / 255) as u8;
        }
        let at = |c: f32, size: u32| (c - size as f32 / 2.0).round() as i64;
        // src: FUN_0014b880: flags 0x14, centred; x of 640, y of 720, here at 1080
        imageops::overlay(&mut out, &band, at(x * 3.0, band.width()), at(y * 1.5, band.height()));
    }
    Ok(out)
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
    // src: FUN_0014b880 (ground()): the ground, opening with the menu
    let ground = if dry_run { None } else { Some(imageops::resize(&ground(romfs)?, 1280, 720, imageops::FilterType::Triangle)) };
    let mut backdrop = vec![placed("ground", ground, (0, 0))?];
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
        // src: exe 0x14f240 (FUN_001a62c0's 1280 / 640 × 100 × 0.827 = 165 by
        // 720 / 448 × 53 × 1.092 = 93, scaled 1.15: 190 × 107; exe 0x14f240)
        thumb: Rect { x: tx, y: ty, w: 190, h: 107 },
        // ours: "%Y/%m/%d %H:%M" in the font, 11 px a digit (5.5 x 3 x 2/3,
        // size 22); the hour and minute land 3.8 stage units (8 px) left
        // src: FUN_0014f240: 12 digits of savedate_number (0x223, year, month,
        // day) and savetime_number (0x228, hour, minute), 24 x 32 cells from
        // tables 0x692298/0x6922F0, at the row's place + table 0x450650:
        // x 118 + 5.5 a digit (the hour at 182.333), y 24.889
        date: text(rel(133.667 + 118.0, 24.889), 22, 16, 1),
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
    // src: exe 0x14cba0 (DAT_00c359f4 + 1/18 a frame: 18 frames, 300 ms, opening);
    // ours: the same for closing and tabs (0x14d8c0 closes in 12 or 30 frames)
    Ok(SystemMenu { backdrop, tabs, fade_ms: 300 })
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
