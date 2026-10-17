//! 続・殺戮のジャンゴ's extras (`extra_menu.nss`, `extra_cg.nss`), built
//! from its own pictures (`cg/sys/extra/`) with the places and times of
//! its scripts, as filmed in the original under wine
//! (`docs/games/django.md`, "Extras"). The title shows EXTRA once the
//! story is finished ([`CLEARED`]); everything in the CG library is open
//! from then on.

use std::collections::BTreeMap;

use conv_core::ir::{ExtrasMode, GalleryCategory, PictureExtras, PictureExtrasItem, PictureGallery, PictureReplay, PictureSlot, Placed, ReplayEntry, RoomTrack, SlotCg, SoundRoom};

use super::super::{Assets, Story};
use crate::nitroplus::nss::{AssignOp, BinaryOp, Expr, Item, NssFile, Stmt, StmtKind, Var};

/// Set when the story is finished (`main_root`): the title's EXTRA and
/// LINER NOTES show.
pub const CLEARED: &str = "#ClearG";
/// Set by the title's congratulations after every ending
/// (`TitleCongratulations`): the gallery's おめでとう picture opens.
pub const CLEARED_ALL: &str = "#ClearAll";

/// The flags the title and the extras read.
pub fn flags(_: &BTreeMap<String, NssFile>) -> Vec<String> {
    // TitleCongratulations ([`super::DJANGO`]'s `story_end`) reads every
    // ending's flag and $ClearL, set as an ending is reached.
    let mut out = vec![CLEARED.to_string(), CLEARED_ALL.to_string(), "$ClearL".to_string()];
    out.extend((1..=10).map(|n| format!("#Clear{n:02}")));
    out
}

/// The menu (`extra_menu.nss`): the saloon (`extramenu背景`) with its
/// closed doors (`extramenu扉01` at 89,58), CG LIBRARY (281,193),
/// RECOLLECTION LIBRARY (156,260), SOUND LIBRARY (241,328) and EXIT
/// (615,551), each `001` at rest, `002` under the pointer, `003` pressed;
/// it comes in from black over 500 ms. Left out: the doors swinging open
/// (`扉02`, `扉03`) with the screen zooming in on the way to a library,
/// the menu's own music (`dj_INS12` where the sound library has chosen
/// nothing; ours keeps the title's).
pub fn extras(a: &mut dyn Assets, story: &Story) -> Option<PictureExtras> {
    const DIR: &str = "cg/sys/extra/menu";
    // src: extra_menu.nss (places; Fade("blk", 500))
    let mut parts = Vec::new();
    for (name, x, y) in [("extramenu背景.png", 0, 0), ("extramenu扉01.png", 89, 58)] {
        parts.push((a.image(&format!("{DIR}/{name}"))?.0, x, y, 1000));
    }
    let (background, _, _) = a.composite(&parts)?;
    let mut items = Vec::new();
    for (name, x, y, mode) in [
        ("MENU-CG", 281, 193, ExtrasMode::Gallery),
        ("MENU-回想", 156, 260, ExtrasMode::Recollection),
        ("MENU-SOUND", 241, 328, ExtrasMode::Sound),
        ("extra-exit-", 615, 551, ExtrasMode::Back),
    ] {
        let state = |n: u32| if mode == ExtrasMode::Back { format!("{DIR}/{name}{n:02}.png") } else { format!("{DIR}/{name}{n:03}.png") };
        let (image, w, h) = a.strip(&[&state(1), &state(2), &state(3)])?;
        items.push(PictureExtrasItem { button: Placed { image, x, y, w: w as i32, h: h as i32 }, mode, unlocked_by: None, locked_alpha: 255, click_se: None });
    }
    Some(PictureExtras {
        background,
        items,
        hover_se: None,
        back_se: None,
        fade_ms: 500,
        gallery: gallery(a, story),
        sound: sound(a),
        replay: replay(a, story),
        turntable: None,
    })
}

/// One slot of `cg_init`: its thumbnail, its pictures by number, how many
/// of them it shows (`$pic_numN`, which may be fewer than are named), and
/// `$actN` (1: a tall picture starts at its bottom and scrolls up).
#[derive(Default)]
struct Slot {
    thumbnail: String,
    pictures: BTreeMap<u32, String>,
    count: u32,
    scrolls: bool,
    /// Shown only after [`CLEARED_ALL`] (else 未見画像アイコン).
    needs_all: bool,
}

/// Where `cg_init`'s statements are: category `$c`, page `$p`, slot `$i`,
/// inside `if(#ClearAll)`.
#[derive(Clone, Copy, Default)]
struct At {
    category: Option<i64>,
    page: Option<i64>,
    slot: Option<i64>,
    all: bool,
}

/// `cg_init`'s table (`extra_cg.nss`): (category, page, slot) from 1.
fn slots(files: &BTreeMap<String, NssFile>) -> BTreeMap<(i64, i64, i64), Slot> {
    let mut out = BTreeMap::new();
    let Some(f) = files.get("extra_cg.nss") else { return out };
    for item in &f.items {
        if let Item::Function(func) = item
            && func.name == "cg_init"
        {
            walk(&func.body, At::default(), &mut out);
        }
    }
    out
}

fn walk(stmts: &[Stmt], at: At, out: &mut BTreeMap<(i64, i64, i64), Slot>) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::If { cond, then, otherwise } => {
                let mut inner = at;
                match cond {
                    Expr::Binary { op: BinaryOp::Eq, left, right } => {
                        if let (Expr::Var(Var::Local(name)), Expr::Int(n)) = (left.as_ref(), right.as_ref()) {
                            match name.as_str() {
                                "c" => inner.category = Some(*n),
                                "p" => inner.page = Some(*n),
                                "i" => inner.slot = Some(*n),
                                _ => {}
                            }
                        }
                    }
                    Expr::Var(Var::Global(name)) if name == "ClearAll" => inner.all = true,
                    _ => {}
                }
                walk(then, inner, out);
                if let Some(o) = otherwise {
                    walk(o, at, out);
                }
            }
            StmtKind::While { body, .. } | StmtKind::Block(body) => walk(body, at, out),
            StmtKind::Assign { target: Var::Local(name), op: AssignOp::Set, value: Some(value), .. } => {
                let (Some(c), Some(i)) = (at.category, at.slot) else { continue };
                let slot = out.entry((c, at.page.unwrap_or(1), i)).or_default();
                match value {
                    Expr::Str(v) if name == "thum" && v != "nopic" && !v.is_empty() => {
                        slot.thumbnail = v.clone();
                        slot.needs_all = at.all;
                    }
                    Expr::Str(v) => {
                        if let Some(j) = name.strip_prefix(&format!("pic_name{i}_")).and_then(|j| j.parse().ok()) {
                            slot.pictures.insert(j, v.clone());
                        }
                    }
                    Expr::Int(n) if *name == format!("pic_num{i}") => slot.count = *n as u32,
                    Expr::Int(n) if *name == format!("act{i}") => slot.scrolls = *n == 1,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// The CG library (`extra_cg.nss`): CG-背景 with five categories at the
/// bottom (`chara_btn`: a wanted poster's face at y 440 over the name
/// plate at 500; the name turns yellow under the pointer, `002`, and the
/// open category's face is its red silhouette, `顔003`). Twenty thumbnails (`cg/thum/`, 100×75) a page, five
/// across from (130,80), 110 and 86 apart; only ARTIFACT has more pages,
/// and only there BACK (280,536, the next page) and NEXT (460,536, the
/// one before) show; EXIT at (702,536). Every picture is open: the
/// scripts check nothing but `#ClearAll` (おめでとう画像, else
/// 未見画像アイコン). The viewer (`show_pic`): a picture larger than the
/// screen is shown at half size, centred; a tall one (`$act`) starts at
/// its bottom and a click scrolls it up over 3 s; each picture fades in
/// over 300 ms; the screen comes in from black over 500 ms. Left out: the
/// rifle animation over a category under the pointer and the shot fired
/// on choosing one.
fn gallery(a: &mut dyn Assets, story: &Story) -> Option<PictureGallery> {
    const DIR: &str = "cg/sys/extra/cg";
    let cleared = *story.flags.get(CLEARED)?;
    let cleared_all = story.flags.get(CLEARED_ALL).copied().unwrap_or(cleared);
    let button = |a: &mut dyn Assets, name: &str, x: i32, y: i32| {
        let (image, w, h) = a.strip(&[&format!("{DIR}/{name}001.png"), &format!("{DIR}/{name}002.png"), &format!("{DIR}/{name}003.png")])?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    };
    let table = slots(story.files);
    let mut categories = Vec::new();
    // src: extra_cg.nss chara_btn (x; face at y 440, plate at 500, リリィ's 名前001 6 px up)
    for (c, (name, x)) in [("フランコ", 76), ("リリィ", 202), ("名無し", 376), ("OTHERS", 508), ("ART", 616)].into_iter().enumerate() {
        // The button: face and name plate, three cells (at rest, under the
        // pointer, open: the face's silhouette over the plate at rest).
        // リリィ's plate at rest (`plate_リリィ` under the open face too) is 6 px higher (and taller).
        let (face, w, _) = a.image(&format!("{DIR}/{name}顔001.png"))?;
        let (open, _, _) = a.image(&format!("{DIR}/{name}顔003.png"))?;
        let mut parts = Vec::new();
        for (k, (face, plate)) in [(&face, 1), (&face, 2), (&open, 1)].into_iter().enumerate() {
            let (plate, _, _) = a.image(&format!("{DIR}/{name}名前{plate:03}.png"))?;
            let dx = (k as u32 * w) as i64;
            let dy = if name == "リリィ" && k != 1 { 54 } else { 60 };
            parts.push((face.clone(), dx, 0, 1000));
            parts.push((plate, dx, dy, 1000));
        }
        let (image, _, _) = a.composite(&parts)?;
        let mut slots = Vec::new();
        for ((_, _, _), s) in table.range((c as i64 + 1, i64::MIN, i64::MIN)..(c as i64 + 2, i64::MIN, i64::MIN)) {
            let seen = if s.needs_all { cleared_all } else { cleared };
            let Some((thumbnail, _, _)) = a.image(&format!("cg/thum/{}.jpg", s.thumbnail)) else { continue };
            let mut cgs = Vec::new();
            for picture in s.pictures.values().take(s.count as usize) {
                let Some((image, w, h)) = a.image(&format!("cg/{picture}.jpg")) else { continue };
                let (w, h) = (w as i64, h as i64);
                // src: show_pic: larger than the screen, zoomed to 500 and centred
                let picture = if w > 800 || h > 600 {
                    let (image, _, _, w, h) = a.scaled(&image, 500, 500, (0, 0, w, h))?;
                    let (w, h) = (w as i32, h as i32);
                    Placed { image, x: ((800 - w) / 2).max(0), y: ((600 - h) / 2).max(0), w, h }
                } else {
                    Placed { image, x: (800 - w as i32) / 2, y: (600 - h as i32) / 2, w: w as i32, h: h as i32 }
                };
                cgs.push(SlotCg { seen, picture, thumbnail: thumbnail.clone() });
            }
            if !cgs.is_empty() {
                slots.push(PictureSlot { cgs, reversed: s.scrolls });
            }
        }
        categories.push(GalleryCategory { button: Placed { image, x, y: 440, w: 3 * w as i32, h: 80 }, slots });
    }
    // No cursor: a see-through cell over a thumbnail, as the button.
    let clear = a.transparent();
    let (cursor, _, _) = a.composite(&[(clear.clone(), 0, 0, 1000), (clear, 199, 74, 1000)])?;
    // src: extra_cg.nss (buttons, thumbnails from 130,80, 110 and 86 apart; blk 500; show_pic's fades 300)
    Some(PictureGallery {
        background: a.image(&format!("{DIR}/CG-背景.jpg"))?.0,
        categories,
        previous: button(a, "NEXT", 460, 536)?,
        next: button(a, "BACK", 280, 536)?,
        exit: button(a, "EXIT", 702, 536)?,
        first: (130, 80),
        row_step: 86,
        column_step: 110,
        rows: 4,
        columns: 5,
        blank: a.image(&format!("{DIR}/未見画像アイコン.png"))?.0,
        cursor,
        partial_alpha: 255,
        unseen: a.color(0, 800, 600),
        open_ms: 500,
        page_ms: 0,
        view_fade_ms: 300,
        close_ms: 300,
        // src: show_pic's Move over 3000 ms, here per pixel of the half-size
        // picture's part below the screen (about 470 px for the tallest).
        scroll_ms_per_px: 6,
        across: true,
        arrows_when_paged: true,
        engine_screen: false,
    })
}

/// The sound library's songs in order (`extra_sound.nss`, `snd_play`):
/// the BGM files (`InitBGM`).
const SONGS: [&str; 35] = [
    "DJ_OP_Brindiamo!～俺たちに乾杯",
    "DJ_INS_スィートウォーター讃歌",
    "DJ_INS_スィートウォーター讃歌民衆",
    "DJ_INS_ColdMetal",
    "dj01",
    "dj02",
    "dj02b",
    "dj03",
    "dj04",
    "dj05",
    "dj06",
    "dj07",
    "dj08",
    "dj09",
    "dj09b",
    "dj10",
    "dj11",
    "dj12",
    "dj13",
    "dj14",
    "dj15",
    "dj16",
    "dj17",
    "dj18",
    "dj19",
    "dj20",
    "dj21",
    "dj21b",
    "dj22",
    "dj23",
    "DJ23_harmonica",
    "DJ23_harp",
    "dj23b",
    "dj23b_harp",
    "dj24",
];

/// The sound library (`extra_sound.nss`): `sound-back` with 35 titles
/// (`kyoku/sound-NNN`, `001` at rest, `002` under the pointer, `003` in
/// white while it plays) down two columns from (136,92) and (438,92), 20
/// px apart; the horse (`uma/uma-001`-`011`, 50 ms a frame) runs 34 px
/// left of the title playing; EXIT at (702,538). A title plays its song
/// looping, which stays the extras' music. The title's music
/// (`dj23_harmonica`) plays at first. Differences: clicking the title
/// playing stops it (the original starts it again); the song fades in
/// over 2 s in the original.
fn sound(a: &mut dyn Assets) -> Option<SoundRoom> {
    const DIR: &str = "cg/sys/extra/sound";
    // src: extra_sound.nss (titles from 136,92 and 438,92, 20 apart; uma 11 frames, 50 ms, x - 34;
    // EXIT 702,538; blk 500; SetVolume("@dj*", 1000, 0))
    let mut tracks = Vec::new();
    for (i, song) in SONGS.iter().enumerate() {
        let state = |n: u32| format!("cg2/sys/extra/sound/kyoku/sound-{:03}-{n:03}.png", i + 1);
        let (image, w, h) = a.strip(&[&state(1), &state(2)])?;
        let (x, y) = if i < 18 { (136, 92 + 20 * i as i32) } else { (438, 92 + 20 * (i as i32 - 18)) };
        tracks.push(RoomTrack {
            button: Placed { image, x, y, w: w as i32, h: h as i32 },
            file: a.sound(&format!("sound/bgm/{song}"))?,
            unlocked_by: None,
            locked: None,
            playing: Some(a.image(&state(3))?.0),
        });
    }
    let frames: Vec<String> = (1..=11).map(|n| format!("{DIR}/uma/uma-{n:03}.png")).collect();
    let frames: Vec<&str> = frames.iter().map(String::as_str).collect();
    let (marker, mw, mh) = a.strip(&frames)?;
    let (exit, ew, eh) = a.strip(&[&format!("{DIR}/exit/sound-exit001.png"), &format!("{DIR}/exit/sound-exit002.png"), &format!("{DIR}/exit/sound-exit003.png")])?;
    Some(SoundRoom {
        background: a.image("cg2/sys/extra/sound/sound-back.png")?.0,
        exit: Placed { image: exit, x: 702, y: 538, w: ew as i32, h: eh as i32 },
        tracks,
        movies: Vec::new(),
        title_track: 30,
        marker: Placed { image: marker, x: 0, y: 0, w: mw as i32, h: mh as i32 },
        marker_cells: 11,
        marker_ms: 50,
        marker_dx: 34,
        open_ms: 500,
        stop_fade_ms: 1000,
        movie_fade_ms: 0,
        movie_back_ms: 0,
    })
}

/// The recollection's scenes (`extra_recollection.nss`): each an
/// `extra_N_*.nss` chapter calling the story's scene files, compiled once
/// more as a subroutine ([`Game::subroutines`](super::super::Game)), and
/// its thumbnail (`rec_init`).
pub const SCENES: [(&str, &str); 11] = [
    ("conv_rec1", "nss/extra_1_dja003_フランコのベッド.nss"),
    ("conv_rec2", "nss/extra_1_dja012_配当.nss"),
    ("conv_rec3", "nss/extra_1_dja029_襲撃準備.nss"),
    ("conv_rec4", "nss/extra_1_dja042_イライザ拷問.nss"),
    ("conv_rec5", "nss/extra_1_dja043_エル・ドミンゴへ.nss"),
    ("conv_rec6", "nss/extra_2_dja018_１００人斬り.nss"),
    ("conv_rec7", "nss/extra_2_dja020_総仕上げ.nss"),
    ("conv_rec8", "nss/extra_3_dja005_共同戦線.nss"),
    ("conv_rec9", "nss/extra_3_dja047_リリィ、名無し捕まる.nss"),
    ("conv_rec10", "nss/extra_4_dja001_フランコ馬車を襲う.nss"),
    ("conv_rec11", "nss/extra_5_dja033_エスパルテロ市街戦.nss"),
];

const THUMBNAILS: [&str; 11] = [
    "ev_dja003_イライザ後背位",
    "ev_dja012_イライザオナニー",
    "ev_dja029_イライザ騎乗位３Ｐ_d",
    "ev_dja042_イライザ触手Ａ_a",
    "ev_dja042_イライザ触手Ｂ",
    "ev_dja018_名無し押し倒され_b",
    "ev_dja020_名無し逆レイプ",
    "ev_dja005_リリィ泡風呂_a",
    "ev_dja047_リリィ強姦騎乗位",
    "ev_dja001_宇宙淑女Ａ正常位_a",
    "ev_dja033_ブルネット女強姦",
];

/// The recollection library (`extra_recollection.nss`): エクストラレコレ背景
/// with eleven thumbnails five across from (130,80), 110 and 86 apart, as
/// the CG library's; EXIT at (702,536). Every scene is open. A click fades
/// the music out and the screen to black over 500 ms and plays the scene
/// (save and load locked in the original); after it the list comes back
/// from black over 500 ms with the extras' music.
fn replay(a: &mut dyn Assets, story: &Story) -> Option<PictureReplay> {
    const DIR: &str = "cg/sys/extra/recollection";
    let cleared = *story.flags.get(CLEARED)?;
    let mut scenes = Vec::new();
    for ((label, _), thumbnail) in SCENES.iter().zip(THUMBNAILS) {
        let thumbnail = a.image(&format!("cg/thum/{thumbnail}.jpg"))?.0;
        scenes.push(ReplayEntry { thumbnail, unlocked_by: cleared, label: label.to_string() });
    }
    let (exit, w, h) = a.strip(&[&format!("{DIR}/EXIT001.png"), &format!("{DIR}/EXIT002.png"), &format!("{DIR}/EXIT003.png")])?;
    let clear = a.transparent();
    let (cursor, _, _) = a.composite(&[(clear.clone(), 0, 0, 1000), (clear, 199, 74, 1000)])?;
    // src: extra_recollection.nss (from 130,80, 110 and 86 apart, five a row; EXIT 702,536; blk 500)
    Some(PictureReplay {
        background: a.image(&format!("{DIR}/エクストラレコレ背景.jpg"))?.0,
        exit: Placed { image: exit, x: 702, y: 536, w: w as i32, h: h as i32 },
        first: (130, 80),
        row_step: 86,
        column_step: 110,
        rows: 3,
        blank: a.image("cg/sys/extra/cg/未見画像アイコン.png")?.0,
        cursor,
        scenes,
        open_ms: 500,
        start_ms: 500,
        end_ms: 500,
        across: Some(5),
        hold: None,
        engine_screen: false,
    })
}
