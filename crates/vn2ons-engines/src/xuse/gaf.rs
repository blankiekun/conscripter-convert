//! `GAF4` image containers used by Xuse's in-house engine (`HD/bg0000.004`,
//! `HD/bu0000.004`, and `.004` entries inside XARC archives in
//! 最果てのイマ COMPLETE).
//!
//! Layout:
//!
//! ```text
//! 0x00  "GAF4"
//! 0x04  u16       version (3)
//! 0x06  [u8; 64]  key name, NUL-terminated ("Farthest" or "generic"; the
//!                 padding after the NUL is uninitialised memory)
//! 0x46  u32 LE    image count
//! 0x4a  ...       encrypted: a tagged structure ("DSET", "PICT", …) that
//!                 contains one complete PNG file per image
//! ```
//!
//! Everything from 0x4a on is XORed with a key derived from the name
//! ([`key_for`], transcribed from `Farthest2015.exe` at 0x40b2e0), indexed by
//! absolute file offset: `byte ^= key[offset % (key.len() - 1)]` (the loader
//! at 0x4af460; the key's last byte is a NUL that the cycle skips).
//!
//! The tagged structure around the PNGs is not decoded yet; [`Gaf::images`]
//! finds the PNGs by their signature and walks their chunks to `IEND`. For
//! all 86 files in the game this yields exactly the header's image count.

use thiserror::Error;

use crate::read::{Reader, Truncated};

const MAGIC: &[u8; 4] = b"GAF4";
const NAME_OFFSET: usize = 6;
const NAME_LEN: usize = 64;
const COUNT_OFFSET: usize = 0x46;
const DATA_OFFSET: usize = 0x4a;
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

#[derive(Debug, Error)]
pub enum GafError {
    #[error("not a GAF4 file (bad magic)")]
    BadMagic,
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("key name is empty")]
    EmptyName,
    #[error("PNG at {offset:#x} has no IEND chunk")]
    UnterminatedPng { offset: usize },
    #[error("header says {expected} images but {found} PNGs were found")]
    CountMismatch { expected: usize, found: usize },
}

/// Builds the XOR key for a key name. The returned key ends with a NUL byte
/// that is not part of the cycle.
pub fn key_for(name: &[u8]) -> Vec<u8> {
    let n = name.len();
    let sx = |b: u8| b as i8 as i16 as u16;
    let mut h: u16 = 0;
    for (i, &c) in name.iter().enumerate() {
        h = (sx(c).wrapping_add(i as u16) ^ h).wrapping_add(n as u16);
    }
    let len = (h & 0xff) as usize + 0x40;
    let h2 = name.iter().fold(h, |acc, &c| acc.wrapping_add(sx(c)));

    let mut key = vec![0u8; len];
    let mut bl = (h2 & 0x0f) as u8;
    key[0] = bl;
    key[1] = (h2 >> 8) as u8;
    key[2] = 0x46;
    key[3] = 0x88;
    for c in 4..len - 1 {
        let dl = (name[c % n] ^ bl).wrapping_add(c as u8);
        bl = bl.wrapping_add(dl);
        key[c] = bl;
    }
    key
}

/// XORs `buf`, which starts at absolute file offset `offset`, with `key`.
pub fn apply_key(buf: &mut [u8], offset: usize, key: &[u8]) {
    let cycle = key.len() - 1;
    for (i, b) in buf.iter_mut().enumerate() {
        *b ^= key[(offset + i) % cycle];
    }
}

#[derive(Debug)]
pub struct Gaf {
    pub version: u16,
    pub name: String,
    pub count: usize,
    /// The whole file with everything from 0x4a on decrypted.
    decrypted: Vec<u8>,
}

impl Gaf {
    pub fn parse(bytes: &[u8]) -> Result<Self, GafError> {
        let r = Reader(bytes);
        if r.slice(0, 4)? != MAGIC {
            return Err(GafError::BadMagic);
        }
        let version = r.u16(4)?;
        let name_field = r.slice(NAME_OFFSET, NAME_LEN)?;
        let name = name_field.split(|&b| b == 0).next().unwrap_or_default();
        if name.is_empty() {
            return Err(GafError::EmptyName);
        }
        let count = r.u32(COUNT_OFFSET)? as usize;
        r.slice(0, DATA_OFFSET)?;

        let mut decrypted = bytes.to_vec();
        apply_key(&mut decrypted[DATA_OFFSET..], DATA_OFFSET, &key_for(name));
        Ok(Self { version, name: String::from_utf8_lossy(name).into_owned(), count, decrypted })
    }

    /// The decrypted file (header unchanged).
    pub fn decrypted(&self) -> &[u8] {
        &self.decrypted
    }

    /// The embedded PNG files, in order. Fails if the number found differs
    /// from the header's count.
    pub fn images(&self) -> Result<Vec<&[u8]>, GafError> {
        let d = &self.decrypted;
        let r = Reader(d);
        let mut out = Vec::new();
        let mut pos = DATA_OFFSET;
        while let Some(found) = find(&d[pos..], PNG_SIGNATURE) {
            let start = pos + found;
            let mut chunk = start + PNG_SIGNATURE.len();
            loop {
                let len = r.u32_be(chunk).map_err(|_| GafError::UnterminatedPng { offset: start })? as usize;
                let kind = r.slice(chunk + 4, 4).map_err(|_| GafError::UnterminatedPng { offset: start })?;
                chunk = chunk
                    .checked_add(12 + len)
                    .filter(|&end| end <= d.len())
                    .ok_or(GafError::UnterminatedPng { offset: start })?;
                if kind == b"IEND" {
                    break;
                }
            }
            out.push(&d[start..chunk]);
            pos = chunk;
        }
        if out.len() != self.count {
            return Err(GafError::CountMismatch { expected: self.count, found: out.len() });
        }
        Ok(out)
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(tag: u8) -> Vec<u8> {
        let mut v = PNG_SIGNATURE.to_vec();
        for (kind, data) in [(&b"IHDR"[..], vec![tag; 13]), (b"IDAT", vec![tag; 5]), (b"IEND", vec![])] {
            v.extend_from_slice(&(data.len() as u32).to_be_bytes());
            v.extend_from_slice(kind);
            v.extend_from_slice(&data);
            v.extend_from_slice(&[0; 4]); // CRC (not checked)
        }
        v
    }

    fn build(name: &[u8], images: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"GAF4".to_vec();
        out.extend_from_slice(&3u16.to_le_bytes());
        let mut field = name.to_vec();
        field.resize(NAME_LEN, 0xcc);
        field[name.len()] = 0;
        out.extend_from_slice(&field);
        out.extend_from_slice(&(images.len() as u32).to_le_bytes());
        let mut body = b"DSET\x02\x00\x00\x00".to_vec();
        for img in images {
            body.extend_from_slice(b"PICT");
            body.extend_from_slice(&(img.len() as u32).to_le_bytes());
            body.extend_from_slice(img);
        }
        apply_key(&mut body, DATA_OFFSET, &key_for(name));
        out.extend(body);
        out
    }

    #[test]
    fn key_matches_the_engine() {
        // Values checked against the keystream recovered from the game's
        // files: the key for "generic" is 201 bytes and starts like this.
        let key = key_for(b"generic");
        assert_eq!(key.len(), 201);
        assert_eq!(&key[..8], &[0x06, 0x03, 0x46, 0x88, 0x7e, 0x9a, 0x99, 0x9e]);
        assert_eq!(key.last(), Some(&0));
    }

    #[test]
    fn extracts_pngs() {
        let images = [png(1), png(2), png(3)];
        let bytes = build(b"Farthest", &images);
        let gaf = Gaf::parse(&bytes).unwrap();
        assert_eq!(gaf.name, "Farthest");
        assert_eq!(gaf.count, 3);
        let found = gaf.images().unwrap();
        assert_eq!(found.len(), 3);
        assert_eq!(found[1], images[1].as_slice());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(matches!(Gaf::parse(b"NOPE"), Err(GafError::BadMagic)));
        let bytes = build(b"generic", &[png(1)]);
        assert!(Gaf::parse(&bytes[..0x40]).is_err());

        // Wrong count in the header.
        let mut wrong = bytes.clone();
        wrong[COUNT_OFFSET] = 2;
        assert!(matches!(
            Gaf::parse(&wrong).unwrap().images(),
            Err(GafError::CountMismatch { expected: 2, found: 1 })
        ));

        // PNG cut off before IEND.
        let cut = &bytes[..bytes.len() - 6];
        assert!(matches!(Gaf::parse(cut).unwrap().images(), Err(GafError::UnterminatedPng { .. })));
    }
}
