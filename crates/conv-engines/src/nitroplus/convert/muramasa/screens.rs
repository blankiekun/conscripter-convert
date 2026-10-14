//! 装甲悪鬼村正's system screens, built from its own pictures (`cg/sys/`)
//! with the places, sizes and times of its scripts (`sys_*.nss`), as
//! filmed in the original (`docs/games/muramasa.md`, "System screens").
//!
//! Buttons are made of one picture per state: `001` normal, `002` under
//! the pointer, `003` pressed, `000` disabled (where there is one); they
//! are joined into strips of normal, hover, disabled, pressed.

use conv_core::ir::{ConfirmDialog, Placed, Script, Switch};

use super::super::Assets;

const SCREEN_W: i32 = 1024;
const SCREEN_H: i32 = 576;

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script) {
    script.confirm = confirm(a);
}

/// A button's strip from its state pictures `<dir>/00N-<name>.png`: normal,
/// hover, disabled (normal when there is no `000`), pressed.
fn button(a: &mut dyn Assets, dir: &str, name: &str, x: i32, y: i32, disabled: bool) -> Option<Placed> {
    let state = |n: u32| format!("{dir}/{n:03}-{name}.png");
    let (normal, hover, pressed, off) = (state(1), state(2), state(3), state(0));
    let parts = [normal.as_str(), hover.as_str(), if disabled { off.as_str() } else { normal.as_str() }, pressed.as_str()];
    let (image, w, h) = a.strip(&parts)?;
    Some(Placed { image, x, y, w: w as i32, h: h as i32 })
}

/// A picture centred on the screen (NSS `Center`, `Middle`).
fn centred(a: &mut dyn Assets, path: &str) -> Option<Placed> {
    let (image, w, h) = a.image(path)?;
    let (w, h) = (w as i32, h as i32);
    Some(Placed { image, x: (SCREEN_W - w) / 2, y: (SCREEN_H - h) / 2, w, h })
}

/// The yes/no questions (`sys_close.nss` 70-115, the same in `sys_reset`
/// and the save screen's `SaveDialog`): the question's picture (100×400,
/// text and frame) in the middle of the screen over black at 800/1000,
/// 承認 (yes) at (515,397) and 否認 (no) at (471,397), fading in over
/// 150 ms (the original also unrolls them sideways from the middle).
fn confirm(a: &mut dyn Assets) -> Option<ConfirmDialog> {
    const DIR: &str = "cg/sys/dialog";
    let question = |a: &mut dyn Assets, name: &str| centred(a, &format!("{DIR}/{name}.png"));
    let mut pictures = Vec::new();
    for (kind, name) in [
        (Switch::ConfirmQuit, "ゲームを終了しますか"),
        (Switch::ConfirmTitle, "タイトルへ戻りますか"),
        (Switch::ConfirmSave, "記録を上書きして良いか"),
        (Switch::ConfirmLoad, "この記録を呼び出しますか"),
        (Switch::ConfirmDelete, "記録を削除してよろしいか"),
    ] {
        pictures.push((kind, question(a, name)?));
    }
    let black = a.color(0, SCREEN_W as u32, SCREEN_H as u32);
    Some(ConfirmDialog {
        base: pictures[0].1.clone(),
        yes: button(a, DIR, "承認", 515, 397, false)?,
        no: button(a, DIR, "否認", 471, 397, false)?,
        yes_label: String::new(),
        no_label: String::new(),
        text_y: 0,
        font_size: 20,
        overwrite: String::new(),
        load: String::new(),
        delete: String::new(),
        title: String::new(),
        quit: String::new(),
        pictures,
        backdrop: Some((Placed { image: black, x: 0, y: 0, w: SCREEN_W, h: SCREEN_H }, 204)),
        pressed: true,
        fade_ms: 150,
    })
}
