//! dja050's 400-vs-3 battle (`function_count.nss`).
//!
//! `VS400(movie, sound, count, keep)`: the movie plays while the player
//! holds the button down (a full-screen choice whose `MouseClick` resumes
//! it and whose release pauses it), the "hold" prompt (`PushShow`: ten
//! pictures at 690,20, 100 ms each) going round over it. Its gunfire
//! `sound/se/<sound>` plays with it: `keep` 1 pauses and resumes it with
//! the movie (`SetStream` to the movie's time); 0 starts it again, looped,
//! at each press, and fades it out over 500 ms at each release and at the
//! end.

use conv_core::ir::{HoldPrompt, HoldSound, Op};

use super::super::{Compiler, Value};

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
    /// `VS400(…)`: an [`Op::HoldMovie`].
    pub(in super::super) fn vs400(&mut self, args: &[Value]) -> bool {
        let [movie, sound, _count, keep] = args else { return false };
        let Some(file) = self.assets.movie_file(&format!("movie/{}", movie.string()), false) else {
            self.note("VS400 of a missing movie");
            return true;
        };
        let prompt = self.assets.strip(&PROMPT).map(|(image, _, _)| HoldPrompt { image, x: 690, y: 20, cells: PROMPT.len() as u32, frame_ms: 100 });
        let fade_ms = if keep.int() == 0 { 500 } else { 0 };
        let sound = self.assets.sound(&format!("sound/se/{}", sound.string())).map(|file| HoldSound { file, fade_ms });
        self.sync(0);
        self.ops.push(Op::HoldMovie { file, prompt, sound });
        true
    }
}
