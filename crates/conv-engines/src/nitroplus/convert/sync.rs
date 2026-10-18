//! Turning the objects' changes into ops at each wait ([`Compiler::sync`]).

use super::*;

impl Compiler<'_, '_> {
    /// Shows the changes since the last sync, as a cross-fade over `ms`,
    /// or the pending actions' time up to [`UNWAITED_MS`] if longer.
    pub(super) fn sync(&mut self, ms: u32) {
        // (One that also zooms or turns isn't kept: that fade isn't one
        // motion, and a cross-fade would hold the script; mb02_028vs:123.)
        let (disused, now): (Vec<_>, Vec<_>) = std::mem::take(&mut self.disused_fading).into_iter().partition(|(n, _)| !self.pending_size_from.contains_key(n) && !self.pending_turn.contains_key(n));
        for (n, _) in now {
            self.delete(&n);
        }
        self.draw(ms);
        // (Kept while the engine fades it out, then deleted.)
        for (n, seq) in disused {
            if self.objects.get(&n).is_some_and(|o| o.seq == seq) {
                match self.vanishing.contains_key(&n) {
                    true => self.disused_fading.push((n, seq)),
                    false => self.delete(&n),
                }
            }
        }
    }

    fn draw(&mut self, ms: u32) {
        let blocking = ms;
        let ms = ms.max(std::mem::take(&mut self.pending_ms).min(UNWAITED_MS));
        let timed = std::mem::take(&mut self.pending_by);
        let tempo = std::mem::take(&mut self.pending_tempo);
        let fade_tempo = self.fade_tempo.take();
        let soft = self.transition_soft.take();
        let fades = std::mem::take(&mut self.pending_fade);
        let mut turning = std::mem::take(&mut self.pending_turn);
        let size_from = std::mem::take(&mut self.pending_size_from);
        for (n, t) in &mut turning {
            t.zoom = size_from.get(n).zip(self.objects.get(n)).map(|(&a, o)| (a, o.zoom)).filter(|(a, b)| a != b);
            // (A child fades with the turned picture it is drawn in.)
            t.fade_ms = fades.get(n).or_else(|| fades.get(&t.target)).copied().unwrap_or(0);
        }
        let from_zero = std::mem::take(&mut self.fade_from_zero);
        let cameras = std::mem::take(&mut self.pending_camera);
        let mut shake_once = std::mem::take(&mut self.shake_once);
        for o in self.objects.values_mut() {
            if let Kind::Cover { again, .. } = &mut o.kind
                && o.opacity == 0
            {
                *again = true;
            }
        }
        let mut paths = std::mem::take(&mut self.pending_path);
        let from = std::mem::take(&mut self.pending_from);
        let angle_from = std::mem::take(&mut self.pending_angle_from);
        let zoom_from = std::mem::take(&mut self.pending_zoom_from);
        let wave_clock = self.sound_clock();
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
        // A toned or shaded cover (`SetTone`, `SetShade` on a `Plain`): the
        // frozen pictures under it drawn through its filters.
        let cover_tone = frozen.and_then(|p| self.objects.values().filter(|o| matches!(o.kind, Kind::Cover { .. }) && o.opacity >= 1000 && o.priority == p).map(|o| o.filters((1000, 1000))).find(|f| !f.is_empty()));
        let under = |n: &String| frozen.is_some_and(|p| self.objects.get(n).is_none_or(|o| o.priority <= p));
        let retone = cover_tone.as_ref().is_some_and(|c| self.shown.iter().any(|(n, s)| under(n) && s.tint.as_ref() != Some(&toned(self.objects.get(n), s, c))));
        if frozen.is_some() && nothing_above && !retone {
            if ms > 0 {
                self.ops.push(Op::Wait { ms });
            }
            return;
        }
        // (One whose `DrawTransition` to 0 has ended goes: the exe leaves it at 0.)
        let faded_out = |o: &Object| o.fade.as_ref().is_some_and(|f| f.to == 0 && f.until <= wave_clock);
        // Wanted: visible pictures, by (priority, seq), top first.
        let want: Vec<_> = self
            .objects
            .iter()
            .filter_map(|(n, o)| match &o.kind {
                // (Fading out to 0 as it moves: kept for its motion, below.)
                Kind::Image { image } if (o.opacity > 0 || (timed.contains_key(n) && self.shown.contains_key(n))) && !o.hideable && !faded_out(o) => {
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
                    let shown = Shown { layer: 0, image: image.clone(), x, y, w: o.w, h: o.h, opacity, cells: None, source: String::new(), zoom: (1000, 1000), angle: o.angle(), clip: None, engine: Engine::default(), flash: 0, blend: o.blend, tint: None, over: false, whole: None, fade: o.fade.clone() };
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
        for (n, shown, ..) in want.iter_mut() {
            if let Some(image) = self.stenciled(n, &shown.image) {
                shown.image = image;
            }
        }
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
            let shown = Shown { layer: 0, image, x, y, w: cw as i64, h: ch as i64, opacity: o.opacity, cells: Some((1, looped, zoom, frame_ms)), source: String::new(), zoom: (1000, 1000), angle: 0, clip: None, engine: Engine::default(), flash: 0, blend: o.blend, tint: None, over: false, whole: None, fade: None };
            // (Turned over as a picture is: the frame, or the movie file.)
            want.push((n, shown, (o.priority, o.seq), (1000, 1000), (o.w, o.h), o.mirror()));
        }
        for (n, o) in &self.objects {
            if let Kind::Strip(s) = &o.kind
                && o.opacity > 0
                && !o.hideable
                && !faded_out(o)
            {
                // Zoomed and turned by the engine about its centre; a turn
                // about x or y draws it that much shorter (no perspective).
                let tilt = |deg: i64, z: i64| (z as f64 * (deg as f64).to_radians().cos().abs()).round() as i32;
                let zoom = (s.zoom.0 * tilt(o.turn.1, o.zoom.0) / 1000, s.zoom.1 * tilt(o.turn.0, o.zoom.1) / 1000);
                let cells = Some((s.cells, s.looped, zoom, s.frame_ms));
                let (px, py) = self.parent_offset(n);
                let (x, y) = (o.x + px, o.y + py);
                let shown = Shown { layer: 0, image: s.image.clone(), x, y, w: s.w, h: s.h, opacity: o.opacity, cells, source: String::new(), zoom: (1000, 1000), angle: o.angle(), clip: None, engine: Engine::default(), flash: 0, blend: o.blend, tint: None, over: false, whole: None, fade: o.fade.clone() };
                want.push((n.clone(), shown, (o.priority, o.seq), (1000, 1000), (s.w, s.h), (false, false)));
            }
        }
        for (n, o) in &self.objects {
            if let Kind::Flash(f) = &o.kind
                && o.opacity > 0
                && !o.hideable
            {
                let shown = Shown { layer: 0, image: f.image.clone(), x: o.x, y: o.y, w: o.w, h: o.h, opacity: 1000, cells: None, source: String::new(), zoom: (1000, 1000), angle: 0, clip: None, engine: Engine::default(), flash: o.seq, blend: BLEND_NORMAL, tint: None, over: false, whole: None, fade: None };
                want.push((n.clone(), shown, (o.priority, o.seq), (1000, 1000), (o.w, o.h), (false, false)));
            }
        }
        // A cover moved, zoomed, turned, blended or waved: a copy of the screen under
        // it, taken as it is first shown, drawn as one picture.
        for (n, o) in &self.objects {
            if self.grabs(n, o) {
                let opacity = self.drawn_opacity(n, o.opacity);
                // (Its size as drawn, about its centre: zoomed by the engine.)
                let (w, h) = (o.w * o.zoom.0 / 1000, o.h * o.zoom.1 / 1000);
                let shown = Shown { layer: 0, image: SCREEN_UNDER.into(), x: o.x + (o.w - w) / 2, y: o.y + (o.h - h) / 2, w, h, opacity, cells: None, source: String::new(), zoom: (1000, 1000), angle: o.angle(), clip: None, engine: Engine::default(), flash: 0, blend: o.blend, tint: None, over: false, whole: None, fade: o.fade.clone() };
                want.push((n.clone(), shown, (o.priority, o.seq), o.zoom, (o.w, o.h), (false, false)));
            }
        }
        // Zoomed to 0 along an axis: nothing drawn (squashed away below).
        want.retain(|w| self.objects.get(&w.0).is_none_or(|o| o.zoom.0 > 0 && o.zoom.1 > 0));
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
                    && shown.image != SCREEN_UNDER
                    && let Some((image, x, y, sw, sh)) = self.assets.scaled(&shown.image, zoom.0, zoom.1, (shown.x, shown.y, w, h))
                {
                    let (ww, wh) = (w * zoom.0 / 1000, h * zoom.1 / 1000);
                    if (sw as i64) < ww - 2 || (sh as i64) < wh - 2 {
                        shown.whole = Some((shown.x + (w - ww) / 2, shown.y + (h - wh) / 2, ww, wh));
                    }
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
            .iter()
            .filter(|(_, o)| o.opacity > 0 && !o.hideable)
            .filter_map(|(n, o)| {
                let fs = match &o.kind {
                    Kind::Tint(fs) => fs.clone(),
                    // (A copy of the screen: its own filters, below.)
                    Kind::Cover { .. } if !self.grabs(n, o) => o.filters((1000, 1000)),
                    _ => return None,
                };
                (!fs.is_empty()).then(|| ((o.priority, o.seq), fs.iter().map(|f| SpriteFilter { strength: Operand::Const((o.opacity.min(1000) * 256 / 1000) as i32), ..*f }).collect::<Vec<_>>()))
            })
            .max_by_key(|t| t.0);
        if let Some((above, f)) = tint {
            for w in want.iter_mut().filter(|w| w.2 < above) {
                w.1.tint = Some(f.clone());
            }
        }
        for w in want.iter_mut() {
            let own = self.objects.get(&w.0).map(|o| o.filters(scale(&w.1))).unwrap_or_default();
            if !own.is_empty() {
                let mut fs = [own, w.1.tint.take().unwrap_or_default()].concat();
                fs.truncate(MAX_FILTERS);
                w.1.tint = Some(fs);
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
                let mut shown = shown.clone();
                if let Some(c) = &cover_tone {
                    shown.tint = Some(toned(self.objects.get(n), &shown, c));
                }
                want.push((n.clone(), shown, key));
            }
        }
        // A cover shown again: the pictures it froze, in its place.
        for (c, o) in self.objects.iter().filter(|(_, o)| o.opacity > 0 && !o.hideable && frozen.is_none_or(|p| o.priority > p)) {
            if let Kind::Cover { froze, .. } = &o.kind
                && self.shows_froze(o)
            {
                let fs = o.filters((1000, 1000));
                for (n, s) in froze {
                    let mut shown = Shown { opacity: s.opacity * o.opacity.min(1000) / 1000, engine: Engine::default(), ..s.clone() };
                    if !fs.is_empty() {
                        let mut t = [shown.tint.take().unwrap_or_default(), fs.clone()].concat();
                        t.truncate(MAX_FILTERS);
                        shown.tint = Some(t);
                    }
                    want.push((format!("{c}\u{1}{n}"), shown, (o.priority, o.seq)));
                }
            }
        }
        want.sort_by_key(|w| std::cmp::Reverse(w.2));
        // A picture fading out to 0 stays only when it also moves or zooms
        // as one motion (alpha keys to 0), drawn until a later sync; or
        // when the fade outlasts the wait (ma00_000:1624's unwaited
        // `Fade(…, 300, 0, null, false)`, the text at once under wine).
        // (Or as it turns: the turn's motion fades it too, convert/turn.rs.)
        want.retain(|(n, s, _)| {
            let unwaited = timed.get(n).is_some_and(|&t| t > blocking);
            s.opacity > 0
                || fades.contains_key(n) && self.shown.get(n).is_some_and(|old| turning.contains_key(n) || ((old.x, old.y, old.w, old.h) != (s.x, s.y, s.w, s.h) || unwaited) && motion(old, &Shown { layer: old.layer, ..s.clone() }, 1).is_some())
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
                && s.fade.is_none()
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
        self.motion_until.retain(|_, end| *end > clock);
        self.runs.retain(|_, r| r.at + u64::from(r.ms) > clock);
        for (camera, t) in cameras {
            self.motion_until.insert(camera, clock + u64::from(t));
        }
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
            // Zooming to 0 over time: squashed away by the engine (moved and
            // turned as it goes), then gone at a later sync (its layer kept
            // till then).
            if let Some(&t) = timed.get(&n).filter(|&&t| t > 0)
                && let Some(o) = self.objects.get(&n).filter(|o| o.zoom.0 == 0 || o.zoom.1 == 0)
            {
                // (Against the zoom it had: a camera's depth scales both alike.)
                let (wx, wy) = zoom_from.get(&n).copied().unwrap_or_else(|| scale(&s));
                let z = |now: i64, was: i64| (now * 100 / was.max(1)).clamp(0, 1_000_000) as u32;
                // Its centre at the end: a zoom about the vertex keeps that in place.
                let (px, py) = self.parent_offset(&n);
                let (vx, vy) = o.vertex.unwrap_or((o.w / 2, o.h / 2));
                let end = |p: i64, v: i64, w: i64, zoom: i64| p + v + (w / 2 - v) * zoom / 1000;
                let (cx, cy) = (end(o.x + px, vx, o.w, o.zoom.0), end(o.y + py, vy, o.h, o.zoom.1));
                let (x, y, w, h) = s.whole.unwrap_or((s.x, s.y, s.w, s.h));
                let opacity = if o.opacity > 0 { 256 } else { 0 };
                if let Some(&(ease, exponent)) = tempo.get(&n) {
                    self.ops.push(Op::MoveEase { ease, exponent });
                }
                let keys = vec![
                    MotionKey { ms: 0, x: 0, y: 0, zoom: 100, zoom_y: 100, angle: 0, opacity: 256, ease: None },
                    MotionKey { ms: t, x: (cx - x - w / 2) as i32, y: (cy - y - h / 2) as i32, zoom: z(o.zoom.0, wx), zoom_y: z(o.zoom.1, wy), angle: (o.angle() - s.angle) as i32, opacity, ease: None },
                ];
                self.ops.push(Op::SpriteMotion2 { layer: s.layer, keys, end: MotionEnd::Keep });
                self.vanishing.insert(n.clone(), clock + u64::from(t));
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
        // Pictures whose motion still runs (or loops) keep their layer when
        // there is room around them, so the engine's motion goes on; so do
        // those that start to move as they fade (a motion with alpha keys
        // is for the sprite already there, not one put on another layer).
        let fading_move = |n: &String| fades.contains_key(n) && timed.get(n).is_some_and(|&t| t > 0);
        if crowded {
            let kept: Vec<Option<u32>> = want
                .iter()
                .map(|w| self.shown.get(&w.0).filter(|_| self.motion_until.contains_key(&w.0) || fading_move(&w.0) || self.objects.get(&w.0).is_some_and(|o| o.shake.is_some())).map(|s| s.layer))
                .collect();
            layers = spread(&kept).unwrap_or_else(|| spread(&vec![None; want.len()]).unwrap());
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
        // Placed instead of the wanted (cut) picture, for its motion (below).
        let mut uncut: HashMap<String, Shown> = HashMap::new();
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
                self.ops.push(bent(op, paths.remove(name)));
                self.shown.insert(name.clone(), now);
                moving.push(name.clone());
            }
        }
        // A new picture as it moves: placed where it ends, then moved there
        // from where it starts (under wine ma00_000's pans run as the
        // script goes on).
        for ((name, now, _), layer) in want.iter().zip(&layers) {
            let (Some(layer), Some(&t)) = (*layer, timed.get(name)) else { continue };
            let ((dx, dy), grown) = (from.get(name).copied().unwrap_or_default(), zoom_from.get(name).filter(|z| z.0 == 0 || z.1 == 0));
            // (Not moved at all too, so that a `Random` pick of 0 leaves
            // the sprite as the other picks: moved by the engine.)
            // (A copy of the screen also as it zooms: a picture's zoom is in its image.)
            let grab_zoom = size_from.get(name).filter(|z| now.image == SCREEN_UNDER && z.0 > 0 && z.1 > 0 && **z != now.zoom).copied();
            if t == 0 || !from.contains_key(name) && grown.is_none() && grab_zoom.is_none() || self.shown.contains_key(name) || turning.contains_key(name) || paths.contains_key(name) {
                continue;
            }
            let now = Shown { layer, ..now.clone() };
            // Turning and fading in from 0 with the move too (the title's
            // buttons flying in): from its angle, and from 0 when the fade
            // takes as long and outlasts the wait.
            let angle = angle_from.get(name).copied().unwrap_or(now.angle);
            let faded = from_zero.contains(name) && fades.get(name) == Some(&t) && t > blocking;
            let mut start = Shown { x: now.x + dx, y: now.y + dy, angle, opacity: if faded { 0 } else { now.opacity }, ..now.clone() };
            if let Some(z) = grab_zoom {
                let (w, h) = (now.w * z.0 / now.zoom.0.max(1), now.h * z.1 / now.zoom.1.max(1));
                start = Shown { x: start.x + (now.w - w) / 2, y: start.y + (now.h - h) / 2, w, h, zoom: z, ..start };
            }
            let mut ops = Vec::new();
            let (keys, end) = if let Some(&(zx, zy)) = grown {
                // Growing from 0: from the zoom it had (about its vertex).
                let Some(o) = self.objects.get(name) else { continue };
                let (vx, vy) = o.vertex.unwrap_or((o.w / 2, o.h / 2));
                // (A picture cut to the screen: the whole one, zoomed by the engine.)
                match now.whole {
                    Some((x, y, w, h)) if now.angle == 0 && now.zoom.0 > 0 && now.zoom.1 > 0 => {
                        let (sw, sh) = (w * 1000 / now.zoom.0, h * 1000 / now.zoom.1);
                        let engine = Engine { zoomed: true, moved: true };
                        uncut.insert(name.clone(), Shown { image: now.source.clone(), x: x + w / 2 - sw / 2, y: y + h / 2 - sh / 2, w: sw, h: sh, whole: None, engine, ..now.clone() });
                        ops.push(Op::SpriteAngle { layer, degrees: 0, zoom: ((now.zoom.0 / 10) as i32, (now.zoom.1 / 10) as i32) });
                    }
                    Some(_) => continue,
                    None => {}
                }
                // (Against the zoom it ends at: a camera's depth scales both alike.)
                let ((sx, sy), z) = (o.zoom, |z: i64, s: i64| (z * 100 / s.max(1)).clamp(0, 1_000_000) as u32);
                let shift = |w: i64, v: i64, from: i64, to: i64| (w / 2 - v) * (from - to) / 1000;
                let key = |ms, x, y, zoom, zoom_y| MotionKey { ms, x, y, zoom, zoom_y, angle: 0, opacity: 256, ease: None };
                let start = key(0, (dx + shift(o.w, vx, zx, o.zoom.0)) as i32, (dy + shift(o.h, vy, zy, o.zoom.1)) as i32, z(zx, sx), z(zy, sy));
                (vec![start, key(t, 0, 0, 100, 100)], MotionEnd::Keep)
            } else {
                let Some(Op::SpriteMotion2 { keys, end, .. }) = motion(&start, &now, t) else { continue };
                (from_end(keys), end)
            };
            if let Some(t) = self.objects.get(name).and_then(|o| o.trail) {
                ops.push(Op::SpriteTrail { layer, passes: t.passes, alpha: t.alpha, lag: t.lag, over: t.over });
            }
            if let Some(&(ease, exponent)) = tempo.get(name) {
                ops.push(Op::MoveEase { ease, exponent });
            }
            ops.push(Op::SpriteMotion2 { layer, keys, end });
            moved_over.insert(name.clone(), ops);
            self.runs.insert(name.clone(), Run { from: start, to: now, ms: t, at: clock, ease: tempo.get(name).copied() });
            moving.push(name.clone());
        }
        // A new picture fading in longer than the script waits: faded in by
        // the engine while the script goes on, as a shown one's fade is
        // (below), not held in the cross-fade (dja001_01:128's 3 s fade to
        // black, the text 100 ms later under wine).
        for ((name, now, _), layer) in want.iter().zip(&layers) {
            let (Some(layer), Some(&t)) = (*layer, fades.get(name)) else { continue };
            if t <= blocking || !from_zero.contains(name) || self.shown.contains_key(name) || from.contains_key(name) || moved_over.contains_key(name) || turning.contains_key(name) || paths.contains_key(name) || now.cells.is_some() {
                continue;
            }
            let now = Shown { layer, ..now.clone() };
            let Some(Op::SpriteMotion2 { keys, end, .. }) = motion(&Shown { opacity: 0, ..now.clone() }, &now, t) else { continue };
            let mut ops = Vec::new();
            if let Some((ease, exponent)) = fade_tempo {
                ops.push(Op::MoveEase { ease, exponent });
            }
            ops.push(Op::SpriteMotion2 { layer, keys, end });
            moved_over.insert(name.clone(), ops);
            self.runs.insert(name.clone(), Run { from: Shown { opacity: 0, ..now.clone() }, to: now, ms: t, at: clock, ease: fade_tempo });
            moving.push(name.clone());
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
            // (A cut picture zooming out: the uncut end placed instead.)
            let rect = |s: &Shown| s.whole.unwrap_or((s.x, s.y, s.w, s.h));
            let (o, n) = (rect(old), rect(&now));
            // A new picture (or turned over) as it moves: the new one,
            // placed at once where the old one was (as the exe replaces it).
            let start = if old.source != now.source && old.cells.is_none() && now.zoom.0 > 0 && now.zoom.1 > 0 {
                let (w, h) = (n.2 * old.zoom.0 / now.zoom.0, n.3 * old.zoom.1 / now.zoom.1);
                Shown { layer, x: o.0 + o.2 / 2 - w / 2, y: o.1 + o.3 / 2 - h / 2, w, h, whole: None, zoom: old.zoom, opacity: old.opacity, ..now.clone() }
            } else {
                Shown { layer, x: o.0, y: o.1, w: o.2, h: o.3, whole: None, ..old.clone() }
            };
            let uneven = super::uneven_motion(old, &now, t);
            if (old.layer != layer || old.whole.is_some_and(|_| now.whole.is_none()) || old.source != now.source || uneven.is_some())
                && old.opacity == now.opacity
                && !self.lead_ins.contains_key(name)
                && let Some((keys, end)) = uneven.map(|k| (k, conv_core::ir::MotionEnd::Keep)).or_else(|| match motion(&start, &now, t).map(|op| along(op, paths.remove(name))) {
                    Some(Op::SpriteMotion2 { keys, end, .. }) => Some((from_end(keys), end)),
                    _ => None,
                })
            {
                let mut ops = Vec::new();
                if let Some(t) = self.objects.get(name).and_then(|o| o.trail) {
                    ops.push(Op::SpriteTrail { layer, passes: t.passes, alpha: t.alpha, lag: t.lag, over: t.over });
                }
                if let Some(&(ease, exponent)) = tempo.get(name) {
                    ops.push(Op::MoveEase { ease, exponent });
                }
                ops.push(Op::SpriteMotion2 { layer, keys, end });
                moved_over.insert(name.clone(), ops);
                self.runs.insert(name.clone(), Run { from: start, to: now, ms: t, at: clock, ease: tempo.get(name).copied() });
                moving.push(name.clone());
                continue;
            }
            // A picture cut to the screen moving to another cut of it (a pan
            // over a picture zoomed past the screen): the whole picture,
            // zoomed by the engine (as many pixels as the cut), moves.
            if let (Some(ow), Some(nw)) = (old.whole, now.whole)
                && old.source == now.source
                && old.opacity == now.opacity
                && now.angle == 0
                && now.zoom.0 > 0
                && now.zoom.1 > 0
                && !self.lead_ins.contains_key(name)
            {
                let (w, h) = (nw.2 * 1000 / now.zoom.0, nw.3 * 1000 / now.zoom.1);
                let centre = |r: (i64, i64, i64, i64)| (r.0 + r.2 / 2, r.1 + r.3 / 2);
                let ((x0, y0), (x1, y1)) = (centre(ow), centre(nw));
                let engine = Engine { zoomed: true, moved: true };
                uncut.insert(name.clone(), Shown { image: now.source.clone(), x: x1 - w / 2, y: y1 - h / 2, w, h, whole: None, engine, ..now.clone() });
                let z = |o: i64, n: i64| (o * 100 / n.max(1)) as u32;
                let key = |ms, x, y, zoom, zoom_y| MotionKey { ms, x, y, zoom, zoom_y, angle: 0, opacity: 256, ease: None };
                let keys = vec![key(0, (x0 - x1) as i32, (y0 - y1) as i32, z(ow.2, nw.2), z(ow.3, nw.3)), key(t, 0, 0, 100, 100)];
                let mut ops = vec![Op::SpriteAngle { layer, degrees: 0, zoom: ((now.zoom.0 / 10) as i32, (now.zoom.1 / 10) as i32) }];
                if let Some(&(ease, exponent)) = tempo.get(name) {
                    ops.push(Op::MoveEase { ease, exponent });
                }
                ops.push(Op::SpriteMotion2 { layer, keys, end: conv_core::ir::MotionEnd::Keep });
                moved_over.insert(name.clone(), ops);
                moving.push(name.clone());
                continue;
            }
            // A fade at once (not over time) as the move starts: from the new opacity.
            let start = if fades.contains_key(name) { old.clone() } else { Shown { opacity: now.opacity, ..old.clone() } };
            match motion(&start, &now, t) {
                // The move a loop starts after: its lead-in, from where the
                // picture was to its new place (where it is put now).
                Some(Op::SpriteMotion2 { keys: mk, .. }) if mk[1].zoom == 100 && mk[1].angle == 0 && self.lead_ins.contains_key(name) && let Some(Object { shake: Some((_, keys, from)), .. }) = self.objects.get(name) => {
                    let lead = self.lead_ins.remove(name).unwrap();
                    self.ops.push(super::loops::lead_in_loop(layer, (-mk[1].x, -mk[1].y, t), lead, keys.clone(), *from));
                    self.ops.push(Op::SpriteMove { layer, x: (now.x + now.w / 2) as i32, y: (now.y + now.h / 2) as i32, ms: 0 });
                    self.shown.insert(name.clone(), Shown { engine: Engine { moved: true, ..old.engine }, ..now });
                    moving.push(name.clone());
                }
                Some(op) => {
                    self.runs.insert(name.clone(), Run { from: start.clone(), to: now.clone(), ms: t, at: clock, ease: tempo.get(name).copied() });
                    let engine = Engine { zoomed: old.engine.zoomed || old.zoom != now.zoom || old.cells != now.cells || old.angle != now.angle, moved: true };
                    if let Some(t) = self.objects.get(name).and_then(|o| o.trail) {
                        self.ops.push(Op::SpriteTrail { layer, passes: t.passes, alpha: t.alpha, lag: t.lag, over: t.over });
                    }
                    if old.opacity != now.opacity {
                        self.ops.push(Op::SpriteOpacity { layer: Some(layer), opacity: (super::fade_base(&start, &now) * 256 / 1000) as u32 });
                    }
                    if let Some(&(ease, exponent)) = tempo.get(name) {
                        self.ops.push(Op::MoveEase { ease, exponent });
                    }
                    self.ops.push(along(op, paths.remove(name)));
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
        // A new picture in a camera that moves (dja022's expressions, put
        // up as the camera moves): moved as the camera's other children at
        // its depth, from where the camera draws it now. And a moving
        // picture the layers spread onto another layer (no room was left
        // around it): its motion goes on there.
        for ((name, now, (priority, _)), layer) in want.iter().zip(&layers) {
            let Some(layer) = *layer else { continue };
            let run = match self.shown.get(name) {
                Some(old) if old.layer != layer && !moving.contains(name) && !moved_over.contains_key(name) => self.runs.get(name).filter(|r| {
                    (r.to.x, r.to.y, r.to.w, r.to.h) == (now.x, now.y, now.w, now.h) && r.to.source == now.source && r.from.whole.is_none() && r.to.whole.is_none()
                }),
                Some(_) => None,
                None => {
                    let in_moving_camera = self.motion_until.keys().any(|c| name.starts_with(&format!("{c}/")) && self.objects.get(c).is_some_and(|o| matches!(o.kind, Kind::Camera)));
                    let parent = |n: &str| n.rsplit_once('/').map(|p| p.0.to_string());
                    let sibling = |(s, r): &(&String, &Run)| {
                        r.from.whole.is_none() && r.to.whole.is_none() && self.shown.contains_key(*s) && parent(s) == parent(name) && self.objects.get(*s).is_some_and(|o| o.priority == *priority)
                    };
                    self.runs.iter().find(sibling).map(|r| r.1).filter(|_| in_moving_camera && now.whole.is_none())
                }
            };
            let Some(Run { from: start, to: end, ms: t, at, ease }) = run else { continue };
            // shortcut: begun later than the move, it starts from the straight
            // line's point and eases anew over the rest; exact when begun with it.
            let (e, t) = ((clock - at) as i64, i64::from(*t));
            let f = |a: i64, b: i64| a + (b - a) * e / t;
            let (sx, sy, sw, sh) = (f(start.x, end.x), f(start.y, end.y), f(start.w, end.w), f(start.h, end.h));
            let (w, h) = (now.w * sw / end.w.max(1), now.h * sh / end.h.max(1));
            let (cx, cy) = (sx + sw / 2 + (now.x + now.w / 2 - end.x - end.w / 2) * sw / end.w.max(1), sy + sh / 2 + (now.y + now.h / 2 - end.y - end.h / 2) * sh / end.h.max(1));
            let zoom = (now.zoom.0 * f(start.zoom.0, end.zoom.0) / end.zoom.0.max(1), now.zoom.1 * f(start.zoom.1, end.zoom.1) / end.zoom.1.max(1));
            let from = Shown { layer, x: cx - w / 2, y: cy - h / 2, w, h, zoom, ..now.clone() };
            let left = (t - e) as u32;
            if let Some(Op::SpriteMotion2 { keys, end, .. }) = motion(&from, &Shown { layer, ..now.clone() }, left) {
                let mut ops = Vec::new();
                if let Some((ease, exponent)) = *ease {
                    ops.push(Op::MoveEase { ease, exponent });
                }
                ops.push(Op::SpriteMotion2 { layer, keys: from_end(keys), end });
                moved_over.insert(name.clone(), ops);
                self.motion_until.insert(name.clone(), clock + u64::from(left));
                let run = Run { from, to: Shown { layer, ..now.clone() }, ms: left, at: clock, ease: *ease };
                self.runs.insert(name.clone(), run);
            }
        }
        for name in &moving {
            if let Some(t) = timed.get(name).copied().or(turning.get(name).map(|t| t.ms)) {
                self.motion_until.insert(name.clone(), clock + u64::from(t));
                if t > blocking {
                    self.unwaited_motions.insert(name.clone());
                }
            }
        }
        // (A process's own time holds only the process.)
        // (A `DrawTransition` starts now and runs on in the engine: the
        // print shows its start, then the script waits as it asked.)
        let fading: Vec<&String> = want.iter().filter(|w| w.1.fade.is_some() && self.shown.get(&w.0).is_none_or(|o| o.fade != w.1.fade)).map(|w| &w.0).collect();
        let rest = timed.iter().filter(|(n, _)| !moving.contains(n) && !fading.contains(n) && !matches!(self.objects.get(*n), Some(Object { kind: Kind::Process { .. }, .. }))).map(|(_, &t)| t).max().unwrap_or(0);
        let (ms, after) = match (moving.is_empty(), fading.is_empty()) {
            (true, true) => (ms, 0),
            (false, true) => (blocking.max(rest.min(UNWAITED_MS)), 0),
            (m, false) => {
                let p = if m { rest.min(UNWAITED_MS) } else { blocking.max(rest.min(UNWAITED_MS)) };
                (p, blocking.saturating_sub(p))
            }
        };
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
            // (A movie placed again goes before its motion: lsp stops one.)
            let placing = animated.len();
            if let Some(old) = grow.get(&name)
                && let Some(op) = self.turn_motion(&name, &turning[&name], layer, old.as_ref(), Some(&now), true)
            {
                animated.push(bent(op, paths.remove(&name)));
                now.engine.moved = true;
            }
            if let Some(ops) = moved_over.remove(&name) {
                animated.extend(ops);
                now.engine.moved = true;
            }
            match self.shown.get(&name) {
                Some(old) if *old == Shown { engine: old.engine, ..now.clone() } => continue,
                Some(old) if old.layer == now.layer && old.image == now.image && old.x == now.x && old.y == now.y && old.cells == now.cells && old.flash == now.flash && old.angle == now.angle && old.clip == now.clip && old.fade == now.fade && old.over == now.over => {
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
                    // (Kept as the cut picture, so the next sync leaves it moving.)
                    let sprite = match uncut.remove(&name) {
                        Some(u) => {
                            now.engine = u.engine;
                            SpritePlacement { image: u.image, x: u.x as i32, y: u.y as i32 }
                        }
                        // A copy of the screen: taken unzoomed, zoomed by the engine about its centre.
                        // shortcut: one placed again (another layer) is taken again, from the screen as it is then
                        None if now.image == SCREEN_UNDER => {
                            let (x, y) = self.objects.get(&name).map_or((0, 0), |o| (o.x, o.y));
                            // (Before its motion, which spzoom would cut short.)
                            if now.zoom != (1000, 1000) || now.angle != 0 {
                                animated.insert(placing, Op::SpriteAngle { layer, degrees: now.angle as i32, zoom: ((now.zoom.0 / 10) as i32, (now.zoom.1 / 10) as i32) });
                                now.engine.zoomed = true;
                            }
                            SpritePlacement { image: now.image.clone(), x: x as i32, y: y as i32 }
                        }
                        None => SpritePlacement { image: now.image.clone(), x: now.x as i32, y: now.y as i32 },
                    };
                    match now.cells {
                        // A strip placed again: its last picture.
                        Some((_, _, zoom, frame_ms)) if let Some(Object { kind: Kind::Strip(s), .. }) = self.objects.get(&name).filter(|o| matches!(&o.kind, Kind::Strip(s) if !s.looped) && old.is_some_and(|o| o.image == now.image)) => {
                            let sprite = SpritePlacement { image: s.last.clone(), ..sprite };
                            animated.insert(placing, Op::AnimatedSprite { layer, sprite, cells: 1, frame_ms, looped: false, zoom });
                        }
                        Some((cells, looped, zoom, frame_ms)) => {
                            animated.insert(placing, Op::AnimatedSprite { layer, sprite, cells, frame_ms, looped, zoom });
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
                    if let Some(Object { shake: Some((_, keys, from)), .. }) = self.objects.get(&name) {
                        let end = if *from > 0 { conv_core::ir::MotionEnd::LoopFrom(*from) } else { conv_core::ir::MotionEnd::Loop };
                        animated.push(Op::SpriteMotion2 { layer, keys: keys.clone(), end });
                    }
                    if let Some(keys) = shake_once.remove(&name) {
                        animated.push(Op::SpriteMotion { layer, keys, looped: false });
                    }
                    // (Before its motion: msp ends a kept one, and the keys' alpha is times it.)
                    if now.opacity < 1000 {
                        animated.insert(placing, Op::SpriteOpacity { layer: Some(layer), opacity: (now.opacity * 256 / 1000) as u32 });
                    }
                    // spblend stays through lsp: cleared with the layer.
                    if now.blend != BLEND_NORMAL {
                        opacity.push(Op::SpriteBlend { layer, mode: now.blend });
                    }
                    if let Some(f) = &now.tint {
                        opacity.push(Op::SpriteFilters { layer, filters: f.clone() });
                    }
                    // spwave stays through lsp: cleared with the layer.
                    if let Some(op) = self.objects.get(&name).and_then(|o| o.wave).and_then(|w| w.op(layer, wave_clock, &now)) {
                        opacity.push(op);
                    }
                    // spclip stays through lsp too.
                    if let Some((x, y, w, h)) = now.clip {
                        opacity.push(Op::SpriteClip { layer, rect: (x as i32, y as i32, w as i32, h as i32) });
                        now.engine.zoomed = true;
                    }
                    // spfadeto stays through lsp too: the mask read at the screen pixel
                    // (src: Muramasa.exe 00418320), however the picture moves.
                    if let Some(f) = &now.fade {
                        // shortcut: one placed after its time (under a cover) jumps to its end in one frame
                        let ms = if f.until > clock { f.ms.min((f.until - clock) as u32) } else { 0 };
                        let form = self.game.diesel_transition as u8;
                        opacity.push(Op::SpriteFadeTo { layer, mask: f.mask.clone(), ms, from: f.from as u32, to: f.to as u32, softness: f.softness as u32, form, ease: f.ease });
                        now.engine.zoomed = true;
                    }
                    // spzoom's angle stays through lsp: cleared with the layer later.
                    if now.angle != 0 && now.image != SCREEN_UNDER {
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
            if let (Some(s), Some(Object { shake: Some((_, keys, from)), .. })) = (self.shown.get(&name), self.objects.get(&name)) {
                animated.push(super::loops::lead_in_loop(s.layer, (0, 0, 0), lead, keys.clone(), *from));
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
        if let Some((ease, exponent)) = fade_tempo {
            self.ops.push(Op::TransitionEase { ease, exponent });
        }
        if let Some((softness, reverse)) = soft {
            let form = self.game.diesel_transition as u8;
            self.ops.push(Op::TransitionSoft { softness, reverse, form });
        }
        if !changes.is_empty() {
            self.ops.push(Op::Sprites { changes, fade_ms: 0 });
        }
        self.ops.extend(animated);
        self.ops.extend(opacity);
        self.ops.push(Op::EndTransition);
        if after > 0 {
            self.ops.push(Op::Wait { ms: after });
        }
    }
}

/// A picture's own filters ([`Object::filters`]), then a cover's above it.
impl Compiler<'_, '_> {
    /// A cover shown again over a screen that changed since it froze it
    /// draws what it froze; over the same screen it only keeps it.
    pub(super) fn shows_froze(&self, o: &Object) -> bool {
        let Kind::Cover { froze, again: true } = &o.kind else { return false };
        // shortcut: a cover made over an empty screen keeps the screen (as before), not black; draw black if a scene needs it
        if froze.is_empty() {
            return false;
        }
        // shortcut: a cover moved, zoomed, turned, shaken or blended (mc03_021vs's
        // SubRender copy), or one that froze a strip or movie (md04_037's rain), keeps
        // the screen as before; draw its copy so if a scene needs it
        if (o.x, o.y, o.zoom, o.turn) != (0, 0, (1000, 1000), (0, 0, 0)) || o.shake.is_some() || o.blend != BLEND_NORMAL || froze.iter().any(|(_, s)| s.cells.is_some()) {
            return false;
        }
        self.changed_under(o, froze)
    }

    /// The pictures under a cover differ from those it froze.
    fn changed_under(&self, o: &Object, froze: &[(String, Shown)]) -> bool {
        let under = self.shown.keys().filter(|n| self.objects.get(*n).is_none_or(|x| x.priority <= o.priority)).count();
        under != froze.len() || froze.iter().any(|(n, s)| self.shown.get(n).is_none_or(|t| *t != Shown { engine: t.engine, layer: t.layer, tint: t.tint.clone(), ..s.clone() }))
    }

    /// A cover moved, zoomed, turned, blended or waved is drawn as a copy of the
    /// screen under it ([`SCREEN_UNDER`]), taken when it is first shown
    /// (`name` shown since), while that is still what it froze.
    // shortcut: one shown first over a screen that changed since it froze, or
    // shaken, or over a strip or movie, keeps the screen as before; draw its
    // froze pictures into the copy if a scene needs it
    pub(super) fn grabs(&self, name: &str, o: &Object) -> bool {
        let Kind::Cover { froze, .. } = &o.kind else { return false };
        o.opacity > 0
            && !o.hideable
            && ((o.x, o.y, o.zoom, o.turn) != (0, 0, (1000, 1000), (0, 0, 0)) || o.blend != BLEND_NORMAL || o.wave.is_some())
            && o.shake.is_none()
            && !froze.iter().any(|(_, s)| s.cells.is_some())
            && (self.shown.contains_key(name) || !self.changed_under(o, froze))
    }
}

fn toned(o: Option<&Object>, s: &Shown, cover: &[SpriteFilter]) -> Vec<SpriteFilter> {
    let mut fs = [o.map(|o| o.filters(scale(s))).unwrap_or_default(), cover.to_vec()].concat();
    fs.truncate(MAX_FILTERS);
    fs
}

/// spfilter keeps up to 4 filters a sprite.
// shortcut: the filters past it (a toned, shaded picture under a tint) are
// left out; merge pointwise ones into one if a scene needs them.
const MAX_FILTERS: usize = 4;

/// The scale (per mille) a picture is drawn at against its loaded image:
/// its zoom when the converter scaled the image ([`Assets::scaled`]).
fn scale(s: &Shown) -> (i64, i64) {
    if s.image != s.source { s.zoom } else { (1000, 1000) }
}

/// A motion's keys from its last one: for a sprite placed where it ends.
fn from_end(keys: Vec<MotionKey>) -> Vec<MotionKey> {
    let last = *keys.last().unwrap();
    keys.into_iter().map(|k| MotionKey { x: k.x - last.x, y: k.y - last.y, zoom: k.zoom * 100 / last.zoom.max(1), zoom_y: k.zoom_y * 100 / last.zoom_y.max(1), angle: k.angle - last.angle, ..k }).collect()
}

/// Layers for pictures in drawing order (top first): the given ones kept,
/// the others spread evenly between them (all of them evenly when none is
/// given); `None` when the kept ones leave no room or aren't in order.
fn spread(kept: &[Option<u32>]) -> Option<Vec<Option<u32>>> {
    let mut out = Vec::with_capacity(kept.len());
    let mut prev: Option<u32> = None;
    let mut free = 0;
    // Each run of free pictures ends at a kept one, or after the last.
    for i in 0..=kept.len() {
        let high = match kept.get(i) {
            Some(None) => {
                free += 1;
                continue;
            }
            Some(Some(l)) if prev.is_some_and(|p| *l <= p) => return None,
            Some(Some(l)) => *l,
            None => MAX_LAYER,
        };
        let low = prev.unwrap_or(0);
        if free > 0 && high <= low + free {
            return None;
        }
        out.extend((1..=free).map(|j| Some(low + j * (high - low) / (free + 1))));
        out.push(Some(high));
        (prev, free) = (Some(high), 0);
    }
    // (The last is MAX_LAYER, the end of the last run.)
    out.pop();
    Some(out)
}

/// A two-key motion through a `BezierMove`'s `path` (keys from the start,
/// over the same time): its places in between, the rest of each key
/// (zoom, turn, opacity) as the motion has it then.
fn along(op: Op, path: Option<Vec<MotionKey>>) -> Op {
    let (Op::SpriteMotion2 { layer, keys, end }, Some(path)) = (&op, path) else { return op };
    let ([a, b], Some(last)) = (keys.as_slice(), path.last()) else { return op };
    if last.ms != b.ms || b.ms == 0 {
        return op;
    }
    // (The motion's end, from the pictures as drawn, may differ from the
    // path's by a cut or a zoom's rounding: the difference spread over it.)
    let mix = |p: i64, q: i64, k: &MotionKey| p + (q - p) * i64::from(k.ms) / i64::from(b.ms);
    let keys = path
        .iter()
        .map(|k| MotionKey {
            x: k.x + mix(0, i64::from(b.x - last.x), k) as i32,
            y: k.y + mix(0, i64::from(b.y - last.y), k) as i32,
            zoom: mix(a.zoom.into(), b.zoom.into(), k) as u32,
            zoom_y: mix(a.zoom_y.into(), b.zoom_y.into(), k) as u32,
            opacity: mix(a.opacity.into(), b.opacity.into(), k) as u32,
            angle: mix(a.angle.into(), b.angle.into(), k) as i32,
            ..*k
        })
        .collect();
    Op::SpriteMotion2 { layer: *layer, keys, end: *end }
}

/// A motion straight from its start to its end (a turn's, [`Compiler::turn_motion`])
/// bent along `path`'s offsets (a `BezierMove` ending as far: the shot logo).
fn bent(op: Op, path: Option<Vec<MotionKey>>) -> Op {
    let (Op::SpriteMotion2 { layer, keys, end }, Some(path)) = (&op, path) else { return op };
    let (Some(last), Some(b)) = (path.last(), keys.last()) else { return op };
    if last.ms != b.ms || b.ms == 0 {
        return op;
    }
    // The path's offset at `t` (ms), and how far it is off the straight line.
    let at = |t: u32| {
        let i = path.iter().position(|k| k.ms >= t).unwrap_or(path.len() - 1);
        let (p, q) = (path[i.saturating_sub(1)], path[i]);
        let f = if q.ms > p.ms { f64::from(t - p.ms.min(t)) / f64::from(q.ms - p.ms) } else { 1.0 };
        let line = f64::from(t) / f64::from(last.ms);
        [(p.x, q.x, last.x), (p.y, q.y, last.y)].map(|(a, b, l)| (f64::from(a) + f64::from(b - a) * f - f64::from(l) * line).round() as i32)
    };
    let keys = keys.iter().map(|k| {
        let [dx, dy] = at(k.ms);
        MotionKey { x: k.x + dx, y: k.y + dy, ..*k }
    }).collect();
    Op::SpriteMotion2 { layer: *layer, keys, end: *end }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spread_keeps_given_layers() {
        // None kept: as evenly as before.
        assert_eq!(spread(&[None, None]), Some(vec![Some(MAX_LAYER / 3), Some(2 * MAX_LAYER / 3)]));
        assert_eq!(spread(&[None, Some(10), None, None, Some(13), None]), Some(vec![Some(5), Some(10), Some(11), Some(12), Some(13), Some(13 + (MAX_LAYER - 13) / 2)]));
        assert_eq!(spread(&[Some(0), None]), Some(vec![Some(0), Some(MAX_LAYER / 2)]));
        // No room, or out of order.
        assert_eq!(spread(&[Some(10), None, Some(11)]), None);
        assert_eq!(spread(&[Some(11), Some(10)]), None);
        assert_eq!(spread(&[None, Some(MAX_LAYER)]), Some(vec![Some(MAX_LAYER / 2), Some(MAX_LAYER)]));
    }
}
