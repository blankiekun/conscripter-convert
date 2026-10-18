//! 月姫's title (`FUN_0018d2e0` sets it up, `FUN_0018e200` runs it,
//! `FUN_0018d5d0` draws it): one of four looping movies by the scenes
//! read, the logo, the copyright, a pulsing light behind the logo's
//! subtitle, and the menu (Continue, New Game, Load Game, Config, Gallery)
//! from `TITLE_PARTS` in `allui`.
//!
//! Coordinates: the exe draws the logo on a 960 × 544 stage and the menu
//! on a 640 × 448 one, the parts at their 1920 × 1080 size; here × 2/3.
//! Left out: the "PRESS ANY BUTTON" step, the logo's particles
//! (`FUN_001907e0`, `FUN_00190e80`), the logo's three bands fading in one
//! after the other and the menu's rows after them (both come in with the
//! title's fade), and the extras menu (`extra_menu_ja`).

use std::path::Path;

use anyhow::{Context, Result};
use conv_core::ir::{TitleAction, TitleButton, TitleMovie, TitlePicture, TitleScreen};
use image::{RgbaImage, imageops};

use super::convert::{SCREEN_W, read_flag};
use super::{bntx, mrg};
use crate::build::map;

const DIR: &str = "sys/title";
/// 1920 × 1080 (the parts' size) to the screen.
const SCALE: f32 = SCREEN_W as f32 / 1920.0;

/// A system sound (`FUN_001acef0(n)`: table 0x4341A0, sound items
/// 0x10040D4 up, the bfsar's wave sounds `WSD_MOON_SE00001`…).
/// `ext`: as the build writes the wave sounds (`.wav` unless re-encoded).
fn se(name: &str, ext: &str) -> Option<String> {
    Some(format!("se/WSD_MOON_{name}.{ext}"))
}

pub(super) fn px(v: f32) -> i32 {
    (v * SCALE).round() as i32
}

/// The title, and (unless `dry_run`) its pictures in `<output>/sys/title`.
pub fn title(romfs: &Path, output: &Path, dry_run: bool, se_ext: &str) -> Result<TitleScreen> {
    let parts = if dry_run { None } else { Some(Parts::read(romfs, "TITLE_PARTS")?) };
    let save = |name: &str, img: Option<RgbaImage>| -> Result<String> {
        let file = format!("{DIR}/{name}.png");
        crate::output::record(&file);
        if let Some(img) = img {
            let dest = output.join(&file);
            std::fs::create_dir_all(dest.parent().unwrap())?;
            img.save(&dest).with_context(|| format!("writing {}", dest.display()))?;
        }
        Ok(file)
    };
    let part = |name: &str| parts.as_ref().map(|p| p.scaled(name, None)).transpose();

    // src: FUN_0018d5d0: the logo (part 0x13A, title_titlelog) at (340, 36.27)
    // on the 960 × 544 stage, the copyright (0x13C) at (436, 507.73), the
    // light (0x13E, titlemenu_lumi) at (360, 278.04), drawn added
    // (layer 0x1E), its opacity 0 → 1 → 0 by 1/120 a frame.
    let stage = |x: f32, y: f32| (px(x * 2.0), px(y * 1080.0 / 544.0));
    let mut pictures = Vec::new();
    let (x, y) = stage(360.0, 278.04);
    pictures.push(TitlePicture { image: save("lumi", part("titlemenu_lumi")?)?, x, y, add: true, keys: vec![(0, 0), (2000, 256), (4000, 0)], repeat: true });
    let (x, y) = stage(340.0, 72.0 * 544.0 / 1080.0);
    pictures.push(TitlePicture { image: save("logo", part("title_titlelog")?)?, x, y, ..Default::default() });
    let (x, y) = stage(436.0, 507.73);
    pictures.push(TitlePicture { image: save("copyright", part("titlemenu_(c)type-moon")?)?, x, y, ..Default::default() });

    // src: FUN_0018d1c0: the movie by the scenes read (`_RDST`): 560 the
    // Arcueid ending (14_00_ARC14_1), then 383 (05_01_CIEL05_3), then 562
    // (15_01B_CIEL15B_1); the movie table at 0x50B890 (looping); the music
    // (FUN_0018e200 case 0) _MPLY 2, or 72 with the fourth.
    let read = |n: u16| (read_flag(n), 1);
    let movie = |n: u32, when: Vec<_>| TitleMovie {
        movie: format!("movie/titlebg{n}.mp4"),
        bgm: (n == 4).then(|| "stream/MOON01_BGM59.ogg".to_string()),
        when,
    };
    let movies = vec![
        movie(4, vec![read(560), read(383), read(562)]),
        movie(3, vec![read(560), read(383)]),
        movie(2, vec![read(560)]),
        movie(1, vec![]),
    ];

    // src: FUN_0018d2e0 (the items shown: Continue only with a save to
    // resume; 4 and 6, the manual, never), the places at 0x452AF4 for 5
    // items (x 272, y 298.67 + 23.23 a row on the 640 × 448 stage; for 4,
    // 312.36 + 23.23: without Continue, New Game 9.54 higher);
    // the sounds (FUN_001acef0): 0 on a move, 1 on a choice, 6 New Game.
    // The strip's rows in titlemenu: New Game, Load Game, Config, Gallery,
    // Continue; its three columns at rest, chosen and lit.
    let menu = |v: f32| v * 1080.0 / 448.0;
    let (decide, start) = (se("SE00002", se_ext), se("SE00011", se_ext));
    let mut buttons = Vec::new();
    for (i, (row, action, click_se)) in [(4, TitleAction::Continue, decide.clone()), (0, TitleAction::Start, start), (1, TitleAction::Load, decide.clone()), (2, TitleAction::Settings, decide.clone()), (3, TitleAction::Extras, decide)].into_iter().enumerate() {
        let cells = if action == TitleAction::Unavailable { 1 } else { 3 };
        let img = parts.as_ref().map(|p| p.strip(row, cells));
        buttons.push(TitleButton {
            image: save(&format!("menu{row}"), img)?,
            x: px(272.0 * 3.0),
            y: px(menu(298.666_66 + 23.229_63 * i as f32)),
            action,
            click_se,
            shown_by: None,
        });
    }
    Ok(TitleScreen {
        background: save("bg1", part("title_bg1")?)?,
        // src: FUN_0018e200 case 2 (the black over it 1 → 0 by 0.012 a frame)
        fade_ms: 1389,
        buttons,
        pressed: true,
        hover_se: se("SE00001", se_ext),
        bgm: Some("stream/MOON01_BGM02.ogg".into()),
        // src: FUN_0018e200 New Game: FUN_0019c9b0(1.5) fades the music; the
        // white comes up by 0.01 a frame (case 0x14)
        start_bgm_fade_ms: 1500,
        start_white: true,
        movies,
        pictures,
        continue_hides: Some(px(menu(312.355_53 - (298.666_66 + 23.229_63)))),
        ..Default::default()
    })
}

/// A picture set of `allui` (`TITLE_PARTS`, …), decoded.
pub(super) struct Parts {
    textures: Vec<(String, RgbaImage)>,
}

impl Parts {
    pub(super) fn read(romfs: &Path, prefix: &str) -> Result<Parts> {
        let path = romfs.join("allui.mrg");
        let (hed, nam, bytes) = (map(&path.with_extension("hed"))?, map(&path.with_extension("nam"))?, map(&path)?);
        let entry = mrg::parse(&hed, Some(&nam))?.into_iter().find(|e| e.name.as_deref().is_some_and(|n| n.starts_with(prefix))).with_context(|| format!("allui has no {prefix}"))?;
        let data = super::unpack(mrg::data(&bytes, &entry)?)?;
        let textures = bntx::parse(&data)?.iter().map(|t| Ok((t.name.clone(), bntx::decode(t)?))).collect::<Result<_>>()?;
        Ok(Parts { textures })
    }

    pub(super) fn get(&self, name: &str) -> Result<&RgbaImage> {
        self.textures.iter().find(|(n, _)| n == name).map(|(_, t)| t).with_context(|| format!("no {name} in allui"))
    }

    /// A texture (or a part of it: x, y, w, h) at the screen's scale.
    pub(super) fn scaled(&self, name: &str, rect: Option<(u32, u32, u32, u32)>) -> Result<RgbaImage> {
        let t = self.get(name)?;
        let (x, y, w, h) = rect.unwrap_or((0, 0, t.width(), t.height()));
        let part = imageops::crop_imm(t, x, y, w, h).to_image();
        Ok(imageops::resize(&part, px(w as f32) as u32, px(h as f32).max(1) as u32, imageops::FilterType::Triangle))
    }

    /// Menu row `row`'s first `cells` cells of titlemenu side by side.
    fn strip(&self, row: u32, cells: u32) -> RgbaImage {
        // ours: the cells cut 312 × 88 from (24, 22) so the widget's hit box
        // (0, 4, 98 × 25 on the 640 × 448 stage, table 0x75CBD8) is centred
        // on each label, as measured from the picture's alpha
        let rects: Vec<_> = (0..cells).map(|c| (24 + 312 * c, 22 + 88 * row, 312, 88)).collect();
        self.cells("titlemenu", &rects).expect("titlemenu was read")
    }

    /// Parts of a texture (x, y, w, h) side by side at the screen's scale.
    pub(super) fn cells(&self, name: &str, rects: &[(u32, u32, u32, u32)]) -> Result<RgbaImage> {
        let cells = rects.iter().map(|&r| self.scaled(name, Some(r))).collect::<Result<Vec<_>>>()?;
        let mut out = RgbaImage::new(cells.iter().map(|c| c.width()).sum(), cells.iter().map(|c| c.height()).max().unwrap_or(1));
        let mut x = 0;
        for c in &cells {
            imageops::replace(&mut out, c, i64::from(x), 0);
            x += c.width();
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_from_the_stages() {
        let t = title(Path::new("/nonexistent"), Path::new("/nonexistent"), true, "wav").unwrap();
        // Continue's row at y 720 of 1080, the logo at (680, 72), the menu at x 816.
        assert_eq!((t.buttons[0].x, t.buttons[0].y), (544, 480));
        assert_eq!((t.pictures[1].x, t.pictures[1].y), (453, 48));
        assert_eq!(t.continue_hides, Some(-15));
        assert_eq!(t.movies.last().unwrap().when, vec![]);
    }
}
