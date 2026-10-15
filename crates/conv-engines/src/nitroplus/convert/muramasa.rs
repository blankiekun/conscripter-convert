//! 装甲悪鬼村正's own additions to the NSS compiler ([`MURAMASA`]): the
//! `sys_hmode.nss` functions that rank the heroines and decide who dies
//! (their loops and computed names are written as NScripter comparisons),
//! and the 立方陣 minigame, ported ([`magiccube`]); the favour screen,
//! ported ([`favour`]); the system screens, built from the game's own
//! pictures ([`screens`]).

use conv_core::ir::{ArithOp, Op, Operand};

use std::collections::BTreeMap;

use super::{negate, Compiler, Game, NssFile, Value};

mod extras;
mod favour;
pub mod magiccube;
mod opening;
pub mod screens;

/// The game's hooks; the CLI uses them for a game with `Muramasa.exe`.
pub const MURAMASA: Game = Game { call, scene, flags, files: &[magiccube::FILE], menus: screens::menus, subroutines: &[(screens::FAVOUR, "nss/sys_hmode.nss")], ..Game::PLAIN };

fn call(c: &mut Compiler<'_, '_>, name: &str) -> bool {
    if let Some(r) = RANKINGS.iter().find(|r| r.0 == name) {
        return c.ranking(r);
    }
    (name == "SatugaiSystem" && c.satugai()) || (name == "SetHex" && c.set_hex()) || (name == "judgment_of_count" && c.favour_screen())
}

fn scene(c: &mut Compiler<'_, '_>, file: &str, scene: &str) -> bool {
    if file == magiccube::FILE && scene == magiccube::FILE {
        c.magic_cube();
        return true;
    }
    false
}

/// The rankings' counters and results, the death and revival flags that
/// SatugaiSystem and SetHex set through names built at run time
/// (VariableValue), and the 立方陣's result.
fn flags(files: &BTreeMap<String, NssFile>) -> Vec<String> {
    let mut out = Vec::new();
    for (_, order, array, last) in RANKINGS {
        for n in order {
            out.extend([route_counter(n).to_string(), format!("{array}[{n}]")]);
        }
        out.push(last.to_string());
    }
    for (name, _) in HEX {
        out.extend([hex_counter(name), hex_flag(name, "死亡"), hex_flag(name, "復活")]);
    }
    out.push(magiccube::SOLVED.to_string());
    // Set by the story, read by the settings' voice window.
    out.extend(screens::VOICES.iter().map(|(name, _)| screens::heard_flag(name)));
    out.push(screens::CHAPTER_ONE_CLEARED.to_string());
    out.push(extras::CLEARED.to_string());
    out.extend(extras::flags(files));
    // The routes cleared and the title's marks for them (TitleSet).
    for route in ["英雄編", "復讐編", "魔王編", "悪鬼編"] {
        out.extend([format!("#{route}終了"), format!("#{route}通過")]);
    }
    out
}

/// Library functions that rank the heroines by their route counters with
/// a loop (`sys_hmode.nss`): written as NScripter comparisons instead.
/// Each counter counts up from its flag to 10, checked in this order, so
/// a heroine's rank is 1 + the number with a higher flag + the number
/// earlier in the order with the same flag. Results: `ARRAY["name"]` =
/// rank (1-4), `LAST` = the name ranked last.
const RANKINGS: [(&str, [&str; 4], &str, &str); 2] = [
    ("RouteChicker", ["アナザー", "一条", "香奈枝", "村正"], "$RC_N", "$ルート分岐用フラグ"),
    ("RouteChicker2", ["村正", "一条", "香奈枝", "アナザー"], "$RC_N2", "$ルート分岐用フラグ2"),
];

/// The heroines of `sys_hmode.nss` (`ArrayHex`): counter name and the
/// name in their `<name>死亡` / `<name>復活` flags, in `SetHex` order.
const HEX: [(&str, &str); 4] = [("Others", "その他"), ("Muramasa", "村正"), ("Kanae", "香奈枝"), ("Ichizyou", "一条")];
/// `SatugaiSystem`'s order: the first heroine whose counter is at least
/// every other one dies (善悪相殺).
const SATUGAI_ORDER: [&str; 4] = ["Others", "Ichizyou", "Kanae", "Muramasa"];

fn hex_counter(name: &str) -> String {
    format!("${name}_Flag")
}

fn hex_flag(name: &str, what: &str) -> String {
    let jp = HEX.iter().find(|h| h.0 == name).map_or(name, |h| h.1);
    format!("${jp}{what}")
}

/// The route counter of a heroine in [`RANKINGS`].
fn route_counter(name: &str) -> &'static str {
    match name {
        "一条" => "$Ichizyou_Flag",
        "香奈枝" => "$Kanae_Flag",
        "村正" => "$Muramasa_Flag",
        _ => "$Others_Flag",
    }
}

impl Compiler<'_, '_> {
    /// A [`RANKINGS`] function, if its counters are story flags.
    fn ranking(&mut self, &(_, order, array, last): &(&str, [&str; 4], &str, &str)) -> bool {
        use conv_core::ir::Compare;
        let counters: Option<Vec<_>> = order.iter().map(|n| self.flags.get(route_counter(n)).copied()).collect();
        let outputs: Option<Vec<_>> = order.iter().map(|n| self.flags.get(&format!("{array}[{n}]")).copied()).collect();
        let (Some(counters), Some(outputs), Some(&last_var)) = (counters, outputs, self.flags.get(last)) else { return false };
        for (x, (&fx, &out)) in counters.iter().zip(&outputs).enumerate() {
            self.ops.push(Op::Arith { var: out, op: ArithOp::Set, value: Operand::Const(1) });
            for (y, &fy) in counters.iter().enumerate().filter(|&(y, _)| y != x) {
                // y ranks above x: a higher flag, or the same flag and earlier.
                let above = if y < x { Compare::Ge } else { Compare::Gt };
                self.choices += 1;
                let skip = format!("rank{}", self.choices);
                self.ops.push(Op::If { var: fy, cmp: negate(above), value: Operand::Var(fx), label: skip.clone() });
                self.ops.push(Op::Arith { var: out, op: ArithOp::Add, value: Operand::Const(1) });
                self.ops.push(Op::Label(skip));
            }
            self.choices += 1;
            let skip = format!("rank{}", self.choices);
            let code = self.code(&Value::Str(order[x].to_string())).unwrap_or(0);
            self.ops.push(Op::If { var: out, cmp: Compare::Ne, value: Operand::Const(4), label: skip.clone() });
            self.ops.push(Op::Arith { var: last_var, op: ArithOp::Set, value: Operand::Const(code) });
            self.ops.push(Op::Label(skip));
            self.vars.insert(format!("{array}[{}]", order[x]), Value::Runtime(out));
        }
        self.vars.insert(last.to_string(), Value::Runtime(last_var));
        true
    }

    /// `SatugaiSystem` (sys_hmode.nss): the heroine with the highest
    /// counter dies, the first in [`SATUGAI_ORDER`] on a tie.
    fn satugai(&mut self) -> bool {
        use conv_core::ir::Compare;
        let get = |c: &Self, k: String| c.flags.get(&k).copied();
        let counters: Option<Vec<_>> = SATUGAI_ORDER.iter().map(|n| get(self, hex_counter(n))).collect();
        let deaths: Option<Vec<_>> = SATUGAI_ORDER.iter().map(|n| get(self, hex_flag(n, "死亡"))).collect();
        let (Some(counters), Some(deaths)) = (counters, deaths) else { return false };
        self.choices += 1;
        let done = format!("satugai{}", self.choices);
        for (x, (&fx, &death)) in counters.iter().zip(&deaths).enumerate() {
            self.choices += 1;
            let next = format!("satugai{}", self.choices);
            for (_, &fy) in counters.iter().enumerate().filter(|&(y, _)| y != x) {
                self.ops.push(Op::If { var: fx, cmp: Compare::Lt, value: Operand::Var(fy), label: next.clone() });
            }
            self.ops.push(Op::Arith { var: death, op: ArithOp::Set, value: Operand::Const(1) });
            self.ops.push(Op::Jump(done.clone()));
            self.ops.push(Op::Label(next));
            self.vars.insert(hex_flag(SATUGAI_ORDER[x], "死亡"), Value::Runtime(death));
        }
        self.ops.push(Op::Label(done));
        true
    }

    /// `SetHex` (sys_hmode.nss), the part that changes the story: a dead
    /// heroine's counter goes to 0; a revived one also loses her death and
    /// revival flags. (The rest updates the affection icons.)
    fn set_hex(&mut self) -> bool {
        use conv_core::ir::Compare;
        let vars: Option<Vec<_>> = HEX
            .iter()
            .map(|&(n, _)| Some((self.flags.get(&hex_counter(n)).copied()?, self.flags.get(&hex_flag(n, "死亡")).copied()?, self.flags.get(&hex_flag(n, "復活")).copied()?)))
            .collect();
        let Some(vars) = vars else { return false };
        for (i, &(counter, dead, revived)) in vars.iter().enumerate() {
            let zero = |var| Op::Arith { var, op: ArithOp::Set, value: Operand::Const(0) };
            self.choices += 1;
            let alive = format!("hex{}", self.choices);
            self.ops.push(Op::If { var: dead, cmp: Compare::Eq, value: Operand::Const(0), label: alive.clone() });
            self.ops.push(zero(counter));
            self.ops.push(Op::Label(alive));
            self.choices += 1;
            let stays = format!("hex{}", self.choices);
            self.ops.push(Op::If { var: revived, cmp: Compare::Eq, value: Operand::Const(0), label: stays.clone() });
            self.ops.extend([zero(counter), zero(dead), zero(revived)]);
            self.ops.push(Op::Label(stays));
            let name = HEX[i].0;
            for (key, var) in [(hex_counter(name), counter), (hex_flag(name, "死亡"), dead), (hex_flag(name, "復活"), revived)] {
                self.vars.insert(key, Value::Runtime(var));
            }
        }
        true
    }

    /// The 立方陣 minigame's content scene: the ported puzzle. It ends as
    /// the original does (`MathAnswer`): white, then `Delete("*")` and a
    /// black screen.
    fn magic_cube(&mut self) {
        self.sync(0);
        if self.window_visible {
            self.ops.push(Op::HideTextWindow);
            self.window_visible = false;
        }
        let easy = self.flags.get(magiccube::EASY).copied();
        let Some(&solved) = self.flags.get(magiccube::SOLVED) else { return };
        let Some(p) = magiccube::puzzle(self.assets, easy, solved) else {
            self.note("立方陣: a picture is missing");
            return;
        };
        self.ops.push(Op::TilePuzzle(Box::new(p)));
        self.vars.insert(magiccube::SOLVED.into(), Value::Runtime(solved));
        // The puzzle stops the music and clears the screen.
        self.objects.clear();
        self.aliases.clear();
        self.shown.clear();
    }
}
