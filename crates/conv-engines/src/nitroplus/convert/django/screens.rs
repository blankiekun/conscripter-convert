//! 続・殺戮のジャンゴ's system screens, built from its own pictures
//! (`cg/sys/`) with the places and times of its scripts (`boot.nss`,
//! `sys_*.nss`). Not compared with the original yet (it asks for its disc
//! under wine).

use conv_core::ir::{
    BarAction, ButtonBar, CommandItem, CommandMenu, ConfirmDialog, GridText, LineLog, Placed, Rect, SaveGrid, Script, Switch, TitleAction, TitleButton, TitleScreen, WaitIcon,
};

use conv_core::ir::{PairOption, PicturePair, PictureSettings, PictureSlider, SliderSetting, SliderSteps};

use super::super::{Assets, Story};

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script, story: &Story) {
    script.title_screen = title(a);
    // The game has no button bar; the command menu runs a bar's actions.
    script.button_bar = Some(ButtonBar { buttons: Vec::new(), top: None, base: None, slide: None, pressed: true, quick_save_se: None, load_fade_ms: 0 });
    script.command_menu = command_menu(a);
    script.save_grid = save_grid(a);
    script.confirm = confirm(a);
    script.line_log = line_log(a);
    script.wait_icon = wait_icon(a);
    script.picture_settings = settings(a);
    script.picture_extras = super::extras::extras(a, story);
    if let Some(t) = script.title_screen.as_mut() {
        let cleared = story.flags.get(super::extras::CLEARED).copied();
        for b in &mut t.buttons {
            if b.action == TitleAction::Extras || b.action == TitleAction::Unavailable {
                b.shown_by = cleared;
            }
        }
        if script.picture_extras.is_none() {
            t.buttons.iter_mut().filter(|b| b.action == TitleAction::Extras).for_each(|b| b.action = TitleAction::Unavailable);
        }
    }
}

/// The settings screen (`sys_config.nss`) over `コンフィグ地`: rows of ten
/// bullets (`bar-弾`, 12 px apart from x 306) for the text speed (y 212),
/// auto mode's wait (308) and the BGM, SE and voice volumes (422, 470,
/// 518), every bullet up to the chosen one red (`setgrade`); ON (x 386)
/// and OFF (440) pairs; GAME-SPEED's four presets at y 188; EXIT
/// (650,518). Pictures: `-001` at rest, `-002` under the pointer, `-003`
/// chosen. Values as `config_default` and `setgrade` set them: text speed
/// level k is `k × 100` (level 1 none), its slowest 50 ms a character
/// (ours: the engine's unit isn't known); auto mode waits `20 × k` ms a
/// character (default 5); volumes `k × 100` of 1000 (defaults from
/// system.ini: 7, 9, 10); GAME-SPEED is the engine's play speed 3
/// (NORMAL, default), 5 (QUICK-A), 12 (QUICK-B), 1 (LOWSPEC), each the
/// percent of 装甲悪鬼村正's exe table (assumed the same engine). Left
/// out and drawn as they stand by default: the two questions' rows (ON
/// chosen, OFF at rest; one switch here turns all questions off), the
/// display options, the character voices window, the voice test, DEFAULT (it would ask with no
/// picture of its own), the links.
fn settings(a: &mut dyn Assets) -> Option<PictureSettings> {
    const DIR: &str = "cg/sys/config";
    let button = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let state = |n: u32| format!("{DIR}/{name}-{n:03}.png");
        let (s1, s2, s3) = (state(1), state(2), state(3));
        let (image, w, h) = a.strip(&[&s1, &s2, &s3])?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let picture = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let (background, w, h) = a.image(&format!("{DIR}/コンフィグ地.png"))?;
    let none = Placed { image: a.transparent(), x: 0, y: 0, w: 1, h: 1 };
    let stepped = |setting: SliderSetting, buttons: Vec<(Placed, u32)>, default: u32, filled: bool| PictureSlider {
        setting,
        knob: none.clone(),
        knob_y: (0, 0),
        track: Rect { x: 0, y: 0, w: 0, h: 0 },
        default,
        steps: Some(SliderSteps { buttons, filled }),
    };
    let bullets = |a: &mut dyn Assets, y: i32, position: &dyn Fn(u32) -> u32| -> Option<Vec<(Placed, u32)>> {
        (1..=10).map(|k| Some((button(a, "bar-弾", 306 + 12 * (k as i32 - 1), y)?, position(k)))).collect()
    };
    let text_speed = |k: u32| if k == 1 { 0 } else { k * 100 };
    let per_char = |k: u32| k * 100;
    let volume = |k: u32| 1000 - k * 100;
    // The level a position of SliderSetting::GameSpeed gives: 12 - p × 12 / 1000.
    let speed = |level: u32| ((12 - level) * 1000).div_ceil(12);
    let mut presets = Vec::new();
    for (name, x, level) in [("normal", 162, 3), ("quick-a", 238, 5), ("quick-b", 314, 12), ("lowspec", 392, 1)] {
        presets.push((button(a, name, x, 188)?, speed(level)));
    }
    let sliders = vec![
        stepped(SliderSetting::GameSpeed { levels: [50, 75, 100, 150, 200, 300, 400, 550, 800, 1200, 1600, 3200] }, presets, speed(3), false),
        // The engine starts at 500 (level 5, under wine; config_default's
        // 1000 is DEFAULT's).
        stepped(SliderSetting::TextSpeed { ms: 50 }, bullets(a, 212, &text_speed)?, text_speed(5), true),
        stepped(SliderSetting::AutoWait { ms: 200 }, bullets(a, 308, &per_char)?, per_char(5), true),
        stepped(SliderSetting::BgmVolume, bullets(a, 422, &volume)?, volume(7), true),
        stepped(SliderSetting::SeVolume, bullets(a, 470, &volume)?, volume(9), true),
        stepped(SliderSetting::VoiceVolume, bullets(a, 518, &volume)?, volume(10), true),
    ];
    let mut pairs = Vec::new();
    for (option, y, default_on) in [
        (PairOption::MovieStop, 130, false),
        (PairOption::StopAtChoice, 236, true),
        (PairOption::SkipUnread, 260, false),
        (PairOption::Bgm, 398, true),
        (PairOption::Se, 446, true),
        (PairOption::Voice, 494, true),
    ] {
        pairs.push(PicturePair { option, on: button(a, "on", 386, y)?, off: button(a, "off", 440, y)?, default_on });
    }
    pairs.push(PicturePair { option: PairOption::VoicePlaysOn, on: button(a, "全て再生", 294, 284)?, off: button(a, "クリックで停止", 368, 284)?, default_on: true });
    pairs.push(PicturePair { option: PairOption::AutoWaitsForVoice, on: button(a, "終了まで待つ", 272, 332)?, off: button(a, "再生を中断する", 368, 332)?, default_on: true });
    let fixed = vec![
        picture(a, "001-初期化_x450-y58", 450, 58)?,
        picture(a, "on-003", 386, 82)?,
        picture(a, "on-003", 386, 106)?,
        picture(a, "off-001", 440, 82)?,
        picture(a, "off-001", 440, 106)?,
        picture(a, "各キャラ-001", 293, 362)?,
        picture(a, "vioce-test-01", 440, 542)?,
        picture(a, "ディスプレイ依存-003", 516, 82)?,
        picture(a, "ユーザー-001", 516, 476)?,
    ];
    Some(PictureSettings {
        background: Placed { image: background, x: 0, y: 0, w: w as i32, h: h as i32 },
        fixed,
        close: button(a, "exit", 650, 518)?,
        reset: None,
        voice_test: None,
        test_voice: None,
        sliders,
        pairs,
        texts: Vec::new(),
        fade_ms: 300,
        pair_fade_ms: 0,
        voice_window: None,
    })
}

/// The click-wait icon (system.ini `icon_page`: `Bullet-%03d.png#20`; the
/// archive has 18 pictures of 18×18), looped after the text. The line icon
/// (`Tumbleweed`, `icon_line`) isn't used: CONscripter has one icon. The
/// frame time isn't in the files: 50 ms, ours.
fn wait_icon(a: &mut dyn Assets) -> Option<WaitIcon> {
    let parts: Vec<String> = (1..=18).map(|n| format!("cg/sys/icon/page/bullet-{n:03}.png")).collect();
    let (image, w, h) = a.strip(&parts.iter().map(String::as_str).collect::<Vec<_>>())?;
    Some(WaitIcon { image, frame_ms: vec![50; 18], w: w / 18, h })
}

/// The text log (`sys_backlog.nss`): the parchment scroll
/// (`バックログ背景.jpg`); one row back (708,228) and on (724,284), EXIT
/// (690,530), the knob on the rail at x 646 from y 90 to 490
/// (`CreateScrollbar`). Black 20 px text (`SetFont`, ＭＳ ゴシック) from
/// (120,99), rows 24 px apart, 24 characters wide (measured under wine;
/// the rows fill the parchment down to 507). The voice icon (`スピーカー`)
/// left of a voiced row is ours. Left out: the text's #AAAA88 outline,
/// the page buttons (698,138 and 724,362) and the rolls turning as it
/// scrolls.
fn line_log(a: &mut dyn Assets) -> Option<LineLog> {
    const DIR: &str = "cg/sys/backlog";
    let strip = |a: &mut dyn Assets, names: &[&str], x: i32, y: i32| {
        let parts: Vec<String> = names.iter().map(|n| format!("{DIR}/{n}.png")).collect();
        let (image, w, h) = a.strip(&parts.iter().map(String::as_str).collect::<Vec<_>>())?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let button = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (n1, n2, n3) = (format!("{name}-01"), format!("{name}-02"), format!("{name}-03"));
        strip(a, &[&n1, &n2, &n1, &n3], x, y)
    };
    let (background, w, h) = a.image(&format!("{DIR}/バックログ背景.jpg"))?;
    Some(LineLog {
        background: Placed { image: background, x: 0, y: 0, w: w as i32, h: h as i32 },
        up: button(a, "バックログrew002", 708, 228)?,
        down: button(a, "バックログff003", 724, 284)?,
        close: strip(a, &["backlog-exit-001", "backlog-exit-002", "backlog-exit-001", "backlog-exit-003"], 690, 530)?,
        knob: strip(a, &["スクロール001", "スクロール002"], 635, 90)?,
        knob_y: (90, 460),
        voice_icon: strip(a, &["スピーカー001", "スピーカー002", "スピーカー003"], 60, 0)?,
        text_x: 120,
        text_y: 99,
        row_step: 24,
        rows: 17,
        columns: 24,
        font_size: 20,
        colour: "#000000".into(),
        fade_ms: 300,
    })
}

/// The yes/no questions (`sys_close.nss` and the others): a full-screen
/// picture (a black silhouette with the question in orange) over the
/// screen as it is, はい at (281,238) and いいえ at (454,238), fading in
/// and out over 200 ms. Opened with no menu up, the original draws them
/// 70 px lower; they are always opened from a menu here. The game asks
/// nothing before deleting a save (it can't) or resetting the settings.
fn confirm(a: &mut dyn Assets) -> Option<ConfirmDialog> {
    const DIR: &str = "cg/sys/dialog";
    let mut pictures = Vec::new();
    for (kind, name) in [
        (Switch::ConfirmQuit, "ゲーム終了？"),
        (Switch::ConfirmTitle, "タイトル戻る？"),
        (Switch::ConfirmSave, "セーブする？"),
        (Switch::ConfirmLoad, "ロードする？"),
        (Switch::ConfirmBackToChoice, "選択肢にもどる？"),
    ] {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        pictures.push((kind, Placed { image, x: 0, y: 0, w: w as i32, h: h as i32 }));
    }
    let button = |a: &mut dyn Assets, name: &str, x: i32| {
        let state = |n: u32| format!("{DIR}/{name}-{n:02}.png");
        let (s1, s2, s3) = (state(1), state(2), state(3));
        let (image, w, h) = a.strip(&[&s1, &s2, &s1, &s3])?;
        Some(Placed { image, x, y: 238, w: w as i32, h: h as i32 })
    };
    Some(ConfirmDialog {
        base: pictures[0].1.clone(),
        yes: button(a, "yes", 281)?,
        no: button(a, "no", 454)?,
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
        backdrop: None,
        pressed: true,
        fade_ms: 200,
        unroll: None,
    })
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
/// from y 113 on the pull-down base (282,93), which has the rows' dark
/// labels; a row at rest shows the base alone (the script's `00-` lies
/// under it, all but hidden, under wine), under the pointer `001` (red,
/// over it), pressed `002`. The base comes and goes with a 600 ms wipe
/// down (`DrawTransition`, `downr_0_0.png`), drawn as a fade. The keys are the ones printed on the rows (A, R, B, S, L, F,
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
        let (s1, s2) = (state("001"), state("002"));
        let (image, w, h) = a.strip(&["", &s1, "", &s2])?;
        items.push(CommandItem { button: Placed { image, x: 282, y: 113 + 24 * n as i32, w: w as i32, h: h as i32 }, action, key, label: None });
    }
    let (image, w, h) = a.image(&format!("{DIR}/プルダウン台座.png"))?;
    Some(CommandMenu { backdrop: Placed { image, x: 282, y: 93, w: w as i32, h: h as i32 }, items, fade_ms: 600, step_ms: 0, key: None, pressed: true })
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
        ("extra", 22, 415, TitleAction::Extras),
        ("exit", 691, 449, TitleAction::Exit),
        ("liner", 457, 479, TitleAction::Unavailable),
    ] {
        let state = |n: u32| format!("{DIR}/{name}-{n:02}.png");
        let (s1, s2) = (state(1), state(2));
        let cells: Vec<&str> = if action == TitleAction::Unavailable { vec![&s1] } else { vec![&s1, &s2, &s2] };
        let (image, _, _) = a.strip(&cells)?;
        let click_se = (action != TitleAction::Unavailable).then(|| shot.clone()).flatten();
        buttons.push(TitleButton { image, x, y, action, click_se, shown_by: None });
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
        ..Default::default()
    })
}
