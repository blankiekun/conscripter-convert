//! 続・殺戮のジャンゴ's system screens, built from its own pictures
//! (`cg/sys/`) with the places and times of its scripts (`boot.nss`,
//! `sys_*.nss`). Not compared with the original yet (it asks for its disc
//! under wine).

use conv_core::ir::{Script, TitleAction, TitleButton, TitleScreen};

use super::super::{Assets, Story};

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script, _story: &Story) {
    script.title_screen = title(a);
}

/// The title (`boot.nss`, `TitleSet` and `TitleSelect`): the background
/// with the logo at (84,111) and the tumbleweed at (300,420); six buttons
/// where `TitleSet` puts them, white at rest (`-01`), red under the
/// pointer (`-02`, with a revolver being cocked); a shot when one is
/// clicked. Start fades the music and the screen to black over a second.
/// 特別 and LINER NOTES answer only once the story is finished
/// (`#ClearG`): the extras aren't converted yet, and LINER NOTES opened a
/// page of Nitroplus's site; both are shown and refuse a click. Left out:
/// the buttons flying in turning (`Rotate`, `Move`), their shaking, the
/// tumbleweed rolling, the red slash over the screen on a click (`-03`),
/// and the tumbleweed's hidden chapter select.
fn title(a: &mut dyn Assets) -> Option<TitleScreen> {
    const DIR: &str = "cg/sys/title";
    let mut parts = Vec::new();
    for (name, x, y) in [("title-back.jpg", 0, 0), ("title-logo.png", 84, 111), ("tumbleweed.png", 300, 420)] {
        parts.push((a.image(&format!("{DIR}/{name}"))?.0, x, y, 1000));
    }
    let (background, _, _) = a.composite(&parts)?;
    let shot = a.sound("sound/se/SE_戦闘_銃器_発砲_マカロニ1");
    let mut buttons = Vec::new();
    for (name, x, y, action) in [
        ("start", 210, 291, TitleAction::Start),
        ("load", 434, 356, TitleAction::Load),
        ("config", 142, 393, TitleAction::Settings),
        ("extra", 22, 415, TitleAction::Unavailable),
        ("exit", 691, 449, TitleAction::Exit),
        ("liner", 457, 479, TitleAction::Unavailable),
    ] {
        let state = |n: u32| format!("{DIR}/{name}-{n:02}.png");
        let (s1, s2) = (state(1), state(2));
        let cells: Vec<&str> = if action == TitleAction::Unavailable { vec![&s1] } else { vec![&s1, &s2, &s2] };
        let (image, _, _) = a.strip(&cells)?;
        let click_se = (action != TitleAction::Unavailable).then(|| shot.clone()).flatten();
        buttons.push(TitleButton { image, x, y, action, click_se });
    }
    Some(TitleScreen {
        background,
        fade_ms: 1000,
        buttons,
        pressed: true,
        hover_se: a.sound("sound/se/SE_戦闘_銃器_拳銃_構える3"),
        bgm: a.sound("sound/bgm/dj23_harmonica"),
        start_bgm_fade_ms: 1000,
        start_wait_ms: 0,
        exit_without_asking: false,
    })
}
