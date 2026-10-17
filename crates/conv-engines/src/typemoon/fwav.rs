//! FWAV, one wave of the sound archive: DSP ADPCM in this game.
//!
//! The NW4F frame (`bfsar.rs`); INFO (0x7000): u8 encoding (2: DSP
//! ADPCM), u8 loops, u16, u32 sample rate, u32 loop start, u32 loop end
//! (the sample count), u32, then at 0x14 a table of channels: a
//! reference to the samples (offset into DATA's body), a reference to
//! the ADPCM state (16 i16 coefficients, u16 predictor/scale, i16 two
//! history samples, then the same three at the loop start: the decode
//! is checked against them). DSP ADPCM: 8-byte frames of 14 samples, a header
//! byte (coefficient pair in the high nibble, log2 scale in the low)
//! and 14 signed nibbles.

use super::FormatError;
use crate::read::Reader;

pub struct Wave {
    pub rate: u32,
    pub channels: u16,
    /// Interleaved.
    pub samples: Vec<i16>,
    /// Loop start and end, in sample frames.
    pub looped: Option<(u32, u32)>,
}

fn i16_at(r: &Reader, at: usize) -> Result<i16, FormatError> {
    Ok(r.u16(at)? as i16)
}

pub fn decode(data: &[u8]) -> Result<Wave, FormatError> {
    if !data.starts_with(b"FWAV") {
        return Err(FormatError::BadMagic("FWAV"));
    }
    let r = Reader(data);
    let (mut info, mut samples_at) = (None, None);
    for i in 0..usize::from(r.u16(0x10)?) {
        let at = r.u32(0x18 + 12 * i)? as usize + 8;
        match r.u16(0x14 + 12 * i)? {
            0x7000 => info = Some(at),
            0x7001 => samples_at = Some(at),
            _ => {}
        }
    }
    let (info, samples_at) = info.zip(samples_at).ok_or_else(|| FormatError::Bad("FWAV without INFO or DATA".into()))?;
    let encoding = r.slice(info, 1)?[0];
    if encoding != 2 {
        return Err(FormatError::Bad(format!("FWAV encoding {encoding} (only DSP ADPCM is read)")));
    }
    let rate = r.u32(info + 4)?;
    let (loop_start, frames) = (r.u32(info + 8)?, r.u32(info + 12)? as usize);
    let channels: Vec<usize> = (0..r.u32(info + 0x14)? as usize)
        .map(|c| Ok(info + 0x14 + r.u32(info + 0x14 + 8 + 8 * c)? as usize))
        .collect::<Result<_, FormatError>>()?;
    let mut out = vec![0i16; frames * channels.len()];
    for (c, &ch) in channels.iter().enumerate() {
        let at = samples_at + r.u32(ch + 4)? as usize;
        let state = ch + r.u32(ch + 12)? as usize;
        let coef: Vec<i32> = (0..16).map(|k| Ok(i32::from(i16_at(&r, state + 2 * k)?))).collect::<Result<_, FormatError>>()?;
        let (mut h1, mut h2) = (i32::from(i16_at(&r, state + 34)?), i32::from(i16_at(&r, state + 36)?));
        let bytes = r.slice(at, frames.div_ceil(14) * 8)?;
        for n in 0..frames {
            let frame = &bytes[n / 14 * 8..][..8];
            let byte = frame[1 + n % 14 / 2];
            let nibble = i32::from((if n % 2 == 0 { byte } else { byte << 4 }) as i8 >> 4);
            let pair = usize::from(frame[0] >> 4 & 7) * 2;
            let scale = 1i32 << (frame[0] & 0xF);
            let s = ((nibble * scale) << 11) + 1024 + coef[pair] * h1 + coef[pair + 1] * h2;
            let s = (s >> 11).clamp(-32768, 32767);
            (h2, h1) = (h1, s);
            out[n * channels.len() + c] = s as i16;
        }
        // The state stored for the loop start: its two samples before.
        let ls = loop_start as usize;
        if r.slice(info + 1, 1)?[0] != 0 && (2..=frames).contains(&ls) {
            let (y1, y2) = (out[(ls - 1) * channels.len() + c], out[(ls - 2) * channels.len() + c]);
            if (y1, y2) != (i16_at(&r, state + 40)?, i16_at(&r, state + 42)?) {
                return Err(FormatError::Bad(format!("DSP ADPCM: channel {c}'s samples before the loop start differ from the stored loop state")));
            }
        }
    }
    let looped = (r.slice(info + 1, 1)?[0] != 0).then_some((loop_start, frames as u32));
    Ok(Wave { rate, channels: channels.len() as u16, samples: out, looped })
}

/// A PCM16 WAV file; a looping wave gets a `smpl` chunk with its loop
/// (its last sample inclusive), which CONscripter loops by.
pub fn to_wav(w: &Wave) -> Vec<u8> {
    let data = 2 * w.samples.len() as u32;
    let le = |out: &mut Vec<u8>, v: &[u32]| v.iter().for_each(|x| out.extend(x.to_le_bytes()));
    let smpl = w.looped.map_or(0, |_| 8 + 60);
    let mut out = Vec::with_capacity(44 + smpl as usize + data as usize);
    out.extend(b"RIFF");
    out.extend((36 + smpl + data).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(w.channels.to_le_bytes());
    out.extend(w.rate.to_le_bytes());
    out.extend((w.rate * 2 * u32::from(w.channels)).to_le_bytes());
    out.extend((2 * w.channels).to_le_bytes());
    out.extend(16u16.to_le_bytes());
    if let Some((start, end)) = w.looped {
        out.extend(b"smpl");
        // manufacturer … SMPTE offset, one loop, no sampler data; loop id 0, forward, start, last, fraction, forever
        le(&mut out, &[60, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, start, end.saturating_sub(1), 0, 0]);
    }
    out.extend(b"data");
    out.extend(data.to_le_bytes());
    w.samples.iter().for_each(|s| out.extend(s.to_le_bytes()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono FWAV of one DSP frame: header and nibbles as given.
    fn fwav(frame: [u8; 8], coef: [i16; 2], frames: u32) -> Vec<u8> {
        let mut f = b"FWAV\xFF\xFE\x40\0".to_vec();
        f.extend([0u8; 8]);
        f.extend(2u16.to_le_bytes());
        f.extend([0, 0]);
        for (kind, at) in [(0x7000u16, 0x40u32), (0x7001, 0xC0)] {
            f.extend(kind.to_le_bytes());
            f.extend([0, 0]);
            f.extend(at.to_le_bytes());
            f.extend(0u32.to_le_bytes());
        }
        f.resize(0x48, 0);
        // INFO body at 0x48: DSP, no loop, 48 kHz, frames; channel table at +0x14
        f.extend([2, 0, 0, 0]);
        for v in [48000u32, 0, frames, 0, 1, 0x0100_7100, 0x0C] {
            f.extend(v.to_le_bytes());
        }
        // channel info at 0x48 + 0x20: samples at DATA + 0, ADPCM state at +0x10
        for v in [0x1F00u32, 0, 0x300, 0x10] {
            f.extend(v.to_le_bytes());
        }
        let mut state = [0i16; 19];
        state[..2].copy_from_slice(&coef);
        state.iter().for_each(|v| f.extend(v.to_le_bytes()));
        f.resize(0xC8, 0);
        f.extend(frame);
        f
    }

    #[test]
    fn dsp_adpcm() {
        // Scale 1 (2^0), coefficients 2048 and 0 (one times the last sample):
        // each sample adds its nibble to the previous one.
        let w = decode(&fwav([0x00, 0x12, 0xF0, 0, 0, 0, 0, 0], [2048, 0], 4)).unwrap();
        assert_eq!((w.rate, w.channels, w.looped), (48000, 1, None));
        assert_eq!(w.samples, [1, 3, 2, 2]);
        assert!(decode(&fwav([0; 8], [0, 0], 15)).is_err()); // a second frame is missing
        assert_eq!(&to_wav(&w)[..4], b"RIFF");
        assert_eq!(to_wav(&w).len(), 44 + 8);
        let looped = to_wav(&Wave { looped: Some((1, 2)), ..w });
        assert_eq!((looped.len(), &looped[36..40]), (44 + 8 + 68, &b"smpl"[..]));
        assert_eq!(&looped[88..96], &[1, 0, 0, 0, 1, 0, 0, 0]); // start 1, last 1
    }
}
