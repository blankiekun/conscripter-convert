//! 続・殺戮のジャンゴ's system screens, built from its own pictures
//! (`cg/sys/`) with the places and times of its scripts (`boot.nss`,
//! `sys_*.nss`). Not compared with the original yet (it asks for its disc
//! under wine).

use conv_core::ir::{BarAction, ButtonBar, CommandItem, CommandMenu, Placed, Script, TitleAction, TitleButton, TitleScreen};

use super::super::{Assets, Story};

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script, _story: &Story) {
    script.title_screen = title(a);
    // The game has no button bar; the command menu runs a bar's actions.
    script.button_bar = Some(ButtonBar { buttons: Vec::new(), top: None, base: None, slide: None, pressed: true, quick_save_se: None, load_fade_ms: 0 });
    script.command_menu = command_menu(a);
}

/// The right-click menu (`sys_menu.nss`): twelve rows (236×24) at x 282
/// from y 113 on the pull-down base (282,93); a row's normal picture is
/// `00-` (drawn under the half see-through base, so it shows faintly),
/// under the pointer `001` (over it), pressed `002`. No fades in the
/// script. The keys are the ones printed on the rows (A, R, B, S, L, F,
/// C, T; SPACE, CTRL and ESC are the engine's own).
fn command_menu(a: &mut dyn Assets) -> Option<CommandMenu> {
    const DIR: &str = "cg/sys/menu";
    let mut items = Vec::new();
    for (n, (name, action, key)) in [
        ("キャンセル", BarAction::Close, None),
        ("メッセージ", BarAction::HideWindow, None),
        ("自動メッセージ", BarAction::Auto, Some('A')),
        ("スキップ", BarAction::Skip, None),
        ("選択肢", BarAction::BackToChoice, Some('R')),
        ("ログ", BarAction::Log, Some('B')),
        ("セーブ", BarAction::Save, Some('S')),
        ("ロード", BarAction::Load, Some('L')),
        ("フルスクリーン", BarAction::Fullscreen, Some('F')),
        ("設定画面", BarAction::Settings, Some('C')),
        ("タイトルに戻る", BarAction::Title, Some('T')),
        ("ゲームを終了", BarAction::Quit, None),
    ]
    .into_iter()
    .enumerate()
    {
        let state = |s: &str| format!("{DIR}/{name}{s}.png");
        let (s0, s1, s2) = (state("00-"), state("001"), state("002"));
        let (image, w, h) = a.strip(&[&s0, &s1, &s0, &s2])?;
        items.push(CommandItem { button: Placed { image, x: 282, y: 113 + 24 * n as i32, w: w as i32, h: h as i32 }, action, key, label: None });
    }
    let (image, w, h) = a.image(&format!("{DIR}/プルダウン台座.png"))?;
    Some(CommandMenu { backdrop: Placed { image, x: 282, y: 93, w: w as i32, h: h as i32 }, items, fade_ms: 0, step_ms: 0, key: None, pressed: true })
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
        glow: None,
    })
}
