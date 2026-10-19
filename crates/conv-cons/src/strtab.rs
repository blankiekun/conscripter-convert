//! Text tables (docs/string-tables.md): the emitter writes each text line
//! as `^ID` and each string players read as `"^ID"`, and the text goes in
//! the table of the script's language (`text.<code>.txt`: `ID<TAB>text`),
//! which CONscripter fills in as it reads the script and swaps on
//! `language`.
//!
//! A line's place is the source instruction it came from (the script's
//! source map), else the last label and its own text; its ID is a hash of
//! the place, its kind and how many lines of that kind came before it
//! there. A rebuild keeps the ID unless the line moves to another source
//! instruction (or, without a map, its text changes).
//!
//! A build with a translation (the same game compiled in another
//! language) fills that language's table by place ([`translated`]); the
//! text window's lines then go in the tables too, as they differ by
//! language.

use std::collections::HashMap;
use std::fmt::Write;

/// Kinds of table lines.
pub const TEXT: char = 't';
pub const CHOICE: char = 'c';
pub const NAME: char = 'n';
pub const LOG: char = 'l';
/// A whole command line holding text (a choice plate's, placed by its width).
pub const PLATE: char = 'p';
pub const WINDOW: char = 'w';
/// A line of code that differs between two editions ([`merge`]).
pub const DIFFERS: char = 'x';

/// One table line: its kind, place, count of its kind at that place, ID
/// and text (a 0.txt line, or a string's contents as written in quotes).
#[derive(Debug, Clone)]
pub struct Row {
    pub kind: char,
    pub place: String,
    pub k: usize,
    pub id: String,
    pub text: String,
}

/// Where the emitter puts text: inline ([`Strings::default`]) or in a table.
#[derive(Default)]
pub struct Strings {
    on: bool,
    /// Window lines go in the table too.
    windows: bool,
    /// The source instruction being emitted (`file:index`); empty: unknown.
    pub at: String,
    /// The last label: with `at` unknown, the place (with the text).
    pub label: String,
    pub rows: Vec<Row>,
    seen: HashMap<(char, String), usize>,
    ids: HashMap<u64, String>,
}

fn fnv64(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for p in parts {
        for b in p.bytes().chain([0]) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

impl Strings {
    /// Text in a table; `windows`: the window lines too.
    pub fn tables(windows: bool) -> Self {
        Strings { on: true, windows, ..Default::default() }
    }

    fn id(&mut self, kind: char, text: &str) -> String {
        let place = match () {
            _ if !self.at.is_empty() => self.at.clone(),
            _ if kind == WINDOW => self.label.clone(),
            _ => format!("{}\u{1}{text}", self.label),
        };
        let k = {
            let n = self.seen.entry((kind, place.clone())).or_insert(0);
            *n += 1;
            *n - 1
        };
        let mut salt = String::new();
        let id = loop {
            let h = fnv64(&[&place, &kind.to_string(), &k.to_string(), &salt]) & 0xffff_ffff_ffff;
            let full = format!("{place}\u{1}{kind}\u{1}{k}\u{1}{salt}");
            match self.ids.get(&h) {
                Some(other) if *other != full => salt.push('+'), // a 48-bit clash: take the next
                _ => {
                    self.ids.insert(h, full);
                    break format!("{h:012x}");
                }
            }
        };
        self.rows.push(Row { kind, place, k, id: id.clone(), text: text.to_string() });
        id
    }

    /// 0.txt lines: each non-empty one `^ID` (window lines only with `windows`).
    pub fn lines(&mut self, kind: char, block: &str) -> String {
        if !self.on || (kind == WINDOW && !self.windows) {
            return block.to_string();
        }
        block.split('\n').map(|l| if l.is_empty() { String::new() } else { format!("^{}", self.id(kind, l)) }).collect::<Vec<_>>().join("\n")
    }

    /// A string's contents (escaped, as written in quotes): `^ID`.
    pub fn string(&mut self, kind: char, s: String) -> String {
        if !self.on || s.is_empty() {
            return s;
        }
        format!("^{}", self.id(kind, &s))
    }
}

/// The table of `rows`.
pub fn table(rows: &[Row]) -> String {
    let mut out = String::new();
    for r in rows {
        writeln!(out, "{}\t{}", r.id, r.text).unwrap();
    }
    out
}

/// The table of another language for `rows`, from its own build's rows
/// (`other`): each place's k-th row takes the other's k-th of the same
/// kind; the last text line of a place takes the text lines left over,
/// rows past the other's are empty; a place the other lacks keeps its
/// own text. Returns the table and the count of rows kept so.
pub fn translated(rows: &[Row], other: &[Row]) -> (String, usize) {
    let mut by_place: HashMap<(char, &str), Vec<&str>> = HashMap::new();
    for r in other {
        by_place.entry((r.kind, &r.place)).or_default().push(&r.text);
    }
    let mut count: HashMap<(char, &str), usize> = HashMap::new();
    for r in rows {
        *count.entry((r.kind, &r.place)).or_default() += 1;
    }
    let (mut out, mut kept) = (String::new(), 0);
    for r in rows {
        let key = (r.kind, r.place.as_str());
        let text = match by_place.get(&key) {
            None => {
                kept += 1;
                r.text.clone()
            }
            Some(o) if r.kind == TEXT && r.k + 1 == count[&key] && o.len() > r.k => o[r.k..].concat(),
            Some(o) => o.get(r.k).map_or(String::new(), |s| s.to_string()),
        };
        writeln!(out, "{}\t{text}", r.id).unwrap();
    }
    (out, kept)
}

/// One edition's emitted script for [`merge`]: the script, its line map
/// ([`crate::Emitted::lines`]) and its rows.
pub struct Edition<'a> {
    pub script: &'a str,
    pub lines: &'a [(usize, Option<conv_core::ir::SourceLoc>)],
    pub rows: &'a [Row],
}

/// What [`merge`] made.
#[derive(Debug, Default)]
pub struct Merged {
    pub script: String,
    /// The base's table, then the other's.
    pub tables: (String, String),
    /// Text rows whose place the other lacks (they keep the base's text).
    pub kept: usize,
    /// Runs of lines from one source instruction whose code differs (or
    /// that only one edition has), and their lines, now table lines.
    pub blocks: usize,
    pub lines: usize,
}

/// A script's runs of lines by source instruction: (place, lines).
fn runs<'a>(e: &Edition<'a>) -> Vec<(String, Vec<&'a str>)> {
    let lines: Vec<&str> = e.script.lines().collect();
    let mut starts: Vec<(usize, String)> = e.lines.iter().map(|(n, at)| (n - 1, at.as_ref().map_or("-".to_string(), |a| format!("{}:{}", a.file, a.index)))).collect();
    starts.insert(0, (0, "<start>".to_string()));
    let mut out: Vec<(String, Vec<&str>)> = Vec::new();
    for (i, (from, place)) in starts.iter().enumerate() {
        let to = starts.get(i + 1).map_or(lines.len(), |s| s.0).min(lines.len());
        let part = &lines[(*from).min(to)..to];
        match out.last_mut() {
            Some(last) if last.0 == *place => last.1.extend_from_slice(part),
            _ if part.is_empty() => {}
            _ => out.push((place.clone(), part.to_vec())),
        }
    }
    out
}

/// Pairs the runs of `a` and `b` by place, in order: (Some, Some) for a
/// pair, else a run only one has. Greedy: after a mismatch the nearest
/// place both have (within a window) resumes the pairing.
fn align(a: &[(String, Vec<&str>)], b: &[(String, Vec<&str>)]) -> Vec<(Option<usize>, Option<usize>)> {
    const WINDOW: usize = 400;
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i].0 == b[j].0 {
            out.push((Some(i), Some(j)));
            (i, j) = (i + 1, j + 1);
            continue;
        }
        let resync = (1..WINDOW).find_map(|d| (0..=d).find(|&x| i + x < a.len() && j + d - x < b.len() && a[i + x].0 == b[j + d - x].0).map(|x| (x, d - x)));
        let (x, y) = resync.unwrap_or((a.len() - i, b.len() - j));
        out.extend((i..i + x).map(|k| (Some(k), None)));
        out.extend((j..j + y).map(|k| (None, Some(k))));
        (i, j) = (i + x, j + y);
    }
    out
}

/// Two editions of one game in one script (the user's decision,
/// 2026-10-18: each language plays exactly like its original): the text
/// by ID as [`translated`], and each run of lines from one source
/// instruction whose code differs between them (or that only one has) as
/// per-language table lines, the shorter padded with empty lines. The
/// `language` lines stay in the script (the engine reads the first).
pub fn merge(base: Edition, other: Edition) -> Merged {
    let is_ref = |l: &str| l.strip_prefix('^').is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
    // a run's code with its text lines' places in it
    let code = |run: &[&str]| -> Vec<String> { run.iter().map(|l| if is_ref(l) { "^".to_string() } else { l.to_string() }).collect() };
    let refs = |run: &[&str]| -> Vec<String> { run.iter().filter(|l| is_ref(l)).map(|l| l[1..].to_string()).collect() };
    let own = (table(base.rows), table(other.rows));
    let (other_table, kept) = translated(base.rows, other.rows);
    // the other's text of each base row: by place ([`translated`]), then
    // where a run pairs up, its k-th line and the strings of its code
    let mut other_text: HashMap<String, String> = other_table.lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let own_other: HashMap<&str, &str> = other.rows.iter().map(|r| (r.id.as_str(), r.text.as_str())).collect();
    let mut differs = (String::new(), String::new());
    let (ra, rb) = (runs(&base), runs(&other));
    let mut ids = Strings::tables(true);
    for r in base.rows {
        ids.ids.insert(u64::from_str_radix(&r.id, 16).unwrap_or(0), String::new());
    }
    let mut m = Merged { kept, ..Default::default() };
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (x, y) in align(&ra, &rb) {
        let a = x.map_or(&[][..], |i| &ra[i].1[..]);
        let b = y.map_or(&[][..], |j| &rb[j].1[..]);
        let (ca, ka, kb) = (code(a), refs(a), refs(b));
        if x.is_some() && y.is_some() && ca == code(b) {
            for (ia, ib) in ka.iter().zip(&kb) {
                if let Some(t) = own_other.get(ib.as_str()) {
                    other_text.insert(ia.clone(), t.to_string());
                }
            }
            for l in &ca {
                for id in l.split("\"^").skip(1).map(|r| r.split('"').next().unwrap_or("")) {
                    if let Some(t) = own_other.get(id) {
                        other_text.insert(id.to_string(), t.to_string());
                    }
                }
            }
            for l in a {
                m.script.push_str(l);
                m.script.push('\n');
            }
            continue;
        }
        let place = x.map_or(&rb[y.unwrap_or(0)].0, |i| &ra[i].0);
        let n = seen.entry(place.clone()).or_insert(0);
        *n += 1;
        ids.at = format!("{place}\u{1}{n}");
        let filled = |run: &[&str], t: &str| -> Vec<String> { if run.is_empty() { vec![] } else { fill(&run.join("\n"), t).lines().map(str::to_string).collect() } };
        let (mut fa, mut fb) = (filled(a, &own.0), filled(b, &own.1));
        m.blocks += 1;
        // the `language` line stays in the script, where the base has it
        let is_language = |l: &String| l.trim_start().starts_with("language \"");
        let (ia, ib) = (fa.iter().position(is_language), fb.iter().position(is_language));
        let (tail_a, tail_b) = match (ia, ib) {
            (Some(i), Some(j)) => (fa.split_off(i + 1), fb.split_off(j + 1)),
            _ => (vec![], vec![]),
        };
        let language = ia.zip(ib).map(|_| (fa.pop().unwrap_or_default(), fb.pop()));
        for (part_a, part_b, after) in [(&fa, &fb, language.as_ref().map(|l| &l.0)), (&tail_a, &tail_b, None)] {
            for k in 0..part_a.len().max(part_b.len()) {
                let (la, lb) = (part_a.get(k).map_or("", String::as_str), part_b.get(k).map_or("", String::as_str));
                let id = ids.id(DIFFERS, la);
                writeln!(m.script, "^{id}").unwrap();
                writeln!(differs.0, "{id}\t{la}").unwrap();
                writeln!(differs.1, "{id}\t{lb}").unwrap();
                m.lines += 1;
            }
            if let Some(l) = after {
                writeln!(m.script, "{l}").unwrap();
            }
        }
    }
    let mut en = String::new();
    for r in base.rows {
        writeln!(en, "{}\t{}", r.id, other_text.get(&r.id).map_or("", String::as_str)).unwrap();
    }
    m.tables = (own.0 + &differs.0, en + &differs.1);
    m
}

/// The language a script's text starts in: its first `language "xx"`.
pub fn first_language(script: &str) -> Option<&str> {
    script.lines().find_map(|l| l.trim_start().strip_prefix("language \"")?.split('"').next())
}

/// `script` with its `^ID` lines and `"^ID"` strings filled from `table`,
/// as CONscripter reads it (for lints and diffs).
pub fn fill(script: &str, table: &str) -> String {
    let t: HashMap<&str, &str> = table.lines().filter(|l| !l.starts_with(';')).filter_map(|l| l.split_once('\t')).collect();
    let is_id = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    let mut out = String::with_capacity(script.len() * 2);
    for line in script.lines() {
        let s = line.trim_start_matches([' ', '\t']);
        if let Some(id) = s.strip_prefix('^').filter(|id| is_id(id)) {
            out.push_str(t.get(id).copied().unwrap_or_default());
        } else if s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && s.contains("\"^") {
            let mut rest = line;
            while let Some(i) = rest.find("\"^") {
                let after = &rest[i + 2..];
                let end = after.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(after.len());
                out.push_str(&rest[..i + 1]);
                match t.get(&after[..end]).filter(|_| after[end..].starts_with('"')) {
                    Some(text) => out.push_str(text),
                    None => out.push_str(&rest[i + 1..i + 2 + end]),
                }
                rest = &after[end..];
            }
            out.push_str(rest);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_by_place_and_translation() {
        let mut ja = Strings::tables(true);
        ja.at = "a.nss:1".into();
        let page = ja.lines(TEXT, "　一行目\n二行目\\");
        let opts = [ja.string(CHOICE, "はい".into()), ja.string(CHOICE, "いいえ".into())];
        let win = ja.lines(WINDOW, "setwindow3 1");
        ja.at = "a.nss:2".into();
        let again = ja.lines(TEXT, "　一行目\\");
        let script = format!("{page}\nselect \"{}\",*x,\"{}\",*y\n{win}\n{again}\n", opts[0], opts[1]);
        assert!(script.lines().take(2).all(|l| l.starts_with('^')));
        assert_ne!(script.lines().next(), script.lines().last(), "the same text in two places: two IDs");
        let mut en = Strings::tables(true);
        en.at = "a.nss:1".into();
        en.lines(TEXT, "`One line.\\");
        en.string(CHOICE, "Yes".into());
        en.string(CHOICE, "No".into());
        en.lines(WINDOW, "setwindow3 2");
        let (en_t, kept) = translated(&ja.rows, &en.rows);
        assert_eq!(kept, 1, "a.nss:2 has no English");
        let filled = fill(&script, &en_t);
        assert_eq!(filled, "`One line.\\\n\nselect \"Yes\",*x,\"No\",*y\nsetwindow3 2\n　一行目\\\n");
        assert_eq!(fill(&script, &table(&ja.rows)), "　一行目\n二行目\\\nselect \"はい\",*x,\"いいえ\",*y\nsetwindow3 1\n　一行目\\\n");
        // inline: nothing changes; without `windows` the window stays in the script
        assert_eq!(Strings::default().lines(TEXT, "あ\\"), "あ\\");
        assert_eq!(Strings::tables(false).lines(WINDOW, "setwindow3 1"), "setwindow3 1");
        assert_eq!(first_language("*define\nlanguage \"en\"\ngame\n"), Some("en"));
    }

    #[test]
    fn merge_keeps_each_edition() {
        use conv_core::ir::SourceLoc;
        let at = |i| Some(SourceLoc { file: "a.nss".into(), index: i });
        // place 1: same code, text by ID; place 2: code differs; place 3: only the English has it
        let mut ja = Strings::tables(false);
        ja.at = "a.nss:1".into();
        let t1 = ja.lines(TEXT, "　一\\");
        ja.at = "a.nss:2".into();
        let t2 = ja.lines(TEXT, "　二\\");
        let ja_script = format!("*define\nlanguage \"ja\"\ngame\n*start\nbg 1\n{t1}\nlsp 1,\"a.png\",10,0\n{t2}\nend\n");
        let ja_lines = vec![(5, at(1)), (7, at(2)), (9, None)];
        let mut en = Strings::tables(false);
        en.at = "a.nss:1".into();
        let e1 = en.lines(TEXT, "`One\\");
        en.at = "a.nss:2".into();
        let e2 = en.lines(TEXT, "`Two\\");
        let en_script = format!("*define\nlanguage \"en\"\ngame\n*start\nbg 1\n{e1}\nlsp 1,\"a.png\",99,0\n*en_only\n{e2}\nquake 1\nend\n");
        let en_lines = vec![(5, at(1)), (7, at(2)), (10, at(3)), (11, None)];
        let m = merge(Edition { script: &ja_script, lines: &ja_lines, rows: &ja.rows }, Edition { script: &en_script, lines: &en_lines, rows: &en.rows });
        let blank = |s: String| s.lines().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n") + "\n";
        assert_eq!(blank(fill(&m.script, &m.tables.0)), fill(&ja_script, &table(&ja.rows)));
        assert_eq!(blank(fill(&m.script, &m.tables.1)).replace("language \"ja\"", "language \"en\""), fill(&en_script, &table(&en.rows)));
        assert!(m.script.contains("language \"ja\"") && m.script.contains("bg 1") && m.script.contains(&t1));
        assert_eq!(m.blocks, 3, "the define (its language line), place 2 and place 3");
    }

    #[test]
    fn without_a_map_the_text_is_the_place() {
        let mut s = Strings::tables(false);
        s.label = "f_a".into();
        let a = s.lines(TEXT, "「はい」\\");
        let b = s.lines(TEXT, "「はい」\\");
        assert_ne!(a, b);
        let mut again = Strings::tables(false);
        again.label = "f_a".into();
        again.lines(TEXT, "新しい行\\");
        assert_eq!(again.lines(TEXT, "「はい」\\"), a, "a line added before keeps the others' IDs");
    }
}
