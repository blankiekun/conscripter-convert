//! `.cd` compiled scenario scripts used by Xuse's in-house engine
//! (`HD/CDFiles/0000.cd` … in 最果てのイマ COMPLETE).
//!
//! This covers the numbered scripts. `0.cd` and `1.cd` use a different,
//! still unknown encoding and are rejected by [`CdScript::parse`].
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! 0x000  [u8; 32]   unknown (unique per file; not a hash of the contents)
//! 0x020  [u8; 260]  source file name (`*.rio`), Shift-JIS, NUL-padded,
//!                   every byte XORed with 0x16
//! 0x124  u32        n (the label table has n - 1 entries)
//!        u32        code size in bytes (multiple of 12)
//!        u32        pool size in bytes
//!        u32        script id (equals the file number)
//!        u32, u32   reserved (0)
//! 0x13c  label[n-1]: u32 script id, u32 label number, u32 code offset
//!                   (entries for other script ids are cross-script targets)
//!        instr[code size / 12]: u16 opcode, u16 sub, i32 arg, u32 source pos
//!        [u8; pool size]  operand pool; strings in it are a u32 byte length
//!                   followed by Shift-JIS bytes XORed with 0x53
//!        [u8; 16]   MD5 of everything before it
//! ```
//!
//! For most opcodes `arg` is a byte offset of the operand record in the pool.
//! `source pos` never decreases within a file and looks like a byte offset
//! into the original `.rio` source.
//!
//! Known opcodes (all 46,578 message records in the game parse this way):
//!
//! ```text
//! 0x40  message          sub 0x12 / 0x13 (meaning not known yet)
//! 0x41  voiced message   sub 0x13
//!       record: u32 id (0 for 0x40; voice id for 0x41), u32 size of the rest,
//!               u32 line number (counts up within the script),
//!               u32 segment count, then per segment a pool string
//! ```
//!
//! Segments are at most ~25 characters, so they look like pre-wrapped display
//! lines; a short first segment is often the speaker name. Voice ids look like
//! `group * 10000 + n`; how they map to `.xsa` slots is not known yet (neither
//! sorted order nor order of appearance matches the clip lengths).

use md5::{Digest, Md5};
use thiserror::Error;

use crate::read::{Reader, Truncated};

const HEAD_LEN: usize = 32;
const NAME_OFFSET: usize = 0x20;
const NAME_LEN: usize = 260;
const NAME_KEY: u8 = 0x16;
const STRING_KEY: u8 = 0x53;
const HEADER_OFFSET: usize = 0x124;
const LABELS_OFFSET: usize = 0x13c;
const ENTRY_LEN: usize = 12;
const FOOTER_LEN: usize = 16;

#[derive(Debug, Error)]
pub enum CdError {
    #[error(transparent)]
    Truncated(#[from] Truncated),
    #[error("label table count is 0")]
    ZeroCount,
    #[error("code size {0} is not a multiple of 12")]
    CodeSize(usize),
    #[error("sections end at {expected:#x} but the file is {actual:#x} bytes")]
    SizeMismatch { expected: usize, actual: usize },
    #[error("operand at pool offset {offset:#x}: {reason}")]
    BadOperand { offset: usize, reason: &'static str },
}

pub const OP_MESSAGE: u16 = 0x40;
pub const OP_VOICED_MESSAGE: u16 = 0x41;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Voice id for [`OP_VOICED_MESSAGE`], `None` for plain messages.
    pub voice_id: Option<u32>,
    pub line: u32,
    pub segments: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Label {
    pub script: u32,
    pub label: u32,
    /// Byte offset into that script's code (divide by 12 for the index).
    pub code_offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instr {
    pub op: u16,
    pub sub: u16,
    pub arg: i32,
    pub source_pos: u32,
}

#[derive(Debug)]
pub struct CdScript<'a> {
    pub head: [u8; HEAD_LEN],
    /// Source file name, decoded from Shift-JIS.
    pub name: String,
    pub script_id: u32,
    pub reserved: [u32; 2],
    pub labels: Vec<Label>,
    pub code: Vec<Instr>,
    /// Operand pool, still encoded (strings XORed with 0x53).
    pub pool: &'a [u8],
    pub stored_md5: [u8; 16],
    hashed: &'a [u8],
}

impl<'a> CdScript<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CdError> {
        let r = Reader(bytes);
        let head = r.slice(0, HEAD_LEN)?.try_into().unwrap();

        let raw_name: Vec<u8> = r
            .slice(NAME_OFFSET, NAME_LEN)?
            .iter()
            .map(|b| b ^ NAME_KEY)
            .take_while(|&b| b != 0)
            .collect();
        let (name, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_name);

        let count = r.u32(HEADER_OFFSET)? as usize;
        let code_size = r.u32(HEADER_OFFSET + 4)? as usize;
        let pool_size = r.u32(HEADER_OFFSET + 8)? as usize;
        let script_id = r.u32(HEADER_OFFSET + 12)?;
        let reserved = [r.u32(HEADER_OFFSET + 16)?, r.u32(HEADER_OFFSET + 20)?];
        if count == 0 {
            return Err(CdError::ZeroCount);
        }
        if !code_size.is_multiple_of(ENTRY_LEN) {
            return Err(CdError::CodeSize(code_size));
        }

        // Check the total size before allocating anything sized from the header.
        let code_start = (count - 1)
            .checked_mul(ENTRY_LEN)
            .and_then(|n| n.checked_add(LABELS_OFFSET))
            .ok_or(r.truncated(LABELS_OFFSET, usize::MAX))?;
        let pool_start = code_start
            .checked_add(code_size)
            .ok_or(r.truncated(code_start, usize::MAX))?;
        let footer_start = pool_start
            .checked_add(pool_size)
            .ok_or(r.truncated(pool_start, usize::MAX))?;
        let expected = footer_start + FOOTER_LEN;
        if expected != bytes.len() {
            return Err(CdError::SizeMismatch { expected, actual: bytes.len() });
        }

        let labels = (0..count - 1)
            .map(|i| {
                let o = LABELS_OFFSET + i * ENTRY_LEN;
                Ok(Label { script: r.u32(o)?, label: r.u32(o + 4)?, code_offset: r.u32(o + 8)? })
            })
            .collect::<Result<_, Truncated>>()?;
        let code = (0..code_size / ENTRY_LEN)
            .map(|i| {
                let o = code_start + i * ENTRY_LEN;
                Ok(Instr {
                    op: r.u16(o)?,
                    sub: r.u16(o + 2)?,
                    arg: r.u32(o + 4)? as i32,
                    source_pos: r.u32(o + 8)?,
                })
            })
            .collect::<Result<_, Truncated>>()?;

        Ok(Self {
            head,
            name: name.into_owned(),
            script_id,
            reserved,
            labels,
            code,
            pool: r.slice(pool_start, pool_size)?,
            stored_md5: r.slice(footer_start, FOOTER_LEN)?.try_into().unwrap(),
            hashed: &bytes[..footer_start],
        })
    }

    pub fn md5_ok(&self) -> bool {
        Md5::digest(self.hashed)[..] == self.stored_md5
    }

    /// Reads a length-prefixed pool string at `offset`, or `None` if the
    /// bytes there don't form a valid Shift-JIS string.
    pub fn pool_string(&self, offset: usize) -> Option<String> {
        let r = Reader(self.pool);
        let len = r.u32(offset).ok()? as usize;
        let raw: Vec<u8> = r.slice(offset + 4, len).ok()?.iter().map(|b| b ^ STRING_KEY).collect();
        encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&raw)
            .map(|s| s.into_owned())
    }

    /// Decodes the message record of a message instruction, or returns
    /// `None` for other opcodes.
    pub fn message(&self, ins: &Instr) -> Option<Result<Message, CdError>> {
        matches!(ins.op, OP_MESSAGE | OP_VOICED_MESSAGE).then(|| self.parse_message(ins))
    }

    fn parse_message(&self, ins: &Instr) -> Result<Message, CdError> {
        let offset = ins.arg as u32 as usize;
        let bad = |reason| CdError::BadOperand { offset, reason };
        let r = Reader(self.pool);
        let id = r.u32(offset)?;
        let size = r.u32(offset + 4)? as usize;
        let line = r.u32(offset + 8)?;
        let count = r.u32(offset + 12)? as usize;
        let end = (offset + 8).checked_add(size).ok_or(bad("size overflows"))?;
        let mut pos = offset + 16;
        let mut segments = Vec::new();
        for _ in 0..count {
            if pos >= end {
                return Err(bad("more segments than the record size allows"));
            }
            let len = r.u32(pos)? as usize;
            segments.push(self.pool_string(pos).ok_or(bad("segment is not valid Shift-JIS"))?);
            pos += 4 + len;
        }
        if pos != end {
            return Err(bad("segments don't fill the record"));
        }
        let voice_id = (ins.op == OP_VOICED_MESSAGE).then_some(id);
        Ok(Message { voice_id, line, segments })
    }

    /// Heuristically finds the strings in the pool: positions where a
    /// plausible length is followed by valid Shift-JIS that is mostly
    /// Japanese text. Use until the operand encoding is understood.
    pub fn scan_strings(&self) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i + 4 <= self.pool.len() {
            let len = u32::from_le_bytes(self.pool[i..i + 4].try_into().unwrap()) as usize;
            if (2..=4096).contains(&len)
                && let Some(s) = self.pool_string(i)
                && looks_japanese(&s)
            {
                out.push((i, s));
                i += 4 + len;
                continue;
            }
            i += 1;
        }
        out
    }
}

fn looks_japanese(s: &str) -> bool {
    let total = s.chars().count();
    let jp = s
        .chars()
        .filter(|c| matches!(c, '\u{3000}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}' | '\u{ff00}'..='\u{ffef}'))
        .count();
    total > 0 && jp * 2 >= total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sjis(s: &str) -> Vec<u8> {
        encoding_rs::SHIFT_JIS.encode(s).0.into_owned()
    }

    fn build(name: &str, labels: &[Label], code: &[Instr], pool: &[u8]) -> Vec<u8> {
        let mut out = vec![0xAB; HEAD_LEN];
        let mut name_field = sjis(name);
        name_field.resize(NAME_LEN, 0);
        out.extend(name_field.iter().map(|b| b ^ NAME_KEY));
        for v in [labels.len() as u32 + 1, (code.len() * 12) as u32, pool.len() as u32, 7, 0, 0] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for l in labels {
            for v in [l.script, l.label, l.code_offset] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for i in code {
            out.extend_from_slice(&i.op.to_le_bytes());
            out.extend_from_slice(&i.sub.to_le_bytes());
            out.extend_from_slice(&i.arg.to_le_bytes());
            out.extend_from_slice(&i.source_pos.to_le_bytes());
        }
        out.extend_from_slice(pool);
        let md5 = Md5::digest(&out);
        out.extend_from_slice(&md5);
        out
    }

    fn pool_entry(s: &str) -> Vec<u8> {
        let raw = sjis(s);
        let mut out = (raw.len() as u32).to_le_bytes().to_vec();
        out.extend(raw.iter().map(|b| b ^ STRING_KEY));
        out
    }

    #[test]
    fn parses_sections() {
        let labels = [Label { script: 7, label: 0, code_offset: 0 }, Label { script: 2, label: 5, code_offset: 96 }];
        let code = [
            Instr { op: 0x10, sub: 0xffff, arg: 0, source_pos: 10 },
            Instr { op: 0x22, sub: 3, arg: -1, source_pos: 30 },
        ];
        let mut pool = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let text_at = pool.len();
        pool.extend(pool_entry("テストの文章です。"));
        let bytes = build("01_テスト.rio", &labels, &code, &pool);

        let cd = CdScript::parse(&bytes).unwrap();
        assert_eq!(cd.name, "01_テスト.rio");
        assert_eq!(cd.script_id, 7);
        assert_eq!(cd.labels, labels);
        assert_eq!(cd.code, code);
        assert!(cd.md5_ok());
        assert_eq!(cd.pool_string(text_at).as_deref(), Some("テストの文章です。"));
        assert_eq!(cd.scan_strings(), vec![(text_at, "テストの文章です。".to_string())]);
    }

    fn message_record(id: u32, line: u32, segments: &[&str]) -> Vec<u8> {
        let body: Vec<u8> = segments.iter().flat_map(|s| pool_entry(s)).collect();
        let mut out = Vec::new();
        for v in [id, (body.len() + 8) as u32, line, segments.len() as u32] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend(body);
        out
    }

    #[test]
    fn decodes_messages() {
        let mut pool = message_record(0, 3, &["ナレーションの行"]);
        let voiced_at = pool.len();
        pool.extend(message_record(120005, 4, &["名前", "一行目の台詞", "二行目"]));
        let code = [
            Instr { op: OP_MESSAGE, sub: 0x12, arg: 0, source_pos: 0 },
            Instr { op: OP_VOICED_MESSAGE, sub: 0x13, arg: voiced_at as i32, source_pos: 9 },
            Instr { op: 0x8e, sub: 1, arg: 0, source_pos: 20 },
        ];
        let bytes = build("m.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();

        let plain = cd.message(&cd.code[0]).unwrap().unwrap();
        assert_eq!(plain, Message { voice_id: None, line: 3, segments: vec!["ナレーションの行".into()] });
        let voiced = cd.message(&cd.code[1]).unwrap().unwrap();
        assert_eq!(voiced.voice_id, Some(120005));
        assert_eq!(voiced.segments, ["名前", "一行目の台詞", "二行目"]);
        assert!(cd.message(&cd.code[2]).is_none());
    }

    #[test]
    fn rejects_bad_message_records() {
        let mut pool = message_record(0, 1, &["あいう"]);
        pool[4] += 1; // size one byte too large
        let code = [Instr { op: OP_MESSAGE, sub: 0x12, arg: 0, source_pos: 0 }];
        let bytes = build("m.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        assert!(cd.message(&cd.code[0]).unwrap().is_err());

        let code = [Instr { op: OP_MESSAGE, sub: 0x12, arg: 1_000_000, source_pos: 0 }];
        let bytes = build("m.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        assert!(cd.message(&cd.code[0]).unwrap().is_err());
    }

    #[test]
    fn detects_corruption() {
        let mut bytes = build("a.rio", &[], &[Instr { op: 1, sub: 0, arg: 0, source_pos: 0 }], &[]);
        let n = bytes.len();
        bytes[n - 20] ^= 1;
        assert!(!CdScript::parse(&bytes).unwrap().md5_ok());

        assert!(matches!(CdScript::parse(&bytes[..n - 1]), Err(CdError::SizeMismatch { .. })));
        assert!(CdScript::parse(&bytes[..100]).is_err());

        // Huge header values must fail cleanly instead of allocating.
        let mut huge = bytes.clone();
        huge[HEADER_OFFSET..HEADER_OFFSET + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(CdScript::parse(&huge).is_err());
    }
}
