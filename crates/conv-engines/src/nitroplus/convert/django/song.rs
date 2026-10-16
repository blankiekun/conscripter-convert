//! dja026's song (`ShowWords`): each `CreateText("songtxtNN", …,
//! "<I>荒.野.開.く…</I>")` line types out from its `Request(…, Enter)`.
//! The exe's text layout (`layout_create_text`) makes each `.` a 100 ms
//! pause before the next letter (game speed NORMAL). Under wine (text
//! speed 500) a letter comes every 125 ms, the full-width space included
//! (the first two lines: 9 and 10 steps in 1,170 and 1,213 ms), the
//! first one about as long after `Enter`. Ours keeps that time whatever
//! the text speed setting (the original's 25 ms beside the dot may follow
//! it: unchecked).

use super::super::text_object::{Style, draw};
use super::super::{Compiler, Kind, StripObject, Value};

/// The time a letter takes, measured under wine (100 of it the dot).
const LETTER_MS: u32 = 125;

/// Where a line's letters wait for its `Enter`.
fn key(name: &str) -> String {
    format!("$SongLine/{name}")
}

/// The time a line entered takes to type, until its `WaitAction`.
fn wait_key(name: &str) -> String {
    format!("$SongWait/{name}")
}

impl Compiler<'_, '_> {
    /// `CreateText` of a song line: its letters are kept for its `Enter`;
    /// the picture is made as for any `CreateText`.
    pub(super) fn song_line(&mut self, args: &[Value]) {
        let text = args.get(6).map(Value::string).unwrap_or_default();
        if let Some(inner) = text.strip_prefix("<I>").and_then(|t| t.strip_suffix("</I>")) {
            let name = self.object_name(&args[0]);
            self.vars.insert(key(&name), Value::Str(inner.replace('.', "")));
        }
    }

    /// `Request(line, Enter)`: the line becomes a strip of its first 0, 1,
    /// … letters, shown in turn from now.
    pub(super) fn song_enter(&mut self, args: &[Value]) -> bool {
        if !args.get(1).is_some_and(|a| a.string().eq_ignore_ascii_case("enter")) {
            return false;
        }
        let name = self.object_name(&args[0]);
        let Some(Value::Str(text)) = self.vars.get(&key(&name)).cloned() else { return false };
        let Some(o) = self.objects.get(&name) else { return false };
        let Kind::Image { image } = &o.kind else { return false };
        let (last, w, h) = (image.clone(), o.w, o.h);
        let letters: Vec<char> = text.chars().collect();
        let cells = letters.len() as u32 + 1;
        let style = Style::from_set_font(&self.font);
        let font = self.assets.font().to_vec();
        let file = format!("{}_typed", last.trim_start_matches("system/strip/").trim_end_matches(".png"));
        let strip = self.assets.frames_strip(&file, w as u32, h as u32, cells, &mut |i, out| {
            let shown: String = letters[..i as usize].iter().collect();
            if let Some(img) = draw(&font, &shown, &style, None) {
                image::imageops::replace(out, &img, 0, 0);
            }
        });
        let Some(strip) = strip else { return false };
        let o = self.objects.get_mut(&name).unwrap();
        o.kind = Kind::Strip(StripObject { image: strip, cells, frame_ms: LETTER_MS, w, h, zoom: (100, 100), last });
        o.erased = false;
        // Shown now, so that it types (a sync over time would fade it in).
        self.sync(0);
        self.vars.insert(wait_key(&name), Value::Int(i64::from(LETTER_MS * (cells - 1))));
        true
    }

    /// `WaitAction(line)` after its `Enter`: until its last letter.
    pub(super) fn song_wait(&mut self, args: &[Value]) -> bool {
        let Some(Value::Int(ms)) = args.first().and_then(|a| self.vars.remove(&wait_key(&self.object_name(a)))) else { return false };
        self.sync(ms as u32);
        true
    }
}
