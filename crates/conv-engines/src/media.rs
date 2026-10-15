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

#[cfg(test)]
mod tests {
    use super::*;

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
