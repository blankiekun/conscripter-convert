//! 装甲悪鬼村正's extras, 特別付加目録 (`extra_*.nss`), built from its own
//! pictures (`cg/sys/extra/`) with the places and times of its scripts, as
//! filmed in the original (`docs/games/muramasa.md`, "Extras").

use conv_core::ir::{ExtrasMode, PictureExtras, PictureExtrasItem, Placed};

use super::super::{Assets, Story};

/// Set at the end of any route (the end roll) and when a game starts after
/// one: the gallery, the recollection and the sound room open.
pub const CLEARED: &str = "#ClearG";

/// The menu (`extra_menu.nss`, `ExtraSet`): MENU背景画像 with five buttons;
/// 画像, 回想 and 音楽 drawn at 200/1000 and not clickable until
/// [`CLEARED`]. 鎧_装着01 under the pointer, 装着06 on opening a mode,
/// 剣戟弾く01 on leaving (`決定サウンズ`, made by the title); the screen
/// cross-fades over 500 ms (`ExtraFade`).
pub fn extras(a: &mut dyn Assets, story: &Story) -> Option<PictureExtras> {
    const DIR: &str = "cg/sys/extra/main";
    let se = |a: &mut dyn Assets, name: &str| a.sound(&format!("sound/se/{name}"));
    let cleared = story.flags.get(CLEARED).copied();
    let mut items = Vec::new();
    for (name, x, y, mode, locked) in [
        ("CG", 534, 114, ExtrasMode::Gallery, true),
        ("回想", 434, 114, ExtrasMode::Recollection, true),
        ("SOUNDMOVIE", 234, 114, ExtrasMode::Sound, true),
        ("ツルギ回転", 134, 114, ExtrasMode::Turntable, false),
        ("メニュー退出", 0, 438, ExtrasMode::Back, false),
    ] {
        let state = |n: u32| format!("{DIR}/{name}-{n:03}.png");
        let (image, w, h) = a.strip(&[&state(1), &state(2), &state(3)])?;
        let click_se = if mode == ExtrasMode::Back { None } else { se(a, "se特殊_鎧_装着06") };
        items.push(PictureExtrasItem {
            button: Placed { image, x, y, w: w as i32, h: h as i32 },
            mode,
            unlocked_by: if locked { cleared } else { None },
            locked_alpha: 51,
            click_se,
        });
    }
    Some(PictureExtras {
        background: a.image(&format!("{DIR}/MENU背景画像.png"))?.0,
        items,
        hover_se: se(a, "se特殊_鎧_装着01"),
        back_se: se(a, "se戦闘_攻撃_剣戟弾く01"),
        fade_ms: 500,
    })
}
