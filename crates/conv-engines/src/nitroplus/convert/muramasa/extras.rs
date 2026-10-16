//! 装甲悪鬼村正's extras, 特別付加目録 (`extra_*.nss`), built from its own
//! pictures (`cg/sys/extra/`) with the places and times of its scripts, as
//! filmed in the original (`docs/games/muramasa.md`, "Extras").

use std::collections::BTreeMap;

use conv_core::ir::{ExtrasMode, GalleryCategory, PictureExtras, PictureExtrasItem, PictureGallery, PictureReplay, PictureSlot, Placed, ReplayEntry, RoomTrack, RoomVideo, SlotCg, SoundRoom, Turntable, TurntableEntry};

use super::super::{Assets, Replay, Story, SCREEN_H, SCREEN_W};
use crate::nitroplus::nss::{Expr, Item, NssFile, StmtKind};

/// Set at the end of any route (the end roll) and when a game starts after
/// one: the gallery, the recollection and the sound room open.
pub const CLEARED: &str = "#ClearG";

/// The screen-effect slot that holds the recollection's list over a
/// scene's first pictures until `RecoIn` fades it out.
pub const HOLD_SLOT: u32 = 4;

/// The menu (`extra_menu.nss`, `ExtraSet`): MENU背景画像 with five buttons;
/// 画像, 回想 and 音楽 drawn at 200/1000 and not clickable until
/// [`CLEARED`]. 鎧_装着01 under the pointer, 装着06 on opening a mode,
/// 剣戟弾く01 on leaving (`決定サウンズ`, made by the title); the screen
/// cross-fades over 500 ms (`ExtraFade`).
pub fn extras(a: &mut dyn Assets, story: &Story) -> Option<PictureExtras> {
    const DIR: &str = "cg/sys/extra/main";
    let se = |a: &mut dyn Assets, name: &str| a.sound(&format!("sound/se/{name}"));
    let cleared = story.flags.get(CLEARED).copied();
    let mut items = Vec::new();
    for (name, x, y, mode, locked) in [
        ("CG", 534, 114, ExtrasMode::Gallery, true),
        ("回想", 434, 114, ExtrasMode::Recollection, true),
        ("SOUNDMOVIE", 234, 114, ExtrasMode::Sound, true),
        ("ツルギ回転", 134, 114, ExtrasMode::Turntable, false),
        ("メニュー退出", 0, 438, ExtrasMode::Back, false),
    ] {
        let state = |n: u32| format!("{DIR}/{name}-{n:03}.png");
        let (image, w, h) = a.strip(&[&state(1), &state(2), &state(3)])?;
        let click_se = if mode == ExtrasMode::Back { None } else { se(a, "se特殊_鎧_装着06") };
        items.push(PictureExtrasItem {
            button: Placed { image, x, y, w: w as i32, h: h as i32 },
            mode,
            unlocked_by: if locked { cleared } else { None },
            locked_alpha: 51,
            click_se,
        });
    }
    Some(PictureExtras {
        background: a.image(&format!("{DIR}/MENU背景画像.png"))?.0,
        items,
        hover_se: se(a, "se特殊_鎧_装着01"),
        back_se: se(a, "se戦闘_攻撃_剣戟弾く01"),
        fade_ms: 500,
        gallery: gallery(a, story),
        sound: sound(a, story),
        replay: replay(a, story),
        turntable: turntable(a, story),
    })
}

/// The `Array(...)` calls of `function` in `file`, in order: the array's
/// key (`#Cg["bg003_荒れ野_01"]` → `bg003_荒れ野_01`, none for the array
/// itself) and the values, as written.
fn arrays(files: &BTreeMap<String, NssFile>, file: &str, function: &str) -> Vec<(String, Option<String>, Vec<String>)> {
    let Some(f) = files.get(file) else { return Vec::new() };
    let mut out = Vec::new();
    for item in &f.items {
        let Item::Function(func) = item else { continue };
        if func.name != function {
            continue;
        }
        for stmt in &func.body {
            let StmtKind::Expr(Expr::Call { name, args }) = &stmt.kind else { continue };
            if name != "Array" || args.is_empty() {
                continue;
            }
            let (array, key) = match &args[0] {
                Expr::Var(v) => (v.to_string(), None),
                Expr::Index { base, index } => match (base.as_ref(), index.as_ref()) {
                    (Expr::Var(v), Expr::Str(k) | Expr::Word(k)) => (v.to_string(), Some(k.clone())),
                    _ => continue,
                },
                _ => continue,
            };
            let values = args[1..]
                .iter()
                .filter_map(|e| match e {
                    Expr::Str(s) | Expr::Word(s) => Some(s.clone()),
                    Expr::Int(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect();
            out.push((array, key, values));
        }
    }
    out
}

/// The gallery's slots (`extra_gallery.nss`, `Cg`), in the order of the
/// `#Cg` array: category (2D, 3D, BG), the way a large picture scrolls
/// (`No`, `Vr`, `Hr`), the CGs.
fn gallery_slots(files: &BTreeMap<String, NssFile>) -> Vec<(String, String, Vec<String>)> {
    let calls = arrays(files, "extra_gallery.nss", "Cg");
    let Some((_, _, order)) = calls.iter().find(|(a, k, _)| a == "#Cg" && k.is_none()) else { return Vec::new() };
    let entries: BTreeMap<&str, &Vec<String>> = calls.iter().filter(|(a, _, _)| a == "#Cg").filter_map(|(_, k, v)| Some((k.as_deref()?, v))).collect();
    order
        .iter()
        .filter_map(|key| {
            let v = entries.get(key.as_str())?;
            (v.len() > 2).then(|| (v[0].clone(), v[1].clone(), v[2..].to_vec()))
        })
        .collect()
}

/// The sound room's entries (`extra_sound.nss`, `Sound`), in the order of
/// the `#Sound` array: name (`@mbgm01`, `mvOP`), category (BGM, SONG, MV),
/// picture, the flag that opens it (`open`: always).
fn sound_entries(files: &BTreeMap<String, NssFile>) -> Vec<(String, String, String, String)> {
    let calls = arrays(files, "extra_sound.nss", "Sound");
    let Some((_, _, order)) = calls.iter().find(|(a, k, _)| a == "#Sound" && k.is_none()) else { return Vec::new() };
    let entries: BTreeMap<&str, &Vec<String>> = calls.iter().filter(|(a, _, _)| a == "#Sound").filter_map(|(_, k, v)| Some((k.as_deref()?, v))).collect();
    order
        .iter()
        .filter_map(|key| {
            let v = entries.get(key.as_str())?;
            (v.len() >= 3).then(|| (key.clone(), v[0].clone(), v[1].clone(), v[2].clone()))
        })
        .collect()
}

/// The flags the extras read: each CG's `#<name>`, the songs' and
/// movies' (`#msong01`, `#mvOP02`).
pub fn flags(files: &BTreeMap<String, NssFile>) -> Vec<String> {
    let mut out: Vec<String> = gallery_slots(files).into_iter().flat_map(|(_, _, cgs)| cgs).map(|n| format!("#{n}")).collect();
    out.extend(sound_entries(files).into_iter().filter(|e| e.3 != "open").map(|e| format!("#{}", e.3)));
    out.extend(replay_entries(files).into_iter().map(|(_, file)| format!("#{file}")));
    out.extend(turntable_entries(files).into_iter().flat_map(|(key, _, _)| [format!("#av_{key}"), format!("#aw_{key}")]));
    out
}

/// The turntable's entries (`extra_dimension.nss`, `Dx`), in the order of
/// the `$Dx` array less its first (`何か`, which `Dimension` leaves out):
/// key (its flags are `#av_<key>`, seen, and
/// `#aw_<key>`, its details told), movie number (`dx/3d/ex_NNN.ngs`), the
/// name of its pictures.
fn turntable_entries(files: &BTreeMap<String, NssFile>) -> Vec<(String, u32, String)> {
    let calls = arrays(files, "extra_dimension.nss", "Dx");
    let Some((_, _, order)) = calls.iter().find(|(a, k, _)| a == "$Dx" && k.is_none()) else { return Vec::new() };
    let entries: BTreeMap<&str, &Vec<String>> = calls.iter().filter(|(a, _, _)| a == "$Dx").filter_map(|(_, k, v)| Some((k.as_deref()?, v))).collect();
    order
        .iter()
        .skip(1)
        .filter_map(|key| {
            let v = entries.get(key.as_str())?;
            Some((key.clone(), v.first()?.parse().ok()?, v.get(1)?.clone()))
        })
        .collect()
}

/// The recollection's scenes (`extra_recollection.nss`, `Reco`), in the
/// order of the `#Reco` array: the CG whose thumbnail stands for it, and
/// the story file it replays (unlocked by `#<file>`, which the file sets
/// at its end).
fn replay_entries(files: &BTreeMap<String, NssFile>) -> Vec<(String, String)> {
    let calls = arrays(files, "extra_recollection.nss", "Reco");
    let Some((_, _, order)) = calls.iter().find(|(a, k, _)| a == "#Reco" && k.is_none()) else { return Vec::new() };
    let entries: BTreeMap<&str, &Vec<String>> = calls.iter().filter(|(a, _, _)| a == "#Reco").filter_map(|(_, k, v)| Some((k.as_deref()?, v))).collect();
    order.iter().filter_map(|key| Some((key.clone(), entries.get(key.as_str())?.get(2)?.clone()))).collect()
}

fn replay_label(file: &str) -> String {
    format!("conv_reco_{file}")
}

/// The recollection's scenes as subroutines: each file's `_MAIN` with
/// `$PLACE_reco` true, which takes the files' replay branches (they set up
/// the scene's first picture and music instead of the story's way in, and
/// call `RecoIn`/`RecoOut`, which aren't in the story's library: the
/// fades they make are the screen's own, [`PictureReplay`]).
pub fn replays(files: &BTreeMap<String, NssFile>) -> Vec<Replay> {
    replay_entries(files)
        .into_iter()
        .map(|(_, file)| Replay { label: replay_label(&file), file: format!("{file}.nss"), set: vec![("$PLACE_reco".to_string(), 1)] })
        .collect()
}

/// The recollection (`extra_recollection.nss`): 背景画像, the ten scenes in
/// the gallery's grid (`Recollection`), 空白アイコン until the scene's file
/// has been read, its CG's thumbnail after, 選択カーソル under the pointer.
/// A scene (`RecoFade`) fades the music out over 1 s with the screen held
/// 500 ms, and the held list fades out over 500 ms at the scene's `RecoIn`
/// ([`HOLD_SLOT`]); at its end (`RecoOut`) the music fades over 2 s with the screen
/// held, and the list fades in over 500 ms.
fn replay(a: &mut dyn Assets, story: &Story) -> Option<PictureReplay> {
    const DIR: &str = "cg/sys/extra/reco";
    let mut scenes = Vec::new();
    for (cg, file) in replay_entries(story.files) {
        let Some(&unlocked_by) = story.flags.get(&format!("#{file}")) else { continue };
        let thumbnail = a.image(&format!("cg/sys/extra/thum/ev/{cg}.jpg"))?.0;
        scenes.push(ReplayEntry { thumbnail, unlocked_by, label: replay_label(&file) });
    }
    let state = |n: u32| format!("{DIR}/退出-{n:03}.png");
    let (exit, w, h) = a.strip(&[&state(1), &state(2), &state(3)])?;
    Some(PictureReplay {
        background: a.image(&format!("{DIR}/背景画像.png"))?.0,
        exit: Placed { image: exit, x: 0, y: 438, w: w as i32, h: h as i32 },
        first: (672, 38),
        row_step: 100,
        column_step: -190,
        rows: 5,
        blank: a.image(&format!("{DIR}/空白アイコン.png"))?.0,
        cursor: a.strip(&["", &format!("{DIR}/選択カーソル.png")])?.0,
        scenes,
        open_ms: 500,
        start_ms: 1000,
        end_ms: 2000,
        across: None,
        hold: Some((HOLD_SLOT, (SCREEN_W as i32 / 2, SCREEN_H as i32 / 2))),
    })
}

/// The sound room (`extra_sound.nss`, `Media`): 背景画像 with a button per
/// entry, in the script's order and places (BGM down columns at x 380,
/// 651, 787 from y 84, 24 apart; the songs at x 651 from y 396; the movies'
/// thumbnails at x 102 from y 84, 150 apart). A locked song shows its
/// `000` picture, a locked movie 空白アイコン. The playing track has the
/// ten-picture A-icon turning 26 px left of it, 340 ms a picture (filmed
/// under wine at 100 frames a second: the process's loop has no wait, but
/// the engine gives it a step every 340 ms). Choosing the playing track fades it out over 2 s
/// (`SoundFade`); a movie fades the music and the screen to black over 1 s,
/// then back, the black over 300 ms.
fn sound(a: &mut dyn Assets, story: &Story) -> Option<SoundRoom> {
    const DIR: &str = "cg/sys/extra/sound";
    let entries = sound_entries(story.files);
    let (mut x, mut y) = (380, 84);
    let mut tracks = Vec::new();
    let mut movies = Vec::new();
    let mut title_track = 0;
    for (k, (name, category, picture, flag)) in entries.iter().enumerate() {
        let unlocked_by = if flag == "open" { None } else { Some(*story.flags.get(&format!("#{flag}"))?) };
        if category == "MV" {
            let (image, w, h) = a.image(&format!("cg/sys/extra/thum/dx/{picture}.jpg"))?;
            let file = a.movie_file(&format!("dx/{name}.ngs"), true)?;
            movies.push(RoomVideo {
                thumbnail: Placed { image, x, y, w: w as i32, h: h as i32 },
                cursor: a.strip(&["", &format!("{DIR}/選択カーソル.png")])?.0,
                file,
                unlocked_by,
                locked: a.image(&format!("{DIR}/空白アイコン.png"))?.0,
            });
        } else {
            let state = |n: u32| format!("{DIR}/{picture}-{n:03}.png");
            let (image, w, h) = a.strip(&[&state(1), &state(2)])?;
            let locked = if category == "SONG" { a.image(&state(0)).map(|i| i.0) } else { None };
            let file = a.sound(&format!("sound/bgm/{}", name.trim_start_matches('@')))?;
            if name == "@mbgm04" {
                title_track = tracks.len();
            }
            tracks.push(RoomTrack { button: Placed { image, x, y, w: w as i32, h: h as i32 }, file, unlocked_by, locked, playing: None });
        }
        // The next entry's place (`Media`).
        match k + 2 {
            n if n <= 18 => y += 24,
            19 => (x, y) = (651, 84),
            n if n <= 28 => y += 24,
            29 => (x, y) = (787, 84),
            n if n <= 37 => y += 24,
            38 => (x, y) = (651, 396),
            n if n <= 42 => y += 24,
            43 => (x, y) = (102, 84),
            _ => y += 150,
        }
    }
    let icons: Vec<String> = (1..=10).map(|n| format!("{DIR}/anime/A-icon-{n:03}.png")).collect();
    let (image, w, h) = a.strip(&icons.iter().map(String::as_str).collect::<Vec<_>>())?;
    let state = |n: u32| format!("{DIR}/退出-{n:03}.png");
    let (exit, ew, eh) = a.strip(&[&state(1), &state(2), &state(3)])?;
    Some(SoundRoom {
        background: a.image(&format!("{DIR}/背景画像.png"))?.0,
        exit: Placed { image: exit, x: 0, y: 438, w: ew as i32, h: eh as i32 },
        tracks,
        movies,
        title_track,
        marker: Placed { image, x: 0, y: 0, w: w as i32, h: h as i32 },
        marker_cells: 10,
        marker_ms: 340,
        marker_dx: 26,
        open_ms: 500,
        stop_fade_ms: 2000,
        movie_fade_ms: 1000,
        movie_back_ms: 300,
    })
}

/// The CG gallery (`extra_gallery.nss`): 背景画像 with the categories'
/// buttons (人物画, 三次元的画, 背景画; the open one pressed), 前頁, 次頁,
/// 退出; 20 slots a page down four columns from the right (`Gallery`),
/// 空白アイコン for a slot with nothing seen, its first CG's thumbnail
/// (`thum/`) at 500/1000 while some are unseen, 選択カーソル over the
/// pointer's slot. The viewer (`CgFade`): each CG fades in over 500 ms
/// (未見 for an unseen one, black over 150 ms between two unseen); one
/// larger than the screen scrolls 12 ms a pixel after a click, from the
/// top left (`Vr`/`Hr`: from the far end); closing fades over 300 ms. The
/// screen fades in from the menu over 500 ms, page to page over 300.
fn gallery(a: &mut dyn Assets, story: &Story) -> Option<PictureGallery> {
    const DIR: &str = "cg/sys/extra/cg";
    let button = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let state = |n: u32| format!("{DIR}/{name}-{n:03}.png");
        let (image, w, h) = a.strip(&[&state(1), &state(2), &state(3)])?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let slots = gallery_slots(story.files);
    let mut categories = Vec::new();
    for (name, label, y) in [("2D", "人物画", 98), ("3D", "三次元的画", 194), ("BG", "背景画", 334)] {
        let mut out = Vec::new();
        for (_, mode, names) in slots.iter().filter(|s| s.0 == name) {
            let mut cgs = Vec::new();
            for n in names {
                let Some(&seen) = story.flags.get(&format!("#{n}")) else { continue };
                let index = if n.contains("ev") { "ev" } else { "bg" };
                let Some((image, w, h)) = a.image(&format!("cg/{index}/{n}.jpg")) else { continue };
                let Some((thumbnail, _, _)) = a.image(&format!("cg/sys/extra/thum/{index}/{n}.jpg")) else { continue };
                cgs.push(SlotCg { seen, picture: Placed { image, x: 0, y: 0, w: w as i32, h: h as i32 }, thumbnail });
            }
            if !cgs.is_empty() {
                out.push(PictureSlot { cgs, reversed: mode == "Vr" || mode == "Hr" });
            }
        }
        categories.push(GalleryCategory { button: button(a, label, 862, y)?, slots: out });
    }
    Some(PictureGallery {
        background: a.image(&format!("{DIR}/背景画像.png"))?.0,
        categories,
        previous: button(a, "前頁", 912, 438)?,
        next: button(a, "次頁", 862, 438)?,
        exit: button(a, "退出", 0, 438)?,
        first: (672, 38),
        row_step: 100,
        column_step: -190,
        rows: 5,
        columns: 4,
        blank: a.image(&format!("{DIR}/空白アイコン.png"))?.0,
        cursor: a.strip(&["", &format!("{DIR}/選択カーソル.png")])?.0,
        partial_alpha: 128,
        unseen: a.image(&format!("{DIR}/未見.jpg"))?.0,
        open_ms: 500,
        page_ms: 300,
        view_fade_ms: 500,
        close_ms: 300,
        scroll_ms_per_px: 12,
        across: false,
        arrows_when_paged: false,
    })
}

/// 劒冑回転目録 (`extra_dimension.nss`): 3Dビュアー背景 with 24 name plates
/// a page in two rows of twelve from the right (`Dimension`), NO for one
/// not seen; the chosen one's movie (400×400, looping, played by the
/// engine as a sprite) at 80,28, its
/// large name plate at 480,28 and its details at 60,436 (the （開放）
/// ones once `#aw_` is set). 九〇式指揮官騎 and 九四式指揮官騎 are open
/// from the first visit, the first chosen. The screen fades in from the
/// menu over 500 ms, a page or a choice over 300 (`DxFade`).
fn turntable(a: &mut dyn Assets, story: &Story) -> Option<Turntable> {
    const DIR: &str = "cg/sys/extra/3d";
    const OPEN: [&str; 2] = ["九〇式指揮官騎", "九四式指揮官騎"];
    let button = |a: &mut dyn Assets, name: &str, y: i32| {
        let state = |n: u32| format!("{DIR}/ボタン/{name}-{n:03}.png");
        let (image, w, h) = a.strip(&[&state(1), &state(2), &state(3)])?;
        Some(Placed { image, x: 948, y, w: w as i32, h: h as i32 })
    };
    let picture = |a: &mut dyn Assets, path: &str, x: i32, y: i32| {
        let (image, w, h) = a.image(path)?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let mut entries = Vec::new();
    let mut first = 0;
    for (k, (key, number, name)) in turntable_entries(story.files).into_iter().enumerate() {
        let state = |n: u32| format!("{DIR}/ネームプレート/{name}-{n:03}.png");
        let (image, w, h) = a.strip(&[&state(1), &state(2), &state(3)])?;
        let place = k % 24;
        let (x, y) = (910 - 30 * (place % 12) as i32, if place < 12 { 28 } else { 238 });
        let unlocked_by = if OPEN.contains(&key.as_str()) { None } else { Some(*story.flags.get(&format!("#av_{key}"))?) };
        if key == OPEN[0] {
            first = k;
        }
        let details_after = story.flags.get(&format!("#aw_{key}")).and_then(|&v| Some((v, a.image(&format!("{DIR}/各スペック/{name}（開放）スペック.png"))?.0)));
        let (movie, mw, mh) = a.movie_sprite(&format!("dx/3d/ex_{number:03}.ngs"), false, false)?;
        entries.push(TurntableEntry {
            plate: Placed { image, x, y, w: w as i32, h: h as i32 },
            unlocked_by,
            name: picture(a, &format!("{DIR}/ネームプレート大/{name}.png"), 480, 28)?,
            details: picture(a, &format!("{DIR}/各スペック/{name}スペック.png"), 60, 436)?,
            details_after,
            movie: Placed { image: movie, x: 80, y: 28, w: mw as i32, h: mh as i32 },
            cells: 1,
            frame_ms: 0,
            zoom: 400 * 100 / mw.max(1),
        });
    }
    Some(Turntable {
        background: a.image(&format!("{DIR}/3Dビュアー背景.png"))?.0,
        previous: button(a, "前頁", 438)?,
        next: button(a, "次頁", 474)?,
        exit: button(a, "退出", 510)?,
        locked: a.image(&format!("{DIR}/NO.png"))?.0,
        entries,
        per_page: 24,
        first,
        open_ms: 500,
        change_ms: 300,
    })
}
