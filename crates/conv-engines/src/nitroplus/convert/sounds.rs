//! How long a sound has played, from the time the ops emitted so far
//! take: their waits and fades, and a long while for each wait for the
//! player (a click, a page, a choice). `WaitPlay` waits for the rest of a
//! sound, and a disused one-shot (`Request(…, Disused)`) stays matched by
//! wildcards until its end, as in the exe (`FUN_00436040` deletes a
//! disused object each frame only once no action runs on it and its
//! sound has stopped).

use conv_core::ir::Op;

use super::{Compiler, Kind};

/// A wait for the player: past the end of any one-shot.
const PLAYER_MS: u64 = 3_600_000;

impl Compiler<'_, '_> {
    /// The time the ops emitted so far take, ms.
    pub(super) fn sound_clock(&mut self) -> u64 {
        // Ops inserted behind the scanned ones (a movie before its
        // transition's end) are skipped: they don't wait.
        for op in self.ops.get(self.clock_scanned..).unwrap_or_default() {
            self.clock_ms += match op {
                Op::Wait { ms } | Op::Pause { ms } => u64::from(*ms),
                Op::BeginTransition { fade_ms, .. } => u64::from(*fade_ms),
                Op::ClickWait { .. } | Op::Choice { .. } => PLAYER_MS,
                // A page that stops for code inside it doesn't wait.
                Op::Text { continues: false, text, .. } if !text.ends_with(conv_core::ir::GOES_ON_MARK) => PLAYER_MS,
                _ => 0,
            };
        }
        self.clock_scanned = self.ops.len();
        self.clock_ms
    }

    /// The ms left of the one-shot sound `name` (its length when it is
    /// only starting); `None` for anything else or an unknown length.
    pub(super) fn sound_left(&mut self, name: &str) -> Option<u32> {
        let now = self.sound_clock();
        let Some(Kind::Sound(s)) = self.objects.get(name).map(|o| &o.kind) else { return None };
        if s.looped || (s.playing.is_none() && !s.starting) {
            return None;
        }
        let len = s.len_ms?;
        if s.starting {
            return Some(len);
        }
        Some(len.saturating_sub(now.saturating_sub(s.started_at).min(u64::from(u32::MAX)) as u32))
    }

    /// The ms left of sound `name`'s fade to 0 (`SetVolume`), or of the
    /// one-shot itself if it ends first; `None` if it isn't fading out.
    pub(super) fn fade_left(&mut self, name: &str) -> Option<u32> {
        let now = self.sound_clock();
        let Some(Kind::Sound(s)) = self.objects.get(name).map(|o| &o.kind) else { return None };
        let mut end = s.stops_at?;
        if let (false, Some(len)) = (s.looped, s.len_ms) {
            end = end.min(s.started_at + u64::from(len));
        }
        Some(end.saturating_sub(now).min(u64::from(u32::MAX)) as u32)
    }

    /// Disused one-shots that have ended, and disused sounds faded out to
    /// 0, are forgotten, unless locked (`CreateBGM`'s music, which
    /// 装甲悪鬼村正's `MusicStart` marks disused, is played again).
    pub(super) fn forget_ended_sounds(&mut self) {
        let ended: Vec<String> = self
            .objects
            .iter()
            .filter(|(_, o)| !o.locked && matches!(&o.kind, Kind::Sound(s) if s.disused && !s.starting))
            .map(|(n, _)| n.clone())
            .collect();
        for name in ended {
            // An unknown end (no length, or started looped) keeps it.
            if self.sound_left(&name) == Some(0) || self.fade_left(&name) == Some(0) {
                self.objects.remove(&name);
            }
        }
    }
}
