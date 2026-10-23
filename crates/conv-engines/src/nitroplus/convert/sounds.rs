//! Sound: starting sounds and changing their volume, pan and speed.
//!
//! How long a sound has played, from the time the ops emitted so far
//! take: their waits and fades, and a long while for each wait for the
//! player (a click, a page, a choice). `WaitPlay` waits for the rest of a
//! sound, and a disused one-shot (`Request(…, Disused)`) stays matched by
//! wildcards until its end, as in the exe (`FUN_00436040` deletes a
//! disused object each frame only once no action runs on it and its
//! sound has stopped).

use conv_core::ir::Op;

use super::{Compiler, Kind, Object, Value};

/// A wait for the player: past the end of any one-shot.
const PLAYER_MS: u64 = 3_600_000;

impl Compiler<'_, '_> {
    /// The time the ops emitted so far take, ms.
    pub(super) fn sound_clock(&mut self) -> u64 {
        // Ops inserted behind the scanned ones (a movie before its
        // transition's end) are skipped: they don't wait.
        for op in self.ops.get(self.clock_scanned..).unwrap_or_default() {
            self.clock_ms += match op {
                // (A move the script waits for: the NSS compiler's are all waited.)
                Op::Wait { ms } | Op::Pause { ms } | Op::SpriteMove { ms, .. } => u64::from(*ms),
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

impl<'a> Compiler<'a, '_> {
    /// `SetFrequency`: a sound effect's speed and pitch (per mille) over
    /// `ms` along `ease` (its tempo, as `Move`'s: `FUN_00478570`); one
    /// that hasn't started yet starts with it.
    pub(super) fn set_frequency(&mut self, target: &Value, ms: i64, rate: i64, ease: Option<(u8, u32)>) {
        let (ms, rate) = (ms.max(0) as u32, rate.clamp(100, 4000) as u32);
        for t in self.matching(target) {
            let Some(Object { kind: Kind::Sound(s), .. }) = self.objects.get_mut(&t) else { continue };
            // (Music keeps 1000: only a change of it is a gap; `MusicStart` sets 1000.)
            if s.bgm {
                if rate != s.freq {
                    self.note("SetFrequency on music");
                }
            } else if s.starting {
                s.rate = Some((rate, ms, ease));
            } else if let Some(file) = s.playing.clone().filter(|_| s.freq != rate) {
                s.freq = rate;
                self.sound_ease(ease, ms);
                self.ops.push(Op::SoundRate { file, rate, ms });
            }
        }
    }

    /// Before a sound ramp over `ms`: its curve, if eased.
    pub(super) fn sound_ease(&mut self, ease: Option<(u8, u32)>, ms: u32) {
        if let Some((ease, exponent)) = ease.filter(|_| ms > 0) {
            self.ops.push(Op::SoundEase { ease, exponent });
        }
    }

    /// `SetVolume`: over `ms` along `ease` (its tempo, as `SetFrequency`'s).
    pub(super) fn set_volume(&mut self, target: &Value, ms: i64, volume: i64, ease: Option<(u8, u32)>) {
        let ms = ms.max(0) as u32;
        let now = self.sound_clock();
        for t in self.matching(target) {
            let Some(Object { kind: Kind::Sound(s), .. }) = self.objects.get_mut(&t) else { continue };
            if s.starting {
                s.volume = volume;
                s.fade_ms = ms;
            } else if s.playing.is_some() && volume == 0 {
                // Reaching 0 stops it (`WaitPlay` waits for the fade).
                let file = s.playing.take().unwrap();
                s.stops_at = Some(now + u64::from(ms));
                let bgm = s.bgm;
                if bgm && ease.is_some() && ms > 0 {
                    self.note("SetVolume tempo of a music fade-out drawn straight");
                } else {
                    self.sound_ease(ease, ms);
                }
                self.ops.push(match (bgm, ms) {
                    (true, _) => Op::BgmStop { fade_out_ms: ms },
                    (false, 0) => Op::SeStop { file: Some(file) },
                    (false, ms) => Op::SeFadeOut { file: Some(file), ms },
                });
            } else if let Some(file) = &s.playing {
                s.volume = volume;
                let file = (!s.bgm).then(|| file.clone());
                self.sound_ease(ease, ms);
                self.ops.push(Op::SoundVolume { file, percent: percent(volume), ms });
            } else {
                s.volume = volume;
            }
        }
    }

    /// `SetPan(target, ms, pan, tempo)`: a sound playing pans over `ms`;
    /// one starting (or stopped) starts at it. The music isn't panned.
    pub(super) fn set_pan(&mut self, target: &Value, ms: i64, pan: i64) {
        let (ms, pan) = (ms.max(0) as u32, pan.clamp(-1000, 1000));
        for t in self.matching(target) {
            let Some(Object { kind: Kind::Sound(s), .. }) = self.objects.get_mut(&t) else { continue };
            if std::mem::replace(&mut s.pan, pan) == pan {
                continue;
            }
            if s.bgm {
                if pan != 0 {
                    self.note("SetPan on the music");
                }
            } else if let (false, Some(file)) = (s.starting, &s.playing) {
                let file = file.clone();
                self.ops.push(Op::SoundPan { file, pan: pan as i32, ms });
            }
        }
    }

    /// Starts the sounds whose `Play` came since the last flush.
    pub(super) fn flush_sounds(&mut self) {
        let starting: Vec<String> =
            self.objects.iter().filter(|(_, o)| matches!(&o.kind, Kind::Sound(s) if s.starting)).map(|(n, _)| n.clone()).collect();
        let mut marked = false;
        for name in starting {
            if std::mem::take(&mut marked) {
                self.running.pop();
                self.mark();
            }
            if let Some(at) = self.played_at.remove(&name) {
                self.running.push(at);
                self.mark();
                marked = true;
            }
            let Kind::Sound(s) = &mut self.objects.get_mut(&name).unwrap().kind else { continue };
            s.starting = false;
            if s.volume <= 0 && s.fade_ms == 0 {
                continue;
            }
            // A music process's sound effect after its first wait: only
            // music waits for its time (`Op::BgmLater`).
            if !s.bgm && self.process_clock.is_some_and(|t| t > 0) {
                self.music_refused = true;
                continue;
            }
            let (path, bgm, looped, fade_ms, volume, loop_ms, pan) = (s.path.clone(), s.bgm, s.looped, s.fade_ms, s.volume, s.loop_ms, s.pan);
            let file = match loop_ms {
                Some((start, end)) if looped => self.assets.looped_sound(&path, start, end),
                _ => self.assets.sound(&path),
            };
            let Some(file) = file else {
                if std::env::var_os("NSS_TRACE").is_some() {
                    eprintln!("missing sound {path}");
                }
                self.note("sound missing from the archives");
                continue;
            };
            if bgm {
                // One BGM at a time.
                for o in self.objects.values_mut() {
                    if let Kind::Sound(other) = &mut o.kind
                        && other.bgm
                    {
                        other.playing = None;
                    }
                }
                if let Some(after_ms) = self.process_clock {
                    // Started by a music process, on its clock.
                    self.ops.push(Op::BgmLater { file: file.clone(), after_ms, looped });
                } else {
                    if !looped {
                        self.note("BGM without loop played looping");
                    }
                    if percent(volume) < 100 {
                        self.ops.push(Op::SoundVolume { file: None, percent: percent(volume), ms: 0 });
                    }
                    self.ops.push(Op::BgmPlay { file: file.clone(), fade_in_ms: fade_ms });
                }
            } else {
                if fade_ms > 0 {
                    self.ops.push(Op::SeFadeIn { ms: fade_ms });
                }
                if pan != 0 {
                    self.ops.push(Op::SoundPan { file: file.clone(), pan: pan as i32, ms: 0 });
                }
                if percent(volume) < 100 {
                    self.ops.push(Op::SoundVolume { file: Some(file.clone()), percent: percent(volume), ms: 0 });
                }
                self.ops.push(Op::SePlay { file: file.clone(), looped });
                if let Some(Object { kind: Kind::Sound(s), .. }) = self.objects.get_mut(&name) {
                    s.freq = 1000;
                    if let Some((rate, ms, ease)) = s.rate.take().filter(|&(rate, _, _)| rate != 1000) {
                        s.freq = rate;
                        self.sound_ease(ease, ms);
                        self.ops.push(Op::SoundRate { file: file.clone(), rate, ms });
                    }
                }
            }
            let len_ms = if looped { None } else { self.assets.sound_ms(&path) };
            let started_at = self.sound_clock();
            if let Some(Object { kind: Kind::Sound(s), .. }) = self.objects.get_mut(&name) {
                (s.playing, s.len_ms, s.started_at, s.stops_at) = (Some(file), len_ms, started_at, None);
            }
        }
        if marked {
            self.running.pop();
            self.mark();
        }
    }
}

/// An NSS volume (0-1000) as a percent of the player's setting: the exe
/// sends 3322·log10(volume/1000) hundredths of a dB, an amplitude of
/// (volume/1000)^1.661 (`research/nitroplus/builtins.md`, `SetVolume`).
// src: exe sound_set_volume 00475f50 (3322.0 at 0x514fe8 × log10, at least -10000.0 at 0x514fe0)
pub(super) fn percent(volume: i64) -> u32 {
    (100.0 * (volume.clamp(0, 1000) as f64 / 1000.0).powf(1.661)).round() as u32
}

#[cfg(test)]
mod tests {
    use super::percent;

    // (volume, DirectSound hundredths of a dB) from sound_set_volume 00475f50
    // run in the exe: research/nitroplus/volume_exe.py <Muramasa.exe>
    const EXE_VOLUMES: [(i64, i32); 16] = [
        (1, -9966), (10, -6644), (50, -4322), (100, -3322), (200, -2321), (250, -2000), (300, -1737), (400, -1321),
        (500, -1000), (600, -736), (700, -514), (750, -415), (800, -321), (900, -152), (950, -74), (1000, 0),
    ];

    #[test]
    fn volumes_are_the_exes() {
        for (volume, db) in EXE_VOLUMES {
            let amplitude = (100.0 * 10f64.powf(db as f64 / 2000.0)).round() as u32;
            assert_eq!(percent(volume), amplitude, "volume {volume}");
        }
    }
}
