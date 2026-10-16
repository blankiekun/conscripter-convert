//! The English text of Full Metal Daemon Muramasa (JAST USA 2021), put
//! into the Japanese release's NSS scripts.
//!
//! The English release's scenes (`script.npk`: `media/script/nut/<scene>.nut`)
//! transcribe the NSS line by line; each `<PRE>` block becomes one
//! `Print(key, japanese)` per stretch of text between its inline `{…}`
//! code, and `media/script/lang/en/<scene>.nut` holds
//! `TransText[key] <- english` (the Japanese again as `//` lines, then the
//! English). So each stretch of an NSS text block whose Japanese equals a
//! Print's (comment lines, tags and whitespace left out) takes that key's
//! English; 99 % of them do.

use std::collections::{BTreeMap, HashMap};

use super::npk::{Npk, NpkError};
use super::nss::{self, Inline, Item, NssFile, Stmt, StmtKind};
use super::squirrel;
use crate::metrics::Metrics;

#[derive(Debug, thiserror::Error)]
pub enum EnglishError {
    #[error(transparent)]
    Npk(#[from] NpkError),
    #[error("{0}: {1}")]
    Script(String, squirrel::SquirrelError),
}

/// Scene name (`ma01_019`) → its Prints in order: (Japanese key, English).
pub struct English {
    scenes: HashMap<String, Vec<(String, String)>>,
    /// Scene name → its choices' options: Japanese → English. The scenes'
    /// `SetChoice(id, japanese…)` and the lang file's
    /// `TransChoice["<id>_<n>"]` (n from 1).
    choices: HashMap<String, HashMap<String, String>>,
    /// Scene name → the variables its choices take options from
    /// (`SetChoice(60, Route.選択肢名前1, …)`) → the English the port puts
    /// in them, in order: the NSS's `$選択肢名前1 = "…"` take them in turn.
    choice_vars: HashMap<String, HashMap<String, Vec<String>>>,
    /// Scene name → the scene's calls on its telop sprites (the ones made
    /// with a `cg/sys/telop/` picture), in order ([`place_telops`]).
    telops: HashMap<String, Vec<(String, Vec<squirrel::Arg>)>>,
    /// Scene name → its subtitled voices ([`subtitles`]).
    subtitles: HashMap<String, Vec<Subtitle>>,
    /// The port's calls the converter takes from, as `port <function>`,
    /// with the arguments it doesn't read ([`note_unread`]).
    unread: Metrics,
}

/// Arguments of the port's calls the converter takes that it doesn't read
/// and that change nothing: (function, argument, the value covered, or
/// `None` for any, why). A value is written as [`arg_text`] gives it.
const UNREAD_HARMLESS: &[(&str, usize, Option<&str>, &str)] = &[
    // ma05_021vs's debug menu (`$GameDebugSelect`): names the port never defines.
    ("SetChoice", 1, Some("魔王編"), "debug menu"),
    ("SetChoice", 2, Some("英雄編・香奈枝生存"), "debug menu"),
    ("SetChoice", 3, Some("英雄編・香奈枝死亡"), "debug menu"),
    ("SetChoice", 4, Some("復讐編・一条生存"), "debug menu"),
    ("SetChoice", 5, Some("復讐編・一条死亡"), "debug menu"),
    ("SetChoice", 6, Some("バッドエンド"), "debug menu"),
];

/// An argument as [`UNREAD_HARMLESS`] names it: a number, a string or a
/// constant as written, `true`/`false`, `@n`, `other` (null, a variable).
fn arg_text(a: &squirrel::Arg) -> String {
    use squirrel::Arg;
    match a {
        Arg::Int(n) => n.to_string(),
        Arg::Str(s) | Arg::Word(s) => s.clone(),
        Arg::Relative(n) => format!("@{n}"),
        Arg::Bool(b) => b.to_string(),
        Arg::Other => "other".into(),
    }
}

/// Which arguments of the port's call the converter reads, for the calls
/// whose effect it takes from the port: the subtitles, the choices' and
/// the pages' text; `telop`: a call on a telop sprite (its places).
fn port_reads(call: &str, args: &[squirrel::Arg], telop: bool) -> Option<Vec<usize>> {
    use squirrel::Arg;
    Some(match call {
        // (sound, add)
        "ShowVOICESEText" => vec![0, 1],
        // (id, option, option, …): the strings and the `Route.<name>` variables
        "SetChoice" => (0..args.len()).filter(|&i| i == 0 || matches!(args[i], Arg::Str(_)) || route_var(args, i).is_some()).collect(),
        // (number, text)
        "Print" => vec![0, 1],
        _ if telop && !placing(call).is_empty() => {
            let mut read = vec![0];
            read.extend(placing(call));
            read
        }
        _ => return None,
    })
}

/// Counts in `unread` a call [`port_reads`] takes, noting each argument it
/// doesn't read, but for [`UNREAD_HARMLESS`]'s.
fn note_unread(unread: &mut Metrics, call: &str, args: &[squirrel::Arg], telop: bool) {
    let Some(read) = port_reads(call, args, telop) else { return };
    let reasons = args
        .iter()
        .enumerate()
        .filter(|&(i, a)| !read.contains(&i) && !UNREAD_HARMLESS.iter().any(|&(f, k, v, _)| f == call && k == i && v.is_none_or(|v| v == arg_text(a))))
        .map(|(i, _)| format!("{call} argument {i} ignored"))
        .collect();
    unread.call(&format!("port {call}"), reasons);
}

/// The variable `args[i]` or `args[i..=i + 1]` names: `Route.<name>` is
/// `Word("Route"), Word(name)`.
fn route_var(args: &[squirrel::Arg], i: usize) -> Option<&str> {
    use squirrel::Arg;
    match (i.checked_sub(1).map(|j| &args[j]), &args[i], args.get(i + 1)) {
        (_, Arg::Word(r), Some(Arg::Word(name))) if r == "Route" => Some(name),
        (Some(Arg::Word(r)), Arg::Word(name), _) if r == "Route" => Some(name),
        _ => None,
    }
}

/// A voice the port subtitles (`ShowVOICESEText(sound)` after starting it,
/// its lines from the lang file's `TransVoice[file]`): put into the NSS
/// after the `n`-th `call(sound, …)` as
/// `ShowVOICESEText(sound, add, y, file, ms, text, ms, text, …)`.
#[derive(Debug, Clone, PartialEq)]
struct Subtitle {
    call: String,
    sound: String,
    n: usize,
    /// A line under the one shown (`ShowVOICESEText(sound, true)`).
    add: bool,
    y: i32,
    /// The voice (`ma04/0230130a13`).
    file: String,
    /// (ms into the voice, text)
    lines: Vec<(i32, String)>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub replaced: usize,
    /// Stretches whose Japanese the port split or edited, given the rows
    /// between their neighbours' ([`align`]).
    pub aligned: usize,
    /// (file, Japanese key) of the stretches the translation left out.
    pub dropped: Vec<(String, String)>,
    /// (file, Japanese key) of the stretches left in Japanese.
    pub missed: Vec<(String, String)>,
    /// Choice options given their English.
    pub options: usize,
    /// (file, option) of the choice options left in Japanese.
    pub options_missed: Vec<(String, String)>,
    /// Subtitled voices given their `ShowVOICESEText`.
    pub subtitles: usize,
    /// (file, sound) of the subtitles with no place found.
    pub subtitles_missed: Vec<(String, String)>,
    /// Calls on telop sprites given the port's places.
    pub telops_placed: usize,
    /// (file, call, sprite and its number) that the port makes a
    /// different number of times: left at the Japanese places.
    pub telops_unpaired: Vec<(String, String)>,
    /// The port's calls the converter takes from, with the arguments not
    /// read (`<function> argument <i> ignored`).
    pub unread: Metrics,
}

/// The form both sides are compared in: no `//` lines, tags, `<?>` or
/// whitespace.
fn key(s: &str) -> String {
    let mut out = String::new();
    for line in s.lines().filter(|l| !l.trim_start().starts_with("//")) {
        let mut in_tag = false;
        for c in line.chars() {
            match c {
                '<' => in_tag = true,
                '>' if in_tag => in_tag = false,
                _ if in_tag || c.is_whitespace() => {}
                _ => out.push(c),
            }
        }
    }
    out
}

impl English {
    /// Reads the English release's `script.npk`.
    pub fn load(scripts: &Npk) -> Result<English, EnglishError> {
        let load = |e: &super::npk::Entry| -> Result<squirrel::Function, EnglishError> {
            squirrel::load(&scripts.data(e)?).map_err(|err| EnglishError::Script(e.name.clone(), err))
        };
        let mut scenes = HashMap::new();
        let mut choices = HashMap::new();
        let mut choice_vars = HashMap::new();
        let mut telops = HashMap::new();
        let mut subtitles = HashMap::new();
        let mut unread = Metrics::default();
        for e in &scripts.entries {
            let Some(scene) = e.name.strip_prefix("media/script/nut/").and_then(|n| n.strip_suffix(".nut")) else { continue };
            let Some(lang) = scripts.find(&format!("media/script/lang/en/{scene}.nut")) else { continue };
            let lang = load(lang)?;
            let english: HashMap<i32, String> = lang.table_rows("TransText").into_iter().collect();
            let options: HashMap<String, String> = lang.table_str_rows("TransChoice").into_iter().collect();
            let scene_fn = load(e)?;
            let mut sets = Vec::new();
            scene_fn.calls_int_strs("SetChoice", &mut sets);
            let scene_choices: HashMap<String, String> = sets
                .into_iter()
                .flat_map(|(id, jp)| jp.into_iter().enumerate().filter_map(move |(n, jp)| Some((format!("{id}_{}", n + 1), jp?))))
                .filter_map(|(k, jp)| Some((jp, options.get(&k)?.clone())))
                .collect();
            choices.insert(scene.to_lowercase(), scene_choices);
            let mut calls = Vec::new();
            scene_fn.calls(&mut calls);
            subtitles.insert(scene.to_lowercase(), scene_subtitles(&calls, &lang.table_literal_rows("TransVoice")));
            let mut vars: HashMap<String, Vec<String>> = HashMap::new();
            for (call, args) in &calls {
                note_unread(&mut unread, call, args, false);
                if call == "SetChoice" {
                    for name in (1..args.len()).filter_map(|i| route_var(args, i)) {
                        vars.entry(name.to_string()).or_insert_with(|| {
                            let mut en = Vec::new();
                            scene_fn.slot_strs(name, &mut en);
                            en
                        });
                    }
                }
            }
            choice_vars.insert(scene.to_lowercase(), vars);
            let scene_telops = telop_calls(calls);
            for (call, args) in &scene_telops {
                note_unread(&mut unread, call, args, true);
            }
            telops.insert(scene.to_lowercase(), scene_telops);
            let mut prints = Vec::new();
            scene_fn.calls_int_str("Print", &mut prints);
            // Prints with no English show nothing in the port, which types
            // its own lines instead (ma01_026's document: `SetText`): the
            // first of them takes those lines.
            let mut typed = Vec::new();
            scene_fn.strs_before_call("SetText", &mut typed);
            let mut typed = Some(typed.join("\n"));
            let rows = prints
                .into_iter()
                .map(|(k, jp)| {
                    let en = english.get(&k).map(|en| en.replace("\r\n", "\n"));
                    (key(&jp), en.unwrap_or_else(|| typed.take().unwrap_or_default()))
                })
                .collect();
            scenes.insert(scene.to_lowercase(), rows);
        }
        Ok(English { scenes, choices, choice_vars, telops, subtitles, unread })
    }

    /// Puts the English into every text block of `files` (by file name,
    /// `ma01_019.nss`).
    pub fn apply(&self, files: &mut BTreeMap<String, NssFile>) -> Report {
        let mut report = Report { unread: self.unread.clone(), ..Report::default() };
        for (name, file) in files.iter_mut() {
            let scene = name.to_lowercase();
            let scene = scene.strip_suffix(".nss").unwrap_or(&scene);
            if let Some(options) = self.choices.get(scene) {
                each_choice_option(file, &mut |jp| match options.get(jp.as_str()) {
                    Some(en) => {
                        *jp = en.clone();
                        report.options += 1;
                    }
                    None => report.options_missed.push((name.clone(), jp.clone())),
                });
            }
            for (var, en) in self.choice_vars.get(scene).into_iter().flatten() {
                // The port is a line-by-line transcription, its branches at
                // times merged: the n-th value the NSS gives the variable is
                // the port's n-th.
                let mut jp: Vec<String> = Vec::new();
                each_choice_var(file, var, &mut |s, assigned| if assigned && !jp.contains(s) { jp.push(s.clone()) });
                let mut en_values: Vec<&String> = Vec::new();
                en.iter().for_each(|e| if !en_values.contains(&e) { en_values.push(e) });
                if jp.len() == en_values.len() {
                    report.options += jp.len();
                    let map: HashMap<String, String> = jp.into_iter().zip(en_values.into_iter().cloned()).collect();
                    each_choice_var(file, var, &mut |s, _| if let Some(en) = map.get(s.as_str()) { s.clone_from(en) });
                } else {
                    report.options_missed.extend(jp.into_iter().map(|jp| (name.clone(), jp)));
                }
            }
            for sub in self.subtitles.get(scene).into_iter().flatten() {
                let mut args = vec![nss::Expr::Str(sub.sound.clone()), nss::Expr::Int(i64::from(sub.add)), nss::Expr::Int(i64::from(sub.y)), nss::Expr::Str(sub.file.clone())];
                for (ms, text) in &sub.lines {
                    args.extend([nss::Expr::Int(i64::from(*ms)), nss::Expr::Str(text.clone())]);
                }
                let mut seen = 0;
                let found = insert_call(file, &mut |c, a| {
                    let hit = c == sub.call && matches!(a.first(), Some(nss::Expr::Str(s)) if *s == sub.sound);
                    seen += usize::from(hit);
                    hit && seen == sub.n + 1
                }, nss::Expr::Call { name: "ShowVOICESEText".into(), args }, true);
                match found {
                    true => report.subtitles += 1,
                    false => report.subtitles_missed.push((name.clone(), sub.sound.clone())),
                }
            }
            if let Some(port) = self.telops.get(scene) {
                place_telops(name, file, port, &mut report);
            }
            let Some(rows) = self.scenes.get(scene) else { continue };
            let mut keys = Vec::new();
            each_run(file, &mut |run, _| {
                let jp = run_key(&run);
                if !jp.is_empty() {
                    keys.push(jp);
                }
                run
            });
            let mut picks = align(rows, &keys).into_iter().zip(keys);
            each_run(file, &mut |run, line| {
                if run_key(&run).is_empty() {
                    return run;
                }
                let (pick, jp) = picks.next().expect("the same stretches on both walks");
                let english: String = match &pick {
                    Pick::Row(i) => rows[*i].1.clone(),
                    Pick::Gap(gap) => gap.iter().map(|&i| rows[i].1.as_str()).collect(),
                };
                match replace(run, &english, line) {
                    Ok(run) => {
                        match pick {
                            Pick::Row(_) => report.replaced += 1,
                            Pick::Gap(gap) if gap.is_empty() => report.dropped.push((name.clone(), jp)),
                            Pick::Gap(_) => report.aligned += 1,
                        }
                        run
                    }
                    Err(run) => {
                        report.missed.push((name.clone(), jp));
                        run
                    }
                }
            });
        }
        report
    }
}

/// What a stretch of Japanese takes.
#[derive(Debug, PartialEq)]
enum Pick {
    /// The row with the same Japanese.
    Row(usize),
    /// No row has its Japanese: these rows' English, joined (none: the
    /// stretch is dropped).
    Gap(Vec<usize>),
}

/// Pairs each stretch (`keys`, in order) with the rows. Equal Japanese
/// takes its row, the next one after the last used. Where the port split
/// or edited the Japanese, the stretches between two matches share the
/// rows left unused between them: the first stretch takes them all, the
/// others none. With no rows left the translation folded the line into a
/// neighbour's (`ma01_003`: the narration after Konatsu's line is in her
/// row), and the stretch is dropped.
fn align(rows: &[(String, String)], keys: &[String]) -> Vec<Pick> {
    let n = rows.len();
    let mut next = 0;
    let found: Vec<Option<usize>> = keys
        .iter()
        .map(|jp| {
            let i = (next..n).chain(0..next).find(|&i| rows[i].0 == *jp)?;
            next = i + 1;
            Some(i)
        })
        .collect();
    let mut used = vec![false; n];
    found.iter().flatten().for_each(|&i| used[i] = true);
    let mut prev = None;
    found
        .iter()
        .enumerate()
        .map(|(k, f)| {
            if let Some(i) = *f {
                prev = Some(i);
                return Pick::Row(i);
            }
            if k > 0 && found[k - 1].is_none() {
                return Pick::Gap(Vec::new());
            }
            let start = prev.map_or(0, |p| p + 1);
            let end = found[k..].iter().flatten().next().copied().unwrap_or(n);
            Pick::Gap((start..end.max(start)).filter(|&i| !used[i]).collect())
        })
        .collect()
}

/// Calls `f` on each stretch of text between inline code, in order, with
/// its block's line, and puts back the items it returns.
fn each_run(file: &mut NssFile, f: &mut dyn FnMut(Vec<Inline>, u32) -> Vec<Inline>) {
    fn stmts(list: &mut [Stmt], f: &mut dyn FnMut(Vec<Inline>, u32) -> Vec<Inline>) {
        for s in list {
            match &mut s.kind {
                StmtKind::If { then, otherwise, .. } => {
                    stmts(then, f);
                    otherwise.iter_mut().for_each(|o| stmts(o, f));
                }
                StmtKind::While { body, .. } | StmtKind::Select(body) | StmtKind::Case { body, .. } | StmtKind::Block(body) => stmts(body, f),
                StmtKind::Text(t) => {
                    let mut out = Vec::with_capacity(t.items.len());
                    let mut run = Vec::new();
                    for item in std::mem::take(&mut t.items).into_iter().chain([Inline::Code(Vec::new())]) {
                        if matches!(item, Inline::Code(_)) {
                            out.extend(f(std::mem::take(&mut run), s.line));
                            out.push(item);
                        } else {
                            run.push(item);
                        }
                    }
                    out.pop();
                    t.items = out;
                }
                _ => {}
            }
        }
    }
    for item in &mut file.items {
        match item {
            Item::Chapter(b) | Item::Scene(b) => stmts(&mut b.body, f),
            Item::Function(fun) => stmts(&mut fun.body, f),
            Item::Include(_) => {}
        }
    }
}

/// The port's subtitled voices in a scene: each `ShowVOICESEText(sound
/// [, add])` in `calls`, after the sound's start (`MusicStart`, `SoundPlay`)
/// and its file (`CreateVOICE…`, a `rows` key: the lang file's
/// `TransVoice`). A row is a text, or a list of tables (`time`, `text`,
/// `x`, `y`, `add`); a text shows at y 480 (`serif0`'s place,
/// `function_textbox.nut`). Texts lose their `<center>` tags (every
/// subtitle is centred); a voice whose texts are all empty has none.
fn scene_subtitles(calls: &[(String, Vec<squirrel::Arg>)], rows: &[(String, Vec<squirrel::Literal>)]) -> Vec<Subtitle> {
    use squirrel::{Arg, Literal};
    let mut out = Vec::new();
    for (i, (call, args)) in calls.iter().enumerate() {
        let Some(Arg::Str(sound)) = args.first().filter(|_| call == "ShowVOICESEText") else { continue };
        let is_sound = |a: &[Arg]| a.first() == Some(&Arg::Str(sound.clone()));
        let Some(start) = calls[..i].iter().rposition(|(c, a)| (c == "MusicStart" || c == "SoundPlay") && is_sound(a)) else { continue };
        let file = calls[..i].iter().rev().filter(|(c, a)| c.starts_with("CreateVOICE") && is_sound(a)).find_map(|(_, a)| {
            a.iter().find_map(|f| match f {
                Arg::Str(f) => rows.iter().find(|(k, _)| k == f),
                _ => None,
            })
        });
        let Some((file, row)) = file else { continue };
        let (mut y, mut lines) = (480, Vec::new());
        match row.as_slice() {
            [Literal::Str(text)] => lines.push((0, text.clone())),
            row => {
                // `add = false` loads no literal: read field by field.
                let mut ms = 0;
                for pair in row.windows(2) {
                    match pair {
                        [Literal::Str(k), Literal::Int(v)] if k == "time" => ms = *v,
                        [Literal::Str(k), Literal::Int(v)] if k == "y" => y = *v,
                        [Literal::Str(k), Literal::Str(text)] if k == "text" => lines.push((ms, text.clone())),
                        _ => {}
                    }
                }
            }
        }
        for (_, text) in &mut lines {
            *text = text.replace("<center>", "").replace("</center>", "").trim().to_string();
        }
        if lines.iter().all(|(_, t)| t.is_empty()) {
            continue;
        }
        let (c, _) = &calls[start];
        let n = calls[..start].iter().filter(|(c2, a)| c2 == c && is_sound(a)).count();
        out.push(Subtitle { call: c.clone(), sound: sound.clone(), n, add: args.get(1) == Some(&Arg::Bool(true)), y, file: file.clone(), lines });
    }
    out
}

/// The calls of `calls` on telop sprites: the ones a `Create…` call
/// makes with a `cg/sys/telop/` picture.
fn telop_calls(calls: Vec<(String, Vec<squirrel::Arg>)>) -> Vec<(String, Vec<squirrel::Arg>)> {
    use squirrel::Arg;
    let telop = |a: &Arg| matches!(a, Arg::Str(p) if p.to_lowercase().contains("/telop/"));
    let names: std::collections::HashSet<String> = calls
        .iter()
        .filter(|(f, a)| f.starts_with("Create") && a.iter().any(telop))
        .filter_map(|(_, a)| match a.first() {
            Some(Arg::Str(s)) => Some(s.clone()),
            _ => None,
        })
        .collect();
    calls.into_iter().filter(|(_, a)| matches!(a.first(), Some(Arg::Str(s)) if names.contains(s))).collect()
}

/// The arguments of `call` that place a picture: x, y and the picture of
/// `CreateTexture…(name, priority, x, y, picture)`, x and y of
/// `Move(name, time, x, y, …)`.
fn placing(call: &str) -> &'static [usize] {
    match call {
        _ if call.starts_with("CreateTexture") => &[2, 3, 4],
        "Move" => &[2, 3],
        _ => &[],
    }
}

/// Moves the telops of `file` to the port's places (`port`: the scene's
/// calls on its telop sprites). The English telops (`cg/sys/telop/`) are
/// mostly other sizes than the Japanese (chapter titles lying, not
/// standing; captions wider), and the port's scenes place them anew.
/// The k-th call of a kind on a sprite takes the port's k-th one's
/// places, when both make that call as often (otherwise a picture the
/// port makes once on the sprite takes its place); the picture only when the
/// port's is a telop (the port names its other pictures `.png` for
/// `.jpg`).
fn place_telops(file_name: &str, file: &mut NssFile, port: &[(String, Vec<squirrel::Arg>)], report: &mut Report) {
    use squirrel::Arg;
    let mut theirs: HashMap<(&str, &str), Vec<&[Arg]>> = HashMap::new();
    for (call, args) in port {
        if let Some(Arg::Str(sprite)) = args.first() && !placing(call).is_empty() {
            theirs.entry((sprite.as_str(), call.as_str())).or_default().push(args);
        }
    }
    let key = |call: &str, args: &[nss::Expr]| match args.first() {
        Some(nss::Expr::Str(sprite)) if !placing(call).is_empty() => Some((sprite.clone(), call.to_string())),
        _ => None,
    };
    let mut ours: HashMap<(String, String), usize> = HashMap::new();
    each_call(file, &mut |call, args| {
        if let Some(k) = key(call, args) {
            *ours.entry(k).or_default() += 1;
        }
    });
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    each_call(file, &mut |call, args| {
        let Some(k) = key(call, args) else { return };
        let Some(list) = theirs.get(&(k.0.as_str(), k.1.as_str())) else { return };
        let n = seen.entry(k.clone()).or_default();
        *n += 1;
        let port = if list.len() == ours[&k] {
            list[*n - 1]
        } else {
            // The port rewrote this part: a picture it makes once on the
            // sprite still takes its place.
            let ours = match args.get(4) {
                Some(nss::Expr::Str(p)) if call.starts_with("CreateTexture") => p.to_lowercase(),
                _ => String::new(),
            };
            let same: Vec<_> = list.iter().filter(|a| matches!(a.get(4), Some(Arg::Str(p)) if p.to_lowercase() == ours)).collect();
            match same[..] {
                [port] => port,
                _ => {
                    report.telops_unpaired.push((file_name.to_string(), format!("{} {} #{n}", k.1, k.0)));
                    return;
                }
            }
        };
        for &i in placing(call) {
            let (Some(ours), Some(theirs)) = (args.get_mut(i), port.get(i)) else { continue };
            *ours = match theirs {
                Arg::Int(v) => nss::Expr::Int(i64::from(*v)),
                Arg::Word(w) => nss::Expr::Word(w.clone()),
                Arg::Relative(v) => nss::Expr::Unary { op: nss::UnaryOp::Relative, expr: Box::new(nss::Expr::Int(i64::from(*v))) },
                Arg::Str(p) if p.to_lowercase().contains("/telop/") => nss::Expr::Str(p.clone()),
                _ => continue,
            };
        }
        report.telops_placed += 1;
    });
    // A fade out the port adds on a telop sprite (ma01_026 clears its
    // preface before the chapter title comes up where it was) goes before
    // the port's next placing call.
    let mut faded = std::collections::HashSet::new();
    each_call(file, &mut |call, args| {
        if call == "FadeDelete" && let Some(nss::Expr::Str(sprite)) = args.first() {
            faded.insert(sprite.clone());
        }
    });
    for (i, (call, args)) in port.iter().enumerate() {
        let (Some(Arg::Str(sprite)), Some(Arg::Int(time)), Some(Arg::Bool(wait))) = (args.first(), args.get(1), args.get(3)) else { continue };
        if call != "FadeDelete" || faded.contains(sprite) {
            continue;
        }
        let Some((next, Some(Arg::Str(at)))) = port[i + 1..].iter().find(|(c, a)| !placing(c).is_empty() && matches!(a.first(), Some(Arg::Str(_)))).map(|(c, a)| (c, a.first())) else { continue };
        let nth = port[..i].iter().filter(|(c, a)| c == next && a.first() == Some(&Arg::Str(at.clone()))).count();
        let fade = nss::Expr::Call {
            name: call.clone(),
            args: vec![nss::Expr::Str(sprite.clone()), nss::Expr::Int(i64::from(*time)), nss::Expr::Word(wait.to_string())],
        };
        let mut seen = 0;
        let found = insert_call(file, &mut |c, a| {
            let hit = c == next && matches!(a.first(), Some(nss::Expr::Str(s)) if s == at);
            seen += usize::from(hit);
            hit && seen == nth + 1
        }, fade, false);
        if found {
            report.telops_placed += 1;
        }
    }
}

/// Puts `call` as a statement before (or `after`) the first call
/// statement of `file` that `at` takes. Returns whether it found one.
fn insert_call(file: &mut NssFile, at: &mut dyn FnMut(&str, &[nss::Expr]) -> bool, call: nss::Expr, after: bool) -> bool {
    fn stmts(list: &mut Vec<Stmt>, at: &mut dyn FnMut(&str, &[nss::Expr]) -> bool, call: &mut Option<nss::Expr>, after: bool) {
        let mut i = 0;
        while i < list.len() && call.is_some() {
            match &mut list[i].kind {
                StmtKind::If { then, otherwise, .. } => {
                    stmts(then, at, call, after);
                    otherwise.iter_mut().for_each(|o| stmts(o, at, call, after));
                }
                StmtKind::While { body, .. } | StmtKind::Select(body) | StmtKind::Case { body, .. } | StmtKind::Block(body) => stmts(body, at, call, after),
                StmtKind::Text(t) => t.items.iter_mut().for_each(|i| if let Inline::Code(body) = i { stmts(body, at, call, after) }),
                StmtKind::Expr(nss::Expr::Call { name, args }) if at(name, args) => {
                    let line = list[i].line;
                    list.insert(i + usize::from(after), Stmt { line, kind: StmtKind::Expr(call.take().expect("checked")) });
                    return;
                }
                _ => {}
            }
            i += 1;
        }
    }
    let mut call = Some(call);
    for item in &mut file.items {
        match item {
            Item::Chapter(b) | Item::Scene(b) => stmts(&mut b.body, at, &mut call, after),
            Item::Function(fun) => stmts(&mut fun.body, at, &mut call, after),
            Item::Include(_) => {}
        }
    }
    call.is_none()
}

/// Calls `f` on each call statement of `file`: its name and arguments.
fn each_call(file: &mut NssFile, f: &mut dyn FnMut(&str, &mut [nss::Expr])) {
    each_stmt(file, &mut |s| if let StmtKind::Expr(nss::Expr::Call { name, args }) = s { f(name, args) });
}

/// Calls `f` on the string of each `$var = "…";` in `file` (`true`) and
/// each `$var == "…"` or `!=` in a condition (`false`), in order.
fn each_choice_var(file: &mut NssFile, var: &str, f: &mut dyn FnMut(&mut String, bool)) {
    fn compared(e: &mut nss::Expr, var: &str, f: &mut dyn FnMut(&mut String, bool)) {
        use nss::{BinaryOp, Expr};
        if let Expr::Binary { op, left, right } = e {
            if matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
                match (&mut **left, &mut **right) {
                    (Expr::Var(nss::Var::Local(v)), Expr::Str(s)) | (Expr::Str(s), Expr::Var(nss::Var::Local(v))) if v == var => f(s, false),
                    _ => {}
                }
            }
            compared(left, var, f);
            compared(right, var, f);
        }
    }
    each_stmt(file, &mut |s| match s {
        StmtKind::Assign { target: nss::Var::Local(v), index, op: nss::AssignOp::Set, value: Some(nss::Expr::Str(s)) } if v == var && index.is_empty() => f(s, true),
        StmtKind::If { cond, .. } | StmtKind::While { cond, .. } => compared(cond, var, f),
        _ => {}
    });
}

/// Calls `f` on each statement of `file`, a compound one before the ones
/// inside it.
fn each_stmt(file: &mut NssFile, f: &mut dyn FnMut(&mut StmtKind)) {
    fn stmts(list: &mut [Stmt], f: &mut dyn FnMut(&mut StmtKind)) {
        for s in list {
            f(&mut s.kind);
            match &mut s.kind {
                StmtKind::If { then, otherwise, .. } => {
                    stmts(then, f);
                    otherwise.iter_mut().for_each(|o| stmts(o, f));
                }
                StmtKind::While { body, .. } | StmtKind::Select(body) | StmtKind::Case { body, .. } | StmtKind::Block(body) => stmts(body, f),
                _ => {}
            }
        }
    }
    for item in &mut file.items {
        match item {
            Item::Chapter(b) | Item::Scene(b) => stmts(&mut b.body, f),
            Item::Function(fun) => stmts(&mut fun.body, f),
            Item::Include(_) => {}
        }
    }
}

/// Calls `f` on each string argument of the scenes' `SetChoice02` …
/// `SetChoice06` (function_select.nss): the choices' options.
fn each_choice_option(file: &mut NssFile, f: &mut dyn FnMut(&mut String)) {
    each_call(file, &mut |name, args| {
        if name.starts_with("SetChoice0") {
            args.iter_mut().for_each(|a| if let nss::Expr::Str(s) = a { f(s) });
        }
    });
}

/// A stretch's Japanese in the form [`key`] compares.
fn run_key(run: &[Inline]) -> String {
    let jp: String = run
        .iter()
        .filter_map(|i| match i {
            Inline::Str(s) => Some(s.as_str()),
            Inline::Newline => Some("\n"),
            _ => None,
        })
        .collect();
    key(&jp.replace("<?>", ""))
}

/// The stretch with `english` in place of its text, or the stretch as it
/// was if the English doesn't parse.
fn replace(mut run: Vec<Inline>, english: &str, line: u32) -> Result<Vec<Inline>, Vec<Inline>> {
    // ㉑-㉗ are the port's spaces of set widths (its font draws them
    // blank), around italics; Shift-JIS has none of them.
    let english: String = english.chars().map(|c| if ('\u{3251}'..='\u{3257}').contains(&c) { ' ' } else { c }).collect();
    // `<?>` at the end: the text goes on after the code that follows
    // (ma01_026's typewriter), so the code runs before the page.
    let joined = run.iter().rev().find_map(|i| if let Inline::Str(s) = i { Some(s.trim_end().ends_with("<?>")) } else { None });
    let english = if joined == Some(true) { english + "<?>" } else { english };
    let Ok(parsed) = nss::parse_text(&english, line, &mut Vec::new()) else { return Err(run) };
    // Keep the markers and newlines around the stretch; the English
    // takes the middle.
    let lead = run.iter().take_while(|i| matches!(i, Inline::Newline | Inline::Marker(_))).count();
    let tail = run[lead..].iter().rev().take_while(|i| matches!(i, Inline::Newline)).count();
    let tail_items = run.split_off(run.len() - tail);
    run.truncate(lead);
    run.extend(parsed);
    run.extend(tail_items);
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_drops_comments_tags_and_spaces() {
        let jp = "\n//【鈴川】\n<voice name='鈴川' src='voice/ma01/0190010b57'>\n//「永遠に」\n「何故だ」\n\u{3000}<RUBY text='わなな'>戦慄</RUBY>いた。\n";
        assert_eq!(key(jp), "「何故だ」戦慄いた。");
    }

    #[test]
    fn replaces_stretches_between_code() {
        let rows = vec![(key("「……"), "\"...\"".to_string()), (key("ああ」"), "\"Ah.\"".to_string())];
        let english = English { scenes: HashMap::from([("a".to_string(), rows)]), choices: HashMap::new(), choice_vars: HashMap::new(), telops: HashMap::new(), subtitles: HashMap::new(), unread: Metrics::default() };
        let src = "chapter main\n{\n<PRE @box0>\n[text0010]\n「……{WaitKey(1100);}ああ」\n</PRE>\n}\n";
        let mut files = BTreeMap::from([("a.nss".to_string(), nss::parse(src).unwrap())]);
        let report = english.apply(&mut files);
        let Item::Chapter(b) = &files["a.nss"].items[0] else { panic!() };
        let StmtKind::Text(t) = &b.body[0].kind else { panic!("{:?}", b.body[0]) };
        let strs: Vec<_> = t.items.iter().filter_map(|i| if let Inline::Str(s) = i { Some(s.as_str()) } else { None }).collect();
        assert_eq!(strs, ["\"...\"", "\"Ah.\""]);
        assert!(matches!(t.items[0], Inline::Marker(_)));
        assert_eq!(report.replaced, 2);
    }

    #[test]
    fn choice_options_take_their_english() {
        let options = HashMap::from([("お任せします".to_string(), "As you wish.".to_string())]);
        let english = English { scenes: HashMap::new(), choices: HashMap::from([("a".to_string(), options)]), choice_vars: HashMap::new(), telops: HashMap::new(), subtitles: HashMap::new(), unread: Metrics::default() };
        let src = "chapter main\n{\n\tSetChoice02(\"お任せします\",\"お断りします\");\n}\n";
        let mut files = BTreeMap::from([("a.nss".to_string(), nss::parse(src).unwrap())]);
        let report = english.apply(&mut files);
        let Item::Chapter(b) = &files["a.nss"].items[0] else { panic!() };
        let StmtKind::Expr(nss::Expr::Call { args, .. }) = &b.body[0].kind else { panic!("{:?}", b.body[0]) };
        assert_eq!(args[0], nss::Expr::Str("As you wish.".into()));
        assert_eq!((report.options, report.options_missed.len()), (1, 1));
    }

    #[test]
    fn choice_variables_take_the_ports_values_in_order() {
        // The port merged the NSS's first two branches.
        let vars = HashMap::from([("n".to_string(), vec!["Ask".to_string(), "Leave".to_string()])]);
        let english = English { scenes: HashMap::new(), choices: HashMap::new(), choice_vars: HashMap::from([("a".to_string(), HashMap::from([("n".to_string(), vars["n"].clone())]))]), telops: HashMap::new(), subtitles: HashMap::new(), unread: Metrics::default() };
        let src = "chapter main\n{\n\tif($x==1){$n = \"頼む\";}else if($x==2){$n = \"頼む\";}else{$n = \"別れる\";}\n\tif($n==\"別れる\"){$m = \"頼む\";}\n}\n";
        let mut files = BTreeMap::from([("a.nss".to_string(), nss::parse(src).unwrap())]);
        let report = english.apply(&mut files);
        assert_eq!((report.options, report.options_missed.len()), (2, 0));
        let mut seen = Vec::new();
        each_stmt(files.get_mut("a.nss").unwrap(), &mut |s| match s {
            StmtKind::Assign { target, value: Some(nss::Expr::Str(v)), .. } => seen.push(format!("{target}={v}")),
            StmtKind::If { cond: nss::Expr::Binary { right, .. }, .. } => seen.push(format!("{right:?}")),
            _ => {}
        });
        assert_eq!(seen, ["Int(1)", "$n=Ask", "Int(2)", "$n=Ask", "$n=Leave", "Str(\"Leave\")", "$m=頼む"]);
    }

    #[test]
    fn unread_arguments_of_the_ports_calls_are_counted() {
        use squirrel::Arg;
        let mut unread = Metrics::default();
        let mv = [Arg::Str("t".into()), Arg::Int(500), Arg::Int(10), Arg::Int(20), Arg::Other, Arg::Bool(true)];
        note_unread(&mut unread, "Move", &mv, true);
        // Not a telop: the NSS's own Move, nothing taken from the port's.
        note_unread(&mut unread, "Move", &mv, false);
        note_unread(&mut unread, "SetChoice", &[Arg::Int(60), Arg::Str("見る".into()), Arg::Word("Route".into()), Arg::Word("n".into()), Arg::Word("魔".into())], false);
        note_unread(&mut unread, "Fade", &[Arg::Str("t".into()), Arg::Int(500)], true);
        let reasons = unread.reasons();
        let notes: Vec<_> = reasons.iter().map(|(k, &n)| (k.as_str(), n)).collect();
        assert_eq!(unread.totals(), (2, 2));
        assert_eq!(notes, [("Move argument 1 ignored", 1), ("Move argument 4 ignored", 1), ("Move argument 5 ignored", 1), ("SetChoice argument 4 ignored", 1)]);
    }

    #[test]
    fn telops_take_the_ports_places() {
        use squirrel::Arg::{Int, Relative, Str, Word};
        let s = |v: &str| Str(v.to_string());
        let port = telop_calls(vec![
            ("CreateTextureEX".into(), vec![s("t"), Int(100), Word("Center".into()), Word("Middle".into()), s("cg/sys/Telop/outline/a.png")]),
            ("Move".into(), vec![s("t"), Int(0), Relative(0), Relative(-25)]),
            ("Move".into(), vec![s("bg"), Int(0), Int(5), Int(5)]),
        ]);
        assert_eq!(port.len(), 2, "only the telop sprite's calls");
        let english = English { scenes: HashMap::new(), choices: HashMap::new(), choice_vars: HashMap::new(), telops: HashMap::from([("a".to_string(), port)]), subtitles: HashMap::new(), unread: Metrics::default() };
        let src = "chapter main\n{\n\tCreateTextureEX(\"t\", 100, 300, Middle, \"cg/sys/Telop/a.png\");\n\tMove(\"t\", 0, @0, @-40, null, true);\n}\n";
        let mut files = BTreeMap::from([("a.nss".to_string(), nss::parse(src).unwrap())]);
        let report = english.apply(&mut files);
        let Item::Chapter(b) = &files["a.nss"].items[0] else { panic!() };
        let StmtKind::Expr(nss::Expr::Call { args, .. }) = &b.body[0].kind else { panic!() };
        assert_eq!(args[2], nss::Expr::Word("Center".into()));
        assert_eq!(args[4], nss::Expr::Str("cg/sys/Telop/outline/a.png".into()));
        let StmtKind::Expr(nss::Expr::Call { args, .. }) = &b.body[1].kind else { panic!() };
        assert_eq!(args[3], nss::Expr::Unary { op: nss::UnaryOp::Relative, expr: Box::new(nss::Expr::Int(-25)) });
        assert_eq!(args[5], nss::Expr::Word("true".into()));
        assert_eq!((report.telops_placed, report.telops_unpaired.len()), (2, 0));
    }

    #[test]
    fn subtitles_follow_their_voices_start() {
        use squirrel::{Arg, Literal};
        let s = |v: &str| Arg::Str(v.to_string());
        let calls = vec![
            ("CreateVOICE".into(), vec![s("v"), s("ma04/1")]),
            ("MusicStart".into(), vec![s("v"), Arg::Int(0)]),
            ("ShowVOICESEText".into(), vec![s("v")]),
            ("CreateVOICE".into(), vec![s("v"), s("ma04/2")]),
            ("MusicStart".into(), vec![s("v"), Arg::Int(0)]),
            ("ShowVOICESEText".into(), vec![s("v"), Arg::Bool(true)]),
        ];
        let l = |v: &str| Literal::Str(v.to_string());
        let rows = vec![
            ("ma04/1".to_string(), vec![l("<center>Hi!</center>")]),
            ("ma04/2".to_string(), vec![l("time"), Literal::Int(500), l("text"), l("A"), l("x"), Literal::Int(0), l("y"), Literal::Int(13), l("add"), l("time"), Literal::Int(900), l("text"), l("B")]),
        ];
        let subs = scene_subtitles(&calls, &rows);
        assert_eq!(subs.len(), 2);
        assert_eq!((subs[0].n, subs[0].y, subs[0].lines.clone()), (0, 480, vec![(0, "Hi!".to_string())]));
        assert_eq!((subs[1].n, subs[1].add, subs[1].y), (1, true, 13));
        assert_eq!(subs[1].lines, vec![(500, "A".to_string()), (900, "B".to_string())]);
        let english = English { scenes: HashMap::new(), choices: HashMap::new(), choice_vars: HashMap::new(), telops: HashMap::new(), subtitles: HashMap::from([("a".to_string(), subs)]), unread: Metrics::default() };
        let src = "chapter main\n{\n\tMusicStart(\"v\",0);\n\tMusicStart(\"v\",0);\n}\n";
        let mut files = BTreeMap::from([("a.nss".to_string(), nss::parse(src).unwrap())]);
        assert_eq!(english.apply(&mut files).subtitles, 2);
        let Item::Chapter(b) = &files["a.nss"].items[0] else { panic!() };
        let names: Vec<_> = b.body.iter().map(|s| match &s.kind { StmtKind::Expr(nss::Expr::Call { name, .. }) => name.as_str(), _ => "" }).collect();
        assert_eq!(names, ["MusicStart", "ShowVOICESEText", "MusicStart", "ShowVOICESEText"]);
    }

    #[test]
    fn aligns_the_stretches_between_matches() {
        let rows: Vec<_> = ["a", "bc", "d", "e", "f", "g"].iter().map(|s| (s.to_string(), s.to_uppercase())).collect();
        let keys: Vec<_> = ["a", "b", "c", "d", "x", "e", "g", "y"].iter().map(|s| s.to_string()).collect();
        use Pick::*;
        assert_eq!(
            align(&rows, &keys),
            [Row(0), Gap(vec![1]), Gap(vec![]), Row(2), Gap(vec![]), Row(3), Row(5), Gap(vec![])]
        );
    }
}
