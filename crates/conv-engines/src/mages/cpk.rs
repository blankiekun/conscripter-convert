//! CRI archives of 俺たちに翼はない (PS Vita): CPK (`*.cpk`, and the `.AWB`
//! sound banks, which are CPKs without names) and AFS2 (`VOICE.AWB`).
//!
//! CPK (integers big-endian): chunks of a 16-byte header (`"CPK "`,
//! `"TOC "`, `"ITOC"`; u64 little-endian size at 8) followed by an `@UTF`
//! table. The `CpkHeader` table at 0 gives `TocOffset` and
//! `ContentOffset`; with a TOC, file `i` is at `min(TocOffset,
//! ContentOffset) + FileOffset` and has a `FileName` and an `ID`. Without
//! one (`ItocOffset` only), the ITOC's `DataL`/`DataH` tables give each
//! `ID`'s size, and the files follow `ContentOffset` in ID order, each
//! start rounded up to `Align`.
//!
//! `@UTF` table: u32 size, then (from offset 8) u32 rows, strings and
//! data offsets, u32 name, u16 columns, u16 row length, u32 rows. A
//! column is u8 flags (storage `0x10` none, `0x30` constant with its
//! value inline, `0x50` per row; type in the low nibble) and u32 name.
//!
//! AFS2 (little-endian): `"AFS2"`, u8 version, u8 offset size, u16 ID
//! size, u32 count, u32 alignment (low 16 bits), IDs, then `count + 1`
//! offsets; each file starts at its offset rounded up to the alignment.
//!
//! None of the game's files are CRILAYLA-compressed; such a file is an
//! error here.

use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CpkError {
    #[error("not a CPK or AFS2 archive")]
    BadMagic,
    #[error("bad @UTF table at {0:#x}")]
    BadTable(usize),
    #[error("truncated at {0:#x}")]
    Truncated(usize),
    #[error("column {0} missing")]
    Missing(&'static str),
    #[error("{0}: CRILAYLA-compressed files are not supported")]
    Compressed(String),
}

/// A value in a `@UTF` table.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Data(Vec<u8>),
    None,
}

impl Value {
    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }
}

pub type Row = HashMap<String, Value>;

fn be(data: &[u8], at: usize, n: usize) -> Result<u64, CpkError> {
    let b = data.get(at..at + n).ok_or(CpkError::Truncated(at))?;
    Ok(b.iter().fold(0u64, |v, &x| v << 8 | x as u64))
}

fn cstr(data: &[u8], at: usize) -> Result<String, CpkError> {
    let rest = data.get(at..).ok_or(CpkError::Truncated(at))?;
    let end = rest.iter().position(|&b| b == 0).ok_or(CpkError::Truncated(at))?;
    Ok(encoding_rs::SHIFT_JIS.decode(&rest[..end]).0.into_owned())
}

/// Parses a `@UTF` table: its name and rows.
pub fn utf_table(data: &[u8]) -> Result<(String, Vec<Row>), CpkError> {
    if data.get(..4) != Some(b"@UTF") {
        return Err(CpkError::BadTable(0));
    }
    let base = 8;
    let rows_off = base + be(data, 8, 4)? as usize;
    let str_off = base + be(data, 12, 4)? as usize;
    let data_off = base + be(data, 16, 4)? as usize;
    let name = cstr(data, str_off + be(data, 20, 4)? as usize)?;
    let ncols = be(data, 24, 2)? as usize;
    let row_len = be(data, 26, 2)? as usize;
    let nrows = be(data, 28, 4)? as usize;

    let value = |t: u8, at: usize| -> Result<(Value, usize), CpkError> {
        Ok(match t {
            0 | 2 | 4 | 6 => {
                let n = 1 << (t / 2);
                (Value::Int(be(data, at, n)? as i64), n)
            }
            1 | 3 | 5 | 7 => {
                let n = 1 << (t / 2);
                let v = be(data, at, n)?;
                let shift = 64 - 8 * n as u32;
                (Value::Int(((v << shift) as i64) >> shift), n)
            }
            8 => (Value::Float(f32::from_bits(be(data, at, 4)? as u32) as f64), 4),
            9 => (Value::Float(f64::from_bits(be(data, at, 8)?)), 8),
            0xA => (Value::Str(cstr(data, str_off + be(data, at, 4)? as usize)?), 4),
            0xB => {
                let off = data_off + be(data, at, 4)? as usize;
                let len = be(data, at + 4, 4)? as usize;
                let bytes = data.get(off..off + len).ok_or(CpkError::Truncated(off))?;
                (Value::Data(bytes.to_vec()), 8)
            }
            _ => return Err(CpkError::BadTable(at)),
        })
    };

    let mut cols = Vec::with_capacity(ncols);
    let mut at = 32;
    for _ in 0..ncols {
        let flags = *data.get(at).ok_or(CpkError::Truncated(at))?;
        let name = cstr(data, str_off + be(data, at + 1, 4)? as usize)?;
        at += 5;
        let (storage, t) = (flags & 0xF0, flags & 0x0F);
        let constant = if storage == 0x30 {
            let (v, n) = value(t, at)?;
            at += n;
            Some(v)
        } else {
            None
        };
        cols.push((name, storage, t, constant));
    }
    let mut rows = Vec::with_capacity(nrows);
    for r in 0..nrows {
        let mut at = rows_off + r * row_len;
        let mut row = Row::new();
        for (name, storage, t, constant) in &cols {
            let v = match storage {
                0x50 => {
                    let (v, n) = value(*t, at)?;
                    at += n;
                    v
                }
                0x30 => constant.clone().unwrap_or(Value::None),
                _ => Value::None,
            };
            row.insert(name.clone(), v);
        }
        rows.push(row);
    }
    Ok((name, rows))
}

/// The `@UTF` table of the chunk at `at`.
fn chunk(data: &[u8], at: usize) -> Result<Vec<Row>, CpkError> {
    let size = data.get(at + 8..at + 16).ok_or(CpkError::Truncated(at))?;
    let size = u64::from_le_bytes(size.try_into().unwrap()) as usize;
    let table = data.get(at + 16..at + 16 + size).ok_or(CpkError::Truncated(at))?;
    Ok(utf_table(table)?.1)
}

/// A file in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    /// The file name, or the ID in five digits when the archive has none.
    pub name: String,
    pub offset: usize,
    pub size: usize,
}

/// A CPK or AFS2 archive's index.
#[derive(Debug, Clone)]
pub struct Archive {
    /// In ID order.
    pub entries: Vec<Entry>,
}

fn int(row: &Row, key: &'static str) -> Result<i64, CpkError> {
    row.get(key).and_then(Value::int).ok_or(CpkError::Missing(key))
}

impl Archive {
    pub fn parse(data: &[u8]) -> Result<Archive, CpkError> {
        let mut entries = match data.get(..4) {
            Some(b"CPK ") => Self::cpk(data)?,
            Some(b"AFS2") => Self::afs2(data)?,
            _ => return Err(CpkError::BadMagic),
        };
        entries.sort_by_key(|e| e.id);
        Ok(Archive { entries })
    }

    fn cpk(data: &[u8]) -> Result<Vec<Entry>, CpkError> {
        let header = chunk(data, 0)?;
        let h = header.first().ok_or(CpkError::BadTable(0))?;
        let content = int(h, "ContentOffset")? as usize;
        if let Ok(toc) = int(h, "TocOffset") {
            let toc = toc as usize;
            let base = toc.min(content);
            return chunk(data, toc)?
                .iter()
                .map(|r| {
                    let dir = match r.get("DirName") {
                        Some(Value::Str(d)) if !d.is_empty() => format!("{d}/"),
                        _ => String::new(),
                    };
                    let name = match r.get("FileName") {
                        Some(Value::Str(n)) => format!("{dir}{n}"),
                        _ => return Err(CpkError::Missing("FileName")),
                    };
                    let size = int(r, "FileSize")? as usize;
                    if int(r, "ExtractSize")? as usize != size {
                        return Err(CpkError::Compressed(name));
                    }
                    Ok(Entry { id: int(r, "ID")? as u32, name, offset: base + int(r, "FileOffset")? as usize, size })
                })
                .collect();
        }
        let itoc = chunk(data, int(h, "ItocOffset")? as usize)?;
        let itoc = itoc.first().ok_or(CpkError::Missing("ItocOffset"))?;
        let mut sized = Vec::new();
        for key in ["DataL", "DataH"] {
            if let Some(Value::Data(t)) = itoc.get(key) {
                for r in utf_table(t)?.1 {
                    let size = int(&r, "FileSize")? as usize;
                    if int(&r, "ExtractSize")? as usize != size {
                        return Err(CpkError::Compressed(format!("{:05}", int(&r, "ID")?)));
                    }
                    sized.push((int(&r, "ID")? as u32, size));
                }
            }
        }
        sized.sort_unstable();
        let align = int(h, "Align").unwrap_or(1).max(1) as usize;
        let mut offset = content;
        Ok(sized
            .into_iter()
            .map(|(id, size)| {
                let e = Entry { id, name: format!("{id:05}"), offset, size };
                offset += size.div_ceil(align) * align;
                e
            })
            .collect())
    }

    fn afs2(data: &[u8]) -> Result<Vec<Entry>, CpkError> {
        let le = |at: usize, n: usize| -> Result<usize, CpkError> {
            let b = data.get(at..at + n).ok_or(CpkError::Truncated(at))?;
            Ok(b.iter().rev().fold(0usize, |v, &x| v << 8 | x as usize))
        };
        let osize = le(5, 1)?;
        let isize = le(6, 2)?;
        let count = le(8, 4)?;
        let align = (le(12, 4)? & 0xFFFF).max(1);
        let ids = 16;
        let offs = ids + count * isize;
        (0..count)
            .map(|i| {
                let id = le(ids + i * isize, isize)? as u32;
                let start = le(offs + i * osize, osize)?.div_ceil(align) * align;
                let end = le(offs + (i + 1) * osize, osize)?;
                Ok(Entry { id, name: format!("{id:05}"), offset: start, size: end.saturating_sub(start) })
            })
            .collect()
    }

    pub fn by_id(&self, id: u32) -> Option<&Entry> {
        self.entries.binary_search_by_key(&id, |e| e.id).ok().map(|i| &self.entries[i])
    }

    /// The bytes of `entry` in the archive `data`.
    pub fn read<'a>(&self, data: &'a [u8], entry: &Entry) -> Result<&'a [u8], CpkError> {
        data.get(entry.offset..entry.offset + entry.size).ok_or(CpkError::Truncated(entry.offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `@UTF` table with one constant and two per-row columns.
    fn table() -> Vec<u8> {
        let strings = b"<NULL>\0T\0ID\0FileName\0Align\0a.png\0b.png\0";
        let mut cols = Vec::new();
        cols.extend([0x30 | 4]);
        cols.extend(21u32.to_be_bytes()); // "Align"
        cols.extend(32u32.to_be_bytes());
        cols.extend([0x50 | 4]);
        cols.extend(9u32.to_be_bytes()); // "ID"
        cols.extend([0x50 | 0xA]);
        cols.extend(12u32.to_be_bytes()); // "FileName"
        let rows: Vec<u8> = [(7u32, 27u32), (3, 33)].iter().flat_map(|(id, s)| [id.to_be_bytes(), s.to_be_bytes()].concat()).collect();
        let rows_off = 24 + cols.len();
        let str_off = rows_off + rows.len();
        let mut t = b"@UTF".to_vec();
        t.extend(0u32.to_be_bytes());
        for v in [rows_off as u32, str_off as u32, (str_off + strings.len()) as u32, 7] {
            t.extend(v.to_be_bytes());
        }
        t.extend(3u16.to_be_bytes());
        t.extend(8u16.to_be_bytes());
        t.extend(2u32.to_be_bytes());
        t.extend(cols);
        t.extend(rows);
        t.extend(strings);
        t
    }

    #[test]
    fn utf_rows() {
        let (name, rows) = utf_table(&table()).unwrap();
        assert_eq!(name, "T");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["Align"], Value::Int(32));
        assert_eq!(rows[1]["ID"], Value::Int(3));
        assert_eq!(rows[1]["FileName"], Value::Str("b.png".into()));
    }

    #[test]
    fn afs2_entries() {
        let mut d = b"AFS2\x01\x04\x02\x00".to_vec();
        d.extend(2u32.to_le_bytes());
        d.extend(0x20u32.to_le_bytes());
        d.extend([5u8, 0, 9, 0]);
        for o in [0x24u32, 0x45, 0x50] {
            d.extend(o.to_le_bytes());
        }
        let a = Archive::parse(&d).unwrap();
        assert_eq!(a.entries[0], Entry { id: 5, name: "00005".into(), offset: 0x40, size: 5 });
        assert_eq!(a.by_id(9).unwrap().offset, 0x60);
        assert_eq!(a.by_id(9).unwrap().size, 0);
    }
}
