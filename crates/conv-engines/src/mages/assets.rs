//! The game's files for the compiler ([`super::convert::Assets`]), taken
//! from the CRI archives and written into the output directory.
//!
//! Pictures are copied as they are (`cg/<name>.png`, `chr/<name>.png`,
//! `etc/<name>.png`); system pictures drawn in the 1280×720 space are
//! scaled by 3/4 into `system/`. Sounds can't be copied: ADX and HCA
//! become Ogg Vorbis, which the caller does with ffmpeg from
//! [`ArchiveAssets::audio`] (`voice/<id>.ogg`, `se/<id>.ogg`,
//! `bgm/<id>.ogg`, with the ADX loop as `LOOPSTART`/`LOOPEND`), and the
//! USM movies MP4 files (`movie/`, loose: [`ArchiveAssets::movies`]).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage, imageops};

use conv_core::ir::Placed;

use super::convert::{Assets, SCREEN_H, SCREEN_W};
use super::particles::Kind;
use super::menus::Labels;
use super::cpk::Archive;
use crate::output::{Output, image_size};

/// Top-level output folders the assets go to.
pub const OUTPUT_DIRS: [&str; 7] = ["cg", "chr", "etc", "system", "voice", "se", "bgm"];
/// The output folders with pictures (`--webp` converts them).
pub const PICTURE_DIRS: [&str; 4] = ["cg", "chr", "etc", "system"];

/// A picture in the output: path, width, height.
type Picture = (String, u32, u32);

/// An archive and its bytes.
pub struct Bank<'a> {
    pub archive: Archive,
    pub data: &'a [u8],
}

impl<'a> Bank<'a> {
    pub fn new(data: &'a [u8]) -> Result<Bank<'a>, super::cpk::CpkError> {
        Ok(Bank { archive: Archive::parse(data)?, data })
    }

    fn file(&self, id: u32) -> Option<(&str, &'a [u8])> {
        let e = self.archive.by_id(id)?;
        Some((&e.name, self.archive.read(self.data, e).ok()?))
    }

    fn by_name(&self, name: &str) -> Option<&'a [u8]> {
        let e = self.archive.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
        self.archive.read(self.data, e).ok()
    }
}

/// A sound to convert: the source bytes and where the Ogg goes.
pub struct AudioJob<'a> {
    pub source: &'a [u8],
    pub output: PathBuf,
    /// Loop start and end sample frames (ADX header).
    pub looped: Option<(u64, u64)>,
}

pub struct ArchiveAssets<'a> {
    pub cg: Bank<'a>,
    pub chr: Bank<'a>,
    pub etc: Bank<'a>,
    pub system: Bank<'a>,
    pub voice: Bank<'a>,
    pub se: Bank<'a>,
    pub bgm: Bank<'a>,
    /// `SystemSE.AWB`: the menus' two sounds (0 a tick, 1 a confirm).
    pub system_se: Option<Bank<'a>>,
    images: HashMap<(u8, u32), Option<Picture>>,
    sounds: HashMap<(u8, u32), Option<String>>,
    /// Movies to convert: output path (relative) → the USM file.
    pub movies: BTreeMap<String, PathBuf>,
    /// The folder of the USM files (`media/data`).
    pub movie_dir: PathBuf,
    /// Sounds to convert, by output path.
    pub audio: BTreeMap<String, AudioJob<'a>>,
    /// Where the files go; a dry run reads only picture sizes.
    pub out: Output,
    /// The `--font`, for the words drawn into pictures.
    pub font: Option<&'a [u8]>,
    /// Pictures cut from SystemData's atlases, by output path.
    cut: HashSet<String>,
}

/// The loop of an ADX (version 4) file: start and end sample frames.
pub fn adx_loop(data: &[u8]) -> Option<(u64, u64)> {
    let be = |at: usize| Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as u64);
    if data.get(..2)? != [0x80, 0] || *data.get(0x12)? != 4 || be(0x24)? == 0 {
        return None;
    }
    Some((be(0x28)?, be(0x30)? + 1))
}

/// An ADX file's length in ms (sample rate at 0x08, sample frames at 0x0c).
fn adx_ms(data: &[u8]) -> Option<u32> {
    let be = |at: usize| Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as u64);
    if data.get(..2)? != [0x80, 0] || be(0x08)? == 0 {
        return None;
    }
    Some((be(0x0c)? * 1000 / be(0x08)?) as u32)
}

impl<'a> ArchiveAssets<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(cg: Bank<'a>, chr: Bank<'a>, etc: Bank<'a>, system: Bank<'a>, voice: Bank<'a>, se: Bank<'a>, bgm: Bank<'a>, output: &Path) -> Self {
        ArchiveAssets {
            cg,
            chr,
            etc,
            system,
            voice,
            se,
            bgm,
            system_se: None,
            images: HashMap::new(),
            sounds: HashMap::new(),
            audio: BTreeMap::new(),
            movies: BTreeMap::new(),
            movie_dir: PathBuf::new(),
            out: Output::new(output),
            font: None,
            cut: HashSet::new(),
        }
    }

    /// Copies picture `id` of bank `which` (0 CG, 1 CHR) to `dir/`.
    fn picture(&mut self, which: u8, id: u32) -> Option<(String, u32, u32)> {
        if let Some(r) = self.images.get(&(which, id)) {
            return r.clone();
        }
        let (dir, bank) = match which {
            0 => ("cg", &self.cg),
            _ => ("chr", &self.chr),
        };
        let found = bank.file(id).and_then(|(name, data)| {
            let (w, h) = image_size(data).filter(|_| data.starts_with(b"\x89PNG"))?;
            let stem = name.rsplit_once('.').map_or(name, |(s, _)| s).to_ascii_lowercase();
            Some((format!("{dir}/{stem}.png"), data, w, h))
        });
        let r = match found {
            Some((path, data, w, h)) => {
                self.out.write(&path, data);
                Some((path, w, h))
            }
            None => {
                self.out.errors.push(format!("{dir} picture {id} missing or not a PNG"));
                None
            }
        };
        self.images.insert((which, id), r.clone());
        r
    }

    /// Queues sound `id` of bank `which` (0 voice, 1 SE, 2 BGM, 3 system
    /// SE).
    fn sound(&mut self, which: u8, id: u32) -> Option<String> {
        if let Some(r) = self.sounds.get(&(which, id)) {
            return r.clone();
        }
        let (dir, bank) = match which {
            0 => ("voice", Some(&self.voice)),
            1 => ("se", Some(&self.se)),
            2 => ("bgm", Some(&self.bgm)),
            _ => ("se", self.system_se.as_ref()),
        };
        let prefix = if which == 3 { "sys" } else { "" };
        let r = bank.and_then(|b| b.file(id)).map(|(_, data)| {
            let path = format!("{dir}/{prefix}{id:05}.ogg");
            let looped = if which == 2 { adx_loop(data) } else { None };
            self.audio.insert(path.clone(), AudioJob { source: data, output: self.out.root.join(&path), looped });
            path
        });
        if r.is_none() {
            self.out.errors.push(format!("{dir} {id} missing"));
        }
        self.sounds.insert((which, id), r.clone());
        r
    }
}

impl ArchiveAssets<'_> {
    /// System sound `id` (0 the cursor's tick, 1 a confirm), converted.
    pub fn system_sound(&mut self, id: u32) -> Option<String> {
        self.sound(3, id)
    }

    /// The PNG of CG.cpk picture `id`, as stored.
    pub fn cg_bytes(&self, id: u32) -> Option<&[u8]> {
        self.cg.file(id).map(|(_, d)| d)
    }

    /// SystemData picture `file`, decoded.
    pub fn system_image(&mut self, file: &str) -> Option<RgbaImage> {
        let data = self.system.by_name(file)?;
        match image::load_from_memory(data) {
            Ok(i) => Some(i.to_rgba8()),
            Err(e) => {
                self.out.errors.push(format!("{file}: {e}"));
                None
            }
        }
    }

    /// Writes a picture made by the converter as `system/<name>.png`.
    pub fn put_system_png(&mut self, name: &str, img: &RgbaImage) -> String {
        let path = format!("system/{name}.png");
        self.out.write_png(&path, img);
        path
    }

    /// Cuts rectangles (x, y, w, h) out of SystemData picture `file` and
    /// lays them side by side, scaled by `scale_percent`, as `system/<out>`.
    pub fn system_cells(&mut self, file: &str, out: &str, cells: &[(u32, u32, u32, u32)], scale_percent: u32) -> Option<(String, u32, u32)> {
        let data = self.system.by_name(file)?;
        let img = match image::load_from_memory(data) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.out.errors.push(format!("{file}: {e}"));
                return None;
            }
        };
        let h = cells.iter().map(|c| c.3).max()?;
        let w: u32 = cells.iter().map(|c| c.2).sum();
        let mut sheet = RgbaImage::new(w, h);
        let mut x = 0;
        for &(cx, cy, cw, ch) in cells {
            let part = imageops::crop_imm(&img, cx, cy, cw, ch).to_image();
            imageops::replace(&mut sheet, &part, x as i64, 0);
            x += cw;
        }
        let (w, h) = (w * scale_percent / 100, h * scale_percent / 100);
        if scale_percent != 100 {
            sheet = imageops::resize(&sheet, w, h, imageops::FilterType::CatmullRom);
        }
        let path = format!("system/{out}");
        self.out.write_png(&path, &sheet);
        Some((path, w, h))
    }
}

impl Assets for ArchiveAssets<'_> {
    fn cg(&mut self, id: u32) -> Option<(String, u32, u32)> {
        self.picture(0, id)
    }

    fn chr_id(&self, name: &str) -> Option<u32> {
        Some(self.chr.archive.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?.id)
    }

    fn cg_named(&mut self, name: &str) -> Option<(String, u32, u32)> {
        let id = self.cg.archive.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?.id;
        self.picture(0, id)
    }

    fn chr(&mut self, id: u32) -> Option<(String, u32, u32)> {
        self.picture(1, id)
    }

    fn rule(&mut self, n: u32) -> Option<String> {
        let name = format!("RULE{n:03}.PNG");
        let path = format!("etc/rule{n:03}.png");
        if !self.images.contains_key(&(2, n)) {
            let data = self.etc.by_name(&name)?;
            // Rules are drawn over the whole screen.
            match image::load_from_memory(data) {
                Ok(img) => {
                    let img = imageops::resize(&img.to_rgba8(), SCREEN_W as u32, SCREEN_H as u32, imageops::FilterType::Triangle);
                    self.out.write_png(&path, &img);
                }
                Err(e) => {
                    self.out.errors.push(format!("{name}: {e}"));
                    return None;
                }
            }
            self.images.insert((2, n), Some((path.clone(), SCREEN_W as u32, SCREEN_H as u32)));
        }
        Some(path)
    }

    fn voice(&mut self, id: u32) -> Option<String> {
        self.sound(0, id)
    }

    fn se(&mut self, id: u32) -> Option<String> {
        self.sound(1, id)
    }

    fn se_ms(&self, id: u32) -> Option<u32> {
        adx_ms(self.se.file(id)?.1)
    }

    fn bgm(&mut self, id: u32) -> Option<String> {
        self.sound(2, id)
    }

    fn movie(&mut self, n: u32) -> Option<String> {
        let name = super::usm::MOVIES.get(n as usize)?;
        let source = self.movie_dir.join(format!("{name}.usm"));
        if !source.exists() {
            self.out.errors.push(format!("{} missing", source.display()));
            return None;
        }
        let path = format!("movie/{}.mp4", name.to_ascii_lowercase());
        self.movies.insert(path.clone(), source);
        Some(path)
    }

    fn noise(&mut self, frames: u32) -> String {
        let path = format!("system/noise{frames}.png");
        if !self.images.contains_key(&(4, frames)) {
            let (w, h) = (SCREEN_W as u32, SCREEN_H as u32);
            let mut img = RgbaImage::new(w * frames, h);
            // A fixed LCG: the same static on every build.
            let mut seed: u32 = 0x2545_f491;
            for p in img.pixels_mut() {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let v = (seed >> 24) as u8;
                *p = Rgba([v, v, v, 255]);
            }
            self.out.write_png(&path, &img);
            self.images.insert((4, frames), None);
        }
        path
    }

    fn wave_map(&mut self, cycles_x100: u32) -> String {
        let path = format!("system/wave{cycles_x100}.png");
        if !self.images.contains_key(&(6, cycles_x100)) {
            let (w, h) = (SCREEN_W as u32, SCREEN_H as u32);
            let img = RgbaImage::from_fn(w, h, |_, y| {
                let v = (u64::from(y) * u64::from(cycles_x100) * 256 / (u64::from(h) * 100) % 256) as u8;
                Rgba([v, v, v, 255])
            });
            self.out.write_png(&path, &img);
            self.images.insert((6, cycles_x100), None);
        }
        path
    }

    // The eboot's eye catch type 1 (0x81048c68): the place (SystemData
    // 74 + picture, 352×517) cut to 312×517 and centred at (173, 275)
    // (top 16.5, drawn from row 17);
    // the card (67 + card: ec1_bg … ec6_bg, the protagonists' chapters)
    // over the screen; the date, cell date % 6, date / 6 (169×18) of the
    // card's calendar below y 545, centred at (754, 484). The place's
    // cut moves 40 pixels right over the 30 frames it fades in: drawn
    // where it ends.
    fn chapter_card(&mut self, card: u32, picture: u32, date: i32) -> Option<String> {
        let key = (7, card << 16 | picture << 8 | (date + 1).clamp(0, 255) as u32);
        let path = format!("system/chapter_{card}_{picture}_{}.png", date.max(-1) + 1);
        if self.images.contains_key(&key) {
            return Some(path);
        }
        let decode = |bank: &Bank, id: u32| image::load_from_memory(bank.file(id)?.1).ok().map(|i| i.to_rgba8());
        if card > 6 {
            return None;
        }
        let card_img = decode(&self.system, 67 + card)?;
        let place = decode(&self.system, 74 + picture)?;
        let mut img = RgbaImage::from_pixel(SCREEN_W as u32, SCREEN_H as u32, Rgba([0, 0, 0, 255]));
        // src: exe 0x81048c68 (this block's rectangles)
        let place = imageops::crop_imm(&place, 40, 0, 312, 517).to_image();
        imageops::overlay(&mut img, &place, 173 - 156, 275 - 258);
        imageops::overlay(&mut img, &imageops::crop_imm(&card_img, 0, 0, SCREEN_W as u32, SCREEN_H as u32).to_image(), 0, 0);
        if (0..34).contains(&date) {
            let (dx, dy) = (date as u32 % 6 * 169, 545 + date as u32 / 6 * 19);
            let cell = imageops::crop_imm(&card_img, dx, dy, 169, 18).to_image();
            imageops::overlay(&mut img, &cell, 754 - 84, 484 - 9);
        }
        self.out.write_png(&path, &img);
        self.images.insert(key, None);
        Some(path)
    }

    // The eboot's eye catch type 0 (0x81048978, 0x81048b8a): the halves
    // (960×272) centred at y −136 + 272s and 680 − 272s, s going 0 → 1
    // while the eye catch comes and 1 → 0 when it ends.
    fn shutters(&mut self, n: u32, closing: bool, frames: u32) -> Option<String> {
        let path = format!("system/shutters_{n}_{}.png", if closing { "in" } else { "out" });
        let key = (8, n << 1 | u32::from(closing));
        if self.images.contains_key(&key) {
            return Some(path);
        }
        // The eboot (0x81048978) loads CG ID n + 457, at least 458: EC01
        // is 1, but the archive has no EC20 or EC30, so 27 is EC28.
        let id = (n + 457).max(458);
        let pic = image::load_from_memory(self.cg_bytes(id)?).ok()?.to_rgba8();
        let (w, h) = (SCREEN_W as u32, SCREEN_H as u32);
        let mut screen = RgbaImage::new(w, h);
        imageops::overlay(&mut screen, &pic, (i64::from(w) - i64::from(pic.width())) / 2, (i64::from(h) - i64::from(pic.height())) / 2);
        let top = imageops::crop_imm(&screen, 0, 0, w, h / 2).to_image();
        let bottom = imageops::crop_imm(&screen, 0, h / 2, w, h / 2).to_image();
        let mut strip = RgbaImage::new(w * frames, h);
        for i in 0..frames {
            let step = i64::from(if closing { i + 1 } else { frames - 1 - i });
            let gap = i64::from(h / 2) * (i64::from(frames) - step) / i64::from(frames);
            let x = i64::from(w * i);
            let mut frame = RgbaImage::new(w, h);
            imageops::overlay(&mut frame, &top, 0, -gap);
            imageops::overlay(&mut frame, &bottom, 0, i64::from(h / 2) + gap);
            imageops::replace(&mut strip, &frame, x, 0);
        }
        self.out.write_png(&path, &strip);
        self.images.insert(key, None);
        Some(path)
    }

    fn particle_sheet(&mut self, kind: Kind) -> Option<String> {
        // src: ETC.cpk, 32-px cells (measured), written as they are.
        let path = format!("system/{}", kind.sheet());
        let key = (5, kind as u32); // ours: the image cache's key
        if !self.images.contains_key(&key) {
            let img = image::load_from_memory(self.etc.by_name(kind.sheet())?).ok()?.to_rgba8();
            self.out.write_png(&path, &img);
            self.images.insert(key, None);
        }
        Some(path)
    }

    fn colour(&mut self, rgb: u32) -> String {
        let path = format!("system/c{rgb:06x}.png");
        if !self.images.contains_key(&(3, rgb)) {
            let px = Rgba([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255]);
            let img = RgbaImage::from_pixel(SCREEN_W as u32, SCREEN_H as u32, px);
            self.out.write_png(&path, &img);
            self.images.insert((3, rgb), None);
        }
        path
    }

    fn window(&mut self) -> Option<(String, u32, u32)> {
        let data = self.system.by_name("meswin.png")?;
        let img = image::load_from_memory(data).ok()?.to_rgba8();
        // The window is the left 960 pixels; two loose feathers follow.
        let (w, h) = (img.width().min(SCREEN_W as u32), img.height());
        let img = imageops::crop_imm(&img, 0, 0, w, h).to_image();
        let path = "system/meswin.png".to_string();
        self.out.write_png(&path, &img);
        Some((path, w, h))
    }

    // The eboot's skip dialog (vtable 0x8115befc: layout 0x8103e7d6,
    // refresh 0x8103e248, draw 0x8103ebf2), from SystemData 62 + the
    // protagonist (ss_bg1 … ss_bg5), each rectangle (x, y, w, h) of the
    // atlas centred at a point: the screen (0, 0, 960, 544); the question
    // (606, 746, 302, 39) at (480, 184); the date (n % 6 × 138,
    // 545 + n / 6 × 21, 138, 20) at (129, 48); the emblem (158n, 839, 158,
    // 173) at (164, 406); up to 10 faces (id % 25 × 37, 671 + id / 25 × 37,
    // 37, 37), the last at (858, 60) and each one before 38 to the left;
    // Yes and No (425, 746 and 768, 60, 22; selected 60 to the right) at
    // (415, 241) and (545, 241). The summary is written in white from
    // (300, 319), its lines 27 apart.
    // The story menu (0x81083086): `title_bg.png`'s cells 242×60 from x 353
    // (lit from x 595), one row each from y 545: 始めから, the 〜Prelude〜
    // header, 美咲's, 明日香's, 京's and 狩男's after-stories, ？？？？？;
    // centred at x 788 and y 225, 305, 365, 395, 425, 455 (？？？？？ in
    // 狩男's place). The single-line labels lie in rows 5-33 of their cell:
    // their buttons are those 30 rows (cut from row 3), not overlapping.
    // The header is drawn unlit (the Vita lights it while the cursor is on
    // an after-story).
    fn story_menu(&mut self, locked: bool) -> Option<(Vec<Placed>, Vec<Placed>)> {
        // src: exe 0x81083086 (story_menu_init)
        const X: i64 = 788;
        let atlas = self.system_image("title_bg.png")?;
        let title = self.put_system_png("title", &imageops::crop_imm(&atlas, 0, 0, 960, 544).to_image());
        let cut = |assets: &mut Self, name: &str, row: u32, cy: i64, lit: bool, (top, h): (u32, u32)| -> Placed {
            let path = format!("system/story_{name}.png");
            let xs: &[u32] = if lit { &[353, 595] } else { &[353] };
            let mut sheet = RgbaImage::new(242 * xs.len() as u32, h);
            for (i, &x) in xs.iter().enumerate() {
                let part = imageops::crop_imm(&atlas, x, 545 + row * 60 + top, 242, h).to_image();
                imageops::replace(&mut sheet, &part, i as i64 * 242, 0);
            }
            if assets.cut.insert(path.clone()) {
                assets.out.write_png(&path, &sheet);
            }
            Placed { image: path, x: (X - 121) as i32, y: (cy - 30 + top as i64) as i32, w: sheet.width() as i32, h: h as i32 }
        };
        let full = (0, 60);
        let line = (3, 30);
        let mut pictures = vec![
            Placed { image: title, x: 0, y: 0, w: 960, h: 544 },
            cut(self, "prelude", 1, 305, false, full),
        ];
        let mut buttons = vec![cut(self, "start", 0, 225, true, full)];
        for (i, name) in ["misaki", "asuka", "miyako", "karuo"].iter().enumerate() {
            let (row, cy) = (2 + i as u32, 365 + 30 * i as i64);
            if *name == "karuo" && locked {
                pictures.push(cut(self, "locked", 6, cy, false, line));
            } else {
                buttons.push(cut(self, name, row, cy, true, line));
            }
        }
        Some((pictures, buttons))
    }

    fn skip_dialog(&mut self, bg: &[i64], faces: &[i64], text: &str, name: &str) -> Option<(Vec<Placed>, [Placed; 2])> {
        let labels = Labels::new(self.font?)?;
        let n = *bg.first()?;
        if !(0..5).contains(&n) {
            return None;
        }
        let mut atlas: Option<RgbaImage> = None;
        let mut cut = |assets: &mut Self, part: String, cells: &[(i64, i64)], (w, h): (i64, i64), (cx, cy): (i64, i64)| -> Option<Placed> {
            let path = format!("system/ss{}_{part}.png", n + 1);
            if !assets.cut.contains(&path) {
                if atlas.is_none() {
                    atlas = Some(image::load_from_memory(assets.system.file(62 + n as u32)?.1).ok()?.to_rgba8());
                }
                let img = atlas.as_ref()?;
                let mut sheet = RgbaImage::new(w as u32 * cells.len() as u32, h as u32);
                for (i, &(x, y)) in cells.iter().enumerate() {
                    let part = imageops::crop_imm(img, x as u32, y as u32, w as u32, h as u32).to_image();
                    imageops::replace(&mut sheet, &part, i as i64 * w, 0);
                }
                assets.out.write_png(&path, &sheet);
                assets.cut.insert(path.clone());
            }
            Some(Placed { image: path, x: (cx - w / 2) as i32, y: (cy - h / 2) as i32, w: (w * cells.len() as i64) as i32, h: h as i32 })
        };
        // src: exe 0x8103e248, 0x8103e7d6 (every cell and centre below)
        let mut pictures = vec![cut(self, "bg".into(), &[(0, 0)], (960, 544), (480, 272))?, cut(self, "question".into(), &[(606, 746)], (302, 39), (480, 184))?];
        if let Some(&d) = bg.get(2).filter(|d| (0..36).contains(*d)) {
            pictures.push(cut(self, format!("date{d}"), &[(d % 6 * 138, 545 + d / 6 * 21)], (138, 20), (129, 48))?);
        }
        if let Some(&e) = bg.get(1).filter(|e| (0..5).contains(*e)) {
            pictures.push(cut(self, format!("emblem{e}"), &[(158 * e, 839)], (158, 173), (164, 406))?);
        }
        let shown = &faces[faces.len().saturating_sub(10)..];
        for (k, &id) in shown.iter().rev().enumerate() {
            if (0..50).contains(&id) {
                pictures.push(cut(self, format!("face{id}"), &[(id % 25 * 37, 671 + id / 25 * 37)], (37, 37), (858 - 38 * k as i64, 60))?);
            }
        }
        // src: exe 0x8103ebf2 (origin, 27 apart, scale 1.0), 0x8104a120 (font cell 22.0)
        let lines: Vec<RgbaImage> = text.lines().map(|l| labels.draw(l, 22.0, [255, 255, 255], [0, 0, 0], 255)).collect();
        if let Some(h) = lines.iter().map(|l| l.height()).max() {
            // draw() leaves 4 pixels around the letters.
            let mut img = RgbaImage::new(lines.iter().map(|l| l.width()).max()?, 27 * (lines.len() as u32 - 1) + h);
            for (i, l) in lines.iter().enumerate() {
                imageops::overlay(&mut img, l, 0, 27 * i as i64);
            }
            let path = format!("system/skip_{name}.png");
            self.out.write_png(&path, &img);
            pictures.push(Placed { image: path, x: 296, y: 315, w: img.width() as i32, h: img.height() as i32 });
        }
        let yes = cut(self, "yes".into(), &[(425, 746), (485, 746)], (60, 22), (415, 241))?;
        let no = cut(self, "no".into(), &[(425, 768), (485, 768)], (60, 22), (545, 241))?;
        Some((pictures, [yes, no]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adx_v4_loop() {
        let mut h = vec![0u8; 0x40];
        h[0] = 0x80;
        h[0x12] = 4;
        h[0x24..0x28].copy_from_slice(&1u32.to_be_bytes());
        h[0x28..0x2c].copy_from_slice(&3_004_800u32.to_be_bytes());
        h[0x30..0x34].copy_from_slice(&6_076_799u32.to_be_bytes());
        assert_eq!(adx_loop(&h), Some((3_004_800, 6_076_800)));
        h[0x27] = 0;
        assert_eq!(adx_loop(&h), None);
    }
}
