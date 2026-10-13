//! Route map: which story file follows which, and under what condition.
//!
//! The story driver loops `call_scene nss/$GameName->$GameName_MAIN` (see
//! [`super::refs`]), so the successors of file X are the values `$GameName`
//! can hold when X's `_MAIN` scene returns. They are found by walking that
//! scene symbolically:
//!
//! - local string variables are tracked (`$GameName`, `$MainGameName =
//!   "@->" + $GameName`, `$Next_deadGame`, …); anything else is unknown;
//! - `if`/`else` and each `case` of a `select` fork the state, labelled with
//!   the condition or choice; forks that end with the same variables merge
//!   again, so only branches that matter keep their labels;
//! - `call_scene` to a scene of the same file (`"@->" + $GameName +
//!   "_SELECT"`, `_routeA`, the content scene) is walked inline;
//! - a variable the file reads but never sets (`$Next_deadGame` in the
//!   game-over files `mz01_*`, `$Next_GameName` in the timed exploration
//!   `mc02_031time.nss`) takes each string value assigned to it anywhere:
//!   literals, and values seen during the walk (`$Next_GameName =
//!   $GameName`). The map is rebuilt until no new values turn up.
//!
//! Paths where no branch sets `$GameName` (an `if`/`else if` chain on flags
//! from earlier files with no `else`, e.g. which heroine was killed) are
//! reported as unresolved; by the story's logic one branch always matches.
//! `$GameName` set to the file's own name is a retry (battle choices).
//!
//! The story stops when `$GameCircle` is set to 0/false or `$GameName` to a
//! non-string. Starting points are the `$GameName` literals in files that
//! aren't story files (the boot script, title and extra menus).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::nss::{AssignOp, BinaryOp, Expr, Item, NssFile, Stmt, StmtKind, Target, Var};

/// Where a story file goes next.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Next {
    File(String),
    /// The story loop ends (`$GameCircle = 0`, `$GameName = false`)
    End(String),
    /// `$GameName` names no file, or can't be worked out
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: Next,
    /// Conditions and choices on the way (`$香奈枝死亡 == true`,
    /// `choice @選択肢１`, `not …`)
    pub path: Vec<String>,
}

#[derive(Debug, Default)]
pub struct RouteMap {
    /// Files the boot script and menus start the story at
    pub starts: Vec<String>,
    /// Edges of every reachable story file
    pub edges: Vec<Edge>,
    /// Reachable story files, breadth-first from the starts
    pub reachable: Vec<String>,
    /// Story files (with a `_MAIN` scene) that nothing reaches
    pub unreachable: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Val {
    Str(String),
    /// A non-string literal, for `$GameName = false`, `$GameCircle = 0`
    Lit(String),
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    vars: BTreeMap<String, Val>,
    path: Vec<String>,
    /// `$GameName` was assigned (to tell a retry, `$GameName = <this
    /// file>`, from a path that sets nothing)
    next_set: bool,
}

/// Story files are the ones with a `<name>_MAIN` scene.
fn is_story(name: &str, file: &NssFile) -> bool {
    let main = format!("{name}_MAIN");
    file.items.iter().any(|i| matches!(i, Item::Scene(b) if b.name == main))
}

pub fn build(files: &BTreeMap<String, NssFile>) -> RouteMap {
    // Every string literal assigned to each local, game-wide.
    let mut literals: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut starts = BTreeSet::new();
    for (name, file) in files {
        let story = is_story(name, file);
        for_each_stmt(file, &mut |s| {
            if let StmtKind::Assign { target: Var::Local(v), op: AssignOp::Set, value: Some(Expr::Str(lit)), .. } = &s.kind {
                literals.entry(v.clone()).or_default().insert(lit.clone());
                if v == "GameName" && !story && lit.ends_with(".nss") && files.contains_key(lit) {
                    starts.insert(lit.clone());
                }
            }
        });
    }

    let mut known = literals;
    for _ in 0..8 {
        let walker = Walker { files, known: &known, learned: RefCell::new(BTreeMap::new()) };
        let map = walk(&walker, &starts, files);
        let learned = walker.learned.into_inner();
        let mut grew = false;
        for (var, vals) in learned {
            let set = known.entry(var).or_default();
            for v in vals {
                grew |= set.insert(v);
            }
        }
        if !grew {
            return map;
        }
    }
    let walker = Walker { files, known: &known, learned: RefCell::new(BTreeMap::new()) };
    walk(&walker, &starts, files)
}

fn walk(walker: &Walker, starts: &BTreeSet<String>, files: &BTreeMap<String, NssFile>) -> RouteMap {
    let mut map = RouteMap { starts: starts.iter().cloned().collect(), ..RouteMap::default() };
    let mut seen: BTreeSet<String> = starts.clone();
    let mut queue: VecDeque<String> = starts.iter().cloned().collect();
    while let Some(name) = queue.pop_front() {
        let edges = walker.successors(&name);
        for e in &edges {
            if let Next::File(f) = &e.to
                && seen.insert(f.clone())
            {
                queue.push_back(f.clone());
            }
        }
        map.reachable.push(name);
        map.edges.extend(edges);
    }
    map.unreachable =
        files.iter().filter(|(n, f)| !seen.contains(*n) && is_story(n, f)).map(|(n, _)| n.clone()).collect();
    map
}

fn for_each_stmt(file: &NssFile, f: &mut dyn FnMut(&Stmt)) {
    fn walk(stmts: &[Stmt], f: &mut dyn FnMut(&Stmt)) {
        for s in stmts {
            f(s);
            match &s.kind {
                StmtKind::If { then, otherwise, .. } => {
                    walk(then, f);
                    if let Some(o) = otherwise {
                        walk(o, f);
                    }
                }
                StmtKind::While { body, .. }
                | StmtKind::Select(body)
                | StmtKind::Block(body)
                | StmtKind::Case { body, .. } => walk(body, f),
                _ => {}
            }
        }
    }
    for item in &file.items {
        match item {
            Item::Function(func) => walk(&func.body, f),
            Item::Chapter(b) | Item::Scene(b) => walk(&b.body, f),
            Item::Include(_) => {}
        }
    }
}

struct Walker<'a> {
    files: &'a BTreeMap<String, NssFile>,
    /// String values each variable is known to take somewhere
    known: &'a BTreeMap<String, BTreeSet<String>>,
    /// Values assigned during this walk, for the next round
    learned: RefCell<BTreeMap<String, BTreeSet<String>>>,
}

/// Inline scene calls this deep at most.
const MAX_DEPTH: usize = 4;
/// Above this many states, flags are forgotten (see [`Walker::stmts`]).
const MAX_STATES: usize = 256;

impl Walker<'_> {
    fn successors(&self, name: &str) -> Vec<Edge> {
        let start = State { vars: BTreeMap::from([("GameName".into(), Val::Str(name.into()))]), path: Vec::new(), next_set: false };
        let main = format!("{name}_MAIN");
        let states = self.scene(name, &main, vec![start], 0);
        let edges: Vec<Edge> = states
            .into_iter()
            .map(|s| {
                let to = match (s.vars.get("GameCircle"), s.vars.get("GameName")) {
                    (Some(Val::Lit(v)), _) if v == "0" || v == "false" => Next::End(format!("$GameCircle = {v}")),
                    (_, Some(Val::Lit(v))) => Next::End(format!("$GameName = {v}")),
                    (_, Some(Val::Str(f))) if f == name && !s.next_set => {
                        Next::Unknown("no next file set: no branch matched".into())
                    }
                    (_, Some(Val::Str(f))) if self.files.get(f).is_some_and(|file| is_story(f, file)) => {
                        Next::File(f.clone())
                    }
                    (_, Some(Val::Str(f))) => Next::Unknown(format!("no story file {f}")),
                    (_, None) => Next::Unknown("$GameName not known".into()),
                };
                Edge { from: name.into(), to, path: s.path }
            })
            .collect();
        // One edge per destination, with the conditions every path there
        // shares: branches that only chose a background or ran the skip
        // check drop out.
        let mut by_target: BTreeMap<Next, Vec<Vec<String>>> = BTreeMap::new();
        for e in edges {
            by_target.entry(e.to).or_default().push(e.path);
        }
        by_target.into_iter().map(|(to, paths)| Edge { from: name.into(), to, path: shared(paths) }).collect()
    }

    /// Walks scene `scene` of `file` for each state.
    fn scene(&self, file: &str, scene: &str, states: Vec<State>, depth: usize) -> Vec<State> {
        let Some(body) = self.files.get(file).and_then(|f| {
            f.items.iter().find_map(|i| match i {
                Item::Scene(b) if b.name == scene => Some(&b.body),
                _ => None,
            })
        }) else {
            return states;
        };
        self.stmts(file, body, states, depth)
    }

    fn stmts(&self, file: &str, stmts: &[Stmt], mut states: Vec<State>, depth: usize) -> Vec<State> {
        for s in stmts {
            if states.len() > MAX_STATES {
                // Forget the flags (the minigame in function_magiccube.nss
                // forks on dozens); keep what steers the route.
                for st in &mut states {
                    st.vars.retain(|k, _| k.contains("GameName") || k == "GameCircle" || k.ends_with("deadGame"));
                }
                states = merge(states);
            }
            states = self.stmt(file, s, states, depth);
        }
        states
    }

    fn stmt(&self, file: &str, s: &Stmt, states: Vec<State>, depth: usize) -> Vec<State> {
        match &s.kind {
            StmtKind::Assign { target, op, value, index } if index.is_empty() => {
                let v = &key(target);
                let mut out = Vec::new();
                for st in states {
                    let vals = match (op, value) {
                        (AssignOp::Set, Some(e)) => self.eval(e, &st, file),
                        (AssignOp::Add, Some(e)) => self.eval(
                            &Expr::Binary {
                                op: BinaryOp::Add,
                                left: Box::new(Expr::Var(target.clone())),
                                right: Box::new(e.clone()),
                            },
                            &st,
                            file,
                        ),
                        _ => Vec::new(),
                    };
                    if vals.is_empty() {
                        let mut st = st;
                        st.vars.remove(v);
                        out.push(st);
                        continue;
                    }
                    let fork = vals.len() > 1;
                    for (val, from) in vals {
                        let mut st = st.clone();
                        if fork && let Some(from) = from {
                            st.path.push(from);
                        }
                        st.next_set |= v == "GameName";
                        if let Val::Str(s) = &val
                            && v != "GameName"
                        {
                            self.learned.borrow_mut().entry(v.clone()).or_default().insert(s.clone());
                        }
                        st.vars.insert(v.clone(), val);
                        out.push(st);
                    }
                }
                merge(out)
            }
            StmtKind::If { cond, then, otherwise } => {
                // Conditions on values set earlier in the walk (the flags a
                // choice scene sets) are decided per state.
                let (mut yes, mut no, mut open) = (Vec::new(), Vec::new(), Vec::new());
                for st in states {
                    match truth(cond, &st) {
                        Some(true) => yes.push(st),
                        Some(false) => no.push(st),
                        None => open.push(st),
                    }
                }
                let mut out = self.stmts(file, then, yes, depth);
                out.extend(match otherwise {
                    Some(o) => self.stmts(file, o, no, depth),
                    None => no,
                });
                let states = open;
                if states.is_empty() {
                    return merge(out);
                }
                // Engine state (`$SYSTEM_skip`, …) never steers the story;
                // such branches fork but aren't labelled.
                let labelled = !format!("{cond}").contains("SYSTEM_");
                let label = |neg: bool| if neg { format!("not {cond}") } else { format!("{cond}") };
                let with = |label: String, holds: bool| -> Vec<State> {
                    states
                        .iter()
                        .cloned()
                        .map(|mut st| {
                            if labelled {
                                st.path.push(label.clone());
                            }
                            assume(cond, holds, &mut st);
                            st
                        })
                        .collect()
                };
                out.extend(self.stmts(file, then, with(label(false), true), depth));
                let other = with(label(true), false);
                out.extend(match otherwise {
                    Some(o) => self.stmts(file, o, other, depth),
                    None => other,
                });
                merge(out)
            }
            StmtKind::Select(body) => {
                let mut cases = Vec::new();
                collect_cases(body, &mut Vec::new(), &mut cases);
                let mut out = Vec::new();
                for (conds, label, body) in cases {
                    let branch: Vec<State> = states
                        .iter()
                        .cloned()
                        .map(|mut st| {
                            st.path.extend(conds.iter().cloned());
                            st.path.push(format!("choice {label}"));
                            st
                        })
                        .collect();
                    out.extend(self.stmts(file, body, branch, depth));
                }
                if out.is_empty() { states } else { merge(out) }
            }
            StmtKind::While { body, .. } => {
                let mut out = states.clone();
                out.extend(self.stmts(file, body, states, depth));
                merge(out)
            }
            StmtKind::Block(body) => self.stmts(file, body, states, depth),
            StmtKind::CallScene(target) if depth < MAX_DEPTH => {
                let mut out = Vec::new();
                for st in states {
                    let names: Vec<String> = match target {
                        Target::Name(n) => vec![n.clone()],
                        Target::Var(v) => self
                            .eval(&Expr::Var(v.clone()), &st, file)
                            .into_iter()
                            .filter_map(|(v, _)| if let Val::Str(s) = v { Some(s) } else { None })
                            .collect(),
                    };
                    let local: Vec<&str> = names
                        .iter()
                        .filter_map(|n| n.strip_prefix("@->").or_else(|| n.strip_prefix(&format!("nss/{file}->"))))
                        .collect();
                    match local.as_slice() {
                        [scene] => out.extend(self.scene(file, scene, vec![st], depth + 1)),
                        _ => out.push(st),
                    }
                }
                merge(out)
            }
            _ => states,
        }
    }

    /// Possible values of a string expression, each with where it came
    /// from when it's a literal set in another file.
    fn eval(&self, e: &Expr, st: &State, file: &str) -> Vec<(Val, Option<String>)> {
        match e {
            Expr::Str(s) => vec![(Val::Str(s.clone()), None)],
            Expr::Int(v) => vec![(Val::Lit(v.to_string()), None)],
            Expr::Word(w) if w == "true" || w == "false" => vec![(Val::Lit(w.clone()), None)],
            Expr::Call { name, args } if name == "ModuleFileName" && args.is_empty() => {
                vec![(Val::Str(file.into()), None)]
            }
            Expr::Var(var) => match st.vars.get(&key(var)) {
                Some(val) => vec![(val.clone(), None)],
                None if matches!(var, Var::Global(_)) => Vec::new(),
                None => self
                    .known
                    .get(&key(var))
                    .map(|lits| lits.iter().map(|l| (Val::Str(l.clone()), Some(format!("{var} = \"{l}\" (set elsewhere)")))).collect())
                    .unwrap_or_default(),
            },
            Expr::Binary { op: BinaryOp::Add, left, right } => {
                let (l, r) = (self.eval(left, st, file), self.eval(right, st, file));
                let mut out = Vec::new();
                for (a, fa) in &l {
                    for (b, fb) in &r {
                        if let (Val::Str(a), Val::Str(b)) = (a, b) {
                            out.push((Val::Str(format!("{a}{b}")), fa.clone().or(fb.clone())));
                        }
                    }
                }
                out
            }
            _ => Vec::new(),
        }
    }
}

/// Variables are tracked by name, globals with their `#`.
fn key(v: &Var) -> String {
    match v {
        Var::Local(n) => n.clone(),
        Var::Global(n) => format!("#{n}"),
    }
}

/// Records what taking a branch says about a flag, so a later test of the
/// same flag goes the same way: `$x`, `$x == true/false/0/1`, `!…`, and
/// both sides of `&&` when it holds.
fn assume(cond: &Expr, holds: bool, st: &mut State) {
    let flag = |v: &Var, value: bool, st: &mut State| {
        st.vars.insert(key(v), Val::Lit(if value { "1" } else { "0" }.into()));
    };
    let as_bool = |e: &Expr| match e {
        Expr::Word(w) if w == "true" => Some(true),
        Expr::Word(w) if w == "false" => Some(false),
        Expr::Int(1) => Some(true),
        Expr::Int(0) => Some(false),
        _ => None,
    };
    match cond {
        Expr::Var(v) => flag(v, holds, st),
        Expr::Unary { op: super::nss::UnaryOp::Not, expr } => assume(expr, !holds, st),
        Expr::Binary { op: BinaryOp::And, left, right } if holds => {
            assume(left, true, st);
            assume(right, true, st);
        }
        Expr::Binary { op: BinaryOp::Or, left, right } if !holds => {
            assume(left, false, st);
            assume(right, false, st);
        }
        Expr::Binary { op: op @ (BinaryOp::Eq | BinaryOp::Ne), left, right } => {
            if let (Expr::Var(v), Some(b)) = (&**left, as_bool(right)) {
                flag(v, b == (holds == (*op == BinaryOp::Eq)), st);
            }
        }
        _ => {}
    }
}

/// The value of a condition, when the variables it reads are known.
fn truth(e: &Expr, st: &State) -> Option<bool> {
    fn value(e: &Expr, st: &State) -> Option<Val> {
        match e {
            Expr::Var(v) => st.vars.get(&key(v)).cloned(),
            Expr::Str(s) => Some(Val::Str(s.clone())),
            Expr::Int(v) => Some(Val::Lit(v.to_string())),
            Expr::Word(w) if w == "true" || w == "false" => Some(Val::Lit(w.clone())),
            _ => None,
        }
    }
    fn truthy(v: &Val) -> bool {
        match v {
            Val::Str(s) => !s.is_empty(),
            Val::Lit(l) => l != "false" && l != "0",
        }
    }
    // `true` and 1, `false` and 0 compare equal
    fn norm(v: Val) -> Val {
        match v {
            Val::Lit(l) if l == "true" => Val::Lit("1".into()),
            Val::Lit(l) if l == "false" => Val::Lit("0".into()),
            v => v,
        }
    }
    match e {
        Expr::Unary { op: super::nss::UnaryOp::Not, expr } => truth(expr, st).map(|b| !b),
        Expr::Binary { op: BinaryOp::And, left, right } => match (truth(left, st), truth(right, st)) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        },
        Expr::Binary { op: BinaryOp::Or, left, right } => match (truth(left, st), truth(right, st)) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        },
        Expr::Binary { op: op @ (BinaryOp::Eq | BinaryOp::Ne), left, right } => {
            let (l, r) = (norm(value(left, st)?), norm(value(right, st)?));
            Some((l == r) == (*op == BinaryOp::Eq))
        }
        _ => value(e, st).map(|v| truthy(&v)),
    }
}

/// Where two paths differ only by an `X` / `not X` pair (equal paths:
/// `Some(len)`).
fn opposite_at(a: &[String], b: &[String]) -> Option<usize> {
    let common = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    match (a.get(common), b.get(common)) {
        (Some(x), Some(y)) => {
            let opposite = x.strip_prefix("not ") == Some(y.as_str()) || y.strip_prefix("not ") == Some(x.as_str());
            (opposite && a[common + 1..] == b[common + 1..]).then_some(common)
        }
        (None, None) => Some(common),
        _ => None,
    }
}

/// The conditions all `paths` share: first `X` / `not X` pairs cancel
/// (repeatedly, across all paths), then what every remaining path contains.
fn shared(mut paths: Vec<Vec<String>>) -> Vec<String> {
    loop {
        let pair = (0..paths.len()).find_map(|i| {
            (i + 1..paths.len()).find(|&j| opposite_at(&paths[i], &paths[j]).is_some()).map(|j| (i, j))
        });
        let Some((i, j)) = pair else { break };
        let b = paths.remove(j);
        paths[i] = join_paths(&paths[i], &b);
    }
    let first = paths[0].clone();
    first.into_iter().filter(|c| paths.iter().all(|p| p.contains(c))).collect()
}

/// The conditions two paths to the same state share: without the
/// `X` / `not X` pair they differ by, or else their common prefix.
fn join_paths(a: &[String], b: &[String]) -> Vec<String> {
    match opposite_at(a, b) {
        Some(i) if i < a.len() => a[..i].iter().chain(&a[i + 1..]).cloned().collect(),
        Some(_) => a.to_vec(),
        None => a.iter().zip(b).take_while(|(x, y)| x == y).map(|(x, _)| x.clone()).collect(),
    }
}

/// The `case`s of a `select`, also those inside `if`/`else` (choices that
/// only appear under a flag: `if(#魔王編解放==true){ case @選択肢３{…} }`),
/// with the conditions around them.
fn collect_cases<'a>(stmts: &'a [Stmt], conds: &mut Vec<String>, out: &mut Vec<(Vec<String>, &'a str, &'a [Stmt])>) {
    for s in stmts {
        match &s.kind {
            StmtKind::Case { label, body } => out.push((conds.clone(), label, body)),
            StmtKind::If { cond, then, otherwise } => {
                conds.push(format!("{cond}"));
                collect_cases(then, conds, out);
                conds.pop();
                if let Some(o) = otherwise {
                    conds.push(format!("not {cond}"));
                    collect_cases(o, conds, out);
                    conds.pop();
                }
            }
            StmtKind::Block(b) | StmtKind::While { body: b, .. } => collect_cases(b, conds, out),
            _ => {}
        }
    }
}

/// Merges states with the same variables; their paths keep only the
/// conditions they share (a branch that didn't matter loses its label).
fn merge(states: Vec<State>) -> Vec<State> {
    let mut out: Vec<State> = Vec::new();
    for st in states {
        match out.iter_mut().find(|o| o.vars == st.vars && o.next_set == st.next_set) {
            Some(o) => o.path = join_paths(&o.path, &st.path),
            None => out.push(st),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nitroplus::nss::parse;

    fn corpus(files: &[(&str, &str)]) -> BTreeMap<String, NssFile> {
        files.iter().map(|(n, s)| (n.to_string(), parse(s).unwrap())).collect()
    }

    const BOOT: &str = "chapter main{ $GameName = \"a.nss\"; }";

    #[test]
    fn follows_branches_choices_and_suffix_scenes() {
        let files = corpus(&[
            ("boot.nss", BOOT),
            (
                "a.nss",
                "scene a.nss_MAIN{ $MainGameName=\"@->\"+$GameName; call_scene $MainGameName; if($x == 1){ Foo(); }\n\
                 $SelectGameName=\"@->\"+$GameName+\"_SELECT\"; call_scene $SelectGameName; }\n\
                 scene a.nss{}\n\
                 scene a.nss_SELECT{ select{ Cursor(); if(#解放){ case @選択肢１{ $GameName = \"b.nss\"; } case @選択肢２{ $GameName = \"c.nss\"; } }else{ case @選択肢１{ $GameName = \"b.nss\"; } } } }",
            ),
            ("b.nss", "scene b.nss_MAIN{ if($dead == true){ $Next_deadGame = \"c.nss\"; $GameName = \"z.nss\"; } else { $GameName = $GameName + \"_x\"; } }\nscene b.nss_x{}"),
            ("c.nss", "scene c.nss_MAIN{ select{ case @1{ $f = true; } case @2{ $f = false; $g = true; } } if($f == true){ $GameName = \"z.nss\"; } else if($g){ $GameCircle=0; } }"),
            ("z.nss", "scene z.nss_MAIN{ $GameName = $Next_deadGame; }"),
            ("u.nss", "scene u.nss_MAIN{}"),
            ("menu.nss", "chapter main{ $GameName = \"r.nss\"; }"),
            ("t.nss", "scene t.nss_MAIN{ $GameName = $Next_GameName; }"),
            ("r.nss", "scene r.nss_MAIN{ select{ case @1{ $GameName = \"r.nss\"; } case @2{} case @3{ $GameName = \"c.nss\"; $Next_GameName = $GameName; $GameName = \"t.nss\"; } } }"),
        ]);
        let map = build(&files);
        assert_eq!(map.starts, vec!["a.nss", "r.nss"]);
        assert_eq!(map.reachable, vec!["a.nss", "r.nss", "b.nss", "c.nss", "t.nss", "z.nss"]);
        assert_eq!(map.unreachable, vec!["u.nss"]);
        let edges: Vec<(&str, &Next, Vec<&str>)> =
            map.edges.iter().map(|e| (e.from.as_str(), &e.to, e.path.iter().map(String::as_str).collect())).collect();
        assert_eq!(
            edges,
            vec![
                ("a.nss", &Next::File("b.nss".into()), vec!["choice @選択肢１"]),
                ("a.nss", &Next::File("c.nss".into()), vec!["#解放", "choice @選択肢２"]),
                ("r.nss", &Next::File("r.nss".into()), vec!["choice @1"]),
                ("r.nss", &Next::File("t.nss".into()), vec!["choice @3"]),
                ("r.nss", &Next::Unknown("no next file set: no branch matched".into()), vec!["choice @2"]),
                ("b.nss", &Next::File("z.nss".into()), vec!["($dead == true)"]),
                ("b.nss", &Next::Unknown("no story file b.nss_x".into()), vec!["not ($dead == true)"]),
                ("c.nss", &Next::File("z.nss".into()), vec!["choice @1"]),
                ("c.nss", &Next::End("$GameCircle = 0".into()), vec!["choice @2"]),
                ("t.nss", &Next::File("c.nss".into()), vec![]),
                ("z.nss", &Next::File("c.nss".into()), vec![]),
            ]
        );
    }
}
