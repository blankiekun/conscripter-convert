//! Checks on a finished `0.txt` for mistakes that stop the engine or lose
//! content without an error message: jumps to labels that don't exist,
//! effect numbers past 255, half-width characters in vertical text (the
//! engine exits), files the script names that aren't there, labels
//! defined twice (the engine keeps the first), strings left open at
//! the end of a line (the next lines show as text) and sprites changed
//! while their `spmotion` still runs ([`Motions`]).

use std::collections::{BTreeMap, HashMap, HashSet};

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
    let mut defined: HashMap<String, usize> = HashMap::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(label) = line.strip_prefix('*') else { continue };
        let label = label.trim_end().to_ascii_lowercase();
        if let Some(first) = defined.get(&label) {
            report.add("label defined twice (the engine keeps the first)", i, format!("*{label}, first at line {}", first + 1));
        } else {
            defined.insert(label, i);
        }
    }
    // a jump to `name` reaches a language's `name@en` (CONscripter's label variants)
    let labels: HashSet<String> = defined.into_keys().flat_map(|l| [l.split('@').next().unwrap_or_default().to_string(), l]).collect();
    let mut motions = Motions::default();
    let mut vertical = false;
    let mut checked_files = HashSet::new();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(';') || line.trim().is_empty() {
            continue;
        }
        if line.starts_with('*') || is_text(line) {
            motions.reset();
        }
        if line.starts_with('*') {
            continue;
        }
        if is_text(line) {
            text_resumes(&mut report, i, line);
            if vertical && let Some(c) = half_width_in_text(line) {
                report.add("half-width character in vertical text (the engine exits)", i, format!("{c:?} in {}", short(line)));
            }
            if line.contains('?') && !line.starts_with('`') { // a backquote line has no variables
                report.add("'?' in text (read as an array: the engine exits)", i, short(line));
            }
            continue;
        }
        let (code, strings, open) = split_strings(line);
        if open {
            report.add("string runs past the end of its line (the engine ends it there; the next lines show as text)", i, short(line));
        }
        if !label_refs(&code).is_empty() {
            motions.reset();
        }
        for cmd in code.split(':') {
            let cmd = cmd.trim();
            if let Some((slot, left)) = motions.step(cmd) {
                report.add("sprite changed while its spmotion runs (no wait for it between)", i, format!("sprite {slot}, {left} ms left: {}", short(cmd)));
            }
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

/// Each sprite's `spmotion` end on a clock that only timed commands move
/// (`wait`, `delay`, a `print` with a timed effect, `spmotionwait`); a
/// click wait, a label or a jump makes it unknown, and nothing is judged
/// across it. `lsp`, `csp`, `msp`, `amsp` or another `spmotion` on a sprite
/// before its end cuts the motion short (COMMANDS.md: they finish it at its
/// last key), unless it loops; `spmotion n[,2]` stops it on purpose.
#[derive(Default)]
struct Motions {
    clock: u64,
    ends: HashMap<u32, u64>,
    effects: HashMap<u32, u64>,
}

impl Motions {
    fn reset(&mut self) {
        self.ends.clear();
    }

    /// One command: the sprite cut short and the ms its motion had left.
    fn step(&mut self, cmd: &str) -> Option<(u32, u64)> {
        let (name, rest) = cmd.split_once(' ').unwrap_or((cmd, ""));
        let args: Vec<&str> = rest.split(',').map(str::trim).collect();
        let num = |i: usize| args.get(i).and_then(|a| a.parse::<u64>().ok());
        let slot = num(0).map(|n| n as u32);
        match name {
            "effect" => {
                if let (Some(n), Some(ms)) = (slot, num(2)) {
                    self.effects.insert(n, ms);
                }
                None
            }
            "wait" | "delay" => {
                self.clock += num(0).unwrap_or(0);
                None
            }
            "print" => {
                self.clock += num(1).or_else(|| slot.and_then(|n| self.effects.get(&n).copied())).unwrap_or(0);
                None
            }
            "click" | "lrclick" | "btnwait" | "btnwait2" | "textbtnwait" | "select" | "csel" | "mpegplay" => {
                self.reset();
                None
            }
            "spmotionwait" => {
                if let Some(end) = slot.and_then(|n| self.ends.remove(&n)) {
                    self.clock = self.clock.max(end);
                }
                None
            }
            "spmotion" if args.len() <= 2 => {
                slot.map(|n| self.ends.remove(&n));
                None
            }
            "lsp" | "lsp2" | "csp" | "msp" | "amsp" | "spmotion" => {
                let n = slot?;
                let cut = self.ends.remove(&n).filter(|&e| e > self.clock).map(|e| (n, e - self.clock));
                // (Once, kept or removed at its end: loop 0, 2, 3 or -n; 1 and 4 loop.)
                if name == "spmotion" && !matches!(args.get(1), Some(&"1" | &"4")) {
                    // Keys of 8 after the slot and loop: time, x, y, zoom x, y, angle, alpha, curve.
                    if let Some(t) = args.len().checked_sub(8).and_then(|i| args.get(i)).and_then(|a| a.parse::<u64>().ok()) {
                        self.ends.insert(n, self.clock + t);
                    }
                }
                cut
            }
            _ => None,
        }
    }
}

/// A text line: NScripter reads a line that starts with a non-ASCII
/// character (or a backquote) as text.
fn is_text(line: &str) -> bool {
    line.starts_with(|c: char| !c.is_ascii() || c == '`')
}

/// The kind [`text_resumes`] reports.
pub const TEXT_AS_COMMAND: &str = "text read as a command (after a closing backquote, or a wait and a space)";

/// Text lines of `script` whose text goes on where the engine reads a
/// command (engine/src/cons/ScriptHandler.cpp `readToken`): after a
/// closing backquote, a letter starts a command (`` `a`b ``); after a click
/// or page wait and a space, a textgosub's return skips the space and
/// reads on from a letter (`` `a@ b ``, `あ@ b`). `` @` ``, `` \` ``, a
/// `` `/ `` at the end, `:` and `:br:br:` between strings and the text tags are the forms meant.
pub fn text_resumes(report: &mut Report, i: usize, line: &str) {
    let b = line.as_bytes();
    let command = |at: usize| b.get(at).is_some_and(|&c| c.is_ascii_alphabetic() || c == b'_');
    let after_wait_space = |mut at: usize| {
        let start = at;
        while matches!(b.get(at), Some(b' ' | b'\t')) {
            at += 1;
        }
        at > start && command(at)
    };
    let mut at = 0;
    loop {
        while matches!(b.get(at), Some(b' ' | b'\t')) {
            at += 1;
        }
        match b.get(at) {
            None => return,
            Some(b'`') => {
                at += 1;
                while let Some(&c) = b.get(at).filter(|&&c| c != b'`') {
                    at += 1;
                    if matches!(c, b'@' | b'\\') && after_wait_space(at) {
                        return report.add(TEXT_AS_COMMAND, i, around(line, at));
                    }
                }
                at += 1; // the closing backquote
                if b.get(at) == Some(&b'/') && at + 1 == b.len() {
                    return;
                }
            }
            Some(b':') => {
                // the joined lines' `:`, `:br:br:` (strtab::join_text_lines)
                at += 1;
                while line[at..].starts_with("br:") {
                    at += 3;
                }
                if b.get(at) != Some(&b'`') {
                    return;
                }
            }
            Some(_) if command(at) => return report.add(TEXT_AS_COMMAND, i, around(line, at)),
            Some(b';' | b'~' | b'*') => return,
            Some(_) => {
                // two-byte text runs to the line's end
                while let Some(&c) = b.get(at) {
                    at += 1;
                    if matches!(c, b'@' | b'\\') && after_wait_space(at) {
                        return report.add(TEXT_AS_COMMAND, i, around(line, at));
                    }
                }
                return;
            }
        }
    }
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

/// The line without its `"…"` strings or its comment, the strings, and
/// whether the last string is still open at the end of the line.
fn split_strings(line: &str) -> (String, Vec<&str>, bool) {
    let mut code = String::new();
    let mut strings = Vec::new();
    let mut rest = line;
    // A comment (`;` outside a string) ends the line.
    while let Some(start) = rest.find(['"', ';']).filter(|&s| rest.as_bytes()[s] == b'"') {
        code.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else {
            strings.push(after);
            return (code, strings, true);
        };
        strings.push(&after[..end]);
        rest = &after[end + 1..];
    }
    code.push_str(rest.split(';').next().unwrap_or(""));
    (code, strings, false)
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
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '@')).unwrap_or(rest.len());
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

/// Up to 30 characters of `line` from about 10 bytes before byte `at`.
fn around(line: &str, at: usize) -> String {
    let mut from = at.saturating_sub(10).min(line.len());
    while !line.is_char_boundary(from) {
        from -= 1;
    }
    short(&line[from..])
}

fn short(line: &str) -> String {
    line.chars().take(30).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_text_read_as_a_command() {
        let lines = ["`For example...@ what if?\\", "`For example...@`` what if?\\", "`a,`I slay them.", "`a,`:br:`b@`/", "`a`:br:br:`b`:`c", "`a`:`b`c", "`a@!i1b\\", "あ@ what", "`x`/"];
        let flagged: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(i, l)| {
                let mut r = Report::default();
                text_resumes(&mut r, *i, l);
                !r.is_empty()
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(flagged, [0, 2, 5, 7]);
    }

    #[test]
    fn finds_each_kind_of_problem() {
        let script = "\
*start
effect 300,10,100
goto *missing
goto *v:goto *v@en
*v@en
select \"a\",*start,\"b\",*gone
mov %1,%2*%3
mov %1,(%1+1)%3:goto *start
lsp 1,\":a;cg\\a.png\",0,0
spmotion 1,2,0,0,0,100,100,0,256,0,500,10,0,100,100,0,256,0
wait 200
csp 1
spmotion 1,2,0,0,0,100,100,0,256,0,500,10,0,100,100,0,256,0:spmotionwait 1
csp 1
lsp 2,\":c;cg\\b.jpg\",0,0
add $1,\".bmp\"
*start
logadd \"two
　lines\",1
mov %1,1 ; a \" in a comment
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
                ("label defined twice (the engine keeps the first)", 1),
                ("sprite changed while its spmotion runs (no wait for it between)", 1),
                ("string runs past the end of its line (the engine ends it there; the next lines show as text)", 1),
            ],
            "{r}"
        );
    }
}
