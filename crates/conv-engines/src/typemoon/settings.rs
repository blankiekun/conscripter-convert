//! 月姫's settings: the system menu's Config tab (`FUN_0012c810` runs it,
//! `FUN_0012aa80` draws it, `FUN_0012a550` reads the values into it), from
//! `allui`'s `CONF_PARTS` on the shared paged settings ([`PagedSettings`]).
//!
//! The original: tabs システム設定, サウンド設定, 操作説明 over a panel
//! that scrolls (the clip 1402 × 858 at (407, 165) of 1080p); each row a
//! slider (− + and a knob, 0-250 ms by 1, 0-10000 ms by 50, 0-100 % by
//! 1), check boxes and options stacked one a row. Ours: the System tab
//! in two pages turned with ▲ ▼ (the panel is 1336 px tall), the Sound
//! tab without the characters' voices (the next item, T2), 操作説明 the
//! manual picture. Live: the unread text speed, the auto wait, skip all
//! or read only, the music, sound and system sound volumes, the music and
//! sound mutes, all voices on or off, a click stopping the voice, the
//! language (ours, with both languages). Drawn as the exe's defaults,
//! with no effect (no engine feature yet): the read text speed and its
//! sample, the two "whole page at once" boxes, the auto page wait (3000
//! ms; the engine has one auto wait), the read scenes' auto skip and its
//! question box, the text fade-ins (unread, read), the system sound mute,
//! the SE test, the values' digits (`config_number`). Left out: the
//! unused `effect_bg` (no page draws it), the 推奨設定 question.

use std::path::Path;

use anyhow::{Context, Result};
use conv_core::ir::{PageTab, PagedSettings, Placed, RowControl, Setting, SettingRow, SettingValue, SettingsMenu, SettingsPage, Switch, Var};
use image::{imageops, GenericImageView, Rgba, RgbaImage};

use super::title::{px, Parts};
use crate::labels::Labels;

const DIR: &str = "sys/conf";

/// A point of the 640 × 448 stage in 1080p pixels.
fn hd(x: f32, y: f32) -> (i32, i32) {
    ((x * 3.0).round() as i32, (y * 1080.0 / 448.0).round() as i32)
}

/// 1080p to the screen.
fn sc((x, y): (i32, i32)) -> (i32, i32) {
    (px(x as f32), px(y as f32))
}

// src: FUN_0012aa80 (the stage, 1/3 × 448/1080 of the parts): the panel's
// clip 0x197, 0xA5, 0x57A × 0x35A (1080p); config_innerbg at (120, 0),
// tab_bg at (133.33, 39.82), the tabs (widgets 0x33-0x35) at x 136, 218.67,
// 296 and 推奨設定 (0x36) at 512, y 39.82; the pages' panels at
// (128.33, 68.03) (FUN_0012d570, FUN_0012d980; voice_bg at y 199.94), the
// manual at (133.33, 73.0)
const CLIP: (i32, i32, i32, i32) = (407, 165, 1402, 858);
const PANEL: (f32, f32) = (128.333, 68.03);
const VOICE_Y: f32 = 199.941;

// src: the pages' tables at 0x56CDB0 (0x9D0 bytes a page, +0x488 the
// controls' top left on the stage, unscrolled; +0x280 their widgets)
const UNREAD: [(f32, f32); 4] = [(245.0, 100.8), (120.667, 56.0), (452.0, 100.8), (475.0, 94.163)];
const READ: [(f32, f32); 4] = [(245.0, 175.052), (120.667, 130.252), (452.0, 175.052), (475.0, 168.415)];
const AUTO_PAGE: [(f32, f32); 3] = [(245.0, 246.4), (120.667, 201.6), (452.0, 246.4)];
const AUTO_CLICK: [(f32, f32); 3] = [(245.0, 306.548), (120.667, 261.748), (452.0, 306.548)];
/// The read scenes' auto skip ON (and its question box), OFF.
const SCENE_SKIP: [(f32, f32); 3] = [(243.667, 364.622), (290.333, 360.059), (243.667, 384.948)];
/// メッセージの早送り 全文, 既読のみ; the text fade-ins (unread, read) フェードイン, なし.
const FAST: [(f32, f32); 2] = [(243.667, 427.667), (243.667, 448.0)];
const FADE_IN: [(f32, f32); 2] = [(243.667, 491.141), (243.667, 511.467)];
const READ_FADE_IN: [(f32, f32); 2] = [(243.667, 557.096), (243.667, 577.422)];
/// −, knob, +, mute (and the SE test).
const BGM: [(f32, f32); 4] = [(245.333, 104.948), (121.0, 60.148), (452.333, 104.948), (476.667, 99.97)];
const SE: [(f32, f32); 5] = [(245.333, 132.326), (121.0, 87.526), (452.333, 132.326), (476.667, 127.348), (560.667, 128.593)];
const SYS_SE: [(f32, f32); 4] = [(245.333, 160.533), (121.0, 115.733), (452.333, 160.533), (476.667, 155.556)];
/// すべてのボイスの再生 ON, OFF; メッセージ送り時の音声停止 ON, OFF.
const VOICES: [(f32, f32); 4] = [(242.667, 236.444), (242.667, 258.844), (242.667, 297.007), (242.667, 319.407)];
/// src: FUN_00129fa0: the characters' rows (19), row n's line at stage
/// y 370.0148 + 50.607407 n; FUN_0012d980 draws each row's strip of
/// voice_bg 9.956 above it and its face and name (FUN_0012d040) at x 144,
/// 4.978 above it and 24.889 below.
const CHARACTERS: u32 = 19;
const ROW_Y: (f32, f32) = (370.0148, 50.607407);
/// src: exe 0x44D640: voice_bg's header, a row's strip, the foot under the last.
const ROW_STRIP: (u32, u32) = (386, 508);
const ROW_FOOT: (u32, u32) = (508, 527);
/// src: exe 0x44D500 (faces in conf_faceicon) and 0x44D5A0 (names in
/// conf_name): x0, y0, x1, y1 by character, as FUN_00129fa0 picks them
/// once the character is met (cell n for row n).
const FACES: [(u32, u32, u32, u32); 19] = [
    (32, 32, 336, 112), (32, 128, 336, 208), (32, 224, 336, 304), (32, 320, 336, 400), (32, 416, 336, 496), (32, 512, 336, 592), (32, 608, 336, 688),
    (32, 704, 336, 784), (32, 800, 336, 880), (32, 896, 336, 976), (352, 32, 656, 112), (352, 128, 656, 208), (352, 320, 656, 400), (352, 224, 656, 304),
    (352, 416, 656, 496), (352, 512, 656, 592), (352, 608, 656, 688), (352, 704, 656, 784), (352, 896, 656, 976),
];
const NAMES: [(u32, u32, u32, u32); 19] = [
    (32, 32, 168, 80), (32, 96, 168, 144), (32, 160, 168, 208), (32, 224, 168, 272), (32, 288, 168, 336), (32, 352, 168, 400), (32, 416, 168, 464),
    (184, 32, 320, 80), (184, 96, 320, 144), (184, 160, 320, 208), (184, 224, 320, 272), (184, 288, 320, 336), (184, 416, 320, 464), (184, 352, 320, 400),
    (336, 32, 472, 80), (336, 96, 472, 144), (336, 160, 472, 208), (336, 224, 472, 272), (336, 352, 472, 400),
];
/// ours: the rows on pages of their own after the Sound page, 7 a page
/// (the clip is 858 px, a row 122), the first row's strip at the clip's top.
const ROWS_A_PAGE: u32 = 7;
/// src: FUN_0012b930: a slider's knob moves 169.33 (stage) from value 0 to its most.
const TRAVEL: f32 = 169.333;

// The parts' cells (measured from their alpha): normal, under the pointer, lit.
/// btn_base0: システム設定, サウンド設定, 操作説明.
fn tab_cell(row: u32, col: u32) -> (u32, u32, u32, u32) {
    (200 * col, 88 * row, 200, 88)
}
/// btn_base1: ON, OFF, 全文, 既読のみ, フェードイン, なし(瞬間表示), ….
fn option_cell(row: u32, col: u32) -> (u32, u32, u32, u32) {
    (253 * col, 87 * row, 253, 87)
}
/// btn_base2: ページ末まで一気に表示, 確認メッセージを表示する.
fn check_cell(row: u32, col: u32) -> (u32, u32, u32, u32) {
    (301 * col, 76 * row, 301, 76)
}
/// conf_stxt01: −, +.
fn step_cell(plus: bool, col: u32) -> (u32, u32, u32, u32) {
    (84 * u32::from(plus), 46 * col, 84, 46)
}
fn mute_cell(col: u32) -> (u32, u32, u32, u32) {
    (168 * col, 0, 168, 128)
}
/// ours: conf_sbtn's knob (its first row, a glow with a long streak) cut
/// to the glow, 200 × 100 from (368, 76).
const KNOB: (u32, u32, u32, u32) = (368, 76, 200, 100);

/// The system page's panel cut in two (system_bg's section lines, measured):
/// its header bar to 59, page 1 to the line at 839, page 2 from it; the panel's
/// foot (the last 40 rows); the last section (1152-1296, 既読テキスト表示演出)
/// is copied under page 2 for the language.
const HEADER: u32 = 59;
const SPLIT: u32 = 839;
const FOOT: u32 = 40;
const LAST_SECTION: (u32, u32) = (1152, 1296);
/// voice_bg to the line under メッセージ送り時の音声停止 (the characters' rows after it).
const VOICE_CUT: u32 = 363;

/// What a page is drawn from: pictures at 1080p places.
struct Canvas(RgbaImage);

impl Canvas {
    fn new() -> Canvas {
        Canvas(RgbaImage::new(1920, 1080))
    }
    fn put(&mut self, img: &RgbaImage, (x, y): (i32, i32)) {
        imageops::overlay(&mut self.0, img, i64::from(x), i64::from(y));
    }
    /// The page's panel, clipped as the exe clips it.
    fn panel(&mut self, img: &RgbaImage, (x, y): (i32, i32)) {
        let mut clip = RgbaImage::new(CLIP.2 as u32, CLIP.3 as u32);
        imageops::overlay(&mut clip, img, i64::from(x - CLIP.0), i64::from(y - CLIP.1));
        self.put(&clip, (CLIP.0, CLIP.1));
    }
    fn screen(&self) -> RgbaImage {
        imageops::resize(&self.0, px(1920.0) as u32, px(1080.0) as u32, imageops::FilterType::Triangle)
    }
}

fn rows(img: &RgbaImage, from: u32, to: u32) -> RgbaImage {
    imageops::crop_imm(img, 0, from, img.width(), to - from).to_image()
}

/// Pictures stacked top to bottom.
fn stack(parts: &[RgbaImage]) -> RgbaImage {
    let mut out = RgbaImage::new(parts.iter().map(|p| p.width()).max().unwrap_or(1), parts.iter().map(|p| p.height()).sum());
    let mut y = 0;
    for p in parts {
        imageops::replace(&mut out, p, 0, i64::from(y));
        y += p.height();
    }
    out
}

/// Where the screen's pictures come from (`None` in a dry run: the places only).
struct Source<'a> {
    conf: Option<Parts>,
    menu: Option<Parts>,
    output: &'a Path,
}

impl Source<'_> {
    fn get(&self, name: &str) -> Result<RgbaImage> {
        Ok(self.conf.as_ref().context("no pictures in a dry run")?.get(name)?.clone())
    }
    fn cut(&self, name: &str, r: (u32, u32, u32, u32)) -> Result<RgbaImage> {
        Ok(imageops::crop_imm(&self.get(name)?, r.0, r.1, r.2, r.3).to_image())
    }
    fn save(&self, file: &str, img: &RgbaImage) -> Result<()> {
        let dest = self.output.join(file);
        std::fs::create_dir_all(dest.parent().unwrap())?;
        Ok(img.save(&dest)?)
    }
    /// A button at a 1080p point: `cells` of `name` side by side (the
    /// first's size is the button's).
    fn button(&self, file: &str, name: &str, cells: &[(u32, u32, u32, u32)], at: (i32, i32)) -> Result<Placed> {
        if let Some(c) = &self.conf {
            self.save(&format!("{DIR}/{file}.png"), &c.cells(name, cells)?)?;
        }
        let (x, y) = sc(at);
        Ok(Placed { image: format!("{DIR}/{file}.png"), x, y, w: px(cells[0].2 as f32), h: px(cells[0].3 as f32) })
    }
}

/// A slider of the stage's `points` (−, knob, +) at `dy` 1080p pixels:
/// levels 0-10 left to right, or right to left (`reversed`: the exe's
/// value grows to the right as ours falls), its − and + then swapped.
fn slider(src: &Source, name: &str, value: SettingValue, points: &[(f32, f32)], dy: i32, reversed: bool) -> Result<RowControl> {
    let at = |p: (f32, f32)| {
        let (x, y) = hd(p.0, p.1);
        (x, y + dy)
    };
    let travel = (TRAVEL * 3.0).round() as i32;
    let (kx, ky) = at(points[1]);
    let knob_at = (kx + KNOB.0 as i32 + if reversed { travel } else { 0 }, ky + KNOB.1 as i32);
    let knob = src.button(&format!("{name}_knob"), "conf_sbtn", &[KNOB], knob_at)?;
    let minus = src.button(&format!("{name}_minus"), "conf_stxt01", &[step_cell(false, 0), step_cell(false, 1)], at(points[0]))?;
    let plus = src.button(&format!("{name}_plus"), "conf_stxt01", &[step_cell(true, 0), step_cell(true, 1)], at(points[2]))?;
    let (minus, plus) = if reversed { (plus, minus) } else { (minus, plus) };
    // No fill on these bars: an empty picture.
    let fill = Placed { image: format!("{DIR}/none.png"), ..knob.clone() };
    Ok(RowControl::Slider { value, knob, travel: if reversed { -px(travel as f32) } else { px(travel as f32) }, fill, bar: None, minus, plus })
}

/// Options stacked a row each: (value, btn_base1's row, stage point).
fn choice(src: &Source, name: &str, value: SettingValue, options: &[(i32, u32, (f32, f32))], dy: i32) -> Result<RowControl> {
    let mut out = Vec::new();
    for (i, &(v, row, p)) in options.iter().enumerate() {
        let (x, y) = hd(p.0, p.1);
        let file = format!("{name}{i}");
        let button = src.button(&file, "btn_base1", &[option_cell(row, 0), option_cell(row, 1)], (x, y + dy))?;
        let lit = src.button(&format!("{file}_on"), "btn_base1", &[option_cell(row, 2), option_cell(row, 2)], (x, y + dy))?;
        out.push((v, button, lit.image));
    }
    Ok(RowControl::Choice { value, options: out })
}

/// A part's cell drawn at a stage point (a control drawn as its default).
fn draw(canvas: &mut Canvas, src: &Source, name: &str, cell: (u32, u32, u32, u32), p: (f32, f32), dy: i32) -> Result<()> {
    let (x, y) = hd(p.0, p.1);
    canvas.put(&src.cut(name, cell)?, (x, y + dy));
    Ok(())
}

/// A knob drawn where `value` of `most` puts it.
fn draw_knob(canvas: &mut Canvas, src: &Source, p: (f32, f32), value: f32, most: f32) -> Result<()> {
    draw(canvas, src, "conf_sbtn", (0, 0, 936, 252), (p.0 + TRAVEL * value / most, p.1), 0)
}

/// The steps − + of a slider drawn at rest.
fn draw_steps(canvas: &mut Canvas, src: &Source, minus: (f32, f32), plus: (f32, f32)) -> Result<()> {
    draw(canvas, src, "conf_stxt01", step_cell(false, 0), minus, 0)?;
    draw(canvas, src, "conf_stxt01", step_cell(true, 0), plus, 0)
}

/// The settings screen; `language`: the build has both languages, kept in
/// this global (0 Japanese, 1 English).
pub fn settings(romfs: &Path, output: &Path, font: &[u8], dry_run: bool, language: Option<Var>) -> Result<SettingsMenu> {
    let src = if dry_run { Source { conf: None, menu: None, output } } else { Source { conf: Some(Parts::read(romfs, "CONF_PARTS")?), menu: Some(Parts::read(romfs, "MENU_PARTS")?), output } };
    let labels = if dry_run { None } else { Some(Labels::new(font).context("the font doesn't load")?) };
    let shift = |page2: bool| if page2 { HEADER as i32 - SPLIT as i32 } else { 0 };
    let sw = SettingValue::Switch;

    // The live controls.
    let page1 = vec![
        slider(&src, "unread", SettingValue::Level(Setting::TextSpeed), &UNREAD, 0, true)?,
        slider(&src, "auto", SettingValue::Level(AUTO), &AUTO_CLICK, 0, true)?,
    ];
    let mut page2 = vec![choice(&src, "fast", SettingValue::SkipUnread, &[(1, 2, FAST[0]), (0, 3, FAST[1])], shift(true))?];
    // ours: the language's section under the last one, its options where
    // that section has its own.
    let lang_top = (HEADER + LAST_SECTION.1 - SPLIT) as i32 + hd(0.0, PANEL.1).1;
    let lang_points = |i: usize| {
        let (x, y) = hd(READ_FADE_IN[i].0, READ_FADE_IN[i].1);
        (x, y - hd(0.0, PANEL.1).1 - LAST_SECTION.0 as i32 + lang_top)
    };
    if let Some(var) = language {
        let mut options = Vec::new();
        for (i, word) in ["日本語", "English"].into_iter().enumerate() {
            let at = sc(lang_points(i));
            let file = |n: &str| format!("{DIR}/lang{i}{n}.png");
            let size = (px(253.0), px(87.0));
            if let Some(labels) = &labels {
                let cells = [0, 1, 2].map(|c| option_word(&src, labels, word, c)).into_iter().collect::<Result<Vec<_>>>()?;
                src.save(&file(""), &side_by_side(&[&cells[0], &cells[1]]))?;
                src.save(&file("_on"), &side_by_side(&[&cells[2], &cells[2]]))?;
            }
            options.push((i as i32, Placed { image: file(""), x: at.0, y: at.1, w: size.0, h: size.1 }, file("_on")));
        }
        page2.push(RowControl::Choice { value: SettingValue::Language(var), options });
    }
    let sound = vec![
        slider(&src, "bgm", SettingValue::Level(Setting::BgmVolume), &BGM, 0, false)?,
        slider(&src, "se", SettingValue::Level(Setting::SeVolume), &SE, 0, false)?,
        slider(&src, "sys", SettingValue::Level(Setting::SystemVolume), &SYS_SE, 0, false)?,
        mute(&src, "bgm_mute", sw(Switch::BgmOn), BGM[3], 0)?,
        mute(&src, "se_mute", sw(Switch::SeOn), SE[3], 0)?,
        choice(&src, "voices", sw(Switch::VoiceOn), &[(1, 0, VOICES[0]), (0, 1, VOICES[1])], 0)?,
        // ON: the click stops the voice.
        choice(&src, "stop", sw(Switch::VoiceKeepsPlaying), &[(0, 0, VOICES[2]), (1, 1, VOICES[3])], 0)?,
    ];

    // The characters' voices on or off (the user's choice for the exe's
    // volume sliders and mute boxes), on pages after the Sound page; a
    // voice's character is its row (build.rs).
    let mut voice_pages: Vec<Vec<RowControl>> = Vec::new();
    for n in 0..CHARACTERS {
        if n % ROWS_A_PAGE == 0 {
            voice_pages.push(Vec::new());
        }
        let y = ROW_Y.0 + ROW_Y.1 * n as f32;
        voice_pages.last_mut().unwrap().push(mute(&src, "voice_mute", SettingValue::Voices(vec![n]), (476.333, y - 4.978), row_shift(n))?);
    }
    let voice_count = voice_pages.len();

    // The pages' pictures.
    let bases = labels.as_ref().map(|l| bases(&src, l, language.is_some())).transpose()?;
    let mut pages = Vec::new();
    let all: Vec<Vec<RowControl>> = [page1, page2, sound].into_iter().chain(voice_pages).chain([Vec::new()]).collect();
    for (i, rows) in all.into_iter().enumerate() {
        let file = format!("{DIR}/page{}.png", i + 1);
        if let Some(b) = &bases {
            src.save(&file, &b[i])?;
        }
        pages.push(SettingsPage { base: Placed { image: file, x: 0, y: 0, w: px(1920.0), h: px(1080.0) }, rows });
    }
    if !dry_run {
        src.save(&format!("{DIR}/none.png"), &RgbaImage::new(1, 1))?;
    }

    let mut tabs = Vec::new();
    let manual = 3 + voice_count;
    for (row, x, pages) in [(0, 136.0, (0, 1)), (1, 218.667, (2, manual - 1)), (2, 296.0, (manual, manual))] {
        let button = src.button(&format!("tab{row}"), "btn_base0", &[tab_cell(row, 0), tab_cell(row, 1)], hd(x, 39.822))?;
        let selected = src.button(&format!("tab{row}_on"), "btn_base0", &[tab_cell(row, 2)], hd(x, 39.822))?;
        tabs.push(PageTab { button, selected: selected.image, pages });
    }
    // ours: ▲ ▼ at the panel's right edge (the exe scrolls), 閉じる under
    // the menu bar's buttons (the exe closes with B)
    let word = |file: &str, text: &str, size: f32, (x, y): (i32, i32)| -> Result<Placed> {
        let (mut w, mut h) = (0, 0);
        if let Some(l) = &labels {
            let cells = [l.draw(text, size, [255, 255, 255], [40, 70, 160], 255), l.draw(text, size, [200, 250, 255], [0, 150, 255], 255)];
            (w, h) = (cells[0].width() as i32, cells[0].height() as i32);
            src.save(&format!("{DIR}/{file}.png"), &side_by_side(&[&cells[0], &cells[1]]))?;
        }
        Ok(Placed { image: format!("{DIR}/{file}.png"), x, y, w, h })
    };
    let edge = px((CLIP.0 + CLIP.2) as f32) - 44;
    let prev = word("up", "▲", 26.0, (edge, px(CLIP.1 as f32) + 40))?;
    let next = word("down", "▼", 26.0, (edge, px((CLIP.1 + CLIP.3) as f32) - 40))?;
    let close = word("close", "閉じる", 22.0, sc(hd(40.0, 43.556 + 31.111 * 5.0 + 4.0)))?;
    let defaults = src.button("default", "btn_default", &[(0, 0, 352, 104), (0, 104, 352, 104)], hd(512.0, 39.822))?;
    // The menu bar's Title (the exe's own tab): back to the title, in game.
    let title = src.menu.as_ref().map(|m| m.cells("menu_btn_ja", &[(24, 26 + 104 * 4, 240, 104), (264, 26 + 104 * 4, 240, 104)])).transpose()?;
    if let Some(t) = &title {
        src.save(&format!("{DIR}/title.png"), t)?;
    }
    let (tx, ty) = sc(hd(26.667, 43.556 + 31.111 * 4.0));
    let title = Placed { image: format!("{DIR}/title.png"), x: tx, y: ty, w: px(240.0), h: px(104.0) };

    let dummy = Placed { image: String::new(), x: 0, y: 0, w: 0, h: 0 };
    Ok(SettingsMenu {
        base: dummy.clone(),
        base_opacity: 255,
        header: dummy.clone(),
        close: dummy,
        // What the auto wait applies (the pages draw the rows).
        rows: vec![SettingRow { label: String::new(), setting: AUTO }],
        label_x: 0,
        minus_x: 0,
        level_x: 0,
        plus_x: 0,
        y: 0,
        pitch: 0,
        font_size: 0,
        title_label: String::new(),
        quit_label: String::new(),
        exits_y: 0,
        tab_labels: Vec::new(),
        tabs_x: 0,
        tabs_y: 0,
        voices: Vec::new(),
        voice_column_x: 0,
        voice_pitch: 0,
        skip_unread_label: Some(String::new()),
        switches: Vec::new(),
        pictures: Some(Box::new(PagedSettings { pages, prev, next, close, defaults, title: Some(title), quit: None, tabs })),
        // src: exe 0x56B080 (the reset in FUN_0012c810 case 8): BGM +4 40,
        // SE +8 40, system SE +0xA 10 (%); a voice at its character's 50 %
        // (+0x34…) of all voices' +6 100; unread text +0xE 35 ms, the auto
        // wait at a click +0x1C 2200 ms; skip read text only (+0x1E 1); a
        // click stops the voice (+0x2E 1)
        defaults: vec![(Setting::BgmVolume, 4), (Setting::SeVolume, 4), (Setting::SystemVolume, 1), (Setting::VoiceVolume, 5), (Setting::TextSpeed, 9), (AUTO, 8)],
        switches_off: vec![Switch::VoiceKeepsPlaying],
        text_ms: Some(TEXT_MS),
    })
}

/// The auto wait at a click wait: (10 − level) × 1100 ms, 2200 at level 8
/// (the exe: 0-10000 ms by 50).
const AUTO: Setting = Setting::AutoWait { page_step: 1100, char_step: 0 };
/// The unread text speed's levels (the exe: 0-250 ms by 1, 35 the
/// default): 25 ms a level, 35 at level 9.
const TEXT_MS: [u32; 11] = [250, 225, 200, 175, 150, 125, 100, 75, 50, 35, 0];

/// A mute box at a stage point `dy` 1080p pixels lower: checked while the
/// value is off.
fn mute(src: &Source, name: &str, value: SettingValue, p: (f32, f32), dy: i32) -> Result<RowControl> {
    let (x, y) = hd(p.0, p.1);
    let button = src.button(name, "btn_mute", &[mute_cell(0), mute_cell(1)], (x, y + dy))?;
    let checked = src.button(&format!("{name}_on"), "btn_mute", &[mute_cell(2), mute_cell(2)], (x, y + dy))?;
    Ok(RowControl::Toggle { value, on: 0, button, checked: checked.image })
}

/// How far (1080p) a character's row moves up from the exe's unscrolled
/// panel to its page: its page's first row's strip at the clip's top.
fn row_shift(n: u32) -> i32 {
    let first = n / ROWS_A_PAGE * ROWS_A_PAGE;
    CLIP.1 - hd(0.0, VOICE_Y).1 - ROW_STRIP.0 as i32 - hd(0.0, ROW_Y.1 * first as f32).1
}

fn side_by_side(cells: &[&RgbaImage]) -> RgbaImage {
    let mut out = RgbaImage::new(cells.iter().map(|c| c.width()).sum(), cells.iter().map(|c| c.height()).max().unwrap_or(1));
    let mut x = 0;
    for c in cells {
        imageops::replace(&mut out, *c, i64::from(x), 0);
        x += c.width();
    }
    out
}

/// An option of ours in btn_base1's look (cell `col`: normal, under the
/// pointer, lit): its bullet (ON's first 44 px) and the word in the font,
/// in the colour of ON's letters, at the screen's size.
fn option_word(src: &Source, labels: &Labels, word: &str, col: u32) -> Result<RgbaImage> {
    let (x, y, w, h) = option_cell(0, col);
    let cell = src.cut("btn_base1", (x, y, w, h))?;
    let mut out = RgbaImage::new(w, h);
    imageops::replace(&mut out, &imageops::crop_imm(&cell, 0, 0, 44, h).to_image(), 0, 0);
    // The letters' colour: the mean of ON's opaque letter pixels.
    let ink: Vec<_> = imageops::crop_imm(&cell, 46, 0, w - 46, h).pixels().filter(|(_, _, p)| p[3] > 230).map(|(_, _, p)| p).collect();
    let mean = |c: usize| (ink.iter().map(|p| u32::from(p[c])).sum::<u32>() / ink.len().max(1) as u32) as u8;
    let glow = if col == 0 { [20, 30, 60] } else { [0, 140, 255] };
    let text = labels.draw(word, 32.0, [mean(0), mean(1), mean(2)], glow, 255);
    imageops::overlay(&mut out, &text, 46, (i64::from(h) - i64::from(text.height())) / 2);
    Ok(imageops::resize(&out, px(w as f32) as u32, px(h as f32) as u32, imageops::FilterType::Triangle))
}

/// The four pages' pictures at the screen's size: the menu as its Config
/// tab shows it, the tabs' bar, the page's panel and its controls drawn
/// as their defaults.
fn bases(src: &Source, labels: &Labels, language: bool) -> Result<Vec<RgbaImage>> {
    let menu = src.menu.as_ref().context("no pictures in a dry run")?;
    let mut common = Canvas::new();
    // guess: the game dimmed under it, as the system menu (menu.rs)
    common.put(&RgbaImage::from_pixel(1920, 1080, Rgba([0, 0, 0, 160])), (0, 0));
    common.put(&src.get("config_innerbg")?, hd(120.0, 0.0));
    // src: FUN_0014c370 (menu.rs): the bar, its buttons at rest, Config lit
    common.put(menu.get("system_titlebg")?, hd(0.0, 0.0));
    common.put(menu.get("system_scenebg")?, hd(250.667, 0.0));
    common.put(menu.get("underline")?, hd(0.0, 29.867));
    common.put(menu.get("menu_window")?, hd(18.667, 1.0));
    let btn = menu.get("menu_btn_ja")?;
    for r in 0..5u32 {
        let col = if r == 2 { 2 } else { 0 };
        common.put(&imageops::crop_imm(btn, 24 + 240 * col, 26 + 104 * r, 240, 104).to_image(), hd(26.667, 43.556 + 31.111 * r as f32));
    }
    common.put(&src.get("tab_bg")?, hd(133.333, 39.822));
    let panel_at = hd(PANEL.0, PANEL.1);

    // System, page 1: the panel to the line under the auto skip, its foot.
    let sys = src.get("system_bg")?;
    let (h, foot) = (sys.height(), rows(&sys, sys.height() - FOOT, sys.height()));
    let mut p1 = Canvas(common.0.clone());
    p1.panel(&stack(&[rows(&sys, 0, SPLIT), foot.clone()]), panel_at);
    // The read speed (15 of 250 ms), the auto page wait (3000 of 10000 ms):
    // their knobs, steps, the two boxes unchecked, the auto skip OFF.
    draw_knob(&mut p1, src, READ[1], 15.0, 250.0)?;
    draw_steps(&mut p1, src, READ[0], READ[2])?;
    draw_knob(&mut p1, src, AUTO_PAGE[1], 3000.0, 10000.0)?;
    draw_steps(&mut p1, src, AUTO_PAGE[0], AUTO_PAGE[2])?;
    draw(&mut p1, src, "btn_base2", check_cell(0, 0), UNREAD[3], 0)?;
    draw(&mut p1, src, "btn_base2", check_cell(0, 0), READ[3], 0)?;
    draw(&mut p1, src, "btn_base1", option_cell(0, 0), SCENE_SKIP[0], 0)?;
    draw(&mut p1, src, "btn_base2", check_cell(1, 0), SCENE_SKIP[1], 0)?;
    draw(&mut p1, src, "btn_base1", option_cell(1, 2), SCENE_SKIP[2], 0)?;

    // System, page 2: the header, the rest, the language's section.
    let mut parts = vec![rows(&sys, 0, HEADER), rows(&sys, SPLIT, h - FOOT)];
    if language {
        let mut section = rows(&sys, LAST_SECTION.0, LAST_SECTION.1);
        // Its label painted over with the section's first rows (the panel
        // changes across, not down), and ours written in.
        let label = (0, 12, 640, LAST_SECTION.1 - LAST_SECTION.0 - 24);
        let clean = imageops::crop_imm(&section, 0, 6, 640, 6).to_image();
        for y in (label.1..label.1 + label.3).step_by(6) {
            imageops::replace(&mut section, &clean, 0, i64::from(y));
        }
        let words = labels.draw("言語 / Language", 32.0, [255, 255, 255], [20, 30, 60], 255);
        let y = (i64::from(section.height()) - i64::from(words.height())) / 2;
        imageops::overlay(&mut section, &words, 50, y);
        parts.push(section);
    }
    parts.push(foot.clone());
    let mut p2 = Canvas(common.0.clone());
    p2.panel(&stack(&parts), panel_at);
    let dy = HEADER as i32 - SPLIT as i32;
    for (cell, p) in [(option_cell(4, 2), FADE_IN[0]), (option_cell(5, 0), FADE_IN[1]), (option_cell(4, 2), READ_FADE_IN[0]), (option_cell(5, 0), READ_FADE_IN[1])] {
        let (x, y) = hd(p.0, p.1);
        p2.put(&src.cut("btn_base1", cell)?, (x, y + dy));
    }

    // Sound: sound_bg, voice_bg to the characters, its foot; the system
    // sound's mute unchecked, the SE test at rest.
    let voice = src.get("voice_bg")?;
    let mut p3 = Canvas(common.0.clone());
    p3.panel(&src.get("sound_bg")?, panel_at);
    p3.panel(&stack(&[rows(&voice, 0, VOICE_CUT), rows(&voice, voice.height() - FOOT, voice.height())]), hd(PANEL.0, VOICE_Y));
    draw(&mut p3, src, "btn_mute", mute_cell(0), SYS_SE[3], 0)?;
    draw(&mut p3, src, "btn_test", (0, 0, 136, 120), SE[4], 0)?;

    // The characters' pages: their rows' strips (the last page's foot
    // under them), each row's face and name, its slider at the exe's 50 %
    // and the voice test at rest.
    let (strip, row_foot) = (rows(&voice, ROW_STRIP.0, ROW_STRIP.1), rows(&voice, ROW_FOOT.0, ROW_FOOT.1));
    let mut pages = vec![p1, p2, p3];
    for first in (0..CHARACTERS).step_by(ROWS_A_PAGE as usize) {
        let last = (first + ROWS_A_PAGE).min(CHARACTERS);
        let mut parts = vec![strip.clone(); (last - first) as usize];
        if last == CHARACTERS {
            parts.push(row_foot.clone());
        }
        let mut page = Canvas(common.0.clone());
        page.panel(&stack(&parts), (hd(PANEL.0, 0.0).0, CLIP.1));
        for n in first..last {
            let (y, dy) = (ROW_Y.0 + ROW_Y.1 * n as f32, row_shift(n));
            let cell = |r: (u32, u32, u32, u32)| (r.0, r.1, r.2 - r.0, r.3 - r.1);
            draw(&mut page, src, "conf_faceicon", cell(FACES[n as usize]), (144.0, y - 4.978), dy)?;
            draw(&mut page, src, "conf_name", cell(NAMES[n as usize]), (144.0, y + 24.889), dy)?;
            draw(&mut page, src, "conf_sbtn", (0, 0, 936, 252), (120.667 + TRAVEL * 0.5, y - 44.8), dy)?;
            draw(&mut page, src, "conf_stxt01", step_cell(false, 0), (245.0, y), dy)?;
            draw(&mut page, src, "conf_stxt01", step_cell(true, 0), (452.0, y), dy)?;
            draw(&mut page, src, "btn_test", (0, 0, 136, 120), (560.333, y - 3.733), dy)?;
        }
        pages.push(page);
    }

    // 操作説明: the manual.
    let mut p4 = Canvas(common.0);
    p4.panel(&src.get("conf_manual0_ja")?, hd(133.333, 73.007));
    pages.push(p4);
    Ok(pages.iter().map(Canvas::screen).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_from_the_stage() {
        let m = settings(Path::new("/nonexistent"), Path::new("/nonexistent"), &[], true, None).unwrap();
        let p = m.pictures.unwrap();
        assert_eq!(p.pages.len(), 7);
        // The characters' boxes: row 7 first on its page, 24 px under the
        // clip's top as row 0 is under its strip's top (1080p 892 - 868),
        // its box 12 px above the line.
        let RowControl::Toggle { button, value, .. } = &p.pages[4].rows[0] else { panic!() };
        assert_eq!((value, button.y), (&SettingValue::Voices(vec![7]), px(165.0 + 24.0 - 12.0)));
        // The unread speed's knob at 0 ms (level 10) on the left: its glow
        // at the track's left end, stage 276.67 (1080p 830).
        let RowControl::Slider { knob, travel, .. } = &p.pages[0].rows[0] else { panic!() };
        assert_eq!((knob.x + knob.w / 2 + travel, *travel), (px(830.0) - 1, -339)); // 1 px rounding the parts apart
        assert_eq!(p.tabs[0].pages, (0, 1));
    }
}
