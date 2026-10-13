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

use super::convert::Assets;
use super::npa::{Npa, normalize};

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
}

impl<'a> NpaAssets<'a> {
    pub fn new(cg: Npa<'a>, sound: Npa<'a>, voice: Npa<'a>, output: &Path) -> Self {
        NpaAssets { cg, sound, voice, output: output.to_path_buf(), images: HashMap::new(), sounds: HashMap::new(), errors: Vec::new() }
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
        let dest = self.output.join(rel);
        let result = dest.parent().map_or(Ok(()), fs::create_dir_all).and_then(|_| fs::write(&dest, data));
        if let Err(e) = result {
            self.errors.push(format!("{}: {e}", dest.display()));
        }
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
        let mut layers = Vec::new();
        for (image, x, y, opacity) in parts {
            let mut img = image::open(self.output.join(image)).ok()?.into_rgba8();
            for p in img.pixels_mut() {
                p.0[3] = (p.0[3] as i64 * opacity / 1000) as u8;
            }
            layers.push((img, *x, *y));
        }
        let x0 = layers.iter().map(|l| l.1).min()?;
        let y0 = layers.iter().map(|l| l.2).min()?;
        let x1 = layers.iter().map(|l| l.1 + l.0.width() as i64).max()?;
        let y1 = layers.iter().map(|l| l.2 + l.0.height() as i64).max()?;
        let mut out = image::RgbaImage::new((x1 - x0) as u32, (y1 - y0) as u32);
        for (img, x, y) in &layers {
            image::imageops::overlay(&mut out, img, x - x0, y - y0);
        }
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        parts.hash(&mut h);
        let rel = format!("system/window_{:016x}.png", h.finish());
        if !self.output.join(&rel).exists() {
            self.write_png(&rel, &out);
        }
        Some((rel, x0, y0))
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
