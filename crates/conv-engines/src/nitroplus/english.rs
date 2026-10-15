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
}

#[derive(Debug, Default)]
pub struct Report {
    pub replaced: usize,
    /// (file, Japanese key) of the stretches left in Japanese.
    pub missed: Vec<(String, String)>,
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
        for e in &scripts.entries {
            let Some(scene) = e.name.strip_prefix("media/script/nut/").and_then(|n| n.strip_suffix(".nut")) else { continue };
            let Some(lang) = scripts.find(&format!("media/script/lang/en/{scene}.nut")) else { continue };
            let english: HashMap<i32, String> = load(lang)?.table_rows("TransText").into_iter().collect();
            let mut prints = Vec::new();
            load(e)?.calls_int_str("Print", &mut prints);
            let rows = prints
                .into_iter()
                .filter_map(|(k, jp)| Some((key(&jp), english.get(&k)?.replace("\r\n", "\n"))))
                .collect();
            scenes.insert(scene.to_lowercase(), rows);
        }
        Ok(English { scenes })
    }

    /// Puts the English into every text block of `files` (by file name,
    /// `ma01_019.nss`).
    pub fn apply(&self, files: &mut BTreeMap<String, NssFile>) -> Report {
        let mut report = Report::default();
        for (name, file) in files.iter_mut() {
            let Some(rows) = name.to_lowercase().strip_suffix(".nss").and_then(|s| self.scenes.get(s)) else { continue };
            let mut scene = Scene { rows, next: 0, file: name, report: &mut report };
            for item in &mut file.items {
                match item {
                    Item::Chapter(b) | Item::Scene(b) => scene.stmts(&mut b.body),
                    Item::Function(f) => scene.stmts(&mut f.body),
                    Item::Include(_) => {}
                }
            }
        }
        report
    }
}

struct Scene<'a> {
    rows: &'a [(String, String)],
    /// The row after the last one used: equal Japanese lines take their
    /// English in order.
    next: usize,
    file: &'a str,
    report: &'a mut Report,
}

impl Scene<'_> {
    fn stmts(&mut self, stmts: &mut [Stmt]) {
        for s in stmts {
            match &mut s.kind {
                StmtKind::If { then, otherwise, .. } => {
                    self.stmts(then);
                    otherwise.iter_mut().for_each(|o| self.stmts(o));
                }
                StmtKind::While { body, .. } | StmtKind::Select(body) | StmtKind::Case { body, .. } | StmtKind::Block(body) => self.stmts(body),
                StmtKind::Text(t) => {
                    let items = std::mem::take(&mut t.items);
                    t.items = self.text(items, s.line);
                }
                _ => {}
            }
        }
    }

    fn find(&mut self, jp: &str) -> Option<&str> {
        let n = self.rows.len();
        let i = (self.next..n).chain(0..self.next).find(|&i| self.rows[i].0 == jp)?;
        self.next = i + 1;
        Some(&self.rows[i].1)
    }

    /// The block's items with each stretch between code replaced.
    fn text(&mut self, items: Vec<Inline>, line: u32) -> Vec<Inline> {
        let mut out = Vec::with_capacity(items.len());
        let mut run = Vec::new();
        for item in items.into_iter().chain([Inline::Code(Vec::new())]) {
            if matches!(item, Inline::Code(_)) {
                self.run(std::mem::take(&mut run), line, &mut out);
                out.push(item);
            } else {
                run.push(item);
            }
        }
        out.pop();
        out
    }

    fn run(&mut self, run: Vec<Inline>, line: u32, out: &mut Vec<Inline>) {
        let jp: String = run
            .iter()
            .filter_map(|i| match i {
                Inline::Str(s) => Some(s.as_str()),
                Inline::Newline => Some("\n"),
                _ => None,
            })
            .collect::<String>()
            .replace("<?>", "");
        let jp = key(&jp);
        if jp.is_empty() {
            out.extend(run);
            return;
        }
        let Some(english) = self.find(&jp).map(str::to_string) else {
            self.report.missed.push((self.file.to_string(), jp));
            out.extend(run);
            return;
        };
        // ㉑-㉗ are the port's spaces of set widths (its font draws them
        // blank), around italics; Shift-JIS has none of them.
        let english: String = english.chars().map(|c| if ('\u{3251}'..='\u{3257}').contains(&c) { ' ' } else { c }).collect();
        let Ok(parsed) = nss::parse_text(&english, line, &mut Vec::new()) else {
            self.report.missed.push((self.file.to_string(), jp));
            out.extend(run);
            return;
        };
        // Keep the markers and newlines around the stretch; the English
        // takes the middle.
        let lead = run.iter().take_while(|i| matches!(i, Inline::Newline | Inline::Marker(_))).count();
        let tail = run[lead..].iter().rev().take_while(|i| matches!(i, Inline::Newline)).count();
        let mut run = run;
        let tail_items = run.split_off(run.len() - tail);
        run.truncate(lead);
        out.extend(run);
        out.extend(parsed);
        out.extend(tail_items);
        self.report.replaced += 1;
    }
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
        let mut report = Report::default();
        let mut scene = Scene { rows: &rows, next: 0, file: "a.nss", report: &mut report };
        let src = "chapter main\n{\n<PRE @box0>\n[text0010]\n「……{WaitKey(1100);}ああ」\n</PRE>\n}\n";
        let mut file = nss::parse(src).unwrap();
        let Item::Chapter(b) = &mut file.items[0] else { panic!() };
        let stmts = &mut b.body;
        scene.stmts(stmts);
        let StmtKind::Text(t) = &stmts[0].kind else { panic!("{:?}", stmts[0]) };
        let strs: Vec<_> = t.items.iter().filter_map(|i| if let Inline::Str(s) = i { Some(s.as_str()) } else { None }).collect();
        assert_eq!(strs, ["\"...\"", "\"Ah.\""]);
        assert!(matches!(t.items[0], Inline::Marker(_)));
        assert_eq!(report.replaced, 2);
    }
}
