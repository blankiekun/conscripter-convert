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

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage, imageops};

use super::convert::{Assets, SCREEN_H, SCREEN_W};
use super::cpk::Archive;

/// Top-level output folders the assets go to.
pub const OUTPUT_DIRS: [&str; 7] = ["cg", "chr", "etc", "system", "voice", "se", "bgm"];

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
    output: PathBuf,
    images: HashMap<(u8, u32), Option<Picture>>,
    sounds: HashMap<(u8, u32), Option<String>>,
    /// Movies to convert: output path (relative) → the USM file.
    pub movies: BTreeMap<String, PathBuf>,
    /// The folder of the USM files (`media/data`).
    pub movie_dir: PathBuf,
    /// Sounds to convert, by output path.
    pub audio: BTreeMap<String, AudioJob<'a>>,
    pub errors: Vec<String>,
    /// Write nothing (only picture sizes are read).
    pub dry_run: bool,
}

/// Width and height from a PNG header.
fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.get(..8)? != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(data.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(data.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

/// The loop of an ADX (version 4) file: start and end sample frames.
pub fn adx_loop(data: &[u8]) -> Option<(u64, u64)> {
    let be = |at: usize| Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as u64);
    if data.get(..2)? != [0x80, 0] || *data.get(0x12)? != 4 || be(0x24)? == 0 {
        return None;
    }
    Some((be(0x28)?, be(0x30)? + 1))
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
            output: output.to_path_buf(),
            images: HashMap::new(),
            sounds: HashMap::new(),
            audio: BTreeMap::new(),
            movies: BTreeMap::new(),
            movie_dir: PathBuf::new(),
            errors: Vec::new(),
            dry_run: false,
        }
    }

    fn write(&mut self, rel: &str, data: &[u8]) {
        if self.dry_run {
            return;
        }
        let dest = self.output.join(rel);
        let result = dest.parent().map_or(Ok(()), fs::create_dir_all).and_then(|_| fs::write(&dest, data));
        if let Err(e) = result {
            self.errors.push(format!("{}: {e}", dest.display()));
        }
    }

    fn write_png(&mut self, rel: &str, img: &RgbaImage) {
        if self.dry_run {
            return;
        }
        let dest = self.output.join(rel);
        let result = dest.parent().map_or(Ok(()), |p| fs::create_dir_all(p).map_err(image::ImageError::IoError)).and_then(|_| img.save(&dest));
        if let Err(e) = result {
            self.errors.push(format!("{}: {e}", dest.display()));
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
            let (w, h) = png_size(data)?;
            let stem = name.rsplit_once('.').map_or(name, |(s, _)| s).to_ascii_lowercase();
            Some((format!("{dir}/{stem}.png"), data, w, h))
        });
        let r = match found {
            Some((path, data, w, h)) => {
                self.write(&path, data);
                Some((path, w, h))
            }
            None => {
                self.errors.push(format!("{dir} picture {id} missing or not a PNG"));
                None
            }
        };
        self.images.insert((which, id), r.clone());
        r
    }

    /// Queues sound `id` of bank `which` (0 voice, 1 SE, 2 BGM).
    fn sound(&mut self, which: u8, id: u32) -> Option<String> {
        if let Some(r) = self.sounds.get(&(which, id)) {
            return r.clone();
        }
        let (dir, bank) = match which {
            0 => ("voice", &self.voice),
            1 => ("se", &self.se),
            _ => ("bgm", &self.bgm),
        };
        let r = bank.file(id).map(|(_, data)| {
            let path = format!("{dir}/{id:05}.ogg");
            let looped = if which == 2 { adx_loop(data) } else { None };
            self.audio.insert(path.clone(), AudioJob { source: data, output: self.output.join(&path), looped });
            path
        });
        if r.is_none() {
            self.errors.push(format!("{dir} {id} missing"));
        }
        self.sounds.insert((which, id), r.clone());
        r
    }
}

impl ArchiveAssets<'_> {
    /// Cuts rectangles (x, y, w, h) out of SystemData picture `file` and
    /// lays them side by side, scaled by `scale_percent`, as `system/<out>`.
    pub fn system_cells(&mut self, file: &str, out: &str, cells: &[(u32, u32, u32, u32)], scale_percent: u32) -> Option<(String, u32, u32)> {
        let data = self.system.by_name(file)?;
        let img = match image::load_from_memory(data) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.errors.push(format!("{file}: {e}"));
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
        self.write_png(&path, &sheet);
        Some((path, w, h))
    }
}

impl Assets for ArchiveAssets<'_> {
    fn cg(&mut self, id: u32) -> Option<(String, u32, u32)> {
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
                    self.write_png(&path, &img);
                }
                Err(e) => {
                    self.errors.push(format!("{name}: {e}"));
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

    fn bgm(&mut self, id: u32) -> Option<String> {
        self.sound(2, id)
    }

    fn movie(&mut self, n: u32) -> Option<String> {
        let name = super::usm::MOVIES.get(n as usize)?;
        let source = self.movie_dir.join(format!("{name}.usm"));
        if !source.exists() {
            self.errors.push(format!("{} missing", source.display()));
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
            self.write_png(&path, &img);
            self.images.insert((4, frames), None);
        }
        path
    }

    fn colour(&mut self, rgb: u32) -> String {
        let path = format!("system/c{rgb:06x}.png");
        if !self.images.contains_key(&(3, rgb)) {
            let px = Rgba([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255]);
            let img = RgbaImage::from_pixel(SCREEN_W as u32, SCREEN_H as u32, px);
            self.write_png(&path, &img);
            self.images.insert((3, rgb), None);
        }
        path
    }

    fn window(&mut self) -> Option<(String, u32, u32)> {
        let data = self.system.by_name("meswin.png")?;
        let img = image::load_from_memory(data).ok()?.to_rgba8();
        let (w, h) = (img.width() * 3 / 4, img.height() * 3 / 4);
        let img = imageops::resize(&img, w, h, imageops::FilterType::CatmullRom);
        let path = "system/meswin.png".to_string();
        self.write_png(&path, &img);
        Some((path, w, h))
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
