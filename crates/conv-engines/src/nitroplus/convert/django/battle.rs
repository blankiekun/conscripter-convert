//! dja050's 400-vs-3 battle (`function_count.nss`).
//!
//! `VS400(movie, sound, count, keep)`: the movie plays while the player
//! holds the button down (a full-screen choice whose `MouseClick` resumes
//! it and whose release pauses it), the "hold" prompt (`PushShow`: ten
//! pictures at 690,20, 100 ms each) going round over it. Its gunfire
//! `sound/se/<sound>` plays with it: `keep` 1 pauses and resumes it with
//! the movie (`SetStream` to the movie's time); 0 starts it again, looped,
//! at each press, and fades it out over 500 ms at each release and at the
//! end. The counter (under the movie) shows `count` at once.
//!
//! `GunFire(times, ms, opacity)`: muzzle flashes while the script goes on
//! (its process `Fire`): a white screen `仮色` at priority 150, added on
//! (`AddRender`), shown at `opacity` (0-1000) for a frame (`Draw`), faded
//! out over `ms`, black for a frame, `times` times.

use conv_core::ir::{HoldPrompt, HoldSound, Op};

use super::super::{Compiler, FlashObject, Kind, Value};

/// `Fire`'s colour object, its priority, and the time a `Draw` takes (a
/// frame at 60 a second).
const FLASH: &str = "仮色";
const FLASH_PRIORITY: i64 = 150;
const FRAME_MS: u32 = 17;

/// `PushInit`'s pictures, shown in turn by `PushShow`.
const PROMPT: [&str; 10] = [
    "cg/sys/push/長押しメッセージ001.png",
    "cg/sys/push/長押しメッセージ002.png",
    "cg/sys/push/長押しメッセージ003.png",
    "cg/sys/push/長押しメッセージ004.png",
    "cg/sys/push/長押しメッセージ005.png",
    "cg/sys/push/長押しメッセージ006.png",
    "cg/sys/push/長押しメッセージ007.png",
    "cg/sys/push/長押しメッセージ008.png",
    "cg/sys/push/長押しメッセージ009.png",
    "cg/sys/push/長押しメッセージ010.png",
];

impl Compiler<'_, '_> {
    /// `GunFire(…)`: a [`Kind::Flash`], started again by the next one.
    pub(in super::super) fn gun_fire(&mut self, args: &[Value]) -> bool {
        let [times, ms, opacity] = args else { return false };
        let (sw, sh) = self.game.screen;
        let image = self.assets.color(0xFFFFFF, sw as u32, sh as u32);
        let (ms, alpha) = (ms.int().max(0) as u32, (opacity.int().clamp(0, 1000) * 256 / 1000) as u32);
        let keys = vec![(0, alpha), (FRAME_MS, alpha), (FRAME_MS + ms, 0), (2 * FRAME_MS + ms, 0)];
        let flash = FlashObject { image, keys, times: times.int().max(1) as u32, started: false };
        self.create(FLASH.into(), Kind::Flash(flash), FLASH_PRIORITY, sw, sh);
        true
    }

    /// `VS400(…)`: an [`Op::HoldMovie`].
    pub(in super::super) fn vs400(&mut self, args: &[Value]) -> bool {
        let [movie, sound, count, keep] = args else { return false };
        let Some(file) = self.assets.movie_file(&format!("movie/{}", movie.string()), false) else {
            self.note("VS400 of a missing movie");
            return true;
        };
        let prompt = self.assets.strip(&PROMPT).map(|(image, _, _)| HoldPrompt { image, x: 690, y: 20, cells: PROMPT.len() as u32, frame_ms: 100 });
        let fade_ms = if keep.int() == 0 { 500 } else { 0 };
        let sound = self.assets.sound(&format!("sound/se/{}", sound.string())).map(|file| HoldSound { file, fade_ms });
        // The counter, under the movie, shows its count at once.
        self.counter_play(&[count.clone(), Value::Int(0), Value::Int(0), Value::Int(0)]);
        self.sync(0);
        self.ops.push(Op::HoldMovie { file, prompt, sound });
        true
    }
}
