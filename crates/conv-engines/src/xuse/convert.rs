//! Lowers a Xuse `.cd` script to the IR and writes the assets it uses.
//!
//! Assumptions that still need checking in a running conversion are marked
//! "assumption".

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use conv_core::ir::{
    ArithOp, Backlog, BarAction, BarButton, ButtonBar, ConfirmDialog, NamePlate, Setting, SettingRow, SettingsMenu, Op, Operand, PageButton, Placed, Rect, Ruby, SaveMenu, TextStyle, SourceLoc, Voice, VoiceToggle,
    SpriteChange, SpriteFilter, SpritePlacement, TextLink, TextSpot, TextWindow, TitleAction, TitleButton, TitleScreen, TopBars,
    SlideLayer, TransitionStyle, Var, ZoomStyle, FILTER_BLEND_ADD, FILTER_BLEND_COPY, FILTER_BLEND_MULTIPLY, FILTER_BLEND_SCREEN, FILTER_BRIGHTNESS, FILTER_COLOUR, FILTER_MONOTONE,
    FILTER_GREYSCALE, FILTER_NEGATIVE,
};

use crate::xuse::cd::{
    CdError, CdScript, Instr, Link, PresetModule, Ruby as RubyRecord, ScreenModule, Sound, SpriteEntry, TextStyle as StyleRecord, OP_TEXT_INDENT, PRESET_KIND_MONOTONE, PRESET_BLEND_ADD, PRESET_BLEND_ALPHA, PRESET_BLEND_COPY,
    PRESET_BLEND_MULTI, PRESET_BLEND_SCREEN, PRESET_KIND_BRIGHTNESS, PRESET_KIND_COLOUR, PRESET_KIND_GREYSCALE,
    PRESET_KIND_NEGATIVE,
};
use crate::xuse::gaf::{Gaf, GafError};
use crate::xuse::symbols::SymbolTable;
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
/// How the current text window shows its messages (handler 0x467a50; byte
/// +0x1c of the window, copied to the message at 0x46e882): 0 types one
/// character at a time, 1 shows the whole message at once, 2 types all
/// its lines at the same time (seen in the original under wine). The
/// window presets set it (1 in three full-screen ones, else 0); the story
/// sets 1 for full-screen adverts and 2 for lines two characters say
/// together. ONScripter can't type lines
/// in parallel, so 2 types them one after another.
const OP_TEXT_DISPLAY: u16 = 0x4d;
/// Starts a chapter: pool[arg] = (own label number, label-table index of
/// the chapter's "|chpe" label). The engine shows the name of the label it
/// sits on (`0.cd`): "chapter<route>]?<title>". 0x37 and 0x38 sit on the
/// chapter's "|chpe" and "|chpl" labels and link back (chapter history).
const OP_CHAPTER: u16 = 0x36;
/// Waits for a click with the text window, button tray and top bars hidden
/// (handler 0x4784b0, wait callback 0x478370). arg 0 also turns skip mode
/// off; with arg 1 skip mode passes through.
const OP_CLICK_WAIT: u16 = 0x68;
/// Removes every character bust-up (arg is always -1).
const OP_CLEAR_SPRITES: u16 = 0x8f;
/// A second background object, "bg-<slot>" (handlers 0x422440, 0x4222c0,
/// 0x412fe0), mostly one of the `frameNN` pictures: letterbox bars, speed
/// lines, a heart border. Record: u8 slot (always 2), u8 layer, u32 depth,
/// u32 background (1-based as in 0x54, image id 10,000,000 + n - 1; 0
/// removes it), u8 "mark as seen". Layers 0 and 1 are the background's
/// (under the bust-ups), 2 the bust-ups' (0x10); 3 and 4 aren't used.
/// Assumption: within a layer the higher depth is drawn on top, so the
/// usual depth 1000 puts it over the bust-ups (10-100 in 0x8e). No fade of
/// its own. It stays when the background changes or the bust-ups go.
const OP_OVERLAY: u16 = 0x55;
/// Effect presets (0x88 defines a module, 0x90 attaches a preset to a
/// bust-up) are kept in variables, so they follow the game's control flow:
/// preset p, slot s is [`PRESET_FIELDS`] local variables from
/// [`PRESET_VAR_BASE`] (the game's own locals are 0-2). Preset 1 is the
/// lighting tint (script 2 redefines it per mood); 2 and 3 are used in a
/// few single scenes.
const PRESET_VAR_BASE: u16 = 30;
const PRESETS: u32 = 3;
const PRESET_SLOTS: u32 = 3;
/// kind, blend, strength, three parameters (as in [`SpriteFilter`]).
const PRESET_FIELDS: u32 = 6;
const OP_WAIT: u16 = 0x66;
/// Plays `HD/mvNNNN.bin` (handler 0x478ab0, task 0x408ed0): arg's low 16
/// bits are the number, its high 16 bits whether a click or key ends it
/// (checked at 0x408e5b).
const OP_MOVIE: u16 = 0x97;
/// 0x72 and 0x74 share one timed wait (0x44a2f0): arg's low 16 bits are
/// the ms; 0x74 also sets the input mask the wait checks (0x51ea0c) to
/// 0xffffff instead of 0. Both are converted as `delay`.
const OP_WAIT_2: u16 = 0x72;
const OP_WAIT_3: u16 = 0x74;
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
/// A voice id's character is `id / VOICE_CHARACTER`.
const VOICE_CHARACTER: u32 = 100_000;
/// The original's voice switches (the settings dialog's 音声 page, in its
/// order) and the voice characters each covers. 0 and 1 are the minor
/// women and men (フランシス, 紅緒 / 邑西, 宗多, ...); 2 is unused.
const VOICE_TOGGLES: [(&str, &[u32]); 13] = [
    ("紅緒　あずさ", &[3]),
    ("本堂　沙也加", &[4]),
    ("塚本　葉子", &[5]),
    ("伊月　笛子", &[6]),
    ("樋口　章二", &[7]),
    ("塚本　斎", &[8]),
    ("イマ", &[9]),
    ("貴宮　千鳥", &[10]),
    ("南　レイ", &[11]),
    ("シャーリー・磯下", &[12]),
    ("ジーン・ブランシェット", &[13]),
    ("その他の女性陣", &[0]),
    ("その他の男達", &[1]),
];
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
    #[error("{path} slot {slot}: {source}")]
    Ogg { path: PathBuf, slot: usize, source: crate::ogg::OggError },
    #[error("{path}: {source}")]
    Xarc { path: PathBuf, source: XarcError },
    #[error("{what}: {source}")]
    Image { what: String, source: image::ImageError },
    #[error(transparent)]
    Cd(#[from] CdError),
    #[error("{kind} {number} does not exist")]
    MissingAsset { kind: &'static str, number: usize },
    #[error("converting {path} with ffmpeg: {detail}")]
    Ffmpeg { path: PathBuf, detail: String },
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
            if ins.op == OP_OVERLAY {
                let number = pool_u32(cd.pool, ins.arg as u32 as usize + 6).unwrap_or(0);
                if number > 0 {
                    self.backgrounds.insert(number as usize);
                }
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
        self.sound("bm0000.bin", slot, "bgm", format!("bgm{track:02}.ogg"), BGM, track)
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

    /// Monotone palette `number` (`pal%04u.pal` in `HD/pal.arc`, a RIFF
    /// PAL of 256 RGB entries for the grey levels) as offsets added to the
    /// grey level (CONscripter's monotone filter). The game's one palette
    /// is sepia, grey + (28, -8, -38) up to rounding; the offsets are the
    /// mean over the grey levels 64-191, where nothing is clamped.
    pub fn palette_offsets(&mut self, number: u32) -> Result<[i32; 3], ConvertError> {
        let missing = || ConvertError::MissingAsset { kind: "palette", number: number as usize };
        let path = self.game.join("HD").join("pal.arc");
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let arc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let file = format!("pal{number:04}.pal");
        let pal = arc.data(arc.entries.iter().find(|e| e.name == file).ok_or_else(missing)?);
        // "RIFF", size, "PAL data", size, version, count, then r, g, b, flags.
        let entries = pal.get(0x18..0x18 + 256 * 4).filter(|_| pal.starts_with(b"RIFF")).ok_or_else(missing)?;
        let mut sum = [0i32; 3];
        for grey in 64..192 {
            for (k, s) in sum.iter_mut().enumerate() {
                *s += i32::from(entries[grey * 4 + k]) - grey as i32;
            }
        }
        Ok(sum.map(|s| (f64::from(s) / 128.0).round() as i32))
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

    /// The bottom button bar from `HD/wnd.arc` → `SysTray.004`: rows of
    /// eight states side by side (normal, under the pointer, pressed,
    /// disabled, then the same four while the mode is on). Base and frame
    /// (0, 1) are empty. Positions measured on a screenshot of the
    /// original, where the bar runs along the bottom edge. Not converted:
    /// the lock button (9, keeps the bar from hiding; this bar never hides).
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
            let image = if action == BarAction::Unavailable { strip(name, &[3])? } else { strip(name, &[0, 1, 3])? };
            let active = match action {
                BarAction::Auto | BarAction::Skip => Some(strip(&format!("{name}_on"), &[4, 5, 7])?),
                _ => None,
            };
            buttons.push(BarButton { image, active, action });
        }
        // Hotspots: the pointer within 25 px of the bottom shows the bar;
        // between the top and bottom bars it hides them.
        let reveal = self.transparent("bar_reveal", 0, 575, 800, 25)?;
        let hide = self.transparent("bar_hide", 0, 52, 800, 523)?;
        Ok(ButtonBar { buttons, reveal, hide, top: Some(self.top_bars()?) })
    }

    /// The images of a GAF4 container in `HD/{arc}`, decoded.
    fn gaf_rgba(&self, arc: &str, file: &str, kind: &'static str) -> Result<Vec<image::RgbaImage>, ConvertError> {
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
    fn place(&self, name: &str, img: &image::RgbaImage, cells: Option<(u32, &[u32])>, x: i32, y: i32) -> Result<Placed, ConvertError> {
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
                row("オート速度", Setting::AutoSpeed),
            ],
            label_x: 140,
            minus_x: 340,
            level_x: 380,
            plus_x: 680,
            y: 140,
            pitch: 70,
            font_size: 22,
            title_label: "タイトルに戻る".into(),
            quit_label: "ゲームを終了".into(),
            exits_y: 540,
            tab_labels: ["音量・速度".into(), "音声".into()],
            tabs_x: 300,
            tabs_y: 34,
            voices: VOICE_TOGGLES
                .iter()
                .map(|(label, characters)| VoiceToggle { label: label.to_string(), characters: characters.to_vec() })
                .collect(),
            voice_column_x: 440,
            voice_pitch: 50,
            skip_unread_label: Some("未読テキストもスキップする".into()),
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
            text_y: 16,
            font_size: 20,
            overwrite: "のデータに上書きします。よろしいですか？".into(),
            load: "のデータをロードします。よろしいですか？".into(),
            delete: "のデータを削除します。よろしいですか？".into(),
            title: "タイトルへ戻りますか？".into(),
            quit: "ゲームを終了します。よろしいですか？".into(),
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
            base: self.place("menu_base", &darker(get(0)?), None, 0, 0)?,
            base_opacity: 210,
            frame: self.place("log_frame", &get(1)?, None, 0, 0)?,
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

    /// A fully transparent image, for a hotspot.
    fn transparent(&self, name: &str, x: i32, y: i32, w: u32, h: u32) -> Result<Placed, ConvertError> {
        let image = format!("system/{name}.png");
        self.save_image(&image, &image::RgbaImage::new(w, h))?;
        Ok(Placed { image, x, y, w: w as i32, h: h as i32 })
    }

    /// The top bars: `HD/wnd.arc` → `ChapterBar.004` (base, frame: "chapter
    /// »") on the left and `BlogStackBar.004` on the right (bar 0/1, drop-down
    /// 2/3, exit 4/5, next 6/7, prev 8/9, scene list 10-12, depth box 13/14,
    /// digits 15; buttons are four states side by side, base and frame
    /// layers separately). The scene list and the buttons, which browse
    /// the link scenes visited, aren't converted: the buttons are shown
    /// disabled. Positions measured on a screenshot of the original.
    fn top_bars(&mut self) -> Result<TopBars, ConvertError> {
        let blog = self.gaf_images("wnd.arc", "BlogStackBar.004", "blog bar")?;
        let chapter = self.gaf_images("wnd.arc", "ChapterBar.004", "chapter bar")?;
        let load = |images: &[Vec<u8>], n: usize| {
            let png = images.get(n).ok_or(ConvertError::MissingAsset { kind: "top bar image", number: n })?;
            image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .map(|i| i.to_rgba8())
                .map_err(|source| ConvertError::Image { what: format!("top bar #{n}"), source })
        };
        // Layers `base`/`base + 1`, cell `cell` of `cells` side by side.
        let layered = |this: &Self, images: &[Vec<u8>], base: usize, cell: u32, cells: u32, name: &str, x: i32, y: i32| {
            let (b, f) = (load(images, base)?, load(images, base + 1)?);
            let w = b.width() / cells;
            let crop = |i: &image::RgbaImage| image::imageops::crop_imm(i, cell * w, 0, w, i.height()).to_image();
            let img = blend_layers(crop(&b), &crop(&f));
            let image = format!("system/{name}.png");
            this.save_image(&image, &img)?;
            Ok::<_, ConvertError>(Placed { image, x, y, w: img.width() as i32, h: img.height() as i32 })
        };
        let digit_strip = load(&blog, 15)?;
        let dw = digit_strip.width() / 10;
        let mut digits = Vec::new();
        for d in 0..10 {
            let img = image::imageops::crop_imm(&digit_strip, d * dw, 0, dw, digit_strip.height()).to_image();
            let image = format!("system/digit{d}.png");
            self.save_image(&image, &img)?;
            digits.push(image);
        }
        let disabled = 3;
        Ok(TopBars {
            reveal: self.transparent("bar_reveal_top", 0, 0, 800, 25)?,
            chapter_bar: layered(self, &chapter, 0, 0, 1, "chapter_bar", 0, 0)?,
            chapter_text: TextSpot { x: 72, y: 7, font_size: 12, columns: 21 },
            scene_bar: layered(self, &blog, 0, 0, 1, "scene_bar", 417, 0)?,
            scene_text: TextSpot { x: 529, y: 7, font_size: 12, columns: 20 },
            root_scene: "root scene".into(),
            decorations: vec![
                layered(self, &blog, 13, 0, 1, "scene_depth", 612, 28)?,
                layered(self, &blog, 8, disabled, 4, "scene_prev", 710, 28)?,
                layered(self, &blog, 6, disabled, 4, "scene_next", 736, 28)?,
                layered(self, &blog, 4, disabled, 4, "scene_exit", 762, 28)?,
                layered(self, &blog, 2, disabled, 4, "scene_list", 780, 3)?,
            ],
            digits,
            depth_at: (662, 32),
            max_depth_at: (682, 32),
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
        self.save_image(name, &blend_layers(load(base)?, &load(top)?))
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
        // BGM: param 0 is where the loop starts, in sample frames (the audio
        // before it matches the track's end); the loop ends at the end.
        let loop_start = entry.params[0];
        if kind == BGM && loop_start > 0 {
            let tagged = crate::ogg::set_loop_start(xsa.data(entry), loop_start.into())
                .map_err(|source| ConvertError::Ogg { path: path.clone(), slot, source })?;
            write_file(&self.out.join(&name), &tagged)?;
        } else {
            write_file(&self.out.join(&name), xsa.data(entry))?;
        }
        self.written.insert((kind, number), name.clone());
        Ok(name)
    }
}

const BGM: &str = "BGM track";
const MOVIE: &str = "movie";

impl AssetBank {
    /// Movie `number` (`HD/mvNNNN.bin`, an MPEG-1 program stream) as H.264
    /// and AAC in MP4, which browsers play. Movies stay loose files (the
    /// web build hands the file to a `<video>` element). A movie already
    /// in the output is kept: encoding takes a minute or so each.
    pub fn movie(&mut self, number: u16) -> Result<String, ConvertError> {
        let key = (MOVIE, usize::from(number));
        if let Some(p) = self.written.get(&key) {
            return Ok(p.clone());
        }
        let src = self.game.join("HD").join(format!("mv{number:04}.bin"));
        if !src.exists() {
            return Err(ConvertError::MissingAsset { kind: MOVIE, number: key.1 });
        }
        let name = format!("movie/mv{number:04}.mp4");
        let dest = self.out.join(&name);
        if !dest.exists() {
            let io = |source| ConvertError::Io { path: dest.clone(), source };
            fs::create_dir_all(dest.parent().unwrap()).map_err(io)?;
            // Encode under another name so an interrupted run leaves no
            // half-written movie behind.
            let part = dest.with_extension("part.mp4");
            let ffmpeg = |detail: String| ConvertError::Ffmpeg { path: src.clone(), detail };
            let status = std::process::Command::new("ffmpeg")
                .args(["-v", "error", "-y", "-i"])
                .arg(&src)
                .args(["-c:v", "libx264", "-preset", "slow", "-crf", "22", "-pix_fmt", "yuv420p"])
                .args(["-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart"])
                .arg(&part)
                .status()
                .map_err(|e| ffmpeg(e.to_string()))?;
            if !status.success() {
                let _ = fs::remove_file(&part);
                return Err(ffmpeg(status.to_string()));
            }
            fs::rename(&part, &dest).map_err(io)?;
        }
        self.written.insert(key, name.clone());
        Ok(name)
    }
}

/// Label names from `0.cd`, by (script id, code offset in bytes).
#[derive(Debug, Default)]
pub struct LabelNames(HashMap<(u32, u32), String>);

impl LabelNames {
    pub fn new(table: &SymbolTable) -> Self {
        let mut names = HashMap::new();
        for (script, symbols) in table.scripts.iter().enumerate() {
            for s in symbols {
                names.insert((script as u32, s.code_offset), s.name.clone());
            }
        }
        Self(names)
    }

    /// Name of `cd`'s label-table entry `index` (index 0 is implicit).
    fn table_entry(&self, cd: &CdScript, index: i64) -> Option<&str> {
        let l = cd.labels.get(usize::try_from(index).ok()?.checked_sub(1)?)?;
        self.0.get(&(l.script, l.code_offset)).map(String::as_str)
    }

    /// Name of label number `label` of `cd` itself.
    fn own(&self, cd: &CdScript, label: u32) -> Option<&str> {
        let l = cd.labels.iter().find(|l| l.script == cd.script_id && l.label == label)?;
        self.0.get(&(l.script, l.code_offset)).map(String::as_str)
    }
}

/// "chapter<route>]?<title>" → title.
fn chapter_title(name: &str) -> Option<&str> {
    name.strip_prefix("chapter")?.split_once("]?").map(|(_, title)| title)
}

/// "blog?<title>" → title: the scenes links open.
fn scene_title(name: &str) -> Option<&str> {
    name.strip_prefix("blog?")
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
    /// Shows messages whole ([`OP_TEXT_DISPLAY`] 1).
    pub instant: bool,
}

/// Subroutines that set up shared state, keyed by IR label name.
#[derive(Debug, Default)]
pub struct Presets {
    /// Text window presets: labels whose code sets a window position.
    pub window: HashMap<String, WindowPreset>,
}

/// Finds the preset subroutines (see [`Presets`]).
pub fn presets(scripts: &[(String, CdScript)]) -> Result<Presets, ConvertError> {
    let mut presets = Presets::default();
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
            let Some(l) = label else { continue };
            let name = format!("s{}_l{l}", cd.script_id);
            match ins.op {
                OP_WINDOW_MODE => mode = ins.arg,
                OP_WINDOW_STYLE => style = ins.arg,
                OP_WINDOW_POS => {
                    let preset = WindowPreset {
                        full_screen: mode == 1,
                        style: style as u32,
                        x: ins.arg & 0xffff,
                        y: ins.arg >> 16,
                        instant: false,
                    };
                    presets.window.insert(name, preset);
                }
                OP_TEXT_DISPLAY => {
                    if let Some(p) = presets.window.get_mut(&name) {
                        p.instant = ins.arg == 1;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(presets)
}

/// The variable holding `field` of a preset slot (1-based preset and slot).
fn preset_var(preset: u32, slot: u32, field: u32) -> Var {
    let n = ((preset - 1) * PRESET_SLOTS + slot - 1) * PRESET_FIELDS + field;
    Var { global: false, index: PRESET_VAR_BASE + n as u16 }
}

/// A 0x88/0x70 module as CONscripter filter fields (kind, blend,
/// strength, three parameters), or what can't be expressed.
fn module_filter(
    kind: u32,
    blend: u16,
    opacity: u16,
    colour: Option<[u8; 3]>,
    brightness: Option<i16>,
    monotone: Option<[i32; 3]>,
) -> Result<[i32; 6], String> {
    let (kind, params) = match (kind, colour, brightness, monotone) {
        (PRESET_KIND_COLOUR, Some([r, g, b]), _, _) => (FILTER_COLOUR, [r.into(), g.into(), b.into()]),
        (PRESET_KIND_GREYSCALE, ..) => (FILTER_GREYSCALE, [0; 3]),
        (PRESET_KIND_NEGATIVE, ..) => (FILTER_NEGATIVE, [0; 3]),
        // The amount is on a 0-255 scale: the game goes down to -250.
        (PRESET_KIND_BRIGHTNESS, _, Some(n), _) => (FILTER_BRIGHTNESS, [n.into(), 0, 0]),
        (PRESET_KIND_MONOTONE, _, _, Some(offsets)) => (FILTER_MONOTONE, offsets),
        _ => return Err(format!("filter kind {kind}")),
    };
    let blend = match blend {
        PRESET_BLEND_ALPHA | PRESET_BLEND_COPY => FILTER_BLEND_COPY,
        PRESET_BLEND_MULTI => FILTER_BLEND_MULTIPLY,
        PRESET_BLEND_ADD => FILTER_BLEND_ADD,
        PRESET_BLEND_SCREEN => FILTER_BLEND_SCREEN,
        _ => return Err(format!("blend {blend}")),
    };
    Ok([kind, blend, i32::from(opacity.min(256)), params[0], params[1], params[2]])
}

/// Slots of CONscripter's `screenfilter` (the game uses 1-3).
const SCREEN_FILTER_SLOTS: u32 = 4;

/// Opcode 0x70: sets one slot of the screen filter. A module the
/// converter can't express is reported and leaves the slot as it was.
fn lower_screen_module(
    m: &ScreenModule,
    assets: &mut AssetBank,
    ops: &mut Vec<Op>,
    at: impl Fn() -> SourceLoc,
) -> Result<(), ConvertError> {
    if m.slot > SCREEN_FILTER_SLOTS {
        ops.push(Op::Unsupported { what: format!("opcode 0x70 slot {}", m.slot), at: at() });
        return Ok(());
    }
    if m.slot == 0 || m.kind == 0 {
        ops.push(Op::ScreenFilter { slot: m.slot, filter: None, fade_ms: m.fade_ms });
        return Ok(());
    }
    let monotone = match m.palette.map(|p| assets.palette_offsets(p)) {
        Some(Ok(offsets)) => Some(offsets),
        Some(Err(ConvertError::MissingAsset { .. })) | None => None,
        Some(Err(e)) => return Err(e),
    };
    match module_filter(m.kind, m.blend, m.opacity, m.colour, m.brightness, monotone) {
        Ok([kind, blend, strength, p0, p1, p2]) => {
            let filter = SpriteFilter {
                kind: Operand::Const(kind),
                blend: Operand::Const(blend),
                strength: Operand::Const(strength),
                params: [p0, p1, p2].map(Operand::Const),
            };
            ops.push(Op::ScreenFilter { slot: m.slot, filter: Some(filter), fade_ms: m.fade_ms });
        }
        Err(what) => ops.push(Op::Unsupported { what: format!("opcode 0x70 {what}"), at: at() }),
    }
    Ok(())
}

/// Opcode 0x88: sets one slot of a preset (slot 0 clears every slot). A
/// module the converter can't express clears its slot and is reported.
fn lower_preset_module(m: &PresetModule, ops: &mut Vec<Op>, at: impl Fn() -> SourceLoc) {
    let unsupported = |what: String| Op::Unsupported { what, at: at() };
    if !(1..=PRESETS).contains(&m.preset) || m.slot > PRESET_SLOTS {
        ops.push(unsupported(format!("opcode 0x88 preset {} slot {}", m.preset, m.slot)));
        return;
    }
    let set = |slot: u32, field: u32, n: i32| Op::Arith {
        var: preset_var(m.preset, slot, field),
        op: ArithOp::Set,
        value: Operand::Const(n),
    };
    if m.slot == 0 {
        ops.extend((1..=PRESET_SLOTS).map(|slot| set(slot, 0, 0)));
        return;
    }
    if m.kind == 0 {
        ops.push(set(m.slot, 0, 0));
        return;
    }
    match module_filter(m.kind, m.blend, m.opacity, m.colour, m.brightness, None) {
        Ok(fields) => ops.extend(fields.into_iter().enumerate().map(|(field, n)| set(m.slot, field as u32, n))),
        Err(what) => {
            ops.push(unsupported(format!("opcode 0x88 {what}")));
            ops.push(set(m.slot, 0, 0));
        }
    }
}

/// Opcode 0x90: the filters of a preset (0 = none), read from its
/// variables when the script runs.
fn preset_filters(preset: u32) -> Option<Vec<SpriteFilter>> {
    match preset {
        0 => Some(Vec::new()),
        1..=PRESETS => Some(
            (1..=PRESET_SLOTS)
                .map(|slot| {
                    let v = |field| Operand::Var(preset_var(preset, slot, field));
                    SpriteFilter { kind: v(0), blend: v(1), strength: v(2), params: [v(3), v(4), v(5)] }
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Where a bust-up entry's image goes on screen.
fn placement(assets: &mut AssetBank, e: &SpriteEntry) -> Result<SpritePlacement, ConvertError> {
    let img = assets.sprite(e.sprite as usize)?;
    // Assumption: (x, y) is the sprite's centre on the 800×600 screen.
    Ok(SpritePlacement {
        image: img.path,
        x: i32::from(e.x) - img.width as i32 / 2,
        y: i32::from(e.y) - img.height as i32 / 2,
    })
}

/// Lowers one script. Instructions that can't be expressed yet become
/// [`Op::Unsupported`]; assets are written through `assets`.
pub fn lower_script(
    cd: &CdScript,
    file: &str,
    flow: Flow,
    assets: &mut AssetBank,
    presets: &Presets,
    names: &LabelNames,
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
    // Links, ruby, styles and indent records apply to the next message.
    let mut pending = MessageRecords::default();
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
            pending.links.push(link?);
            continue;
        }
        if let Some(ruby) = cd.ruby(ins) {
            pending.ruby.push(ruby?);
            continue;
        }
        if let Some(style) = cd.text_style(ins) {
            pending.styles.push(style?);
            continue;
        }
        if ins.op == OP_TEXT_INDENT {
            pending.indent = Some((ins.arg as u32 >> 16) as u16);
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
                    ops.push(Op::BeginTransition { effect, fade_ms, mask, style: TransitionStyle::Fade });
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (bad record)", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_OVERLAY {
            let o = ins.arg as u32 as usize;
            let record = cd.pool.get(o..o + 2).zip(pool_u32(cd.pool, o + 6));
            match record {
                Some((&[2, layer @ 0..=2], number)) => {
                    let image = match number {
                        0 => None,
                        n => Some(assets.background(n as usize)?.path),
                    };
                    ops.push(Op::Overlay { image, over_sprites: layer == 2 });
                }
                _ => ops.push(Op::Unsupported { what: format!("opcode {:#04x} record {record:?}", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_CHAPTER {
            let title = pool_u32(cd.pool, ins.arg as u32 as usize)
                .and_then(|label| names.own(cd, label))
                .and_then(chapter_title);
            match title {
                Some(title) => ops.push(Op::Chapter { title: title.to_string() }),
                None => ops.push(Op::Unsupported { what: format!("opcode {:#04x} (no chapter name)", ins.op), at: at() }),
            }
            continue;
        }
        if ins.op == OP_TRANSITION {
            match pool_u32(cd.pool, ins.arg as u32 as usize) {
                Some(fade_ms) => {
                    transition_armed = true;
                    ops.push(Op::BeginTransition {
                        effect: transition_name(ins.sub).to_string(),
                        fade_ms,
                        mask: None,
                        style: transition_style(ins.sub),
                    });
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
        if let Some(module) = cd.preset_module(ins) {
            lower_preset_module(&module?, &mut ops, at);
            continue;
        }
        if let Some(module) = cd.screen_module(ins) {
            lower_screen_module(&module?, assets, &mut ops, at)?;
            continue;
        }
        if let Some(apply) = cd.apply_preset(ins) {
            let apply = apply?;
            // Without an object for the character the handler does nothing;
            // neither does spfilter on a layer without a sprite.
            match preset_filters(apply.preset) {
                Some(filters) => ops.push(Op::SpriteFilters { layer: u32::from(apply.character), filters }),
                None => ops.push(Op::Unsupported { what: format!("opcode {:#04x} preset {}", ins.op, apply.preset), at: at() }),
            }
            continue;
        }
        if ins.op == OP_TRANSITION_WAIT {
            if std::mem::take(&mut transition_armed) {
                ops.push(Op::EndTransition);
            }
            continue;
        }
        let records = if cd.message(ins).is_some() { std::mem::take(&mut pending) } else { MessageRecords::default() };
        let unsupported = || format!("opcode {:#04x} sub {:#x} arg {}", ins.op, ins.sub, ins.arg);
        match lower_instr(cd, ins, &records, assets, names) {
            Ok(Some(Op::Text { speaker, text, voice, mut links, ruby, styles, hang, continues })) if flow == Flow::Linear => {
                // No jumps in linear mode, so links are colour only.
                links.iter_mut().for_each(|l| l.target = None);
                ops.push(Op::Text { speaker, text, voice, links, ruby, styles, hang, continues });
            }
            Ok(Some(Op::Text { speaker, text, voice, links, ruby, styles, hang, .. })) => {
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
                ops.push(Op::Text { speaker, text, voice, links, ruby, styles, hang, continues: false });
            }
            Ok(Some(Op::Gosub(target))) => {
                let preset = presets.window.get(&target).copied();
                ops.push(Op::Gosub(target));
                if let Some(p) = preset {
                    let window = assets.text_window_style(p)?;
                    full_screen_rows = p.full_screen.then_some(window.rows as usize);
                    last_text = None;
                    ops.push(Op::SetWindow(window));
                    // setwindow resets the text speed the preset set.
                    if p.instant {
                        ops.push(Op::InstantText(true));
                    }
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

/// Character range of `ruby` in the text lines `body` joined with '\n'.
fn ruby_range(body: &[String], ruby: &RubyRecord) -> Option<(usize, usize)> {
    let line = usize::from(ruby.line).checked_sub(1)?;
    let col = usize::from(ruby.column).checked_sub(1)?;
    let len = usize::from(ruby.length);
    if len == 0 || col + len > body.get(line)?.chars().count() {
        return None;
    }
    let before: usize = body[..line].iter().map(|s| s.chars().count() + 1).sum();
    Some((before + col, len))
}

/// Records that apply to the next message: links (0x46), ruby (0x44),
/// text styles (0x49) and the hanging indent (0x4e, half-width units).
#[derive(Default)]
struct MessageRecords {
    links: Vec<Link>,
    ruby: Vec<RubyRecord>,
    styles: Vec<StyleRecord>,
    indent: Option<u16>,
}

/// The character of `line` that `units` half-width units (0x4e) reach:
/// ASCII, half-width katakana and Greek letters are one unit in the game's
/// layout, other characters two.
fn hang_char(line: &str, units: u16) -> Option<usize> {
    let mut width = 0;
    for (i, c) in line.chars().enumerate() {
        if width >= usize::from(units) {
            return (i > 0).then_some(i);
        }
        let half = c.is_ascii() || ('\u{FF61}'..='\u{FF9F}').contains(&c) || ('\u{0391}'..='\u{03C9}').contains(&c);
        width += if half { 1 } else { 2 };
    }
    None
}

/// Size [`StyleRecord::size`] that keeps the window's size.
const STYLE_SIZE_KEEP: u16 = 0xf000;
/// Colour [`StyleRecord::colour`] that keeps the text colour.
const STYLE_COLOUR_KEEP: u32 = 0xff00_0000;

/// A 0x49 record as a range of the text lines `body` joined with '\n'.
/// Length 30000 (or any past the end) runs to the end of the message: the
/// game wraps the lines of enlarged messages short enough to fit.
fn text_style(body: &[String], s: &StyleRecord) -> Option<TextStyle> {
    let line = usize::from(s.line).checked_sub(1)?;
    let col = usize::from(s.column).checked_sub(1)?;
    if col >= body.get(line)?.chars().count() {
        return None;
    }
    let before: usize = body[..line].iter().map(|s| s.chars().count() + 1).sum();
    let total = body.iter().map(|s| s.chars().count() + 1).sum::<usize>() - 1;
    let len = usize::from(s.length).min(total - before - col);
    let colour = (s.colour != STYLE_COLOUR_KEEP).then(|| {
        // COLORREF 0x00BBGGRR to 0xRRGGBB
        let [r, g, b, _] = s.colour.to_le_bytes();
        u32::from_be_bytes([0, r, g, b])
    });
    // Sizes are percent; 0x1000 + n is a preset (table at 0x4fa850,
    // looked up at 0x491d00).
    const PRESET_PERCENT: [u16; 8] = [0, 25, 50, 75, 100, 125, 150, 175];
    let size = match s.size {
        STYLE_SIZE_KEEP => None,
        n if n & 0xf000 == 0x1000 => Some(PRESET_PERCENT[usize::from(n & 0xfff).clamp(1, 7)]),
        n if n < 0x1000 => Some(n),
        _ => None,
    };
    if len == 0 || (colour.is_none() && size.is_none()) {
        return None;
    }
    Some(TextStyle { start: before + col, len, colour, size_percent: size })
}

fn lower_instr(
    cd: &CdScript,
    ins: &Instr,
    records: &MessageRecords,
    assets: &mut AssetBank,
    names: &LabelNames,
) -> Result<Option<Op>, ConvertError> {
    if let Some(msg) = cd.message(ins) {
        let msg = msg?;
        let segs = &msg.segments;
        // A short first segment followed by a quoted one is the speaker's name.
        let has_speaker = segs.len() >= 2
            && (1..=8).contains(&segs[0].chars().count())
            && segs[1].starts_with(['「', '『', '（']);
        let (speaker, body) = if has_speaker { (Some(segs[0].clone()), &segs[1..]) } else { (None, &segs[..]) };
        let links = records
            .links
            .iter()
            .filter_map(|l| {
                let (start, len) = link_range(segs, has_speaker, l)?;
                let scene = names.table_entry(cd, l.target.into()).and_then(scene_title).map(String::from);
                Some(TextLink { start, len, target: table_label(cd, l.target.into()), scene })
            })
            .collect();
        // Segments are the game's own display lines.
        // A missing clip only drops the voice, not the text.
        let voice = match msg.voice_id.map(|id| assets.voice(id).map(|file| Voice { file, character: id / VOICE_CHARACTER })) {
            Some(Err(ConvertError::MissingAsset { .. })) | None => None,
            Some(r) => Some(r?),
        };
        let ruby = records
            .ruby
            .iter()
            .filter_map(|r| {
                let (start, len) = ruby_range(body, r)?;
                Some(Ruby { start, len, text: r.text.clone() })
            })
            .collect();
        let styles = records.styles.iter().filter_map(|s| text_style(body, s)).collect();
        let hang = records.indent.and_then(|units| hang_char(body.first()?, units));
        return Ok(Some(Op::Text { speaker, text: body.join("\n"), voice, links, ruby, styles, hang, continues: false }));
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
                changes.push(SpriteChange { layer, sprite: None, keep_filters: false });
                continue;
            }
            // Mode bit 2 keeps the character's object, and with it the
            // attached filter; otherwise the handler (0x434130) deletes it
            // and creates a new one.
            let sprite = placement(assets, e)?;
            changes.push(SpriteChange { layer, sprite: Some(sprite), keep_filters: e.mode & 4 != 0 });
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
        OP_TEXT_DISPLAY => return Ok(Some(Op::InstantText(ins.arg == 1))),
        OP_CLICK_WAIT => return Ok(Some(Op::ClickWait { skippable: ins.arg != 0 })),
        OP_MOVIE => {
            let file = assets.movie(ins.arg as u16)?;
            return Ok(Some(Op::Movie { file, skippable: ins.arg >> 16 != 0 }));
        }
        OP_WAIT | OP_WAIT_2 | OP_WAIT_3 => return Ok(Some(Op::Wait { ms: u32::from(ins.arg as u16) })),
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
/// `Trans_*.cpp` source files (the handler at `0x449df0` picks the class
/// with a jump table on `sub`).
fn transition_name(sub: u16) -> &'static str {
    match sub {
        0..=3 => "Wave",
        4 => "Dot4x4",
        5..=8 => "SoftEdgeWipe",
        9 => "CrossFade",
        10..=17 => "Slide in",
        18..=25 => "Slide out",
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

/// How 0x76's `sub` moves, for the families CONscripter can draw (the
/// rest are cross-fades). From the exe:
/// - Slide (`0x446f10`): `sub - 10` (in) or `sub - 18` (out) indexes the
///   direction tables at `0x5006d0`/`0x5006d8` (x, y). Sliding in, the new
///   picture starts one screen length that way and moves back into place;
///   sliding out, the old picture moves that way. Linear in time.
/// - Scroll (`0x446be0`): both pictures move together, `sub - 41` picks
///   one of four draw routines (table `0x518040`): the new picture enters
///   from below, the right, above, the left.
/// - Mosaic (`0x446160`): blocks up to 100 px (cubic), old then new.
/// - Zoom (`0x449800`): mode `sub - 37`, magnification `1 + f * t^5` with
///   f = 50 (f = 1.2 for 40), 16.16 fixed point about the centre; the
///   draw routines at `0x449160`, `0x449220`, `0x449270`, `0x449330`.
fn transition_style(sub: u16) -> TransitionStyle {
    const DIRECTIONS: [(i8, i8); 8] = [(0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1), (1, 0), (1, 1)];
    let zoom = |style, max_percent| TransitionStyle::Zoom { style, max_percent, curve: 5 };
    match sub {
        10..=17 => {
            let (dx, dy) = DIRECTIONS[usize::from(sub - 10)];
            TransitionStyle::Slide { moving: SlideLayer::New, dx: -dx, dy: -dy }
        }
        18..=25 => {
            let (dx, dy) = DIRECTIONS[usize::from(sub - 18)];
            TransitionStyle::Slide { moving: SlideLayer::Old, dx, dy }
        }
        41..=44 => {
            let (dx, dy) = [(0, -1), (-1, 0), (0, 1), (1, 0)][usize::from(sub - 41)];
            TransitionStyle::Slide { moving: SlideLayer::Both, dx, dy }
        }
        27 => TransitionStyle::Mosaic,
        37 => zoom(ZoomStyle::NewIn, 5100),
        38 => zoom(ZoomStyle::OldOut, 5100),
        39 => zoom(ZoomStyle::Through, 5100),
        40 => zoom(ZoomStyle::Cross, 220),
        _ => TransitionStyle::Fade,
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

/// The image at half brightness. The log base is a white-to-black gradient;
/// white text over its top half was hard to read.
fn darker(mut img: image::RgbaImage) -> image::RgbaImage {
    for px in img.pixels_mut() {
        for c in &mut px.0[..3] {
            *c /= 2;
        }
    }
    img
}

/// `top` over `base`, the base made translucent like the text window's.
fn blend_layers(mut base: image::RgbaImage, top: &image::RgbaImage) -> image::RgbaImage {
    for px in base.pixels_mut() {
        px.0[3] = (u32::from(px.0[3]) * WINDOW_OPACITY_PERCENT / 100) as u8;
    }
    image::imageops::overlay(&mut base, top, 0, 0);
    base
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
    fn transition_styles_follow_the_exe() {
        let slide = |moving, dx, dy| TransitionStyle::Slide { moving, dx, dy };
        // Sub 10 (direction 0, +y): the new picture starts below and moves up.
        assert_eq!(transition_style(10), slide(SlideLayer::New, 0, -1));
        assert_eq!(transition_style(12), slide(SlideLayer::New, 1, 0));
        assert_eq!(transition_style(24), slide(SlideLayer::Old, 1, 0));
        assert_eq!(transition_style(41), slide(SlideLayer::Both, 0, -1));
        assert_eq!(transition_style(44), slide(SlideLayer::Both, 1, 0));
        assert_eq!(transition_style(40), TransitionStyle::Zoom { style: ZoomStyle::Cross, max_percent: 220, curve: 5 });
        assert_eq!(transition_style(9), TransitionStyle::Fade);
        assert_eq!(transition_name(20), "Slide out");
    }

    #[test]
    fn unique_names() {
        let mut used = HashSet::new();
        assert_eq!(unique(&mut used, "bg", "a b.png", 3), "bg/a_b.png");
        assert_eq!(unique(&mut used, "bg", "a b.png", 9), "bg/9_a_b.png");
        assert_eq!(unique(&mut used, "sprite", "背景.png", 1), "sprite/__.png");
    }

    #[test]
    fn presets_live_in_variables() {
        let at = || SourceLoc { file: "t".into(), index: 0 };
        let set = |var: Var, n: i32| Op::Arith { var, op: ArithOp::Set, value: Operand::Const(n) };
        let tint = PresetModule {
            preset: 2,
            slot: 1,
            kind: PRESET_KIND_COLOUR,
            blend: PRESET_BLEND_MULTI,
            opacity: 200,
            colour: Some([0x64, 0, 0]),
            brightness: None,
        };
        let mut ops = Vec::new();
        lower_preset_module(&tint, &mut ops, at);
        let v = |field| preset_var(2, 1, field);
        let expected =
            [(0, FILTER_COLOUR), (1, FILTER_BLEND_MULTIPLY), (2, 200), (3, 0x64), (4, 0), (5, 0)].map(|(f, n)| set(v(f), n));
        assert_eq!(ops, expected);
        // Preset 2 slot 1 comes after preset 1's three slots.
        assert_eq!(v(0).index, PRESET_VAR_BASE + 18);

        ops.clear();
        lower_preset_module(&PresetModule { slot: 0, ..tint }, &mut ops, at);
        assert_eq!(ops, (1..=3).map(|s| set(preset_var(2, s, 0), 0)).collect::<Vec<_>>());

        ops.clear();
        lower_preset_module(&PresetModule { kind: 9, colour: None, ..tint }, &mut ops, at);
        assert!(matches!(ops[..], [Op::Unsupported { .. }, Op::Arith { .. }]));

        assert_eq!(preset_filters(0), Some(vec![]));
        assert_eq!(preset_filters(3).unwrap().len(), 3);
        assert_eq!(preset_filters(4), None);
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
    fn ruby_ranges() {
        let body: Vec<String> = ["　吹く。", "　戦ぐ。"].iter().map(|s| s.to_string()).collect();
        let ruby = |line, column, length| RubyRecord { line, column, length, flags: 3, text: "そよ".into() };
        // Lines count text lines only; '\n' counts as one character.
        assert_eq!(ruby_range(&body, &ruby(2, 2, 1)), Some((6, 1)));
        assert_eq!(ruby_range(&body, &ruby(1, 2, 3)), Some((1, 3)));
        assert_eq!(ruby_range(&body, &ruby(1, 3, 3)), None);
        assert_eq!(ruby_range(&body, &ruby(3, 1, 1)), None);
        assert_eq!(ruby_range(&body, &ruby(0, 1, 1)), None);
    }

    #[test]
    fn text_style_ranges() {
        let body: Vec<String> = ["「牛めし", "　……カルビ"].iter().map(|s| s.to_string()).collect();
        let style = |line, column, length, size, colour| StyleRecord { line, column, length, size, colour, edge: STYLE_COLOUR_KEEP };
        // 30000 runs to the end of the message; sizes below 0x1000 are percent.
        assert_eq!(
            text_style(&body, &style(2, 1, 30000, 94, STYLE_COLOUR_KEEP)),
            Some(TextStyle { start: 5, len: 6, colour: None, size_percent: Some(94) })
        );
        assert_eq!(text_style(&body, &style(1, 2, 30000, 94, STYLE_COLOUR_KEEP)).map(|s| (s.start, s.len)), Some((1, 10)));
        // COLORREF 0x00BBGGRR; 0xf000 keeps the size.
        assert_eq!(
            text_style(&body, &style(1, 2, 2, STYLE_SIZE_KEEP, 0x00d3_55ba)),
            Some(TextStyle { start: 1, len: 2, colour: Some(0xba55d3), size_percent: None })
        );
        assert_eq!(
            text_style(&body, &style(1, 1, 1, 0x1006, STYLE_COLOUR_KEEP)).and_then(|s| s.size_percent),
            Some(150)
        );
        assert_eq!(text_style(&body, &style(1, 1, 30000, STYLE_SIZE_KEEP, STYLE_COLOUR_KEEP)), None);
        assert_eq!(text_style(&body, &style(3, 1, 1, 90, STYLE_COLOUR_KEEP)), None);
        assert_eq!(text_style(&body, &style(1, 6, 1, 90, STYLE_COLOUR_KEEP)), None);
    }

    #[test]
    fn module_filters() {
        assert_eq!(module_filter(PRESET_KIND_BRIGHTNESS, 5, 256, None, Some(-200), None), Ok([4, 0, 256, -200, 0, 0]));
        assert_eq!(
            module_filter(PRESET_KIND_MONOTONE, 5, 300, None, None, Some([28, -8, -38])),
            Ok([5, 0, 256, 28, -8, -38])
        );
        assert_eq!(module_filter(PRESET_KIND_COLOUR, 2, 128, Some([1, 2, 3]), None, None), Ok([1, 1, 128, 1, 2, 3]));
        assert!(module_filter(1, 5, 256, None, None, None).is_err()); // Smooth (blur)
        assert!(module_filter(PRESET_KIND_GREYSCALE, 4, 256, None, None, None).is_err());
    }

    #[test]
    fn hanging_indent() {
        assert_eq!(hang_char("Ghost　忍、いるか！？", 7), Some(6));
        assert_eq!(hang_char("θ　すべて", 3), Some(2));
        assert_eq!(hang_char("Ghost　", 7), None);
        assert_eq!(hang_char("Ghost", 0), None);
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
