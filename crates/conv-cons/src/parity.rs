//! Timing parity, our side: the time an emitted `0.txt` animates in each
//! scene (`metrics.json`'s `parity`, beside the motion time the source
//! asks for). A gap points at a timed move or fade the conversion lost,
//! such as a running pan swallowed by a later fade's sync.

use std::collections::{BTreeMap, HashMap, HashSet};

use conv_core::ir::SourceLoc;

/// Per scene, the ms `source` (an emitted `0.txt`, before encoding)
/// animates: `spmotion`'s last key, `print`/`bg` effects, `screenshake`,
/// `screenripple`, `screenpulse` and `screenzoom` times. A line counts
/// for its scene through `lines` (the source map from
/// [`crate::emit_mapped`]): its file when `is_scene` says it is one, else
/// the scene last seen (a function file the scene calls into); lines with
/// no source count nowhere. With `per_sprite`, a `print` counts its time
/// once per sprite changed since the last one (an engine whose every
/// picture fades on its own, NSS's `Fade`).
pub fn animated_ms(source: &str, lines: &[(usize, Option<SourceLoc>)], is_scene: impl Fn(&str) -> bool, per_sprite: bool) -> BTreeMap<String, u64> {
    let mut effects: HashMap<&str, u64> = HashMap::new();
    let mut out = BTreeMap::new();
    let mut runs = lines.iter().peekable();
    let (mut scene, mut here): (Option<&str>, Option<&str>) = (None, None);
    let mut changed: HashSet<&str> = HashSet::new();
    for (i, line) in source.lines().enumerate() {
        while runs.peek().is_some_and(|r| r.0 <= i + 1) {
            let at = runs.next().unwrap().1.as_ref();
            here = match at {
                Some(at) if is_scene(&at.file) => {
                    scene = Some(&at.file);
                    scene
                }
                Some(_) => scene,
                None => None,
            };
        }
        for cmd in commands(line) {
            let (name, args) = cmd.split_once(' ').unwrap_or((cmd, ""));
            let args: Vec<&str> = args.split(',').map(str::trim).collect();
            let int = |i: usize| args.get(i).and_then(|a| a.parse::<i64>().ok());
            let ms = match name {
                "effect" => {
                    if let (Some(n), Some(t)) = (args.first(), int(1)) {
                        effects.insert(n, if t > 1 { int(2).unwrap_or(0).max(0) as u64 } else { 0 });
                    }
                    continue;
                }
                "lsp" | "lsph" | "lsp2" | "lsph2" | "lsp2add" | "csp" | "csp2" | "msp" | "msp2" | "amsp" | "amsp2" | "vsp" | "vsp2" => {
                    changed.insert(args[0]);
                    continue;
                }
                "print" | "bg" => {
                    // `print N[,ms]`, `bg pic,N[,ms]`: effect N, or type N for ms
                    let e = usize::from(name == "bg");
                    let t = match (args.get(e), int(e + 1)) {
                        (Some(&n), Some(ms)) if n.parse::<i64>().is_ok_and(|n| n > 1) => ms.max(0) as u64,
                        (Some(&n), _) => effects.get(n).copied().unwrap_or(0),
                        _ => 0,
                    };
                    let sprites = if per_sprite { changed.len().max(1) as u64 } else { 1 };
                    changed.clear();
                    t * sprites
                }
                "spmotion" if args.len() > 2 => {
                    // num,loop[,k when loop 4],then keys of 8 starting with t
                    let first = if int(1) == Some(4) { 3 } else { 2 };
                    let keys = (args.len() - first) / 8;
                    if keys == 0 { 0 } else { int(first + (keys - 1) * 8).unwrap_or(0).max(0) as u64 }
                }
                // mode 3: one pass of its "x,y ..." list, an offset per interval
                "screenshake" if int(3) == Some(3) => {
                    let list = cmd.split('"').nth(1).unwrap_or("");
                    let n = list.split(|c: char| !(c.is_ascii_digit() || c == '-')).filter(|s| !s.is_empty()).count() / 2;
                    n as u64 * int(2).unwrap_or(0).max(0) as u64
                }
                "screenshake" | "screenpulse" | "screenzoom" => int(1).unwrap_or(0).max(0) as u64,
                "screenripple" => int(2).unwrap_or(0).max(0) as u64,
                _ => continue,
            };
            if let Some(s) = here
                && ms > 0
            {
                *out.entry(s.to_string()).or_default() += ms;
            }
        }
    }
    out
}

/// A line's commands: split at `:` outside quotes, comments dropped; an
/// `if` line's commands are left out (shortcut: conditional fades count 0,
/// count them when a parity gap points at one).
fn commands(line: &str) -> Vec<&str> {
    let mut v = Vec::new();
    let (mut start, mut quoted) = (0, false);
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ';' if !quoted => {
                v.push(&line[start..i]);
                start = line.len();
                break;
            }
            ':' if !quoted => {
                v.push(&line[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    v.push(&line[start.min(line.len())..]);
    let v: Vec<&str> = v.into_iter().map(str::trim).filter(|c| !c.is_empty()).collect();
    if v.iter().any(|c| c.starts_with("if ") || c.starts_with("notif ")) { Vec::new() } else { v }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(file: &str) -> Option<SourceLoc> {
        Some(SourceLoc { file: file.into(), index: 0 })
    }

    #[test]
    fn times_by_scene() {
        let src = "effect 11,10,1000\n\
                   lsp 960,\"a.png\",0,0:csp 961\n\
                   print 11\n\
                   spmotion 960,2,0,-320,0,100,100,0,256,0,16000,0,0,100,100,0,256,0\n\
                   print 10,500 ; a comment: print 11\n\
                   screenshake 1,500,16,0,1300,1700,10,10,10,10,500\n\
                   if %1 == 1 print 11\n\
                   bg black,11\n\
                   screenshake 4,0,33,3,\"1,0 -2,3 0,0\",1,0,66\n";
        // line 3 in a function file the scene calls into; 7 no source
        let lines = vec![(1, None), (2, at("s.nss")), (3, at("function.nss")), (5, at("t.nss")), (8, None), (9, at("t.nss"))];
        let scene = |f: &str| f != "function.nss";
        let per = animated_ms(src, &lines, scene, true);
        assert_eq!(per.get("s.nss"), Some(&(2000 + 16000)));
        assert_eq!(per.get("t.nss"), Some(&(500 + 500 + 99)));
        assert_eq!(animated_ms(src, &lines, scene, false).get("s.nss"), Some(&(1000 + 16000)));
    }
}
