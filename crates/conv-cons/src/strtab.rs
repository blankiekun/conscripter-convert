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
