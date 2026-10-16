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
//! ```text
//! 0x46  hyperlink on the next message (several may precede one message)
//!       record (18 bytes): u16 row, u16 column, u16 length in characters,
//!               u32 style (bit 15 set; low bits 7 or 18 in this game),
//!               u32 target, u32 reserved (0)
//! ```
//!
//! ```text
//! 0x54  background       arg = background number, 1-based (0 = none; the
//!                        handler skips it). Background n is image n - 1
//!                        counted across HD/bg0000.004, bg0001.004, … (64 each)
//! 0x8e  show sprites     sub = number of entries; record = sub × 44 bytes:
//!                        u16 mode, u16 character, u16 sprite number (1-based,
//!                        across HD/bu*.004, 128 each), u16 depth,
//!                        u16 x, u16 y, then fields not decoded yet (the tail
//!                        is mostly uninitialised memory)
//! 0x57  play BGM          arg = track (1-based, HD/bm0000.bin slot arg - 1),
//!                        sub = fade-in in ms (the engine caps it at 30000)
//! 0x58  stop BGM          arg = track, or 0 for all; sub = fade-out in ms
//! 0x5a  play SE           record (9 bytes, packed): u32 fade-in ms, u8 loop flag,
//!                        u32 sound number (1-based across HD/k0000.bin,
//!                        k0001.bin)
//! 0x5b  stop SE           arg = sound number, or 0 for all; sub = fade in ms
//! ```
//!
//! ```text
//! 0x88  effect preset    defines one module of an effect preset
//!                        (`AdvObjectScript_Filter.cpp`, handler 0x415b50):
//!                        u32 preset (1-based), u32 slot (1-based; 0 clears
//!                        the preset), u32 module field, u32 kind (module
//!                        type = kind + 0x63; 8 = CModule_Filter_Color),
//!                        u16, u16, then the kind's data (colour: R, G, B)
//! 0x90  apply preset     record: u16 character (+ 2 unused bytes), u32
//!                        module field, u32 preset (0 removes it). Attaches
//!                        the preset to the character's bust-up object
//!                        (handler 0x434960, module type 1 = CModule_Preset)
//! ```
//!
//! All four sound opcodes go through the engine's sound request routine (0x47aab0), which
//! takes a channel (0 = BGM, 1 = SE), a 1-based number and a play/stop flag.
//!
//! Link rows and columns are 1-based positions in the text window relative to
//! where the next message starts: row 1 continues the line the cursor is on
//! (the engine adds the current column), row 2+ are the following lines. For
//! 175 of the game's 214 links `target` is a valid index into this script's
//! label table, which matches how the engine looks it up (`0x4bab70` copies a
//! 12-byte entry); the rest are unresolved.
//!
//! Segments are at most ~25 characters, so they look like pre-wrapped display
//! lines; a short first segment is often the speaker name. Voice ids look like
//! `group * 10000 + n`; how they map to `.xsa` slots is not known yet (neither
//! sorted order nor order of appearance matches the clip lengths).

use std::cell::RefCell;

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
pub const OP_LINK: u16 = 0x46;
/// Ruby (furigana) for the next message (handler from `TextRuby.cpp`).
pub const OP_RUBY: u16 = 0x44;
/// Size and colour for characters of the next message (handler 0x477990).
pub const OP_TEXT_STYLE: u16 = 0x49;
/// Hanging indent for the next message (handler 0x494750): arg's high 16
/// bits are where wrapped lines start, in half-width units of the first
/// line (the chat log's handle and the space after it; the game puts it
/// only before messages that wrap); the low bits are 1 (once 2).
pub const OP_TEXT_INDENT: u16 = 0x4e;
pub const OP_BACKGROUND: u16 = 0x54;
pub const OP_SPRITES: u16 = 0x8e;
const SPRITE_ENTRY_LEN: usize = 44;
pub const OP_PRESET_MODULE: u16 = 0x88;
pub const OP_APPLY_PRESET: u16 = 0x90;
/// Filter on the whole screen (handler 0x415a30), from 0x88's modules.
pub const OP_SCREEN_MODULE: u16 = 0x70;
/// 0x88's module kinds (module type = kind + 0x63; (type, creator) pairs
/// at 0x501428, class names from each class's vtable). Each kind's data
/// follows the u16 blend and u16 opacity. Others: 7 RotateScale,
/// 10 LensFlare, 11 Mosaic (u32 width, height), 12 Zigzag (i32 centre x,
/// y, direction, distance, frequency, begin angle, reduction; one use).
/// Smooth (draw 0x43e670): the mean of the pixels up to u8 x, u8 y
/// (each clamped to 0..6) away, a box clipped to the picture.
pub const PRESET_KIND_SMOOTH: u32 = 1;
pub const PRESET_KIND_NEGATIVE: u32 = 2;
/// HueSatura (draw 0x436a40): i16 hue, i16 saturation, applied in HLS on
/// 0-255 scales (0x4367e0): the hue is added; a negative saturation
/// scales s by (256 + n) / 256, a positive one moves it toward 255 by
/// n / 256.
pub const PRESET_KIND_HUE_SATURATION: u32 = 3;
/// Monotone: u32 palette number (`pal%04u.pal` in `HD/pal.arc`).
pub const PRESET_KIND_MONOTONE: u32 = 6;
pub const PRESET_KIND_BRIGHTNESS: u32 = 4;
pub const PRESET_KIND_GREYSCALE: u32 = 5;
pub const PRESET_KIND_COLOUR: u32 = 8;
/// BrightnessInvert (draw 0x436f60): no data; the pixel's hue turned by
/// half a turn, then inverted, i.e. its lightness inverted.
pub const PRESET_KIND_BRIGHTNESS_INVERT: u32 = 9;
/// Filter-Replace (draw 0x41d390): [`FilterReplace`].
pub const PRESET_KIND_REPLACE: u32 = 13;
pub const PRESET_KIND_ZIGZAG: u32 = 12;
/// [`PresetModule::blend`] values used by the game (the exe's names are
/// Alpha, Add, Multi, Screen, SignedAdd, Copy, dissolve).
pub const PRESET_BLEND_ALPHA: u16 = 0;
pub const PRESET_BLEND_ADD: u16 = 1;
pub const PRESET_BLEND_MULTI: u16 = 2;
pub const PRESET_BLEND_SCREEN: u16 = 3;
pub const PRESET_BLEND_COPY: u16 = 5;
/// An effect over the whole screen, "environment-<slot>" (handler
/// 0x4153c0, `AdvObjectScript_Env.cpp`). Record: u32 slot (0 clears every
/// slot), u32 layer | depth << 8 as in 0x70 (the game uses layer 0, depth
/// mostly 1000), u32 kind (0 clears the slot), then the kind's data. Each
/// kind is one animation module on an object as large as the screen (setup
/// table at 0x5013bc; module classes from the creator table at 0x501434):
/// 1 Animation-Simple on image 50,000,000 + n from `HD/env.arc` (light
/// rays; unused), 2 Animation-RandomNoise, 3 Filter-LensFlare (unused),
/// 4 Animation-Replace (a ripple through a displacement map), 5
/// Animation-Vibrate (shaking), 6 and 7 unused.
pub const OP_ENVIRONMENT: u16 = 0x8c;
pub const ENV_KIND_NOISE: u32 = 2;
pub const ENV_KIND_RIPPLE: u32 = 4;
pub const ENV_KIND_SHAKE: u32 = 5;
/// A quake of the picture behind the text window (handler 0x442000, object
/// 0x441ee0 at sort key 0x06000000, the bottom of the text window's band).
/// Record: u32 speed, u32 x and y range in pixels, then 12 zero bytes. The
/// picture moves along the 7-point path of 0x8c's shaking, `speed` points
/// a second; after 4 points both ranges shrink by max(speed / 4, 50)
/// pixels a second, and the effect ends when both reach 0. The script
/// waits for it; a click ends it early. Skipped while skipping, or when
/// the effects setting (bit 0 of 0x51ed14) is off.
pub const OP_QUAKE: u16 = 0x6d;
/// A flash (handler 0x441550, object 0x4413d0 at sort key 0xffffffff, over
/// everything): `arg` is a colour (COLORREF, 0x00BBGGRR) filled over the
/// screen with blend 1 (Add) at opacity 128 for 100 ms. The script waits
/// for it; a click ends it early. Skipped while skipping, or with the
/// setting at bit 16 of 0x52056c.
pub const OP_FLASH: u16 = 0x6e;
pub const FLASH_MS: u32 = 100;
/// A "surprise" pulse (handler 0x441a00, Surprise.cpp, object at sort key
/// 0x06000000, behind the text window): a copy of the picture taken when it
/// starts, tinted, drawn over the live picture scaled by 1 + f × zoom
/// around the centre at f × opacity, f = sin(π × remaining / time). Record:
/// u32 time in ms (clamped to 1..32767), u32 preset (0-8, others = 0) into
/// [`SURPRISE_PRESETS`]. The script waits for it; a click ends it early.
/// Skipped while skipping, or when the effects setting is off.
pub const OP_SURPRISE: u16 = 0x6f;
pub const FLASH_OPACITY: u32 = 128;
pub const OP_BGM_PLAY: u16 = 0x57;
pub const OP_BGM_STOP: u16 = 0x58;
/// A BGM crossfade (handler 0x473610): record u32 fade in ms (capped at
/// 30000), u16 track to stop (0 = every track but the new one), u16
/// track to play. The new track starts fading in, then the old one fades
/// out over the same time; both run at once.
pub const OP_BGM_CROSSFADE: u16 = 0x59;
pub const OP_SE_PLAY: u16 = 0x5a;
pub const OP_SE_STOP: u16 = 0x5b;

/// A sound command. Numbers are 1-based as in the script; 0 in a stop
/// command means "everything on that channel".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    BgmPlay { track: u32, fade_in_ms: u16 },
    BgmStop { track: u32, fade_out_ms: u16 },
    BgmCrossfade { from: u32, to: u32, fade_ms: u32 },
    SePlay { sound: u32, looped: bool, fade_in_ms: u32 },
    SeStop { sound: u32, fade_out_ms: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteEntry {
    pub mode: u16,
    pub character: u16,
    /// 0-based image number across the `bu*.004` archives.
    pub sprite: u16,
    /// Drawing order among the bust-ups (higher on top; 10-150 in this
    /// game, 1100-5000 over the frame overlays at 1000). Handler 0x434130
    /// passes it to the object it creates (0x4109a0) as the overlay's depth
    /// is (0x412fe0); it is no fade time: a bust-up fades over a fixed
    /// 500 ms whatever its value (filmed: 50 and 1500 both take 0.5 s).
    pub depth: u16,
    pub x: u16,
    pub y: u16,
}

/// One module of an effect preset (opcode 0x88).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetModule {
    /// 1-based preset number.
    pub preset: u32,
    /// 1-based slot in the preset; 0 clears the whole preset.
    pub slot: u32,
    pub kind: u32,
    /// How the module's colour is combined with the pixel's (see
    /// `PRESET_BLEND_*`).
    pub blend: u16,
    /// 0-256: how much of the result replaces the pixel (the exe's debug
    /// output calls it "Opacity").
    pub opacity: u16,
    pub data: ModuleData,
}

/// The kind data of a 0x88/0x70 module, for the kinds the converter
/// reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModuleData {
    #[default]
    None,
    /// A colour filter (kind 8), as R, G, B.
    Colour([u8; 3]),
    /// The amount of a brightness filter (kind 4), printed by the exe as
    /// "Brightness=%d".
    Brightness(i16),
    /// A monotone's palette (kind 6).
    Palette(u32),
    /// A blur's x and y range (kind 1, "XRange", "YRange").
    Smooth(u8, u8),
    /// Hue and saturation (kind 3).
    HueSaturation(i16, i16),
    Replace(FilterReplace),
    Zigzag(FilterZigzag),
}

/// Filter-Zigzag (kernel 0x43f0b0, set up by 0x41dab0): each pixel taken
/// from `distance` × sin(2π·i/1024) pixels away along `direction` (0.1°),
/// i = d × `frequency`/100 (+ a phase), d being the pixel's distance from
/// (`centre_x`, `centre_y`): rings around the centre. Record: i32 centre
/// x, y, direction, distance, frequency, begin angle, reduction (the
/// exe's debug output names them CenterX … Reduc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterZigzag {
    pub centre_x: i32,
    pub centre_y: i32,
    pub direction: i32,
    pub distance: i32,
    pub frequency: i32,
}

/// Filter-Replace: the picture displaced through a map as by
/// Animation-Replace ([`Ripple`]), but still: each pixel taken from
/// `distance` × sin(2π(m/256 + begin_angle/100)) pixels away along
/// `direction` (0.1°), m being the pixel's grey level in `mask%04u.png`
/// number `map`. Record: u32 map, i32 direction, distance, begin angle
/// (the exe's debug output: MapID, Direction, Distance, BeginAngle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterReplace {
    pub map: u32,
    pub direction: i32,
    pub distance: i32,
}

/// Opcode 0x70: one slot of the filter on the whole screen, made of the
/// same modules as 0x88 (the object is named `filter-<slot>`). Record:
/// u32 slot (0 clears every slot), u32 layer | depth << 8, u32 kind
/// (0 clears the slot), u16 blend, u16 opacity, then the kind's data as
/// in 0x88 (monotone: u32 palette). The layer byte picks where the filter
/// sits among the screen's layers (table at 0x503e68: 0 and 2 → 0x10,
/// 1 → 0, 3 → 0x20, 4 → 0x30); the game uses 0 and, for colour overlays
/// and blurs set inside a transition, 2 and 3. The depth is the object's
/// sort depth within that band (0x4156d0 passes it to the object
/// creator 0x4109a0; mostly 1000, above the bust-ups' 10-100). The filter
/// has no fade of its own: it shows at the next frame, or with the
/// transition around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenModule {
    pub slot: u32,
    pub layer: u8,
    pub kind: u32,
    pub blend: u16,
    pub opacity: u16,
    pub data: ModuleData,
}

/// Reads a module's kind data at `o` (after its blend and opacity).
fn module_data(r: &PoolReader, kind: u32, o: usize) -> Result<ModuleData, Truncated> {
    let i = |k: usize| r.u32(o + 4 * k).map(|n| n as i32);
    Ok(match kind {
        PRESET_KIND_COLOUR => ModuleData::Colour(r.slice(o, 3)?.try_into().unwrap()),
        PRESET_KIND_BRIGHTNESS => ModuleData::Brightness(r.u16(o)? as i16),
        PRESET_KIND_MONOTONE => ModuleData::Palette(r.u32(o)?),
        PRESET_KIND_SMOOTH => {
            let d = r.slice(o, 2)?;
            ModuleData::Smooth(d[0], d[1])
        }
        PRESET_KIND_HUE_SATURATION => ModuleData::HueSaturation(r.u16(o)? as i16, r.u16(o + 2)? as i16),
        PRESET_KIND_REPLACE => ModuleData::Replace(FilterReplace {
            map: r.u32(o)?,
            direction: i(1)?,
            distance: i(2)?,
        }),
        PRESET_KIND_ZIGZAG => ModuleData::Zigzag(FilterZigzag {
            centre_x: i(0)?,
            centre_y: i(1)?,
            direction: i(2)?,
            distance: i(3)?,
            frequency: i(4)?,
        }),
        _ => ModuleData::None,
    })
}

/// Opcode 0x8c: one slot of the effects over the whole screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvModule {
    pub slot: u32,
    pub kind: u32,
    /// Static (kind 2), else `None`.
    pub noise: Option<Noise>,
    /// A ripple (kind 4), else `None`.
    pub ripple: Option<Ripple>,
    /// Shaking (kind 5), else `None`.
    pub shake: Option<Shake>,
}

/// Animation-Replace (module 0x73, draw 0x41a2d0, update 0x41a4a0): the
/// picture under the object, each pixel taken from amp × sin(2π(m/256 +
/// phase/1000)) pixels away along `angle` (0.1°), m being the pixel's grey
/// level in `mask%04u.png` (`HD/mask.arc`) number `map` (sine table
/// 0x501938, per-pixel loop 0x43d9d0), then blended over the picture with
/// `blend` at an opacity going from `begin_opacity` to `end_opacity`
/// (0-256) over `time_ms`, as the amplitude goes from `begin_amplitude` to
/// `end_amplitude` pixels. Every `interval_ms` the angle turns by
/// `angle_step` and the phase moves by `phase_step`. Record: u32 option,
/// time, interval, u8 blend, u16 begin and end opacity, u32 map, i32
/// amplitudes, angle, angle step, phase step (the exe's debug output:
/// AmLenBegin, AmLenEnd, AmAngle, AmAngleMove, FreqMove). Option 0x10
/// switches to a mode around a centre point; the game never uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ripple {
    pub option: u32,
    pub time_ms: u32,
    pub interval_ms: u32,
    pub blend: u8,
    pub begin_opacity: u16,
    pub end_opacity: u16,
    pub map: u32,
    pub begin_amplitude: i32,
    pub end_amplitude: i32,
    pub angle: i32,
    pub angle_step: i32,
    pub phase_step: i32,
}

/// Animation-Vibrate (module 0x74, draw 0x41ac70, update 0x41ad60): moves
/// the picture under the object (0x441b80 blits it onto itself) by an
/// offset up to a range that grows from `begin_range` to `end_range`
/// pixels over `time_ms`, updated every `interval_ms`. Option bit 0 clear:
/// each axis on a 256-step sine wave (table 0x501538) whose phase advances
/// by speed × ms (16.16 turns); set: along a 7-point path (table
/// 0x501300), the next point when speed x × ms passes 65536. Record: nine
/// u32: option, time, interval, speed x, y, begin range x, y, end range
/// x, y.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shake {
    pub option: u32,
    pub time_ms: u32,
    pub interval_ms: u32,
    pub speed: (i32, i32),
    pub begin_range: (i32, i32),
    pub end_range: (i32, i32),
}

/// Opcode 0x6d: a quake ([`OP_QUAKE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quake {
    pub speed: u32,
    pub range: (u32, u32),
}

/// Opcode 0x6f: a pulse ([`OP_SURPRISE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surprise {
    pub time_ms: u32,
    pub preset: usize,
}

/// One of 0x6f's presets (table 0x500b68, seven i32 each).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurprisePreset {
    /// 16.16 fractions of the picture's size.
    pub zoom: (i32, i32),
    /// 0-256.
    pub opacity: i32,
    /// COLORREF (0x00BBGGRR).
    pub tint: u32,
    /// 0-256.
    pub tint_opacity: i32,
    /// The blend of the tint: 2 (Multi) or 6 (in 0x88's list "dissolve").
    pub tint_blend: i32,
}

pub const SURPRISE_PRESETS: [SurprisePreset; 9] = {
    const fn p(zoom: i32, opacity: i32, tint: u32, tint_opacity: i32, tint_blend: i32) -> SurprisePreset {
        SurprisePreset { zoom: (zoom, zoom), opacity, tint, tint_opacity, tint_blend }
    }
    [
        p(0x1999, 100, 0xffffff, 0, 0),
        p(0x2666, 150, 0xffffff, 0, 0),
        p(0x3333, 200, 0xffffff, 0, 0),
        p(0x1999, 100, 0x8080ff, 256, 2),
        p(0x2666, 150, 0x8080ff, 256, 2),
        p(0x3333, 200, 0x8080ff, 256, 2),
        p(0x1999, 100, 0xffffff, 256, 6),
        p(0x2666, 150, 0xffffff, 256, 6),
        p(0x3333, 200, 0xffffff, 256, 6),
    ]
};

/// Animation-RandomNoise (module 0x72): greyscale static, one random byte
/// per pixel from a 256 KiB table (0x43bab0's sibling 0x4398d0; mode bit 0
/// clear picks a colour variant), drawn with `blend` at an opacity that
/// goes from `begin_opacity` to `end_opacity` (0-256) over `time_ms`,
/// redrawn from a new random offset every `interval_ms` (update 0x419570).
/// Record: u8 mode, u8 blend, u16 begin, u16 end, u32 time, u32 interval
/// (the field names are the exe's debug output). The game always uses
/// mode 1, blend 0 (alpha), opacity from 0 and an interval of 100 ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Noise {
    pub mode: u8,
    pub blend: u8,
    pub begin_opacity: u16,
    pub end_opacity: u16,
    pub time_ms: u32,
    pub interval_ms: u32,
}

/// Opcode 0x90: attach an effect preset to a character's bust-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyPreset {
    pub character: u16,
    /// 1-based preset number; 0 removes the preset module.
    pub preset: u32,
}

/// Opcode 0x44: a reading shown above characters of the next message.
/// Record: u16 line, u16 column, u16 length, u16 flags (1 or 3; no visible
/// difference found), u32 0, then the reading as a pool string. Lines
/// count the message's text lines (not the speaker's name), 1-based, as
/// do columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ruby {
    pub line: u16,
    pub column: u16,
    pub length: u16,
    pub text: String,
}

/// Opcode 0x49: size and colour for characters of the next message.
/// Record: u16 line, u16 column, u16 length (30000 = to the end of the
/// message), u16 size, u32 colour, u32 edge colour. Lines and columns count as
/// for ruby. Size: a percentage below 0x1000, 0x1000 + n a preset of
/// n × 25 percent (0x1003 for mumbling, 0x1005-0x1007 for shouting),
/// 0xf000 to keep the size. Colours are Windows COLORREFs (0x00BBGGRR);
/// 0xff000000 keeps the colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    pub line: u16,
    pub column: u16,
    pub length: u16,
    pub size: u16,
    pub colour: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub row: u16,
    pub column: u16,
    pub length: u16,
    pub style: u32,
    pub target: u32,
}

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

/// An operand given and never read ([`CdScript::ignored`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Unread {
    /// The instruction's index in `code`.
    pub index: usize,
    pub op: u16,
    /// `sub`, `arg`, `+n` or `-n` (pool record bytes), or "record out of place".
    pub which: String,
    /// `sub` and `arg` in decimal; record bytes in hex.
    pub value: String,
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
    /// What the converter has read, for [`CdScript::ignored`]: each pool
    /// byte, and each instruction's `sub` (bit 0) and `arg` (bit 1).
    read_pool: RefCell<Vec<bool>>,
    read_fields: RefCell<Vec<u8>>,
}

/// [`Reader`] over a script's pool that notes each byte it reads.
struct PoolReader<'s, 'a>(&'s CdScript<'a>);

impl<'a> PoolReader<'_, 'a> {
    fn slice(&self, offset: usize, len: usize) -> Result<&'a [u8], Truncated> {
        let bytes = Reader(self.0.pool).slice(offset, len)?;
        self.0.read_pool.borrow_mut()[offset..offset + len].fill(true);
        Ok(bytes)
    }

    fn u16(&self, offset: usize) -> Result<u16, Truncated> {
        Ok(u16::from_le_bytes(self.slice(offset, 2)?.try_into().unwrap()))
    }

    fn u32(&self, offset: usize) -> Result<u32, Truncated> {
        Ok(u32::from_le_bytes(self.slice(offset, 4)?.try_into().unwrap()))
    }
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
        let code: Vec<Instr> = (0..code_size / ENTRY_LEN)
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

        let pool = r.slice(pool_start, pool_size)?;
        Ok(Self {
            read_pool: RefCell::new(vec![false; pool.len()]),
            read_fields: RefCell::new(vec![0; code.len()]),
            head,
            name: name.into_owned(),
            script_id,
            reserved,
            labels,
            code,
            pool,
            stored_md5: r.slice(footer_start, FOOTER_LEN)?.try_into().unwrap(),
            hashed: &bytes[..footer_start],
        })
    }

    /// `ins.sub`, noted as read.
    pub fn sub(&self, ins: &Instr) -> u16 {
        self.mark_field(ins, 1);
        ins.sub
    }

    /// `ins.arg`, noted as read.
    pub fn arg(&self, ins: &Instr) -> i32 {
        self.mark_field(ins, 2);
        ins.arg
    }

    fn mark_field(&self, ins: &Instr, bit: u8) {
        // `ins` is one of `self.code`; ones built elsewhere (tests) aren't tracked.
        let i = (ins as *const Instr as usize).wrapping_sub(self.code.as_ptr() as usize) / size_of::<Instr>();
        if self.code.get(i).is_some_and(|c| std::ptr::eq(c, ins)) {
            self.read_fields.borrow_mut()[i] |= bit;
        }
    }

    /// Pool bytes `offset..offset + len`, noted as read.
    pub fn bytes(&self, offset: usize, len: usize) -> Option<&'a [u8]> {
        self.reader().slice(offset, len).ok()
    }

    fn reader(&self) -> PoolReader<'_, 'a> {
        PoolReader(self)
    }

    /// The operands given but never read through [`Self::sub`],
    /// [`Self::arg`] or the pool readers, whatever their value: one
    /// [`Unread`] per instruction and operand. `which` is `sub`, `arg`, or
    /// `+n` for a run of unread pool record bytes from byte `n` (`-n`: a
    /// run that ends the record, from `n` bytes before its end).
    /// `records` are the opcodes whose `arg` is a pool record; the records
    /// lie end to end in instruction order (a gap or overlap is returned as
    /// `which` = "record out of place", and the pool isn't checked further).
    pub fn ignored(&self, records: &[u16]) -> Vec<Unread> {
        let fields = self.read_fields.borrow();
        let read = self.read_pool.borrow();
        let mut out = Vec::new();
        let starts: Vec<usize> = (0..self.code.len()).filter(|&i| records.contains(&self.code[i].op)).collect();
        let mut tiled = true;
        let mut pos = 0;
        for (i, ins) in self.code.iter().enumerate() {
            let mut push = |which: String, value: String| out.push(Unread { index: i, op: ins.op, which, value });
            if fields[i] & 1 == 0 {
                push("sub".to_string(), ins.sub.to_string());
            }
            if !records.contains(&ins.op) {
                if fields[i] & 2 == 0 {
                    push("arg".to_string(), ins.arg.to_string());
                }
                continue;
            }
            let start = ins.arg as u32 as usize;
            let next = starts.get(starts.partition_point(|&s| s <= i)).map_or(self.pool.len(), |&s| self.code[s].arg as u32 as usize);
            if !tiled || start != pos || next < start || next > self.pool.len() {
                if tiled {
                    push("record out of place".to_string(), String::new());
                }
                tiled = false;
                continue;
            }
            pos = next;
            let mut n = start;
            while n < next {
                let run = read[n..next].iter().position(|&r| r != read[n]).map_or(next, |k| n + k);
                if !read[n] {
                    // A run at the end of a record of varying length is named from its end.
                    let which = if run == next && n > start { format!("-{}", next - n) } else { format!("+{}", n - start) };
                    push(which, self.pool[n..run].iter().map(|b| format!("{b:02x}")).collect());
                }
                n = run;
            }
        }
        if tiled && pos != self.pool.len() {
            out.push(Unread { index: self.code.len(), op: 0, which: "record out of place".to_string(), value: String::new() });
        }
        out
    }

    pub fn md5_ok(&self) -> bool {
        Md5::digest(self.hashed)[..] == self.stored_md5
    }

    /// Reads a length-prefixed pool string at `offset`, or `None` if the
    /// bytes there don't form a valid Shift-JIS string.
    pub fn pool_string(&self, offset: usize) -> Option<String> {
        let r = self.reader();
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
        let r = self.reader();
        let id = r.u32(offset)?;
        let size = r.u32(offset + 4)? as usize;
        // For `cd messages`: the converter doesn't use it.
        let line = Reader(self.pool).u32(offset + 8)?;
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

    /// Decodes the record of a hyperlink instruction, or returns `None` for
    /// other opcodes.
    pub fn link(&self, ins: &Instr) -> Option<Result<Link, CdError>> {
        (ins.op == OP_LINK).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            Ok(Link {
                row: r.u16(o)?,
                column: r.u16(o + 2)?,
                length: r.u16(o + 4)?,
                // For `cd messages`: the converter doesn't use it.
                style: Reader(self.pool).u32(o + 6)?,
                target: r.u32(o + 10)?,
            })
        })
    }

    /// Decodes the record of a ruby instruction, or returns `None` for
    /// other opcodes.
    pub fn ruby(&self, ins: &Instr) -> Option<Result<Ruby, CdError>> {
        (ins.op == OP_RUBY).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            Ok(Ruby {
                line: r.u16(o)?,
                column: r.u16(o + 2)?,
                length: r.u16(o + 4)?,
                text: self
                    .pool_string(o + 12)
                    .ok_or(CdError::BadOperand { offset: o, reason: "ruby is not valid Shift-JIS" })?,
            })
        })
    }

    /// Decodes the record of a text style instruction, or returns `None`
    /// for other opcodes.
    pub fn text_style(&self, ins: &Instr) -> Option<Result<TextStyle, CdError>> {
        (ins.op == OP_TEXT_STYLE).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            Ok(TextStyle {
                line: r.u16(o)?,
                column: r.u16(o + 2)?,
                length: r.u16(o + 4)?,
                size: r.u16(o + 6)?,
                colour: r.u32(o + 8)?,
            })
        })
    }

    /// Decodes a sound instruction, or returns `None` for other opcodes.
    pub fn sound(&self, ins: &Instr) -> Option<Result<Sound, CdError>> {
        let number = || self.arg(ins).max(0) as u32;
        Some(Ok(match ins.op {
            OP_BGM_PLAY => Sound::BgmPlay { track: number(), fade_in_ms: self.sub(ins).min(30000) },
            OP_BGM_STOP => Sound::BgmStop { track: number(), fade_out_ms: self.sub(ins).min(30000) },
            OP_SE_STOP => Sound::SeStop { sound: number(), fade_out_ms: self.sub(ins) },
            OP_BGM_CROSSFADE => {
                let r = self.reader();
                let o = ins.arg as u32 as usize;
                let read = || -> Result<Sound, CdError> {
                    Ok(Sound::BgmCrossfade {
                        fade_ms: r.u32(o)?.min(30000),
                        from: r.u16(o + 4)?.into(),
                        to: r.u16(o + 6)?.into(),
                    })
                };
                return Some(read());
            }
            OP_SE_PLAY => {
                let r = self.reader();
                let o = ins.arg as u32 as usize;
                let read = || -> Result<Sound, CdError> {
                    Ok(Sound::SePlay {
                        fade_in_ms: r.u32(o)?,
                        looped: r.slice(o + 4, 1)?[0] != 0,
                        sound: r.u32(o + 5)?,
                    })
                };
                return Some(read());
            }
            _ => return None,
        }))
    }

    /// Decodes the entries of a show-sprites instruction, or returns `None`
    /// for other opcodes.
    pub fn sprites(&self, ins: &Instr) -> Option<Result<Vec<SpriteEntry>, CdError>> {
        (ins.op == OP_SPRITES).then(|| {
            let r = self.reader();
            let base = ins.arg as u32 as usize;
            (0..self.sub(ins) as usize)
                .map(|i| {
                    let o = base + i * SPRITE_ENTRY_LEN;
                    Reader(self.pool).slice(o, SPRITE_ENTRY_LEN)?;
                    Ok(SpriteEntry {
                        mode: r.u16(o)?,
                        character: r.u16(o + 2)?,
                        sprite: r.u16(o + 4)?,
                        depth: r.u16(o + 6)?,
                        x: r.u16(o + 8)?,
                        y: r.u16(o + 10)?,
                    })
                })
                .collect()
        })
    }

    /// Decodes an effect preset module (opcode 0x88), or returns `None` for
    /// other opcodes.
    pub fn preset_module(&self, ins: &Instr) -> Option<Result<PresetModule, CdError>> {
        (ins.op == OP_PRESET_MODULE).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            // preset, slot, u32 ?, kind, u16 blend, u16 opacity, kind data.
            let (preset, slot, kind) = (r.u32(o)?, r.u32(o + 4)?, r.u32(o + 12)?);
            let (blend, opacity) = if slot == 0 { (0, 0) } else { (r.u16(o + 16)?, r.u16(o + 18)?) };
            let data = if slot == 0 { ModuleData::None } else { module_data(&r, kind, o + 20)? };
            Ok(PresetModule { preset, slot, kind, blend, opacity, data })
        })
    }

    /// Decodes opcode 0x70, or returns `None` for other opcodes.
    pub fn screen_module(&self, ins: &Instr) -> Option<Result<ScreenModule, CdError>> {
        (ins.op == OP_SCREEN_MODULE).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            let (slot, timing, kind) = (r.u32(o)?, r.slice(o + 4, 1)?[0], r.u32(o + 8)?);
            let set = slot != 0 && kind != 0;
            let (blend, opacity) = if set { (r.u16(o + 12)?, r.u16(o + 14)?) } else { (0, 0) };
            let data = if set { module_data(&r, kind, o + 16)? } else { ModuleData::None };
            Ok(ScreenModule { slot, layer: timing, kind, blend, opacity, data })
        })
    }

    /// Decodes opcode 0x8c, or returns `None` for other opcodes.
    pub fn environment(&self, ins: &Instr) -> Option<Result<EnvModule, CdError>> {
        (ins.op == OP_ENVIRONMENT).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            let (slot, kind) = (r.u32(o)?, r.u32(o + 8)?);
            let noise = if slot != 0 && kind == ENV_KIND_NOISE {
                let d = r.slice(o + 12, 2)?;
                Some(Noise {
                    mode: d[0],
                    blend: d[1],
                    begin_opacity: r.u16(o + 14)?,
                    end_opacity: r.u16(o + 16)?,
                    time_ms: r.u32(o + 18)?,
                    interval_ms: r.u32(o + 22)?,
                })
            } else {
                None
            };
            let (mut ripple, mut shake) = (None, None);
            let d = o + 12;
            if slot != 0 && kind == ENV_KIND_RIPPLE {
                let i = |k: usize| r.u32(d + 17 + 4 * k).map(|n| n as i32);
                ripple = Some(Ripple {
                    option: r.u32(d)?,
                    time_ms: r.u32(d + 4)?,
                    interval_ms: r.u32(d + 8)?,
                    blend: r.slice(d + 12, 1)?[0],
                    begin_opacity: r.u16(d + 13)?,
                    end_opacity: r.u16(d + 15)?,
                    map: r.u32(d + 17)?,
                    begin_amplitude: i(1)?,
                    end_amplitude: i(2)?,
                    angle: i(3)?,
                    angle_step: i(4)?,
                    phase_step: i(5)?,
                });
            }
            if slot != 0 && kind == ENV_KIND_SHAKE {
                let i = |k: usize| r.u32(d + 4 * k).map(|n| n as i32);
                shake = Some(Shake {
                    option: r.u32(d)?,
                    time_ms: r.u32(d + 4)?,
                    interval_ms: r.u32(d + 8)?,
                    speed: (i(3)?, i(4)?),
                    begin_range: (i(5)?, i(6)?),
                    end_range: (i(7)?, i(8)?),
                });
            }
            Ok(EnvModule { slot, kind, noise, ripple, shake })
        })
    }

    /// Decodes opcode 0x6d, or returns `None` for other opcodes.
    pub fn quake(&self, ins: &Instr) -> Option<Result<Quake, CdError>> {
        (ins.op == OP_QUAKE).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            Ok(Quake { speed: r.u32(o)?, range: (r.u32(o + 4)?, r.u32(o + 8)?) })
        })
    }

    /// Decodes opcode 0x6f, or returns `None` for other opcodes.
    pub fn surprise(&self, ins: &Instr) -> Option<Result<Surprise, CdError>> {
        (ins.op == OP_SURPRISE).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            let preset = r.u32(o + 4)? as usize;
            Ok(Surprise {
                time_ms: r.u32(o)?.clamp(1, 32767),
                preset: if preset < SURPRISE_PRESETS.len() { preset } else { 0 },
            })
        })
    }

    /// Decodes opcode 0x90, or returns `None` for other opcodes.
    pub fn apply_preset(&self, ins: &Instr) -> Option<Result<ApplyPreset, CdError>> {
        (ins.op == OP_APPLY_PRESET).then(|| {
            let r = self.reader();
            let o = ins.arg as u32 as usize;
            Ok(ApplyPreset { character: r.u16(o)?, preset: r.u32(o + 8)? })
        })
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
pub(crate) mod tests {
    use super::*;

    fn sjis(s: &str) -> Vec<u8> {
        encoding_rs::SHIFT_JIS.encode(s).0.into_owned()
    }

    /// A script file with these sections (for tests here and in `convert`).
    pub(crate) fn build(name: &str, labels: &[Label], code: &[Instr], pool: &[u8]) -> Vec<u8> {
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
    fn decodes_links() {
        let mut pool = Vec::new();
        for v in [2u16, 8, 4] {
            pool.extend_from_slice(&v.to_le_bytes());
        }
        for v in [0x8007u32, 6, 0] {
            pool.extend_from_slice(&v.to_le_bytes());
        }
        let code = [Instr { op: OP_LINK, sub: 0xffff, arg: 0, source_pos: 0 }];
        let bytes = build("l.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        let link = cd.link(&cd.code[0]).unwrap().unwrap();
        assert_eq!(link, Link { row: 2, column: 8, length: 4, style: 0x8007, target: 6 });
        assert!(cd.message(&cd.code[0]).is_none());

        let code = [Instr { op: OP_LINK, sub: 0xffff, arg: 10, source_pos: 0 }];
        let bytes = build("l.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        assert!(cd.link(&cd.code[0]).unwrap().is_err());
    }

    #[test]
    fn decodes_sound() {
        let mut pool = vec![0u8; 3];
        pool.extend_from_slice(&7u32.to_le_bytes());
        pool.push(1);
        pool.extend_from_slice(&125u32.to_le_bytes());
        // At 12: a crossfade from track 16 to 1 over 3 s.
        pool.extend_from_slice(&[0xb8, 0x0b, 0, 0, 16, 0, 1, 0]);
        let code = [
            Instr { op: OP_BGM_PLAY, sub: 2000, arg: 5, source_pos: 0 },
            Instr { op: OP_BGM_STOP, sub: 40000, arg: 0, source_pos: 1 },
            Instr { op: OP_SE_PLAY, sub: 0xffff, arg: 3, source_pos: 2 },
            Instr { op: OP_SE_STOP, sub: 300, arg: 125, source_pos: 3 },
            Instr { op: OP_BGM_CROSSFADE, sub: 0xffff, arg: 12, source_pos: 3 },
            Instr { op: 0x10, sub: 0xffff, arg: 0, source_pos: 4 },
        ];
        let bytes = build("snd.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        let got: Vec<_> = cd.code.iter().map(|i| cd.sound(i).map(|r| r.unwrap())).collect();
        assert_eq!(
            got,
            [
                Some(Sound::BgmPlay { track: 5, fade_in_ms: 2000 }),
                Some(Sound::BgmStop { track: 0, fade_out_ms: 30000 }),
                Some(Sound::SePlay { sound: 125, looped: true, fade_in_ms: 7 }),
                Some(Sound::SeStop { sound: 125, fade_out_ms: 300 }),
                Some(Sound::BgmCrossfade { from: 16, to: 1, fade_ms: 3000 }),
                None,
            ]
        );
    }

    #[test]
    fn notes_ignored_operands() {
        // An 0x90 record (u16 character, u16 0, u32 module field, u32
        // preset), then an SE record.
        let mut pool = vec![3, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0];
        pool.extend_from_slice(&[0, 0, 0, 0, 1, 7, 0, 0, 0]);
        let code = [
            Instr { op: OP_APPLY_PRESET, sub: 0xffff, arg: 0, source_pos: 0 },
            Instr { op: OP_SE_PLAY, sub: 0xffff, arg: 12, source_pos: 1 },
            Instr { op: OP_BGM_PLAY, sub: 500, arg: 2, source_pos: 2 },
            Instr { op: 0x52, sub: 0xffff, arg: 9, source_pos: 3 },
        ];
        let bytes = build("ops.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        let records = [OP_APPLY_PRESET, OP_SE_PLAY];
        assert_eq!(cd.ignored(&records).len(), 5, "nothing read yet: a run per record, BGM sub and arg, 0x52 arg");
        cd.apply_preset(&cd.code[0]).unwrap().unwrap();
        cd.sound(&cd.code[1]).unwrap().unwrap();
        cd.sound(&cd.code[2]).unwrap().unwrap();
        // The module field (bytes 2-7) and 0x52's arg; sub 0xffff isn't given.
        assert_eq!(cd.ignored(&records), [(OP_APPLY_PRESET, "+2".to_string()), (0x52, "arg".to_string())]);
        // A record list that leaves the 0x90 record out finds the SE record out of place.
        assert!(cd.ignored(&[OP_SE_PLAY]).contains(&(OP_SE_PLAY, "record out of place".to_string())));
    }

    #[test]
    fn decodes_sprites() {
        let mut pool = Vec::new();
        for entry in [[1u16, 2, 62, 50, 400, 300], [3, 7, 683, 20, 150, 320]] {
            let mut rec = entry.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
            rec.resize(SPRITE_ENTRY_LEN, 0xcd);
            pool.extend(rec);
        }
        let code = [Instr { op: OP_SPRITES, sub: 2, arg: 0, source_pos: 0 }];
        let bytes = build("s.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        let sprites = cd.sprites(&cd.code[0]).unwrap().unwrap();
        assert_eq!(sprites[1], SpriteEntry { mode: 3, character: 7, sprite: 683, depth: 20, x: 150, y: 320 });

        let code = [Instr { op: OP_SPRITES, sub: 3, arg: 0, source_pos: 0 }];
        let bytes = build("s.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        assert!(cd.sprites(&cd.code[0]).unwrap().is_err());
    }

    #[test]
    fn decodes_presets() {
        // Records from script 2 (clear preset 1, colour ffe0b9 in slot 1)
        // and 0x90 in script 7.
        let pool: Vec<u8> = [
            "0100000000000000000000000000000000000000",
            "0100000001000000000000000800000002000001ffe0b9",
            "0e0000000100000001000000",
        ]
        .iter()
        .flat_map(|h| (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()))
        .collect();
        let code = [
            Instr { op: OP_PRESET_MODULE, sub: 0xffff, arg: 0, source_pos: 0 },
            Instr { op: OP_PRESET_MODULE, sub: 0xffff, arg: 20, source_pos: 0 },
            Instr { op: OP_APPLY_PRESET, sub: 0xffff, arg: 43, source_pos: 0 },
        ];
        let bytes = build("p.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        let clear = cd.preset_module(&cd.code[0]).unwrap().unwrap();
        assert_eq!(
            clear,
            PresetModule { preset: 1, slot: 0, kind: 0, blend: 0, opacity: 0, data: ModuleData::None }
        );
        let colour = cd.preset_module(&cd.code[1]).unwrap().unwrap();
        assert_eq!(colour.data, ModuleData::Colour([0xff, 0xe0, 0xb9]));
        assert_eq!((colour.blend, colour.opacity), (PRESET_BLEND_MULTI, 256));
        assert_eq!(cd.apply_preset(&cd.code[2]).unwrap().unwrap(), ApplyPreset { character: 14, preset: 1 });
        assert!(cd.apply_preset(&cd.code[0]).is_none());
    }

    #[test]
    fn decodes_environment() {
        // Static at 120/256 (script 5) and the clear after it.
        let pool: Vec<u8> = [concat!("01000000", "00e80300", "02000000", "0100", "0000", "7800", "64000000", "64000000"), "0100000000e8030000000000"]
            .iter()
            .flat_map(|h| (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()))
            .collect();
        let code = [
            Instr { op: OP_ENVIRONMENT, sub: 0xffff, arg: 0, source_pos: 0 },
            Instr { op: OP_ENVIRONMENT, sub: 0xffff, arg: 26, source_pos: 0 },
        ];
        let bytes = build("e.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        let noise = Noise { mode: 1, blend: 0, begin_opacity: 0, end_opacity: 120, time_ms: 100, interval_ms: 100 };
        assert_eq!(cd.environment(&cd.code[0]).unwrap().unwrap(), EnvModule { slot: 1, kind: 2, noise: Some(noise), ripple: None, shake: None });
        assert_eq!(cd.environment(&cd.code[1]).unwrap().unwrap(), EnvModule { slot: 1, kind: 0, noise: None, ripple: None, shake: None });

        // The ripple in script 3 and the shaking in script 3.
        let pool: Vec<u8> = [
            concat!("01000000", "00e80300", "04000000", "00000000", "b80b0000", "64000000", "00", "6400", "9600"),
            concat!("e9030000", "05000000", "0a000000", "4e0c0000", "14000000", "1e000000"),
            concat!("01000000", "00e80300", "05000000", "00000000", "20030000", "0a000000", "20030000", "00000000"),
            concat!("00000000", "00000000", "1e000000", "05000000"),
        ]
        .concat()
        .as_bytes()
        .chunks(2)
        .map(|h| u8::from_str_radix(std::str::from_utf8(h).unwrap(), 16).unwrap())
        .collect();
        let code = [
            Instr { op: OP_ENVIRONMENT, sub: 0xffff, arg: 0, source_pos: 0 },
            Instr { op: OP_ENVIRONMENT, sub: 0xffff, arg: 53, source_pos: 0 },
        ];
        let bytes = build("e.rio", &[], &code, &pool);
        let cd = CdScript::parse(&bytes).unwrap();
        let ripple = Ripple {
            option: 0,
            time_ms: 3000,
            interval_ms: 100,
            blend: 0,
            begin_opacity: 100,
            end_opacity: 150,
            map: 1001,
            begin_amplitude: 5,
            end_amplitude: 10,
            angle: 3150,
            angle_step: 20,
            phase_step: 30,
        };
        assert_eq!(cd.environment(&cd.code[0]).unwrap().unwrap().ripple, Some(ripple));
        let shake = Shake { option: 0, time_ms: 800, interval_ms: 10, speed: (800, 0), begin_range: (0, 0), end_range: (30, 5) };
        assert_eq!(cd.environment(&cd.code[1]).unwrap().unwrap().shake, Some(shake));
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
