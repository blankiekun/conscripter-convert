//! Writes NSA archives (`arc.nsa`, `arc1.nsa`, …) that ONScripter reads.
//!
//! Format (big-endian, as read by ONScripter's `SarReader::readArchive`):
//!
//! ```text
//! u16   file count
//! u32   offset of the data area
//! entry[count]: name (the script's encoding, UTF-8; NUL-terminated, '\\' separators),
//!               u8 compression (0 = none), u32 offset (from the data
//!               area), u32 stored length, u32 original length
//! data
//! ```

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// ONScripter loads `arc.nsa` and then `arc1.nsa` … `arc9.nsa`.
const MAX_ARCHIVES: usize = 10;
const MAX_FILES: usize = u16::MAX as usize;
/// Offsets are u32; stay well clear of the limit.
const MAX_BYTES: u64 = 2_000_000_000;

pub struct NsaFile {
    /// Name as the script refers to it (`bg\\room.png`).
    pub name: String,
    pub source: PathBuf,
}

/// Packs `files` into `dir/arc.nsa`, `dir/arc1.nsa`, …, splitting when an
/// archive would exceed the file-count or size limit. Returns the archive
/// paths written.
pub fn write_archives(dir: &Path, files: &[NsaFile]) -> io::Result<Vec<PathBuf>> {
    let mut groups: Vec<Vec<(&NsaFile, u64)>> = vec![Vec::new()];
    let mut bytes = 0u64;
    for f in files {
        let len = f.source.metadata()?.len();
        let full = groups.last().unwrap().len() >= MAX_FILES || bytes + len > MAX_BYTES;
        if full && !groups.last().unwrap().is_empty() {
            groups.push(Vec::new());
            bytes = 0;
        }
        groups.last_mut().unwrap().push((f, len));
        bytes += len;
    }
    if groups.len() > MAX_ARCHIVES {
        return Err(io::Error::other(format!("{} archives needed, ONScripter reads at most {MAX_ARCHIVES}", groups.len())));
    }

    let mut written = Vec::new();
    for (i, group) in groups.iter().enumerate() {
        let name = if i == 0 { "arc.nsa".to_string() } else { format!("arc{i}.nsa") };
        let path = dir.join(name);
        write_one(&path, group)?;
        written.push(path);
    }
    Ok(written)
}

/// Entries with identical bytes point at one copy of them (the engine
/// only seeks to an entry's offset).
fn write_one(path: &Path, files: &[(&NsaFile, u64)]) -> io::Result<()> {
    // each entry's data offset, and whether its bytes are stored here (else an earlier copy's)
    let mut seen: HashMap<(u64, u64), Vec<(usize, u64)>> = HashMap::new();
    let mut placed: Vec<(u64, bool)> = Vec::with_capacity(files.len());
    let mut offset = 0u64;
    for (i, (f, len)) in files.iter().enumerate() {
        let key = (*len, hash_file(&f.source)?);
        let copies = seen.entry(key).or_default();
        let mut same = None;
        for &(j, at) in copies.iter() {
            if fs::read(&files[j].0.source)? == fs::read(&f.source)? {
                same = Some(at);
                break;
            }
        }
        match same {
            Some(at) => placed.push((at, false)),
            None => {
                copies.push((i, offset));
                placed.push((offset, true));
                offset += len;
            }
        }
    }

    let names: Vec<Vec<u8>> = files
        .iter()
        // in the script's encoding, UTF-8 (`encode`): CONscripter compares the bytes
        .map(|(f, _)| f.name.as_bytes().to_vec())
        .collect();
    let header_len: usize = 6 + names.iter().map(|n| n.len() + 1 + 1 + 12).sum::<usize>();

    let mut out = BufWriter::new(File::create(path)?);
    out.write_all(&(files.len() as u16).to_be_bytes())?;
    out.write_all(&(header_len as u32).to_be_bytes())?;
    for ((name, (_, len)), (at, _)) in names.iter().zip(files).zip(&placed) {
        out.write_all(name)?;
        out.write_all(&[0, 0])?; // NUL, compression = none
        out.write_all(&(*at as u32).to_be_bytes())?;
        out.write_all(&(*len as u32).to_be_bytes())?;
        out.write_all(&(*len as u32).to_be_bytes())?;
    }
    for ((f, len), _) in files.iter().zip(&placed).filter(|(_, (_, stored))| *stored) {
        let copied = io::copy(&mut File::open(&f.source)?, &mut out)?;
        if copied != *len {
            return Err(io::Error::other(format!("{} changed size while packing", f.source.display())));
        }
    }
    out.flush()
}

fn hash_file(path: &Path) -> io::Result<u64> {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut r = io::BufReader::new(File::open(path)?);
    loop {
        let buf = io::BufRead::fill_buf(&mut r)?;
        if buf.is_empty() {
            return Ok(h.finish());
        }
        h.write(buf);
        let n = buf.len();
        io::BufRead::consume(&mut r, n);
    }
}

/// Where the archive keeps [`script_hash`] of the 0.txt it was packed for:
/// the engine's debug trace warns when the running 0.txt differs.
pub const SCRIPT_HASH: &str = "cons\\script.hash";

/// FNV-1a 64 of `script`, as 16 hex digits.
pub fn script_hash(script: &[u8]) -> String {
    let h = script.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    format!("{h:016x}")
}

/// The names of the files in an NSA archive, as the script refers to them.
pub fn read_names(path: &Path) -> io::Result<Vec<String>> {
    use std::io::{BufReader, Read};
    let mut r = BufReader::new(File::open(path)?);
    let mut head = [0u8; 6];
    r.read_exact(&mut head)?;
    let count = u16::from_be_bytes([head[0], head[1]]);
    let mut names = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut name = Vec::new();
        let mut b = [0u8; 1];
        loop {
            r.read_exact(&mut b)?;
            if b[0] == 0 {
                break;
            }
            name.push(b[0]);
        }
        let mut rest = [0u8; 13];
        r.read_exact(&mut rest)?;
        names.push(String::from_utf8_lossy(&name).into_owned());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_hash_is_fnv1a() {
        assert_eq!(script_hash(b""), "cbf29ce484222325");
        assert_eq!(script_hash(b"a"), "af63dc4c8601ec8c");
    }

    #[test]
    fn writes_a_readable_archive() {
        let dir = std::env::temp_dir().join(format!("conscripter-convert-nsa-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.png");
        let b = dir.join("b.ogg");
        std::fs::write(&a, b"AAAA").unwrap();
        std::fs::write(&b, b"BB").unwrap();
        let files = [
            NsaFile { name: "bg\\a.png".into(), source: a },
            NsaFile { name: "se\\b.ogg".into(), source: b },
        ];
        let written = write_archives(&dir, &files).unwrap();
        assert_eq!(written, [dir.join("arc.nsa")]);

        let data = std::fs::read(dir.join("arc.nsa")).unwrap();
        let header_len = 6 + (8 + 1 + 1 + 12) * 2;
        assert_eq!(&data[..2], &2u16.to_be_bytes());
        assert_eq!(&data[2..6], &(header_len as u32).to_be_bytes());
        assert_eq!(&data[6..15], b"bg\\a.png\0");
        assert_eq!(data[15], 0);
        assert_eq!(&data[16..28], &[0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 4]);
        assert_eq!(&data[header_len..], b"AAAABB");
        assert_eq!(read_names(&dir.join("arc.nsa")).unwrap(), ["bg\\a.png", "se\\b.ogg"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn identical_entries_share_one_copy() {
        let dir = std::env::temp_dir().join(format!("conscripter-convert-nsa-dedupe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = |n: &str, data: &[u8]| {
            std::fs::write(dir.join(n), data).unwrap();
            NsaFile { name: n.into(), source: dir.join(n) }
        };
        let files = [src("a", b"AAAA"), src("b", b"BBBB"), src("c", b"AAAA")];
        write_archives(&dir, &files).unwrap();
        let data = std::fs::read(dir.join("arc.nsa")).unwrap();
        let header_len = 6 + (1 + 1 + 1 + 12) * 3;
        assert_eq!(&data[header_len..], b"AAAABBBB");
        // c's offset is a's (0), its lengths its own
        assert_eq!(&data[6 + 15 * 2 + 3..6 + 15 * 3], &[0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 4]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
