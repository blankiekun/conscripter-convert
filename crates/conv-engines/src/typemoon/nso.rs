//! NSO (the Switch exe, ExeFS `main`): a 0x100-byte header (magic `NSO0`,
//! u32 flags at 0xC: bits 0–2 say text, rodata and data are LZ4), the
//! three segments' (file offset, memory offset, size) at 0x10, 0x20 and
//! 0x30, and their packed sizes at 0x60. Only the loaded image is built;
//! relocations aren't applied (the tables read here hold no pointers).

use super::FormatError;
use crate::read::Reader;

/// The exe as loaded at address 0: the three segments at their memory
/// offsets, gaps zeroed (bss not included).
pub fn image(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    if !data.starts_with(b"NSO0") {
        return Err(FormatError::BadMagic("NSO"));
    }
    let r = Reader(data);
    let flags = r.u32(0xC)?;
    let mut image = Vec::new();
    for k in 0..3 {
        let (file, memory, size) = (r.u32(0x10 + 0x10 * k)? as usize, r.u32(0x14 + 0x10 * k)? as usize, r.u32(0x18 + 0x10 * k)? as usize);
        let packed = r.u32(0x60 + 4 * k)? as usize;
        let segment = if flags >> k & 1 != 0 { lz4_block(r.slice(file, packed)?, size)? } else { r.slice(file, size)?.to_vec() };
        if image.len() < memory + size {
            image.resize(memory + size, 0);
        }
        image[memory..memory + size].copy_from_slice(&segment);
    }
    Ok(image)
}

/// An LZ4 block (no frame) that unpacks to `size` bytes.
fn lz4_block(src: &[u8], size: usize) -> Result<Vec<u8>, FormatError> {
    let bad = || FormatError::Bad("LZ4 block runs past its end".into());
    let mut out = Vec::with_capacity(size);
    let mut i = 0;
    // A length nibble of 15 goes on in bytes, each added, until one under 255.
    let length = |nibble: u8, i: &mut usize| -> Result<usize, FormatError> {
        let mut n = usize::from(nibble);
        if nibble == 15 {
            loop {
                let b = *src.get(*i).ok_or_else(bad)?;
                *i += 1;
                n += usize::from(b);
                if b != 255 {
                    break;
                }
            }
        }
        Ok(n)
    };
    while i < src.len() {
        let token = src[i];
        i += 1;
        let literals = length(token >> 4, &mut i)?;
        out.extend_from_slice(src.get(i..i + literals).ok_or_else(bad)?);
        i += literals;
        if i == src.len() {
            break; // the last sequence has no match
        }
        let distance = usize::from(u16::from_le_bytes([src[i], *src.get(i + 1).ok_or_else(bad)?]));
        i += 2;
        let matched = length(token & 15, &mut i)? + 4;
        if distance == 0 || distance > out.len() {
            return Err(FormatError::Bad(format!("LZ4 match {distance} back at {}", out.len())));
        }
        // Byte by byte: a match may overlap what it writes.
        let from = out.len() - distance;
        for k in 0..matched {
            out.push(out[from + k]);
        }
    }
    if out.len() != size {
        return Err(FormatError::Bad(format!("LZ4 block unpacked to {} bytes, expected {size}", out.len())));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lz4_literals_and_overlapping_match() {
        // "abc", then 9 bytes from 3 back (overlapping): "abcabcabcabc"; then "Z" alone.
        let block = [0x35, b'a', b'b', b'c', 3, 0, 0x10, b'Z'];
        assert_eq!(lz4_block(&block, 13).unwrap(), b"abcabcabcabcZ");
    }

    #[test]
    fn lz4_long_lengths() {
        // 15 + 5 = 20 literals "x", then a match of 15 + 4 + 1 = 20 from 1 back.
        let mut block = vec![0xFF, 5];
        block.extend([b'x'; 20]);
        block.extend([1, 0, 1, 0x00]);
        assert_eq!(lz4_block(&block, 40).unwrap(), vec![b'x'; 40]);
    }

    #[test]
    fn lz4_rejects_a_match_before_the_start() {
        assert!(lz4_block(&[0x10, b'a', 2, 0], 5).is_err());
    }
}
