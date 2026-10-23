//! The tables in 月姫's exe (1.0.2's `main`; the fan patch's differs by 65
//! bytes elsewhere): pictures, motions and scene numbers. Addresses are
//! Ghidra's (the image loaded at 0x100000); `docs/games/tsukihime.md`,
//! "The exe", says how each was found.

use super::FormatError;
use super::nso;
use conv_core::ease::eased;
use crate::read::Reader;

const BASE: usize = 0x10_0000;
const PICTURE_NAMES: usize = 0x6E_02A4;
const PICTURE_RECORDS: usize = 0x56_F830;
/// The lookup's limit; 14,713 are used.
const PICTURES: usize = 23_000;
const MOTION_KEY_END: usize = 0x4B_1940;
const MOTION_LENGTH: usize = 0x49_9F40;
/// u64 per motion, indexed by m (`FUN_00199960`).
const MOTION_MASKS: usize = 0x46_AB38;
const MOTION_KEYS: usize = 0x75_CE18;
const MOTIONS: usize = 24_192;
const MOTION_KEY_COUNT: usize = 71_905;
/// `FUN_001999a0(m)`: where motion m loops from (a switch the compiler
/// made into jump tables, so it is run, not read: [`run_switch`]).
const MOTION_LOOP: usize = 0x19_99A0;
const SCENES: usize = 0x45_0D58;
const SCENE_COUNT: usize = 551;
/// `_VPLY`'s voice prefixes (`FUN_00102360`): 55 × {name pointer, character}.
const VOICE_PREFIXES: usize = 0x54_D970;
/// The pointer to each character's bit of the settings' voice rows
/// (`FUN_00129390`: the row is its log2).
const VOICE_MASKS: usize = 0xBE_04D8;
/// The movie table (`FUN_001017c0`): 0x30 bytes an entry, the file's
/// path first (`movie/titlebg1.mp4`), up to a null path.
const MOVIES: usize = 0x50_B890;

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
/// −32766 (`Key::THROUGH`) is resolved by `Motion::through_resolved`.
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
    /// 0x06, how the motion goes on from this key (sticky until a key sets
    /// it again): 2 holds each value until the next key, 1 a curve
    /// (`FUN_001972c0`), others straight (`FUN_00197710`).
    pub fn mode(&self) -> Option<i16> {
        self.field(0x06).filter(|&v| v != Self::THROUGH)
    }
    /// −100001 at 0x14 (an i32): no turn here.
    pub const TURN_UNSET: i32 = -100_001;
    /// −100000 at 0x14: no key for the turn here (as `THROUGH`).
    pub const TURN_THROUGH: i32 = -100_000;
    fn turn_raw(&self) -> i32 {
        (u32::from(self.fields[8] as u16) | u32::from(self.fields[9] as u16) << 16) as i32
    }
    pub fn set_turn(&mut self, v: i32) {
        (self.fields[8], self.fields[9]) = (v as i16, (v >> 16) as i16);
    }
    /// The i32 at 0x14: the layer's turn in 1/100 degrees, clockwise, as
    /// `_STRT`'s (`FUN_00197b80` writes it to the state's 0x284).
    pub fn turn(&self) -> Option<i32> {
        Some(self.turn_raw()).filter(|&v| v != Self::TURN_UNSET && v != Self::TURN_THROUGH)
    }
}

/// `_STMA(layer, m)`'s motion `m` (from 1).
#[derive(Debug, Clone, PartialEq)]
pub struct Motion {
    pub length_ms: u32,
    pub keys: Vec<Key>,
    /// The fields it drives (`FUN_00197b80` applies a field only under
    /// its bits: `Motion::X` …).
    pub mask: u64,
    /// Where it loops from (`FUN_001999a0`; `FUN_00197b80` wraps the time
    /// past the length): 0 it doesn't, 1 the start, else that many ms (one
    /// at or past the length: the start).
    pub loop_from: u32,
}

impl Motion {
    pub const X: u64 = 0x4_0880;
    pub const Y: u64 = 0x510_0000;
    pub const ALPHA: u64 = 1 << 14;
    pub const ZOOM_X: u64 = 0x8020_0000;
    pub const ZOOM_Y: u64 = 0x1_0040_0000;
    pub const TURN: u64 = 0x8_8000;

    /// The value fields (index into `Key::fields`; 8 is the turn's i32 at
    /// 0x14, which takes 9 too); 0 and 1 (easing, mode) are looked for.
    const FIELDS: [usize; 27] = [2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29];

    fn get(k: &Key, f: usize) -> i32 {
        if f == 8 { k.turn_raw() } else { k.fields[f].into() }
    }

    fn put(k: &mut Key, f: usize, v: i32) {
        if f == 8 { k.set_turn(v) } else { k.fields[f] = v as i16 }
    }

    /// Field `f`'s "no key here" and "not set" values.
    fn skips(f: usize) -> (i32, i32) {
        if f == 8 { (Key::TURN_THROUGH, Key::TURN_UNSET) } else { (Key::THROUGH.into(), Key::UNSET.into()) }
    }

    /// Field `f` at `t` ms as `FUN_00197710` works it out: from the last key
    /// at or before `t` that doesn't pass it through to the next, eased once
    /// across the keys between (the easing of the last key before `t` that
    /// gives the field and an easing); held at the value before under mode
    /// 2 and in the stepped fields (0x24, 0x28–0x3C); a key leaving it unset
    /// holds the value before until its own time. `None` before it is set.
    /// Mode 1: `FUN_001972c0`'s cubic through the field's keys (see
    /// [`Motion::spline`]), on the same eased share.
    fn value_at(&self, f: usize, t: u32) -> Option<f64> {
        let (through, unset) = Self::skips(f);
        let v = |k: &Key| Self::get(k, f);
        let held = |end: usize| self.keys[..end].iter().rev().map(v).find(|&x| x != through && x != unset).map(f64::from);
        let Some(end) = self.keys.iter().position(|k| k.time_ms > t) else { return held(self.keys.len()) };
        let p = self.keys[..end].iter().rposition(|k| v(k) != through)?;
        let from = held(p + 1)?;
        let Some(next) = self.keys[end..].iter().find(|k| v(k) != through).filter(|k| v(k) != unset) else { return Some(from) };
        let mode = self.keys[..end].iter().rev().find_map(Key::mode);
        if mode == Some(2) || f == 16 || (18..=28).contains(&f) {
            return Some(from);
        }
        let set = |e: &i16| ![Key::UNSET, Key::THROUGH].contains(e);
        let code = self.keys[..end].iter().rev().find(|k| v(k) != through && set(&k.fields[0])).map_or(0, |k| k.fields[0]);
        let (t0, t1) = (self.keys[p].time_ms, next.time_ms);
        let u = if t1 <= t0 { 1.0 } else { f64::from(t - t0) / f64::from(t1 - t0) };
        let u = super::convert::ease(code.into()).map_or(u, |(e, n)| eased(e, n, u));
        let to = f64::from(v(next));
        if mode == Some(1)
            && let Some((a, b)) = self.spline(f, end)
        {
            // FUN_00197710, in floats as the exe works it out
            let (from, to, u) = (from as f32, to as f32, u as f32);
            return Some(f64::from(from + u * ((to - from) - (b + a + a) + u * (a * 3.0 + u * (b - a)))));
        }
        Some(from + (to - from) * u)
    }

    /// `FUN_001972c0`: a natural cubic spline through field `f`'s values at
    /// the keys, one point a key (equally spaced, whatever the times): not
    /// at a key passing the field through or one repeating the keys on
    /// both sides; a key leaving it unset takes the value before it.
    /// Gives the second derivatives / 6 at the ends of the segment from
    /// key `k − 1` (`k` the first key after the time), or `None` (the
    /// exe's linear way) under 3 points or at the first key.
    fn spline(&self, f: usize, k: usize) -> Option<(f32, f32)> {
        let (through, unset) = Self::skips(f);
        let v = |i: usize| Self::get(&self.keys[i], f);
        let last = self.keys.len().checked_sub(1)?;
        if k == 0 || last < 2 {
            return None;
        }
        // The value at key i, else the one set last before it; the exe takes the layer's
        // own (`current`) when none is, which the converter doesn't know: no spline.
        let value = |i: usize| (0..=i).rev().map(v).find(|&x| x != through && x != unset).map(|x| x as f32);
        let mut points = vec![value(0)?];
        let mut point_of = vec![0; last + 1];
        for (i, point) in point_of.iter_mut().enumerate().take(last).skip(1) {
            if v(i) != through && !(v(i - 1) == v(i) && v(i) == v(i + 1)) {
                points.push(value(i)?);
            }
            *point = points.len() - 1;
        }
        if v(last - 1) != v(last) && v(last) != unset {
            points.push(value(last)?);
        }
        let n = points.len();
        if n < 3 {
            return None;
        }
        // c[i−1] + 4 c[i] + c[i+1] = d[i] − d[i−1], c[0] = c[n−1] = 0, by Thomas's
        // elimination; but the exe takes off diag / diag (about 1) where the row before's
        // right side / diag belongs: its values, so its curve.
        let d: Vec<f32> = points.windows(2).map(|w| w[1] - w[0]).collect();
        let (mut diag, mut rhs) = (vec![0f32; n], vec![0f32; n]);
        (diag[1], rhs[1]) = (4.0, d[1] - d[0]);
        for i in 2..n - 1 {
            rhs[i] = (d[i] - d[i - 1]) - diag[i - 1] * (1.0 / diag[i - 1]);
            diag[i] = 4.0 - 1.0 / diag[i - 1];
        }
        let mut c = vec![0f32; n];
        for i in (1..n - 1).rev() {
            c[i] = (rhs[i] - c[i + 1]) / diag[i];
        }
        let j = point_of[k - 1];
        Some((c[j], c.get(j + 1).copied().unwrap_or(0.0)))
    }

    /// Each `Key::THROUGH` field given the exe's value at its key; and where
    /// the converter's one curve per key would miss a field eased across
    /// such a key (each key's way there eased on its own) or under mode 1's
    /// cubic ([`Motion::spline`]), the way cut into
    /// straight steps of the exe's values (8 a key, fewer past 64 keys).
    fn through_resolved(self) -> Self {
        let through = |k: &Key| Self::FIELDS.iter().any(|&f| Self::get(k, f) == Self::skips(f).0);
        if !self.keys.iter().any(|k| through(k) || k.mode() == Some(1)) {
            return self;
        }
        let exact = |t: u32, k: &mut Key| {
            for f in Self::FIELDS {
                Self::put(k, f, self.value_at(f, t).map_or(Self::skips(f).1, |x| x.round() as i32));
            }
        };
        let mut keys = self.keys.clone();
        for k in &mut keys {
            let passed: Vec<usize> = Self::FIELDS.into_iter().filter(|&f| Self::get(k, f) == Self::skips(f).0).collect();
            for f in passed {
                Self::put(k, f, self.value_at(f, k.time_ms).map_or(Self::skips(f).1, |x| x.round() as i32));
            }
        }
        // What the converter draws (convert.rs `run_motion`): each key reached along the easing last
        // set before it, a field a key leaves unset holding; mode 2 holds, so it is exact at the keys.
        let in_force = |j: usize| self.keys[..=j].iter().rev().find_map(|k| k.easing().filter(|&e| e != Key::THROUGH)).unwrap_or(0);
        let followed = [(2, Self::X), (3, Self::Y), (4, Self::ALPHA), (10, Self::ZOOM_X), (11, Self::ZOOM_Y), (8, Self::TURN)];
        let carried = |f: usize, j: usize| keys[..=j].iter().rev().map(|k| Self::get(k, f)).find(|&x| x != Self::skips(f).1).map(f64::from);
        let pieces = |a: u32, b: u32, n: u32| (1..n).map(move |s| a + (b - a) * s / n);
        let off = |j: usize, n: u32| {
            let (a, b) = (&self.keys[j], &self.keys[j + 1]);
            b.time_ms > a.time_ms + 1
                && self.keys[..=j].iter().rev().find_map(Key::mode) != Some(2)
                && followed.iter().filter(|(_, m)| self.mask & m != 0).any(|&(f, _)| {
                    let (Some(va), Some(vb)) = (carried(f, j), carried(f, j + 1)) else { return false };
                    pieces(a.time_ms, b.time_ms, n).any(|t| {
                        let u = f64::from(t - a.time_ms) / f64::from(b.time_ms - a.time_ms);
                        let u = super::convert::ease(in_force(j).into()).map_or(u, |(e, n)| eased(e, n, u));
                        self.value_at(f, t).is_some_and(|x| (x - (va + (vb - va) * u)).abs() > 1.0)
                    })
                })
        };
        let cut: Vec<bool> = (0..keys.len()).map(|j| j + 1 < keys.len() && off(j, 8)).collect();
        let count = cut.iter().filter(|&&c| c).count() as u32;
        let n = 64u32.saturating_sub(keys.len() as u32).checked_div(count).map_or(1, |n| (n + 1).min(8));
        if n < 2 {
            return Motion { keys, ..self };
        }
        let mut out = Vec::new();
        for (j, mut k) in keys.into_iter().enumerate() {
            // Every key says its easing, as the steps between change the one in force.
            k.fields[0] = if cut[j] { 0 } else { in_force(j) };
            out.push(k);
            if cut[j] {
                for t in pieces(self.keys[j].time_ms, self.keys[j + 1].time_ms, n) {
                    let mut step = Key { time_ms: t, fields: [Key::UNSET; 30] };
                    step.fields[0] = 0;
                    exact(t, &mut step);
                    out.push(step);
                }
            }
        }
        Motion { keys: out, ..self }
    }
}

pub struct Exe {
    image: Vec<u8>,
    /// Each pointer's place and value ([`nso::relative`]).
    pointers: std::collections::HashMap<usize, usize>,
}

impl Exe {
    /// From the NSO; fails unless it is 1.0.2's layout.
    pub fn parse(nso: &[u8]) -> Result<Self, FormatError> {
        let image = nso::image(nso)?;
        let exe = Exe { pointers: nso::relative(&image)?, image };
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

    pub(super) fn i32(&self, address: usize) -> Result<i32, FormatError> {
        Ok(self.u32(address)? as i32)
    }

    /// The pointer at `address` (its relocation), 0 for none.
    pub(super) fn pointer(&self, address: usize) -> usize {
        self.pointers.get(&(address - BASE)).map_or(0, |v| v + BASE)
    }

    /// The NUL-ended string at `address`.
    pub(super) fn string(&self, address: usize) -> Result<String, FormatError> {
        let rest = self.image.get(address - BASE..).ok_or_else(|| FormatError::Bad(format!("no string at {address:#x}")))?;
        Ok(String::from_utf8_lossy(&rest[..rest.iter().position(|&b| b == 0).unwrap_or(rest.len())]).into_owned())
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
                let mask = u64::from_le_bytes(self.bytes(MOTION_MASKS + 8 * (m + 1), 8)?.try_into().unwrap());
                let loop_from = run_switch(&self.image, MOTION_LOOP - BASE, m as u64 + 1)?;
                Ok(Motion { length_ms: self.u32(MOTION_LENGTH + 4 * m)?, keys, mask, loop_from }.through_resolved())
            })
            .collect()
    }

    /// Each voice prefix (a `_VPLY` name's first 3 letters) and its row of
    /// the settings' voices; a prefix not listed is character 0's row.
    pub fn voice_rows(&self) -> Result<(std::collections::HashMap<String, u32>, u32), FormatError> {
        let masks = self.pointer(VOICE_MASKS);
        let row = |c: i32| -> Result<u32, FormatError> { Ok(self.u32(masks + 4 * c as usize)?.max(1).ilog2()) };
        let mut out = std::collections::HashMap::new();
        for i in 0..55 {
            let name = self.string(self.pointer(VOICE_PREFIXES + 16 * i))?;
            out.insert(name, row(self.i32(VOICE_PREFIXES + 16 * i + 8)?)?);
        }
        Ok((out, row(0)?))
    }

    /// The movie table's paths (`movie/<name>.mp4`): `_TOMP(n)` plays
    /// entry n.
    pub fn movies(&self) -> Result<Vec<String>, FormatError> {
        (0..).map(|i| self.pointer(MOVIES + 0x30 * i)).take_while(|&p| p != 0).map(|p| self.string(p)).collect()
    }

    /// Each script's scene number, in `allscr` order: `_JUMP(n)` goes to
    /// the script whose number is `n`.
    pub fn scenes(&self) -> Result<Vec<i16>, FormatError> {
        let raw = self.bytes(SCENES, 2 * SCENE_COUNT)?;
        Ok(raw.as_chunks::<2>().0.iter().map(|&c| i16::from_le_bytes(c)).collect())
    }
}

/// Runs the AArch64 function at `pc` (an offset in `image`) with `x0` =
/// `arg` and returns `w0`: only what a compiled switch uses (moves,
/// compares and branches, adds, `adr`/`adrp`, table loads, `br`, `ret`);
/// anything else is an error.
fn run_switch(image: &[u8], mut pc: usize, arg: u64) -> Result<u32, FormatError> {
    let bad = |what: &str, pc: usize| FormatError::Bad(format!("motion loop function: {what} at {:#x}", pc + BASE));
    let mut x = [0u64; 32];
    x[0] = arg;
    let mut nzcv = (false, false, false, false);
    let sext = |v: u64, bits: u32| ((v << (64 - bits)) as i64 >> (64 - bits)) as u64;
    for _ in 0..1000 {
        let i = Reader(image).u32(pc).map_err(|_| bad("outside the image", pc))?;
        let (rd, rn, rm) = ((i & 31) as usize, ((i >> 5) & 31) as usize, ((i >> 16) & 31) as usize);
        let wide = i >> 31 == 1;
        let fit = |v: u64| if wide { v } else { v & 0xFFFF_FFFF };
        let r = |n: usize| if n == 31 { 0 } else { fit(x[n]) };
        let mut next = pc + 4;
        // what goes into rd, if anything
        let out = match i {
            _ if i == 0xD65F_03C0 => return Ok(x[0] as u32), // ret
            _ if i & 0xFFFF_FC1F == 0xD61F_0000 => {
                next = x[rn] as usize - BASE; // br
                None
            }
            _ if i & 0xFC00_0000 == 0x1400_0000 => {
                next = (pc as u64).wrapping_add(sext(u64::from(i & 0x3FF_FFFF) << 2, 28)) as usize; // b
                None
            }
            _ if i & 0xFF00_0010 == 0x5400_0000 => {
                let (n, z, c, v) = nzcv;
                let taken = match i & 15 {
                    0 => z,
                    1 => !z,
                    8 => c && !z,
                    9 => !(c && !z),
                    10 => n == v,
                    11 => n != v,
                    12 => !z && n == v,
                    13 => z || n != v,
                    _ => return Err(bad("condition", pc)),
                };
                if taken {
                    next = (pc as u64).wrapping_add(sext(u64::from((i >> 5) & 0x7_FFFF) << 2, 21)) as usize;
                }
                None
            }
            _ if i & 0x7F80_0000 == 0x5280_0000 => Some(u64::from((i >> 5) & 0xFFFF) << (16 * ((i >> 21) & 3))), // movz
            _ if i & 0x7F80_0000 == 0x1280_0000 => Some(!(u64::from((i >> 5) & 0xFFFF) << (16 * ((i >> 21) & 3)))), // movn
            _ if i & 0x7F80_0000 == 0x7280_0000 => {
                let sh = 16 * ((i >> 21) & 3);
                Some((r(rd) & !(0xFFFF << sh)) | (u64::from((i >> 5) & 0xFFFF) << sh)) // movk
            }
            _ if i & 0x7F20_FC00 == 0x2A00_0000 && rn == 31 => Some(r(rm)), // mov (orr)
            _ if i & 0xFFFF_FC00 == 0x1300_3C00 => Some(sext(r(rn) & 0xFFFF, 16)), // sxth
            _ if i & 0x9F00_0000 == 0x9000_0000 => {
                let imm = sext(u64::from(((i >> 5) & 0x7_FFFF) << 2 | ((i >> 29) & 3)), 21) << 12;
                Some(((pc + BASE) as u64 & !0xFFF).wrapping_add(imm)) // adrp
            }
            _ if i & 0x9F00_0000 == 0x1000_0000 => Some(((pc + BASE) as u64).wrapping_add(sext(u64::from(((i >> 5) & 0x7_FFFF) << 2 | ((i >> 29) & 3)), 21))), // adr
            _ if i & 0x7F00_0000 == 0x1100_0000 || i & 0x7F00_0000 == 0x7100_0000 || i & 0x7F00_0000 == 0x5100_0000 || i & 0x7F20_0000 == 0x0B00_0000 || i & 0x7F20_0000 == 0x6B00_0000 => {
                // add/sub (immediate, or a register shifted left), maybe setting the flags
                let b = if i & 0x1F00_0000 == 0x1100_0000 { u64::from((i >> 10) & 0xFFF) << (12 * ((i >> 22) & 1)) } else if (i >> 22) & 3 == 0 { r(rm) << ((i >> 10) & 63) } else { return Err(bad("shift", pc)) };
                let a = if i & 0x1F00_0000 == 0x1100_0000 && rn == 31 { return Err(bad("sp", pc)) } else { r(rn) };
                let sub = (i >> 30) & 1 == 1;
                let v = fit(if sub { a.wrapping_sub(b) } else { a.wrapping_add(b) });
                if (i >> 29) & 1 == 1 {
                    let bits = if wide { 64 } else { 32 };
                    let (sa, sb, sv) = (sext(a, bits) as i64, sext(fit(b), bits) as i64, sext(v, bits) as i64);
                    let carry = if sub { a >= fit(b) } else { v < a };
                    let over = if sub { (sa < 0) != (sb < 0) && (sv < 0) != (sa < 0) } else { (sa < 0) == (sb < 0) && (sv < 0) != (sa < 0) };
                    nzcv = (sv < 0, v == 0, carry, over);
                }
                Some(v)
            }
            _ if i & 0xBFE0_0C00 == 0x3860_0800 || i & 0xFFE0_0C00 == 0x7860_0800 => {
                // ldrb / ldrh [xn, xm{, lsl}]
                let half = i >> 30 == 1;
                let at = (x[rn] + (r(rm) << if half && (i >> 12) & 1 == 1 { 1 } else { 0 })) as usize - BASE;
                let v = if half { u64::from(Reader(image).u16(at).map_err(|_| bad("table", pc))?) } else { u64::from(*image.get(at).ok_or_else(|| bad("table", pc))?) };
                Some(v)
            }
            _ => return Err(bad(&format!("instruction {i:#010x}"), pc)),
        };
        if let Some(v) = out.filter(|_| rd != 31) {
            x[rd] = fit(v);
        }
        pc = next;
    }
    Err(bad("no return", pc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_1_is_the_exes_spline() {
        // research/typemoon/spline_exe.py <main> 2577 4250 --keys --fields=3,5: FUN_00197710's
        // values (exe field f is Key::fields[f − 1] here), x and alpha; 4250 has an unset key.
        type Samples = &'static [(usize, u32, f64)];
        let motions: [(Vec<Key>, Samples); 2] = [
            // motion 2577
            (
                vec![
                Key { time_ms: 0, fields: [0, 1, 252, 171, 0, -32767, -32767, 0, 3139, 0, 169, 169, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 1, 255, 255, 255, 22, 22, -32767, -32767, -32767, 2766, 0] },
                Key { time_ms: 650, fields: [3, -32767, 248, 166, 175, -32767, -32767, 0, 31071, -2, 346, 346, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 2600, fields: [0, -32767, 239, 157, 0, -32767, -32767, 0, 31071, -2, 454, 454, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                ],
                &[(3, 0, 252.0000), (3, 81, 251.6549), (3, 162, 251.2953), (3, 243, 250.9066), (3, 325, 250.4688), (3, 406, 249.9777), (3, 487, 249.4139), (3, 568, 248.7628), (3, 650, 248.0000), (3, 893, 246.2921), (3, 1137, 244.5466), (3, 1381, 242.9310), (3, 1625, 241.5430), (3, 1868, 240.4409), (3, 2112, 239.6416), (3, 2356, 239.1605), (3, 2600, 239.0000), (5, 0, 0.0000), (5, 81, 32.5422), (5, 162, 64.0685), (5, 243, 93.5628), (5, 325, 120.3125), (5, 406, 142.6386), (5, 487, 159.8725), (5, 568, 170.9980), (5, 650, 175.0000), (5, 893, 161.7800), (5, 1137, 132.1521), (5, 1381, 97.3593), (5, 1625, 64.2578), (5, 1868, 36.7450), (5, 2112, 16.4184), (5, 2356, 4.1096), (5, 2600, 0.0000)],
            ),
            // motion 4250
            (
                vec![
                Key { time_ms: 0, fields: [0, 1, 361, 129, 30, -32767, -32767, 0, 31071, -2, 754, 754, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 4814, 0] },
                Key { time_ms: 200, fields: [-32767, -32767, 370, 112, 62, -32767, -32767, 0, 31071, -2, 880, 880, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 400, fields: [-32767, -32767, 390, 114, 30, -32767, -32767, 0, 31071, -2, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 600, fields: [-32767, -32767, 378, 130, -32767, -32767, -32767, 0, 31071, -2, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 800, fields: [-32767, -32767, 361, 129, -32767, -32767, -32767, 0, 31071, -2, 754, 754, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 1000, fields: [-32767, -32767, 370, 112, 62, -32767, -32767, 0, 31071, -2, 880, 880, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 1200, fields: [-32767, -32767, 390, 114, 30, -32767, -32767, 0, 31071, -2, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 1400, fields: [-32767, -32767, 378, 130, -32767, -32767, -32767, 0, 31071, -2, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 1600, fields: [-32767, 2, 361, 129, -32767, -32767, -32767, 0, 31071, -2, 754, 754, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 2500, fields: [-32767, 1, -32767, -32767, -32767, -32767, -32767, 0, 31071, -2, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 2700, fields: [-32767, 0, 348, 125, 78, -32767, -32767, 0, 31071, -2, 802, 802, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 2850, fields: [-32767, 1, 361, 129, 30, -32767, -32767, 0, 31071, -2, 754, 754, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 3000, fields: [-32767, 0, 333, 122, 62, -32767, -32767, 0, 31071, -2, 802, 802, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                Key { time_ms: 3250, fields: [-32767, 1, 361, 129, 30, -32767, -32767, 0, 31071, -2, 754, 754, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, -32767, 0] },
                ],
                &[(3, 0, 361.0000), (3, 25, 361.5410), (3, 50, 362.1376), (3, 75, 362.8454), (3, 100, 363.7202), (3, 125, 364.8173), (3, 150, 366.1926), (3, 175, 367.9016), (3, 200, 370.0000), (3, 225, 372.5092), (3, 250, 375.3141), (3, 275, 378.2657), (3, 300, 381.2146), (3, 325, 384.0117), (3, 350, 386.5077), (3, 375, 388.5536), (3, 400, 390.0000), (3, 425, 390.5136), (3, 450, 390.3363), (3, 475, 389.5260), (3, 500, 388.1403), (3, 525, 386.2371), (3, 550, 383.8742), (3, 575, 381.1092), (3, 600, 378.0000), (3, 625, 375.8362), (3, 650, 373.4852), (3, 675, 371.0460), (3, 700, 368.6179), (3, 725, 366.3001), (3, 750, 364.1917), (3, 775, 362.3919), (3, 800, 361.0000), (3, 825, 360.4233), (3, 850, 360.3466), (3, 875, 360.7631), (3, 900, 361.6658), (3, 925, 363.0477), (3, 950, 364.9020), (3, 975, 367.2218), (3, 1000, 370.0000), (3, 1025, 372.4936), (3, 1050, 375.2889), (3, 1075, 378.2359), (3, 1100, 381.1848), (3, 1125, 383.9855), (3, 1150, 386.4882), (3, 1175, 388.5430), (3, 1200, 390.0000), (3, 1225, 390.5335), (3, 1250, 390.3749), (3, 1275, 389.5797), (3, 1300, 388.2034), (3, 1325, 386.3017), (3, 1350, 383.9302), (3, 1375, 381.1444), (3, 1400, 378.0000), (3, 1425, 375.7917), (3, 1450, 373.3892), (3, 1475, 370.9016), (3, 1500, 368.4377), (3, 1525, 366.1067), (3, 1550, 364.0174), (3, 1575, 362.2789), (3, 1600, 361.0000), (3, 1712, 361.0000), (3, 1825, 361.0000), (3, 1937, 361.0000), (3, 2050, 361.0000), (3, 2162, 361.0000), (3, 2275, 361.0000), (3, 2387, 361.0000), (3, 2500, 361.0000), (3, 2525, 359.4108), (3, 2550, 357.4033), (3, 2575, 355.1815), (3, 2600, 352.9495), (3, 2625, 350.9114), (3, 2650, 349.2711), (3, 2675, 348.2326), (3, 2700, 348.0000), (3, 2718, 349.5600), (3, 2737, 351.2067), (3, 2756, 352.8533), (3, 2775, 354.5000), (3, 2793, 356.0600), (3, 2812, 357.7067), (3, 2831, 359.3533), (3, 2850, 361.0000), (3, 2868, 358.9152), (3, 2887, 355.6325), (3, 2906, 351.5973), (3, 2925, 347.1746), (3, 2943, 342.9580), (3, 2962, 338.8285), (3, 2981, 335.3874), (3, 3000, 333.0000), (3, 3031, 336.4720), (3, 3062, 339.9440), (3, 3093, 343.4160), (3, 3125, 347.0000), (3, 3156, 350.4720), (3, 3187, 353.9440), (3, 3218, 357.4160), (3, 3250, 361.0000), (5, 0, 30.0000), (5, 25, 36.2553), (5, 50, 42.2958), (5, 75, 47.9068), (5, 100, 52.8733), (5, 125, 56.9807), (5, 150, 60.0142), (5, 175, 61.7589), (5, 200, 62.0000), (5, 225, 60.6126), (5, 250, 57.8308), (5, 275, 53.9786), (5, 300, 49.3800), (5, 325, 44.3588), (5, 350, 39.2392), (5, 375, 34.3449), (5, 400, 30.0000), (5, 425, 30.0000), (5, 450, 30.0000), (5, 475, 30.0000), (5, 500, 30.0000), (5, 525, 30.0000), (5, 550, 30.0000), (5, 575, 30.0000), (5, 600, 30.0000), (5, 625, 30.0000), (5, 650, 30.0000), (5, 675, 30.0000), (5, 700, 30.0000), (5, 725, 30.0000), (5, 750, 30.0000), (5, 775, 30.0000), (5, 800, 30.0000), (5, 825, 33.6635), (5, 850, 38.2144), (5, 875, 43.2570), (5, 900, 48.3958), (5, 925, 53.2351), (5, 950, 57.3793), (5, 975, 60.4328), (5, 1000, 62.0000), (5, 1025, 60.8966), (5, 1050, 58.2627), (5, 1075, 54.4497), (5, 1100, 49.8092), (5, 1125, 44.6926), (5, 1150, 39.4512), (5, 1175, 34.4365), (5, 1200, 30.0000), (5, 1225, 30.0000), (5, 1250, 30.0000), (5, 1275, 30.0000), (5, 1300, 30.0000), (5, 1325, 30.0000), (5, 1350, 30.0000), (5, 1375, 30.0000), (5, 1400, 30.0000), (5, 1425, 30.0000), (5, 1450, 30.0000), (5, 1475, 30.0000), (5, 1500, 30.0000), (5, 1525, 30.0000), (5, 1550, 30.0000), (5, 1575, 30.0000), (5, 1600, 30.0000), (5, 1712, 30.0000), (5, 1825, 30.0000), (5, 1937, 30.0000), (5, 2050, 30.0000), (5, 2162, 30.0000), (5, 2275, 30.0000), (5, 2387, 30.0000), (5, 2500, 30.0000), (5, 2525, 35.6586), (5, 2550, 42.6880), (5, 2575, 50.4516), (5, 2600, 58.3128), (5, 2625, 65.6348), (5, 2650, 71.7811), (5, 2675, 76.1151), (5, 2700, 78.0000), (5, 2718, 72.2400), (5, 2737, 66.1600), (5, 2756, 60.0800), (5, 2775, 54.0000), (5, 2793, 48.2400), (5, 2812, 42.1600), (5, 2831, 36.0800), (5, 2850, 30.0000), (5, 2868, 30.7710), (5, 2887, 33.5109), (5, 2906, 37.7108), (5, 2925, 42.8432), (5, 2943, 48.0879), (5, 2962, 53.5229), (5, 2981, 58.3361), (5, 3000, 62.0000), (5, 3031, 58.0320), (5, 3062, 54.0640), (5, 3093, 50.0960), (5, 3125, 46.0000), (5, 3156, 42.0320), (5, 3187, 38.0640), (5, 3218, 34.0960), (5, 3250, 30.0000)],
            ),
        ];
        for (keys, want) in motions {
            let m = Motion { length_ms: 0, keys, mask: 0, loop_from: 0 };
            for &(f, t, v) in want {
                let got = m.value_at(f - 1, t).unwrap();
                assert!((got - v).abs() < 0.01, "field {f} at {t}: {got}, the exe {v}");
            }
        }
    }

    #[test]
    fn runs_a_compiled_switch() {
        // m > 5: a jump table of bytes at 0x100048 (m 6: 100 ms, m 7: from
        // the start, else none); m <= 5: none.
        let code: [u32; 13] = [
            0x2A0003E8, // mov w8, w0
            0x2A1F03E0, // mov w0, wzr
            0x7100151F, // cmp w8, #5
            0x5400004D, // b.le +8 (ret)
            0x14000002, // b +8
            0xD65F03C0, // ret
            0x51001909, // sub w9, w8, #6
            0x90000008, // adrp x8, 0x100000
            0x91012108, // add x8, x8, #0x48
            0x3869690B, // ldrb w11, [x8, x9]
            0x1000006A, // adr x10, +12
            0x8B0B094A, // add x10, x10, x11, lsl #2
            0xD61F0140, // br x10
        ];
        let mut image: Vec<u8> = code.iter().flat_map(|i| i.to_le_bytes()).collect();
        // the cases, from 0x100034 on: 0 -> w0 = 100, 2 -> w0 = 1, 4 -> ret (0)
        for i in [0x52800C80u32, 0xD65F03C0, 0x52800020, 0xD65F03C0, 0xD65F03C0] {
            image.extend(i.to_le_bytes());
        }
        image.resize(0x48, 0);
        image.extend([0, 2, 4]);
        let run = |m| run_switch(&image, 0, m).unwrap();
        assert_eq!([3, 5, 6, 7, 8].map(run), [0, 0, 100, 1, 0]);
        assert!(run_switch(&image, 0x48, 0).is_err());
    }

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
        let m = Motion { mask: 0, length_ms: 1700, keys: vec![key(0, -205), key(1000, 885), key(1400, Key::THROUGH), key(1700, 175), key(1800, Key::THROUGH)], loop_from: 0 }.through_resolved();
        let xs: Vec<_> = m.keys.iter().map(Key::x).collect();
        // 885 + (175 − 885) × 400 / 700; the last held
        assert_eq!(xs, [Some(-205), Some(885), Some(479), Some(175), Some(175)]);
    }

    #[test]
    fn a_field_passed_through_is_eased_once_across() {
        let key = |time_ms, easing, x, y| {
            let mut fields = [Key::UNSET; 30];
            (fields[0], fields[2], fields[3]) = (easing, x, y);
            Key { time_ms, fields }
        };
        // easing 3: out quad; x from 0 to 1000 across the key at 500, y to 100 at it
        let keys = vec![key(0, 3, 0, 0), key(500, Key::UNSET, Key::THROUGH, 100), key(1000, Key::UNSET, 1000, Key::UNSET)];
        let m = Motion { mask: Motion::X | Motion::Y, length_ms: 1000, keys, loop_from: 0 }.through_resolved();
        let at = |t| m.keys.iter().find(|k| k.time_ms == t).map(|k| (k.easing(), k.x(), k.y()));
        // x: 1000 × (1 − (1 − t)²) across both; y: 100 × the same to 500, then held. Cut in 8 to
        // 500; from there out quad from 750 to 1000 is the exe's curve already.
        assert_eq!(m.keys.len(), 3 + 7);
        assert_eq!(at(250), Some((Some(0), Some(438), Some(75))));
        assert_eq!(at(500), Some((Some(3), Some(750), Some(100))));
        assert_eq!(at(1000), Some((Some(3), Some(1000), None)));
    }

    #[test]
    fn the_turn_is_an_i32_with_its_own_skip_values() {
        let key = |time_ms, turn| {
            let mut k = Key { time_ms, fields: [Key::UNSET; 30] };
            k.set_turn(turn);
            k
        };
        let m = Motion { mask: Motion::TURN, length_ms: 400, keys: vec![key(0, 0), key(100, Key::TURN_THROUGH), key(200, 36000), key(400, Key::TURN_UNSET)], loop_from: 0 }.through_resolved();
        assert_eq!(m.keys.iter().map(Key::turn).collect::<Vec<_>>(), [Some(0), Some(18000), Some(36000), None]);
    }
}
