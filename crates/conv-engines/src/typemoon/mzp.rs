//! MZP archives (magic `mrgd00`): self-contained, used for `allscr.mrg`,
//! `script_text.mrg` and some entries of the MRG archives.
//!
//! u16 entry count at 6, then 8-byte entries: u16 sector (0x800 bytes)
//! and u16 byte of the entry's start, counted from the end of the table;
//! u16 size in sectors (rounded up) and the size's low 16 bits.

use super::FormatError;
use crate::read::Reader;

/// The entries' data, in order.
pub fn parse(data: &[u8]) -> Result<Vec<&[u8]>, FormatError> {
    if !data.starts_with(b"mrgd00") {
        return Err(FormatError::BadMagic("MZP"));
    }
    let r = Reader(data);
    let count = usize::from(r.u16(6)?);
    let base = 8 + 8 * count;
    (0..count)
        .map(|i| {
            let at = 8 + 8 * i;
            let start = usize::from(r.u16(at)?) * 0x800 + usize::from(r.u16(at + 2)?);
            Ok(r.slice(base + start, size(r.u16(at + 4)?, r.u16(at + 6)?))?)
        })
        .collect()
}

/// The size whose low 16 bits are `low`, within the last of `sectors`.
fn size(sectors: u16, low: u16) -> usize {
    let upper = usize::from(sectors) * 0x800;
    let size = (upper & !0xFFFF) | usize::from(low);
    // mangetsu and deepLuna stop at the first line, which is 64 KiB too
    // big when rounding up to the sector crossed a 64 KiB boundary.
    if size > upper { size - 0x10000 } else { size }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        for n in [0usize, 1, 0x800, 0xFFFF, 0x10000, 0x10001, 0x1F000, 0x1FFF1, 0x2_3456] {
            assert_eq!(size(n.div_ceil(0x800) as u16, n as u16), n, "{n:#x}");
        }
    }

    #[test]
    fn entries() {
        let mut mzp = b"mrgd00".to_vec();
        mzp.extend(2u16.to_le_bytes());
        for [sector, byte, sectors, low] in [[0u16, 0, 1, 3], [0, 16, 1, 2]] {
            for v in [sector, byte, sectors, low] {
                mzp.extend(v.to_le_bytes());
            }
        }
        mzp.extend(b"abc");
        mzp.resize(8 + 16 + 16, 0xFF);
        mzp.extend(b"de");
        assert_eq!(parse(&mzp).unwrap(), [&b"abc"[..], b"de"]);
        assert!(parse(&mzp[..mzp.len() - 1]).is_err());
    }
}
