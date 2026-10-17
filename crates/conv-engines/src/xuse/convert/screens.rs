//! The game's screens, built from its own pictures: title, save menu,
//! button and top bars, settings, confirmation, backlog, extras.

use super::*;

impl AssetBank {
    /// The title screen from `HD/gd.arc` → `Title.004`: background (0) with
    /// the logo (10) and "COMPLETE" (14) on it, and the menu words, each a
    /// column of four states (normal, under the pointer, pressed, disabled).
    /// Positions measured on a screenshot of the original. The extra modes
    /// (cg 4, bgm 5, chapter 8, wss 13) appear after clearing the game and
    /// aren't converted yet; neither is the options menu.
    pub fn title_screen(&mut self) -> Result<TitleScreen, ConvertError> {
        let images = self.gaf_images("gd.arc", "Title.004", "title screen")?;
        let get = |n: usize| {
            let png = images.get(n).ok_or(ConvertError::MissingAsset { kind: "title screen image", number: n })?;
            image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .map(|i| i.to_rgba8())
                .map_err(|source| ConvertError::Image { what: format!("Title.004 #{n}"), source })
        };
        let mut background = get(0)?;
        // src: exe 0x4599a0 (the logo at (0, 171), "COMPLETE" at (100, 383)).
        image::imageops::overlay(&mut background, &get(10)?, 0, 171);
        image::imageops::overlay(&mut background, &get(14)?, 100, 383);
        let background_path = "system/title.png".to_string();
        self.save_image(&background_path, &background)?;

        let mut buttons = Vec::new();
        for (n, name, x, y, action) in [
            (6, "start", 631, 221, TitleAction::Start),
            (2, "load", 631, 250, TitleAction::Load),
            (7, "option", 631, 299, TitleAction::Settings),
            (3, "exit", 631, 370, TitleAction::Exit),
        ] {
            let states = get(n)?;
            let (w, h) = (states.width(), states.height() / 4);
            let cell = |i: u32| image::imageops::crop_imm(&states, 0, i * h, w, h).to_image();
            let strip = if action == TitleAction::Unavailable {
                cell(3)
            } else {
                let mut strip = image::RgbaImage::new(w * 2, h);
                image::imageops::replace(&mut strip, &cell(0), 0, 0);
                image::imageops::replace(&mut strip, &cell(1), i64::from(w), 0);
                strip
            };
            let image = format!("system/title_{name}.png");
            self.save_image(&image, &strip)?;
            buttons.push(TitleButton { image, x, y, action, ..Default::default() });
        }
        Ok(TitleScreen { background: background_path, fade_ms: TITLE_FADE_MS, buttons, ..Default::default() })
    }

    /// The extras menu and the CG gallery (see [`gallery`]). Positions
    /// measured on screenshots of the original.
    /// `label(file, name)`: the converted label of a scene replay entry
    /// ([`LabelNames::find`]); entries without one are left out.
    pub fn extras_menu(&mut self, label: impl Fn(&str, &str) -> Option<String>) -> Result<ExtrasMenu, ConvertError> {
        let title = self.gaf_rgba("gd.arc", "Title.004", "title screen")?;
        let common = self.gaf_rgba("gd.arc", "SubModeCommon.004", "extras")?;
        let viewer = self.gaf_rgba("gd.arc", "CgViewer.004", "CG viewer")?;
        let get = |images: &[image::RgbaImage], n: usize| {
            images.get(n).cloned().ok_or(ConvertError::MissingAsset { kind: "extras image", number: n })
        };
        let background = "system/extras.png".to_string();
        self.save_image(&background, &get(&common, 19)?)?;

        // Menu words: columns of four states (normal, under the pointer,
        // pressed, disabled).
        let mut items = Vec::new();
        // src: exe 0x4fb8a8; the heading's place 0x459b90.
        for (n, name, x, y, action) in [
            (12, "option", 625, 129, ExtrasAction::Settings),
            (4, "cg", 625, 221, ExtrasAction::Gallery),
            (5, "bgm", 626, 250, ExtrasAction::Music),
            (8, "chapter", 626, 283, ExtrasAction::Replay),
            (13, "wss", 626, 320, ExtrasAction::Unavailable),
            (3, "exit", 626, 384, ExtrasAction::Back),
        ] {
            let img = get(&title, n)?;
            // The heading is one picture; as our settings button it is its
            // own pointer state.
            let strip = match action {
                ExtrasAction::Settings => column_cells(&img, 1, &[0, 0, 0]),
                ExtrasAction::Unavailable => column_cells(&img, 4, &[3]),
                _ => column_cells(&img, 4, &[0, 1, 3]),
            };
            items.push(ExtrasItem { button: self.place(&format!("extras_{name}"), &strip, None, x, y)?, action });
        }

        // The thumbnails' pointer frame (#21, added to the screen) and the
        // locked slot (#20), both columns of four states.
        let frame = column_cells(&get(&common, 21)?, 4, &[1]);
        let locked = column_cells(&get(&common, 20)?, 4, &[0]);
        let locked = self.place("cg_locked", &locked, None, 0, 0)?.image;
        let mut tabs = Vec::new();
        // src: exe 0x4552c0 (tabs at x 624 from y 160, 33 px apart under
        // 6 items), 0x44dc90 (the viewer's name at (601, 545)); the
        // headings at (96, 97): shot (the user's, scaled).
        for (i, &(tab, label, heading, name)) in gallery::TAB_PICTURES.iter().enumerate() {
            let label_img = get(&common, label)?;
            let button = self.place(&format!("cg_tab{i}"), &column_cells(&label_img, 4, &[0, 1]), None, 624, 160 + 33 * i as i32)?;
            let selected = self.place(&format!("cg_tab{i}_on"), &column_cells(&label_img, 4, &[1]), None, 0, 0)?.image;
            let title = self.place(&format!("cg_title{i}"), &get(&common, heading)?, None, 96, 97)?;
            let viewer_name = self.place(&format!("cg_name{i}"), &get(&viewer, name)?, None, 601, 545)?;
            let mut pages = Vec::new();
            for (p, slots) in gallery::TABS[tab].iter().enumerate() {
                let sheet = self.arc_png("gd.arc", &format!("SubModeThumb-{:02}-{p}.png", tab + 1))?;
                let mut page = Vec::new();
                for (k, cgs) in slots.iter().enumerate() {
                    let (cx, cy) = (k as u32 % 5 * gallery::THUMB_W, k as u32 / 5 * gallery::THUMB_H);
                    let thumb = image::imageops::crop_imm(&sheet, cx, cy, gallery::THUMB_W, gallery::THUMB_H).to_image();
                    let strip = thumbnail_cells(&thumb, &frame);
                    let thumbnail = self.place(&format!("cg_thumb{i}_{p}_{k}"), &strip, None, 0, 0)?.image;
                    let mut pictures = Vec::new();
                    for &bit in *cgs {
                        let number = usize::from(bit) + 1;
                        pictures.push((number as u32, self.background(number)?.path));
                    }
                    page.push(GallerySlot { thumbnail, cgs: pictures });
                }
                pages.push(page);
            }
            tabs.push(GalleryTab { button, selected, title, viewer_name, pages });
        }
        let page_numbers = (0..7)
            .map(|p| self.place(&format!("cg_page{}", p + 1), &column_cells(&get(&common, 24 + p)?, 4, &[0, 1]), None, 0, 0).map(|p| p.image))
            .collect::<Result<_, _>>()?;
        // src: exe 0x4fbba0 (exit), 0x4fb970 (grid and page buttons),
        // 0x4585b0 (page label); header and page marker: shot (the
        // user's, scaled).
        let gallery = CgGallery {
            header: self.place("cg_header", &get(&common, 16)?, None, 621, 122)?,
            exit: self.place("cg_exit", &column_cells(&get(&common, 15)?, 4, &[0, 1]), None, 624, 470)?,
            tabs,
            grid_x: 44,
            grid_y: 142,
            grid_dx: 108,
            grid_dy: 92,
            columns: 5,
            locked,
            page_label: self.place("cg_page", &get(&common, 22)?, None, 31, 529)?,
            page_numbers,
            pages_x: 67,
            pages_y: 521,
            page_dx: 24,
            page_marker: self.place("cg_page_on", &column_cells(&get(&common, 23)?, 4, &[0]), None, 0, 0)?.image,
            marker_dx: 0,
            marker_dy: -3,
            viewer: CgViewer {
                // src: exe 0x44dc90: the panel at the viewer's origin
                // (461, 545, 0x44d290), its base at the window opacity
                // (0x51ed12, default 128 of 256), the name 140 px right.
                panel: vec![
                    self.place("cgv_base", &with_opacity(get(&viewer, 11)?, WINDOW_OPACITY_PERCENT), None, 461, 545)?,
                    self.place("cgv_lines", &get(&viewer, 12)?, None, 461, 545)?,
                ],
                name_x: 601,
                name_y: 545,
                // src: exe 0x4fe3c0 (from the viewer's origin (461, 545)).
                previous: self.place("cgv_prev", &column_cells(&get(&viewer, 8)?, 4, &[0, 1]), None, 721, 549)?,
                next: self.place("cgv_next", &column_cells(&get(&viewer, 7)?, 4, &[0, 1]), None, 737, 549)?,
                hide: self.place("cgv_hide", &column_cells(&get(&viewer, 6)?, 4, &[0, 1]), None, 756, 548)?,
                back: self.place("cgv_back", &column_cells(&get(&viewer, 5)?, 4, &[0, 1]), None, 742, 568)?,
            },
        };
        let replay = self.scene_replay(&common, &label)?;
        let music = self.music_room(&common)?;
        Ok(ExtrasMenu { background, items, gallery: Some(gallery), replay: Some(replay), music: Some(music), movies: None, menu_background: None })
    }

    /// The BGM room (see [`music`]); `common` is `SubModeCommon.004`.
    pub(super) fn music_room(&mut self, common: &[image::RgbaImage]) -> Result<MusicRoom, ConvertError> {
        let titles = self.gaf_rgba("gd.arc", "BgmMode.004", "BGM room")?;
        let get = |images: &[image::RgbaImage], n: usize| {
            images.get(n).cloned().ok_or(ConvertError::MissingAsset { kind: "BGM room image", number: n })
        };
        let mut songs = Vec::new();
        for (i, &track) in music::TRACKS.iter().enumerate() {
            let img = get(&titles, i)?;
            let (w, h) = (img.width() / 2, img.height() / 4);
            // Cells (normal, under the pointer) of the left or right column.
            let cells = |column: u32| {
                let mut out = image::RgbaImage::new(w * 2, h);
                for row in 0..2 {
                    let cell = image::imageops::crop_imm(&img, column * w, row * h, w, h).to_image();
                    image::imageops::replace(&mut out, &cell, (row * w).into(), 0);
                }
                out
            };
            let y = music::Y + music::DY * i as i32;
            let button = self.place(&format!("music{i}"), &cells(0), None, music::X, y)?;
            let playing = self.place(&format!("music{i}_on"), &cells(1), None, music::X, y)?.image;
            let progress = (i == music::LOCKED).then_some(music::LOCK_BIT);
            songs.push(MusicSong { button, playing, file: self.bgm(track)?, progress });
        }
        // Header and exit as in the CG gallery.
        Ok(MusicRoom {
            header: self.place("music_header", &get(common, 17)?, None, 621, 122)?,
            exit: self.place("music_exit", &column_cells(&get(common, 15)?, 4, &[0, 1]), None, 624, 470)?,
            songs,
            background: None,
            player: None,
        })
    }

    /// The scene replay (see [`replay`]); `common` is `SubModeCommon.004`
    /// (its #19 is the extras' background).
    pub(super) fn scene_replay(&mut self, common: &[image::RgbaImage], label: &impl Fn(&str, &str) -> Option<String>) -> Result<SceneReplay, ConvertError> {
        let parts = self.gaf_rgba("gd.arc", "SceneList.004", "scene replay")?;
        let get = |images: &[image::RgbaImage], n: usize| {
            images.get(n).cloned().ok_or(ConvertError::MissingAsset { kind: "scene replay image", number: n })
        };
        let header = get(common, 18)?;
        let exit = get(common, 15)?;
        // The menu as it stands under an open list, darkened by the band.
        let mut menu = get(common, 19)?;
        image::imageops::overlay(&mut menu, &header, 621, 122);
        image::imageops::overlay(&mut menu, &column_cells(&exit, 4, &[0]), 624, 470);
        let band = get(&parts, 0)?;
        let darken = |img: &mut image::RgbaImage, x0: i32, y0: i32| {
            for (x, y, px) in img.enumerate_pixels_mut() {
                let (bx, by) = (x as i32 + x0, y as i32 + y0);
                let inside = bx >= 0 && by >= 0 && band.get_pixel_checked(bx as u32, by as u32).is_some_and(|b| b.0[3] > 0);
                if inside {
                    for c in &mut px.0[..3] {
                        *c = c.saturating_sub(replay::DARKEN);
                    }
                }
            }
        };
        let mut lists = Vec::new();
        for (i, l) in replay::LISTS.iter().enumerate() {
            let img = get(common, l.picture)?;
            let (x, y) = (replay::MENU_X, replay::MENU_Y + replay::MENU_DY * i as i32);
            image::imageops::overlay(&mut menu, &column_cells(&img, 4, &[0]), x.into(), y.into());
            let mut strip = column_cells(&img, 4, &[0, 1, 3]);
            if !l.disabled {
                // Made up: the normal state at half opacity.
                let w = img.width();
                image::imageops::replace(&mut strip, &with_opacity(column_cells(&img, 4, &[0]), 50), (2 * w).into(), 0);
            }
            let button = self.place(&format!("replay_list{i}"), &strip, None, x, y)?;
            let mut on = column_cells(&img, 4, &[1]);
            darken(&mut on, x, y);
            let selected = self.place(&format!("replay_list{i}_on"), &on, None, x, y)?;
            let scenes = l
                .scenes
                .iter()
                .filter_map(|&(scene, highlight, name)| {
                    Some(ReplayScene { scene: scene.into(), title: replay::title(name).to_string(), label: label(l.file, name)?, highlight })
                })
                .collect();
            lists.push(ReplayList { button, selected, scenes });
        }
        darken(&mut menu, 0, 0);
        let list_background = "system/replay_back.png".to_string();
        self.save_image(&list_background, &menu)?;
        // The row under the pointer is added to the screen: as alpha, its
        // brightness.
        let selector = get(&parts, 3)?;
        let mut cells = column_cells(&selector, 4, &[0, 1]);
        let w = selector.width();
        for (x, _, px) in cells.enumerate_pixels_mut() {
            let a = px.0[..3].iter().copied().max().unwrap_or(0);
            if x < w || a == 0 {
                px.0 = [0; 4];
            } else {
                for c in &mut px.0[..3] {
                    *c = (u32::from(*c) * 255 / u32::from(a)) as u8;
                }
                px.0[3] = a;
            }
        }
        Ok(SceneReplay {
            header: self.place("replay_header", &header, None, 621, 122)?,
            exit: self.place("replay_exit", &column_cells(&exit, 4, &[0, 1]), None, 624, 470)?,
            lists,
            list_background,
            frame: self.place("replay_frame", &get(&parts, 1)?, None, 0, 0)?,
            // src: exe 0x4529c0.
            close: self.place("replay_close", &column_cells(&get(&parts, 2)?, 4, &[0, 1]), None, 715, 577)?,
            selector: self.place("replay_row", &cells, None, 0, 0)?.image,
            new_mark: self.place("replay_new", &get(&parts, 4)?, None, 0, 0)?.image,
            rows: replay::ROWS,
            columns: replay::COLUMNS,
            column_dx: replay::COLUMN_DX,
            row_dy: replay::ROW_DY,
            row_y: replay::ROW_Y,
            selector_x: replay::SELECTOR_X,
            new_x: replay::NEW_X,
            text_x: replay::TEXT_X,
            text_y: replay::TEXT_Y,
            font_size: replay::FONT_SIZE,
            text_color: replay::TEXT_COLOR.to_string(),
            highlight_color: replay::H_COLOR.to_string(),
            leave_label: "シーン選択に戻る".to_string(),
            leave_question: "シーン選択に戻りますか？".to_string(),
        })
    }

    /// A loose PNG inside the XARC archive `HD/{arc}`.
    pub(super) fn arc_png(&self, arc: &str, file: &str) -> Result<image::RgbaImage, ConvertError> {
        let path = self.game.join("HD").join(arc);
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let xarc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let entry = xarc.entries.iter().find(|e| e.name == file).ok_or(ConvertError::MissingAsset { kind: "extras image", number: 0 })?;
        image::load_from_memory_with_format(xarc.data(entry), image::ImageFormat::Png)
            .map(|i| i.to_rgba8())
            .map_err(|source| ConvertError::Image { what: format!("{arc}/{file}"), source })
    }

    /// The save/load screen from `HD/wnd.arc` → `Save.004`: full-screen base
    /// (2, blended like the text window) and frame (3), headers (11 load, 12
    /// save), buttons (4 close, 5 delete, 6 load, 8 save; columns of four
    /// states: normal, under the pointer, pressed, disabled), page numbers
    /// (7, the same four states), current-page marker (1), row cursor (9,
    /// opaque, black where nothing is added: the game blends it
    /// additively), "New" icon (10). Positions measured on a screenshot of
    /// the original and on the frame's thumbnail boxes.
    pub fn save_menu(&mut self) -> Result<SaveMenu, ConvertError> {
        let images = self.gaf_images("wnd.arc", "Save.004", "save menu")?;
        let get = |n: usize| {
            let png = images.get(n).ok_or(ConvertError::MissingAsset { kind: "save menu image", number: n })?;
            image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .map(|i| i.to_rgba8())
                .map_err(|source| ConvertError::Image { what: format!("Save.004 #{n}"), source })
        };
        let placed = |this: &Self, name: &str, img: &image::RgbaImage, x: i32, y: i32| {
            let image = format!("system/save_{name}.png");
            this.save_image(&image, img).map(|()| Placed { image, x, y, w: img.width() as i32, h: img.height() as i32 })
        };
        // Cells `which` of a column of four states, side by side.
        let cells = |img: &image::RgbaImage, which: &[u32]| {
            let (w, h) = (img.width(), img.height() / 4);
            let mut strip = image::RgbaImage::new(w * which.len() as u32, h);
            for (i, &c) in which.iter().enumerate() {
                let cell = image::imageops::crop_imm(img, 0, c * h, w, h).to_image();
                image::imageops::replace(&mut strip, &cell, i64::from(w) * i as i64, 0);
            }
            strip
        };
        let button = |this: &Self, name: &str, n: usize, x: i32, y: i32| placed(this, name, &cells(&get(n)?, &[0, 1, 3]), x, y);

        let pages_strip = get(7)?;
        let (strip_x, strip_y) = (458, 480);
        let cell_h = pages_strip.height() / 4;
        // Cut between the numbers.
        let bounds = [0, 31, 67, 98, 136, 169, 203, 237, 271, 303, pages_strip.width()];
        let marker = get(1)?;
        let marker_cell = image::imageops::crop_imm(&marker, 0, 0, marker.width(), marker.height() / 4).to_image();
        let page_marker = placed(self, "page_marker", &marker_cell, 0, 0)?;
        let mut pages = Vec::new();
        for (i, w) in bounds.windows(2).enumerate() {
            let column = image::imageops::crop_imm(&pages_strip, w[0], 0, w[1] - w[0], cell_h * 4).to_image();
            let x = strip_x + w[0] as i32;
            let button = placed(self, &format!("page{}", i + 1), &cells(&column, &[0, 1, 2]), x, strip_y)?;
            let centre = x + (w[1] - w[0]) as i32 / 2;
            pages.push(PageButton { button, marker_x: centre - marker_cell.width() as i32 / 2, marker_y: strip_y });
        }

        // Row cursor: under the pointer it is drawn with alpha (brightness),
        // leaving the thumbnail clear; the selected row is added.
        let thumbnail = Rect { x: 14, y: 10, w: 100, h: 75 };
        let cursor = get(9)?;
        let mut hover = cells(&cursor, &[0, 1]);
        let row_w = cursor.width();
        for (x, y, px) in hover.enumerate_pixels_mut() {
            let in_thumb = x >= row_w + thumbnail.x as u32 - 1
                && x <= row_w + (thumbnail.x + thumbnail.w) as u32
                && y >= thumbnail.y as u32 - 1
                && y <= (thumbnail.y + thumbnail.h) as u32;
            let a = px.0[..3].iter().copied().max().unwrap_or(0);
            if x < row_w || in_thumb || a == 0 {
                px.0 = [0; 4];
            } else {
                for c in &mut px.0[..3] {
                    *c = (u32::from(*c) * 255 / u32::from(a)) as u8;
                }
                px.0[3] = a;
            }
        }
        let row_hover = placed(self, "row_hover", &hover, 0, 0)?;
        let row_selected = placed(self, "row_selected", &cells(&cursor, &[2]), 0, 0)?;

        Ok(SaveMenu {
            base: placed(self, "base", &get(2)?, 0, 0)?,
            base_opacity: (255 * WINDOW_OPACITY_PERCENT / 100) as u8,
            frame: placed(self, "frame", &get(3)?, 0, 0)?,
            // src: exe 0x46afd0.
            save_header: placed(self, "header_save", &get(12)?, 627, 0)?,
            load_header: placed(self, "header_load", &get(11)?, 627, 0)?,
            delete: button(self, "delete", 5, 512, 531)?,
            save: button(self, "save", 8, 615, 531)?,
            load: button(self, "load", 6, 615, 531)?,
            close: button(self, "close", 4, 702, 531)?,
            pages,
            page_marker,
            // src: exe 0x469850 (rows of 92 px, the cursor from y 44).
            rows: 5,
            row_y: 44,
            row_pitch: 92,
            row_hover,
            row_selected,
            thumbnail,
            // guess: the exe puts "New" after the slot's text (0x46ac50);
            // the text: shot (the user's, scaled).
            new_icon: placed(self, "new", &get(10)?, 50, 67)?,
            text_x: 124,
            text_y: 11,
            font_size: 14,
            line_height: 15,
            // src: exe 0x46ac50 (the preview at (389, 136)).
            preview: Rect { x: 389, y: 136, w: 400, h: 300 },
            text_width: 0,
            // The scene behind is blurred slightly (measured under wine:
            // Gaussian σ about 1): a 3×3 mean, in SmoothCrossFade's slot.
            backdrop: Some((SMOOTH_BLUR_SLOT, [FILTER_BLUR, FILTER_BLEND_COPY, 256, 1, 1, 0])),
            engine_screen: true,
        })
    }

    /// The bottom button bar from `HD/wnd.arc` → `SysTray.004`: rows of
    /// eight states side by side (normal, under the pointer, pressed,
    /// disabled, then the same four while the mode is on). Base and frame
    /// (0, 1) are empty. Positions measured on a screenshot of the
    /// original, where the bar runs along the bottom edge. The lock button
    /// (9) unpins the bars (filmed under wine): the top bars hide and this
    /// one shows only while the pointer is on it (below y 573); its open
    /// padlock (cells 4-7) is the pinned default, the closed one unpinned.
    pub fn button_bar(&mut self) -> Result<ButtonBar, ConvertError> {
        let images = self.gaf_images("wnd.arc", "SysTray.004", "button bar")?;
        let mut buttons = Vec::new();
        for (n, name, x, y, action) in [
            (4, "auto", 365, 581, BarAction::Auto),
            (5, "skip", 419, 581, BarAction::Skip),
            (6, "log", 473, 581, BarAction::Log),
            (2, "save", 550, 581, BarAction::Save),
            (3, "load", 607, 581, BarAction::Load),
            (7, "setting", 682, 581, BarAction::Settings),
            (8, "hide", 756, 583, BarAction::HideWindow),
            // src: exe 0x51b610 (the bar's hot area from y 581 - 10),
            // 0x4a47e0 (a 100 ms fade).
            (9, "lock", 304, 581, BarAction::Lock { reveal_y: 571, fade_ms: 100 }),
        ] {
            let png = images.get(n).ok_or(ConvertError::MissingAsset { kind: "button bar image", number: n })?;
            let states = image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .map(|i| i.to_rgba8())
                .map_err(|source| ConvertError::Image { what: format!("SysTray.004 #{n}"), source })?;
            let (w, h) = (states.width() / 8, states.height());
            let strip = |file: &str, cells: &[u32]| -> Result<Placed, ConvertError> {
                let mut out = image::RgbaImage::new(w * cells.len() as u32, h);
                for (i, &c) in cells.iter().enumerate() {
                    let cell = image::imageops::crop_imm(&states, c * w, 0, w, h).to_image();
                    image::imageops::replace(&mut out, &cell, i64::from(w) * i as i64, 0);
                }
                let image = format!("system/bar_{file}.png");
                self.save_image(&image, &out)?;
                Ok(Placed { image, x, y, w: out.width() as i32, h: out.height() as i32 })
            };
            let image = match action {
                BarAction::Unavailable => strip(name, &[3])?,
                BarAction::Lock { .. } => strip(name, &[4, 5, 7])?,
                _ => strip(name, &[0, 1, 3])?,
            };
            let active = match action {
                BarAction::Auto | BarAction::Skip => Some(strip(&format!("{name}_on"), &[4, 5, 7])?),
                BarAction::Lock { .. } => Some(strip(&format!("{name}_on"), &[0, 1, 3])?),
                _ => None,
            };
            buttons.push(BarButton { image, active, action });
        }
        Ok(ButtonBar { buttons, top: Some(self.top_bars()?), ..Default::default() })
    }

    /// The images of a GAF4 container in `HD/{arc}`, decoded.
    pub(super) fn gaf_rgba(&self, arc: &str, file: &str, kind: &'static str) -> Result<Vec<image::RgbaImage>, ConvertError> {
        self.gaf_images(arc, file, kind)?
            .iter()
            .enumerate()
            .map(|(n, png)| {
                image::load_from_memory_with_format(png, image::ImageFormat::Png)
                    .map(|i| i.to_rgba8())
                    .map_err(|source| ConvertError::Image { what: format!("{file} #{n}"), source })
            })
            .collect()
    }

    /// Writes `img` (cells `which` of a row of `count` side by side, or all
    /// of it) as `system/{name}.png` at (x, y).
    pub(super) fn place(&self, name: &str, img: &image::RgbaImage, cells: Option<(u32, &[u32])>, x: i32, y: i32) -> Result<Placed, ConvertError> {
        let out = match cells {
            None => img.clone(),
            Some((count, which)) => {
                let w = img.width() / count;
                let mut out = image::RgbaImage::new(w * which.len() as u32, img.height());
                for (i, &c) in which.iter().enumerate() {
                    let cell = image::imageops::crop_imm(img, c * w, 0, w, img.height()).to_image();
                    image::imageops::replace(&mut out, &cell, i64::from(w) * i as i64, 0);
                }
                out
            }
        };
        let image = format!("system/{name}.png");
        self.save_image(&image, &out)?;
        Ok(Placed { image, x, y, w: out.width() as i32, h: out.height() as i32 })
    }

    /// The settings screen. The original's settings are a Windows dialog,
    /// so this one is made of the game's parts: the backlog's base
    /// (`LogWnd.004` #0) and close button (#5, four states side by side),
    /// and the title screen's "option" word (`Title.004` #12).
    pub fn settings_menu(&mut self) -> Result<SettingsMenu, ConvertError> {
        let log = self.gaf_rgba("wnd.arc", "LogWnd.004", "backlog")?;
        let title = self.gaf_rgba("gd.arc", "Title.004", "title screen")?;
        let get = |images: &[image::RgbaImage], n: usize| {
            images.get(n).cloned().ok_or(ConvertError::MissingAsset { kind: "menu image", number: n })
        };
        let row = |label: &str, setting| SettingRow { label: label.to_string(), setting };
        // ours: the layout, sizes and opacity (the original has a dialog).
        Ok(SettingsMenu {
            base: self.place("menu_base", &darker(get(&log, 0)?), None, 0, 0)?,
            base_opacity: 210,
            header: self.place("settings_header", &get(&title, 12)?, None, 40, 20)?,
            close: self.place("menu_close", &get(&log, 5)?, Some((4, &[0, 1, 3])), 700, 540)?,
            rows: vec![
                row("ＢＧＭ", Setting::BgmVolume),
                row("効果音", Setting::SeVolume),
                row("音声", Setting::VoiceVolume),
                row("文字速度", Setting::TextSpeed),
                // src: exe 0x46d000 (level 5 is the default auto wait):
                // 2.5 s a page plus 20 ms a character (0x476f50).
                row("オート速度", Setting::AutoWait { page_step: 500, char_step: 4 }),
                row("ウィンドウ透過", Setting::WindowTransparency),
            ],
            // Labels and levels share a text line: level_x - label_x is
            // ten characters of 22 + 1 px.
            label_x: 150,
            minus_x: 340,
            level_x: 380,
            plus_x: 680,
            y: 140,
            pitch: 60,
            font_size: 22,
            title_label: "タイトルに戻る".into(),
            quit_label: "ゲームを終了".into(),
            exits_y: 540,
            tab_labels: vec!["音量・速度".into(), "音声".into(), "その他".into()],
            tabs_x: 300,
            tabs_y: 34,
            voices: VOICE_TOGGLES
                .iter()
                .map(|(label, characters)| VoiceToggle { label: label.to_string(), characters: characters.to_vec() })
                .collect(),
            voice_column_x: 440,
            // 15 switches in one column above the exits at 540.
            voice_pitch: 26,
            skip_unread_label: Some("未読テキストもスキップする".into()),
            pictures: None,
            defaults: Vec::new(),
            switches_off: Vec::new(),
            engine_screen: true,
            // The original's マウス and 確認メッセージ pages (labels as there).
            switches: [
                ("マウスホイールでテキストを進める", Switch::WheelReadsOn),
                ("マウスホイールで履歴を閉じる", Switch::WheelClosesLog),
                ("セーブ確認", Switch::ConfirmSave),
                ("ロード確認", Switch::ConfirmLoad),
                ("セーブデータの削除確認", Switch::ConfirmDelete),
                ("タイトルに戻る確認", Switch::ConfirmTitle),
                ("ゲーム終了確認", Switch::ConfirmQuit),
                // The 音声 page's; on by default (0x51ed59, 0x46d000).
                ("次の音声まで停止しない", Switch::VoiceKeepsPlaying),
                // The サウンド page's ミュート boxes (unticked by default).
                ("ＢＧＭを鳴らす", Switch::BgmOn),
                ("効果音を鳴らす", Switch::SeOn),
                ("音声を鳴らす", Switch::VoiceOn),
                // The その他, テキスト and 確認メッセージ pages' (all on by
                // default, as filmed under wine).
                ("画面エフェクトを使用", Switch::Effects),
                ("セーブファイルのプレビューを表示", Switch::SavePreview),
                ("文字装飾", Switch::TextShadow),
                ("チャプタ履歴の上書き確認", Switch::ConfirmChapter),
            ]
            .into_iter()
            .map(|(label, switch)| SettingSwitch { label: label.into(), switch })
            .collect(),
        })
    }

    /// The yes/no dialog from `HD/wnd.arc` → `YesNoDialog.004`: base (0,
    /// translucent like the text window) and frame (1), button base (2)
    /// and frame (3), four states side by side: normal, under the pointer,
    /// pressed, disabled. The images have no words; the messages are the
    /// executable's.
    pub fn confirm_dialog(&mut self) -> Result<ConfirmDialog, ConvertError> {
        let parts = self.gaf_rgba("wnd.arc", "YesNoDialog.004", "yes/no dialog")?;
        let get = |n: usize| parts.get(n).cloned().ok_or(ConvertError::MissingAsset { kind: "yes/no dialog image", number: n });
        let base = blend_layers(get(0)?, &get(1)?);
        let mut button = get(2)?;
        image::imageops::overlay(&mut button, &get(3)?, 0, 0);
        let (x, y) = ((800 - base.width() as i32) / 2, (600 - base.height() as i32) / 2);
        let bw = (button.width() / 4) as i32;
        let by = y + base.height() as i32 - button.height() as i32 - 12;
        let cells: &[u32] = &[0, 1, 3];
        let yes = self.place("dialog_button", &button, Some((4, cells)), 400 - bw - 16, by)?;
        let no = Placed { x: 400 + 16, ..yes.clone() };
        Ok(ConfirmDialog {
            base: self.place("dialog_base", &base, None, x, y)?,
            yes,
            no,
            yes_label: "はい".into(),
            no_label: "いいえ".into(),
            // guess: text_y (no shot of the original's dialog).
            text_y: 16,
            // src: exe 0x4a6410 (16 px, weight 600, for the message and
            // the button words).
            font_size: 16,
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

    /// The backlog from `HD/wnd.arc` → `LogWnd.004`: base (0, an opaque
    /// gradient, drawn mostly opaque), frame (1, "back log »"), scroll track
    /// (2), down and up arrows (3, 4) and close (5), the buttons four states
    /// side by side. Layout is ours (no screenshot of the original's).
    pub fn backlog(&mut self) -> Result<Backlog, ConvertError> {
        let log = self.gaf_rgba("wnd.arc", "LogWnd.004", "backlog")?;
        let get = |n: usize| log.get(n).cloned().ok_or(ConvertError::MissingAsset { kind: "backlog image", number: n });
        let cells: &[u32] = &[0, 1, 3];
        Ok(Backlog {
            // src: exe 0x41fc50 (the base at the window opacity, 0x51ed12).
            base: self.place("log_base", &get(0)?, None, 0, 0)?,
            base_opacity: (255 * WINDOW_OPACITY_PERCENT / 100) as u8,
            frame: self.place("log_frame", &get(1)?, None, 0, 0)?,
            // ours: the layout (Status, "Accuracy").
            track: self.place("log_track", &get(2)?, None, 745, 40)?,
            up: self.place("log_up", &get(4)?, Some((4, cells)), 760, 44)?,
            down: self.place("log_down", &get(3)?, Some((4, cells)), 760, 546)?,
            close: self.place("menu_close", &get(5)?, Some((4, cells)), 729, 5)?,
            area: Rect { x: 30, y: 44, w: 700, h: 516 },
            font_size: 16,
            line_height: 24,
            columns: 41,
        })
    }

    /// The top bars: `HD/wnd.arc` → `ChapterBar.004` (base, frame: "chapter
    /// »") on the left and `BlogStackBar.004` on the right (bar 0/1, drop-down
    /// 2/3, exit 4/5, next 6/7, prev 8/9, list row 10/11 and its cursor 12,
    /// depth box 13/14, digits 15; buttons and rows are four states side by
    /// side: normal, pointer, pressed, disabled; base and frame layers
    /// separately). Positions measured on screenshots of the original
    /// (frame layers matched to within a pixel).
    pub(super) fn top_bars(&mut self) -> Result<TopBars, ConvertError> {
        let blog = self.gaf_images("wnd.arc", "BlogStackBar.004", "blog bar")?;
        let chapter = self.gaf_images("wnd.arc", "ChapterBar.004", "chapter bar")?;
        let load = |images: &[Vec<u8>], n: usize| {
            let png = images.get(n).ok_or(ConvertError::MissingAsset { kind: "top bar image", number: n })?;
            image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .map(|i| i.to_rgba8())
                .map_err(|source| ConvertError::Image { what: format!("top bar #{n}"), source })
        };
        // Layers `base`/`base + 1`, cut into `count` cells side by side;
        // `cells` of them in that order, each with `mark` drawn over it.
        // `w` of the result is one cell's width.
        let layered = |this: &Self,
                       images: &[Vec<u8>],
                       base: usize,
                       cells: &[u32],
                       count: u32,
                       mark: Option<(&image::RgbaImage, i64, i64)>,
                       name: &str,
                       (x, y): (i32, i32)| {
            let img = blend_layers(load(images, base)?, &load(images, base + 1)?);
            let (w, h) = (img.width() / count, img.height());
            let mut out = image::RgbaImage::new(w * cells.len() as u32, h);
            for (i, &c) in cells.iter().enumerate() {
                let mut cell = image::imageops::crop_imm(&img, c * w, 0, w, h).to_image();
                if let Some((m, mx, my)) = mark {
                    image::imageops::overlay(&mut cell, m, mx, my);
                }
                image::imageops::replace(&mut out, &cell, i64::from(w) * i as i64, 0);
            }
            let image = format!("system/{name}.png");
            this.save_image(&image, &out)?;
            Ok::<_, ConvertError>(Placed { image, x, y, w: w as i32, h: h as i32 })
        };
        // src: exe 0x496fe0 (scene bar at (416, 0), list at (778, 5)); the
        // rest: shot (the user's, scaled), and #223's checks; row colours:
        // guess.
        let digit_strip = load(&blog, 15)?;
        let dw = digit_strip.width() / 10;
        let mut digits = Vec::new();
        for d in 0..10 {
            let img = image::imageops::crop_imm(&digit_strip, d * dw, 0, dw, digit_strip.height()).to_image();
            let image = format!("system/digit{d}.png");
            self.save_image(&image, &img)?;
            digits.push(image);
        }
        let all = &[0, 1, 2, 3];
        // A row: cursor (») at (9, 5) on the current level's, title at
        // (19, 4); rows push the level box down by their height.
        let (row_x, row_y) = (531, 30);
        let cursor = load(&blog, 12)?;
        Ok(TopBars {
            chapter_bar: layered(self, &chapter, 0, &[0], 1, None, "chapter_bar", (0, 0))?,
            chapter_text: TextSpot { x: 72, y: 7, font_size: 12, columns: 21 },
            scene_bar: layered(self, &blog, 0, &[0], 1, None, "scene_bar", (416, 0))?,
            scene_text: TextSpot { x: 524, y: 6, font_size: 14, columns: 18 },
            root_scene: "root scene".into(),
            level_box: layered(self, &blog, 13, &[0], 1, None, "scene_depth", (610, 30))?,
            digits,
            depth_at: (664, 34),
            max_depth_at: (685, 34),
            back: layered(self, &blog, 8, all, 4, None, "scene_prev", (708, 30))?,
            forward: layered(self, &blog, 6, all, 4, None, "scene_next", (734, 30))?,
            exit: layered(self, &blog, 4, all, 4, None, "scene_exit", (764, 30))?,
            list: layered(self, &blog, 2, all, 4, None, "scene_list", (778, 5))?,
            row: layered(self, &blog, 10, all, 4, None, "scene_row", (row_x, row_y))?,
            current_row: layered(self, &blog, 10, all, 4, Some((&cursor, 9, 5)), "scene_row_current", (row_x, row_y))?,
            row_pitch: 26,
            row_text: TextSpot { x: row_x + 19, y: row_y + 4, font_size: 14, columns: 17 },
            row_color: "#A0A0A0".into(),
            current_row_color: "#FFFFFF".into(),
            chapter_history: Some(self.chapter_history()?),
        })
    }

    /// The chapter history (`wnd.arc` → `ChapterHistory.004`: #0 base, #1
    /// frame, #2 and #3 the close button's base and lettering, four states
    /// of 21 px stacked, #4 the current chapter's bracket, #5 the row
    /// highlight, additive, four states of 25 px stacked). Layout and
    /// opacity measured on screenshots of the original under wine: rows 24
    /// px apart from (16, 83), columns 258 px apart, 19 rows to a column.
    pub(super) fn chapter_history(&mut self) -> Result<ChapterHistory, ConvertError> {
        let images = self.gaf_rgba("wnd.arc", "ChapterHistory.004", "chapter history")?;
        let get = |n: usize| images.get(n).cloned().ok_or(ConvertError::MissingAsset { kind: "chapter history image", number: n });
        let (close_base, close_text) = (get(2)?, get(3)?);
        let (cw, ch) = (close_base.width(), close_base.height() / 4);
        // Normal, under the pointer, disabled: the lettering over the base.
        let mut close = image::RgbaImage::new(cw * 3, ch);
        for (i, state) in [0u32, 1, 3].into_iter().enumerate() {
            let mut cell = image::imageops::crop_imm(&close_base, 0, state * ch, cw, ch).to_image();
            image::imageops::overlay(&mut cell, &image::imageops::crop_imm(&close_text, 0, state * ch, cw, ch).to_image(), 0, 0);
            image::imageops::replace(&mut close, &cell, i64::from(cw) * i as i64, 0);
        }
        // The highlight is added to the screen: as alpha, its brightest
        // channel is the opacity.
        let cursor = get(5)?;
        let mut hover = image::imageops::crop_imm(&cursor, 0, cursor.height() / 4, cursor.width(), cursor.height() / 4).to_image();
        for p in hover.pixels_mut() {
            let a = p[0].max(p[1]).max(p[2]);
            if a == 0 {
                *p = image::Rgba([0, 0, 0, 0]);
            } else {
                let scale = |c: u8| (u32::from(c) * 255 / u32::from(a)) as u8;
                *p = image::Rgba([scale(p[0]), scale(p[1]), scale(p[2]), a]);
            }
        }
        Ok(ChapterHistory {
            base: self.place("chapters_base", &get(0)?, None, 0, 0)?,
            // src: exe 0x468be0 (the base at the window opacity, 0x51ed12).
            base_opacity: (255 * WINDOW_OPACITY_PERCENT / 100) as u8,
            frame: self.place("chapters_frame", &get(1)?, None, 0, 0)?,
            // src: shot (wine), for the rest of the layout.
            close: self.place("chapters_close", &close, None, 715, 576)?,
            hover: self.place("chapters_hover", &hover, None, 0, 0)?,
            current: self.place("chapters_current", &get(4)?, None, -16, 0)?,
            x: 16,
            y: 83,
            row_pitch: 24,
            column_pitch: 258,
            rows: 19,
            columns: 3,
            row_width: 256,
            text_dx: 5,
            text_dy: 5,
            font_size: 13,
            question: ("チャプタ“".into(), "”に移動します。よろしいですか？".into()),
        })
    }
}
