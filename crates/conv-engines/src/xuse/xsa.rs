//! Sound archives used by Xuse's in-house engine (最果てのイマ COMPLETE):
//! voices (`CD/BV0000.xsa` …) and, in the same format, `HD/bm0000.bin` (BGM),
//! `HD/k000*.bin`, `HD/cgv.bin`, `HD/wss.bin` and `HD/silent.bin`.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! u32           version (always 1)
//! slot[n]:      n = (offset of slot 0's data - 4) / 16; 256 for most files
//!   u32         data size
//!   u32         absolute data offset
//!   u32         param 0: loop start in sample frames in bm0000.bin (the
//!               loop runs to the end of the clip), else 0
//!   u32         param 1: total length in samples in wss.bin / silent.bin, else 0
//! data          concatenated clips (Ogg Vorbis), no padding
//! ```
//!
//! Unused slots are all zero and only appear after the used ones. Entries have
//! no names. The engine refers to
//! archives as `BV%04d.xsa`, so a clip's global voice number is presumably
//! `archive_number * 256 + slot`; this is unconfirmed until the scripts are
//! decoded.

use std::ops::Range;

use thiserror::Error;

use crate::read::{Reader, Truncated};

const SLOT_LEN: usize = 16;

#[derive(Debug, Error)]
pub enum XsaError {
    #[error("unsupported .xsa version {0} (expected 1)")]
    BadVersion(u32),
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("slot {slot}: data {start:#x}..{end:#x} overlaps the slot table")]
    OverlapsTable { slot: usize, start: usize, end: usize },
    #[error("slot table size {0:#x} is not 4 + a multiple of 16")]
    BadTableSize(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsaEntry {
    pub slot: usize,
    /// Byte range of the clip within the archive.
    pub data: Range<usize>,
    /// The two extra u32s; see the module docs for what they mean per file.
    pub params: [u32; 2],
}

/// A parsed archive borrowing the archive bytes (typically a memory map).
#[derive(Debug)]
pub struct Xsa<'a> {
    bytes: &'a [u8],
    /// Used slots, in slot order.
    pub entries: Vec<XsaEntry>,
}

impl<'a> Xsa<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, XsaError> {
        let r = Reader(bytes);
        let version = r.u32(0)?;
        if version != 1 {
            return Err(XsaError::BadVersion(version));
        }
        // Slot 0 is always used; its data starts right after the table.
        let table_end = r.u32(8)? as usize;
        if table_end < 4 + SLOT_LEN || !(table_end - 4).is_multiple_of(SLOT_LEN) {
            return Err(XsaError::BadTableSize(table_end));
        }
        r.slice(0, table_end)?;

        let mut entries = Vec::new();
        for slot in 0..(table_end - 4) / SLOT_LEN {
            let base = 4 + slot * SLOT_LEN;
            let size = r.u32(base)? as usize;
            let offset = r.u32(base + 4)? as usize;
            let params = [r.u32(base + 8)?, r.u32(base + 12)?];
            if size == 0 && offset == 0 {
                continue;
            }
            r.slice(offset, size)?;
            if offset < table_end {
                return Err(XsaError::OverlapsTable { slot, start: offset, end: offset + size });
            }
            entries.push(XsaEntry { slot, data: offset..offset + size, params });
        }
        Ok(Self { bytes, entries })
    }

    pub fn data(&self, entry: &XsaEntry) -> &'a [u8] {
        &self.bytes[entry.data.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(slots: usize, clips: &[&[u8]]) -> Vec<u8> {
        let mut out = 1u32.to_le_bytes().to_vec();
        let mut offset = 4 + slots * SLOT_LEN;
        for slot in 0..slots {
            let (size, off) = match clips.get(slot) {
                Some(c) => (c.len(), offset),
                None => (0, 0),
            };
            offset += size;
            for v in [size as u32, off as u32, slot as u32 * 7, 0] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for c in clips {
            out.extend_from_slice(c);
        }
        out
    }

    #[test]
    fn parses_used_slots() {
        let bytes = build(256, &[b"OggS-one", b"OggS-two!"]);
        let xsa = Xsa::parse(&bytes).unwrap();
        assert_eq!(xsa.entries.len(), 2);
        assert_eq!(xsa.entries[1].slot, 1);
        assert_eq!(xsa.entries[1].params, [7, 0]);
        assert_eq!(xsa.data(&xsa.entries[0]), b"OggS-one");
        assert_eq!(xsa.data(&xsa.entries[1]), b"OggS-two!");
    }

    #[test]
    fn slot_count_comes_from_the_table() {
        let bytes = build(182, &[b"OggS-a", b"OggS-b", b"OggS-c"]);
        let xsa = Xsa::parse(&bytes).unwrap();
        assert_eq!(xsa.entries.len(), 3);
        assert_eq!(xsa.data(&xsa.entries[2]), b"OggS-c");
    }

    #[test]
    fn rejects_bad_input() {
        let mut bytes = build(256, &[b"OggS"]);
        assert!(matches!(Xsa::parse(&bytes[..100]), Err(XsaError::Truncated(_))));
        assert!(matches!(Xsa::parse(&bytes[..bytes.len() - 1]), Err(XsaError::Truncated(_))));

        // Table size that isn't 4 + 16n.
        let mut odd = bytes.clone();
        odd[8..12].copy_from_slice(&30u32.to_le_bytes());
        assert!(matches!(Xsa::parse(&odd), Err(XsaError::BadTableSize(30))));

        // A later slot pointing into the slot table.
        bytes[4 + 16 + 4..4 + 16 + 8].copy_from_slice(&16u32.to_le_bytes());
        bytes[4 + 16..4 + 16 + 4].copy_from_slice(&4u32.to_le_bytes());
        assert!(matches!(Xsa::parse(&bytes), Err(XsaError::OverlapsTable { .. })));

        bytes[0] = 2;
        assert!(matches!(Xsa::parse(&bytes), Err(XsaError::BadVersion(2))));
    }
}
