//! WebP output (`--webp`): re-encodes the converted PNGs as near-lossless
//! WebP, about half the size, and points the script at them. With
//! `--webp-quality` the colours are lossy at that quality instead (alpha
//! stays lossless), smaller still.
//!
//! CONscripter's web build decodes WebP with its own libwebp (emscripten's
//! SDL_image has none); the other builds read it through SDL_image when it
//! was built with libwebp. Stock ONScripter builds usually can't, so PNG
//! stays the default.

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
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)?.to_rgba8();
    let (w, h) = img.dimensions();
    let opaque = img.pixels().all(|p| p[3] == 255);
    let rgb: Vec<u8>;
    let encoder = if opaque {
        rgb = img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
        webp::Encoder::from_rgb(&rgb, w, h)
    } else {
        webp::Encoder::from_rgba(&img, w, h)
    };
    // Fails only when the linked libwebp doesn't match its headers.
    let mut config = webp::WebPConfig::new().expect("libwebp config");
    match quality {
        Some(q) => {
            config.lossless = 0;
            config.quality = f32::from(q.min(100));
            config.alpha_quality = 100;
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
    let next = AtomicUsize::new(0);
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

/// The script with its image names changed from `.png` to `.webp`. Every
/// image name the emitter writes ends a string literal (`bg "bg\a.png"`,
/// `lsp 1,":a;sprite\a.png",…`).
pub fn retarget(script: &str) -> String {
    script.replace(".png\"", ".webp\"")
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
    fn retargets_image_names_only() {
        let script = "bg \"bg\\a.png\",1\nlsp 1,\":a/3,0,3;system\\b.png\",0,0\nbgm \"bgm\\c.ogg\"\n";
        assert_eq!(retarget(script), "bg \"bg\\a.webp\",1\nlsp 1,\":a/3,0,3;system\\b.webp\",0,0\nbgm \"bgm\\c.ogg\"\n");
    }
}
