//! `HD/CDFiles/0.cd`: the label symbol table for all numbered scripts
//! (最果てのイマ COMPLETE).
//!
//! The file is encrypted with [`decrypt`] (taken from `Farthest2015.exe` at
//! 0x4bcb20). Decrypted layout (integers little-endian, CRCs are big-endian
//! CRC-16/XMODEM like in XARC archives):
//!
//! ```text
//! u32           script count (n)
//! u32[n]        absolute offset of each script's section
//! u16 BE        CRC of the above
//! section (one per script, in script id order):
//!   u32         record count (m)
//!   u32[m]      record offsets, relative to the section start
//!   u16 BE      CRC of the section header
//!   record:
//!     u32       name length in bytes (including the trailing NUL)
//!     [u8]      label name, Shift-JIS, NUL-terminated
//!     u32       code offset (matches the script's own label table)
//!     u32[3]    unknown (look like source line numbers)
//!     u16 BE    CRC of the record
//! ```

use thiserror::Error;

use crate::read::{Reader, Truncated};
use crate::xuse::xarc::crc16_xmodem;

#[derive(Debug, Error)]
pub enum SymbolsError {
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("CRC mismatch in {what}: stored {stored:04x}, computed {computed:04x}")]
    Crc { what: String, stored: u16, computed: u16 },
}

/// Decrypts in place. The keystream starts at 0x1b and after each byte
/// becomes `((k ^ 0xa1) >> 1) + (index & 0xff)`, so it settles into a
/// 256-byte cycle.
pub fn decrypt(buf: &mut [u8]) {
    let mut k: u8 = 0x1b;
    for (i, b) in buf.iter_mut().enumerate() {
        *b ^= k;
        k = ((k ^ 0xa1) >> 1).wrapping_add(i as u8);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// Byte offset into the script's code (divide by 12 for the index).
    pub code_offset: u32,
    pub unknown: [u32; 3],
}

#[derive(Debug)]
pub struct SymbolTable {
    /// Symbols of script `i` at index `i`.
    pub scripts: Vec<Vec<Symbol>>,
}

impl SymbolTable {
    /// Parses an already decrypted `0.cd`, checking every CRC.
    pub fn parse(bytes: &[u8]) -> Result<Self, SymbolsError> {
        let r = Reader(bytes);
        let count = r.u32(0)? as usize;
        let header_len = count
            .checked_mul(4)
            .and_then(|n| n.checked_add(4))
            .ok_or(r.truncated(0, usize::MAX))?;
        check_crc(&r, 0, header_len, || "file header".into())?;

        let section_offsets: Vec<usize> =
            (0..count).map(|i| Ok(r.u32(4 + i * 4)? as usize)).collect::<Result<_, Truncated>>()?;
        let scripts = section_offsets
            .iter()
            .enumerate()
            .map(|(script, &start)| parse_section(&r, script, start))
            .collect::<Result<_, _>>()?;
        Ok(Self { scripts })
    }
}

fn parse_section(r: &Reader, script: usize, start: usize) -> Result<Vec<Symbol>, SymbolsError> {
    let count = r.u32(start)? as usize;
    let header_len = count
        .checked_mul(4)
        .and_then(|n| n.checked_add(4))
        .ok_or(r.truncated(start, usize::MAX))?;
    check_crc(r, start, header_len, || format!("section header of script {script}"))?;

    (0..count)
        .map(|i| {
            let rec = start + r.u32(start + 4 + i * 4)? as usize;
            let name_len = r.u32(rec)? as usize;
            let raw = r.slice(rec + 4, name_len)?;
            let raw = raw.split(|&b| b == 0).next().unwrap_or_default();
            let (name, _, _) = encoding_rs::SHIFT_JIS.decode(raw);
            let fields = rec + 4 + name_len;
            let symbol = Symbol {
                name: name.into_owned(),
                code_offset: r.u32(fields)?,
                unknown: [r.u32(fields + 4)?, r.u32(fields + 8)?, r.u32(fields + 12)?],
            };
            check_crc(r, rec, fields + 16 - rec, || format!("symbol {} of script {script}", symbol.name))?;
            Ok(symbol)
        })
        .collect()
}

fn check_crc(r: &Reader, start: usize, len: usize, what: impl FnOnce() -> String) -> Result<(), SymbolsError> {
    let computed = crc16_xmodem(r.slice(start, len)?);
    let stored = r.u16_be(start + len)?;
    if computed != stored {
        return Err(SymbolsError::Crc { what: what(), stored, computed });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_crc(mut v: Vec<u8>) -> Vec<u8> {
        let crc = crc16_xmodem(&v);
        v.extend_from_slice(&crc.to_be_bytes());
        v
    }

    fn record(name: &str, code_offset: u32) -> Vec<u8> {
        let mut raw = encoding_rs::SHIFT_JIS.encode(name).0.into_owned();
        raw.push(0);
        let mut v = (raw.len() as u32).to_le_bytes().to_vec();
        v.extend(raw);
        for x in [code_offset, 1, 2, 3] {
            v.extend_from_slice(&x.to_le_bytes());
        }
        with_crc(v)
    }

    fn section(records: &[Vec<u8>]) -> Vec<u8> {
        let header_len = 4 + 4 * records.len() + 2;
        let mut header = (records.len() as u32).to_le_bytes().to_vec();
        let mut at = header_len;
        for rec in records {
            header.extend_from_slice(&(at as u32).to_le_bytes());
            at += rec.len();
        }
        let mut out = with_crc(header);
        for rec in records {
            out.extend_from_slice(rec);
        }
        out
    }

    fn table(sections: &[Vec<u8>]) -> Vec<u8> {
        let header_len = 4 + 4 * sections.len() + 2;
        let mut header = (sections.len() as u32).to_le_bytes().to_vec();
        let mut at = header_len;
        for s in sections {
            header.extend_from_slice(&(at as u32).to_le_bytes());
            at += s.len();
        }
        let mut out = with_crc(header);
        for s in sections {
            out.extend_from_slice(s);
        }
        out
    }

    #[test]
    fn decrypt_round_trips_and_matches_known_keystream() {
        let mut buf = [0u8; 4];
        decrypt(&mut buf);
        // k0 = 0x1b; k1 = ((0x1b ^ 0xa1) >> 1) + 0 = 0x5d; k2 = ((0x5d ^ 0xa1) >> 1) + 1 = 0x7f
        assert_eq!(&buf[..3], &[0x1b, 0x5d, 0x7f]);
        let mut data = b"symbol table".to_vec();
        decrypt(&mut data);
        decrypt(&mut data);
        assert_eq!(data, b"symbol table");
    }

    #[test]
    fn parses_symbols() {
        let bytes = table(&[
            section(&[record("_begin", 0), record("テスト場面", 96)]),
            section(&[record("_begin", 0)]),
        ]);
        let t = SymbolTable::parse(&bytes).unwrap();
        assert_eq!(t.scripts.len(), 2);
        assert_eq!(t.scripts[0][1], Symbol { name: "テスト場面".into(), code_offset: 96, unknown: [1, 2, 3] });
        assert_eq!(t.scripts[1][0].name, "_begin");
    }

    #[test]
    fn rejects_bad_crc_and_truncation() {
        let mut bytes = table(&[section(&[record("_begin", 0)])]);
        let n = bytes.len();
        bytes[n - 3] ^= 1; // inside the last record's fields
        assert!(matches!(SymbolTable::parse(&bytes), Err(SymbolsError::Crc { .. })));
        assert!(SymbolTable::parse(&bytes[..n - 1]).is_err());
        let mut huge = bytes.clone();
        huge[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(SymbolTable::parse(&huge).is_err());
    }
}
