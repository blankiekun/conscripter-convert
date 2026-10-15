//! The settings screen in the Vita's CONFIG look, from SystemData's
//! `bg_config_01.png` (基本設定) and `bg_config_02.png` (サウンド設定):
//! each is the page (960×544, its rows' labels and options drawn in) over
//! an atlas: the options lit (a glow and a cloud), a snowflake, and the
//! rows' cursor plates. The option words are cut out of the page (the
//! page minus the page with them painted over) so that the current one
//! can be drawn lit; the level bars are the page's own, emptied past the
//! level, with the snowflake at its end. A third page holds our own
//! switches (the confirmations, the mouse wheel) in the same style.
//!
//! Left out: 選んだ選択肢の色を変更 and 自動クイックセーブ (not converted);
//! their rows are painted over.

use image::{Rgba, RgbaImage, imageops};

use conv_core::ir::{PagedSettings, Placed, RowControl, Setting, SettingValue, SettingsPage, Switch};

use super::assets::ArchiveAssets;
use super::menus::{Labels, difference, patch, place, side_by_side};

/// The rows' middles on page 1 (page 2's are the first six).
const ROWS: [u32; 8] = [147, 185, 222, 258, 297, 334, 370, 409];
/// A row's label starts here on page 1 (page 2's are further left).
const LABEL_X: i32 = 140;
/// The level bars: the fill line from x 508 to 776, and the part of the
/// page each bar picture covers.
const FILL: (u32, u32) = (508, 776);
const BAR_X: (u32, u32) = (500, 786);
/// The ◀ and ▶ beside a bar.
const MINUS_X: (u32, u32) = (484, 16);
const PLUS_X: (u32, u32) = (785, 16);
/// The bar's empty colour (the page between its lines).
const EMPTY: [u8; 3] = [210, 217, 222];
/// The button hints (△デフォルト設定 ◎選択 ✕戻る) at the bottom right, to
/// the screen's edge.
const HINTS: (u32, u32, u32, u32) = (570, 478, 390, 66);
/// The page arrows (← L, → R).
const PREV: (u32, u32, u32, u32) = (20, 262, 42, 40);
const NEXT: (u32, u32, u32, u32) = (898, 262, 42, 40);
/// The page's title (基本設定).
const PAGE_TITLE: (u32, u32, u32, u32) = (84, 88, 130, 26);
const GOLD: [u8; 3] = [255, 200, 90];

/// An option: where its plain word is on the page (top left) and its lit
/// version in the atlas (x, y, w, h).
struct Opt {
    at: (u32, u32),
    lit: (u32, u32, u32, u32),
}

const fn opt(x: u32, y: u32, lx: u32, ly: u32, w: u32, h: u32) -> Opt {
    Opt { at: (x, y), lit: (lx, ly, w, h) }
}

/// Page 1's options by row: シーンスキップ機能, 音声スキップ, スキップモード,
/// 選択後のスキップ, (選んだ選択肢の色), (自動クイックセーブ), メッセージ速度.
const PAGE1: [&[Opt]; 7] = [
    &[opt(554, 133, 313, 547, 46, 26), opt(686, 133, 397, 547, 57, 26)],
    &[opt(552, 171, 311, 577, 48, 26), opt(681, 171, 392, 577, 67, 26)],
    &[opt(533, 208, 292, 607, 86, 27), opt(688, 208, 399, 607, 54, 26)],
    &[opt(550, 244, 309, 636, 55, 27), opt(687, 245, 398, 637, 56, 26)],
    &[opt(554, 282, 313, 667, 46, 26), opt(686, 282, 397, 667, 57, 26)],
    &[opt(423, 319, 292, 697, 86, 27), opt(535, 319, 384, 697, 84, 26), opt(643, 319, 480, 697, 153, 27), opt(805, 319, 658, 697, 70, 26)],
    &[opt(490, 355, 309, 726, 55, 27), opt(580, 356, 387, 727, 57, 26), opt(670, 356, 480, 727, 54, 26), opt(761, 355, 569, 726, 53, 27)],
];
/// Page 2's: 音声再生時にBGM音量を下げる, キャラクター音声.
const PAGE2: [&[Opt]; 2] = [
    &[opt(553, 283, 313, 547, 49, 26), opt(690, 283, 385, 547, 57, 26)],
    &[opt(539, 320, 291, 577, 75, 26), opt(678, 320, 368, 577, 77, 26)],
];
const KNOB1: (u32, u32, u32, u32) = (290, 764, 25, 22);

fn crop(img: &RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) -> RgbaImage {
    imageops::crop_imm(img, x, y, w, h).to_image()
}

/// Paints over `rect`: each row blends from the pixel left of it to the
/// one right of it (words on the panel's smooth gradient disappear).
fn erase(img: &mut RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) {
    let (left, right) = (x.saturating_sub(1), (x + w).min(img.width() - 1));
    for cy in y..(y + h).min(img.height()) {
        let (a, b) = (*img.get_pixel(left, cy), *img.get_pixel(right, cy));
        for cx in x..x + w {
            let t = (cx - left) as f32 / (right - left).max(1) as f32;
            let mix = |i: usize| (a.0[i] as f32 * (1.0 - t) + b.0[i] as f32 * t) as u8;
            img.put_pixel(cx, cy, Rgba([mix(0), mix(1), mix(2), mix(3)]));
        }
    }
}

/// Covers `rect` with the snow on its left, feathered in at the top,
/// bottom and left.
fn cover_corner(img: &mut RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) {
    const FEATHER: u32 = 14;
    let src = crop(img, (x - w, y, w, h));
    for (cx, cy, p) in src.enumerate_pixels() {
        let edge = cx.min(cy).min(h - 1 - cy).min(FEATHER);
        let q = img.get_pixel_mut(x + cx, y + cy);
        for i in 0..4 {
            q.0[i] = ((p.0[i] as u32 * edge + q.0[i] as u32 * (FEATHER - edge)) / FEATHER) as u8;
        }
    }
}

/// Paints over `rect` with the panel's gradient: each column blends from
/// the row above to the row below, both averaged over 60 pixels across
/// (the snow seen through the panel would leave streaks).
fn smooth_patch(img: &mut RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) {
    let (top, bottom) = (y - 1, y + h);
    let mean = |img: &RgbaImage, cx: u32, row: u32| {
        let (a, b) = (cx.saturating_sub(30).max(x), (cx + 30).min(x + w - 1));
        let mut sum = [0u32; 4];
        for sx in a..=b {
            for (i, v) in img.get_pixel(sx, row).0.iter().enumerate() {
                sum[i] += *v as u32;
            }
        }
        sum.map(|v| v as f32 / (b - a + 1) as f32)
    };
    let edges: Vec<([f32; 4], [f32; 4])> = (x..x + w).map(|cx| (mean(img, cx, top), mean(img, cx, bottom))).collect();
    for (i, (a, b)) in edges.into_iter().enumerate() {
        for cy in y..y + h {
            let t = (cy - top) as f32 / (bottom - top) as f32;
            let px = Rgba([0, 1, 2, 3].map(|k| (a[k] * (1.0 - t) + b[k] * t) as u8));
            img.put_pixel(x + i as u32, cy, px);
        }
    }
}

/// `img` with its light pixels turned toward gold: a button under the
/// pointer. Light means brighter than most of the picture (the page
/// behind it, cut out with the button), so the page keeps its colour.
fn golden(img: &RgbaImage) -> RgbaImage {
    let lum = |p: &Rgba<u8>| (p.0[0] as u32 + p.0[1] as u32 + p.0[2] as u32) / 3;
    let mut all: Vec<u32> = img.pixels().map(lum).collect();
    all.sort_unstable();
    let from = (all.get(all.len() / 2).copied().unwrap_or(200) + 10).min(250);
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let l = lum(p);
        if l > from {
            let t = (l - from) as f32 / (255 - from) as f32;
            for (c, g) in p.0.iter_mut().zip(GOLD) {
                *c = (*c as f32 * (1.0 - t) + g as f32 * t) as u8;
            }
        }
    }
    out
}

/// The 11 bar pictures (levels 0-10) of the bar whose fill line is at
/// `line` on `page`, placed at `y`, its knob `knob`.
fn bars(assets: &mut ArchiveAssets, page: &RgbaImage, line: u32, knob: &RgbaImage, name: &str, y: i32, inverted: bool) -> (Placed, Vec<String>) {
    let top = line - 11;
    let rect = (BAR_X.0, top, BAR_X.1 - BAR_X.0, 24);
    let base = crop(page, rect);
    let mut paths = Vec::new();
    for k in 0..=10u32 {
        let level = if inverted { 10 - k } else { k };
        let end = FILL.0 + (FILL.1 - FILL.0) * level / 10;
        let mut img = base.clone();
        for x in end + 1..=FILL.1 {
            for dy in 0..2 {
                img.put_pixel(x - BAR_X.0, line - top + dy, Rgba([EMPTY[0], EMPTY[1], EMPTY[2], 255]));
            }
        }
        let kx = end as i64 - BAR_X.0 as i64 - knob.width() as i64 / 2;
        imageops::overlay(&mut img, knob, kx, (line - top) as i64 + 1 - knob.height() as i64 / 2);
        paths.push(assets.put_system_png(&format!("{name}_{k}"), &img));
    }
    let bar = Placed { image: paths[0].clone(), x: BAR_X.0 as i32, y: y - (line - top) as i32, w: rect.2 as i32, h: 24 };
    (bar, paths)
}

/// The fill line of the bar on row `y` of `page` (its first dark row).
fn fill_line(page: &RgbaImage, y: u32) -> u32 {
    (y - 12..y + 4).find(|&r| page.get_pixel(600, r).0[..3].iter().map(|&c| c as u32).sum::<u32>() < 470).map_or(y - 2, |r| r + 3)
}

/// A level row: bar pictures and its ◀ ▶ buttons, cut from `page`'s bar
/// whose fill line is at `line`, drawn on row `y` (`line`'s own row when
/// the same). `inverted`: the bar shows 10 minus the level, ◀ raises it.
#[allow(clippy::too_many_arguments)]
fn slider(assets: &mut ArchiveAssets, page: &RgbaImage, line: u32, knob: &RgbaImage, name: &str, y_line: u32, value: SettingValue, inverted: bool) -> RowControl {
    let (bar, bars) = bars(assets, page, line, knob, name, y_line as i32, inverted);
    let arrow = |assets: &mut ArchiveAssets, (x, w): (u32, u32), part: &str| {
        let img = crop(page, (x, line - 7, w, 16));
        place(assets, &format!("{name}_{part}"), &side_by_side(&[img.clone(), golden(&img)]), x as i32, y_line as i32 - 7)
    };
    let (left, right) = (arrow(assets, MINUS_X, "minus"), arrow(assets, PLUS_X, "plus"));
    let (minus, plus) = if inverted { (right, left) } else { (left, right) };
    RowControl::Slider { value, bar, bars, minus, plus }
}

/// An option list: each word cut out of the page `atlas` starts with
/// (where `clean` has it painted over), lit from the atlas below it,
/// moved down by `dy`.
#[allow(clippy::too_many_arguments)]
fn choice(assets: &mut ArchiveAssets, atlas: &RgbaImage, clean: &RgbaImage, opts: &[Opt], values: &[i32], value: SettingValue, name: &str, dy: i32) -> RowControl {
    let options = opts
        .iter()
        .zip(values)
        .enumerate()
        .map(|(i, (o, &v))| {
            let (w, h) = (o.lit.2, o.lit.3);
            let glow = crop(atlas, o.lit);
            let plain = letters_only(&difference(atlas, clean, (o.at.0, o.at.1, w, h)), &glow);
            let mut lit = plain.clone();
            imageops::overlay(&mut lit, &glow, 0, 0);
            (v, place(assets, &format!("{name}_{i}"), &side_by_side(&[plain, lit]), o.at.0 as i32, o.at.1 as i32 + dy))
        })
        .collect();
    RowControl::Choice { value, options }
}

/// `word`'s pixels near `lit`'s letters (its white core): the cut-out
/// word without the panel's faint snow around it.
fn letters_only(word: &RgbaImage, lit: &RgbaImage) -> RgbaImage {
    let (w, h) = word.dimensions();
    let core = |x: u32, y: u32| {
        x < lit.width() && y < lit.height() && {
            let p = lit.get_pixel(x, y).0;
            p[3] > 200 && p[..3].iter().map(|&c| c as u32).sum::<u32>() > 690
        }
    };
    let mut out = word.clone();
    for (x, y, p) in out.enumerate_pixels_mut() {
        let near = (x.saturating_sub(2)..=(x + 2).min(w - 1)).any(|cx| (y.saturating_sub(2)..=(y + 2).min(h - 1)).any(|cy| core(cx, cy)));
        if !near {
            p.0[3] = 0;
        }
    }
    out
}

/// A label in the pages' style (white, a soft blue halo), its left edge
/// at `x`, centred on row `y`.
fn draw_label(page: &mut RgbaImage, labels: &Labels, text: &str, x: i32, y: u32) {
    let img = labels.draw(text, 17.0, [255, 255, 255], [60, 70, 130], 255);
    imageops::overlay(page, &img, x as i64 - 4, y as i64 - img.height() as i64 / 2);
}

/// A button with a label in the pages' style, gold under the pointer,
/// its left edge at `x`, centred on `y`.
fn text_button(assets: &mut ArchiveAssets, labels: &Labels, name: &str, text: &str, x: i32, y: i32) -> Placed {
    let normal = labels.draw(text, 18.0, [255, 255, 255], [90, 110, 220], 255);
    let hover = labels.draw(text, 18.0, [255, 245, 220], [230, 150, 40], 255);
    let img = side_by_side(&[normal, hover]);
    let h = img.height() as i32;
    place(assets, name, &img, x, y - h / 2)
}

/// The pages; `switches` (ours: the wheel, the confirmations) go on a
/// third, `characters` are the voiced characters (キャラクター音声).
pub fn pictures(assets: &mut ArchiveAssets, labels: &Labels, switches: &[(String, Switch)], characters: Vec<u32>) -> Option<PagedSettings> {
    let atlas1 = assets.system_image("bg_config_01.png")?;
    let atlas2 = assets.system_image("bg_config_02.png")?;
    let knob = crop(&atlas1, KNOB1);
    let screen = |img: &RgbaImage| crop(img, (0, 0, 960, 544));
    let (orig1, orig2) = (screen(&atlas1), screen(&atlas2));
    // Option words painted over; the rows not converted too.
    let mut clean1 = orig1.clone();
    let mut clean2 = orig2.clone();
    for o in PAGE1.iter().flat_map(|r| r.iter()) {
        erase(&mut clean1, (o.at.0, o.at.1, o.lit.2, o.lit.3));
    }
    for o in PAGE2.iter().flat_map(|r| r.iter()) {
        erase(&mut clean2, (o.at.0, o.at.1, o.lit.2, o.lit.3));
    }
    for img in [&mut clean1, &mut clean2] {
        cover_corner(img, HINTS);
    }
    // Rows painted over top to bottom: the panel is a vertical gradient.
    let mut page1 = clean1.clone();
    patch(&mut page1, (96, ROWS[4] - 16, 776, ROWS[5] - ROWS[4] + 32));
    // The window's bar (row 8) also makes our オート速度 bar, on row 5.
    let window_line = fill_line(&orig1, ROWS[7]);
    let mut auto_page = page1.clone();
    let bar_rect = (MINUS_X.0, window_line - 11, PLUS_X.0 + PLUS_X.1 - MINUS_X.0, 24);
    let mut no_bar = orig1.clone();
    patch(&mut no_bar, bar_rect);
    let bar = difference(&orig1, &no_bar, bar_rect);
    let auto_line = ROWS[4] - (ROWS[7] - window_line);
    imageops::overlay(&mut auto_page, &bar, MINUS_X.0 as i64, (auto_line - 11) as i64);
    draw_label(&mut auto_page, labels, "オート速度", LABEL_X, ROWS[4]);
    let page1 = auto_page;

    let o = PAGE1;
    let sw = SettingValue::Switch;
    let rows1 = vec![
        choice(assets, &atlas1, &clean1, o[0], &[1, 0], sw(Switch::SceneSkipDialog), "cfg_scene", 0),
        // 音声スキップ する: the click stops the voice.
        choice(assets, &atlas1, &clean1, o[1], &[0, 1], sw(Switch::VoiceKeepsPlaying), "cfg_voice", 0),
        // 既読のみ / 全て.
        choice(assets, &atlas1, &clean1, o[2], &[0, 1], SettingValue::SkipUnread, "cfg_mode", 0),
        // 持続: skip goes on after a choice.
        choice(assets, &atlas1, &clean1, o[3], &[0, 1], sw(Switch::StopSkipAtChoice), "cfg_after", 0),
        slider(assets, &page1, auto_line, &knob, "cfg_auto", auto_line, SettingValue::Level(Setting::AutoSpeed), false),
        // 遅い 普通 早い 一括: text speed levels.
        choice(assets, &atlas1, &clean1, o[6], &[4, 7, 9, 10], SettingValue::Level(Setting::TextSpeed), "cfg_speed", 0),
        // ウインドウの不透明度: the transparency, shown the other way round.
        slider(assets, &orig1, window_line, &knob, "cfg_window", window_line, SettingValue::Level(Setting::WindowTransparency), true),
    ];
    let mut rows2 = Vec::new();
    for (i, setting) in [Setting::VoiceVolume, Setting::BgmVolume, Setting::SeVolume, Setting::SystemVolume].into_iter().enumerate() {
        let line = fill_line(&orig2, ROWS[i]);
        rows2.push(slider(assets, &orig2, line, &knob, &format!("cfg_vol{i}"), line, SettingValue::Level(setting), false));
    }
    // ON: the music is lowered (the switch keeps it full).
    rows2.push(choice(assets, &atlas2, &clean2, PAGE2[0], &[0, 1], sw(Switch::BgmFullForVoice), "cfg_duck", 0));
    rows2.push(choice(assets, &atlas2, &clean2, PAGE2[1], &[1, 0], SettingValue::Voices(characters), "cfg_voices", 0));

    // Page 3: page 1's panel, emptied, with our switches as ON / OFF rows.
    let mut page3 = clean1.clone();
    smooth_patch(&mut page3, (96, ROWS[0] - 20, 776, ROWS[7] - ROWS[0] + 40));
    erase(&mut page3, PAGE_TITLE);
    let title = labels.draw("その他の設定", 17.0, [255, 255, 255], [60, 70, 130], 255);
    imageops::overlay(&mut page3, &title, 84, 88);
    let mut rows3 = Vec::new();
    for (i, (label, s)) in switches.iter().take(ROWS.len()).enumerate() {
        draw_label(&mut page3, labels, label, LABEL_X, ROWS[i]);
        let dy = ROWS[i] as i32 - ROWS[0] as i32;
        rows3.push(choice(assets, &atlas1, &clean1, o[0], &[1, 0], sw(*s), &format!("cfg_sw{i}"), dy));
    }

    let mut pages = vec![(page1, rows1), (clean2.clone(), rows2)];
    if !rows3.is_empty() {
        pages.push((page3, rows3));
    }
    let pages = pages
        .into_iter()
        .enumerate()
        .map(|(i, (img, rows))| SettingsPage { base: place(assets, &format!("cfg_page{}", i + 1), &img, 0, 0), rows })
        .collect();
    let arrow = |assets: &mut ArchiveAssets, rect: (u32, u32, u32, u32), name: &str| {
        let img = crop(&orig1, rect);
        place(assets, name, &side_by_side(&[img.clone(), golden(&img)]), rect.0 as i32, rect.1 as i32)
    };
    Some(PagedSettings {
        pages,
        prev: arrow(assets, PREV, "cfg_prev"),
        next: arrow(assets, NEXT, "cfg_next"),
        defaults: text_button(assets, labels, "cfg_defaults", "デフォルト設定", 600, 505),
        close: text_button(assets, labels, "cfg_close", "戻る", 860, 505),
        title: text_button(assets, labels, "cfg_title", "タイトルに戻る", 90, 505),
        quit: text_button(assets, labels, "cfg_quit", "ゲームを終了", 260, 505),
    })
}
