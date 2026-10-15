//! 装甲悪鬼村正's extras, 特別付加目録 (`extra_*.nss`), built from its own
//! pictures (`cg/sys/extra/`) with the places and times of its scripts, as
//! filmed in the original (`docs/games/muramasa.md`, "Extras").

use std::collections::BTreeMap;

use conv_core::ir::{ExtrasMode, GalleryCategory, PictureExtras, PictureExtrasItem, PictureGallery, PictureSlot, Placed, SlotCg};

use super::super::{Assets, Story};
use crate::nitroplus::nss::{Expr, Item, NssFile, StmtKind};

/// Set at the end of any route (the end roll) and when a game starts after
/// one: the gallery, the recollection and the sound room open.
pub const CLEARED: &str = "#ClearG";

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

/// The flags the extras read: each CG's `#<name>`.
pub fn flags(files: &BTreeMap<String, NssFile>) -> Vec<String> {
    gallery_slots(files).into_iter().flat_map(|(_, _, cgs)| cgs).map(|n| format!("#{n}")).collect()
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
