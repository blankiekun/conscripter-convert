//! Ogg Vorbis comment editing: sets the `LOOPSTART` and `LOOPEND` tags
//! SDL_mixer reads (since 2.0.2) to loop music from a sample frame instead
//! of the start. SDL_mixer ignores `LOOPSTART` without an end.
//!
//! An Ogg Vorbis stream starts with three header packets (identification,
//! comment, setup); audio starts on a fresh page after them. Only the
//! comment packet changes, so the identification page is copied, the
//! comment and setup packets are paginated again, and the audio pages are
//! copied with new sequence numbers and checksums.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OggError {
    #[error("not an Ogg page at offset {0:#x}")]
    BadPage(usize),
    #[error("stream ends inside the Vorbis headers")]
    Truncated,
    #[error("not a Vorbis comment header")]
    BadComment,
    #[error("audio data shares a page with the Vorbis headers")]
    MixedPage,
}

struct Page<'a> {
    header_type: u8,
    serial: u32,
    /// Lacing values.
    segments: &'a [u8],
    body: &'a [u8],
    /// The whole page.
    raw: &'a [u8],
}

fn page_at(data: &[u8], at: usize) -> Result<Page<'_>, OggError> {
    let bad = || OggError::BadPage(at);
    let head = data.get(at..at + 27).ok_or_else(bad)?;
    if &head[..4] != b"OggS" || head[4] != 0 {
        return Err(bad());
    }
    let count = usize::from(head[26]);
    let segments = data.get(at + 27..at + 27 + count).ok_or_else(bad)?;
    let body_len: usize = segments.iter().map(|&s| usize::from(s)).sum();
    let body_at = at + 27 + count;
    let body = data.get(body_at..body_at + body_len).ok_or_else(bad)?;
    Ok(Page {
        header_type: head[5],
        serial: u32::from_le_bytes(head[14..18].try_into().unwrap()),
        segments,
        body,
        raw: &data[at..body_at + body_len],
    })
}

/// The Ogg checksum: CRC-32, polynomial 0x04c11db7, not reflected, over the
/// page with its checksum field zeroed.
fn crc(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    for &b in data {
        crc ^= u32::from(b) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 { (crc << 1) ^ 0x04c1_1db7 } else { crc << 1 };
        }
    }
    crc
}

fn write_page(out: &mut Vec<u8>, header_type: u8, granule: u64, serial: u32, sequence: u32, segments: &[u8], body: &[u8]) {
    let start = out.len();
    out.extend_from_slice(b"OggS");
    out.push(0);
    out.push(header_type);
    out.extend_from_slice(&granule.to_le_bytes());
    out.extend_from_slice(&serial.to_le_bytes());
    out.extend_from_slice(&sequence.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.push(segments.len() as u8);
    out.extend_from_slice(segments);
    out.extend_from_slice(body);
    let sum = crc(&out[start..]);
    out[start + 22..start + 26].copy_from_slice(&sum.to_le_bytes());
}

/// Pages holding `packets` in order, the first starting a fresh page.
fn paginate(out: &mut Vec<u8>, serial: u32, sequence: &mut u32, packets: &[Vec<u8>]) {
    // Lacing: 255s, then the remainder (0 when the length is a multiple).
    let mut lacing: Vec<(u8, usize)> = Vec::new(); // (value, packet index)
    for (i, p) in packets.iter().enumerate() {
        lacing.extend(std::iter::repeat_n((255, i), p.len() / 255));
        lacing.push(((p.len() % 255) as u8, i));
    }
    let body: Vec<u8> = packets.concat();
    let (mut seg, mut at, mut continued) = (0, 0, false);
    while seg < lacing.len() {
        let page: Vec<u8> = lacing[seg..(seg + 255).min(lacing.len())].iter().map(|&(v, _)| v).collect();
        let len: usize = page.iter().map(|&v| usize::from(v)).sum();
        write_page(out, if continued { 1 } else { 0 }, 0, serial, *sequence, &page, &body[at..at + len]);
        *sequence += 1;
        seg += page.len();
        at += len;
        // The next page continues a packet when this one ended on a 255.
        continued = page.last() == Some(&255);
    }
}

/// `comment` with every `LOOP*` tag replaced by `LOOPSTART` and `LOOPEND`.
fn with_loop(comment: &[u8], start: u64, end: u64) -> Result<Vec<u8>, OggError> {
    let bad = || OggError::BadComment;
    let u32_at = |at: usize| comment.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(bad);
    if comment.get(..7) != Some(b"\x03vorbis") {
        return Err(bad());
    }
    let vendor_end = 11 + u32_at(7)?;
    let count = u32_at(vendor_end)?;
    let mut at = vendor_end + 4;
    let mut tags: Vec<&[u8]> = Vec::new();
    for _ in 0..count {
        let len = u32_at(at)?;
        let tag = comment.get(at + 4..at + 4 + len).ok_or_else(bad)?;
        if !tag.to_ascii_uppercase().starts_with(b"LOOP") {
            tags.push(tag);
        }
        at += 4 + len;
    }
    let new_tags = [format!("LOOPSTART={start}"), format!("LOOPEND={end}")];
    tags.extend(new_tags.iter().map(|t| t.as_bytes()));

    let mut out = comment[..vendor_end].to_vec();
    out.extend_from_slice(&(tags.len() as u32).to_le_bytes());
    for t in tags {
        out.extend_from_slice(&(t.len() as u32).to_le_bytes());
        out.extend_from_slice(t);
    }
    out.push(1); // framing bit
    Ok(out)
}

/// The Ogg Vorbis file tagged to loop from sample frame `frame` to its end
/// (the last page's granule position).
pub fn set_loop_start(data: &[u8], frame: u64) -> Result<Vec<u8>, OggError> {
    set_loop(data, frame, None)
}

/// The Ogg Vorbis file tagged to loop from sample frame `start` to `end`
/// (`None`: its end).
pub fn set_loop(data: &[u8], start: u64, end: Option<u64>) -> Result<Vec<u8>, OggError> {
    // Collect the three header packets.
    let mut packets: Vec<Vec<u8>> = vec![Vec::new()];
    let mut at = 0;
    let first = page_at(data, 0)?;
    let serial = first.serial;
    while packets.len() <= 3 {
        let page = page_at(data, at)?;
        let mut body = page.body;
        for &s in page.segments {
            if packets.len() > 3 {
                return Err(OggError::MixedPage);
            }
            let (part, rest) = body.split_at(usize::from(s));
            packets.last_mut().unwrap().extend_from_slice(part);
            body = rest;
            if s < 255 {
                packets.push(Vec::new());
            }
        }
        at += page.raw.len();
        if at >= data.len() && packets.len() <= 3 {
            return Err(OggError::Truncated);
        }
    }
    packets.pop(); // the empty packet started after the setup header
    let audio_at = at;
    // The identification header must be alone on the first page.
    if first.segments.len() != 1 || first.header_type != 2 {
        return Err(OggError::MixedPage);
    }

    let mut audio = Vec::new();
    let mut at = audio_at;
    while at < data.len() {
        let page = page_at(data, at)?;
        at += page.raw.len();
        audio.push(page);
    }
    let granule = |p: &Page| u64::from_le_bytes(p.raw[6..14].try_into().unwrap());
    let end = end.unwrap_or_else(|| audio.last().map_or(0, granule));

    let comment = with_loop(&packets[1], start, end)?;
    let mut out = first.raw.to_vec();
    let mut sequence = 1;
    paginate(&mut out, serial, &mut sequence, &[comment, packets[2].clone()]);
    for page in &audio {
        write_page(&mut out, page.header_type, granule(page), page.serial, sequence, page.segments, page.body);
        sequence += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(tag: u8, len: usize) -> Vec<u8> {
        (0..len).map(|i| tag.wrapping_add(i as u8)).collect()
    }

    fn comment(tags: &[&str]) -> Vec<u8> {
        let mut c = b"\x03vorbis".to_vec();
        c.extend_from_slice(&4u32.to_le_bytes());
        c.extend_from_slice(b"test");
        c.extend_from_slice(&(tags.len() as u32).to_le_bytes());
        for t in tags {
            c.extend_from_slice(&(t.len() as u32).to_le_bytes());
            c.extend_from_slice(t.as_bytes());
        }
        c.push(1);
        c
    }

    /// A stream: id page, comment + setup pages, two audio pages.
    fn stream(tags: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        write_page(&mut out, 2, 0, 7, 0, &[30], &packet(1, 30));
        let mut seq = 1;
        paginate(&mut out, 7, &mut seq, &[comment(tags), packet(5, 600)]);
        write_page(&mut out, 0, 1000, 7, seq, &[100], &packet(9, 100));
        write_page(&mut out, 4, 2000, 7, seq + 1, &[50], &packet(11, 50));
        out
    }

    fn pages(data: &[u8]) -> Vec<Page<'_>> {
        let mut at = 0;
        let mut v = Vec::new();
        while at < data.len() {
            let p = page_at(data, at).unwrap();
            at += p.raw.len();
            v.push(p);
        }
        v
    }

    #[test]
    fn replaces_loop_tags_and_keeps_audio() {
        let out = set_loop_start(&stream(&["TITLE=a", "LOOPSTART=5", "LoopLength=9"]), 323233).unwrap();
        let expected = stream(&["TITLE=a", "LOOPSTART=323233", "LOOPEND=2000"]);
        assert_eq!(out, expected);
        let out = set_loop(&stream(&["TITLE=a"]), 10, Some(1500)).unwrap();
        assert_eq!(out, stream(&["TITLE=a", "LOOPSTART=10", "LOOPEND=1500"]));
        for (i, p) in pages(&out).iter().enumerate() {
            let seq = u32::from_le_bytes(p.raw[18..22].try_into().unwrap());
            assert_eq!(seq as usize, i);
            let mut zeroed = p.raw.to_vec();
            zeroed[22..26].fill(0);
            assert_eq!(crc(&zeroed), u32::from_le_bytes(p.raw[22..26].try_into().unwrap()));
        }
    }

    #[test]
    fn long_packets_span_pages() {
        let mut out = Vec::new();
        let mut seq = 0;
        paginate(&mut out, 1, &mut seq, &[packet(0, 255 * 300), packet(1, 10)]);
        let p = pages(&out);
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].header_type, 1);
        assert_eq!(p.iter().map(|p| p.body.len()).sum::<usize>(), 255 * 300 + 10);
    }
}
