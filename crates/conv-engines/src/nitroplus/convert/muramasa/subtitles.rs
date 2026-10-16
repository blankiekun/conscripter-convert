//! The English port's subtitles for voices played as sound effects
//! (`ShowVOICESEText` in `function.nut`, put into the NSS by
//! `english.rs`): a centred line in `serif0` (`function_textbox.nut`: the
//! screen's width at y 480, over everything, `字幕` style: Droid Serif
//! Muramasa 20, white with a dark edge), shown at once and changed at the
//! times its `TransVoice` list gives; the port clears it as the voice ends
//! (the list's process, or `ClearVOICESEText` after waiting for it).

use std::hash::{Hash, Hasher};

use ab_glyph::{Font, FontRef};
use image::imageops;

use super::super::{Compiler, Kind, StripObject, Value};
use crate::labels::Labels;

/// `serif0`'s priority: over the text window.
const PRIORITY: i64 = 20005;
/// The `字幕` style's size (SDL_ttf's: the em) and line pitch (`rowSpace` 1).
const SIZE: f32 = 20.0;
const PITCH: i32 = 21;
/// A cell of the subtitle's strip: its changes land on these steps.
const FRAME_MS: u32 = 250;
/// Shown this long when the voice's length is unknown.
const UNKNOWN_MS: u32 = 3000;
/// [`Labels::draw`]'s margin around the letters.
const PAD: i32 = 4;

impl Compiler<'_, '_> {
    /// `ShowVOICESEText(sound, add, y, file, ms, text, ms, text, …)`: the lines
    /// as a strip of [`FRAME_MS`] cells, the last one empty at the voice's
    /// end. `add` puts it a line under the one shown (its own object).
    pub(in super::super) fn voice_subtitle(&mut self, args: &[Value]) {
        let (Some(add), Some(y), Some(file)) = (args.get(1), args.get(2), args.get(3)) else { return };
        let lines: Vec<(u32, String)> = args[4..].chunks(2).filter_map(|p| match p {
            [ms, text] => Some((ms.int().max(0) as u32, text.string())),
            _ => None,
        }).collect();
        let Some(font) = self.assets.english_font() else {
            self.note("ShowVOICESEText without the English font");
            return;
        };
        // The voice's object is gone once it has started.
        let path = format!("voice/{}", file.string());
        let last = lines.last().map_or(0, |l| l.0);
        let end = self.assets.sound_ms(&path).unwrap_or(last + UNKNOWN_MS).max(last + FRAME_MS);
        let (Some(labels), Ok(face)) = (Labels::new(&font), FontRef::try_from_slice(&font)) else { return };
        // ab_glyph scales by the line's height, SDL_ttf by the em.
        let size = SIZE * face.height_unscaled() / face.units_per_em().unwrap_or(1000.0);
        let drawn: Vec<_> = lines.iter().map(|(_, t)| (!t.is_empty()).then(|| labels.draw(t, size, [255, 255, 255], [0, 0, 0], 255))).collect();
        let w = drawn.iter().flatten().map(|l| l.width()).max().unwrap_or(1);
        let h = drawn.iter().flatten().map(|l| l.height()).max().unwrap_or(1);
        let cells = end.div_ceil(FRAME_MS) + 1;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        (&lines, end, w, h).hash(&mut hash);
        let name = format!("subtitle_{:016x}", hash.finish());
        let image = self.assets.frames_strip(&name, w, h, cells, &mut |i, out| {
            let t = i * FRAME_MS;
            if t >= end {
                return;
            }
            if let Some(Some(l)) = lines.iter().rposition(|(ms, _)| *ms <= t).map(|k| &drawn[k]) {
                imageops::overlay(out, l, i64::from((w - l.width()) / 2), 0);
            }
        });
        let blank = self.assets.frames_strip(&format!("subtitle_blank_{w}x{h}"), w, h, 1, &mut |_, _| {});
        let (Some(image), Some(blank)) = (image, blank) else { return };
        let strip = StripObject { image, cells, frame_ms: FRAME_MS, w: w as i64, h: h as i64, zoom: (100, 100), last: blank };
        let object = if add.truthy() { "serif0+" } else { "serif0" };
        self.delete(object);
        let o = self.create(object.into(), Kind::Strip(strip), PRIORITY, w as i64, h as i64);
        o.over_text = true;
        o.x = (super::super::SCREEN_W - w as i64) / 2;
        o.y = y.int() + i64::from(if add.truthy() { PITCH } else { 0 }) - i64::from(PAD);
    }
}
