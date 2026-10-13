//! `XARC` archives used by Xuse's in-house engine (e.g. `HD/wnd.arc` in
//! 最果てのイマ COMPLETE).
//!
//! Layout (all integers little-endian unless noted):
//!
//! ```text
//! header:
//!   "XARC"
//!   u32        entry count (n)
//!   u32[n]     absolute offset of each entry
//!   u16 BE     CRC-16/XMODEM of everything above
//!
//! entry (at offsets[i]):
//!   "DATA"
//!   [u8; 4]    unknown (seen: 20 00 00 00, 20 20 00 00)
//!   SYSTEMTIME 8 x u16: year, month, day_of_week, day, hour, minute, second, ms
//!   u32        name length (bytes)
//!   u32        data size
//!   [u8]       name, each byte nibble-swapped (0x07 -> 0x70 'p')
//!   u16 BE     CRC-16/XMODEM of the entry header ("DATA" .. end of name)
//!   [u8]       data, stored raw
//!   u16 BE     CRC-16/XMODEM of the data
//! ```

use std::ops::Range;

use thiserror::Error;

use crate::read::{Reader, Truncated};

const MAGIC: &[u8; 4] = b"XARC";
const ENTRY_MAGIC: &[u8; 4] = b"DATA";
/// "DATA" + unknown + SYSTEMTIME + name length + data size.
const ENTRY_FIXED_LEN: usize = 4 + 4 + 16 + 4 + 4;

#[derive(Debug, Error)]
pub enum XarcError {
    #[error("not an XARC archive (bad magic)")]
    BadMagic,
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("entry {index} at {offset:#x} does not start with \"DATA\"")]
    BadEntryMagic { index: usize, offset: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTime {
    pub year: u16,
    pub month: u16,
    pub day_of_week: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub millisecond: u16,
}

#[derive(Debug, Clone)]
pub struct XarcEntry {
    /// Name decoded from Shift-JIS (lossy if the bytes are invalid).
    pub name: String,
    /// Name bytes after nibble-swapping, before charset decoding.
    pub raw_name: Vec<u8>,
    pub unknown: [u8; 4],
    pub timestamp: SystemTime,
    /// Absolute offset of the entry ("DATA" tag).
    pub offset: usize,
    /// Byte range of the file data within the archive.
    pub data: Range<usize>,
    pub header_crc: u16,
    pub data_crc: u16,
    header_range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrcTarget {
    ArchiveHeader,
    EntryHeader(usize),
    EntryData(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcMismatch {
    pub target: CrcTarget,
    pub stored: u16,
    pub computed: u16,
}

/// A parsed archive borrowing the archive bytes (typically a memory map).
#[derive(Debug)]
pub struct Xarc<'a> {
    bytes: &'a [u8],
    header_crc: u16,
    header_len: usize,
    pub entries: Vec<XarcEntry>,
}

impl<'a> Xarc<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, XarcError> {
        let r = Reader(bytes);
        if r.slice(0, 4)? != MAGIC {
            return Err(XarcError::BadMagic);
        }
        let count = r.u32(4)? as usize;
        // Reject absurd counts before allocating: each offset takes 4 bytes.
        let header_len = 8usize
            .checked_add(count.checked_mul(4).ok_or(r.truncated(8, usize::MAX))?)
            .ok_or(r.truncated(8, usize::MAX))?;
        r.slice(0, header_len + 2)?;
        let header_crc = r.u16_be(header_len)?;

        let entries = (0..count)
            .map(|index| {
                let offset = r.u32(8 + index * 4)? as usize;
                parse_entry(&r, index, offset)
            })
            .collect::<Result<_, _>>()?;

        Ok(Self { bytes, header_crc, header_len, entries })
    }

    pub fn data(&self, entry: &XarcEntry) -> &'a [u8] {
        &self.bytes[entry.data.clone()]
    }

    /// Checks every stored CRC and returns the ones that don't match.
    pub fn verify(&self) -> Vec<CrcMismatch> {
        let mut checks = vec![(CrcTarget::ArchiveHeader, 0..self.header_len, self.header_crc)];
        for (i, e) in self.entries.iter().enumerate() {
            checks.push((CrcTarget::EntryHeader(i), e.header_range.clone(), e.header_crc));
            checks.push((CrcTarget::EntryData(i), e.data.clone(), e.data_crc));
        }
        checks
            .into_iter()
            .filter_map(|(target, range, stored)| {
                let computed = crc16_xmodem(&self.bytes[range]);
                (computed != stored).then_some(CrcMismatch { target, stored, computed })
            })
            .collect()
    }
}

fn parse_entry(r: &Reader, index: usize, offset: usize) -> Result<XarcEntry, XarcError> {
    if r.slice(offset, 4)? != ENTRY_MAGIC {
        return Err(XarcError::BadEntryMagic { index, offset });
    }
    let unknown = r.slice(offset + 4, 4)?.try_into().unwrap();
    let t = |i: usize| r.u16(offset + 8 + i * 2);
    let timestamp = SystemTime {
        year: t(0)?,
        month: t(1)?,
        day_of_week: t(2)?,
        day: t(3)?,
        hour: t(4)?,
        minute: t(5)?,
        second: t(6)?,
        millisecond: t(7)?,
    };
    let name_len = r.u32(offset + 24)? as usize;
    let data_len = r.u32(offset + 28)? as usize;

    let name_start = offset + ENTRY_FIXED_LEN;
    let raw_name: Vec<u8> = r.slice(name_start, name_len)?.iter().map(|b| b.rotate_left(4)).collect();
    let name_end = name_start + name_len;
    let header_crc = r.u16_be(name_end)?;

    let data_start = name_end + 2;
    r.slice(data_start, data_len)?;
    let data_end = data_start + data_len;
    let data_crc = r.u16_be(data_end)?;

    let (name, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_name);
    Ok(XarcEntry {
        name: name.into_owned(),
        raw_name,
        unknown,
        timestamp,
        offset,
        data: data_start..data_end,
        header_crc,
        data_crc,
        header_range: offset..name_end,
    })
}

/// CRC-16/XMODEM: polynomial 0x1021, initial value 0, no reflection.
pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an archive in the format described at the top of this module.
    fn build(files: &[(&[u8], &[u8])]) -> Vec<u8> {
        let header_len = 8 + 4 * files.len();
        let mut entries = Vec::new();
        let mut offsets = Vec::new();
        for (name, data) in files {
            offsets.push((header_len + 2 + entries.len()) as u32);
            let mut hdr = Vec::new();
            hdr.extend_from_slice(b"DATA");
            hdr.extend_from_slice(&[0x20, 0, 0, 0]);
            for v in [2004u16, 11, 1, 22, 8, 55, 20, 248] {
                hdr.extend_from_slice(&v.to_le_bytes());
            }
            hdr.extend_from_slice(&(name.len() as u32).to_le_bytes());
            hdr.extend_from_slice(&(data.len() as u32).to_le_bytes());
            hdr.extend(name.iter().map(|b| b.rotate_left(4)));
            entries.extend_from_slice(&hdr);
            entries.extend_from_slice(&crc16_xmodem(&hdr).to_be_bytes());
            entries.extend_from_slice(data);
            entries.extend_from_slice(&crc16_xmodem(data).to_be_bytes());
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"XARC");
        out.extend_from_slice(&(files.len() as u32).to_le_bytes());
        for o in offsets {
            out.extend_from_slice(&o.to_le_bytes());
        }
        let crc = crc16_xmodem(&out);
        out.extend_from_slice(&crc.to_be_bytes());
        out.extend_from_slice(&entries);
        out
    }

    #[test]
    fn crc16_xmodem_check_value() {
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
    }

    #[test]
    fn parses_entries() {
        let bytes = build(&[(b"pal0001.pal", b"RIFF...."), (b"Bar.004", b"GAF4")]);
        let arc = Xarc::parse(&bytes).unwrap();
        assert_eq!(arc.entries.len(), 2);
        assert_eq!(arc.entries[0].name, "pal0001.pal");
        assert_eq!(arc.data(&arc.entries[0]), b"RIFF....");
        assert_eq!(arc.entries[1].name, "Bar.004");
        assert_eq!(arc.data(&arc.entries[1]), b"GAF4");
        assert_eq!(arc.entries[0].timestamp.year, 2004);
        assert!(arc.verify().is_empty());
    }

    #[test]
    fn decodes_shift_jis_names() {
        let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("背景.png");
        let bytes = build(&[(&sjis, b"")]);
        assert_eq!(Xarc::parse(&bytes).unwrap().entries[0].name, "背景.png");
    }

    #[test]
    fn reports_crc_mismatch() {
        let mut bytes = build(&[(b"a.bin", b"hello")]);
        let arc = Xarc::parse(&bytes).unwrap();
        let pos = arc.entries[0].data.start;
        bytes[pos] ^= 0xff;
        let mismatches = Xarc::parse(&bytes).unwrap().verify();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].target, CrcTarget::EntryData(0));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(matches!(Xarc::parse(b"NOPE\0\0\0\0"), Err(XarcError::BadMagic)));
        let bytes = build(&[(b"a.bin", b"hello")]);
        for len in [3, 10, bytes.len() - 1] {
            assert!(Xarc::parse(&bytes[..len]).is_err(), "len {len}");
        }
        // Huge entry count must fail cleanly, not allocate.
        let mut huge = b"XARC".to_vec();
        huge.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(Xarc::parse(&huge), Err(XarcError::Truncated(_))));
    }
}
