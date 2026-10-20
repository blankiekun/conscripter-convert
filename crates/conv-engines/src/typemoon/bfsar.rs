//! Nintendo's sound archive (`knife1.bfsar`, the sound effects) and the
//! FWSD/FWAR files in it, as far as the game needs: every wave sound
//! (`WSD_*`) by name, with its FWAV. The music and voices are streams
//! (`STRM_*`) whose files are `stream/*.opus` outside the archive.
//!
//! All these files share one frame: magic, BOM, u16 header size, u32
//! version, u32 size, u16 block count at 0x10, then 12-byte block
//! entries at 0x14 (u16 kind, u16, u32 offset, u32 size). Inside a
//! block, a reference is u16 kind, u16, u32 offset; a table is u32
//! count then references, offsets from the table's start.
//!
//! The chain, as Switch Toolbox and vgmstream read it: STRG's string
//! table names the sounds; INFO lists sounds (u32 file, ..., at 0x0C a
//! reference to the wave sound's details (u32 index in its set), at 0x14
//! option flags, bit 0: u32 string ID at 0x18), wave archives (u32 file)
//! and files (internal: at 0x0C u32 kind, u32 offset into FILE's body,
//! u32 size). A sound's file is an FWSD: INFO with a table of wave IDs
//! (u32 wave archive ID, u32 index) and a table of wave sounds, each
//! with, third, a table of notes (u32 index into the wave IDs). The wave
//! archive is an FWAR: INFO, a table of (u32 kind, u32 offset into FILE's
//! body, u32 size) of FWAVs.

use super::FormatError;
use crate::read::Reader;

/// The body (past its 8-byte block header) of block `kind`.
fn block(r: &Reader, kind: u16) -> Result<usize, FormatError> {
    for i in 0..usize::from(r.u16(0x10)?) {
        if r.u16(0x14 + 12 * i)? == kind {
            return Ok(r.u32(0x18 + 12 * i)? as usize + 8);
        }
    }
    Err(FormatError::Bad(format!("{} has no block {kind:#06x}", String::from_utf8_lossy(r.slice(0, 4)?))))
}

/// The targets of a reference table at `at`.
fn table(r: &Reader, at: usize, stride: usize) -> Result<Vec<usize>, FormatError> {
    (0..r.u32(at)? as usize).map(|i| Ok(at + r.u32(at + 8 + stride * i)? as usize)).collect()
}

/// A file inside a sized reference table entry: (offset, size) from `base`.
fn sized(r: &Reader, entry: usize, base: usize) -> Result<std::ops::Range<usize>, FormatError> {
    let start = base + r.u32(entry)? as usize;
    Ok(start..start + r.u32(entry + 4)? as usize)
}

/// Every sound's name, in the archive's order (its item ID less
/// `0x01000000`); empty for a sound without one. `_MPLY(n)` plays sound
/// `n − 1`, a stream (`STRM_MOON01_BGM01`, the file `stream/MOON01_BGM01.opus`).
pub fn sound_names(data: &[u8]) -> Result<Vec<String>, FormatError> {
    if !data.starts_with(b"FSAR") {
        return Err(FormatError::BadMagic("FSAR"));
    }
    let r = Reader(data);
    let strings = block(&r, 0x2000)? + r.u32(block(&r, 0x2000)? + 4)? as usize;
    let info = block(&r, 0x2001)?;
    let sounds = table(&r, info + r.u32(info + 4)? as usize, 8)?;
    sounds
        .iter()
        .map(|&s| {
            if r.u32(s + 0x14)? & 1 == 0 {
                return Ok(String::new());
            }
            let entry = strings + 4 + 12 * r.u32(s + 0x18)? as usize;
            let b = r.slice(strings + r.u32(entry + 4)? as usize, r.u32(entry + 8)? as usize)?;
            Ok(String::from_utf8_lossy(b.strip_suffix(b"\0").unwrap_or(b)).into_owned())
        })
        .collect()
}

/// The loop of each looping stream, by its name less `STRM_`
/// (`MOON01_BGM01`): (start, end) in samples, the end exclusive. A
/// stream's details (INFO sound at 0x0C, kind 0x2201) hold at 0x18 a
/// reference (kind 0x2210) to its extension: u32 stream type (byte 1:
/// loops), u32 loop start, u32 loop end; 131 streams loop, 105 of them
/// after an intro. The positions count the decoded samples after the Opus
/// pre-skip, and the end is one past what it says: the decoded music
/// repeats itself from `end − 1` (59 of the 80 tracks with enough after
/// their end to compare, the rest a fade or noise).
pub fn stream_loops(data: &[u8]) -> Result<std::collections::HashMap<String, (u32, u32)>, FormatError> {
    let names = sound_names(data)?;
    let r = Reader(data);
    let info = block(&r, 0x2001)?;
    let sounds = table(&r, info + r.u32(info + 4)? as usize, 8)?;
    let mut out = std::collections::HashMap::new();
    for (&s, name) in sounds.iter().zip(&names) {
        if r.u16(s + 0x0C)? != 0x2201 {
            continue;
        }
        let details = s + r.u32(s + 0x10)? as usize;
        if r.u16(details + 0x18)? != 0x2210 {
            continue; // no extension: no loop
        }
        let ext = details + r.u32(details + 0x1C)? as usize;
        if r.u32(ext)? >> 8 & 0xFF == 0 {
            continue;
        }
        let (start, end) = (r.u32(ext + 4)?, r.u32(ext + 8)?.saturating_sub(1));
        if let (Some(n), true) = (name.strip_prefix("STRM_"), end > start) {
            out.insert(n.to_string(), (start, end));
        }
    }
    Ok(out)
}

pub struct WaveSound<'a> {
    pub name: String,
    pub fwav: &'a [u8],
}

pub fn wave_sounds(data: &[u8]) -> Result<Vec<WaveSound<'_>>, FormatError> {
    if !data.starts_with(b"FSAR") {
        return Err(FormatError::BadMagic("FSAR"));
    }
    let r = Reader(data);
    let strings = block(&r, 0x2000)? + r.u32(block(&r, 0x2000)? + 4)? as usize;
    let name = |id: usize| -> Result<String, FormatError> {
        let entry = strings + 4 + 12 * id;
        let s = r.slice(strings + r.u32(entry + 4)? as usize, r.u32(entry + 8)? as usize)?;
        Ok(String::from_utf8_lossy(s.strip_suffix(b"\0").unwrap_or(s)).into_owned())
    };
    let info = block(&r, 0x2001)?;
    let files_base = block(&r, 0x2002)?;
    let list = |n: usize| -> Result<Vec<usize>, FormatError> { table(&r, info + r.u32(info + 8 * n + 4)? as usize, 8) };
    let (sounds, archives, files) = (list(0)?, list(3)?, list(6)?);
    let file = |id: u32| -> Result<&[u8], FormatError> {
        let at = *files.get((id & 0xFF_FFFF) as usize).ok_or_else(|| FormatError::Bad(format!("no file {id:#x}")))?;
        if r.u16(at)? != 0x220C {
            return Err(FormatError::Bad(format!("file {id:#x} is not inside the archive")));
        }
        let inner = at + r.u32(at + 4)? as usize;
        let range = sized(&r, inner + 4, files_base)?;
        Ok(r.slice(range.start, range.len())?)
    };

    let mut out = Vec::new();
    for &s in &sounds {
        if r.u16(s + 0x0C)? != 0x2202 {
            continue; // streams and sequences
        }
        let index = r.u32(s + r.u32(s + 0x10)? as usize)? as usize;
        let name = if r.u32(s + 0x14)? & 1 != 0 { name(r.u32(s + 0x18)? as usize)? } else { format!("{index}") };
        let wsd = Reader(file(r.u32(s)?)?);
        if !wsd.0.starts_with(b"FWSD") {
            return Err(FormatError::BadMagic("FWSD"));
        }
        let body = block(&wsd, 0x6800)?;
        let wave_ids = body + wsd.u32(body + 4)? as usize;
        let sound = *table(&wsd, body + wsd.u32(body + 12)? as usize, 8)?
            .get(index)
            .ok_or_else(|| FormatError::Bad(format!("{name}: no wave sound {index}")))?;
        let notes = table(&wsd, sound + wsd.u32(sound + 0x14)? as usize, 8)?;
        let note = *notes.first().ok_or_else(|| FormatError::Bad(format!("{name}: no notes")))?;
        let wave = wave_ids + 4 + 8 * wsd.u32(note)? as usize;
        let (archive, wave_index) = (wsd.u32(wave)?, wsd.u32(wave + 4)? as usize);
        let at = *archives.get((archive & 0xFF_FFFF) as usize).ok_or_else(|| FormatError::Bad(format!("{name}: no wave archive {archive:#x}")))?;
        let war = Reader(file(r.u32(at)?)?);
        if !war.0.starts_with(b"FWAR") {
            return Err(FormatError::BadMagic("FWAR"));
        }
        let (war_info, war_files) = (block(&war, 0x6800)?, block(&war, 0x6801)?);
        if wave_index >= war.u32(war_info)? as usize {
            return Err(FormatError::Bad(format!("{name}: no wave {wave_index} in its archive")));
        }
        let range = sized(&war, war_info + 4 + 12 * wave_index + 4, war_files)?;
        out.push(WaveSound { name, fwav: war.slice(range.start, range.len())? });
    }
    Ok(out)
}
