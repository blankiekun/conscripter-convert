//! Sofdec2 USM movies (`media/data/*.usm`): chunks of a 4-byte signature
//! (`CRID`, `@SFV` video, `@SFA` audio), a big-endian u32 size, then
//! u8, u8 payload offset (from byte 8), u16 padding, u8 channel, u16,
//! u8 type (0 data, 1 header, 2 comment, 3 seek table). The data payloads
//! of a stream, in order, make its elementary stream: here MPEG-1 video
//! (960×544, 60 fps) and ADX audio, neither encrypted.

/// The video and audio elementary streams of channel 0.
pub fn demux(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut video = Vec::new();
    let mut audio = Vec::new();
    let mut p = 0;
    while p + 16 <= data.len() {
        let sig = &data[p..p + 4];
        let size = u32::from_be_bytes(data[p + 4..p + 8].try_into().ok()?) as usize;
        let offset = data[p + 9] as usize;
        let padding = u16::from_be_bytes(data[p + 10..p + 12].try_into().ok()?) as usize;
        let (channel, kind) = (data[p + 12], data[p + 15]);
        let end = p + 8 + size;
        if end > data.len() || offset + padding > size {
            return None;
        }
        if kind == 0 && channel == 0 {
            let payload = &data[p + 8 + offset..end - padding];
            match sig {
                b"@SFV" => video.extend_from_slice(payload),
                b"@SFA" => audio.extend_from_slice(payload),
                _ => {}
            }
        }
        p = end;
    }
    (!video.is_empty()).then_some((video, audio))
}

/// The movie files in `:Movie` number order (checked against the scenes
/// that play them: the heroines' endings play their `ED_*`).
pub const MOVIES: [&str; 12] = [
    "OP00", "OP01", "OP02", "ED_ai", "ED_asuka", "ED_hiyoko", "ED_kobato", "ED_misaki", "ED_miyako", "ED_naru", "Poem", "OP03",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(sig: &[u8], kind: u8, payload: &[u8], padding: usize) -> Vec<u8> {
        let size = 0x18 + payload.len() + padding;
        let mut c = sig.to_vec();
        c.extend((size as u32).to_be_bytes());
        c.extend([0, 0x18]);
        c.extend((padding as u16).to_be_bytes());
        c.extend([0, 0, 0, kind]);
        c.extend([0u8; 0x10]);
        c.extend(payload);
        c.extend(vec![0u8; padding]);
        c
    }

    #[test]
    fn streams() {
        let mut d = chunk(b"CRID", 1, b"@UTF", 0);
        d.extend(chunk(b"@SFV", 1, b"@UTF", 0));
        d.extend(chunk(b"@SFV", 0, b"\x00\x00\x01\xb3", 4));
        d.extend(chunk(b"@SFA", 0, b"\x80\x00", 0));
        d.extend(chunk(b"@SFV", 0, b"ab", 0));
        let (v, a) = demux(&d).unwrap();
        assert_eq!(v, b"\x00\x00\x01\xb3ab");
        assert_eq!(a, b"\x80\x00");
    }
}
