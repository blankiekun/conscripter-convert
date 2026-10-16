//! The ending's credits (`SetRoll`, dja056_09): its process
//! `ProcessRoll` stops the insert song looping, then moves
//! `cg/sys/good/EDROLL.jpg` (made at y 300) up by its height + 65 − 300
//! over the song's time left + the closing song's length + 6 s; the
//! closing song plays when the insert song ends. `SetRoll` waits for the
//! process the first time through (`$待機`), then for a click. Compiled
//! as one [`Op::MusicRoll`] at the process's `Request(Start)`, whose
//! click is `SetRoll`'s `WaitKey`.

use conv_core::ir::Op;

use super::super::{Compiler, Kind, Value};

/// The process `SetRoll` makes.
const PROCESS: &str = "エンドロールプロセス";
/// The picture `ProcessRoll` makes.
const PICTURE: &str = "エンドロール背景";
const IMAGE: &str = "cg/sys/good/EDROLL.jpg";
/// The insert song (`SetRoll`'s `$ナット名`, both times; the parser reads
/// its `@dj_INS01` without quotes as a relative number).
const SONG: &str = "@dj_INS01";
/// The closing song, after the insert song.
const NEXT: &str = "@dj_INS11";
const AFTER_MS: u32 = 6000;

impl Compiler<'_, '_> {
    /// `CreateProcess(…, "ProcessRoll")`, `Request(…, Start)` of it, its
    /// `WaitAction`, the `WaitKey` while it runs and its `Request(…,
    /// Stop)`.
    pub(in super::super) fn roll(&mut self, name: &str, args: &[Value]) -> bool {
        let process = || args.first().is_some_and(|a| a.string().trim_start_matches('@') == PROCESS);
        match name {
            "CreateProcess" if args.get(4).is_some_and(|f| f.string() == "ProcessRoll") => {
                self.create(PROCESS.to_string(), Kind::Other, 0, 0, 0);
                true
            }
            "Request" if process() && args.get(1).is_some_and(|a| a.string().eq_ignore_ascii_case("start")) => {
                self.roll_start();
                true
            }
            "Request" if process() => {
                self.delete(PROCESS);
                true
            }
            "WaitAction" if process() => true,
            "WaitKey" => self.objects.contains_key(PROCESS),
            _ => false,
        }
    }

    fn roll_start(&mut self) {
        let sound_ms = |c: &mut Self, v: &Value| {
            let path = c.matching(v).iter().find_map(|t| match &c.objects[t].kind {
                Kind::Sound(s) => Some(s.path.clone()),
                _ => None,
            })?;
            Some((c.assets.sound_ms(&path)?, c.assets.sound(&path)?))
        };
        let skippable = !self.vars.get("$待機").is_some_and(|v| v.truthy());
        let (Some((song_ms, song)), Some((next_ms, next))) = (sound_ms(self, &Value::Str(SONG.into())), sound_ms(self, &Value::Str(NEXT.into()))) else {
            self.note("ProcessRoll without its songs");
            return;
        };
        self.builtin("CreateTexture", &[Value::Str(PICTURE.into()), Value::Int(100), Value::Int(0), Value::Int(300), Value::Str(IMAGE.into())]);
        self.sync(0);
        let Some(o) = self.objects.get_mut(PICTURE) else { return };
        let dy = 235 - o.h;
        o.y += dy;
        let Some(sh) = self.shown.get_mut(PICTURE) else { return };
        sh.y += dy;
        let layer = sh.layer;
        self.ops.push(Op::MusicRoll { layer, dy: dy as i32, song, song_ms, next: Some(next), after_ms: next_ms + AFTER_MS, skippable });
    }
}
