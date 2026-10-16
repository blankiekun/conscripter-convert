//! TYPE-MOON's engine (月姫 -A piece of blue glass moon-, Switch, 2021; the
//! 魔法使いの夜 remake's family): MRG and MZP archives, MZX and NXGX/NXCX
//! compression. Formats as read by mangetsu (MIT) and deepLuna (BSD-2),
//! checked over the whole game (`docs/games/tsukihime.md`).

pub mod bfsar;
pub mod bntx;
pub mod fwav;
pub mod mrg;
pub mod mzp;
pub mod mzx;
pub mod nxopus;
pub mod nxx;

use std::borrow::Cow;

use thiserror::Error;

use crate::read::Truncated;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("not {0} data")]
    BadMagic(&'static str),
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("{0}")]
    Bad(String),
}

/// `data` without its MZX0, NXGX or NXCX wrapper; anything else as it is.
pub fn unpack(data: &[u8]) -> Result<Cow<'_, [u8]>, FormatError> {
    Ok(match data.get(..4) {
        Some(b"MZX0") => Cow::Owned(mzx::decompress(data)?),
        Some(b"NXGX" | b"NXCX") => Cow::Owned(nxx::decompress(data)?),
        _ => Cow::Borrowed(data),
    })
}

/// A file extension for unpacked data, from its magic.
pub fn extension(data: &[u8]) -> &'static str {
    match data {
        [b'B', b'N', b'T', b'X', ..] => "bntx",
        [b'm', b'r', b'g', b'd', b'0', b'0', ..] => "mzp",
        [0xFF, 0xD8, ..] => "jpg",
        [0x89, b'P', b'N', b'G', ..] => "png",
        _ => "bin",
    }
}
