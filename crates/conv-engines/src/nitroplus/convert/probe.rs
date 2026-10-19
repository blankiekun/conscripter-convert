//! The NSS helper census and probe (`nss helpers`): the game's own
//! functions that chain timed built-ins, and what the compiler makes of
//! each in the scheduler's fixed contexts (composition checks, idea 3).

use super::*;
use crate::metrics::TIMING;
use crate::nitroplus::nss::{self, Item, NssFile, Param};
use crate::nitroplus::refs::calls_in;

/// Built-ins that start or wait for timed actions.
const TIMED: &[&str] = &["Fade", "Move", "Zoom", "Rotate", "BezierMove", "Shake", "DrawTransition", "SetVolume", "Wait", "WaitAction", "WaitKey"];

/// Assets that are all there, every picture 1024×576 (the probe and the tests).
pub struct Placeholder {
    pub font: &'static [u8],
}

impl Assets for Placeholder {
    fn image(&mut self, path: &str) -> Option<(String, u32, u32)> {
        Some((path.to_lowercase(), 1024, 576))
    }
    fn sound(&mut self, path: &str) -> Option<String> {
        Some(format!("{path}.ogg"))
    }
    fn color(&mut self, rgb: u32, w: u32, h: u32) -> String {
        format!("color/{rgb:06x}_{w}x{h}.png")
    }
    fn composite(&mut self, parts: &[(String, i64, i64, i64)]) -> Option<(String, i64, i64)> {
        let (_, x, y, _) = parts.first()?;
        let names: Vec<String> = parts.iter().map(|(i, _, _, o)| format!("{i}@{o}")).collect();
        Some((names.join("+"), *x, *y))
    }
    fn transparent(&mut self) -> String {
        "none.png".into()
    }
    fn scaled(&mut self, image: &str, zx: i64, zy: i64, at: (i64, i64, i64, i64)) -> Option<(String, i64, i64, u32, u32)> {
        let (w, h) = (1024 * zx / 1000, 576 * zy / 1000);
        let (x, y) = (at.0 + (at.2 - w) / 2, at.1 + (at.3 - h) / 2);
        // Cut to the screen past 4096 px, as Assets::scaled.
        if w.max(h) > 4096 {
            return Some((format!("{image}@{zx}x{zy}_screen"), x.max(0), y.max(0), (x + w).min(1024).saturating_sub(x.max(0)) as u32, (y + h).min(576).saturating_sub(y.max(0)) as u32));
        }
        Some((format!("{image}@{zx}x{zy}"), x, y, w as u32, h as u32))
    }
    fn mirrored(&mut self, image: &str, x: bool, y: bool) -> Option<String> {
        Some(format!("{image}~{}{}", if x { "x" } else { "" }, if y { "y" } else { "" }))
    }
    fn strip(&mut self, parts: &[&str]) -> Option<(String, u32, u32)> {
        Some((parts.join("|"), 100 * parts.len() as u32, 100))
    }
    fn movie_info(&mut self, path: &str) -> Option<MovieInfo> {
        let small = path.contains("k100");
        Some(MovieInfo { w: if small { 400 } else { 1024 }, h: if small { 432 } else { 576 }, ms: 2000, sound: path.contains("OP") })
    }
    fn movie_file(&mut self, path: &str, sound: bool) -> Option<String> {
        Some(format!("movie/{path}{}", if sound { "" } else { "-mute" }))
    }
    fn rgba(&mut self, _path: &str) -> Option<image::RgbaImage> {
        None
    }
    fn frames_movie(&mut self, name: &str, _fps: u32, _frames: u32, _draw: &mut dyn FnMut(u32, &mut image::RgbImage)) -> Option<String> {
        Some(format!("movie/{name}.mp4"))
    }
    fn frames_strip(&mut self, name: &str, _w: u32, _h: u32, _frames: u32, _draw: &mut dyn FnMut(u32, &mut image::RgbaImage)) -> Option<String> {
        Some(format!("system/strip/{name}.png"))
    }
    fn font(&self) -> &[u8] {
        self.font
    }
    fn delayed_sound(&mut self, path: &str, delay_ms: u32) -> Option<String> {
        Some(format!("{path}+{delay_ms}.ogg"))
    }
    fn sound_ms(&mut self, _path: &str) -> Option<u32> {
        Some(4000)
    }
    fn overlaid(&mut self, base: &str, top: &str, zoom: i64, opacity: i64) -> Option<String> {
        Some(format!("{base}+{top}@{zoom}x{opacity}"))
    }
    fn stenciled(&mut self, image: &str, mask: &str, colour: bool, (dx, dy): (i64, i64), zoom: (i64, i64)) -> Option<String> {
        Some(format!("{image}%{mask}{}@{dx},{dy}x{}", if colour { "*" } else { "" }, zoom.0))
    }
    fn movie_frames(&mut self, path: &str, at: Option<u32>, alpha: bool, dark: bool) -> Option<(String, u32, u32, u32)> {
        let (w, h) = if alpha { (200, 108) } else { (1024, 576) };
        let cells = if at.is_some() { 1 } else { 30 };
        Some((format!("{path}@{at:?}{}", if dark { "d" } else { "" }), cells, w, h))
    }
    fn movie_sprite(&mut self, path: &str, alpha: bool, dark: bool) -> Option<(String, u32, u32)> {
        let (w, h) = if alpha { (400, 216) } else { (1024, 576) };
        Some((format!("{path}{}{}.mp4", if dark { "_dark" } else { "" }, if alpha { "_alpha" } else { "" }), w, h))
    }
}

/// A function whose body calls two or more timed built-ins: its name, file,
/// those calls in order, and how often the scripts call it.
pub struct Helper {
    pub name: String,
    pub file: String,
    pub timed: Vec<String>,
    pub calls: usize,
    /// Its first parameter takes an object name (a string parameter).
    pub takes_name: bool,
    pub params: usize,
}

/// Every [`Helper`] in `files`, most called first.
pub fn census(files: &BTreeMap<String, NssFile>) -> Vec<Helper> {
    let mut called: HashMap<String, usize> = HashMap::new();
    let mut helpers = Vec::new();
    for (file, f) in files {
        for item in &f.items {
            let body = match item {
                Item::Function(f) => &f.body,
                Item::Chapter(b) | Item::Scene(b) => &b.body,
                Item::Include(_) => continue,
            };
            let calls = calls_in(body);
            for c in &calls {
                *called.entry(c.clone()).or_default() += 1;
            }
            if let Item::Function(func) = item {
                let timed: Vec<String> = calls.into_iter().filter(|c| TIMED.contains(&c.as_str())).collect();
                if timed.len() >= 2 {
                    let takes_name = matches!(func.params.first(), Some(Param::Str(_)));
                    helpers.push(Helper { name: func.name.clone(), file: file.clone(), timed, calls: 0, takes_name, params: func.params.len() });
                }
            }
        }
    }
    for h in &mut helpers {
        h.calls = called.get(&h.name).copied().unwrap_or(0);
    }
    helpers.sort_by(|a, b| b.calls.cmp(&a.calls).then(a.name.cmp(&b.name)));
    helpers
}

/// The fixed contexts a helper is probed in: what runs before it.
pub const CONTEXTS: &[(&str, &str)] = &[
    ("after an unwaited 16 s Move of it", "\tCreateTexture(\"p\", 100, 0, 0, \"cg/a.jpg\");\n\tWait(0);\n\tMove(\"p\", 16000, @320, @0, null, false);\n"),
    ("after an unwaited 16 s Move of another", "\tCreateTexture(\"p\", 100, 0, 0, \"cg/a.jpg\");\n\tCreateTexture(\"q\", 200, 0, 0, \"cg/b.jpg\");\n\tWait(0);\n\tMove(\"q\", 16000, @320, @0, null, false);\n"),
    ("a picture just made, moved unwaited 16 s", "\tCreateTexture(\"p\", 100, -560, 0, \"cg/a.jpg\");\n\tWait(0);\n\tMove(\"p\", 16000, @320, @0, null, false);\n"),
];

/// One probe: the time ledger's notes beyond the context's own, and
/// whether a 16 s motion is still drawn (the context's move kept),
/// compiling `call` after `context` with the game's own library (as the
/// scene `probe.nss`, put in `files`).
pub fn probe(files: &mut BTreeMap<String, NssFile>, game: &'static Game, context: &str, call: &str) -> Result<(BTreeMap<String, usize>, bool), String> {
    let mut run = |call: &str| -> Result<Compiled, String> {
        let src = format!("#include \"nss/function.nss\"\nscene probe.nss\n{{\n{context}\t{call}\n\tWait(100);\n}}\n");
        files.insert("probe.nss".into(), nss::parse(&src).map_err(|e| e.to_string())?);
        compile_scene(files, "probe.nss", "probe.nss", game, &mut Placeholder { font: &[] })
    };
    let base = run("")?.notes;
    let c = run(call)?;
    let kept = c.ops.iter().any(|op| matches!(op, Op::SpriteMotion2 { keys, .. } if keys.last().is_some_and(|k| k.ms >= 16000)));
    let notes = c.notes.into_iter().filter(|(n, _)| TIMING.contains(&n.as_str())).filter_map(|(n, c)| Some((n.clone(), c.checked_sub(base.get(&n).copied().unwrap_or(0)).filter(|&c| c > 0)?))).collect();
    Ok((notes, kept))
}
