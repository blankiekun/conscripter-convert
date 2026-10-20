//! The game's variables against the ones conv-cons keeps for itself
//! ([`conv_core::alloc`]).

use conv_core::alloc;
use conv_core::ir::{Op, Operand, Script, Var};
use std::collections::BTreeSet;

/// One line per game variable that falls in a range conv-cons uses
/// (empty when none does).
// shortcut: looks at the variables the ops use, not the ones only a
// screen names (a gallery's unlock flags are the story's anyway).
pub fn clashes(script: &Script) -> Vec<String> {
    let border = script.global_border.unwrap_or(crate::GLOBAL_VAR_BASE);
    let mut used = BTreeSet::new();
    let mut add = |v: Var| {
        used.insert(alloc::number(v, border));
    };
    for op in &script.ops {
        match op {
            Op::Arith { var, value, .. } | Op::If { var, value, .. } => {
                add(*var);
                if let Operand::Var(v) = value {
                    add(*v);
                }
            }
            _ => {}
        }
    }
    let puzzle = script.ops.iter().any(|op| matches!(op, Op::TilePuzzle(_)));
    let reserved = |n| alloc::reserved(n).or((puzzle && alloc::PUZZLE.contains(&n)).then_some("the tile puzzle"));
    used.into_iter().filter_map(|n| reserved(n).map(|who| format!("game variable %{n} is conv-cons' ({who})"))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::ArithOp;

    #[test]
    fn a_game_flag_on_a_menu_global_is_reported() {
        let flag = Var { global: true, index: 107 };
        let script = Script { global_border: Some(800), ops: vec![Op::Arith { var: flag, op: ArithOp::Set, value: Operand::Const(1) }], ..Default::default() };
        assert_eq!(clashes(&script).len(), 1);
        let script = Script { global_border: Some(800), ops: vec![Op::Arith { var: Var { global: true, index: 99 }, op: ArithOp::Set, value: Operand::Const(1) }], ..Default::default() };
        assert!(clashes(&script).is_empty());
    }

    /// Every `%N` the code names, and the bases it counts from, are in
    /// the table.
    #[test]
    fn conv_cons_variables_are_in_the_table() {
        let mut dirs = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
        let mut files = Vec::new();
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() { dirs.push(path) } else { files.push(path) }
            }
        }
        for path in files.into_iter().filter(|p| p.file_name().is_some_and(|n| n != "tests.rs")) {
            let text = std::fs::read_to_string(&path).unwrap();
            let code = text.split("\nmod tests").next().unwrap();
            for (i, line) in code.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for (at, _) in line.match_indices('%') {
                    let digits: String = line[at + 1..].chars().take_while(char::is_ascii_digit).collect();
                    if let Ok(n) = digits.parse::<u32>() {
                        assert!(alloc::reserved(n).is_some() || alloc::PUZZLE.contains(&n), "{}:{}: %{n} is not in conv_core::alloc", path.display(), i + 1);
                    }
                }
            }
        }
        let bases = [crate::menu::VOICE_OFF_BASE];
        for n in bases.into_iter().chain((0..=crate::MAX_LINK_DEPTH).flat_map(|d| [crate::OVERLAY_SAVED + d, crate::NOISE_SAVED + d, crate::ANIMATION_SAVED + d])) {
            assert!(alloc::reserved(n).is_some(), "%{n} is not in conv_core::alloc");
        }
        for i in 0..crate::switches::MAX_INDEX {
            assert!(alloc::reserved(crate::switches::off_var(i)).is_some(), "switch {i}");
        }
    }
}
