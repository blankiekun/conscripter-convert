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
            Some(o) if r.kind == TEXT && r.k + 1 == count[&key] && o.len() > r.k => join_text_lines(&o[r.k..]),
            Some(o) => o.get(r.k).map_or(String::new(), |s| s.to_string()),
        };
        writeln!(out, "{}\t{text}", r.id).unwrap();
    }
    (out, kept)
}

/// Text lines as one line, shown as the lines one by one would be
/// (checked in the engine: each string's end makes the break its line's
/// end made). A single-byte line (`` `text ``) is closed, else the next
/// one's backquote would close it and its text be read as a command, and
/// the next follows after `:`; an empty one (`` ` ``, two breaks) becomes
/// `br:br`; after a `` `/ `` the next one's text goes on in the same
/// string. Two-byte lines run to the line's end (`:` would be text): they
/// join as they are.
fn join_text_lines(lines: &[&str]) -> String {
    let mut out = String::new();
    let mut single = false; // `out` ends in a single-byte line
    for l in lines {
        if single && out.ends_with("`/") && l.starts_with('`') {
            out.truncate(out.len() - 2);
            out.push_str(&l[1..]);
            continue;
        }
        if single {
            if out.matches('`').count() % 2 == 1 {
                out.push('`');
            }
            out.push(':');
        }
        single = l.starts_with('`');
        out.push_str(if *l == "`" { "br:br" } else { l });
    }
    out
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
    /// The merged script's line (1-based) of each base line, by its
    /// index: the base's `--source-map` lines moved to the merged script.
    pub at: Vec<usize>,
}

/// A script's runs of lines by source instruction: (place, lines).
/// The define section's run ([`merge`] joins both editions').
const DEFINE: &str = "<define>";

fn runs<'a>(e: &Edition<'a>) -> Vec<(String, Vec<&'a str>)> {
    let lines: Vec<&str> = e.script.lines().collect();
    let mut starts: Vec<(usize, String)> = e.lines.iter().map(|(n, at)| (n - 1, at.as_ref().map_or("-".to_string(), |a| format!("{}:{}", a.file, a.index)))).collect();
    starts.insert(0, (0, "<start>".to_string()));
    // the define section (through `game`) is a run of its own
    if let Some(game) = lines.iter().position(|l| *l == "game") {
        starts.insert(1, (game + 1, "<start>".to_string()));
        starts[0].1 = DEFINE.to_string();
    }
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

/// The plain code lines `a` and `b` both have, in order: (index in a,
/// index in b). A longest common subsequence where the block is small;
/// a larger one (past 32M cells, 128 MB; the boot code is 16M) is cut at
/// the lines each side has once, in the same order (patience diff).
fn common_lines(a: &[(String, bool)], b: &[(String, bool)]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    common_in(a, b, (0, a.len()), (0, b.len()), &mut out);
    out
}

fn common_in(a: &[(String, bool)], b: &[(String, bool)], (mut a0, mut a1): (usize, usize), (mut b0, mut b1): (usize, usize), out: &mut Vec<(usize, usize)>) {
    const LIMIT: usize = 1 << 25;
    let same = |i: usize, j: usize| a[i].1 && b[j].1 && a[i].0 == b[j].0;
    while a0 < a1 && b0 < b1 && same(a0, b0) {
        out.push((a0, b0));
        (a0, b0) = (a0 + 1, b0 + 1);
    }
    let mut tail = Vec::new();
    while a0 < a1 && b0 < b1 && same(a1 - 1, b1 - 1) {
        (a1, b1) = (a1 - 1, b1 - 1);
        tail.push((a1, b1));
    }
    let (n, m) = (a1 - a0, b1 - b0);
    if n * m <= LIMIT {
        // len[i][j]: the longest common run of a[a0 + i..a1] and b[b0 + j..b1]
        let w = m + 1;
        let mut len = vec![0u32; (n + 1) * w];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                len[i * w + j] = if same(a0 + i, b0 + j) { len[(i + 1) * w + j + 1] + 1 } else { len[(i + 1) * w + j].max(len[i * w + j + 1]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if same(a0 + i, b0 + j) {
                out.push((a0 + i, b0 + j));
                (i, j) = (i + 1, j + 1);
            } else if len[(i + 1) * w + j] >= len[i * w + j + 1] {
                i += 1;
            } else {
                j += 1;
            }
        }
    } else {
        // lines once on each side, then the longest run of them in order
        let mut count: HashMap<&str, (usize, usize, usize)> = HashMap::new();
        for i in (a0..a1).filter(|&i| a[i].1) {
            let e = count.entry(&a[i].0).or_insert((0, 0, 0));
            (e.0, e.2) = (e.0 + 1, i);
        }
        let mut pairs = Vec::new();
        for j in (b0..b1).filter(|&j| b[j].1) {
            if let Some(e) = count.get_mut(b[j].0.as_str()) {
                e.1 += 1;
            }
        }
        for j in (b0..b1).filter(|&j| b[j].1) {
            if let Some(&(1, 1, i)) = count.get(b[j].0.as_str()) {
                pairs.push((i, j));
            }
        }
        // longest increasing run of a's index (pairs are in b's order)
        let (mut tops, mut prev): (Vec<usize>, Vec<Option<usize>>) = (Vec::new(), vec![None; pairs.len()]);
        for k in 0..pairs.len() {
            let at = tops.partition_point(|&t| pairs[t].0 < pairs[k].0);
            prev[k] = at.checked_sub(1).map(|p| tops[p]);
            if at == tops.len() {
                tops.push(k);
            } else {
                tops[at] = k;
            }
        }
        let mut anchors = Vec::new();
        let mut k = tops.last().copied();
        while let Some(x) = k {
            anchors.push(pairs[x]);
            k = prev[x];
        }
        anchors.reverse();
        let (mut i, mut j) = (a0, b0);
        for (ai, bj) in anchors {
            common_in(a, b, (i, ai), (j, bj), out);
            out.push((ai, bj));
            (i, j) = (ai + 1, bj + 1);
        }
        // past the last anchor (or with none): only if small enough now
        if (a1 - i) * (b1 - j) <= LIMIT {
            common_in(a, b, (i, a1), (j, b1), out);
        }
    }
    out.extend(tail.into_iter().rev());
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
    // read once: rebuilding them per block made the JP+EN merge take ~17 min
    let own_ids = (ids_of(&own.0), ids_of(&own.1));
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
    // the merged script's lines so far ([`Merged::at`])
    let mut out = 0;
    // runs that don't pair up, side by side, are one block: the code both
    // have in them stays in the script ([`common_lines`])
    type Group<'s> = (Option<usize>, Option<usize>, Vec<&'s str>, Vec<&'s str>);
    let mut groups: Vec<Group> = Vec::new();
    let mut alone = false;
    for (x, y) in align(&ra, &rb) {
        match groups.last_mut() {
            Some(g) if alone && (x.is_none() || y.is_none()) => {
                g.0 = g.0.or(x);
                g.1 = g.1.or(y);
                g.2.extend(x.map_or(vec![], |i| ra[i].1.clone()));
                g.3.extend(y.map_or(vec![], |j| rb[j].1.clone()));
            }
            _ => groups.push((x, y, x.map_or(vec![], |i| ra[i].1.clone()), y.map_or(vec![], |j| rb[j].1.clone()))),
        }
        alone = x.is_none() || y.is_none();
    }
    for (x, y, a, b) in groups {
        let (a, b) = (&a[..], &b[..]);
        let (ca, ka, kb) = (code(a), refs(a), refs(b));
        // the define section runs once, at start, whatever the language:
        // both editions' declarations (their `defvar`s), the base's `language`
        if x.is_some_and(|i| ra[i].0 == DEFINE) && y.is_some() {
            let is_language = |l: &&str| l.trim_start().starts_with("language \"");
            let extra: Vec<&&str> = b.iter().filter(|l| !is_language(l) && **l != "game" && !a.contains(l)).collect();
            for l in a {
                if *l == "game" {
                    extra.iter().for_each(|e| writeln!(m.script, "{e}").unwrap());
                    out += extra.len();
                }
                writeln!(m.script, "{l}").unwrap();
                out += 1;
                m.at.push(out);
            }
            continue;
        }
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
                out += 1;
                m.at.push(out);
            }
            continue;
        }
        let place = x.map_or(&rb[y.unwrap_or(0)].0, |i| &ra[i].0);
        let n = seen.entry(place.clone()).or_insert(0);
        *n += 1;
        ids.at = format!("{place}\u{1}{n}");
        // each line filled, and whether it is plain code (no text by ID)
        let filled = |run: &[&str], t| -> Vec<(String, bool)> {
            if run.is_empty() {
                return vec![];
            }
            fill_from(&run.join("\n"), t).lines().zip(run).map(|(l, raw)| (l.to_string(), !raw.contains('^'))).collect()
        };
        let (mut fa, mut fb) = (filled(a, &own_ids.0), filled(b, &own_ids.1));
        m.blocks += 1;
        // the `language` line stays in the script, where the base has it
        let is_language = |l: &(String, bool)| l.0.trim_start().starts_with("language \"");
        let (ia, ib) = (fa.iter().position(is_language), fb.iter().position(is_language));
        let (tail_a, tail_b) = match (ia, ib) {
            (Some(i), Some(j)) => (fa.split_off(i + 1), fb.split_off(j + 1)),
            _ => (vec![], vec![]),
        };
        let language = ia.zip(ib).map(|_| (fa.pop().unwrap_or_default().0, fb.pop()));
        for (part_a, part_b, after) in [(&fa, &fb, language.as_ref().map(|l| &l.0)), (&tail_a, &tail_b, None)] {
            // Code both have stays in the script, in order (labels, jumps:
            // what reads 0.txt without the tables still finds them); only
            // the lines between differ per language.
            let (mut i, mut j) = (0, 0);
            for (ci, cj) in common_lines(part_a, part_b).into_iter().chain([(part_a.len(), part_b.len())]) {
                // the base's lines i..ci are the block's first rows
                m.at.extend((1..=ci - i).map(|k| out + k));
                for k in 0..(ci - i).max(cj - j) {
                    let (la, lb) = (part_a.get(i + k).filter(|_| i + k < ci), part_b.get(j + k).filter(|_| j + k < cj));
                    let (la, lb) = (la.map_or("", |l| l.0.as_str()), lb.map_or("", |l| l.0.as_str()));
                    let id = ids.id(DIFFERS, la);
                    writeln!(m.script, "^{id}").unwrap();
                    writeln!(differs.0, "{id}\t{la}").unwrap();
                    writeln!(differs.1, "{id}\t{lb}").unwrap();
                    m.lines += 1;
                    out += 1;
                }
                if let Some(l) = part_a.get(ci) {
                    writeln!(m.script, "{}", l.0).unwrap();
                    out += 1;
                    m.at.push(out);
                }
                (i, j) = (ci + 1, cj + 1);
            }
            if let Some(l) = after {
                writeln!(m.script, "{l}").unwrap();
                out += 1;
                m.at.push(out);
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
    fill_from(script, &ids_of(table))
}

/// A table's text by id, for [`fill_from`].
fn ids_of(table: &str) -> HashMap<&str, &str> {
    table.lines().filter(|l| !l.starts_with(';')).filter_map(|l| l.split_once('\t')).collect()
}

/// [`fill`] with the table already read ([`ids_of`]).
fn fill_from(script: &str, t: &HashMap<&str, &str>) -> String {
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
    fn single_byte_lines_join_closed() {
        assert_eq!(join_text_lines(&["`!cWhere there are demons,", "`I slay them.\\", "`", "`Next@", "`F`/", "`go\\"]), "`!cWhere there are demons,`:`I slay them.\\`:br:br:`Next@`:`Fgo\\");
        assert_eq!(join_text_lines(&["一@", "二\\"]), "一@二\\");
    }

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
        let en_script = format!("*define\nlanguage \"en\"\ndefvar a\ngame\n*start\nbg 1\n{e1}\nlsp 1,\"a.png\",99,0\n*en_only\n{e2}\nquake 1\nend\n");
        let en_lines = vec![(6, at(1)), (8, at(2)), (11, at(3)), (12, None)];
        let m = merge(Edition { script: &ja_script, lines: &ja_lines, rows: &ja.rows }, Edition { script: &en_script, lines: &en_lines, rows: &en.rows });
        let blank = |s: String| s.lines().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n") + "\n";
        // the define section holds both editions' declarations
        let with_both = |s: String| s.replace("game\n", "defvar a\ngame\n");
        assert_eq!(blank(fill(&m.script, &m.tables.0)), with_both(fill(&ja_script, &table(&ja.rows))));
        assert_eq!(blank(fill(&m.script, &m.tables.1)).replace("language \"ja\"", "language \"en\""), fill(&en_script, &table(&en.rows)));
        assert!(m.script.contains("language \"ja\"") && m.script.contains("bg 1") && m.script.contains(&t1));
        assert_eq!(m.blocks, 2, "place 2 and place 3");
        // each Japanese line lands on the merged line that holds it (the source map's lines)
        let merged: Vec<String> = fill(&m.script, &m.tables.0).lines().map(String::from).collect();
        let base: Vec<String> = fill(&ja_script, &table(&ja.rows)).lines().map(String::from).collect();
        assert_eq!(m.at.len(), base.len());
        for (k, l) in base.iter().enumerate() {
            assert_eq!(&merged[m.at[k] - 1], l, "line {}", k + 1);
        }
    }

    #[test]
    fn a_differing_choice_keeps_its_labels_in_the_script() {
        // ma02_014: the choice's plates differ by language; its labels and
        // jumps stayed only in the tables, so nstrace and engrun, which
        // read 0.txt, found `goto *c_end` without `*c_end`
        use conv_core::ir::SourceLoc;
        let at = |i| Some(SourceLoc { file: "a.nss".into(), index: i });
        let choice = |plate: &str| format!("*start\n{plate}\nbtnwait %9\nif %9 == 1 goto *c_0\n*c_0\ngoto *c_end\n*c_end\nend\n");
        let (ja_script, en_script) = (choice("vstrsp 1,\"はい\",9,9"), choice("strsp 1,\"Yes\",5,5"));
        let lines = vec![(2, at(1)), (8, None)];
        let none: Vec<Row> = vec![];
        let m = merge(Edition { script: &ja_script, lines: &lines, rows: &none }, Edition { script: &en_script, lines: &lines, rows: &none });
        for l in ["btnwait %9", "if %9 == 1 goto *c_0", "*c_0", "goto *c_end", "*c_end"] {
            assert!(m.script.lines().any(|s| s == l), "{l} in the script");
        }
        assert_eq!((m.blocks, m.lines), (1, 1), "only the plate is per language");
        let blank = |s: String| s.lines().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n") + "\n";
        assert_eq!(blank(fill(&m.script, &m.tables.0)), ja_script);
        assert_eq!(blank(fill(&m.script, &m.tables.1)), en_script);
    }

    #[test]
    fn a_large_block_keeps_its_labels_too() {
        // past the DP's limit: anchored on the lines each side has once
        let side = |lang: &str| -> Vec<(String, bool)> { (0..3000).flat_map(|i| [(format!("*l{i}"), true), (format!("lsp 1,\"{lang}\",0,0"), true), ("print 1".into(), true)]).collect() };
        let (a, b) = (side("ja"), side("en"));
        let common = common_lines(&a, &b);
        assert_eq!(common.len(), 6000, "the labels and the prints");
        assert!(common.iter().all(|&(i, j)| i == j && a[i].0 != "lsp"));
    }

    #[test]
    fn a_hold_in_one_edition_changes_no_english_shown() {
        // ma01_022vs:2546: the Japanese two pages with a hold between them
        // (another place's line), the English one page; a picture changed
        // (place 3) before the last voice. The table rows may go to
        // per-language lines: what plays in English is the same.
        use conv_core::ir::SourceLoc;
        let at = |i| Some(SourceLoc { file: "a.nss".into(), index: i });
        let mut ja = Strings::tables(false);
        ja.at = "a.nss:2".into();
        let [a, b, c, d] = ["　一\\", "　二\\", "　三\\", "「四」\\"].map(|t| ja.lines(TEXT, t));
        let mut en = Strings::tables(false);
        en.at = "a.nss:2".into();
        let [e1, e4] = ["`One two three\\", "`Four\\"].map(|t| en.lines(TEXT, t));
        let en_script = format!("dwave 0,\"v1\"\n{e1}\nlsp 5,\"f.png\",0,0\ndwave 0,\"v4\"\n{e4}\n");
        let en_lines = vec![(1, at(2)), (3, at(3)), (4, at(2)), (6, None)];
        let shown = |hold: bool| {
            let w = if hold { "wait 100\n" } else { "" };
            let script = format!("dwave 0,\"v1\"\n{a}\n{b}\n{w}dwave 0,\"v3\"\n{c}\nlsp 5,\"f.png\",0,0\ndwave 0,\"v4\"\n{d}\n");
            let lines = if hold { vec![(1, at(2)), (4, at(9)), (5, at(2)), (7, at(3)), (8, at(2)), (10, None)] } else { vec![(1, at(2)), (6, at(3)), (7, at(2)), (9, None)] };
            let m = merge(Edition { script: &script, lines: &lines, rows: &ja.rows }, Edition { script: &en_script, lines: &en_lines, rows: &en.rows });
            fill(&m.script, &m.tables.1).lines().filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
        };
        assert_eq!(shown(true), shown(false));
        assert_eq!(shown(true), fill(&en_script, &table(&en.rows)).trim_end());
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
