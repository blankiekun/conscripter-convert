//! Cross-reference checks over a whole set of parsed NSS files.
//!
//! The engine loads scripts by path (`nss/ma01_002.nss`) and calls scenes
//! and chapters by name:
//!
//! ```text
//! call_scene nss/ma01_002.nss->ma01_002.nss_MAIN;   file + scene
//! call_scene @->ma01_002.nss;                       scene in the current file
//! call_chapter nss/sys_load.nss;                    the file's `chapter main`
//! call_chapter nss/extra_gallery.nss->main2;        another chapter
//! ```
//!
//! Most targets are built at run time: the story driver in
//! `0_boot_開始スクリプト.nss` loops `call_scene "nss/" + $GameName + "->" +
//! $GameName + "_MAIN"`, and each file's `_MAIN` scene runs
//! `call_scene "@->" + $GameName` (its own content scene) and then sets
//! `$GameName` to the next file. So besides written-out targets, every
//! string literal that names a script file (`"ma01_003.nss"`) is checked:
//! the file must exist and hold both scenes.
//!
//! Functions share one namespace across all files (story files call the
//! helpers in `function*.nss` without `#include`). A called name that no
//! file defines is an engine built-in (or a typo); [`Report::builtins`]
//! lists them with their use counts.
//!
//! Calls may leave out trailing arguments (the body then sees the
//! parameter empty, judging by `if(!$回想判定)` in `FwC`), so only calls with
//! more arguments than parameters are reported.

use std::collections::{BTreeMap, BTreeSet};

use super::nss::{Expr, Inline, Item, NssFile, Stmt, StmtKind, Target, UnaryOp};

#[derive(Debug, Default)]
pub struct Report {
    /// `file: message` for every reference that doesn't resolve
    pub problems: Vec<String>,
    /// Called names no file defines, with how they are called
    pub builtins: BTreeMap<String, Builtin>,
    /// Functions defined in more than one file: name → files
    pub duplicates: BTreeMap<String, Vec<String>>,
    pub checked_targets: usize,
    pub checked_names: usize,
    pub checked_calls: usize,
}

/// How the scripts call one built-in, to work out what it does.
#[derive(Debug, Default)]
pub struct Builtin {
    pub calls: usize,
    /// argument count → calls
    pub arities: BTreeMap<usize, usize>,
    /// per argument position: value as written (literals, variable names,
    /// `Call(…)`, `<expr>` for anything else) → calls
    pub args: Vec<BTreeMap<String, usize>>,
}

/// Argument summary for [`Builtin::args`].
fn shape(e: &Expr) -> String {
    match e {
        Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Color(_) | Expr::Word(_) | Expr::Var(_) => e.to_string(),
        Expr::Unary { op: UnaryOp::Relative | UnaryOp::Neg, expr }
            if matches!(**expr, Expr::Int(_) | Expr::Float(_) | Expr::Var(_)) =>
        {
            e.to_string()
        }
        Expr::Call { name, .. } => format!("{name}(…)"),
        Expr::Point { .. } | Expr::Path(_) => "<point>".into(),
        _ => "<expr>".into(),
    }
}

struct Def {
    file: String,
    params: usize,
}

/// `files` maps file names as the engine sees them (`ma01_001.nss`, without
/// `nss/`) to their parsed scripts.
pub fn check(files: &BTreeMap<String, NssFile>) -> Report {
    let mut report = Report::default();
    let mut functions: BTreeMap<&str, Vec<Def>> = BTreeMap::new();
    let mut blocks: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (name, file) in files {
        let names = blocks.entry(name.as_str()).or_default();
        for item in &file.items {
            match item {
                Item::Function(f) => {
                    functions.entry(&f.name).or_default().push(Def { file: name.clone(), params: f.params.len() })
                }
                Item::Chapter(b) | Item::Scene(b) => {
                    names.insert(&b.name);
                }
                Item::Include(_) => {}
            }
        }
    }
    for (name, defs) in &functions {
        if defs.len() > 1 {
            report.duplicates.insert(name.to_string(), defs.iter().map(|d| d.file.clone()).collect());
        }
    }

    let resolve = |current: &str, target: &str, chapter: bool| -> Option<String> {
        let (file, block) = match target.split_once("->") {
            Some(("@", block)) => (current, block),
            Some((path, block)) => (path.strip_prefix("nss/").unwrap_or(path), block),
            None if chapter => (target.strip_prefix("nss/").unwrap_or(target), "main"),
            // `call_cut main_stage` (続・殺戮のジャンゴ): a scene of this file
            None if blocks.get(current).is_some_and(|names| names.contains(target)) => return None,
            None => return Some(format!("scene target without '->': {target}")),
        };
        match blocks.get(file) {
            None => Some(format!("no file {file} ({target})")),
            Some(names) if !names.contains(block) => Some(format!("no scene/chapter {block} in {file}")),
            Some(_) => None,
        }
    };

    for (name, file) in files {
        let mut walk = Walk { strings: Vec::new(), calls: Vec::new(), targets: Vec::new() };
        for item in &file.items {
            match item {
                Item::Function(f) => walk.stmts(&f.body),
                Item::Chapter(b) | Item::Scene(b) => walk.stmts(&b.body),
                Item::Include(path) => walk.strings.push((0, path.clone())),
            }
        }
        for (line, target, chapter) in &walk.targets {
            report.checked_targets += 1;
            if let Some(p) = resolve(name, target, *chapter) {
                report.problems.push(format!("{name}:{line}: {p}"));
            }
        }
        for (line, s) in &walk.strings {
            // "ma01_003.nss", "nss/sys_load.nss", "nss/x.nss->main2"
            let path = s.strip_prefix("nss/");
            let (base, block) = s.split_once("->").map_or((s.as_str(), None), |(a, b)| (a, Some(b)));
            if !base.ends_with(".nss") || base.contains(['%', ' ']) {
                continue;
            }
            report.checked_names += 1;
            let file = base.strip_prefix("nss/").unwrap_or(base);
            let problem = if path.is_some() {
                // A path: the file (and block, if named) must exist.
                match (blocks.get(file), block) {
                    (None, _) => Some(format!("no file {file} (\"{s}\")")),
                    (Some(names), Some(b)) if !names.contains(b) => Some(format!("no scene/chapter {b} in {file}")),
                    _ => None,
                }
            } else {
                // A story file name for $GameName: both of its scenes.
                match blocks.get(file) {
                    None => Some(format!("no file {file} (\"{s}\")")),
                    Some(names) => [file.to_string(), format!("{file}_MAIN")]
                        .into_iter()
                        .find(|scene| !names.contains(scene.as_str()))
                        .map(|scene| format!("no scene {scene} in {file}")),
                }
            };
            if let Some(p) = problem {
                report.problems.push(format!("{name}:{line}: {p}"));
            }
        }
        for (line, callee, shapes, process) in &walk.calls {
            report.checked_calls += 1;
            let args = &shapes.len();
            match functions.get(callee.as_str()) {
                None if *process => report.problems.push(format!("{name}:{line}: no function {callee} for CreateProcess")),
                None => {
                    let b = report.builtins.entry(callee.clone()).or_default();
                    b.calls += 1;
                    *b.arities.entry(*args).or_default() += 1;
                    if b.args.len() < *args {
                        b.args.resize_with(*args, BTreeMap::new);
                    }
                    for (pos, s) in shapes.iter().enumerate() {
                        *b.args[pos].entry(s.clone()).or_default() += 1;
                    }
                }
                Some(defs) => {
                    // Trailing arguments may be left out (`FwC($画像, $回想判定)`
                    // is called 18,827 times with the image only).
                    if !*process && !defs.iter().any(|d| d.params >= *args) {
                        let want: Vec<String> = defs.iter().map(|d| d.params.to_string()).collect();
                        report.problems.push(format!(
                            "{name}:{line}: {callee} called with {args} arguments, defined with {}",
                            want.join(" / ")
                        ));
                    }
                }
            }
        }
    }
    report
}

/// The names called in `stmts`, in order (built-ins and functions alike).
pub fn calls_in(stmts: &[Stmt]) -> Vec<String> {
    let mut walk = Walk { strings: Vec::new(), calls: Vec::new(), targets: Vec::new() };
    walk.stmts(stmts);
    walk.calls.into_iter().map(|c| c.1).collect()
}

/// Collects what one file refers to.
struct Walk {
    /// (line, string literal)
    strings: Vec<(u32, String)>,
    /// (line, name, argument shapes, named by CreateProcess)
    calls: Vec<(u32, String, Vec<String>, bool)>,
    /// (line, written-out call_scene/call_chapter target, is call_chapter)
    targets: Vec<(u32, String, bool)>,
}

impl Walk {
    fn stmts(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        let line = s.line;
        match &s.kind {
            StmtKind::Expr(e) => self.expr(line, e),
            StmtKind::Assign { index, value, .. } => {
                for e in index.iter().chain(value) {
                    self.expr(line, e);
                }
            }
            StmtKind::If { cond, then, otherwise } => {
                self.expr(line, cond);
                self.stmts(then);
                if let Some(o) = otherwise {
                    self.stmts(o);
                }
            }
            StmtKind::While { cond, body } => {
                self.expr(line, cond);
                self.stmts(body);
            }
            StmtKind::Select(b) | StmtKind::Block(b) | StmtKind::Case { body: b, .. } => self.stmts(b),
            StmtKind::CallScene(Target::Name(t)) => self.targets.push((line, t.clone(), false)),
            StmtKind::CallChapter(Target::Name(t)) => self.targets.push((line, t.clone(), true)),
            StmtKind::Text(t) => {
                for item in &t.items {
                    if let Inline::Code(c) = item {
                        self.stmts(c);
                    }
                }
            }
            StmtKind::CallScene(Target::Var(_))
            | StmtKind::CallChapter(Target::Var(_))
            | StmtKind::Return
            | StmtKind::Break
            | StmtKind::Label(_)
            | StmtKind::Tag(_)
            | StmtKind::Invalid(_) => {}
        }
    }

    fn expr(&mut self, line: u32, e: &Expr) {
        match e {
            Expr::Str(s) => self.strings.push((line, s.clone())),
            Expr::Call { name, args } => {
                self.calls.push((line, name.clone(), args.iter().map(shape).collect(), false));
                // CreateProcess(name, priority, x, y, "function")
                if name == "CreateProcess"
                    && let Some(Expr::Str(f)) = args.get(4)
                {
                    self.calls.push((line, f.clone(), Vec::new(), true));
                }
                for a in args {
                    self.expr(line, a);
                }
            }
            Expr::Unary { expr, .. } => self.expr(line, expr),
            Expr::Binary { left, right, .. } => {
                self.expr(line, left);
                self.expr(line, right);
            }
            Expr::Index { base, index } => {
                self.expr(line, base);
                self.expr(line, index);
            }
            Expr::Point { x, y, .. } => {
                self.expr(line, x);
                self.expr(line, y);
            }
            Expr::Path(points) => {
                for p in points {
                    self.expr(line, p);
                }
            }
            Expr::Int(_) | Expr::Float(_) | Expr::Color(_) | Expr::Word(_) | Expr::Var(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nitroplus::nss::parse;

    fn corpus(files: &[(&str, &str)]) -> BTreeMap<String, NssFile> {
        files.iter().map(|(n, s)| (n.to_string(), parse(s).unwrap())).collect()
    }

    #[test]
    fn resolves_targets_names_and_calls() {
        let files = corpus(&[
            (
                "a.nss",
                "chapter main{ call_scene @->a.nss; call_chapter nss/b.nss; call_scene nss/b.nss->x; $GameName = \"b.nss\"; Helper(1, 2); Fade(\"x\", 300, @0, null, true); Fade(\"y\"); CreateProcess(\"p\", 1, 0, 0, \"Helper\"); }\nscene a.nss{}\nscene a.nss_MAIN{}",
            ),
            ("b.nss", "chapter main{}\nfunction Helper($a){}"),
        ]);
        let r = check(&files);
        assert_eq!(
            r.problems,
            vec![
                "a.nss:1: no scene/chapter x in b.nss",
                "a.nss:1: no scene b.nss in b.nss",
                "a.nss:1: Helper called with 2 arguments, defined with 1",
            ]
        );
        let fade = &r.builtins["Fade"];
        assert_eq!(fade.calls, 2);
        assert_eq!(fade.arities, BTreeMap::from([(1, 1), (5, 1)]));
        assert_eq!(fade.args[0], BTreeMap::from([("\"x\"".to_string(), 1), ("\"y\"".to_string(), 1)]));
        assert_eq!(fade.args[2], BTreeMap::from([("@0".to_string(), 1)]));
        assert!(!r.builtins.contains_key("Helper"));
    }
}
