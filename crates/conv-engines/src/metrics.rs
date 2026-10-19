//! How much of a game converts: each source command call (a built-in, a
//! task, an opcode) is either fully converted or has a gap, when the
//! converter drops it, approximates it or leaves an argument it was given
//! unread (any note made while the call is open), or is left out on
//! purpose (code the game can't reach, such as a debug menu: counted as
//! done in the completion figure, apart from the gaps). Each call also
//! counts its weighted share ([`share`]): the arguments it was given that
//! the converter read. Every engine
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
    /// The calls' [`share`]s summed (left-out calls not in it).
    pub weighted: f64,
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
    /// Timing parity: per scene (a script file), the motion time in ms
    /// its source calls ask for (every timed move, fade, zoom, turn and
    /// shake: [`motion`](Self::motion)); every scene that made a call is
    /// here, with 0 when none was timed.
    pub source_ms: BTreeMap<String, u64>,
    /// Per scene, the time our `0.txt` animates there (the dry run fills
    /// it in from the emitted script and its source map).
    pub ours_ms: BTreeMap<String, u64>,
}

/// How many reasons and outside notes the JSON keeps per list.
const TOP: usize = 5;
/// How many of the scenes most short of motion time the JSON names.
const PARITY_WORST: usize = 10;

impl Metrics {
    /// A source call in `scene` that asks for `ms` of motion (0: none, but
    /// the scene is known: its own `0.txt` lines are told apart from those
    /// of the files it calls into).
    pub fn motion(&mut self, scene: &str, ms: u64) {
        match self.source_ms.get_mut(scene) {
            Some(t) => *t += ms,
            None => {
                self.source_ms.insert(scene.to_string(), ms);
            }
        }
    }

    /// Per scene: (scene, source ms, ours, short: what ours lacks), the
    /// scenes most short first.
    pub fn parity(&self) -> Vec<(&str, u64, u64, u64)> {
        let mut v: Vec<_> = self
            .source_ms
            .iter()
            .map(|(s, &src)| {
                let ours = self.ours_ms.get(s).copied().unwrap_or(0);
                (s.as_str(), src, ours, src.saturating_sub(ours))
            })
            .collect();
        v.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(b.0)));
        v
    }

    /// A call starts: notes until its [`end`](Self::end) are its gaps.
    pub fn begin(&mut self, kind: &str) {
        self.open.push((kind.to_string(), Vec::new()));
    }

    /// The innermost call ends; it was given `args` arguments.
    pub fn end(&mut self, args: usize) {
        if let Some((kind, reasons)) = self.pop() {
            self.call(&kind, args, reasons);
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

    /// One whole call given `args` arguments (options counted as
    /// arguments of their own), with the reasons noted on it (none: fully
    /// converted).
    pub fn call(&mut self, kind: &str, args: usize, mut reasons: Vec<String>) {
        let k = self.kinds.entry(kind.to_string()).or_default();
        k.calls += 1;
        reasons.sort();
        reasons.dedup();
        k.weighted += share(args, &reasons);
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
        for (s, ms) in other.source_ms {
            *self.source_ms.entry(s).or_default() += ms;
        }
        for (name, o) in other.kinds {
            let k = self.kinds.entry(name).or_default();
            k.calls += o.calls;
            k.gaps += o.gaps;
            k.left_out += o.left_out;
            k.weighted += o.weighted;
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

    /// The weighted shares over every kind.
    pub fn weighted_total(&self) -> f64 {
        self.kinds.values().map(|k| k.weighted).sum()
    }

    /// Calls left out on purpose, over every kind.
    pub fn left_out_total(&self) -> usize {
        self.kinds.values().map(|k| k.left_out).sum()
    }

    /// The JSON the dry run writes: `commit` and `commit_date` each on a
    /// line of their own (`tests/snapshot.sh` compares without them), the
    /// kinds by gaps, each with its [`TOP`] reasons.
    pub fn json(&self, game: &str, engine: &str, engine_doc: &str, commit: &str, commit_date: &str) -> String {
        let (calls, gaps) = self.totals();
        let left_out = self.left_out_total();
        // "left_out" only when there are some, so other games' files stay as they were.
        let lo = |n: usize| if n > 0 { format!(", \"left_out\": {n}") } else { String::new() };
        let mut s = format!(
            "{{\n\"game\": {},\n\"engine\": {},\n\"engine_doc\": {},\n\"commit\": {},\n\"commit_date\": {},\n\"totals\": {{\"calls\": {calls}, \"full\": {}, \"gaps\": {gaps}{}, \"weighted\": {:.1}}},\n\"kinds\": {{",
            q(game),
            q(engine),
            q(engine_doc),
            q(commit),
            q(commit_date),
            calls - gaps - left_out,
            lo(left_out),
            self.weighted_total()
        );
        let mut kinds: Vec<_> = self.kinds.iter().collect();
        kinds.sort_by(|a, b| b.1.gaps.cmp(&a.1.gaps).then(b.1.calls.cmp(&a.1.calls)).then(a.0.cmp(b.0)));
        for (i, (name, k)) in kinds.iter().enumerate() {
            s += &format!(
                "{}\n{}: {{\"calls\": {}, \"gaps\": {}{}, \"reasons\": {}, \"weighted\": {:.1}}}",
                if i > 0 { "," } else { "" },
                q(name),
                k.calls,
                k.gaps,
                lo(k.left_out),
                top(&k.reasons),
                k.weighted
            );
        }
        s += &format!("\n}},\n\"outside_calls\": {}", top(&self.outside));
        // Timing parity (tests/snapshot.sh ratchets short_ms), when the engine reports it.
        if !self.source_ms.is_empty() {
            let p = self.parity();
            let sum = |f: fn(&(&str, u64, u64, u64)) -> u64| p.iter().map(f).sum::<u64>();
            let worst: Vec<String> = p.iter().take(PARITY_WORST).filter(|r| r.3 > 0).map(|r| format!("[{}, {}, {}]", q(r.0), r.1, r.2)).collect();
            s += &format!(
                ",\n\"parity\": {{\"source_ms\": {}, \"ours_ms\": {}, \"short_ms\": {}, \"scenes_short\": {}}},\n\"parity_worst\": [{}]",
                sum(|r| r.1),
                sum(|r| r.2),
                sum(|r| r.3),
                p.iter().filter(|r| r.3 > 0).count(),
                worst.join(", ")
            );
        }
        s + "\n}\n"
    }
}

/// A call's weighted share, from 0 to 1: the part of its `args` arguments
/// read. Each "… ignored" reason (an argument, option, operand or key) is
/// one argument lost; "not converted" makes it 0. Other reasons (an
/// approximation) leave the share as is: the arguments were read.
pub fn share(args: usize, reasons: &[String]) -> f64 {
    if reasons.iter().any(|r| r.contains("not converted")) {
        return 0.0;
    }
    let lost = reasons.iter().filter(|r| r.ends_with(" ignored")).count();
    if args == 0 {
        return if lost > 0 { 0.0 } else { 1.0 };
    }
    args.saturating_sub(lost) as f64 / args as f64
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
        m.end(1);
        m.end(8);
        m.begin("Move");
        m.end(6);
        m.note("text tag <x>");
        assert_eq!(m.totals(), (3, 1));
        assert_eq!(m.kinds["Move"].gaps, 1);
        assert_eq!(m.kinds["Move"].reasons["Move argument 4 ignored"], 1);
        assert_eq!(m.kinds["Fade"].gaps, 0);
        assert_eq!(m.outside["text tag <x>"], 1);
        let json = m.json("g", "e", "d", "abc", "2026-10-16");
        assert!(json.contains("\"totals\": {\"calls\": 3, \"full\": 2, \"gaps\": 1, \"weighted\": 2.8}"), "{json}");
        assert!(json.contains("\n\"commit\": \"abc\",\n"));
        assert!(!json.contains("parity"), "{json}");
    }

    #[test]
    fn parity_totals_and_worst_scenes() {
        let mut m = Metrics::default();
        m.motion("a.nss", 16000);
        m.motion("a.nss", 1000);
        m.motion("b.nss", 0);
        m.motion("c.nss", 500);
        m.ours_ms = [("a.nss".to_string(), 1000), ("b.nss".to_string(), 300), ("c.nss".to_string(), 500)].into();
        assert_eq!(m.parity()[0], ("a.nss", 17000, 1000, 16000));
        let json = m.json("g", "e", "c", "d");
        assert!(json.ends_with("\"parity\": {\"source_ms\": 17500, \"ours_ms\": 1800, \"short_ms\": 16000, \"scenes_short\": 1},\n\"parity_worst\": [[\"a.nss\", 17000, 1000]]\n}\n"), "{json}");
    }

    #[test]
    fn left_out_calls_are_neither_full_nor_gaps() {
        let mut m = Metrics::default();
        m.call("opcode 0x30", 2, vec![]);
        m.left_out("opcode 0x30", "the debug menu");
        assert_eq!((m.totals(), m.left_out_total()), ((2, 0), 1));
        let json = m.json("g", "e", "doc", "c", "d");
        assert!(json.contains("\"totals\": {\"calls\": 2, \"full\": 1, \"gaps\": 0, \"left_out\": 1, \"weighted\": 1.0}"), "{json}");
        assert!(json.contains("\"calls\": 2, \"gaps\": 0, \"left_out\": 1, \"reasons\""), "{json}");
    }

    #[test]
    fn share_is_the_arguments_read() {
        let r = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(super::share(3, &r(&["_VPLY argument 1 ignored"])), 2.0 / 3.0);
        assert_eq!(super::share(4, &r(&["_MPLY argument 1 ignored", "_MPLY option 17 ignored", "approximated"])), 0.5);
        assert_eq!(super::share(2, &r(&["not converted"])), 0.0);
        assert_eq!(super::share(0, &r(&["approximated"])), 1.0);
    }
}
