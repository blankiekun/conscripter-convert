//! NPK3 archives of Full Metal Daemon Muramasa's English release (JAST USA
//! 2021, Nitroplus's "Diesel" engine: `script.npk`, `cg.npk`, …).
//!
//! Layout (integers little-endian):
//!
//! ```text
//! header:
//!   "NPK3", u32 version (1)
//!   16 bytes  IV
//!   u32  entries, u32 index size; the index follows, AES-256-CBC
//! entry:
//!   u8   flag, u16 name length, name (UTF-8, `/` separators)
//!   u32  size, 32-byte hash, u32 segments
//!   segment: u64 offset, u32 stored size (16-aligned), u32 packed size,
//!            u32 size
//! ```
//!
//! A segment is AES-256-CBC with the same key and the archive's IV, then
//! zstd when its packed size is below its size (otherwise stored as is).
//! The key is [`KEY`]: `muramasa_en.exe` builds it from dword immediates
//! (0x401c28) and hands it to the archive opener (0x54cf40), so a scan of
//! the exe's bytes doesn't find it. `research/nitroplus/npk.py` is the
//! same reader in Python.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;

use aes::cipher::{BlockDecrypt, KeyInit};
use aes::{Aes256, Block};
use thiserror::Error;

use crate::read::{Reader, Truncated};

pub const KEY: [u8; 32] = [
    0x51, 0x83, 0x1a, 0x9e, 0x69, 0xef, 0xa8, 0x5e, 0xe6, 0xc5, 0x51, 0xc2, 0x08, 0xdb, 0x18, 0x04, //
    0xb4, 0x37, 0x50, 0x23, 0x56, 0xae, 0x1f, 0x5e, 0x29, 0xb1, 0x28, 0x88, 0x40, 0x3e, 0x78, 0xa8,
];

#[derive(Debug, Error)]
pub enum NpkError {
    #[error("not an NPK3 archive (bad magic)")]
    BadMagic,
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("{name}: {source}")]
    Unpack { name: String, source: std::io::Error },
    #[error("{name}: unpacked {got} bytes, the index says {want}")]
    Size { name: String, got: usize, want: usize },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
struct Segment {
    offset: u64,
    stored: usize,
    packed: usize,
    size: usize,
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// As stored (`media/cg/bg/bg019_廃校教室_01.png`).
    pub name: String,
    pub size: usize,
    segments: Vec<Segment>,
}

pub struct Npk {
    file: Mutex<File>,
    iv: [u8; 16],
    cipher: Aes256,
    pub entries: Vec<Entry>,
    /// Lower-cased name → entry index.
    by_name: HashMap<String, usize>,
}

/// AES-256-CBC decryption in place (`data` is a whole number of blocks).
fn decrypt(cipher: &Aes256, iv: &[u8; 16], data: &mut [u8]) {
    let mut prev = *iv;
    for chunk in data.as_chunks_mut::<16>().0 {
        let next = *chunk;
        cipher.decrypt_block(Block::from_mut_slice(chunk));
        chunk.iter_mut().zip(prev).for_each(|(b, p)| *b ^= p);
        prev = next;
    }
}

impl Npk {
    /// Reads the header and index; [`Npk::data`] reads each entry from the
    /// file when asked (`cg.npk` and `voice.npk` are over a gigabyte).
    pub fn open(mut file: File) -> Result<Npk, NpkError> {
        let mut head = [0; 32];
        file.read_exact(&mut head)?;
        if &head[..4] != b"NPK3" {
            return Err(NpkError::BadMagic);
        }
        let r = Reader(&head);
        let count = r.u32(24)? as usize;
        let mut index = vec![0; r.u32(28)? as usize & !15];
        file.read_exact(&mut index)?;
        let iv: [u8; 16] = head[8..24].try_into().unwrap();
        let cipher = Aes256::new(&KEY.into());
        decrypt(&cipher, &iv, &mut index);

        let r = Reader(&index);
        let (mut pos, mut entries) = (0, Vec::with_capacity(count));
        for _ in 0..count {
            let len = r.u16(pos + 1)? as usize;
            let name = String::from_utf8_lossy(r.slice(pos + 3, len)?).into_owned();
            pos += 3 + len;
            let size = r.u32(pos)? as usize;
            let nseg = r.u32(pos + 36)? as usize;
            pos += 40;
            let mut segments = Vec::with_capacity(nseg);
            for _ in 0..nseg {
                let offset = u64::from(r.u32(pos)?) | u64::from(r.u32(pos + 4)?) << 32;
                let stored = r.u32(pos + 8)? as usize;
                let packed = r.u32(pos + 12)? as usize;
                let size = r.u32(pos + 16)? as usize;
                segments.push(Segment { offset, stored, packed, size });
                pos += 20;
            }
            entries.push(Entry { name, size, segments });
        }
        let by_name = entries.iter().enumerate().map(|(i, e)| (e.name.to_lowercase(), i)).collect();
        Ok(Npk { file: Mutex::new(file), iv, cipher, entries, by_name })
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.by_name.get(&name.to_lowercase()).map(|&i| &self.entries[i])
    }

    pub fn data(&self, entry: &Entry) -> Result<Vec<u8>, NpkError> {
        let mut out = Vec::with_capacity(entry.size);
        for s in &entry.segments {
            let mut buf = vec![0; s.stored];
            {
                let mut f = self.file.lock().unwrap();
                f.seek(SeekFrom::Start(s.offset))?;
                f.read_exact(&mut buf)?;
            }
            decrypt(&self.cipher, &self.iv, &mut buf);
            if s.packed >= s.size {
                out.extend_from_slice(buf.get(..s.size).ok_or_else(|| Reader(&buf).truncated(0, s.size))?);
            } else {
                let packed = buf.get(..s.packed).ok_or_else(|| Reader(&buf).truncated(0, s.packed))?;
                // Some frames don't record their size: stream them.
                zstd::stream::copy_decode(packed, &mut out)
                    .map_err(|source| NpkError::Unpack { name: entry.name.clone(), source })?;
            }
        }
        if out.len() != entry.size {
            return Err(NpkError::Size { name: entry.name.clone(), got: out.len(), want: entry.size });
        }
        Ok(out)
    }
}
