//! Turning the objects' changes into ops at each wait ([`Compiler::sync`]).

use super::*;

impl Compiler<'_, '_> {
    /// Shows the changes since the last sync, as a cross-fade over `ms`,
    /// or the pending actions' time up to [`UNWAITED_MS`] if longer.
    pub(super) fn sync(&mut self, ms: u32) {
        let blocking = ms;
        let ms = ms.max(std::mem::take(&mut self.pending_ms).min(UNWAITED_MS));
        let timed = std::mem::take(&mut self.pending_by);
        let tempo = std::mem::take(&mut self.pending_tempo);
        let turning = std::mem::take(&mut self.pending_turn);
        let fades = std::mem::take(&mut self.pending_fade);
        let mut shake_once = std::mem::take(&mut self.shake_once);
        let mask = self.pending_mask.take();
        self.flush_sounds();
        self.end_one_shots();
        self.check_window();
        if !self.shaken.is_empty() && !self.shaken.iter().any(|t| self.objects.get(t).is_some_and(|o| o.opacity > 0)) {
            self.shaken.clear();
            self.ops.push(Op::ScreenEffect { slot: 1, effect: None });
        }
        // Under an opaque cover the screen stays as it was; what lies
        // above the cover still changes (mb03_022's captions over its
        // `CreatePlainSP`).
        let frozen = self.frozen_at();
        let above = |o: &Object| frozen.is_some_and(|p| o.priority > p);
        let nothing_above = !self.objects.values().any(|o| above(o) && o.opacity > 0 && !o.hideable) && !self.shown.keys().any(|n| self.objects.get(n).is_some_and(above));
        if frozen.is_some() && nothing_above {
            if ms > 0 {
                self.ops.push(Op::Wait { ms });
            }
            return;
        }
        // Wanted: visible pictures, by (priority, seq), top first.
        let want: Vec<_> = self
            .objects
            .iter()
            .filter_map(|(n, o)| match &o.kind {
                // (Fading out to 0 as it moves: kept for its motion, below.)
                Kind::Image { image } if (o.opacity > 0 || (timed.contains_key(n) && self.shown.contains_key(n))) && !o.hideable => {
                    let opacity = self.drawn_opacity(n, o.opacity);
                    if opacity == 0 && o.opacity > 0 {
                        return None;
                    }
                    let (mut px, mut py) = self.parent_offset(n);
                    // A zoom about the vertex: the picture scaled about its
                    // centre, moved so that the vertex stays in place.
                    if let Some((vx, vy)) = o.vertex {
                        px += (vx - o.w / 2) * (1000 - o.zoom.0) / 1000;
                        py += (vy - o.h / 2) * (1000 - o.zoom.1) / 1000;
                        // A turn about the vertex: the engine turns it about
                        // its centre, so the centre goes where the turn takes it.
                        if o.turn.2.rem_euclid(360) != 0 {
                            let (dx, dy) = ((o.w / 2 - vx) as f64, (o.h / 2 - vy) as f64);
                            let (sin, cos) = (o.turn.2 as f64).to_radians().sin_cos();
                            px += (dx * cos - dy * sin - dx).round() as i64;
                            py += (dx * sin + dy * cos - dy).round() as i64;
                        }
                    }
                    let (mut x, mut y, mut zoom) = (o.x + px, o.y + py, o.zoom);
                    // A camera's zoom: the picture's centre moved away from
                    // the screen's, the picture scaled as much.
                    let k = self.camera_zoom(n, o.priority)?;
                    if k != 1000 {
                        let (cx, cy) = (x + o.w / 2, y + o.h / 2);
                        let (sx, sy) = (self.game.screen.0 / 2, self.game.screen.1 / 2);
                        x = sx + (cx - sx) * k / 1000 - o.w / 2;
                        y = sy + (cy - sy) * k / 1000 - o.h / 2;
                        zoom = (zoom.0 * k / 1000, zoom.1 * k / 1000);
                    }
                    // Turned about x or y: squashed (convert/turn.rs).
                    let sq = self.squash(n, None);
                    if sq != super::turn::FLAT {
                        let c = [(x + o.w / 2) as f64, (y + o.h / 2) as f64];
                        x = (sq[0].0 * c[0] + sq[0].1).round() as i64 - o.w / 2;
                        y = (sq[1].0 * c[1] + sq[1].1).round() as i64 - o.h / 2;
                        zoom = ((zoom.0 as f64 * sq[0].0).round().max(1.0) as i64, (zoom.1 as f64 * sq[1].0).round().max(1.0) as i64);
                    }
                    let shown = Shown { layer: 0, image: image.clone(), x, y, w: o.w, h: o.h, opacity, cells: None, source: String::new(), zoom: (1000, 1000), angle: o.angle(), clip: None, engine: Engine::default(), flash: 0, blend: o.blend, tint: None, over: false };
                    Some((n.clone(), shown, (o.priority, o.seq), zoom, (o.w, o.h), o.mirror()))
                }
                _ => None,
            })
            .collect();
        let movies: Vec<(String, MovieObject, Object)> = self
            .objects
            .iter()
            .filter_map(|(n, o)| match &o.kind {
                Kind::Movie(m) if o.opacity > 0 && !o.hideable => Some((n.clone(), m.clone(), Object { kind: Kind::Other, ..o.clone() })),
                _ => None,
            })
            .collect();
        let mut want = want;
        for (n, m, o) in movies {
            // Playing: the movie itself, played by the engine as a
            // picture; else the frame showing.
            let at = match m.state {
                MovieState::Playing { .. } => None,
                MovieState::Paused { at } => Some(at),
                MovieState::Played => Some(m.info.ms.saturating_sub(1000 / MOVIE_FPS)),
            };
            let picture = match at {
                None => self.assets.movie_sprite(&m.path, m.alpha, m.dark),
                Some(_) => self.assets.movie_frames(&m.path, at, m.alpha, m.dark).map(|(image, _, w, h)| (image, w, h)),
            };
            let Some((image, cw, ch)) = picture else {
                self.note("movie frames missing");
                continue;
            };
            // Drawn zoomed to the movie's size and its Zoom, about the
            // centre of its place.
            let zoom = ((o.zoom.0 * o.w / cw as i64 / 10) as i32, (o.zoom.1 * o.h / ch as i64 / 10) as i32);
            let (x, y) = (o.x + (o.w - cw as i64) / 2, o.y + (o.h - ch as i64) / 2);
            // One cell: a movie file loops with no time a cell (`:c/1,0,0;`).
            let looped = at.is_none() && m.looped;
            let frame_ms = if at.is_none() { 0 } else { 1000 / MOVIE_FPS };
            let shown = Shown { layer: 0, image, x, y, w: cw as i64, h: ch as i64, opacity: o.opacity, cells: Some((1, looped, zoom, frame_ms)), source: String::new(), zoom: (1000, 1000), angle: 0, clip: None, engine: Engine::default(), flash: 0, blend: o.blend, tint: None, over: false };
            // (Turned over as a picture is: the frame, or the movie file.)
            want.push((n, shown, (o.priority, o.seq), (1000, 1000), (o.w, o.h), o.mirror()));
        }
        for (n, o) in &self.objects {
            if let Kind::Strip(s) = &o.kind
                && o.opacity > 0
                && !o.hideable
            {
                // Zoomed and turned by the engine about its centre; a turn
                // about x or y draws it that much shorter (no perspective).
                let tilt = |deg: i64, z: i64| (z as f64 * (deg as f64).to_radians().cos().abs()).round() as i32;
                let zoom = (s.zoom.0 * tilt(o.turn.1, o.zoom.0) / 1000, s.zoom.1 * tilt(o.turn.0, o.zoom.1) / 1000);
                let cells = Some((s.cells, s.looped, zoom, s.frame_ms));
                let (px, py) = self.parent_offset(n);
                let shown = Shown { layer: 0, image: s.image.clone(), x: o.x + px, y: o.y + py, w: s.w, h: s.h, opacity: o.opacity, cells, source: String::new(), zoom: (1000, 1000), angle: o.angle(), clip: None, engine: Engine::default(), flash: 0, blend: o.blend, tint: None, over: false };
                want.push((n.clone(), shown, (o.priority, o.seq), (1000, 1000), (s.w, s.h), (false, false)));
            }
        }
        for (n, o) in &self.objects {
            if let Kind::Flash(f) = &o.kind
                && o.opacity > 0
                && !o.hideable
            {
                let shown = Shown { layer: 0, image: f.image.clone(), x: o.x, y: o.y, w: o.w, h: o.h, opacity: 1000, cells: None, source: String::new(), zoom: (1000, 1000), angle: 0, clip: None, engine: Engine::default(), flash: o.seq, blend: BLEND_NORMAL, tint: None, over: false };
                want.push((n.clone(), shown, (o.priority, o.seq), (1000, 1000), (o.w, o.h), (false, false)));
            }
        }
        // Zoomed pictures: scaled copies, centred where the original was.
        let mut want: Vec<(String, Shown, (i64, u64))> = want
            .into_iter()
            .map(|(n, mut shown, key, zoom, (w, h), (mx, my))| {
                if (mx || my)
                    && let Some(image) = self.assets.mirrored(&shown.image, mx, my)
                {
                    shown.image = image;
                }
                (shown.source, shown.zoom) = (shown.image.clone(), zoom);
                shown.over = self.over_text(&n);
                if zoom != (1000, 1000)
                    && let Some((image, x, y, sw, sh)) = self.assets.scaled(&shown.image, zoom.0, zoom.1, (shown.x, shown.y, w, h))
                {
                    (shown.x, shown.y) = (x, y);
                    (shown.w, shown.h) = (sw as i64, sh as i64);
                    shown.image = image;
                }
                shown.clip = self.window_clip(&n);
                // Windows that don't overlap show nothing.
                if shown.clip.is_some_and(|c| c.2 == 0 || c.3 == 0) {
                    shown.opacity = 0;
                }
                (n, shown, key)
            })
            .collect();
        // The topmost tint shown: the pictures below it drawn through it.
        let tint = self
            .objects
            .values()
            .filter(|o| o.opacity > 0 && !o.hideable)
            .filter_map(|o| match &o.kind {
                Kind::Tint(fs) => Some(((o.priority, o.seq), fs.iter().map(|f| SpriteFilter { strength: Operand::Const((o.opacity.min(1000) * 256 / 1000) as i32), ..*f }).collect::<Vec<_>>())),
                _ => None,
            })
            .max_by_key(|t| t.0);
        if let Some((above, f)) = tint {
            for w in want.iter_mut().filter(|w| w.2 < above) {
                w.1.tint = Some(f.clone());
            }
        }
        if let Some(p) = frozen {
            // shortcut: a picture above the cover deleted while frozen stays
            // up until the cover goes; track the cover's own set if a game needs it.
            want.retain(|w| w.2.0 > p);
            for (n, shown) in &self.shown {
                let key = match self.objects.get(n) {
                    Some(o) if o.priority > p => continue,
                    Some(o) => (o.priority, o.seq),
                    None => (p, 0),
                };
                want.push((n.clone(), shown.clone(), key));
            }
        }
        want.sort_by_key(|w| std::cmp::Reverse(w.2));
        // A picture fading out to 0 stays only when it also moves or zooms
        // as one motion (alpha keys to 0), drawn until a later sync.
        want.retain(|(n, s, _)| {
            s.opacity > 0 || fades.contains_key(n) && self.shown.get(n).is_some_and(|old| (old.x, old.y, old.w, old.h) != (s.x, s.y, s.w, s.h) && motion(old, &Shown { layer: old.layer, ..s.clone() }, 1).is_some())
        });
        // Pictures under an opaque one that fills the screen (an old
        // background left up under a new one) aren't drawn.
        let (sw, sh) = self.game.screen;
        let cover = want.iter().position(|(n, s, _)| {
            let still = !self.shown.get(n).is_some_and(|o| o.engine.moved) && !timed.contains_key(n);
            still
                && s.cells.is_none()
                && s.flash == 0
                && s.angle == 0
                && s.clip.is_none()
                && s.opacity >= 1000
                && s.image.to_ascii_lowercase().ends_with(".jpg")
                && s.x <= 0
                && s.y <= 0
                && s.x + s.w >= sw
                && s.y + s.h >= sh
        });
        if let Some(i) = cover {
            want.truncate(i + 1);
        }

        let mut changes = Vec::new();
        let mut opacity = Vec::new();
        let gone: Vec<String> = self.shown.keys().filter(|n| !want.iter().any(|w| &w.0 == *n)).cloned().collect();
        let mut moving = Vec::new();
        let clock = self.sound_clock();
        self.vanishing.retain(|_, end| *end > clock);
        for n in gone {
            // Still fading out as it moves (the script went on).
            if self.vanishing.contains_key(&n) && self.objects.contains_key(&n) {
                continue;
            }
            self.vanishing.remove(&n);
            let s = self.shown.remove(&n).unwrap();
            // Turning edge-on: squashed away by the engine, then gone at a
            // later sync (its layer kept till then).
            if let Some(t) = turning.get(&n).filter(|_| self.objects.contains_key(&n))
                && let Some(op) = self.turn_motion(&n, t, s.layer, Some(&s), None, false)
            {
                self.ops.push(op);
                self.shown.insert(n.clone(), Shown { engine: Engine { zoomed: true, moved: true }, ..s });
                moving.push(n);
                continue;
            }
            changes.push(SpriteChange { layer: s.layer, sprite: None, keep_filters: false });
        }
        // Layers of the wanted pictures in drawing order, top first: each
        // must be above (lower than) the next one.
        let mut layers: Vec<Option<u32>> = want.iter().map(|w| self.shown.get(&w.0).map(|s| s.layer)).collect();
        let mut crowded = false;
        for i in 0..want.len() {
            if layers[i].is_some() {
                continue;
            }
            let above = layers[..i].iter().flatten().max().map_or(0, |l| l + 1);
            let below = layers[i + 1..].iter().flatten().min().copied().unwrap_or(MAX_LAYER + 1);
            let ideal = ((30000 - want[i].2.0).clamp(0, 30000) * MAX_LAYER as i64 / 30000) as u32;
            if above >= below {
                crowded = true;
                break;
            }
            layers[i] = Some(ideal.clamp(above, below - 1));
        }
        // No gap left between two pictures: spread them all out again, in
        // the same order (pictures that move layer are drawn anew).
        if crowded {
            let n = want.len() as u32;
            layers = (0..n).map(|i| Some((i + 1) * MAX_LAYER / (n + 1))).collect();
        }
        // A picture still fading out whose layer another takes: gone now.
        let mut freed = Vec::new();
        for n in self.vanishing.keys().filter(|n| !want.iter().any(|w| &w.0 == *n)).cloned().collect::<Vec<_>>() {
            if let Some(s) = self.shown.get(&n).filter(|s| layers.contains(&Some(s.layer))) {
                changes.push(SpriteChange { layer: s.layer, sprite: None, keep_filters: false });
                freed.push(s.layer);
                self.shown.remove(&n);
                self.vanishing.remove(&n);
            }
        }
        // Pictures that only move or zoom over time: moved by the engine
        // while the script goes on (and during the cross-fade of the other
        // changes), which waits only for the blocking action's time.
        // Turning pictures that grow (or come in from edge-on): placed as
        // they end, then the motion from the start (below).
        let mut grow: HashMap<String, Option<Shown>> = HashMap::new();
        // Moving pictures drawn on another layer (the layers spread out
        // again): placed where they end, then these ops from where they were.
        let mut moved_over: HashMap<String, Vec<Op>> = HashMap::new();
        let mut vanished = Vec::new();
        for ((name, now, _), layer) in want.iter().zip(&layers) {
            let (Some(layer), Some(t)) = (*layer, turning.get(name)) else { continue };
            let old = self.shown.get(name);
            if old.is_none_or(|o| o.layer != layer || now.w * now.h >= o.w * o.h) {
                grow.insert(name.clone(), old.cloned());
                moving.push(name.clone());
            } else if let Some(old) = old
                && let Some(op) = self.turn_motion(name, t, layer, Some(old), Some(now), false)
            {
                let now = Shown { layer, engine: Engine { zoomed: true, moved: true }, ..now.clone() };
                self.ops.push(op);
                self.shown.insert(name.clone(), now);
                moving.push(name.clone());
            }
        }
        for ((name, now, _), layer) in want.iter().zip(&layers) {
            if turning.contains_key(name) {
                continue;
            }
            let (Some(layer), Some(old), Some(&t)) = (*layer, self.shown.get(name), timed.get(name)) else { continue };
            let now = Shown { layer, ..now.clone() };
            // (Only tinted anew: spfilter below, no motion.)
            if t == 0 || *old == (Shown { engine: old.engine, tint: old.tint.clone(), ..now.clone() }) {
                continue;
            }
            // (A fade too: its keys' alpha is for the old sprite, left as a cross-fade.)
            if old.layer != layer
                && old.opacity == now.opacity
                && !self.lead_ins.contains_key(name)
                && let Some(Op::SpriteMotion2 { keys, end, .. }) = motion(&Shown { layer, ..old.clone() }, &now, t)
            {
                let mut ops = Vec::new();
                if let Some(t) = self.objects.get(name).and_then(|o| o.trail) {
                    ops.push(Op::SpriteTrail { layer, passes: t.passes, alpha: t.alpha, lag: t.lag });
                }
                if let Some(&(ease, exponent)) = tempo.get(name) {
                    ops.push(Op::MoveEase { ease, exponent });
                }
                let last = *keys.last().unwrap();
                let from_end = |k: MotionKey| MotionKey { x: k.x - last.x, y: k.y - last.y, zoom: k.zoom * 100 / last.zoom.max(1), zoom_y: k.zoom_y * 100 / last.zoom_y.max(1), angle: k.angle - last.angle, ..k };
                ops.push(Op::SpriteMotion2 { layer, keys: keys.into_iter().map(from_end).collect(), end });
                moved_over.insert(name.clone(), ops);
                moving.push(name.clone());
                continue;
            }
            // A fade at once (not over time) as the move starts: from the new opacity.
            let start = if fades.contains_key(name) { old.clone() } else { Shown { opacity: now.opacity, ..old.clone() } };
            match motion(&start, &now, t) {
                // The move a loop starts after: its lead-in, from where the
                // picture was to its new place (where it is put now).
                Some(Op::SpriteMotion2 { keys: mk, .. }) if mk[1].zoom == 100 && mk[1].angle == 0 && self.lead_ins.contains_key(name) && let Some(Object { shake: Some((_, keys)), .. }) = self.objects.get(name) => {
                    let lead = self.lead_ins.remove(name).unwrap();
                    self.ops.push(super::loops::lead_in_loop(layer, (-mk[1].x, -mk[1].y, t), lead, keys.clone()));
                    self.ops.push(Op::SpriteMove { layer, x: (now.x + now.w / 2) as i32, y: (now.y + now.h / 2) as i32, ms: 0 });
                    self.shown.insert(name.clone(), Shown { engine: Engine { moved: true, ..old.engine }, ..now });
                    moving.push(name.clone());
                }
                Some(op) => {
                    let engine = Engine { zoomed: old.engine.zoomed || old.zoom != now.zoom || old.angle != now.angle, moved: true };
                    if let Some(t) = self.objects.get(name).and_then(|o| o.trail) {
                        self.ops.push(Op::SpriteTrail { layer, passes: t.passes, alpha: t.alpha, lag: t.lag });
                    }
                    if old.opacity != now.opacity {
                        self.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: (super::fade_base(&start, &now) * 256 / 1000) as u32 });
                    }
                    if let Some(&(ease, exponent)) = tempo.get(name) {
                        self.ops.push(Op::MoveEase { ease, exponent });
                    }
                    self.ops.push(op);
                    if now.opacity == 0 {
                        self.vanishing.insert(name.clone(), clock + u64::from(t));
                    }
                    // (Tinted anew as it moves: spfilter keeps the motion.)
                    if old.tint != now.tint {
                        opacity.push(Op::SpriteFilters { layer, filters: now.tint.clone().unwrap_or_default() });
                    }
                    self.shown.insert(name.clone(), Shown { engine, ..now });
                    moving.push(name.clone());
                }
                None if (old.x, old.y, old.w, old.h) != (now.x, now.y, now.w, now.h) => {
                    // Faded out to 0 (kept above for a motion): removed.
                    if now.opacity == 0 {
                        let layer = old.layer;
                        changes.push(SpriteChange { layer, sprite: None, keep_filters: false });
                        self.shown.remove(name);
                        vanished.push(name.clone());
                    }
                    self.note("move or zoom over time with a fade or a new picture drawn as a cross-fade");
                }
                None => {}
            }
        }
        // (A process's own time holds only the process.)
        let rest = timed.iter().filter(|(n, _)| !moving.contains(n) && !matches!(self.objects.get(*n), Some(Object { kind: Kind::Process { .. }, .. }))).map(|(_, &t)| t).max().unwrap_or(0);
        let ms = if moving.is_empty() { ms } else { blocking.max(rest.min(UNWAITED_MS)) };
        let mut placed = Vec::new();
        let mut animated = Vec::new();
        // Layers whose movie (zoomed with spzoom) goes: removed even when a
        // picture takes the layer, which would keep the zoom.
        // So do flashes' layers (which would keep their blend and keys).
        let mut unzoom: Vec<u32> = changes.iter().filter(|c| self.shown.values().any(|s| s.layer == c.layer && (s.cells.is_some() || s.flash != 0 || s.blend != BLEND_NORMAL))).map(|c| c.layer).collect();
        unzoom.extend(freed);
        let mut new_shown = Vec::new();
        for ((name, now, _), layer) in want.into_iter().zip(&layers) {
            let Some(layer) = *layer else { continue };
            if vanished.contains(&name) {
                continue;
            }
            let mut now = Shown { layer, ..now };
            if let Some(old) = grow.get(&name)
                && let Some(op) = self.turn_motion(&name, &turning[&name], layer, old.as_ref(), Some(&now), true)
            {
                animated.push(op);
                now.engine.moved = true;
            }
            if let Some(ops) = moved_over.remove(&name) {
                animated.extend(ops);
                now.engine.moved = true;
            }
            match self.shown.get(&name) {
                Some(old) if *old == Shown { engine: old.engine, ..now.clone() } => continue,
                Some(old) if old.layer == now.layer && old.image == now.image && old.x == now.x && old.y == now.y && old.cells == now.cells && old.flash == now.flash && old.angle == now.angle && old.clip == now.clip && old.over == now.over => {
                    now.engine = old.engine;
                    if old.tint != now.tint {
                        opacity.push(Op::SpriteFilters { layer, filters: now.tint.clone().unwrap_or_default() });
                    }
                    if old.blend != now.blend {
                        opacity.push(Op::SpriteBlend { layer, mode: now.blend });
                    }
                    opacity.push(Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                }
                old => {
                    // (`spovertext` stays on the layer until it is cleared.)
                    if let Some(old) = old.filter(|o| o.layer != layer || o.engine.zoomed || (o.cells.is_some() && now.cells.is_none()) || o.flash != 0 || (o.over && !now.over)) {
                        changes.push(SpriteChange { layer: old.layer, sprite: None, keep_filters: false });
                        if old.cells.is_some() || old.engine.zoomed || old.flash != 0 || old.blend != BLEND_NORMAL {
                            unzoom.push(old.layer);
                        }
                    }
                    let sprite = SpritePlacement { image: now.image.clone(), x: now.x as i32, y: now.y as i32 };
                    match now.cells {
                        // A strip placed again: its last picture.
                        Some((_, _, zoom, frame_ms)) if let Some(Object { kind: Kind::Strip(s), .. }) = self.objects.get(&name).filter(|o| matches!(&o.kind, Kind::Strip(s) if !s.looped) && old.is_some_and(|o| o.image == now.image)) => {
                            let sprite = SpritePlacement { image: s.last.clone(), ..sprite };
                            animated.push(Op::AnimatedSprite { layer, sprite, cells: 1, frame_ms, looped: false, zoom });
                        }
                        Some((cells, looped, zoom, frame_ms)) => {
                            animated.push(Op::AnimatedSprite { layer, sprite, cells, frame_ms, looped, zoom });
                        }
                        None => placed.push(SpriteChange { layer, sprite: Some(sprite), keep_filters: false }),
                    }
                    // A flash starts the first time it is placed; placed
                    // again (on another layer, or after something hid it)
                    // it stays at its end (how far it got isn't known).
                    if let Some(Object { kind: Kind::Flash(f), .. }) = self.objects.get_mut(&name) {
                        let (keys, times) = match f.started {
                            true => (f.keys.last().map(|&(_, a)| vec![(0, a)]).unwrap_or_default(), 1),
                            false => (f.keys.clone(), f.times),
                        };
                        f.started = true;
                        animated.push(Op::SpriteFlashing { layer, keys, times });
                    }
                    if now.over {
                        animated.push(Op::SpriteOverText { layer });
                    }
                    if let Some(Object { shake: Some((_, keys)), .. }) = self.objects.get(&name) {
                        animated.push(Op::SpriteMotion2 { layer, keys: keys.clone(), end: conv_core::ir::MotionEnd::Loop });
                    }
                    if let Some(keys) = shake_once.remove(&name) {
                        animated.push(Op::SpriteMotion { layer, keys, looped: false });
                    }
                    if now.opacity < 1000 {
                        opacity.push(Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                    }
                    // spblend stays through lsp: cleared with the layer.
                    if now.blend != BLEND_NORMAL {
                        opacity.push(Op::SpriteBlend { layer, mode: now.blend });
                    }
                    if let Some(f) = &now.tint {
                        opacity.push(Op::SpriteFilters { layer, filters: f.clone() });
                    }
                    // spclip stays through lsp too.
                    if let Some((x, y, w, h)) = now.clip {
                        opacity.push(Op::SpriteClip { layer, rect: (x as i32, y as i32, w as i32, h as i32) });
                        now.engine.zoomed = true;
                    }
                    // spzoom's angle stays through lsp: cleared with the layer later.
                    if now.angle != 0 {
                        let zoom = now.cells.map_or((100, 100), |c| c.2);
                        opacity.push(Op::SpriteAngle { layer, degrees: now.angle as i32, zoom });
                        now.engine.zoomed = true;
                    }
                }
            }
            new_shown.push((name, now));
        }
        // Removals first: a placement may reuse a freed layer.
        changes.retain(|c| unzoom.contains(&c.layer) || !placed.iter().any(|p| p.layer == c.layer));
        changes.extend(placed);
        self.shown.extend(new_shown);
        animated.extend(self.start_one_shots());
        // Loops started during a change that isn't a move: after their lead.
        for (name, lead) in std::mem::take(&mut self.lead_ins) {
            if let (Some(s), Some(Object { shake: Some((_, keys)), .. })) = (self.shown.get(&name), self.objects.get(&name)) {
                animated.push(super::loops::lead_in_loop(s.layer, (0, 0, 0), lead, keys.clone()));
            }
        }
        if changes.is_empty() && animated.is_empty() && opacity.is_empty() {
            if ms > 0 {
                self.ops.push(Op::Wait { ms });
            }
            return;
        }
        let effect = if mask.is_some() { "DrawTransition" } else { "Fade" }.to_string();
        self.ops.push(Op::BeginTransition { effect, fade_ms: ms, mask, style: TransitionStyle::Fade });
        if !changes.is_empty() {
            self.ops.push(Op::Sprites { changes, fade_ms: 0 });
        }
        self.ops.extend(animated);
        self.ops.extend(opacity);
        self.ops.push(Op::EndTransition);
    }
}
