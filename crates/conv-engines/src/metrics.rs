//! How much of a game converts: each source command call (a built-in, a
//! task, an opcode) is either fully converted or has a gap, when the
//! converter drops it, approximates it or leaves an argument it was given
//! unread (any note made while the call is open), or is left out on
//! purpose (code the game can't reach, such as a debug menu: counted as
//! done in the completion figure, apart from the gaps). Every engine
//! reports its calls here; the dry run writes them as `metrics.json`, and
//! `tests/snapshot.sh --update` keeps them in `tests/snapshots/`.

use std::collections::BTreeMap;

/// The calls of one kind (a command's name).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Kind {
    pub calls: usize,
    pub gaps: usize,
    /// Calls left out on purpose ([`Metrics::left_out`]), not gaps.
    pub left_out: usize,
    /// Each reason with the number of calls it was noted on.
    pub reasons: BTreeMap<String, usize>,
}

#[derive(Debug, Default, Clone)]
pub struct Metrics {
    pub kinds: BTreeMap<String, Kind>,
    /// Notes made outside any call (a statement's, a text tag's), with
    /// counts: not in the totals.
    pub outside: BTreeMap<String, usize>,
    /// Calls being converted, innermost last: kind, reasons noted so far.
    open: Vec<(String, Vec<String>)>,
}

/// How many reasons and outside notes the JSON keeps per list.
const TOP: usize = 5;

impl Metrics {
    /// A call starts: notes until its [`end`](Self::end) are its gaps.
    pub fn begin(&mut self, kind: &str) {
        self.open.push((kind.to_string(), Vec::new()));
    }

    /// The innermost call ends.
    pub fn end(&mut self) {
        if let Some((kind, reasons)) = self.pop() {
            self.call(&kind, reasons);
        }
    }

    /// The innermost call ends but isn't counted yet: its kind and reasons,
    /// for an engine that notes more about it later, then calls [`call`](Self::call).
    pub fn pop(&mut self) -> Option<(String, Vec<String>)> {
        self.open.pop()
    }

    /// A note, on the innermost open call (or outside any).
    pub fn note(&mut self, reason: &str) {
        match self.open.last_mut() {
            Some((_, reasons)) => reasons.push(reason.to_string()),
            None => *self.outside.entry(reason.to_string()).or_default() += 1,
        }
    }

    /// One whole call with the reasons noted on it (none: fully converted).
    pub fn call(&mut self, kind: &str, mut reasons: Vec<String>) {
        let k = self.kinds.entry(kind.to_string()).or_default();
        k.calls += 1;
        reasons.sort();
        reasons.dedup();
        k.gaps += usize::from(!reasons.is_empty());
        for r in reasons {
            *k.reasons.entry(r).or_default() += 1;
        }
    }

    /// One whole call left out on purpose (unreachable, such as a debug
    /// menu), with why: done for the completion figure, not a gap.
    pub fn left_out(&mut self, kind: &str, why: &str) {
        let k = self.kinds.entry(kind.to_string()).or_default();
        k.calls += 1;
        k.left_out += 1;
        *k.reasons.entry(why.to_string()).or_default() += 1;
    }

    /// Adds `other`'s counts.
    pub fn merge(&mut self, other: Metrics) {
        for (name, o) in other.kinds {
            let k = self.kinds.entry(name).or_default();
            k.calls += o.calls;
            k.gaps += o.gaps;
            k.left_out += o.left_out;
            for (r, n) in o.reasons {
                *k.reasons.entry(r).or_default() += n;
            }
        }
        for (r, n) in other.outside {
            *self.outside.entry(r).or_default() += n;
        }
    }

    /// Every kind's reasons together, with the calls each was noted on.
    pub fn reasons(&self) -> BTreeMap<String, usize> {
        let mut all = BTreeMap::new();
        for (r, n) in self.kinds.values().flat_map(|k| &k.reasons) {
            *all.entry(r.clone()).or_default() += n;
        }
        all
    }

    /// (calls, gaps) over every kind.
    pub fn totals(&self) -> (usize, usize) {
        self.kinds.values().fold((0, 0), |(c, g), k| (c + k.calls, g + k.gaps))
    }

    /// Calls left out on purpose, over every kind.
    pub fn left_out_total(&self) -> usize {
        self.kinds.values().map(|k| k.left_out).sum()
    }

    /// The JSON the dry run writes: `commit` and `commit_date` each on a
    /// line of their own (`tests/snapshot.sh` compares without them), the
    /// kinds by gaps, each with its [`TOP`] reasons.
    pub fn json(&self, game: &str, engine: &str, commit: &str, commit_date: &str) -> String {
        let (calls, gaps) = self.totals();
        let left_out = self.left_out_total();
        // "left_out" only when there are some, so other games' files stay as they were.
        let lo = |n: usize| if n > 0 { format!(", \"left_out\": {n}") } else { String::new() };
        let mut s = format!(
            "{{\n\"game\": {},\n\"engine\": {},\n\"commit\": {},\n\"commit_date\": {},\n\"totals\": {{\"calls\": {calls}, \"full\": {}, \"gaps\": {gaps}{}}},\n\"kinds\": {{",
            q(game),
            q(engine),
            q(commit),
            q(commit_date),
            calls - gaps - left_out,
            lo(left_out)
        );
        let mut kinds: Vec<_> = self.kinds.iter().collect();
        kinds.sort_by(|a, b| b.1.gaps.cmp(&a.1.gaps).then(b.1.calls.cmp(&a.1.calls)).then(a.0.cmp(b.0)));
        for (i, (name, k)) in kinds.iter().enumerate() {
            s += &format!(
                "{}\n{}: {{\"calls\": {}, \"gaps\": {}{}, \"reasons\": {}}}",
                if i > 0 { "," } else { "" },
                q(name),
                k.calls,
                k.gaps,
                lo(k.left_out),
                top(&k.reasons)
            );
        }
        s += &format!("\n}},\n\"outside_calls\": {}\n}}\n", top(&self.outside));
        s
    }
}

/// The `TOP` entries with the highest counts, as `[[reason, n], …]`, and
/// `["…", n]` for the rest's sum.
fn top(counts: &BTreeMap<String, usize>) -> String {
    let mut v: Vec<_> = counts.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let mut items: Vec<String> = v.iter().take(TOP).map(|(r, n)| format!("[{}, {n}]", q(r))).collect();
    let rest: usize = v.iter().skip(TOP).map(|(_, n)| **n).sum();
    if rest > 0 {
        items.push(format!("[\"{} more\", {rest}]", v.len() - TOP));
    }
    format!("[{}]", items.join(", "))
}

/// `s` as a JSON string.
fn q(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out += "\\\"",
            '\\' => out += "\\\\",
            c if (c as u32) < 0x20 => out += &format!("\\u{:04x}", c as u32),
            c => out.push(c),
        }
    }
    out + "\""
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn counts_calls_not_notes() {
        let mut m = Metrics::default();
        m.begin("Move");
        m.note("Move argument 4 ignored");
        m.note("Move argument 5 ignored");
        m.note("Move argument 4 ignored");
        m.begin("Fade"); // nested: its notes are its own
        m.end();
        m.end();
        m.begin("Move");
        m.end();
        m.note("text tag <x>");
        assert_eq!(m.totals(), (3, 1));
        assert_eq!(m.kinds["Move"].gaps, 1);
        assert_eq!(m.kinds["Move"].reasons["Move argument 4 ignored"], 1);
        assert_eq!(m.kinds["Fade"].gaps, 0);
        assert_eq!(m.outside["text tag <x>"], 1);
        let json = m.json("g", "e", "abc", "2026-10-16");
        assert!(json.contains("\"totals\": {\"calls\": 3, \"full\": 2, \"gaps\": 1}"), "{json}");
        assert!(json.contains("\n\"commit\": \"abc\",\n"));
    }

    #[test]
    fn left_out_calls_are_neither_full_nor_gaps() {
        let mut m = Metrics::default();
        m.call("opcode 0x30", vec![]);
        m.left_out("opcode 0x30", "the debug menu");
        assert_eq!((m.totals(), m.left_out_total()), ((2, 0), 1));
        let json = m.json("g", "e", "c", "d");
        assert!(json.contains("\"totals\": {\"calls\": 2, \"full\": 1, \"gaps\": 0, \"left_out\": 1}"), "{json}");
        assert!(json.contains("\"calls\": 2, \"gaps\": 0, \"left_out\": 1, \"reasons\""), "{json}");
    }
}
