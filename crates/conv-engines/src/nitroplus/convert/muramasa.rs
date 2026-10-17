//! 装甲悪鬼村正's own additions to the NSS compiler ([`MURAMASA`]): the
//! `sys_hmode.nss` functions that rank the heroines and decide who dies
//! (their loops and computed names are written as NScripter comparisons),
//! and the 立方陣 minigame, ported ([`magiccube`]); the favour screen,
//! ported ([`favour`]); the system screens, built from the game's own
//! pictures ([`screens`]).

use conv_core::ir::{ArithOp, ChoicePlates, ChoiceStyle, Op, Operand, TextWindow, TransitionStyle};

use std::collections::BTreeMap;

use super::{negate, BoxText, Compiler, Game, NssFile, Value};
use crate::nitroplus::english;

mod extras;
mod favour;
pub mod magiccube;
mod opening;
pub mod screens;
mod subtitles;

/// The game's hooks; the CLI uses them for a game with `Muramasa.exe`.
pub const MURAMASA: Game = Game { call, scene, flags, files: &[magiccube::FILE], menus: screens::menus, subroutines: &[(screens::FAVOUR, "nss/sys_hmode.nss")], replays: extras::replays, string_vars: true, shake_jolts: true, loaded_textures: true, fade_tempo: Some(super::tempo), ..Game::PLAIN };

/// Full Metal Daemon Muramasa (the English release's text in the Japanese
/// scripts): its port writes the vertical text boxes horizontally, 280
/// wide, font 20, `rowSpace` 1 under the font's height of 24
/// (`function_textbox.nut`; a 25 px pitch in the original), down to the
/// click icon at 500: 12 lines; every box in its own window picture
/// ([`EN_BOXES`]), with the text where [`EN_TEXT`] puts it.
// src: function_textbox.nut text0 (280 wide, font 20, rowSpace 1); text_drop 2: shot en/orig-page1
pub const MURAMASA_EN: Game = Game { call: call_en, menus: screens::menus_en, horizontal_box: Some((280, 300, 25)), box_text: &EN_TEXT, choice_style: choice_style_en, text_drop: 2, shake_jolts: false, fade_tempo: Some(diesel_tempo), ..MURAMASA };

/// The port's tempo of a `Fade` or `Transition` (its `DrawTransition`), as
/// [`super::tempo`]: the port's scenes name the NSS's tempos as their
/// `Axl1`-`3`, `Dxl1`-`3`, `Axl1d` (`AxlAuto`), `Dxl1d` (`DxlAuto`),
/// `AxlDxl1` (`AxlDxl`), `DxlAxl1` (`DxlAxl`), `null` (`Linear`), and the
/// exe's curves (its tempo table) give the same as Muramasa.exe's but the
/// last two: t² to the middle then out, and out to the middle then t².
// src: muramasa_en.exe's tempo table 0x5e40a0 (17: 0x40cec0, 18: 0x40cf50), docs/games/muramasa-en.md "Eased fades"
fn diesel_tempo(v: &Value) -> Option<(u8, u32)> {
    match v.string().as_str() {
        "AxlDxl" => Some((3, 2)),
        "DxlAxl" => Some((8, 2)),
        _ => super::tempo(v),
    }
}

/// The port's choices (`SetChoice` in `function_select.nut`, its
/// `LANG_NAME == "en"` branch): 440×50 plates (`cg/sys/select/en/
/// select001.png`, `select002.png` under the pointer) centred on the
/// screen, 100 px apart, the group centred vertically; each option
/// centred on the screen 13 px below its plate's top, in black.
/// shortcut: conv-cons has sprites for 4 picture-button options; the 8
/// selects of 5 or 6 are the plates in the window, their texts at the
/// plates' left and red under the pointer, until it has more.
fn choice_style_en(c: &mut Compiler<'_, '_>, n: i64) -> Option<ChoiceStyle> {
    // src: function_select.nut SetChoice (100 apart, text y + 13); FONT_LIST 選択肢 size 20
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
        // ours: the window's text inset for 5-6 options (the port has no such window)
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
        // ours: red under the pointer (the port changes the plate instead)
        return Some(ChoiceStyle { window, on_color: 0xA00000, off_color: 0, plates: None });
    }
    let (image, _, _) = c.assets.strip(&[FREE, "cg/sys/select/en/select002.png"])?;
    // src: function_select.nut (Fade "choice*/txt", "choice*/free" over 500 ms, in and out)
    let plates = ChoicePlates { image, x: x as i32, y: y as i32, step: PITCH as i32, text_x: (sw / 2) as i32, text_dy: 13, font_size: FONT as u32, colour: "#000000".into(), fade_ms: 500, shot: None };
    Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) })
}

/// The port's text places (`BoxInfo` in `val.nut`): box, text x, y and
/// width, the click-wait icon's place (`icon_pos`, the English table);
/// in pairs: the box with a face window first, as both may be shown,
/// then the same box without one (drawn for it after `<FW img=''>`).
// src: val.nut BoxInfo (LANG_NAME "en" table: text_pos(_fw), field_size(_fw), icon_pos)
const EN_TEXT: [BoxText; 12] = [
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
// src: function_textbox.nut CreateAllTextBox
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

/// Where the port puts a face (`Fw…`) or name plate (`Nw…`) picture of
/// `function_stand.nss`'s `FwC`, `NwC`, … (`$FwCName` = `FwC01` …): in
/// its window's frame (`FwInfo` in `val.nut`, the English table: 92 px
/// into the box, C 452,55; R 708,55; L 195,55; RR and LL as far into
/// theirs), not at the Japanese place (C 451,4; H 228,450) the NSS
/// `Move`s it to.
fn en_face(c: &Compiler<'_, '_>, name: &str) -> Option<(i64, i64)> {
    let kind = name.strip_prefix("Fw").map(|k| (k, true)).or(name.strip_prefix("Nw").map(|k| (k, false)))?;
    let (box_kind, face) = (kind.0.trim_end_matches(|c: char| c.is_ascii_digit()), kind.1);
    if box_kind.len() == kind.0.len() {
        return None;
    }
    // src: val.nut FwInfo (the "en" table): 92 px into the box at y 55; H 87/89,439
    // The box's x where it is up, else its usual place.
    let in_box = |b: &str, x: i64| Some((c.objects.get(&format!("box{b}T")).map_or(x, |o| o.x) + 92, 55));
    match (box_kind, face) {
        ("C", _) => in_box("C", 360),
        ("R", _) => in_box("R", 616),
        ("L", _) => in_box("L", 103),
        ("H", true) => Some((87, 439)),
        ("H", false) => Some((89, 439)),
        _ => None,
    }
}

/// The side boxes' x (with a face and without): the port's R and L, or
/// ma01_009's RR and LL (`SetFwRR`/`SetFwLL` where the NSS calls its only
/// `SetFwR`/`SetFwL`: three boxes side by side, `FwInfo` 690 and 28).
fn place_side_box(c: &mut Compiler<'_, '_>, side: &str, wide: bool) {
    // src: function_textbox.nut CreateAllTextBox (616, 103); val.nut FwInfo RR/LL (690, 28)
    let x = match (side, wide) {
        ("R", false) => 616,
        ("R", true) => 690,
        ("L", false) => 103,
        _ => 28,
    };
    for b in [format!("box{side}T"), format!("box{side}{side}")] {
        if let Some(o) = c.objects.get_mut(&b) {
            o.x = x;
        }
    }
}

/// [`call`], with the port's boxes for `function.nss`'s: each made at the
/// port's place, and the vertical ones not moved by the `Move` after them.
fn call_en(c: &mut Compiler<'_, '_>, name: &str, args: &[Value]) -> bool {
    // The box the text goes in when two are up (`SetTextEXL` with `boxRT`
    // shown for a caption: mb04_015vs).
    if let Some(k) = name.strip_prefix("SetTextEX") {
        c.text_box = Some(match k { "hyperR" => "R", "CColor" => "C", k => k }.to_string());
    }
    match name {
        "SetFwR" | "SetFwL" => place_side_box(c, &name[5..], c.file == "ma01_009.nss"),
        "SetTextEXR" | "SetTextEXL" => place_side_box(c, &name[9..], false),
        _ => {}
    }
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
        // The port's English captions (`english::put_captions`), drawn
        // across: the NSS's `SetFont` is for vertical text (`@`).
        ("CreateText", _) if args.len() == 8 && args[7].string() == english::CAPTION => {
            let font = c.font.clone();
            if let Some(face) = c.font.first_mut() {
                *face = Value::Str(face.string().trim_start_matches('@').to_string());
            }
            c.builtin("CreateText", &args[..7]);
            c.font = font;
            true
        }
        // The rest would draw the Japanese: left out. The choices'
        // labels (function_select.nss) are the engine's choice, in English.
        ("CreateText", _) if c.file != "function_select.nss" => {
            c.note("CreateText left out: the Japanese caption (English build)");
            true
        }
        ("Move", None) if args.len() >= 4 && en_face(c, &args[0].string()).is_some() => {
            let (x, y) = en_face(c, &args[0].string()).unwrap_or_default();
            let mut args = args.to_vec();
            (args[2], args[3]) = (Value::Int(x), Value::Int(y));
            c.builtin("Move", &args);
            true
        }
        ("ShowVOICESEText", _) => {
            c.voice_subtitle(args);
            true
        }
        // mb03_022's black bar behind its captions: the port's English
        // branch scales it across, `Scale("絵色100", 0, 620, 220)`
        // (mb03_022.nut), where the NSS stands it 500×1000 (the game's one
        // such `Zoom`; it runs from the text block, in function.nss).
        ("Zoom", _) if args.len() >= 4 && args[0].string() == "絵色100" && args[1..4] == [Value::Int(0), Value::Int(500), Value::Int(1000)] => {
            let mut args = args.to_vec();
            (args[2], args[3]) = (Value::Int(620), Value::Int(220));
            c.builtin("Zoom", &args);
            true
        }
        _ => call(c, name, args),
    }
}

fn call(c: &mut Compiler<'_, '_>, name: &str, args: &[Value]) -> bool {
    if let Some(r) = RANKINGS.iter().find(|r| r.0 == name) {
        return c.ranking(r);
    }
    // The ending's roll (SetRoll).
    (name == "CreateProcess" && args.get(4).is_some_and(|f| f.string().starts_with("ProcessRoll")) && c.inline_process(args))
        || (name == "RecoIn" && c.reco_in()) || (name == "SatugaiSystem" && c.satugai()) || (name == "SetHex" && c.set_hex()) || (name == "judgment_of_count" && c.favour_screen())
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
    // Each ending's roll seen (TheEND: `VariableValue(#, "ClearRoll%02d")`).
    // src: function.nss TheEND (ClearRoll00-05)
    out.extend((0..6).map(|n| format!("#ClearRoll{n:02}")));
    // The scenes 超速スキップ jumps over once read (SkipOut).
    let main_scene = |name: &str, f: &NssFile| f.items.iter().any(|i| matches!(i, crate::nitroplus::nss::Item::Scene(b) if b.name == format!("{name}_MAIN")));
    out.extend(files.iter().filter(|(name, f)| main_scene(name, f)).map(|(name, _)| super::read_flag(name)));
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
// src: sys_hmode.nss RouteChicker, RouteChicker2
const RANKINGS: [(&str, [&str; 4], &str, &str); 2] = [
    ("RouteChicker", ["アナザー", "一条", "香奈枝", "村正"], "$RC_N", "$ルート分岐用フラグ"),
    ("RouteChicker2", ["村正", "一条", "香奈枝", "アナザー"], "$RC_N2", "$ルート分岐用フラグ2"),
];

/// The heroines of `sys_hmode.nss` (`ArrayHex`): counter name and the
/// name in their `<name>死亡` / `<name>復活` flags, in `SetHex` order.
// src: sys_hmode.nss ArrayHex ($H_Name, $SatugaiRate)
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
        // src: extra_recollection.nss RecoIn (Fade 500, Wait 1000)
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
