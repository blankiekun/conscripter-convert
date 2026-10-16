//! MZX0 compression (the scripts in `allscr.mrg`).
//!
//! `MZX0`, u32 unpacked size, then commands working on 16-bit words. A
//! command byte's low 2 bits pick the command and the rest is `n`:
//! 0 repeats the last word `n + 1` times; 1 copies `n + 1` words from
//! `2 * (next byte + 1)` bytes back; 2 writes word `n` of a 64-word ring
//! of the last literals; 3 writes `n + 1` literal words, each byte
//! inverted (XOR 0xFF), and puts them in the ring. The last word and
//! the ring start as `0xFFFF`. The output is cut to the unpacked size.

use super::FormatError;
use crate::read::Reader;

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    if !data.starts_with(b"MZX0") {
        return Err(FormatError::BadMagic("MZX0"));
    }
    let r = Reader(data);
    let size = r.u32(4)? as usize;
    let mut out = Vec::with_capacity(size + 256);
    let mut last = [0xFF; 2];
    let mut ring = [[0xFF; 2]; 64];
    let mut ring_at = 0;
    let mut at = 8;
    while out.len() < size {
        let cmd = r.slice(at, 1)?[0];
        at += 1;
        let n = usize::from(cmd >> 2);
        match cmd & 3 {
            0 => (0..=n).for_each(|_| out.extend(last)),
            1 => {
                let back = 2 * (usize::from(r.slice(at, 1)?[0]) + 1);
                at += 1;
                for _ in 0..=n {
                    let from = out.len().checked_sub(back).ok_or_else(|| FormatError::Bad(format!("MZX back-reference before the start at {at:#x}")))?;
                    last = [out[from], out[from + 1]];
                    out.extend(last);
                }
            }
            2 => {
                last = ring[n];
                out.extend(last);
            }
            _ => {
                let want = 2 * (n + 1);
                let mut words = data.get(at..).unwrap_or_default();
                words = &words[..want.min(words.len())];
                // mangetsu's compressor (the fan patch's scripts) writes an
                // odd last chunk one byte short: fine while it fills `size`.
                if words.len() < want && out.len() + words.len() < size {
                    return Err(r.truncated(at, want).into());
                }
                for word in words.chunks(2) {
                    last = [word[0] ^ 0xFF, word.get(1).map_or(0, |b| b ^ 0xFF)];
                    ring[ring_at] = last;
                    ring_at = (ring_at + 1) % 64;
                    out.extend(last);
                }
                at += want;
            }
        }
    }
    out.truncate(size);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands() {
        let mut mzx = b"MZX0".to_vec();
        mzx.extend(9u32.to_le_bytes());
        // 2 literal words "abcd", repeat "cd" once, copy 1 word from 6 back
        // ("ab"), ring word 0 ("ab"), cut to 9 bytes
        mzx.extend([3 | 1 << 2, !b'a', !b'b', !b'c', !b'd', 0, 1, 2, 2]);
        assert_eq!(decompress(&mzx).unwrap(), b"abcdcdaba");
        assert!(decompress(&mzx[..mzx.len() - 1]).is_err());
    }

    #[test]
    fn odd_last_literal_one_byte_short() {
        let mut mzx = b"MZX0".to_vec();
        mzx.extend(3u32.to_le_bytes());
        mzx.extend([3 | 1 << 2, !b'a', !b'b', !b'c']);
        assert_eq!(decompress(&mzx).unwrap(), b"abc");
        mzx[4] = 4;
        assert!(decompress(&mzx).is_err());
    }
}
