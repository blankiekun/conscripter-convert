//! Checks on a finished `0.txt` for mistakes that stop the engine or lose
//! content without an error message: jumps to labels that don't exist,
//! effect numbers past 255, half-width characters in vertical text (the
//! engine exits), and files the script names that aren't there.

use std::collections::{BTreeMap, HashSet};

/// Examples kept per kind of problem.
const EXAMPLES: usize = 5;

/// Problems found, by kind: how many, and the first few (line: detail).
#[derive(Debug, Default)]
pub struct Report {
    pub problems: BTreeMap<&'static str, (usize, Vec<String>)>,
}

impl Report {
    fn add(&mut self, kind: &'static str, line: usize, detail: impl Into<String>) {
        let e = self.problems.entry(kind).or_default();
        e.0 += 1;
        if e.1.len() < EXAMPLES {
            e.1.push(format!("line {}: {}", line + 1, detail.into()));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.problems.is_empty()
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (kind, (n, examples)) in &self.problems {
            writeln!(f, "{n:>8}  {kind}")?;
            for e in examples {
                writeln!(f, "          {e}")?;
            }
        }
        Ok(())
    }
}

/// File extensions the engine loads by name.
const EXTENSIONS: [&str; 10] = ["png", "jpg", "jpeg", "bmp", "ogg", "wav", "mp3", "mid", "mpg", "ttf"];

/// Lints `script` (decoded `0.txt`). `has_file` says whether a file name
/// as the script writes it (`cg\bg\a.png`) is in an archive or loose.
pub fn lint(script: &str, has_file: &dyn Fn(&str) -> bool) -> Report {
    let mut report = Report::default();
    let lines: Vec<&str> = script.lines().collect();
    let labels: HashSet<String> =
        lines.iter().filter_map(|l| l.strip_prefix('*')).map(|l| l.trim_end().to_ascii_lowercase()).collect();
    let mut vertical = false;
    let mut checked_files = HashSet::new();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('*') || line.starts_with(';') || line.trim().is_empty() {
            continue;
        }
        if is_text(line) {
            if vertical && let Some(c) = half_width_in_text(line) {
                report.add("half-width character in vertical text (the engine exits)", i, format!("{c:?} in {}", short(line)));
            }
            if line.contains('?') && !line.starts_with('`') { // a backquote line has no variables
                report.add("'?' in text (read as an array: the engine exits)", i, short(line));
            }
            continue;
        }
        let (code, strings) = split_strings(line);
        for cmd in code.split(':') {
            let cmd = cmd.trim();
            if cmd.starts_with("tateyoko") {
                vertical = cmd.trim_start_matches("tateyoko").trim() == "1";
            }
            if let Some(n) = cmd.strip_prefix("effect ").and_then(|r| r.split(',').next()).and_then(|n| n.trim().parse::<u32>().ok())
                && n > 255
            {
                report.add("effect number past 255", i, cmd);
            }
            if cmd.contains(")%") {
                report.add("'%' after ')' (a variable, not modulo: that is 'mod'; the rest shows as text)", i, cmd);
            }
        }
        for label in label_refs(&code) {
            if !labels.contains(&label.to_ascii_lowercase()) {
                report.add("jump to a label that doesn't exist", i, format!("*{label}"));
            }
        }
        for s in strings {
            let Some(name) = file_name(s) else { continue };
            if checked_files.insert(name.to_ascii_lowercase()) && !has_file(name) {
                report.add("file not in the archives or the game directory", i, name);
            }
        }
    }
    report
}

/// A text line: NScripter reads a line that starts with a non-ASCII
/// character (or a backquote) as text.
fn is_text(line: &str) -> bool {
    line.starts_with(|c: char| !c.is_ascii() || c == '`')
}

/// The first half-width character in a text line, past the tags the
/// backend writes (`#rrggbb`, `!zNNN`, `!i1`, `!c`, `!sN`, `!wN`, `!dN`, ruby `(…/…)`)
/// and the page and click waits.
fn half_width_in_text(line: &str) -> Option<char> {
    let mut chars = line.trim_end_matches(['\\', '@']).chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '#' => {
                chars.by_ref().take(6).for_each(drop);
            }
            '!' if chars.peek().is_some_and(|c| matches!(c, 's' | 'w' | 'd' | 'z' | 'i' | 'c')) => {
                chars.next();
                while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                    chars.next();
                }
            }
            '(' | '/' | ')' | '@' | '\\' | ' ' => {}
            c if c.is_ascii() || ('\u{FF61}'..='\u{FF9F}').contains(&c) => return Some(c),
            _ => {}
        }
    }
    None
}

/// The line without its `"…"` strings, and the strings.
fn split_strings(line: &str) -> (String, Vec<&str>) {
    let mut code = String::new();
    let mut strings = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        code.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('"').unwrap_or(after.len());
        strings.push(&after[..end]);
        rest = after.get(end + 1..).unwrap_or("");
    }
    code.push_str(rest);
    // A comment ends the line.
    if let Some(c) = code.find(';') {
        code.truncate(c);
    }
    (code, strings)
}

/// `*name` references in code.
fn label_refs(code: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for (i, _) in code.match_indices('*') {
        let rest = &code[i + 1..];
        if !rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            continue;
        }
        // `%a*b` multiplies: only a `*` after a separator starts a label.
        if !code[..i].ends_with([' ', ',', '\t']) {
            continue;
        }
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
        out.push(&rest[..end]);
    }
    out
}

/// A file name in a string argument (`":a;cg\bg\a.png"` → `cg\bg\a.png`).
fn file_name(s: &str) -> Option<&str> {
    let name = if s.starts_with(':') { &s[s.find(';')? + 1..] } else { s };
    let (stem, ext) = name.rsplit_once('.')?;
    // A bare extension is added to a name built at run time.
    (!stem.is_empty() && EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) && !name.contains(['$', '%'])).then_some(name)
}

fn short(line: &str) -> String {
    line.chars().take(30).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_each_kind_of_problem() {
        let script = "\
*start
effect 300,10,100
goto *missing
select \"a\",*start,\"b\",*gone
mov %1,%2*%3
mov %1,(%1+1)%3:goto *start
lsp 1,\":a;cg\\a.png\",0,0
lsp 2,\":c;cg\\b.jpg\",0,0
add $1,\".bmp\"
tateyoko 1:setwindow 1,1,1,1,1,1,1,0,1,0,1,#000000
　縦の!?テキスト\\
　#ff0000赤い!s20字(る/び)@
tateyoko 0:setwindow 1,1,1,1,1,1,1,0,1,0,1,#000000
　横のOKテキスト\\
　横の?\\
";
        let r = lint(script, &|n| n == "cg\\a.png");
        let kinds: Vec<_> = r.problems.iter().map(|(k, (n, _))| (*k, *n)).collect();
        assert_eq!(
            kinds,
            [
                ("'%' after ')' (a variable, not modulo: that is 'mod'; the rest shows as text)", 1),
                ("'?' in text (read as an array: the engine exits)", 2),
                ("effect number past 255", 1),
                ("file not in the archives or the game directory", 1),
                ("half-width character in vertical text (the engine exits)", 1),
                ("jump to a label that doesn't exist", 2),
            ],
            "{r}"
        );
    }
}
