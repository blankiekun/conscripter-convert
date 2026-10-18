//! What the game shows before the title (`FUN_001b5650`, one state a
//! frame): 15 frames of black (always), the TYPE-MOON logo movie (`TM_CI`, movie
//! table 0x50B890 entry 0x26) on white, then the caution screen
//! (`parts.mrg` entry 1, `caution2`) faded in and out. The steps' table at
//! 0x4C98FC (0x18 a step): picture, hold frames, frames before a press
//! skips, …, the colour behind (1 white, 0 black).
//!
//! The movie and the caution only once the prologue has been read (scene-read flag 485, `_RDST(485)`
//! in `00_01`): with no such save (the first launch) the exe goes straight
//! on (state 0x10). The fan patch has its own movie and caution picture.
//! Left out: state 0xD (the logo picture `logo_fix` held 8 s), which needs
//! `DAT_01a4df08`, a BSS word nothing sets.

use std::path::Path;

use anyhow::{Context, Result};
use conv_core::ir::{BootStep, Var};

use super::convert::read_flag;
use super::{bntx, mrg};
use crate::build::map;

const DIR: &str = "sys/boot";
/// 60 frames a second.
const fn frames(n: u32) -> u32 {
    n * 1000 / 60
}

/// The steps, and (unless `dry_run`) the caution pictures in
/// `<output>/sys/boot`. `romfs` is the Japanese RomFS (or the fan patch's
/// alone); `english`, the fan patch's beside it, whose copies show while
/// `lang` is 1.
pub fn steps(romfs: &Path, english: Option<(&Path, Var)>, output: &Path, dry_run: bool) -> Result<Vec<BootStep>> {
    // src: FUN_001b5650 state 1: 15 frames (black: the colour starts at 0), before the flag is looked at
    let mut steps = vec![BootStep { backdrop: "black".into(), hold_ms: frames(15), ..Default::default() }];
    let read = (read_flag(485), 1);
    let mut add = |dir: &Path, sub: &str, lang: Option<(Var, i32)>| -> Result<()> {
        let image = format!("{DIR}/{sub}caution.png");
        if !dry_run {
            let dest = output.join(&image);
            std::fs::create_dir_all(dest.parent().unwrap())?;
            caution(dir)?.save(&dest).with_context(|| format!("writing {}", dest.display()))?;
        }
        let when: Vec<_> = std::iter::once(read).chain(lang).collect();
        // src: states 3, 9–0xB: TM_CI; a press ends it once 120 frames (table +8) have played; white behind (+0x14 = 1)
        steps.push(BootStep { image: format!("movie/{sub}TM_CI.mp4"), movie: true, backdrop: "white".into(), skip_after_ms: Some(frames(120)), when: when.clone(), ..Default::default() });
        // src: states 3–7: +0.025 opacity a frame, over 1.0 at the 41st (from the white still behind), held
        // 480 frames (table +4) or till a press (+8 = 0), −0.025 a frame to below 0, on black (+0x14 = 0)
        steps.push(BootStep {
            image,
            backdrop: "white".into(),
            fade_in_ms: frames(41),
            hold_ms: frames(480),
            fade_out_ms: frames(41),
            fade_out_to: Some("black".into()),
            skip_after_ms: Some(0),
            when,
            ..Default::default()
        });
        Ok(())
    };
    match english {
        Some((en, lang)) => {
            add(romfs, "", Some((lang, 0)))?;
            add(en, "en/", Some((lang, 1)))?;
        }
        None => add(romfs, "", None)?,
    }
    Ok(steps)
}

/// `parts.mrg`'s entry 1 (`FUN_001b4a20(1, 0x47, 0x1B)`): the caution
/// screen, 1920 × 1080, scaled to the screen.
fn caution(romfs: &Path) -> Result<image::RgbaImage> {
    let path = romfs.join("parts.mrg");
    let bytes = map(&path)?;
    let entries = mrg::parse(&map(&path.with_extension("hed"))?, None).context("reading parts.hed")?;
    let entry = entries.get(1).context("parts.mrg has no entry 1")?;
    let data = super::unpack(mrg::data(&bytes, entry)?)?;
    let img = bntx::decode(bntx::parse(&data)?.first().context("parts.mrg entry 1 has no texture")?)?;
    Ok(image::imageops::resize(&img, super::convert::SCREEN_W, super::convert::SCREEN_H, image::imageops::FilterType::Triangle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_then_caution_once_the_prologue_is_read() {
        let lang = Var { global: true, index: 1000 };
        let s = steps(Path::new("/nonexistent"), Some((Path::new("/nonexistent"), lang)), Path::new("/nonexistent"), true).unwrap();
        assert_eq!(s.len(), 5);
        assert_eq!((s[0].hold_ms, s[0].when.len()), (250, 0));
        assert_eq!(s[1].image, "movie/TM_CI.mp4");
        assert_eq!(s[3].image, "movie/en/TM_CI.mp4");
        assert_eq!(s[4].image, "sys/boot/en/caution.png");
        assert_eq!(s[4].when, vec![(read_flag(485), 1), (lang, 1)]);
        assert_eq!((s[2].fade_in_ms, s[2].hold_ms, s[1].skip_after_ms), (683, 8000, Some(2000)));
    }
}
