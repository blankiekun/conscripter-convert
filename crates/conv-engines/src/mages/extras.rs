//! The extras: a menu (ALBUM, CONFIG, MUSIC, back) and the CG gallery
//! ("ALBUM") from `CGMODE`, in the Vita's look and our layout (the
//! original's is in the encrypted eboot):
//!
//! - `gm_bg.png` (960×1699): the album background in the top 544 rows,
//!   then eight heroine cards (94×378, x = 1 + 96·i) locked (y 548),
//!   normal (y 934) and highlighted (y 1318), in CGMODE's page order.
//! - `gm_bg1.png`: a "NO IMAGE" plate (4, 546, 184, 110) and a thumbnail
//!   frame (196, 546, 186, 110).
//! - The menu's labels come from `title_bg.png` (ALBUM, CONFIG, MUSIC).
//! - `mu_bg.png` (960×1626): the music room's background in the top 544
//!   rows, then the song titles, 30 rows each, from y 574: 35 in the left
//!   column (x 0), 32 in the right (x 480), in [`TRACKS`] order.

use image::{Rgba, RgbaImage, imageops};

use conv_core::ir::{
    CgGallery, CgViewer, ExtrasAction, ExtrasItem, ExtrasMenu, GallerySlot, GalleryTab, MusicRoom, MusicSong, Placed,
};

use super::assets::ArchiveAssets;
use super::convert::{Assets, Game, SCREEN_H, SCREEN_W};
use super::menus::Labels;

/// The tabs' names, in CGMODE's page order.
const HEROINES: [&str; 8] = ["渡来明日香", "山科京", "玉泉日和子", "千歳鳴", "伊丹亜衣", "羽田小鳩", "林田美咲", "その他"];
const THUMB_W: u32 = 120;
const THUMB_H: u32 = 68;
const COLUMNS: usize = 4;
const PER_PAGE: usize = 20;

/// The music room's songs (BGM.AWB IDs) in the order of `mu_bg.png`'s
/// titles: the eboot's table at 0x8116310c, read by its music room
/// (67 entries, 7 rows shown). A second table after it (34, 35, 29-33,
/// 36), with no titles in the picture, is left out.
const TRACKS: [u32; 67] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 14, 15, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 40, 41, 42, 43, 45, 47, 48, 49,
    50, 51, 53, 60, 61, 62, 63, 64, 46, 70, 71, 73, 74, 75, 76, 77, 78, 80, 81, 82, 83, 85, 52, 86, 87, 88, 89, 90, 91, 92, 93, 94,
    95,
];
/// The titles in the picture: rows of 30 from y 574, 35 on the left.
const TITLE_Y: u32 = 574;
const TITLE_H: u32 = 30;
const TITLES_LEFT: usize = 35;
/// Ours: the list panel stretched over the screen, three columns of 23
/// titles at 3/5 size in it.
const PANEL: (u32, u32, u32, u32) = (78, 238, 804, 238);
const PANEL_TO: (u32, u32, u32, u32) = (24, 80, 912, 420);
const MUSIC_ROWS: usize = 23;
const MUSIC_X: i32 = 44;
const MUSIC_DX: i32 = 292;
const MUSIC_Y: i32 = 86;
const MUSIC_DY: i32 = 18;

fn place(assets: &mut ArchiveAssets, name: &str, img: &RgbaImage, x: i32, y: i32) -> Placed {
    let image = assets.put_system_png(name, img);
    Placed { image, x, y, w: img.width() as i32, h: img.height() as i32 }
}

fn cells(parts: &[RgbaImage]) -> RgbaImage {
    let (w, h) = (parts[0].width(), parts[0].height());
    let mut out = RgbaImage::new(w * parts.len() as u32, h);
    for (i, p) in parts.iter().enumerate() {
        imageops::overlay(&mut out, p, (i as u32 * w) as i64, 0);
    }
    out
}

fn faded(img: &RgbaImage) -> RgbaImage {
    let mut img = img.clone();
    for px in img.pixels_mut() {
        px.0[3] = (px.0[3] as u32 * 35 / 100) as u8;
    }
    img
}

/// The title atlas's label in row `row` (normal, highlighted, faded).
fn title_label(title: &RgbaImage, row: u32) -> RgbaImage {
    let y = 545 + row * 36;
    let normal = imageops::crop_imm(title, 0, y, 170, 36).to_image();
    let hover = imageops::crop_imm(title, 170, y, 170, 36).to_image();
    cells(&[normal.clone(), hover, faded(&normal)])
}

pub fn extras(assets: &mut ArchiveAssets, labels: &Labels, game: &Game) -> Option<ExtrasMenu> {
    let album = assets.system_image("gm_bg.png")?;
    let parts = assets.system_image("gm_bg1.png")?;
    let title = assets.system_image("title_bg.png")?;
    let mut background = imageops::crop_imm(&album, 0, 0, SCREEN_W as u32, SCREEN_H as u32).to_image();
    // ALBUM's header apart; the Vita's button hints painted over.
    let header_rect = (24, 22, 120, 50);
    let plain = {
        let mut b = background.clone();
        super::menus::patch(&mut b, header_rect);
        b
    };
    let header = super::menus::difference(&background, &plain, header_rect);
    background = plain;
    super::menus::patch(&mut background, super::menus::HINTS);
    let background_path = assets.put_system_png("album_bg", &background);

    let item = |assets: &mut ArchiveAssets, name: &str, img: &RgbaImage, i: i32| place(assets, name, img, (SCREEN_W - 170) / 2, 170 + i * 50);
    let back_img = labels.button("戻る", 20.0);
    let items = vec![
        ExtrasItem { button: item(assets, "extras_album", &title_label(&title, 5), 0), action: ExtrasAction::Gallery },
        ExtrasItem { button: item(assets, "extras_config", &title_label(&title, 4), 1), action: ExtrasAction::Settings },
        ExtrasItem { button: item(assets, "extras_music", &title_label(&title, 7), 2), action: ExtrasAction::Music },
        ExtrasItem { button: place(assets, "extras_back", &back_img, (SCREEN_W - back_img.width() as i32 / 3) / 2, 400), action: ExtrasAction::Back },
    ];

    let gallery = gallery(assets, labels, game, &album, &parts, header);
    let music = music(assets, labels);
    Some(ExtrasMenu { background: background_path, items, gallery, replay: None, music })
}

/// A title tinted for the pointer (warm) or the song playing (blue).
fn tinted(img: &RgbaImage, colour: [u8; 3]) -> RgbaImage {
    let mut img = img.clone();
    for px in img.pixels_mut() {
        // The titles are dark letters on a white outline: the dark ones take the colour.
        let dark = 255 - px.0[0].min(px.0[1]).min(px.0[2]);
        for (c, &add) in px.0.iter_mut().zip(&colour) {
            *c = (*c as u32 + add as u32 * dark as u32 / 255).min(255) as u8;
        }
    }
    img
}

fn music(assets: &mut ArchiveAssets, labels: &Labels) -> Option<MusicRoom> {
    let atlas = assets.system_image("mu_bg.png")?;
    // The background with its list panel over the other panels and the
    // player's bar (painted out, softened); MUSIC stays.
    let mut room = imageops::crop_imm(&atlas, 0, 0, SCREEN_W as u32, SCREEN_H as u32).to_image();
    let bar = (376, 38, 520, 48);
    super::menus::patch(&mut room, bar);
    let soft = imageops::blur(&imageops::crop_imm(&room, bar.0 - 16, bar.1 - 16, bar.2 + 32, bar.3 + 32).to_image(), 8.0);
    imageops::replace(&mut room, &imageops::crop_imm(&soft, 16, 16, bar.2, bar.3).to_image(), bar.0 as i64, bar.1 as i64);
    let panel = stretched(&imageops::crop_imm(&atlas, PANEL.0, PANEL.1, PANEL.2, PANEL.3).to_image(), PANEL_TO.2, PANEL_TO.3);
    imageops::overlay(&mut room, &panel, PANEL_TO.0 as i64, PANEL_TO.1 as i64);
    let header = imageops::crop_imm(&room, 24, 24, 130, 48).to_image();
    let mut songs = Vec::new();
    for (i, &track) in TRACKS.iter().enumerate() {
        let (column, row) = if i < TITLES_LEFT { (0, i) } else { (1, i - TITLES_LEFT) };
        let title = imageops::crop_imm(&atlas, column * 480, TITLE_Y + row as u32 * TITLE_H, 480, TITLE_H).to_image();
        let title = trim(&title);
        let title = imageops::resize(&title, title.width() * 3 / 5, TITLE_H * 3 / 5, imageops::FilterType::Triangle);
        let file = assets.bgm(track)?;
        let (x, y) = (MUSIC_X + MUSIC_DX * (i / MUSIC_ROWS) as i32, MUSIC_Y + MUSIC_DY * (i % MUSIC_ROWS) as i32);
        let hover = tinted(&title, [230, 120, 20]);
        let button = place(assets, &format!("music{i}"), &cells(&[title.clone(), hover.clone()]), x, y);
        let on = tinted(&title, [40, 90, 230]);
        let playing = assets.put_system_png(&format!("music{i}_on"), &cells(&[on, hover]));
        songs.push(MusicSong { button, playing, file, progress: None });
    }
    let exit_img = labels.button("戻る", 20.0);
    let exit = cells(&[
        imageops::crop_imm(&exit_img, 0, 0, exit_img.width() / 3, exit_img.height()).to_image(),
        imageops::crop_imm(&exit_img, exit_img.width() / 3, 0, exit_img.width() / 3, exit_img.height()).to_image(),
    ]);
    let background = Some(assets.put_system_png("music_bg", &room));
    Some(MusicRoom { header: place(assets, "music_header", &header, 24, 24), exit: place(assets, "music_exit", &exit, 860, 506), songs, background })
}

/// A panel stretched to `w`×`h`: its corners kept, its edges stretched
/// along, the inside from its middle (clear of the scroll bar).
fn stretched(panel: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    const B: u32 = 16;
    let (pw, ph) = (panel.width(), panel.height());
    let part = |x, y, cw, ch, tw, th| imageops::resize(&imageops::crop_imm(panel, x, y, cw, ch).to_image(), tw, th, imageops::FilterType::Triangle);
    let mut out = RgbaImage::new(w, h);
    let (mw, mh) = (w - 2 * B, h - 2 * B);
    let pieces = [
        (part(0, 0, B, B, B, B), 0, 0),
        (part(pw - B, 0, B, B, B, B), w - B, 0),
        (part(0, ph - B, B, B, B, B), 0, h - B),
        (part(pw - B, ph - B, B, B, B, B), w - B, h - B),
        (part(B, 0, pw / 2, B, mw, B), B, 0),
        (part(B, ph - B, pw / 2, B, mw, B), B, h - B),
        (part(0, B, B, ph - 2 * B, B, mh), 0, B),
        (part(pw - B, B, B, ph - 2 * B, B, mh), w - B, B),
        (part(B, B, pw / 2, ph - 2 * B, mw, mh), B, B),
    ];
    for (img, x, y) in pieces {
        imageops::replace(&mut out, &img, x as i64, y as i64);
    }
    out
}

/// `img` without its transparent columns on the right.
fn trim(img: &RgbaImage) -> RgbaImage {
    let w = (0..img.width()).rev().find(|&x| (0..img.height()).any(|y| img.get_pixel(x, y).0[3] > 0)).map_or(1, |x| x + 1);
    imageops::crop_imm(img, 0, 0, w, img.height()).to_image()
}

fn gallery(assets: &mut ArchiveAssets, labels: &Labels, game: &Game, album: &RgbaImage, parts: &RgbaImage, header: RgbaImage) -> Option<CgGallery> {
    let no_image = imageops::resize(&imageops::crop_imm(parts, 4, 546, 184, 110).to_image(), THUMB_W, THUMB_H, imageops::FilterType::Triangle);
    let frame = imageops::resize(&imageops::crop_imm(parts, 196, 546, 186, 110).to_image(), THUMB_W, THUMB_H, imageops::FilterType::Triangle);
    let locked = assets.put_system_png("album_locked", &no_image);
    let card = |row: u32, i: u32| {
        let c = imageops::crop_imm(album, 1 + 96 * i, row, 94, 378).to_image();
        imageops::resize(&c, 47, 189, imageops::FilterType::Triangle)
    };
    let mut tabs = Vec::new();
    for (i, entries) in game.gallery.iter().enumerate().take(HEROINES.len()) {
        let name = HEROINES[i];
        let (x, y) = (24 + 50 * (i as i32 % 4), 90 + 200 * (i as i32 / 4));
        let button = place(assets, &format!("album_tab{i}"), &card(934, i as u32), x, y);
        let selected = assets.put_system_png(&format!("album_tab{i}_on"), &card(1318, i as u32));
        let title_img = labels.draw(name, 22.0, [255, 255, 255], [90, 130, 255], 255);
        let title = place(assets, &format!("album_title{i}"), &title_img, 430, 44);
        let viewer_img = labels.draw(name, 18.0, [255, 255, 255], [90, 130, 255], 255);
        let viewer_name = place(assets, &format!("album_name{i}"), &viewer_img, 0, 0);
        let mut slots = Vec::new();
        for (n, entry) in entries.iter().enumerate() {
            let cgs: Vec<(u32, String)> = entry.iter().filter_map(|&(tex, flag)| Some((flag + 1, assets.cg(tex)?.0))).collect();
            let Some(&(first, _)) = entry.first() else { continue };
            let thumb = thumbnail(assets, first)?;
            let mut lit = thumb.clone();
            imageops::overlay(&mut lit, &frame, 0, 0);
            let thumbnail = assets.put_system_png(&format!("album_thumb{i}_{n}"), &cells(&[thumb, lit]));
            slots.push(GallerySlot { thumbnail, cgs });
        }
        let pages = slots.chunks(PER_PAGE).take(4).map(|c| c.to_vec()).collect();
        tabs.push(GalleryTab { button, selected, title, viewer_name, pages });
    }
    let page_numbers = (1..=4)
        .map(|n| {
            let img = labels.button(&n.to_string(), 20.0);
            let two = cells(&[imageops::crop_imm(&img, 0, 0, img.width() / 3, img.height()).to_image(), imageops::crop_imm(&img, img.width() / 3, 0, img.width() / 3, img.height()).to_image()]);
            assets.put_system_png(&format!("album_page{n}"), &two)
        })
        .collect();
    let marker = labels.draw("─", 24.0, [255, 230, 160], [255, 190, 70], 255);
    let exit_img = labels.button("戻る", 20.0);
    let exit2 = cells(&[
        imageops::crop_imm(&exit_img, 0, 0, exit_img.width() / 3, exit_img.height()).to_image(),
        imageops::crop_imm(&exit_img, exit_img.width() / 3, 0, exit_img.width() / 3, exit_img.height()).to_image(),
    ]);
    // The viewer's panel: a translucent bar along the bottom.
    let bar = RgbaImage::from_pixel(SCREEN_W as u32, 40, Rgba([20, 30, 70, 170]));
    let viewer_button = |assets: &mut ArchiveAssets, name: &str, text: &str, x: i32| {
        let img = labels.button(text, 18.0);
        let two = cells(&[imageops::crop_imm(&img, 0, 0, img.width() / 3, img.height()).to_image(), imageops::crop_imm(&img, img.width() / 3, 0, img.width() / 3, img.height()).to_image()]);
        place(assets, name, &two, x, SCREEN_H - 36)
    };
    let viewer = CgViewer {
        panel: vec![place(assets, "album_panel", &bar, 0, SCREEN_H - 40)],
        name_x: 20,
        name_y: SCREEN_H - 34,
        previous: viewer_button(assets, "album_prev", "◀", 600),
        next: viewer_button(assets, "album_next", "▶", 660),
        hide: viewer_button(assets, "album_hide", "HIDE", 730),
        back: viewer_button(assets, "album_back", "戻る", 840),
    };
    Some(CgGallery {
        header: place(assets, "album_header", &header, 24, 22),
        exit: place(assets, "album_exit", &exit2, 870, 500),
        tabs,
        grid_x: 430,
        grid_y: 90,
        grid_dx: 128,
        grid_dy: 78,
        columns: COLUMNS,
        locked,
        page_label: place(assets, "album_page_label", &labels.draw("PAGE", 18.0, [255, 255, 255], [90, 130, 255], 255), 430, 492),
        page_numbers,
        pages_x: 500,
        pages_y: 488,
        page_dx: 40,
        page_marker: assets.put_system_png("album_marker", &marker),
        marker_dx: -2,
        marker_dy: 22,
        viewer,
    })
}

/// CG `tex` at thumbnail size.
fn thumbnail(assets: &mut ArchiveAssets, tex: u32) -> Option<RgbaImage> {
    let data = assets.cg_bytes(tex)?;
    let img = image::load_from_memory(data).ok()?.to_rgba8();
    Some(imageops::resize(&img, THUMB_W, THUMB_H, imageops::FilterType::Triangle))
}
