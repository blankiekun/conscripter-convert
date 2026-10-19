//! Bounds-checked little-endian reads over a byte slice.

use thiserror::Error;

#[derive(Debug, Error)]
#[error("truncated: needed {needed} bytes at offset {offset:#x}, file is {len} bytes")]
pub struct Truncated {
    pub offset: usize,
    pub needed: usize,
    pub len: usize,
}

/// `Err` unless `len` bytes at `offset` lie inside `file`: a size read
/// from a header, checked before a buffer is made of it.
pub(crate) fn in_file(file: &std::fs::File, offset: u64, len: usize) -> std::io::Result<()> {
    let file_len = file.metadata()?.len();
    if offset.checked_add(len as u64).is_some_and(|end| end <= file_len) {
        return Ok(());
    }
    Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, format!("{len} bytes at {offset} are past the file's end ({file_len})")))
}

pub(crate) struct Reader<'a>(pub &'a [u8]);

impl<'a> Reader<'a> {
    pub fn truncated(&self, offset: usize, needed: usize) -> Truncated {
        Truncated { offset, needed, len: self.0.len() }
    }

    pub fn slice(&self, offset: usize, len: usize) -> Result<&'a [u8], Truncated> {
        offset
            .checked_add(len)
            .and_then(|end| self.0.get(offset..end))
            .ok_or_else(|| self.truncated(offset, len))
    }

    pub fn u16(&self, offset: usize) -> Result<u16, Truncated> {
        Ok(u16::from_le_bytes(self.slice(offset, 2)?.try_into().unwrap()))
    }

    pub fn u16_be(&self, offset: usize) -> Result<u16, Truncated> {
        Ok(u16::from_be_bytes(self.slice(offset, 2)?.try_into().unwrap()))
    }

    pub fn u32(&self, offset: usize) -> Result<u32, Truncated> {
        Ok(u32::from_le_bytes(self.slice(offset, 4)?.try_into().unwrap()))
    }
}
