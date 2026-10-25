//! 続・殺戮のジャンゴ's PNG gamma: its loader (Django crk.exe 0044f9e0)
//! reads a picture's `gAMA` and calls libpng 1.2.5's
//! `png_set_gamma(png, 2.2, file_gamma)` (0044fa74), so the colours it
//! draws are the file's passed through libpng's 8-bit gamma table.
//! CONscripter's SDL_image ignores `gAMA`, so the converter writes the
//! corrected pixels ([`correct`]) and drops the chunk.

/// The screen gamma the loader passes.
pub const DJANGO_SCREEN_GAMMA: f64 = 2.2; // src: Django crk.exe 0x004d5768 (double), pushed at 0044fa74

/// `png_set_gamma` turns gamma on past this distance of
/// screen × file from 1.
const THRESHOLD: f64 = 0.05; // src: Django crk.exe 0x004ca8f8, read by png_set_gamma 00480530

/// The picture's colour type and `gAMA` (as libpng keeps it, an f32 of
/// the chunk's value / 100000), read from the chunks before `IDAT`.
fn header(png: &[u8]) -> Option<(u8, f32)> {
    let mut i = 8; // src: PNG spec, the signature's length
    let mut color_type = None;
    while i + 8 <= png.len() {
        let len = u32::from_be_bytes(png[i..i + 4].try_into().ok()?) as usize;
        // src: PNG spec, chunk = length, type, body, CRC
        let body = png.get(i + 8..i + 8 + len)?;
        match &png[i + 4..i + 8] { // src: PNG spec
            b"IHDR" => color_type = Some(*body.get(9)?), // src: PNG spec, IHDR byte 9 is the colour type
            b"gAMA" => return Some((color_type?, u32::from_be_bytes(body.get(..4)?.try_into().ok()?) as f32 / 100_000.0)), // src: libpng 1.2.5 png_handle_gAMA
            b"IDAT" => return None,
            _ => {}
        }
        i += 12 + len; // src: PNG spec, length + type + body + CRC
    }
    None
}

/// libpng 1.2.5's 8-bit gamma table for `file_gamma` on `screen_gamma`,
/// or None where `png_set_gamma` leaves gamma off (00480530: off when
/// |screen × file − 1| ≤ 0.05, unless the colour type has alpha or a
/// palette) or the table changes nothing.
// src: exe 004830c0 allocates the 0x100-entry table
pub fn table(color_type: u8, file_gamma: f32, screen_gamma: f64) -> Option<[u8; 256]> {
    let (file, screen) = (f64::from(file_gamma), f64::from(screen_gamma as f32)); // src: png_set_gamma stores both as f32 (+0x15c, +0x160)
    let on = (f64::from(file_gamma) * screen_gamma - 1.0).abs() > THRESHOLD || color_type & 4 != 0 || color_type == 3; // src: exe 00480530 (alpha bit 4, palette 3)
    if !on {
        return None;
    }
    // src: Django crk.exe 004830c0 (png_build_gamma_table): g = 1 / (gamma × screen) when screen > 1e-6;
    // table[i] = ftol(pow(i × (1/255), g) × 255 + 0.5)
    // src: exe 004830c0, 1e-6 at 0x4ca918
    let g = if screen > 1e-6 { 1.0 / (screen * file) } else { 1.0 };
    let mut t = [0u8; 256]; // src: exe 004830c0 (0x100 entries)
    for (i, v) in t.iter_mut().enumerate() {
        *v = ((i as f64 * (1.0 / 255.0)).powf(g) * 255.0 + 0.5) as u8; // src: 004830c0 (constants 0x4d57b0, 0x4d5798, 0x4d5668)
    }
    (t.iter().enumerate().any(|(i, &v)| v as usize != i)).then_some(t)
}

/// `png` with its colours as the exe draws them, re-encoded without
/// `gAMA`; None when the exe draws it as it is (no `gAMA`, gamma off, an
/// identity table) or it can't be read. Colour channels only (alpha
/// untouched); a palette is expanded, which maps its entries.
pub fn correct(png: &[u8], screen_gamma: f64) -> Option<Vec<u8>> {
    let (color_type, gamma) = header(png)?;
    let t = table(color_type, gamma, screen_gamma)?;
    let mut img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let (channels, colours) = (usize::from(img.color().channel_count()), if img.color().has_color() { 3 } else { 1 });
    let bytes: &mut [u8] = match &mut img {
        image::DynamicImage::ImageRgb8(b) => b,
        image::DynamicImage::ImageRgba8(b) => b,
        image::DynamicImage::ImageLuma8(b) => b,
        image::DynamicImage::ImageLumaA8(b) => b,
        _ => return None, // 16-bit: none in the game
    };
    for px in bytes.chunks_exact_mut(channels) {
        for c in &mut px[..colours] {
            *c = t[usize::from(*c)];
        }
    }
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_table_and_rule() {
        // gAMA 0.5 (windowbox.png): +8 at 128; 0.27777 (bar01.png): −45 at 128.
        let t = table(2, 0.5, DJANGO_SCREEN_GAMMA).unwrap();
        assert_eq!((t[0], t[128], t[255]), (0, 136, 255));
        assert_eq!(table(6, 0.27777, DJANGO_SCREEN_GAMMA).unwrap()[128], 83);
        // 0.45 RGB: |2.2 × 0.45 − 1| = 0.01, gamma off; with alpha it is on.
        assert!(table(2, 0.45, DJANGO_SCREEN_GAMMA).is_none());
        assert_eq!(table(6, 0.45, DJANGO_SCREEN_GAMMA).unwrap()[128], 127);
        // 0.45454 with alpha: on, but the table is the identity.
        assert!(table(6, 0.45454, DJANGO_SCREEN_GAMMA).is_none());
    }
}
