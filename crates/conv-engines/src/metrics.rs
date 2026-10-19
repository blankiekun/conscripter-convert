//! How much of a game converts: each source command call (a built-in, a
//! task, an opcode) is either fully converted or has a gap, when the
//! converter drops it, approximates it or leaves an argument it was given
//! unread (any note made while the call is open), or is left out on
//! purpose (code the game can't reach, such as a debug menu: counted as
//! done in the completion figure, apart from the gaps). Each call also
//! counts its weighted share ([`share`]): the arguments it was given that
//! the converter read, and its [`credit`]: how much of it plays as in the
//! original, approximations counted. Every engine
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
    /// The calls' [`credit`]s summed (left-out and unmeasured calls not in it).
    pub credit: f64,
    /// Calls [`credit`] can't judge in this build (a file this install
    /// lacks): neither credit nor loss.
    pub unmeasured: usize,
    /// Each reason with the number of calls it was noted on.
    pub reasons: BTreeMap<String, usize>,
    /// [`credit`] "as decided": a loss the user chose ([`DROPPED`]) counts
    /// as done.
    pub decided: f64,
    /// Left-out calls the user chose to drop: lost "as original".
    pub dropped: usize,
}

/// A reason's prefix for what the user chose to drop: done "as decided"
/// (the headline `score`), lost "as original".
pub const DROPPED: &str = "user dropped: ";

/// What a player gets from a call: `score`'s channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Text,
    Sound,
    Picture,
    Motion,
    Timing,
    Movie,
    /// Not in an engine's table yet: scored, so it shows and gets classed.
    Unclassed,
    /// Flow, flags, bookkeeping: weighs nothing.
    None,
}

impl Channel {
    const SCORED: [Channel; 7] = [Channel::Text, Channel::Sound, Channel::Picture, Channel::Motion, Channel::Timing, Channel::Movie, Channel::Unclassed];

    pub fn name(self) -> &'static str {
        match self {
            Channel::Text => "text",
            Channel::Sound => "sound",
            Channel::Picture => "picture",
            Channel::Motion => "motion",
            Channel::Timing => "timing",
            Channel::Movie => "movie",
            Channel::Unclassed => "unclassed",
            Channel::None => "none",
        }
    }

    /// The user's weights (2026-10-18), renormalised over the channels a
    /// game has.
    fn weight(self) -> f64 {
        match self {
            Channel::Text => 0.30,
            Channel::Sound => 0.25,
            Channel::Picture => 0.20,
            Channel::Motion => 0.12,
            Channel::Timing => 0.08,
            Channel::Movie | Channel::Unclassed => 0.05,
            Channel::None => 0.0,
        }
    }
}

/// Each engine's table of its kinds' channels, by the engine's name.
fn channels(engine: &str) -> fn(&str) -> Channel {
    match engine {
        "nitroplus" => crate::nitroplus::convert::channel,
        "mages" => crate::mages::convert::channel,
        "typemoon" => crate::typemoon::convert::channel,
        "xuse" => crate::xuse::build::channel,
        _ => |_| Channel::Unclassed,
    }
}

/// `score`: per channel, the calls' credit over the calls measured
/// (left-out and unmeasured ones in neither), in per cent; the total
/// weighs the channels present. "As decided" (the headline) counts the
/// user's drops as done; `as_original` counts them as lost.
#[derive(Debug, PartialEq)]
pub struct Score {
    pub total: f64,
    pub as_original: f64,
    pub channels: Vec<(Channel, f64)>,
    /// Channels with calls this build can't judge, or with none measured.
    pub unmeasured: Vec<Channel>,
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
        match credit(args, &reasons) {
            Some(c) => {
                k.credit += c;
                let chosen: Vec<String> = reasons.iter().filter(|r| !r.starts_with(DROPPED)).cloned().collect();
                k.decided += credit(args, &chosen).unwrap_or(c);
            }
            None => k.unmeasured += 1,
        }
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
        k.dropped += usize::from(why.starts_with(DROPPED));
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
            k.credit += o.credit;
            k.unmeasured += o.unmeasured;
            k.decided += o.decided;
            k.dropped += o.dropped;
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

    /// The credits over every kind.
    pub fn credit_total(&self) -> f64 {
        self.kinds.values().map(|k| k.credit).sum()
    }

    /// Unmeasured calls, over every kind.
    pub fn unmeasured_total(&self) -> usize {
        self.kinds.values().map(|k| k.unmeasured).sum()
    }

    /// Calls left out on purpose, over every kind.
    pub fn left_out_total(&self) -> usize {
        self.kinds.values().map(|k| k.left_out).sum()
    }

    /// The [`Score`], each kind in the channel `channel` gives it.
    pub fn score(&self, channel: fn(&str) -> Channel) -> Score {
        // per channel: decided, original, calls measured, dropped left-out calls, unmeasured calls
        let mut sums = [(0.0, 0.0, 0usize, 0usize, 0usize); 7];
        for (name, k) in &self.kinds {
            let Some(i) = Channel::SCORED.iter().position(|&c| c == channel(name)) else { continue };
            let s = &mut sums[i];
            (s.0, s.1) = (s.0 + k.decided, s.1 + k.credit);
            s.2 += k.calls - k.left_out - k.unmeasured;
            (s.3, s.4) = (s.3 + k.dropped, s.4 + k.unmeasured);
        }
        let (mut channels, mut unmeasured) = (Vec::new(), Vec::new());
        let (mut w, mut decided, mut original) = (0.0, 0.0, 0.0);
        for (&c, &(d, o, n, dropped, um)) in Channel::SCORED.iter().zip(&sums) {
            if um > 0 || (n == 0 && c != Channel::Unclassed) {
                unmeasured.push(c);
            }
            if n == 0 {
                continue;
            }
            let (d, o) = (100.0 * d / n as f64, 100.0 * o / (n + dropped) as f64);
            channels.push((c, d));
            w += c.weight();
            (decided, original) = (decided + c.weight() * d, original + c.weight() * o);
        }
        let total = |x: f64| if w > 0.0 { x / w } else { 100.0 };
        Score { total: total(decided), as_original: total(original), channels, unmeasured }
    }

    /// The JSON the dry run writes: `commit` and `commit_date` each on a
    /// line of their own (`tests/snapshot.sh` compares without them), the
    /// kinds by gaps, each with its [`TOP`] reasons.
    pub fn json(&self, game: &str, engine: &str, engine_doc: &str, commit: &str, commit_date: &str) -> String {
        let channel = channels(engine);
        let score = self.score(channel);
        let (calls, gaps) = self.totals();
        let left_out = self.left_out_total();
        // "left_out" only when there are some, so other games' files stay as they were.
        let lo = |n: usize| if n > 0 { format!(", \"left_out\": {n}") } else { String::new() };
        let um = |n: usize| if n > 0 { format!(", \"unmeasured\": {n}") } else { String::new() };
        let mut s = format!(
            "{{\n\"game\": {},\n\"engine\": {},\n\"engine_doc\": {},\n\"commit\": {},\n\"commit_date\": {},\n\"totals\": {{\"calls\": {calls}, \"full\": {}, \"gaps\": {gaps}{}, \"weighted\": {:.1}, \"credit\": {:.1}{}, \"score\": {{\"total\": {:.2}, \"as_original\": {:.2}, \"channels\": {{{}}}, \"unmeasured\": [{}]}}}},\n\"kinds\": {{",
            q(game),
            q(engine),
            q(engine_doc),
            q(commit),
            q(commit_date),
            calls - gaps - left_out,
            lo(left_out),
            self.weighted_total(),
            self.credit_total(),
            um(self.unmeasured_total()),
            score.total,
            score.as_original,
            score.channels.iter().map(|(c, v)| format!("{}: {v:.2}", q(c.name()))).collect::<Vec<_>>().join(", "),
            score.unmeasured.iter().map(|c| q(c.name())).collect::<Vec<_>>().join(", ")
        );
        let mut kinds: Vec<_> = self.kinds.iter().collect();
        kinds.sort_by(|a, b| b.1.gaps.cmp(&a.1.gaps).then(b.1.calls.cmp(&a.1.calls)).then(a.0.cmp(b.0)));
        for (i, (name, k)) in kinds.iter().enumerate() {
            s += &format!(
                "{}\n{}: {{\"calls\": {}, \"gaps\": {}{}, \"reasons\": {}, \"weighted\": {:.1}, \"credit\": {:.1}{}, \"channel\": {}}}",
                if i > 0 { "," } else { "" },
                q(name),
                k.calls,
                k.gaps,
                lo(k.left_out),
                top(&k.reasons),
                k.weighted,
                k.credit,
                um(k.unmeasured),
                q(channel(name).name())
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

/// A call's credit, from 0 to 1: how much of it plays as in the original.
/// Unlike [`share`], an approximation counts: "not converted" and the
/// notes naming a whole call left unconverted (`built-in X`, `Request X`,
/// `CreateProcess X`, `CreateEffect X`) 0; a pick made when converting
/// (seeded `Random`) 0.8; each "… ignored" one argument lost, as in
/// [`share`]; any other note (an approximation) 0.5; the lowest wins.
/// `None` (unmeasured): a note says a file is missing in this build.
pub fn credit(args: usize, reasons: &[String]) -> Option<f64> {
    if reasons.iter().any(|r| r.contains("in this build")) {
        return None;
    }
    let whole_call = |r: &str| {
        ["built-in ", "Request ", "CreateProcess ", "CreateEffect "].iter().any(|p| r.strip_prefix(p).is_some_and(|rest| !rest.contains(' ')))
    };
    let mut c = share(args, reasons);
    for r in reasons {
        c = c.min(match r.as_str() {
            r if whole_call(r) || r.contains("not converted") => 0.0,
            r if r.ends_with(" ignored") => 1.0,
            r if r.contains("picked when converting") => 0.8,
            _ => 0.5,
        });
    }
    Some(c)
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
        let json = m.json("g", "e", "doc", "abc", "2026-10-16");
        assert!(json.contains("\"totals\": {\"calls\": 3, \"full\": 2, \"gaps\": 1, \"weighted\": 2.8, \"credit\": 2.8, \"score\": "), "{json}");
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
        let json = m.json("g", "e", "doc", "c", "d");
        assert!(json.ends_with("\"parity\": {\"source_ms\": 17500, \"ours_ms\": 1800, \"short_ms\": 16000, \"scenes_short\": 1},\n\"parity_worst\": [[\"a.nss\", 17000, 1000]]\n}\n"), "{json}");
    }

    #[test]
    fn left_out_calls_are_neither_full_nor_gaps() {
        let mut m = Metrics::default();
        m.call("opcode 0x30", 2, vec![]);
        m.left_out("opcode 0x30", "the debug menu");
        assert_eq!((m.totals(), m.left_out_total()), ((2, 0), 1));
        let json = m.json("g", "e", "doc", "c", "d");
        assert!(json.contains("\"totals\": {\"calls\": 2, \"full\": 1, \"gaps\": 0, \"left_out\": 1, \"weighted\": 1.0, \"credit\": 1.0, \"score\": "), "{json}");
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

    #[test]
    fn score_weighs_channels_and_the_users_drops() {
        use super::{Channel::*, DROPPED};
        let mut m = Metrics::default();
        m.call("Fade", 2, vec![]);
        m.call("Fade", 2, vec![format!("{DROPPED}Fade argument 1 ignored")]);
        m.left_out("Fade", &format!("{DROPPED}the user's choice"));
        m.left_out("Fade", "unreachable");
        m.call("LoadText", 1, vec!["approximated".into()]);
        m.call("CreateMovie", 7, vec!["CreateMovie: no movies in this build".into()]);
        m.call("SetAlias", 2, vec!["not converted".into()]);
        let s = m.score(crate::nitroplus::convert::channel);
        assert_eq!(s.channels, [(Text, 50.0), (Motion, 100.0)]);
        assert_eq!(s.unmeasured, [Sound, Picture, Timing, Movie]);
        // (0.30 x 50 + 0.12 x 100) / 0.42; as original motion is (1 + 0.5 + 0) / 3.
        assert!((s.total - 27.0 / 0.42).abs() < 1e-9, "{s:?}");
        assert!((s.as_original - 21.0 / 0.42).abs() < 1e-9, "{s:?}");
        let json = m.json("g", "nitroplus", "doc", "c", "d");
        assert!(json.contains("\"score\": {\"total\": 64.29, \"as_original\": 50.00, \"channels\": {\"text\": 50.00, \"motion\": 100.00}, \"unmeasured\": [\"sound\", \"picture\", \"timing\", \"movie\"]}}"), "{json}");
        assert!(json.contains("\"channel\": \"motion\"}"), "{json}");
    }

    #[test]
    fn credit_counts_approximations_and_unconverted_calls() {
        let r = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let c = |args, v: &[&str]| super::credit(args, &r(v));
        assert_eq!(c(4, &[]), Some(1.0));
        assert_eq!(c(4, &["Move argument 1 ignored"]), Some(0.75));
        assert_eq!(c(2, &["built-in SetSurface"]), Some(0.0));
        assert_eq!(c(2, &["Request stop"]), Some(0.0));
        assert_eq!(c(2, &["CreateEffect Lens"]), Some(0.0));
        assert_eq!(c(2, &["not converted: 0x30"]), Some(0.0));
        // Approximations, though they name a call.
        assert_eq!(c(2, &["Request Enter for another text than the next"]), Some(0.5));
        assert_eq!(c(7, &["CreateEffect KitanoBlue on part of the screen"]), Some(0.5));
        assert_eq!(c(1, &["Random picked when converting"]), Some(0.8));
        assert_eq!(c(4, &["Rotate about z", "Move argument 1 ignored"]), Some(0.5));
        assert_eq!(c(7, &["CreateMovie: no movies in this build"]), None);
        let mut m = super::Metrics::default();
        m.call("CreateMovie", 7, r(&["CreateMovie: no movies in this build"]));
        m.call("Rotate", 3, r(&["Rotate about z"]));
        let json = m.json("g", "e", "doc", "c", "d");
        assert!(json.contains("\"weighted\": 2.0, \"credit\": 0.5, \"unmeasured\": 1, \"score\": "), "{json}");
    }
}
