//! WebP output (`--webp`): re-encodes the converted PNGs as near-lossless
//! WebP, about half the size, and points the script at them. With
//! `--webp-quality` the colours are lossy at that quality instead (alpha
//! stays lossless), smaller still.
//!
//! CONscripter's web build decodes WebP with its own libwebp (emscripten's
//! SDL_image has none); the other builds read it through SDL_image when it
//! was built with libwebp. Stock ONScripter builds usually can't, so PNG
//! stays the default.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebpError {
    #[error("reading {path}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("decoding {path}")]
    Png { path: PathBuf, source: image::ImageError },
    #[error("encoding {path}: {reason:?}")]
    Encode { path: PathBuf, reason: webp::WebPEncodingError },
}

/// Near-lossless preprocessing level (0-100, lower changes more pixels):
/// 60 changes each channel by at most 2, which doesn't show.
const NEAR_LOSSLESS: i32 = 60;

/// The PNG image `png` as WebP: near-lossless, or lossy at `quality`
/// (0-100, as `cwebp -q`). Opaque images are encoded without an alpha
/// channel.
pub fn encode(png: &[u8], quality: Option<u8>) -> Result<Vec<u8>, EncodeError> {
    encode_rgba(&image::load_from_memory_with_format(png, image::ImageFormat::Png)?.to_rgba8(), quality)
}

/// [`encode`] for a decoded picture.
pub fn encode_rgba(img: &image::RgbaImage, quality: Option<u8>) -> Result<Vec<u8>, EncodeError> {
    let (w, h) = img.dimensions();
    let opaque = img.pixels().all(|p| p[3] == 255);
    let rgb: Vec<u8>;
    let encoder = if opaque {
        rgb = img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
        webp::Encoder::from_rgb(&rgb, w, h)
    } else {
        webp::Encoder::from_rgba(img, w, h)
    };
    // Fails only when the linked libwebp doesn't match its headers.
    let mut config = webp::WebPConfig::new().expect("libwebp config");
    match quality {
        Some(q) => {
            config.lossless = 0;
            config.quality = f32::from(q.min(100));
            config.alpha_quality = 100;
            // Sharper colour edges at 4:2:0 (docs/lessons.md).
            config.use_sharp_yuv = 1;
        }
        None => {
            config.lossless = 1;
            config.near_lossless = NEAR_LOSSLESS;
            config.quality = 75.0; // effort, in lossless mode
        }
    }
    config.method = 4;
    Ok(encoder.encode_advanced(&config).map_err(EncodeError::Webp)?.to_vec())
}

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error(transparent)]
    Png(#[from] image::ImageError),
    #[error("{0:?}")]
    Webp(webp::WebPEncodingError),
}

/// Totals of [`convert_dirs`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Converted {
    pub files: usize,
    pub png_bytes: u64,
    pub webp_bytes: u64,
}

/// Replaces every `.png` file directly inside `dirs` with a `.webp` file of
/// the same stem (see [`encode`] for `quality`), using all CPU cores.
pub fn convert_dirs(dirs: &[PathBuf], quality: Option<u8>) -> Result<Converted, WebpError> {
    let mut files = Vec::new();
    for d in dirs {
        let Ok(entries) = fs::read_dir(d) else { continue };
        for e in entries {
            let path = e.map_err(|source| WebpError::Io { path: d.clone(), source })?.path();
            if path.extension().is_some_and(|x| x == "png") {
                files.push(path);
            }
        }
    }
    convert_files(&files, quality)
}

/// The largest width or height a WebP picture can have.
pub const MAX_SIDE: u32 = 16383;

/// What [`convert_tree`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tree {
    pub totals: Converted,
    /// The pictures converted, relative to the root, as [`key`] writes them.
    pub names: HashSet<String>,
    /// Pictures left as PNG: wider or taller than WebP allows.
    pub too_large: usize,
}

/// Replaces every `.png` file under `root/dirs` (subfolders included) with
/// a `.webp` file of the same stem, as [`convert_dirs`]. Pictures larger
/// than [`MAX_SIDE`] (long animation strips) stay PNG; [`retarget_names`]
/// changes the script's names of the others only.
pub fn convert_tree(root: &Path, dirs: &[&str], quality: Option<u8>) -> Result<Tree, WebpError> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>, too_large: &mut usize) -> Result<(), WebpError> {
        let Ok(entries) = fs::read_dir(dir) else { return Ok(()) };
        for e in entries {
            let path = e.map_err(|source| WebpError::Io { path: dir.to_path_buf(), source })?.path();
            if path.is_dir() {
                walk(&path, files, too_large)?;
            } else if path.extension().is_some_and(|x| x == "png") {
                match png_size(&path) {
                    Some((w, h)) if w <= MAX_SIDE && h <= MAX_SIDE => files.push(path),
                    _ => *too_large += 1,
                }
            }
        }
        Ok(())
    }
    let mut tree = Tree::default();
    let mut files = Vec::new();
    for d in dirs {
        walk(&root.join(d), &mut files, &mut tree.too_large)?;
    }
    tree.totals = convert_files(&files, quality)?;
    tree.names = files.iter().filter_map(|f| f.strip_prefix(root).ok()).map(|rel| key(&rel.to_string_lossy())).collect();
    Ok(tree)
}

/// Width and height from a PNG file's header; None if it isn't a PNG.
fn png_size(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut head = [0u8; 24];
    fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    let be32 = |i: usize| u32::from_be_bytes([head[i], head[i + 1], head[i + 2], head[i + 3]]);
    head.starts_with(b"\x89PNG").then(|| (be32(16), be32(20)))
}

/// A picture name as [`Tree::names`] holds it: `/` between folders,
/// lowercase (the script writes `\`; the archives ignore case).
pub fn key(name: &str) -> String {
    name.replace('\\', "/").to_lowercase()
}

/// The script with the names of the pictures in `names` (see
/// [`convert_tree`]) changed from `.png` to `.webp`. Every picture name
/// the emitter writes ends a string literal, and starts it or follows the
/// `;` of a sprite's options (`bg "cg\a.png"`, `lsp 1,":a;cg\b.png",…`);
/// names of pictures that weren't converted stay as they are.
pub fn retarget_names(script: &str, names: &HashSet<String>) -> String {
    retarget_by(script, |name| names.contains(name))
}

/// As [`retarget_names`] for a dry run, which has no files to convert: every
/// picture under `dirs` is taken as converted (shortcut: a picture too large
/// for WebP is named `.webp` too; no such picture is under the UI folders).
pub fn retarget_dirs(script: &str, dirs: &[&str]) -> String {
    retarget_by(script, |name| dirs.iter().any(|d| name.starts_with(&format!("{}/", key(d)))))
}

fn retarget_by(script: &str, converted: impl Fn(&str) -> bool) -> String {
    let mut out = String::with_capacity(script.len());
    let mut done = 0;
    for (at, _) in script.match_indices(".png\"") {
        let start = script[..at].rfind(['"', ';']).map_or(0, |i| i + 1);
        if converted(&key(&script[start..at + 4])) {
            out.push_str(&script[done..at]);
            out.push_str(".webp");
            done = at + 4;
        }
    }
    out.push_str(&script[done..]);
    out
}

/// Converts `files`, using all CPU cores.
fn convert_files(files: &[PathBuf], quality: Option<u8>) -> Result<Converted, WebpError> {
    let next =AtomicUsize::new(0);
    let totals = Mutex::new(Converted::default());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| -> Result<(), WebpError> {
                    while let Some(path) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let (png, webp) = convert_file(path, quality)?;
                        let mut t = totals.lock().unwrap();
                        t.files += 1;
                        t.png_bytes += png;
                        t.webp_bytes += webp;
                    }
                    Ok(())
                })
            })
            .collect();
        workers.into_iter().try_for_each(|w| w.join().unwrap())
    })?;
    Ok(totals.into_inner().unwrap())
}

/// Converts one file; returns the PNG and WebP sizes.
fn convert_file(path: &Path, quality: Option<u8>) -> Result<(u64, u64), WebpError> {
    let io = |source| WebpError::Io { path: path.to_path_buf(), source };
    let png = fs::read(path).map_err(io)?;
    let webp = encode(&png, quality).map_err(|e| match e {
        EncodeError::Png(source) => WebpError::Png { path: path.to_path_buf(), source },
        EncodeError::Webp(reason) => WebpError::Encode { path: path.to_path_buf(), reason },
    })?;
    fs::write(path.with_extension("webp"), &webp).map_err(io)?;
    fs::remove_file(path).map_err(io)?;
    Ok((png.len() as u64, webp.len() as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(img: &image::RgbaImage) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn keeps_alpha_and_drops_it_when_opaque() {
        let mut img = image::RgbaImage::from_pixel(16, 8, image::Rgba([200, 10, 10, 255]));
        let opaque = encode(&png(&img), None).unwrap();
        let decoded = webp::Decoder::new(&opaque).decode().unwrap();
        assert!(!decoded.is_alpha());
        let at = (3 * 16 + 3) * 3;
        assert_eq!(&decoded[at..at + 3], [200, 10, 10]);

        img.put_pixel(0, 0, image::Rgba([0, 0, 255, 128]));
        let translucent = encode(&png(&img), None).unwrap();
        let decoded = webp::Decoder::new(&translucent).decode().unwrap();
        assert!(decoded.is_alpha());
        let px = &decoded[..4];
        assert!(px[3].abs_diff(128) <= 2 && px[2].abs_diff(255) <= 2, "{px:?}");
    }

    #[test]
    fn lossy_quality_is_smaller_and_keeps_alpha() {
        // A gradient with noise, as in painted art, which lossless can't
        // squeeze much.
        let mut seed = 0x2545_f491_u32;
        let img = image::RgbaImage::from_fn(256, 256, |x, y| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let n = (seed % 24) as u8;
            image::Rgba([x as u8 / 2 + n, y as u8 / 2 + n, 100 + n, if x < 8 { 0 } else { 255 }])
        });
        let near = encode(&png(&img), None).unwrap();
        let lossy = encode(&png(&img), Some(50)).unwrap();
        assert!(lossy.len() < near.len(), "{} >= {}", lossy.len(), near.len());
        let decoded = webp::Decoder::new(&lossy).decode().unwrap();
        assert!(decoded.is_alpha());
        assert_eq!(decoded[3], 0); // (0, 0): transparent, exactly
        assert_eq!(decoded[(10 * 256 + 20) * 4 + 3], 255);
    }

    #[test]
    fn dry_run_names_match_converted_names() {
        let script = "lsp 1,\":a;sys\\title\\bg1.png\",0,0\nbg \"pic\\a.webp\",1\nlsp 2,\"sys\\txtwindow.png\",0,0\nlsp 3,\"pic\\rule_01.png\",0,0\n";
        let names: HashSet<String> = ["sys/title/bg1.png", "sys/txtwindow.png"].map(String::from).into();
        let out = retarget_dirs(script, &["sys"]);
        assert_eq!(out, retarget_names(script, &names));
        assert!(out.contains("sys\\title\\bg1.webp") && out.contains("rule_01.png"));
    }

    #[test]
    fn converts_subfolders_and_retargets_only_converted_names() {
        let root = std::env::temp_dir().join(format!("conv-webp-tree-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let write = |rel: &str, w: u32, h: u32| {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, png(&image::RgbaImage::from_pixel(w, h, image::Rgba([1, 2, 3, 255])))).unwrap();
        };
        write("cg/sys/アイコン/001.png", 4, 4);
        write("cg/Top.png", 4, 4);
        write("system/strip/wide.png", MAX_SIDE + 1, 1);
        fs::create_dir_all(root.join("sound")).unwrap();
        fs::write(root.join("sound/a.ogg"), b"OggS").unwrap();

        let tree = convert_tree(&root, &["cg", "system", "sound"], None).unwrap();
        assert_eq!(tree.totals.files, 2);
        assert_eq!(tree.too_large, 1);
        assert!(root.join("cg/sys/アイコン/001.webp").exists() && !root.join("cg/sys/アイコン/001.png").exists());
        assert!(root.join("cg/Top.webp").exists());
        assert!(root.join("system/strip/wide.png").exists());
        assert!(root.join("sound/a.ogg").exists());

        let script = "lsp 1,\":a/32,100,0;cg\\sys\\アイコン\\001.png\",0,0\nbg \"cg\\top.png\",1\nlsp 2,\":a;system\\strip\\wide.png\",0,0\nlsp 3,\":a;cg\\gone.png\",0,0\nmov $1,\"cg\\Top.png\"\n";
        assert_eq!(
            retarget_names(script, &tree.names),
            "lsp 1,\":a/32,100,0;cg\\sys\\アイコン\\001.webp\",0,0\nbg \"cg\\top.webp\",1\nlsp 2,\":a;system\\strip\\wide.png\",0,0\nlsp 3,\":a;cg\\gone.png\",0,0\nmov $1,\"cg\\Top.webp\"\n"
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
