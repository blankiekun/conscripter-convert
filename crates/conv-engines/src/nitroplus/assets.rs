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
use std::process::Command;

use super::convert::{Assets, MOVIE_FPS, MovieInfo, SCREEN_H, SCREEN_W};
use super::npa::{Npa, normalize};
use super::npk::Npk;
use crate::output::{Output, image_size};

/// A zoomed picture wider or taller than this is cut to the screen.
const MAX_SCALED: i64 = 4096;

/// Top-level output folders the assets go to.
pub const OUTPUT_DIRS: [&str; 6] = ["cg", "cg2", "en", "sound", "voice", "system"];
/// The output folders with pictures (`--webp` converts them).
pub const PICTURE_DIRS: [&str; 4] = ["cg", "cg2", "en", "system"];

/// Pictures with words that the English `cg.npk` has under the Japanese
/// names, taken from it first when it is given: the name plates and the
/// quick bar (the same places in `boot.nut`'s `QuickMenu`), the telops
/// (chapter titles, captions; placed as the port does by
/// `english::place_telops`).
const ENGLISH_FIRST: &[&str] = &["cg/fw/nw", "cg/sys/quick/", "cg/sys/telop/"];

pub struct NpaAssets<'a> {
    cg: Npa<'a>,
    sound: Npa<'a>,
    voice: Npa<'a>,
    /// `cg2.npa`, which 続・殺戮のジャンゴ's 1.01 update adds (its scripts'
    /// `cg2/…` pictures).
    pub cg2: Option<Npa<'a>>,
    /// The English release's `cg.npk` (`--english`): the pictures only it
    /// has (`cg/sys/adv/en/…`), taken when `cg.npa` lacks them.
    pub cg_en: Option<Npk>,
    /// The English release's `droidserif-muramasa.ttf` (`font.npk`): its
    /// subtitles are drawn in it.
    pub font_en: Option<Vec<u8>>,
    images: HashMap<String, Option<(String, u32, u32)>>,
    sounds: HashMap<String, Option<String>>,
    movies: HashMap<String, Option<MovieInfo>>,
    /// The disc's movie folder (`ムービー`, beside the game data): the
    /// scripts' `dx/<name>.ngs` are `<name>.ngs` there.
    pub movie_dir: Option<PathBuf>,
    /// Where the files go; a dry run reads only image sizes.
    pub out: Output,
    /// The screen's size, which big zooms are cut to.
    pub screen: (i64, i64),
}

impl<'a> NpaAssets<'a> {
    pub fn new(cg: Npa<'a>, sound: Npa<'a>, voice: Npa<'a>, output: &Path) -> Self {
        NpaAssets { cg, sound, voice, cg2: None, cg_en: None, font_en: None, images: HashMap::new(), sounds: HashMap::new(), movies: HashMap::new(), movie_dir: None, out: Output::new(output), screen: (SCREEN_W, SCREEN_H) }
    }

    /// The movie file a script path names, and its name in the output:
    /// the file's own name when it is ASCII, else a hash of it.
    fn movie_source(&self, path: &str) -> Option<(PathBuf, String)> {
        // `dx/<name>.ngs`, or `dx/3d/<name>.ngs` (the extras' armour).
        let name = path.replace('\\', "/");
        let name = name.split_once('/').map_or(name.as_str(), |(_, rest)| rest);
        let src = self.movie_dir.as_ref()?.join(name);
        let stem = name.strip_suffix(".ngs").unwrap_or(name).replace('/', "_");
        let out = if stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            stem
        } else {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            stem.hash(&mut h);
            format!("mv_{:08x}", h.finish() as u32)
        };
        src.is_file().then_some((src, out))
    }

    /// Copies `entry` of the archive that `path`'s first folder names to
    /// `out` (relative to the output). Returns the data.
    fn extract(&mut self, path: &str, out: &str) -> Option<Vec<u8>> {
        let path = normalize(path);
        if self.cg_en.is_some() && ENGLISH_FIRST.iter().any(|p| path.starts_with(p)) && let Some(data) = self.extract_english(&path, out) {
            return Some(data);
        }
        let (archive, rest) = path.split_once('/')?;
        let npa = match archive {
            "cg" => &self.cg,
            "cg2" => self.cg2.as_ref()?,
            "sound" => &self.sound,
            "voice" => &self.voice,
            // The English release's own pictures where the Japanese have
            // the same name (`en/cg/sys/title/…`).
            "en" => return self.extract_english(rest, out),
            _ => return None,
        };
        let Some(entry) = npa.find(rest) else { return self.extract_english(&path, out) };
        if self.out.dry_run && archive != "cg" && archive != "cg2" {
            return Some(Vec::new());
        }
        let data = match npa.data(entry) {
            Ok(d) => d,
            Err(e) => {
                self.out.errors.push(e.to_string());
                return None;
            }
        };
        self.out.write(out, &data);
        Some(data)
    }

    /// `path` from the English release's `cg.npk`, whose `.png`s are
    /// lossless WebP: written as PNG.
    fn extract_english(&mut self, path: &str, out: &str) -> Option<Vec<u8>> {
        let npk = self.cg_en.as_ref()?;
        let entry = npk.find(&format!("media/{path}"))?;
        let png = npk.data(entry).map_err(|e| e.to_string()).and_then(|webp| {
            let img = image::load_from_memory_with_format(&webp, image::ImageFormat::WebP).map_err(|e| format!("{path}: {e}"))?;
            let mut png = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).map_err(|e| e.to_string())?;
            Ok(png)
        });
        match png {
            Ok(png) => {
                self.out.write(out, &png);
                Some(png)
            }
            Err(e) => {
                self.out.errors.push(e);
                None
            }
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
        let data = fs::read(self.out.root.join(image)).ok()?;
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
        if self.out.dry_run {
            return Some((rel, ow, oh));
        }
        let img = image::open(self.out.root.join(image)).ok()?;
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
        self.out.write(&rel, &data);
        Some((rel, ow, oh))
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
        if !self.out.root.join(&rel).exists() {
            let [_, r, g, b] = rgb.to_be_bytes();
            self.out.write_png(&rel, &image::RgbaImage::from_pixel(w, h, image::Rgba([r, g, b, 255])));
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
        if self.out.dry_run || self.out.root.join(&rel).exists() {
            return Some((rel, x0, y0));
        }
        let mut out = image::RgbaImage::new((x1 - x0) as u32, (y1 - y0) as u32);
        for (image, x, y, opacity) in parts {
            let mut img = image::open(self.out.root.join(image)).ok()?.into_rgba8();
            for p in img.pixels_mut() {
                p.0[3] = (p.0[3] as i64 * opacity / 1000) as u8;
            }
            image::imageops::overlay(&mut out, &img, x - x0, y - y0);
        }
        self.out.write_png(&rel, &out);
        Some((rel, x0, y0))
    }

    fn screen(&mut self, parts: &[(String, i64, i64, i64)]) -> Option<String> {
        let (rel, x0, y0) = self.composite(parts)?;
        if (x0, y0) == (0, 0) {
            return Some(rel);
        }
        let cut = rel.replace(".png", "_screen.png");
        if self.out.dry_run || self.out.root.join(&cut).exists() {
            return Some(cut);
        }
        let img = image::open(self.out.root.join(&rel)).ok()?.into_rgba8();
        let mut out = image::RgbaImage::new(SCREEN_W as u32, SCREEN_H as u32);
        image::imageops::overlay(&mut out, &img, x0, y0);
        self.out.write_png(&cut, &out);
        Some(cut)
    }

    fn scaled(&mut self, image: &str, zx: i64, zy: i64, at: (i64, i64, i64, i64)) -> Option<(String, i64, i64, u32, u32)> {
        let (iw, ih) = self.size(image)?;
        let (w, h) = (((iw as i64 * zx + 500) / 1000).max(1), ((ih as i64 * zy + 500) / 1000).max(1));
        let (x, y) = (at.0 + (at.2 - w) / 2, at.1 + (at.3 - h) / 2);
        // A big zoom (up to 60x, a dive into the picture) is cut to the
        // part on the screen: the whole would take gigabytes to make and be
        // too large for the engine to load.
        let clip = if w.max(h) > MAX_SCALED {
            let (x0, x1) = ((-x).clamp(0, w), (self.screen.0 - x).clamp(0, w));
            let (y0, y1) = ((-y).clamp(0, h), (self.screen.1 - y).clamp(0, h));
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
            if self.out.dry_run {
                let (w, h) = self.size(image)?;
                return Some((rel, w, h));
            }
            let img = image::open(self.out.root.join(image)).ok()?;
            let blurred = img.blur(sigma as f32 / 10.0);
            let mut data = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut data);
            if jpeg {
                blurred.into_rgb8().write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90)).ok()?;
            } else {
                blurred.into_rgba8().write_to(&mut cursor, image::ImageFormat::Png).ok()?;
            }
            self.out.write(&rel, &data);
            Some((rel, img.width(), img.height()))
        })();
        self.images.insert(key, r.clone());
        r.map(|r| r.0)
    }

    fn mirrored(&mut self, image: &str, x: bool, y: bool) -> Option<String> {
        if image == "system/transparent.png" || image.starts_with("system/color_") {
            return Some(image.to_string());
        }
        let flip = format!("{}{}", if x { "x" } else { "" }, if y { "y" } else { "" });
        let key = format!("{image}~{flip}");
        if let Some(r) = self.images.get(&key) {
            return r.as_ref().map(|r| r.0.clone());
        }
        let path = Path::new(image);
        let stem = path.file_stem()?.to_string_lossy().into_owned();
        let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let r = (|| {
            let dir = path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
            let rel = format!("system/mirror/{}/{stem}_{flip}.{ext}", dir.replace('/', "_"));
            if self.out.dry_run {
                let (w, h) = self.size(image)?;
                return Some((rel, w, h));
            }
            // Read and written as they are: a JPEG keeps its quality, a
            // PNG (perhaps a strip of cells) is the same size.
            let mut img = image::open(self.out.root.join(image)).ok()?;
            let (w, h) = (img.width(), img.height());
            if x {
                img = img.fliph();
            }
            if y {
                img = img.flipv();
            }
            let mut data = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut data);
            if ext == "jpg" || ext == "jpeg" {
                img.into_rgb8().write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90)).ok()?;
            } else {
                img.into_rgba8().write_to(&mut cursor, image::ImageFormat::Png).ok()?;
            }
            self.out.write(&rel, &data);
            Some((rel, w, h))
        })();
        // Its size, for pictures made from it (a dry run has no file).
        if let Some((rel, w, h)) = &r {
            self.images.insert(normalize(rel), Some((rel.clone(), *w, *h)));
        }
        self.images.insert(key, r.clone());
        r.map(|r| r.0)
    }

    fn strip(&mut self, parts: &[&str]) -> Option<(String, u32, u32)> {
        let mut cells = Vec::new();
        for part in parts {
            // "" is an empty cell.
            cells.push(if part.is_empty() { None } else { Some(self.image(part)?) });
        }
        let w = cells.iter().flatten().map(|c| c.1).max()?;
        let h = cells.iter().flatten().map(|c| c.2).max()?;
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        parts.hash(&mut hash);
        let rel = format!("system/strip/{:016x}.png", hash.finish());
        let size = (w * cells.len() as u32, h);
        if self.out.dry_run || self.out.root.join(&rel).exists() {
            return Some((rel, size.0, size.1));
        }
        let mut out = image::RgbaImage::new(size.0, size.1);
        for (i, (cell, _, _)) in cells.iter().enumerate().filter_map(|(i, c)| Some((i, c.as_ref()?))) {
            let img = image::open(self.out.root.join(cell)).ok()?.into_rgba8();
            image::imageops::overlay(&mut out, &img, (i as u32 * w) as i64, 0);
        }
        self.out.write_png(&rel, &out);
        Some((rel, size.0, size.1))
    }

    fn movie_info(&mut self, path: &str) -> Option<MovieInfo> {
        if let Some(r) = self.movies.get(path) {
            return *r;
        }
        let r = self.movie_source(path).and_then(|(src, _)| probe(&src));
        self.movies.insert(path.to_string(), r);
        r
    }

    fn movie_file(&mut self, path: &str, sound: bool) -> Option<String> {
        let (src, name) = self.movie_source(path)?;
        let rel = format!("movie/{name}{}.mp4", if sound { "" } else { "_mute" });
        let dest = self.out.root.join(&rel);
        if !self.out.dry_run && !dest.exists() {
            let mut args: Vec<std::ffi::OsString> = vec!["-i".into(), src.clone().into()];
            args.extend(crate::media::mp4_options(22, "slow").into_iter().map(Into::into));
            if !sound {
                args.push("-an".into());
            }
            if let Err(e) = crate::media::ffmpeg(&dest, args, |_| Ok(())) {
                self.out.errors.push(format!("{}: {e}", src.display()));
                return None;
            }
        }
        Some(rel)
    }

    fn movie_frames(&mut self, path: &str, at: Option<u32>, alpha: bool, dark: bool) -> Option<(String, u32, u32, u32)> {
        let info = self.movie_info(path)?;
        let (src, name) = self.movie_source(path)?;
        // Frames at half size (a cut that fills the screen is 2.4 MB a
        // frame when loaded), and an alpha movie (a cloud of smoke,
        // blurred) always: the sprite is zoomed back. A still is whole, and
        // a small movie (the extras' 400×400 armour) too.
        let (w, h) = match (alpha, at) {
            (true, _) => (info.w / 2, info.h / 4),
            (false, None) if info.w as i64 > SCREEN_W / 2 => (info.w / 2, info.h / 2),
            (false, _) => (info.w, info.h),
        };
        let when = at.map_or("all".to_string(), |ms| ms.to_string());
        let rel = format!("system/movie/{name}_{when}{}.{}", if dark { "_dark" } else { "" }, if alpha { "png" } else { "jpg" });
        let key = format!("movie:{rel}");
        if let Some(r) = self.images.get(&key) {
            return r.as_ref().map(|(rel, cells, h)| (rel.clone(), *cells, w, *h));
        }
        // A long movie (続・殺戮のジャンゴ's 70 s ブラックホーク) at fewer
        // frames a second: a JPEG strip is at most 65535 pixels wide.
        let full = (info.ms * MOVIE_FPS).div_ceil(1000).max(1);
        let fit = if alpha { full } else { (65535 / w.max(1)).max(1) };
        let cells = at.map_or(full.min(fit), |_| 1);
        let fps = if cells < full { f64::from(cells) * 1000.0 / f64::from(info.ms.max(1)) } else { f64::from(MOVIE_FPS) };
        let r = if self.out.dry_run {
            Some(cells)
        } else if self.out.root.join(&rel).exists() {
            self.size(&rel).map(|(sw, _)| sw / w)
        } else {
            movie_strip(&src, at, alpha, dark, (w, h), fps, cells).map(|(strip, n)| {
                let mut data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut data);
                let ok = if alpha {
                    strip.write_to(&mut cursor, image::ImageFormat::Png).is_ok()
                } else {
                    image::DynamicImage::ImageRgba8(strip).into_rgb8().write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90)).is_ok()
                };
                if ok {
                    self.out.write(&rel, &data);
                }
                n
            })
        };
        self.images.insert(key, r.map(|n| (rel.clone(), n, h)));
        r.map(|n| (rel, n, w, h))
    }

    fn movie_sprite(&mut self, path: &str, alpha: bool, dark: bool) -> Option<(String, u32, u32)> {
        let info = self.movie_info(path)?;
        let (src, name) = self.movie_source(path)?;
        let rel = format!("movie/{name}_mute{}{}.mp4", if dark { "_dark" } else { "" }, if alpha { "_alpha" } else { "" });
        let size = if alpha { (info.w, info.h / 2) } else { (info.w, info.h) };
        let dest = self.out.root.join(&rel);
        if self.out.dry_run || dest.exists() {
            return Some((rel, size.0, size.1));
        }
        // All in video range (16-235): black is 16. `:a` takes the right
        // half as the opacity inverted (white is clear), so the bottom
        // half's luma v becomes 16 + 235 - v.
        let colour = if dark { ",lutyuv=y=16:u=128:v=128" } else { "" };
        let graph = if alpha {
            format!("[0:v]format=yuv420p,split[a][b];[a]crop=iw:ih/2:0:0{colour}[c];[b]crop=iw:ih/2:0:ih/2,lutyuv=y=clip(251-val\\,16\\,235):u=128:v=128[m];[c][m]hstack")
        } else {
            format!("[0:v]format=yuv420p{colour}")
        };
        let mut args: Vec<std::ffi::OsString> = vec!["-i".into(), src.clone().into(), "-filter_complex".into(), graph.into()];
        args.extend(crate::media::mp4_options(22, "slow").into_iter().map(Into::into));
        args.push("-an".into());
        if let Err(e) = crate::media::ffmpeg(&dest, args, |_| Ok(())) {
            self.out.errors.push(format!("{}: {e}", src.display()));
            return None;
        }
        Some((rel, size.0, size.1))
    }

    fn overlaid(&mut self, base: &str, top: &str, zoom: i64, opacity: i64) -> Option<String> {
        let (top, tw, th) = self.image(top)?;
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (base, &top, zoom, opacity).hash(&mut h);
        let rel = format!("system/overlay_{:016x}.jpg", h.finish());
        if self.out.dry_run || self.out.root.join(&rel).exists() {
            return Some(rel);
        }
        let base_img = image::open(self.out.root.join(base)).ok()?.into_rgb8();
        let top_img = image::open(self.out.root.join(&top)).ok()?.into_rgb8();
        let (sw, sh) = base_img.dimensions();
        let (cx, cy) = (tw as f64 / 2.0, th as f64 / 2.0);
        let z = zoom.max(1) as f64 / 1000.0;
        let a = opacity.clamp(0, 1000) as f64 / 1000.0;
        let mut out = base_img.clone();
        for (x, y, p) in out.enumerate_pixels_mut() {
            // The top picture's pixel under this one (nearest).
            let (tx, ty) = (((x as f64 + 0.5 - cx) / z + cx) as i64, ((y as f64 + 0.5 - cy) / z + cy) as i64);
            if tx < 0 || ty < 0 || tx >= tw as i64 || ty >= th as i64 || x >= sw || y >= sh {
                continue;
            }
            let t = top_img.get_pixel(tx as u32, ty as u32);
            for c in 0..3 {
                let (b, f) = (p.0[c] as f64 / 255.0, t.0[c] as f64 / 255.0);
                let o = if b < 0.5 { 2.0 * b * f } else { 1.0 - 2.0 * (1.0 - b) * (1.0 - f) };
                p.0[c] = ((b + (o - b) * a) * 255.0).round() as u8;
            }
        }
        let mut data = Vec::new();
        image::DynamicImage::ImageRgb8(out).write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut std::io::Cursor::new(&mut data), 92)).ok()?;
        self.out.write(&rel, &data);
        Some(rel)
    }

    fn rgba(&mut self, path: &str) -> Option<image::RgbaImage> {
        if self.out.dry_run {
            return None;
        }
        let rel = if path.starts_with("cg/") || path.starts_with("en/") { self.image(path)?.0 } else { path.to_string() };
        Some(image::open(self.out.root.join(rel)).ok()?.into_rgba8())
    }

    fn frames_movie(&mut self, name: &str, fps: u32, frames: u32, draw: &mut dyn FnMut(u32, &mut image::RgbImage)) -> Option<String> {
        let rel = format!("movie/{name}.mp4");
        let dest = self.out.root.join(&rel);
        if self.out.dry_run || dest.exists() {
            return Some(rel);
        }
        let (w, h) = (SCREEN_W as u32, SCREEN_H as u32);
        let size = format!("{w}x{h}");
        let mut args: Vec<std::ffi::OsString> = ["-f", "rawvideo", "-pix_fmt", "rgb24", "-s", &size, "-r", &fps.to_string(), "-i", "-"].map(Into::into).to_vec();
        args.extend(crate::media::mp4_options(20, "slow").into_iter().map(Into::into));
        args.push("-an".into());
        let mut frame = image::RgbImage::new(w, h);
        let result = crate::media::ffmpeg(&dest, args, |stdin| {
            (0..frames).try_for_each(|i| {
                draw(i, &mut frame);
                stdin.write_all(frame.as_raw())
            })
        });
        if let Err(e) = result {
            self.out.errors.push(format!("{rel}: {e}"));
            return None;
        }
        Some(rel)
    }

    fn frames_strip(&mut self, name: &str, w: u32, h: u32, frames: u32, draw: &mut dyn FnMut(u32, &mut image::RgbaImage)) -> Option<String> {
        let rel = format!("system/strip/{name}.png");
        if self.out.dry_run || self.out.root.join(&rel).exists() {
            return Some(rel);
        }
        let mut out = image::RgbaImage::new(w * frames, h);
        let mut frame = image::RgbaImage::new(w, h);
        for i in 0..frames {
            frame.pixels_mut().for_each(|p| *p = image::Rgba([0; 4]));
            draw(i, &mut frame);
            image::imageops::replace(&mut out, &frame, (i * w) as i64, 0);
        }
        self.out.write_png(&rel, &out);
        Some(rel)
    }

    fn delayed_sound(&mut self, path: &str, delay_ms: u32) -> Option<String> {
        let src = self.sound(path)?;
        let rel = format!("{}_after{delay_ms}.ogg", src.trim_end_matches(".ogg"));
        let dest = self.out.root.join(&rel);
        if self.out.dry_run || dest.exists() {
            return Some(rel);
        }
        let delay = format!("adelay={delay_ms}:all=1");
        let args: Vec<std::ffi::OsString> = vec!["-i".into(), self.out.root.join(&src).into(), "-af".into(), delay.into(), "-c:a".into(), "libvorbis".into(), "-q:a".into(), "5".into()];
        if let Err(e) = crate::media::ffmpeg(&dest, args, |_| Ok(())) {
            self.out.errors.push(format!("{rel}: {e}"));
            return None;
        }
        Some(rel)
    }

    fn looped_sound(&mut self, path: &str, start_ms: u32, end_ms: u32) -> Option<String> {
        let src = self.sound(path)?;
        let rel = format!("{}_loop{start_ms}-{end_ms}.ogg", src.trim_end_matches(".ogg"));
        if self.out.dry_run || self.out.root.join(&rel).exists() {
            return Some(rel);
        }
        let data = std::fs::read(self.out.root.join(&src)).ok()?;
        let rate = ogg_rate(&data)? as u64;
        match crate::ogg::set_loop(&data, start_ms as u64 * rate / 1000, Some(end_ms as u64 * rate / 1000)) {
            Ok(tagged) => self.out.write(&rel, &tagged),
            Err(e) => {
                self.out.errors.push(format!("{rel}: {e}"));
                return None;
            }
        }
        Some(rel)
    }

    fn sound_ms(&mut self, path: &str) -> Option<u32> {
        let key = normalize(path);
        let file = if key.ends_with(".ogg") { key } else { format!("{key}.ogg") };
        let (npa, rest) = match file.split_once('/')? {
            ("sound", rest) => (&self.sound, rest),
            ("voice", rest) => (&self.voice, rest),
            _ => return None,
        };
        let data = npa.data(npa.find(rest)?).ok()?;
        ogg_ms(&data)
    }

    fn english_font(&mut self) -> Option<Vec<u8>> {
        self.font_en.clone()
    }

    fn transparent(&mut self) -> String {
        let rel = "system/transparent.png".to_string();
        if !self.out.root.join(&rel).exists() {
            self.out.write_png(&rel, &image::RgbaImage::new(1, 1));
        }
        rel
    }
}

/// An Ogg Vorbis file's length: its last page's granule position (the
/// samples so far) over the sample rate of its identification header.
fn ogg_ms(data: &[u8]) -> Option<u32> {
    let rate = ogg_rate(data)?;
    let last = data.windows(4).rposition(|w| w == b"OggS")?;
    let samples = u64::from_le_bytes(data.get(last + 6..last + 14)?.try_into().ok()?);
    (rate > 0).then(|| (samples * 1000 / rate as u64) as u32)
}

/// An Ogg Vorbis file's sample rate, from its identification header.
fn ogg_rate(data: &[u8]) -> Option<u32> {
    let id = data.windows(7).position(|w| w == b"\x01vorbis")?;
    Some(u32::from_le_bytes(data.get(id + 12..id + 16)?.try_into().ok()?))
}

/// A movie's size, length and whether its sound is above -60 dB (some
/// battle cuts carry a silent track), by ffprobe and ffmpeg.
fn probe(src: &Path) -> Option<MovieInfo> {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height:format=duration", "-of", "default=nw=1"])
        .arg(src)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)).map(str::trim);
    let w = field("width=")?.parse().ok()?;
    let h = field("height=")?.parse().ok()?;
    let ms = (field("duration=")?.parse::<f64>().ok()? * 1000.0).round() as u32;
    let out = Command::new("ffmpeg").args(["-v", "info", "-nostdin", "-i"]).arg(src).args(["-vn", "-af", "volumedetect", "-f", "null", "-"]).output().ok()?;
    let log = String::from_utf8_lossy(&out.stderr);
    let max = log.lines().find_map(|l| l.split_once("max_volume:")).and_then(|(_, v)| v.trim().trim_end_matches("dB").trim().parse::<f64>().ok());
    Some(MovieInfo { w, h, ms, sound: max.is_some_and(|db| db > -60.0) })
}

/// A movie's frames at `fps` (or the one at `at` ms), each `size`, side by
/// side, at most `max` of them: the picture and the number of frames.
#[allow(clippy::too_many_arguments)]
fn movie_strip(src: &Path, at: Option<u32>, alpha: bool, dark: bool, (w, h): (u32, u32), fps: f64, max: u32) -> Option<(image::RgbaImage, u32)> {
    let colour = if dark { ",lutrgb=r=0:g=0:b=0" } else { "" };
    // The opacity is the bottom half's luma, in video range (16-235).
    let graph = if alpha {
        format!("[0:v]split[a][b];[a]crop=iw:ih/2:0:0{colour}[c];[b]crop=iw:ih/2:0:ih/2,lutyuv=y=clip((val-16)*255/219\\,0\\,255),format=gray[m];[c][m]alphamerge,scale={w}:{h}")
    } else {
        format!("[0:v]scale={w}:{h}{colour}")
    };
    let graph = match at {
        Some(_) => graph,
        None if fps == f64::from(MOVIE_FPS) => format!("{graph},fps={MOVIE_FPS}"),
        None => format!("{graph},fps={fps:.4}"),
    };
    let decode = |seek: &[&str], frames: bool| {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-v", "error", "-nostdin"]).args(seek).arg("-i").arg(src).args(["-filter_complex", &graph, "-an"]);
        if frames {
            cmd.args(["-frames:v", "1"]);
        }
        cmd.args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"]).output().ok().map(|o| o.stdout)
    };
    let frame = (w * h * 4) as usize;
    let mut data = match at {
        Some(ms) => decode(&["-ss", &format!("{:.3}", f64::from(ms) / 1000.0)], true)?,
        None => decode(&[], false)?,
    };
    // A time past the last frame (seeks in these streams land late): the
    // last frame.
    if data.len() < frame && at.is_some() {
        let all = decode(&["-sseof", "-1"], false)?;
        let n = all.len() / frame;
        data = all.get(n.checked_sub(1)? * frame..n * frame)?.to_vec();
    }
    let n = ((data.len() / frame) as u32).min(max.max(1));
    if n == 0 {
        return None;
    }
    data.truncate(n as usize * frame);
    let mut strip = image::RgbaImage::new(w * n, h);
    for (i, data) in data.chunks_exact(frame).enumerate() {
        let img = image::RgbaImage::from_raw(w, h, data.to_vec())?;
        image::imageops::replace(&mut strip, &img, i as i64 * w as i64, 0);
    }
    Some((strip, n))
}
