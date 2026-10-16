//! The in-game menus (button bar, save/load, settings, backlog, yes/no
//! dialog) in the Vita version's look. The Vita drives them with its
//! buttons and keeps their layout in the encrypted eboot, so the screens
//! are ours, built from SystemData's pictures (backgrounds, slot plates,
//! the dialog frame) with mouse buttons whose labels are drawn with the
//! `--font` (white with a blue glow, like the game's own labels).
//!
//! conv-cons draws the menus' words in white, and the Vita's backgrounds
//! are light: backgrounds behind words are darkened.

use image::{Rgba, RgbaImage, imageops};

use conv_core::ir::{
    BarAction, BarButton, Backlog, ButtonBar, ConfirmDialog, PageButton, Placed, Rect, SaveMenu, Setting, SettingRow, SettingSwitch,
    SettingsMenu, Switch, VoiceToggle,
};

pub use crate::labels::Labels;
use crate::labels::spread;

use super::assets::ArchiveAssets;
use super::convert::{SCREEN_H, SCREEN_W};

/// Where SystemData's screens show the Vita's button hints (◎決定 …) and
/// the page box (/10).
pub(crate) const HINTS: (u32, u32, u32, u32) = (636, 486, 310, 38);
/// Around them, to the screen's right edge.
const HINTS_CORNER: (u32, u32, u32, u32) = (626, 470, 334, 70);
const PAGE_BOX: (u32, u32, u32, u32) = (818, 22, 142, 64);
const GLOW: [u8; 3] = [90, 130, 255];
const HOVER_GLOW: [u8; 3] = [255, 190, 70];
/// The atlas labels' glow.
const BAR_GLOW: [u8; 3] = [70, 80, 255];

impl Labels<'_> {
    /// A button: three cells side by side, normal, under the pointer
    /// (gold halo), disabled (faded).
    pub(crate) fn button(&self, text: &str, size: f32) -> RgbaImage {
        let cells = [
            self.draw(text, size, [255, 255, 255], GLOW, 255),
            self.draw(text, size, [255, 255, 240], HOVER_GLOW, 255),
            self.draw(text, size, [200, 200, 210], GLOW, 90),
        ];
        side_by_side(&cells)
    }
}

/// `img` over a dark halo of its own shape (`strength` 0-1), so that
/// light letters stay readable on light pictures.
fn shadowed(img: &RgbaImage, strength: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mask: Vec<f32> = img.pixels().map(|p| p.0[3] as f32 / 255.0).collect();
    let halo = spread(&mask, w, h);
    let mut out = RgbaImage::new(w, h);
    for (i, (px, p)) in out.pixels_mut().zip(img.pixels()).enumerate() {
        let (m, g) = (mask[i], halo[i] * strength);
        let a = m + g * (1.0 - m);
        if a > 0.0 {
            let c = |k: usize| (p.0[k] as f32 * m / a) as u8;
            *px = Rgba([c(0), c(1), c(2), (a * 255.0) as u8]);
        }
    }
    out
}

pub(crate) fn side_by_side(cells: &[RgbaImage]) -> RgbaImage {
    let (w, h) = (cells.iter().map(|c| c.width()).max().unwrap_or(1), cells.iter().map(|c| c.height()).max().unwrap_or(1));
    let mut out = RgbaImage::new(w * cells.len() as u32, h);
    for (i, c) in cells.iter().enumerate() {
        imageops::overlay(&mut out, c, (i as u32 * w + (w - c.width()) / 2) as i64, ((h - c.height()) / 2) as i64);
    }
    out
}

fn darker(mut img: RgbaImage, percent: u32) -> RgbaImage {
    for px in img.pixels_mut() {
        for c in &mut px.0[..3] {
            *c = (*c as u32 * percent / 100) as u8;
        }
    }
    img
}

/// Paints over `rect`: each column blends from the pixel above it to
/// the one below (words on a soft background disappear).
pub(crate) fn patch(img: &mut RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) {
    let (top, bottom) = (y.saturating_sub(1), (y + h).min(img.height() - 1));
    for cx in x..(x + w).min(img.width()) {
        let (a, b) = (*img.get_pixel(cx, top), *img.get_pixel(cx, bottom));
        for cy in y..y + h {
            let t = (cy - top) as f32 / (bottom - top).max(1) as f32;
            let mix = |i: usize| (a.0[i] as f32 * (1.0 - t) + b.0[i] as f32 * t) as u8;
            img.put_pixel(cx, cy, Rgba([mix(0), mix(1), mix(2), mix(3)]));
        }
    }
}

/// Covers the bottom right corner (the button hints) with the snow on its
/// left, feathered in over [`FEATHER`] pixels at the top, bottom and left.
pub(crate) fn cover_hints(img: &mut RgbaImage) {
    const FEATHER: u32 = 14;
    let (x, y, w, h) = HINTS_CORNER;
    let src = imageops::crop_imm(img, x - w, y, w, h).to_image();
    for (cx, cy, p) in src.enumerate_pixels() {
        let edge = cx.min(cy).min(h - 1 - cy).min(FEATHER);
        let q = img.get_pixel_mut(x + cx, y + cy);
        for i in 0..4 {
            q.0[i] = ((p.0[i] as u32 * edge + q.0[i] as u32 * (FEATHER - edge)) / FEATHER) as u8;
        }
    }
}

/// Copies `rect` of `from` (another screen with the same decoration).
fn cover(img: &mut RgbaImage, from: &RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) {
    let part = imageops::crop_imm(from, x, y, w, h).to_image();
    imageops::replace(img, &part, x as i64, y as i64);
}

/// The pixels of `a` in `rect` that differ from `b` (a word one picture
/// has and the other doesn't), the rest transparent.
pub(crate) fn difference(a: &RgbaImage, b: &RgbaImage, (x, y, w, h): (u32, u32, u32, u32)) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    for (ox, oy, px) in out.enumerate_pixels_mut() {
        let (p, q) = (a.get_pixel(x + ox, y + oy), b.get_pixel(x + ox, y + oy));
        let d: u32 = (0..3).map(|i| (p.0[i] as i32 - q.0[i] as i32).unsigned_abs()).sum();
        *px = Rgba([p.0[0], p.0[1], p.0[2], (d.saturating_sub(24) * 4).min(255) as u8]);
    }
    out
}

fn screen_part(img: &RgbaImage) -> RgbaImage {
    imageops::crop_imm(img, 0, 0, SCREEN_W as u32, SCREEN_H as u32).to_image()
}

fn transparent(assets: &mut ArchiveAssets) -> String {
    assets.put_system_png("none", &RgbaImage::new(1, 1))
}

pub(crate) fn place(assets: &mut ArchiveAssets, name: &str, img: &RgbaImage, x: i32, y: i32) -> Placed {
    let image = assets.put_system_png(name, img);
    Placed { image, x, y, w: img.width() as i32, h: img.height() as i32 }
}

/// A three-cell button placed with its normal cell's centre at (cx, cy).
fn button(assets: &mut ArchiveAssets, labels: &Labels, name: &str, text: &str, size: f32, cx: i32, cy: i32) -> Placed {
    let img = labels.button(text, size);
    let (w, h) = ((img.width() / 3) as i32, img.height() as i32);
    place(assets, name, &img, cx - w / 2, cy - h / 2)
}

/// The Vita's in-game labels in the skip dialog's atlas (`ss_bg1.png`):
/// Q-Save, Q-Load, Return, Save, Load, Log, Config in 60×24 cells from
/// x = 0, in rows from y = 746: white, glowing blue (selected), navy (for
/// light backgrounds), faded.
const ATLAS_LABELS: (u32, u32, u32, u32) = (0, 746, 60, 24);

/// Label `column` of the atlas as a button (normal, glowing, faded),
/// trimmed to the columns any of them covers. The Vita draws them on
/// dark panels; here they cross the text window's white feather, so the
/// white and faded ones get a dark halo like the font's labels.
fn atlas_label(atlas: &RgbaImage, column: u32) -> RgbaImage {
    let (x0, y0, w, h) = ATLAS_LABELS;
    let x = x0 + column * w;
    let rows = [0, 1, 3].map(|r| imageops::crop_imm(atlas, x, y0 + r * h, w, h).to_image());
    let used = |cx: u32| rows.iter().any(|r| (0..h).any(|cy| r.get_pixel(cx, cy).0[3] > 0));
    let (left, right) = ((0..w).find(|&c| used(c)).unwrap_or(0), (0..w).rfind(|&c| used(c)).unwrap_or(w - 1));
    let [normal, glowing, faded] = rows.map(|r| imageops::crop_imm(&r, left, 0, right + 1 - left, h).to_image());
    side_by_side(&[shadowed(&normal, 0.85), glowing, shadowed(&faded, 0.5)])
}

/// The bar along the bottom right, under the text window's edge: the
/// Vita's own labels (Log, Save, Load, Config) from `ss_bg1.png`; Auto,
/// Skip and Hide, which it has no label for, drawn with the font in their
/// style (white, a blue glow under the pointer or while on).
pub fn button_bar(assets: &mut ArchiveAssets, labels: &Labels) -> ButtonBar {
    let atlas = assets.system_image("ss_bg1.png");
    let items = [
        ("Auto", None, BarAction::Auto),
        ("Skip", None, BarAction::Skip),
        ("Log", Some(5), BarAction::Log),
        ("Save", Some(3), BarAction::Save),
        ("Load", Some(4), BarAction::Load),
        ("Config", Some(6), BarAction::Settings),
        ("Hide", None, BarAction::HideWindow),
    ];
    let size = 17.0;
    let drawn = |text: &str, rgb: [u8; 3], glow: [u8; 3], alpha: u8| labels.draw(text, size, rgb, glow, alpha);
    let mut x = SCREEN_W - 12;
    let mut buttons: Vec<BarButton> = Vec::new();
    for &(text, column, action) in items.iter().rev() {
        let img = match (&atlas, column) {
            (Some(atlas), Some(c)) => atlas_label(atlas, c),
            _ => side_by_side(&[
                drawn(text, [255, 255, 255], [0, 0, 0], 255),
                drawn(text, [255, 255, 255], BAR_GLOW, 255),
                drawn(text, [255, 255, 255], [0, 0, 0], 110),
            ]),
        };
        let (w, h) = ((img.width() / 3) as i32, img.height() as i32);
        x -= w;
        let name = format!("bar_{}", text.to_ascii_lowercase());
        // The atlas's letters sit 2 pixels higher in their cells than the font's.
        let lower = if column.is_some() && atlas.is_some() { 2 } else { 0 };
        let image = place(assets, &name, &img, x, SCREEN_H - 16 - h / 2 + lower);
        let active = matches!(action, BarAction::Auto | BarAction::Skip).then(|| {
            let on = drawn(text, [255, 255, 255], BAR_GLOW, 255);
            let off = drawn(text, [255, 255, 255], [0, 0, 0], 110);
            place(assets, &format!("{name}_on"), &side_by_side(&[on.clone(), on, off]), image.x, image.y)
        });
        buttons.insert(0, BarButton { image, active, action });
        x -= 4;
    }
    ButtonBar { buttons, top: None, ..Default::default() }
}

/// Save/load: `save_bg.png`/`load_bg.png` with their headers, three
/// `sl_data.png` slot plates a page, ten pages.
pub fn save_menu(assets: &mut ArchiveAssets, labels: &Labels) -> Option<SaveMenu> {
    let save_bg = assets.system_image("save_bg.png")?;
    let load_bg = assets.system_image("load_bg.png")?;
    let slots = assets.system_image("sl_data.png")?;
    let header = (24, 22, 106, 50);
    let mut base = save_bg.clone();
    // Without the header, the controller hints and the page box.
    patch(&mut base, header);
    let save_header = difference(&save_bg, &base, header);
    let load_header = difference(&load_bg, &base, header);
    patch(&mut base, HINTS);
    let log = assets.system_image("bklog.png")?;
    cover(&mut base, &log, PAGE_BOX);

    let plate = imageops::crop_imm(&slots, 3, 2, 416, 131).to_image();
    let (row_x, row_y, pitch) = (40i32, 96i32, 140i32);
    let mut frame = RgbaImage::new(SCREEN_W as u32, SCREEN_H as u32);
    for r in 0..3 {
        imageops::overlay(&mut frame, &plate, row_x as i64, (row_y + r * pitch) as i64);
    }
    let thumbnail = Rect { x: row_x + 33, y: 27, w: 120, h: 68 };
    // Under the pointer: the plate lit; selected: added warmth.
    let mut lit = RgbaImage::new(plate.width() + (row_x as u32), plate.height());
    let mut warm = RgbaImage::new(plate.width(), plate.height());
    for (x, y, px) in plate.enumerate_pixels() {
        let in_thumb = (x as i32 + row_x) >= thumbnail.x && (x as i32 + row_x) < thumbnail.x + thumbnail.w && (y as i32) >= thumbnail.y && (y as i32) < thumbnail.y + thumbnail.h;
        if px.0[3] > 0 && !in_thumb {
            lit.put_pixel(x + row_x as u32, y, Rgba([255, 255, 255, px.0[3] / 5]));
            warm.put_pixel(x, y, Rgba([70, 40, 10, 255]));
        }
    }
    let hover = side_by_side(&[RgbaImage::new(lit.width(), lit.height()), lit]);
    let preview = Rect { x: 500, y: 110, w: 400, h: 227 };
    let pages = (0..10)
        .map(|i| {
            let b = button(assets, labels, &format!("sl_page{}", i + 1), &(i + 1).to_string(), 20.0, 520 + i * 40, 372);
            PageButton { marker_x: b.x + b.w / 2 - 12, marker_y: b.y + b.h - 2, button: b }
        })
        .collect();
    let marker = labels.draw("─", 24.0, [255, 230, 160], HOVER_GLOW, 255);
    Some(SaveMenu {
        base: place(assets, "sl_base", &base, 0, 0),
        base_opacity: 255,
        frame: place(assets, "sl_frame", &frame, 0, 0),
        save_header: place(assets, "sl_header_save", &save_header, header.0 as i32, header.1 as i32),
        load_header: place(assets, "sl_header_load", &load_header, header.0 as i32, header.1 as i32),
        delete: button(assets, labels, "sl_delete", "削除", 20.0, 700, 505),
        save: button(assets, labels, "sl_save", "セーブ", 20.0, 790, 505),
        load: button(assets, labels, "sl_load", "ロード", 20.0, 790, 505),
        close: button(assets, labels, "sl_close", "戻る", 20.0, 890, 505),
        pages,
        page_marker: place(assets, "sl_marker", &marker, 0, 0),
        rows: 3,
        row_y,
        row_pitch: pitch,
        row_hover: place(assets, "sl_row_hover", &hover, 0, 0),
        row_selected: place(assets, "sl_row_selected", &warm, row_x, 0),
        thumbnail,
        new_icon: place(assets, "sl_new", &labels.draw("NEW", 14.0, [255, 240, 200], HOVER_GLOW, 255), row_x + 36, 74),
        text_x: row_x + 172,
        text_y: 34,
        font_size: 14,
        line_height: 28,
        preview,
        // To the plate's right edge, less a margin.
        text_width: plate.width().saturating_sub(172 + 12),
        backdrop: None,
    })
}

/// The settings screen on the plain snow background, with
/// `bg_config_01.png`'s CONFIG header.
pub fn settings_menu(assets: &mut ArchiveAssets, labels: &Labels) -> Option<SettingsMenu> {
    let config = assets.system_image("bg_config_01.png")?;
    let load = assets.system_image("load_bg.png")?;
    let log = assets.system_image("bklog.png")?;
    let header_rect = (24, 22, 140, 50);
    let mut base = load;
    patch(&mut base, header_rect);
    patch(&mut base, HINTS);
    cover(&mut base, &log, PAGE_BOX);
    let header = difference(&config, &base, header_rect);
    let base = darker(base, 45);
    let row = |label: &str, setting| SettingRow { label: label.to_string(), setting };
    let switches: Vec<(String, Switch)> = [
        ("マウスホイールでテキストを進める", Switch::WheelReadsOn),
        ("マウスホイールで履歴を閉じる", Switch::WheelClosesLog),
        ("セーブ確認", Switch::ConfirmSave),
        ("ロード確認", Switch::ConfirmLoad),
        ("セーブデータの削除確認", Switch::ConfirmDelete),
        ("タイトルに戻る確認", Switch::ConfirmTitle),
        ("ゲーム終了確認", Switch::ConfirmQuit),
    ]
    .into_iter()
    .map(|(l, s)| (l.to_string(), s))
    .collect();
    // One switch for all the voices (キャラクター音声), whose lines are
    // all character 0.
    let characters = vec![0];
    let voices = vec![VoiceToggle { label: "キャラクター音声".into(), characters: characters.clone() }];
    Some(SettingsMenu {
        base: place(assets, "menu_base", &base, 0, 0),
        base_opacity: 255,
        header: place(assets, "config_header", &header, header_rect.0 as i32, header_rect.1 as i32),
        close: button(assets, labels, "menu_close", "戻る", 20.0, 890, 512),
        rows: vec![
            row("ＢＧＭ", Setting::BgmVolume),
            row("効果音", Setting::SeVolume),
            row("音声", Setting::VoiceVolume),
            row("文字速度", Setting::TextSpeed),
            row("オート速度", Setting::AutoSpeed),
            row("ウィンドウ透過", Setting::WindowTransparency),
        ],
        label_x: 150,
        minus_x: 340,
        level_x: 380,
        plus_x: 680,
        y: 130,
        pitch: 52,
        font_size: 22,
        title_label: "タイトルに戻る".into(),
        quit_label: "ゲームを終了".into(),
        exits_y: 500,
        // No voice page: the second is the switches'.
        tab_labels: vec!["音量・速度".into(), "その他".into()],
        tabs_x: 300,
        tabs_y: 40,
        voices,
        voice_column_x: 440,
        voice_pitch: 50,
        skip_unread_label: Some("未読テキストもスキップする".into()),
        pictures: super::config::pictures(assets, labels, &switches, characters).map(Box::new),
        switches: switches.into_iter().map(|(label, switch)| SettingSwitch { label, switch }).collect(),
        // The Vita's デフォルト設定 (0x81070e70): BGM, SE and system sounds
        // 55 %, voices 85 %, the window 70 % opaque, the music lowered for
        // voices.
        defaults: vec![
            (Setting::BgmVolume, 6),
            (Setting::SeVolume, 6),
            (Setting::VoiceVolume, 9),
            (Setting::WindowTransparency, 3),
            (Setting::SystemVolume, 6),
        ],
        switches_off: vec![Switch::BgmFullForVoice],
    })
}

/// The backlog: `bklog.png`'s header and panel, darkened.
pub fn backlog(assets: &mut ArchiveAssets, labels: &Labels) -> Option<Backlog> {
    let mut base = darker(screen_part(&assets.system_image("bklog.png")?), 45);
    patch(&mut base, HINTS);
    let none = transparent(assets);
    let empty = Placed { image: none, x: 0, y: 0, w: 1, h: 1 };
    Some(Backlog {
        base: place(assets, "log_base", &base, 0, 0),
        base_opacity: 255,
        frame: empty.clone(),
        track: empty,
        up: button(assets, labels, "log_up", "▲", 20.0, 900, 100),
        down: button(assets, labels, "log_down", "▼", 20.0, 900, 400),
        close: button(assets, labels, "menu_close", "戻る", 20.0, 890, 512),
        area: Rect { x: 90, y: 92, w: 690, h: 330 },
        font_size: 18,
        line_height: 26,
        columns: 37,
    })
}

/// The yes/no dialog: `dlg_bg_26.png` (the frame without words), darkened.
pub fn confirm_dialog(assets: &mut ArchiveAssets, labels: &Labels) -> Option<ConfirmDialog> {
    let frame = darker(assets.system_image("dlg_bg_26.png")?, 55);
    let (x, y) = ((SCREEN_W - frame.width() as i32) / 2, (SCREEN_H - frame.height() as i32) / 2);
    // Plain plates: conv-cons writes はい/いいえ over them.
    let plate = |a: u8| RgbaImage::from_pixel(110, 34, Rgba([40, 60, 140, a]));
    let cells = side_by_side(&[plate(150), plate(230), plate(60)]);
    let by = y + frame.height() as i32 - 80;
    let yes = place(assets, "dialog_button", &cells, SCREEN_W / 2 - 130, by);
    let no = Placed { x: SCREEN_W / 2 + 20, ..yes.clone() };
    let _ = labels;
    Some(ConfirmDialog {
        base: place(assets, "dialog_base", &frame, x, y),
        yes,
        no,
        yes_label: "はい".into(),
        no_label: "いいえ".into(),
        text_y: 90,
        font_size: 20,
        overwrite: "のデータに上書きします。よろしいですか？".into(),
        load: "のデータをロードします。よろしいですか？".into(),
        delete: "のデータを削除します。よろしいですか？".into(),
        title: "タイトルへ戻りますか？".into(),
        quit: "ゲームを終了します。よろしいですか？".into(),
        pictures: Vec::new(),
        backdrop: None,
        pressed: false,
        fade_ms: 0,
        unroll: None,
    })
}
