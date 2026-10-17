//! The tables in 月姫's exe (1.0.2's `main`; the fan patch's differs by 65
//! bytes elsewhere): pictures, motions and scene numbers. Addresses are
//! Ghidra's (the image loaded at 0x100000); `docs/games/tsukihime.md`,
//! "The exe", says how each was found.

use super::FormatError;
use super::nso;
use crate::read::Reader;

const BASE: usize = 0x10_0000;
const PICTURE_NAMES: usize = 0x6E_02A4;
const PICTURE_RECORDS: usize = 0x56_F830;
/// The lookup's limit; 14,713 are used.
const PICTURES: usize = 23_000;
const MOTION_KEY_END: usize = 0x4B_1940;
const MOTION_LENGTH: usize = 0x49_9F40;
const MOTION_KEYS: usize = 0x75_CE18;
const MOTIONS: usize = 24_192;
const MOTION_KEY_COUNT: usize = 71_905;
const SCENES: usize = 0x45_0D58;
const SCENE_COUNT: usize = 551;

/// A picture `_STCH` can name: the name's archive entry is `<name>.NXGZ`
/// in `allpac`.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub name: String,
    /// Size on the engine's 1280-wide stage: half an expression texture's
    /// width and 724/1440 of its height; three times a bare pose's
    /// (a third-size texture), rounded.
    pub width: f32,
    pub height: f32,
    /// `_STCH` puts this point at its x, y (half the size, mostly).
    pub centre: (f32, f32),
    /// Byte 0x41: 3 for the plain colours (`BLACK` …), 0 otherwise; bit 0
    /// makes `_STCH` draw it opaque.
    pub flags: u8,
    /// Byte 0x43: 1 for an expression (`…_a_b`), 0 for anything else.
    pub expression: bool,
    /// u16 at 0x46: a sprite's shot (`_A_` 0, `_L_` 1, `_M_` 2, `_S_` 3).
    pub shot: u16,
    /// u16 at 0x44: per character pose, meaning not known.
    pub pose_value: u16,
}

/// One keyframe of a motion: fields not given are −32767 in the exe;
/// −32766 (`Key::THROUGH`) is resolved by `Motion::through_filled`.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub time_ms: u32,
    /// The 30 s16 after the time (0x04 easing, 0x08 x, 0x0A y, 0x0C
    /// alpha, 0x18 and 0x1A zoom ‰ …; field `i` is at 0x04 + 2i).
    pub fields: [i16; 30],
}

impl Key {
    pub const UNSET: i16 = -32767;
    /// No key for this field here: the exe interpolates it between the
    /// keys around it that have one (`FUN_00197710` skips these keys).
    pub const THROUGH: i16 = -32766;
    fn field(&self, at: usize) -> Option<i16> {
        Some(self.fields[(at - 4) / 2]).filter(|&v| v != Self::UNSET)
    }
    pub fn easing(&self) -> Option<i16> {
        self.field(0x04)
    }
    pub fn x(&self) -> Option<i16> {
        self.field(0x08)
    }
    pub fn y(&self) -> Option<i16> {
        self.field(0x0A)
    }
    pub fn alpha(&self) -> Option<i16> {
        self.field(0x0C)
    }
    pub fn zoom(&self) -> (Option<i16>, Option<i16>) {
        (self.field(0x18), self.field(0x1A))
    }
}

/// `_STMA(layer, m)`'s motion `m` (from 1).
#[derive(Debug, Clone, PartialEq)]
pub struct Motion {
    pub length_ms: u32,
    pub keys: Vec<Key>,
}

impl Motion {
    /// Each `Key::THROUGH` field given its value on the straight line
    /// between the keys before and after it that set the field; with none
    /// after, unset (the exe holds the last value set).
    fn through_filled(mut self) -> Self {
        // Fields 8 and 9 (0x14) are one i32, whose skip value is −100000.
        for f in (0..30).filter(|f| !matches!(f, 8 | 9)) {
            let set = |k: &Key| ![Key::UNSET, Key::THROUGH].contains(&k.fields[f]);
            for i in 0..self.keys.len() {
                if self.keys[i].fields[f] != Key::THROUGH {
                    continue;
                }
                let before = self.keys[..i].iter().rev().find(|k| set(k));
                let after = self.keys[i + 1..].iter().find(|k| set(k));
                let t = self.keys[i].time_ms as f32;
                self.keys[i].fields[f] = match (before, after) {
                    (Some(a), Some(b)) if b.time_ms > a.time_ms => {
                        let (va, vb) = (f32::from(a.fields[f]), f32::from(b.fields[f]));
                        (va + (vb - va) * (t - a.time_ms as f32) / (b.time_ms - a.time_ms) as f32).round() as i16
                    }
                    (Some(a), Some(_)) => a.fields[f],
                    _ => Key::UNSET,
                };
            }
        }
        self
    }
}

pub struct Exe {
    image: Vec<u8>,
}

impl Exe {
    /// From the NSO; fails unless it is 1.0.2's layout.
    pub fn parse(nso: &[u8]) -> Result<Self, FormatError> {
        let exe = Exe { image: nso::image(nso)? };
        if exe.bytes(PICTURE_NAMES, 6)? != b"BLACK\0" {
            return Err(FormatError::Bad("not 1.0.2's or the fan patch's main (the base game's?): no picture table at 0x6E02A4".into()));
        }
        Ok(exe)
    }

    fn bytes(&self, address: usize, len: usize) -> Result<&[u8], FormatError> {
        Ok(Reader(&self.image).slice(address - BASE, len)?)
    }

    fn u32(&self, address: usize) -> Result<u32, FormatError> {
        Ok(Reader(&self.image).u32(address - BASE)?)
    }

    fn f32(&self, address: usize) -> Result<f32, FormatError> {
        Ok(f32::from_bits(self.u32(address)?))
    }

    /// Every named picture, in table order (`_STCH` finds them by name).
    pub fn pictures(&self) -> Result<Vec<Picture>, FormatError> {
        let mut out = Vec::new();
        for i in 0..PICTURES {
            let raw = self.bytes(PICTURE_NAMES + 32 * i, 32)?;
            if raw[0] == 0 || raw[0] == 0xFF {
                break; // an empty name ends the table; the rest is 0xFF
            }
            let name = raw.split(|&b| b == 0).next().unwrap_or_default();
            let name = std::str::from_utf8(name).map_err(|_| FormatError::Bad(format!("picture {i}: name not ASCII")))?;
            let at = PICTURE_RECORDS + 0x48 * i;
            let tail = self.bytes(at + 0x40, 8)?;
            out.push(Picture {
                name: name.to_string(),
                width: self.f32(at + 0x2C)?,
                height: self.f32(at + 0x30)?,
                centre: (self.f32(at + 0x34)?, self.f32(at + 0x38)?),
                flags: tail[1],
                expression: tail[3] == 1,
                pose_value: u16::from_le_bytes([tail[4], tail[5]]),
                shot: u16::from_le_bytes([tail[6], tail[7]]),
            });
        }
        Ok(out)
    }

    /// Motions 1 to 24,192, as `motions()[m - 1]`, each with its keys (the
    /// keys run on from one motion to the next).
    pub fn motions(&self) -> Result<Vec<Motion>, FormatError> {
        // The table's entry m − 1 is one past motion m's last key; motion 1 starts at key 0.
        let end = |m: usize| -> Result<usize, FormatError> { Ok(self.u32(MOTION_KEY_END + 4 * m)? as usize) };
        (0..MOTIONS)
            .map(|m| {
                let (from, to) = (if m == 0 { 0 } else { end(m - 1)? }, end(m)?);
                if from > to || to > MOTION_KEY_COUNT {
                    return Err(FormatError::Bad(format!("motion {}: keys {from}..{to}", m + 1)));
                }
                let keys = (from..to)
                    .map(|k| {
                        let raw = self.bytes(MOTION_KEYS + 0x40 * k, 0x40)?;
                        let mut fields = [0; 30];
                        for (i, f) in fields.iter_mut().enumerate() {
                            *f = i16::from_le_bytes([raw[4 + 2 * i], raw[5 + 2 * i]]);
                        }
                        Ok(Key { time_ms: u32::from_le_bytes(raw[..4].try_into().unwrap()), fields })
                    })
                    .collect::<Result<_, FormatError>>()?;
                Ok(Motion { length_ms: self.u32(MOTION_LENGTH + 4 * m)?, keys }.through_filled())
            })
            .collect()
    }

    /// Each script's scene number, in `allscr` order: `_JUMP(n)` goes to
    /// the script whose number is `n`.
    pub fn scenes(&self) -> Result<Vec<i16>, FormatError> {
        let raw = self.bytes(SCENES, 2 * SCENE_COUNT)?;
        Ok(raw.as_chunks::<2>().0.iter().map(|&c| i16::from_le_bytes(c)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_fields_unset_and_set() {
        let mut fields = [Key::UNSET; 30];
        fields[2] = 226; // 0x08: x
        fields[10] = 584; // 0x18: zoom
        let key = Key { time_ms: 0, fields };
        assert_eq!((key.x(), key.y(), key.zoom()), (Some(226), None, (Some(584), None)));
    }

    #[test]
    fn through_keys_take_the_line_between_their_neighbours() {
        let key = |time_ms, x| {
            let mut fields = [Key::UNSET; 30];
            fields[2] = x;
            Key { time_ms, fields }
        };
        let m = Motion { length_ms: 1700, keys: vec![key(0, -205), key(1000, 885), key(1400, Key::THROUGH), key(1700, 175), key(1800, Key::THROUGH)] }.through_filled();
        let xs: Vec<_> = m.keys.iter().map(Key::x).collect();
        // 885 + (175 − 885) × 400 / 700
        assert_eq!(xs, [Some(-205), Some(885), Some(479), Some(175), None]);
    }
}
