//! Lowers a Xuse `.cd` script to the IR and writes the assets it uses.
//!
//! Assumptions that still need checking in a running conversion are marked
//! "assumption".

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use vn2ons_core::ir::{
    ArithOp, NamePlate, Op, Operand, PageButton, Placed, Rect, SaveMenu, SourceLoc, SpriteChange, SpritePlacement,
    TextLink, TextWindow, TitleAction, TitleButton, TitleScreen, Var,
};

use crate::xuse::cd::{CdError, CdScript, Instr, Link, Sound};
use crate::xuse::gaf::{Gaf, GafError};
use crate::xuse::xarc::{Xarc, XarcError};
use crate::xuse::xsa::{Xsa, XsaError};

/// Control flow, from the engine's handlers.
///
/// - 0x10/0x11 take a label *number* in the current script (`arg`); the
///   engine resolves it with (current script, label) at 0x4706a0.
/// - 0x15/0x16/0x22 (and links) take a label-table *index* (`sub`), resolved
///   at 0x470830. The engine's in-memory table has one extra entry at index
///   0 (the header counts it, the file doesn't store it), so index `n` is
///   stored entry `n - 1`.
const OP_GOTO: u16 = 0x10;
const OP_GOTO_2: u16 = 0x11;
const OP_GOTO_SUB: u16 = 0x15;
/// `if v[a] == b goto label[sub]`, (a, b) as two u32 at pool[arg].
const OP_IF_EQUAL: u16 = 0x16;
const OP_GOSUB: u16 = 0x22;
const OP_RETURN: u16 = 0x23;
/// Ends a scene entered through a link and goes back to the page with the
/// link. The handler (`ScriptText.cpp`) fails with "in the root scene, can't
/// go back further" when no link scene is active.
const OP_LINK_RETURN: u16 = 0x47;
/// Timed waits: the low 16 bits of `arg` are milliseconds (the high half
/// is a flag, meaning unknown). Both handlers install a wait callback that
/// also checks for clicks.
/// Arms a screen transition: changes until the next [`OP_TRANSITION_WAIT`]
/// are revealed through it. `sub` = effect (jump table at 0x44a138);
/// pool[arg] = u32 duration ms, u32 flags, u32 curve/parameter.
const OP_TRANSITION: u16 = 0x76;
/// Arms a mask ("rule") transition (`Trans_MaskBlend2`): pool[arg] = u32
/// mask number (`mask%04u.png` in `HD/mask.arc`), i32 softness (sign =
/// direction), then the same 12-byte timing block as 0x76.
const OP_MASK_TRANSITION: u16 = 0x77;
/// Waits for the running transition (a click finishes it early).
const OP_TRANSITION_WAIT: u16 = 0x71;
/// Text window preset parts, found only in script 2's preset subroutines:
/// mode (0 = message box, 1 = full screen), style (`TextWnd{style}.004` in
/// `HD/wnd.arc`), position (x | y << 16).
const OP_WINDOW_MODE: u16 = 0x4c;
const OP_WINDOW_STYLE: u16 = 0x4f;
const OP_WINDOW_POS: u16 = 0x50;
/// Text control tags between messages (handler names "brn" and "p"):
/// continue below after a click / start a new page. Only used in
/// full-screen modes, where untagged messages continue too.
const OP_TEXT_BRN: u16 = 0x52;
const OP_TEXT_PAGE: u16 = 0x53;
/// Hides the text window (handler 0x4772a0; arg 0 waits for the fade, 1
/// doesn't). Used before most transitions.
const OP_HIDE_TEXT: u16 = 0x43;
/// Removes every character bust-up (arg is always -1).
const OP_CLEAR_SPRITES: u16 = 0x8f;
const OP_WAIT: u16 = 0x66;
const OP_WAIT_2: u16 = 0x72;
const BG_PER_ARCHIVE: usize = 64;
const BU_PER_ARCHIVE: usize = 128;
const SE_FIRST_BANK: usize = 256;
/// Assumption: backgrounds fade over this long (the `0x54` sub field may
/// select the transition; not decoded yet).
const BG_FADE_MS: u32 = 500;
/// Opacity of the text window's base layer (estimated from a screenshot of
/// the original game).
const WINDOW_OPACITY_PERCENT: u32 = 55;
/// Assumption: the title screen fades in and out over this long.
const TITLE_FADE_MS: u32 = 1000;
/// Assumption: the sprite entry's fade field counts 10 ms units.
const SPRITE_FADE_UNIT_MS: u32 = 10;

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Gaf { path: PathBuf, source: GafError },
    #[error("{path}: {source}")]
    Xsa { path: PathBuf, source: XsaError },
    #[error("{path}: {source}")]
    Xarc { path: PathBuf, source: XarcError },
    #[error("{what}: {source}")]
    Image { what: String, source: image::ImageError },
    #[error(transparent)]
    Cd(#[from] CdError),
    #[error("{kind} {number} does not exist")]
    MissingAsset { kind: &'static str, number: usize },
}

/// Reads images and sounds from an installed game and writes the ones that
/// are used into the output directory. Keeps only names and sizes in memory;
/// use [`AssetBank::prefetch`] to read each archive once for a whole game.
pub struct AssetBank {
    game: PathBuf,
    out: PathBuf,
    images: HashMap<(&'static str, usize), Image>,
    written: HashMap<(&'static str, usize), String>,
    used_names: HashSet<String>,
    /// The sound bank read last; voices arrive mostly in bank order.
    last_bank: Option<(PathBuf, Vec<u8>)>,
}

/// The assets a set of scripts uses (numbers as in the script).
#[derive(Debug, Default)]
pub struct Needs {
    pub backgrounds: BTreeSet<usize>,
    pub sprites: BTreeSet<usize>,
    pub bgm: BTreeSet<usize>,
    pub se: BTreeSet<usize>,
}

impl Needs {
    /// Adds what `cd` refers to. Undecodable records are skipped here; the
    /// lowering reports them.
    pub fn add_script(&mut self, cd: &CdScript) {
        for ins in &cd.code {
            if ins.op == crate::xuse::cd::OP_BACKGROUND && ins.arg > 0 {
                self.backgrounds.insert(ins.arg as usize);
            }
            if let Some(Ok(entries)) = cd.sprites(ins) {
                for e in entries {
                    if e.mode & 1 != 0 {
                        self.sprites.insert(e.sprite as usize);
                    }
                }
            }
            match cd.sound(ins) {
                Some(Ok(Sound::BgmPlay { track, .. })) => {
                    self.bgm.insert(track as usize);
                }
                Some(Ok(Sound::SePlay { sound, .. })) => {
                    self.se.insert(sound as usize);
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

impl AssetBank {
    pub fn new(game: &Path, out: &Path) -> Self {
        Self {
            game: game.to_path_buf(),
            out: out.to_path_buf(),
            images: HashMap::new(),
            written: HashMap::new(),
            used_names: HashSet::new(),
            last_bank: None,
        }
    }

    /// Background `number` as used by opcode 0x54 (1-based).
    pub fn background(&mut self, number: usize) -> Result<Image, ConvertError> {
        let index = number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "background", number })?;
        self.image("bg", "background", "bg", index, BG_PER_ARCHIVE, number)
    }

    /// Sprite `number` as used by opcode 0x8e (1-based: the handler loads
    /// image id 19,999,999 + number, bust-ups being 20,000,000 + index).
    pub fn sprite(&mut self, number: usize) -> Result<Image, ConvertError> {
        let index = number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "sprite", number })?;
        self.image("bu", "sprite", "sprite", index, BU_PER_ARCHIVE, number)
    }

    /// BGM track (1-based).
    pub fn bgm(&mut self, track: usize) -> Result<String, ConvertError> {
        let slot = track.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "BGM track", number: track })?;
        self.sound("bm0000.bin", slot, "bgm", format!("bgm{track:02}.ogg"), "BGM track", track)
    }

    /// Sound effect (1-based, across `k0000.bin` and `k0001.bin`).
    pub fn se(&mut self, number: usize) -> Result<String, ConvertError> {
        let index = number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "sound effect", number })?;
        let (bank, slot) = if index < SE_FIRST_BANK {
            ("k0000.bin", index)
        } else {
            ("k0001.bin", index - SE_FIRST_BANK)
        };
        self.sound(bank, slot, "se", format!("se{number:03}.ogg"), "sound effect", number)
    }

    /// Voice clip for a message's voice id. The engine plays clip
    /// `(id - 1) % 100000` (`id / 100000` is the character, for the
    /// per-character voice switch), from `CD/BV{clip >> 8}.xsa`, slot
    /// `clip & 0xff` (handlers 0x41f080 and 0x47f620).
    pub fn voice(&mut self, id: u32) -> Result<String, ConvertError> {
        let number = id as usize;
        let clip = (number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "voice", number })?) % 100_000;
        let path = self.game.join("CD").join(format!("bv{:04}.xsa", clip >> 8));
        self.sound_at(path, clip & 0xff, "voice", format!("v{id}.ogg"), "voice", number)
    }

    /// Transition mask `number` from `HD/mask.arc` as a greyscale PNG
    /// (inverted if asked).
    pub fn mask(&mut self, number: usize, inverted: bool) -> Result<String, ConvertError> {
        let kind = if inverted { "inverted mask" } else { "mask" };
        if let Some(p) = self.written.get(&(kind, number)) {
            return Ok(p.clone());
        }
        let path = self.game.join("HD").join("mask.arc");
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let arc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let file = format!("mask{number:04}.png");
        let entry = arc.entries.iter().find(|e| e.name == file).ok_or(ConvertError::MissingAsset { kind, number })?;
        let mut img = image::load_from_memory_with_format(arc.data(entry), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: file.clone(), source })?
            .to_luma8();
        if inverted {
            image::imageops::invert(&mut img);
        }
        let name = format!("system/mask{number:04}{}.png", if inverted { "r" } else { "" });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|source| ConvertError::Image { what: name.clone(), source })?;
        write_file(&self.out.join(&name), &png)?;
        self.written.insert((kind, number), name.clone());
        Ok(name)
    }

    /// Text window for a preset: the measured ADV box for style 0, else the
    /// style's `TextWnd{n}.004` graphics at the preset's height, centred.
    ///
    /// Metrics per style are estimates from the window size and the longest
    /// lines / most lines per page the game shows in that style (not yet
    /// checked against screenshots of the original): font size, line gap,
    /// columns, rows.
    pub fn text_window_style(&mut self, preset: WindowPreset) -> Result<TextWindow, ConvertError> {
        if preset.style == 0 {
            return self.text_window();
        }
        let (font_size, line_gap, columns, rows) = match preset.style {
            1 => (21, 14, 26, 12),
            2 | 7 => (24, 11, 25, 13),
            3 | 4 => (18, 4, 40, 23),
            5 => (16, 11, 40, 16),
            6 => (24, 12, 26, 5),
            8 => (19, 15, 36, 15),
            _ => (24, 12, 26, 3),
        };
        let file = format!("TextWnd{}.004", preset.style);
        let images = self.wnd_images(&file)?;
        let image = format!("system/textwnd{}.png", preset.style);
        self.layered(&image, &images[0], &images[1])?;
        let (w, _) = png_size(&images[0]);
        let x = (800 - w as i32) / 2;
        // The ADV box draws 3 px below its preset position (measured).
        let y = if preset.style == 6 { preset.y + 3 } else { preset.y };
        let name_plate = if png_size(&images[2]).1 > 1 {
            let plate = format!("system/nameplate{}.png", preset.style);
            self.layered(&plate, &images[2], &images[3])?;
            Some(NamePlate { image: plate, x: x - 5, y: y - 40, text_x: x + 11, text_y: y - 28, font_size: 20 })
        } else {
            None
        };
        let text_x = x + (w as i32 - columns as i32 * (font_size as i32 + 1)) / 2;
        Ok(TextWindow { image, x, y, text_x, text_y: y + 20, columns, rows, font_size, line_gap, name_plate })
    }

    /// The four PNG layers (message base, frame, name base, frame) of a
    /// `TextWnd*.004` in `HD/wnd.arc`.
    fn wnd_images(&self, file: &str) -> Result<Vec<Vec<u8>>, ConvertError> {
        let images = self.gaf_images("wnd.arc", file, "text window")?;
        if images.len() < 4 {
            return Err(ConvertError::MissingAsset { kind: "text window layer", number: images.len() });
        }
        Ok(images.into_iter().take(4).collect())
    }

    /// The PNGs of GAF4 container `file` inside the XARC archive `HD/{arc}`.
    fn gaf_images(&self, arc: &str, file: &str, kind: &'static str) -> Result<Vec<Vec<u8>>, ConvertError> {
        let path = self.game.join("HD").join(arc);
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let arc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let entry = arc.entries.iter().find(|e| e.name == file).ok_or(ConvertError::MissingAsset { kind, number: 0 })?;
        let gaf_path = path.join(file);
        let gaf = Gaf::parse(arc.data(entry)).map_err(|source| ConvertError::Gaf { path: gaf_path.clone(), source })?;
        let images = gaf.images().map_err(|source| ConvertError::Gaf { path: gaf_path, source })?;
        Ok(images.iter().map(|i| i.png.to_vec()).collect())
    }

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
        image::imageops::overlay(&mut background, &get(10)?, 0, 171);
        image::imageops::overlay(&mut background, &get(14)?, 101, 384);
        let background_path = "system/title.png".to_string();
        self.save_image(&background_path, &background)?;

        let mut buttons = Vec::new();
        for (n, name, x, y, action) in [
            (6, "start", 631, 221, TitleAction::Start),
            (2, "load", 631, 250, TitleAction::Load),
            (7, "option", 631, 299, TitleAction::Unavailable),
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
            buttons.push(TitleButton { image, x, y, action });
        }
        Ok(TitleScreen { background: background_path, fade_ms: TITLE_FADE_MS, buttons })
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
            save_header: placed(self, "header_save", &get(12)?, 627, 0)?,
            load_header: placed(self, "header_load", &get(11)?, 627, 0)?,
            delete: button(self, "delete", 5, 512, 531)?,
            save: button(self, "save", 8, 615, 531)?,
            load: button(self, "load", 6, 615, 531)?,
            close: button(self, "close", 4, 702, 531)?,
            pages,
            page_marker,
            rows: 5,
            row_y: 44,
            row_pitch: 92,
            row_hover,
            row_selected,
            thumbnail,
            new_icon: placed(self, "new", &get(10)?, 50, 67)?,
            text_x: 124,
            text_y: 11,
            font_size: 14,
            line_height: 15,
            preview: Rect { x: 390, y: 137, w: 400, h: 300 },
        })
    }

    fn save_image(&self, name: &str, img: &image::RgbaImage) -> Result<(), ConvertError> {
        let dest = self.out.join(name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| ConvertError::Io { path: parent.to_path_buf(), source })?;
        }
        img.save(&dest).map_err(|source| ConvertError::Image { what: name.to_string(), source })
    }

    /// Builds the ADV text box from the game's own window graphics
    /// (`HD/wnd.arc` → `TextWnd0.004`: message base + frame, name base +
    /// frame). Positions match the original game's 800×600 layout.
    pub fn text_window(&mut self) -> Result<TextWindow, ConvertError> {
        let images = self.wnd_images("TextWnd0.004")?;
        let window = "system/textwnd.png".to_string();
        let plate = "system/nameplate.png".to_string();
        self.layered(&window, &images[0], &images[1])?;
        self.layered(&plate, &images[2], &images[3])?;
        // Positions measured on screenshots of the original game.
        Ok(TextWindow {
            image: window,
            x: 47,
            y: 452,
            text_x: 89,
            text_y: 474,
            columns: 26,
            rows: 3,
            font_size: 24,
            line_gap: 12,
            name_plate: Some(NamePlate { image: plate, x: 42, y: 412, text_x: 58, text_y: 424, font_size: 20 }),
        })
    }

    /// Writes `top` composited over `base` (both PNGs of the same size).
    ///
    /// The game alpha-blends the base layer (an opaque grey gradient) over
    /// the scene at about 55% opacity; measured on an original screenshot,
    /// where the box's light top brightens the scene and its dark middle
    /// darkens it.
    fn layered(&self, name: &str, base: &[u8], top: &[u8]) -> Result<(), ConvertError> {
        let load = |png: &[u8]| {
            image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .map(|i| i.to_rgba8())
                .map_err(|source| ConvertError::Image { what: name.to_string(), source })
        };
        let mut out = load(base)?;
        for px in out.pixels_mut() {
            px.0[3] = (u32::from(px.0[3]) * WINDOW_OPACITY_PERCENT / 100) as u8;
        }
        image::imageops::overlay(&mut out, &load(top)?, 0, 0);
        self.save_image(name, &out)
    }

    /// Reads every archive the `needs` touch once and writes the used
    /// assets. Numbers that don't exist are left for the lowering to report.
    pub fn prefetch(&mut self, needs: &Needs) -> Result<(), ConvertError> {
        let bg: Vec<usize> = needs.backgrounds.iter().filter_map(|n| n.checked_sub(1)).collect();
        self.prefetch_images("bg", "background", "bg", &bg, BG_PER_ARCHIVE, 1)?;
        let bu: Vec<usize> = needs.sprites.iter().filter_map(|n| n.checked_sub(1)).collect();
        self.prefetch_images("bu", "sprite", "sprite", &bu, BU_PER_ARCHIVE, 1)?;
        for &t in &needs.bgm {
            let _ = self.bgm(t);
        }
        for &n in &needs.se {
            let _ = self.se(n);
        }
        Ok(())
    }

    /// `indices` are 0-based; `number_offset` converts back to the script's
    /// numbering (1 for backgrounds and sprites).
    fn prefetch_images(
        &mut self,
        prefix: &str,
        kind: &'static str,
        dir: &str,
        indices: &[usize],
        per_archive: usize,
        number_offset: usize,
    ) -> Result<(), ConvertError> {
        let mut by_archive: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &i in indices {
            by_archive.entry(i / per_archive).or_default().push(i);
        }
        for (archive, wanted) in by_archive {
            let path = self.game.join("HD").join(format!("{prefix}{archive:04}.004"));
            if !path.exists() {
                continue;
            }
            let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
            let gaf = Gaf::parse(&bytes).map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
            let images = gaf.images().map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
            for i in wanted {
                if let Some(img) = images.get(i % per_archive) {
                    self.store_image(kind, dir, i + number_offset, img)?;
                }
            }
        }
        Ok(())
    }

    fn store_image(
        &mut self,
        kind: &'static str,
        dir: &str,
        number: usize,
        img: &crate::xuse::gaf::GafImage<'_>,
    ) -> Result<Image, ConvertError> {
        if let Some(done) = self.images.get(&(kind, number)) {
            return Ok(done.clone());
        }
        let (width, height) = png_size(img.png);
        let name = unique(&mut self.used_names, dir, &img.file_name(), number);
        write_file(&self.out.join(&name), img.png)?;
        let image = Image { path: name, width, height };
        self.images.insert((kind, number), image.clone());
        Ok(image)
    }

    fn image(
        &mut self,
        prefix: &str,
        kind: &'static str,
        dir: &str,
        index: usize,
        per_archive: usize,
        number: usize,
    ) -> Result<Image, ConvertError> {
        if let Some(done) = self.images.get(&(kind, number)) {
            return Ok(done.clone());
        }
        let path = self.game.join("HD").join(format!("{prefix}{:04}.004", index / per_archive));
        if !path.exists() {
            return Err(ConvertError::MissingAsset { kind, number });
        }
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let gaf = Gaf::parse(&bytes).map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
        let images = gaf.images().map_err(|source| ConvertError::Gaf { path: path.clone(), source })?;
        let img = images.get(index % per_archive).ok_or(ConvertError::MissingAsset { kind, number })?;
        self.store_image(kind, dir, number, img)
    }

    fn sound(
        &mut self,
        bank: &str,
        slot: usize,
        dir: &str,
        file_name: String,
        kind: &'static str,
        number: usize,
    ) -> Result<String, ConvertError> {
        let path = self.game.join("HD").join(bank);
        self.sound_at(path, slot, dir, file_name, kind, number)
    }

    fn sound_at(
        &mut self,
        path: PathBuf,
        slot: usize,
        dir: &str,
        file_name: String,
        kind: &'static str,
        number: usize,
    ) -> Result<String, ConvertError> {
        if let Some(p) = self.written.get(&(kind, number)) {
            return Ok(p.clone());
        }
        if self.last_bank.as_ref().is_none_or(|(p, _)| *p != path) {
            let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
            self.last_bank = Some((path.clone(), bytes));
        }
        let bytes = &self.last_bank.as_ref().unwrap().1;
        let xsa = Xsa::parse(bytes).map_err(|source| ConvertError::Xsa { path: path.clone(), source })?;
        let entry = xsa.entries.iter().find(|e| e.slot == slot).ok_or(ConvertError::MissingAsset { kind, number })?;
        let name = format!("{dir}/{file_name}");
        write_file(&self.out.join(&name), xsa.data(entry))?;
        self.written.insert((kind, number), name.clone());
        Ok(name)
    }
}

/// How jumps are converted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Convert unconditional jumps; conditional ones are left out.
    Jumps,
    /// Convert no jumps at all: every script plays straight through. A
    /// stopgap until conditions are decoded, since a jump without its
    /// condition can loop forever.
    Linear,
}

/// A text window preset subroutine (see [`OP_WINDOW_MODE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPreset {
    pub full_screen: bool,
    pub style: u32,
    pub x: i32,
    pub y: i32,
}

/// Finds the preset subroutines: labels whose code sets a window position.
/// Keyed by IR label name.
pub fn window_presets(scripts: &[(String, CdScript)]) -> HashMap<String, WindowPreset> {
    let mut presets = HashMap::new();
    for (_, cd) in scripts {
        let own: HashMap<usize, u32> = cd
            .labels
            .iter()
            .filter(|l| l.script == cd.script_id)
            .map(|l| (l.code_offset as usize / 12 + 1, l.label))
            .collect();
        let (mut label, mut mode, mut style) = (None, 0, 0);
        for (index, ins) in cd.code.iter().enumerate() {
            if let Some(l) = own.get(&index) {
                label = Some(*l);
            }
            match ins.op {
                OP_WINDOW_MODE => mode = ins.arg,
                OP_WINDOW_STYLE => style = ins.arg,
                OP_WINDOW_POS => {
                    if let Some(l) = label {
                        let preset = WindowPreset {
                            full_screen: mode == 1,
                            style: style as u32,
                            x: ins.arg & 0xffff,
                            y: ins.arg >> 16,
                        };
                        presets.insert(format!("s{}_l{l}", cd.script_id), preset);
                    }
                }
                _ => {}
            }
        }
    }
    presets
}

/// Lowers one script. Instructions that can't be expressed yet become
/// [`Op::Unsupported`]; assets are written through `assets`.
pub fn lower_script(
    cd: &CdScript,
    file: &str,
    flow: Flow,
    assets: &mut AssetBank,
    presets: &HashMap<String, WindowPreset>,
) -> Result<Vec<Op>, ConvertError> {
    // A label's offset points at a marker instruction (a 0x10 that targets
    // itself); a jump lands just after it, so the label goes after that
    // instruction.
    let own_labels: HashMap<usize, u32> = cd
        .labels
        .iter()
        .filter(|l| l.script == cd.script_id)
        .map(|l| (l.code_offset as usize / 12 + 1, l.label))
        .collect();

    let mut ops = Vec::new();
    // Links (opcode 0x46) apply to the next message.
    let mut pending_links: Vec<Link> = Vec::new();
    // A transition armed by 0x76 and not yet waited on (file order).
    let mut transition_armed = false;
    // Full-screen text mode (file order): rows of the window, the index of
    // the last text op and the lines on its page, and whether a "p" tag
    // followed it.
    let mut full_screen_rows: Option<usize> = None;
    let mut last_text: Option<(usize, usize)> = None;
    let mut page_break = false;
    for (index, ins) in cd.code.iter().enumerate() {
        if let Some(label) = own_labels.get(&index) {
            ops.push(Op::Label(format!("s{}_l{label}", cd.script_id)));
        }
        if let Some(link) = cd.link(ins) {
            pending_links.push(link?);
            continue;
        }
        let at = || SourceLoc { file: file.to_string(), index };
        if matches!(ins.op, OP_GOTO | OP_GOTO_2) && own_label_at(cd, ins.arg).is_some_and(|i| i == index) {
            continue; // label marker: a jump to the label sitting on this instruction
        }
        if flow == Flow::Linear && matches!(ins.op, OP_GOTO | OP_GOTO_2 | OP_GOTO_SUB | OP_IF_EQUAL | OP_GOSUB | OP_RETURN | OP_LINK_RETURN) {
            ops.push(Op::Unsupported { what: format!("opcode {:#04x} (linear mode)", ins.op), at: at() });
            continue;
        }
        if ins.op == OP_MASK_TRANSITION {
            let o = ins.arg as u32 as usize;
            match (pool_u32(cd.pool, o), pool_u32(cd.pool, o + 4), pool_u32(cd.pool, o + 8)) {
                (Some(number), Some(soft), Some(fade_ms)) => {
                    // Assumption: negative softness runs the mask bright-first.
                    let mask = match assets.mask(number as usize, (soft as i32) < 0) {
                        Ok(m) => Some(m),
                        Err(ConvertError::MissingAsset { .. }) => None,
                        Err(e) => return Err(e),
                    };
                    transition_armed = true;
                    let effect = format!("MaskBlend2 mask {number} softness {}", soft as i32);
                    ops.push(Op::BeginTransition { effect, fade_ms, mask });
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (bad record)", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_TRANSITION {
            match pool_u32(cd.pool, ins.arg as u32 as usize) {
                Some(fade_ms) => {
                    transition_armed = true;
                    ops.push(Op::BeginTransition { effect: transition_name(ins.sub).to_string(), fade_ms, mask: None });
                }
                None => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (bad record)", ins.op), at: at() }),
            }
            continue;
        }
        if matches!(ins.op, OP_TEXT_BRN | OP_WINDOW_MODE | OP_WINDOW_STYLE | OP_WINDOW_POS) {
            continue; // brn = the default in full-screen modes; presets: see window_presets
        }
        if ins.op == OP_TEXT_PAGE {
            page_break = true;
            continue;
        }
        if ins.op == OP_TRANSITION_WAIT {
            if std::mem::take(&mut transition_armed) {
                ops.push(Op::EndTransition);
            }
            continue;
        }
        let links = if cd.message(ins).is_some() { std::mem::take(&mut pending_links) } else { Vec::new() };
        let unsupported = || format!("opcode {:#04x} sub {:#x} arg {}", ins.op, ins.sub, ins.arg);
        match lower_instr(cd, ins, &links, assets) {
            Ok(Some(Op::Text { speaker, text, voice, mut links, continues })) if flow == Flow::Linear => {
                // No jumps in linear mode, so links are colour only.
                links.iter_mut().for_each(|l| l.target = None);
                ops.push(Op::Text { speaker, text, voice, links, continues });
            }
            Ok(Some(Op::Text { speaker, text, voice, links, .. })) => {
                let lines = text.split('\n').count();
                if let (Some(rows), Some((i, used))) = (full_screen_rows, last_text) {
                    let joins = !page_break && used + lines <= rows;
                    if joins && let Some(Op::Text { continues, .. }) = ops.get_mut(i) {
                        *continues = true;
                    }
                    let used = if joins { used + lines } else { lines };
                    last_text = Some((ops.len(), used));
                } else if full_screen_rows.is_some() {
                    last_text = Some((ops.len(), lines));
                }
                page_break = false;
                ops.push(Op::Text { speaker, text, voice, links, continues: false });
            }
            Ok(Some(Op::Gosub(target))) => {
                let preset = presets.get(&target).copied();
                ops.push(Op::Gosub(target));
                if let Some(p) = preset {
                    let window = assets.text_window_style(p)?;
                    full_screen_rows = p.full_screen.then_some(window.rows as usize);
                    last_text = None;
                    ops.push(Op::SetWindow(window));
                }
            }
            Ok(Some(op)) => ops.push(op),
            Ok(None) => ops.push(Op::Unsupported { what: unsupported(), at: at() }),
            Err(ConvertError::MissingAsset { kind, number }) => {
                ops.push(Op::Unsupported { what: format!("{} (missing {kind} {number})", unsupported()), at: at() })
            }
            Err(e) => return Err(e),
        }
    }
    Ok(ops)
}

/// Converts a link's (row, column, length) into a character range of the
/// page text. Rows count message segments from 1, including the speaker's
/// name segment; links that don't land inside the text are dropped.
fn link_range(segments: &[String], has_speaker: bool, link: &Link) -> Option<(usize, usize)> {
    let row = usize::from(link.row).checked_sub(1)?;
    let col = usize::from(link.column).checked_sub(1)?;
    let first_body = usize::from(has_speaker);
    if row < first_body || row >= segments.len() {
        return None;
    }
    let len = usize::from(link.length);
    if col + len > segments[row].chars().count() {
        return None;
    }
    // Body segments are joined with '\n', which counts as one character.
    let before: usize = segments[first_body..row].iter().map(|s| s.chars().count() + 1).sum();
    Some((before + col, len))
}

fn lower_instr(cd: &CdScript, ins: &Instr, links: &[Link], assets: &mut AssetBank) -> Result<Option<Op>, ConvertError> {
    if let Some(msg) = cd.message(ins) {
        let msg = msg?;
        let segs = &msg.segments;
        // A short first segment followed by a quoted one is the speaker's name.
        let has_speaker = segs.len() >= 2
            && (1..=8).contains(&segs[0].chars().count())
            && segs[1].starts_with(['「', '『', '（']);
        let (speaker, body) = if has_speaker { (Some(segs[0].clone()), &segs[1..]) } else { (None, &segs[..]) };
        let links = links
            .iter()
            .filter_map(|l| {
                let (start, len) = link_range(segs, has_speaker, l)?;
                Some(TextLink { start, len, target: table_label(cd, l.target.into()) })
            })
            .collect();
        // Segments are the game's own display lines.
        // A missing clip only drops the voice, not the text.
        let voice = match msg.voice_id.map(|id| assets.voice(id)) {
            Some(Err(ConvertError::MissingAsset { .. })) | None => None,
            Some(r) => Some(r?),
        };
        return Ok(Some(Op::Text { speaker, text: body.join("\n"), voice, links, continues: false }));
    }
    if let Some(sound) = cd.sound(ins) {
        return Ok(Some(match sound? {
            Sound::BgmPlay { track, fade_in_ms } => {
                Op::BgmPlay { file: assets.bgm(track as usize)?, fade_in_ms: fade_in_ms.into() }
            }
            Sound::BgmStop { fade_out_ms, .. } => Op::BgmStop { fade_out_ms: fade_out_ms.into() },
            Sound::SePlay { sound, looped, .. } => Op::SePlay { file: assets.se(sound as usize)?, looped },
            Sound::SeStop { sound: 0, .. } => Op::SeStop { file: None },
            Sound::SeStop { sound, .. } => Op::SeStop { file: Some(assets.se(sound as usize)?) },
        }));
    }
    if let Some(entries) = cd.sprites(ins) {
        let entries = entries?;
        let mut changes = Vec::new();
        for e in &entries {
            // Each character (1-100) has its own bust-up object, and an entry
            // only touches that one. Mode bit 0 clear removes it (handler
            // 0x434130 deletes the "bustup-%u" object).
            let layer = u32::from(e.character);
            if e.mode & 1 == 0 {
                changes.push(SpriteChange { layer, sprite: None });
                continue;
            }
            let img = assets.sprite(e.sprite as usize)?;
            // Assumption: (x, y) is the sprite's centre on the 800×600 screen.
            let sprite = SpritePlacement {
                image: img.path,
                x: i32::from(e.x) - img.width as i32 / 2,
                y: i32::from(e.y) - img.height as i32 / 2,
            };
            changes.push(SpriteChange { layer, sprite: Some(sprite) });
        }
        let fade_ms = entries.iter().map(|e| u32::from(e.fade) * SPRITE_FADE_UNIT_MS).max().unwrap_or(0);
        return Ok(Some(Op::Sprites { changes, fade_ms }));
    }
    // Label-table index -> label name (index 0 is the engine's implicit
    // entry, not stored in the file; its meaning isn't known).
    let label = |n: i64| table_label(cd, n);
    let var = |v: u32| Var { global: v & 0x8000 != 0, index: (v & 0x7fff) as u16 };
    match ins.op {
        OP_GOTO | OP_GOTO_2 => {
            return Ok(own_label_at(cd, ins.arg).map(|_| Op::Jump(format!("s{}_l{}", cd.script_id, ins.arg))));
        }
        OP_GOTO_SUB => return Ok(label(ins.sub.into()).map(Op::Jump)),
        OP_GOSUB => return Ok(label(ins.sub.into()).map(Op::Gosub)),
        OP_RETURN | OP_LINK_RETURN => return Ok(Some(Op::Return)),
        OP_CLEAR_SPRITES => return Ok(Some(Op::ClearSprites)),
        OP_HIDE_TEXT => return Ok(Some(Op::HideTextWindow)),
        OP_WAIT | OP_WAIT_2 => return Ok(Some(Op::Wait { ms: u32::from(ins.arg as u16) })),
        OP_IF_EQUAL => {
            let Some(target) = label(ins.sub.into()) else { return Ok(None) };
            let o = ins.arg as u32 as usize;
            let (Some(a), Some(b)) = (pool_u32(cd.pool, o), pool_u32(cd.pool, o + 4)) else { return Ok(None) };
            return Ok(Some(Op::IfEqual { var: var(a), value: b as i32, label: target }));
        }
        // Arithmetic on variables (sub = target variable). 0x03 shares
        // the "-= variable" handler in the engine's dispatch table.
        0x01..=0x0e => {
            let (op, by_var) = match ins.op {
                0x01 => (ArithOp::Set, false),
                0x02 => (ArithOp::Add, false),
                0x03 => (ArithOp::Sub, true),
                0x04 => (ArithOp::Mul, false),
                0x05 => (ArithOp::Div, false),
                0x06 => (ArithOp::BitAnd, false),
                0x07 => (ArithOp::BitOr, false),
                0x08 => (ArithOp::Set, true),
                0x09 => (ArithOp::Add, true),
                0x0a => (ArithOp::Sub, true),
                0x0b => (ArithOp::Mul, true),
                0x0c => (ArithOp::Div, true),
                0x0d => (ArithOp::BitAnd, true),
                _ => (ArithOp::BitOr, true),
            };
            let value = if by_var { Operand::Var(var(ins.arg as u32 & 0xffff)) } else { Operand::Const(ins.arg) };
            return Ok(Some(Op::Arith { var: var(ins.sub.into()), op, value }));
        }
        _ => {}
    }
    if ins.op == crate::xuse::cd::OP_BACKGROUND {
        if ins.arg <= 0 {
            return Ok(None);
        }
        let img = assets.background(ins.arg as usize)?;
        return Ok(Some(Op::Background { image: img.path, fade_ms: BG_FADE_MS }));
    }
    Ok(None)
}

/// The instruction index a label number of this script is placed at, if
/// the script defines it.
fn own_label_at(cd: &CdScript, label: i32) -> Option<usize> {
    cd.labels
        .iter()
        .find(|l| l.script == cd.script_id && i64::from(l.label) == i64::from(label))
        .map(|l| l.code_offset as usize / 12)
}

/// Transition effect for 0x76's `sub`, named after the engine's
/// `Trans_*.cpp` source files (directions within a family not known).
fn transition_name(sub: u16) -> &'static str {
    match sub {
        0..=3 => "Wave",
        4 => "Dot4x4",
        5..=8 => "SoftEdgeWipe",
        9 => "CrossFade",
        10..=17 => "Slide",
        26 => "Dissolve",
        27 => "Mosaic",
        28..=31 => "BoxRolling",
        32..=35 => "RollFlip",
        36 => "SmoothCrossFade",
        37..=40 => "Zoom",
        41..=44 => "Scroll",
        45 => "MeltDown black",
        46 => "MeltDown grey",
        47 => "MeltDown white",
        48 => "TvPowerSwitch",
        _ => "unknown",
    }
}

/// Resolves a label-table index (as used by 0x15/0x16/0x22 and links) to
/// the IR label name. Index 0 is the engine's implicit entry.
pub fn table_label(cd: &CdScript, index: i64) -> Option<String> {
    let stored = usize::try_from(index).ok()?.checked_sub(1)?;
    cd.labels.get(stored).map(|l| format!("s{}_l{}", l.script, l.label))
}

fn pool_u32(pool: &[u8], offset: usize) -> Option<u32> {
    pool.get(offset..offset + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn png_size(png: &[u8]) -> (u32, u32) {
    let be = |o: usize| png.get(o..o + 4).map_or(0, |b| u32::from_be_bytes(b.try_into().unwrap()));
    (be(16), be(20))
}

/// `dir/name`, made unique if another asset already used that name.
fn unique(used: &mut HashSet<String>, dir: &str, name: &str, number: usize) -> String {
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' })
        .collect();
    let mut candidate = format!("{dir}/{safe}");
    if !used.insert(candidate.clone()) {
        candidate = format!("{dir}/{number}_{safe}");
        used.insert(candidate.clone());
    }
    candidate
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), ConvertError> {
    let io = |source| ConvertError::Io { path: path.to_path_buf(), source };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io)?;
    }
    fs::write(path, data).map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names() {
        let mut used = HashSet::new();
        assert_eq!(unique(&mut used, "bg", "a b.png", 3), "bg/a_b.png");
        assert_eq!(unique(&mut used, "bg", "a b.png", 9), "bg/9_a_b.png");
        assert_eq!(unique(&mut used, "sprite", "背景.png", 1), "sprite/__.png");
    }

    #[test]
    fn link_ranges() {
        let segs: Vec<String> = ["名前", "「一行目です", "二行目」"].iter().map(|s| s.to_string()).collect();
        let link = |row, column, length| Link { row, column, length, style: 0x8007, target: 0, reserved: 0 };
        // Row 2 is the first text line (row 1 is the name).
        assert_eq!(link_range(&segs, true, &link(2, 2, 3)), Some((1, 3)));
        assert_eq!(link_range(&segs, true, &link(3, 1, 2)), Some((7, 2)));
        assert_eq!(link_range(&segs, true, &link(1, 1, 2)), None);
        assert_eq!(link_range(&segs, true, &link(3, 3, 5)), None);
        assert_eq!(link_range(&segs[1..], false, &link(1, 2, 3)), Some((1, 3)));
    }

    #[test]
    fn reads_png_size() {
        let mut png = vec![0u8; 24];
        png[16..20].copy_from_slice(&348u32.to_be_bytes());
        png[20..24].copy_from_slice(&700u32.to_be_bytes());
        assert_eq!(png_size(&png), (348, 700));
        assert_eq!(png_size(&[]), (0, 0));
    }
}
