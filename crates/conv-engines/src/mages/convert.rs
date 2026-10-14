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
    Animation, ArithOp, ButtonScreen, Compare, FILTER_BLEND_COPY, FILTER_BLUR, Noise, Op, Operand, ScreenEffect, SlideLayer, SourceLoc, SpriteChange, SpriteFilter,
    Placed, SpritePlacement, TextWindow, TransitionStyle, Var, Voice,
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
    /// The shutters of eye catch `ECnn` (`n`): `frames` screens side by
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
}

/// A character (`chrNNN.txt`): its pictures by pose, dress, face, size.
#[derive(Debug, Clone, Default)]
pub struct Character {
    pub offset_y: Vec<i64>,
    pub sprites: HashMap<[i64; 4], u32>,
}

/// A character action (`chract`): keyframes (frame, x, y offsets in
/// script units) and whether it repeats until `CharacterActionEnd`.
#[derive(Debug, Clone, Default)]
pub struct Action {
    pub keys: Vec<(i64, i64, i64)>,
    pub looped: bool,
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
                        let mut keys: Vec<(i64, i64, i64)> = a
                            .children
                            .iter()
                            .filter_map(|f| {
                                let frame = f.name.strip_prefix("Frame")?.parse().ok()?;
                                Some((frame, f.int("X").unwrap_or(0), f.int("Y").unwrap_or(0)))
                            })
                            .collect();
                        keys.sort_unstable();
                        g.actions.insert(n, Action { keys, looped: a.int("Loop").unwrap_or(0) != 0 });
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
}

struct Lowering<'a> {
    game: &'a Game,
    assets: &'a mut dyn Assets,
    out: Compiled,
    /// Pictures by key: `c<number>` for characters, `t<texture>`, `bg`.
    shown: HashMap<String, Shown>,
    next_slot: HashMap<u32, u32>,
    /// A transition is open (staged changes wait for InOut).
    staging: bool,
    choices: usize,
    /// The colour a FadeOut covers the picture with, until FadeIn.
    cover: Option<u32>,
    /// An eye catch is on the cover's layer.
    eye_catch: bool,
    /// It is `ECnn` (n), which leaves through the shutters.
    shutters: Option<u32>,
}

impl Lowering<'_> {
    fn note(&mut self, what: impl Into<String>) {
        *self.out.notes.entry(what.into()).or_default() += 1;
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
        let n = self.next_slot.entry(base).or_insert(PRIORITY_STEP - 2);
        let layer = base + *n;
        *n = if *n == 0 { PRIORITY_STEP - 2 } else { *n - 1 };
        layer
    }

    /// Shows `image` centred at script position (x, y) on `key`'s layer.
    #[allow(clippy::too_many_arguments)]
    fn show(&mut self, key: &str, tex: i64, image: (String, u32, u32), x: i64, y: i64, priority: i64, fade_ms: u32) {
        let layer = self.layer(key, priority);
        let (path, w, h) = image;
        let (w, h) = (w as i32, h as i32);
        let (cx, cy) = (sx(x), sy(y));
        let mut changes = Vec::new();
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
        self.shown.insert(key.to_string(), Shown { layer, tex, x: cx, y: cy, image: path, w, h, base_y: cy });
        self.out.ops.push(Op::Sprites { changes, fade_ms: if self.staging { 0 } else { fade_ms } });
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
        self.shown.insert(key.to_string(), Shown { layer, ..s });
        self.out.ops.push(Op::Sprites { changes, fade_ms: 0 });
    }

    fn hide(&mut self, key: &str, fade_ms: u32) {
        if let Some(old) = self.shown.remove(key) {
            let change = SpriteChange { layer: old.layer, sprite: None, keep_filters: false };
            self.out.ops.push(Op::Sprites { changes: vec![change], fade_ms: if self.staging { 0 } else { fade_ms } });
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
                    Some("InOut") => (time, t.int("Type").and_then(|n| self.assets.rule(n as u32)), TransitionStyle::Fade),
                    _ => (time, None, act_style(t)),
                }
            }
            None => (0, None, TransitionStyle::Fade),
        };
        self.out.ops.push(Op::BeginTransition { effect: "InOut".into(), fade_ms, mask, style });
        self.staging = true;
    }

    fn character_image(&mut self, number: &[Val]) -> Option<(String, u32, u32, i64)> {
        let v: Vec<i64> = number.iter().filter_map(Val::int).collect();
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
        match ty {
            "Message" => {
                let text = t.str("Message").unwrap_or("").to_string();
                let speaker = t.int("Name").and_then(|n| self.game.names.get(&n).cloned());
                let voice = t.int("Voice").and_then(|id| {
                    let file = self.assets.voice(id as u32)?;
                    Some(Voice { file, character: t.int("Name").unwrap_or(0) as u32 })
                });
                if self.staging {
                    self.out.ops.push(Op::EndTransition);
                    self.staging = false;
                }
                self.out.ops.push(Op::Text { speaker, text, voice, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false });
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
                let key = if ty == "SpriteBG" { "bg".to_string() } else { format!("t{tex}") };
                let (x, y) = (t.int_at("Position", 0).unwrap_or(640), t.int_at("Position", 1).unwrap_or(360));
                let prio = t.int("Priority").unwrap_or(if ty == "SpriteBG" { 10 } else { 5 });
                self.show(&key, tex, img, x, y, prio, fade(t));
                // A gallery CG: seen from now on (CG numbers are 1-based).
                if let Some(flag) = t.int("CGFlag").filter(|f| *f >= 0) {
                    self.out.ops.push(Op::CgSeen { number: flag as u32 + 1 });
                }
            }
            "SpriteDelete" | "SpriteDeleteBG" => {
                let tex = t.int("SpriteNumber").or_else(|| t.int("Texture")).unwrap_or(-1);
                let key = if ty == "SpriteDeleteBG" && self.shown.contains_key("bg") { "bg".to_string() } else { format!("t{tex}") };
                let f = if t.int("FadeOut").unwrap_or(0) == 0 { 0 } else { ms(t.int("Time").unwrap_or(15)) };
                self.hide(&key, f);
            }
            "CharacterOpen" | "CharacterOpenBS" => {
                if ty == "CharacterOpenBS" {
                    self.stage(tasks, i);
                }
                for c in t.children.iter().filter(|c| c.name.starts_with("Chara")) {
                    let Some(number) = c.get("Number") else { continue };
                    let Some((path, w, h, offset)) = self.character_image(number) else {
                        self.note(format!("{ty}: unknown character picture {number:?}"));
                        continue;
                    };
                    let key = format!("c{}", number[0].int().unwrap_or(-1));
                    let x = c.int_at("Position", 0).unwrap_or(640);
                    // y 0 is the default height: centred, raised by the
                    // character's offset.
                    let y = 360 + c.int_at("Position", 1).unwrap_or(0) + offset;
                    let prio = c.int("Priority").unwrap_or(9);
                    self.show(&key, -1, (path, w, h), x, y, prio, if ty == "CharacterOpen" { 200 } else { 0 });
                    if let Some(s) = self.shown.get_mut(&key) {
                        s.base_y = sy(360 + offset);
                    }
                }
            }
            "CharacterDelete" => {
                for c in t.children.iter().filter(|c| c.name.starts_with("Chara")) {
                    let n = c.int("Number").unwrap_or(-1);
                    self.hide(&format!("c{n}"), 200);
                }
            }
            "CharacterMove" => {
                let time = ms(t.int("Time").unwrap_or(0));
                for c in t.children.iter().filter(|c| c.name.starts_with("Chara")) {
                    let key = format!("c{}", c.int("Number").unwrap_or(-1));
                    let Some(s) = self.shown.get(&key).cloned() else { continue };
                    let x = c.int_at("Position", 0).map_or(s.x, sx);
                    // y is from the character's usual height.
                    let y = s.base_y + sy(c.int_at("Position", 1).unwrap_or(0));
                    self.out.ops.push(Op::SpriteMove { layer: s.layer, x, y, ms: time });
                    self.shown.insert(key, Shown { x, y, ..s });
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
                if t.int("Visible") == Some(0) {
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
                    let key = format!("c{}", c.int("Number").unwrap_or(-1));
                    let (Some(s), Some(a)) = (self.shown.get(&key).cloned(), c.int("Action").and_then(|n| self.game.actions.get(&n)))
                    else {
                        continue;
                    };
                    if a.looped {
                        self.note("CharacterAction (looping)");
                        continue;
                    }
                    // Straight moves from keyframe to keyframe, waited on.
                    let mut last = 0;
                    for &(frame, x, y) in &a.keys {
                        let step = ms(frame - last);
                        last = frame;
                        self.out.ops.push(Op::SpriteMove { layer: s.layer, x: s.x + sx(x), y: s.y + sy(y), ms: step });
                    }
                    self.out.ops.push(Op::SpriteMove { layer: s.layer, x: s.x, y: s.y, ms: 0 });
                }
            }
            "Flash" => {
                let c: Vec<i64> = t.get("Color").unwrap_or(&[]).iter().filter_map(Val::int).collect();
                let alpha = c.get(3).copied().unwrap_or(255).clamp(0, 255) as u32;
                let time = ms(t.int("InTime").unwrap_or(3) + t.int("OutTime").unwrap_or(3));
                self.out.ops.push(Op::Flash { colour: colour(t), opacity: alpha * 256 / 255, ms: time.max(16) });
            }
            "EffectQuake" | "SpriteQuake" => {
                let ax = sx(t.int_at("Param", 0).unwrap_or(30));
                let ay = sy(t.int_at("Param", 1).unwrap_or(30));
                let time = t.int("Time").unwrap_or(30);
                // Shakes for :Time frames, dying down (0: until the end task).
                let end = if time > 0 { (0, 0) } else { (ax, ay) };
                let shake = ScreenEffect::Shake { time_ms: ms(time).max(1), interval_ms: 16, path: false, speed: (2200, 2900), range: [(ax, ay), end] };
                self.out.ops.push(Op::ScreenEffect { slot: 1, effect: Some(shake) });
                if t.int("Wait").unwrap_or(0) != 0 {
                    self.out.ops.push(Op::Wait { ms: ms(time) });
                }
                if ty == "SpriteQuake" {
                    self.note("SpriteQuake (shakes the screen)");
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
                self.out.ops.push(Op::SpriteMove { layer: s.layer, x, y, ms: time });
                self.shown.insert(key, Shown { x, y, ..s });
                if t.get("Scale").is_some_and(|v| v.first().and_then(Val::float) != Some(1.0)) {
                    self.note("SpriteMove :Scale");
                }
            }
            // `:Image 0,N`: CG.cpk's ECnn (27 is the title logo) over
            // everything, on the cover's layer, coming in through the
            // shutters; EyeCatchEnd puts back what was there (the black of
            // a FadeOut, or nothing) as the shutters open over `:Fade`.
            // `:Image 1,…` (a chapter card) fades in and out.
            "EyeCatch" => {
                let n = t.int_at("Image", 1).unwrap_or(-1);
                let image = match t.int_at("Image", 0) {
                    Some(0) if n >= 0 => self.assets.cg_named(&format!("EC{n:02}.png")),
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
                    // 画面キャプチャースプライト: a copy of the screen drawn
                    // zoomed (impacts). Approximated by a pulse: the copy,
                    // growing, over the live picture, fading.
                    57 => {
                        let effect = start.then_some(ScreenEffect::Pulse {
                            time_ms: 400,
                            interval_ms: 16,
                            zoom: (0, 80),
                            opacity: 160,
                            colour: 0xffffff,
                            tint_opacity: 0,
                            tint: 0,
                        });
                        self.out.ops.push(Op::ScreenEffect { slot: 2, effect });
                        if start {
                            self.note("EffectStart 57 (approximated by a pulse)");
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
            "BacklogMessage" | "SceneJumpDialogEnd" | "MovieStop" | "CharacterStop" | "CharacterWait" | "SpriteMoveEnd" | "CharacterActionEnd" => {}
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

    /// START's story menu (the original's labels in `title_bg.png`): once
    /// any ending was seen, the after-stories (adv.txt's start scenes 1-4)
    /// are offered beside the main story. Each opens with its heroine's
    /// ending (かるお's with the true ending); the original's rule is in
    /// the eboot, so this one is ours.
    fn story_menu(&mut self) {
        const STORIES: [(&str, &str, &str); 4] = [
            ("ある日の林田美咲", "P03_01A", "misaki_ed"),
            ("ある日の渡来明日香", "P01_01A", "asuka_ed"),
            ("ある日の山科京", "P02_01A", "miyako_ed"),
            ("ある日の軽部狩男", "P04_01A", "kobato_ed"),
        ];
        let menu = "mg_story_menu".to_string();
        let start = scene_label(&self.game.start);
        for (_, _, flag) in STORIES {
            if let Some(var) = self.game.flag(flag, true) {
                self.out.ops.push(Op::If { var, cmp: Compare::Ne, value: Operand::Const(0), label: menu.clone() });
            }
        }
        self.out.ops.push(Op::Jump(start.clone()));
        self.out.ops.push(Op::Label(menu.clone()));
        let mut options = vec![("俺たちに翼はない　始めから".to_string(), start)];
        let mut gates = Vec::new();
        for (title, scene, flag) in STORIES {
            let gate = format!("mg_story_{}", scene.to_ascii_lowercase());
            options.push((title.to_string(), gate.clone()));
            gates.push((gate, scene, flag));
        }
        self.out.ops.push(Op::Choice { options, style: None });
        for (gate, scene, flag) in gates {
            self.out.ops.push(Op::Label(gate));
            if let Some(var) = self.game.flag(flag, true) {
                self.out.ops.push(Op::If { var, cmp: Compare::Ne, value: Operand::Const(0), label: scene_label(scene) });
            }
            let text = "　このお話は、まだ読むことができません。".to_string();
            self.out.ops.push(Op::Text { speaker: None, text, voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false });
            self.out.ops.push(Op::Jump(menu.clone()));
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
/// - 1: a fade with the rows waving sideways, 6: a fade blurred sideways
///   halfway (both a plain fade here); 4: not looked at (2 uses).
fn act_style(t: &Block) -> TransitionStyle {
    let direction = t.int_at("Param", 0).unwrap_or(0).clamp(0, 3);
    let (dx, dy) = [(1, 0), (0, 1), (-1, 0), (0, -1)][direction as usize];
    match t.int("Type") {
        Some(0 | 7) => TransitionStyle::Slide { moving: SlideLayer::Both, dx, dy },
        Some(5) if direction < 2 => TransitionStyle::ThroughWhite,
        Some(2) => TransitionStyle::Mosaic,
        Some(3) => TransitionStyle::Flip { horizontal: direction < 2, grow: true },
        _ => TransitionStyle::Fade,
    }
}

/// Compiles `scenes` (all of them if empty), each at its label, after a
/// jump to the game's first scene.
pub fn compile(game: &Game, scenes: &[String], assets: &mut dyn Assets) -> Compiled {
    let mut l = Lowering { game, assets, out: Compiled::default(), shown: HashMap::new(), next_slot: HashMap::new(), staging: false, choices: 0, cover: None, eye_catch: false, shutters: None };
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
        for i in 0..scene.tasks.len() {
            l.task(name, &scene.tasks, i);
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
