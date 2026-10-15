//! 続・殺戮のジャンゴ's system screens, built from its own pictures
//! (`cg/sys/`) with the places and times of its scripts (`boot.nss`,
//! `sys_*.nss`). Not compared with the original yet (it asks for its disc
//! under wine).

use conv_core::ir::{BarAction, ButtonBar, CommandItem, CommandMenu, GridText, Placed, Rect, SaveGrid, Script, TitleAction, TitleButton, TitleScreen};

use super::super::{Assets, Story};

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script, _story: &Story) {
    script.title_screen = title(a);
    // The game has no button bar; the command menu runs a bar's actions.
    script.button_bar = Some(ButtonBar { buttons: Vec::new(), top: None, base: None, slide: None, pressed: true, quick_save_se: None, load_fade_ms: 0 });
    script.command_menu = command_menu(a);
    script.save_grid = save_grid(a);
}

/// The save and load screens (`sys_save.nss`, `sys_load.nss`): three
/// pages of 20 slots over a picture each (`save-load-back01-20` …, the
/// slot numbers printed on it, with the empty slots' icon and the black
/// box under the preview drawn in), five across from (50,80), 150 apart, the
/// next row 96 below; thumbnails 100×75. The save or load logo at
/// (50,24). Under the pointer: the thumbnail larger at (461,481),
/// 136×101, the date at (50,486) and the last line at (50,518) in white,
/// the play time at (606,30) in #EE9933 (18 px; the number above the date
/// is left out). NEXT (650,490) goes round the pages; EXIT (632,542). The
/// original has no previous page, delete, frame or new-save mark: those
/// are see-through here, the buttons off the screen.
fn save_grid(a: &mut dyn Assets) -> Option<SaveGrid> {
    let mut pages = [Vec::new(), Vec::new()];
    for range in ["01-20", "21-40", "41-60"] {
        for (mode, list) in ["save", "load"].iter().zip(pages.iter_mut()) {
            let back = a.image(&format!("cg/sys/{mode}/save-load-back{range}.png"))?.0;
            let logo = a.image(&format!("cg/sys/{mode}/{mode}-logo.png"))?.0;
            let mut parts = vec![(back, 0, 0, 1000), (logo, 50, 24, 1000), (a.color(0, 136, 101), 461, 481, 1000)];
            // An empty slot's icon; a save's thumbnail covers it.
            let icon = a.image(&format!("cg/sys/{mode}/save-load-icon.png"))?.0;
            parts.extend((0..20).map(|i| (icon.clone(), 50 + 150 * (i % 5), 80 + 96 * (i / 5), 1000)));
            list.push(a.composite(&parts)?.0);
        }
    }
    let [save_pages, load_pages] = pages;
    let button = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let state = |n: u32| format!("cg/sys/save/{name}-{n:02}.png");
        let (s1, s2, s3) = (state(1), state(2), state(3));
        let (image, w, h) = a.strip(&[&s1, &s2, &s1, &s3])?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let none = |a: &mut dyn Assets, x: i32, y: i32| Placed { image: a.transparent(), x, y, w: 1, h: 1 };
    let text = |x: i32, y: i32, lines: u32, chars: u32, colour: &str| GridText { x, y, font_size: 18, lines, chars, line_gap: 0, char_gap: 0, colour: colour.into(), vertical: false };
    Some(SaveGrid {
        save_pages,
        load_pages,
        frames: None,
        first: (50, 80),
        row_step: 96,
        column_step: 150,
        rows: 4,
        columns: 5,
        thumb_w: 100,
        thumb_h: 75,
        hover: none(a, 0, 0),
        hover_offset: (0, 0),
        new_mark: none(a, 0, 0),
        new_offset: (0, 0),
        previous: none(a, -100, -100),
        next: button(a, "next", 650, 490)?,
        delete: none(a, -100, -100),
        delete_on: none(a, -100, -100),
        close: button(a, "exit", 632, 542)?,
        preview: Rect { x: 461, y: 481, w: 136, h: 101 },
        date: text(50, 486, 1, 20, "#ffffff"),
        play_time: text(606, 30, 1, 12, "#ee9933"),
        chapter: text(50, 464, 0, 0, "#ffffff"),
        line: text(50, 518, 2, 19, "#ffffff"),
        saved: None,
        fade_ms: 300,
        across: true,
    })
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
        openings: Vec::new(),
    })
}
