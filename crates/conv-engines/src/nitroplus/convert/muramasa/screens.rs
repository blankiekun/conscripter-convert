//! 装甲悪鬼村正's system screens, built from its own pictures (`cg/sys/`)
//! with the places, sizes and times of its scripts (`sys_*.nss`), as
//! filmed in the original (`docs/games/muramasa.md`, "System screens").
//!
//! Buttons are made of one picture per state: `001` normal, `002` under
//! the pointer, `003` pressed, `000` disabled (where there is one); they
//! are joined into strips of normal, hover, disabled, pressed.

use conv_core::ir::{
    BarAction, BarButton, BarSlide, ButtonBar, CommandItem, CommandMenu, ConfirmDialog, GridText, LineLog, PairOption, PicturePair, PictureSettings, PictureSlider, PictureText, Placed, SettingsScreen, Rect, SaveGrid, SlotPopup,
    Script, SliderSetting, Switch, TitleAction, TitleButton, TitleGlow, TitleOpening, TitleScreen, Unroll, Var, VoiceEntry, VoiceLayout, VoiceWindow, WaitIcon,
};

use super::super::{Assets, Story};
use super::opening::{self, TitleLook};

/// The favour screen (`sys_hmode.nss`), compiled as a subroutine for the
/// menu's 好感度記録.
pub const FAVOUR: &str = "conv_favour";

// src: sys_config.nss ConfigSet (CreateWindow 1024×576)
const SCREEN_W: i32 = 1024;
const SCREEN_H: i32 = 576;

/// Fills the script's system screens.
pub fn menus(a: &mut dyn Assets, script: &mut Script, story: &Story) {
    menus_in(a, script, story, false)
}

/// [`menus`] with the English port's title ([`opening::EN`]), command
/// menu ([`command_menu_en`]) and questions ([`confirm_en`]).
pub fn menus_en(a: &mut dyn Assets, script: &mut Script, story: &Story) {
    menus_in(a, script, story, true)
}

fn menus_in(a: &mut dyn Assets, script: &mut Script, story: &Story, en: bool) {
    script.confirm = if en { confirm_en(a) } else { confirm(a) };
    script.title_screen = title(a, story, if en { &opening::EN } else { &opening::JP });
    script.button_bar = quick_bar(a, en);
    script.command_menu = if en { command_menu_en(a) } else { command_menu(a) };
    script.save_grid = if en { save_grid_en(a) } else { save_grid(a) };
    script.line_log = line_log(a, en);
    script.wait_icon = wait_icon(a);
    script.picture_settings = if en { settings_en(a) } else { settings(a, story) };
    script.picture_extras = super::extras::extras(a, story);
}

/// The settings screen (`sys_config.nss`): コンフィグ over the screen, its
/// controls laid over it (`ConfigSet`). Sliders at x 358 (voice), 430
/// (effects), 502 (music), 610 (auto wait), 754 (text speed), 862 (game
/// speed): the knob from y 67 to 206 over a track from y 67, 147 high
/// (`CreateScrollbar`, バー裏側アタリ判定); a knob's place is
/// 67 + position × 139/1000, its default position the value's at the first
/// start (`ConfigFormat`; the volumes and speeds add 7, `$CVer`), checked
/// against the original's knobs. The 入/切 pairs at y 183 and 213 (two take
/// their own pictures higher up); every control's resting picture is erased
/// (the picture shows it), its `002` shows under the pointer, a pair's
/// chosen side its `003`, cross-faded over 100 ms. The display options
/// (left column) are drawn as they stand by default. 承認 asks
/// 設定を初期状態に戻す and puts the defaults back; 実験 plays a line of
/// 景明's. The game speed's levels are the exe's table (`FUN_0047c250`:
/// 0.5, 0.75, 1 (default 3), 1.5, 2, 3, 4, 5.5, 8, 12, 16, 32 times).
fn settings(a: &mut dyn Assets, story: &Story) -> Option<PictureSettings> {
    const DIR: &str = "cg/sys/config";
    let picture = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    // At rest (erased: empty, or `rest`), under the pointer, pressed.
    let control = |a: &mut dyn Assets, rest: &str, hover: &str, pressed: &str, x: i32, y: i32| {
        let part = |n: &str| if n.is_empty() { String::new() } else { format!("{DIR}/{n}.png") };
        let (rest, hover, pressed) = (part(rest), part(hover), part(pressed));
        let (image, w, h) = a.strip(&[&rest, &hover, &pressed])?;
        Some(Placed { image, x, y, w: w as i32 / 3, h: h as i32 })
    };
    let knob = |a: &mut dyn Assets, x: i32| {
        let (image, w, h) = a.strip(&[&format!("{DIR}/001-矢印.png"), &format!("{DIR}/002-矢印移動時.png")])?;
        Some(Placed { image, x, y: 67, w: w as i32 / 2, h: h as i32 })
    };
    let mut sliders = Vec::new();
    // src: sys_config.nss ConfigSet ($Config*X, $ConfigMathY 67, $ConfigCursor2 206, $pos + $CVer 7); system.ini [サウンド] volumes 800/800/600
    for (setting, x, default) in [
        (SliderSetting::VoiceVolume, 358, 207),
        (SliderSetting::SeVolume, 430, 207),
        (SliderSetting::BgmVolume, 502, 407),
        // 0-200 ms a character (default 100): the exe waits that long for
        // each character of the page.
        (SliderSetting::AutoWait { ms: 200 }, 610, 507),
        // Default 200 of 1000: 9.6 ms a character, as the story is shown.
        // src: exe 00483930 (a character's ms = $SYSTEM_text_interval × text_speed ÷ 1000), function.nss SystemInit (interval 48)
        (SliderSetting::TextSpeed { ms: 48 }, 754, 207),
        // src: exe FUN_0047c250 (the speed table)
        (SliderSetting::GameSpeed { levels: [50, 75, 100, 150, 200, 300, 400, 550, 800, 1200, 1600, 3200] }, 862, 750),
    ] {
        let (_, tw, th) = a.image(&format!("{DIR}/バー裏側アタリ判定.png"))?;
        sliders.push(PictureSlider { setting, knob: knob(a, x)?, knob_y: (67, 206), track: Rect { x, y: 67, w: tw as i32, h: th as i32 }, default, steps: None, horizontal: false, fill: None });
    }
    let mut pairs = Vec::new();
    let mut chosen = Vec::new();
    // src: sys_config.nss ConfigSet ($Config*X, $ConfigMathY3/4 183/213, KVoice 63/133, SVoice 49/137) and ConfigFormat (defaults)
    for (option, x, default_on, on, off) in [
        (PairOption::Confirm, 827, true, ("入", 183), ("切", 213)),
        (PairOption::MovieStop, 791, false, ("入", 183), ("切", 213)),
        (PairOption::StopAtChoice, 719, true, ("入", 183), ("切", 213)),
        (PairOption::SkipUnread, 683, false, ("入", 183), ("切", 213)),
        (PairOption::VoicePlaysOn, 647, true, ("全てを再生", 63), ("クリックで停止", 133)),
        (PairOption::AutoWaitsForVoice, 575, true, ("終了まで待つ", 49), ("再生を中断する", 137)),
        (PairOption::Bgm, 539, true, ("入", 183), ("切", 213)),
        (PairOption::Se, 467, true, ("入", 183), ("切", 213)),
        (PairOption::Voice, 395, true, ("入", 183), ("切", 213)),
    ] {
        let side = |a: &mut dyn Assets, (name, y): (&str, i32)| control(a, "", &format!("002-{name}"), &format!("003-{name}"), x, y);
        pairs.push(PicturePair { option, on: side(a, on)?, off: side(a, off)?, default_on });
        let picked = |a: &mut dyn Assets, (name, y): (&str, i32)| {
            let n = format!("003-{name}");
            control(a, &n, &n, &n, x, y)
        };
        chosen.push((picked(a, on)?, picked(a, off)?));
    }
    // The display options as a first start leaves them: aspect not set by
    // itself (切), display dependent, the scaling methods not chosen.
    // src: sys_config.nss ConfigSet ($ConfigAAutoX 251, FAspect 215,51, $ConfigWay 177,255)
    let fixed = vec![picture(a, "003-切", 251, 213)?, picture(a, "003-ディスプレイ依存", 215, 51)?, picture(a, "002-アスペクト方法OFF", 177, 255)?];
    // src: sys_config.nss ConfigSet (バージョン情報: SetFont 11 #bb7420, 453/678,548; Exit 26,108, Default 899,197, Test 287,197; Fade 300, 100)
    let version = |text: &str, x: i32| PictureText { text: format!("◆ＧＡＭＥ{text}バージョン情報：1.00"), x, y: 548, font_size: 11, colour: "#bb7420".into() };
    let voice_window = voice_window(a, story);
    let mut voices_checked = Vec::new();
    for layout in voice_window.iter().flat_map(|v| &v.layouts) {
        let check = &voice_window.as_ref()?.check.image;
        let entries = layout.entries.iter().map(|e| a.composite(&[(e.picture.image.clone(), 0, 0, 1000), (check.clone(), 0, 0, 1000)]).map(|c| c.0)).collect::<Option<Vec<_>>>()?;
        voices_checked.push(entries);
    }
    Some(PictureSettings {
        background: picture(a, "コンフィグ", 0, 0)?,
        fixed,
        close: control(a, "001-退場", "002-退場", "003-退場", 26, 108)?,
        reset: control(a, "", "002-承認", "003-承認", 899, 197),
        voice_test: control(a, "", "002-実験", "003-実験", 287, 197),
        test_voice: a.sound("voice/ma01/0250430b56"),
        sliders,
        pairs,
        texts: vec![version("システム", 453), version("スクリプト", 678)],
        fade_ms: 300,
        pair_fade_ms: 100,
        voice_window,
        engine_screen: Some(SettingsScreen { chosen, voices_checked }),
    })
}

/// The English port's settings screen (`sys_config.nut`, its own design):
/// `ConfigBG` with the controls at the places `main` gives them. Check
/// boxes (`ConfigCreateToggle1`, 52×52 `ConfigButton-box…`): skip movies
/// (91,74), wait for the voice in auto mode (377,396); the mutes
/// (`Toggle3`, round, 769 × 57/130/203); the ON/OFF pairs (`Toggle2`, 37×37
/// `ConfigButton-small-…`, ON at x 394, OFF 35 right): skip unread (y 179),
/// skip the voice on a click (239), stop auto and skip at a choice (299).
/// Gauges (`ConfigCreateGage`): a 141×34 track, the knob (7×13) 19 down
/// and running 134 across, `ConfigSlider-Full` lit from 4 left and 7 down
/// to the knob: auto speed (133,405), text speed (133,483), music,
/// effects and voices (588 × 65/138/211). Restore (377,74), the voice
/// test (851,203), Close (854,555). What we can't set is drawn as a first
/// start leaves it: auto save at choices (on), read text's colour (on),
/// a click stopping auto mode (on), syncing text to voices (off), the
/// effects test, full screen (off), 1024×576, the display's aspect.
/// Compared with the original at a first start; the character voices
/// window (`OpenCVSetting`) and the text sample are not done.
fn settings_en(a: &mut dyn Assets) -> Option<PictureSettings> {
    const DIR: &str = "en/cg/sys/config";
    let picture = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let strip = |a: &mut dyn Assets, names: &[&str], x: i32, y: i32| {
        let names: Vec<String> = names.iter().map(|n| if n.is_empty() { String::new() } else { format!("{DIR}/{n}.png") }).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let (image, w, h) = a.strip(&refs)?;
        Some(Placed { image, x, y, w: w as i32 / names.len() as i32, h: h as i32 })
    };
    let box_on = ["ConfigButton-boxONcheck", "ConfigButton-boxONcheckOver", "ConfigButton-boxON"];
    let box_off = ["ConfigButton-box", "ConfigButton-boxMouseOver", "ConfigButton-boxMousePress"];
    let mut pairs = Vec::new();
    // Check boxes need none (their sides are the pictures): theirs repeated.
    let mut chosen = Vec::new();
    // src: sys_config.nut main (ConfigCreateToggle1-3, ConfigCreateGage places); sys_val.nut defaults
    for (option, x, y, default_on) in [(PairOption::MovieStop, 91, 74, false), (PairOption::AutoWaitsForVoice, 377, 396, true)] {
        pairs.push(PicturePair { option, on: strip(a, &box_on, x, y)?, off: strip(a, &box_off, x, y)?, default_on });
        chosen.push((pairs.last()?.on.clone(), pairs.last()?.off.clone()));
    }
    // The mutes: checked is muted (the pair's off).
    for (option, y) in [(PairOption::Bgm, 57), (PairOption::Se, 130), (PairOption::Voice, 203)] {
        let heard = strip(a, &["ConfigButton-round", "ConfigButton-roundMouseOver", "ConfigButton-roundMousePress"], 769, y)?;
        let muted = strip(a, &["ConfigButton-roundONcheck", "ConfigButton-roundONcheckMouseOver", "ConfigButton-roundON"], 769, y)?;
        pairs.push(PicturePair { option, on: heard, off: muted, default_on: true });
        chosen.push((pairs.last()?.on.clone(), pairs.last()?.off.clone()));
    }
    // ON and OFF: (the option's on side's column, …).
    let small = |a: &mut dyn Assets, side: &str, x: i32, y: i32| strip(a, &["ConfigButton-small-no", &format!("ConfigButton-small-{side}"), &format!("ConfigButton-small-{side}check")], x, y);
    for (option, y, on_is_on, default_on) in [(PairOption::SkipUnread, 179, true, false), (PairOption::VoicePlaysOn, 239, false, true), (PairOption::StopAtChoice, 299, true, true)] {
        let (on_x, off_x) = if on_is_on { (394, 429) } else { (429, 394) };
        let (on_side, off_side) = if on_is_on { ("on", "off") } else { ("off", "on") };
        pairs.push(PicturePair { option, on: small(a, on_side, on_x, y)?, off: small(a, off_side, off_x, y)?, default_on });
        // The chosen side as the routine draws it (its third cell over its
        // first), in all three cells: an engine radio's picture while on.
        let picked = |a: &mut dyn Assets, side: &str, x: i32| {
            let (no, w, h) = a.image(&format!("{DIR}/ConfigButton-small-no.png"))?;
            let (check, _, _) = a.image(&format!("{DIR}/ConfigButton-small-{side}check.png"))?;
            let cells: Vec<(String, i64, i64, i64)> = (0..3).flat_map(|k| [(no.clone(), k * w as i64, 0, 1000), (check.clone(), k * w as i64, 0, 1000)]).collect();
            let (image, _, _) = a.composite(&cells)?;
            Some(Placed { image, x, y, w: w as i32, h: h as i32 })
        };
        chosen.push((picked(a, on_side, on_x)?, picked(a, off_side, off_x)?));
    }
    let knob = strip(a, &["ConfigSlider-off", "ConfigSlider-on"], 0, 0)?;
    let full = picture(a, "ConfigSlider-Full", 0, 0)?;
    let mask = a.color(0, full.w as u32, full.h as u32);
    let mut sliders = Vec::new();
    // A first start's (`sys_val.nut`): auto_wait_per_character 60 (of
    // 100), text_speed 550, the volumes 800, 1000, 700 (of 1000); the
    // knobs where the original draws them.
    for (setting, x, y, default) in [
        (SliderSetting::AutoWait { ms: 200 }, 133, 405, 600),
        (SliderSetting::TextSpeed { ms: 100 }, 133, 483, 550),
        (SliderSetting::BgmVolume, 588, 65, 200),
        (SliderSetting::SeVolume, 588, 138, 0),
        (SliderSetting::VoiceVolume, 588, 211, 300),
    ] {
        sliders.push(PictureSlider {
            setting,
            knob: Placed { x, y: y + 19, ..knob.clone() },
            knob_y: (x, x + 134),
            track: Rect { x, y, w: 141, h: 34 },
            default,
            steps: None,
            horizontal: true,
            fill: Some((Placed { x: x - 4, y: y + 7, ..full.clone() }, mask.clone())),
        });
    }
    let mut fixed = Vec::new();
    // src: sys_config.nut main (the controls' places); their states: shot en/orig-config
    for (name, x, y) in [
        ("ConfigButton-boxONcheck", 229, 74),
        ("ConfigButton-small-oncheck", 394, 209),
        ("ConfigButton-small-no", 429, 209),
        ("ConfigButton-small-oncheck", 394, 269),
        ("ConfigButton-small-no", 429, 269),
        ("ConfigButton-box", 377, 472),
        ("ConfigButton-round", 851, 130),
        ("ConfigButton-box", 570, 423),
        ("ConfigButton-small-no", 742, 353),
        ("ConfigButton-small-no", 818, 353),
        ("ConfigButton-small-check", 894, 353),
        ("ConfigButton-small-no", 742, 392),
        ("ConfigButton-small-no", 818, 392),
        ("ConfigButton-small-no", 894, 392),
        ("ConfigButton-boxONcheck", 713, 472),
        ("ConfigButton-box", 838, 472),
    ] {
        fixed.push(picture(a, name, x, y)?);
    }
    Some(PictureSettings {
        background: picture(a, "ConfigBG", 0, 0)?,
        fixed,
        close: strip(a, &["CloseButton-box", "CloseButton-boxMouseOver", "CloseButton-boxMousePress"], 854, 555)?,
        reset: strip(a, &box_off, 377, 74),
        voice_test: strip(a, &["ConfigButton-round", "ConfigButton-roundMouseOver", "ConfigButton-roundMousePress"], 851, 203),
        test_voice: a.sound("voice/ma01/0250430b56"),
        sliders,
        pairs,
        texts: Vec::new(),
        // guess: fade 300 as the Japanese, pairs at once; not read from sys_config.nut
        fade_ms: 300,
        pair_fade_ms: 0,
        voice_window: None,
        engine_screen: Some(SettingsScreen { chosen, voices_checked: Vec::new() }),
    })
}

/// The characters of the voice window (`VoiceResetSet`): each one's name
/// (its pictures, its `#voice_on_` flag) and its voices' class.
// src: sys_config.nss VoiceResetSet; system.ini [ボイス] all_name
pub(super) const VOICES: [(&str, &str); 55] = [
    ("湊斗景明", "景明"), ("村正", "村正さん"), ("新田雄飛", "雄飛"), ("来栖野小夏", "小夏"), ("稲城忠保", "忠保"),
    ("鈴川令法", "鈴川"), ("大鳥香奈枝", "香奈枝"), ("永倉さよ", "さよ"), ("綾弥一条", "一条"), ("雪車町一蔵", "雪車町"),
    ("真改", "真改"), ("長坂右京", "長坂"), ("小太郎", "小太郎"), ("弥源太", "弥源太"), ("ふき", "ふき"),
    ("ふな", "ふな"), ("足利護氏", "護氏"), ("大鳥獅子吼", "獅子吼"), ("遊佐童心", "童心"), ("今川雷蝶", "雷蝶"),
    ("足利茶々丸", "茶々丸"), ("署長", "署長"), ("キャノン", "キャノン"), ("ガーゲット", "ガーゲット"), ("舞殿宮", "舞殿宮"),
    ("皇路操", "操"), ("皇路卓", "卓"), ("二世村正", "二世村正"), ("柳生常闇斎", "常闇斎"), ("芳養", "芳養"),
    ("研究所長", "研究所長"), ("義清", "義清"), ("青江", "青江"), ("皆斗本家", "本家"), ("湊斗統", "統"),
    ("山賊の首領", "首領姉"), ("首領の弟", "首領弟"), ("正宗", "正宗"), ("岡部桜子", "桜子"), ("足利邦氏", "邦氏"),
    ("ウィロー", "ウィロー"), ("黒瀬童子", "黒瀬"), ("コブデン", "コブデン"), ("ウォルフ", "ウォルフ"), ("永倉翁", "永倉翁"),
    ("大鳥花枝", "花枝"), ("始祖村正", "始祖村正"), ("飽間", "飽間"), ("浦夢", "浦夢"), ("サシュアント", "サシュアント"),
    ("オーリガ", "オーリガ"), ("孤児の光", "コヒカリ"), ("銀星号", "光"), ("その他男声", "その他男声"), ("その他女声", "その他女声"),
];
/// The flag set at the end of chapter 1 (`ma01_026`): the window then
/// lists everyone, with other pictures.
pub(super) const CHAPTER_ONE_CLEARED: &str = "#一章クリア";

pub(super) fn heard_flag(name: &str) -> String {
    format!("#voice_on_{name}")
}

/// The voice window (`ConfigVoiceSet`, `ConfigVoiceArray`): ボイスセレクト
/// ウィンドウ背景 over black at 700/1000, a character's picture (32×142, its
/// box at the top) once its `#voice_on_` flag is set, nineteen to a row
/// from (790,82) leftwards 32 apart, rows at y 82, 229 and 376, the box's
/// 15 px the button; ボイスチェック on the box while the voices play. Until
/// 一章クリア it lists nine characters in the first places and five in the
/// last, with their `1章_` pictures. Each switch mutes the voices of its
/// class (voices of other classes always play: `#SYSTEM_voice_enable_another`
/// is never turned off). It cross-fades in and out over 300 ms.
fn voice_window(a: &mut dyn Assets, story: &Story) -> Option<VoiceWindow> {
    const DIR: &str = "cg/sys/config";
    // src: sys_config.nss ConfigVoiceSet ($ConfigCvX 790 -= 32, $ConfigCvY 82/229/376; Fade 700, 300)
    let place = |slot: i32| match slot {
        1..=19 => (790 - 32 * (slot - 1), 82),
        20..=38 => (790 - 32 * (slot - 20), 229),
        _ => (790 - 32 * (slot - 39), 376),
    };
    let (_, hw, hh) = a.image(&format!("{DIR}/data/hit.png"))?;
    let hit_size = (hw as i32, hh as i32);
    let layout = |a: &mut dyn Assets, names: &[(i32, &str)], prefix: &str| -> Option<Vec<VoiceEntry>> {
        let mut entries = Vec::new();
        for &(slot, name) in names {
            let (x, y) = place(slot);
            let (image, w, h) = a.image(&format!("{DIR}/data/{prefix}{name}.png"))?;
            let class = VOICES.iter().find(|v| v.0 == name).map(|v| v.1)?;
            let characters = story.voice_classes.iter().enumerate().filter(|(_, c)| c.as_str() == class).map(|(i, _)| i as u32).collect();
            entries.push(VoiceEntry {
                picture: Placed { image, x, y, w: w as i32, h: h as i32 },
                hit: hit_size,
                characters,
                heard: story.flags.get(&heard_flag(name)).copied(),
            });
        }
        Some(entries)
    };
    let all: Vec<(i32, &str)> = VOICES.iter().enumerate().map(|(i, v)| (i as i32 + 1, v.0)).collect();
    // src: sys_config.nss ConfigVoiceArray (before 一章クリア)
    let chapter_one: Vec<(i32, &str)> = ["新田雄飛", "来栖野小夏", "稲城忠保", "鈴川令法", "大鳥香奈枝", "永倉さよ", "綾弥一条", "雪車町一蔵", "真改"]
        .into_iter()
        .enumerate()
        .map(|(i, n)| (i as i32 + 1, n))
        .chain(["湊斗景明", "村正", "銀星号", "その他男声", "その他女声"].into_iter().enumerate().map(|(i, n)| (i as i32 + 53, n)))
        .collect();
    let layouts = vec![
        VoiceLayout { when: story.flags.get(CHAPTER_ONE_CLEARED).copied(), entries: layout(a, &all, "")? },
        VoiceLayout { when: None, entries: layout(a, &chapter_one, "1章_")? },
    ];
    let (background, w, h) = a.image(&format!("{DIR}/data/ボイスセレクトウィンドウ背景.png"))?;
    let (check, cw, ch) = a.image(&format!("{DIR}/data/ボイスチェック.png"))?;
    let part = |n: &str| format!("{DIR}/{n}-別窓を開く.png");
    let (open, ow, oh) = a.strip(&["", &part("002"), &part("003")])?;
    let black = a.color(0, SCREEN_W as u32, SCREEN_H as u32);
    Some(VoiceWindow {
        // src: sys_config.nss ConfigSet ($ConfigAction 323,151)
        open: Placed { image: open, x: 323, y: 151, w: ow as i32 / 3, h: oh as i32 },
        backdrop: (Placed { image: black, x: 0, y: 0, w: SCREEN_W, h: SCREEN_H }, 178),
        background: Placed { image: background, x: 0, y: 0, w: w as i32, h: h as i32 },
        check: Placed { image: check, x: 0, y: 0, w: cw as i32, h: ch as i32 },
        layouts,
        fade_ms: 300,
    })
}

/// The click-wait flame (System.ini `icon_line`, `icon_page`: the 32
/// pictures of 改行改頁アイコン), looped, at each window's
/// `$SYSTEM_position_{x,y}_text_icon`. Pictures 1-19 are the flame
/// burning, 20-32 it withering to a wisp of smoke. Watched in the original
/// (wine under Xvfb redraws the waiting screen only about once a second,
/// so it couldn't be filmed): the flame burns about a second, withers
/// about as long, and lights again after a pause (here half a second).
fn wait_icon(a: &mut dyn Assets) -> Option<WaitIcon> {
    let parts: Vec<String> = (1..=32).map(|n| format!("cg/sys/icon/改行改頁アイコン/{n:03}.png")).collect();
    let (image, w, h) = a.strip(&parts.iter().map(String::as_str).collect::<Vec<_>>())?;
    // guess: watched, not filmed (clickwait-icon-60.mkv redraws about once a second)
    let mut frame_ms = vec![53; 19];
    frame_ms.extend([77; 13]);
    frame_ms[31] += 500;
    Some(WaitIcon { image, frame_ms, w: w / 32, h })
}

/// The backlog (`sys_backlog.nss`, System.ini `[バックログ]`): sixteen
/// rows of 26 characters from (140,49), 30 apart, MS 明朝 20 in black,
/// over バックログ背景; a voiced line's speaker icon (`icon_voice`) at
/// x 102 on its first row. 逆 (898,168) and 進 (898,303) scroll a row,
/// 退出 (927,438) closes; the scrollbar's knob runs from y 50 to 461 at
/// x 860 (`CreateScrollbar`). It fades in and out over 300 ms.
///
/// `en`: the English port's (`sys_backlog.nut`): its own pictures of the
/// same names, 退出 at (906,443), the rows from x 150, 570 wide in the
/// text's font at 20 (`TextBox.backlogFieldRect`, `FONT_LIST`
/// `バックログ` in `function_textbox.nut`): rows 570 px wide; the
/// field from y 10, 480 high, `backlogRowSpace` 10: 34 px apart (the
/// font's height 24 + 10), 14 rows, the first drawn from y 22 (where the
/// original's are, measured under wine).
fn line_log(a: &mut dyn Assets, en: bool) -> Option<LineLog> {
    let dir = if en { "en/cg/sys/backlog" } else { "cg/sys/backlog" };
    // src: sys_backlog.nss (buttons, scrollbar, SetFont 20, Fade 300), system.ini word_in_row 26; rows: shot backlog-1 (within 2 px)
    // src: sys_backlog.nut 退出 906,443; function_textbox.nut backlogFieldRect, backlogRowSpace; rows: shot en/orig-backlog
    // guess: the speaker icon's x 102 in the English port (no voiced row filmed)
    let ((close_x, close_y), text_x, columns) = if en { ((906, 443), 150, 28) } else { ((927, 438), 140, 26) };
    let strip = |a: &mut dyn Assets, names: &[&str], x: i32, y: i32| {
        let parts: Vec<String> = names.iter().map(|n| format!("{dir}/{n}.png")).collect();
        let (image, w, h) = a.strip(&parts.iter().map(String::as_str).collect::<Vec<_>>())?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let (background, w, h) = a.image(&format!("{dir}/バックログ背景.png"))?;
    Some(LineLog {
        background: Placed { image: background, x: 0, y: 0, w: w as i32, h: h as i32 },
        up: button(a, dir, "逆", 898, 168, false)?,
        down: button(a, dir, "進", 898, 303, false)?,
        close: button(a, dir, "退出", close_x, close_y, false)?,
        knob: strip(a, &["001-スクロール", "002-スクロール"], 860, 461)?,
        knob_y: (50, 461),
        voice_icon: strip(a, &["001-スピーカ", "002-スピーカ", "003-スピーカ"], 102, 0)?,
        text_x,
        text_y: if en { 22 } else { 49 },
        row_step: if en { 34 } else { 30 },
        rows: if en { 14 } else { 16 },
        columns,
        font_size: 20,
        colour: "#000000".into(),
        fade_ms: 300,
        outline: None,
        row_width: en.then_some(570),
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
    // src: sys_save.nss ($PreSave*, $Save*_plus, $Gyou*, $SaveNew*_plus, SetFont 18/20, SaveDialog, SaveFade)
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
        // src: shot save-hover-ruby-slot2 (columns' ink from x 929, 899, 874: 25 apart)
        chapter: text(929, 41, 20, 1, 24, 0, "#000000", true),
        line: GridText { line_gap: 7, ..text(898, 45, 18, 3, 20, -1, "#000000", true) },
        saved,
        fade_ms: 300,
        across: false,
        tabs: Vec::new(),
        popup: None,
        empty: None,
        quick_page: false,
        play_time_prefix: String::new(),
        play_time_hm: false,
    })
}

/// The right-click menu (`sys_menu.nss`): sixteen rows (220×20) at x 636
/// from y 89, over a black backdrop (frame included) that fades in over
/// 100 ms before the rows appear 10 ms apart (24-318); closing reverses
/// it (539-577). The letters are the game's shortcuts (`0_boot.nss`
/// 27-55, `SetShortcut`), which work at any click wait; M opens and closes
/// the menu. 好感度記録 runs the favour screen ([`FAVOUR`]).
fn command_menu(a: &mut dyn Assets) -> Option<CommandMenu> {
    let rows = [
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
    ];
    // No pressed picture: filmed, the menu keeps the hover one while the
    // mouse button is held.
    // src: sys_menu.nss ($MenuX 636, $Menu*Y 89 + 20 a row; Fade 100, $menuwait 10); 0_boot.nss SetShortcut
    let items = menu_items(a, "cg/sys/menu", (636, 89), &rows, &[1, 2, 0])?;
    let backdrop = picture(a, "cg/sys/menu/メニュー黒バック.png", 0, 0)?;
    Some(CommandMenu { backdrop, items, fade_ms: 100, step_ms: 10, key: Some('M'), pressed: false })
}

/// The English port's save and load screen (`sys_saveload.nut`, its own
/// design, one screen for both): the page's picture (`ナンバリング001`-`004`,
/// the slots' numbers; the quick page takes 001) with `Slots BG` at 43,99
/// under the thumbnails (158×90, 5×3 from 42,101, 195 and 128 apart),
/// `Slots-Grid` (42,101) over them; `未選択時` in an empty slot;
/// `MouseOverBoard` at -3,-3 under the pointer; `new.png` at +126,+68 on
/// the newest. Tabs hang from the top at x 157, 299, 441, 583 and 725 (the
/// quick saves), the current one in its pressed picture; Exit at 0,0. A
/// click on a slot opens `menu_window.png` over it with Save, Load, Delete
/// and Lock/Unlock (`SaveCOME1`-`4` at 213,22 / 295,21 / 295,68 / 213,68,
/// taken as from the window's corner at 213,21); `locked.png` at -2,-2 on
/// a locked slot. The play time ("PLAY TIME …", 18 px white) at 25,550,
/// the last line at 339,511 (15 px black, 340×100, 18 apart). Pages
/// cross-fade over 300 ms. The port shows a slot's details once it is
/// clicked; here under the pointer.
fn save_grid_en(a: &mut dyn Assets) -> Option<SaveGrid> {
    const DIR: &str = "en/cg/sys/save";
    // src: sys_saveload.nut main (PreSave*, Save*_plus, Page1-5, SaveNew*_plus, SaveTotal*, SaveComment*, SaveCOME1-4)
    let slots_bg = a.image(&format!("{DIR}/Slots BG.png"))?.0;
    let mut pages = Vec::new();
    for n in [1, 2, 3, 4, 1] {
        let number = a.image(&format!("{DIR}/ナンバリング{n:03}.png"))?.0;
        pages.push(a.composite(&[(number, 0, 0, 1000), (slots_bg.clone(), 43, 99, 1000)])?.0);
    }
    let picture = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(&format!("{DIR}/{name}.png"))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    // At rest, under the pointer, disabled (or current), pressed.
    let button = |a: &mut dyn Assets, names: [&str; 4], x: i32, y: i32| {
        let names = names.map(|n| format!("{DIR}/{n}.png"));
        let (image, w, h) = a.strip(&names.each_ref().map(|n| n.as_str()))?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let menu = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let [normal, over, no, click] = ["normal", "RO", "no", "click"].map(|s| format!("menu_{name}_{s}"));
        button(a, [&normal, &over, &no, &click], x, y)
    };
    let mut tabs = Vec::new();
    for (x, normal, other) in [(157, "1-15", "1-15"), (299, "16-30", "16-30"), (441, "31-45", "31-45"), (583, "46-60", "45-60"), (725, "_quick", "_quick")] {
        let name = |range: &str, state: &str| format!("pages{range}_button_number_{state}");
        let (n, o, c) = (name(normal, "normal"), name(other, "mouseover"), name(other, "click"));
        tabs.push(button(a, [&n, &o, &c, &c], x, 0)?);
    }
    let none = |a: &mut dyn Assets| Placed { image: a.transparent(), x: -100, y: -100, w: 1, h: 1 };
    let text = |x: i32, y: i32, font_size: u32, lines: u32, chars: u32, line_gap: u32, colour: &str| GridText {
        x,
        y,
        font_size,
        lines,
        chars,
        line_gap,
        char_gap: 0,
        colour: colour.into(),
        vertical: false,
    };
    let saved = centred(a, "en/cg/sys/dialog/記憶をしました.png");
    Some(SaveGrid {
        save_pages: pages.clone(),
        load_pages: pages,
        frames: picture(a, "Slots-Grid", 42, 101),
        first: (42, 101),
        row_step: 128,
        column_step: 195,
        rows: 3,
        columns: 5,
        thumb_w: 158,
        thumb_h: 90,
        hover: picture(a, "MouseOverBoard", 0, 0)?,
        hover_offset: (-3, -3),
        new_mark: picture(a, "new", 0, 0)?,
        new_offset: (126, 68),
        previous: none(a),
        next: none(a),
        delete: none(a),
        delete_on: none(a),
        close: button(a, ["EXITボタン_bt_exit_off", "EXITボタン_bt_exit_over", "EXITボタン_bt_exit_off", "EXITボタン_bt_exit_on"], 0, 0)?,
        // guess: thumbnails 158×90 (Slots-Grid's cells), page fade 300
        // The port has no larger picture, date or chapter.
        preview: Rect { x: -2000, y: -2000, w: 158, h: 90 },
        date: text(-2000, -2000, 18, 0, 0, 0, "#000000"),
        play_time: text(25, 550, 18, 1, 24, 0, "#ffffff"),
        chapter: text(-2000, -2000, 18, 0, 0, 0, "#000000"),
        line: text(339, 511, 15, 5, 22, 3, "#000000"),
        saved,
        fade_ms: 300,
        across: true,
        tabs,
        popup: Some(SlotPopup {
            window: picture(a, "menu_window", 0, 0)?,
            save: menu(a, "save", 0, 1)?,
            load: menu(a, "load", 82, 0)?,
            delete: menu(a, "delete", 82, 47)?,
            lock: menu(a, "lock", 0, 47)?,
            unlock: menu(a, "unlock", 0, 47)?,
            locked: picture(a, "locked", -2, -2)?,
            empty_saves: true,
        }),
        empty: picture(a, "未選択時", 0, 0),
        quick_page: true,
        play_time_prefix: "PLAY TIME ".into(),
        play_time_hm: true,
    })
}

/// The English port's right-click menu (`sys_menu.nut`, from its own
/// `cg.npk`): fourteen rows (220×20) at x 637 from y 90, with no 選択肢に
/// 戻る; セーブ opens the save screen, クイッセーブ and クイックロード take
/// 栞を差す's and 栞を開く's places (no ロード). Backdrop and rows fade in
/// together over 300 ms (the rows through メニューエフェクト, a wipe left
/// out here; both fade out together too). Its shortcuts (`IsKeyDown` in `main`): S quick save, L
/// quick load, D the save screen; the rest as in the Japanese.
fn command_menu_en(a: &mut dyn Assets) -> Option<CommandMenu> {
    let rows = [
        ("キャンセル", BarAction::Close, None),
        ("好感度記録", BarAction::Call, Some('H')),
        ("メッセージ消す", BarAction::HideWindow, None),
        ("自動メッセージ", BarAction::Auto, Some('A')),
        ("スキップ", BarAction::Skip, None),
        ("超速スキップ", BarAction::FastSkip, Some('N')),
        ("ログ表示", BarAction::Log, Some('B')),
        ("セーブ", BarAction::Save, Some('D')),
        ("クイッセーブ", BarAction::QuickSave, Some('S')),
        ("クイックロード", BarAction::QuickLoad, Some('L')),
        ("フルスクリーン", BarAction::Fullscreen, Some('F')),
        ("設定画面", BarAction::Settings, Some('C')),
        ("タイトルに戻る", BarAction::Title, Some('T')),
        ("ゲーム終了", BarAction::Quit, None),
    ];
    // src: sys_menu.nut (637, 90; Fade 300); main IsKeyDown
    let items = menu_items(a, "en/cg/sys/menu", (637, 90), &rows, &[1, 2, 0, 3])?;
    let backdrop = picture(a, "en/cg/sys/menu/メニュー黒バック.png", 0, 0)?;
    Some(CommandMenu { backdrop, items, fade_ms: 300, step_ms: 0, key: Some('M'), pressed: true })
}

/// The menu's rows, 20 apart from `at`, each a strip of its `states`
/// pictures (`001-<name>.png`, …).
fn menu_items(a: &mut dyn Assets, dir: &str, at: (i32, i32), rows: &[(&str, BarAction, Option<char>)], states: &[u32]) -> Option<Vec<CommandItem>> {
    let mut items = Vec::new();
    for (n, &(name, action, key)) in rows.iter().enumerate() {
        let parts: Vec<String> = states.iter().map(|s| format!("{dir}/{s:03}-{name}.png")).collect();
        let (image, w, h) = a.strip(&parts.iter().map(String::as_str).collect::<Vec<_>>())?;
        // src: sys_menu.nss $Menu*Y (20 apart)
        let button = Placed { image, x: at.0, y: at.1 + 20 * n as i32, w: w as i32, h: h as i32 };
        let label = (action == BarAction::Call).then(|| FAVOUR.to_string());
        items.push(CommandItem { button, action, key, label });
    }
    Some(items)
}

fn picture(a: &mut dyn Assets, path: &str, x: i32, y: i32) -> Option<Placed> {
    let (image, w, h) = a.image(path)?;
    Some(Placed { image, x, y, w: w as i32, h: h as i32 })
}

/// The quick bar (`function_quick.nss`): a base (992,26) and four buttons
/// at x 998, off the right edge until the pointer passes x 968, then
/// sliding in over 200 ms (`Dxl2`, 272-286). Quick save plays しおり; quick
/// load fades to black over 300 ms (208-210). The original runs it as a
/// process, so it also slides while text is being drawn; here it moves
/// at click waits only. The English port says クイックセーブ完了 after a
/// quick save until a click (`sys_quicksave.nut`, its second way; the
/// other closes on its own); not yet compared with the original.
fn quick_bar(a: &mut dyn Assets, en: bool) -> Option<ButtonBar> {
    const DIR: &str = "cg/sys/quick";
    // src: function_quick.nss ($Quick*X/Y, QuickBase 992,26, hidden at 1030, $PosX 968, Move 200 Dxl2, Fade BLACK 300)
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
        quick_saved: if en { centred(a, "en/cg/sys/dialog/クイックセーブ完了.png") } else { None },
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
/// `look` places the pictures: the Japanese or the English port's.
fn title(a: &mut dyn Assets, story: &Story, look: &TitleLook) -> Option<TitleScreen> {
    let dir = look.dir;
    // src: 0_boot.nss TitleSet (タイトル版権 50,540; $TitleNutName; $Title*SE; mbgm04), TitleFade, title start 1099-1125 (SetVolume 3000, Fade 1000, Wait 2000)
    let mut parts = Vec::new();
    for (name, x, y) in [("偽富士背景画像", 0, 0), ("ロゴ画像", look.logo.0 as i64, look.logo.1 as i64), ("コピーライト", 50, 540)] {
        parts.push((a.image(&format!("{dir}/{name}.png"))?.0, x, y, 1000));
    }
    let background = a.screen(&parts)?;
    // The fire (ef046_炎a, 2x, OverlayRender) glows from nothing to 300
    // over 600 ms, holds, and back, each 1100 ms (CreateTPO1 in
    // 0_boot.nss's TitleSet).
    // The openings end with the fire at 300 (at launch, partway through
    // its glow) or at nothing (on coming back).
    let mut openings = Vec::new();
    let launch_sound = a.delayed_sound("sound/se/se特殊_雰囲気_鎧登場演02", 10200);
    let mut movies = std::collections::HashMap::new();
    for (picture, when, sets, random) in launch_pictures(story) {
        if !movies.contains_key(picture) {
            let name = format!("title_launch2_{}", &picture[6..11]);
            let m = super::opening::at_launch(a, look, picture, &name);
            movies.insert(picture, m);
        }
        let Some(movie) = movies[picture].clone() else { continue };
        // src: 0_boot.nss CreateTPO1(…, 600, 300, 1100, 2), 1000 ms into its loop as the opening ends
        let glow_keys = Some(vec![(0, 256), (100, 256), (700, 0), (1200, 0), (1800, 256), (2200, 256)]);
        openings.push(TitleOpening { movie, at_launch: true, sound: launch_sound.clone(), glow_keys, when, sets, random, ..Default::default() });
    }
    if let Some(movie) = super::opening::on_return(a, look) {
        let sound = a.delayed_sound("sound/se/se特殊_雰囲気_鎧登場演02", 1800);
        openings.push(TitleOpening { movie, sound, ..Default::default() });
    }
    // The music goes on; the sword's clash with the white.
    if let Some(movie) = super::opening::after_extras(a, look) {
        let sound = a.sound("sound/se/se戦闘_攻撃_刀衝突01");
        openings.push(TitleOpening { movie, sound, after_extras: true, ..Default::default() });
    }
    // src: 0_boot.nss CreateTPO1(…, 600, 300, 1100, 2)
    let glow = a.overlaid(&background, "cg/ef/ef046_炎a.jpg", 2000, 300).map(|image| TitleGlow {
        image,
        keys: vec![(0, 0), (600, 256), (1100, 256), (1700, 0), (2200, 0)],
    });
    let se = |a: &mut dyn Assets, name: &str| a.sound(&format!("sound/se/{name}"));
    let mut buttons = Vec::new();
    let actions = [
        (TitleAction::Continue, Some("se特殊_鎧_装着03")),
        (TitleAction::Start, Some("se特殊_鎧_装着03")),
        (TitleAction::Load, Some("se戦闘_動作_刀構え01")),
        (TitleAction::Settings, Some("se戦闘_動作_刀構え01")),
        (TitleAction::Extras, Some("se特殊_鎧_装着04")),
        (TitleAction::Exit, None),
    ];
    for (&(name, x, y), (action, click)) in look.buttons.iter().zip(actions) {
        let state = |n: u32| format!("{dir}/{name}-{n:03}.png");
        let (s1, s2, s3) = (state(1), state(2), state(3));
        let cells: Vec<&str> = if action == TitleAction::Unavailable { vec![&s1] } else { vec![&s1, &s2, &s3] };
        let (image, _, _) = a.strip(&cells)?;
        let click_se = click.and_then(|c| se(a, c));
        buttons.push(TitleButton { image, x: x as i32, y: y as i32, action, click_se, shown_by: None });
    }
    // ライナー画像 (`0_boot.nss`), once 悪鬼編 is over: the staff comments
    // on Nitroplus's site (`System("OPEN:...")`).
    if let Some(&cleared) = story.flags.get("#悪鬼編終了") {
        let state = |n: u32| format!("{dir}/ライナー画像-{n:03}.png");
        let (image, _, _) = a.strip(&[&state(1), &state(2), &state(3)])?;
        let action = TitleAction::Link(look.notes);
        buttons.push(TitleButton { image, x: 60, y: 524, action, click_se: se(a, "se戦闘_動作_刀構え01"), shown_by: Some(cleared) });
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
        glow,
        openings,
        hide_without_saves: look.hides,
        extras_shown_by: if look.hides { ["#英雄編終了", "#復讐編終了"].iter().filter_map(|f| story.flags.get(*f).copied()).collect() } else { Vec::new() },
    })
}

/// A launch picture and its [`TitleOpening`] fields: the file, `when`,
/// `sets`, `random`.
type LaunchPicture = (&'static str, Vec<(Var, i32)>, Vec<Var>, bool);

/// The launch opening's picture, as `TitleSet` picks it: a route's own the
/// first time after clearing it (悪鬼編, then 魔王編's while 悪鬼編 isn't
/// cleared, 英雄編, 復讐編; each marked `#…通過` once shown), else one at
/// random among the cleared routes' and 村正鎮座. Rules whose flags the
/// story lacks are left out.
fn launch_pictures(story: &Story) -> Vec<LaunchPicture> {
    let flag = |route: &str, what: &str| story.flags.get(&format!("#{route}{what}")).copied();
    let (akki, maou, eiyuu, fukushuu) = ("cg/ev/ev267_真生悪鬼ＥＤ_a.jpg", "cg/ev/ev501_村正鎮座.jpg", "cg/ev/ev165_最終正宗_a.jpg", "cg/ev/ev004_香奈枝の凶眼_a.jpg");
    let mut out = Vec::new();
    for (picture, when, sets) in [
        (akki, [("悪鬼編", "終了", 1), ("悪鬼編", "通過", 0)], "悪鬼編"),
        (maou, [("魔王編", "終了", 1), ("悪鬼編", "終了", 0)], "魔王編"),
        (eiyuu, [("英雄編", "終了", 1), ("英雄編", "通過", 0)], "英雄編"),
        (fukushuu, [("復讐編", "終了", 1), ("復讐編", "通過", 0)], "復讐編"),
    ] {
        let when: Option<Vec<(Var, i32)>> = when.iter().map(|(r, w, v)| flag(r, w).map(|f| (f, *v))).collect();
        if let (Some(when), Some(set)) = (when, flag(sets, "通過")) {
            out.push((picture, when, vec![set], false));
        }
    }
    for (picture, route) in [(eiyuu, "英雄編"), (fukushuu, "復讐編"), (akki, "悪鬼編")] {
        if let Some(done) = flag(route, "終了") {
            out.push((picture, vec![(done, 1)], Vec::new(), true));
        }
    }
    out.push((maou, Vec::new(), Vec::new(), true));
    out
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
    // src: sys_close.nss (MSG_yes 515,397, MSG_no 471,397, MSG_bak 800, Zoom/Fade 150 Dxl1; close 200)
    let question = |a: &mut dyn Assets, name: &str| centred(a, &format!("{DIR}/{name}.png"));
    let mut pictures = Vec::new();
    for (kind, name) in [
        (Switch::ConfirmQuit, "ゲームを終了しますか"),
        (Switch::ConfirmTitle, "タイトルへ戻りますか"),
        (Switch::ConfirmSave, "記録を上書きして良いか"),
        (Switch::ConfirmLoad, "この記録を呼び出しますか"),
        (Switch::ConfirmDelete, "記録を削除してよろしいか"),
        (Switch::ConfirmBackToChoice, "選択肢まで戻りますか"),
        (Switch::ConfirmReset, "設定を初期状態に戻す"),
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
        // Zoom x 0 to 1000 in 150 ms (Dxl1) about the question's centre
        // line (the buttons' vertices lie within 13 px of it); closing,
        // back in 150 ms while fading out in 200 ms (sys_close.nss).
        unroll: Some(Unroll { x: 512, close_ms: 200, y: None }),
    })
}

/// The English port's yes/no questions (`sys_dialog.nut`,
/// `CreateDialogImage_YesNo`): the question's picture (400×150) in the
/// middle of the screen over black at 800/1000, Yes and No at 108,100 and
/// 230,100 from its corner, linked to it; it turns open about its middle
/// line from edge-on (`Rotate` x -90 to 0) over 150 ms as the buttons and
/// backdrop fade in, and shut (to 90) as they go.
fn confirm_en(a: &mut dyn Assets) -> Option<ConfirmDialog> {
    const DIR: &str = "en/cg/sys/dialog";
    // src: sys_dialog.nut CreateDialogImage_YesNo (yes 108,100, no 230,100; 150 ms, backdrop 800)
    let mut pictures = Vec::new();
    for (kind, name) in [
        (Switch::ConfirmQuit, "終了しますか"),
        (Switch::ConfirmTitle, "タイトル画面へ"),
        (Switch::ConfirmSave, "この記憶を上書き"),
        (Switch::ConfirmLoad, "記憶呼び出し"),
        (Switch::ConfirmDelete, "セーブ削除"),
        (Switch::ConfirmBackToChoice, "選択肢まで"),
        (Switch::ConfirmReset, "初期設定"),
        (Switch::ConfirmQuickLoad, "クイックロード"),
    ] {
        pictures.push((kind, centred(a, &format!("{DIR}/{name}.png"))?));
    }
    let base = pictures[0].1.clone();
    let button = |a: &mut dyn Assets, name: &str, dx: i32| {
        let state = |n: u32| format!("{DIR}/{name}-{n:03}.png");
        let (normal, hover, pressed) = (state(1), state(2), state(3));
        let (image, w, h) = a.strip(&[&normal, &hover, &normal, &pressed])?;
        Some(Placed { image, x: base.x + dx, y: base.y + 100, w: w as i32, h: h as i32 })
    };
    let black = a.color(0, SCREEN_W as u32, SCREEN_H as u32);
    Some(ConfirmDialog {
        yes: button(a, "yes", 108)?,
        no: button(a, "no", 230)?,
        unroll: Some(Unroll { x: 0, close_ms: 150, y: Some(base.y + base.h / 2) }),
        base,
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
