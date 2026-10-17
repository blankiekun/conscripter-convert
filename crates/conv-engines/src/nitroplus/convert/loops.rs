//! `CreateProcess` of a function that ends in an endless loop of timed
//! actions (`while(1) { Fade(…); Move(…); Wait(…); }`): the engine loops
//! the pictures' motion (`spmotion`) through the same times while the
//! script goes on. 装甲悪鬼村正's `FlyMoving` (a picture bobbing along a
//! Bézier path), `TurboBlur`, `fire01`-`03` (firelight flickering),
//! `RotetoLoop1`; 続・殺戮のジャンゴ's `ShakeVLoop` (a background jolting up
//! and down as a carriage rides).
//!
//! When the process starts (`Request(…, Start)`), what comes before the
//! loop runs as an action process's does (its waits hold only the
//! process; with [`super::Game::relative_children`], the pictures it makes
//! are placed from the process, which the script may have moved before
//! starting it). Then the loop's body
//! is played on paper, twice: the second pass, which starts where the
//! first ended, is the one looped (`RotetoLoop`'s first turn from where
//! the picture was is drawn at once). Each picture the body changes gets
//! one motion: its keys at every time an action starts or ends (and along
//! a Bézier path), as offsets from where it stood when the pass began, its
//! zoom and angle from those it had then, and its opacity from the
//! highest the pass reaches, at which it is placed. Deleting the process
//! stops them.

use std::collections::{HashMap, HashSet};

use conv_core::ir::{MotionKey, Op};

use super::{Compiler, Expr, Function, Stmt, StmtKind, Value, shake::shake_keys};

/// What a looped body may call.
const LOOP_CALLS: &[&str] = &["Fade", "Move", "Zoom", "Rotate", "BezierMove", "Wait"];
/// A shake loop's body (ShakeVLoop: `Shake` and `Wait` only).
const SHAKE_CALLS: &[&str] = &["Shake", "Wait"];
/// `spmotion` takes 64 keys (engine #44).
pub(super) const MAX_KEYS: usize = 64;

/// The statements before the loop, its condition and its body, when
/// `stmts` end in `while(…) { … }` of the calls in `calls` (and labels).
fn split<'a>(stmts: &'a [Stmt], calls: &[&str]) -> Option<(&'a [Stmt], &'a Expr, &'a [Stmt])> {
    let at = stmts.iter().rposition(|s| !matches!(s.kind, StmtKind::Label(_)))?;
    let StmtKind::While { cond, body } = &stmts[at].kind else { return None };
    let only = body.iter().all(|s| match &s.kind {
        StmtKind::Label(_) => true,
        StmtKind::Expr(Expr::Call { name, .. }) => calls.contains(&name.as_str()),
        _ => false,
    });
    (only && body.iter().any(|s| matches!(s.kind, StmtKind::Expr(_)))).then_some((&stmts[..at], cond, &body[..]))
}

/// The statements before an `if` that `stmts` end in, its condition and
/// its branches (`CP_SpeedAuto`: no wobble while the meter is at 0 or
/// 1000; `ComicProcess`: one effect per sign).
#[allow(clippy::type_complexity)]
fn split_if(stmts: &[Stmt]) -> Option<(&[Stmt], &Expr, &[Stmt], &[Stmt])> {
    let at = stmts.iter().rposition(|s| !matches!(s.kind, StmtKind::Label(_)))?;
    let StmtKind::If { cond, then, otherwise } = &stmts[at].kind else { return None };
    Some((&stmts[..at], cond, then, otherwise.as_deref().unwrap_or_default()))
}

/// What a branch that doesn't loop may do: run actions at the process's
/// start ([`Compiler::start_process`]'s action processes) and set names.
fn one_shot(stmts: &[Stmt]) -> bool {
    stmts.iter().all(|s| match &s.kind {
        StmtKind::Label(_) | StmtKind::Assign { .. } => true,
        StmtKind::Expr(Expr::Call { name, .. }) => ONE_SHOT_CALLS.contains(&name.as_str()),
        _ => false,
    })
}

/// What a branch that doesn't loop may call.
const ONE_SHOT_CALLS: &[&str] = &["Fade", "Move", "Zoom", "Rotate", "Shake"];

/// Whether `stmts` end in a loop this module converts, or in an `if`
/// whose branches do so or run once, at least one looping.
fn ends_in_loop(stmts: &[Stmt]) -> bool {
    fn looped(stmts: &[Stmt]) -> bool {
        split(stmts, LOOP_CALLS).is_some() || split(stmts, SHAKE_CALLS).is_some()
    }
    fn fits(stmts: &[Stmt]) -> bool {
        looped(stmts) || one_shot(stmts) || split_if(stmts).is_some_and(|(_, _, t, o)| fits(t) && fits(o))
    }
    fn loops(stmts: &[Stmt]) -> bool {
        looped(stmts) || split_if(stmts).is_some_and(|(_, _, t, o)| loops(t) || loops(o))
    }
    fits(stmts) && loops(stmts)
}

/// Whether `f` is a loop process this module converts.
pub(super) fn is_loop(f: &Function) -> bool {
    ends_in_loop(&f.body)
}

/// One property of a picture over a pass: its value at the start and the
/// actions on it (start ms, ms, to), in order.
#[derive(Debug, Clone, Default)]
pub(super) struct Track {
    start: f64,
    actions: Vec<(u32, u32, f64)>,
}

impl Track {
    fn at(&self, t: u32) -> f64 {
        let mut v = self.start;
        for &(s, ms, to) in &self.actions {
            if t >= s + ms {
                v = to;
            } else if t > s {
                v += (to - v) * f64::from(t - s) / f64::from(ms);
            }
        }
        v
    }

    fn end(&self) -> f64 {
        self.actions.last().map_or(self.start, |a| a.2)
    }

    /// The value now (at `t`), for an action starting then.
    pub(super) fn push(&mut self, t: u32, ms: u32, to: f64) {
        self.actions.push((t, ms.max(1), to));
    }
}

/// A picture over a pass: x, y, zoom x, zoom y (per mille), angle
/// (counterclockwise), opacity (0-1000); and the extra key times a Bézier
/// path adds.
#[derive(Debug, Clone, Default)]
pub(super) struct Pass {
    pub(super) props: [Track; 6],
    times: Vec<u32>,
}

impl Pass {
    pub(super) fn starting(v: [f64; 6]) -> Self {
        Pass { props: v.map(|start| Track { start, actions: Vec::new() }), times: Vec::new() }
    }

    fn now(&self, i: usize) -> f64 {
        self.props[i].end()
    }

    pub(super) fn ends(&self) -> [f64; 6] {
        [0, 1, 2, 3, 4, 5].map(|i| self.props[i].end())
    }

    pub(super) fn values(&self, t: u32) -> [f64; 6] {
        [0, 1, 2, 3, 4, 5].map(|i| self.props[i].at(t))
    }

    /// The times keys go at over `len` ms: every time an action starts or
    /// ends.
    fn key_times(&self, len: u32) -> Vec<u32> {
        let mut times: Vec<u32> = self.props.iter().flat_map(|p| p.actions.iter().flat_map(|&(s, ms, _)| [s, s + ms])).chain(self.times.iter().copied()).chain([0, len]).filter(|&t| t <= len).collect();
        times.sort_unstable();
        times.dedup();
        times
    }

    /// Keys over `len` ms, as offsets from the values at the start and
    /// opacity from the highest at a key; `None` when never seen.
    fn keys(&self, len: u32) -> Option<Vec<MotionKey>> {
        let top = self.key_times(len).iter().map(|&t| self.values(t)[5]).fold(0.0, f64::max);
        (top > 0.0).then(|| self.keyed(len, self.values(0), top))
    }

    /// Keys over `len` ms as offsets from the values at the end and
    /// opacity from `top`, for a motion that ends where the picture is
    /// placed ([`super::one_shot`]).
    pub(super) fn keys_to_end(&self, len: u32, top: f64) -> Vec<MotionKey> {
        self.keyed(len, self.values(len), top)
    }

    /// When the last action ends.
    pub(super) fn end_time(&self) -> u32 {
        self.props.iter().flat_map(|p| p.actions.iter().map(|&(s, ms, _)| s + ms)).chain(self.times.iter().copied()).max().unwrap_or(0)
    }

    fn keyed(&self, len: u32, base: [f64; 6], top: f64) -> Vec<MotionKey> {
        let times = self.key_times(len);
        let key = |t: u32| {
            let v = self.values(t);
            let zoom = (v[2] * 100.0 / base[2].max(1.0)).round().max(1.0) as u32;
            MotionKey {
                ms: t,
                x: (v[0] - base[0]).round() as i32,
                y: (v[1] - base[1]).round() as i32,
                zoom,
                angle: (v[4] - base[4]).round() as i32,
                opacity: (v[5] * 256.0 / top).round() as u32,
                zoom_y: zoom,
            }
        };
        let mut keys: Vec<MotionKey> = times.into_iter().map(key).collect();
        // A key between two like it adds nothing.
        let same = |a: &MotionKey, b: &MotionKey| (a.x, a.y, a.zoom, a.zoom_y, a.angle, a.opacity) == (b.x, b.y, b.zoom, b.zoom_y, b.angle, b.opacity);
        let mut i = 1;
        while i + 1 < keys.len() {
            if same(&keys[i - 1], &keys[i]) && same(&keys[i], &keys[i + 1]) {
                keys.remove(i);
            } else {
                i += 1;
            }
        }
        keys
    }

    pub(super) fn top(&self, len: u32) -> f64 {
        (0..=len).step_by(10).map(|t| self.values(t)[5]).fold(self.values(len)[5], f64::max)
    }
}

impl<'a> Compiler<'a, '_> {
    /// Starts loop process `name` (its function `f`, in `file`).
    pub(super) fn start_loop(&mut self, name: &str, file: &'a str, f: &'a Function) {
        let saved = std::mem::replace(&mut self.file, file);
        self.frames.push(HashMap::new());
        self.in_function += 1;
        let before: HashSet<String> = self.objects.keys().cloned().collect();
        self.process_lead = 0;
        self.run_loop(name, &f.name, &f.body, &before);
        self.in_function -= 1;
        self.frames.pop();
        self.file = saved;
    }

    /// Runs `stmts`, which [`ends_in_loop`]: the statements before the
    /// loop, then the loop; an `if` is decided as the process starts.
    fn run_loop(&mut self, name: &str, function: &str, stmts: &'a [Stmt], before: &HashSet<String>) {
        // A loop that runs while a flag is set: until the script clears it,
        // which is drawn as the process going on until it is deleted (the
        // scripts fade its pictures out then).
        if let Some((prelude, cond, body)) = split(stmts, SHAKE_CALLS) {
            self.exec_block(prelude);
            let made = self.adopt(name, before);
            if self.eval(cond).truthy() {
                self.shake_loop(name, body, &made);
            }
        } else if let Some((prelude, cond, body)) = split(stmts, LOOP_CALLS) {
            self.action_process = Some(name.to_string());
            self.exec_block(prelude);
            self.action_process = None;
            let made = self.adopt(name, before);
            if self.eval(cond).truthy() {
                self.action_loop(name, function, body, &made);
            }
        } else if let Some((prelude, cond, then, otherwise)) = split_if(stmts) {
            self.action_process = Some(name.to_string());
            self.exec_block(prelude);
            self.action_process = None;
            let branch = if self.eval(cond).truthy() { then } else { otherwise };
            if ends_in_loop(branch) {
                self.run_loop(name, function, branch, before);
            } else {
                self.action_process = Some(name.to_string());
                self.exec_block(branch);
                self.action_process = None;
            }
        }
    }

    /// With [`super::Game::family`], the pictures a process made are its
    /// children: moving or deleting the process moves or deletes them.
    /// The names the loop's body still knows them by, and their new ones.
    fn adopt(&mut self, name: &str, before: &HashSet<String>) -> Vec<(String, String)> {
        if !self.game.family {
            return Vec::new();
        }
        let made: Vec<String> = self.objects.keys().filter(|k| !before.contains(*k) && !k.contains('/')).cloned().collect();
        let renamed = |k: &str| match made.iter().find(|m| k == m.as_str() || k.starts_with(&format!("{m}/"))) {
            Some(_) => format!("{name}/{k}"),
            None => k.to_string(),
        };
        let moved: Vec<(String, String)> = self.objects.keys().map(|k| (k.clone(), renamed(k))).filter(|(k, r)| k != r).collect();
        for (k, r) in &moved {
            let o = self.objects.remove(k).unwrap();
            self.objects.insert(r.clone(), o);
        }
        for v in self.aliases.values_mut() {
            *v = renamed(v);
        }
        moved
    }

    /// The objects `arg` names in a loop's body, after [`Self::adopt`].
    fn targets(&self, arg: &Value, made: &[(String, String)]) -> Vec<String> {
        let name = self.object_name(arg);
        match made.iter().find(|(k, _)| *k == name) {
            Some((_, r)) => vec![r.clone()],
            None => self.matching(arg),
        }
    }

    /// A shake loop: `Shake(picture, ms, x, y, x_end, y_end, …)` and
    /// `Wait`s ([`shake_keys`]).
    fn shake_loop(&mut self, name: &str, body: &'a [Stmt], made: &[(String, String)]) {
        // (ms, from, to) a segment, 0 px for a wait.
        let mut target = None;
        let mut parts = Vec::new();
        let calls: Vec<(&String, &Vec<Expr>)> = body.iter().filter_map(|s| if let StmtKind::Expr(Expr::Call { name, args }) = &s.kind { Some((name, args)) } else { None }).collect();
        for (i, &(call, args)) in calls.iter().enumerate() {
            let v: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
            let int = |i: usize| v.get(i).map_or(0, |a| a.int().abs() as i32);
            match call.as_str() {
                "Shake" => {
                    // One that doesn't wait, followed by another: that one
                    // takes over at once (FrameShakeProcess).
                    if matches!(v.get(8), Some(Value::Null | Value::Int(0))) && calls.get(i + 1).is_some_and(|c| c.0 == "Shake") {
                        continue;
                    }
                    if target.is_none() {
                        target = self.targets(&v[0], made).into_iter().next().or_else(|| Some(self.object_name(&v[0])));
                    }
                    parts.push((int(1) as u32, (int(2), int(3)), (int(4), int(5))));
                }
                _ => parts.push((int(0) as u32, (0, 0), (0, 0))),
            }
        }
        let Some(target) = target else { return };
        let keys = shake_keys(&parts).into_iter().map(|(ms, x, y)| MotionKey { ms, x, y, zoom: 100, angle: 0, opacity: 256, zoom_y: 100 }).collect();
        self.set_loop(name, &target, keys);
    }

    /// A loop of `Fade`, `Move`, `Zoom`, `Rotate`, `BezierMove` and `Wait`.
    fn action_loop(&mut self, name: &str, function: &str, body: &'a [Stmt], made: &[(String, String)]) {
        let (sw, sh) = self.game.screen;
        let mut first: HashMap<String, Pass> = HashMap::new();
        let len = self.play(body, &mut first, (sw, sh), made);
        if len == 0 {
            self.note(format!("loop process without time: {function}"));
            return;
        }
        // The second pass, from where the first left each picture.
        let mut second: HashMap<String, Pass> = first.iter().map(|(k, p)| (k.clone(), Pass::starting(p.ends()))).collect();
        self.play(body, &mut second, (sw, sh), made);
        if second.iter().any(|(k, p)| p.ends() != first[k].ends()) {
            self.note(format!("loop process that doesn't come back to where it started: {function}"));
        }
        let mut targets: Vec<(String, Pass)> = second.into_iter().collect();
        targets.sort_by(|a, b| a.0.cmp(&b.0));
        for (target, pass) in targets {
            let v = pass.values(0);
            let unlike = |t: u32| {
                let n = pass.values(t);
                (n[2] * v[3] - n[3] * v[2]).abs() > v[2].max(1.0) * v[3].max(1.0) / 100.0
            };
            if (0..=len).step_by(10).any(unlike) {
                self.note(format!("loop process zooming unlike in x and y: {function}"));
                continue;
            }
            let Some(keys) = pass.keys(len) else { continue };
            if keys.len() > MAX_KEYS {
                self.note(format!("loop process over {MAX_KEYS} keys: {function} ({})", keys.len()));
                continue;
            }
            let top = pass.top(len);
            if let Some(o) = self.objects.get_mut(&target) {
                (o.x, o.y) = (v[0].round() as i64, v[1].round() as i64);
                o.zoom = (v[2].round() as i64, v[3].round() as i64);
                o.turn.2 = -(v[4].round() as i64);
                o.opacity = top.round() as i64;
            }
            self.set_loop(name, &target, keys);
        }
    }

    /// Plays `body` once on paper into `passes` (a picture's pass starts
    /// where the object is, unless it is there already); its length.
    fn play(&mut self, body: &'a [Stmt], passes: &mut HashMap<String, Pass>, (sw, sh): (i64, i64), made: &[(String, String)]) -> u32 {
        let mut clock = 0u32;
        for s in body {
            let StmtKind::Expr(Expr::Call { name: call, args }) = &s.kind else { continue };
            if call == "Wait" {
                clock += self.eval(&args[0]).int().max(0) as u32;
                continue;
            }
            clock += self.paper_action(call, args, passes, clock, (sw, sh), made);
        }
        clock
    }

    /// Plays the timed action `call` (`Fade`, `Move`, `Zoom`, `Rotate`,
    /// `BezierMove`) on paper into `passes` at `clock`; the time it takes
    /// when it is waited for, else 0.
    pub(super) fn paper_action(&mut self, call: &str, args: &'a [Expr], passes: &mut HashMap<String, Pass>, clock: u32, (sw, sh): (i64, i64), made: &[(String, String)]) -> u32 {
        {
            let v: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
            let arg = |i: usize| v.get(i).cloned().unwrap_or(Value::Null);
            let ms = arg(1).int().max(0) as u32;
            // The wait argument: Fade(obj, ms, to[, tempo], wait),
            // BezierMove(obj, ms, path, tempo, wait), Move and Zoom (obj,
            // ms, x, y, tempo, wait), Rotate(obj, ms, x, y, z, tempo, wait).
            let wait = match call {
                "Fade" if v.len() == 4 => arg(3),
                "Fade" | "BezierMove" => arg(4),
                "Move" | "Zoom" => arg(5),
                _ => arg(6),
            };
            for t in self.targets(&arg(0), made) {
                let Some(o) = self.objects.get(&t) else { continue };
                let (w, h) = (o.w, o.h);
                let p = passes.entry(t).or_insert_with(|| Pass::starting([o.x as f64, o.y as f64, o.zoom.0 as f64, o.zoom.1 as f64, -o.turn.2 as f64, o.opacity as f64]));
                let now = |i: usize| p.now(i).round() as i64;
                match call {
                    "Fade" => {
                        let to = arg(2).position(now(5), 0, 0).clamp(0, 1000);
                        p.props[5].push(clock, ms, to as f64);
                    }
                    "Move" => {
                        let (x, y) = (arg(2).position(now(0), w, sw), arg(3).position(now(1), h, sh));
                        p.props[0].push(clock, ms, x as f64);
                        p.props[1].push(clock, ms, y as f64);
                    }
                    "Zoom" => {
                        let (zx, zy) = (arg(2).position(now(2), 0, 0).max(1), arg(3).position(now(3), 0, 0).max(1));
                        p.props[2].push(clock, ms, zx as f64);
                        p.props[3].push(clock, ms, zy as f64);
                    }
                    "Rotate" => {
                        // About z, clockwise in NSS.
                        let to = -arg(4).position(-now(4), 0, 0);
                        p.props[4].push(clock, ms, to as f64);
                    }
                    _ => {
                        let Some(Expr::Path(points)) = args.get(2) else { continue };
                        let at = (p.now(0), p.now(1));
                        let mut pts = Vec::new();
                        for pt in points {
                            let Expr::Point { control, x, y } = pt else { continue };
                            let (x, y) = (self.eval(x), self.eval(y));
                            pts.push((*control, x.position(at.0 as i64, w, sw) as f64, y.position(at.1 as i64, h, sh) as f64));
                        }
                        bezier(p, clock, ms, &pts);
                    }
                }
            }
            if !matches!(wait, Value::Null | Value::Int(0)) {
                return ms;
            }
            0
        }
    }

    /// Picture `target` loops `keys` while process `name` runs, after the
    /// process's waits ([`Compiler::process_lead`]).
    fn set_loop(&mut self, name: &str, target: &str, keys: Vec<MotionKey>) {
        let Some(o) = self.objects.get_mut(target) else { return };
        o.shake = Some((name.to_string(), keys.clone()));
        let Some(s) = self.shown.get(target) else { return };
        // Moving now: the next sync makes the move the lead-in.
        let moving = self.pending_by.get(target).copied().unwrap_or(0);
        if moving > 0 {
            self.lead_ins.insert(target.to_string(), self.process_lead.max(moving));
            return;
        }
        // Already shown: it starts now, or after the lead.
        self.ops.push(lead_in_loop(s.layer, (0, 0, 0), self.process_lead, keys));
    }

    /// Loop process `owner` deleted: its pictures stop.
    pub(super) fn stop_loops(&mut self, owner: &str) {
        let stopped: Vec<String> = self.objects.iter().filter(|(_, o)| o.shake.as_ref().is_some_and(|(p, _)| p == owner)).map(|(k, _)| k.clone()).collect();
        for k in stopped {
            self.objects.get_mut(&k).unwrap().shake = None;
            if let Some(s) = self.shown.get(&k) {
                self.ops.push(Op::SpriteMotion { layer: s.layer, keys: Vec::new(), looped: false });
            }
        }
    }
}

/// A loop of `keys` on `layer` after a lead-in of `lead` ms, in which the
/// picture comes from (x, y) away (its offsets from its place) back to its
/// place over `ms` (`spmotion` loop 4). Without a lead-in, or past
/// [`MAX_KEYS`], the plain loop.
pub(super) fn lead_in_loop(layer: u32, (x, y, ms): (i32, i32, u32), lead: u32, keys: Vec<MotionKey>) -> Op {
    use conv_core::ir::MotionEnd;
    let lead = lead.max(ms);
    let Some(&first) = keys.first().filter(|_| lead > 0 && keys.len() + 2 <= MAX_KEYS) else {
        return Op::SpriteMotion2 { layer, keys, end: MotionEnd::Loop };
    };
    let mut all = vec![MotionKey { ms: 0, x: first.x + x, y: first.y + y, ..first }];
    if ms > 0 && ms < lead {
        all.push(MotionKey { ms, ..first });
    }
    let from = all.len();
    all.extend(keys.into_iter().map(|k| MotionKey { ms: k.ms + lead, ..k }));
    Op::SpriteMotion2 { layer, keys: all, end: MotionEnd::LoopFrom(from) }
}

/// `BezierMove` along `pts` (anchors, and between them any number of
/// control points: `true`) from `clock` over `ms`, into `p`'s x and y: the
/// anchors and a point between each, in even time per curve.
fn bezier(p: &mut Pass, clock: u32, ms: u32, pts: &[(bool, f64, f64)]) {
    let mut curves: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, &(control, x, y)) in pts.iter().enumerate() {
        if i == 0 {
            curves.push(vec![(x, y)]);
            continue;
        }
        curves.last_mut().unwrap().push((x, y));
        if !control && i + 1 < pts.len() {
            curves.push(vec![(x, y)]);
        }
    }
    curves.retain(|c| c.len() > 1);
    if curves.is_empty() {
        return;
    }
    // As many straight pieces per curve as its degree, while they fit.
    let fit = ((MAX_KEYS - 2) / curves.len()).max(1);
    let count = curves.len() as u32;
    for (k, c) in curves.iter().enumerate() {
        let pieces = (c.len() - 1).min(fit) as u32;
        for i in 1..=pieces {
            let at = |i: u32| clock + (ms as u64 * (k as u64 * u64::from(pieces) + u64::from(i)) / (u64::from(count) * u64::from(pieces))) as u32;
            let (x, y) = casteljau(c, f64::from(i) / f64::from(pieces));
            let (t0, t1) = (at(i - 1), at(i));
            p.props[0].push(t0, t1 - t0, x);
            p.props[1].push(t0, t1 - t0, y);
            p.times.push(t1);
        }
    }
}

/// The point at `q` (0-1) of the Bézier curve through `pts`.
fn casteljau(pts: &[(f64, f64)], q: f64) -> (f64, f64) {
    let mut v = pts.to_vec();
    while v.len() > 1 {
        v = v.windows(2).map(|w| (w[0].0 + (w[1].0 - w[0].0) * q, w[0].1 + (w[1].1 - w[0].1) * q)).collect();
    }
    v[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flicker_pass_keys_its_fades_from_the_highest() {
        // fire01: 100 → 200 over 150 ms, → 150 over 200 ms.
        let mut p = Pass::starting([0.0, 0.0, 1000.0, 1000.0, 0.0, 100.0]);
        p.props[5].push(0, 150, 200.0);
        p.props[5].push(150, 200, 150.0);
        let keys = p.keys(350).unwrap();
        assert_eq!(keys.iter().map(|k| (k.ms, k.opacity)).collect::<Vec<_>>(), [(0, 128), (150, 256), (350, 192)]);
    }

    #[test]
    fn a_bezier_path_comes_back_to_its_start() {
        let mut p = Pass::starting([10.0, 20.0, 1000.0, 1000.0, 0.0, 1000.0]);
        let pt = |c, x, y| (c, x, y);
        bezier(&mut p, 0, 1000, &[pt(false, 10.0, 20.0), pt(true, 15.0, 10.0), pt(true, 0.0, 30.0), pt(false, 0.0, 15.0), pt(true, 15.0, 10.0), pt(true, 0.0, 25.0), pt(false, 10.0, 20.0)]);
        let keys = p.keys(1000).unwrap();
        // Three pieces per cubic curve.
        assert_eq!(keys.len(), 7, "{keys:?}");
        assert_eq!((keys[3].ms, keys[3].x, keys[3].y), (500, -10, -5));
        assert_eq!((keys[6].x, keys[6].y), (0, 0));
    }

    #[test]
    fn a_curve_with_four_control_points_is_one_curve() {
        // BGMoveAutoProcess: (@0,@0){@+50,@+10}{@-30,@+20}{@+30,@-20}{@-50,@-10}(@0,@0).
        let mut p = Pass::starting([0.0, 0.0, 1000.0, 1000.0, 0.0, 1000.0]);
        let pts = [(false, 0.0, 0.0), (true, 50.0, 10.0), (true, -30.0, 20.0), (true, 30.0, -20.0), (true, -50.0, -10.0), (false, 0.0, 0.0)];
        bezier(&mut p, 0, 15000, &pts);
        let keys = p.keys(15000).unwrap();
        assert_eq!(keys.len(), 6, "{keys:?}");
        assert_eq!((keys[1].ms, keys[1].x, keys[1].y), (3000, 16, 7));
        assert_eq!((keys[5].x, keys[5].y), (0, 0));
    }
}
