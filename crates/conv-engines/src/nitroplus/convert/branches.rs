//! Forks: selects, `if`s on runtime story flags and the scenes they
//! lead to, and the story flags set and tested at run time.

use super::*;
use conv_core::ir::ChoicePlates;

/// Branches that end with different pictures on screen (the favour
/// screen adds a picture in one branch, and every sprite after it moves
/// up a layer): the code after them runs on all of them, so the next sync
/// removes every sprite a branch left that the merged state doesn't use,
/// and draws anew each picture some branch doesn't have where the merged
/// state has it.
pub(super) fn reconcile_shown(merged: &mut BTreeMap<String, Shown>, leaves: &[Leaf<'_>]) {
    let mut stale = BTreeMap::new();
    for l in leaves {
        for s in l.state.shown.values() {
            if !merged.values().any(|m| m.layer == s.layer) {
                stale.entry(s.layer).or_insert_with(|| s.clone());
            }
        }
    }
    for m in merged.values_mut() {
        if leaves.iter().any(|l| !l.state.shown.values().any(|s| s == m)) {
            // Not equal to any wanted picture: placed again.
            m.image.clear();
        }
    }
    for (layer, s) in stale {
        merged.insert(format!(" branch layer {layer}"), s);
    }
}

impl<'a> Compiler<'a, '_> {
    /// The game's subroutines ([`Game`]), each from `initial` (an empty
    /// screen) with the story flags as they are at run time; only reached
    /// by a gosub. They run in the middle of a file, so their helper
    /// variables come after every file's (`dynamic_base`), and the fork
    /// variables the file may still need are kept aside and put back.
    pub(super) fn subroutines(&mut self, initial: &State<'a>, dynamic_base: u16) {
        let local = |index| conv_core::ir::Var { global: false, index };
        let kept: Vec<u16> = (FORK_VARS.start..FORK_VARS.end).chain([LEAF_VAR]).collect();
        // Label, target, variables set.
        type Entry = (String, String, Vec<(String, Value)>);
        let mut list: Vec<Entry> = self.game.subroutines.iter().map(|&(l, t)| (l.to_string(), t.to_string(), Vec::new())).collect();
        for r in (self.game.replays)(self.files) {
            let mut set: Vec<(String, Value)> = r.set.into_iter().map(|(k, n)| (k, Value::Int(n))).collect();
            set.push(("$GameName".to_string(), Value::Str(r.file.clone())));
            list.push((r.label, format!("nss/{}->{}_MAIN", r.file, r.file), set));
        }
        for (label, target, set) in list {
            self.restore(initial.clone());
            for (k, &v) in &self.flags {
                self.vars.insert(k.clone(), Value::Runtime(v));
            }
            for (k, v) in set {
                self.vars.insert(k, v);
            }
            self.dynamic.clear();
            self.dynamic_values.clear();
            self.next_dynamic = dynamic_base + kept.len() as u16;
            self.steps = 0;
            self.forks_in_file = 0;
            self.leaves_in_file = 0;
            self.ops.push(Op::Label(label));
            for (i, &v) in kept.iter().enumerate() {
                self.ops.push(Op::Arith { var: local(dynamic_base + i as u16), op: ArithOp::Set, value: Operand::Var(local(v)) });
            }
            self.call_scene(&target, true);
            self.sync(0);
            for (i, &v) in kept.iter().enumerate() {
                self.ops.push(Op::Arith { var: local(v), op: ArithOp::Set, value: Operand::Var(local(dynamic_base + i as u16)) });
            }
            self.ops.push(Op::Return);
        }
    }

    pub(super) fn snapshot(&self) -> State<'a> {
        State {
            vars: self.vars.clone(),
            objects: self.objects.clone(),
            aliases: self.aliases.clone(),
            seq: self.seq,
            shown: self.shown.clone(),
            font_size: self.font_size,
            window: self.window.clone(),
            window_visible: self.window_visible,
        }
    }

    pub(super) fn restore(&mut self, s: State<'a>) {
        self.vars = s.vars;
        self.objects = s.objects;
        self.aliases = s.aliases;
        self.seq = s.seq;
        self.shown = s.shown;
        self.font_size = s.font_size;
        self.window = s.window;
        self.window_visible = s.window_visible;
        self.pending_ms = 0;
        self.pending_mask = None;
    }

    /// The `case`s of a select body, also inside `if`s (options shown
    /// only after a clear: the condition is taken as it is now).
    /// `decided`: runtime `if`s already split on (by statement), and the
    /// branch taken. Returns the first runtime `if` holding cases that
    /// isn't decided yet.
    pub(super) fn collect_cases(&mut self, stmts: &'a [Stmt], out: &mut Vec<&'a [Stmt]>, decided: &[(*const Stmt, bool)]) -> Option<&'a Stmt> {
        let mut open = None;
        for s in stmts {
            match &s.kind {
                StmtKind::Case { body, .. } => out.push(body),
                StmtKind::Block(b) => open = open.or(self.collect_cases(b, out, decided)),
                StmtKind::If { cond, then, otherwise } => {
                    let has_cases = |b: &[Stmt]| b.iter().any(|s| matches!(s.kind, StmtKind::Case { .. }));
                    let taken = match decided.iter().find(|d| std::ptr::eq(d.0, s)) {
                        Some(&(_, t)) => t,
                        None if self.is_runtime(cond) && (has_cases(then) || otherwise.as_deref().is_some_and(has_cases)) => {
                            open = open.or(Some(s));
                            continue;
                        }
                        None => self.eval(cond).truthy(),
                    };
                    if taken {
                        open = open.or(self.collect_cases(then, out, decided));
                    } else if let Some(o) = otherwise {
                        open = open.or(self.collect_cases(o, out, decided));
                    }
                }
                _ => {}
            }
        }
        open
    }

    /// `select { … case @x { … } … }`: the player picks a case. Each case
    /// is compiled from the state before the choice; the code after the
    /// choice goes on in the first case's state, and the run forks at the
    /// end of the file (see [`compile_run`]). The statements before the
    /// first `case` are the menu's input loop and are left out.
    pub(super) fn select(&mut self, body: &'a [Stmt]) {
        self.select_variant(body, &mut Vec::new());
    }

    /// A select whose options depend on runtime flags (an option shown
    /// only after a clear): one NScripter select per variant, behind the
    /// runtime test.
    pub(super) fn select_variant(&mut self, body: &'a [Stmt], decided: &mut Vec<(*const Stmt, bool)>) {
        let mut cases = Vec::new();
        if let Some(split) = self.collect_cases(body, &mut cases, decided) {
            let StmtKind::If { cond, .. } = &split.kind else { unreachable!() };
            self.sync(0);
            let own = self.begin_fork();
            self.choices += 1;
            let (else_label, end_label) = (format!("sel{}_else", self.choices), format!("sel{}_end", self.choices));
            self.jump_if(cond, false, &else_label);
            let base = self.snapshot();
            let mut leaves = Vec::new();
            for taken in [true, false] {
                if !taken {
                    self.ops.push(Op::Jump(end_label.clone()));
                    self.ops.push(Op::Label(else_label.clone()));
                }
                decided.push((split as *const Stmt, taken));
                leaves.extend(self.branch(&base, |c| c.select_variant(body, decided)));
                decided.pop();
            }
            self.ops.push(Op::Label(end_label));
            self.finish_fork(leaves, own, true);
            return;
        }
        if cases.is_empty() {
            self.note("select without cases");
            return;
        }
        self.sync(0);
        let own = self.begin_fork();
        let n = self.choices;
        self.choices += 1;
        // SetChoice02…06 keep the button texts in these parameters; a
        // story flag's ([`choice_flags`]) is picked at run time from the
        // strings it has been given.
        let mut runtime = Vec::new();
        let texts: Vec<String> = (1..=cases.len())
            .map(|i| {
                let digit = char::from_u32('０' as u32 + i as u32).unwrap_or('?');
                let v = self.vars.get(&format!("$テキストデータ{digit}"));
                let known = match v {
                    Some(&Value::Runtime(var)) => self.flag_texts.get(&var).filter(|t| !t.is_empty()).map(|t| (var, t)),
                    _ => None,
                };
                runtime.push(known.map(|(var, t)| conv_core::ir::RuntimeText { var, texts: t.iter().map(|(&c, s)| (c, s.clone())).collect() }));
                match known {
                    Some((_, t)) => t.values().next().cloned().unwrap_or_default(),
                    None => v.map(Value::string).filter(|t| !t.is_empty()).unwrap_or_else(|| format!("選択肢{i}")),
                }
            })
            .collect();
        let labels: Vec<String> = (0..cases.len()).map(|i| format!("choice{n}_{i}")).collect();
        let after = format!("choice{n}_end");
        let style = (self.game.choice_style)(self, texts.len() as i64);
        let style_shown = style.is_some();
        let options = texts.into_iter().zip(labels.iter().cloned()).collect();
        self.ops.push(match runtime.iter().any(Option::is_some) {
            true => Op::RuntimeChoice { options, texts: runtime, style },
            false => Op::Choice { options, style, disabled: vec![] },
        });
        if style_shown {
            // The next text sets its own window again.
            self.window = None;
        }
        let base = self.snapshot();
        let mut leaves = Vec::new();
        for (case, label) in cases.iter().zip(&labels) {
            self.ops.push(Op::Label(label.clone()));
            leaves.extend(self.branch(&base, |c| {
                c.exec_block(case);
            }));
            self.ops.push(Op::Jump(after.clone()));
        }
        self.ops.push(Op::Label(after));
        self.finish_fork(leaves, own, true);
    }

    /// 装甲悪鬼村正's choice plates of `SetChoiceBase` (function_select.nss):
    /// `n` vertical plates 100 px apart, the first at the right, centred on
    /// the screen; the options in black, vertical, one per plate. The plate
    /// under the pointer is Select002; the chosen one Select003 for a
    /// second (ChoiceFade), then all fade out.
    pub(super) fn vertical_choice_style(&mut self, n: i64) -> Option<ChoiceStyle> {
        // src: function_select.nss SetChoiceBase ($SelectX_plus 100, + 50, SetFont 20 #000000,
        // MouseUsual Select001, MouseOver Select002, Fade 500); ChoiceFade (img2 Select003, Wait 1000, Fade 500)
        // ours: MARGIN, the text 40 px down its plate: the original's is a child of the plate at its
        // origin (CreateText inherit, auto size, font "@ＭＳ 明朝"), Rotate z 90, Move x -11; where
        // the exe's auto size and rotation put it is not decoded
        const PITCH: i64 = 100;
        const FONT: i64 = 20;
        const MARGIN: i64 = 40;
        const PLATES: [&str; 3] = ["cg/sys/select/select001.png", "cg/sys/select/select002.png", "cg/sys/select/select003.png"];
        let (plate, w, h) = self.assets.image(PLATES[0])?;
        let (w, h) = (w as i64, h as i64);
        let first = PITCH * (n - 2) + 50 + self.game.screen.0 / 2;
        let y = (self.game.screen.1 - h) / 2;
        let parts: Vec<_> = (0..n).map(|i| (plate.clone(), first - PITCH * i, y, 1000)).collect();
        let (image, wx, wy) = self.assets.composite(&parts)?;
        let text_x = first + (w - FONT) / 2;
        let window = TextWindow {
            wait_icon_at: None,
            image,
            x: wx as i32,
            y: wy as i32,
            text_x: (text_x - PITCH * (n - 1)) as i32,
            text_y: (y + MARGIN) as i32,
            columns: ((h - 2 * MARGIN) / FONT) as u32,
            rows: n as u32,
            font_size: FONT as u32,
            line_gap: (PITCH - FONT) as u32,
            vertical: true,
            name_plate: None,
        };
        let (image, _, _) = self.assets.strip(&PLATES)?;
        let plates = ChoicePlates {
            image,
            x: first as i32,
            y: y as i32,
            step: PITCH as i32,
            text_x: text_x as i32,
            text_dy: MARGIN as i32,
            font_size: FONT as u32,
            colour: "#000000".into(),
            fade_ms: 500,
            shot: None,
            vertical: true,
            chosen_ms: 1000,
        };
        Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) })
    }

    /// Starts a fork; true if it's the outermost one (nested forks share
    /// its variable).
    pub(super) fn begin_fork(&mut self) -> bool {
        if self.fork_var.is_some() {
            return false;
        }
        let index = FORK_VARS.start + self.forks_in_file % (FORK_VARS.end - FORK_VARS.start);
        self.forks_in_file += 1;
        self.fork_var = Some(conv_core::ir::Var { global: false, index });
        true
    }

    /// Compiles one branch of a fork from `base`: its leaves (one, or those
    /// of the forks inside it, each with the branch's later changes).
    pub(super) fn branch(&mut self, base: &State<'a>, body: impl FnOnce(&mut Self)) -> Vec<Leaf<'a>> {
        self.restore(base.clone());
        let outer = self.pending_fork.take();
        body(self);
        self.sync(0);
        let end = self.snapshot();
        let leaves = match self.pending_fork.take() {
            None => {
                let marker = self.ops.len();
                let var = self.fork_var.unwrap();
                self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(0) });
                self.leaves_in_file += 1;
                let id = self.leaves_in_file;
                self.ops.push(Op::Arith { var: conv_core::ir::Var { global: false, index: LEAF_VAR }, op: ArithOp::Set, value: Operand::Const(id) });
                vec![Leaf { state: end, marker, id }]
            }
            Some((_, inner, first)) => {
                inner.into_iter().map(|l| Leaf { state: with_branch_vars(&end, &l.state, &first), marker: l.marker, id: l.id }).collect()
            }
        };
        self.pending_fork = outer;
        leaves
    }

    /// Numbers a fork's leaves. Where they lead to different files (or it's
    /// a choice), the file's end dispatches on them; otherwise the branches
    /// merge here, and story flags that differ become unknown.
    pub(super) fn finish_fork(&mut self, leaves: Vec<Leaf<'a>>, own: bool, choice: bool) {
        let var = self.fork_var.unwrap();
        for (i, l) in leaves.iter().enumerate() {
            self.ops[l.marker] = Op::Arith { var, op: ArithOp::Set, value: Operand::Const(i as i32) };
        }
        if own {
            self.fork_var = None;
        }
        let differ = |k: &str| leaves.iter().any(|l| l.state.vars.get(k) != leaves[0].state.vars.get(k));
        if !own || choice || differ("$GameName") || differ("$GameCircle") {
            if own && self.pending_fork.is_some() {
                self.note("second fork in one file (the run forks on the last one)");
            }
            self.restore(leaves[0].state.clone());
            let mut first = leaves[0].state.clone();
            // The code after the fork runs on every branch: story flags the
            // branches set differently are tested at run time, and so are
            // names of scenes or files they set differently (the code after
            // a choice may copy `$GameName` into a story flag). Where that
            // code leaves them alone, each branch still goes on with its own
            // value (`first` holds the runtime ones). In a whole story
            // ([`compile_story`]) a fork inside another too: its choices
            // are inside the runtime ifs of the deaths before them.
            // A choice's flags too: its options are drawn at run time.
            {
                for (k, &v) in &self.flags {
                    if differ(k) && (own || self.whole_story || (self.choice_reads.contains(k) && k.starts_with('$'))) {
                        self.vars.insert(k.clone(), Value::Runtime(v));
                    }
                }
            }
            let keys: std::collections::BTreeSet<String> = leaves.iter().flat_map(|l| l.state.vars.keys().cloned()).collect();
            let helpers: Vec<&String> = keys.iter().filter(|k| differ(k) && !self.flags.contains_key(*k) && !k.starts_with("$SYSTEM")).collect();
            for k in helpers {
                if let Some(v) = one_runtime(k, &leaves) {
                    self.vars.insert(k.clone(), Value::Runtime(v));
                    first.vars.insert(k.clone(), Value::Runtime(v));
                    continue;
                }
                let Some((var, codes)) = self.dynamic_names(k, &leaves) else { continue };
                // Set here from the way the run came (inner forks aren't
                // numbered for good yet, so not at the markers).
                let leaf = conv_core::ir::Var { global: false, index: LEAF_VAR };
                for (i, code) in codes {
                    self.choices += 1;
                    let skip = format!("leaf{}", self.choices);
                    self.ops.push(Op::If { var: leaf, cmp: conv_core::ir::Compare::Ne, value: Operand::Const(leaves[i].id), label: skip.clone() });
                    self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(code) });
                    self.ops.push(Op::Label(skip));
                }
                self.vars.insert(k.clone(), Value::Runtime(var));
                first.vars.insert(k.clone(), Value::Runtime(var));
            }
            self.pending_fork = Some((var, leaves, first));
            return;
        }
        let mut merged = leaves[0].state.clone();
        let keys: std::collections::BTreeSet<String> = leaves.iter().flat_map(|l| l.state.vars.keys().cloned()).collect();
        let mut inserts = Vec::new();
        for k in keys.iter().filter(|k| differ(k)) {
            match self.flags.get(k) {
                Some(&v) => {
                    merged.vars.insert(k.clone(), Value::Runtime(v));
                }
                None if k.starts_with("$SYSTEM") => {}
                None if one_runtime(k, &leaves).is_some() => {
                    merged.vars.insert(k.clone(), Value::Runtime(one_runtime(k, &leaves).unwrap()));
                }
                // Different names (of a scene to call, say): kept at run time
                // from here on, set at the end of each branch.
                None => match self.dynamic_names(k, &leaves) {
                    Some((var, codes)) => {
                        inserts.extend(codes.into_iter().map(|(i, code)| (leaves[i].marker, Op::Arith { var, op: ArithOp::Set, value: Operand::Const(code) })));
                        merged.vars.insert(k.clone(), Value::Runtime(var));
                    }
                    None => self.note("branches of a runtime if end with different helper variables"),
                },
            }
        }
        // After each marker, last first so the earlier positions hold.
        inserts.sort_by_key(|i| std::cmp::Reverse(i.0));
        for (at, op) in inserts {
            self.insert_op(at + 1, op);
        }
        if leaves.iter().any(|l| l.state.shown != leaves[0].state.shown) {
            self.note("branches of a runtime if end with different pictures");
            reconcile_shown(&mut merged.shown, &leaves);
        }
        self.restore(merged);
    }

    /// A helper variable the leaves of a fork set to different names of
    /// scenes or files to go to (`@->x_SELECT02`, `y.nss`): a runtime
    /// variable from here on, with the code each leaf sets it to (leaves
    /// that already hold it at run time left out). None for other values.
    pub(super) fn dynamic_names(&mut self, k: &str, leaves: &[Leaf<'a>]) -> Option<(conv_core::ir::Var, Vec<(usize, i32)>)> {
        let known = self.dynamic.get(k).copied();
        let scene = |v: Option<&Value>| match v {
            Some(Value::Str(t)) => t.starts_with("@->") || t.starts_with("nss/") || t.contains(".nss"),
            Some(Value::Null) | None => true,
            Some(&Value::Runtime(r)) => Some(r) == known,
            _ => false,
        };
        let strings = leaves.iter().all(|l| scene(l.state.vars.get(k))) && leaves.iter().any(|l| matches!(l.state.vars.get(k), Some(Value::Str(_))));
        if !strings || (known.is_none() && self.next_dynamic >= CHOICE_FLAGS.start) {
            return None;
        }
        let var = known.unwrap_or_else(|| {
            let var = conv_core::ir::Var { global: false, index: self.next_dynamic };
            self.next_dynamic += 1;
            self.dynamic.insert(k.to_string(), var);
            var
        });
        let mut codes = Vec::new();
        for (i, l) in leaves.iter().enumerate() {
            let v = l.state.vars.get(k).cloned().unwrap_or(Value::Null);
            if matches!(v, Value::Runtime(_)) {
                continue;
            }
            codes.push((i, self.code(&v).unwrap_or(0)));
            self.dynamic_values.entry(var).or_default().insert(v.string());
        }
        Some((var, codes))
    }

    /// `if` on a story flag only the running game knows: both branches are
    /// compiled, behind a runtime test.
    pub(super) fn runtime_if(&mut self, cond: &'a Expr, then: &'a [Stmt], otherwise: Option<&'a [Stmt]>) {
        self.runtime_if_then(cond, then, otherwise, &[]);
    }

    /// [`Compiler::runtime_if`] followed by `rest`, which runs at the end of
    /// each branch that doesn't end in `return`.
    pub(super) fn runtime_if_then(&mut self, cond: &'a Expr, then: &'a [Stmt], otherwise: Option<&'a [Stmt]>, rest: &[&'a [Stmt]]) {
        let with_rest = |branch: &'a [Stmt]| {
            let mut parts = vec![branch];
            if !ends_in_return(branch) {
                parts.extend_from_slice(rest);
            }
            parts
        };
        let (then_parts, else_parts) = (with_rest(then), with_rest(otherwise.unwrap_or(&[])));
        let returned = self.returned.take().filter(|r| r.0 == cond);
        self.sync(0);
        let own = self.begin_fork();
        let n = self.choices;
        self.choices += 1;
        let (else_label, end_label) = (format!("if{n}_else"), format!("if{n}_end"));
        match self.fast_skip_read.take() {
            Some(read) => self.jump_unless_fast_skip(read, &else_label),
            None => self.jump_if(cond, false, &else_label),
        }
        let base = self.snapshot();
        let (then_base, stops) = match returned {
            Some((_, state, stops)) => (state, stops),
            None => (base.clone(), Vec::new()),
        };
        let mut leaves = self.branch(&then_base, |c| {
            c.ops.extend(stops);
            c.exec_seq(&then_parts);
        });
        self.ops.push(Op::Jump(end_label.clone()));
        self.ops.push(Op::Label(else_label));
        let else_leaves = self.branch(&base, |c| {
            c.exec_seq(&else_parts);
        });
        // A branch that returned left the scene: the code after the if
        // goes on from the other one (its pictures, its variables).
        match (ends_in_return(then), otherwise.is_some_and(ends_in_return)) {
            (true, false) => {
                if let [leaf] = &leaves[..] {
                    self.returned = Some((cond, leaf.state.clone(), Vec::new()));
                }
                leaves = else_leaves;
            }
            (false, true) => {}
            _ => leaves.extend(else_leaves),
        }
        self.ops.push(Op::Label(end_label));
        self.finish_fork(leaves, own, false);
    }

    /// `call_scene $x` where `$x` is only known at run time: each scene it
    /// can name, behind a runtime test.
    pub(super) fn call_runtime_scene(&mut self, var: conv_core::ir::Var, chapter: bool) {
        let values: Vec<String> = self.dynamic_values.get(&var).map(|v| v.iter().cloned().collect()).unwrap_or_default();
        if values.is_empty() {
            self.note("call_scene to a runtime value with no known names");
            return;
        }
        self.sync(0);
        let own = self.begin_fork();
        self.choices += 1;
        let n = self.choices;
        let end = format!("callrt{n}_end");
        let base = self.snapshot();
        let mut leaves = Vec::new();
        for (i, v) in values.iter().enumerate() {
            let next = format!("callrt{n}_{i}");
            let code = self.code(&Value::Str(v.clone())).unwrap_or(0);
            self.ops.push(Op::If { var, cmp: conv_core::ir::Compare::Ne, value: Operand::Const(code), label: next.clone() });
            leaves.extend(self.branch(&base, |c| c.call_scene(v, chapter)));
            self.ops.push(Op::Jump(end.clone()));
            self.ops.push(Op::Label(next));
        }
        self.ops.push(Op::Label(end));
        self.finish_fork(leaves, own, false);
    }

    /// [`add`], where a string takes a choice's runtime text as a mark
    /// ([`RUNTIME_MARK`]) instead of nothing.
    pub(super) fn add_texts(&self, l: &Value, r: &Value) -> Value {
        if !matches!((l, r), (Value::Str(_), _) | (_, Value::Str(_))) {
            return add(l, r);
        }
        let part = |v: &Value| match v {
            &Value::Runtime(var) if self.choice_reads.iter().any(|k| self.flags.get(k) == Some(&var)) => {
                format!("{}{}{}{}", RUNTIME_MARK.0, if var.global { "g" } else { "" }, var.index, RUNTIME_MARK.1)
            }
            v => v.string(),
        };
        Value::Str(part(l) + &part(r))
    }

    /// Whether `e` reads a story flag whose value isn't known now.
    pub(super) fn is_runtime(&mut self, e: &'a Expr) -> bool {
        match e {
            Expr::Var(v) => matches!(self.vars.get(&v.to_string()), Some(Value::Runtime(_))),
            Expr::Index { .. } => {
                let key = self.index_key(e);
                matches!(self.vars.get(&key), Some(Value::Runtime(_)))
            }
            Expr::Unary { expr, .. } => self.is_runtime(expr),
            Expr::Binary { left, right, .. } => self.is_runtime(left) || self.is_runtime(right),
            Expr::Call { name, args } if name == "VariableValue" && args.len() == 2 => {
                let key = self.variable_value_key(args);
                key.is_some_and(|k| matches!(self.vars.get(&k), Some(Value::Runtime(_))))
            }
            _ => false,
        }
    }

    /// The number a known value is stored as in a flag variable.
    pub(super) fn code(&mut self, v: &Value) -> Option<i32> {
        Some(match v {
            Value::Null => 0,
            Value::Int(n) | Value::Rel(n) => *n as i32,
            Value::Str(s) if s.is_empty() => 0,
            Value::Str(s) => match s.trim().parse::<i32>() {
                Ok(n) => n,
                Err(_) => {
                    let next = self.codes.values().filter(|&&c| c < CHOICE_CODES).count() as i32 + 1;
                    *self.codes.entry(s.clone()).or_insert(next)
                }
            },
            Value::Runtime(_) => return None,
        })
    }

    /// A new string's code when a flag only a choice reads ([`CHOICE_FLAGS`])
    /// is the first to hold it: numbered apart, so the other codes stay.
    /// Its text is kept for the choice ([`Compiler::flag_texts`]).
    pub(super) fn choice_code(&mut self, var: conv_core::ir::Var, v: &Value) -> Option<i32> {
        let Value::Str(s) = v else { return None };
        if s.is_empty() || s.trim().parse::<i32>().is_ok() {
            return None;
        }
        if !var.global && CHOICE_FLAGS.contains(&var.index) && !self.codes.contains_key(s) {
            let next = CHOICE_CODES + self.codes.values().filter(|&&c| c >= CHOICE_CODES).count() as i32;
            self.codes.insert(s.clone(), next);
        }
        let code = self.code(v)?;
        self.flag_texts.entry(var).or_default().insert(code, s.clone());
        Some(code)
    }

    /// Emits a jump to `label` taken when `e` is `want` (true or false).
    pub(super) fn jump_if(&mut self, e: &'a Expr, want: bool, label: &str) {
        use conv_core::ir::Compare;
        let fresh = |c: &mut Self| {
            c.choices += 1;
            format!("cond{}", c.choices)
        };
        match e {
            Expr::Binary { op: BinaryOp::And, left, right } | Expr::Binary { op: BinaryOp::Or, left, right } => {
                let and = matches!(e, Expr::Binary { op: BinaryOp::And, .. });
                if and != want {
                    // a && b false: either is false; a || b true: either is true.
                    self.jump_if(left, want, label);
                    self.jump_if(right, want, label);
                } else {
                    let skip = fresh(self);
                    self.jump_if(left, !want, &skip);
                    self.jump_if(right, want, label);
                    self.ops.push(Op::Label(skip));
                }
            }
            Expr::Unary { op: UnaryOp::Not, expr } => self.jump_if(expr, !want, label),
            Expr::Binary { op, left, right } if matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge) => {
                let (l, r) = (self.eval(left), self.eval(right));
                let cmp = match op {
                    BinaryOp::Eq => Compare::Eq,
                    BinaryOp::Ne => Compare::Ne,
                    BinaryOp::Lt => Compare::Lt,
                    BinaryOp::Gt => Compare::Gt,
                    BinaryOp::Le => Compare::Le,
                    _ => Compare::Ge,
                };
                let (var, cmp, value) = match (&l, &r) {
                    (Value::Runtime(v), k) if !matches!(k, Value::Runtime(_)) => (*v, cmp, k.clone()),
                    // k < var is var > k.
                    (k, Value::Runtime(v)) if !matches!(k, Value::Runtime(_)) => {
                        let mirrored = match cmp {
                            Compare::Lt => Compare::Gt,
                            Compare::Gt => Compare::Lt,
                            Compare::Le => Compare::Ge,
                            Compare::Ge => Compare::Le,
                            c => c,
                        };
                        (*v, mirrored, k.clone())
                    }
                    (Value::Runtime(a), Value::Runtime(b)) => {
                        let cmp = if want { cmp } else { negate(cmp) };
                        self.ops.push(Op::If { var: *a, cmp, value: Operand::Var(*b), label: label.to_string() });
                        return;
                    }
                    _ => {
                        let truth = self.eval(e).truthy();
                        if truth == want {
                            self.ops.push(Op::Jump(label.to_string()));
                        }
                        return;
                    }
                };
                let cmp = if want { cmp } else { negate(cmp) };
                let value = Operand::Const(self.code(&value).unwrap_or(0));
                self.ops.push(Op::If { var, cmp, value, label: label.to_string() });
            }
            _ => match self.eval(e) {
                Value::Runtime(var) => {
                    let cmp = if want { Compare::Ne } else { Compare::Eq };
                    self.ops.push(Op::If { var, cmp, value: Operand::Const(0), label: label.to_string() });
                }
                v => {
                    if self.is_runtime(e) {
                        self.note("runtime if on an expression (taken as false)");
                    }
                    if v.truthy() == want {
                        self.ops.push(Op::Jump(label.to_string()));
                    }
                }
            },
        }
    }

    /// An assignment to a story flag: also done at run time.
    pub(super) fn assign_flag(&mut self, var: conv_core::ir::Var, op: AssignOp, old: &Value, value: &Value, new: &Value) {
        match (op, old, value) {
            (AssignOp::Add | AssignOp::Sub, Value::Runtime(_), v) | (AssignOp::Inc | AssignOp::Dec, Value::Runtime(_), v) if !matches!(v, Value::Runtime(_)) => {
                let n = match op {
                    AssignOp::Inc | AssignOp::Dec => 1,
                    _ => v.int() as i32,
                };
                let ir_op = if matches!(op, AssignOp::Add | AssignOp::Inc) { ArithOp::Add } else { ArithOp::Sub };
                self.ops.push(Op::Arith { var, op: ir_op, value: Operand::Const(n) });
            }
            (AssignOp::Set, _, Value::Runtime(other)) => {
                self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Var(*other) });
            }
            _ => match self.choice_code(var, new).or_else(|| self.code(new)) {
                Some(n) => self.ops.push(Op::Arith { var, op: ArithOp::Set, value: Operand::Const(n) }),
                None => self.note("story flag set from a runtime expression"),
            },
        }
    }
}
