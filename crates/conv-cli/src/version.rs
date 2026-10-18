//! The version line every build carries, for players to read: on the
//! settings screen ([`Script::version`](conv_core::ir::Script)) and in
//! `version.txt` beside `0.txt`. It names the day of the build, the
//! converter's commit and the CONscripter version it was made for.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use conv_engines::media::{AudioCodec, AudioFormat};

/// `build 2026-10-14, converter 562cbe2, CONscripter 0.17.0`.
pub fn line() -> String {
    format!(
        "build {}, converter {}, CONscripter {}",
        today(),
        env!("CONV_COMMIT"),
        env!("CONV_ENGINE_VERSION")
    )
}

/// The media flags a build used, for `version.txt`:
/// `--webp --audio-codec opus`, or `none (PNG, Vorbis)`.
pub fn media(webp: Option<Option<u8>>, audio: AudioFormat) -> String {
    let mut flags = Vec::new();
    if let Some(quality) = webp {
        flags.push("--webp".to_string());
        flags.extend(quality.map(|q| format!("--webp-quality {q}")));
    }
    if audio.codec == AudioCodec::Opus {
        flags.push("--audio-codec opus".into());
    }
    flags.extend(audio.quality.map(|q| format!("--audio-quality {q}")));
    if flags.is_empty() { "none (PNG, Vorbis)".into() } else { flags.join(" ") }
}

/// Writes `version.txt`: the title, the line, what it was made from
/// (`source`, such as the original's ID and version) when known, and the
/// media flags ([`media`]).
pub fn write(output: &Path, title: &str, line: &str, source: Option<&str>, media: &str) -> Result<()> {
    let mut text = format!("{title}\r\n{line}\r\n");
    if let Some(source) = source {
        text += &format!("from {source}\r\n");
    }
    text += &format!("media: {media}\r\n");
    fs::write(output.join("version.txt"), text)?;
    Ok(())
}

/// Today's date (UTC) as YYYY-MM-DD.
fn today() -> String {
    let days = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    let (y, m, d) = civil(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 to (year, month, day), from Howard Hinnant's
/// `civil_from_days`.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::{civil, media};
    use conv_engines::media::{AudioCodec, AudioFormat};

    #[test]
    fn media_flags() {
        let vorbis = AudioFormat { codec: AudioCodec::Vorbis, quality: None };
        assert_eq!(media(None, vorbis), "none (PNG, Vorbis)");
        assert_eq!(media(Some(None), AudioFormat { codec: AudioCodec::Opus, quality: None }), "--webp --audio-codec opus");
        assert_eq!(media(Some(Some(80)), AudioFormat { codec: AudioCodec::Opus, quality: Some(24.0) }), "--webp --webp-quality 80 --audio-codec opus --audio-quality 24");
    }

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(civil(20_740), (2026, 10, 14));
    }
}
