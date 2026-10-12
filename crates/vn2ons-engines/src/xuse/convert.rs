//! Lowers a Xuse `.cd` script to the IR and writes the assets it uses.
//!
//! Assumptions that still need checking in a running conversion are marked
//! "assumption".

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use vn2ons_core::ir::{NamePlate, Op, SourceLoc, SpritePlacement, TextWindow};

use crate::xuse::cd::{CdError, CdScript, Instr, Link, Sound};
use crate::xuse::gaf::{Gaf, GafError};
use crate::xuse::xarc::{Xarc, XarcError};
use crate::xuse::xsa::{Xsa, XsaError};

/// Unconditional jumps (`0x10` and `0x11` share one handler in the engine).
const OP_GOTO: u16 = 0x10;
const OP_GOTO_2: u16 = 0x11;
/// Variable test; a 0x10 right after it is a conditional jump.
const OP_TEST: u16 = 0x15;
const BG_PER_ARCHIVE: usize = 64;
const BU_PER_ARCHIVE: usize = 128;
const SE_FIRST_BANK: usize = 256;
/// Assumption: backgrounds fade over this long (the `0x54` sub field may
/// select the transition; not decoded yet).
const BG_FADE_MS: u32 = 500;
/// Opacity of the text window's base layer (estimated from a screenshot of
/// the original game).
const WINDOW_OPACITY_PERCENT: u32 = 55;
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
                    if !(e.sprite == 0 && e.x == 0 && e.y == 0) {
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
        }
    }

    /// Background `number` as used by opcode 0x54 (1-based).
    pub fn background(&mut self, number: usize) -> Result<Image, ConvertError> {
        let index = number.checked_sub(1).ok_or(ConvertError::MissingAsset { kind: "background", number })?;
        self.image("bg", "background", "bg", index, BG_PER_ARCHIVE, number)
    }

    /// Sprite `number` as used by opcode 0x8e (0-based).
    pub fn sprite(&mut self, number: usize) -> Result<Image, ConvertError> {
        self.image("bu", "sprite", "sprite", number, BU_PER_ARCHIVE, number)
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

    /// Builds the ADV text box from the game's own window graphics
    /// (`HD/wnd.arc` → `TextWnd0.004`: message base + frame, name base +
    /// frame). Positions match the original game's 800×600 layout.
    pub fn text_window(&mut self) -> Result<TextWindow, ConvertError> {
        let path = self.game.join("HD").join("wnd.arc");
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let arc = Xarc::parse(&bytes).map_err(|source| ConvertError::Xarc { path: path.clone(), source })?;
        let entry = arc
            .entries
            .iter()
            .find(|e| e.name == "TextWnd0.004")
            .ok_or(ConvertError::MissingAsset { kind: "text window", number: 0 })?;
        let gaf_path = path.join("TextWnd0.004");
        let gaf = Gaf::parse(arc.data(entry)).map_err(|source| ConvertError::Gaf { path: gaf_path.clone(), source })?;
        let images = gaf.images().map_err(|source| ConvertError::Gaf { path: gaf_path, source })?;
        if images.len() < 4 {
            return Err(ConvertError::MissingAsset { kind: "text window layer", number: images.len() });
        }
        let window = "system/textwnd.png".to_string();
        let plate = "system/nameplate.png".to_string();
        self.layered(&window, images[0].png, images[1].png)?;
        self.layered(&plate, images[2].png, images[3].png)?;
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
        let dest = self.out.join(name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|source| ConvertError::Io { path: parent.to_path_buf(), source })?;
        }
        out.save(&dest).map_err(|source| ConvertError::Image { what: name.to_string(), source })
    }

    /// Reads every archive the `needs` touch once and writes the used
    /// assets. Numbers that don't exist are left for the lowering to report.
    pub fn prefetch(&mut self, needs: &Needs) -> Result<(), ConvertError> {
        let bg: Vec<usize> = needs.backgrounds.iter().filter_map(|n| n.checked_sub(1)).collect();
        self.prefetch_images("bg", "background", "bg", &bg, BG_PER_ARCHIVE, 1)?;
        let bu: Vec<usize> = needs.sprites.iter().copied().collect();
        self.prefetch_images("bu", "sprite", "sprite", &bu, BU_PER_ARCHIVE, 0)?;
        for &t in &needs.bgm {
            let _ = self.bgm(t);
        }
        for &n in &needs.se {
            let _ = self.se(n);
        }
        Ok(())
    }

    /// `indices` are 0-based; `number_offset` converts back to the script's
    /// numbering (1 for backgrounds).
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
        if let Some(p) = self.written.get(&(kind, number)) {
            return Ok(p.clone());
        }
        let path = self.game.join("HD").join(bank);
        let bytes = fs::read(&path).map_err(|source| ConvertError::Io { path: path.clone(), source })?;
        let xsa = Xsa::parse(&bytes).map_err(|source| ConvertError::Xsa { path: path.clone(), source })?;
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

/// Lowers one script. Instructions that can't be expressed yet become
/// [`Op::Unsupported`]; assets are written through `assets`.
pub fn lower_script(cd: &CdScript, file: &str, flow: Flow, assets: &mut AssetBank) -> Result<Vec<Op>, ConvertError> {
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
    for (index, ins) in cd.code.iter().enumerate() {
        if let Some(label) = own_labels.get(&index) {
            ops.push(Op::Label(format!("s{}_l{label}", cd.script_id)));
        }
        if let Some(link) = cd.link(ins) {
            pending_links.push(link?);
            continue;
        }
        let at = || SourceLoc { file: file.to_string(), index };
        if matches!(ins.op, OP_GOTO | OP_GOTO_2) {
            let target = cd.labels.get(ins.arg.max(0) as usize);
            if target.is_some_and(|l| l.script == cd.script_id && l.code_offset as usize / 12 == index) {
                continue; // label marker
            }
            if flow == Flow::Linear {
                ops.push(Op::Unsupported { what: format!("jump to label table entry {} (linear mode)", ins.arg), at: at() });
                continue;
            }
            // 425 of the game's 521 real jumps follow opcode 0x15 (a variable
            // test), so they are conditional. Conditions aren't decoded yet;
            // leave those out so the text plays through instead of always
            // jumping.
            if index > 0 && cd.code[index - 1].op == OP_TEST {
                ops.push(Op::Unsupported { what: format!("conditional jump to label table entry {}", ins.arg), at: at() });
                continue;
            }
        }
        let links = if cd.message(ins).is_some() { std::mem::take(&mut pending_links) } else { Vec::new() };
        let unsupported = || format!("opcode {:#04x} sub {:#x} arg {}", ins.op, ins.sub, ins.arg);
        match lower_instr(cd, ins, &links, assets) {
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
        let links = links.iter().filter_map(|l| link_range(segs, has_speaker, l)).collect();
        // Segments are the game's own display lines.
        return Ok(Some(Op::Text { speaker, text: body.join("\n"), voice: None, links }));
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
        let mut sprites = Vec::new();
        for e in &entries {
            // Entries with sprite 0 at (0, 0) (modes 0 and 2, 382 in the game)
            // don't show an image; most likely "remove this character".
            if e.sprite == 0 && e.x == 0 && e.y == 0 {
                continue;
            }
            let img = assets.sprite(e.sprite as usize)?;
            // Assumption: (x, y) is the sprite's centre on the 800×600 screen.
            sprites.push(SpritePlacement {
                image: img.path,
                x: i32::from(e.x) - img.width as i32 / 2,
                y: i32::from(e.y) - img.height as i32 / 2,
            });
        }
        let fade_ms = entries.iter().map(|e| u32::from(e.fade) * SPRITE_FADE_UNIT_MS).max().unwrap_or(0);
        return Ok(Some(Op::Sprites { sprites, fade_ms }));
    }
    if matches!(ins.op, OP_GOTO | OP_GOTO_2) {
        // arg indexes this script's label table; the target may be in
        // another script. Out-of-range targets (one past the table) aren't
        // understood yet.
        return Ok(cd.labels.get(ins.arg.max(0) as usize).map(|l| Op::Jump(format!("s{}_l{}", l.script, l.label))));
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
