//! Processes that loop until deleted: the shakes (`ShakeVLoop`,
//! `ShakeVLoop01`-`05`, 9 uses: a background jolting up and down, 1 to 4
//! px, as a carriage or a horde rides) and the strobe (`strobe`, dja050:
//! white added on between the bars, flashing).
//!
//! A shake process first makes its pictures (as its children: moving or
//! deleting the process moves or deletes them), then loops `Shake(picture,
//! ms, 0, y, 0, y_end, …)` and `Wait`s; the engine loops the picture's
//! motion (`spmotion`) through the same times, the picture at ±y every
//! [`STEP_MS`], y going to y_end. A move of the picture over time
//! (`spmotion2`) replaces it.

use std::collections::{HashMap, HashSet};

use conv_core::ir::Op;

use super::super::{Compiler, Expr, FlashObject, Kind, StmtKind, Value};

/// How often a shaken picture turns.
const STEP_MS: u32 = 100;
/// `spmotion` takes 32 keys.
const MAX_KEYS: u32 = 32;

impl Compiler<'_, '_> {
    /// `CreateProcess(name, priority, x, y, function)` of a looping one.
    pub(in super::super) fn loop_process(&mut self, args: &[Value]) -> bool {
        let [name, priority, _, _, function] = args else { return false };
        match function.string().as_str() {
            "strobe" => self.strobe(self.object_name(name), priority.int()),
            f if f.starts_with("ShakeVLoop") => self.shake_loop(self.object_name(name), priority.int(), f),
            _ => false,
        }
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

    fn shake_loop(&mut self, name: String, priority: i64, function: &str) -> bool {
        let Some((file, f)) = self.function(function) else { return false };
        let Some(at) = f.body.iter().position(|s| matches!(s.kind, StmtKind::While { .. })) else { return false };
        let StmtKind::While { body, .. } = &f.body[at].kind else { return false };
        // Not a Kind::Process: its Request(Start) mustn't run the loop.
        self.create(name.clone(), Kind::Other, priority, 0, 0);
        // The pictures it makes: its children.
        let before: HashSet<String> = self.objects.keys().cloned().collect();
        let saved = std::mem::replace(&mut self.file, file);
        self.frames.push(HashMap::new());
        self.in_function += 1;
        self.exec_block(&f.body[..at]);
        self.in_function -= 1;
        self.frames.pop();
        self.file = saved;
        let made: Vec<String> = self.objects.keys().filter(|k| !before.contains(*k) && !k.contains('/')).cloned().collect();
        let renamed = |k: &str| match made.iter().find(|m| k == m.as_str() || k.starts_with(&format!("{m}/"))) {
            Some(_) => format!("{name}/{k}"),
            None => k.to_string(),
        };
        let moved: Vec<String> = self.objects.keys().filter(|k| renamed(k) != **k).cloned().collect();
        for k in moved {
            let o = self.objects.remove(&k).unwrap();
            self.objects.insert(renamed(&k), o);
        }
        for v in self.aliases.values_mut() {
            *v = renamed(v);
        }
        // The loop: (ms, y from, y to) a segment, 0 px for a wait.
        let mut target = None;
        let mut parts = Vec::new();
        for s in body {
            let StmtKind::Expr(Expr::Call { name: call, args }) = &s.kind else { continue };
            let v: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
            let int = |i: usize| v.get(i).map_or(0, |a| a.int());
            match call.as_str() {
                "Shake" => {
                    target.get_or_insert_with(|| renamed(&self.object_name(&v[0])));
                    parts.push((int(1).max(0) as u32, int(3).abs() as i32, int(5).abs() as i32));
                }
                "Wait" => parts.push((int(0).max(0) as u32, 0, 0)),
                _ => {}
            }
        }
        let Some(target) = target else { return false };
        let keys = shake_keys(&parts);
        if let Some(o) = self.objects.get_mut(&target) {
            o.shake = Some((name, keys.clone()));
        }
        // Already shown: it starts now.
        if let Some(s) = self.shown.get(&target) {
            self.ops.push(Op::SpriteMotion { layer: s.layer, keys, looped: true });
        }
        true
    }

    /// `Delete` of a shake process whose picture isn't its child: the
    /// picture stops. (The deletion itself goes on as usual.)
    pub(in super::super) fn stop_shakes(&mut self, args: &[Value]) {
        let Some(arg) = args.first() else { return };
        let owners = self.matching(arg);
        let stopped: Vec<String> = self.objects.iter().filter(|(_, o)| o.shake.as_ref().is_some_and(|(p, _)| owners.contains(p))).map(|(k, _)| k.clone()).collect();
        for k in stopped {
            self.objects.get_mut(&k).unwrap().shake = None;
            if let Some(s) = self.shown.get(&k) {
                self.ops.push(Op::SpriteMotion { layer: s.layer, keys: Vec::new(), looped: false });
            }
        }
    }
}

/// `spmotion` keys for shake `parts` (ms, y from, y to; 0 px a wait),
/// looped: up and down every step (longer when the keys would run out).
fn shake_keys(parts: &[(u32, i32, i32)]) -> Vec<(u32, i32, i32)> {
    let waits = parts.iter().filter(|p| p.1 == 0 && p.2 == 0).count() as u32;
    let shaking: u32 = parts.iter().filter(|p| p.1 != 0 || p.2 != 0).map(|p| p.0).sum();
    let step = STEP_MS.max(shaking.div_ceil(MAX_KEYS - 2 - waits).div_ceil(10) * 10);
    let (mut keys, mut t, mut sign) = (vec![(0, 0, 0)], 0, 1);
    for &(ms, from, to) in parts {
        if from == 0 && to == 0 {
            t += ms;
            keys.push((t, 0, 0));
            continue;
        }
        let n = (ms / step).max(1);
        for i in 1..=n {
            let y = from + (to - from) * i as i32 / n as i32;
            keys.push((t + ms * i / n, 0, sign * y));
            sign = -sign;
        }
        t += ms;
    }
    if keys.last().is_some_and(|k| k.2 != 0) {
        keys.push((t + step, 0, 0));
    }
    keys.dedup_by(|b, a| a.2 == 0 && b.2 == 0 && b.0 == a.0);
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shake_loop_goes_up_and_down_in_at_most_32_keys() {
        // ShakeVLoop01 (dja016): shakes, a 700 ms wait, shakes, 1500 ms.
        let parts = [(1000, 3, 1), (500, 2, 0), (700, 0, 0), (1000, 3, 1), (500, 2, 0), (1500, 0, 0)];
        let keys = shake_keys(&parts);
        assert!(keys.len() <= MAX_KEYS as usize, "{keys:?}");
        assert_eq!(keys[0], (0, 0, 0));
        assert_eq!(keys.last().unwrap().0, 5200);
        assert!(keys.iter().any(|k| k.2 > 0) && keys.iter().any(|k| k.2 < 0));
        assert!(keys.windows(2).all(|w| w[0].0 < w[1].0), "{keys:?}");
        // ShakeVLoop: 3 s of shaking, every 100 ms.
        let keys = shake_keys(&[(1000, 3, 1), (500, 2, 0), (1000, 3, 1), (500, 2, 0)]);
        assert_eq!(keys[1], (100, 0, 3));
        assert!(keys.len() <= MAX_KEYS as usize, "{keys:?}");
    }
}
