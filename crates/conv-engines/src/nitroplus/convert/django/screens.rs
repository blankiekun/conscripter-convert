//! 続・殺戮のジャンゴ's system screens, built from its own pictures
//! (`cg/sys/`) with the places and times of its scripts (`boot.nss`,
//! `sys_*.nss`). Not compared with the original yet (it asks for its disc
//! under wine).

use conv_core::ir::{
    BarAction, ButtonBar, CommandItem, CommandMenu, ConfirmDialog, GridText, LineLog, Op, Placed, Rect, SaveGrid, Script, Switch, TitleAction, TitleButton, TitleMotion, TitleOpening, TitlePicture, TitleScreen, TitleSprite, WaitIcon,
};

use conv_core::ir::{PairOption, PicturePair, PictureSettings, PictureSlider, SettingsScreen, SliderSetting, SliderSteps, VoiceEntry, VoiceLayout, VoiceWindow};

use super::super::{Assets, Story};

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script, story: &Story) {
    script.title_screen = title(a);
    // The title comes up as TitleSet draws it: the first of a run with
    // $Logo 0, coming back to it (from the story or the extras) with
    // $Logo 1; compiled for the story finished and not (#ClearG).
    let cleared = story.flags.get(super::extras::CLEARED).copied();
    if let (Some(t), Some(cleared)) = (script.title_screen.as_mut(), cleared) {
        let (opening, back) = (super::super::OPENING_LABEL, super::super::OPENING_BACK_LABEL);
        for (base, at_launch, after_extras) in [(opening, true, false), (back, false, false), (back, false, true)] {
            for v in [1, 0] {
                let label = if v == 0 { base.to_string() } else { format!("{base}{v}") };
                let Some(loops) = opening_loops(&script.ops, &label) else { continue };
                let clocks = loops.iter().filter_map(|(image, layer, keys, from_key)| Some((title_motion(a, t, image, keys, *from_key)?, *layer))).collect();
                t.openings.push(TitleOpening { at_launch, after_extras, script: Some(label), when: vec![(cleared, v)], clocks, ..Default::default() });
            }
        }
    }
    // The game has no button bar; the command menu runs a bar's actions.
    script.button_bar = Some(ButtonBar { buttons: Vec::new(), top: None, base: None, slide: None, pressed: true, quick_save_se: None, load_fade_ms: 0, quick_saved: None });
    script.command_menu = command_menu(a);
    script.save_grid = save_grid(a);
    script.confirm = confirm(a);
    script.line_log = line_log(a);
    script.wait_icon = wait_icon(a);
    script.picture_settings = settings(a, story);
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

/// The looping motions TitleSet's processes leave on the opening compiled
/// at `label` (the buttons' `*Shake`, `TumbleweedMove`): each picture's
/// image, layer, keys and the key its loop goes back to. None: no such
/// opening.
#[allow(clippy::type_complexity)]
fn opening_loops(ops: &[Op], label: &str) -> Option<Vec<(String, u32, Vec<conv_core::ir::MotionKey>, usize)>> {
    use conv_core::ir::MotionEnd;
    let start = ops.iter().position(|op| matches!(op, Op::Label(l) if l == label))?;
    let mut images = std::collections::HashMap::new();
    let mut loops = Vec::new();
    for op in ops[start..].iter().take_while(|op| !matches!(op, Op::Return)) {
        match op {
            Op::Sprites { changes, .. } => {
                for c in changes {
                    images.insert(c.layer, c.sprite.as_ref().map(|p| p.image.clone()));
                }
            }
            Op::SpriteMotion2 { layer, keys, end: end @ (MotionEnd::Loop | MotionEnd::LoopFrom(_)) } => {
                let from = if let MotionEnd::LoopFrom(k) = end { *k } else { 0 };
                if let Some(Some(image)) = images.get(layer) {
                    loops.retain(|l: &(String, u32, _, usize)| l.1 != *layer);
                    loops.push((image.clone(), *layer, keys.clone(), from));
                }
            }
            _ => {}
        }
    }
    Some(loops)
}

/// The title's sprite drawing the opening's picture `image` (a button's
/// `-01` picture, or the tumbleweed, added to its pictures), given the
/// motion `keys` at rest if it has none yet.
fn title_motion(a: &mut dyn Assets, t: &mut TitleScreen, image: &str, keys: &[conv_core::ir::MotionKey], from_key: usize) -> Option<TitleSprite> {
    const NAMES: [&str; 6] = ["start", "load", "config", "extra", "exit", "liner"];
    let file = image.rsplit('/').next()?.to_ascii_lowercase();
    let sprite = if let Some(i) = NAMES.iter().position(|n| file == format!("{n}-01.png")) {
        TitleSprite::Button(i)
    } else if file == "tumbleweed.png" {
        // src: boot.nss TitleSet (タイトル毛玉 at 300, 420); the title's only picture.
        if t.pictures.is_empty() {
            let (image, _, _) = a.image("cg/sys/title/Tumbleweed.png")?;
            t.pictures.push(TitlePicture { image, x: 300, y: 420, ..Default::default() });
        }
        TitleSprite::Picture(0)
    } else {
        return None;
    };
    if t.motions.iter().all(|m| m.sprite != sprite) {
        t.motions.push(TitleMotion { sprite, keys: keys.to_vec(), from_key });
    }
    Some(sprite)
}

/// The settings screen (`sys_config.nss`) over `コンフィグ地`: rows of ten
/// bullets (`bar-弾`, 12 px apart from x 306) for the text speed (y 212),
/// auto mode's wait (308) and the BGM, SE and voice volumes (422, 470,
/// 518), every bullet up to the chosen one red (`setgrade`); ON (x 386)
/// and OFF (440) pairs; GAME-SPEED's four presets at y 188; EXIT
/// (650,518). Pictures: `-001` at rest, `-002` under the pointer, `-003`
/// chosen. Values as `config_default` and `setgrade` set them: text speed
/// level k is `k × 100` (level 1 none), a character taking the exe's
/// base 30 ms × that ÷ 1000; auto mode waits `20 × k` ms a character
/// (the exe adds characters × #SYSTEM_auto_wait_per_character, no page
/// base; default 5); volumes `k × 100` of 1000 (defaults from
/// system.ini: 7, 9, 10); GAME-SPEED is the engine's play speed 3
/// (NORMAL, default), 5 (QUICK-A), 12 (QUICK-B), 1 (LOWSPEC), each the
/// percent of 装甲悪鬼村正's exe table (assumed the same engine). Left
/// out and drawn as they stand by default: the two questions' rows (ON
/// chosen, OFF at rest; one switch here turns all questions off), the
/// display options, the voice test, DEFAULT (it would ask with no
/// picture of its own), the links. 各キャラ opens [`voice_window`].
/// A character's ms in the exe at a text speed (0-1000) and game speed
/// NORMAL: the interval 30 × the speed stored as an f32 × 0.001, cut to
/// an integer (so 700 gives 20, 900 gives 26); 0 puts the page at once.
// src: Django.exe set_system_variable case 0xb 0045f11a-0045f126 (#SYSTEM_text_speed × 0.001 into the f32 at 005293d0);
// 004605b0 (005295b0 = 0x1e: the scripts set no $SYSTEM_text_interval); 00461e40 at 004625de-004625f9 (an
// interval of 0 at 0x28: fild 005295b0, fmul 005293d0, _ftol2), 0046259a-004625bc (elapsed ms × text_speed_seconds,
// 1.0 at play_speed 3, _ftol2), 00462601-0046261a (characters = that ÷ (wait + interval)), 00462576-0046258e (005293d0
// == 0.0: no wait); numbers: research/nitroplus/django_text_speed_exe.py
fn letter_ms(speed: u32) -> u32 {
    (30.0 * f64::from((f64::from(speed) * 0.001) as f32)) as u32  // src: exe 0045f120, 004625f3, 004625f9
}

/// The slider position (TextSpeed { ms: 30 }: position × 30 ÷ 1000)
/// whose ms is the exe's at a text speed.
fn text_speed_position(speed: u32) -> u32 {
    (letter_ms(speed) * 1000).div_ceil(30)  // src: exe 005295b0 (30), picture_settings.rs TextSpeed
}

fn settings(a: &mut dyn Assets, story: &Story) -> Option<PictureSettings> {
    const DIR: &str = "cg/sys/config";
    let button = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let state = |n: u32| format!("{DIR}/{name}-{n:03}.png");
        let (s1, s2, s3) = (state(1), state(2), state(3));
        let (image, w, h) = a.strip(&[&s1, &s2, &s3])?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    // The chosen cell in all three: an engine radio's picture while on.
    let picked = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let s3 = format!("{DIR}/{name}-003.png");
        let (image, w, h) = a.strip(&[&s3, &s3, &s3])?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let picture = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let states = |a: &mut dyn Assets, n: [&str; 3], x: i32, y: i32| {
        let f = |i: usize| format!("{DIR}/{}.png", n[i]);
        let (image, w, h) = a.strip(&[&f(0), &f(1), &f(2)])?;
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
        horizontal: false,
        fill: None,
    };
    // src: sys_config.nss btn_*_01-10 (x 306-414, 12 apart)
    let bullets = |a: &mut dyn Assets, y: i32, position: &dyn Fn(u32) -> u32| -> Option<Vec<(Placed, u32)>> {
        (1..=10).map(|k| Some((button(a, "bar-弾", 306 + 12 * (k as i32 - 1), y)?, position(k)))).collect()
    };
    let bullets_chosen = |a: &mut dyn Assets, y: i32| -> Option<Vec<Placed>> { (1..=10).map(|k| picked(a, "bar-弾", 306 + 12 * (k - 1), y)).collect() };
    // src: sys_config.nss setgrade (text speed k × 100, level 1 none; auto 20 ms × k; volumes k × 100)
    let text_speed = |k: u32| text_speed_position(if k == 1 { 0 } else { k * 100 });
    let per_char = |k: u32| k * 100;
    let volume = |k: u32| 1000 - k * 100;
    // The level a position of SliderSetting::GameSpeed gives: 12 - p × 12 / 1000.
    let speed = |level: u32| ((12 - level) * 1000).div_ceil(12);
    let mut presets = Vec::new();
    let mut presets_chosen = Vec::new();
    // src: sys_config.nss NORMAL, QUICKA, QUICKB, LOWSPEC (y 188; #SYSTEM_play_speed 3, 5, 12, 1)
    for (name, x, level) in [("normal", 162, 3), ("quick-a", 238, 5), ("quick-b", 314, 12), ("lowspec", 392, 1)] {
        presets.push((button(a, name, x, 188)?, speed(level)));
        presets_chosen.push(picked(a, name, x, 188)?);
    }
    let sliders = vec![
        // src: Django.exe reset_system_defaults 0045cb30 (the table at 00529408 that text_speed_seconds 0045d520 reads)
        stepped(SliderSetting::GameSpeed { levels: [50, 75, 100, 150, 200, 300, 400, 550, 800, 1200, 1600, 3200] }, presets, speed(3), false),
        // The engine starts at 500 (level 5, under wine; config_default's
        // 1000 is DEFAULT's); a character's ms: letter_ms.
        stepped(SliderSetting::TextSpeed { ms: 30 }, bullets(a, 212, &text_speed)?, text_speed(5), true),
        // src: setgrade's 20 ms × level, config_default's 100 (level 5); Django.exe 00461e40 (0x46217e,
        // 0x462268: auto wait += characters × #SYSTEM_auto_wait_per_character); volumes: system.ini 700, 900, 1000
        stepped(SliderSetting::AutoWait { ms: 200 }, bullets(a, 308, &per_char)?, per_char(5), true),
        stepped(SliderSetting::BgmVolume, bullets(a, 422, &volume)?, volume(7), true),
        stepped(SliderSetting::SeVolume, bullets(a, 470, &volume)?, volume(9), true),
        stepped(SliderSetting::VoiceVolume, bullets(a, 518, &volume)?, volume(10), true),
    ];
    let mut steps_chosen = vec![presets_chosen];
    for y in [212, 308, 422, 470, 518] {
        steps_chosen.push(bullets_chosen(a, y)?);
    }
    // src: sys_config.nss btn_*_ON/OFF (x 386/440) and config_default's values
    let mut pairs = Vec::new();
    let mut chosen = Vec::new();
    for (option, y, default_on) in [
        (PairOption::ConfirmSave, 82, true),
        (PairOption::ConfirmLoad, 106, true),
        (PairOption::MovieStop, 130, false),
        (PairOption::StopAtChoice, 236, true),
        (PairOption::SkipUnread, 260, false),
        (PairOption::Bgm, 398, true),
        (PairOption::Se, 446, true),
        (PairOption::Voice, 494, true),
    ] {
        pairs.push(PicturePair { option, on: button(a, "on", 386, y)?, off: button(a, "off", 440, y)?, default_on });
        chosen.push((picked(a, "on", 386, y)?, picked(a, "off", 440, y)?));
    }
    for (option, (on, on_x), (off, off_x), y) in [(PairOption::VoicePlaysOn, ("全て再生", 294), ("クリックで停止", 368), 284), (PairOption::AutoWaitsForVoice, ("終了まで待つ", 272), ("再生を中断する", 368), 332)] {
        pairs.push(PicturePair { option, on: button(a, on, on_x, y)?, off: button(a, off, off_x, y)?, default_on: true });
        chosen.push((picked(a, on, on_x, y)?, picked(a, off, off_x, y)?));
    }
    // src: sys_config.nss (DEFAULT, the question rows, CHARACTER_ONOFF, VOICE_TEST, ASPECT_UNFIXED, ENTRY)
    let fixed = vec![
        picture(a, "ディスプレイ依存-003", 516, 82)?,
        picture(a, "ユーザー-001", 516, 476)?,
    ];
    let (voice_window, voices_checked) = voice_window(a, story)?;
    Some(PictureSettings {
        background: Placed { image: background, x: 0, y: 0, w: w as i32, h: h as i32 },
        fixed,
        close: button(a, "exit", 650, 518)?,
        // src: sys_config.nss DEFAULT (450,58) and VOICE_TEST (440,542); no question before the reset (ours)
        reset: states(a, ["001-初期化_x450-y58", "002-初期化_x450-y58", "003-初期化_x450-y58"], 450, 58),
        voice_test: states(a, ["vioce-test-01", "vioce-test-02", "vioce-test-03"], 440, 542),
        test_voice: a.sound("cg/sys/config/test.wav"),
        sliders,
        pairs,
        texts: Vec::new(),
        fade_ms: 300, // src: sys_config.nss Fade("video", 300)
        // src: sys_config.nss: bullets switch at once; the pairs fade over 200 (one value here)
        pair_fade_ms: 0,
        voice_window: Some(voice_window),
        screen: SettingsScreen { chosen, voices_checked, steps_chosen },
    })
}

/// The character voices window (`charavoiceconfig`), opened by 各キャラ
/// (293,362): `window.png` at (240,230), a check box per voice class
/// (`#voice_skip_<class>`, `チェックボックス-01` while heard, `-02` muted),
/// OK at (390,340). Every class always listed; no backdrop. Ours: it fades
/// in and out over 300 ms (the original zooms the window in over 500 ms and
/// out over 300).
fn voice_window(a: &mut dyn Assets, story: &Story) -> Option<(VoiceWindow, Vec<Vec<String>>)> {
    const DIR: &str = "cg/sys/config";
    let placed = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let cells = |a: &mut dyn Assets, n: [&str; 3], x: i32, y: i32| {
        let f = |i: usize| format!("{DIR}/{}.png", n[i]);
        let (image, w, h) = a.strip(&[&f(0), &f(1), &f(2)])?;
        Some(Placed { image, x, y, w: w as i32 / 3, h: h as i32 })
    };
    let (on, _, _) = a.image(&format!("{DIR}/チェックボックス-01.png"))?;
    let mut entries = Vec::new();
    // src: sys_config.nss charavoiceconfig (CHAWND/cha<class> boxes)
    for (class, x, y) in [
        ("イライザ", 266, 258),
        ("名無し", 338, 258),
        ("リリィ", 398, 258),
        ("アルフィー", 458, 258),
        ("クラウス", 266, 282),
        ("ジュリアン", 338, 282),
        ("アイリス", 422, 282),
        ("その他女", 266, 306),
        ("その他男", 362, 306),
    ] {
        let picture = placed(a, "チェックボックス-02", x, y)?;
        let characters = story.voice_classes.iter().enumerate().filter(|(_, c)| c.as_str() == class).map(|(i, _)| i as u32).collect();
        entries.push(VoiceEntry { hit: (picture.w, picture.h), picture, characters, heard: None });
    }
    let checked = vec![vec![on.clone(); entries.len()]];
    let window = VoiceWindow {
        open: cells(a, ["各キャラ-001", "各キャラ-002", "各キャラ-003"], 293, 362)?,
        backdrop: (Placed { image: a.transparent(), x: 0, y: 0, w: 1, h: 1 }, 0),
        background: placed(a, "window", 240, 230)?,
        check: Placed { image: on, x: 0, y: 0, w: 20, h: 20 },
        layouts: vec![VoiceLayout { when: None, entries }],
        fade_ms: 300,
        close: cells(a, ["チェックボックスOK-01", "チェックボックスOK-02", "チェックボックスOK-03"], 390, 340),
    };
    Some((window, checked))
}

/// The click-wait icon (system.ini `icon_page`: `Bullet-%03d.png#20`; the
/// archive has 18 pictures of 18×18), looped after the text. The line icon
/// (`Tumbleweed`, `icon_line`) isn't used: CONscripter has one icon. A
/// picture lasts 8 passes of the render thread, 16 ms each at NORMAL:
/// 128 ms. Left out: the exe fades the icon in (alpha from 0, +16 at each
/// picture), CONscripter's icon has no fade.
fn wait_icon(a: &mut dyn Assets) -> Option<WaitIcon> {
    let parts: Vec<String> = (1..=18).map(|n| format!("cg/sys/icon/page/bullet-{n:03}.png")).collect();
    let (image, w, h) = a.strip(&parts.iter().map(String::as_str).collect::<Vec<_>>())?;
    // src: Django.exe 0x42f30a (step 0x2000 of a 16.16 counter: 1/8 picture a pass), 004477f0 (the step,
    // alpha +16), its caller 00435df0 and 00469580 (a pass waits 0x531a5c[4] = 16 ms ÷ the play speed 1.0)
    Some(WaitIcon { image, frame_ms: vec![8 * 16; 18], w: w / 18, h })
}

/// The text log (`sys_backlog.nss`): the parchment scroll
/// (`バックログ背景.jpg`); one row back (708,228) and on (724,284), EXIT
/// (690,530), the knob on the rail at x 646 from y 90 to 490
/// (`CreateScrollbar`). Black 20 px text with an #AAAA88 outline
/// (`SetFont`, ＭＳ ゴシック) from (120,100), 17 rows 24 px apart, 25
/// characters wide; the voice icon (`スピーカー`) at x 82 on a voiced row.
/// Left out: the page buttons (698,138 and 724,362) and the rolls turning as it
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
    // src: sys_backlog.nss (buttons, CreateScrollbar 646 from 90 to 490, SetFont 20 #000000/#AAAA88, Fade 300);
    // src: Django.exe 00403af0 (17 rows, row top 100), 00403400 (text from x 0x78 = 120, rows 0x18 = 24 apart),
    // 00403230 (a row breaks after 25 characters, 24 when the 25th is a bad_end), 00402020 (voice icons at x 0x52
    // = 82, the row's y), 00430af0 (the knob's top left on CreateScrollbar's line: 646, from 490 to 90)
    Some(LineLog {
        background: Placed { image: background, x: 0, y: 0, w: w as i32, h: h as i32 },
        up: button(a, "バックログrew002", 708, 228)?,
        down: button(a, "バックログff003", 724, 284)?,
        close: strip(a, &["backlog-exit-001", "backlog-exit-002", "backlog-exit-001", "backlog-exit-003"], 690, 530)?,
        knob: strip(a, &["スクロール001", "スクロール002"], 646, 90)?, // src: exe 00430af0
        knob_y: (90, 490), // src: exe 00430af0
        voice_icon: strip(a, &["スピーカー001", "スピーカー002", "スピーカー003"], 82, 0)?, // src: exe 00402020
        text_x: 120,
        text_y: 100, // src: exe 00403af0
        row_step: 24,
        rows: 17,
        columns: 25, // src: exe 00403230
        font_size: 20,
        colour: "#000000".into(),
        fade_ms: 300,
        outline: Some("#AAAA88".into()),
        row_width: None,
        gap: -1, // src: exe 00403400 (the pen moves 0x13 = 19 px a character, the font is 20)
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
    // src: sys_close.nss (yes 281,238, no 454,238, Fade 200); font_size ours (no text drawn)
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
            // src: sys_save.nss (LOGO 50,24; no-pic 461,481 136×101; slots from 50,80, 150 and 96 apart)
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
    // src: sys_save.nss CreateText places, <FONT size=18>, #EE9933; the date's and time's chars: ours
    let text = |x: i32, y: i32, font_size: u32, lines: u32, chars: u32, colour: &str| GridText { x, y, font_size, lines, chars, line_gap: 0, char_gap: 0, colour: colour.into(), vertical: false };
    Some(SaveGrid {
        save_pages,
        load_pages,
        frames: None,
        first: (50, 80),
        row_step: 96,
        column_step: 150,
        rows: 4,
        columns: 5,
        thumb_w: 100, // src: save-load-icon.png's size; the preview is it at 1360 (sys_save.nss Zoom)
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
        date: text(50, 486, 18, 1, 20, "#ffffff"),
        play_time: text(606, 30, 18, 1, 12, "#ee9933"),
        chapter: text(50, 464, 18, 0, 0, "#ffffff"),
        // src: sys_save.nss: the comment in SetFont's 19 px, a 354×54 box (18 characters, 2 rows)
        line: text(50, 518, 19, 2, 18, "#ffffff"),
        saved: None,
        // src: sys_save.nss: it opens over 200 and closes over 300 (one value here)
        fade_ms: 300,
        across: true,
        tabs: Vec::new(),
        popup: None,
        empty: None,
        quick_page: false,
        play_time_prefix: String::new(),
        play_time_hm: false,
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
    // src: sys_menu.nss (rows at 282, 113 + 24n; BAK 282,93; DrawTransition 600); keys: the rows' pictures
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
    Some(CommandMenu { backdrop: Placed { image, x: 282, y: 93, w: w as i32, h: h as i32 }, items, fade_ms: 600, step_ms: 0, key: None, pressed: true, keys: vec![] })
}

/// The title (`boot.nss`, `TitleSet` and `TitleSelect`): the background
/// with the logo at (84,111); six buttons
/// where `TitleSet` puts them, white at rest (`-01`), red under the
/// pointer (`-02`, with a revolver being cocked); a shot when one is
/// clicked. Start fades the music and the screen to black over a second.
/// 特別 and LINER NOTES answer only once the story is finished
/// (`#ClearG`): the extras aren't converted yet, and LINER NOTES opened a
/// page of Nitroplus's site; both are shown and refuse a click. The first
/// title of a run comes up as `TitleSet` draws it (compiled, [`menus`]).
/// Left out: the opening on coming back to the title (it fades in), the
/// buttons' shaking, the tumbleweed rolling by (once it has rolled off in
/// the opening, it is never seen at rest), the red slash over the screen
/// on a click (`-03`), and the tumbleweed's hidden chapter select.
fn title(a: &mut dyn Assets) -> Option<TitleScreen> {
    const DIR: &str = "cg/sys/title";
    // src: boot.nss TitleSet (places), TitleSelect (Start: SetVolume 1000, Fade 1000)
    let mut parts = Vec::new();
    for (name, x, y) in [("title-back.jpg", 0, 0), ("title-logo.png", 84, 111)] {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Django.exe under Unicorn (research/nitroplus/django_text_speed_exe.py):
    /// text speed, the first elapsed ms 00461e40 puts a character at.
    const EXE_LETTER_MS: [(u32, u32); 11] =
        [(0, 0), (100, 3), (200, 6), (300, 9), (400, 12), (500, 15), (600, 18), (700, 20), (800, 24), (900, 26), (1000, 30)];

    #[test]
    fn text_speed_matches_exe() {
        for (speed, ms) in EXE_LETTER_MS {
            assert_eq!(letter_ms(speed), ms, "text speed {speed}");
            // what the engine's slider line (position × 30 / 1000) makes of the position
            assert_eq!(text_speed_position(speed) * 30 / 1000, ms, "text speed {speed}");
        }
    }
}
