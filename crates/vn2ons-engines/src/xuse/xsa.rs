//! `.xsa` voice archives used by Xuse's in-house engine (`CD/BV0000.xsa` …
//! in 最果てのイマ COMPLETE).
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! u32           version (always 1)
//! slot[256]:
//!   u32         data size
//!   u32         absolute data offset
//!   u32, u32    reserved (always 0)
//! data          concatenated clips (Ogg Vorbis), no padding
//! ```
//!
//! Unused slots are all zero and only appear after the used ones (only the
//! last archive is partly filled). Entries have no names. The engine refers to
//! archives as `BV%04d.xsa`, so a clip's global voice number is presumably
//! `archive_number * 256 + slot`; this is unconfirmed until the scripts are
//! decoded.

use std::ops::Range;

use thiserror::Error;

use crate::read::{Reader, Truncated};

pub const SLOTS: usize = 256;
const SLOT_LEN: usize = 16;
const TABLE_END: usize = 4 + SLOTS * SLOT_LEN;

#[derive(Debug, Error)]
pub enum XsaError {
    #[error("unsupported .xsa version {0} (expected 1)")]
    BadVersion(u32),
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("slot {slot}: data {start:#x}..{end:#x} overlaps the slot table")]
    OverlapsTable { slot: usize, start: usize, end: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsaEntry {
    pub slot: usize,
    /// Byte range of the clip within the archive.
    pub data: Range<usize>,
    /// The two reserved u32s; always zero in known files.
    pub reserved: [u32; 2],
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
        r.slice(0, TABLE_END)?;

        let mut entries = Vec::new();
        for slot in 0..SLOTS {
            let base = 4 + slot * SLOT_LEN;
            let size = r.u32(base)? as usize;
            let offset = r.u32(base + 4)? as usize;
            let reserved = [r.u32(base + 8)?, r.u32(base + 12)?];
            if size == 0 && offset == 0 {
                continue;
            }
            r.slice(offset, size)?;
            if offset < TABLE_END {
                return Err(XsaError::OverlapsTable { slot, start: offset, end: offset + size });
            }
            entries.push(XsaEntry { slot, data: offset..offset + size, reserved });
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

    fn build(clips: &[&[u8]]) -> Vec<u8> {
        let mut out = 1u32.to_le_bytes().to_vec();
        let mut offset = TABLE_END;
        for slot in 0..SLOTS {
            let (size, off) = match clips.get(slot) {
                Some(c) => (c.len(), offset),
                None => (0, 0),
            };
            offset += size;
            for v in [size as u32, off as u32, 0, 0] {
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
        let bytes = build(&[b"OggS-one", b"OggS-two!"]);
        let xsa = Xsa::parse(&bytes).unwrap();
        assert_eq!(xsa.entries.len(), 2);
        assert_eq!(xsa.entries[1].slot, 1);
        assert_eq!(xsa.data(&xsa.entries[0]), b"OggS-one");
        assert_eq!(xsa.data(&xsa.entries[1]), b"OggS-two!");
    }

    #[test]
    fn rejects_bad_input() {
        let mut bytes = build(&[b"OggS"]);
        assert!(matches!(Xsa::parse(&bytes[..100]), Err(XsaError::Truncated(_))));
        assert!(matches!(Xsa::parse(&bytes[..bytes.len() - 1]), Err(XsaError::Truncated(_))));

        // Offset pointing into the slot table.
        bytes[8..12].copy_from_slice(&16u32.to_le_bytes());
        assert!(matches!(Xsa::parse(&bytes), Err(XsaError::OverlapsTable { .. })));

        bytes[0] = 2;
        assert!(matches!(Xsa::parse(&bytes), Err(XsaError::BadVersion(2))));
    }
}
