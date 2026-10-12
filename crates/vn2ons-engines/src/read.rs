//! Bounds-checked little-endian reads over a byte slice.

use thiserror::Error;

#[derive(Debug, Error)]
#[error("truncated: needed {needed} bytes at offset {offset:#x}, file is {len} bytes")]
pub struct Truncated {
    pub offset: usize,
    pub needed: usize,
    pub len: usize,
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
