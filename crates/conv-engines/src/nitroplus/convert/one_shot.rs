//! `CreateProcess` of a function that runs once: timed actions, waits and
//! the control flow around them (装甲悪鬼村正's counters `MyTr_*` and
//! `MyLife_*` flipping digits one after another, `CP_LockOnMoveProcess`,
//! `CP_IHPMove` fading one gauge picture into the next).
//!
//! When the process starts, its function runs on paper: its `if`s and
//! `while`s are decided with the values known then, its assignments run,
//! and its `Fade`/`Move`/`Zoom`/`Rotate`/`BezierMove` go on a clock that
//! its `Wait`s and waited-for actions move on (they hold only the
//! process, not the script). Each picture it changes gets one motion
//! (`spmotion` loop 2) through its keys, started at the next sync: the
//! picture is placed where the process leaves it, at the highest opacity
//! it reaches, and keyed from where it was. Once the clock has passed the
//! motion's end, the picture takes its last opacity (or goes, when the
//! process deleted it).

use std::collections::HashMap;

use conv_core::ir::{MotionEnd, MotionKey, Op};

use super::loops::{MAX_KEYS, Pass};
use super::shake::shake_keys;
use super::{Compiler, Expr, Flow, Function, Kind, Stmt, StmtKind, Value};

/// What a one-shot process may call.
const CALLS: &[&str] = &["Fade", "Move", "Zoom", "Rotate", "BezierMove", "Wait", "WaitAction", "WaitFade", "WaitMove", "WaitZoom", "Request", "Delete", "SetAlias", "SetVertex", "Shake"];
/// The timed actions among them.
const ACTIONS: &[&str] = &["Fade", "Move", "Zoom", "Rotate", "BezierMove"];
/// Loop turns on paper before a process is taken as one that doesn't end.
const MAX_STEPS: u32 = 10_000;
/// The longest a process may take on paper, ms.
const MAX_MS: u32 = 600_000;

/// A picture's motion from a one-shot process ([`Compiler::one_shots`]).
#[derive(Debug, Clone)]
pub(super) struct OneShot {
    keys: Vec<MotionKey>,
    /// When it ends on [`Compiler::sound_clock`], once started.
    end: Option<u64>,
    /// The opacity it ends at.
    opacity: i64,
    /// The process deleted it (`Delete`, `Request(…, Disused)`).
    deleted: bool,
}

/// Whether `f` runs once with only what this module plays on paper (no
/// `Random`: the run decides it).
pub(super) fn is_one_shot(f: &Function) -> bool {
    fits(&f.body) && calls_action(&f.body)
}

/// Whether a branch of a loop process that doesn't loop plays on paper
/// ([`super::loops`]: `ComicProcess`'s signs that run once).
pub(super) fn runs_once(stmts: &[Stmt]) -> bool {
    fits(stmts) && calls_action(stmts) && !has_loop(stmts)
}

fn has_loop(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        StmtKind::While { .. } => true,
        StmtKind::If { then, otherwise, .. } => has_loop(then) || otherwise.as_deref().is_some_and(has_loop),
        StmtKind::Block(b) => has_loop(b),
        _ => false,
    })
}

fn fits(stmts: &[Stmt]) -> bool {
    stmts.iter().all(|s| match &s.kind {
        StmtKind::Label(_) | StmtKind::Break | StmtKind::Return => true,
        StmtKind::Assign { index, value, .. } => index.iter().chain(value).all(pure),
        StmtKind::If { cond, then, otherwise } => pure(cond) && fits(then) && otherwise.as_deref().is_none_or(fits),
        StmtKind::While { cond, body } => pure(cond) && fits(body),
        StmtKind::Block(b) => fits(b),
        StmtKind::Expr(Expr::Call { name, args }) => CALLS.contains(&name.as_str()) && args.iter().all(pure),
        _ => false,
    })
}

fn calls_action(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        StmtKind::If { then, otherwise, .. } => calls_action(then) || otherwise.as_deref().is_some_and(calls_action),
        StmtKind::While { body, .. } | StmtKind::Block(body) => calls_action(body),
        StmtKind::Expr(Expr::Call { name, .. }) => ACTIONS.contains(&name.as_str()),
        _ => false,
    })
}

fn pure(e: &Expr) -> bool {
    match e {
        Expr::Call { args, .. } => args.iter().all(pure),
        Expr::Index { base, index } => pure(base) && pure(index),
        Expr::Unary { expr, .. } => pure(expr),
        Expr::Binary { left, right, .. } => pure(left) && pure(right),
        Expr::Point { x, y, .. } => pure(x) && pure(y),
        Expr::Path(p) => p.iter().all(pure),
        _ => true,
    }
}

/// A process played on paper.
#[derive(Default)]
struct Paper {
    passes: HashMap<String, Pass>,
    clock: u32,
    steps: u32,
    /// Pictures it deleted: when, and whether only once their actions end
    /// (`Disused`).
    gone: Vec<(String, u32, bool)>,
}

impl Paper {
    fn over(&self) -> bool {
        self.steps > MAX_STEPS || self.clock > MAX_MS
    }
}

impl<'a> Compiler<'a, '_> {
    /// Plays one-shot process `name` (its function `f`, in `file`) on
    /// paper and gives its pictures their motions.
    pub(super) fn start_one_shot(&mut self, name: &str, file: &'a str, f: &'a Function) {
        let saved = std::mem::replace(&mut self.file, file);
        self.frames.push(HashMap::new());
        self.in_function += 1;
        self.play_once(name, &f.name, &f.body, 0);
        self.in_function -= 1;
        self.frames.pop();
        self.file = saved;
    }

    /// Plays `body` of process `name` (function `function`) on paper from
    /// `clock` ms into it, in the function's frame.
    pub(super) fn play_once(&mut self, name: &str, function: &str, body: &'a [Stmt], clock: u32) {
        let mut paper = Paper { clock, ..Paper::default() };
        let before = self.snapshot();
        let pending = (self.pending_ms, self.pending_mask.clone());
        let counted = (self.notes.clone(), self.metrics.clone());
        self.walk(body, &mut paper);
        if paper.over() {
            // As if it hadn't run (it waits on what the script sets later),
            // its turns not counted.
            self.restore(before);
            (self.pending_ms, self.pending_mask) = pending;
            (self.notes, self.metrics) = counted;
            self.note(format!("process that doesn't end on paper: {function}"));
            return;
        }
        let mut gone: HashMap<String, bool> = HashMap::new();
        for (t, at, disused) in std::mem::take(&mut paper.gone) {
            let Some(o) = self.objects.get(&t) else { continue };
            if !matches!(o.kind, Kind::Image { .. }) {
                // (Sounds and the rest go as they would.)
                self.delete_or_disuse(&t, disused);
                continue;
            }
            let p = paper.passes.entry(t.clone()).or_insert_with(|| Pass::starting([o.x as f64, o.y as f64, o.zoom.0 as f64, o.zoom.1 as f64, -o.turn.2 as f64, o.opacity as f64]));
            let at = if disused { at.max(p.end_time()) } else { at };
            p.props[5].push(at, 1, 0.0);
            gone.insert(t, true);
        }
        let len = paper.passes.values().map(Pass::end_time).max().unwrap_or(0).max(paper.clock);
        let mut targets: Vec<(String, Pass)> = paper.passes.into_iter().collect();
        targets.sort_by(|a, b| a.0.cmp(&b.0));
        for (t, pass) in targets {
            let end = pass.values(len);
            let top = pass.top(len);
            let deleted = gone.contains_key(&t);
            let keys = pass.keys_to_end(len, top.max(1.0));
            let Some(o) = self.objects.get_mut(&t) else { continue };
            (o.x, o.y) = (end[0].round() as i64, end[1].round() as i64);
            o.zoom = (end[2].round() as i64, end[3].round() as i64);
            o.turn.2 = -(end[4].round() as i64);
            o.opacity = end[5].round() as i64;
            if top <= 0.0 || len == 0 {
                if deleted {
                    self.delete(&t);
                }
                continue;
            }
            if keys.len() > MAX_KEYS {
                self.note(format!("one-shot process over the keys: {function}"));
                if deleted {
                    self.delete(&t);
                }
                continue;
            }
            o.opacity = top.round() as i64;
            self.one_shots.insert(t, OneShot { keys, end: None, opacity: end[5].round() as i64, deleted });
        }
        // A WaitAction on the process waits for all of it.
        self.pending_by.insert(name.to_string(), len);
    }

    /// `Shake(picture, ms, x, y, x_end, y_end, …, wait)` on paper: the
    /// picture moved to and fro about where it is ([`shake_keys`]). The
    /// ms it holds the process.
    fn paper_shake(&mut self, args: &'a [Expr], passes: &mut HashMap<String, Pass>, clock: u32) -> u32 {
        let v: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
        let int = |i: usize| v.get(i).map_or(0, |a| a.int().abs() as i32);
        let ms = int(1) as u32;
        let keys = shake_keys(&[(ms, (int(2), int(3)), (int(4), int(5)))]);
        for t in v.first().map(|a| self.matching(a)).unwrap_or_default() {
            let Some(o) = self.objects.get(&t) else { continue };
            let p = passes.entry(t).or_insert_with(|| Pass::starting([o.x as f64, o.y as f64, o.zoom.0 as f64, o.zoom.1 as f64, -o.turn.2 as f64, o.opacity as f64]));
            let base = (p.ends()[0], p.ends()[1]);
            for w in keys.windows(2) {
                let ((t0, ..), (t1, x, y)) = (w[0], w[1]);
                p.props[0].push(clock + t0, t1 - t0, base.0 + f64::from(x));
                p.props[1].push(clock + t0, t1 - t0, base.1 + f64::from(y));
            }
        }
        if matches!(v.get(8), None | Some(Value::Null | Value::Int(0))) { 0 } else { ms }
    }

    fn delete_or_disuse(&mut self, name: &str, disused: bool) {
        if disused {
            self.request(name, "disused");
        } else {
            self.delete(name);
        }
    }

    /// Runs `stmts` on paper (see the module).
    fn walk(&mut self, stmts: &'a [Stmt], paper: &mut Paper) -> Flow {
        let screen = self.game.screen;
        for s in stmts {
            if paper.over() {
                return Flow::Return;
            }
            match &s.kind {
                StmtKind::Label(_) => {}
                StmtKind::Break => return Flow::Break,
                StmtKind::Return => return Flow::Return,
                StmtKind::Block(b) => match self.walk(b, paper) {
                    Flow::Normal => {}
                    flow => return flow,
                },
                StmtKind::If { cond, then, otherwise } => {
                    let branch = if self.eval(cond).truthy() { &then[..] } else { otherwise.as_deref().unwrap_or_default() };
                    match self.walk(branch, paper) {
                        Flow::Normal => {}
                        flow => return flow,
                    }
                }
                StmtKind::While { cond, body } => {
                    while self.eval(cond).truthy() {
                        paper.steps += 1;
                        if paper.over() {
                            return Flow::Return;
                        }
                        match self.walk(body, paper) {
                            Flow::Break => break,
                            Flow::Return => return Flow::Return,
                            Flow::Normal => {}
                        }
                    }
                }
                StmtKind::Expr(Expr::Call { name, args }) if ACTIONS.contains(&name.as_str()) => {
                    paper.clock += self.paper_action(name, args, &mut paper.passes, paper.clock, screen, &[]);
                }
                StmtKind::Expr(Expr::Call { name, args }) if name == "Shake" => {
                    paper.clock += self.paper_shake(args, &mut paper.passes, paper.clock);
                }
                StmtKind::Expr(Expr::Call { name, args }) if name == "Wait" => {
                    paper.clock += args.first().map_or(0, |a| self.eval(a).int().max(0) as u32);
                }
                StmtKind::Expr(Expr::Call { name, args }) if name.starts_with("Wait") => {
                    let target = args.first().map_or(Value::Null, |a| self.eval(a));
                    let ends = self.matching(&target).iter().filter_map(|t| paper.passes.get(t).map(Pass::end_time).or_else(|| self.pending_by.get(t).copied())).max();
                    paper.clock = paper.clock.max(ends.unwrap_or(0));
                }
                StmtKind::Expr(Expr::Call { name, args }) if name == "Delete" || (name == "Request" && args.get(1).is_some_and(|a| self.eval(a).string().eq_ignore_ascii_case("disused"))) => {
                    let target = args.first().map_or(Value::Null, |a| self.eval(a));
                    let disused = name == "Request";
                    for t in self.matching(&target) {
                        paper.gone.push((t, paper.clock, disused));
                    }
                }
                _ => {
                    if let Flow::Return = self.exec_block(std::slice::from_ref(s)) {
                        return Flow::Return;
                    }
                }
            }
        }
        Flow::Normal
    }

    /// At a sync: the one-shot motions that ended leave their pictures at
    /// their last opacity (or delete them).
    pub(super) fn end_one_shots(&mut self) {
        let now = self.sound_clock();
        let ended: Vec<String> = self.one_shots.iter().filter(|(_, s)| s.end.is_some_and(|e| e <= now)).map(|(k, _)| k.clone()).collect();
        for k in ended {
            let s = self.one_shots.remove(&k).unwrap();
            if s.deleted {
                self.delete(&k);
            } else if let Some(o) = self.objects.get_mut(&k) {
                o.opacity = s.opacity;
            }
        }
    }

    /// At a sync, once the pictures are placed: the one-shot motions not
    /// started yet, for the pictures shown (one not shown takes its end at
    /// once).
    pub(super) fn start_one_shots(&mut self) -> Vec<Op> {
        let now = self.sound_clock();
        let mut ops = Vec::new();
        let mut unseen = Vec::new();
        for (k, s) in self.one_shots.iter_mut().filter(|(_, s)| s.end.is_none()) {
            let Some(shown) = self.shown.get(k) else {
                unseen.push(k.clone());
                continue;
            };
            ops.push(Op::SpriteMotion2 { layer: shown.layer, keys: s.keys.clone(), end: MotionEnd::Keep });
            s.end = Some(now + u64::from(s.keys.last().map_or(0, |k| k.ms)));
        }
        for k in unseen {
            if let Some(s) = self.one_shots.get_mut(&k) {
                s.end = Some(0);
            }
        }
        ops
    }
}
