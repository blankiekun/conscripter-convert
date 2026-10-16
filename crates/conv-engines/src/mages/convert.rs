//! Lowers TaskList scenes to the IR.
//!
//! Screen: the Vita's 960×544. Scripts place things in a 1280×720 space
//! (`:Position` is a picture's centre), so coordinates are scaled by 3/4
//! across and 544/720 down; the CG and CHR pictures are already at the
//! Vita's scale and are used as they are.
//!
//! Layers: every picture is a sprite layer, chosen by its `:Priority`
//! (0 front … 10 back): layer `PRIORITY_STEP · priority + n`, where `n`
//! counts down as pictures arrive, so newer pictures of one priority lie
//! over older ones. `SpriteBG` is the background plane: a new one replaces
//! the old. Characters are keyed by their character number, other
//! pictures by their texture.
//!
//! Tasks named `…BS` (SpriteBS, CharacterOpenBS) change the picture
//! without showing it; the next `InOut`/`InOut_Act` reveals the changes
//! (a rule mask `RULE<Type>` from ETC.cpk over `:Time` frames).
//!
//! Labels: scene `S01_01A` is `mg_s01_01a`; its tag `Jump` is
//! `mg_s01_01a_jump`.

use std::collections::{BTreeMap, HashMap};

use conv_core::ir::{
    Animation, ArithOp, ButtonScreen, Compare, FILTER_BLEND_COPY, FILTER_BLEND_MULTIPLY, FILTER_BLUR, FILTER_COLOUR, FILTER_GREYSCALE, FILTER_MONOTONE, FILTER_NEGATIVE, Noise, Op, Operand, ScreenEffect, SlideLayer, SourceLoc, SpriteChange, SpriteFilter,
    MotionEnd, MotionKey, Placed, SpritePlacement, Switch, TextStyle, TextWindow, TransitionStyle, Var, Voice,
};

use super::tasks::{Block, Scene, Val, parse};

pub const SCREEN_W: i32 = 960;
pub const SCREEN_H: i32 = 544;
/// The eye catch's shutters: frames and how long they take to close (the
/// eboot's time is not known; its chapter cards fade in over 30 frames).
const SHUTTER_FRAMES: u32 = 10;
const SHUTTER_MS: u32 = 500;
/// The skip dialog fades in and out over 15 frames (the eboot adds 1/15
/// a frame to its opacity).
const SKIP_DIALOG_FADE_MS: u32 = 250;
/// The story menu fades in over the title, and out once an item is chosen.
const STORY_MENU_FADE_MS: u32 = 300;
/// Variables from here up are global (kept across playthroughs).
pub const GLOBAL_BORDER: u32 = 200;
/// First local of the per-save flags (`[Flag]` in adv.txt): %60-%92,
/// clear of conv-cons's locals (%40-%45, %100 up).
const FLAG_BASE: u16 = 50;
/// First global (`[GlobalFlag]`): %410-%439, clear of conv-cons's
/// globals (%200-%301, %900 up).
const GLOBAL_FLAG_BASE: u16 = 210;
/// Global variable index of scene `n`'s "read" flag (the scenes in name
/// order): %1200 up, past conv-cons's and smoke.py's (%1000-%1099).
const SCENE_READ_BASE: u16 = 1000;
const PRIORITY_STEP: u32 = 20;
/// The front layer that `FadeOut` covers the picture with.
const COVER_LAYER: u32 = 0;

/// Frames (60 fps) to milliseconds.
/// A character's key: `c<number>`, the eboot's number + `:Copy` × 1000
/// (a second picture of one character, as the cut-ins in S03_02A).
fn chara_key(c: &Block, number: i64) -> String {
    format!("c{}", number + c.int("Copy").unwrap_or(0) * 1000)
}

fn ms(frames: i64) -> u32 {
    (frames.max(0) * 1000 / 60) as u32
}

fn sx(x: i64) -> i32 {
    (x * 3 / 4) as i32
}

fn sy(y: i64) -> i32 {
    (y * 544 / 720) as i32
}

/// Particle effects drawn as looping animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Particles {
    /// 光玉: floating, twinkling balls of light.
    LightBalls,
    /// 雪: falling snow.
    Snow,
    /// 紙吹雪: falling paper.
    Confetti,
}

/// The game's files, by archive ID.
pub trait Assets {
    /// A CG.cpk picture: output path, width, height.
    fn cg(&mut self, id: u32) -> Option<(String, u32, u32)>;
    /// A CG.cpk picture by file name (`EC27.png`), case-insensitive.
    fn cg_named(&mut self, name: &str) -> Option<(String, u32, u32)>;
    /// The ID of a CHR.cpk picture by file name (`ST_ASU02H.png`),
    /// case-insensitive.
    fn chr_id(&self, name: &str) -> Option<u32>;
    /// A CHR.cpk picture.
    fn chr(&mut self, id: u32) -> Option<(String, u32, u32)>;
    /// The rule mask `RULE<n>` of ETC.cpk, as large as the screen.
    fn rule(&mut self, n: u32) -> Option<String>;
    fn voice(&mut self, id: u32) -> Option<String>;
    fn se(&mut self, id: u32) -> Option<String>;
    fn bgm(&mut self, id: u32) -> Option<String>;
    /// Movie `n` (`:Movie`), converted to `movie/<name>.mp4`.
    fn movie(&mut self, n: u32) -> Option<String>;
    /// `frames` screens of grey static side by side (alpha 255).
    fn noise(&mut self, frames: u32) -> String;
    /// A `screenripple` map for horizontal waves: grey rising from 0 to
    /// 256 `cycles_x100`/100 times down the screen.
    fn wave_map(&mut self, cycles_x100: u32) -> String;
    /// A chapter card (`EyeCatch :Image 1,card,picture,date`) as one
    /// screen: SystemData's card `card` over the place `picture` in its
    /// frame, with date `date` (0 = 11月21日; negative: none).
    fn chapter_card(&mut self, card: u32, picture: u32, date: i32) -> Option<String>;
    /// The shutters of eye catch `n` (CG ID n + 457): `frames` screens side by
    /// side of the picture's top and bottom halves sliding in from above
    /// and below until they meet (`closing`), or sliding away from the
    /// whole picture.
    fn shutters(&mut self, n: u32, closing: bool, frames: u32) -> Option<String>;
    /// `count` particles of `kind` over the screen: a strip of
    /// [`super::assets::PARTICLE_FRAMES`] screens that loops.
    fn particles(&mut self, kind: Particles, count: u32) -> Option<String>;
    /// A picture of one colour (0xRRGGBB) as large as the screen.
    fn colour(&mut self, rgb: u32) -> String;
    /// The text window's picture (SystemData's `meswin.png`, scaled to the
    /// screen) and its size.
    fn window(&mut self) -> Option<(String, u32, u32)>;
    /// The skip dialog (`SceneJumpDialog`) in the Vita's look for `:BG`
    /// (protagonist, emblem, date) and `:CharaIcon` (`faces`), with the
    /// summary `text` drawn into a picture named after `name`: its
    /// pictures from the bottom up, and its Yes and No buttons. `None`
    /// without the font.
    fn skip_dialog(&mut self, bg: &[i64], faces: &[i64], text: &str, name: &str) -> Option<(Vec<Placed>, [Placed; 2])>;
    /// START's story menu in the Vita's look: its pictures from the bottom
    /// up (the title, the 〜Prelude〜 header, ？？？？？ in 狩男's place when
    /// `locked`), and the buttons 始めから and the after-stories of 美咲,
    /// 明日香, 京 and (unless `locked`) 狩男.
    fn story_menu(&mut self, locked: bool) -> Option<(Vec<Placed>, Vec<Placed>)>;
}

/// A character (`chrNNN.txt`): its pictures by pose, dress, face, size.
#[derive(Debug, Clone, Default)]
pub struct Character {
    pub offset_y: Vec<i64>,
    pub sprites: HashMap<[i64; 4], u32>,
}

/// A character action (`chract`): keyframes and whether it repeats until
/// `CharacterActionEnd`. The eboot (0x81008008) reads each field of a
/// frame (`:X`, `:Y`, `:Roll`, `:Scale`, `:Alpha`, `:Priority`) as a track
/// of its own, keyed where the field is given; the keys here are the
/// tracks sampled at every frame any of them has a key on (straight
/// between keys; a key's easing, given once, is left out, and so is
/// `:Priority`).
#[derive(Debug, Clone, Default)]
pub struct Action {
    pub keys: Vec<ActionKey>,
    pub looped: bool,
    /// Its length in frames (`:Frame`): a loop starts again after it.
    pub frames: i64,
    /// `:EndSet 1` (read only when it doesn't loop): the character stays
    /// as the last key leaves it (moved off, faded out).
    pub end_set: bool,
}

/// A frame of an [`Action`]: the offset in script units, the scale, the
/// roll (radians) and the alpha (0-255).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionKey {
    pub frame: i64,
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub roll: f64,
    pub alpha: f64,
}

impl ActionKey {
    /// Only an offset: what `spmotion` can do.
    fn plain(&self) -> bool {
        self.scale == 1.0 && self.roll == 0.0 && self.alpha >= 255.0
    }
}

/// The value of a track (frame, value; sorted) at `frame`: straight
/// between its keys, held before the first and after the last.
fn track_at(track: &[(i64, f64)], frame: i64, default: f64) -> f64 {
    let (Some(first), Some(last)) = (track.first(), track.last()) else { return default };
    if frame <= first.0 {
        return first.1;
    }
    if frame >= last.0 {
        return last.1;
    }
    let i = track.iter().rposition(|k| k.0 <= frame).unwrap_or(0);
    let (a, b) = (track[i], track[i + 1]);
    a.1 + (b.1 - a.1) * (frame - a.0) as f64 / (b.0 - a.0) as f64
}

impl Action {
    fn parse(a: &Block) -> Action {
        let names = ["X", "Y", "Scale", "Roll", "Alpha"];
        let mut tracks: Vec<Vec<(i64, f64)>> = vec![Vec::new(); names.len()];
        let mut frames: Vec<i64> = Vec::new();
        for f in &a.children {
            let Some(frame) = f.name.strip_prefix("Frame").and_then(|n| n.parse().ok()) else { continue };
            for (track, name) in tracks.iter_mut().zip(names) {
                if let Some(v) = f.float(name) {
                    track.push((frame, v));
                    frames.push(frame);
                }
            }
        }
        for track in &mut tracks {
            track.sort_by_key(|k| k.0);
        }
        frames.sort_unstable();
        frames.dedup();
        let keys = frames
            .into_iter()
            .map(|frame| {
                let v = |i: usize, default| track_at(&tracks[i], frame, default);
                ActionKey { frame, x: v(0, 0.0), y: v(1, 0.0), scale: v(2, 1.0), roll: v(3, 0.0), alpha: v(4, 255.0) }
            })
            .collect();
        let looped = a.int("Loop").unwrap_or(0) == 1;
        Action { keys, looped, frames: a.int("Frame").unwrap_or(0), end_set: !looped && a.int("EndSet").unwrap_or(0) == 1 }
    }
}

/// The game: every scene plus the tables in adv.txt and chrNNN.txt.
#[derive(Debug, Default)]
pub struct Game {
    pub scenes: BTreeMap<String, Scene>,
    pub characters: HashMap<i64, Character>,
    pub flags: Vec<String>,
    pub global_flags: Vec<String>,
    /// Speaker names by `:Name` number, from the scripts' comments.
    pub names: HashMap<i64, String>,
    pub actions: HashMap<i64, Action>,
    /// The CG gallery (`CGMODE`): pages of entries, each entry's CG.cpk
    /// textures and their seen flags (`:CGFlag` in the scenes).
    pub gallery: Vec<Vec<Vec<(u32, u32)>>>,
    /// The first scene of a new game.
    pub start: String,
}

impl Game {
    /// Builds the game from SCRIPT.cpk's files (name → decoded text).
    pub fn load(scripts: &BTreeMap<String, String>) -> Result<Game, String> {
        let mut g = Game::default();
        let mut name_votes: HashMap<i64, HashMap<String, usize>> = HashMap::new();
        for (file, src) in scripts {
            let blocks = parse(src).map_err(|e| format!("{file}: {e}"))?;
            if file.starts_with("chr") && file.ends_with(".txt") {
                for b in blocks.iter().filter(|b| b.name == "Character") {
                    let mut c = Character {
                        offset_y: b.get("OffsetY").unwrap_or(&[]).iter().filter_map(Val::int).collect(),
                        ..Character::default()
                    };
                    for s in &b.children {
                        let key: Vec<i64> = s.get("Character").unwrap_or(&[]).iter().filter_map(Val::int).collect();
                        if let (Ok(key), Some(tex)) = (key.try_into(), s.int("Texture")) {
                            c.sprites.insert(key, tex as u32);
                        }
                    }
                    if let Some(n) = b.int("CharacterNumber") {
                        g.characters.insert(n, c);
                    }
                }
            } else if file == "chract" {
                for b in blocks.iter().filter(|b| b.name == "Action") {
                    for a in &b.children {
                        let Some(n) = a.name.strip_prefix("Action").and_then(|n| n.parse().ok()) else { continue };
                        g.actions.insert(n, Action::parse(a));
                    }
                }
            } else if file == "CGMODE" {
                for b in blocks.iter().filter(|b| b.name == "CGMODE") {
                    for page in b.children.iter().filter(|c| c.name.starts_with("Page")) {
                        let entries = page
                            .children
                            .iter()
                            .filter(|c| c.name.starts_with("List"))
                            .map(|l| {
                                let ints = |k| l.get(k).unwrap_or(&[]).iter().filter_map(Val::int).map(|v| v as u32).collect::<Vec<_>>();
                                ints("Texture").into_iter().zip(ints("Flag")).collect()
                            })
                            .collect();
                        g.gallery.push(entries);
                    }
                }
            } else if file == "adv.txt" {
                let names = |b: Option<&Block>| -> Vec<String> {
                    b.map(|b| b.children.iter().map(|f| f.str("Name").unwrap_or("").to_string()).collect()).unwrap_or_default()
                };
                g.flags = names(blocks.iter().find(|b| b.name == "Flag"));
                g.global_flags = names(blocks.iter().find(|b| b.name == "GlobalFlag"));
                g.start = blocks
                    .iter()
                    .find(|b| b.name == "Scene")
                    .and_then(|b| b.child("StartScene0"))
                    .and_then(|b| b.str("SceneName"))
                    .unwrap_or("S01_01")
                    .to_string();
            } else {
                let scene = Scene::parse(src).map_err(|e| format!("{file}: {e}"))?;
                if !scene.tasks.is_empty() {
                    g.scenes.insert(file.clone(), scene);
                }
                // `:Name 12 // 鷹志`
                for line in src.lines() {
                    let t = line.trim_start();
                    let Some(rest) = t.strip_prefix(":Name") else { continue };
                    let Some((num, comment)) = rest.split_once("//") else { continue };
                    let (Ok(n), name) = (num.trim().parse::<i64>(), comment.trim()) else { continue };
                    if !name.is_empty() && !name.contains(' ') {
                        *name_votes.entry(n).or_default().entry(name.to_string()).or_default() += 1;
                    }
                }
            }
        }
        for (n, votes) in name_votes {
            if let Some((name, _)) = votes.into_iter().max_by_key(|(name, c)| (*c, std::cmp::Reverse(name.clone()))) {
                g.names.insert(n, name);
            }
        }
        Ok(g)
    }

    fn flag(&self, name: &str, global: bool) -> Option<Var> {
        let list = if global { &self.global_flags } else { &self.flags };
        let i = list.iter().position(|f| f == name)? as u16;
        Some(Var { global, index: if global { GLOBAL_FLAG_BASE } else { FLAG_BASE } + i })
    }
}

pub fn scene_label(scene: &str) -> String {
    format!("mg_{}", scene.to_ascii_lowercase())
}

fn tag_label(scene: &str, tag: &str) -> String {
    let tag: String = tag.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
    format!("{}_{tag}", scene_label(scene))
}

/// The text window: `meswin.png` across the bottom of the screen, above
/// the button bar. Its panel is x 77-883, y 28-152, with round ends; the
/// text fills x 120-840, y 40-144: four lines of 30, enough for the
/// longest message (109 characters with the name).
pub fn text_window(assets: &mut dyn Assets) -> TextWindow {
    let (image, w, h) = assets.window().unwrap_or_default();
    let (w, h) = (w as i32, h as i32);
    let x = (SCREEN_W - w) / 2;
    let y = SCREEN_H - h - 16;
    TextWindow {
        wait_icon_at: None,
        image,
        x,
        y,
        text_x: x + 120,
        text_y: y + 40,
        columns: 30,
        rows: 4,
        font_size: 23,
        line_gap: 4,
        vertical: false,
        name_plate: None,
    }
}

/// What the compiler collected besides the ops.
#[derive(Debug, Default)]
pub struct Compiled {
    pub ops: Vec<Op>,
    /// Task types not converted (or approximated), with counts.
    pub notes: BTreeMap<String, usize>,
    /// The tasks, converted fully or with gaps.
    pub metrics: crate::metrics::Metrics,
}

/// A picture on screen, for moving and removing it.
#[derive(Debug, Clone)]
struct Shown {
    layer: u32,
    /// The texture (CG ID), or -1 for a character.
    tex: i64,
    /// Centre on screen.
    x: i32,
    y: i32,
    image: String,
    w: i32,
    h: i32,
    /// A character's centre at `:Position` y 0.
    base_y: i32,
    look: Look,
    /// Its zoom (SpriteMove `:Scale`), in percent.
    zoom: u32,
    /// A motion (CharacterAction) runs: looping, or once for this many ms
    /// from the message count given.
    motion: Option<Motion>,
}

#[derive(Debug, Clone, Copy)]
struct Motion {
    looped: bool,
    /// It leaves the character at its last key ([`MotionEnd::Keep`]).
    keep: bool,
    ms: u32,
    since_message: usize,
}

/// How a picture is drawn: its colour filters (`:Color` tint,
/// `:Grayscale`, `:Gamma` sepia, `:Floor`/`:Ceil` negative) and whether it
/// is see-through (`:Color` alpha 0, shown later by a CharacterMove).
#[derive(Debug, Clone, Default, PartialEq)]
struct Look {
    filters: Vec<SpriteFilter>,
    hidden: bool,
}

impl Look {
    /// The look a task's (or a `[CharaN]` block's) keys ask for.
    fn of(b: &Block) -> Look {
        let c = Operand::Const;
        let filter = |kind: i32, blend: i32, params: [i32; 3]| SpriteFilter { kind: c(kind), blend: c(blend), strength: c(256), params: params.map(c) };
        let mut filters = Vec::new();
        if b.int("Grayscale").unwrap_or(0) != 0 {
            let gamma: Vec<f64> = b.get("Gamma").unwrap_or(&[]).iter().filter_map(Val::float).collect();
            match gamma.as_slice() {
                // Sepia (3.0,1.4,0.6): each channel of the grey raised to
                // 1/gamma, as an offset at mid-grey.
                [r, g, bl, ..] => {
                    let offset = |g: f64| if g > 0.0 { (255.0 * (0.5f64.powf(1.0 / g) - 0.5)).round() as i32 } else { 0 };
                    filters.push(filter(FILTER_MONOTONE, FILTER_BLEND_COPY, [offset(*r), offset(*g), offset(*bl)]));
                }
                _ => filters.push(filter(FILTER_GREYSCALE, FILTER_BLEND_COPY, [0; 3])),
            }
        }
        let colour: Vec<i64> = b.get("Color").unwrap_or(&[]).iter().filter_map(Val::int).collect();
        if let [r, g, bl, ..] = colour[..]
            && (r, g, bl) != (255, 255, 255)
        {
            filters.push(filter(FILTER_COLOUR, FILTER_BLEND_MULTIPLY, [r as i32, g as i32, bl as i32]));
        }
        // "Film reversal": white floor, black ceiling.
        if b.get("Floor").is_some() && b.get("Ceil").is_some() {
            filters.push(filter(FILTER_NEGATIVE, FILTER_BLEND_COPY, [0; 3]));
        }
        Look { filters, hidden: colour.get(3) == Some(&0) }
    }
}

struct Lowering<'a> {
    game: &'a Game,
    assets: &'a mut dyn Assets,
    out: Compiled,
    /// Pictures by key: `c<number>` for characters, `t<texture>`, `bg`.
    shown: BTreeMap<String, Shown>,
    next_slot: HashMap<u32, u32>,
    /// The task has `:FadeWait 0`: its pictures fade without the script
    /// waiting.
    fading: bool,
    /// The task's `:Rule` mask (sprite fades), if any.
    rule: Option<String>,
    /// The task's `:Scale` (SpriteBG, SpriteBS, Sprite), in percent.
    scale: u32,
    /// A transition is open (staged changes wait for InOut).
    staging: bool,
    choices: usize,
    /// The colour a FadeOut covers the picture with, until FadeIn.
    cover: Option<u32>,
    /// An eye catch is on the cover's layer.
    eye_catch: bool,
    /// It is `ECnn` (n), which leaves through the shutters.
    shutters: Option<u32>,
    /// Pictures an earlier scene left may be up: the eboot drops a
    /// scene's pictures when the next one loads, so the first picture it
    /// shows (`SpriteBG`, `SpriteBS`, or a `Sprite` in 2 scenes) removes
    /// them all.
    inherited: bool,
    /// No message yet: the scene's opening, which the scripts write once
    /// per branch (skipped or not), each after a `Tag`.
    opening: bool,
    /// Messages so far, to tell whether one came since an action started.
    messages: usize,
}

impl Lowering<'_> {
    fn note(&mut self, what: impl Into<String>) {
        let what = what.into();
        self.out.metrics.note(&what);
        *self.out.notes.entry(what).or_default() += 1;
    }

    fn unsupported(&mut self, scene: &str, index: usize, what: String) {
        self.note(what.clone());
        self.out.ops.push(Op::Unsupported { what, at: SourceLoc { file: scene.to_string(), index } });
    }

    fn layer(&mut self, key: &str, priority: i64) -> u32 {
        let base = PRIORITY_STEP * priority.clamp(0, 10) as u32 + 1;
        if let Some(s) = self.shown.get(key)
            && s.layer >= base
            && s.layer < base + PRIORITY_STEP - 1
        {
            return s.layer;
        }
        // The next slot of the priority's, skipping those in use.
        let n = self.next_slot.entry(base).or_insert(PRIORITY_STEP - 2);
        for _ in 0..PRIORITY_STEP - 1 {
            let layer = base + *n;
            *n = if *n == 0 { PRIORITY_STEP - 2 } else { *n - 1 };
            if !self.shown.values().any(|s| s.layer == layer) {
                return layer;
            }
        }
        base + *n
    }

    /// Shows `image` centred at script position (x, y) on `key`'s layer,
    /// removing the pictures `replaces` in the same fade.
    #[allow(clippy::too_many_arguments)]
    fn show(&mut self, key: &str, tex: i64, image: (String, u32, u32), x: i64, y: i64, priority: i64, fade_ms: u32, replaces: &[String], look: Look) {
        self.show_at(key, tex, image, sx(x), sy(y), priority, fade_ms, replaces, look);
    }

    /// `show` with the centre in screen pixels. A picture with filters
    /// fades in inside a transition, so that it does so filtered.
    #[allow(clippy::too_many_arguments)]
    fn show_at(&mut self, key: &str, tex: i64, image: (String, u32, u32), cx: i32, cy: i32, priority: i64, fade_ms: u32, replaces: &[String], look: Look) {
        // A zoomed picture is shown zoomed in the same print.
        let opened = (look != Look::default() || self.rule.is_some() || self.scale != 100) && (fade_ms > 0 || self.scale != 100) && !self.staging;
        if opened {
            self.out.ops.push(Op::BeginTransition { effect: "Sprite".into(), fade_ms, mask: self.rule.clone(), style: TransitionStyle::Fade });
            self.staging = true;
        }
        let mut changes: Vec<SpriteChange> = replaces
            .iter()
            .filter(|k| *k != key)
            .filter_map(|k| self.shown.remove(k))
            .map(|old| SpriteChange { layer: old.layer, sprite: None, keep_filters: false })
            .collect();
        // `:FadeWait 0`: the picture fades while the script goes on (the
        // eboot's Message task doesn't wait for it). A picture changing
        // goes to another layer, the old one fading out as it fades in;
        // one with a motion, zoom or look keeps its layer and waits.
        let fading = self.fading
            && !self.staging
            && fade_ms > 0
            && self.shown.get(key).is_none_or(|o| o.motion.is_none() && o.zoom == 100 && o.look == Look::default());
        if fading && let Some(old) = self.shown.remove(key) {
            changes.push(SpriteChange { layer: old.layer, sprite: None, keep_filters: false });
        }
        let layer = self.layer(key, priority);
        let (path, w, h) = image;
        let (w, h) = (w as i32, h as i32);
        if let Some(old) = self.shown.get(key)
            && old.layer != layer
        {
            changes.push(SpriteChange { layer: old.layer, sprite: None, keep_filters: false });
        }
        changes.push(SpriteChange {
            layer,
            sprite: Some(SpritePlacement { image: path.clone(), x: cx - w / 2, y: cy - h / 2 }),
            keep_filters: false,
        });
        if fading {
            self.out.ops.push(Op::SpritesFading { changes, fade_ms });
        } else {
            self.out.ops.push(Op::Sprites { changes, fade_ms: if self.staging { 0 } else { fade_ms } });
        }
        self.apply_look(layer, &look);
        // A new picture on the layer would keep the old one's zoom.
        let zoom = self.scale;
        if zoom != 100 || self.shown.get(key).is_some_and(|old| old.layer == layer && old.zoom != 100) {
            self.out.ops.push(Op::SpriteZoom { layer, x: cx, y: cy, from: zoom, to: zoom, ms: 0 });
        }
        self.shown.insert(key.to_string(), Shown { layer, tex, x: cx, y: cy, image: path, w, h, base_y: cy, look, zoom, motion: None });
        if opened {
            self.out.ops.push(Op::EndTransition);
            self.staging = false;
        }
    }

    /// Takes a motion that leaves `key`'s character at its last key to its
    /// end (the engine does when another motion command reaches the
    /// sprite), before the character is moved or shown anew.
    fn settle(&mut self, key: &str) {
        let Some(s) = self.shown.get_mut(key) else { return };
        if s.motion.is_some_and(|m| m.keep) {
            s.motion = None;
            let layer = s.layer;
            self.out.ops.push(Op::SpriteMotion { layer, keys: vec![], looped: false });
        }
    }

    /// Puts a look on a layer whose picture was just shown.
    fn apply_look(&mut self, layer: u32, look: &Look) {
        if !look.filters.is_empty() {
            self.out.ops.push(Op::SpriteFilters { layer, filters: look.filters.clone() });
        }
        if look.hidden {
            self.out.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: 0 });
        }
    }

    /// Moves `key`'s picture to the layers of another priority, in place.
    fn reprioritize(&mut self, key: &str, priority: i64) {
        let Some(s) = self.shown.get(key).cloned() else { return };
        let layer = self.layer(key, priority);
        if layer == s.layer {
            return;
        }
        let changes = vec![
            SpriteChange { layer: s.layer, sprite: None, keep_filters: false },
            SpriteChange { layer, sprite: Some(SpritePlacement { image: s.image.clone(), x: s.x - s.w / 2, y: s.y - s.h / 2 }), keep_filters: false },
        ];
        self.out.ops.push(Op::Sprites { changes, fade_ms: 0 });
        self.apply_look(layer, &s.look);
        if s.zoom != 100 {
            self.out.ops.push(Op::SpriteZoom { layer, x: s.x, y: s.y, from: 100, to: s.zoom, ms: 0 });
        }
        self.shown.insert(key.to_string(), Shown { layer, ..s });
    }

    fn hide(&mut self, key: &str, fade_ms: u32) {
        self.hide_all(&[key.to_string()], fade_ms);
    }

    /// Removes several pictures in one fade.
    fn hide_all(&mut self, keys: &[String], fade_ms: u32) {
        let changes: Vec<SpriteChange> = keys
            .iter()
            .filter_map(|k| self.shown.remove(k))
            .map(|old| SpriteChange { layer: old.layer, sprite: None, keep_filters: false })
            .collect();
        if changes.is_empty() {
        } else if let Some(mask) = self.rule.clone().filter(|_| !self.staging && fade_ms > 0) {
            self.out.ops.push(Op::BeginTransition { effect: "Sprite".into(), fade_ms, mask: Some(mask), style: TransitionStyle::Fade });
            self.out.ops.push(Op::Sprites { changes, fade_ms: 0 });
            self.out.ops.push(Op::EndTransition);
        } else if self.fading && !self.staging && fade_ms > 0 {
            self.out.ops.push(Op::SpritesFading { changes, fade_ms });
        } else {
            self.out.ops.push(Op::Sprites { changes, fade_ms: if self.staging { 0 } else { fade_ms } });
        }
    }

    /// Opens a transition for a staged task, if none is open: its look
    /// comes from the next InOut in `tasks`.
    fn stage(&mut self, tasks: &[Block], from: usize) {
        if self.staging {
            return;
        }
        let reveal = tasks[from..].iter().find(|t| matches!(t.task_type(), Some("InOut" | "InOut_Act")));
        let (fade_ms, mask, style) = match reveal {
            Some(t) => {
                let time = ms(t.int("Time").unwrap_or(30));
                match t.task_type() {
                    // `:Type N` is RULE<N-1>, 0 a plain fade (table 0x81162e38).
                    Some("InOut") => (time, t.int("Type").filter(|&n| n > 0).and_then(|n| self.assets.rule(n as u32 - 1)), TransitionStyle::Fade),
                    _ => (time, None, act_style(t)),
                }
            }
            None => (0, None, TransitionStyle::Fade),
        };
        self.out.ops.push(Op::BeginTransition { effect: "InOut".into(), fade_ms, mask, style });
        self.staging = true;
    }

    /// A scene's first picture: clears every picture (an
    /// earlier scene's are unknown here) in its fade, keeping the FadeOut
    /// cover if the scene starts under it.
    fn replace_inherited(&mut self, key: &str, tex: i64, img: (String, u32, u32), t: &Block, priority: i64, fade_ms: u32) {
        let opened = !self.staging;
        if opened {
            self.out.ops.push(Op::BeginTransition { effect: "SpriteBG".into(), fade_ms, mask: None, style: TransitionStyle::Fade });
            self.staging = true;
        }
        self.shown.clear();
        self.out.ops.push(Op::ClearSprites);
        if let Some(rgb) = self.cover {
            let image = self.assets.colour(rgb);
            let cover = SpriteChange { layer: COVER_LAYER, sprite: Some(SpritePlacement { image, x: 0, y: 0 }), keep_filters: false };
            self.out.ops.push(Op::Sprites { changes: vec![cover], fade_ms: 0 });
        }
        let (x, y) = (t.int_at("Position", 0).unwrap_or(640), t.int_at("Position", 1).unwrap_or(360));
        self.show(key, tex, img, x, y, priority, 0, &[], Look::of(t));
        if let Some(flag) = t.int("CGFlag").filter(|f| *f >= 0) {
            self.out.ops.push(Op::CgSeen { number: flag as u32 + 1 });
        }
        if opened {
            self.out.ops.push(Op::EndTransition);
            self.staging = false;
        }
    }

    /// A `[CharaN]` `:Number` (character, pose, dress, face, size). One
    /// script (S04_16B) has the placeholder `<ST_ASU02H>>` left in its
    /// place: the picture's name, resolved through the character tables.
    fn character_number(&self, number: &[Val]) -> Option<Vec<i64>> {
        if let Some(Val::Word(w)) = number.first() {
            let name = w.trim_matches(|c| c == '<' || c == '>');
            let id = self.assets.chr_id(&format!("{name}.png"))?;
            let (c, key) = self.game.characters.iter().find_map(|(c, ch)| ch.sprites.iter().find(|(_, t)| **t == id).map(|(k, _)| (*c, *k)))?;
            return Some([&[c][..], &key[..]].concat());
        }
        Some(number.iter().filter_map(Val::int).collect())
    }

    fn character_image(&mut self, v: &[i64]) -> Option<(String, u32, u32, i64)> {
        let (&c, key) = v.split_first()?;
        let ch = self.game.characters.get(&c)?;
        let key: [i64; 4] = key.get(..4)?.try_into().ok()?;
        let tex = *ch.sprites.get(&key)?;
        let offset = ch.offset_y.get(key[3] as usize).copied().unwrap_or(0);
        let (path, w, h) = self.assets.chr(tex)?;
        Some((path, w, h, offset))
    }

    fn task(&mut self, scene: &str, tasks: &[Block], i: usize) {
        let t = &tasks[i];
        let Some(ty) = t.task_type() else { return };
        let fade = |t: &Block| if t.int("FadeIn").unwrap_or(1) == 0 { 0 } else { ms(t.int("Time").unwrap_or(15)) };
        // `:FadeWait 0` (task_run 0x81021070 goes to the next task at
        // once); SpriteBG and SpriteBS, which replace the whole picture,
        // still wait.
        self.fading = t.int("FadeWait") == Some(0) && matches!(ty, "CharacterOpen" | "CharacterDelete" | "Sprite" | "SpriteDelete" | "SpriteDeleteBG");
        // `:Rule N` (sprite fades; 0 a plain fade) is ETC.cpk's file N - 1,
        // RULE<N-1> (the eboot's table 0x811635fc, file ID + 200000).
        // `:Scale` (scene_load cases 9, 10 and SpriteBS's, task +0xa0, the
        // sprite's scale +0x48) zooms the picture about its centre, which
        // `:Position` places, as SpriteMove's does.
        self.scale = match ty {
            "SpriteBG" | "SpriteBS" | "Sprite" => t.float("Scale").map_or(100, |f| (f * 100.0).round().max(1.0) as u32),
            _ => 100,
        };
        self.rule = match t.int("Rule") {
            Some(n) if n > 0 && ty.starts_with("Sprite") => self.assets.rule(n as u32 - 1),
            _ => None,
        };
        // A character's fade (task_charopen_new, task_chardelete_new): over
        // `:Fade` frames, 18 without one, at once for `:Fade 0` (or
        // CharacterDelete's `:FadeOut 0`).
        let char_fade = |t: &Block, off: &str| match t.int("Fade") {
            Some(f) => ms(f),
            None if t.int(off) == Some(0) => 0,
            None => ms(18),
        };
        // The eboot reads the first `:SpriteMax` of Chara0-Chara9.
        if ty.starts_with("Character")
            && let Some(max) = t.int("SpriteMax")
            && (max as usize) < t.children.iter().filter(|c| c.name.starts_with("Chara")).count()
        {
            self.note(format!("{ty} :SpriteMax {max} below its [CharaN] blocks"));
        }
        match ty {
            "Message" => {
                let text = t.str("Message").unwrap_or("").to_string();
                let speaker = t.int("Name").and_then(|n| self.game.names.get(&n).cloned());
                let voice = t.int("Voice").and_then(|id| {
                    let file = self.assets.voice(id as u32)?;
                    // One voice switch for everyone (キャラクター音声
                    // 全てON / 全てOFF): character 0.
                    Some(Voice { file, character: 0 })
                });
                if self.staging {
                    self.out.ops.push(Op::EndTransition);
                    self.staging = false;
                }
                self.opening = false;
                self.messages += 1;
                // `:FontScale` (record +0x8fc) sizes the whole message:
                // shouts and whispers.
                let styles = match t.float("FontScale").map(|f| (f * 100.0).round() as u16) {
                    Some(p) if p != 100 && p > 0 => vec![TextStyle { start: 0, len: text.chars().count(), size_percent: Some(p), ..Default::default() }],
                    _ => vec![],
                };
                self.out.ops.push(Op::Text { speaker, text, voice, links: vec![], ruby: vec![], styles, hang: None, continues: false });
            }
            "SpriteBG" | "Sprite" | "SpriteBS" => {
                let Some(tex) = t.int("Texture") else { return };
                let Some(img) = self.assets.cg(tex as u32) else {
                    self.note(format!("{ty}: missing texture {tex}"));
                    return;
                };
                if ty == "SpriteBS" {
                    self.stage(tasks, i);
                }
                let key = format!("t{tex}");
                let prio = t.int("Priority").unwrap_or(if ty == "Sprite" { 5 } else { 10 });
                if std::mem::take(&mut self.inherited) {
                    self.replace_inherited(&key, tex, img, t, prio, fade(t));
                    return;
                }
                // SpriteBG and SpriteBS replace every picture (a background,
                // a black cover, a white flash, an event CG), the
                // characters too unless `:SpriteDelete 0` (the scripts
                // then delete them themselves, or leave them over a black
                // cover); Sprite adds one, which the scripts remove by
                // texture with SpriteDelete(BG).
                let keep_characters = t.int("SpriteDelete") == Some(0);
                let replaces: Vec<String> = if ty == "Sprite" {
                    vec![]
                } else {
                    self.shown.keys().filter(|k| k.starts_with('t') || !keep_characters).cloned().collect()
                };
                let (x, y) = (t.int_at("Position", 0).unwrap_or(640), t.int_at("Position", 1).unwrap_or(360));
                self.show(&key, tex, img, x, y, prio, fade(t), &replaces, Look::of(t));
                // A gallery CG: seen from now on (CG numbers are 1-based).
                if let Some(flag) = t.int("CGFlag").filter(|f| *f >= 0) {
                    self.out.ops.push(Op::CgSeen { number: flag as u32 + 1 });
                }
            }
            "SpriteDelete" | "SpriteDeleteBG" => {
                let tex = t.int("SpriteNumber").or_else(|| t.int("Texture")).unwrap_or(-1);
                let f = if t.int("FadeOut").unwrap_or(0) == 0 { 0 } else { ms(t.int("Time").unwrap_or(15)) };
                self.hide(&format!("t{tex}"), f);
            }
            "CharacterOpen" | "CharacterOpenBS" => {
                if ty == "CharacterOpenBS" {
                    self.stage(tasks, i);
                    // The characters it doesn't name leave with the old
                    // scene (the scripts re-open or delete-all them after,
                    // and never delete them by name).
                    let named: Vec<String> = t
                        .children
                        .iter()
                        .filter(|c| c.name.starts_with("Chara"))
                        .filter_map(|c| Some(chara_key(c, c.int("Number")?)))
                        .collect();
                    let mut gone: Vec<String> = self.shown.keys().filter(|k| k.starts_with('c') && !named.contains(k)).cloned().collect();
                    gone.sort();
                    self.hide_all(&gone, 0);
                }
                for c in t.children.iter().filter(|c| c.name.starts_with("Chara")) {
                    let Some(raw) = c.get("Number") else { continue };
                    let Some(number) = self.character_number(raw) else {
                        self.note(format!("{ty}: unknown character picture {raw:?}"));
                        continue;
                    };
                    let number = &number[..];
                    let Some((path, w, h, offset)) = self.character_image(number) else {
                        self.note(format!("{ty}: unknown character picture {number:?}"));
                        continue;
                    };
                    let key = chara_key(c, number.first().copied().unwrap_or(-1));
                    self.settle(&key);
                    let x = sx(c.int_at("Position", 0).unwrap_or(640));
                    // y 0 is the usual height: a picture taller than the
                    // screen (the standing characters, cut at the waist)
                    // stands on its bottom edge, others are centred,
                    // raised by the character's offset. (The eboot scales
                    // OffsetY, -68 for everyone standing, to the screen,
                    // but centred that leaves them 8 pixels above the
                    // edge; where it anchors them is not found yet.)
                    // `:Center cx,cy` (4 uses, all 0,1, with y 720 or 840)
                    // is the point of the picture that `:Position` places,
                    // -1 to 1 from its top edge to its bottom one (the
                    // eboot's quad, 0x8103d53c): there y counts from the
                    // picture's centre raised by cy × half its height.
                    // shortcut: cx is not read (always 0); a non-zero cx
                    // needs a base_x for CharacterMove too.
                    let base_y = match c.at("Center", 1).and_then(Val::float) {
                        Some(cy) => -(cy * h as f64 / 2.0).round() as i32,
                        None if h as i32 > SCREEN_H => SCREEN_H - h as i32 / 2,
                        None => sy(360 + offset),
                    };
                    let y = base_y + sy(c.int_at("Position", 1).unwrap_or(0));
                    let prio = c.int("Priority").unwrap_or(9);
                    self.show_at(&key, -1, (path, w, h), x, y, prio, if ty == "CharacterOpen" { char_fade(t, "Fade") } else { 0 }, &[], Look::of(c));
                    if let Some(s) = self.shown.get_mut(&key) {
                        s.base_y = base_y;
                    }
                }
            }
            "CharacterDelete" => {
                let listed: Vec<String> =
                    t.children.iter().filter(|c| c.name.starts_with("Chara")).map(|c| chara_key(c, c.int("Number").unwrap_or(-1))).collect();
                // Without a list (`:SpriteMax 0`, 962 of 1414 uses) every
                // character leaves.
                let keys = if listed.is_empty() {
                    let mut all: Vec<String> = self.shown.keys().filter(|k| k.starts_with('c')).cloned().collect();
                    all.sort();
                    all
                } else {
                    listed
                };
                self.hide_all(&keys, char_fade(t, "FadeOut"));
            }
            "CharacterMove" => {
                let time = ms(t.int("Time").unwrap_or(0));
                for c in t.children.iter().filter(|c| c.name.starts_with("Chara")) {
                    let key = chara_key(c, c.int("Number").unwrap_or(-1));
                    self.settle(&key);
                    let Some(s) = self.shown.get(&key).cloned() else { continue };
                    let x = c.int_at("Position", 0).map_or(s.x, sx);
                    // y is from the character's usual height.
                    let y = s.base_y + sy(c.int_at("Position", 1).unwrap_or(0));
                    // `:Color` is where the colour and opacity end up.
                    let look = if c.get("Color").is_some() { Look::of(c) } else { s.look.clone() };
                    if look.filters != s.look.filters {
                        self.out.ops.push(Op::SpriteFilters { layer: s.layer, filters: look.filters.clone() });
                    }
                    if look.hidden != s.look.hidden && !self.staging {
                        // Fading in or out: one cross-fade to the new
                        // place and opacity over the move's time.
                        self.out.ops.push(Op::BeginTransition { effect: "CharacterMove".into(), fade_ms: time, mask: None, style: TransitionStyle::Fade });
                        self.out.ops.push(Op::SpriteMove { layer: s.layer, x, y, ms: 0 });
                        self.out.ops.push(Op::SpriteOpacity { layer: Some(s.layer), opacity: if look.hidden { 0 } else { 256 } });
                        self.out.ops.push(Op::EndTransition);
                    } else {
                        self.ease(c);
                        self.out.ops.push(Op::SpriteMove { layer: s.layer, x, y, ms: time });
                    }
                    self.shown.insert(key, Shown { x, y, look, ..s });
                }
            }
            "InOut" | "InOut_Act" => {
                if self.staging {
                    self.out.ops.push(Op::EndTransition);
                    self.staging = false;
                }
            }
            "FadeOut" => {
                let rgb = colour(t);
                self.cover = Some(rgb);
                let image = self.assets.colour(rgb);
                let cover = SpriteChange { layer: COVER_LAYER, sprite: Some(SpritePlacement { image, x: 0, y: 0 }), keep_filters: false };
                self.out.ops.push(Op::Sprites { changes: vec![cover.clone()], fade_ms: ms(t.int("Time").unwrap_or(0)) });
                // Under the cover: remove every picture (also ones an
                // earlier scene left), then put the cover back.
                if t.int("SpriteDelete").unwrap_or(0) != 0 {
                    self.shown.clear();
                    self.inherited = false;
                    self.out.ops.push(Op::ClearSprites);
                    self.out.ops.push(Op::Sprites { changes: vec![cover], fade_ms: 0 });
                }
            }
            "FadeIn" => {
                self.cover = None;
                let change = SpriteChange { layer: COVER_LAYER, sprite: None, keep_filters: false };
                self.out.ops.push(Op::Sprites { changes: vec![change], fade_ms: ms(t.int("Time").unwrap_or(0)) });
            }
            "TimeWait" => self.out.ops.push(Op::Wait { ms: ms(t.int("Time").unwrap_or(0)) }),
            "Window" => {
                // `:WindowNumber` n ≥ 0 (2 uses) only sets the shown flag
                // of the eboot's extra text box n (0x81120abc), which the
                // next message sets again: the text window stays. -1 (the
                // default, 1,506 uses) is the window itself.
                if t.int("Visible") == Some(0) && t.int("WindowNumber").unwrap_or(-1) < 0 {
                    self.out.ops.push(Op::HideTextWindow);
                }
            }
            "BgmPlay" => {
                if let Some(file) = t.int("BGM").and_then(|id| self.assets.bgm(id as u32)) {
                    let div = t.int("Div").unwrap_or(1);
                    self.out.ops.push(Op::BgmPlay { file, fade_in_ms: if div > 1 { ms(div) } else { 0 } });
                }
            }
            "BgmStop" => self.out.ops.push(Op::BgmStop { fade_out_ms: ms(t.int("Div").unwrap_or(0)) }),
            "SePlay" => {
                if let Some(file) = t.int("SE").and_then(|id| self.assets.se(id as u32)) {
                    self.out.ops.push(Op::SePlay { file, looped: t.int("Loop").unwrap_or(0) != 0 });
                }
            }
            "SeStop" => {
                let file = t.int("SE").and_then(|id| self.assets.se(id as u32));
                self.out.ops.push(Op::SeStop { file });
            }
            "Tag" => {
                if let Some(name) = t.str("Name") {
                    // Another branch of the opening: it starts again from
                    // the earlier scene's pictures.
                    self.inherited |= self.opening;
                    self.out.ops.push(Op::Label(tag_label(scene, name)));
                }
            }
            "Jump_Tag" => {
                if let Some(tag) = t.str("NextTag") {
                    self.out.ops.push(Op::Jump(tag_label(scene, tag)));
                }
            }
            "SceneJumpDialog" => self.scene_jump(scene, t),
            "NextScene" => {
                // `"S01_02A","SJ"`: at that scene's tag.
                let v = t.get("NextScene").unwrap_or(&[]);
                match (v.first().and_then(Val::str), v.get(1).and_then(Val::str)) {
                    (Some(next), Some(tag)) => self.out.ops.push(Op::Jump(tag_label(next, tag))),
                    (Some(next), None) => self.out.ops.push(Op::Jump(scene_label(next))),
                    _ => {}
                }
            }
            "End" => self.out.ops.push(Op::ReturnToTitle),
            "FlagSwitch_Tag" | "GlobalFlagSwitch_Tag" => {
                let global = ty.starts_with("Global");
                let (Some(name), Some(tag)) = (t.str("FlagName"), t.str("NextTag")) else { return };
                let Some(var) = self.game.flag(name, global) else {
                    self.unsupported(scene, i, format!("{ty}: unknown flag {name}"));
                    return;
                };
                let cmp = match t.str("Operator").unwrap_or("==") {
                    "!=" => Compare::Ne,
                    "<" => Compare::Lt,
                    ">" => Compare::Gt,
                    "<=" => Compare::Le,
                    ">=" => Compare::Ge,
                    _ => Compare::Eq,
                };
                let value = Operand::Const(t.int("Param").unwrap_or(0) as i32);
                self.out.ops.push(Op::If { var, cmp, value, label: tag_label(scene, tag) });
            }
            "FlagOperation" | "GlobalFlagOperation" => {
                let global = ty.starts_with("Global");
                let Some(name) = t.str("FlagName") else { return };
                let Some(var) = self.game.flag(name, global) else {
                    self.unsupported(scene, i, format!("{ty}: unknown flag {name}"));
                    return;
                };
                if let Some(op) = self.arith(t.get("Action").unwrap_or(&[])) {
                    self.out.ops.push(Op::Arith { var, op: op.0, value: Operand::Const(op.1) });
                }
            }
            "MessageSwitch_Tag" => self.choice(scene, t),
            "CharacterAction" => {
                for c in t.children.iter().filter(|c| c.name.starts_with("Chara")) {
                    let key = chara_key(c, c.int("Number").unwrap_or(-1));
                    let (Some(s), Some(a)) = (self.shown.get(&key).cloned(), c.int("Action").and_then(|n| self.game.actions.get(&n)))
                    else {
                        continue;
                    };
                    // Keyframe to keyframe from its place; a loop goes back
                    // to its first key at the action's length.
                    let mut keys: Vec<MotionKey> = a
                        .keys
                        .iter()
                        .map(|k| MotionKey {
                            ms: ms(k.frame),
                            x: sx(k.x.round() as i64),
                            y: sy(k.y.round() as i64),
                            zoom: (k.scale * 100.0).round().max(0.0) as u32,
                            // Radians, clockwise on the screen (our guess).
                            angle: (-k.roll.to_degrees()).round() as i32,
                            opacity: (k.alpha.clamp(0.0, 255.0) * 256.0 / 255.0).round() as u32,
                        })
                        .collect();
                    if a.looped
                        && let Some(&first) = keys.first()
                        && a.frames > a.keys.last().map_or(0, |k| k.frame)
                    {
                        keys.push(MotionKey { ms: ms(a.frames), ..first });
                    }
                    let length = keys.last().map_or(0, |k| k.ms);
                    if keys.is_empty() || length == 0 {
                        continue;
                    }
                    // `:EndSet` leaves the character where the last key
                    // has it; spmotion's motions end back at the place.
                    let last = *keys.last().unwrap();
                    let keep = a.end_set && (last.x, last.y, last.zoom, last.opacity) != (0, 0, 100, 256);
                    self.settle(&key);
                    if !keep && a.keys.iter().all(ActionKey::plain) {
                        let keys = keys.iter().map(|k| (k.ms, k.x, k.y)).collect();
                        self.out.ops.push(Op::SpriteMotion { layer: s.layer, keys, looped: a.looped });
                    } else {
                        let end = if a.looped {
                            MotionEnd::Loop
                        } else if keep {
                            MotionEnd::Keep
                        } else {
                            MotionEnd::Back
                        };
                        self.out.ops.push(Op::SpriteMotion2 { layer: s.layer, keys, end });
                    }
                    let motion = Motion { looped: a.looped, keep, ms: length, since_message: self.messages };
                    let mut s = Shown { motion: Some(motion), ..s };
                    if keep {
                        s.x += last.x;
                        s.y += last.y;
                        s.zoom = s.zoom * last.zoom / 100;
                        s.look.hidden |= last.opacity == 0;
                    }
                    self.shown.insert(key, s);
                    if t.int("EndWait").unwrap_or(0) != 0 && !a.looped {
                        self.out.ops.push(Op::Wait { ms: length });
                    }
                }
            }
            "CharacterActionEnd" => {
                // Ends the characters' actions (`:Character`, or every
                // one); `:EndWait 1` waits for an action still running.
                let only = t.int("Character").map(|n| format!("c{n}"));
                let mut keys: Vec<String> = self.shown.iter().filter(|(k, v)| v.motion.is_some() && only.as_ref().is_none_or(|o| o == *k)).map(|(k, _)| k.clone()).collect();
                keys.sort();
                let wait = t.int("EndWait").unwrap_or(0) != 0;
                for key in keys {
                    let Some(s) = self.shown.get_mut(&key) else { continue };
                    let Some(m) = s.motion.take() else { continue };
                    let layer = s.layer;
                    if m.looped || !wait {
                        self.out.ops.push(Op::SpriteMotion { layer, keys: vec![], looped: false });
                    } else if m.since_message == self.messages {
                        self.out.ops.push(Op::Wait { ms: m.ms });
                    }
                }
            }
            "Flash" => {
                let c: Vec<i64> = t.get("Color").unwrap_or(&[]).iter().filter_map(Val::int).collect();
                let alpha = c.get(3).copied().unwrap_or(255).clamp(0, 255) as u32;
                let time = ms(t.int("InTime").unwrap_or(3) + t.int("OutTime").unwrap_or(3));
                self.out.ops.push(Op::Flash { colour: colour(t), opacity: alpha * 256 / 255, ms: time.max(16) });
            }
            "SpriteQuake" => {
                // Shakes one picture (a background) by `:Param` 0, 1
                // (x, y) for `:Time` frames, dying down with `:Fade 1`, at
                // full size with 0; zeros stop it.
                let tex = t.int("SpriteNumber").or_else(|| t.int("Texture")).unwrap_or(-1);
                let Some(s) = self.shown.get(&format!("t{tex}")).cloned() else { return };
                let (ax, ay) = (sx(t.int_at("Param", 0).unwrap_or(0)), sy(t.int_at("Param", 1).unwrap_or(0)));
                let time = ms(t.int("Time").unwrap_or(0));
                let fades = t.int("Fade") == Some(1);
                if ax == 0 && ay == 0 {
                    self.out.ops.push(Op::SpriteMotion { layer: s.layer, keys: vec![], looped: false });
                    return;
                }
                // `:Time 0` with a size (once, S04_07A, after a `// wt`):
                // waits for the shake an earlier task started, which a
                // message in between has already outlasted.
                if time == 0 {
                    return;
                }
                self.out.ops.push(Op::SpriteMotion { layer: s.layer, keys: quake_keys(ax, ay, time, fades), looped: false });
                if t.int("Wait").unwrap_or(0) != 0 {
                    self.out.ops.push(Op::Wait { ms: time });
                }
            }
            "EffectQuake" => {
                let ax = sx(t.int_at("Param", 0).unwrap_or(30));
                let ay = sy(t.int_at("Param", 1).unwrap_or(30));
                let time = t.int("Time").unwrap_or(30);
                // scene_load case 99 (defaults from 0x8112a26c: `:Fade` 0,
                // `:Wait` 1): `:Loop 1` or a negative `:Time` shakes at full
                // size until EffectQuakeEnd, `:Fade` and `:Wait` unread;
                // else for `:Time` frames, dying down with `:Fade 1`, at
                // full size with 0, then stops (ours: `:Time 0` until the
                // end task).
                let (fade, wait) = (t.int("Fade"), t.int("Wait"));
                let looped = t.int("Loop") == Some(1) || time <= 0;
                let (end, end_ms) = match (looped, fade == Some(1)) {
                    (true, _) => ((ax, ay), 0),
                    (false, true) => ((0, 0), ms(time)),
                    (false, false) => ((ax, ay), ms(time)),
                };
                let shake = ScreenEffect::Shake { time_ms: ms(time).max(1), interval_ms: 16, path: false, speed: (2200, 2900), range: [(ax, ay), end], end_ms };
                self.out.ops.push(Op::ScreenEffect { slot: 1, effect: Some(shake) });
                if !looped && wait.unwrap_or(1) != 0 {
                    self.out.ops.push(Op::Wait { ms: ms(time) });
                }
            }
            "SpritePriority" | "CharacterPriority" => {
                let v: Vec<i64> = t.get("Texture").or_else(|| t.get("Number")).unwrap_or(&[]).iter().filter_map(Val::int).collect();
                let key = match v.as_slice() {
                    [c, _, _, _, _] => format!("c{c}"),
                    [tex] => self.shown.iter().find(|(_, s)| s.tex == *tex).map_or(format!("t{tex}"), |(k, _)| k.clone()),
                    _ => return,
                };
                self.reprioritize(&key, t.int("Priority").unwrap_or(5));
            }
            "EffectQuakeEnd" => self.out.ops.push(Op::ScreenEffect { slot: 1, effect: None }),
            "SpriteMove" => {
                let tex = t.int("SpriteNumber").or_else(|| t.int("Texture")).unwrap_or(-1);
                let Some((key, s)) = self.shown.iter().find(|(_, s)| s.tex == tex).map(|(k, s)| (k.clone(), s.clone())) else {
                    return;
                };
                let x = t.int_at("Position", 0).map_or(s.x, sx);
                let y = t.int_at("Position", 1).map_or(s.y, sy);
                let time = ms(t.int("Time").unwrap_or(0));
                // `:Scale` zooms the picture about its centre (a camera
                // zoom); without it the zoom stays.
                let zoom = t.float("Scale").map_or(s.zoom, |f| (f * 100.0).round() as u32);
                self.ease(t);
                // A zoomed picture moves with spzoom too, which redraws
                // where its zoomed picture was.
                if zoom != s.zoom || zoom != 100 {
                    self.out.ops.push(Op::SpriteZoom { layer: s.layer, x, y, from: s.zoom, to: zoom, ms: time });
                } else {
                    self.out.ops.push(Op::SpriteMove { layer: s.layer, x, y, ms: time });
                }
                self.shown.insert(key, Shown { x, y, zoom, ..s });
            }
            // `:Image 0,N`: CG.cpk's ECnn (27 is the title logo) over
            // everything, on the cover's layer, coming in through the
            // shutters; EyeCatchEnd puts back what was there (the black of
            // a FadeOut, or nothing) as the shutters open over `:Fade`.
            // `:Image 1,…` (a chapter card) fades in and out.
            "EyeCatch" => {
                let n = t.int_at("Image", 1).unwrap_or(-1);
                let image = match t.int_at("Image", 0) {
                    // CG ID n + 457, at least 458 (the eboot, 0x81048978).
                    Some(0) if n >= 0 => self.assets.cg((n as u32 + 457).max(458)),
                    // A chapter card: card, place, date.
                    Some(1) if n >= 0 => {
                        let (place, date) = (t.int_at("Image", 2).unwrap_or(-1), t.int_at("Image", 3).unwrap_or(-1));
                        u32::try_from(place)
                            .ok()
                            .and_then(|p| self.assets.chapter_card(n as u32, p, date as i32))
                            .map(|path| (path, SCREEN_W as u32, SCREEN_H as u32))
                    }
                    _ => None,
                };
                match image {
                    Some((path, w, h)) => {
                        let (x, y) = ((SCREEN_W - w as i32) / 2, (SCREEN_H - h as i32) / 2);
                        let change = SpriteChange { layer: COVER_LAYER, sprite: Some(SpritePlacement { image: path, x, y }), keep_filters: false };
                        self.out.ops.push(Op::HideTextWindow);
                        let closing = if t.int_at("Image", 0) == Some(0) { self.assets.shutters(n as u32, true, SHUTTER_FRAMES) } else { None };
                        self.shutters = None;
                        match closing {
                            Some(image) => {
                                self.out.ops.push(Op::Animation(Some(Animation { image, frame_ms: vec![SHUTTER_MS / SHUTTER_FRAMES; SHUTTER_FRAMES as usize], looped: false, opacity: 256 })));
                                self.out.ops.push(Op::Wait { ms: SHUTTER_MS });
                                self.out.ops.push(Op::Sprites { changes: vec![change], fade_ms: 0 });
                                self.out.ops.push(Op::Animation(None));
                                self.shutters = Some(n as u32);
                            }
                            None => self.out.ops.push(Op::Sprites { changes: vec![change], fade_ms: 500 }),
                        }
                        self.eye_catch = true;
                    }
                    // 0,-1: no picture.
                    None if t.int_at("Image", 0) == Some(0) && n < 0 => {}
                    None => self.note(format!("EyeCatch {:?}", t.get("Image").unwrap_or(&[]))),
                }
            }
            "EyeCatchEnd" => {
                if std::mem::take(&mut self.eye_catch) {
                    let sprite = self.cover.map(|rgb| SpritePlacement { image: self.assets.colour(rgb), x: 0, y: 0 });
                    let change = SpriteChange { layer: COVER_LAYER, sprite, keep_filters: false };
                    let fade_ms = ms(t.int("Fade").unwrap_or(30));
                    match self.shutters.take().and_then(|n| self.assets.shutters(n, false, SHUTTER_FRAMES)) {
                        // The opening shutters, first the whole picture,
                        // over what comes back.
                        Some(image) if fade_ms > 0 => {
                            let frame_ms = vec![fade_ms.div_ceil(SHUTTER_FRAMES); SHUTTER_FRAMES as usize];
                            self.out.ops.push(Op::Animation(Some(Animation { image, frame_ms, looped: false, opacity: 256 })));
                            self.out.ops.push(Op::Sprites { changes: vec![change], fade_ms: 0 });
                            self.out.ops.push(Op::Wait { ms: fade_ms });
                            self.out.ops.push(Op::Animation(None));
                        }
                        _ => self.out.ops.push(Op::Sprites { changes: vec![change], fade_ms }),
                    }
                }
            }
            "MoviePlay" => match t.int("Movie").and_then(|n| self.assets.movie(n as u32)) {
                Some(file) => self.out.ops.push(Op::Movie { file, skippable: t.int("Skip").unwrap_or(-1) != 0 }),
                None => self.unsupported(scene, i, format!("MoviePlay {:?}", t.int("Movie"))),
            },
            "EffectStart" | "EffectEnd" => {
                let n = t.int("EffectNumber").unwrap_or(-1);
                let start = ty == "EffectStart";
                let fade_ms = ms(t.int("Fade").unwrap_or(0));
                match n {
                    // ノイズ: static, :Param density 0-255.
                    53 => {
                        let noise = start.then(|| {
                            let density = t.int_at("Param", 0).unwrap_or(128).clamp(0, 255) as u32;
                            Noise { image: self.assets.noise(4), frames: 4, frame_ms: 50, opacity: density * 256 / 255 }
                        });
                        self.out.ops.push(Op::Noise(noise));
                    }
                    // ブラー: blur, :Param radius (script pixels).
                    42 => {
                        let filter = start.then(|| {
                            let r = sx(t.int_at("Param", 0).unwrap_or(4)).max(1);
                            let c = Operand::Const;
                            SpriteFilter { kind: c(FILTER_BLUR), blend: c(FILTER_BLEND_COPY), strength: c(256), params: [c(r), c(r), c(0)] }
                        });
                        self.out.ops.push(Op::ScreenFilter { slot: 1, filter, fade_ms });
                    }
                    // 画面キャプチャースプライト (the eboot's 0x81040714): a
                    // copy of the screen, frozen, zoomed onto a point at an
                    // impact. :Param x, y (where its centre goes), scale,
                    // frames (the eboot steps 2/frames a frame: it takes
                    // half), ease type 0-3, power; it fades in over :Fade
                    // and out over EffectEnd's :Fade.
                    57 => {
                        if start {
                            let p = |k: usize, d: f64| t.at("Param", k).and_then(Val::float).unwrap_or(d);
                            let power = p(5, 1.0);
                            let effect = ScreenEffect::Zoom {
                                interval_ms: 16,
                                zoom: (p(2, 1.0).max(0.01) * 1000.0).round() as i32,
                                centre: (sx(p(0, 640.0) as i64), sy(p(1, 360.0) as i64)),
                                zoom_ms: ms(p(3, 1.0).max(1.0) as i64) / 2,
                                fade_ms,
                                ease: p(4, 0.0).clamp(0.0, 3.0) as u8,
                                power: if power == 0.0 { 1000 } else { (power * 1000.0).round() as i32 },
                            };
                            self.out.ops.push(Op::ScreenEffect { slot: 2, effect: Some(effect) });
                        } else if self.staging {
                            self.out.ops.push(Op::ScreenEffect { slot: 2, effect: None });
                        } else {
                            self.out.ops.push(Op::BeginTransition { effect: "EffectEnd".into(), fade_ms, mask: None, style: TransitionStyle::Fade });
                            self.out.ops.push(Op::ScreenEffect { slot: 2, effect: None });
                            self.out.ops.push(Op::EndTransition);
                        }
                    }
                    // A wave (the eboot's shader 0x8115f304, over the
                    // starry monologue background): each row's picture
                    // moves along the diagonal by 1.5% of the screen width
                    // on each axis × sin(2π(600y/(height × :Param 0) +
                    // 2t/:Param 1)), t in ms, growing in over :Fade. A
                    // negative :Param 0 waves the columns (unused).
                    40 => {
                        let effect = start.then(|| {
                            let p0 = t.int_at("Param", 0).unwrap_or(400).max(1) as u32;
                            let p1 = t.int_at("Param", 1).unwrap_or(3000).max(1);
                            let amp = (SCREEN_W as f64 * 0.015 * std::f64::consts::SQRT_2).round() as i32;
                            ScreenEffect::Ripple {
                                map: self.assets.wave_map(60_000 / p0),
                                time_ms: fade_ms,
                                interval_ms: 30,
                                opacity: (256, 256),
                                amplitude: (if fade_ms > 0 { 0 } else { amp }, amp),
                                angle: 450,
                                angle_step: 0,
                                phase_step: (60_000 / p1) as i32,
                                multiply: false,
                            }
                        });
                        self.out.ops.push(Op::ScreenEffect { slot: 3, effect });
                    }
                    51 | 52 | 54 => {
                        let kind = match n {
                            51 => Particles::Snow,
                            52 => Particles::LightBalls,
                            _ => Particles::Confetti,
                        };
                        let count = t.int_at("Param", 0).unwrap_or(60).max(1) as u32;
                        let animation = if start {
                            self.assets.particles(kind, count).map(|image| Animation {
                                image,
                                frame_ms: vec![125; super::assets::PARTICLE_FRAMES as usize],
                                looped: true,
                                opacity: 256,
                            })
                        } else {
                            None
                        };
                        if start && animation.is_none() {
                            self.note(format!("{ty} {n}: no particle picture"));
                        } else {
                            self.out.ops.push(Op::Animation(animation));
                        }
                    }
                    _ => self.note(format!("{ty} {n}")),
                }
            }
            "Achievement" => {}
            "SetSceneName" => {
                if let Some(title) = t.str("SceneName") {
                    self.out.ops.push(Op::Chapter { title: title.to_string() });
                }
            }
            // CharacterStop stops a looping action, which isn't played.
            "BacklogMessage" | "SceneJumpDialogEnd" | "MovieStop" | "CharacterStop" | "CharacterWait" | "SpriteMoveEnd" => {}
            _ => self.unsupported(scene, i, ty.to_string()),
        }
    }

    /// `SceneJumpDialog`: a scene read before may be skipped. Unread, it
    /// plays (from `:UnreadTag` if given, else `:NextTag`) and counts as
    /// read; read, its summary (`:Text`) is shown with the choice to read
    /// it again (`:NextTag`) or skip it (`:JumpNextTag`).
    fn scene_jump(&mut self, scene: &str, t: &Block) {
        let (Some(play), Some(skip)) = (t.str("NextTag"), t.str("JumpNextTag")) else { return };
        let Some(n) = self.game.scenes.keys().position(|k| k == scene) else { return };
        let read = Var { global: true, index: SCENE_READ_BASE + n as u16 };
        let unread = t.str("UnreadTag").unwrap_or(play);
        let offer = format!("{}_offer{}", scene_label(scene), self.choices);
        self.out.ops.push(Op::If { var: read, cmp: Compare::Ne, value: Operand::Const(0), label: offer.clone() });
        self.out.ops.push(Op::Arith { var: read, op: ArithOp::Set, value: Operand::Const(1) });
        self.out.ops.push(Op::Jump(tag_label(scene, unread)));
        self.out.ops.push(Op::Label(offer.clone()));
        // シーンスキップ機能 OFF: a read scene just plays.
        self.out.ops.push(Op::IfSwitch { switch: Switch::SceneSkipDialog, on: false, label: tag_label(scene, play) });
        let summary = t.str("Text").unwrap_or("").trim_end().to_string();
        let ints = |key: &str| t.get(key).unwrap_or(&[]).iter().filter_map(Val::int).collect::<Vec<_>>();
        if let Some((pictures, [yes, no])) = self.assets.skip_dialog(&ints("BG"), &ints("CharaIcon"), &summary, &offer) {
            // このシーンをスキップしますか？ Yes skips.
            let buttons = vec![(yes, tag_label(scene, skip)), (no, tag_label(scene, play))];
            self.out.ops.push(Op::ButtonScreen(Box::new(ButtonScreen { pictures, buttons, fade_ms: SKIP_DIALOG_FADE_MS })));
            return;
        }
        if !summary.trim().is_empty() {
            self.out.ops.push(Op::Text { speaker: None, text: summary, voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false });
        }
        let options = vec![("このシーンを読む".to_string(), tag_label(scene, play)), ("このシーンをスキップする".to_string(), tag_label(scene, skip))];
        self.choices += 1;
        self.out.ops.push(Op::Choice { options, style: None });
    }

    /// START's story menu (the original's labels in `title_bg.png`), as
    /// the eboot has it (0x8105e69c, 0x81083086, 0x81084c26): START opens
    /// it once any of the first six global flags is set (明日香's, 京's,
    /// 亜衣's or 鳴's ending, part 1 or 2 cleared), otherwise it starts the
    /// story. 美咲's, 明日香's and 京's after-stories (adv.txt's start
    /// scenes 1-3) are then open; 狩男's (4) once part 1, 2 or 3 is
    /// cleared (the Vita shows ？？？？？ until then).
    fn story_menu(&mut self) {
        const SHOWN: [&str; 6] = ["asuka_ed", "miyako_ed", "ai_ed", "naru_ed", "one_clear", "two_clear"];
        const STORIES: [(&str, &str, &[&str]); 4] = [
            ("ある日の林田美咲", "P03_01A", &[]),
            ("ある日の渡来明日香", "P01_01A", &[]),
            ("ある日の山科京", "P02_01A", &[]),
            ("ある日の軽部狩男", "P04_01A", &["one_clear", "two_clear", "three_clear"]),
        ];
        let menu = "mg_story_menu".to_string();
        let start = scene_label(&self.game.start);
        for flag in SHOWN {
            if let Some(var) = self.game.flag(flag, true) {
                self.out.ops.push(Op::If { var, cmp: Compare::Ne, value: Operand::Const(0), label: menu.clone() });
            }
        }
        self.out.ops.push(Op::Jump(start.clone()));
        self.out.ops.push(Op::Label(menu.clone()));
        if self.story_menu_pictures(&start, &STORIES) {
            return;
        }
        let mut options = vec![("俺たちに翼はない　始めから".to_string(), start)];
        let mut gates = Vec::new();
        for (title, scene, flags) in STORIES {
            if flags.is_empty() {
                options.push((title.to_string(), scene_label(scene)));
                continue;
            }
            let gate = format!("mg_story_{}", scene.to_ascii_lowercase());
            options.push((title.to_string(), gate.clone()));
            gates.push((gate, scene, flags));
        }
        self.out.ops.push(Op::Choice { options, style: None });
        for (gate, scene, flags) in gates {
            self.out.ops.push(Op::Label(gate));
            for flag in flags {
                if let Some(var) = self.game.flag(flag, true) {
                    self.out.ops.push(Op::If { var, cmp: Compare::Ne, value: Operand::Const(0), label: scene_label(scene) });
                }
            }
            let text = "　このお話は、まだ読むことができません。".to_string();
            self.out.ops.push(Op::Text { speaker: None, text, voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false });
            self.out.ops.push(Op::Jump(menu.clone()));
        }
    }

    /// The story menu as the Vita draws it ([`Assets::story_menu`]): one
    /// screen with 狩男's after-story open, one with it locked, chosen by
    /// its flags. False without the pictures.
    fn story_menu_pictures(&mut self, start: &str, stories: &[(&str, &str, &[&str]); 4]) -> bool {
        let (Some(open), Some(locked)) = (self.assets.story_menu(false), self.assets.story_menu(true)) else { return false };
        let open_label = "mg_story_menu_open".to_string();
        for flag in stories[3].2 {
            if let Some(var) = self.game.flag(flag, true) {
                self.out.ops.push(Op::If { var, cmp: Compare::Ne, value: Operand::Const(0), label: open_label.clone() });
            }
        }
        let targets: Vec<String> = std::iter::once(start.to_string()).chain(stories.iter().map(|s| scene_label(s.1))).collect();
        for (label, (pictures, buttons)) in [(None, locked), (Some(open_label), open)] {
            if let Some(label) = label {
                self.out.ops.push(Op::Label(label));
            }
            let buttons = buttons.into_iter().zip(targets.iter().cloned()).collect();
            self.out.ops.push(Op::ButtonScreen(Box::new(ButtonScreen { pictures, buttons, fade_ms: STORY_MENU_FADE_MS })));
        }
        true
    }

    /// A move's curve: `:Position x,y,type,power` eases with the eboot's
    /// ease_value (0x8103cc18; 0 straight, 1 in, 2 out, 3 in and out),
    /// which raises t² to the power.
    fn ease(&mut self, t: &Block) {
        let v = t.get("Position").unwrap_or(&[]);
        let ease = v.get(2).and_then(Val::float).unwrap_or(0.0) as u8;
        if (1..=3).contains(&ease) {
            let power = v.get(3).and_then(Val::float).filter(|p| *p > 0.0).unwrap_or(1.0);
            let exponent = ((power * 2.0).round() as u32).max(1);
            self.out.ops.push(Op::MoveEase { ease, exponent });
        }
    }

    /// `"+", 1` → (Add, 1).
    fn arith(&self, action: &[Val]) -> Option<(ArithOp, i32)> {
        let op = match action.first()?.str()? {
            "=" => ArithOp::Set,
            "+" => ArithOp::Add,
            "-" => ArithOp::Sub,
            _ => return None,
        };
        Some((op, action.get(1)?.int()? as i32))
    }

    /// `MessageSwitch_Tag`: the question (`:Text`), then the answers; an
    /// answer with a `:Message<N>Flag` changes the flag before going on.
    fn choice(&mut self, scene: &str, t: &Block) {
        // The question is usually the last message again, still on screen
        // in the original: not shown twice.
        let last = self.out.ops.iter().rev().find_map(|op| match op {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        });
        if let Some(text) = t.str("Text").filter(|q| Some(*q) != last) {
            self.out.ops.push(Op::Text {
                speaker: None,
                text: text.to_string(),
                voice: None,
                links: vec![],
                ruby: vec![],
                styles: vec![],
                hang: None,
                continues: false,
            });
        }
        let n = t.int("AnswerMax").unwrap_or(0);
        let mut options = Vec::new();
        let mut stubs = Vec::new();
        for k in 0..n {
            let Some(answer) = t.get(&format!("AnswerMessageNumber{k}")) else { continue };
            let (Some(tag), Some(text)) = (answer.first().and_then(Val::str), answer.get(1).and_then(Val::str)) else { continue };
            let target = tag_label(scene, tag);
            match t.get(&format!("Message{k}Flag")) {
                Some(flag) => {
                    let stub = format!("{}_choice{}_{k}", scene_label(scene), self.choices);
                    let var = flag.first().and_then(Val::str).and_then(|f| self.game.flag(f, false));
                    let op = self.arith(flag.get(1..).unwrap_or(&[]));
                    stubs.push((stub.clone(), var.zip(op), target));
                    options.push((text.to_string(), stub));
                }
                None => options.push((text.to_string(), target)),
            }
        }
        self.choices += 1;
        self.out.ops.push(Op::Choice { options, style: None });
        for (stub, change, target) in stubs {
            self.out.ops.push(Op::Label(stub));
            if let Some((var, (op, v))) = change {
                self.out.ops.push(Op::Arith { var, op, value: Operand::Const(v) });
            }
            self.out.ops.push(Op::Jump(target));
        }
    }
}

/// A shake dying down over `time` ms: offsets of up to (ax, ay) pixels,
/// every 33 ms or more, alternating sides, ending at the picture's place.
/// A shake's keys over `time` ms, dying down when `fades`.
fn quake_keys(ax: i32, ay: i32, time: u32, fades: bool) -> Vec<(u32, i32, i32)> {
    // spmotion takes up to 32 keys.
    let step = (time / 31).max(33);
    let n = (time / step).max(1);
    let signs = [(1, 1), (-1, -1), (1, -1), (-1, 1)];
    let mut keys: Vec<(u32, i32, i32)> = (0..n)
        .map(|k| {
            let left = if fades { (n - k) as i32 } else { n as i32 };
            let (sx_, sy_) = signs[k as usize % 4];
            (k * step, sx_ * ax * left / n as i32, sy_ * ay * left / n as i32)
        })
        .collect();
    keys.push((time, 0, 0));
    keys
}

fn colour(t: &Block) -> u32 {
    let c: Vec<i64> = t.get("Color").unwrap_or(&[]).iter().filter_map(Val::int).collect();
    match c.as_slice() {
        [r, g, b, ..] => ((*r as u32 & 255) << 16) | ((*g as u32 & 255) << 8) | (*b as u32 & 255),
        _ => 0,
    }
}

/// An `InOut_Act`'s look, from the eboot's effect 25 (`:Type` picks its
/// class; its shaders read with Vita3K's recompiler). `:Param` 0 is a
/// direction 0-3 (right, down, left, up) unless said otherwise:
/// - 0: the old screen squeezes to that edge while the new one grows from
///   the opposite edge (a push here);
/// - 7: the old screen pushed out that way by the new one;
/// - 5: the old screen to white (0) or the new one from white (1), the
///   same through black (2, 3);
/// - 2: a mosaic, its blocks growing to `:Param` 1 pixels (of 720 rows)
///   halfway and back, while the old screen fades into the new;
/// - 3: a card turning over (perspective), the old screen in front, the
///   new behind, about the vertical axis (`:Param` 0 below 2) or the
///   horizontal one (a turn of the new screen growing from a line here);
/// - 1: a fade with the rows waving sideways (`:Param` 1 the size, 2 the
///   waves; their scale is a guess: the shader's constants are uniforms);
/// - 6: a fade blurred sideways, the most halfway, by `:Param` 1 pixels
///   of 1280 either side, `:Param` 0 = 1 only the new screen;
/// - 4: not looked at (2 uses).
fn act_style(t: &Block) -> TransitionStyle {
    let direction = t.int_at("Param", 0).unwrap_or(0).clamp(0, 3);
    let (dx, dy) = [(1, 0), (0, 1), (-1, 0), (0, -1)][direction as usize];
    match t.int("Type") {
        Some(0 | 7) => TransitionStyle::Slide { moving: SlideLayer::Both, dx, dy },
        Some(5) if direction < 2 => TransitionStyle::ThroughWhite,
        Some(2) => TransitionStyle::Mosaic,
        Some(3) => TransitionStyle::Flip { horizontal: direction < 2, grow: true },
        Some(1) => {
            let param = |i: usize| -> f64 { t.at("Param", i).and_then(Val::float).unwrap_or(0.0) };
            TransitionStyle::Wave { direction: direction as u8, amplitude: (param(1) * 100.0).round() as u32, waves: (param(2) * 10.0).round() as u32 }
        }
        Some(6) => TransitionStyle::Blur {
            radius: t.int_at("Param", 1).map_or(0, |r| (r.max(0) as f64 * 0.75).round() as u32),
            new_only: t.int_at("Param", 0) == Some(1),
        },
        _ => TransitionStyle::Fade,
    }
}

/// Compiles `scenes` (all of them if empty), each at its label, after a
/// jump to the game's first scene.
pub fn compile(game: &Game, scenes: &[String], assets: &mut dyn Assets) -> Compiled {
    let mut l = Lowering { game, assets, out: Compiled::default(), shown: BTreeMap::new(), next_slot: HashMap::new(), fading: false, rule: None, scale: 100, staging: false, choices: 0, cover: None, eye_catch: false, shutters: None, inherited: false, opening: false, messages: 0 };
    let names: Vec<&String> = if scenes.is_empty() { game.scenes.keys().collect() } else { scenes.iter().collect() };
    let first = names.first().map(|s| s.as_str()).unwrap_or(&game.start);
    let first = if scenes.is_empty() { game.start.as_str() } else { first };
    if scenes.is_empty() {
        l.story_menu();
    }
    l.out.ops.push(Op::Jump(scene_label(first)));
    for name in names {
        let Some(scene) = game.scenes.get(name) else { continue };
        l.out.ops.push(Op::Label(scene_label(name)));
        if let Some(title) = &scene.title {
            l.out.ops.push(Op::Chapter { title: title.clone() });
        }
        l.shown.clear();
        l.staging = false;
        l.inherited = true;
        l.opening = true;
        // A scene whose first fade is a FadeIn starts under the cover.
        l.cover = scene.tasks.iter().find(|t| matches!(t.task_type(), Some("FadeIn" | "FadeOut"))).filter(|t| t.task_type() == Some("FadeIn")).map(colour);
        // Each task is a call; its unread arguments are known only once
        // every task has run (a later one may read an earlier one's).
        let mut calls = Vec::with_capacity(scene.tasks.len());
        for i in 0..scene.tasks.len() {
            l.out.metrics.begin(scene.tasks[i].task_type().unwrap_or("?"));
            l.task(name, &scene.tasks, i);
            calls.extend(l.out.metrics.pop());
        }
        for (t, (kind, mut reasons)) in scene.tasks.iter().zip(calls) {
            let mut unread = BTreeMap::new();
            ignored(t.task_type().unwrap_or("?"), "", t, &mut unread);
            for (what, n) in unread {
                reasons.push(what.clone());
                *l.out.notes.entry(what).or_default() += n;
            }
            l.out.metrics.call(&kind, reasons);
        }
        if l.staging {
            l.out.ops.push(Op::EndTransition);
            l.staging = false;
        }
        // A scene without NextScene or End falls through: end the game.
        if !matches!(l.out.ops.last(), Some(Op::Jump(_) | Op::ReturnToTitle)) {
            l.out.ops.push(Op::ReturnToTitle);
        }
    }
    l.out
}

/// Arguments known not to matter: the start of their note, with
/// " = <values>" when only that value is what we do, and why.
const NOT_NEEDED: &[(&str, &str)] = &[
    ("FadeIn :Color", "always the colour of the FadeOut before it (579 of 579)"),
    ("CharacterDelete :FadeOut", "with a :Fade always 1 (fade), which :Fade times"),
    ("CharacterActionEnd :Character argument", "scene_load (0x8102840a) case 0x21 reads only its first value"),
    ("CharacterDelete [Chara] :FadeOut = 1", "fade, as the task does"),
    ("CharacterMove :Wait = 1", "wait, as every move of ours does"),
    ("SpriteMove :Wait = 1", "the same"),
    ("SePlay :Wait = 0", "don't wait, as ours don't"),
    ("SeStop :Wait = 0", "the same"),
    ("SePlay :Div = 1", "a 1-frame fade-in: none, as ours"),
    ("EffectQuakeEnd :EndWait = 0", "stop at once, as ours does"),
    ("CharacterOpenBS :Fade = 0", "at once, as ours opens"),
    ("MoviePlay :Loop = 0", "play once, as ours"),
    ("CharacterMove [Chara] :Position argument 2 = 0", "straight (with power 0), as our cross-fading moves are"),
    ("CharacterMove [Chara] :Position argument 3 = 0", "the same"),
    ("CharacterOpen [Chara] :Center argument 0 = 0", "the centre across, where we place it"),
    // Traced in the eboot: scene_load (0x8102840a) reads the task,
    // 0x8100da30 starts it, task_run (0x81021070) runs it.
    ("EffectQuake :Window = 0", "the text window stays still (0x8100da30 case 99 shakes it only with 1), as under our screenshake"),
    ("EffectQuakeEnd :Window = 0", "the window's shake isn't running: every EffectQuake has :Window 0"),
    ("EffectQuakeEnd :Fade = 0", "stops at once, as ours (task_run case 100)"),
    ("EffectQuakeEnd :EndWait", "scene_load case 100 clears the flag right after setting it: it never waits"),
    ("EffectQuake :Param argument 2 = 1", "a 1-pixel swing (0x81008a3c: argument 2, 3 its size, 4, 5 its half-waves over :Time)"),
    ("EffectQuake :Param argument 3 = 1", "the same"),
    ("EffectQuake :Param argument 4 = 1", "the same"),
    ("EffectQuake :Param argument 5 = 1", "the same"),
    ("FadeOut :Type = 0", "a plain fade, as ours: the type goes to the transition's rule table (0x81049292), 0 none, as InOut's"),
    ("FadeIn :Type = 0", "the same"),
    ("MessageSwitch_Tag :Frame = 0", "the choice box's first look (0x810160e2 moves its cell down :Frame rows)"),
    ("Sprite :Time = 0", "at once, as ours (scene_load case 9 takes :Time over :FadeIn)"),
    ("SpriteBG :Time", "used only with :FadeIn 1, as ours (scene_load case 10's FadeIn flag, +0x95, goes with it in task_run)"),
    ("SpriteDelete :Time", "read only with :FadeOut 1, as ours (scene_load case 0x25)"),
    ("SpriteDeleteBG :Time = 0", "at once, as ours"),
    ("SpriteDeleteBG :FadeIn", "not read by scene_load case 0x26"),
    ("SpriteDeleteBG :Position", "the same"),
    ("SpriteDeleteBG :ReleaseTexture", "texture memory"),
    ("SpriteMoveEnd :Sprite", "every SpriteMove is waited on, so none is left running"),
    ("MoviePlay :EndWait = 1", "wait for the end, as our movies always do"),
    // The whole key is read only from :NumberDirect (never used); from
    // :Number the key stays -1 and the task takes whatever picture is
    // shown under the number, as ours.
    ("CharacterDelete [Chara] :Number argument", "scene_load case 0x1c reads only the character from :Number"),
    ("CharacterMove [Chara] :Number argument", "the same (case 0x1e)"),
    ("CharacterAction [Chara] :Number argument", "the same (case 0x20)"),
    ("Achievement :ID", "trophies: nothing to show"),
    // Blocks and keys the eboot never asks for (cfg_find of
    // "TaskList/Task%d/Chara%d/<key>").
    ("CharacterOpen []", "a [CharaN] block without its name"),
    ("CharacterAction [Char]", "a misspelt [CharaN] block"),
    ("SpriteBG :Priorit", "a typo"),
    ("CharacterOpen [Chara] :Prioriry", "a typo"),
    ("CharacterDelete :fFade", "a typo"),
];

/// Notes each argument of `b` (and its sub-blocks) the converter never
/// read: "<Task> :<Key> ignored", or "… argument <i> ignored" for one
/// value of several.
fn ignored(task: &str, at: &str, b: &Block, notes: &mut BTreeMap<String, usize>) {
    for (key, i, vals) in b.unread() {
        let value = vals.iter().map(|v| v.float().map_or_else(|| v.str().unwrap_or("").to_string(), |f| f.to_string())).collect::<Vec<_>>().join(",");
        let what = match i {
            Some(i) => format!("{task}{at} :{key} argument {i} ignored"),
            None => format!("{task}{at} :{key} ignored"),
        };
        let covered = |row: &str| match row.split_once(" = ") {
            Some((start, v)) => what.starts_with(&format!("{start} ")) && v == value,
            None => what.starts_with(&format!("{row} ")),
        };
        if !NOT_NEEDED.iter().any(|(row, _)| covered(row)) {
            *notes.entry(what).or_default() += 1;
        }
    }
    for c in &b.children {
        ignored(task, &format!(" [{}]", c.name.trim_end_matches(|c: char| c.is_ascii_digit())), c, notes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_not_read_are_noted() {
        let src = "[Task1]\n{\n:Tasktype \"SpriteMove\"\n:Position 320,360,2,1\n:Curve 3\n:Wait 0\n:Roll 0\n[Chara0]\n{\n:Number 8,0,1,2,0\n}\n}\n";
        let blocks = parse(src).unwrap();
        let t = &blocks[0];
        t.task_type();
        t.int_at("Position", 0);
        t.int_at("Position", 1);
        t.at("Position", 3);
        let mut notes = BTreeMap::new();
        ignored("SpriteMove", "", t, &mut notes);
        // `:Wait 0` is noted (only `= 1` is covered); Chara0's Number is never read.
        let want = [
            "SpriteMove :Curve ignored",
            "SpriteMove :Position argument 2 ignored",
            "SpriteMove :Roll ignored",
            "SpriteMove :Wait ignored",
            "SpriteMove [Chara] :Number ignored",
        ];
        assert_eq!(notes.keys().map(String::as_str).collect::<Vec<_>>(), want);
        // `SpriteMove :Wait = 1` is covered.
        let blocks = parse("[Task2]\n{\n:Tasktype \"SpriteMove\"\n:Wait 1\n}\n").unwrap();
        blocks[0].task_type();
        let mut notes = BTreeMap::new();
        ignored("SpriteMove", "", &blocks[0], &mut notes);
        assert!(notes.is_empty());
    }

    #[test]
    fn action_tracks_are_sampled_at_every_key() {
        // Y is keyed at frames 0 and 20 only, Roll at 10, Alpha at 20.
        let src = "[Action]\n{\n\t[Action7]\n\t{\n\t\t:Frame\t30\n\t\t:Loop\t0\n\t\t:EndSet\t1\n\
            \t\t[Frame0]\n\t\t{\n\t\t\t:X\t0\n\t\t\t:Y\t0\n\t\t}\n\
            \t\t[Frame10]\n\t\t{\n\t\t\t:X\t100\n\t\t\t:Roll\t0.5,2,1\n\t\t}\n\
            \t\t[Frame20]\n\t\t{\n\t\t\t:X\t200\n\t\t\t:Y\t-40\n\t\t\t:Alpha\t0\n\t\t}\n\t}\n}\n";
        let blocks = parse(src).unwrap();
        let a = Action::parse(&blocks[0].children[0]);
        assert!(a.end_set && !a.looped);
        let k: Vec<_> = a.keys.iter().map(|k| (k.frame, k.x, k.y, k.roll, k.alpha)).collect();
        // A track holds its first key before it and its last after it.
        assert_eq!(k, vec![(0, 0.0, 0.0, 0.5, 0.0), (10, 100.0, -20.0, 0.5, 0.0), (20, 200.0, -40.0, 0.5, 0.0)]);
        assert!(!a.keys[0].plain());
    }
}
