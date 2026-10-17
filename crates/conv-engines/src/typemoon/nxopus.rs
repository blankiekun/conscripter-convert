//! Nintendo Opus (`stream/*.opus`, the voices and music) to Ogg Opus.
//!
//! Header chunk: u32 `0x80000001`, u32 size, u8 version, u8 channels,
//! u16 frame size, u32 sample rate, u32 data chunk offset, u32, u32
//! context offset, u16 pre-skip (as vgmstream reads it). Data chunk: u32
//! `0x80000004`, u32 size, then frames: u32 BE packet size, u32 BE final
//! range, the Opus packet. No sample count: it comes from the packets.

use super::FormatError;
use crate::ogg::write_page;
use crate::read::Reader;

/// Samples (at 48 kHz) in an Opus packet, from its TOC byte (RFC 6716 3.1).
fn packet_samples(packet: &[u8]) -> Result<u64, FormatError> {
    let toc = *packet.first().ok_or_else(|| FormatError::Bad("empty Opus packet".into()))?;
    let config = toc >> 3;
    let frame = match config {
        0..=11 => [480, 960, 1920, 2880][usize::from(config % 4)],
        12..=15 => [480, 960][usize::from(config % 2)],
        _ => [120, 240, 480, 960][usize::from(config % 4)],
    };
    let frames = match toc & 3 {
        0 => 1,
        1 | 2 => 2,
        _ => u64::from(*packet.get(1).ok_or_else(|| FormatError::Bad("Opus packet cut after its TOC".into()))? & 0x3F),
    };
    Ok(frame * frames)
}

/// The Ogg Opus file and its length in samples (pre-skip included).
/// `looped`: its loop (`bfsar::stream_loops`), written as `LOOPSTART` and
/// `LOOPEND` tags, which CONscripter loops by.
pub fn to_ogg(data: &[u8], looped: Option<(u32, u32)>) -> Result<(Vec<u8>, u64), FormatError> {
    let r = Reader(data);
    if r.u32(0)? != 0x8000_0001 {
        return Err(FormatError::BadMagic("Nintendo Opus"));
    }
    let channels = r.slice(9, 1)?[0];
    let rate = r.u32(0xC)?;
    let pre_skip = r.u16(0x1C)?;
    let data_at = r.u32(0x10)? as usize;
    if r.u32(data_at)? != 0x8000_0004 || !(1..=2).contains(&channels) {
        return Err(FormatError::Bad(format!("Nintendo Opus: data chunk {:#x}, {channels} channels", r.u32(data_at)?)));
    }
    let end = data_at + 8 + r.u32(data_at + 4)? as usize;
    r.slice(data_at, end - data_at)?;

    let mut out = Vec::new();
    let serial = 1;
    let mut head = b"OpusHead\x01".to_vec();
    head.push(channels);
    head.extend(pre_skip.to_le_bytes());
    head.extend(rate.to_le_bytes());
    head.extend([0, 0, 0]); // gain 0, mapping family 0 (mono or stereo)
    write_page(&mut out, 2, 0, serial, 0, &[head.len() as u8], &head);
    // Vendor "conscripter", then the comments; opusfile refuses a wrong length.
    let comments: Vec<String> = looped.map(|(s, e)| vec![format!("LOOPSTART={s}"), format!("LOOPEND={e}")]).unwrap_or_default();
    let mut tags = b"OpusTags\x0b\0\0\0conscripter".to_vec();
    tags.extend((comments.len() as u32).to_le_bytes());
    for c in &comments {
        tags.extend((c.len() as u32).to_le_bytes());
        tags.extend(c.as_bytes());
    }
    write_page(&mut out, 0, 0, serial, 1, &[tags.len() as u8], &tags);

    let (mut at, mut granule, mut sequence) = (data_at + 8, 0u64, 2);
    while at < end {
        let size = u32::from_be_bytes(r.slice(at, 4)?.try_into().unwrap()) as usize;
        let packet = r.slice(at + 8, size)?;
        at += 8 + size;
        granule += packet_samples(packet)?;
        // One packet a page: lacing is 255s and the remainder.
        let mut lacing = vec![255u8; size / 255];
        lacing.push((size % 255) as u8);
        if lacing.len() > 255 {
            return Err(FormatError::Bad(format!("Opus packet of {size} bytes")));
        }
        write_page(&mut out, if at >= end { 4 } else { 0 }, granule, serial, sequence, &lacing, packet);
        sequence += 1;
    }
    Ok((out, granule))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toc_durations() {
        assert_eq!(packet_samples(&[0xFC]).unwrap(), 960); // CELT 20 ms, 1 frame
        assert_eq!(packet_samples(&[1 << 3 | 3, 3]).unwrap(), 3 * 960); // SILK 20 ms, code 3: 3 frames
        assert!(packet_samples(&[1 << 3 | 3]).is_err());
        assert_eq!(packet_samples(&[0x81]).unwrap(), 2 * 120); // CELT 2.5 ms, 2 frames
    }

    #[test]
    fn wraps_packets() {
        let mut nx = Vec::new();
        for v in [0x8000_0001u32, 0x18, 0x0100, 48000, 0x20, 0, 0, 120] {
            nx.extend(v.to_le_bytes());
        }
        nx.extend(0x8000_0004u32.to_le_bytes());
        nx.extend(22u32.to_le_bytes());
        for packet in [&[0xFCu8, 1, 2][..], &[0xFC, 3, 4, 5, 6, 7]] {
            nx.extend((packet.len() as u32).to_be_bytes());
            nx.extend([0; 4]);
            nx.extend(packet);
        }
        let (ogg, samples) = to_ogg(&nx, None).unwrap();
        assert_eq!(samples, 1920);
        assert_eq!(ogg.windows(8).filter(|w| w == b"OpusHead").count(), 1);
        assert_eq!(ogg.windows(4).filter(|w| w == b"OggS").count(), 4);
        // OpusTags: its vendor length covers the vendor string, then a count of 0.
        let tags = ogg.windows(8).position(|w| w == b"OpusTags").unwrap() + 8;
        let vendor = u32::from_le_bytes(ogg[tags..tags + 4].try_into().unwrap()) as usize;
        assert_eq!(&ogg[tags + 4..tags + 4 + vendor], b"conscripter");
        assert_eq!(&ogg[tags + 4 + vendor..tags + 8 + vendor], &[0; 4]);
        assert!(to_ogg(&nx[..nx.len() - 1], None).is_err());
        let (ogg, _) = to_ogg(&nx, Some((5, 900))).unwrap();
        let tags = ogg.windows(8).position(|w| w == b"OpusTags").unwrap() + 8 + 4 + vendor;
        assert_eq!(&ogg[tags..tags + 4], &2u32.to_le_bytes());
        assert_eq!(&ogg[tags + 4..tags + 8], &11u32.to_le_bytes());
        assert_eq!(&ogg[tags + 8..tags + 19], b"LOOPSTART=5");
        assert_eq!(&ogg[tags + 23..tags + 34], b"LOOPEND=900");
    }
}
