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
//!
//! 続・殺戮のジャンゴ (`Django.exe`, 2008) uses the same scheme with
//! another table, which its exe doesn't hold as plain bytes:
//! [`DJANGO_TABLE`] was recovered from the fixed bytes of the Ogg headers
//! in `sound.npa` and `voice.npa` (`research/nitroplus/npa_table.py`; no
//! contradiction over 6,600 files).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;

use thiserror::Error;

use crate::read::{Reader, Truncated};

/// Where `Muramasa.exe` keeps the data substitution table.
pub const TABLE_OFFSET: usize = 0x134830;
const HEADER_LEN: usize = 0x29;

/// 続・殺戮のジャンゴ's substitution table (the bytes `Muramasa.exe` keeps
/// at [`TABLE_OFFSET`]).
pub const DJANGO_TABLE: [u8; 256] = [
    0xc9, 0x27, 0x0e, 0xa3, 0xd8, 0x7f, 0x5b, 0x8f, 0x16, 0x49, 0xaa, 0xb2, 0x18, 0xa7, 0xb8, 0x41,
    0xdb, 0x48, 0xca, 0xde, 0xae, 0xcd, 0x13, 0x1f, 0x15, 0x2e, 0x39, 0xf5, 0x4e, 0xdd, 0x33, 0xe4,
    0x4c, 0x98, 0x36, 0xb4, 0x3f, 0x09, 0x83, 0xfd, 0x32, 0xba, 0x14, 0x30, 0x7a, 0x63, 0x0f, 0x88,
    0x95, 0x61, 0xcc, 0x8b, 0xef, 0xda, 0xe5, 0x2c, 0xdc, 0x12, 0x1a, 0x67, 0x23, 0x50, 0xb9, 0x56,
    0x7e, 0x6d, 0x66, 0x90, 0x3c, 0xb3, 0x0b, 0xe2, 0x91, 0x70, 0xa8, 0xdf, 0x44, 0xc4, 0xd1, 0xc3,
    0x5c, 0x10, 0x06, 0xe7, 0x54, 0x40, 0x43, 0x72, 0x38, 0xbc, 0xe3, 0x07, 0xfa, 0x34, 0xf4, 0x01,
    0xf7, 0x74, 0xa9, 0x4d, 0x42, 0xa5, 0x85, 0x35, 0x79, 0xd2, 0x76, 0x97, 0x45, 0x4f, 0x02, 0xa4,
    0xb0, 0xee, 0x51, 0x73, 0x69, 0x9e, 0x94, 0x47, 0x77, 0x29, 0xd9, 0x64, 0x11, 0xeb, 0x08, 0x5a,
    0x20, 0x62, 0x9a, 0x6b, 0x9c, 0x75, 0x22, 0x87, 0xab, 0x78, 0x53, 0xc8, 0x5d, 0xad, 0x37, 0xac,
    0xcb, 0xb7, 0x0d, 0xed, 0x86, 0x55, 0xff, 0x19, 0x57, 0xd7, 0xd5, 0x60, 0xc6, 0x3d, 0x2a, 0xf2,
    0x6c, 0xe1, 0xc0, 0x65, 0x84, 0xc5, 0xe0, 0x3e, 0x7d, 0x28, 0xb6, 0xaf, 0x1c, 0x9b, 0xea, 0xc1,
    0x1e, 0x26, 0x59, 0x2b, 0x5f, 0x7b, 0xe8, 0x8d, 0x52, 0x7c, 0xf8, 0x82, 0x0c, 0xf9, 0xcf, 0x81,
    0xb5, 0xe6, 0x31, 0x93, 0x46, 0x5e, 0x1d, 0x1b, 0x4b, 0x71, 0xd6, 0x92, 0x3a, 0xa6, 0x8c, 0xe9,
    0x9d, 0xbb, 0x6e, 0xf0, 0x99, 0xce, 0x21, 0x0a, 0xd0, 0xf6, 0xfe, 0xa2, 0x8a, 0x96, 0x2d, 0x00,
    0x6a, 0xbf, 0xa1, 0xc7, 0x8e, 0xfb, 0xd4, 0x2f, 0x80, 0x58, 0x4a, 0x17, 0x3b, 0xb1, 0x6f, 0x05,
    0xa0, 0x9f, 0xd3, 0xfc, 0xc2, 0x04, 0x68, 0x03, 0xf3, 0x25, 0xbe, 0x24, 0xf1, 0xbd, 0x89, 0xec,
];

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
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The inverse of the exe's substitution table.
#[derive(Clone)]
pub struct Key([u8; 256]);

impl Key {
    /// The game's table: [`DJANGO_TABLE`] for `Django.exe`, otherwise
    /// the one `Muramasa.exe` keeps at [`TABLE_OFFSET`].
    pub fn for_exe(name: &str, exe: &[u8]) -> Result<Key, NpaError> {
        if name.eq_ignore_ascii_case("Django.exe") {
            return Key::from_table(&DJANGO_TABLE);
        }
        Key::from_exe(exe)
    }

    /// Reads the table from `Muramasa.exe`.
    pub fn from_exe(exe: &[u8]) -> Result<Key, NpaError> {
        Key::from_table(Reader(exe).slice(TABLE_OFFSET, 256)?)
    }

    /// The decryption table in hex, as CONscripter's `cons_archives.txt`
    /// takes it.
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn from_table(table: &[u8]) -> Result<Key, NpaError> {
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

/// Where the archive's bytes are: in memory, or read from the file entry
/// by entry ([`Npa::open`]), which keeps the gigabytes of `cg.npa` and
/// `voice.npa` out of the converter's memory.
enum Source<'a> {
    Bytes(&'a [u8]),
    File(Mutex<File>),
}

pub struct Npa<'a> {
    source: Source<'a>,
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
        Npa::parse_index(bytes, key, Source::Bytes(bytes))
    }

    /// Reads the header and index from `file`; [`Npa::data`] then reads
    /// each entry from the file when asked.
    pub fn open(mut file: File, key: Key) -> Result<Npa<'static>, NpaError> {
        let mut index = vec![0; HEADER_LEN];
        file.read_exact(&mut index)?;
        let index_len = Reader(&index).u32(37)? as usize;
        crate::read::in_file(&file, HEADER_LEN as u64, index_len)?;
        index.resize(HEADER_LEN + index_len, 0);
        file.read_exact(&mut index[HEADER_LEN..])?;
        Npa::parse_index(&index, key, Source::File(Mutex::new(file)))
    }

    /// Parses the header and index at the start of `bytes`.
    fn parse_index(bytes: &[u8], key: Key, source: Source<'a>) -> Result<Npa<'a>, NpaError> {
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
        Ok(Npa { source, key, key12, compressed, encrypted, data_start, entries, by_name })
    }

    /// Finds a file by name, ignoring case and separator style.
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.by_name.get(&normalize(name)).map(|&i| &self.entries[i])
    }

    /// The file's contents, decrypted and unpacked.
    pub fn data(&self, e: &Entry) -> Result<Vec<u8>, NpaError> {
        let mut b = match &self.source {
            Source::Bytes(bytes) => Reader(bytes).slice(self.data_start + e.offset, e.stored)?.to_vec(),
            Source::File(file) => {
                let mut file = file.lock().unwrap_or_else(|e| e.into_inner());
                let at = (self.data_start + e.offset) as u64;
                crate::read::in_file(&file, at, e.stored)?;
                file.seek(SeekFrom::Start(at))?;
                let mut b = vec![0; e.stored];
                file.read_exact(&mut b)?;
                b
            }
        };
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
            // not sized from the index: checked after
            let mut out = Vec::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn django_table_is_a_permutation() {
        assert!(Key::for_exe("Django.exe", &[]).is_ok());
    }

    #[test]
    fn an_index_past_the_end_is_refused_unread() {
        let path = std::env::temp_dir().join(format!("conv-npa-test-{}", std::process::id()));
        let mut head = vec![0; HEADER_LEN];
        head[..3].copy_from_slice(b"NPA");
        head[37..41].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
        std::fs::write(&path, &head).unwrap();
        let r = Npa::open(File::open(&path).unwrap(), Key::for_exe("Django.exe", &[]).unwrap());
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(r, Err(NpaError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof));
    }
}
