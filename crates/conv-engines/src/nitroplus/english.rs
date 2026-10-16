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
                .flat_map(|(id, jp)| jp.into_iter().enumerate().map(move |(n, jp)| (format!("{id}_{}", n + 1), jp)))
                .filter_map(|(k, jp)| Some((jp, options.get(&k)?.clone())))
                .collect();
            choices.insert(scene.to_lowercase(), scene_choices);
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
        Ok(English { scenes, choices })
    }

    /// Puts the English into every text block of `files` (by file name,
    /// `ma01_019.nss`).
    pub fn apply(&self, files: &mut BTreeMap<String, NssFile>) -> Report {
        let mut report = Report::default();
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

/// Calls `f` on each string argument of the scenes' `SetChoice02` …
/// `SetChoice06` (function_select.nss): the choices' options.
fn each_choice_option(file: &mut NssFile, f: &mut dyn FnMut(&mut String)) {
    fn stmts(list: &mut [Stmt], f: &mut dyn FnMut(&mut String)) {
        for s in list {
            match &mut s.kind {
                StmtKind::If { then, otherwise, .. } => {
                    stmts(then, f);
                    otherwise.iter_mut().for_each(|o| stmts(o, f));
                }
                StmtKind::While { body, .. } | StmtKind::Select(body) | StmtKind::Case { body, .. } | StmtKind::Block(body) => stmts(body, f),
                StmtKind::Expr(nss::Expr::Call { name, args }) if name.starts_with("SetChoice0") => {
                    args.iter_mut().for_each(|a| if let nss::Expr::Str(s) = a { f(s) });
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
        let english = English { scenes: HashMap::from([("a".to_string(), rows)]), choices: HashMap::new() };
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
        let english = English { scenes: HashMap::new(), choices: HashMap::from([("a".to_string(), options)]) };
        let src = "chapter main\n{\n\tSetChoice02(\"お任せします\",\"お断りします\");\n}\n";
        let mut files = BTreeMap::from([("a.nss".to_string(), nss::parse(src).unwrap())]);
        let report = english.apply(&mut files);
        let Item::Chapter(b) = &files["a.nss"].items[0] else { panic!() };
        let StmtKind::Expr(nss::Expr::Call { args, .. }) = &b.body[0].kind else { panic!("{:?}", b.body[0]) };
        assert_eq!(args[0], nss::Expr::Str("As you wish.".into()));
        assert_eq!((report.options, report.options_missed.len()), (1, 1));
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
