//! Sounds and movies made with ffmpeg, for every engine.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// The output options of a movie CONscripter plays: H.264 and AAC in an
/// MP4 that starts playing before it has fully loaded (the web build's
/// `<video>`), at quality `crf` and encoder speed `preset`.
pub fn mp4_options(crf: u32, preset: &str) -> Vec<String> {
    let crf = crf.to_string();
    ["-c:v", "libx264", "-preset", preset, "-crf", &crf, "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart"]
        .map(String::from)
        .to_vec()
}

/// Makes `dest` with ffmpeg: `args` are the inputs and options, `input`
/// writes what an input `-` (or `pipe:0`) reads. ffmpeg writes to
/// `<name>.part.<ext>`, renamed to `dest` once it succeeds: a build skips
/// files that exist, so an interrupted run must not leave half a file
/// under the real name. The error is ffmpeg's message.
pub fn ffmpeg<A: AsRef<OsStr>>(dest: &Path, args: impl IntoIterator<Item = A>, input: impl FnOnce(&mut dyn Write) -> io::Result<()>) -> Result<(), String> {
    let ext = dest.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let part = dest.with_extension(format!("part.{ext}"));
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let result = (|| {
        let mut child = Command::new("ffmpeg")
            .args(["-v", "error", "-y"])
            .args(args)
            .arg(&part)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not run ffmpeg: {e}"))?;
        let mut stdin = child.stdin.take().unwrap();
        // ffmpeg may stop reading early; its exit status tells.
        let written = input(&mut stdin);
        drop(stdin);
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            let message = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if message.is_empty() { out.status.to_string() } else { message });
        }
        written.map_err(|e| e.to_string())?;
        fs::rename(&part, dest).map_err(|e| format!("{}: {e}", dest.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result
}

/// The codec of voices and sound effects (`--audio-codec`). Music stays
/// Ogg Vorbis: CONscripter decodes Opus music whole into memory.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AudioCodec {
    #[default]
    Vorbis,
    /// Ogg Opus: about half the size of Vorbis at the same quality, most
    /// of all for speech. Needs CONscripter 0.21 or later.
    Opus,
}

/// How voices and sound effects are encoded: `--audio-codec` and
/// `--audio-quality` (Vorbis: ffmpeg's `-q:a`, -1 to 10; Opus: kbit/s per
/// channel, 6 to 256).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AudioFormat {
    pub codec: AudioCodec,
    pub quality: Option<f32>,
}

/// Opus's bit rate per channel when no quality is given: speech at 32
/// kbit/s sounds like Vorbis at about twice that.
pub const OPUS_DEFAULT_KBPS: f32 = 32.0;
/// Vorbis's quality when none is given (the games' own sounds keep theirs).
pub const VORBIS_DEFAULT_QUALITY: f32 = 4.0;

impl AudioFormat {
    /// Whether the game's own Ogg Vorbis sounds are re-encoded (otherwise
    /// they are copied as they are).
    pub fn reencodes(&self) -> bool {
        self.codec == AudioCodec::Opus || self.quality.is_some()
    }

    /// The comment that marks a sound made with this format, so a rebuild
    /// over loose files doesn't encode it again.
    pub fn tag(&self) -> String {
        match self.codec {
            AudioCodec::Vorbis => format!("CONSCRIPTER_AUDIO=vorbis {}", self.quality.unwrap_or(VORBIS_DEFAULT_QUALITY)),
            AudioCodec::Opus => format!("CONSCRIPTER_AUDIO=opus {}", self.quality.unwrap_or(OPUS_DEFAULT_KBPS)),
        }
    }

    /// ffmpeg's output options for a sound with `channels` channels.
    pub fn ffmpeg_options(&self, channels: u8) -> Vec<String> {
        let mut o: Vec<String> = match self.codec {
            AudioCodec::Vorbis => {
                vec!["-c:a".into(), "libvorbis".into(), "-q:a".into(), self.quality.unwrap_or(VORBIS_DEFAULT_QUALITY).to_string()]
            }
            AudioCodec::Opus => {
                let kbps = self.quality.unwrap_or(OPUS_DEFAULT_KBPS) * f32::from(channels.clamp(1, 8));
                vec!["-c:a".into(), "libopus".into(), "-b:a".into(), format!("{}k", kbps.round()), "-vbr".into(), "on".into()]
            }
        };
        o.extend(["-metadata".into(), self.tag()]);
        o
    }
}

/// The channel count of an Ogg Vorbis, Ogg Opus, CRI ADX or CRI HCA
/// sound, from its header.
pub fn channels(data: &[u8]) -> Option<u8> {
    if data.starts_with(&[0x80, 0x00]) {
        return data.get(7).copied(); // ADX
    }
    // HCA: "HCA\0", version, header size, then "fmt\0" and the channels;
    // the names may have their high bits set (encrypted files)
    let unmasked = |at: usize, name: &[u8; 4]| data.get(at..at + 4).is_some_and(|b| b.iter().zip(name).all(|(x, n)| x & 0x7f == *n));
    if unmasked(0, b"HCA\0") && unmasked(8, b"fmt\0") {
        return data.get(12).copied();
    }
    if !data.starts_with(b"OggS") {
        return None;
    }
    let packet = 27 + usize::from(*data.get(26)?);
    let p = data.get(packet..)?;
    if p.starts_with(b"\x01vorbis") {
        p.get(11).copied()
    } else if p.starts_with(b"OpusHead") {
        p.get(9).copied()
    } else {
        None
    }
}

/// Totals of [`reencode_dirs`].
#[derive(Debug, Default)]
pub struct Reencoded {
    pub files: usize,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub failed: Vec<String>,
}

/// Re-encodes the `.ogg` sounds under `dirs` (and their folders) as
/// `format`, in place, with ffmpeg on all CPU cores. Their tags go (only
/// music has loop tags, and music isn't given here); sounds already
/// carrying `format`'s tag are left alone.
pub fn reencode_dirs(dirs: &[std::path::PathBuf], format: AudioFormat) -> Reencoded {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("ogg")) {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    for d in dirs {
        walk(d, &mut files);
    }
    files.sort();
    let tag = format.tag();
    let next = AtomicUsize::new(0);
    let totals = Mutex::new(Reencoded::default());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                while let Some(path) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let data = match fs::read(path) {
                        Ok(d) => d,
                        Err(e) => {
                            totals.lock().unwrap().failed.push(format!("{}: {e}", path.display()));
                            continue;
                        }
                    };
                    let head = &data[..data.len().min(8192)];
                    if head.windows(tag.len()).any(|w| w == tag.as_bytes()) {
                        continue;
                    }
                    let mut args: Vec<String> = vec!["-i".into(), "pipe:0".into(), "-map".into(), "0:a:0".into(), "-map_metadata".into(), "-1".into()];
                    args.extend(format.ffmpeg_options(channels(&data).unwrap_or(2)));
                    let result = ffmpeg(path, args, |stdin| stdin.write_all(&data));
                    let mut t = totals.lock().unwrap();
                    match result {
                        Ok(()) => {
                            t.files += 1;
                            t.bytes_before += data.len() as u64;
                            t.bytes_after += fs::metadata(path).map_or(0, |m| m.len());
                        }
                        Err(e) => t.failed.push(format!("{}: {e}", path.display())),
                    }
                }
            });
        }
    });
    totals.into_inner().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_options() {
        let opus = AudioFormat { codec: AudioCodec::Opus, quality: None };
        assert_eq!(opus.ffmpeg_options(1)[..4], ["-c:a", "libopus", "-b:a", "32k"]);
        assert_eq!(opus.ffmpeg_options(2)[3], "64k");
        assert!(opus.reencodes());
        let vorbis = AudioFormat { codec: AudioCodec::Vorbis, quality: Some(1.0) };
        assert_eq!(vorbis.ffmpeg_options(2)[..4], ["-c:a", "libvorbis", "-q:a", "1"]);
        assert!(vorbis.reencodes());
        assert!(!AudioFormat::default().reencodes());
        assert_eq!(vorbis.ffmpeg_options(1).last().unwrap(), "CONSCRIPTER_AUDIO=vorbis 1");
    }

    #[test]
    fn channels_from_headers() {
        let mut ogg = b"OggS".to_vec();
        ogg.resize(26, 0);
        ogg.push(1); // one segment
        ogg.push(30);
        ogg.extend(b"\x01vorbis\0\0\0\0\x02");
        assert_eq!(channels(&ogg), Some(2));
        assert_eq!(channels(&[0x80, 0, 0, 0x20, 3, 18, 4, 1]), Some(1)); // ADX
        assert_eq!(channels(b"HCA\0\x02\0\0\x60fmt\0\x01\0\x56\x22"), Some(1));
        assert_eq!(channels(b"\xc8\xc3\xc1\0\x02\0\0\x60\xe6\xed\xf4\0\x02\0"), Some(2)); // masked
        assert_eq!(channels(b"RIFF"), None);
    }

    /// Voice and SE re-encoded to Opus in place, once. Skipped without
    /// ffmpeg.
    #[test]
    fn reencodes_once() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("conv-reencode-test-{}", std::process::id()));
        let file = dir.join("voice/a/v.ogg");
        ffmpeg(&file, ["-f", "lavfi", "-i", "sine=f=440:d=1", "-c:a", "libvorbis"], |_| Ok(())).unwrap();
        let opus = AudioFormat { codec: AudioCodec::Opus, quality: Some(16.0) };
        let r = reencode_dirs(&[dir.join("voice")], opus);
        assert_eq!((r.files, r.failed.len()), (1, 0), "{:?}", r.failed);
        let data = fs::read(&file).unwrap();
        assert_eq!(channels(&data), Some(1));
        assert!(data[..200].windows(8).any(|w| w == b"OpusHead"));
        assert_eq!(reencode_dirs(&[dir.join("voice")], opus).files, 0); // tagged: left alone
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mp4_options_name_the_quality() {
        let o = mp4_options(22, "slow");
        assert_eq!(o[..6], ["-c:v", "libx264", "-preset", "slow", "-crf", "22"]);
        assert!(o.ends_with(&["-movflags".to_string(), "+faststart".to_string()]));
    }

    /// Frames written to ffmpeg's input become the file, under its real
    /// name only once done; a failed run leaves nothing. Skipped without
    /// ffmpeg.
    #[test]
    fn writes_through_a_part_file() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("conv-media-test-{}", std::process::id()));
        let dest = dir.join("m/a.mp4");
        let args = ["-f", "rawvideo", "-pix_fmt", "rgb24", "-s", "16x16", "-r", "10", "-i", "-", "-c:v", "libx264", "-pix_fmt", "yuv420p"];
        ffmpeg(&dest, args, |stdin| (0..5).try_for_each(|_| stdin.write_all(&[128; 16 * 16 * 3]))).unwrap();
        assert!(dest.exists() && !dir.join("m/a.part.mp4").exists());
        let bad = dir.join("m/b.mp4");
        assert!(ffmpeg(&bad, ["-f", "nonsense", "-i", "-"], |_| Ok(())).is_err());
        assert!(!bad.exists() && !dir.join("m/b.part.mp4").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
