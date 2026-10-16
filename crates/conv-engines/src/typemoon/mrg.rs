//! MRG archives: a `.hed` index, the `.mrg` data and, for most, a `.nam`
//! of names.
//!
//! `.hed`: 8-byte entries, u32 offset in 0x800-byte sectors, u16 size in
//! sectors, u16 unpacked size in sectors (the size once its MZX/NXGX
//! wrapper is off); an offset of `0xFFFFFFFF` ends the table. An entry's
//! data is padded to whole sectors (the formats inside know their size).
//! `.nam`: one 32-byte name per entry, NUL-padded, some ending in CRLF.

use super::FormatError;
use crate::read::Reader;

pub const SECTOR: u64 = 0x800;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: Option<String>,
    /// Bytes, in the `.mrg`.
    pub offset: u64,
    /// Bytes, padded to whole sectors.
    pub size: u64,
    pub unpacked_sectors: u16,
}

/// The entries of a `.hed`, named from the `.nam` if there is one.
pub fn parse(hed: &[u8], nam: Option<&[u8]>) -> Result<Vec<Entry>, FormatError> {
    if !hed.len().is_multiple_of(8) {
        return Err(FormatError::Bad(format!(".hed is {} bytes, not a multiple of 8", hed.len())));
    }
    let names = nam.map(names).unwrap_or_default();
    let r = Reader(hed);
    let mut entries = Vec::new();
    for at in (0..hed.len()).step_by(8) {
        let offset = r.u32(at)?;
        if offset == u32::MAX {
            break;
        }
        entries.push(Entry {
            name: names.get(entries.len()).cloned(),
            offset: u64::from(offset) * SECTOR,
            size: u64::from(r.u16(at + 4)?) * SECTOR,
            unpacked_sectors: r.u16(at + 6)?,
        });
    }
    Ok(entries)
}

/// A table of 32-byte names (`.nam`, and the name entry of `allscr.mrg`).
pub fn names(table: &[u8]) -> Vec<String> {
    table
        .as_chunks::<32>()
        .0
        .iter()
        .map(|c| {
            let end = c.iter().position(|&b| b == 0).unwrap_or(c.len());
            String::from_utf8_lossy(&c[..end]).trim().to_string()
        })
        .collect()
}

/// An entry's data in the `.mrg`.
pub fn data<'a>(mrg: &'a [u8], entry: &Entry) -> Result<&'a [u8], FormatError> {
    let (offset, size) = (entry.offset as usize, entry.size as usize);
    Ok(Reader(mrg).slice(offset, size)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hed_entries_end_at_the_marker() {
        let mut hed = Vec::new();
        for (offset, size, unpacked) in [(0u32, 1u16, 5u16), (1, 2, 2)] {
            hed.extend(offset.to_le_bytes());
            hed.extend(size.to_le_bytes());
            hed.extend(unpacked.to_le_bytes());
        }
        hed.extend([0xFF; 16]);
        let mut nam = b"DUMMY.NXZ".to_vec();
        nam.resize(30, 0);
        nam.extend(b"\r\n");
        let entries = parse(&hed, Some(&nam)).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name.as_deref(), Some("DUMMY.NXZ"));
        assert_eq!(entries[1].name, None);
        assert_eq!((entries[1].offset, entries[1].size, entries[0].unpacked_sectors), (0x800, 0x1000, 5));
        assert!(data(&[0; 0x1000], &entries[1]).is_err());
    }
}
