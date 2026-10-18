//! 月姫's Gallery (title > Gallery, `cGallery`): the menu (`cGalleryMenu`,
//! `FUN_00106124`) and its CG gallery (`cGalleryCG`), from the exe's
//! tables and `GALLERY_PARTS`/`CG_THUMB_PARTS` in `allui`.
//!
//! The tables (`FUN_001042d0`, `FUN_00104470`, `FUN_0010b830`): 458 slots
//! of 1–65 CGs; a CG shows when its slot's condition holds and a picture
//! that unlocks it has its CG flag, or global flag 557 is set. The flags:
//! showing a picture sets its own (`FUN_001774f0`), its index in the
//! picture table, an expression's that of its name without the last six
//! letters, a sprite past 0xA8C the one the table at 0x7533C0 gives it. A
//! CG is unlocked by its own name (`FUN_00175900`: the first 0xA8C names
//! only) or by the pictures the table at 0x5525C8 lists for it, a sprite's
//! shot letter tried as A, L, M and S (`FUN_001051e0`). Here a CG counts
//! as seen once the engine has shown one of the pictures that set that
//! flag (`Seen::Pictures`), once `_STCG` marked one of them (a global each,
//! `convert::cg_flags`), or by flag 557 (the last lesson's `_RDED(557)`;
//! the only test of the 926 CGs nothing unlocks).
//!
//! The grid lists only the slots with a CG to show (`FUN_0010b830`;
//! `PictureGallery::compact`).
//!
//! Coordinates: the exe's widgets are placed in 1080p pixels (`FUN_0011fa60`
//! with 1920 × 905 from y 109); here × 2/3. A slot is listed only while
//! its condition holds (`FUN_0010b58c`: scene-read flags). The viewer
//! shows a CG fitted into the screen (`FUN_0010be84`). Left out: the list
//! in pages of 6 rows (the exe scrolls it, 6 across); the viewer's zoom
//! and pan (`FUN_0010c56c`: the right stick or a pinch zooms from fitted to
//! 1:1, a button toggles fitted and filling the screen, the left stick or a
//! drag pans; here a click goes to the next CG); the Music Player, Flowchart, Movie and Ending List (drawn, not
//! clickable).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use conv_core::ir::{ExtrasMode, GalleryCategory, PictureExtras, PictureExtrasItem, PictureGallery, PictureSlot, Placed, Seen, SlotCg, Var};
use image::{Rgba, RgbaImage, imageops};

use super::exe::{Exe, Picture};
use super::title::{Parts, px};
use crate::labels::Labels;

const DIR: &str = "sys/gallery";
/// Global flag 557 (`_RDED(557)`, the last lesson read): every CG shows.
// ours: a global after language.rs's (1000-1002)
pub const FLAG_557: Var = Var { global: true, index: 1003 };
/// src: FUN_0010b790 (458 slots), the table at 0x54EC88, 0x20 bytes a slot.
const SLOTS: usize = 0x54_EC88;
const SLOT_COUNT: usize = 458;
/// src: FUN_0010b7c8/FUN_0010b798: 105 CGs, 0x28 bytes each (the CG, then
/// up to four pictures).
const UNLOCKS: usize = 0x55_25C8;
const UNLOCK_COUNT: usize = 105;
/// src: FUN_00177360: (name, its index (filled at run time), its flag),
/// 0x10 bytes each, until a null name.
const SPRITE_FLAGS: usize = 0x75_33C0;
/// src: FUN_00156720's limit in FUN_00175900, and FUN_001774f0's.
const NAMED: usize = 0xA8C;
/// src: FUN_001774f0 (the bits at 0x7A8).
const FLAGS: u32 = 0xC80;
/// The thumbnails' cells in `cg_thumb_all` (src: FUN_0010af78: 36 a row,
/// 154 × 87 every 154 × 88, by the CG's number), drawn at 1.5 (here 1).
const THUMB: (u32, u32) = (154, 87);
/// src: FUN_0010af78: 6 a row from (392, 109), 240 × 139 apart.
const COLUMNS: u32 = 6;
const SCREEN: (u32, u32) = (1280, 720);

/// A CG of a slot (the slot table's entries).
#[derive(Debug, Clone, PartialEq)]
pub struct Cg {
    /// `IMG3864` (no prefix, no extension).
    pub name: String,
    /// Its cell in `cg_thumb_all`.
    pub number: u32,
    /// Its size in the viewer at 1080p.
    pub size: (u32, u32),
}

/// Each slot's condition (the slot table's +8) as the scene-read flags
/// (`_RDST`) any of which lets it be listed; none for 0 (always) and past
/// 19 (src: FUN_0010b58c, its flags read by FUN_001926b0).
fn condition(c: i32) -> &'static [u16] {
    const FLAGS: [&[u16]; 20] = [&[], &[2], &[3], &[4], &[5], &[0x17F], &[0xD0], &[7], &[8], &[9], &[0xE2], &[10], &[11], &[12], &[13], &[0x113, 0x114], &[14], &[0x129], &[300], &[0x135]];
    usize::try_from(c).ok().and_then(|c| FLAGS.get(c)).copied().unwrap_or(&[])
}

/// The slots' conditions (`_RDST` flags, any of them; none: always).
fn conditions(exe: &Exe) -> Result<Vec<&'static [u16]>> {
    (0..SLOT_COUNT).map(|i| Ok(condition(exe.i32(SLOTS + 0x20 * i + 8)?))).collect()
}

/// The slots' CGs, in order.
pub fn slots(exe: &Exe) -> Result<Vec<Vec<Cg>>> {
    let mut out = Vec::new();
    for i in 0..SLOT_COUNT {
        let list = exe.pointer(SLOTS + 0x20 * i);
        let mut cgs = Vec::new();
        for k in 0.. {
            let at = list + 0x20 * k;
            let name = exe.pointer(at);
            if name == 0 {
                break;
            }
            let name = exe.string(name)?;
            let name = name.trim_start_matches([' ', '-']).trim_end_matches(".MZP").to_string();
            cgs.push(Cg { name, number: exe.i32(at + 8)? as u32, size: (exe.i32(at + 0x14)? as u32, exe.i32(at + 0x18)? as u32) });
        }
        out.push(cgs);
    }
    Ok(out)
}

/// The CGs other pictures unlock: name → those pictures.
fn unlocks(exe: &Exe) -> Result<HashMap<String, Vec<String>>> {
    let name = |a: usize| -> Result<Option<String>> {
        let p = exe.pointer(a);
        Ok(if p == 0 { None } else { Some(exe.string(p)?.trim_end_matches(".MZP").to_string()) })
    };
    let mut out = HashMap::new();
    for i in 0..UNLOCK_COUNT {
        let at = UNLOCKS + 0x28 * i;
        let cg = name(at)?.context("an unlock without its CG")?;
        let by = (1..5).map(|k| name(at + 8 * k)).collect::<Result<Vec<_>>>()?.into_iter().flatten().collect();
        out.insert(cg, by);
    }
    Ok(out)
}

/// The sprites' flags by name.
fn sprite_flags(exe: &Exe) -> Result<HashMap<String, u32>> {
    let mut out = HashMap::new();
    for at in (SPRITE_FLAGS..).step_by(0x10) {
        let p = exe.pointer(at);
        if p == 0 {
            break;
        }
        out.insert(exe.string(p)?, exe.i32(at + 0xC)? as u32);
    }
    Ok(out)
}

/// Which pictures set which CG flag.
struct Flags<'a> {
    pictures: &'a [Picture],
    index: HashMap<&'a str, usize>,
    sprites: HashMap<String, u32>,
}

impl<'a> Flags<'a> {
    fn new(pictures: &'a [Picture], sprites: HashMap<String, u32>) -> Self {
        let index = pictures.iter().enumerate().map(|(i, p)| (p.name.as_str(), i)).collect();
        Flags { pictures, index, sprites }
    }

    /// The flag showing picture `i` sets (FUN_001774f0).
    fn shown(&self, i: usize) -> Option<u32> {
        let p = &self.pictures[i];
        let i = if p.expression { *self.index.get(p.name.get(..p.name.len().saturating_sub(6))?)? } else { i };
        let flag = if i > NAMED { *self.sprites.get(&self.pictures[i].name)? } else { i as u32 };
        (flag < FLAGS).then_some(flag)
    }

    /// The flag a CG or unlocking picture is tested by (FUN_00175900, then
    /// for an unlocking sprite FUN_00162110 with its shot letters).
    fn tested(&self, name: &str, unlocking: bool) -> Vec<u32> {
        if let Some(&i) = self.index.get(name).filter(|&&i| i < NAMED) {
            return self.shown(i).into_iter().collect();
        }
        if !unlocking || name.len() < 6 {
            return Vec::new();
        }
        "ALMS".chars().filter_map(|c| {
            let mut n = name.to_string();
            n.replace_range(4..5, &c.to_string());
            self.index.get(n.as_str()).and_then(|&i| self.shown(i))
        }).collect()
    }
}

/// A CG's unlocking pictures and `_STCG` globals.
type SeenBy = (Vec<String>, Vec<Var>);

/// The gallery's seen tests, slot by slot: per CG the pictures that set a
/// flag unlocking it (lowercase names) and the `_STCG` globals that do
/// (`cg_flags`: by lowercase name).
fn seen_by(exe: &Exe, pictures: &[Picture], slots: &[Vec<Cg>], cg_flags: &BTreeMap<String, Var>) -> Result<Vec<Vec<SeenBy>>> {
    let flags = Flags::new(pictures, sprite_flags(exe)?);
    // script_command's _STCG: the picture's index, none past 0xA8C
    let marked: Vec<(u32, Var)> = cg_flags.iter().filter_map(|(n, &v)| flags.index.get(n.to_uppercase().as_str()).filter(|&&i| i <= NAMED).and_then(|&i| flags.shown(i)).map(|f| (f, v))).collect();
    let unlocks = unlocks(exe)?;
    let mut setters: HashMap<u32, Vec<String>> = HashMap::new();
    for (i, p) in pictures.iter().enumerate() {
        if let Some(f) = flags.shown(i) {
            setters.entry(f).or_default().push(p.name.to_lowercase());
        }
    }
    Ok(slots
        .iter()
        .map(|cgs| {
            cgs.iter()
                .map(|cg| {
                    // FUN_001051e0: the first picture found by name decides,
                    // else any sprite's shot; then the CG's own name
                    let mut tested = Vec::new();
                    for p in unlocks.get(&cg.name).into_iter().flatten() {
                        if flags.index.get(p.as_str()).is_some_and(|&i| i < NAMED) {
                            tested.extend(flags.tested(p, false));
                            break;
                        }
                        tested.extend(flags.tested(p, true));
                    }
                    tested.extend(flags.tested(&cg.name, false));
                    let mut names: Vec<String> = tested.iter().flat_map(|f| setters.get(f).into_iter().flatten().cloned()).collect();
                    names.sort();
                    names.dedup();
                    (names, marked.iter().filter(|(f, _)| tested.contains(f)).map(|&(_, v)| v).collect())
                })
                .collect()
        })
        .collect())
}

/// Where the build reads and writes the gallery.
pub struct Sources<'a> {
    pub romfs: &'a Path,
    pub output: &'a Path,
    pub font: &'a [u8],
    /// The CGs in `cg/` (`allpaccg`'s), lowercase; the rest are in `pic/`.
    pub cgs: &'a HashSet<String>,
    /// The fan patch's own pictures (`pic/en/`), lowercase.
    pub own: &'a HashSet<String>,
    pub pic_ext: &'a str,
    pub dry_run: bool,
}

/// The Gallery, and (unless a dry run) its pictures in `<output>/sys/gallery`.
/// `flag557`: global flag 557 (`_RDED(557)`).
/// `cg_flags`: the globals `_STCG` sets (`convert::cg_flags`).
pub fn extras(exe: &Exe, pictures: &[Picture], s: &Sources, flag557: Var, cg_flags: &BTreeMap<String, Var>) -> Result<PictureExtras> {
    let slots = slots(exe)?;
    let seen = seen_by(exe, pictures, &slots, cg_flags)?;
    let parts = if s.dry_run { None } else { Some(Parts::read(s.romfs, "GALLERY_PARTS")?) };
    let save = |name: &str, img: Option<RgbaImage>| -> Result<String> {
        let file = format!("{DIR}/{name}.png");
        if let Some(img) = img {
            let dest = s.output.join(&file);
            std::fs::create_dir_all(dest.parent().unwrap())?;
            img.save(&dest).with_context(|| format!("writing {}", dest.display()))?;
        }
        Ok(file)
    };
    let placed = |name: &str, img: Option<RgbaImage>, (x, y): (i32, i32), (w, h): (i32, i32)| -> Result<Placed> { Ok(Placed { image: save(name, img)?, x, y, w, h }) };
    let at = |x: f32, y: f32| (px(x), px(y));

    // src: FUN_00106124's widgets (0x54E2F8…): gallerymenu_window (7181) at
    // (56, 0), config_innerbg (7184) at (365, 0), underline (7183) at (0, 72)
    // guess: over black (what is under the gallery not traced)
    let background = parts.as_ref().map(|p| -> Result<RgbaImage> {
        let mut bg = RgbaImage::from_pixel(SCREEN.0, SCREEN.1, Rgba([0, 0, 0, 255]));
        for (name, x, y) in [("config_innerbg", 365.0, 0.0), ("gallerymenu_window", 56.0, 0.0), ("underline", 0.0, 72.0)] {
            let (x, y) = at(x, y);
            imageops::overlay(&mut bg, &p.scaled(name, None)?, x.into(), y.into());
        }
        Ok(bg)
    });
    let background = background.transpose()?;
    // The gallery's: the menu's other rows at rest under its CG Gallery
    let gallery_background = match (&background, &parts) {
        (Some(bg), Some(p)) => {
            let mut bg = bg.clone();
            for r in 1..6 {
                let (x, y) = at(80.0, 160.0 + 80.0 * r as f32);
                imageops::overlay(&mut bg, &p.cells("gallerymenu_btn", &[(0, r * 632 / 6, 266, 105)])?, x.into(), y.into());
            }
            Some(bg)
        }
        _ => None,
    };
    let background = save("bg", background)?;
    let gallery_background = save("gallery_bg", gallery_background)?;

    // src: the menu's list (0x54E438): rows at x 80, y 160 + 80 a row;
    // gallerymenu_btn's rows (CG Gallery, Music Player, Flowchart, Movie,
    // Ending List, Title), its columns at rest, chosen and lit
    // guess: a cell's top left at its row's place
    let row = |r: u32, name: &str| -> Result<Placed> {
        let img = parts.as_ref().map(|p| p.cells("gallerymenu_btn", &[0, 1, 2].map(|c| (c * 800 / 3, r * 632 / 6, 266, 105)))).transpose()?;
        placed(name, img, at(80.0, 160.0 + 80.0 * r as f32), (px(266.0) * 3, px(105.0)))
    };
    let modes = [ExtrasMode::Gallery, ExtrasMode::Sound, ExtrasMode::Recollection, ExtrasMode::Turntable, ExtrasMode::Recollection, ExtrasMode::Back];
    let items = modes
        .iter()
        .enumerate()
        .map(|(r, &mode)| Ok(PictureExtrasItem { button: row(r as u32, &format!("menu{r}"))?, mode, unlocked_by: None, locked_alpha: 255, click_se: None }))
        .collect::<Result<Vec<_>>>()?;

    // The slots: the thumbnail in a cell the size of the cursor's, at the
    // cursor's (8, 6) inside it (src: cg_cursor's cells (8, 8) and (264, 8),
    // 248 × 144, at (384, 103) to the first thumbnail's (392, 109))
    let box_size = (px(248.0), px(144.0));
    let inset = (px(8.0), px(6.0));
    let thumbs = (!s.dry_run).then(|| Parts::read(s.romfs, "CG_THUMB_PARTS")).transpose()?;
    let boxed = |img: &RgbaImage| {
        let mut b = RgbaImage::new(box_size.0 as u32, box_size.1 as u32);
        imageops::overlay(&mut b, img, inset.0.into(), inset.1.into());
        b
    };
    let blank = save("blank", (!s.dry_run).then(|| boxed(&RgbaImage::from_pixel(THUMB.0, THUMB.1, Rgba([0, 0, 0, 160])))))?;
    let cursor = parts.as_ref().map(|p| -> Result<RgbaImage> {
        let frame = p.scaled("cg_cursor", Some((8, 8, 248, 144)))?;
        let mut strip = RgbaImage::new(frame.width() * 2, frame.height());
        imageops::overlay(&mut strip, &frame, frame.width().into(), 0);
        Ok(strip)
    });
    let cursor = save("cursor", cursor.transpose()?)?;
    let unseen = save("unseen", (!s.dry_run).then(|| RgbaImage::from_pixel(SCREEN.0, SCREEN.1, Rgba([0, 0, 0, 255]))))?;

    let mut views = Vec::new();
    let mut gallery_slots = Vec::new();
    for (cgs, seen) in slots.iter().zip(&seen) {
        let mut out = Vec::new();
        for (cg, (by, marked)) in cgs.iter().zip(seen) {
            let lower = cg.name.to_lowercase();
            let thumb = thumbs.as_ref().map(|t| -> Result<RgbaImage> {
                let (x, y) = ((cg.number % 36) * 154, (cg.number / 36) * 88);
                Ok(boxed(&imageops::crop_imm(t.get("cg_thumb_all")?, x, y, THUMB.0, THUMB.1).to_image()))
            });
            let thumbnail = save(&format!("th/{}", cg.number), thumb.transpose()?)?;
            // src: FUN_0010be84: the CG opens fitted into the screen, centred
            // (its zoom the smaller of the screen's width and height over
            // its size, larger or smaller)
            let k = (SCREEN.0 as f32 / cg.size.0 as f32).min(SCREEN.1 as f32 / cg.size.1 as f32);
            let (w, h) = ((cg.size.0 as f32 * k).round() as i32, (cg.size.1 as f32 * k).round() as i32);
            let source = if s.cgs.contains(&lower) { format!("cg/{lower}.{}", s.pic_ext) } else { format!("pic/{lower}.{}", s.pic_ext) };
            let image = format!("{DIR}/fit/{lower}.png");
            views.push((source, image.clone(), (w as u32, h as u32)));
            // FUN_00104470: its pictures, its own name, then flag 557
            let mut any = Vec::new();
            if !by.is_empty() {
                any.push(Seen::Pictures(by.iter().flat_map(|n| std::iter::once(format!("pic/{n}.{}", s.pic_ext)).chain(s.own.contains(n).then(|| format!("pic/en/{n}.{}", s.pic_ext)))).collect()));
            }
            any.extend(marked.iter().map(|&v| Seen::Var(v)));
            any.push(Seen::Var(flag557));
            let seen = if any.len() == 1 { any.pop().unwrap() } else { Seen::Any(any) };
            let (x, y) = ((SCREEN.0 as i32 - w) / 2, (SCREEN.1 as i32 - h) / 2);
            out.push(SlotCg { seen, picture: Placed { image, x, y, w, h }, thumbnail });
        }
        gallery_slots.push(PictureSlot { cgs: out, reversed: false, shown_if: Vec::new() });
    }
    for (slot, flags) in gallery_slots.iter_mut().zip(conditions(exe)?) {
        slot.shown_if = flags.iter().map(|&n| super::convert::read_flag(n)).collect();
    }
    if !s.dry_run {
        write_views(s.output, &views)?;
    }

    // ours: page buttons and Back, drawn in the font (the exe scrolls the
    // list and goes back with the B button)
    // (drawn in a dry run too: their size is in the script)
    let labels = Labels::new(s.font).context("the font doesn't load")?;
    let label = |name: &str, text: &str, (x, y): (i32, i32)| -> Result<Placed> {
        let img = Some(&labels).map(|l| {
            let cells: Vec<RgbaImage> = [[200, 200, 210], [120, 220, 255], [255, 255, 255]].iter().map(|&c| l.draw(text, 26.0, c, [0, 40, 80], 255)).collect();
            let (cw, ch) = (cells[0].width(), cells[0].height());
            let mut strip = RgbaImage::new(cw * 3, ch);
            for (i, c) in cells.iter().enumerate() {
                imageops::overlay(&mut strip, c, (cw * i as u32).into(), 0);
            }
            strip
        });
        let (w, h) = img.as_ref().map_or((0, 0), |i| (i.width() as i32, i.height() as i32));
        placed(name, img.filter(|_| !s.dry_run), (x, y), (w, h))
    };
    let (gx, gy) = at(392.0, 109.0);
    let (gx, gy) = (gx - inset.0, gy - inset.1);
    let row_step = px(139.0);
    let bottom = gy + row_step * COLUMNS as i32 + px(8.0);
    let gallery = PictureGallery {
        background: gallery_background,
        categories: vec![GalleryCategory { button: items[0].button.clone(), slots: gallery_slots }],
        previous: label("previous", "<", (px(1060.0), bottom))?,
        next: label("next", ">", (px(1180.0), bottom))?,
        exit: label("back", "Back", (px(1640.0), bottom))?,
        first: (gx, gy),
        row_step,
        column_step: px(240.0),
        rows: COLUMNS,
        columns: COLUMNS,
        blank,
        cursor,
        partial_alpha: 255,
        unseen,
        // guess: the fades (the widgets' alpha steps not traced)
        open_ms: 300,
        page_ms: 300,
        view_fade_ms: 300,
        close_ms: 300,
        scroll_ms_per_px: 6,
        across: true,
        arrows_when_paged: true,
        engine_screen: false,
        category_hover: None,
        compact: true,
    };
    Ok(PictureExtras { background, items, hover_se: None, back_se: None, fade_ms: 300, gallery: Some(gallery), sound: None, replay: None, turntable: None, passage: None })
}

/// Each CG at the viewer's size: (source, destination, size), from the
/// media folder's pictures.
fn write_views(output: &Path, views: &[(String, String, (u32, u32))]) -> Result<()> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let failed = Mutex::new(Vec::new());
    let one = |(source, dest, (w, h)): &(String, String, (u32, u32))| -> Result<()> {
        let dest = output.join(dest);
        if dest.exists() {
            return Ok(());
        }
        let img = image::open(output.join(source)).with_context(|| format!("reading {source} (the media folder: typemoon media)"))?.to_rgba8();
        std::fs::create_dir_all(dest.parent().unwrap())?;
        imageops::resize(&img, *w, *h, imageops::FilterType::Triangle).save(&dest)?;
        Ok(())
    };
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                while let Some(v) = views.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Err(e) = one(v) {
                        failed.lock().unwrap().push(format!("{e:#}"));
                    }
                }
            });
        }
    });
    let failed = failed.into_inner().unwrap();
    anyhow::ensure!(failed.is_empty(), "{} gallery pictures failed, the first: {}", failed.len(), failed[0]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(name: &str, expression: bool) -> Picture {
        Picture { name: name.into(), width: 1.0, height: 1.0, centre: (0.0, 0.0), flags: 0, expression, shot: 0, pose_value: 0 }
    }

    #[test]
    fn flags_of_pictures_expressions_and_sprites() {
        // 0xA8D pictures: IMG0001 at 1, its expression at 2, two sprites past 0xA8C
        let mut pictures: Vec<Picture> = (0..=NAMED + 2).map(|i| picture(&format!("P{i}"), false)).collect();
        pictures[1] = picture("IMG0001", false);
        pictures[2] = picture("IMG0001_01_00", true);
        pictures[NAMED + 1] = picture("SIE_L_63_00_N00", false);
        pictures[NAMED + 2] = picture("SIE_S_63_00_N00", false);
        let flags = Flags::new(&pictures, [("SIE_L_63_00_N00".to_string(), 2800)].into());
        assert_eq!((flags.shown(1), flags.shown(2)), (Some(1), Some(1)));
        assert_eq!((flags.shown(NAMED + 1), flags.shown(NAMED + 2)), (Some(2800), None));
        // a sprite unlocks by any shot letter; a CG by its own name only
        assert_eq!(flags.tested("SIE_A_63_00_N00", true), [2800]);
        assert_eq!(flags.tested("SIE_A_63_00_N00", false), Vec::<u32>::new());
        assert_eq!(flags.tested("IMG0001", false), [1]);
    }
}
