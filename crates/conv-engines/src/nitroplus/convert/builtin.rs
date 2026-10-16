//! NSS's built-in functions, as the compiler runs them ([`Compiler::builtin`]).

use std::cell::Cell;

use super::*;

/// Arguments the compiler doesn't read that change nothing it draws or
/// plays: (built-in, index, the value covered (`"null"`: NSS's null) or
/// `None` for any), why.
const UNREAD_HARMLESS: &[(&str, usize, Option<&str>, &str)] = &[
    ("Fade", 3, Some("null"), "no tempo: linear, as ours"),
    ("Move", 4, Some("null"), "no tempo: linear, as ours"),
    ("Zoom", 4, Some("null"), "no tempo: linear, as ours"),
    ("Rotate", 5, Some("null"), "no tempo: linear, as ours"),
    ("SetVolume", 3, Some("null"), "no tempo: linear, as ours"),
    ("SetPan", 3, Some("null"), "no tempo: linear, as ours"),
    ("DrawTransition", 5, Some("null"), "no tempo: linear, as ours"),
    ("CreateProcess", 1, None, "a process draws nothing: its priority is unused"),
    ("CreateProcess", 2, None, "a process draws nothing: its place is unused"),
    ("CreateProcess", 3, None, "a process draws nothing: its place is unused"),
];

impl Compiler<'_, '_> {
    /// A built-in call; every argument passed and not read gets a note
    /// (`<Builtin> argument <i> ignored`), so a call converted only in part
    /// shows in the dry run's list.
    pub(super) fn builtin(&mut self, name: &str, a: &[Value]) -> Value {
        let read = Cell::new(0u64);
        self.forget_ended_sounds();
        let value = self.builtin_reading(name, a, &read);
        for (i, v) in a.iter().enumerate().take(64) {
            let harmless = UNREAD_HARMLESS.iter().any(|&(b, k, value, _)| b == name && k == i && value.is_none_or(|x| if x == "null" { matches!(v, Value::Null) } else { v.string() == x }));
            if read.get() & (1 << i) == 0 && !harmless {
                self.note(format!("{name} argument {i} ignored"));
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
            "LoadImage" if self.game.loaded_textures => {
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
                let src = self.deref(&arg(4));
                let Some((image, w, h)) = self.texture_source(&src) else {
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
            "CreateColor" => {
                let name = self.object_name(&arg(0));
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
                let name = self.object_name(&arg(0));
                let kind = if arg(6).string().eq_ignore_ascii_case("plain") {
                    Kind::Cover
                } else {
                    self.note(format!("CreateEffect {}", arg(6).string()));
                    Kind::Other
                };
                self.create(name, kind, arg(1).int(), screen_w, screen_h);
            }
            // (name, x, y, zoom per mille). With Game::relative_children
            // only (装甲悪鬼村正's cameras aren't checked).
            "CreateCamera" if self.game.relative_children => {
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
                }
                if children.is_empty() {
                    return Value::Null;
                }
                self.timed(arg(1).int(), &arg(6), &children);
            }
            "CreateWindow" | "CreateName" => {
                let name = self.object_name(&arg(0));
                let relative = self.game.relative_children;
                let o = self.create(name, Kind::Other, arg(1).int(), 0, 0);
                o.container = true;
                // (name, priority, x, y, w, h, ?): a text in the window
                // starts at its corner (続・殺戮のジャンゴ's boxes); its
                // children are drawn only inside it.
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
                let path = self.deref(&arg(6));
                let Some(info) = self.assets.movie_info(&path) else {
                    self.note("CreateMovie of a missing movie");
                    self.create(name, Kind::Other, arg(1).int(), 0, 0);
                    return Value::Null;
                };
                let alpha = arg(5).truthy();
                let (w, h) = (info.w as i64, if alpha { info.h as i64 / 2 } else { info.h as i64 });
                let m = MovieObject { path, looped: arg(4).truthy(), alpha, dark: false, info, state: MovieState::Playing { at: 0 }, credit: 0 };
                let o = self.create(name, Kind::Movie(m), arg(1).int(), w, h);
                o.x = arg(2).position(0, w, screen_w);
                o.y = arg(3).position(0, h, screen_h);
            }
            "MoviePlay" => {
                // Full screen, the script waiting. A movie with sound stops
                // the music; a silent cut lets it play on.
                let path = self.deref(&arg(0));
                self.sync(0);
                let Some(info) = self.assets.movie_info(&path) else {
                    // `mv_smg_PV` (the title's): no such file on the disc.
                    self.note("MoviePlay of a missing movie");
                    return Value::Null;
                };
                let skippable = arg(1).truthy();
                if let Some(file) = self.assets.movie_file(&path, info.sound) {
                    self.ops.push(if info.sound { Op::Movie { file, skippable } } else { Op::SceneMovie { file, skippable } });
                }
            }
            "CreateSound" => {
                let name = self.object_name(&arg(0));
                let bgm = arg(1).string().eq_ignore_ascii_case("bgm");
                let s = SoundObject { path: self.named_string(&arg(2)), bgm, volume: 1000, looped: false, starting: false, fade_ms: 0, playing: None, disused: false, loop_ms: None, started_at: 0, len_ms: None, pan: 0, rate: None, freq: 1000 };
                self.create(name, Kind::Sound(s), 0, 0, 0);
            }
            "Fade" => {
                // Four arguments: no tempo.
                let wait = if a.len() == 4 { arg(3) } else { arg(4) };
                let (ms, to) = (arg(1).int(), arg(2));
                let targets = self.matching(&arg(0));
                let mut window_only = !targets.is_empty();
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    o.opacity = to.position(o.opacity, 0, 0).clamp(0, 1000);
                    window_only &= o.hideable;
                }
                // The text window's own fades aren't waited for: the window
                // just shows or hides.
                if !window_only {
                    self.timed(ms, &wait, &targets);
                } else if !matches!(wait, Value::Null | Value::Int(0)) {
                    self.check_window();
                }
            }
            "Move" => {
                let targets = self.matching(&arg(0));
                let (ms, to_x, to_y) = (arg(1).int(), arg(2), arg(3));
                // (In a process its wait holds only the process: `timed`.)
                let blocking = !matches!(arg(5), Value::Null | Value::Int(0)) && self.action_process.is_none();
                // One picture on screen moving while the script waits: a real
                // motion (after showing what changed before it).
                if ms > 0 && blocking && targets.len() == 1 && self.shown.contains_key(&targets[0]) {
                    self.sync(0);
                    let trail = self.objects[&targets[0]].trail;
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
                            self.ops.push(Op::SpriteTrail { layer: sh.layer, passes: t.passes, alpha: t.alpha, lag: t.lag });
                        }
                        self.ops.push(Op::SpriteMove { layer: sh.layer, x: cx, y: cy, ms: ms as u32 });
                    }
                    return Value::Null;
                }
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    o.x = to_x.position(o.x, o.w, screen_w);
                    o.y = to_y.position(o.y, o.h, screen_h);
                }
                self.timed(ms, &arg(5), &targets);
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
                if targets.is_empty() && action == "enter" {
                    self.pending_enter = Some(arg(0).string().trim_start_matches('@').to_string());
                }
                for t in targets {
                    self.request(&t, &action);
                }
            }
            "SetAlias" => {
                let (obj, alias) = (self.object_name(&arg(0)), self.var_string(&arg(1)));
                if obj != alias {
                    self.aliases.insert(alias, obj);
                }
            }
            "Rotate" => {
                // (obj, time, x, y, z degrees, tempo, wait).
                let (ms, turn_x, turn_y, turn_z, wait) = (arg(1).int(), arg(2), arg(3), arg(4), arg(6));
                let targets = self.matching(&arg(0));
                let mut flipped = Vec::new();
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    let (before, mirror) = (o.turn, o.mirror());
                    o.turn = (turn_x.position(o.turn.0, 0, 0), turn_y.position(o.turn.1, 0, 0), turn_z.position(o.turn.2, 0, 0));
                    let (turn, mirrored) = (o.turn, o.mirror() != (false, false));
                    let tilted = turn.2 != before.2 && matches!(o.kind, Kind::Image { .. });
                    let text = matches!(o.kind, Kind::Text(_));
                    let edge = |t: (i64, i64, i64)| t.0.rem_euclid(180) == 90 || t.1.rem_euclid(180) == 90;
                    if (o.mirror() != mirror || tilted || edge(turn) != edge(before)) && !text {
                        flipped.push(t.clone());
                    }
                    match &mut o.kind {
                        Kind::Text(text) => text.rotation = turn.2,
                        Kind::Image { .. } => {}
                        _ if turn.2.rem_euclid(360) != 0 => self.note("Rotate about z"),
                        Kind::Movie(_) if mirrored => self.note("Rotate of a movie"),
                        _ => {}
                    }
                    if ms > 0 && (turn.0, turn.1) != (before.0, before.1) && !text {
                        self.note("Rotate over time drawn at once");
                    }
                }
                // Only a picture that turns over or about z shows a change.
                if flipped.iter().any(|t| self.shown.contains_key(t)) {
                    self.timed(ms, &wait, &flipped);
                }
            }
            "Zoom" => {
                let (zx, zy) = (arg(2), arg(3));
                let targets = self.matching(&arg(0));
                for t in &targets {
                    let o = self.objects.get_mut(t).unwrap();
                    o.zoom = (zx.position(o.zoom.0, 0, 0).max(1), zy.position(o.zoom.1, 0, 0).max(1));
                }
                self.timed(arg(1).int(), &arg(5), &targets);
            }
            "SetBlur" => {
                // (obj, on, passes, alpha, lag, wait): see `Object::trail`.
                let (passes, alpha) = (arg(2).int(), arg(3).int());
                let trail = (arg(1).truthy() && (1..=128).contains(&passes) && (1..=1000).contains(&alpha))
                    .then(|| Trail { passes: passes as u32, alpha: alpha as u32, lag: arg(4).int().max(0) as u32 });
                for t in self.matching(&arg(0)) {
                    let o = self.objects.get_mut(&t).unwrap();
                    // (A trail on screen ends; a new one is set at its next motion.)
                    if o.trail.is_some() && trail.is_none()
                        && let Some(sh) = self.shown.get(&t)
                    {
                        self.ops.push(Op::SpriteTrail { layer: sh.layer, passes: 0, alpha: 0, lag: 0 });
                    }
                    self.objects.get_mut(&t).unwrap().trail = trail;
                }
            }
            "DrawTransition" => {
                // (obj, time, from, to, softness, tempo, mask, wait): a fade
                // to `to` shaped by the mask.
                let to = arg(3).int().clamp(0, 1000);
                let targets = self.matching(&arg(0));
                for t in &targets {
                    self.objects.get_mut(t).unwrap().opacity = to;
                }
                match self.assets.image(&self.deref(&arg(6))) {
                    Some((mask, _, _)) => self.pending_mask = Some(mask),
                    None => self.note(format!("DrawTransition mask missing: {}", arg(6).string())),
                }
                self.timed(arg(1).int(), &arg(7), &targets);
            }
            "Shake" => {
                // (obj, time, x0, y0, x1, y1, n, tempo, wait). A full-screen picture (or all
                // of them): the whole picture shakes.
                let targets = self.matching(&arg(0));
                if !targets.iter().any(|t| self.objects[t].w >= screen_w && self.objects[t].h >= screen_h) {
                    // The picture alone: now if it is on the screen, else
                    // when the next sync shows it.
                    let size = |x: i64, y: i64| (x.abs() as i32, y.abs() as i32);
                    let keys = shake::shake_keys(&[(arg(1).int().max(0) as u32, size(arg(2).int(), arg(3).int()), size(arg(4).int(), arg(5).int()))]);
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
                if ms > 0 && (x > 0 || y > 0) {
                    let shake =
                        ScreenEffect::Shake { time_ms: ms, interval_ms: 16, path: false, speed: (1300, 1700), range: [(x, y), end], end_ms: ms };
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
            "Wait" => {
                // The actions still running are shown over the wait, at
                // most.
                let ms = self.movie_wait(arg(0).int().max(0) as u32);
                let shown = std::mem::take(&mut self.pending_ms).min(ms);
                self.sync(shown);
                if ms > shown {
                    self.ops.push(Op::Wait { ms: ms - shown });
                }
            }
            "WaitKey" => {
                self.sync(0);
                match arg(0) {
                    Value::Null => self.ops.push(Op::ClickWait { skippable: true }),
                    ms => {
                        let ms = self.movie_wait(ms.int().max(0) as u32);
                        if ms > 0 {
                            self.ops.push(Op::Wait { ms });
                        }
                    }
                }
            }
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
                    let shown = std::mem::take(&mut self.pending_ms).min(ms);
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
            "WaitAction" | "WaitFade" | "WaitMove" | "WaitZoom" | "Draw" => {
                // Waits for the actions: their whole time. Given an object,
                // only for that object's (`CP_EHPChange` waits on its own
                // process, not on the 15 s zoom started before it).
                let ms = if name == "Draw" || matches!(arg(0), Value::Null) {
                    std::mem::take(&mut self.pending_ms)
                } else {
                    self.matching(&arg(0)).iter().filter_map(|t| self.pending_by.get(t)).copied().max().unwrap_or(0)
                };
                self.sync(ms);
            }
            "SetVolume" => self.set_volume(&arg(0), arg(1).int(), arg(2).int()),
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
            "SetFrequency" => self.set_frequency(&arg(0), arg(1).int(), arg(2).int()),
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
            "Conquest" | "Random" => return Value::Int(0),
            "ModuleFileName" => return Value::Str(self.file.to_string()),
            "Strstr" => return Value::Int(i64::from(arg(0).string().contains(&arg(1).string()))),
            "Integer" => return Value::Int(arg(0).int()),
            // Drawing and bookkeeping that changes nothing on screen.
            "SetVertex" => {
                // (name, x: left | center | right | px, y: top | middle |
                // bottom | px), from the object's top left: the point a
                // `Zoom` or `Rotate` keeps in place (builtin_SetVertex).
                let place = |v: &Value, size: i64, near: &str, far: &str| match v {
                    Value::Str(w) if w.eq_ignore_ascii_case(near) => 0,
                    Value::Str(w) if w.eq_ignore_ascii_case(far) => size,
                    Value::Str(w) if w.eq_ignore_ascii_case("center") || w.eq_ignore_ascii_case("middle") => size / 2,
                    v => v.int(),
                };
                let (vx, vy) = (arg(1), arg(2));
                for t in self.matching(&arg(0)) {
                    let o = self.objects.get_mut(&t).unwrap();
                    o.vertex = Some((place(&vx, o.w, "left", "right"), place(&vy, o.h, "top", "bottom")));
                }
            }
            "WaitText" | "LockVideo" | "Reset" | "SetShortcut" => {}
            "SetBacklog" => {
                // (text, voice or "null", speaker)
                let text = arg(0).string();
                let voice = Some(arg(1).string()).filter(|v| !v.is_empty() && v != "null").and_then(|v| self.assets.sound(&v));
                // A choice's option known only at run time (its line in
                // the log): each text the flag can hold.
                if let Some((var, before, after)) = super::runtime_text(&text) {
                    let texts = self.flag_texts.get(&var).map(|t| t.iter().map(|(&c, s)| (c, format!("{before}{s}{after}"))).collect()).unwrap_or_default();
                    self.ops.push(Op::RuntimeBacklogLine { text: format!("{before}{after}"), runtime: conv_core::ir::RuntimeText { var, texts } });
                } else if !text.is_empty() {
                    self.ops.push(Op::BacklogLine { text, voice });
                }
            }
            "CreateProcess" => {
                let name = self.object_name(&arg(0));
                self.create_process(name, &arg(4).string());
            }
            // Noted whole: no note per argument.
            other if BUILTINS.contains(&other) => {
                read.set(u64::MAX);
                self.note(format!("built-in {other}"));
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
