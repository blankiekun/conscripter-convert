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

use image::{Rgba, RgbaImage, imageops};

use conv_core::ir::{
    CgGallery, CgViewer, ExtrasAction, ExtrasItem, ExtrasMenu, GallerySlot, GalleryTab, Placed,
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
    Some(ExtrasMenu { background: background_path, items, gallery, replay: None, music: None })
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
