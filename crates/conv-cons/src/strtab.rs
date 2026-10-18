//! Text tables (docs/string-tables.md): a built 0.txt split into the
//! script, whose text lines are `^ID` and whose choice options are `"^ID"`,
//! and one table per language (`text.<code>.txt`: `ID<TAB>line`), which
//! CONscripter fills in as it reads the script and swaps on `language`.
//!
//! A line's place is its scene (the last label starting with the scene
//! prefix) and, with the build's source map (`--source-map`), the source
//! instruction it came from; its ID is a hash of that place and how many
//! lines of the same kind came before it there. Without a map the place is
//! the scene and the line's own text. Either way a rebuild keeps the ID
//! unless the line moves to another source instruction (or, without a
//! map, its text changes).
//!
//! With the same game built in English and its map, the English table
//! takes, for each place, the English build's lines of that place: the
//! k-th Japanese line the k-th English one (the last Japanese line of the
//! place takes the English left over, Japanese lines past the English ones
//! are empty). The text window's lines (`setwindow`, `tateyoko`, the click
//! icon's place) go in the tables the same way, as they differ by
//! language. A place the English build lacks keeps its Japanese.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

/// A split script: the script with IDs, then each language's table.
pub struct Split {
    pub script: String,
    pub tables: Vec<(String, String)>,
    pub stats: Stats,
}

#[derive(Default, Debug)]
pub struct Stats {
    pub text_lines: usize,
    pub choices: usize,
    pub window_lines: usize,
    pub scenes: usize,
    /// Japanese text lines whose place the English build lacks (they stay Japanese in text.en.txt)
    pub english_missing: usize,
}

fn is_text(line: &str) -> bool {
    match line.trim_start_matches([' ', '\t']).chars().next() {
        None => false,
        Some(c) => !(c.is_ascii_alphanumeric() || "*~;$%{}[]_:".contains(c)),
    }
}

/// Lines laid out for one language: the text window and its click icon.
fn is_window(line: &str) -> bool {
    let l = line.trim_start();
    l.starts_with("setwindow") || l.starts_with("tateyoko") || l.starts_with("mov %conv_wait_icon_x")
}

fn is_choice(line: &str) -> bool {
    let l = line.trim_start();
    l.starts_with("select ") || l.starts_with("csel ")
}

/// The option texts of a select line: (start, end) of each, inside its
/// quotes; an option is a string followed by `,*label`.
fn options(line: &str) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut quote = None;
    for (i, c) in line.char_indices() {
        if c != '"' {
            continue;
        }
        match quote.take() {
            None => quote = Some(i + 1),
            Some(s) => {
                let rest = line[i + 1..].trim_start();
                if rest.strip_prefix(',').is_some_and(|r| r.trim_start().starts_with('*')) {
                    out.push((s, i));
                }
            }
        }
    }
    out
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

/// The source of each 0.txt line (1-based) from a `--source-map` file:
/// `<line>\t<script>:<instruction>` runs, `-` for none.
fn source_of(map: &str, lines: usize) -> Vec<Option<String>> {
    let mut out = vec![None; lines + 2];
    let mut runs: Vec<(usize, Option<String>)> = map
        .lines()
        .filter_map(|l| {
            let (n, s) = l.split_once('\t')?;
            Some((n.parse().ok()?, (s != "-").then(|| s.to_string())))
        })
        .collect();
    runs.sort_by_key(|r| r.0);
    for (i, (from, src)) in runs.iter().enumerate() {
        let to = runs.get(i + 1).map_or(lines + 1, |r| r.0).min(lines + 1);
        for slot in out.iter_mut().take(to).skip(*from) {
            *slot = src.clone();
        }
    }
    out
}

/// Each line's kind, scene, place (source or, without a map, the text) and text.
fn places<'a>(script: &'a str, map: Option<&str>, prefix: &str) -> Vec<(char, String, String, &'a str)> {
    let lines: Vec<&str> = script.lines().collect();
    let src = map.map(|m| source_of(m, lines.len()));
    let mut scene = String::new();
    let mut out = vec![];
    for (i, line) in lines.iter().enumerate() {
        if let Some(l) = line.strip_prefix('*').filter(|l| l.starts_with(prefix)) {
            scene = l.to_string();
        }
        let kind = if is_text(line) {
            't'
        } else if is_choice(line) {
            'c'
        } else if is_window(line) {
            'w'
        } else {
            ' '
        };
        let at = match &src {
            Some(s) => s[i + 1].clone().unwrap_or_default(),
            None => line.to_string(),
        };
        out.push((kind, scene.clone(), at, *line));
    }
    out
}

/// The English of a place's n Japanese lines from its English lines.
fn fit(ja: usize, en: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = en.iter().take(ja).map(|s| s.to_string()).collect();
    if en.len() > ja && ja > 0 {
        let rest: String = en[ja..].concat();
        out[ja - 1].push_str(&rest);
    }
    out.resize(ja, String::new());
    out
}

type Key<'a> = (char, &'a str, &'a str);

/// Splits `ja`; `en` is the English build with its map (lined up with the
/// Japanese by `ja_map`, without which the English table keeps the Japanese).
pub fn split(ja: &str, ja_map: Option<&str>, en: Option<(&str, &str)>, scene_prefix: &str) -> Split {
    let jp = places(ja, ja_map, scene_prefix);
    let mapped = ja_map.is_some();
    let ep = en.filter(|_| mapped).map(|(e, m)| places(e, Some(m), scene_prefix));
    let mut english: HashMap<Key, Vec<&str>> = HashMap::new();
    for (kind, scene, at, line) in ep.iter().flatten() {
        if *kind != ' ' && !at.is_empty() {
            english.entry((*kind, scene, at)).or_default().push(line);
        }
    }
    let mut count: HashMap<Key, usize> = HashMap::new();
    for (kind, scene, at, _) in &jp {
        *count.entry((*kind, scene, at)).or_default() += 1;
    }

    let mut stats = Stats::default();
    let mut out = String::with_capacity(ja.len());
    let mut tables: BTreeMap<&str, String> = BTreeMap::new();
    let mut seen: HashMap<Key, usize> = HashMap::new();
    let mut ids: HashMap<u64, String> = HashMap::new();
    let mut fitted: HashMap<Key, Vec<String>> = HashMap::new();
    let mut last_scene = "";
    for (kind, scene, at, line) in &jp {
        if scene != last_scene {
            last_scene = scene;
            stats.scenes += 1;
        }
        let key = (*kind, scene.as_str(), at.as_str());
        let en_lines = english.get(&key).filter(|_| !at.is_empty());
        if *kind == ' ' || (*kind == 'w' && en_lines.is_none()) {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let k = {
            let n = seen.entry(key).or_insert(0);
            *n += 1;
            *n - 1
        };
        let mut id_of = |part: &str| -> String {
            let mut salt = String::new();
            loop {
                let h = fnv64(&[scene, &kind.to_string(), at, &k.to_string(), part, &salt]) & 0xffff_ffff_ffff;
                let full = format!("{scene}\u{1}{kind}\u{1}{at}\u{1}{k}\u{1}{part}\u{1}{salt}");
                match ids.get(&h) {
                    Some(other) if *other != full => salt.push('+'), // a 48-bit clash: take the next
                    _ => {
                        ids.insert(h, full);
                        return format!("{h:012x}");
                    }
                }
            }
        };
        if *kind == 'c' {
            let opts = options(line);
            let en_opts: Option<Vec<String>> = en_lines.and_then(|e| e.get(k)).map(|l| options(l).iter().map(|&(a, b)| l[a..b].to_string()).collect());
            let mut new = String::new();
            let mut pos = 0;
            for (o, &(a, b)) in opts.iter().enumerate() {
                let text = &line[a..b];
                let id = id_of(&o.to_string());
                new.push_str(&line[pos..a]);
                write!(new, "^{id}").unwrap();
                pos = b;
                writeln!(tables.entry("ja").or_default(), "{id}\t{text}").unwrap();
                if en.is_some() {
                    let e = en_opts.as_ref().filter(|e| e.len() == opts.len()).map_or(text, |e| e[o].as_str());
                    writeln!(tables.entry("en").or_default(), "{id}\t{e}").unwrap();
                }
                stats.choices += 1;
            }
            new.push_str(&line[pos..]);
            out.push_str(&new);
            out.push('\n');
            continue;
        }
        let id = id_of(if mapped { "" } else { line });
        writeln!(out, "^{id}").unwrap();
        writeln!(tables.entry("ja").or_default(), "{id}\t{line}").unwrap();
        if en.is_some() {
            let e = match en_lines {
                Some(e) => fitted.entry(key).or_insert_with(|| fit(count[&key], e))[k].clone(),
                None => {
                    stats.english_missing += 1;
                    line.to_string()
                }
            };
            writeln!(tables.entry("en").or_default(), "{id}\t{e}").unwrap();
        }
        match kind {
            't' => stats.text_lines += 1,
            _ => stats.window_lines += 1,
        }
    }
    Split { script: out, tables: tables.into_iter().map(|(k, v)| (k.to_string(), v)).collect(), stats }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_text_and_choices_and_fits_english() {
        let ja = "*define\nlanguage \"ja\"\n*f_a\nsetwindow3 1\n　一行目\n二行目\\\nselect \"はい\",*x,\"いいえ\",*y\n*x\n　一行目\n";
        let ja_map = "1\t-\n4\ta.nss:1\n5\ta.nss:2\n7\ta.nss:3\n8\t-\n9\ta.nss:4\n";
        let en = "*define\nlanguage \"en\"\n*f_a\nsetwindow3 2\n`One line.\\\nselect \"Yes\",*x,\"No\",*y\n*x\n`First.\n";
        let en_map = "1\t-\n4\ta.nss:1\n5\ta.nss:2\n6\ta.nss:3\n7\t-\n8\ta.nss:4\n";
        let s = split(ja, Some(ja_map), Some((en, en_map)), "f_");
        let lines: Vec<&str> = s.script.lines().collect();
        assert!(lines[3].starts_with('^') && lines[4].starts_with('^') && lines[5].starts_with('^'));
        assert!(lines[6].starts_with("select \"^") && lines[6].contains("\",*x,\"^"));
        assert_eq!(s.tables[0].0, "en");
        let (en_t, ja_t) = (&s.tables[0].1, &s.tables[1].1);
        let id = &lines[4][1..];
        assert!(ja_t.contains(&format!("{id}\t　一行目\n")));
        assert!(en_t.contains(&format!("{id}\t`One line.\\\n")));
        assert!(en_t.contains(&format!("{}\t\n", &lines[5][1..])), "the place's second line is empty in English");
        assert!(en_t.contains("\tsetwindow3 2\n") && en_t.contains("\tYes\n") && en_t.contains("\tNo\n"));
        assert!(en_t.contains(&format!("{}\t`First.\n", &lines[8][1..])));
        assert_ne!(&lines[8][1..], id);
        assert_eq!((s.stats.text_lines, s.stats.english_missing), (3, 0));
        // without English: no window lines in the table, the script keeps them;
        // without a map the IDs come from the text
        let j = split(ja, None, None, "f_");
        assert!(j.script.contains("setwindow3 1"));
        assert_eq!(j.tables.len(), 1);
        assert_ne!(j.script.lines().nth(4), j.script.lines().nth(8), "the same text twice: two IDs");
    }
}
