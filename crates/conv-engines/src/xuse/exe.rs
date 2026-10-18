//! The tables `Farthest2015.exe` keeps for the scene replay, the CG gallery
//! and the staff rolls, read from the exe at build time (addresses as in
//! the decompile; the layouts are in [`super::replay`], [`super::gallery`]
//! and [`super::staffroll`]).

use super::replay::{self, List};
use super::staffroll::Roll;
use std::path::{Path, PathBuf};

/// The exe's file name in the install.
pub const EXE: &str = "Farthest2015.exe";

/// What the converter reads from the exe.
pub struct Tables {
    /// The scene replay's lists, the table's used entries in order.
    pub lists: Vec<List>,
    /// Per gallery tab, pages of slots of CG bits.
    pub tabs: Vec<Vec<Vec<Vec<u16>>>>,
    /// Indexed by 0x91's arg.
    pub rolls: Vec<Roll>,
}

const REPLAY_TABLE: u32 = 0x4fe200;
const REPLAY_ENTRIES: u32 = 16;
const GALLERY_TABLE: u32 = 0x500118;
const GALLERY_TABS: u32 = 7;
/// Slots are 64 bytes: the thumbnail index, then up to 15 CG bits.
const SLOT_BITS: u32 = 15;
const ROLL_TABLE: u32 = 0x517f30;
const ROLLS: u32 = 2;
/// Bounds that no table of this exe reaches; past them the exe is another one.
const MAX_COUNT: u32 = 1000;

/// `Farthest2015.exe`'s sections, read by virtual address.
struct Pe<'a> {
    data: &'a [u8],
    /// (address, size in the file, file offset).
    sections: Vec<(u32, u32, u32)>,
}

impl<'a> Pe<'a> {
    fn new(data: &'a [u8]) -> Result<Self, String> {
        let u = |o: usize, n: usize| data.get(o..o + n).ok_or("truncated PE header");
        let le32 = |o| u(o, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let le16 = |o| u(o, 2).map(|b| u16::from_le_bytes(b.try_into().unwrap()));
        if u(0, 2)? != b"MZ" {
            return Err("not a PE file".into());
        }
        let pe = le32(0x3c)? as usize;
        if u(pe, 4)? != b"PE\0\0" {
            return Err("not a PE file".into());
        }
        let (count, optional) = (le16(pe + 6)? as usize, le16(pe + 20)? as usize);
        let base = le32(pe + 24 + 28)?;
        let sections = (0..count)
            .map(|i| {
                let o = pe + 24 + optional + 40 * i;
                Ok((base + le32(o + 12)?, le32(o + 8)?.min(le32(o + 16)?), le32(o + 20)?))
            })
            .collect::<Result<_, &str>>()?;
        Ok(Self { data, sections })
    }

    fn bytes(&self, at: u32, len: u32) -> Result<&'a [u8], String> {
        let &(va, _, raw) = self
            .sections
            .iter()
            .find(|&&(va, size, _)| at >= va && at - va + len <= size)
            .ok_or_else(|| format!("{at:#x} is outside the exe's data"))?;
        let o = (at - va + raw) as usize;
        self.data.get(o..o + len as usize).ok_or_else(|| format!("{at:#x} is past the end of the file"))
    }

    fn u32(&self, at: u32) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(at, 4)?.try_into().unwrap()))
    }

    fn i32(&self, at: u32) -> Result<i32, String> {
        Ok(self.u32(at)? as i32)
    }

    /// A count, refused when out of bounds.
    fn count(&self, at: u32) -> Result<u32, String> {
        let n = self.u32(at)?;
        if n > MAX_COUNT { Err(format!("count {n} at {at:#x}: not the expected exe")) } else { Ok(n) }
    }

    /// The 0-ended Shift-JIS string at the address read at `at`.
    fn string(&self, at: u32) -> Result<String, String> {
        let start = self.u32(at)?;
        let mut len = 0;
        while self.bytes(start + len, 1)?[0] != 0 {
            len += 1;
            if len > 256 {
                return Err(format!("no string at {start:#x}: not the expected exe"));
            }
        }
        let (s, _, bad) = encoding_rs::SHIFT_JIS.decode(self.bytes(start, len)?);
        if bad { Err(format!("no string at {start:#x}: not the expected exe")) } else { Ok(s.into_owned()) }
    }

    /// i32s from `at` up to a -1.
    fn until_minus_one(&self, at: u32, stride: u32) -> Result<Vec<u32>, String> {
        let mut out = Vec::new();
        while self.i32(at + stride * out.len() as u32)? != -1 {
            out.push(at + stride * out.len() as u32);
            if out.len() as u32 > MAX_COUNT {
                return Err(format!("no end to the list at {at:#x}: not the expected exe"));
            }
        }
        Ok(out)
    }
}

impl Tables {
    /// Reads the tables from the exe in `game`.
    pub fn read(game: &Path) -> Result<Self, (PathBuf, String)> {
        let path = crate::find(game, EXE);
        let data = std::fs::read(&path).map_err(|e| (path.clone(), e.to_string()))?;
        Self::parse(&data).map_err(|e| (path, e))
    }

    fn parse(data: &[u8]) -> Result<Self, String> {
        let pe = Pe::new(data)?;
        // 16 × (u32 used, pointer to (u32 count, pointer to 16-byte entries:
        // u32 scene bit, u32 H flag, script name, label name)).
        let mut lists = Vec::new();
        for i in 0..REPLAY_ENTRIES {
            let entry = REPLAY_TABLE + 8 * i;
            if pe.u32(entry)? == 0 {
                continue;
            }
            let head = pe.u32(entry + 4)?;
            let entries = pe.u32(head + 4)?;
            let mut file = None;
            let mut scenes = Vec::new();
            for k in 0..pe.count(head)? {
                let e = entries + 16 * k;
                let name = pe.string(e + 8)?;
                if file.get_or_insert_with(|| name.clone()) != &name {
                    return Err(format!("replay list {i} has scenes in two scripts"));
                }
                let bit = u16::try_from(pe.u32(e)?).map_err(|_| format!("scene bit at {e:#x}: not the expected exe"))?;
                scenes.push((bit, pe.u32(e + 4)? != 0, pe.string(e + 12)?));
            }
            let &(picture, disabled) = replay::BUTTONS
                .get(i as usize)
                .and_then(Option::as_ref)
                .ok_or_else(|| format!("replay list {i} has no button: not the expected exe"))?;
            lists.push(List { picture, disabled, file: file.unwrap_or_default(), scenes });
        }
        // Per tab (u32 page count, pointer to pages of (u32 count, pointer to slots)).
        let mut tabs = Vec::new();
        for t in 0..GALLERY_TABS {
            let pages = pe.u32(GALLERY_TABLE + 8 * t + 4)?;
            let mut tab = Vec::new();
            for p in 0..pe.count(GALLERY_TABLE + 8 * t)? {
                let slots = pe.u32(pages + 8 * p + 4)?;
                let mut page = Vec::new();
                for k in 0..pe.count(pages + 8 * p)? {
                    let slot = slots + 64 * k;
                    let mut bits = Vec::new();
                    for b in 1..=SLOT_BITS {
                        match pe.u32(slot + 4 * b)? {
                            0 => break,
                            bit => bits.push(u16::try_from(bit).map_err(|_| format!("CG bit at {slot:#x}: not the expected exe"))?),
                        }
                    }
                    page.push(bits);
                }
                tab.push(page);
            }
            tabs.push(tab);
        }
        // Descriptors: +0 gaps, +4 key frames (both up to a -1), +8 the
        // `.004`, +0xc first credit, +0x14 centre x, +0x18 start, +0x1c end,
        // +0x20 total, +0x28 BGM.
        let mut rolls = Vec::new();
        for r in 0..ROLLS {
            let d = pe.u32(ROLL_TABLE + 4 * r)?;
            let gaps = pe.until_minus_one(pe.u32(d)?, 4)?.into_iter().map(|a| pe.i32(a)).collect::<Result<_, _>>()?;
            let keys = pe
                .until_minus_one(pe.u32(d + 4)?, 52)?
                .into_iter()
                .map(|a| {
                    let mut k = [0; 13];
                    for (j, v) in k.iter_mut().enumerate() {
                        *v = pe.i32(a + 4 * j as u32)?;
                    }
                    Ok(k)
                })
                .collect::<Result<_, String>>()?;
            rolls.push(Roll {
                file: pe.string(d + 8)?,
                first_part: pe.count(d + 0xc)? as usize,
                gaps,
                centre_x: pe.i32(d + 0x14)?,
                start_ms: pe.i32(d + 0x18)?,
                end_ms: pe.i32(d + 0x1c)?,
                total_ms: pe.u32(d + 0x20)?,
                bgm: pe.count(d + 0x28)? as usize,
                keys,
            });
        }
        Ok(Self { lists, tabs, rolls })
    }

    /// `number` (a background, 1-based) if the CG gallery shows it.
    pub fn gallery_cg(&self, number: i32) -> Option<u32> {
        let bit = u16::try_from(number.checked_sub(1)?).ok()?;
        self.tabs.iter().flatten().flatten().any(|s| s.contains(&bit)).then_some(number as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_what_is_not_the_exe() {
        assert_eq!(Tables::parse(b"MZ").err().unwrap(), "truncated PE header");
        assert_eq!(Tables::parse(&[0; 64]).err().unwrap(), "not a PE file");
    }

    /// Checks the tables when the game is linked for `tests/snapshot.sh`
    /// (they matched the hand-copied ones they replace, entry for entry).
    #[test]
    fn reads_the_installed_exe() {
        let games = std::env::var_os("CONS_GAMES")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config/conscripter-convert/games")));
        let Some(game) = games.map(|g| g.join("saihate-no-ima")).filter(|g| g.exists())
        else {
            return;
        };
        let t = Tables::read(&game).unwrap();
        assert_eq!(t.lists.len(), 11);
        assert_eq!(t.lists.iter().map(|l| l.scenes.len()).sum::<usize>(), 418);
        assert!(t.lists.iter().all(|l| l.scenes.len() <= replay::ROWS * replay::COLUMNS));
        assert_eq!(t.tabs.iter().map(Vec::len).collect::<Vec<_>>(), [1, 2, 1, 2, 0, 0, 3]);
        assert_eq!((t.rolls[1].file.as_str(), t.rolls[1].keys.len(), t.rolls[1].bgm), ("StaffRoll2.004", 35, 22));
    }
}
