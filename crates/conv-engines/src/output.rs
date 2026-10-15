//! Writing a build's files: what every engine's asset bank shares.

use std::fs;
use std::path::{Path, PathBuf};

/// The output directory of a build, as the asset banks write into it.
pub struct Output {
    pub root: PathBuf,
    /// Write nothing (a dry run).
    pub dry_run: bool,
    /// Files that couldn't be read or written.
    pub errors: Vec<String>,
}

impl Output {
    pub fn new(root: &Path) -> Self {
        Output { root: root.to_path_buf(), dry_run: false, errors: Vec::new() }
    }

    /// Writes `data` to `rel` (relative to the root), making its folders.
    /// A failure goes to [`Output::errors`].
    pub fn write(&mut self, rel: &str, data: &[u8]) {
        if self.dry_run {
            return;
        }
        let dest = self.root.join(rel);
        let result = dest.parent().map_or(Ok(()), fs::create_dir_all).and_then(|_| fs::write(&dest, data));
        if let Err(e) = result {
            self.errors.push(format!("{}: {e}", dest.display()));
        }
    }

    /// Writes `img` as a PNG to `rel`, as [`Output::write`].
    pub fn write_png(&mut self, rel: &str, img: &image::RgbaImage) {
        if self.dry_run {
            return;
        }
        let mut png = Vec::new();
        match img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png) {
            Ok(()) => self.write(rel, &png),
            Err(e) => self.errors.push(format!("{rel}: {e}")),
        }
    }
}

/// Width and height of a PNG or JPEG, from its header.
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

    #[test]
    fn a_dry_run_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("conv-output-test-{}", std::process::id()));
        let mut out = Output::new(&dir);
        out.dry_run = true;
        out.write("a/b.bin", b"x");
        assert!(!dir.exists());
        out.dry_run = false;
        out.write_png("a/c.png", &image::RgbaImage::new(3, 2));
        assert_eq!(image_size(&fs::read(dir.join("a/c.png")).unwrap()), Some((3, 2)));
        assert!(out.errors.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
