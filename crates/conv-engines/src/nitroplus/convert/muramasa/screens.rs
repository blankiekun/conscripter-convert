//! 装甲悪鬼村正's system screens, built from its own pictures (`cg/sys/`)
//! with the places, sizes and times of its scripts (`sys_*.nss`), as
//! filmed in the original (`docs/games/muramasa.md`, "System screens").
//!
//! Buttons are made of one picture per state: `001` normal, `002` under
//! the pointer, `003` pressed, `000` disabled (where there is one); they
//! are joined into strips of normal, hover, disabled, pressed.

use conv_core::ir::{BarAction, BarButton, BarSlide, ButtonBar, CommandItem, CommandMenu, ConfirmDialog, Placed, Script, Switch, TitleAction, TitleButton, TitleScreen};

use super::super::Assets;

/// The favour screen (`sys_hmode.nss`), compiled as a subroutine for the
/// menu's 好感度記録.
pub const FAVOUR: &str = "conv_favour";

const SCREEN_W: i32 = 1024;
const SCREEN_H: i32 = 576;

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script) {
    script.confirm = confirm(a);
    script.title_screen = title(a);
    script.button_bar = quick_bar(a);
    script.command_menu = command_menu(a);
}

/// The right-click menu (`sys_menu.nss`): sixteen rows (220×20) at x 636
/// from y 89, over a black backdrop (frame included) that fades in over
/// 100 ms before the rows appear 10 ms apart (24-318); closing reverses
/// it (539-577). The letters are the game's shortcuts (`0_boot.nss`
/// 27-55, `SetShortcut`), which work at any click wait; M opens and closes
/// the menu. 好感度記録 runs the favour screen ([`FAVOUR`]).
fn command_menu(a: &mut dyn Assets) -> Option<CommandMenu> {
    const DIR: &str = "cg/sys/menu";
    let mut items = Vec::new();
    for (n, (name, action, key)) in [
        ("キャンセル", BarAction::Close, None),
        ("好感度記録", BarAction::Call, Some('H')),
        ("メッセージ消す", BarAction::HideWindow, None),
        ("自動メッセージ", BarAction::Auto, Some('A')),
        ("スキップ", BarAction::Skip, None),
        ("超速スキップ", BarAction::FastSkip, Some('N')),
        ("選択肢に戻る", BarAction::BackToChoice, Some('R')),
        ("ログ表示", BarAction::Log, Some('B')),
        ("セーブ", BarAction::Save, Some('S')),
        ("ロード", BarAction::Load, Some('L')),
        ("栞を差す", BarAction::QuickSave, Some('Q')),
        ("栞を開く", BarAction::QuickLoad, Some('P')),
        ("フルスクリーン", BarAction::Fullscreen, Some('F')),
        ("設定画面", BarAction::Settings, Some('C')),
        ("タイトルに戻る", BarAction::Title, Some('T')),
        ("ゲーム終了", BarAction::Quit, None),
    ]
    .into_iter()
    .enumerate()
    {
        // No pressed picture: filmed, the menu keeps the hover one while
        // the mouse button is held.
        let state = |n: u32| format!("{DIR}/{n:03}-{name}.png");
        let (s1, s2, s0) = (state(1), state(2), state(0));
        let (image, w, h) = a.strip(&[&s1, &s2, &s0])?;
        let button = Placed { image, x: 636, y: 89 + 20 * n as i32, w: w as i32, h: h as i32 };
        let label = (action == BarAction::Call).then(|| FAVOUR.to_string());
        items.push(CommandItem { button, action, key, label });
    }
    let (image, w, h) = a.image(&format!("{DIR}/メニュー黒バック.png"))?;
    Some(CommandMenu { backdrop: Placed { image, x: 0, y: 0, w: w as i32, h: h as i32 }, items, fade_ms: 100, step_ms: 10, key: Some('M'), pressed: false })
}

/// The quick bar (`function_quick.nss`): a base (992,26) and four buttons
/// at x 998, off the right edge until the pointer passes x 968, then
/// sliding in over 200 ms (`Dxl2`, 272-286). Quick save plays しおり; quick
/// load fades to black over 300 ms (208-210). The original runs it as a
/// process, so it also slides while text is being drawn; here it moves
/// at click waits only.
fn quick_bar(a: &mut dyn Assets) -> Option<ButtonBar> {
    const DIR: &str = "cg/sys/quick";
    let mut buttons = Vec::new();
    for (name, y, action) in [
        ("栞を差す", 47, BarAction::QuickSave),
        ("栞を開く", 180, BarAction::QuickLoad),
        ("スキップする", 290, BarAction::Skip),
        ("メッセージを消す", 390, BarAction::HideWindow),
    ] {
        // Normal, hover, disabled (normal: the bar greys nothing), pressed.
        let state = |n: u32| format!("{DIR}/{name}{n:03}.png");
        let (s1, s2, s3) = (state(1), state(2), state(3));
        let (image, w, h) = a.strip(&[&s1, &s2, &s1, &s3])?;
        buttons.push(BarButton { image: Placed { image, x: 998, y, w: w as i32, h: h as i32 }, active: None, action });
    }
    let (image, w, h) = a.image(&format!("{DIR}/クイックメニュー台座.png"))?;
    Some(ButtonBar {
        buttons,
        top: None,
        base: Some(Placed { image, x: 992, y: 26, w: w as i32, h: h as i32 }),
        slide: Some(BarSlide { hidden_dx: 32, edge_x: 968, ms: 200 }),
        pressed: true,
        quick_save_se: a.sound("sound/se/SE_sys_しおり"),
        load_fade_ms: 300,
    })
}

/// The title as it stands after its opening (`0_boot.nss`, `TitleSet`
/// 417-673, `TitleFade` for a return visit 828-880): the background, the
/// logo at (346,40) and the copyright at (50,540) in one picture; six
/// vertical buttons (40×280) at y 150 with a hover sound; the title music.
/// Start plays its sound, fades the music out over 3 s and the screen to
/// black over 1 s, and waits 2 s (162-168, 1114-1125). 終了する quits
/// with no question (`sys_close.nss` 45-51). Left out: the fire that
/// flickers over the background (0 to 300/1000 and back, 600 ms each way,
/// 1100 ms holds), the opening, and the first start's intro.
fn title(a: &mut dyn Assets) -> Option<TitleScreen> {
    const DIR: &str = "cg/sys/title";
    let mut parts = Vec::new();
    for (name, x, y) in [("偽富士背景画像", 0, 0), ("ロゴ画像", 346, 40), ("コピーライト", 50, 540)] {
        parts.push((a.image(&format!("{DIR}/{name}.png"))?.0, x, y, 1000));
    }
    let (background, _, _) = a.composite(&parts)?;
    let se = |a: &mut dyn Assets, name: &str| a.sound(&format!("sound/se/{name}"));
    let mut buttons = Vec::new();
    for (name, x, action, click) in [
        ("しおりから", 822, TitleAction::Continue, Some("se特殊_鎧_装着03")),
        ("最初から", 762, TitleAction::Start, Some("se特殊_鎧_装着03")),
        ("電光記録", 702, TitleAction::Load, Some("se戦闘_動作_刀構え01")),
        ("設定画面", 282, TitleAction::Settings, Some("se戦闘_動作_刀構え01")),
        // The extras come with step 8.
        ("特別付加", 222, TitleAction::Unavailable, None),
        ("終了する", 162, TitleAction::Exit, None),
    ] {
        let state = |n: u32| format!("{DIR}/{name}-{n:03}.png");
        let (s1, s2, s3) = (state(1), state(2), state(3));
        let cells: Vec<&str> = if action == TitleAction::Unavailable { vec![&s1] } else { vec![&s1, &s2, &s3] };
        let (image, _, _) = a.strip(&cells)?;
        let click_se = click.and_then(|c| se(a, c));
        buttons.push(TitleButton { image, x, y: 150, action, click_se });
    }
    Some(TitleScreen {
        background,
        fade_ms: 1000,
        buttons,
        pressed: true,
        hover_se: se(a, "se戦闘_動作_刀構え02"),
        bgm: a.sound("sound/bgm/mbgm04"),
        start_bgm_fade_ms: 3000,
        start_wait_ms: 2000,
        exit_without_asking: true,
    })
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
        (Switch::ConfirmBackToChoice, "選択肢まで戻りますか"),
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
