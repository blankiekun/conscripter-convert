//! 装甲悪鬼村正's own additions to the NSS compiler ([`MURAMASA`]): the
//! `sys_hmode.nss` functions that rank the heroines and decide who dies
//! (their loops and computed names are written as NScripter comparisons),
//! and the 立方陣 minigame, ported ([`magiccube`]); the favour screen,
//! ported ([`favour`]); the system screens, built from the game's own
//! pictures ([`screens`]).

use conv_core::ir::{ArithOp, ChoicePlates, ChoiceStyle, Op, Operand, TextWindow, TransitionStyle};

use std::collections::BTreeMap;

use super::{negate, Compiler, Game, NssFile, Value};

mod extras;
mod favour;
pub mod magiccube;
mod opening;
pub mod screens;

/// The game's hooks; the CLI uses them for a game with `Muramasa.exe`.
pub const MURAMASA: Game = Game { call, scene, flags, files: &[magiccube::FILE], menus: screens::menus, subroutines: &[(screens::FAVOUR, "nss/sys_hmode.nss")], replays: extras::replays, ..Game::PLAIN };

/// Full Metal Daemon Muramasa (the English release's text in the Japanese
/// scripts): its port writes the vertical text boxes horizontally, 280
/// wide, font 20, `rowSpace` 1 (`function_textbox.nut`), down to the
/// click icon at 500: 14 lines; every box in its own window picture
/// ([`EN_BOXES`]), with the text where [`EN_TEXT`] puts it.
pub const MURAMASA_EN: Game = Game { call: call_en, menus: screens::menus_en, horizontal_box: Some((280, 300, 21)), box_text: &EN_TEXT, choice_style: choice_style_en, ..MURAMASA };

/// The port's choices (`SetChoice` in `function_select.nut`, its
/// `LANG_NAME == "en"` branch): 440×50 plates (`cg/sys/select/en/
/// select001.png`, `select002.png` under the pointer) centred on the
/// screen, 100 px apart, the group centred vertically; each option
/// centred on the screen 13 px below its plate's top, in black.
/// shortcut: conv-cons has sprites for 4 picture-button options; the 8
/// selects of 5 or 6 are the plates in the window, their texts at the
/// plates' left and red under the pointer, until it has more.
fn choice_style_en(c: &mut Compiler<'_, '_>, n: i64) -> Option<ChoiceStyle> {
    const PITCH: i64 = 100;
    const FONT: i64 = 20;
    const FREE: &str = "cg/sys/select/en/select001.png";
    let (plate, w, h) = c.assets.image(FREE)?;
    let (w, h) = (w as i64, h as i64);
    let (sw, sh) = c.game.screen;
    let (x, y) = ((sw - w) / 2, (sh - (n - 1) * PITCH - h) / 2);
    let parts: Vec<_> = (0..n).map(|i| (plate.clone(), x, y + PITCH * i, 1000)).collect();
    let (image, wx, wy) = c.assets.composite(&parts)?;
    let window = TextWindow {
        wait_icon_at: None,
        image,
        x: wx as i32,
        y: wy as i32,
        text_x: (x + 30) as i32,
        text_y: (y + 13) as i32,
        columns: (w / FONT) as u32,
        rows: n as u32,
        font_size: FONT as u32,
        line_gap: (PITCH - FONT) as u32,
        vertical: false,
        name_plate: None,
    };
    if n > 4 {
        return Some(ChoiceStyle { window, on_color: 0xA00000, off_color: 0, plates: None });
    }
    let (image, _, _) = c.assets.strip(&[FREE, "cg/sys/select/en/select002.png"])?;
    let plates = ChoicePlates { image, x: x as i32, y: y as i32, step: PITCH as i32, text_x: (sw / 2) as i32, text_dy: 13, font_size: FONT as u32, colour: "#000000".into(), fade_ms: 300 };
    Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) })
}

/// The port's text places (`BoxInfo` in `val.nut`): box, text x, y and
/// width, the click-wait icon's place (`icon_pos`, the English table);
/// the ones with a face window first, as both may be shown.
const EN_TEXT: [(&str, i64, i64, i64, (i32, i32)); 12] = [
    ("boxCT", 375, 190, 280, (640, 500)),
    ("boxCC", 375, 190, 280, (640, 500)),
    ("boxRT", 630, 190, 280, (896, 500)),
    ("boxRR", 630, 190, 280, (896, 500)),
    ("boxLT", 118, 190, 280, (383, 500)),
    ("boxLL", 118, 190, 280, (383, 500)),
    ("boxTH", 238, 426, 701, (924, 554)),
    ("boxH", 94, 426, 845, (924, 554)),
    ("boxCTH", 360, 225, 420, (776, 580)),
    ("boxCH", 280, 225, 501, (776, 580)),
    ("boxTNoh", 351, 225, 434, (776, 329)),
    ("boxNoh", 280, 225, 492, (776, 329)),
];

/// The port's boxes (`CreateAllTextBox` in `function_textbox.nut`): name,
/// x, y, picture in `cg/sys/adv/en/`. The vertical ones are 305×475 at y
/// 51; the horizontal ones 880×158, at the Japanese y (`None`, moved by
/// the `Move` after them as there).
const EN_BOXES: [(&str, i64, Option<i64>, &str); 10] = [
    ("boxCC", 360, Some(51), "TextWindow"),
    ("boxCT", 360, Some(51), "TextWindow_Fw"),
    ("boxRR", 616, Some(51), "TextWindow"),
    ("boxRT", 616, Some(51), "TextWindow_Fw"),
    ("boxLL", 103, Some(51), "TextWindow"),
    ("boxLT", 103, Some(51), "TextWindow_Fw"),
    ("boxH", 72, None, "THTextWindow"),
    ("boxTH", 72, None, "THTextWindow_Fw"),
    ("boxCH", 264, None, "THTextWindow"),
    ("boxCTH", 224, None, "THTextWindow_Fw"),
];

/// [`call`], with the port's boxes for `function.nss`'s: each made at the
/// port's place, and the vertical ones not moved by the `Move` after them.
fn call_en(c: &mut Compiler<'_, '_>, name: &str, args: &[Value]) -> bool {
    let en_box = match args.first() {
        Some(Value::Str(s)) => EN_BOXES.iter().find(|b| b.0 == s),
        _ => None,
    };
    match (name, en_box) {
        ("CreateTexture", Some(&(_, x, y, picture))) if args.len() == 5 => {
            let mut args = args.to_vec();
            args[2] = Value::Int(x);
            if let Some(y) = y {
                args[3] = Value::Int(y);
            }
            args[4] = Value::Str(format!("cg/sys/adv/en/{picture}.png"));
            c.builtin("CreateTexture", &args);
            true
        }
        ("Move", Some((_, _, Some(_), _))) => true,
        _ => call(c, name, args),
    }
}

fn call(c: &mut Compiler<'_, '_>, name: &str, _: &[Value]) -> bool {
    if let Some(r) = RANKINGS.iter().find(|r| r.0 == name) {
        return c.ranking(r);
    }
    (name == "RecoIn" && c.reco_in()) || (name == "SatugaiSystem" && c.satugai()) || (name == "SetHex" && c.set_hex()) || (name == "judgment_of_count" && c.favour_screen())
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

    /// `RecoIn` (`extra_recollection.nss`), at a recollection scene's
    /// start: the held list ([`extras::HOLD_SLOT`]) fades out over 500 ms
    /// onto the scene's first picture, then 1 s passes.
    fn reco_in(&mut self) -> bool {
        self.sync(0);
        self.ops.extend([
            Op::BeginTransition { effect: "Fade".into(), fade_ms: 500, mask: None, style: TransitionStyle::Fade },
            Op::ScreenEffect { slot: extras::HOLD_SLOT, effect: None },
            Op::EndTransition,
            Op::Pause { ms: 1000 },
        ]);
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
