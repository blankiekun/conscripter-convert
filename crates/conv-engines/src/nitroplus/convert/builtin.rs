//! NSS's built-in functions, as the compiler runs them ([`Compiler::builtin`]).

use std::cell::Cell;
use crate::metrics::Channel;

use super::*;

/// The built-ins of Muramasa.exe, as `dispatch_builtin` (`00405250`,
/// statements) and `eval_primary` (`0043a780`, values) name them
/// (`research/nitroplus/builtins.md`, "In the exe"). The exe matches them
/// case-sensitively; any other name is an NSS function, and a call to a
/// function that doesn't exist is skipped.
const BUILTINS: [&str; 122] = [
    "CreateName", "CreateAnimation", "CreateProcess", "CreateColor", "CreateEffect",
    "CreateFrame", "CreateTexture", "CreateClipTexture", "CreateDifferenceTexture",
    "CreateMask", "CreateStencil", "CreateSurface", "CreateMovie", "CreateCube", "CreateSound",
    "CreateText", "CreateWindow", "CreateCamera", "CreateChoice", "CreateBacklog",
    "CreateScrollbar", "LoadImage", "LoadClipImage", "LoadColor", "LoadFont", "LoadText",
    "LoadFile", "UnloadFile", "LoadNPT", "Move", "BezierMove", "Request", "Rotate", "Shake",
    "MoveCamera", "MoveCube", "Delete", "Disable", "Fade", "SetChip", "SetAlias", "SetVertex",
    "SetShade", "SetTone", "SetImage", "SetSurface", "SetComposite", "SetBlur", "SetStream",
    "SetFov", "SetNextFocus", "SetScrollbar", "SetScrollSpeed", "SetBacklog", "Zoom", "Draw",
    "DrawEffect", "DrawTransition", "Wait", "WaitKey", "WaitMove", "WaitFade", "WaitZoom",
    "WaitRotate", "WaitShake", "WaitPlay", "WaitText", "WaitAction", "SetVolume", "SetPan",
    "SetFrequency", "SetLoop", "SetLoopPoint", "MoviePlay", "ClearScore", "ClearBacklog",
    "SetBaseTextInterval", "SetFont", "Save", "Load", "DeleteSaveFile", "Position",
    "CursorPosition", "MoveCursor", "DateTime", "WriteFile", "SetShortcut", "Escape", "Reset",
    "Exit", "System",
    "Random", "RemainTime", "DurationTime", "PassageTime", "ImageHorizon", "ImageVertical",
    "Strlen", "Strstr", "String", "EnableBacklog", "ExistSave", "ReadFile", "ModuleFileName",
    "BacklogRow", "BacklogLine", "ScrollbarValue", "Time", "MilliTime", "Integer",
    "SoundAmplitude", "Conquest", "Message", "Array", "AssocArray", "Count", "VariableValue",
    "EnableDirect3DHAL", "Platform", "AvailableFile", "MountSavedata", "LockVideo",
];

/// Arguments the compiler doesn't read that change nothing it draws or
/// plays: (built-in, index, the value covered (`"null"`: NSS's null) or
/// `None` for any), why.
/// What a built-in's argument controls (research/nitroplus/builtins.md's
/// signatures), for the score's credit (`metrics::credit`): an unread one
/// loses that much of its call, not one argument's part.
const ROLES: &[(&str, usize, &str)] = &[
    ("CreateTexture", 1, "order"),
    ("CreateTexture", 2, "place"),
    ("CreateTexture", 3, "place"),
    ("CreateTexture", 4, "picture"),
    ("CreateWindow", 1, "order"),
    ("CreateWindow", 2, "place"),
    ("CreateWindow", 3, "place"),
    ("CreateWindow", 4, "size"),
    ("CreateWindow", 5, "size"),
    ("Fade", 3, "tempo"),
    ("Fade", 4, "tempo"),
    ("Move", 2, "place"),
    ("Move", 3, "place"),
    ("Move", 4, "tempo"),
    ("Move", 5, "tempo"),
    ("Zoom", 2, "size"),
    ("Zoom", 3, "size"),
    ("Zoom", 4, "tempo"),
    ("Zoom", 5, "tempo"),
    ("Rotate", 5, "tempo"),
    ("Rotate", 6, "tempo"),
    ("MoveCamera", 2, "place"),
    ("MoveCamera", 3, "place"),
    ("MoveCamera", 4, "size"),
    ("MoveCamera", 5, "tempo"),
    ("MoveCamera", 6, "tempo"),
    ("SetVolume", 3, "tempo"),
    ("SetBlur", 4, "tempo"),
    ("WaitAction", 1, "tempo"),
    ("WaitPlay", 1, "tempo"),
    ("WaitFade", 1, "tempo"),
    ("WaitMove", 1, "tempo"),
    ("WaitZoom", 1, "tempo"),
];

const UNREAD_HARMLESS: &[(&str, usize, Option<&str>, &str)] = &[
    ("Fade", 3, Some("null"), "no tempo: linear, as ours"),
    ("Rotate", 5, Some("null"), "no tempo: linear, as ours"),
    ("SetVolume", 3, Some("null"), "no tempo: linear, as ours"),
    ("SetPan", 3, Some("null"), "no tempo: linear, as ours"),
    ("DrawTransition", 5, Some("null"), "no tempo: linear, as ours"),
    ("CreateProcess", 1, None, "a process draws nothing: its priority is unused"),
    ("CreateProcess", 2, None, "a process draws nothing: its place is unused"),
    ("CreateProcess", 3, None, "a process draws nothing: its place is unused"),
    ("SetFrequency", 3, Some("null"), "no tempo: at once, as ours"),
    ("MoveCamera", 5, Some("null"), "no tempo: linear, as ours"),
    ("WaitAction", 1, Some("null"), "no time limit: waits to the end, as ours"),
    ("WaitPlay", 1, Some("null"), "no time limit: waits to the end, as ours"),
    ("WaitFade", 1, Some("null"), "no time limit: waits to the end, as ours"),
    ("WaitMove", 1, Some("null"), "no time limit: waits to the end, as ours"),
    ("WaitZoom", 1, Some("null"), "no time limit: waits to the end, as ours"),
    ("WaitText", 0, None, "the page's own text: ours waits on the page"),
    ("WaitText", 1, Some("null"), "no time limit: waits to the end, as ours"),
    // (Muramasa's `LoadText("$構文名", "$SYSTEM_present_preprocess", box, …)`, Django's `LoadText("main_stage", box, …)`.)
    ("LoadText", 0, None, "the script unit's name: the text is compiled where it stands"),
    ("LoadText", 1, None, "the box's preprocess name: the text is compiled where it stands"),
    ("LockVideo", 0, None, "a group of changes shown at once: ours shows changes only at the next wait"),
];

/// The note on a `Conquest` call, left out ([`super::LEFT_OUT`]).
/// How long auto mode lets a `WaitKey` wait, ms.
const WAITKEY_AUTO_MS: u32 = 1500;

pub(super) const CONQUEST: &str = "Conquest: whole-scene read marks";

/// The note on a call of the choice screen's own code (`function_select.nss`:
/// its buttons, labels, masks, focus, pointer and its `Save(9999)` for
/// 選択肢に戻る), left out ([`super::LEFT_OUT`]): the engine's choice
/// (`ChoiceStyle`) stands in, and conv-cons' back-to-choice save where the
/// game's menu has that item.
pub(super) const CHOICE_SCREEN: &str = "choice screen's own call";

impl Compiler<'_, '_> {
    /// A built-in call; every argument passed and not read gets a note
    /// (`<Builtin> argument <i> ignored`), so a call converted only in part
    /// shows in the dry run's list.
    pub(super) fn builtin(&mut self, name: &str, a: &[Value]) -> Value {
        let read = Cell::new(0u64);
        self.forget_ended_sounds();
        let value = self.builtin_reading(name, a, &read);
        if name.starts_with("Create") && let Some(v) = a.first() {
            let made = self.object_name(v);
            if self.objects.contains_key(&made) {
                self.untracked.remove(&made);
            } else {
                self.untracked.insert(made);
            }
        } else if name == "Delete" && let Some(v) = a.first() {
            let gone = self.object_name(v);
            self.untracked.retain(|k| !glob(&gone, k));
        }
        for (i, v) in a.iter().enumerate().take(64) {
            let harmless = UNREAD_HARMLESS.iter().any(|&(b, k, value, _)| b == name && k == i && value.is_none_or(|x| if x == "null" { matches!(v, Value::Null) } else { v.string() == x }));
            if read.get() & (1 << i) == 0 && !harmless {
                match ROLES.iter().find(|&&(b, k, _)| b == name && k == i) {
                    Some((_, _, role)) => self.note(format!("{name} argument {i} ignored ({role})")),
                    None => self.note(format!("{name} argument {i} ignored")),
                }
            }
        }
        value
    }

    fn builtin_reading(&mut self, name: &str, a: &[Value], read: &Cell<u64>) -> Value {
        let (screen_w, screen_h) = self.game.screen;
        let arg = |i: usize| {
            read.set(read.get() | 1u64.checked_shl(i as u32).unwrap_or(0));
            a.get(i).cloned().unwrap_or(Value::Null)
        };
        let last = || a.len().checked_sub(1).map_or(Value::Null, arg);
        match name {
            "LoadImage" if self.game.loaded_textures && self.file != "function_select.nss" => {
                // (name, path): a picture other objects are made from
                // (`CreateTexture("frame03", …, "@LoadFrame")`), not shown.
                let name = self.object_name(&arg(0));
                match self.assets.image(&self.deref(&arg(1))) {
                    Some(img) => {
                        self.loaded_images.insert(name, img);
                    }
                    None => self.note("LoadImage of a missing file"),
                }
            }
            "CreateTexture" => {
                let name = self.object_name(&arg(0));
                let (screen_w, screen_h) = self.frame(&name);
                let src = self.deref(&arg(4));
                let Some((image, w, h)) = self.texture_source(&src) else {
                    if self.file == "function_select.nss" {
                        // The choice plates (装甲悪鬼村正's `select_img*`): the engine's choice.
                        (0..a.len()).for_each(|i| drop(arg(i)));
                        self.note(CHOICE_SCREEN);
                        return Value::Null;
                    }
                    self.note(format!("CreateTexture from {}", if src.contains('/') { "a missing file" } else { "an object or the screen" }));
                    let o = self.create(name, Kind::Other, arg(1).int(), 0, 0);
                    (o.x, o.y) = (arg(2).position(0, 0, screen_w), arg(3).position(0, 0, screen_h));
                    return Value::Null;
                };
                let (w, h) = (w as i64, h as i64);
                let o = self.create(name, Kind::Image { image }, arg(1).int(), w, h);
                o.x = arg(2).position(0, w, screen_w);
                o.y = arg(3).position(0, h, screen_h);
            }
            "CreateStencil" | "CreateMask" => {
                // Stencil (name, priority, x, y, mode, picture, wildcard):
                // draws nothing; its children only where the picture is
                // opaque. Mode 128 (every call) keeps the picture's alpha
                // (`00409fa0`). Mask (name, priority, x, y, picture,
                // wildcard): draws nothing; its children multiplied by the
                // picture, colour and alpha, nothing outside it or where
                // it is clear. `true` lets a wildcard reach the children (the flag at
                // +0x9d4, left 0 otherwise; 00443800 for both).
                // src: exe 00457e80 (a mask's clear pixels → 0), 00416860 (its picture into a screen-sized buffer cleared to 0), 00419210 (a child's pixel × the buffer's, per byte)
                let mask = name == "CreateMask";
                if mask && self.file == "function_select.nss" {
                    read.set(u64::MAX);
                    self.note(CHOICE_SCREEN);
                    return Value::Null;
                }
                let what = name.to_string();
                let name = self.object_name(&arg(0));
                let (screen_w, screen_h) = self.frame(&name);
                if !mask && arg(4).int() != 128 {
                    self.note("CreateStencil mode other than 128");
                }
                let at = usize::from(!mask);
                let Some((image, w, h)) = self.assets.image(&self.deref(&arg(4 + at))) else {
                    self.note(format!("{what} of a missing file"));
                    return Value::Null;
                };
                let (w, h) = (w as i64, h as i64);
                let relative = self.game.relative_children;
                let o = self.create(name, Kind::Other, arg(1).int(), w, h);
                o.x = arg(2).position(0, w, screen_w);
                o.y = arg(3).position(0, h, screen_h);
                o.container = !arg(5 + at).truthy();
                o.screen_children = o.container && !relative;
                o.stencil = Some((image, mask));
            }
            "CreateColor" => {
                let name = self.object_name(&arg(0));
                let (screen_w, screen_h) = self.frame(&name);
                let (w, h) = (arg(4).int().max(1), arg(5).int().max(1));
                let Some(rgb) = color(&arg(6)) else {
                    if std::env::var_os("NSS_TRACE").is_some() {
                        eprintln!("unknown colour {:?}", arg(6));
                    }
                    self.note("CreateColor with an unknown colour");
                    return Value::Null;
                };
                let image = self.assets.color(rgb, w as u32, h as u32);
                let o = self.create(name, Kind::Image { image }, arg(1).int(), w, h);
                o.x = arg(2).position(0, w, screen_w);
                o.y = arg(3).position(0, h, screen_h);
            }
            "CreateEffect" => {
                // (name, priority, x, y, w, h, type): a tone or a frozen
                // copy of the screen over the rectangle, rain in it.
                let name = self.object_name(&arg(0));
                let (w, h) = (arg(4).int(), arg(5).int());
                let (x, y) = (arg(2).position(0, w, screen_w), arg(3).position(0, h, screen_h));
                let whole = x <= 0 && y <= 0 && x + w >= screen_w && y + h >= screen_h;
                let kind = match arg(6).string().to_ascii_lowercase().as_str() {
                    // shortcut: a Plain cover over part of the screen freezes all of it (one call, 800×600)
                    "plain" => {
                        if !whole {
                            self.note("CreateEffect Plain on part of the screen");
                        }
                        let p = arg(1).int();
                        let mut froze: Vec<_> = self.shown.iter().filter(|(n, _)| self.objects.get(*n).is_none_or(|o| o.priority <= p)).map(|(n, s)| (n.clone(), s.clone())).collect();
                        froze.sort_by_key(|(_, s)| s.layer);
                        // Their order kept, not the layers (which differ between a Random's picks).
                        froze.iter_mut().enumerate().for_each(|(i, (_, s))| s.layer = i as u32);
                        Kind::Cover { froze, again: false }
                    }
                    // A tint drawn over part of the screen (装甲悪鬼村正's
                    // mc01_011vsa zoom box) would tint it all: left out.
                    t if let Some(fs) = tone(t) => match whole {
                        true => Kind::Tint(fs),
                        false => {
                            self.note(format!("CreateEffect {} on part of the screen", arg(6).string()));
                            Kind::Other
                        }
                    },
                    "rain" => self.rain(w, h).map_or(Kind::Other, Kind::Particles),
                    "drop" => match self.drop_effect(w, h) {
                        Some(p) => Kind::Particles(p),
                        None => {
                            self.note("CreateEffect Drop without a picture");
                            Kind::Other
                        }
                    },
                    "lens" => Kind::Lens(self.lens()),
                    _ => {
                        self.note(format!("CreateEffect {}", arg(6).string()));
                        Kind::Other
                    }
                };
                if matches!(kind, Kind::Particles(_) | Kind::Lens(_)) {
                    let o = self.create(name, kind, arg(1).int(), w, h);
                    (o.x, o.y) = (x, y);
                } else {
                    self.create(name, kind, arg(1).int(), screen_w, screen_h);
                }
            }
            // (name, x, y, zoom per mille); a place such as `Center` is 0
            // (装甲悪鬼村正's ma04_005a: only then do its pictures cover the
            // screen through the camera).
            "CreateCamera" => {
                let name = self.object_name(&arg(0));
                let o = self.create(name, Kind::Camera, 0, 0, 0);
                o.container = true;
                (o.x, o.y) = (arg(1).int(), arg(2).int());
                let z = arg(3).int().max(1);
                o.zoom = (z, z);
            }
            "MoveCamera" => {
                // (name, time, x, y, zoom, tempo, wait): its children move
                // with it.
                let targets = self.matching(&arg(0));
                let mut children = Vec::new();
                for t in &targets {
                    let Some(o) = self.objects.get_mut(t).filter(|o| matches!(o.kind, Kind::Camera)) else { continue };
                    (o.x, o.y) = (arg(2).position(o.x, 0, 0), arg(3).position(o.y, 0, 0));
                    let z = arg(4).position(o.zoom.0, 0, 0).max(1);
                    o.zoom = (z, z);
                    let prefix = format!("{t}/");
                    children.extend(self.objects.keys().filter(|k| k.starts_with(&prefix)).cloned());
                    if arg(1).int() > 0 {
                        self.pending_camera.insert(t.clone(), arg(1).int() as u32);
                    }
                }
                if children.is_empty() {
                    // Nothing moves: the tempo is moot, and a wait of no
                    // time (or none) loses nothing (findings row MoveCamera
                    // wait); a timed wait stays a gap.
                    drop(arg(5));
                    if arg(1).int() <= 0 || !arg(6).truthy() {
                        drop(arg(6));
                    }
                    return Value::Null;
                }
                self.set_tempo(tempo(&arg(5)), &children);
                self.timed(arg(1).int(), &arg(6), &children);
            }
            "CreateWindow" | "CreateName" => {
                let is_window = name == "CreateWindow";
                let name = self.object_name(&arg(0));
                let relative = self.game.relative_children;
                let o = self.create(name, Kind::Other, arg(1).int(), 0, 0);
                o.container = true;
                // (name, priority, x, y, w, h, relative): a text in the
                // window starts at its corner (続・殺戮のジャンゴ's boxes); its
                // children are drawn only inside it.
                // src: Django.exe FUN_0042fee0: anything but `true` clears the
                // object's +0x3c8 (1 for every new object, FUN_00436de0), and
                // the place parser (FUN_004403a0) and FUN_00437980 then count
                // its children's places from the screen.
                // (`true` is 1 here, `false` 0.)
                // src: Muramasa.exe builtin_CreateWindow 0040b3a0 → create_window 00443500 clears
                // +0x9d4 alike (the value in EAX); init_object_place 0044c8c0, parse_object_place 00458d50.
                o.screen_children = if is_window { !arg(6).truthy() } else { !relative };
                if a.len() >= 6 {
                    let (w, h) = (arg(4).int(), arg(5).int());
                    // shortcut: with Game::relative_children (続・殺戮のジャンゴ) a
                    // named place (OutRight) stays 0, as its children were checked.
                    (o.x, o.y) = match relative {
                        true => (arg(2).int(), arg(3).int()),
                        false => (arg(2).position(0, w, screen_w), arg(3).position(0, h, screen_h)),
                    };
                    o.clip = (w > 0 && h > 0).then_some((w, h));
                }
            }
            "CreateMovie" => {
                let name = self.object_name(&arg(0));
                let (screen_w, screen_h) = self.frame(&name);
                let path = self.deref(&arg(6));
                // Read before the file check, so a missing movie is one note, not five.
                let (order, at_x, at_y, looped, alpha) = (arg(1).int(), arg(2), arg(3), arg(4).truthy(), arg(5).truthy());
                let Some(info) = self.assets.movie_info(&path) else {
                    self.note(missing_movie("CreateMovie", self.assets.has_movies()));
                    self.create(name, Kind::Other, order, 0, 0);
                    return Value::Null;
                };
                let (w, h) = (info.w as i64, if alpha { info.h as i64 / 2 } else { info.h as i64 });
                let m = MovieObject { path, looped, alpha, dark: false, info, state: MovieState::Playing { at: 0 }, credit: 0 };
                let o = self.create(name, Kind::Movie(m), order, w, h);
                o.x = at_x.position(0, w, screen_w);
                o.y = at_y.position(0, h, screen_h);
            }
            "MoviePlay" => {
                // Full screen, the script waiting. A movie with sound stops
                // the music; a silent cut lets it play on.
                let path = self.deref(&arg(0));
                self.sync(0);
                let skippable = arg(1).truthy();
                let Some(info) = self.assets.movie_info(&path) else {
                    // `mv_smg_PV` (the title's): no such file on the disc.
                    self.note(missing_movie("MoviePlay", self.assets.has_movies()));
                    return Value::Null;
                };
                if let Some(file) = self.assets.movie_file(&path, info.sound) {
                    self.ops.push(if info.sound { Op::Movie { file, skippable } } else { Op::SceneMovie { file, skippable } });
                }
            }
            "CreateSound" => {
                let name = self.object_name(&arg(0));
                let bgm = arg(1).string().eq_ignore_ascii_case("bgm");
                let s = SoundObject { path: self.named_string(&arg(2)), bgm, volume: 1000, looped: false, starting: false, fade_ms: 0, playing: None, disused: false, loop_ms: None, started_at: 0, len_ms: None, pan: 0, rate: None, freq: 1000, stops_at: None };
                self.create(name, Kind::Sound(s), 0, 0, 0);
            }
            "Fade" => {
                // Four arguments: no tempo.
                let wait = if a.len() == 4 { arg(3) } else { arg(4) };
                let (ms, to) = (arg(1).int(), arg(2));
                let targets = self.matching(&arg(0));
                if self.no_object(&arg(0)) {
                    (0..a.len()).for_each(|i| drop(arg(i)));
                    return Value::Null;
                }
                let mut window_only = !targets.is_empty();
                // src: cb_Fade 0044f020 (Django.exe cb_00438fb0): an object whose
                // opacity byte the fade leaves as it is (a new one's is 255,
                // FUN_00488740) starts no action, so a fade on only such objects
                // neither waits nor adds to the next sync.
                let byte = |o: i64| (o as f64 * 0.255).round_ties_even() as i64;
                let mut moving = false;
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    window_only &= o.hideable;
                    // A loop process with a lead-in that fades the picture in,
                    // started before it is drawn, then a fade at once
                    // (TitleSet's `$Logo` 1: TumbleweedMove, then
                    // `Fade("タイトル*", 0, 0)`): the opacity holds in its keys
                    // until the process's own fade; it is placed at the loop's.
                    if ms <= 0 && !self.shown.contains_key(t)
                        && let Some((_, keys, from)) = o.shake.as_mut()
                        && *from > 0
                        && let Some((s, d)) = o.loop_fade
                        && let Some(end) = keys.iter().find(|k| k.ms >= s + d).map(|k| k.opacity)
                    {
                        let now = (to.position(o.opacity, 0, 0).clamp(0, 1000) * 256 / 1000) as u32;
                        for k in keys.iter_mut().take_while(|k| k.ms < s + d) {
                            let part = f64::from(k.ms.saturating_sub(s)) / f64::from(d);
                            k.opacity = (f64::from(now) + (f64::from(end) - f64::from(now)) * part).round() as u32;
                        }
                        continue;
                    }
                    if o.opacity == 0 {
                        self.fade_from_zero.insert(t.clone());
                    }
                    let was = o.opacity;
                    o.opacity = to.position(o.opacity, 0, 0).clamp(0, 1000);
                    // (Or one still under way: a `DrawTransition` unsynced.)
                    moving |= byte(was) != byte(o.opacity) || self.pending_fade.contains_key(t) || self.pending_by.contains_key(t);
                }
                if ms <= 0 {
                    // src: Muramasa.exe cb_Fade 0044f020: time 0 sets the opacity at
                    // once, the tempo unused (Django.exe cb_00438fb0 the same).
                    arg(3);
                }
                // (An object left out, `untracked`, waits as before.)
                let still = ms > 0 && !targets.is_empty() && !moving;
                if still && !window_only {
                    arg(3); // (its tempo, unused)
                } else if !window_only {
                    if ms > 0 {
                        for t in &targets {
                            self.pending_fade.insert(t.clone(), ms as u32);
                        }
                        // src: Muramasa.exe's fade step (0044f020) eases by the tempo
                        // (apply_tempo); Django.exe's (cb_00438fb0) by FUN_00434ea0's curve
                        if a.len() > 4 && let Some(e) = self.game.fade_tempo.unwrap_or(|_| None)(&arg(3)) {
                            self.fade_tempo = Some(e);
                        }
                    }
                    // A waited fade that leaves the shown text at 0 as drawn
                    // (its window's opacity × its own: 続・殺戮のジャンゴ's
                    // TypeEnd, `Fade("@box*", 100, 0, null, true)`) fades the
                    // text out with it, instead of the sync's instant `textoff`.
                    let waited = !matches!(wait, Value::Null | Value::Int(0));
                    let text = self.vars.get("$SYSTEM_present_text").map(|v| v.string().trim_start_matches('@').to_string());
                    let gone = text.is_some_and(|t| self.objects.get(&t).is_some_and(|o| self.drawn_opacity(&t, o.opacity) == 0));
                    if waited && ms > 0 && self.window_visible && gone && self.game.relative_children {
                        self.ops.push(Op::WindowFade { alpha: 0, ms: ms as u32, from: None, delay: 0, show: false });
                        self.window_visible = false;
                        self.window_fade_in = None;
                    }
                    self.timed(ms, &wait, &targets);
                } else {
                    // The text window fades as the script's own `Fade`s say
                    // (CONscripter's `windowfade`): out when a waited fade
                    // leaves no box (an unwaited one is a box switch), in
                    // from hidden with the next page (its text after the fade
                    // when the fade is waited).
                    let waited = !matches!(wait, Value::Null | Value::Int(0));
                    let visible = self.objects.values().any(|o| o.hideable && o.opacity > 0 && matches!(o.kind, Kind::Image { .. }));
                    let ms = ms.max(0) as u32;
                    if waited && self.window_visible && !visible {
                        self.ops.push(Op::WindowFade { alpha: 0, ms, from: None, delay: 0, show: false });
                        self.window_visible = false;
                        self.window_fade_in = None;
                        self.timed(i64::from(ms), &wait, &[]);
                    } else if !self.window_visible && visible && ms > 0 {
                        let (m, w) = self.window_fade_in.unwrap_or((0, false));
                        self.window_fade_in = Some((m.max(ms), w || waited));
                    }
                }
            }
            "Move" => {
                let targets = self.matching(&arg(0));
                if self.no_object(&arg(0)) {
                    (0..a.len()).for_each(|i| drop(arg(i)));
                    return Value::Null;
                }
                let (ms, to_x, to_y, tempo) = (arg(1).int(), arg(2), arg(3), tempo(&arg(4)));
                // (In a process its wait holds only the process: `timed`.)
                let blocking = !matches!(arg(5), Value::Null | Value::Int(0)) && self.action_process.is_none();
                // One picture on screen moving while the script waits: a real
                // motion (after showing what changed before it).
                // (A picture with children shown goes the general way below.)
                let carries = |t: &String| self.game.relative_children || self.places_children(t);
                let alone = |t: &String| !carries(t) || !self.shown.keys().any(|k| k.starts_with(&format!("{t}/")));
                if ms > 0 && blocking && targets.len() == 1 && self.shown.contains_key(&targets[0]) && alone(&targets[0]) {
                    self.sync(0);
                    let trail = self.objects[&targets[0]].trail;
                    let (screen_w, screen_h) = self.frame(&targets[0]);
                    let o = self.objects.get_mut(&targets[0]).unwrap();
                    let (x, y) = (to_x.position(o.x, o.w, screen_w), to_y.position(o.y, o.h, screen_h));
                    let (dx, dy) = (x - o.x, y - o.y);
                    (o.x, o.y) = (x, y);
                    if let Some(sh) = self.shown.get_mut(&targets[0]) {
                        sh.x += dx;
                        sh.y += dy;
                        let (cx, cy) = ((sh.x + sh.w / 2) as i32, (sh.y + sh.h / 2) as i32);
                        // (An spmotion: the engine's trail follows it.)
                        if let Some(t) = trail {
                            self.ops.push(Op::SpriteTrail { layer: sh.layer, passes: t.passes, alpha: t.alpha, lag: t.lag, over: t.over });
                        }
                        if let Some((ease, exponent)) = tempo {
                            self.ops.push(Op::MoveEase { ease, exponent });
                        }
                        self.ops.push(Op::SpriteMove { layer: sh.layer, x: cx, y: cy, ms: ms as u32 });
                    }
                    return Value::Null;
                }
                // A 0 ms move stops one in flight where it is (the exe, seen by
                // the snapshot oracle): moved on from there.
                if ms == 0 {
                    targets.iter().for_each(|t| self.stop_move(t));
                }
                let mut from = Vec::new();
                for t in &targets {
                    let (screen_w, screen_h) = self.frame(t);
                    let o = self.objects.get_mut(t).unwrap();
                    let (x, y) = (o.x, o.y);
                    o.x = to_x.position(o.x, o.w, screen_w);
                    o.y = to_y.position(o.y, o.h, screen_h);
                    from.push((t.clone(), (x - o.x, y - o.y)));
                }
                // Children placed from their parent slide with it (a face in
                // its frame: `WindowMoveM` moves only the frame).
                let mut moved = targets.clone();
                for (t, d) in from.clone().iter().filter(|(t, _)| self.game.relative_children || self.places_children(t)) {
                    let inside = format!("{t}/");
                    let children: Vec<String> = self.objects.keys().filter(|k| k.starts_with(&inside) && !targets.contains(k)).cloned().collect();
                    from.extend(children.iter().map(|c| (c.clone(), *d)));
                    moved.extend(children);
                }
                // (Not shown yet: shown at its start by the next sync, and moved;
                // a shown one's start is kept too, should a sync hide it first.)
                for (t, (dx, dy)) in from.into_iter().filter(|_| ms > 0) {
                    let d = self.pending_from.entry(t).or_default();
                    *d = (d.0 + dx, d.1 + dy);
                }
                self.set_tempo(tempo, &moved);
                self.timed(ms, &arg(5), &moved);
            }
            "Delete" if arg(0).string() == "*" && self.game.chapter_objects && self.chapter_mark.is_some() => {
                self.delete_chapter_objects(self.chapter_mark.unwrap());
            }
            "Delete" => {
                // A wildcard that matches a container (a locked one too:
                // `遠３`) still deletes its children (under wine the standing
                // pictures in `遠３/StL01` go with `Delete("*")`).
                let mut targets = self.matching(&arg(0));
                let containers: Vec<String> = targets.iter().filter(|t| self.objects[*t].container).cloned().collect();
                for c in containers {
                    let inside = format!("{c}/");
                    targets.extend(self.objects.keys().filter(|k| k.starts_with(&inside)).cloned());
                }
                for t in targets {
                    // (A child may have gone with its parent.)
                    if self.objects.get(&t).is_some_and(|o| !o.locked) {
                        self.delete(&t);
                    }
                }
            }
            "Request" => {
                let action = arg(1).string().to_ascii_lowercase();
                let targets = self.matching(&arg(0));
                if action == "stop" {
                    self.stop_one_shots(&self.object_name(&arg(0)));
                }
                if targets.is_empty() && action == "enter" {
                    self.pending_enter = Some(arg(0).string().trim_start_matches('@').to_string());
                }
                for t in targets {
                    self.request(&t, &action);
                }
            }
            "SetAlias" => {
                let (obj, alias) = (self.object_name(&arg(0)), self.var_string(&arg(1)));
                // src: Django.exe 00432da0, Muramasa.exe 004475b0: one alias slot
                // per object (+0x100), which the new alias overwrites.
                self.aliases.retain(|_, o| *o != obj);
                if obj != alias {
                    self.aliases.insert(alias, obj);
                }
            }
            "Rotate" => {
                // (obj, time, x, y, z degrees, tempo, wait).
                let (ms, turn_x, turn_y, turn_z, wait) = (arg(1).int(), arg(2), arg(3), arg(4), arg(6));
                let targets = self.matching(&arg(0));
                let mut flipped = Vec::new();
                // src: Muramasa.exe FUN_00446010 keeps the tempo (+0xa0) and the
                // turn's step FUN_0044f620 eases each angle by it (apply_tempo).
                let ease = tempo(&arg(5));
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    let (before, mirror) = (o.turn, o.mirror());
                    o.turn = (turn_x.position(o.turn.0, 0, 0), turn_y.position(o.turn.1, 0, 0), turn_z.position(o.turn.2, 0, 0));
                    if ms > 0 && o.turn.2 != before.2 {
                        self.set_tempo(ease, std::slice::from_ref(t));
                        if !self.shown.contains_key(t) {
                            self.pending_angle_from.entry(t.clone()).or_insert(-before.2);
                        }
                    }
                    let o = self.objects.get_mut(t).unwrap();
                    let turn = o.turn;
                    let tilted = turn.2 != before.2 && matches!(o.kind, Kind::Image { .. });
                    let text = matches!(o.kind, Kind::Text(_));
                    let edge = |t: (i64, i64, i64)| t.0.rem_euclid(180) == 90 || t.1.rem_euclid(180) == 90;
                    let turned = (turn.0, turn.1) != (before.0, before.1);
                    let over = o.mirror() != mirror;
                    if (over || tilted || turned || edge(turn) != edge(before)) && !text {
                        flipped.push(t.clone());
                    }
                    match &mut o.kind {
                        Kind::Text(text) => text.rotation = turn.2,
                        Kind::Image { .. } => {}
                        _ if turn.2.rem_euclid(360) != 0 => self.note("Rotate about z"),
                        _ => {}
                    }
                    if ms > 0 && turned && !text {
                        // (From or to edge-on it shows one side all the way.)
                        if over && !edge(before) && !edge(turn) {
                            self.note("Rotate over time turning a picture over: mirrored at once");
                        }
                        // It and its children squash as it turns (convert/turn.rs).
                        let turning = turn::Turning { target: t.clone(), from: (before.0, before.1), to: (turn.0, turn.1), ms: ms as u32, ease, zoom: None, fade_ms: 0 };
                        let prefix = format!("{t}/");
                        let children: Vec<String> = self.objects.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
                        for n in children.iter().chain([t]) {
                            self.pending_turn.insert(n.clone(), turning.clone());
                        }
                        flipped.extend(children);
                    }
                }
                // Only a picture that turns over or about z shows a change.
                if flipped.iter().any(|t| self.shown.contains_key(t) || self.pending_turn.contains_key(t)) {
                    self.timed(ms, &wait, &flipped);
                }
            }
            "Zoom" => {
                let (zx, zy) = (arg(2), arg(3));
                let targets = self.matching(&arg(0));
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    let was = o.zoom;
                    if arg(1).int() > 0 {
                        self.pending_size_from.entry(t.clone()).or_insert(was);
                    }
                    let o = self.objects.get_mut(t).unwrap();
                    // src: 0 not clamped (FUN_0044f2a0): drawn at size 0, nothing (sync).
                    o.zoom = (zx.position(o.zoom.0, 0, 0).max(0), zy.position(o.zoom.1, 0, 0).max(0));
                    if arg(1).int() > 0 && [was.0, was.1, o.zoom.0, o.zoom.1].contains(&0) {
                        self.pending_zoom_from.entry(t.clone()).or_insert(was);
                    }
                }
                self.set_tempo(tempo(&arg(4)), &targets);
                self.timed(arg(1).int(), &arg(5), &targets);
            }
            "DrawEffect" => {
                // (obj, ms, kind, from, to, tempo): a wave going `from` → `to`
                // px over `ms`, then held (the exe matches the name exactly, a
                // trailing space allowed). The phase's turns over the height
                // by name (`FUN_00430a70`).
                let period = match arg(2).string().trim_end_matches([' ', '\t']) {
                    "LowWave" => 64,
                    "MiddleWave" => 512,
                    "HighWave" => 1024,
                    "SuperWave" => 4096,
                    _ => {
                        self.note(format!("DrawEffect {}", arg(2).string().trim_end()));
                        return Value::Null;
                    }
                };
                let ms = arg(1).int().clamp(0, i64::from(i32::MAX)) as u32;
                let (from, to) = (arg(3).int().clamp(-1000, 1000) as i32, arg(4).int().clamp(-1000, 1000) as i32);
                let wave = Wave { start: self.sound_clock(), ms, period, from, to, ease: tempo(&arg(5)) };
                for t in self.matching(&arg(0)) {
                    let Some(o) = self.objects.get_mut(&t) else { continue };
                    o.wave = Some(wave);
                    // (A cover waves as a copy of the screen, drawn once it is shown.)
                    if self.objects.get(&t).is_some_and(|o| matches!(o.kind, Kind::Cover { .. }) && !self.grabs(&t, &Object { opacity: 1000, ..o.clone() })) {
                        self.objects.get_mut(&t).unwrap().wave = None;
                        self.note("DrawEffect wave on a Plain cover");
                        continue;
                    }
                    if let Some(s) = self.shown.get(&t)
                        && let Some(op) = wave.op(s.layer, wave.start, s)
                    {
                        self.ops.push(op);
                    }
                }
            }
            "SetShade" => {
                // (obj, level): the exe's names (`FUN_00465b10`, matched by the
                // sum of their letters); another name does nothing.
                let passes = match arg(1).string().to_ascii_uppercase().as_str() {
                    "EXTRALIGHT" => 1,
                    "LIGHT" => 2,
                    "MEDIUM" => 4,
                    "SEMIHEAVY" => 8,
                    "HEAVY" => 16,
                    "EXTRAHEAVY" => 24,
                    "NOMORE" => 32,
                    "NULL" | "" => 0,
                    _ => {
                        self.note(format!("SetShade {}", arg(1).string()));
                        return Value::Null;
                    }
                };
                for t in self.matching(&arg(0)) {
                    let o = self.objects.get_mut(&t).unwrap();
                    o.shade = (passes > 0).then(|| o.shade.unwrap_or(0) + passes);
                }
            }
            "SetTone" => {
                // (obj, tone): `null` takes it away.
                let t = arg(1).string().to_ascii_lowercase();
                let fs = tone(&t);
                if fs.is_none() && !matches!(t.as_str(), "" | "null") {
                    self.note(format!("SetTone {}", arg(1).string()));
                    return Value::Null;
                }
                for t in self.matching(&arg(0)) {
                    self.objects.get_mut(&t).unwrap().tone = fs.clone();
                }
            }
            "SetBlur" => {
                // (obj, on, passes, alpha, lag, wait): see `Object::trail`.
                // src: Muramasa.exe 00454ca4: `wait` "TRUE" draws the copies
                // (FUN_00456810) before the picture, anything else after it.
                // shortcut: a literal 1 counts as `true` (the Value can't tell them apart; the story passes none)
                let (passes, alpha) = (arg(2).int(), arg(3).int());
                let over = !matches!(arg(5), Value::Int(1));
                let trail = (arg(1).truthy() && (1..=128).contains(&passes) && (1..=1000).contains(&alpha))
                    .then(|| Trail { passes: passes as u32, alpha: alpha as u32, lag: arg(4).int().max(0) as u32, over });
                for t in self.matching(&arg(0)) {
                    let o = self.objects.get_mut(&t).unwrap();
                    // (A trail on screen ends; a new one is set at its next motion.)
                    if o.trail.is_some() && trail.is_none()
                        && let Some(sh) = self.shown.get(&t)
                    {
                        self.ops.push(Op::SpriteTrail { layer: sh.layer, passes: 0, alpha: 0, lag: 0, over: false });
                    }
                    self.objects.get_mut(&t).unwrap().trail = trail;
                }
            }
            "DrawTransition" => {
                // (obj, time, from, to, softness, tempo, mask, wait): a fade
                // to `to` shaped by the mask.
                // A mask that isn't there (ma04_018vs/020vs name a
                // slide_01_02_2.png cg.npa lacks): Muramasa.exe's step
                // (FUN_0044fe00) returns before the transition when its
                // loader (FUN_00488130) gives nothing, so nothing is drawn
                // (seen); the picture's opacity left as it was (inferred).
                let Some((mask, _, _)) = self.assets.image(&self.deref(&arg(6))) else {
                    (0..a.len()).for_each(|i| drop(arg(i)));
                    self.note(format!("DrawTransition mask missing: {}", arg(6).string()));
                    return Value::Null;
                };
                let (from, to) = (arg(2).int().clamp(0, 1000), arg(3).int().clamp(0, 1000));
                // A picture disused as it fades out goes now, as before its
                // fade was kept (md01_029vs:2107: `FadeDelete("絵色*", 400,
                // false)`, then their wipe).
                for t in self.matching(&arg(0)) {
                    if let Some(i) = self.disused_fading.iter().position(|d| d.0 == t) {
                        self.disused_fading.remove(i);
                        self.delete(&t);
                    }
                }
                let targets = self.matching(&arg(0));
                let softness = arg(4).int().max(0);
                let tempo = if arg(1).int() > 0 { self.game.fade_tempo.unwrap_or(|_| None)(&arg(5)) } else { None };
                // A picture: the engine's spfadeto, from `from` whatever it
                // showed (src: Muramasa.exe transition_start 0044fe00), its
                // opacity left (a later `Fade` multiplies the mask's table,
                // as fire01's in function_process.nss).
                let (pictures, others): (Vec<_>, Vec<_>) = targets.iter().cloned().partition(|t| matches!(self.objects[t].kind, Kind::Image { .. } | Kind::Strip(_)));
                let until = self.sound_clock() + arg(1).int().max(0) as u64;
                for t in &pictures {
                    let o = self.objects.get_mut(t).unwrap();
                    if to == 1000 {
                        o.opacity = 1000;
                    }
                    o.fade = Some(FadeTo { mask: mask.clone(), ms: arg(1).int().max(0) as u32, from, to, softness, ease: tempo, until });
                }
                // Anything else (a cover freezing the screen) is a print
                // through the mask from the screen before to the one after:
                // the threshold swept from 0 to 1000 or back (effectsoft).
                if !others.is_empty() {
                    let shown_at = |c: &Self, t: &String| match &c.objects[t].kind {
                        Kind::Cover { .. } => c.objects[t].opacity,
                        _ => c.shown.get(t).map_or(0, |s| s.opacity),
                    };
                    if others.iter().any(|t| shown_at(self, t) != from) || !matches!((from, to), (0, 1000) | (1000, 0)) {
                        self.note("DrawTransition from another opacity than the picture's");
                    }
                    for t in &others {
                        self.objects.get_mut(t).unwrap().opacity = to;
                    }
                    if tempo.is_some() {
                        self.fade_tempo = tempo;
                    }
                    if matches!((from, to), (0, 1000) | (1000, 0)) && softness > 0 {
                        self.transition_soft = Some((softness as u32, from > to));
                    }
                    self.pending_mask = Some(mask);
                }
                self.timed(arg(1).int(), &arg(7), &targets);
            }
            "Shake" => {
                // (obj, time, x0, y0, x1, y1, n, tempo, wait). A full-screen picture (or all
                // of them): the whole picture shakes.
                // 続・殺戮のジャンゴ (src: Django.exe FUN_00431de0, the
                // shake's frame FUN_0043f570) and 装甲悪鬼村正: the sign flips
                // every [`shake::exe_step`] ms, the size eased by the tempo
                // (the table FUN_00434ea0 picks).
                let (n, tempo) = (arg(6).int(), super::tempo(&arg(7)));
                // src: Muramasa.exe 004462d0, Django FUN_00431de0: n < 0x3e9
                if n > 1000 {
                    return Value::Null; // src: FUN_00431de0, 004462d0 (n < 0x3e9), nor waited for
                }
                // The English port (src: its scenes pass `Shake`'s frequency as (1000 / n) * 20,
                // Squirrel integers; CEffectShake 00484990/00484c60: a swing each frequency ms,
                // the first half as long, eased by the tempo, the size straight): the swings
                // as turns (shortcut: drawn as jumps, not eased; they last 1-2 frames).
                // src: the port's step 1000 / n × 20 (diesel_shake), else shake::exe_step (004462d0)
                let (step, ease) = if self.game.diesel_shake { (if n > 0 { (1000 / n * 20) as u32 } else { 16 }, None) } else { (shake::exe_step(n), tempo) }; // the port's script frequency (1000 / n) * 20; 16 (a frame) when n is 0, see above
                let targets = self.matching(&arg(0));
                if !targets.iter().any(|t| self.objects[t].w >= screen_w && self.objects[t].h >= screen_h) {
                    // The picture alone: now if it is on the screen, else
                    // when the next sync shows it. The exe keeps the signs: (x0, y0) × ±1.
                    let ms = arg(1).int().max(0) as u32;
                    let at = |x: i64, y: i64| (x as i32, y as i32);
                    let keys = shake::shake_keys(&[(ms, at(arg(2).int(), arg(3).int()), at(arg(4).int(), arg(5).int()), ease)], step);
                    for t in &targets {
                        match self.shown.get(t) {
                            Some(s) => self.ops.push(Op::SpriteMotion { layer: s.layer, keys: keys.clone(), looped: false }),
                            None => _ = self.shake_once.insert(t.clone(), keys.clone()),
                        }
                    }
                    self.timed(arg(1).int(), &last(), &targets);
                    return Value::Null;
                }
                // It lasts `ms` (or until the picture is deleted: shakes of
                // 36 minutes end that way); 0 ms or 0 pixels stops it. The
                // strength goes from (x, y) to args 4-5 (the exe's
                // shake_frame), nearly always 0: a jolt that dies down.
                let ms = arg(1).int().max(0) as u32;
                let (x, y) = (arg(2).int().abs() as i32, arg(3).int().abs() as i32);
                let end = (arg(4).int().abs() as i32, arg(5).int().abs() as i32);
                self.sync(0);
                if ms == 0 {
                    last(); // a 0 ms stop has nothing to wait for
                }
                // A shake may grow from 0 px (src: Muramasa.exe queue_shake 004462d0:
                // the size goes from args 2-3 to 4-5 over the time).
                if ms > 0 && (x > 0 || y > 0 || end != (0, 0)) {
                    let range = [(arg(2).int() as i32, arg(3).int() as i32), (arg(4).int() as i32, arg(5).int() as i32)];
                    let shake = ScreenEffect::Jolt { time_ms: ms, interval_ms: step, range, ease, end_ms: ms };
                    self.ops.push(Op::ScreenEffect { slot: 1, effect: Some(shake) });
                    self.shaken = targets;
                    if !matches!(last(), Value::Null | Value::Int(0)) {
                        self.ops.push(Op::Wait { ms });
                    }
                } else if !self.shaken.is_empty() {
                    self.shaken.clear();
                    self.ops.push(Op::ScreenEffect { slot: 1, effect: None });
                }
            }
            "ImageHorizon" => return Value::Int(self.objects.get(&self.object_name(&arg(0))).map_or(0, |o| o.w)),
            "ImageVertical" => return Value::Int(self.objects.get(&self.object_name(&arg(0))).map_or(0, |o| o.h)),
            "SetFont" => {
                self.font_size = arg(1).int().max(1);
                // (face, size, colour, shadow colour, weight, shadow): kept for CreateText.
                self.font = (0..a.len().min(6)).map(arg).collect();
            }
            "CreateText" => self.create_text(&arg),
            "LoadText" if a.len() == 6 => {
                let layout = (arg(2).int(), arg(3).int(), arg(4).int(), arg(5).int(), self.font_size);
                self.box_layouts.insert(self.object_name(&arg(1)), layout);
            }
            "LoadText" => {
                let name = self.object_name(&arg(2));
                let font = self.font_size;
                if let Some(Object { kind: Kind::Text(t), .. }) = self.objects.get_mut(&name) {
                    t.layout = Some((arg(3).int(), arg(4).int(), arg(5).int(), arg(6).int(), font));
                }
            }
            "Wait" if self.process_clock.is_some() => self.process_wait(arg(0).int().max(0) as u32),
            "WaitAction" if self.process_clock.is_some() => {
                let ms = std::mem::take(&mut self.process_fade);
                self.process_wait(ms);
            }
            "WaitPlay" if self.process_clock.is_some() => {
                // Until the sound's end.
                for t in self.matching(&arg(0)) {
                    let path = match &self.objects[&t].kind {
                        Kind::Sound(s) => s.path.clone(),
                        _ => continue,
                    };
                    match self.assets.sound_ms(&path) {
                        Some(ms) => self.process_wait(ms),
                        None => self.note("WaitPlay in a process for a sound of unknown length"),
                    }
                }
            }
            // In a process: it waits, not the script ([`Compiler::process_lead`]).
            "Wait" if self.waits_alone() => self.process_lead += arg(0).int().max(0) as u32,
            "WaitAction" | "WaitFade" | "WaitMove" | "WaitZoom" if self.waits_alone() => {
                let ms = match arg(0) {
                    Value::Null => self.pending_ms,
                    a => self.matching(&a).iter().filter_map(|t| self.pending_by.get(t)).copied().max().unwrap_or(0),
                };
                self.process_lead = self.process_lead.max(ms);
            }
            "Wait" => {
                // The actions still running are shown over the wait, at
                // most.
                let at = self.ops.len();
                let ms = self.movie_wait(arg(0).int().max(0) as u32);
                let shown = self.take_running(ms);
                self.sync(shown);
                if ms > shown {
                    self.ops.push(Op::Wait { ms: ms - shown });
                }
                // The exe's Wait: a click doesn't end it, skip does (0040bd80).
                for op in &mut self.ops[at..] {
                    if let Op::Wait { ms } = *op {
                        *op = Op::Pause { ms };
                    }
                }
            }
            // Auto mode ends it after 1.5 s (the original's trace: every
            // `WaitKey(2000)` and `WaitKey()` of ma00_000 at 1505-1527 ms).
            "WaitKey" => match arg(0) {
                Value::Null => {
                    self.sync(0);
                    self.ops.push(Op::AutoWait { ms: WAITKEY_AUTO_MS });
                    self.ops.push(Op::ClickWait { skippable: true });
                }
                ms => {
                    // As `Wait`: the actions still running count in its
                    // time (a fade out, then `WaitKey` a little longer).
                    let ms = self.movie_wait(ms.int().max(0) as u32);
                    let shown = self.take_running(ms);
                    self.sync(shown);
                    if ms > shown {
                        let auto = WAITKEY_AUTO_MS.saturating_sub(shown);
                        if ms - shown > auto {
                            self.ops.push(Op::AutoWait { ms: auto });
                        }
                        self.ops.push(Op::Wait { ms: ms - shown });
                    }
                }
            },
            "WaitPlay" => {
                // Until the movie's end: what played as a movie has ended.
                // Until a sound's end, or at most the limit in ms (the exe:
                // `null`/`true` or none waits without one).
                let limit = match arg(1) {
                    Value::Int(ms) => Some(ms.max(0) as u32),
                    _ => None,
                };
                let mut wait = 0;
                for t in self.matching(&arg(0)) {
                    if let Kind::Sound(s) = &self.objects[&t].kind {
                        if s.playing.is_none() && !s.starting {
                            // Fading out to 0: until the fade's end.
                            if let Some(left) = self.fade_left(&t) {
                                wait = wait.max(limit.map_or(left, |l| l.min(left)));
                            }
                            continue;
                        }
                        if s.looped {
                            if limit.is_none() {
                                self.note("WaitPlay on a looping sound");
                            }
                            wait = wait.max(limit.unwrap_or(0));
                            continue;
                        }
                        // A starting sound's length is read when it starts.
                        let left = if s.starting { self.assets.sound_ms(&s.path.clone()) } else { self.sound_left(&t) };
                        match left {
                            Some(ms) => wait = wait.max(limit.map_or(ms, |l| l.min(ms))),
                            None => self.note("WaitPlay on a sound of unknown length"),
                        }
                        continue;
                    }
                    self.play_movie(&t);
                    if let Some(Object { kind: Kind::Movie(m), .. }) = self.objects.get_mut(&t) {
                        m.credit = 0;
                    }
                }
                if wait > 0 {
                    // As `Wait`: the actions still running are shown over it.
                    let ms = self.movie_wait(wait);
                    let shown = self.take_running(ms);
                    self.sync(shown);
                    if ms > shown {
                        self.ops.push(Op::Wait { ms: ms - shown });
                    }
                }
            }
            "WaitAction" if self.matching(&arg(0)).iter().any(|t| matches!(self.objects[t].kind, Kind::Movie(_))) => {
                for t in self.matching(&arg(0)) {
                    self.play_movie(&t);
                }
            }
            // src: Django.exe builtin_Draw 0040ba10, Muramasa.exe 0040f470: one
            // frame (waits at most 4 ms), the running actions go on: shown as
            // where the script doesn't wait (up to UNWAITED_MS).
            "Draw" => self.sync(0),
            "WaitAction" | "WaitFade" | "WaitMove" | "WaitZoom" => {
                // Waits for the actions: their whole time. Given an object,
                // only for that object's (`CP_EHPChange` waits on its own
                // process, not on the 15 s zoom started before it).
                let ms = if matches!(arg(0), Value::Null) {
                    self.take_running(u32::MAX)
                } else {
                    self.matching(&arg(0)).iter().filter_map(|t| self.pending_by.get(t)).copied().max().unwrap_or(0)
                };
                self.sync(ms);
                // An action the engine still runs (one the script went on
                // from): waited for to its end.
                if !matches!(arg(0), Value::Null) {
                    for t in self.matching(&arg(0)) {
                        // (Ended: a 0 ms move after it stops nothing.)
                        self.runs.remove(&t);
                        if self.unwaited_motions.remove(&t) && let Some(s) = self.shown.get(&t) {
                            self.ops.push(Op::MotionWait { layer: s.layer });
                        }
                    }
                }
            }
            // In a music process, on a sound playing (not one it starts): on
            // its clock, which only a WaitAction on it moves (EndSong fades
            // out a song that has ended).
            "SetVolume" if self.process_clock.is_some() && self.matching(&arg(0)).iter().all(|t| matches!(&self.objects[t].kind, Kind::Sound(s) if s.playing.is_some() && !s.starting)) => {
                // (A sound effect's fade would be lost: the process is refused.)
                if self.matching(&arg(0)).iter().any(|t| matches!(&self.objects[t].kind, Kind::Sound(s) if !s.bgm)) {
                    self.music_refused = true;
                }
                self.process_fade = arg(1).int().max(0) as u32;
            }
            "SetVolume" => {
                // src: Muramasa.exe sound_set_volume 00475f50: time 0 sets the volume at
                // once; only a fade builds its steps by the tempo (00478570).
                self.set_volume(&arg(0), arg(1).int(), arg(2).int(), tempo(&arg(3)))
            }
            "SetPan" => self.set_pan(&arg(0), arg(1).int(), arg(2).int()),
            "SetLoopPoint" => {
                let points = (arg(1).int().max(0) as u32, arg(2).int().max(0) as u32);
                for t in self.matching(&arg(0)) {
                    if let Kind::Sound(s) = &mut self.objects.get_mut(&t).unwrap().kind {
                        // (0, 999999): the whole sound.
                        s.loop_ms = Some(points).filter(|&(a, b)| a < b && b < 999999);
                    }
                }
            }
            "SetLoop" => {
                let looped = arg(1).truthy();
                for t in self.matching(&arg(0)) {
                    if let Kind::Sound(s) = &mut self.objects.get_mut(&t).unwrap().kind {
                        s.looped = looped;
                    }
                }
            }
            "SetFrequency" => self.set_frequency(&arg(0), arg(1).int(), arg(2).int(), tempo(&arg(3))),
            // A sound: its length (it has only just started); played
            // through for PassageTime, whose busy loops (mc01_017vs waits
            // for its song's end) can't be waited out here.
            "RemainTime" | "DurationTime" | "PassageTime" => {
                let sound = self.matching(&arg(0)).iter().find_map(|t| match &self.objects[t].kind {
                    Kind::Sound(s) => Some(s.path.clone()),
                    _ => None,
                });
                if let Some(ms) = sound.and_then(|p| self.assets.sound_ms(&p)) {
                    return Value::Int(ms as i64);
                }
                if name == "PassageTime" {
                    return Value::Int(0);
                }
                let movie = self.matching(&arg(0)).iter().find_map(|t| match &self.objects[t].kind {
                    Kind::Movie(m) => Some(m.clone()),
                    _ => None,
                });
                return Value::Int(movie.map_or(0, |m| match (name, m.state) {
                    ("DurationTime", _) | (_, MovieState::Playing { at: 0 } | MovieState::Paused { at: 0 }) => m.info.ms,
                    (_, MovieState::Playing { at } | MovieState::Paused { at }) => m.info.ms.saturating_sub(at),
                    (_, MovieState::Played) => 0,
                }) as i64);
            }
            "Conquest" => {
                // (file, scene, mark): read marks for the skip; the engine keeps its own.
                read.set(u64::MAX);
                self.note(CONQUEST);
                return Value::Int(0);
            }
            "Random" => return self.random(arg(0).int()),
            "ModuleFileName" => return Value::Str(self.file.to_string()),
            "Strstr" => return Value::Int(i64::from(arg(0).string().contains(&arg(1).string()))),
            "Integer" => return Value::Int(arg(0).int()),
            // Drawing and bookkeeping that changes nothing on screen.
            "SetVertex" => {
                // (name, x: left | center | right | px, y: top | middle |
                // bottom | px), from the object's top left: the point a
                // `Zoom` or `Rotate` keeps in place (builtin_SetVertex).
                // src: Django.exe FUN_004666c0, Muramasa.exe FUN_004891e0: the
                // names per axis; px outside 0..size leaves that axis as it
                // was (dja004's `center,704` on a 600 px picture).
                // Lower case only: nothing lowers the name first (src: Muramasa.exe
                // eval_arg 00413740 copies it, FUN_004891e0 compares 'c','e',…), so
                // `Center`/`Middle` are atol's 0.
                let place = |v: &Value, size: i64, was: i64, [near, mid, far]: [&str; 3]| match v.string().as_str() {
                    w if w == near => 0,
                    w if w == mid => size / 2,
                    w if w == far => size,
                    _ => Some(v.int()).filter(|p| (0..=size).contains(p)).unwrap_or(was),
                };
                let (vx, vy) = (arg(1), arg(2));
                for t in self.matching(&arg(0)) {
                    let o = self.objects.get_mut(&t).unwrap();
                    let (wx, wy) = o.vertex.unwrap_or((o.w / 2, o.h / 2));
                    o.vertex = Some((place(&vx, o.w, wx, ["left", "center", "right"]), place(&vy, o.h, wy, ["top", "middle", "bottom"])));
                }
            }
            "WaitText" | "LockVideo" | "Reset" | "SetShortcut" => {}
            "SetBacklog" => {
                // (text, voice or "null", speaker)
                let text = arg(0).string();
                let voice = Some(arg(1).string()).filter(|v| !v.is_empty() && v != "null").and_then(|v| self.assets.sound(&v));
                let character = self.backlog_character(&arg(2).string());
                // A choice's option known only at run time (its line in
                // the log): each text the flag can hold.
                if let Some((var, before, after)) = super::runtime_text(&text) {
                    let texts = self.flag_texts.get(&var).map(|t| t.iter().map(|(&c, s)| (c, format!("{before}{s}{after}"))).collect()).unwrap_or_default();
                    self.ops.push(Op::RuntimeBacklogLine { text: format!("{before}{after}"), runtime: conv_core::ir::RuntimeText { var, texts } });
                } else if !text.is_empty() {
                    self.ops.push(Op::BacklogLine { text, voice, character });
                }
            }
            // src: Django.exe load_font_object 004308b0: pre-renders the listed
            // glyphs into a font object no text names; draws nothing.
            "LoadFont" => read.set(u64::MAX),
            "CreateProcess" => {
                let name = self.object_name(&arg(0));
                self.create_process(name, &arg(4).string());
            }
            // Noted whole: no note per argument.
            other if BUILTINS.contains(&other) => {
                read.set(u64::MAX);
                if self.file == "function_select.nss" {
                    self.note(CHOICE_SCREEN);
                } else {
                    self.note(format!("built-in {other}"));
                }
            }
            // No built-in and no NSS function has this name: the exe skips
            // the call (`Waitkey(500)`, `SL_Rightdown(…)`).
            other => {
                read.set(u64::MAX);
                self.note(format!("skipped: no function {other}"));
            }
        }
        Value::Null
    }
}

/// The filters drawing a tone `CreateEffect` and `SetTone` name (lower case).
/// The note on a movie call whose file isn't there: lost, or (no movie
/// folder in this build, such as the snapshot's dry run) unmeasured.
fn missing_movie(call: &str, have_movies: bool) -> String {
    if have_movies { format!("{call} of a missing movie") } else { format!("{call}: no movies in this build") }
}

/// A built-in's channel in `metrics.json`'s score (Muramasa's and
/// Django's kinds; a port's `port X` as X).
pub fn channel(kind: &str) -> Channel {
    match kind.strip_prefix("port ").unwrap_or(kind) {
        "LoadText" | "CreateText" | "CreateChoice" | "SetChoice" | "SetBacklog" | "CreateName" | "SetFont" | "LoadFont" | "ShowVOICESEText" | "Print" | "SetNextFocus" => Channel::Text,
        "CreateSound" | "SetVolume" | "SetFrequency" | "SetLoop" | "SetLoopPoint" | "SetPan" => Channel::Sound,
        "CreateTexture" | "CreateTextureadd" | "CreateTextureEX" | "CreateTextureSP" | "CreateColor" | "CreateColorSaddP" | "LoadColor" | "Delete" | "DeleteFw" | "CreateWindow" | "LoadImage" | "CreateMask" | "CreateStencil"
        | "CreateSurface" | "SetSurface" | "SetTone" | "SetShade" | "SetVertex" | "SetBlur" | "Position" | "ImageHorizon" | "ImageVertical" | "CreateCamera" | "Draw" | "Boxframe" | "Request" | "LockVideo" => Channel::Picture,
        "Fade" | "Move" | "BezierMove" | "Zoom" | "Rotate" | "Shake" | "ShakeOne" | "DrawTransition" | "DrawEffect" | "CreateEffect" | "CreateProcess" | "MoveCamera" => Channel::Motion,
        "Wait" | "WaitAction" | "WaitFade" | "WaitKey" | "Waitkey" | "WaitPlay" | "WaitZoom" | "WaitMove" | "WaitText" => Channel::Timing,
        "CreateMovie" | "MoviePlay" => Channel::Movie,
        "SetAlias" | "Strstr" | "Random" | "RandomMeimetsu" | "ModuleFileName" | "Save" | "ClearBacklog" | "ClearScore" | "Time" | "Conquest" | "ReConquest" | "PassageTime" | "RemainTime" | "CursorPosition" | "MoveCursor" => Channel::None,
        _ => Channel::Unclassed,
    }
}

fn tone(name: &str) -> Option<Vec<SpriteFilter>> {
    use conv_core::ir::{FILTER_BLEND_ADD, FILTER_BLEND_COPY, FILTER_COLOUR, FILTER_GREY_WEIGHTS, FILTER_NEGATIVE};
    let filter = |kind, blend, params: [i32; 3]| SpriteFilter { kind: Operand::Const(kind), blend: Operand::Const(blend), strength: Operand::Const(256), params: params.map(Operand::Const) };
    // src: exe 0043fcc0 (the tables: 28 r + 151 g + 77 b, each /256 rounded down; the
    // exe reads its BGRA pixels as RGB, so red and blue weigh the other way round)
    let grey = filter(FILTER_GREY_WEIGHTS, FILTER_BLEND_COPY, [28, 151, 77]);
    // The grey plus (r, g, b), each at most 255.
    let plus = |rgb| filter(FILTER_COLOUR, FILTER_BLEND_ADD, rgb);
    Some(match name {
        // src: exe 004403d0 (Sepia's draw: b grey, g and r grey + 30)
        "sepia" => vec![grey, plus([30, 30, 0])],
        // src: exe 004401d0
        "monochrome" => vec![grey],
        "negaposi" => vec![filter(FILTER_NEGATIVE, FILTER_BLEND_COPY, [0; 3])],
        // src: exe 004405e0 (KitanoBlue's draw: r grey, g grey + 20, b grey + 80)
        "kitanoblue" => vec![grey, plus([0, 20, 80])],
        _ => return None,
    })
}

/// `SetShade`'s `passes` of the exe's blur, each the mean of the 3×3
/// pixels about one (`FUN_00465b10`), at `scale` per mille: one box blur
/// of the same spread (a pass adds 2/3 px² to the variance, a box of
/// radius r gives r(r + 1)/3). The engine's blur keeps the alpha; the
/// exe's blurs it too.
pub(super) fn shade_blur(passes: i64, scale: (i64, i64)) -> Vec<SpriteFilter> {
    use conv_core::ir::{FILTER_BLEND_COPY, FILTER_BLUR};
    let r = ((1.0 + 8.0 * passes as f64).sqrt() - 1.0) / 2.0;
    let px = |k: i64| (r * k as f64 / 1000.0).round() as i32;
    let (rx, ry) = (px(scale.0), px(scale.1));
    if rx <= 0 && ry <= 0 {
        return Vec::new();
    }
    let f = SpriteFilter { kind: Operand::Const(FILTER_BLUR), blend: Operand::Const(FILTER_BLEND_COPY), strength: Operand::Const(256), params: [rx.max(0), ry.max(0), 0].map(Operand::Const) };
    vec![f]
}
