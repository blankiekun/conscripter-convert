//! 超速スキップ: each story file's `_MAIN` scene starts with
//! `if(($SYSTEM_skip && #SYSTEM_skip_express && $AllRead) || …) SkipOut();
//! else …`, which jumps over the whole scene when it was read before and
//! the player chose 超速スキップ. Compiled as a runtime test of
//! [`Op::IfFastSkip`] and the scene's read flag, a global the game
//! registers ([`read_flag`]) and the test sets once the scene has run.
//! (`$AllRead` is the exe's "every line read"; a scene played through, or
//! jumped over, counts as read here.)

use super::*;

/// The story flag that a file's scene was read.
pub fn read_flag(file: &str) -> String {
    format!("#conv_read:{file}")
}

/// Whether `e` reads `#SYSTEM_skip_express`.
fn reads_skip_express(e: &Expr) -> bool {
    match e {
        Expr::Var(v) => v.to_string() == "#SYSTEM_skip_express",
        Expr::Unary { expr, .. } => reads_skip_express(expr),
        Expr::Binary { left, right, .. } => reads_skip_express(left) || reads_skip_express(right),
        _ => false,
    }
}

impl<'a> Compiler<'a, '_> {
    /// The read flag of the current file, when `cond` is its `SkipOut` test
    /// and the game keeps read flags.
    pub(super) fn fast_skip_test(&self, cond: &Expr) -> Option<conv_core::ir::Var> {
        if !reads_skip_express(cond) {
            return None;
        }
        self.flags.get(&read_flag(self.file)).copied()
    }

    /// Jumps to `label` unless the scene was read and 超速スキップ is on.
    pub(super) fn jump_unless_fast_skip(&mut self, read: conv_core::ir::Var, label: &str) {
        self.choices += 1;
        let skip = format!("fastskip{}", self.choices);
        self.ops.push(Op::If { var: read, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(0), label: label.to_string() });
        self.ops.push(Op::IfFastSkip { label: skip.clone() });
        self.ops.push(Op::Jump(label.to_string()));
        self.ops.push(Op::Label(skip));
    }
}
