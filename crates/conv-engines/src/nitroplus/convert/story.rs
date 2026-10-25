//! Entry points: the boot script, a scene, the story, a run of story
//! files; and the story flags and library files they read.

use super::*;

/// The label of the [`Game::boot_logos`] subroutine ([`conv_core::ir::BootStep::gosub`]).
pub const BOOT_LABEL: &str = "conv_boot";

/// The label of the [`Game::title_opening`] subroutine.
pub const OPENING_LABEL: &str = "conv_title_script";

/// The label of the [`Game::title_opening`] subroutine on coming back to the title.
pub const OPENING_BACK_LABEL: &str = "conv_title_script_back";

/// Compiles the game's [`Game::boot_logos`] into a subroutine at
/// [`BOOT_LABEL`], jumped over, from the story's `initial` state (the
/// state now put back after it). At the end of the ops, so the story's
/// own output stays as it is.
pub(super) fn compile_boot<'a>(c: &mut Compiler<'a, '_>, files: &'a BTreeMap<String, NssFile>, game: &'static Game, initial: &State<'a>) {
    let (file, source) = game.boot_logos;
    if source.is_empty() {
        return;
    }
    let Some((boot, _)) = files.get_key_value(file) else {
        c.note(format!("boot_logos: no {file}"));
        return;
    };
    // Leaked once a build: the compiler borrows what it runs for its lifetime.
    let src: &'static NssFile = Box::leak(Box::new(crate::nitroplus::nss::parse(source).expect("boot_logos parses")));
    let Some(f) = src.items.iter().find_map(|item| match item {
        Item::Function(f) => Some(f),
        _ => None,
    }) else {
        return;
    };
    let before = c.snapshot();
    c.restore(initial.clone());
    // Not conv-cons's `conv_boot_end` (its boot guard): one label each.
    let end = "conv_boot_body_end";
    c.ops.push(Op::Jump(end.to_string()));
    c.ops.push(Op::Label(BOOT_LABEL.to_string()));
    c.call_function(boot, f, Vec::new());
    c.sync(0);
    c.ops.push(Op::Return);
    // Compiled from where the logos leave things, as it runs after them.
    let (function, flag, back) = game.title_opening;
    let opening = files[boot].items.iter().find_map(|item| match item {
        Item::Function(f) if f.name == function => Some(f),
        _ => None,
    });
    if let Some(f) = opening {
        let after_logos = c.snapshot();
        let labels = if back.is_empty() { &[OPENING_LABEL][..] } else { &[OPENING_LABEL, OPENING_BACK_LABEL] };
        for (label, v) in labels.iter().flat_map(|l| (if flag.is_empty() { 0..1 } else { 0..2 }).map(move |v| (*l, v))) {
            c.restore(after_logos.clone());
            if !flag.is_empty() {
                c.vars.insert(flag.to_string(), Value::Int(v));
            }
            if label == OPENING_BACK_LABEL {
                c.vars.insert(back.to_string(), Value::Int(1));
            }
            let suffix = if v == 0 { String::new() } else { v.to_string() };
            c.ops.push(Op::Label(format!("{label}{suffix}")));
            c.call_function(boot, f, Vec::new());
            c.sync(0);
            c.ops.push(Op::Return);
        }
    }
    c.ops.push(Op::Label(end.to_string()));
    c.restore(before);
}

/// Compiles scene `scene` of `file` (`ma01_001.nss`, `ma01_001.nss`).
pub fn compile_scene(files: &BTreeMap<String, NssFile>, file: &str, scene: &str, game: &'static Game, assets: &mut dyn Assets) -> Result<Compiled, String> {
    let (file, _) = files.get_key_value(file).ok_or_else(|| format!("no script {file}"))?;
    let mut c = Compiler::new(files, file, game, assets);
    let body = c.scene(file, scene).ok_or_else(|| format!("no scene {scene} in {file}"))?;
    let initial = c.snapshot();
    c.run_top(body);
    c.sync(0);
    let end = "conv_scene_end";
    c.ops.push(Op::Jump(end.to_string()));
    let base = c.next_dynamic.max(FLAG_LOCAL_BASE);
    c.subroutines(&initial, base);
    c.ops.push(Op::Label(end.to_string()));
    Ok(Compiled { ops: c.ops, window: c.first_window, notes: c.notes, metrics: c.metrics, flags: c.flags, voice_classes: c.voice_classes, source_map: c.source_map })
}

/// Compiles a whole story that one scene drives by calling the chapter
/// files in turn (続・殺戮のジャンゴ's `dja000_開始スクリプト.nss`,
/// `main`): [`compile_scene`] with the story flags kept at run time, so a
/// choice's flag decides what follows it when the game is played.
pub fn compile_story(files: &BTreeMap<String, NssFile>, file: &str, scene: &str, game: &'static Game, assets: &mut dyn Assets) -> Result<Compiled, String> {
    let (file, _) = files.get_key_value(file).ok_or_else(|| format!("no script {file}"))?;
    let mut c = Compiler::new(files, file, game, assets);
    c.whole_story = true;
    c.flags = story_flags(files, game);
    for (k, &v) in &c.flags {
        c.vars.insert(k.clone(), Value::Runtime(v));
    }
    c.next_dynamic = c.flags.values().filter(|v| !v.global && v.index < CHOICE_FLAGS.start).map(|v| v.index + 1).max().unwrap_or(FLAG_LOCAL_BASE);
    let body = c.scene(file, scene).ok_or_else(|| format!("no scene {scene} in {file}"))?;
    let initial = c.snapshot();
    c.run_top(body);
    c.sync(0);
    c.ops.push(Op::Jump(RUN_END.to_string()));
    let base = c.next_dynamic;
    c.subroutines(&initial, base);
    compile_boot(&mut c, files, game, &initial);
    // The story ends back at the title, as the boot script's loop does.
    c.ops.push(Op::Label(RUN_END.to_string()));
    if !game.story_end.is_empty() {
        c.restore(initial.clone());
        for (k, &v) in &c.flags {
            c.vars.insert(k.clone(), Value::Runtime(v));
        }
    }
    for name in game.story_end {
        let in_boot = files.get_key_value(game.boot).and_then(|(boot, nss)| {
            nss.items.iter().find_map(|item| match item {
                Item::Function(f) if f.name == *name => Some((boot.as_str(), f)),
                _ => None,
            })
        });
        match in_boot.or_else(|| c.function(name)) {
            Some((file, f)) => {
                c.call_function(file, f, Vec::new());
            }
            None => c.note(format!("story_end: no function {name}")),
        }
    }
    c.sync(0);
    c.ops.push(Op::ReturnToTitle);
    Ok(Compiled { ops: c.ops, window: c.first_window, notes: c.notes, metrics: c.metrics, flags: c.flags, voice_classes: c.voice_classes, source_map: c.source_map })
}

/// Plays story files from `start` (`ma01_001.nss`), as the story
/// driver's loop does: each file's `X.nss_MAIN` scene runs its content
/// scene and sets `$GameName` to the next file. A choice forks the run:
/// the next file of each branch is compiled from that branch's state, and
/// the end of the file jumps to it by the choice the player made. Each
/// file starts at a label named after it and is compiled once (from the
/// first state that reaches it). Stops after `max_files` files. Returns
/// the files compiled.
pub fn compile_run(files: &BTreeMap<String, NssFile>, start: &str, max_files: usize, game: &'static Game, assets: &mut dyn Assets) -> Result<(Compiled, Vec<String>), String> {
    let (first, _) = files.get_key_value(start).ok_or_else(|| format!("no script {start}"))?;
    let mut c = Compiler::new(files, first, game, assets);
    c.flags = story_flags(files, game);
    let dynamic_base = c.flags.values().filter(|v| !v.global && v.index < CHOICE_FLAGS.start).map(|v| v.index + 1).max().unwrap_or(FLAG_LOCAL_BASE);
    c.vars.insert("$GameName".into(), Value::Str(start.to_string()));
    c.vars.insert("$GameStart".into(), Value::Int(1));
    c.vars.insert("$GameCircle".into(), Value::Int(1));
    let initial = c.snapshot();
    let mut max_dynamic = dynamic_base;
    let mut queue = std::collections::VecDeque::from([(start.to_string(), c.snapshot())]);
    let mut done = std::collections::HashSet::new();
    let mut not_compiled = std::collections::BTreeSet::new();
    let mut played = Vec::new();
    // Ends of files whose next file is in a story flag: label, flag, state,
    // file names dispatched so far.
    let mut dispatches: Vec<(String, conv_core::ir::Var, State, std::collections::BTreeSet<String>)> = Vec::new();
    // The variables each file was compiled with, to report merges of paths
    // that differ.
    let mut entry_vars: HashMap<String, HashMap<String, Value>> = HashMap::new();
    loop {
        let Some((name, state)) = queue.pop_front() else {
            // Queue the file names the dispatched flags can hold by now, in
            // the order they were met (the output stays the same each run).
            let mut names: Vec<(i32, String)> = c.codes.iter().filter(|(k, _)| k.ends_with(".nss") && files.contains_key(*k)).map(|(k, &v)| (v, k.clone())).collect();
            names.sort();
            let names: Vec<String> = names.into_iter().map(|(_, k)| k).collect();
            for (_, _, state, seen) in &mut dispatches {
                for n in &names {
                    if seen.insert(n.clone()) {
                        let mut s = state.clone();
                        s.vars.insert("$GameName".into(), Value::Str(n.clone()));
                        queue.push_back((n.clone(), s));
                    }
                }
            }
            if queue.is_empty() {
                break;
            }
            continue;
        };
        if let Some(first) = entry_vars.get(&name) {
            let mut differ: Vec<&String> = first
                .keys()
                .chain(state.vars.keys())
                .filter(|k| !k.starts_with("$SYSTEM") && !k.contains("GameName") && first.get(*k) != state.vars.get(*k))
                .collect();
            differ.sort();
            differ.dedup();
            for k in differ {
                c.note(format!("paths merge with different {k}"));
            }
            continue;
        }
        if done.contains(&name) {
            continue;
        }
        let Some((file, _)) = files.get_key_value(&name) else {
            c.note(format!("next file {name} doesn't exist"));
            not_compiled.insert(name);
            continue;
        };
        if played.len() >= max_files {
            not_compiled.insert(name);
            continue;
        }
        // The helper library isn't a story file, unless the game ports it
        // (a minigame).
        if (c.library.contains(&file.as_str()) || file.starts_with("function")) && !game.files.contains(&file.as_str()) {
            c.note(format!("run reaches {name}: a minigame, not converted"));
            not_compiled.insert(name);
            continue;
        }
        done.insert(name.clone());
        not_compiled.remove(&name);
        entry_vars.insert(name.clone(), state.vars.clone());
        c.restore(state);
        // Other paths reach the file with another screen: what it makes
        // again is placed anew.
        c.entry_shown = c.shown.keys().cloned().collect();
        c.redraw.clear();
        let playing: Vec<String> = c.objects.iter().filter(|(_, o)| matches!(&o.kind, Kind::Sound(s) if s.bgm && s.playing.is_some())).map(|(n, _)| n.clone()).collect();
        c.entry_music.extend(playing);
        // Story flags may hold anything when another path reaches the file.
        for (k, &v) in &c.flags {
            c.vars.insert(k.clone(), Value::Runtime(v));
        }
        // Helper variables kept at run time belong to the file that made
        // them; the next file sets them again before use.
        c.vars.retain(|k, v| !matches!(v, Value::Runtime(_)) || c.flags.contains_key(k));
        c.dynamic.clear();
        c.dynamic_values.clear();
        c.next_dynamic = dynamic_base;
        c.vars.remove(GAME_NAME_SET);
        // Other paths reach the file with other locks: it sets them anew.
        for k in super::PLAYER_LOCKS {
            c.vars.remove(k);
        }
        c.forks_in_file = 0;
        // Other paths jump to the file's label with another window up
        // (a vertical choice's): its first page sets its own.
        c.window = None;
        c.leaves_in_file = 0;
        c.steps = 0;
        c.ops.push(Op::Label(file_label(&name)));
        c.file = file;
        if std::env::var_os("NSS_TRACE").is_some() {
            eprintln!("compiling {name}");
        }
        c.call_scene(&format!("nss/{name}->{name}_MAIN"), false);
        c.sync(0);
        max_dynamic = max_dynamic.max(c.next_dynamic);
        played.push(name.clone());
        // Branch states: the end of the file, with the variables each
        // branch of a choice set differently.
        let end = c.snapshot();
        let (fork_var, branches) = match c.pending_fork.take() {
            None => (None, vec![end]),
            Some((var, leaves, first)) => (Some(var), leaves.iter().map(|l| with_branch_vars(&end, &l.state, &first)).collect()),
        };
        let fork = branches.len() > 1;
        for (i, b) in branches.into_iter().enumerate() {
            let next = b.vars.get("$GameName").map(Value::string).unwrap_or_default();
            let target = if let Some(&Value::Runtime(var)) = b.vars.get("$GameName") {
                // The next file is in a story flag: decided by the dispatch
                // written at the end, over every file name the flag holds
                // (one a flag: a file is compiled from the first state that
                // reaches it).
                match dispatches.iter().find(|d| d.1 == var) {
                    Some(d) => d.0.clone(),
                    None => {
                        let label = format!("dispatch{}", dispatches.len());
                        dispatches.push((label.clone(), var, b, std::collections::BTreeSet::new()));
                        label
                    }
                }
            } else if !b.vars.get("$GameCircle").is_some_and(Value::truthy) {
                RUN_END.to_string()
            } else if (next == name && !b.vars.contains_key(GAME_NAME_SET)) || next.is_empty() {
                c.note("file didn't name a next file");
                if std::env::var_os("NSS_TRACE").is_some() {
                    eprintln!("{name}: no next file");
                }
                RUN_END.to_string()
            } else {
                let l = file_label(&next);
                if !done.contains(&next) {
                    not_compiled.insert(next.clone());
                }
                queue.push_back((next, b));
                l
            };
            if let (true, Some(var)) = (fork, fork_var) {
                c.ops.push(Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(i as i32), label: target });
            } else {
                c.ops.push(Op::Jump(target));
            }
        }
        if fork {
            c.ops.push(Op::Jump(RUN_END.to_string()));
        }
    }
    c.ops.push(Op::Jump(RUN_END.to_string()));
    c.subroutines(&initial, max_dynamic);
    for (label, var, _, seen) in &dispatches {
        c.ops.push(Op::Label(label.clone()));
        for n in seen {
            if let Some(&code) = c.codes.get(n) {
                let value = Operand::Const(code);
                c.ops.push(Op::If { var: *var, cmp: conv_core::ir::Compare::Eq, value, label: file_label(n) });
            }
        }
        c.ops.push(Op::Jump(RUN_END.to_string()));
    }
    // Files the run reaches but doesn't include end the game.
    for name in not_compiled {
        c.ops.push(Op::Label(file_label(&name)));
    }
    compile_boot(&mut c, files, game, &initial);
    c.ops.push(Op::Label(RUN_END.to_string()));
    Ok((Compiled { ops: c.ops, window: c.first_window, notes: c.notes, metrics: c.metrics, flags: c.flags, voice_classes: c.voice_classes, source_map: c.source_map }, played))
}

/// The flag a helper holds on the leaves that set it, when those that
/// don't leave it unset (a choice's third text, set only where it has
/// three options): the code after reads it only where it was set.
pub(super) fn one_runtime(k: &str, leaves: &[Leaf]) -> Option<conv_core::ir::Var> {
    let mut var = None;
    for l in leaves {
        match l.state.vars.get(k) {
            None | Some(Value::Null) => {}
            Some(&Value::Runtime(v)) if var.is_none_or(|w| w == v) => var = Some(v),
            _ => return None,
        }
    }
    var
}

/// Whether a block's last statement is `return`.
pub(super) fn ends_in_return(stmts: &[Stmt]) -> bool {
    stmts.last().is_some_and(|s| matches!(s.kind, StmtKind::Return))
}

pub(super) fn negate(c: conv_core::ir::Compare) -> conv_core::ir::Compare {
    use conv_core::ir::Compare::*;
    match c {
        Eq => Ne,
        Ne => Eq,
        Lt => Ge,
        Ge => Lt,
        Gt => Le,
        Le => Gt,
    }
}

/// `end` (the state after a fork's first branch went on) with the
/// variables `branch` set differently from that first branch, `first`,
/// unless the code after the fork set them again.
pub(super) fn with_branch_vars<'a>(end: &State<'a>, branch: &State<'a>, first: &State<'a>) -> State<'a> {
    let mut s = end.clone();
    let untouched = |k: &String| end.vars.get(k) == first.vars.get(k);
    for (k, v) in branch.vars.iter().filter(|(k, _)| untouched(k)) {
        if first.vars.get(k) != Some(v) {
            s.vars.insert(k.clone(), v.clone());
        }
    }
    for k in first.vars.keys().filter(|k| !branch.vars.contains_key(*k) && untouched(k)) {
        s.vars.remove(k);
    }
    s
}

/// Story flags: variables the story files (not the helper library) both
/// set and test in a condition, such as route counters (`$Ichizyou_Flag`)
/// and unlocks after a clear (`#魔王編解放`). Their values depend on the
/// path through the game, so they are kept in NScripter variables. The
/// engine's own (`$SYSTEM_*`) and the file chaining (`$GameName`) are left
/// to the convert-time run.
/// The game adds the variables its own code sets and tests ([`Game`]).
pub fn story_flags(files: &BTreeMap<String, NssFile>, game: &Game) -> HashMap<String, conv_core::ir::Var> {
    use std::collections::BTreeSet;
    /// `$a["x"]` with a constant index: its key (as the compiler builds it).
    fn element(base: &Expr, index: &Expr) -> Option<String> {
        let i = match index {
            Expr::Str(s) => s.clone(),
            Expr::Int(n) => n.to_string(),
            _ => return None,
        };
        match base {
            Expr::Var(v) => Some(format!("{v}[{i}]")),
            _ => None,
        }
    }
    fn expr_vars(e: &Expr, out: &mut BTreeSet<String>) {
        match e {
            Expr::Var(v) => {
                out.insert(v.to_string());
            }
            Expr::Index { base, index } => {
                out.extend(element(base, index));
            }
            Expr::Unary { expr, .. } => expr_vars(expr, out),
            Expr::Binary { left, right, .. } => {
                expr_vars(left, out);
                expr_vars(right, out);
            }
            _ => {}
        }
    }
    fn walk(stmts: &[Stmt], set: &mut BTreeSet<String>, tested: &mut BTreeSet<String>) {
        for s in stmts {
            match &s.kind {
                StmtKind::Assign { target, index, .. } if index.is_empty() => {
                    set.insert(target.to_string());
                }
                StmtKind::Assign { target, index, .. } if index.len() == 1 => {
                    set.extend(element(&Expr::Var(target.clone()), &index[0]));
                }
                StmtKind::If { cond, then, otherwise } => {
                    expr_vars(cond, tested);
                    walk(then, set, tested);
                    if let Some(o) = otherwise {
                        walk(o, set, tested);
                    }
                }
                StmtKind::While { cond, body } => {
                    expr_vars(cond, tested);
                    walk(body, set, tested);
                }
                StmtKind::Select(b) | StmtKind::Block(b) | StmtKind::Case { body: b, .. } => walk(b, set, tested),
                StmtKind::Text(t) => {
                    for i in &t.items {
                        if let Inline::Code(c) = i {
                            walk(c, set, tested);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let library: Vec<&str> = library(files, game);
    let (mut set, mut tested) = (BTreeSet::new(), BTreeSet::new());
    for (name, file) in files {
        let story = !library.contains(&name.as_str()) && !["0_", "sys_", "extra_", "function"].iter().any(|p| name.starts_with(p));
        if !story {
            continue;
        }
        for item in &file.items {
            if let Item::Scene(b) | Item::Chapter(b) = item {
                walk(&b.body, &mut set, &mut tested);
            }
        }
    }
    for key in (game.flags)(files) {
        set.insert(key.clone());
        tested.insert(key);
    }
    let mut flags = HashMap::new();
    let (mut local, mut global) = (FLAG_LOCAL_BASE, 0u16);
    for name in set.intersection(&tested) {
        if name.starts_with("$SYSTEM") || name.starts_with("#SYSTEM") || name == "$GameName" || name == "$GameCircle" || name == "$AllRead" {
            continue;
        }
        let var = if name.starts_with('#') {
            global += 1;
            let index = if global <= GLOBAL_ROOM { global - 1 } else { GLOBAL_OVERFLOW + global - 1 - GLOBAL_ROOM };
            conv_core::ir::Var { global: true, index }
        } else {
            local += 1;
            conv_core::ir::Var { global: false, index: local - 1 }
        };
        flags.insert(name.clone(), var);
    }
    // Set but read only by a choice: at the top of the locals, past the
    // helpers that become runtime ones, so no other number moves.
    let mut top = CHOICE_FLAGS.end;
    for name in choice_flags(files).intersection(&set) {
        if !flags.contains_key(name) && name.starts_with('$') && !name.starts_with("$SYSTEM") && top > CHOICE_FLAGS.start {
            top -= 1;
            flags.insert(name.clone(), conv_core::ir::Var { global: false, index: top });
        }
    }
    flags
}

/// The variables a choice reads: its options' texts (`SetChoice04("見る",
/// "話す", $md05_hazure, "移動する")`) and the conditions that pick how many
/// options it has (`if($選択肢カウント==1){SetChoice02(…);}`). Their values
/// can differ between the branches of a runtime if, so they are kept at
/// run time even where other helpers are not ([`Compiler::finish_fork`]).
pub(super) fn choice_flags(files: &BTreeMap<String, NssFile>) -> std::collections::BTreeSet<String> {
    use std::collections::BTreeSet;
    fn is_choice(s: &Stmt) -> bool {
        matches!(&s.kind, StmtKind::Expr(Expr::Call { name, .. }) if name.starts_with("SetChoice"))
    }
    fn vars(e: &Expr, out: &mut BTreeSet<String>) {
        match e {
            Expr::Var(v) => {
                out.insert(v.to_string());
            }
            Expr::Unary { expr, .. } => vars(expr, out),
            Expr::Binary { left, right, .. } => {
                vars(left, out);
                vars(right, out);
            }
            _ => {}
        }
    }
    fn walk(stmts: &[Stmt], out: &mut BTreeSet<String>) {
        for s in stmts {
            match &s.kind {
                StmtKind::Expr(Expr::Call { args, .. }) if is_choice(s) => args.iter().for_each(|a| vars(a, out)),
                StmtKind::If { cond, then, otherwise } => {
                    if then.iter().chain(otherwise.iter().flatten()).any(is_choice) {
                        vars(cond, out);
                    }
                    walk(then, out);
                    walk(otherwise.as_deref().unwrap_or(&[]), out);
                }
                StmtKind::While { body, .. } | StmtKind::Select(body) | StmtKind::Block(body) | StmtKind::Case { body, .. } => walk(body, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeSet::new();
    for file in files.values() {
        for item in &file.items {
            if let Item::Scene(b) | Item::Chapter(b) = item {
                walk(&b.body, &mut out);
            }
        }
    }
    out
}

/// Label of a story file's start.
pub(super) fn file_label(name: &str) -> String {
    format!("f_{}", name.trim_end_matches(".nss"))
}

/// Files the story driver includes, directly or not, in include order.
pub fn library<'a>(files: &'a BTreeMap<String, NssFile>, game: &Game) -> Vec<&'a str> {
    fn visit<'a>(files: &'a BTreeMap<String, NssFile>, name: &str, out: &mut Vec<&'a str>) {
        let Some((key, file)) = files.get_key_value(name) else { return };
        if out.contains(&key.as_str()) {
            return;
        }
        out.push(key);
        for item in &file.items {
            if let Item::Include(path) = item {
                let name = path.rsplit('/').next().unwrap_or(path);
                visit(files, name, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(files, game.boot, &mut out);
    out.retain(|&f| f != game.boot);
    out
}
