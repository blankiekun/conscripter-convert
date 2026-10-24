//! `_SELS(_D000,)`, the yes/no question `QACALL` asks before a lesson
//! (`FUN_00192800`: choice mode 2, two options, index 0 and 1, picked
//! into `F000` for `_GOTS(_D000, no, yes)`). `FUN_00185900` draws it on
//! the 640 × 448 stage, from `DIALOG_PARTS` in `allui`: `dialog_BG`
//! (0x4C2, a dim over the whole screen) and `dialog_win` (0x4C0) centred,
//! the question (system message 0x5A5 + `F027`, from `SYSMES_TEXT.DAT`:
//! 「ヘンテコヒントコーナー(第n回)に入りますか？」) centred at y
//! 224 − 42.31, and the buttons 0xD9 (index 0) at (241, 238.52) and 0xDA
//! (index 1) at (327, 238.52); left and right pick them
//! (`FUN_00185690`). The question is drawn into a picture per lesson
//! with the build's font. Left out: the window's pop from 105 % as it
//! fades in.

use std::path::Path;

use anyhow::{Context, Result};
use conv_core::ir::{ButtonScreen, Compare, Op, Operand, Placed, Var};
use image::{RgbaImage, imageops};

use super::title::{Parts, px};
use super::{mrg, unpack};
use crate::build::map;
use crate::labels::Labels;

const DIR: &str = "sys/question";
/// src: QACALL's lessons (`_WKST(F027,eq,n)`), messages 0x5A6–0x5BE.
const LESSONS: i32 = 25;
/// src: FUN_00183b20 (system message 0x5A5 + F027).
const FIRST_MESSAGE: usize = 0x5a5;
/// The question picture's size: the window's width (824 of 1080p).
const TEXT_W: i32 = 549;
const TEXT_H: i32 = 40;
/// src: exe 0x185900 (FUN_001ab4d0 with 0.37 × 0.46044 of FUN_001a7790's
/// screen width / 32: 1280 / 32 × 0.46 = 18.4 high; ours: the glyphs not
/// narrowed to the exe's 0.37)
const FONT_SIZE: f32 = 18.0;
/// src: exe 0x1848b0 (0.1 a frame, FUN_00140ed0's frames elapsed: 10 frames at 60 Hz)
const FADE_MS: u32 = 167;

/// A point of the 640 × 448 stage on the screen.
fn stage(x: f32, y: f32) -> (i32, i32) {
    (px(x * 3.0), px(y * 1080.0 / 448.0))
}

fn placed(image: String, (x, y): (i32, i32), (w, h): (i32, i32)) -> Placed {
    Placed { image, x, y, w, h }
}

/// `sys/question/<lang>`: `en/` for the English copy of a build with both.
fn dir(english: bool) -> String {
    format!("{DIR}/{}", if english { "en/" } else { "" })
}

/// The question's ops for `F027` (`lesson`): one dialog per lesson,
/// continuing at `no` or `yes`; `tag` keeps their labels apart.
pub(super) fn ops(lesson: Var, no: String, yes: String, english: bool, tag: impl Fn(i32) -> String) -> Vec<Op> {
    let d = dir(english);
    // src: FUN_00185900 (0x4C2 and 0x4C0 at (320, 224), the parts at their 1080p size)
    let (cx, cy) = stage(320.0, 224.0);
    let (ww, wh) = (px(824.0), px(416.0));
    let backdrop = [placed(format!("{DIR}/dim.png"), (0, 0), (px(1920.0), px(1080.0))), placed(format!("{DIR}/window.png"), (cx - px(412.0), cy - px(208.0)), (ww, wh))];
    // src: exe 0x185900 (FUN_001ae000 at (241, 238.518) and (327, 238.518)), 0x183b20 (hit rect 0x75BBB8 from (0, 0): the top left)
    let cell = (px(224.0) * 2, px(66.0));
    let buttons = vec![(placed(format!("{d}no.png"), stage(241.0, 238.5185), cell), no), (placed(format!("{d}yes.png"), stage(327.0, 238.5185), cell), yes)];
    let (_, ty) = stage(320.0, 224.0 - 42.311);
    let mut out = Vec::new();
    for n in 1..=LESSONS {
        out.push(Op::If { var: lesson, cmp: Compare::Ne, value: Operand::Const(n), label: tag(n) });
        let text = placed(format!("{d}q{n}.png"), (cx - TEXT_W / 2, ty - TEXT_H / 2), (TEXT_W, TEXT_H));
        let pictures = backdrop.iter().cloned().chain([text]).collect();
        out.push(Op::ButtonScreen(Box::new(ButtonScreen { pictures, buttons: buttons.clone(), fade_ms: FADE_MS })));
        out.push(Op::Label(tag(n)));
    }
    // another lesson number (none in the game): the dialog without its question
    out.push(Op::ButtonScreen(Box::new(ButtonScreen { pictures: backdrop.to_vec(), buttons, fade_ms: FADE_MS })));
    out
}

/// Writes the dialog's pictures into `<output>/sys/question`, the
/// questions in `romfs`'s language (`en/` with `english`). A dry run
/// only records their names.
pub(super) fn write(romfs: &Path, output: &Path, font: &[u8], english: bool, dry_run: bool) -> Result<()> {
    let d = dir(english);
    let names = [format!("{DIR}/dim.png"), format!("{DIR}/window.png"), format!("{d}yes.png"), format!("{d}no.png")];
    names.iter().cloned().chain((1..=LESSONS).map(|n| format!("{d}q{n}.png"))).for_each(|n| crate::output::record(&n));
    if dry_run {
        return Ok(());
    }
    let parts = Parts::read(romfs, "DIALOG_PARTS")?;
    let save = |name: String, img: &RgbaImage| -> Result<()> {
        let dest = output.join(name);
        std::fs::create_dir_all(dest.parent().unwrap())?;
        Ok(img.save(&dest)?)
    };
    save(format!("{DIR}/dim.png"), &parts.scaled("dialog_BG", None)?)?;
    save(format!("{DIR}/window.png"), &parts.scaled("dialog_win", None)?)?;
    // dialog_btn_ja: 224 × 66 cells, rows Yes, No, then each lit
    save(format!("{d}yes.png"), &parts.cells("dialog_btn_ja", &[(0, 0, 224, 66), (0, 132, 224, 66)])?)?;
    save(format!("{d}no.png"), &parts.cells("dialog_btn_ja", &[(0, 66, 224, 66), (0, 198, 224, 66)])?)?;
    let messages = messages(&entry(romfs, "SYSMES_TEXT.DAT")?)?;
    let labels = Labels::new(font).context("the font doesn't load")?;
    for n in 1..=LESSONS {
        let text = messages.get(FIRST_MESSAGE + n as usize).context("no such system message")?.replace('^', "");
        let line = labels.draw(&text, FONT_SIZE, [255, 255, 255], [0, 0, 0], 255);
        let mut img = RgbaImage::new(TEXT_W as u32, TEXT_H as u32);
        imageops::overlay(&mut img, &line, (i64::from(TEXT_W) - i64::from(line.width())) / 2, (i64::from(TEXT_H) - i64::from(line.height())) / 2);
        save(format!("{d}q{n}.png"), &img)?;
    }
    Ok(())
}

/// An `allui` entry, unpacked.
fn entry(romfs: &Path, name: &str) -> Result<Vec<u8>> {
    let path = romfs.join("allui.mrg");
    let (hed, nam, bytes) = (map(&path.with_extension("hed"))?, map(&path.with_extension("nam"))?, map(&path)?);
    let e = mrg::parse(&hed, Some(&nam))?.into_iter().find(|e| e.name.as_deref() == Some(name)).with_context(|| format!("allui has no {name}"))?;
    Ok(unpack(mrg::data(&bytes, &e)?)?.into_owned())
}

/// `SYSMES_TEXT.DAT` (`FUN_0019a420`): u32 languages, u32 messages, then
/// at 0x10 a u64 offset per language to its u64 offsets of each message
/// (UTF-8, NUL-ended; `^` breaks a line). The first language's.
fn messages(d: &[u8]) -> Result<Vec<String>> {
    let u32_at = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let u64_at = |o: usize| d.get(o..o + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()) as usize);
    let count = u32_at(4).context("SYSMES_TEXT is cut short")?;
    let table = u64_at(0x10).context("SYSMES_TEXT is cut short")?;
    (0..count)
        .map(|i| {
            let o = u64_at(table + 8 * i).context("SYSMES_TEXT is cut short")?;
            let s = d.get(o..).context("a message past the end")?;
            Ok(String::from_utf8_lossy(&s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]).into_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_messages() {
        let mut d = vec![0u8; 0x18];
        d[0] = 1;
        d[4] = 2;
        d[0x10] = 0x18;
        d.extend([0x28, 0, 0, 0, 0, 0, 0, 0, 0x2b, 0, 0, 0, 0, 0, 0, 0]);
        d.extend(b"ab\0cd\0");
        assert_eq!(messages(&d).unwrap(), ["ab", "cd"]);
    }

    #[test]
    fn places_from_the_stage() {
        let v = Var { global: false, index: 5 };
        let ops = ops(v, "N".into(), "Y".into(), true, |n| format!("l{n}"));
        let Op::ButtonScreen(s) = &ops[1] else { panic!() };
        // the window 824 × 416 centred on 1080p, the buttons at (723, 575) and (981, 575)
        assert_eq!((s.pictures[1].x, s.pictures[1].y, s.pictures[1].w), (365, 221, 549));
        assert_eq!((s.buttons[0].0.x, s.buttons[0].0.y, s.buttons[1].0.x), (482, 383, 654));
        assert_eq!((s.buttons[0].1.as_str(), s.pictures[2].image.as_str()), ("N", "sys/question/en/q1.png"));
        assert_eq!(ops.len(), 3 * LESSONS as usize + 1);
    }
}
