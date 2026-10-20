//! BNTX texture files (the pictures, inside NXGX/NXCX): Switch textures,
//! block-linear swizzled, BC-compressed. Layout as read by Switch Toolbox
//! and tegra_swizzle; only what the game uses (2D, one layer, mip 0).
//!
//! Header: `BNTX`; at 0x20 `NX  `, u32 texture count, u64 offset of an
//! array of u64 offsets to the `BRTI` texture headers. `BRTI`: at 0x12 u16
//! tile mode (0 block-linear, 1 linear), 0x16 u16 mip count, 0x1C u32
//! format (high byte the kind, low byte unorm/srgb), 0x24 u32 width,
//! 0x28 u32 height, 0x2C depth, 0x30 layers, 0x34 log2 of the block
//! height in GOBs, 0x50 u32 size of all mips, 0x60 u64 name offset (u16 length, bytes), 0x70 u64
//! offset of the mip offsets (u64 each, from the file's start).

use image::RgbaImage;

use super::FormatError;
use crate::read::Reader;

#[derive(Debug)]
pub struct Texture<'a> {
    pub name: String,
    /// The `BRTI` format, e.g. `0x2006` BC7 sRGB.
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub block_height_log2: u32,
    /// Mip 0, swizzled.
    pub data: &'a [u8],
}

/// A u64 offset (the files are far below 4 GiB).
fn offset(r: &Reader, at: usize) -> Result<usize, FormatError> {
    match r.u32(at + 4)? {
        0 => Ok(r.u32(at)? as usize),
        _ => Err(FormatError::Bad(format!("BNTX offset over 4 GiB at {at:#x}"))),
    }
}

/// The first texture's width and height, from the start of the data
/// (the headers come before the pixels).
pub fn size(head: &[u8]) -> Result<(u32, u32), FormatError> {
    if !head.starts_with(b"BNTX") {
        return Err(FormatError::BadMagic("BNTX"));
    }
    let r = Reader(head);
    let at = offset(&r, offset(&r, 0x28)?)?;
    if r.slice(at, 4)? != b"BRTI" {
        return Err(FormatError::BadMagic("BRTI"));
    }
    Ok((r.u32(at + 0x24)?, r.u32(at + 0x28)?))
}

pub fn parse(data: &[u8]) -> Result<Vec<Texture<'_>>, FormatError> {
    if !data.starts_with(b"BNTX") {
        return Err(FormatError::BadMagic("BNTX"));
    }
    let r = Reader(data);
    let count = r.u32(0x24)? as usize;
    let table = offset(&r, 0x28)?;
    (0..count)
        .map(|i| {
            let at = offset(&r, table + 8 * i)?;
            if r.slice(at, 4)? != b"BRTI" {
                return Err(FormatError::BadMagic("BRTI"));
            }
            let name_at = offset(&r, at + 0x60)?;
            let name = String::from_utf8_lossy(r.slice(name_at + 2, usize::from(r.u16(name_at)?))?).into_owned();
            let (tile, depth, layers) = (r.u16(at + 0x12)?, r.u32(at + 0x2C)?, r.u32(at + 0x30)?);
            if tile != 0 || depth != 1 || layers != 1 {
                return Err(FormatError::Bad(format!("{name}: tile mode {tile}, depth {depth}, {layers} layers (only one block-linear 2D layer is read)")));
            }
            let (format, width, height) = (r.u32(at + 0x1C)?, r.u32(at + 0x24)?, r.u32(at + 0x28)?);
            let block_height_log2 = r.u32(at + 0x34)? & 7;
            let (block, bpp) = kind(format).ok_or_else(|| FormatError::Bad(format!("{name}: format {format:#06x} not read")))?;
            let mip0 = offset(&r, offset(&r, at + 0x70)?)?;
            let size = swizzled_size(width.div_ceil(block), height.div_ceil(block), bpp, block_height_log2);
            let (mips, stored) = (r.u16(at + 0x16)?, r.u32(at + 0x50)? as usize);
            if mips == 1 && size != stored {
                return Err(FormatError::Bad(format!("{name}: {width}x{height} {format:#06x} swizzles to {size} bytes, header says {stored}")));
            }
            Ok(Texture { data: r.slice(mip0, size)?, name, format, width, height, block_height_log2 })
        })
        .collect()
}

/// A format's block side in pixels and bytes per block.
fn kind(format: u32) -> Option<(u32, u32)> {
    Some(match format >> 8 {
        0x0B => (1, 4),                       // R8G8B8A8
        0x1A | 0x1D => (4, 8),                // BC1, BC4
        0x1B | 0x1C | 0x20 => (4, 16),        // BC2, BC3, BC7
        _ => return None,
    })
}

const GOB_BYTES: u32 = 64; // a GOB is 64 bytes by 8 rows, 512 bytes

fn swizzled_size(w: u32, h: u32, bpp: u32, bh_log2: u32) -> usize {
    let rows = 8 << bh_log2;
    ((w * bpp).div_ceil(GOB_BYTES) * GOB_BYTES * h.div_ceil(rows) * rows) as usize
}

/// The byte of element (x, y) in block-linear memory, x in bytes.
fn swizzled(x: u32, y: u32, row_bytes: u32, bh_log2: u32) -> usize {
    let gobs_wide = row_bytes.div_ceil(GOB_BYTES);
    let block = 512 << bh_log2; // one column of GOBs, a block high
    let gob = (y >> (3 + bh_log2)) * gobs_wide * block + (x / GOB_BYTES) * block + ((y >> 3) & ((1 << bh_log2) - 1)) * 512;
    (gob + (x % 64) / 32 * 256 + (y % 8) / 2 * 64 + (x % 32) / 16 * 32 + (y % 2) * 16 + x % 16) as usize
}

/// Mip 0 as RGBA (sRGB formats come out as they are stored; BC4 as grey).
pub fn decode(t: &Texture) -> Result<RgbaImage, FormatError> {
    let (block, bpp) = kind(t.format).ok_or_else(|| FormatError::Bad(format!("{}: format {:#06x} not read", t.name, t.format)))?;
    let (bw, bh) = (t.width.div_ceil(block), t.height.div_ceil(block));
    // Whole blocks first, cropped to the picture at the end.
    let pitch = (bw * block * 4) as usize;
    let mut out = vec![0u8; pitch * (bh * block) as usize];
    for by in 0..bh {
        for bx in 0..bw {
            let at = swizzled(bx * bpp, by, bw * bpp, t.block_height_log2);
            let src = &t.data[at..at + bpp as usize];
            let dst = &mut out[(by * block) as usize * pitch + (bx * block * 4) as usize..];
            match t.format >> 8 {
                0x0B => dst[..4].copy_from_slice(src),
                0x1A => bcdec_rs::bc1(src, dst, pitch),
                0x1B => bcdec_rs::bc2(src, dst, pitch),
                0x1C => bcdec_rs::bc3(src, dst, pitch),
                0x1D => {
                    // One channel (the transition masks): grey, opaque.
                    let mut grey = [0u8; 16];
                    bcdec_rs::bc4(src, &mut grey, 4, false);
                    for (i, v) in grey.into_iter().enumerate() {
                        dst[i / 4 * pitch + i % 4 * 4..][..4].copy_from_slice(&[v, v, v, 255]);
                    }
                }
                _ => bcdec_rs::bc7(src, dst, pitch),
            }
        }
    }
    let full = RgbaImage::from_raw(bw * block, bh * block, out).expect("sized above");
    Ok(image::imageops::crop_imm(&full, 0, 0, t.width, t.height).to_image())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swizzle_addresses() {
        // The first GOB: 16-byte columns of 2 rows, then 32 then 256.
        assert_eq!(swizzled(0, 0, 64, 0), 0);
        assert_eq!(swizzled(0, 1, 64, 0), 16);
        assert_eq!(swizzled(16, 0, 64, 0), 32);
        assert_eq!(swizzled(0, 2, 64, 0), 64);
        assert_eq!(swizzled(32, 0, 64, 0), 256);
        // Block height 2 GOBs: row 8 is the column's second GOB, the next
        // 64 bytes across are the next column.
        assert_eq!(swizzled(0, 8, 128, 1), 512);
        assert_eq!(swizzled(64, 0, 128, 1), 1024);
        assert_eq!(swizzled(0, 16, 128, 1), 2048);
        // The sizes in the game's headers (BC7: 16 bytes a block).
        assert_eq!(swizzled_size(480, 270, 16, 4), 2_949_120); // 1920x1080
        assert_eq!(swizzled_size(5, 5, 16, 0), 1024); // 20x20
        assert_eq!(swizzled_size(1024, 10, 16, 0), 262_144); // 4095x39
    }

    #[test]
    fn rgba_texture() {
        // A 2x2 R8G8B8A8 picture, linear in one GOB: (x, y) at x*4 + y*16.
        let mut data = vec![0u8; 512];
        for (i, at) in [0usize, 4, 16, 20].into_iter().enumerate() {
            data[at..at + 4].copy_from_slice(&[i as u8; 4]);
        }
        let t = Texture { name: "t".into(), format: 0x0B01, width: 2, height: 2, block_height_log2: 0, data: &data };
        assert_eq!(decode(&t).unwrap().into_raw(), [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3]);
    }

    #[test]
    fn bc4_is_grey() {
        // One BC4 block, endpoints 200 and 0, every index 0: all 200.
        let mut data = vec![0u8; 512];
        data[0] = 200;
        let t = Texture { name: "t".into(), format: 0x1D01, width: 3, height: 2, block_height_log2: 0, data: &data };
        assert_eq!(decode(&t).unwrap().into_raw(), [200, 200, 200, 255].repeat(6));
    }
}
