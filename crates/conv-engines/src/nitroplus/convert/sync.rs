//! Turning the objects' changes into ops at each wait ([`Compiler::sync`]).

use super::*;

impl Compiler<'_, '_> {
    /// Shows the changes since the last sync, as a cross-fade over `ms`,
    /// or the pending actions' time up to [`UNWAITED_MS`] if longer.
    pub(super) fn sync(&mut self, ms: u32) {
        let blocking = ms;
        let ms = ms.max(std::mem::take(&mut self.pending_ms).min(UNWAITED_MS));
        let timed = std::mem::take(&mut self.pending_by);
        let mut shake_once = std::mem::take(&mut self.shake_once);
        let mask = self.pending_mask.take();
        self.flush_sounds();
        self.check_window();
        if !self.shaken.is_empty() && !self.shaken.iter().any(|t| self.objects.get(t).is_some_and(|o| o.opacity > 0)) {
            self.shaken.clear();
            self.ops.push(Op::ScreenEffect { slot: 1, effect: None });
        }
        if self.frozen() {
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
                Kind::Image { image } if o.opacity > 0 && !o.hideable => {
                    let opacity = self.drawn_opacity(n, o.opacity);
                    if opacity == 0 {
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
                    let shown = Shown { layer: 0, image: image.clone(), x, y, w: o.w, h: o.h, opacity, cells: None, source: String::new(), zoom: (1000, 1000), angle: o.angle(), clip: None, engine: Engine::default(), flash: 0, blend: o.blend };
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
            let shown = Shown { layer: 0, image, x, y, w: cw as i64, h: ch as i64, opacity: o.opacity, cells: Some((1, looped, zoom, frame_ms)), source: String::new(), zoom: (1000, 1000), angle: 0, clip: None, engine: Engine::default(), flash: 0, blend: o.blend };
            want.push((n, shown, (o.priority, o.seq), (1000, 1000), (o.w, o.h), (false, false)));
        }
        for (n, o) in &self.objects {
            if let Kind::Strip(s) = &o.kind
                && o.opacity > 0
                && !o.hideable
            {
                let cells = Some((s.cells, false, s.zoom, s.frame_ms));
                let shown = Shown { layer: 0, image: s.image.clone(), x: o.x, y: o.y, w: s.w, h: s.h, opacity: o.opacity, cells, source: String::new(), zoom: (1000, 1000), angle: 0, clip: None, engine: Engine::default(), flash: 0, blend: o.blend };
                want.push((n.clone(), shown, (o.priority, o.seq), (1000, 1000), (s.w, s.h), (false, false)));
            }
        }
        for (n, o) in &self.objects {
            if let Kind::Flash(f) = &o.kind
                && o.opacity > 0
                && !o.hideable
            {
                let shown = Shown { layer: 0, image: f.image.clone(), x: o.x, y: o.y, w: o.w, h: o.h, opacity: 1000, cells: None, source: String::new(), zoom: (1000, 1000), angle: 0, clip: None, engine: Engine::default(), flash: o.seq, blend: BLEND_NORMAL };
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
        want.sort_by_key(|w| std::cmp::Reverse(w.2));
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
        for n in gone {
            let s = self.shown.remove(&n).unwrap();
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
        // Pictures that only move or zoom over time: moved by the engine
        // while the script goes on (and during the cross-fade of the other
        // changes), which waits only for the blocking action's time.
        let mut moving = Vec::new();
        for ((name, now, _), layer) in want.iter().zip(&layers) {
            let (Some(layer), Some(old), Some(&t)) = (*layer, self.shown.get(name), timed.get(name)) else { continue };
            let now = Shown { layer, ..now.clone() };
            if t == 0 || *old == (Shown { engine: old.engine, ..now.clone() }) {
                continue;
            }
            match motion(old, &now, t) {
                Some(op) => {
                    let engine = Engine { zoomed: old.engine.zoomed || old.zoom != now.zoom || old.angle != now.angle, moved: true };
                    if let Some(t) = self.objects.get(name).and_then(|o| o.trail) {
                        self.ops.push(Op::SpriteTrail { layer, passes: t.passes, alpha: t.alpha, lag: t.lag });
                    }
                    self.ops.push(op);
                    self.shown.insert(name.clone(), Shown { engine, ..now });
                    moving.push(name.clone());
                }
                None if (old.x, old.y, old.w, old.h) != (now.x, now.y, now.w, now.h) => {
                    self.note("move or zoom over time with a fade or a new picture drawn as a cross-fade");
                }
                None => {}
            }
        }
        let rest = timed.iter().filter(|(n, _)| !moving.contains(n)).map(|(_, &t)| t).max().unwrap_or(0);
        let ms = if moving.is_empty() { ms } else { blocking.max(rest.min(UNWAITED_MS)) };
        let mut placed = Vec::new();
        let mut animated = Vec::new();
        // Layers whose movie (zoomed with spzoom) goes: removed even when a
        // picture takes the layer, which would keep the zoom.
        // So do flashes' layers (which would keep their blend and keys).
        let mut unzoom: Vec<u32> = changes.iter().filter(|c| self.shown.values().any(|s| s.layer == c.layer && (s.cells.is_some() || s.flash != 0 || s.blend != BLEND_NORMAL))).map(|c| c.layer).collect();
        let mut new_shown = Vec::new();
        for ((name, now, _), layer) in want.into_iter().zip(&layers) {
            let Some(layer) = *layer else { continue };
            let mut now = Shown { layer, ..now };
            match self.shown.get(&name) {
                Some(old) if *old == Shown { engine: old.engine, ..now.clone() } => continue,
                Some(old) if old.layer == now.layer && old.image == now.image && old.x == now.x && old.y == now.y && old.cells == now.cells && old.flash == now.flash && old.angle == now.angle && old.clip == now.clip => {
                    now.engine = old.engine;
                    if old.blend != now.blend {
                        opacity.push(Op::SpriteBlend { layer, mode: now.blend });
                    }
                    opacity.push(Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                }
                old => {
                    if let Some(old) = old.filter(|o| o.layer != layer || o.engine.zoomed || (o.cells.is_some() && now.cells.is_none()) || o.flash != 0) {
                        changes.push(SpriteChange { layer: old.layer, sprite: None, keep_filters: false });
                        if old.cells.is_some() || old.engine.zoomed || old.flash != 0 || old.blend != BLEND_NORMAL {
                            unzoom.push(old.layer);
                        }
                    }
                    let sprite = SpritePlacement { image: now.image.clone(), x: now.x as i32, y: now.y as i32 };
                    match now.cells {
                        // A strip placed again: its last picture.
                        Some((_, _, zoom, frame_ms)) if let Some(Object { kind: Kind::Strip(s), .. }) = self.objects.get(&name).filter(|_| old.is_some_and(|o| o.image == now.image)) => {
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
                    if let Some(Object { kind: Kind::Strip(_), over_text: true, .. }) = self.objects.get(&name) {
                        animated.push(Op::SpriteOverText { layer });
                    }
                    if let Some(Object { shake: Some((_, keys)), .. }) = self.objects.get(&name) {
                        animated.push(Op::SpriteMotion { layer, keys: keys.clone(), looped: true });
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
                    // spclip stays through lsp too.
                    if let Some((x, y, w, h)) = now.clip {
                        opacity.push(Op::SpriteClip { layer, rect: (x as i32, y as i32, w as i32, h as i32) });
                        now.engine.zoomed = true;
                    }
                    // sprot stays through lsp: cleared with the layer later.
                    if now.angle != 0 {
                        opacity.push(Op::SpriteAngle { layer, degrees: now.angle as i32 });
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
