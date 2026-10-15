//! 装甲悪鬼村正's system screens, built from its own pictures (`cg/sys/`)
//! with the places, sizes and times of its scripts (`sys_*.nss`), as
//! filmed in the original (`docs/games/muramasa.md`, "System screens").
//!
//! Buttons are made of one picture per state: `001` normal, `002` under
//! the pointer, `003` pressed, `000` disabled (where there is one); they
//! are joined into strips of normal, hover, disabled, pressed.

use conv_core::ir::{
    BarAction, BarButton, BarSlide, ButtonBar, CommandItem, CommandMenu, ConfirmDialog, GridText, LineLog, Placed, Rect, SaveGrid, Script, Switch, TitleAction, TitleButton, TitleScreen,
};

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
    script.save_grid = save_grid(a);
    script.line_log = line_log(a);
}

/// The backlog (`sys_backlog.nss`, System.ini `[バックログ]`): sixteen
/// rows of 26 characters from (140,49), 30 apart, MS 明朝 20 in black,
/// over バックログ背景; a voiced line's speaker icon (`icon_voice`) at
/// x 102 on its first row. 逆 (898,168) and 進 (898,303) scroll a row,
/// 退出 (927,438) closes; the scrollbar's knob runs from y 50 to 461 at
/// x 860 (`CreateScrollbar`). It fades in and out over 300 ms.
fn line_log(a: &mut dyn Assets) -> Option<LineLog> {
    const DIR: &str = "cg/sys/backlog";
    let strip = |a: &mut dyn Assets, names: &[&str], x: i32, y: i32| {
        let parts: Vec<String> = names.iter().map(|n| format!("{DIR}/{n}.png")).collect();
        let (image, w, h) = a.strip(&parts.iter().map(String::as_str).collect::<Vec<_>>())?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let (background, w, h) = a.image(&format!("{DIR}/バックログ背景.png"))?;
    Some(LineLog {
        background: Placed { image: background, x: 0, y: 0, w: w as i32, h: h as i32 },
        up: button(a, DIR, "逆", 898, 168, false)?,
        down: button(a, DIR, "進", 898, 303, false)?,
        close: button(a, DIR, "退出", 927, 438, false)?,
        knob: strip(a, &["001-スクロール", "002-スクロール"], 860, 461)?,
        knob_y: (50, 461),
        voice_icon: strip(a, &["001-スピーカ", "002-スピーカ", "003-スピーカ"], 102, 0)?,
        text_x: 140,
        text_y: 49,
        row_step: 30,
        rows: 16,
        columns: 26,
        font_size: 20,
        colour: "#000000".into(),
        fade_ms: 300,
    })
}

/// The save and load screens (`sys_save.nss`, `sys_load.nss`): five pages
/// of twenty slots, each page's picture with its numbers (数字01-20背景,
/// …), the thumbnails (160×90) in four columns from the right (600,34),
/// 100 down and 190 left, under their frames. The slot under the pointer
/// gets the hover frame (at -3,-3) and shows its details: the date
/// (821,516) and play time (68,528) across, the chapter and the last line
/// down in columns from the right edges x 955 and 924, a larger
/// thumbnail in the box at (810,416). 前頁 (790,34), 次頁 (790,130), 削除
/// (790,226, 削除ボタン-003 over it in delete mode), 退出 (810,322). After
/// a save, 記録しました over black at 800/1000 until a click (`SaveDialog`
/// 987-1015). It fades in and out over 300 ms (`SaveFade` 620).
fn save_grid(a: &mut dyn Assets) -> Option<SaveGrid> {
    const DIR: &str = "cg/sys/save";
    let mut pages = [Vec::new(), Vec::new()];
    for range in ["01-20", "21-40", "41-60", "61-80", "80-100"] {
        for (mode, list) in ["save", "load"].iter().zip(pages.iter_mut()) {
            list.push(a.image(&format!("{DIR}/数字{range}背景{mode}.png"))?.0);
        }
    }
    let [save_pages, load_pages] = pages;
    let picture = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    // Normal, hover, disabled (normal), pressed; 削除 has no pressed
    // picture (its -003 is the delete mode's).
    let button = |a: &mut dyn Assets, name: &str, x: i32, y: i32, pressed: u32| {
        let state = |n: u32| format!("{DIR}/{name}ボタン-{n:03}.png");
        let (s1, s2, s3) = (state(1), state(2), state(pressed));
        let (image, w, h) = a.strip(&[&s1, &s2, &s1, &s3])?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let text = |x: i32, y: i32, font_size: u32, lines: u32, chars: u32, char_gap: i32, colour: &str, vertical: bool| GridText {
        x,
        y,
        font_size,
        lines,
        chars,
        line_gap: 0,
        char_gap,
        colour: colour.into(),
        vertical,
    };
    let saved = a.image("cg/sys/dialog/記録しました.png").map(|(image, w, h)| Placed { image, x: (SCREEN_W - w as i32) / 2, y: 88, w: w as i32, h: h as i32 });
    Some(SaveGrid {
        save_pages,
        load_pages,
        frames: picture(a, "サムネールワク", 0, 0),
        first: (600, 34),
        row_step: 100,
        column_step: -190,
        rows: 5,
        columns: 4,
        thumb_w: 160,
        thumb_h: 90,
        hover: picture(a, "選択中サムネールワク", 0, 0)?,
        hover_offset: (-3, -3),
        new_mark: picture(a, "新マーク", 0, 0)?,
        new_offset: (164, 64),
        previous: button(a, "前頁", 790, 34, 3)?,
        next: button(a, "次頁", 790, 130, 3)?,
        delete: button(a, "削除", 790, 226, 2)?,
        delete_on: picture(a, "削除ボタン-003", 790, 226)?,
        close: button(a, "退出", 810, 322, 3)?,
        preview: Rect { x: 810, y: 416, w: 192, h: 108 },
        date: text(821, 516, 18, 1, 20, 0, "#000000", false),
        play_time: text(68, 528, 18, 1, 12, 0, "#ffffff", false),
        chapter: text(929, 41, 20, 1, 24, 0, "#000000", true),
        line: text(898, 45, 18, 3, 20, -1, "#000000", true),
        saved,
        fade_ms: 300,
    })
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
