//! Objects: the names arguments give and the objects they match,
//! creating and deleting objects, movies, `Request`.

use super::*;

impl<'a> Compiler<'a, '_> {
    /// A string argument that names a variable (`"$Transition"`) means
    /// that variable's value.
    pub(super) fn deref(&self, v: &Value) -> String {
        let mut s = v.string();
        for _ in 0..4 {
            match self.vars.get(&s) {
                Some(v) if s.starts_with(['$', '#']) => s = v.string(),
                _ => break,
            }
        }
        s
    }

    /// The object a name argument means: `$var` names the variable holding
    /// it, `@x` goes through the alias table.
    /// A string argument, or the value of the variable it names
    /// (`CreateSound("x", SE, "$場所指定")`, 続・殺戮のジャンゴ's
    /// `CreateSE`).
    pub(super) fn named_string(&self, v: &Value) -> String {
        let s = v.string();
        match self.vars.get(&s) {
            Some(v) if s.starts_with(['$', '#']) => v.string(),
            _ => s,
        }
    }

    pub(super) fn object_name(&self, v: &Value) -> String {
        let mut s = self.var_string(v);
        while let Some(rest) = s.strip_prefix('@') {
            s = match self.aliases.get(rest) {
                Some(o) => o.clone(),
                // A child named from its parent's alias (`@移動窓/立絵40`,
                // `移動窓` being `窓/移動窓`).
                None => match rest.split_once('/') {
                    Some((parent, child)) if self.game.family && self.aliases.contains_key(parent) => format!("{}/{child}", self.aliases[parent]),
                    _ => rest.to_string(),
                },
            };
        }
        s
    }

    /// A string argument, or the value of the variable it names (`"$顔ナット名"`).
    pub(super) fn var_string(&self, v: &Value) -> String {
        let mut s = v.string();
        for _ in 0..4 {
            match self.vars.get(&s) {
                Some(v) if s.starts_with(['$', '#']) => s = v.string(),
                _ => break,
            }
        }
        s
    }

    /// Objects matched by a name argument (a trailing `*` is a prefix
    /// wildcard, over names and aliases: `@StMR*` finds `遠４/StMR01`,
    /// alias `StMR01`).
    pub(super) fn matching(&self, v: &Value) -> Vec<String> {
        let name = self.object_name(v);
        // A `*` before the end (`@選択肢*/MouseUsual/*`): any characters;
        // the first part may name aliases.
        if self.game.family && name.trim_end_matches('*').contains('*') {
            let mut found: std::collections::BTreeSet<String> = self.objects.keys().filter(|k| glob(&name, k)).cloned().collect();
            if let Some((first, rest)) = name.split_once('/') {
                for (a, o) in &self.aliases {
                    if glob(first, a) {
                        let pattern = format!("{o}/{rest}");
                        found.extend(self.objects.keys().filter(|k| glob(&pattern, k)).cloned());
                    }
                }
            }
            return found.into_iter().collect();
        }
        match name.strip_suffix('*') {
            Some(prefix) => {
                // Not into the children of a container it matches (装甲悪鬼村正
                // under wine: `Fade("@窓*")` leaves a window's or a name's
                // children as they are, `Fade("@絵*")` fades a picture's
                // too); with `family`, of any object: 続・殺戮のジャンゴ's
                // `Fade("@frame0*")` leaves the faces in `frame03/顔窓31`.
                let in_matched = |k: &str| {
                    k[prefix.len()..]
                        .find('/')
                        .is_some_and(|i| self.objects.get(&k[..prefix.len() + i]).is_some_and(|o| self.game.family || o.container))
                };
                let mut found: std::collections::BTreeSet<String> = self
                    .objects
                    .keys()
                    .filter(|k| k.starts_with(prefix) && !in_matched(k))
                    .cloned()
                    .collect();
                if !prefix.is_empty() {
                    found.extend(self.aliases.iter().filter(|(a, o)| a.starts_with(prefix) && self.objects.contains_key(*o)).map(|(_, o)| o.clone()));
                }
                found.into_iter().collect()
            }
            None if self.objects.contains_key(&name) => vec![name],
            None => Vec::new(),
        }
    }

    /// Whether a name or wildcard matches no object, kept or not: an
    /// action on it starts nothing, and a waited one doesn't wait (src:
    /// Muramasa.exe cb_Fade 0044f020 queues no action; the wait 00445ac0
    /// waits on the objects' action slots).
    pub(super) fn no_object(&self, v: &Value) -> bool {
        let name = self.object_name(v);
        self.matching(v).is_empty() && !self.untracked.iter().any(|k| glob(&name, k))
    }

    pub(super) fn create(&mut self, name: String, kind: Kind<'a>, priority: i64, w: i64, h: i64) -> &mut Object<'a> {
        self.seq += 1;
        let o = Object { kind, priority, x: 0, y: 0, w, h, opacity: 1000, locked: false, hideable: false, seq: self.seq, zoom: (1000, 1000), trail: None, blend: BLEND_NORMAL, vertex: None, turn: (0, 0, 0), shake: None, loop_fade: None, clip: None, over_text: false, container: false, erased: false, tone: None, shade: None, wave: None, stencil: None, cut: None, screen_children: false, fade: None };
        self.objects.insert(name.clone(), o);
        self.objects.get_mut(&name).unwrap()
    }

    pub(super) fn delete(&mut self, name: &str) {
        // Its children go with it (a container's always: a wildcard that
        // matches it doesn't reach them).
        let with_children = self.game.family || self.objects.get(name).is_some_and(|o| o.container);
        let children: Vec<String> = self.objects.keys().filter(|_| with_children).filter(|k| k.len() > name.len() && k.starts_with(name) && k.as_bytes()[name.len()] == b'/').cloned().collect();
        for c in children {
            self.delete(&c);
        }
        let Some(o) = self.objects.remove(name) else { return };
        // A new object of the name (maybe on a reused layer) has no motion to wait for.
        self.unwaited_motions.remove(name);
        self.stop_loops(name);
        if let Kind::Sound(s) = o.kind
            && let Some(file) = s.playing
        {
            self.ops.push(if s.bgm { Op::BgmStop { fade_out_ms: 0 } } else { Op::SeStop { file: Some(file) } });
        }
    }

    /// Movie `name`, if it is one that fills the screen and is playing:
    /// plays it as a movie (the sounds started with it play beside it),
    /// then shows its last frame. The script's waits for its remaining
    /// time are counted as played ([`MovieObject::credit`]).
    pub(super) fn play_movie(&mut self, name: &str) {
        let Some(Object { kind: Kind::Movie(m), opacity, .. }) = self.objects.get(name) else { return };
        let MovieState::Playing { at } = m.state else { return };
        if !m.full_screen(self.game.screen) {
            return;
        }
        let (path, opacity, left, (mx, my)) = (m.path.clone(), *opacity, m.info.ms.saturating_sub(at), self.objects[name].mirror());
        let Some(mut file) = self.assets.movie_file(&path, false) else { return };
        if (mx || my)
            && let Some(turned) = self.assets.mirrored(&file, mx, my)
        {
            file = turned;
        }
        // Hidden while the screen catches up, then the last frame shows.
        self.objects.get_mut(name).unwrap().opacity = 0;
        self.sync(0);
        let o = self.objects.get_mut(name).unwrap();
        // Played opaque, so its last frame is (MovieSEStart2 fades the
        // cut in only after its WaitAction).
        o.opacity = if opacity == 0 { 1000 } else { opacity };
        if let Kind::Movie(m) = &mut o.kind {
            m.state = MovieState::Played;
            m.credit = left;
        }
        // The last frame is loaded before the movie and shown at once after
        // it: the engine redraws the screen as the movie ends, which then
        // already holds it (else the scene before flashes and fades back).
        let start = self.ops.len();
        self.sync(0);
        let movie = Op::SceneMovie { file, skippable: true };
        match self.ops[start..].iter().position(|op| matches!(op, Op::BeginTransition { .. })) {
            Some(i) if matches!(self.ops.last(), Some(Op::EndTransition)) => {
                if let Op::BeginTransition { fade_ms, mask, .. } = &mut self.ops[start + i] {
                    (*fade_ms, *mask) = (0, None);
                }
                let end = self.ops.len() - 1;
                self.insert_op(end, movie);
            }
            _ => self.ops.push(movie),
        }
    }

    /// A wait of `ms`: the time movies already played as movies take off
    /// it (what is left is returned), a movie on screen that it mostly
    /// waits for plays as a movie, and the others go on by that much.
    pub(super) fn movie_wait(&mut self, mut ms: u32) -> u32 {
        let screen = self.game.screen;
        for o in self.objects.values_mut() {
            if let Kind::Movie(m) = &mut o.kind {
                let used = m.credit.min(ms);
                m.credit -= used;
                ms -= used;
            }
        }
        if ms == 0 {
            return 0;
        }
        let waited: Vec<String> = self
            .objects
            .iter()
            .filter(|(_, o)| {
                matches!(&o.kind, Kind::Movie(m) if m.full_screen(screen) && o.opacity > 0
                    && matches!(m.state, MovieState::Playing { at } if ms * 2 >= m.info.ms.saturating_sub(at)))
            })
            .map(|(n, _)| n.clone())
            .collect();
        if let Some(name) = waited.first() {
            self.play_movie(name);
            if let Some(Object { kind: Kind::Movie(m), .. }) = self.objects.get_mut(name) {
                let used = m.credit.min(ms);
                m.credit -= used;
                ms -= used;
            }
        }
        for o in self.objects.values_mut() {
            if let Kind::Movie(m) = &mut o.kind
                && let MovieState::Playing { at } = m.state
            {
                let at = at + ms;
                m.state = MovieState::Playing { at: if m.looped { at % m.info.ms.max(1) } else { at.min(m.info.ms) } };
            }
        }
        ms
    }

    pub(super) fn texture_source(&mut self, src: &str) -> Option<(String, u32, u32)> {
        if src.contains('/') {
            return self.assets.image(src);
        }
        let name = self.object_name(&Value::Str(src.to_string()));
        match self.objects.get(&name) {
            Some(Object { kind: Kind::Image { image }, w, h, .. }) => Some((image.clone(), *w as u32, *h as u32)),
            _ => self.loaded_images.get(&name).cloned(),
        }
    }

    pub(super) fn request(&mut self, name: &str, action: &str) {
        if action == "play"
            && let Some(&at) = self.running.get(self.running.len().saturating_sub(2))
        {
            self.played_at.insert(name.to_string(), at);
        }
        let Some(o) = self.objects.get_mut(name) else { return };
        if let Kind::Movie(m) = &mut o.kind {
            match action {
                "play" => {
                    m.state = match m.state {
                        MovieState::Paused { at } => MovieState::Playing { at },
                        _ => MovieState::Playing { at: 0 },
                    };
                    return;
                }
                "stop" => {
                    m.state = MovieState::Paused { at: 0 };
                    return;
                }
                "pause" => {
                    if let MovieState::Playing { at } = m.state {
                        m.state = MovieState::Paused { at };
                    }
                    return;
                }
                // Drawn as its colour in black over the scene (the
                // movie's re-encode), not subtracted.
                "subrender" => {
                    m.dark = true;
                    return;
                }
                _ => {}
            }
        }
        match action {
            "normalrender" => o.blend = BLEND_NORMAL,
            "addrender" => o.blend = BLEND_ADD,
            "mulrender" => o.blend = BLEND_MULTIPLY,
            "subrender" => o.blend = BLEND_SUBTRACT,
            "lock" => o.locked = true,
            "unlock" => o.locked = false,
            "hideable" => o.hideable = true,
            // A BGM still playing, played again by a music process (EndSong
            // when the roll outlasts the song): CONscripter's `bgm` loops it
            // already. shortcut: the exe's 1 s gap and its stop after the
            // second time aren't kept.
            "play" if self.process_clock.is_some() && matches!(&o.kind, Kind::Sound(s) if s.bgm && s.playing.is_some()) => {}
            "play" => {
                // `Play` on a loop that is still playing goes on with it
                // (`md04_048vs` calls MusicStart again on its loop to change
                // the volume and speed); it isn't started a second time.
                // Nor is a disused one-shot still playing (kept only for
                // `WaitPlay`; `MusicStart("SE01*")` in ma04_013vs).
                if let Kind::Sound(s) = &mut o.kind
                    && !(s.playing.is_some() && (s.looped || s.disused))
                {
                    s.starting = true;
                    s.fade_ms = 0;
                }
            }
            "disused" => match &mut o.kind {
                // Looping sounds play on until stopped; one-shots until
                // their end (`forget_ended_sounds`).
                Kind::Sound(s) => s.disused = true,
                // src: under wine ma00_000:137's `FadeDelete("絵背景10", 3000,
                // false)` still shows the sky after the 1 s `Wait`: the fade runs on.
                _ if !o.locked && self.pending_fade.contains_key(name) => self.disused_fading.push((name.to_string(), o.seq)),
                _ if !o.locked => self.delete(name),
                _ => {}
            },
            "enter" | "erase" if matches!(&o.kind, Kind::Image { image } if text_object::is_text(image)) => o.erased = action == "erase",
            "enter" => self.show_text(name),
            "start" if matches!(o.kind, Kind::Process { .. }) => {
                self.paper += 1;
                self.start_process(name);
                self.paper -= 1;
            }
            "smoothing" | "passive" | "erase" | "nolog" | "noicon" => {}
            other => self.note(format!("Request {other}")),
        }
    }
}
