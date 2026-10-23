//! Timing and drawing: actions over time and their waits, and how an
//! object is drawn (opacity, stencils, windows, cameras).

use super::*;

impl<'a> Compiler<'a, '_> {
    /// A `Move`'s or `Zoom`'s tempo for its targets' motion at the next
    /// sync (the last one given wins).
    pub(super) fn set_tempo(&mut self, tempo: Option<(u8, u32)>, targets: &[String]) {
        for t in targets {
            match tempo {
                Some(e) => _ = self.pending_tempo.insert(t.clone(), e),
                None => _ = self.pending_tempo.remove(t),
            }
        }
    }

    /// An action of `ms` with the `wait` argument of the timed built-ins:
    /// true blocks for the action, false lets the script go on.
    pub(super) fn timed(&mut self, ms: i64, wait: &Value, targets: &[String]) {
        let ms = ms.max(0) as u32;
        let wait = match self.action_process.clone() {
            Some(p) => {
                if !matches!(wait, Value::Null | Value::Int(0)) {
                    self.process_lead += ms;
                }
                let e = self.pending_by.entry(p).or_default();
                *e = (*e).max(ms);
                &Value::Null
            }
            None => wait,
        };
        match wait {
            // src: Django.exe 004310e0: a number n waits n ms, the action going on.
            &Value::Int(n) if n > 1 && (n as u32) < ms => {
                // (Drawn as `Wait` draws the actions still running, a click
                // ending it as it ends a whole one.)
                self.timed(i64::from(ms), &Value::Null, targets);
                let n = n as u32;
                let shown = self.take_running(n);
                self.sync(shown);
                if n > shown {
                    self.ops.push(Op::Wait { ms: n - shown });
                }
            }
            Value::Null | Value::Int(0) => {
                self.pending_ms = self.pending_ms.max(ms);
                for t in targets {
                    let p = self.pending_by.entry(t.clone()).or_default();
                    *p = (*p).max(ms);
                }
            }
            _ => {
                if matches!(wait, Value::Int(n) if *n > 1) {
                    self.note("timed action that waits longer than its time");
                }
                for t in targets {
                    let p = self.pending_by.entry(t.clone()).or_default();
                    *p = (*p).max(ms);
                }
                self.sync(ms);
            }
        }
    }

    /// An object's opacity as drawn: 0 while it is turned edge-on, and a
    /// child's times its parents', which also hide it when edge-on (a face
    /// in `frame03` hides and turns away with the frame, under wine).
    pub(super) fn drawn_opacity(&self, name: &str, own: i64) -> i64 {
        let edge_on = |o: &Object| o.turn.0.rem_euclid(180) == 90 || o.turn.1.rem_euclid(180) == 90;
        if self.objects.get(name).is_some_and(|o| edge_on(o) || o.erased) {
            return 0;
        }
        if !self.game.relative_children {
            return own;
        }
        let mut end = 0;
        let mut opacity = own;
        while let Some(i) = name[end..].find('/') {
            end += i;
            if let Some(p) = self.objects.get(&name[..end]) {
                opacity = if edge_on(p) { 0 } else { opacity * p.opacity / 1000 };
            }
            end += 1;
        }
        opacity
    }

    /// Picture `image` of object `name` cut by the stencils it is in
    /// (`CreateStencil`): its opacity times theirs where they lie over
    /// it, none elsewhere (the exe's mode 128 makes the stencil's
    /// pixels its alpha). `None` when it is in none.
    // shortcut: cut for one place and zoom at a time, so a picture moved
    // or zoomed over time under a stencil (none in 装甲悪鬼村正) moves as
    // that one cut; a zoom about a vertex is cut as one about the centre.
    pub(super) fn stenciled(&mut self, name: &str, image: &str) -> Option<String> {
        let stencils: Vec<_> = name
            .match_indices('/')
            .filter_map(|(i, _)| {
                let s = self.objects.get(&name[..i])?;
                let (sx, sy) = self.parent_offset(&name[..i]);
                Some((s.stencil.clone()?, s.x + sx, s.y + sy))
            })
            .collect();
        let (px, py) = self.parent_offset(name);
        let o = self.objects.get(name)?;
        let (x, y, zoom) = (o.x + px, o.y + py, o.zoom);
        let key = format!("{image}{stencils:?}{x},{y}{zoom:?}");
        if let Some((k, cut)) = &o.cut
            && *k == key
        {
            return Some(cut.clone());
        }
        let mut out: Option<String> = None;
        for ((mask, colour), sx, sy) in stencils {
            out = Some(self.assets.stenciled(out.as_deref().unwrap_or(image), &mask, colour, (sx - x, sy - y), zoom)?);
        }
        let cut = out?;
        self.objects.get_mut(name)?.cut = Some((key, cut.clone()));
        Some(cut)
    }

    /// The rectangle of the windows `name` is in (`CreateWindow`), their
    /// overlap when there are several; `None` when it is in none.
    pub(super) fn window_clip(&self, name: &str) -> Option<(i64, i64, i64, i64)> {
        let mut clip: Option<(i64, i64, i64, i64)> = None;
        for (i, _) in name.match_indices('/') {
            let Some((x, y, (w, h))) = self.objects.get(&name[..i]).and_then(|o| Some((o.x, o.y, o.clip?))) else { continue };
            // shortcut: a window seen through a camera (`カメラ/移動窓`)
            // doesn't clip; it would need the camera's zoom and depth.
            if name[..i].match_indices('/').any(|(j, _)| self.objects.get(&name[..j]).is_some_and(|o| matches!(o.kind, Kind::Camera))) {
                continue;
            }
            // With Game::relative_children a window's place counts from its
            // parents' (続・殺戮のジャンゴ's `窓/移動窓`).
            let (px, py) = self.parent_offset(&name[..i]);
            let (x, y) = (x + px, y + py);
            clip = Some(match clip {
                None => (x, y, w, h),
                Some((cx, cy, cw, ch)) => {
                    let (l, t) = (cx.max(x), cy.max(y));
                    (l, t, ((cx + cw).min(x + w) - l).max(0), ((cy + ch).min(y + h) - t).max(0))
                }
            });
        }
        clip
    }

    /// Where a child object's place counts from: the sum of the places of
    /// the objects its name runs through (`@frame03/顔窓31` at 25,25 in
    /// `frame03` at 25,80 is at 50,105).
    pub(super) fn parent_offset(&self, name: &str) -> (i64, i64) {
        let (mut x, mut y) = (0, 0);
        let mut end = 0;
        while let Some(i) = name[end..].find('/') {
            end += i;
            match self.objects.get(&name[..end]) {
                // Seen through a camera: the other way.
                Some(p) if matches!(p.kind, Kind::Camera) => (x, y) = (x - p.x, y - p.y),
                // A child of a window whose last argument isn't `true`, or of a
                // name (but with Game::relative_children), is at its own place.
                Some(p) if p.container && p.screen_children => {}
                Some(p) => (x, y) = (x + p.x, y + p.y),
                None => {}
            }
            end += 1;
        }
        (x, y)
    }

    /// Whether `name` is a window that places its children from its own
    /// place and rectangle (`CreateWindow`'s last argument `true`).
    pub(super) fn places_children(&self, name: &str) -> bool {
        self.objects.get(name).is_some_and(|p| p.container && !p.screen_children && p.clip.is_some() && !matches!(p.kind, Kind::Camera))
    }

    /// The size a named place (`Center`, `InBottom`) of `name` counts in:
    /// its window's ([`Compiler::places_children`]), else the screen's.
    // src: exe 00458d50 (parse_object_place: the rect at the parent's +0x11f4 when its +0x9d4 is set; 00443500 sets it to the window's)
    // src: Django.exe FUN_004403a0 alike (the rect at +0x7f8 when +0x3c8 is set)
    // shortcut: the nearest parent only, so a named place deeper in a window
    // counts from the screen (none in 装甲悪鬼村正 or 続・殺戮のジャンゴ).
    pub(super) fn frame(&self, name: &str) -> (i64, i64) {
        match name.rfind('/') {
            Some(i) if self.places_children(&name[..i]) => self.objects[&name[..i]].clip.unwrap_or(self.game.screen),
            _ => self.game.screen,
        }
    }

    /// How much the cameras `name` is seen through magnify an object of
    /// `priority` ([`Kind::Camera`]); `None`: at or before the camera, not
    /// drawn.
    // src: shot (続・殺戮のジャンゴ under wine: boxes at four priorities, six camera settings; docs/games/django.md)
    // src: exe 00489a52 (装甲悪鬼村正, FUN_00489570: a camera's picture at depth 1000 − priority, seen at the camera's z)
    pub(super) fn camera_zoom(&self, name: &str, priority: i64) -> Option<f64> {
        let mut zoom = 1.0;
        let mut end = 0;
        while let Some(i) = name[end..].find('/') {
            end += i;
            if let Some(p) = self.objects.get(&name[..end]).filter(|p| matches!(p.kind, Kind::Camera)) {
                if priority >= 1000 {
                    return None;
                }
                zoom = zoom * p.zoom.0 as f64 / (1000 - priority) as f64;
            }
            end += 1;
        }
        Some(zoom)
    }

    /// Whether the picture `name` lies over the text window and its text
    /// ([`Op::SpriteOverText`]): a subtitle strip, or one above the window
    /// the text is in (`<PRE @box0>`: `CreateWindow("box0", 20001, …)`)
    /// in priority, or as high and made after it.
    pub(super) fn over_text(&self, name: &str) -> bool {
        let Some(o) = self.objects.get(name) else { return false };
        if o.over_text {
            return true;
        }
        let Some(text) = self.vars.get("$SYSTEM_present_text").map(Value::string) else { return false };
        let text = text.trim_start_matches('@');
        let window = text.split_once('/').map_or(text, |w| w.0);
        let Some(w) = self.objects.get(window).or_else(|| self.objects.get(text)) else { return false };
        (o.priority, o.seq) > (w.priority, w.seq)
    }

    /// The screen is frozen under an opaque `Plain` cover while one is up:
    /// the top one's priority.
    pub(super) fn frozen_at(&self) -> Option<i64> {
        self.objects.iter().filter(|(n, o)| matches!(o.kind, Kind::Cover { .. }) && o.opacity >= 1000 && !self.shows_froze(o) && !self.grabs(n, o)).map(|(_, o)| o.priority).max()
    }

    /// Hides the text window once no picture of it is visible.
    pub(super) fn check_window(&mut self) {
        let visible = self.objects.values().any(|o| o.hideable && o.opacity > 0 && matches!(o.kind, Kind::Image { .. }));
        if self.window_visible && !visible {
            self.ops.push(Op::HideTextWindow);
            self.window_visible = false;
        }
    }
}

/// Where a point at `(x, y)` (the camera's place taken off) is drawn
/// through a camera magnifying `zoom` times ([`Compiler::camera_zoom`]):
/// moved away from the screen's centre, cut toward 0 as the exe's `_ftol`.
// src: exe 004899e1 (装甲悪鬼村正, FUN_00489570's camera branch: x·z/d − (z/d − 1)·512, −y·z/d − (z/d − 1)·288 with y up)
pub(super) fn through_camera((x, y): (i64, i64), zoom: f64, (w, h): (i64, i64)) -> (i64, i64) {
    let (sx, sy) = ((w / 2) as f64, (h / 2) as f64);
    ((sx + (x as f64 - sx) * zoom) as i64, (sy + (y as f64 - sy) * zoom) as i64)
}

#[cfg(test)]
mod tests {
    use super::through_camera;

    #[test]
    fn a_camera_projects_as_the_exe() {
        // research/nitroplus/camera_exe.py <Muramasa.exe>: FUN_00489570 with a camera,
        // one vertex: (x, y, camera x, y, z, depth priority, screen x, y).
        const EXE_CAMERA: [[i64; 8]; 8] = [[0, 0, 0, 0, 1000, 0, 0, 0], [100, 50, 0, 0, 500, 0, 306, 169], [100, 50, 0, 0, 1000, 500, -312, -188], [700, 400, 30, -20, 1200, 200, 749, 486], [-300, 900, 100, 100, 800, -500, 25, 561], [512, 288, 0, 0, 3000, 900, 512, 288], [1, 2, 0, 0, 999, 1, 1, 2], [250, 100, -40, 60, 700, 300, 290, 40]];
        for [x, y, cx, cy, z, p, sx, sy] in EXE_CAMERA {
            assert_eq!(through_camera((x - cx, y - cy), z as f64 / (1000 - p) as f64, (1024, 576)), (sx, sy), "{x},{y} cam {cx},{cy},{z} p {p}");
        }
    }
}
