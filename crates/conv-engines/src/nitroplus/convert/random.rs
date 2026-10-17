//! `Random(n)`, picked when the game runs. The exe gives `rand() % n`
//! (`eval_primary` 0043a780, seeded again from the clock at every call;
//! 0 for n = 0): 0 to |n| − 1. A statement whose code draws one (in a
//! function it calls, or a process it starts) is compiled once per pick,
//! and NScripter's `rnd` chooses among them at run time: every value when
//! it draws once from at most [`EXACT`], else [`SAMPLES`] draws of every
//! call. The statements after it that read what it set go with it. Picks
//! that would hold text, or leave the pictures unlike for what follows,
//! are picked when converting instead (noted).

use super::{Compiler, Flow, State, Value};
use crate::nitroplus::nss::Var;
use crate::nitroplus::nss::{Expr, Stmt, StmtKind};
use conv_core::ir::{Compare, Op, Operand};
use std::collections::{BTreeMap, HashMap, HashSet};

/// A single draw of at most this many values is compiled once per value.
const EXACT: i64 = 16;
/// Picks compiled for a statement that draws more than once.
pub(super) const SAMPLES: u32 = 8;
/// More ops than this over all the picks: picked when converting.
const MAX_OPS: usize = 6000;
/// The compiler's state outside [`State`] that a pick changes: the clock,
/// the pictures moving or shaking, the one-shots, the counters labels are
/// numbered by.
#[derive(Clone)]
pub(super) struct Extra {
    clock_ms: u64,
    vanishing: HashMap<String, u64>,
    motion_until: HashMap<String, u64>,
    runs: BTreeMap<String, super::Run>,
    shaken: Vec<String>,
    one_shots: BTreeMap<String, super::one_shot::OneShot>,
    lead_ins: BTreeMap<String, u32>,
    process_lead: u32,
    choices: usize,
    texts_shown: usize,
    forks_in_file: u16,
    leaves_in_file: i32,
}

/// Statements to run again (`Expr` and `Assign`) in a pick of the
/// `Random` after them, at most this many.
const BACK: usize = 8;

/// The compiler before statement `at`, nothing waiting to be shown: a
/// pick of a `Random` a few statements on starts here.
pub(super) struct Checkpoint<'a> {
    at: usize,
    state: State<'a>,
    extra: Extra,
    ops: usize,
    counted: (BTreeMap<String, usize>, crate::metrics::Metrics),
}

pub(super) fn simple(s: &Stmt) -> bool {
    matches!(s.kind, StmtKind::Expr(_) | StmtKind::Assign { .. })
}

/// The NScripter variable `rnd` sets (a local past the fork variables).
pub(super) const RANDOM_VAR: u16 = 71;

/// The picks being compiled.
#[derive(Debug, Clone)]
pub(super) struct Draws {
    /// `Some(v)`: the one draw gives `v`; `None`: a stream from `seed`.
    exact: Option<i64>,
    seed: u64,
    /// Draws so far, and the widest range of them.
    count: u32,
    range: i64,
    /// Picked when converting (noted).
    fixed: bool,
}

impl Draws {
    fn new(exact: Option<i64>, seed: u64, fixed: bool) -> Self {
        Draws { exact, seed, count: 0, range: 0, fixed }
    }

    fn next(&mut self, n: i64) -> i64 {
        let n = n.abs();
        self.count += 1;
        self.range = self.range.max(n);
        if n <= 1 {
            return 0;
        }
        if let Some(v) = self.exact {
            return v.min(n - 1);
        }
        // splitmix64: the same picks on every build.
        self.seed = self.seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as i64
    }
}

fn fnv(s: &str, line: u32) -> u64 {
    s.bytes().chain(line.to_le_bytes()).fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))
}

/// Calls `f` on every expression in `s`, nested statements included.
fn exprs<'e>(s: &'e Stmt, f: &mut dyn FnMut(&'e Expr)) {
    fn expr<'e>(e: &'e Expr, f: &mut dyn FnMut(&'e Expr)) {
        f(e);
        match e {
            Expr::Index { base, index } => {
                expr(base, f);
                expr(index, f);
            }
            Expr::Call { args, .. } => args.iter().for_each(|a| expr(a, f)),
            Expr::Unary { expr: e, .. } => expr(e, f),
            Expr::Binary { left, right, .. } => {
                expr(left, f);
                expr(right, f);
            }
            Expr::Point { x, y, .. } => {
                expr(x, f);
                expr(y, f);
            }
            Expr::Path(p) => p.iter().for_each(|e| expr(e, f)),
            _ => {}
        }
    }
    let block = |b: &'e [Stmt], f: &mut dyn FnMut(&'e Expr)| b.iter().for_each(|s| exprs(s, f));
    match &s.kind {
        StmtKind::Expr(e) => expr(e, f),
        StmtKind::Assign { index, value, .. } => index.iter().chain(value).for_each(|e| expr(e, f)),
        StmtKind::If { cond, then, otherwise } => {
            expr(cond, f);
            block(then, f);
            block(otherwise.as_deref().unwrap_or_default(), f);
        }
        StmtKind::While { cond, body } => {
            expr(cond, f);
            block(body, f);
        }
        StmtKind::Select(b) | StmtKind::Case { body: b, .. } | StmtKind::Block(b) => block(b, f),
        _ => {}
    }
}

/// Ops that a pick can't hold: text, choices and code jumping about.
fn plain(op: &Op) -> bool {
    !matches!(
        op,
        Op::Label(_)
            | Op::Jump(_)
            | Op::Gosub(_)
            | Op::Return
            | Op::If { .. }
            | Op::IfSwitch { .. }
            | Op::IfSkipping { .. }
            | Op::IfFastSkip { .. }
            | Op::Text { .. }
            | Op::InstantText { .. }
            | Op::ClickWait { .. }
            | Op::Choice { .. }
            | Op::RuntimeChoice { .. }
            | Op::BacklogLine { .. }
            | Op::RuntimeBacklogLine { .. }
            | Op::Chapter { .. }
            | Op::SceneEnd
            | Op::ReturnToRoot
            | Op::ReturnToTitle
            | Op::StartProcess { .. }
            | Op::StopProcess { .. }
    )
}

/// Sound ops: conv-cons gives them channels in turn.
fn sound(op: &Op) -> bool {
    matches!(
        op,
        Op::SePlay { .. }
            | Op::SeStop { .. }
            | Op::SeFadeIn { .. }
            | Op::SeFadeOut { .. }
            | Op::SeNotSkipped
            | Op::SoundVolume { .. }
            | Op::SoundPan { .. }
            | Op::SoundRate { .. }
            | Op::BgmPlay { .. }
            | Op::BgmStop { .. }
            | Op::BgmLater { .. }
            | Op::BgmCrossfade { .. }
    )
}

/// A pick's ops cut at its sounds: the parts between them (one more than
/// the runs of sounds) and the runs.
fn split_sounds(ops: Vec<Op>) -> (Vec<Vec<Op>>, Vec<Vec<Op>>) {
    let (mut parts, mut sounds) = (vec![Vec::new()], Vec::new());
    for op in ops {
        if sound(&op) {
            if parts.len() == sounds.len() + 1 {
                sounds.push(Vec::new());
            }
            sounds.last_mut().unwrap().push(op);
        } else {
            if parts.len() == sounds.len() {
                parts.push(Vec::new());
            }
            parts.last_mut().unwrap().push(op);
        }
    }
    if parts.len() == sounds.len() {
        parts.push(Vec::new());
    }
    (parts, sounds)
}

impl<'a> Compiler<'a, '_> {
    /// `Random(n)`: the pick being compiled, or 0 (noted) outside one.
    pub(super) fn random(&mut self, n: i64) -> Value {
        let (v, noted) = match &mut self.random {
            Some(d) => (d.next(n), d.fixed),
            None => (0, true),
        };
        if noted {
            self.note("Random picked when converting");
        }
        Value::Int(v)
    }

    /// Whether `s` draws a `Random`, in a function it calls or a process
    /// it names.
    fn draws(&mut self, s: &'a Stmt) -> bool {
        let mut names = Vec::new();
        exprs(s, &mut |e| {
            if let Expr::Call { name, args } = e {
                names.push(name.as_str());
                if let (true, Some(Expr::Str(p))) = (name == "CreateProcess", args.get(4)) {
                    names.push(p.as_str());
                }
            }
        });
        names.into_iter().any(|n| self.function_draws(n))
    }

    fn function_draws(&mut self, name: &str) -> bool {
        if name == "Random" {
            return true;
        }
        if let Some(&d) = self.random_users.get(name) {
            return d;
        }
        let Some((_, f)) = self.function(name) else { return false };
        // A loop (a process's) doesn't end on paper: its draws stay noted.
        let endless = |s: &Stmt| matches!(&s.kind, StmtKind::While { cond: Expr::Int(1..) | Expr::Word(_), .. });
        if super::loops::is_loop(f) || f.body.iter().any(endless) {
            return false;
        }
        self.random_users.insert(name.to_string(), false);
        let d = f.body.iter().any(|s| self.draws(s));
        self.random_users.insert(name.to_string(), d);
        d
    }

    /// Where the statements at `part[i..]` that draw end: `part[i]` and
    /// those after it that read a variable it set (or one set from those).
    fn draw_end(part: &[Stmt], i: usize) -> usize {
        let mut set: HashSet<&Var> = HashSet::new();
        if let StmtKind::Assign { target, .. } = &part[i].kind {
            set.insert(target);
        }
        let mut end = i;
        for (k, s) in part.iter().enumerate().skip(i + 1) {
            let mut reads = false;
            exprs(s, &mut |e| reads |= matches!(e, Expr::Var(v) if set.contains(v)));
            if reads {
                end = k;
                if let StmtKind::Assign { target, .. } = &s.kind {
                    set.insert(target);
                }
            }
        }
        end
    }

    /// Statement `part[i]` when it draws a `Random` (and none is being
    /// compiled, nor a process on paper): compiled with what follows it
    /// once per pick, `rnd` choosing; the index of the last statement run
    /// and its flow. None when it doesn't draw.
    pub(super) fn exec_drawing(&mut self, part: &'a [Stmt], i: usize, from: &mut Option<Checkpoint<'a>>) -> Option<(usize, Flow)> {
        if self.random.is_some() || self.paper > 0 || !self.draws(&part[i]) {
            return None;
        }
        let mut end = Self::draw_end(part, i);
        // From the last statement before it where nothing waited to be
        // shown (taken back), else what waits is shown first.
        let start = match from.take() {
            Some(c) => {
                self.ops.truncate(c.ops);
                (self.notes, self.metrics) = c.counted;
                self.clear_pending();
                self.restore(c.state);
                self.set_extra(c.extra);
                c.at
            }
            None => {
                if self.pending() {
                    self.sync(0);
                }
                i
            }
        };
        self.sound_clock();
        let base = (self.snapshot(), self.extra());
        let counted = (self.notes.clone(), self.metrics.clone());
        let seed = fnv(self.file, part[i].line);
        // A probe first: one draw of a few values is compiled once per value.
        // It goes on to the statement that waits for what it started, so
        // that it is shown with what follows it, as before.
        let probe = self.run_pick(&base, part, start, &mut end, true, false, Draws::new(None, seed, false));
        let (n, exact) = match probe {
            // Always 0 (`Random(1)`): once, as it stands.
            Some((_, _, d)) if d.range <= 1 => (1, true),
            Some((_, _, d)) if d.count == 1 && d.range <= EXACT => (d.range as u32, true),
            Some(_) => (SAMPLES, false),
            None => (0, false),
        };
        let probed = (self.notes.clone(), self.metrics.clone());
        // Left unshown at their end when every pick leaves the same to
        // show (it is shown with what follows, as before); else each pick
        // shows its own. Picks that leave the pictures unlike go on over a
        // few more statements (till a picture moved to a random place
        // goes).
        let probe_end = end;
        let mut picks: Vec<Vec<Op>> = Vec::new();
        let mut last = None;
        let mut once = probed.clone();
        'found: for sync_end in [false, true] {
            for grow in 0..=BACK {
                let mut e = probe_end + grow;
                if grow > 0 && !part.get(e).is_some_and(simple) {
                    break;
                }
                picks.clear();
                last = None;
                let mut left = None;
                for k in 0..if n > 1 { n } else { 0 } {
                    let draws = Draws::new(exact.then_some(i64::from(k)), seed.wrapping_add(u64::from(k + 1) << 32), false);
                    // Counted once, as the first pick ran (it may run on
                    // further than the probe).
                    (self.notes, self.metrics) = counted.clone();
                    let r = self.run_pick(&base, part, start, &mut e, false, sync_end, draws);
                    if k == 0 {
                        once = (self.notes.clone(), self.metrics.clone());
                    }
                    (self.notes, self.metrics) = probed.clone();
                    let Some((ops, flow, _)) = r else { break 'found };
                    // Every pick must leave the pictures alike for what follows.
                    if last.as_ref().is_some_and(|(s, _, _): &(State, _, _)| s.shown != self.shown) {
                        break;
                    }
                    // And what is still to show alike (all of it unshown;
                    // shown, the pictures, a loop's keys aside).
                    let unshown = if sync_end { self.pictures() } else { self.unshown() };
                    if *left.get_or_insert_with(|| unshown.clone()) != unshown {
                        break;
                    }
                    picks.push(ops);
                    last = Some((self.snapshot(), self.extra(), flow));
                }
                if picks.len() == n as usize {
                    end = e;
                    break 'found;
                }
            }
        }
        if picks.len() < 2 || picks.len() != n as usize || picks.iter().map(Vec::len).sum::<usize>() > MAX_OPS {
            // Once, as it stood, drawing when converting (noted) where a
            // pick changes the pictures or holds text.
            let (state, extra) = base;
            self.clear_pending();
            self.restore(state);
            self.set_extra(extra);
            (self.notes, self.metrics) = counted;
            self.random = Some(Draws::new(None, seed, n != 1));
            let flow = self.run_stmts(&part[start..=probe_end]);
            self.random = None;
            return Some((probe_end, flow));
        }
        let (state, extra, flow) = last.unwrap();
        (self.notes, self.metrics) = once;
        self.restore(state);
        let clock = extra.clock_ms;
        self.set_extra(extra);
        // Labels made in a pick went with it.
        self.choices = base.1.choices;
        self.texts_shown = base.1.texts_shown;
        // The sounds every pick starts alike are played once, between
        // the choices of the parts around them (conv-cons gives each sound
        // its channel in the order it meets them).
        let split: Vec<_> = picks.iter().cloned().map(split_sounds).collect();
        let (mut parts, sounds): (Vec<Vec<Vec<Op>>>, Vec<Vec<Op>>) = if split.iter().all(|(_, s)| *s == split[0].1) {
            let sounds = split[0].1.clone();
            (split.into_iter().map(|(p, _)| p).collect(), sounds)
        } else {
            // shortcut: picks that start different sounds play them in turn in conv-cons' channel model (none in 装甲悪鬼村正)
            self.note("Random picks with different sounds");
            (picks.into_iter().map(|p| vec![p]).collect(), Vec::new())
        };
        let var = conv_core::ir::Var { global: false, index: RANDOM_VAR };
        self.random_sites += 1;
        let id = self.random_sites;
        let mut drawn = false;
        let count = parts.iter().map(Vec::len).min().unwrap_or(0);
        for j in 0..count {
            let segs: Vec<Vec<Op>> = parts.iter_mut().map(|p| std::mem::take(&mut p[j])).collect();
            self.choose(var, &format!("rnd{id}_{j}"), segs, &mut drawn);
            if j + 1 < count {
                self.ops.extend(sounds[j].iter().cloned());
            }
        }
        // The time the last pick took (its ops counted already).
        self.clock_ms = clock;
        self.clock_scanned = self.ops.len();
        Some((end, flow))
    }

    /// Before statement `part[i]`: a [`Checkpoint`] when nothing waits
    /// to be shown and a statement that draws a `Random` comes within
    /// [`BACK`] simple ones.
    pub(super) fn checkpoint(&mut self, part: &'a [Stmt], i: usize) -> Option<Checkpoint<'a>> {
        if self.random.is_some() || self.paper > 0 || self.pending() || !simple(&part[i]) {
            return None;
        }
        let mut ahead = part[i + 1..].iter().take(BACK);
        if !ahead.try_fold((), |(), s| if self.draws(s) { Err(true) } else if simple(s) { Ok(()) } else { Err(false) }).is_err_and(|d| d) {
            return None;
        }
        self.sound_clock();
        Some(Checkpoint { at: i, state: self.snapshot(), extra: self.extra(), ops: self.ops.len(), counted: (self.notes.clone(), self.metrics.clone()) })
    }

    /// Whether something waits for the next sync (what [`Compiler::sync`]
    /// takes), which a [`State`] doesn't hold.
    fn pending(&self) -> bool {
        self.pending_ms > 0
            || !self.pending_by.is_empty()
            || !self.pending_tempo.is_empty()
            || self.fade_tempo.is_some()
            || !self.pending_turn.is_empty()
            || !self.pending_fade.is_empty()
            || !self.pending_camera.is_empty()
            || !self.shake_once.is_empty()
            || !self.pending_path.is_empty()
            || self.pending_mask.is_some()
            || self.pending_enter.is_some()
    }

    /// What waits to be shown at the next sync, and the objects it
    /// shows, written out to compare picks.
    fn unshown(&self) -> String {
        let sorted = |m: &dyn std::fmt::Debug| {
            let mut v: Vec<String> = format!("{m:?}").split(", \"").map(str::to_string).collect();
            v.sort();
            v.join(",")
        };
        format!(
            "{} {} {} {:?} {} {} {} {} {} {:?} {:?} {:?}",
            self.pending_ms,
            sorted(&self.pending_by),
            sorted(&self.pending_tempo),
            self.fade_tempo,
            sorted(&self.pending_turn),
            sorted(&self.pending_fade),
            sorted(&self.pending_camera),
            sorted(&self.shake_once),
            sorted(&self.pending_path),
            self.pending_mask,
            self.pending_enter,
            self.objects
        )
    }

    /// The pictures, but for the keys of the loops shown ones play (shown
    /// in each pick), written out to compare picks.
    fn pictures(&self) -> String {
        let mut objects = self.objects.clone();
        objects.iter_mut().filter(|(n, _)| self.shown.contains_key(*n)).for_each(|(_, o)| o.shake = None);
        format!("{objects:?}")
    }

    fn clear_pending(&mut self) {
        self.pending_ms = 0;
        self.pending_by.clear();
        self.pending_tempo.clear();
        self.fade_tempo = None;
        self.pending_turn.clear();
        self.pending_fade.clear();
        self.pending_camera.clear();
        self.shake_once.clear();
        self.pending_path.clear();
        self.pending_mask = None;
        self.pending_enter = None;
    }

    /// Ops `picks` (one list per pick) as they stand, those every pick
    /// has alike at the start and end given once, the rest behind an `rnd`
    /// into `var` (drawn once, `drawn`) and a jump to pick k's label.
    fn choose(&mut self, var: conv_core::ir::Var, label: &str, picks: Vec<Vec<Op>>, drawn: &mut bool) {
        let shortest = picks.iter().map(Vec::len).min().unwrap_or(0);
        let head = (0..shortest).take_while(|&k| picks.iter().all(|p| p[k] == picks[0][k])).count();
        let tail = (0..shortest - head).take_while(|&k| picks.iter().all(|p| p[p.len() - 1 - k] == picks[0][picks[0].len() - 1 - k])).count();
        self.ops.extend(picks[0][..head].iter().cloned());
        let after: Vec<Op> = picks[0][picks[0].len() - tail..].to_vec();
        let middles: Vec<&[Op]> = picks.iter().map(|p| &p[head..p.len() - tail]).collect();
        if middles.iter().any(|m| *m != middles[0]) {
            if !std::mem::replace(drawn, true) {
                self.ops.push(Op::Random { var, below: picks.len() as i32 });
            }
            for k in 1..picks.len() {
                self.ops.push(Op::If { var, cmp: Compare::Eq, value: Operand::Const(k as i32), label: format!("{label}_{k}") });
            }
            for (k, ops) in middles.into_iter().enumerate() {
                if k > 0 {
                    self.ops.push(Op::Jump(format!("{label}_end")));
                    self.ops.push(Op::Label(format!("{label}_{k}")));
                }
                self.ops.extend(ops.iter().cloned());
            }
            self.ops.push(Op::Label(format!("{label}_end")));
        } else {
            self.ops.extend(middles[0].iter().cloned());
        }
        self.ops.extend(after);
    }

    fn extra(&self) -> Extra {
        Extra {
            clock_ms: self.clock_ms,
            vanishing: self.vanishing.clone(),
            motion_until: self.motion_until.clone(),
            runs: self.runs.clone(),
            shaken: self.shaken.clone(),
            one_shots: self.one_shots.clone(),
            lead_ins: self.lead_ins.clone(),
            process_lead: self.process_lead,
            choices: self.choices,
            texts_shown: self.texts_shown,
            forks_in_file: self.forks_in_file,
            leaves_in_file: self.leaves_in_file,
        }
    }

    fn set_extra(&mut self, e: Extra) {
        Extra {
            clock_ms: self.clock_ms,
            vanishing: self.vanishing,
            motion_until: self.motion_until,
            runs: self.runs,
            shaken: self.shaken,
            one_shots: self.one_shots,
            lead_ins: self.lead_ins,
            process_lead: self.process_lead,
            choices: self.choices,
            texts_shown: self.texts_shown,
            forks_in_file: self.forks_in_file,
            leaves_in_file: self.leaves_in_file,
        } = e;
    }

    /// One pick of `part[i..=end]` from `base` (`extend`: and on while
    /// something it started waits for a later statement, up to text;
    /// `sync_end`: then shown): its ops (taken off the list), flow and draws. None when it holds text or
    /// code that jumps.
    #[allow(clippy::too_many_arguments)]
    fn run_pick(&mut self, base: &(State<'a>, Extra), part: &'a [Stmt], i: usize, end: &mut usize, extend: bool, sync_end: bool, draws: Draws) -> Option<(Vec<Op>, Flow, Draws)> {
        self.clear_pending();
        self.restore(base.0.clone());
        self.set_extra(base.1.clone());
        let fork = (self.fork_var, self.pending_fork.is_some());
        let start = self.ops.len();
        self.clock_scanned = start;
        self.random = Some(draws);
        let mut flow = self.run_stmts(&part[i..=*end]);
        // (Or till a picture given a loop is shown, which starts it.)
        let mut more = BACK;
        while extend
            && matches!(flow, Flow::Normal)
            && (self.pending() || self.objects.iter().any(|(n, o)| o.shake.is_some() && !self.shown.contains_key(n)) && more > 0)
            && part.get(*end + 1).is_some_and(simple)
        {
            more = more.saturating_sub(1);
            *end += 1;
            flow = self.run_stmts(&part[*end..=*end]);
        }
        if sync_end {
            self.sync(0);
        }
        self.sound_clock();
        let draws = self.random.take().unwrap();
        let ops = self.ops.split_off(start);
        let whole = fork == (self.fork_var, self.pending_fork.is_some());
        (matches!(flow, Flow::Normal) && whole && ops.iter().all(plain)).then_some((ops, flow, draws))
    }

    fn run_stmts(&mut self, stmts: &'a [Stmt]) -> Flow {
        for s in stmts {
            let flow = self.exec(s);
            if self.in_function == 0 {
                self.flush_sounds();
            }
            if !matches!(flow, Flow::Normal) {
                return flow;
            }
        }
        Flow::Normal
    }
}
