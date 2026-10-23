//! The strobe process (`strobe`, dja050: white added on between the bars,
//! flashing until deleted). The shakes (`ShakeVLoop`) are loop processes
//! ([`super::super::loops`]).

use super::super::{Compiler, FlashObject, Kind, Value};

impl Compiler<'_, '_> {
    /// `CreateProcess(name, priority, x, y, "strobe")`.
    pub(in super::super) fn strobe_process(&mut self, args: &[Value]) -> bool {
        let [name, priority, _, _, function] = args else { return false };
        function.string() == "strobe" && self.strobe(self.object_name(name), priority.int())
    }

    /// `strobe`: `ストロボ白`, 800×400 at (0, 70), priority 10160, added on,
    /// at 300 for a `Draw`, faded out over 80 ms, a `Draw` at 0, until the
    /// process is deleted (the flash goes with it here). A `Draw` asks for a
    /// render pass and waits for it at most 4 ms.
    fn strobe(&mut self, name: String, _priority: i64) -> bool {
        // src: dja050_03 strobe (CreateColor 0,70 800×400 at 10160; Fade 300, Draw, 80, Draw); builtin_Draw 0040ba10: WaitForMultipleObjects(2, render events, 4 ms) at 0x40ba8c
        let image = self.assets.color(0xFFFFFF, 800, 400);
        let a = 300 * 256 / 1000;
        let keys = vec![(0, a), (4, a), (84, 0), (88, 0)];
        let o = self.create(name, Kind::Flash(FlashObject { image, keys, times: 0, started: false }), 10160, 800, 400);
        o.y = 70;
        true
    }
}
