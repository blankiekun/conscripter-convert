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
    /// at 300 for a frame, faded out over 80 ms, a frame at 0, until the
    /// process is deleted (the flash goes with it here).
    fn strobe(&mut self, name: String, _priority: i64) -> bool {
        let image = self.assets.color(0xFFFFFF, 800, 400);
        let a = 300 * 256 / 1000;
        let keys = vec![(0, a), (17, a), (97, 0), (114, 0)];
        let o = self.create(name, Kind::Flash(FlashObject { image, keys, times: 0, started: false }), 10160, 800, 400);
        o.y = 70;
        true
    }
}
