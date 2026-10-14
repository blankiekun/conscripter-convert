//! The game's files for the compiler ([`super::convert::Assets`]), taken
//! from the NPA archives and written into the output directory.
//!
//! Script paths name the archive by their first folder: `cg/bg/x.jpg` is
//! `bg\x.jpg` in `cg.npa`, `sound/se/x` is `se\x.ogg` in `sound.npa`,
//! `voice/ma01/x` is `ma01\x.ogg` in `voice.npa`. The output keeps the
//! script's path, lower-cased (`cg/bg/x.jpg`, `sound/se/x.ogg`). Pictures
//! the converter makes go to `system/`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::convert::{Assets, SCREEN_H, SCREEN_W};
use super::npa::{Npa, normalize};

/// A zoomed picture wider or taller than this is cut to the screen.
const MAX_SCALED: i64 = 4096;

/// Top-level output folders the assets go to.
pub const OUTPUT_DIRS: [&str; 4] = ["cg", "sound", "voice", "system"];

pub struct NpaAssets<'a> {
    cg: Npa<'a>,
    sound: Npa<'a>,
    voice: Npa<'a>,
    output: PathBuf,
    images: HashMap<String, Option<(String, u32, u32)>>,
    sounds: HashMap<String, Option<String>>,
    /// Files that couldn't be read or written.
    pub errors: Vec<String>,
    /// Write nothing (a dry run: only image sizes are read).
    pub dry_run: bool,
}

impl<'a> NpaAssets<'a> {
    pub fn new(cg: Npa<'a>, sound: Npa<'a>, voice: Npa<'a>, output: &Path) -> Self {
        NpaAssets { cg, sound, voice, output: output.to_path_buf(), images: HashMap::new(), sounds: HashMap::new(), errors: Vec::new(), dry_run: false }
    }

    /// Copies `entry` of the archive that `path`'s first folder names to
    /// `out` (relative to the output). Returns the data.
    fn extract(&mut self, path: &str, out: &str) -> Option<Vec<u8>> {
        let path = normalize(path);
        let (archive, rest) = path.split_once('/')?;
        let npa = match archive {
            "cg" => &self.cg,
            "sound" => &self.sound,
            "voice" => &self.voice,
            _ => return None,
        };
        let entry = npa.find(rest)?;
        if self.dry_run && archive != "cg" {
            return Some(Vec::new());
        }
        let data = match npa.data(entry) {
            Ok(d) => d,
            Err(e) => {
                self.errors.push(e.to_string());
                return None;
            }
        };
        self.write(out, &data);
        Some(data)
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

    /// Width and height of a picture in the output (one taken from the
    /// archive, or a colour picture), without decoding it: what a dry run
    /// needs to name and place the pictures made from it.
    fn size(&mut self, image: &str) -> Option<(u32, u32)> {
        if let Some(Some((_, w, h))) = self.images.get(&normalize(image)) {
            return Some((*w, *h));
        }
        if let Some(size) = image.strip_prefix("system/color_").and_then(|r| r.strip_suffix(".png")).and_then(|r| r.split_once('_')).and_then(|(_, s)| s.split_once('x')) {
            return Some((size.0.parse().ok()?, size.1.parse().ok()?));
        }
        let data = fs::read(self.output.join(image)).ok()?;
        image_size(&data)
    }

    /// `image` (`iw`×`ih`) scaled to `w`×`h`, or only the part `clip`
    /// (x0, y0, x1, y1 in the scaled picture) of it: its path and size.
    fn make_scaled(&mut self, image: &str, (iw, ih): (u32, u32), (zx, zy): (i64, i64), (w, h): (u32, u32), clip: Option<(i64, i64, i64, i64)>) -> Option<(String, u32, u32)> {
        let path = Path::new(image);
        let stem = path.file_stem()?.to_string_lossy().into_owned();
        let jpeg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"));
        let dir = path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let part = clip.map_or(String::new(), |(x0, y0, x1, y1)| format!("_{x0}_{y0}_{}x{}", x1 - x0, y1 - y0));
        let rel = format!("system/zoom/{}/{stem}_{zx}x{zy}{part}.{}", dir.replace('/', "_"), if jpeg { "jpg" } else { "png" });
        let (ow, oh) = clip.map_or((w, h), |(x0, y0, x1, y1)| ((x1 - x0) as u32, (y1 - y0) as u32));
        // Its size, for pictures made from it (a dry run has no file).
        self.images.insert(normalize(&rel), Some((rel.clone(), ow, oh)));
        if self.dry_run {
            return Some((rel, ow, oh));
        }
        let img = image::open(self.output.join(image)).ok()?;
        let scaled = match clip {
            None => img.resize_exact(w, h, image::imageops::FilterType::Triangle),
            Some((x0, y0, x1, y1)) => {
                // The source pixels under the clip, scaled, then cut exactly.
                let (sx, sy) = (w as f64 / iw as f64, h as f64 / ih as f64);
                let (sx0, sx1) = ((x0 as f64 / sx).floor() as u32, ((x1 as f64 / sx).ceil() as u32).min(iw));
                let (sy0, sy1) = ((y0 as f64 / sy).floor() as u32, ((y1 as f64 / sy).ceil() as u32).min(ih));
                let part = img.crop_imm(sx0, sy0, sx1 - sx0, sy1 - sy0);
                let (pw, ph) = ((((sx1 - sx0) as f64 * sx).round() as u32).max(ow), (((sy1 - sy0) as f64 * sy).round() as u32).max(oh));
                let part = part.resize_exact(pw, ph, image::imageops::FilterType::Triangle);
                let (ox, oy) = (((x0 as f64 - sx0 as f64 * sx).round() as u32).min(pw - ow), ((y0 as f64 - sy0 as f64 * sy).round() as u32).min(ph - oh));
                part.crop_imm(ox, oy, ow, oh)
            }
        };
        let mut data = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut data);
        if jpeg {
            scaled.into_rgb8().write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90)).ok()?;
        } else {
            scaled.into_rgba8().write_to(&mut cursor, image::ImageFormat::Png).ok()?;
        }
        self.write(&rel, &data);
        Some((rel, ow, oh))
    }

    fn write_png(&mut self, rel: &str, img: &image::RgbaImage) {
        let mut png = Vec::new();
        match img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png) {
            Ok(()) => self.write(rel, &png),
            Err(e) => self.errors.push(format!("{rel}: {e}")),
        }
    }
}

impl Assets for NpaAssets<'_> {
    fn image(&mut self, path: &str) -> Option<(String, u32, u32)> {
        let key = normalize(path);
        if let Some(r) = self.images.get(&key) {
            return r.clone();
        }
        let r = self.extract(&key, &key).and_then(|data| image_size(&data)).map(|(w, h)| (key.clone(), w, h));
        self.images.insert(key, r.clone());
        r
    }

    fn sound(&mut self, path: &str) -> Option<String> {
        let key = normalize(path);
        if let Some(r) = self.sounds.get(&key) {
            return r.clone();
        }
        let file = if key.ends_with(".ogg") { key.clone() } else { format!("{key}.ogg") };
        let r = self.extract(&file, &file).map(|_| file);
        self.sounds.insert(key, r.clone());
        r
    }

    fn color(&mut self, rgb: u32, w: u32, h: u32) -> String {
        let rel = format!("system/color_{rgb:06x}_{w}x{h}.png");
        if !self.output.join(&rel).exists() {
            let [_, r, g, b] = rgb.to_be_bytes();
            self.write_png(&rel, &image::RgbaImage::from_pixel(w, h, image::Rgba([r, g, b, 255])));
        }
        rel
    }

    fn composite(&mut self, parts: &[(String, i64, i64, i64)]) -> Option<(String, i64, i64)> {
        if let [(image, x, y, 1000)] = parts {
            return Some((image.clone(), *x, *y));
        }
        let mut bounds = Vec::new();
        for (image, x, y, _) in parts {
            let (w, h) = self.size(image)?;
            bounds.push((*x, *y, *x + w as i64, *y + h as i64));
        }
        let x0 = bounds.iter().map(|b| b.0).min()?;
        let y0 = bounds.iter().map(|b| b.1).min()?;
        let x1 = bounds.iter().map(|b| b.2).max()?;
        let y1 = bounds.iter().map(|b| b.3).max()?;
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        parts.hash(&mut h);
        let rel = format!("system/window_{:016x}.png", h.finish());
        if self.dry_run || self.output.join(&rel).exists() {
            return Some((rel, x0, y0));
        }
        let mut out = image::RgbaImage::new((x1 - x0) as u32, (y1 - y0) as u32);
        for (image, x, y, opacity) in parts {
            let mut img = image::open(self.output.join(image)).ok()?.into_rgba8();
            for p in img.pixels_mut() {
                p.0[3] = (p.0[3] as i64 * opacity / 1000) as u8;
            }
            image::imageops::overlay(&mut out, &img, x - x0, y - y0);
        }
        self.write_png(&rel, &out);
        Some((rel, x0, y0))
    }

    fn scaled(&mut self, image: &str, zx: i64, zy: i64, at: (i64, i64, i64, i64)) -> Option<(String, i64, i64, u32, u32)> {
        let (iw, ih) = self.size(image)?;
        let (w, h) = (((iw as i64 * zx + 500) / 1000).max(1), ((ih as i64 * zy + 500) / 1000).max(1));
        let (x, y) = (at.0 + (at.2 - w) / 2, at.1 + (at.3 - h) / 2);
        // A big zoom (up to 60x, a dive into the picture) is cut to the
        // part on the screen: the whole would take gigabytes to make and be
        // too large for the engine to load.
        let clip = if w.max(h) > MAX_SCALED {
            let (x0, x1) = ((-x).clamp(0, w), (SCREEN_W - x).clamp(0, w));
            let (y0, y1) = ((-y).clamp(0, h), (SCREEN_H - y).clamp(0, h));
            if x0 >= x1 || y0 >= y1 {
                return Some((self.transparent(), 0, 0, 1, 1));
            }
            Some((x0, y0, x1, y1))
        } else {
            None
        };
        let (cx, cy) = clip.map_or((0, 0), |c| (c.0, c.1));
        let key = format!("{image}@{zx}x{zy}{clip:?}");
        let r = match self.images.get(&key) {
            Some(r) => r.clone(),
            None => {
                let r = self.make_scaled(image, (iw, ih), (zx, zy), (w as u32, h as u32), clip);
                self.images.insert(key, r.clone());
                r
            }
        };
        r.map(|(rel, w, h)| (rel, x + cx, y + cy, w, h))
    }

    fn blurred(&mut self, image: &str, sigma: u32) -> Option<String> {
        if image == "system/transparent.png" {
            return Some(image.to_string());
        }
        let key = format!("{image}~{sigma}");
        if let Some(r) = self.images.get(&key) {
            return r.as_ref().map(|r| r.0.clone());
        }
        let path = Path::new(image);
        let stem = path.file_stem()?.to_string_lossy().into_owned();
        let jpeg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg"));
        let r = (|| {
            let dir = path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
            let rel = format!("system/blur/{}/{stem}_{sigma}.{}", dir.replace('/', "_"), if jpeg { "jpg" } else { "png" });
            if self.dry_run {
                let (w, h) = self.size(image)?;
                return Some((rel, w, h));
            }
            let img = image::open(self.output.join(image)).ok()?;
            let blurred = img.blur(sigma as f32 / 10.0);
            let mut data = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut data);
            if jpeg {
                blurred.into_rgb8().write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90)).ok()?;
            } else {
                blurred.into_rgba8().write_to(&mut cursor, image::ImageFormat::Png).ok()?;
            }
            self.write(&rel, &data);
            Some((rel, img.width(), img.height()))
        })();
        self.images.insert(key, r.clone());
        r.map(|r| r.0)
    }

    fn transparent(&mut self) -> String {
        let rel = "system/transparent.png".to_string();
        if !self.output.join(&rel).exists() {
            self.write_png(&rel, &image::RgbaImage::new(1, 1));
        }
        rel
    }
}

/// Width and height of a PNG or JPEG.
pub fn image_size(data: &[u8]) -> Option<(u32, u32)> {
    let be16 = |i: usize| Some(u16::from_be_bytes(data.get(i..i + 2)?.try_into().ok()?) as u32);
    let be32 = |i: usize| Some(u32::from_be_bytes(data.get(i..i + 4)?.try_into().ok()?));
    if data.starts_with(b"\x89PNG") {
        return Some((be32(16)?, be32(20)?));
    }
    if !data.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    // JPEG: walk the segments to a start-of-frame marker.
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        let len = be16(i + 2)? as usize;
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return Some((be16(i + 7)?, be16(i + 5)?));
        }
        i += 2 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_of_png_and_jpeg() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&1024u32.to_be_bytes());
        png.extend_from_slice(&576u32.to_be_bytes());
        assert_eq!(image_size(&png), Some((1024, 576)));
        // SOI, an APP0 segment of 4 bytes, SOF0 with height 600, width 1280.
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xC0, 0, 17, 8, 0x02, 0x58, 0x05, 0x00];
        assert_eq!(image_size(&jpeg), Some((1280, 600)));
    }
}
