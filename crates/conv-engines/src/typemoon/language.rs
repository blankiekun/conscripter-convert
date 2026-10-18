//! 月姫 in both languages in one script: the Japanese copy (labels
//! `tm_<script>…`) and the fan patch's English (`tm_en_<script>…`), with
//! the same flags (the Japanese numbering; flags only the English names
//! come after). A switch ([`SWITCH`]) goes into the other copy at once:
//!
//! - at the same page, when the script's ops up to it are the same in
//!   both copies but for the text (so the screen, sound and flags are too);
//! - else where the scene was entered (a script's start, or a label
//!   another script's `_JUMP` goes to), put back as it was there: each
//!   entry is saved into [`SLOT`] and loaded, like a load. After a load
//!   from the save screen that slot may be of another visit: then the
//!   scene is entered again from a cleared screen with the flags as they
//!   are.
//!
//! The language is the copy the script runs in, so a save keeps it; the
//! engine is told it at the start, on each switch and after each load
//! (`language`, not saved). A new game starts in the language last chosen.

use std::collections::{BTreeMap, HashSet};

use conv_core::ir::{ArithOp, Compare, Op, Operand, TextWindow, Var};

use super::convert::{self, Compiled, Game};

/// The switch (the system menu's Config tab for now).
pub const SWITCH: &str = "tm_switch";
/// Gosubbed after every load.
pub const LOADED: &str = "tm_loaded";
/// The English copy's scripts: `tm_en_<script>`.
const EN: &str = "en_";
/// The save slot of the entry.
// ours: a slot neither the save list (1-100) nor the title's Continue reads
const SLOT: u32 = 897;
// ours: globals %1800-%1802 (alloc.rs: free %1111-%1999, after the `_RDST` flags)
/// The language last chosen (1: English): a new game starts in it.
const PREF: Var = Var { global: true, index: 1000 };
/// The entry whose state [`SLOT`] holds; -1: none to trust.
const SAVED: Var = Var { global: true, index: 1001 };
/// 1 while [`SLOT`] is loaded for a switch.
const PENDING: Var = Var { global: true, index: 1002 };

/// The switch's locals, after the flags.
#[derive(Clone, Copy)]
struct Locals {
    /// The script running (its index).
    script: Var,
    /// The entry it was entered by (its number in [`Entries`]).
    entry: Var,
    /// The page shown, from 1, while it is one of the aligned ones; else -1.
    page: Var,
    /// 1: the English copy.
    lang: Var,
}

/// The entries by their Japanese label, numbered.
type Entries = BTreeMap<String, i32>;

fn set(var: Var, value: i32) -> Op {
    Op::Arith { var, op: ArithOp::Set, value: Operand::Const(value) }
}

fn when(var: Var, cmp: Compare, value: Operand, label: String) -> Op {
    Op::If { var, cmp, value, label }
}

/// A label of the English copy as the Japanese copy's.
fn ja_label(l: &str) -> String {
    l.replacen("tm_en_", "tm_", 1)
}

/// The same label in copy `prefix`.
fn in_copy(ja: &str, prefix: &str) -> String {
    format!("tm_{prefix}{}", &ja[3..])
}

/// An op as the copies are compared: without its text, the English copy's
/// labels and pictures named as the Japanese.
// shortcut: a picture of the fan patch's own (pic/en/) counts as the same; the switch keeps the one shown until it changes.
fn key(op: &Op) -> String {
    match op {
        Op::Text { .. } => "text".into(),
        Op::Choice { options, style, disabled } => format!("{:?}", Op::Choice { options: options.iter().map(|(_, l)| (String::new(), l.clone())).collect(), style: style.clone(), disabled: disabled.clone() }),
        _ => format!("{op:?}"),
    }
    .replace("tm_en_", "tm_")
    .replace("pic/en/", "pic/")
}

/// The pages at the start of a script whose ops up to them are the same
/// in both copies but for the text.
// shortcut: a page reached by a jump back from past the first difference counts as the same; no script found doing that.
fn aligned(ja: &[Op], en: &[Op]) -> usize {
    ja.iter().zip(en).take_while(|(a, b)| key(a) == key(b)).filter(|(a, _)| matches!(a, Op::Text { .. })).count()
}

/// The ops of each script of `c`, by index.
fn scripts(c: &Compiled) -> Vec<&[Op]> {
    (0..c.starts.len()).map(|i| &c.ops[c.starts[i]..c.starts.get(i + 1).copied().unwrap_or(c.ops.len())]).collect()
}

/// Where a scene can be entered, in both copies: each script's start, and
/// the labels a jump from another script goes to.
fn entries(copies: [&[&[Op]]; 2]) -> Entries {
    let mut found = Vec::new();
    let mut both: Option<HashSet<String>> = None;
    for copy in copies {
        let mut owner = BTreeMap::new();
        for (s, ops) in copy.iter().enumerate() {
            for op in *ops {
                if let Op::Label(l) = op {
                    owner.insert(ja_label(l), s);
                }
            }
        }
        for (s, ops) in copy.iter().enumerate() {
            if let Some(Op::Label(l)) = ops.first() {
                found.push(ja_label(l));
            }
            for op in *ops {
                if let Op::Jump(t) | Op::If { label: t, .. } = op {
                    let t = ja_label(t);
                    if owner.get(&t).is_some_and(|&o| o != s) {
                        found.push(t);
                    }
                }
            }
        }
        let labels: HashSet<String> = owner.into_keys().collect();
        both = Some(match both {
            Some(b) => b.intersection(&labels).cloned().collect(),
            None => labels,
        });
    }
    let both = both.unwrap_or_default();
    // shortcut: an entry only one copy has gets no entry block (none in 1.0.2 and the fan patch).
    let found: std::collections::BTreeSet<String> = found.into_iter().filter(|l| both.contains(l)).collect();
    found.into_iter().zip(0..).collect()
}

/// Copy `prefix`'s scripts with the switch's marks: at each entry the
/// script and entry numbers and the state saved, and before each page its
/// number while it is one of the script's first `aligned` pages (else -1).
fn marked(out: &mut Vec<Op>, copy: &[&[Op]], prefix: &str, other: &str, aligned: &[usize], entries: &Entries, v: Locals) {
    for (s, ops) in copy.iter().enumerate() {
        let mut k = 0;
        for op in *ops {
            if matches!(op, Op::Text { .. }) {
                k += 1;
                if k <= aligned[s] {
                    out.push(Op::Label(format!("tm_{prefix}lang_p{s}_{k}")));
                    out.push(set(v.page, k as i32));
                } else {
                    out.push(set(v.page, -1));
                }
            }
            out.push(op.clone());
            if let Op::Label(l) = op {
                let ja = ja_label(l);
                if let Some(&e) = entries.get(&ja) {
                    out.extend([set(v.script, s as i32), set(v.entry, e), set(v.page, -1), Op::SaveState { slot: SLOT }, set(SAVED, e)]);
                    out.push(when(PENDING, Compare::Eq, Operand::Const(1), format!("{}__lang_in", in_copy(&ja, other))));
                    out.push(Op::Label(format!("{l}__lang_go")));
                }
            }
        }
    }
}

/// Both copies in one script: Japanese `ja`, English `en` (the pictures
/// in `own` from `pic/en/`), each with its window.
pub fn compile(ja: &Game, en: &Game, own: &HashSet<String>, ja_window: TextWindow, en_window: TextWindow) -> Compiled {
    // shortcut: both copies have the same scripts in the same order (1.0.2 and the 2023-09-05 fan patch do); match them by name if a patch differs.
    assert!(en.scripts.iter().map(|(n, _)| n).eq(ja.scripts.iter().map(|(n, _)| n)), "the English copy's scripts differ from the Japanese");
    let mut vars = convert::variables(ja);
    let mask = convert::choice_mask(&vars);
    // ours: the English copy's own flags after the choice mask, then the switch's locals
    let mut next = mask.index + 1;
    let mut more: Vec<(String, Var)> = convert::variables(en).into_iter().filter(|(n, _)| !vars.contains_key(n)).collect();
    more.sort_by(|a, b| a.0.cmp(&b.0));
    for (n, v) in more {
        let v = if v.global {
            v
        } else {
            next += 1;
            Var { global: false, index: next - 1 }
        };
        vars.insert(n, v);
    }
    let local = |i: u16| Var { global: false, index: next + i };
    let v = Locals { script: local(0), entry: local(1), page: local(2), lang: local(3) };
    let a = convert::compile_copy(ja, "", vars.clone(), mask, &HashSet::new());
    let b = convert::compile_copy(en, EN, vars, mask, own);
    let (sa, sb) = (scripts(&a), scripts(&b));
    let aligned: Vec<usize> = sa.iter().zip(&sb).map(|(x, y)| aligned(x, y)).collect();
    let entries = entries([&sa, &sb]);

    let enter = |c: &str| format!("tm_{c}lang");
    let first = match sa.first().and_then(|o| o.first()) {
        Some(Op::Label(l)) => l.clone(),
        _ => return a,
    };
    // Each copy's ops start from the same sound channels (never run: after
    // a jump), so a channel the one started the other stops.
    let mut ops = vec![when(PREF, Compare::Eq, Operand::Const(1), "tm_en_lang_new".into()), Op::Gosub(enter("")), Op::Jump(first.clone()), Op::SeStopEvery];
    marked(&mut ops, &sa, "", EN, &aligned, &entries, v);
    ops.push(Op::ReturnToTitle);
    // Here, before the English copy: its window is the one the text is laid out in.
    ops.extend([Op::Label(enter(EN)), set(v.lang, 1), set(PREF, 1), Op::Language("en".into()), Op::SetWindow(en_window), Op::Return]);
    ops.extend([Op::Label("tm_en_lang_new".into()), Op::Gosub(enter(EN)), Op::Jump(in_copy(&first, EN)), Op::SeStopEvery]);
    marked(&mut ops, &sb, EN, "", &aligned, &entries, v);
    ops.push(Op::ReturnToTitle);
    ops.extend([Op::Label(enter("")), set(v.lang, 0), set(PREF, 0), Op::Language("ja".into()), Op::SetWindow(ja_window), Op::Return]);

    // Into a copy at an entry, from the other's (its state loaded).
    for prefix in ["", EN] {
        for l in entries.keys() {
            let l = in_copy(l, prefix);
            ops.extend([Op::Label(format!("{l}__lang_in")), set(PENDING, 0), Op::Gosub(enter(prefix)), Op::Jump(format!("{l}__lang_go"))]);
        }
    }
    ops.extend([Op::Label(SWITCH.into()), when(v.lang, Compare::Eq, Operand::Const(1), "tm_switch_to".into())]);
    for prefix in [EN, ""] {
        let to = |part: &str| format!("tm_switch_to{}", if part.is_empty() { String::new() } else { format!("_{prefix}{part}") });
        if prefix.is_empty() {
            ops.push(Op::Label(to("")));
        }
        ops.extend([Op::Gosub(enter(prefix)), when(v.page, Compare::Lt, Operand::Const(1), to("fb"))]);
        for (s, &n) in aligned.iter().enumerate() {
            if n > 0 {
                ops.push(when(v.script, Compare::Eq, Operand::Const(s as i32), to(&format!("s{s}"))));
            }
        }
        ops.push(Op::Jump(to("fb")));
        for (s, &n) in aligned.iter().enumerate() {
            if n > 0 {
                ops.push(Op::Label(to(&format!("s{s}"))));
                ops.extend((1..=n).map(|k| when(v.page, Compare::Eq, Operand::Const(k as i32), format!("tm_{prefix}lang_p{s}_{k}"))));
                ops.push(Op::Jump(to("fb")));
            }
        }
        // The entry: loaded when the slot holds this visit's.
        ops.extend([Op::Label(to("fb")), when(SAVED, Compare::Ne, Operand::Var(v.entry), to("reset")), set(PENDING, 1), Op::LoadState { slot: SLOT }]);
        ops.extend([Op::Label(to("reset")), set(PENDING, 0), Op::ClearSprites, Op::BgmStop { fade_out_ms: 0 }, Op::SeStopEvery]);
        ops.extend(entries.iter().map(|(l, &e)| when(v.entry, Compare::Eq, Operand::Const(e), in_copy(l, prefix))));
        ops.push(Op::ReturnToTitle);
    }
    // After a load: the language of the copy loaded into.
    ops.extend([
        Op::Label(LOADED.into()),
        when(PENDING, Compare::Eq, Operand::Const(1), "tm_loaded_lang".into()),
        set(SAVED, -1),
        Op::Label("tm_loaded_lang".into()),
        when(v.lang, Compare::Eq, Operand::Const(1), "tm_loaded_en".into()),
        Op::Language("ja".into()),
        Op::Return,
        Op::Label("tm_loaded_en".into()),
        Op::Language("en".into()),
        Op::Return,
    ]);
    Compiled { ops, notes: a.notes, metrics: a.metrics, starts: Vec::new(), source_map: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str) -> Op {
        Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false }
    }

    #[test]
    fn pages_line_up_until_the_copies_differ_but_for_text() {
        let ja = [Op::Label("tm_S".into()), text("あ"), Op::Jump("tm_S__L".into()), text("い"), Op::Wait { ms: 1 }, text("う")];
        let en = [Op::Label("tm_en_S".into()), text("A"), Op::Jump("tm_en_S__L".into()), text("B"), Op::Wait { ms: 2 }, text("C")];
        assert_eq!(aligned(&ja, &en), 2);
        let pick = |o: &str| Op::Choice { options: vec![(o.into(), "tm_S__1".into())], style: None, disabled: vec![] };
        assert_eq!(key(&pick("はい")), key(&pick("Yes")));
    }

    #[test]
    fn entries_are_starts_and_jumps_from_other_scripts() {
        let a: [&[Op]; 2] = [&[Op::Label("tm_A".into()), Op::Label("tm_A__L".into()), Op::Jump("tm_B__M".into())], &[Op::Label("tm_B".into()), Op::Label("tm_B__M".into()), Op::Jump("tm_B__N".into()), Op::Label("tm_B__N".into())]];
        let b: [&[Op]; 2] = [&[Op::Label("tm_en_A".into()), Op::Label("tm_en_A__L".into()), Op::Jump("tm_en_B__M".into())], &[Op::Label("tm_en_B".into()), Op::Label("tm_en_B__M".into()), Op::Jump("tm_en_B__N".into()), Op::Label("tm_en_B__N".into())]];
        let e = entries([&a, &b]);
        assert_eq!(e.keys().collect::<Vec<_>>(), ["tm_A", "tm_B", "tm_B__M"]);
    }

    #[test]
    fn marks_entries_and_each_page_while_aligned() {
        let ja: &[Op] = &[Op::Label("tm_S".into()), text("あ"), text("い")];
        let v = Locals { script: Var { global: false, index: 1 }, entry: Var { global: false, index: 2 }, page: Var { global: false, index: 3 }, lang: Var { global: false, index: 4 } };
        let mut out = Vec::new();
        marked(&mut out, &[ja], "", EN, &[1], &[("tm_S".to_string(), 7)].into(), v);
        let labels: Vec<&str> = out.iter().filter_map(|o| if let Op::Label(l) = o { Some(l.as_str()) } else { None }).collect();
        assert_eq!(labels, ["tm_S", "tm_S__lang_go", "tm_lang_p0_1"]);
        assert!(out.contains(&when(PENDING, Compare::Eq, Operand::Const(1), "tm_en_S__lang_in".into())));
        assert!(out.contains(&set(SAVED, 7)));
        // The second page is past the aligned one: no number.
        assert_eq!(out[out.len() - 2], set(v.page, -1));
    }
}
