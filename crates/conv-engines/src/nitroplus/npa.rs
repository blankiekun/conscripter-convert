//! NPA archives of 装甲悪鬼村正 (`nss.npa`, `cg.npa`, `sound.npa`, …).
//!
//! Layout (integers little-endian):
//!
//! ```text
//! header:
//!   "NPA"
//!   u32  version (1)
//!   u32  key1, u32 key2
//!   u8   compressed (zlib), u8 encrypted
//!   u32  entries, u32 folders, u32 files, u32, u32
//!   u32  index size; data starts at 0x29 + index size
//! entry:
//!   u32  name length, name (encrypted, Shift-JIS, `\` separators)
//!   u8   type (1 folder, 2 file)
//!   u32  file id, u32 offset (from the data start), u32 stored size,
//!   u32  size
//! ```
//!
//! Names: byte x of entry i is stored minus `0xFC·x − bytes(key1·key2) −
//! bytes(i)` (the sums of the four bytes of each 32-bit value), the
//! published Nitroplus scheme.
//!
//! Data: only the first `0x1000 + len(name)` bytes are encrypted, as
//! `b = inv[b] − k − i`, where `inv` is the inverse of a 256-byte
//! permutation in `Muramasa.exe` (file offset [`TABLE_OFFSET`]) and `k` is
//! the low byte of `((0x87654321 − Σname) · len(name) + key1·key2) · size`.
//! Found by using the zlib header `78 9c` as known plaintext; checked on
//! all 913 scripts.

use std::collections::HashMap;
use std::io::Read;

use thiserror::Error;

use crate::read::{Reader, Truncated};

/// Where `Muramasa.exe` keeps the data substitution table.
pub const TABLE_OFFSET: usize = 0x134830;
const HEADER_LEN: usize = 0x29;

#[derive(Debug, Error)]
pub enum NpaError {
    #[error("not an NPA archive (bad magic)")]
    BadMagic,
    #[error("the exe's table at {TABLE_OFFSET:#x} is not a permutation of 0-255")]
    BadTable,
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("{name}: {source}")]
    Inflate { name: String, source: std::io::Error },
    #[error("{name}: unpacked {got} bytes, the index says {want}")]
    Size { name: String, got: usize, want: usize },
}

/// The inverse of the exe's substitution table.
#[derive(Clone)]
pub struct Key([u8; 256]);

impl Key {
    /// Reads the table from `Muramasa.exe`.
    pub fn from_exe(exe: &[u8]) -> Result<Key, NpaError> {
        let table = Reader(exe).slice(TABLE_OFFSET, 256)?;
        let mut inv = [0u8; 256];
        let mut seen = [false; 256];
        for (i, &b) in table.iter().enumerate() {
            if seen[b as usize] {
                return Err(NpaError::BadTable);
            }
            seen[b as usize] = true;
            inv[b as usize] = i as u8;
        }
        Ok(Key(inv))
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// As stored, with `\` separators (`bg\bg001_空a_01.jpg`).
    pub name: String,
    raw_name: Vec<u8>,
    offset: usize,
    stored: usize,
    pub size: usize,
}

pub struct Npa<'a> {
    bytes: &'a [u8],
    key: Key,
    key12: u32,
    compressed: bool,
    encrypted: bool,
    data_start: usize,
    /// Files only, in index order.
    pub entries: Vec<Entry>,
    /// Lower-cased name with `/` separators → entry index.
    by_name: HashMap<String, usize>,
}

fn byte_sum(v: u32) -> u32 {
    v.to_le_bytes().iter().map(|&b| b as u32).sum()
}

/// The lookup form of a name: `/` separators, lower case (the scripts write
/// `cg/sys/adv/TextWindow.png` for `sys\adv\textwindow.png`).
pub fn normalize(name: &str) -> String {
    name.replace('\\', "/").to_lowercase()
}

impl<'a> Npa<'a> {
    pub fn parse(bytes: &'a [u8], key: Key) -> Result<Npa<'a>, NpaError> {
        let r = Reader(bytes);
        if r.slice(0, 3)? != b"NPA" {
            return Err(NpaError::BadMagic);
        }
        let key12 = r.u32(7)?.wrapping_mul(r.u32(11)?);
        let compressed = r.slice(15, 1)?[0] != 0;
        let encrypted = r.slice(16, 1)?[0] != 0;
        let total = r.u32(17)? as usize;
        let data_start = HEADER_LEN + r.u32(37)? as usize;
        let mut entries = Vec::new();
        let mut p = HEADER_LEN;
        for i in 0..total {
            let n = r.u32(p)? as usize;
            let mut raw = r.slice(p + 4, n)?.to_vec();
            for (x, b) in raw.iter_mut().enumerate() {
                let t = 0xFCu32.wrapping_mul(x as u32).wrapping_sub(byte_sum(key12)).wrapping_sub(byte_sum(i as u32));
                *b = b.wrapping_add(t as u8);
            }
            p += 4 + n;
            let kind = r.slice(p, 1)?[0];
            let offset = r.u32(p + 5)? as usize;
            let stored = r.u32(p + 9)? as usize;
            let size = r.u32(p + 13)? as usize;
            p += 17;
            if kind == 2 {
                let name = encoding_rs::SHIFT_JIS.decode(&raw).0.into_owned();
                entries.push(Entry { name, raw_name: raw, offset, stored, size });
            }
        }
        let by_name = entries.iter().enumerate().map(|(i, e)| (normalize(&e.name), i)).collect();
        Ok(Npa { bytes, key, key12, compressed, encrypted, data_start, entries, by_name })
    }

    /// Finds a file by name, ignoring case and separator style.
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.by_name.get(&normalize(name)).map(|&i| &self.entries[i])
    }

    /// The file's contents, decrypted and unpacked.
    pub fn data(&self, e: &Entry) -> Result<Vec<u8>, NpaError> {
        let mut b = Reader(self.bytes).slice(self.data_start + e.offset, e.stored)?.to_vec();
        if self.encrypted {
            let sum: u32 = e.raw_name.iter().map(|&c| c as u32).sum();
            let k = 0x87654321u32
                .wrapping_sub(sum)
                .wrapping_mul(e.raw_name.len() as u32)
                .wrapping_add(self.key12)
                .wrapping_mul(e.size as u32) as u8;
            for (i, c) in b.iter_mut().take(0x1000 + e.raw_name.len()).enumerate() {
                *c = self.key.0[*c as usize].wrapping_sub(k).wrapping_sub(i as u8);
            }
        }
        if self.compressed {
            let mut out = Vec::with_capacity(e.size);
            flate2::read::ZlibDecoder::new(&b[..])
                .read_to_end(&mut out)
                .map_err(|source| NpaError::Inflate { name: e.name.clone(), source })?;
            b = out;
        }
        if b.len() != e.size {
            return Err(NpaError::Size { name: e.name.clone(), got: b.len(), want: e.size });
        }
        Ok(b)
    }
}
