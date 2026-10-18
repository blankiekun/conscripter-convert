//! TYPE-MOON's engine (月姫 -A piece of blue glass moon-, Switch, 2021; the
//! 魔法使いの夜 remake's family): MRG and MZP archives, MZX and NXGX/NXCX
//! compression. Formats as read by mangetsu (MIT) and deepLuna (BSD-2),
//! checked over the whole game (`docs/games/tsukihime.md`).

pub mod bfsar;
pub mod bntx;
pub mod boot;
pub mod build;
pub mod convert;
pub mod exe;
pub mod fwav;
pub mod language;
pub mod mrg;
pub mod menu;
pub mod mzp;
pub mod mzx;
pub mod nso;
pub mod nxopus;
pub mod nxx;
pub mod script;
pub mod stec;
pub mod title;

use std::borrow::Cow;
use std::collections::BTreeMap;

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

/// Per picture name of an MRG (lowercase, extension dropped) its full art:
/// the `.JPG` if there is one, else the largest `.NXGZ`/`.NXZ`/`.PNG`.
pub fn full_copies(entries: &[mrg::Entry]) -> BTreeMap<String, &mrg::Entry> {
    let mut best = BTreeMap::<String, (bool, &mrg::Entry)>::new();
    for e in entries {
        let Some((stem, ext)) = e.name.as_deref().and_then(|n| n.rsplit_once('.')) else { continue };
        let ext = ext.to_ascii_uppercase();
        if !matches!(ext.as_str(), "JPG" | "NXGZ" | "NXZ" | "PNG") {
            continue;
        }
        let rank = (ext == "JPG", e);
        let slot = best.entry(stem.to_lowercase()).or_insert(rank);
        if (rank.0, rank.1.size) > (slot.0, slot.1.size) {
            *slot = rank;
        }
    }
    best.into_iter().map(|(k, (_, e))| (k, e)).collect()
}

/// The names of `en`'s [`full_copies`] whose data differ from `ja`'s
/// (or that `ja` lacks): the pictures an English build has of its own.
/// `ja_data`, `en_data`: the two `.mrg` files.
pub fn differing(ja: &BTreeMap<String, &mrg::Entry>, ja_data: &[u8], en: &BTreeMap<String, &mrg::Entry>, en_data: &[u8]) -> Result<Vec<String>, FormatError> {
    let mut out = Vec::new();
    for (stem, e) in en {
        let same = match ja.get(stem) {
            Some(j) => j.size == e.size && mrg::data(ja_data, j)? == mrg::data(en_data, e)?,
            None => false,
        };
        if !same {
            out.push(stem.clone());
        }
    }
    Ok(out)
}

/// A picture entry's width and height without decoding it: a BNTX's
/// header (the first 64 KiB unpacked), or a JPEG's or PNG's.
pub fn picture_size(data: &[u8]) -> Result<(u32, u32), FormatError> {
    if matches!(data.get(..4), Some(b"NXGX" | b"NXCX")) {
        return bntx::size(&nxx::head(data, 0x10000)?);
    }
    image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok())
        .ok_or_else(|| FormatError::Bad("not a picture".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_copies_prefer_the_jpg_then_the_largest() {
        let e = |name: &str, size| mrg::Entry { name: Some(name.into()), offset: 0, size, unpacked_sectors: 0 };
        let entries = [e("IMG0020.NXGZ", 9), e("IMG0020.JPG", 5), e("ARU_A.NXGZ", 1), e("ARU_A.NXGZ", 7), e("X.MZP", 3)];
        let best = full_copies(&entries);
        let names: Vec<_> = best.iter().map(|(k, e)| (k.as_str(), e.name.as_deref().unwrap(), e.size)).collect();
        assert_eq!(names, [("aru_a", "ARU_A.NXGZ", 7), ("img0020", "IMG0020.JPG", 5)]);
    }

    #[test]
    fn differing_finds_changed_and_new_pictures() {
        let e = |name: &str, offset, size| mrg::Entry { name: Some(name.into()), offset, size, unpacked_sectors: 0 };
        let ja = [e("A.NXGZ", 0, 2), e("B.NXGZ", 2, 2)];
        let en = [e("A.NXGZ", 0, 2), e("B.NXGZ", 2, 2), e("C.NXGZ", 4, 1)];
        let (jd, ed) = (b"aabb".as_slice(), b"aaxbc".as_slice());
        let (ja, en) = (full_copies(&ja), full_copies(&en));
        assert_eq!(differing(&ja, jd, &en, ed).unwrap(), ["b", "c"]);
    }
}
