//! Writes NSA archives (`arc.nsa`, `arc1.nsa`, …) that ONScripter reads.
//!
//! Format (big-endian, as read by ONScripter's `SarReader::readArchive`):
//!
//! ```text
//! u16   file count
//! u32   offset of the data area
//! entry[count]: name (Shift-JIS, NUL-terminated, '\\' separators),
//!               u8 compression (0 = none), u32 offset (from the data
//!               area), u32 stored length, u32 original length
//! data
//! ```

use std::fs::File;
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

fn write_one(path: &Path, files: &[(&NsaFile, u64)]) -> io::Result<()> {
    let names: Vec<Vec<u8>> = files
        .iter()
        .map(|(f, _)| {
            let (bytes, _, bad) = encoding_rs::SHIFT_JIS.encode(&f.name);
            if bad {
                return Err(io::Error::other(format!("{:?} can't be encoded as Shift-JIS", f.name)));
            }
            Ok(bytes.into_owned())
        })
        .collect::<io::Result<_>>()?;
    let header_len: usize = 6 + names.iter().map(|n| n.len() + 1 + 1 + 12).sum::<usize>();

    let mut out = BufWriter::new(File::create(path)?);
    out.write_all(&(files.len() as u16).to_be_bytes())?;
    out.write_all(&(header_len as u32).to_be_bytes())?;
    let mut offset = 0u64;
    for (name, (_, len)) in names.iter().zip(files) {
        out.write_all(name)?;
        out.write_all(&[0, 0])?; // NUL, compression = none
        out.write_all(&(offset as u32).to_be_bytes())?;
        out.write_all(&(*len as u32).to_be_bytes())?;
        out.write_all(&(*len as u32).to_be_bytes())?;
        offset += len;
    }
    for (f, len) in files {
        let copied = io::copy(&mut File::open(&f.source)?, &mut out)?;
        if copied != *len {
            return Err(io::Error::other(format!("{} changed size while packing", f.source.display())));
        }
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
