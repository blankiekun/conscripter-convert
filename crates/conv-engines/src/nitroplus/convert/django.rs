//! 続・殺戮のジャンゴ -地獄の賞金首- (Nitroplus, 2008)'s own additions to
//! the NSS compiler ([`DJANGO`]): an 800×600 screen, the boot script
//! `boot.nss`, whose `#include`s are the helper library, and the story
//! driver compiled whole.

use conv_core::ir::{ChoicePlates, ChoiceShot, ChoiceStyle, TextWindow};

use super::{Compiler, Game, Value};

mod battle;
mod counter;
mod loops;
mod roll;
pub mod extras;
pub mod screens;

/// The game's hooks; the CLI uses them for a game with `Django.exe`.
pub const DJANGO: Game = Game {
    screen: (800, 600),
    boot: "boot.nss",
    story: Some((STORY, "SelectStoryMode")),
    call,
    relative_children: true,
    loaded_textures: true,
    chapter_objects: true,
    text_drop: 7,
    family: true,
    string_vars: true,
    flags: extras::flags,
    subroutines: &extras::SCENES,
    choice_style,
    story_end: &["InitBGM", "TitleCongratulations"],
    menus: screens::menus,
    ..Game::PLAIN
};

/// The functions compiled as the game's own: the battle's movies, its
/// counter and its muzzle flashes, the looping shakes, the strobe and
/// the ending's credits.
fn call(c: &mut Compiler<'_, '_>, name: &str, args: &[Value]) -> bool {
    if c.roll(name, args) {
        return true;
    }
    match name {
        "VS400" => c.vs400(args),
        "GunFire" => c.gun_fire(args),
        "CountInit" => {
            c.count_init();
            true
        }
        "CounterPlay" => {
            c.counter_play(args);
            true
        }
        "CreateProcess" => args.get(4).is_some_and(|f| c.count_process(&f.string())) || c.loop_process(args),
        "Delete" => {
            c.stop_shakes(args);
            false
        }
        "WaitAction" if args.first().is_some_and(|o| o.string().trim_start_matches('@') == "カウント１") => {
            c.count_wait();
            true
        }
        _ => false,
    }
}

/// The story driver: its `SelectStoryMode` (what `main` runs unless
/// `$SelectDebug`, the developers' chapter select, which the boot script
/// clears) calls `main_root`, which calls the 56 chapter files in turn
/// and ends the game after a death (a bad end).
const STORY: &str = "dja000_開始スクリプト.nss";

/// `SetChoice02` (function_select.nss): 530×68 plates (`select001.png`,
/// `select002.png` under the pointer) one under the other from (138, 244),
/// the options in white 22 px text centred about x 386 (not the plate's
/// centre, 403), their tops 21 px below the plate's (measured under
/// wine); they fade in over 300 ms. The window is the same plates for a
/// game without [`ChoicePlates`].
fn choice_style(c: &mut Compiler<'_, '_>, n: i64) -> Option<ChoiceStyle> {
    const X: i64 = 138;
    const Y: i64 = 244;
    const FONT: i64 = 22;
    const INSET: i64 = 40;
    const TEXT_X: i32 = 388;
    const TEXT_DY: i32 = 18;
    let (plate, w, h) = c.assets.image("cg/sys/select/select001.png")?;
    let (w, h) = (w as i64, h as i64);
    let parts: Vec<_> = (0..n).map(|i| (plate.clone(), X, Y + h * i, 1000)).collect();
    let (image, wx, wy) = c.assets.composite(&parts)?;
    let window = TextWindow {
        wait_icon_at: None,
        image,
        x: wx as i32,
        y: wy as i32,
        text_x: (X + INSET) as i32,
        text_y: (Y + (h - FONT) / 2) as i32,
        columns: ((w - 2 * INSET) / FONT) as u32,
        rows: n as u32,
        font_size: FONT as u32,
        line_gap: (h - FONT) as u32,
        vertical: false,
        name_plate: None,
    };
    let (image, _, _) = c.assets.strip(&["cg/sys/select/select001.png", "cg/sys/select/select002.png"])?;
    // ChoiceA02/ChoiceB02: the hole's top left 47, 41 above and left of the pointer.
    let hole = c.assets.image("cg/sys/select/dankon.png").map(|i| i.0);
    let sound = c.assets.sound("sound/se/SE_戦闘_銃器_発砲_マカロニ1");
    let shot = hole.zip(sound).map(|(hole, sound)| ChoiceShot { hole, hole_dx: -47, hole_dy: -41, sound });
    let plates = ChoicePlates { image, x: X as i32, y: Y as i32, step: h as i32, text_x: TEXT_X, text_dy: TEXT_DY, font_size: FONT as u32, colour: "#FFFFFF".into(), fade_ms: 300, shot };
    Some(ChoiceStyle { window, on_color: 0xFFD070, off_color: 0xFFFFFF, plates: Some(plates) })
}
