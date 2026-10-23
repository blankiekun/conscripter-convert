//! The title's openings (`0_boot.nss`, `TitleFade`), drawn frame by frame
//! into silent movies: the pictures are blended in ways CONscripter's
//! sprites can't (added, multiplied, overlaid), zoomed, turned and
//! shaken at once. Each movie ends on the title as it rests, buttons
//! included, so the title's sprites take over without a change.
//!
//! Times are ms from the title music's start; opacities are per mille,
//! zooms per mille, as in the script.

use image::{RgbImage, RgbaImage};

use super::super::{Assets, SCREEN_H, SCREEN_W};

// ours: the frame rate of our pre-rendered opening cells (the exe draws
// it live)
const FPS: u32 = 30;

/// Where the title's pictures are and where they go: the Japanese
/// release's ([`JP`]) or the English port's ([`EN`]).
pub struct TitleLook {
    /// Its pictures: `en/…` takes them from the English `cg.npk`.
    pub dir: &'static str,
    pub logo: (f64, f64),
    /// しおりから, 最初から, 電光記録, 設定画面, 特別付加, 終了する.
    pub buttons: [(&'static str, f64, f64); 6],
    /// Where ライナー画像 leads.
    pub notes: &'static str,
    /// しおりから and 電光記録 only while there is a save, 特別付加 once
    /// 英雄編 or 復讐編 is over (the English port's `sys_titlelogo.nut`;
    /// `0_boot.nss` shows them all).
    pub hides: bool,
}

/// `0_boot.nss`: the buttons upright in two groups of three at y 150.
// src: 0_boot.nss TitleSet (タイトルロゴ 346,40; $TitleNutName; LinerNotes)
pub const JP: TitleLook = TitleLook {
    dir: "cg/sys/title",
    logo: (346.0, 40.0),
    buttons: [("しおりから", 822.0, 150.0), ("最初から", 762.0, 150.0), ("電光記録", 702.0, 150.0), ("設定画面", 282.0, 150.0), ("特別付加", 222.0, 150.0), ("終了する", 162.0, 150.0)],
    notes: "http://www.nitroplus.co.jp/secret/muramasamune",
    hides: false,
};

/// The English port's (`sys_titlelogo.nut`, its `LANG_NAME == "en"`
/// places): the logo a full screen 40 px up, the buttons in a row at
/// y 467, from the English `cg.npk` ([`super::super::Assets`] `en/…`).
// src: sys_titlelogo.nut (the "en" places, the logo 40 up, the notes' URL)
pub const EN: TitleLook = TitleLook {
    dir: "en/cg/sys/title",
    logo: (0.0, -40.0),
    buttons: [("しおりから", 280.0, 467.0), ("最初から", 158.0, 467.0), ("電光記録", 402.0, 467.0), ("設定画面", 646.0, 467.0), ("特別付加", 524.0, 467.0), ("終了する", 768.0, 467.0)],
    notes: "http://secret.muramasa.jastusa.com/",
    hides: true,
};

/// How a picture is drawn over what is under it (NSS `Request`).
#[derive(Clone, Copy, PartialEq)]
enum Blend {
    Normal,
    /// `AddRender`
    Add,
    /// `MulRender`
    Mul,
    /// `OverlayRender`
    Overlay,
}

/// NSS tempos (`research/nitroplus/builtins.md`).
#[derive(Clone, Copy)]
enum Ease {
    Linear,
    /// `Axl1`-`Axl3`: ease-in, quadratic to quartic.
    In(i32),
    /// `Dxl1`-`Dxl3`: ease-out.
    Out(i32),
}

impl Ease {
    fn at(self, p: f64) -> f64 {
        match self {
            Ease::Linear => p,
            Ease::In(n) => p.powi(n + 1),
            Ease::Out(n) => 1.0 - (1.0 - p).powi(n + 1),
        }
    }
}

const AXL1: Ease = Ease::In(1);
const DXL3: Ease = Ease::Out(3);

/// A value changed by timed actions (`Fade`, `Zoom`, …): each goes from
/// wherever the value is when it starts.
#[derive(Clone)]
struct Track {
    start: f64,
    steps: Vec<(f64, f64, f64, Ease)>,
}

impl Track {
    fn new(start: f64) -> Self {
        Track { start, steps: Vec::new() }
    }

    /// From `at` ms, to `to` over `ms` (0: at once).
    fn to(mut self, at: f64, ms: f64, to: f64, ease: Ease) -> Self {
        self.steps.push((at, ms, to, ease));
        self.steps.sort_by(|a, b| a.0.total_cmp(&b.0));
        self
    }

    fn at(&self, t: f64) -> f64 {
        Self::value(self.start, &self.steps, t)
    }

    fn value(start: f64, steps: &[(f64, f64, f64, Ease)], t: f64) -> f64 {
        let mut v = start;
        for (i, &(at, ms, to, ease)) in steps.iter().enumerate() {
            if t < at {
                break;
            }
            let from = Self::value(start, &steps[..i], at);
            let p = if ms <= 0.0 { 1.0 } else { ((t - at) / ms).min(1.0) };
            v = from + (to - from) * ease.at(p);
        }
        v
    }
}

/// A picture in premultiplied alpha, 0-1.
struct Pic {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Pic {
    fn new(img: &RgbaImage) -> Self {
        let px = img
            .pixels()
            .map(|p| {
                let a = p.0[3] as f32 / 255.0;
                [p.0[0] as f32 / 255.0 * a, p.0[1] as f32 / 255.0 * a, p.0[2] as f32 / 255.0 * a, a]
            })
            .collect();
        Pic { w: img.width() as usize, h: img.height() as usize, px }
    }

    fn colour(rgb: [u8; 3], w: usize, h: usize) -> Self {
        let c = rgb.map(|v| v as f32 / 255.0);
        Pic { w, h, px: vec![[c[0], c[1], c[2], 1.0]; w * h] }
    }

    /// Bilinear, transparent outside.
    fn sample(&self, x: f64, y: f64) -> [f32; 4] {
        let (x, y) = (x - 0.5, y - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let get = |x: i64, y: i64| {
            if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
                [0.0; 4]
            } else {
                self.px[y as usize * self.w + x as usize]
            }
        };
        let (a, b, c, d) = (get(x0, y0), get(x0 + 1, y0), get(x0, y0 + 1), get(x0 + 1, y0 + 1));
        let mut out = [0.0; 4];
        for i in 0..4 {
            let top = a[i] + (b[i] - a[i]) * fx;
            let bottom = c[i] + (d[i] - c[i]) * fx;
            out[i] = top + (bottom - top) * fy;
        }
        out
    }
}

/// A random path through 36 points within `range` px, as `MoveFRP1`'s
/// `BezierMove` (twelve cubic curves from and back to the place, over
/// `ms`, again and again). The points come from a fixed seed.
struct Jitter {
    points: Vec<(f64, f64)>,
    ms: f64,
}

impl Jitter {
    fn new(seed: u32, range: i32, ms: f64) -> Self {
        let mut s = seed;
        let mut rnd = || {
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            let v = ((s >> 16) % range as u32) as f64;
            if s & 0x8000 != 0 { -v } else { v }
        };
        let mut points = vec![(0.0, 0.0)];
        for _ in 0..35 {
            points.push((rnd(), rnd()));
        }
        points.push((0.0, 0.0));
        Jitter { points, ms }
    }

    fn at(&self, t: f64) -> (f64, f64) {
        let curves = (self.points.len() - 1) / 3;
        let u = (t.rem_euclid(self.ms) / self.ms) * curves as f64;
        let k = (u as usize).min(curves - 1);
        let p = u - k as f64;
        let [a, b, c, d] = [0, 1, 2, 3].map(|i| self.points[k * 3 + i]);
        let q = 1.0 - p;
        let f = |a: f64, b: f64, c: f64, d: f64| q * q * q * a + 3.0 * q * q * p * b + 3.0 * q * p * p * c + p * p * p * d;
        (f(a.0, b.0, c.0, d.0), f(a.1, b.1, c.1, d.1))
    }
}

/// `SetBlur(obj, true, passes, alpha, lag, false)`: copies of the picture
/// drawn over it, where its motion had it a few frames before.
struct Trail {
    passes: u32,
    /// Per mille of the picture's opacity.
    alpha: f64,
    /// ms.
    lag: f64,
    /// The motion's start: the copies hold there until it is passed.
    from: f64,
}

/// Muramasa.exe's frame period (ms): the int at 005dcb0c + 4 * [005dcaf0]
/// (index 4, FUN_0048c5d0), the frames the motions step in.
const FRAME_MS: f64 = 16.0;

impl Trail {
    /// How far back copy `i` (1 the nearest) shows the motion, in ms: the
    /// exe's _ftol(i * lag * speed / period) frames (FUN_00460530, speed
    /// 1.0 from FUN_0047c250), whole frames.
    fn behind(&self, i: u32) -> f64 {
        (i as f64 * self.lag / FRAME_MS).trunc() * FRAME_MS
    }

    /// A copy's opacity (0-1) under the picture's `o` (0-1): the exe's
    /// alpha byte times alpha/1000, rounded (FUN_00456810).
    fn opacity(&self, o: f32) -> f32 {
        let byte = (o as f64 * 255.0).round();
        ((byte * self.alpha / 1000.0).round_ties_even() / 255.0) as f32
    }
}

struct Layer {
    pic: Pic,
    trail: Option<Trail>,
    x: f64,
    y: f64,
    blend: Blend,
    opacity: Track,
    zoom: Track,
    /// Degrees, clockwise.
    turn: Track,
    /// Where it turns and zooms about, in the picture (`SetVertex`; its
    /// centre by default).
    pivot: (f64, f64),
    /// `Move`s, from (x, y).
    dx: Track,
    dy: Track,
    jitter: Option<Jitter>,
}

impl Layer {
    fn new(pic: Pic, x: f64, y: f64, blend: Blend) -> Self {
        let pivot = (pic.w as f64 / 2.0, pic.h as f64 / 2.0);
        Layer { pic, trail: None, x, y, blend, opacity: Track::new(0.0), zoom: Track::new(1000.0), turn: Track::new(0.0), pivot, dx: Track::new(0.0), dy: Track::new(0.0), jitter: None }
    }

    fn draw(&self, t: f64, out: &mut [[f32; 3]]) {
        let o = (self.opacity.at(t) / 1000.0).clamp(0.0, 1.0) as f32;
        if o <= 0.001 {
            return;
        }
        self.paint(t, o, out);
        // The copies after the picture (wait false), the farthest first;
        // one where the picture is now isn't drawn (as the engine's sptrail).
        if let Some(tr) = &self.trail {
            for i in (1..=tr.passes).rev() {
                let s = (t - tr.behind(i)).max(tr.from);
                if s < t {
                    self.paint(s, tr.opacity(o), out);
                }
            }
        }
    }

    /// The picture placed as its motions have it at `t`, at opacity `o`.
    fn paint(&self, t: f64, o: f32, out: &mut [[f32; 3]]) {
        let pic = &self.pic;
        let z = self.zoom.at(t) / 1000.0;
        let (sin, cos) = self.turn.at(t).to_radians().sin_cos();
        let (jx, jy) = self.jitter.as_ref().map_or((0.0, 0.0), |j| j.at(t));
        let (cx, cy) = (self.x + self.pivot.0 + self.dx.at(t) + jx, self.y + self.pivot.1 + self.dy.at(t) + jy);
        for sy in 0..SCREEN_H as usize {
            for sx in 0..SCREEN_W as usize {
                // The picture's point under this pixel: back through the
                // turn and the zoom about the pivot.
                let (dx, dy) = ((sx as f64 + 0.5 - cx) / z, (sy as f64 + 0.5 - cy) / z);
                let (lx, ly) = (dx * cos + dy * sin + self.pivot.0, -dx * sin + dy * cos + self.pivot.1);
                let s = pic.sample(lx, ly);
                if s[3] <= 0.0 {
                    continue;
                }
                let d = &mut out[sy * SCREEN_W as usize + sx];
                let a = s[3] * o;
                for c in 0..3 {
                    d[c] = match self.blend {
                        Blend::Normal => d[c] * (1.0 - a) + s[c] * o,
                        Blend::Add => (d[c] + s[c] * o).min(1.0),
                        Blend::Mul => d[c] * (1.0 - a) + d[c] * s[c] * o,
                        Blend::Overlay => {
                            let (b, f) = (d[c], s[c] / s[3]);
                            let v = if b < 0.5 { 2.0 * b * f } else { 1.0 - 2.0 * (1.0 - b) * (1.0 - f) };
                            b * (1.0 - a) + v * a
                        }
                    };
                }
            }
        }
    }
}

/// Renders `layers` (bottom first) for `ms` into a movie.
/// `layers` from `from` to `ms`.
fn render(a: &mut dyn Assets, name: &str, from: f64, ms: f64, layers: &[Layer]) -> Option<String> {
    // The last picture held a moment: players may stop a few frames short.
    let frames = ((ms - from) * FPS as f64 / 1000.0).ceil() as u32 + 1 + FPS / 5;
    let mut buf = vec![[0.0f32; 3]; (SCREEN_W * SCREEN_H) as usize];
    a.frames_movie(name, FPS, frames, &mut |i, frame: &mut RgbImage| {
        let t = (from + i as f64 * 1000.0 / FPS as f64).min(ms);
        buf.iter_mut().for_each(|p| *p = [0.0; 3]);
        for l in layers {
            l.draw(t, &mut buf);
        }
        for (p, c) in frame.pixels_mut().zip(&buf) {
            p.0 = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    })
}

/// A picture the scripts name, as a layer at (x, y).
fn layer(a: &mut dyn Assets, path: &str, x: f64, y: f64, blend: Blend) -> Option<Layer> {
    Some(Layer::new(Pic::new(&a.rgba(path)?), x, y, blend))
}

fn blurred(a: &mut dyn Assets, path: &str, sigma: f32) -> Option<Pic> {
    Some(Pic::new(&image::imageops::blur(&a.rgba(path)?, sigma)))
}

/// The title's buttons as they rest (`しおりから` at half, as with no quick
/// save), shown from `at`.
fn buttons(a: &mut dyn Assets, look: &TitleLook, at: f64) -> Option<Vec<Layer>> {
    let mut out = Vec::new();
    for (i, &(name, x, y)) in look.buttons.iter().enumerate() {
        let opacity = if i == 0 { 500.0 } else { 1000.0 };
        let mut l = layer(a, &format!("{}/{name}-001.png", look.dir), x, y, Blend::Normal)?;
        l.opacity = Track::new(0.0).to(at, 0.0, opacity, Ease::Linear);
        out.push(l);
    }
    Some(out)
}

/// `SetBlur(…, true, 2, 500, 50, false)`, from `from` (its zoom's start).
fn trail(from: f64) -> Option<Trail> {
    Some(Trail { passes: 2, alpha: 500.0, lag: 50.0, from })
}

/// `SetShade(MEDIUM)`: the exe's 4 passes of a 3×3 mean (FUN_00465b10)
/// spread as a Gaussian of the same variance (2/3 px² a pass, as
/// builtin.rs's `shade_blur`).
fn shade_sigma(passes: f32) -> f32 {
    (passes * 2.0 / 3.0).sqrt()
}

/// The first title after the game starts (`TitleFade`, `!$Logo`): 善悪相殺
/// drifts in, `picture` (the title picks it by the routes cleared) zooms
/// out of the dark, two bursts of blood and red, a white flash, the title.
/// Ends at 11.2 s, as the white has faded. `name`: the movie's.
pub fn at_launch(a: &mut dyn Assets, look: &TitleLook, picture: &str, name: &str) -> Option<String> {
    let fuji = format!("{}/偽富士背景画像.png", look.dir);
    let logo = format!("{}/ロゴ画像.png", look.dir);
    let muramasa = picture;
    // src: 0_boot.nss TitleFade (!$Logo): every time, zoom and opacity below; MoveFRP1/2 (10000, 15, 15)
    let end = 11200.0;
    let mut layers = Vec::new();

    let mut bg = layer(a, &fuji, 0.0, 0.0, Blend::Normal)?;
    bg.zoom = Track::new(2000.0).to(4000.0, 7000.0, 1000.0, AXL1).to(10200.0, 0.0, 1000.0, Ease::Linear);
    bg.opacity = Track::new(0.0)
        .to(7000.0, 500.0, 300.0, Ease::Linear)
        .to(8000.0, 500.0, 100.0, Ease::Linear)
        .to(8500.0, 500.0, 600.0, Ease::Linear)
        .to(10200.0, 0.0, 1000.0, Ease::Linear);
    layers.push(bg);

    let mut copyright = layer(a, &format!("{}/コピーライト.png", look.dir), 50.0, 540.0, Blend::Normal)?;
    copyright.opacity = Track::new(0.0).to(10200.0, 0.0, 1000.0, Ease::Linear);
    layers.push(copyright);

    // ef046 over the title, 2x (the MiddleWave it comes in with is drawn
    // as a fade).
    let mut fire = layer(a, "cg/ef/ef046_炎a.jpg", 0.0, 0.0, Blend::Overlay)?;
    fire.zoom = Track::new(2000.0);
    fire.opacity = Track::new(0.0).to(7000.0, 3000.0, 700.0, Ease::Linear).to(10200.0, 0.0, 300.0, Ease::Linear);
    layers.push(fire);

    let mut m = layer(a, muramasa, 0.0, 0.0, Blend::Add)?;
    m.zoom = Track::new(1000.0).to(4000.0, 7000.0, 1500.0, AXL1);
    m.opacity = Track::new(0.0).to(5000.0, 3000.0, 800.0, Ease::Linear).to(10200.0, 0.0, 0.0, Ease::Linear);
    layers.push(m);

    // SetShade(MEDIUM): drawn soft.
    let mut m0 = Layer::new(blurred(a, muramasa, shade_sigma(4.0))?, 0.0, 0.0, Blend::Add);
    m0.zoom = Track::new(1000.0).to(4000.0, 7000.0, 1500.0, AXL1);
    m0.opacity = Track::new(0.0).to(4000.0, 3000.0, 800.0, Ease::Linear).to(5000.0, 3000.0, 0.0, Ease::Linear);
    layers.push(m0);

    let mut m2 = layer(a, muramasa, 0.0, 0.0, Blend::Add)?;
    m2.zoom = Track::new(1100.0).to(4000.0, 7000.0, 1600.0, AXL1);
    m2.opacity = Track::new(0.0)
        .to(7000.0, 200.0, 500.0, Ease::Linear)
        .to(8000.0, 500.0, 0.0, Ease::Linear)
        .to(8500.0, 200.0, 500.0, Ease::Linear)
        .to(9500.0, 500.0, 0.0, Ease::Linear);
    m2.jitter = Some(Jitter::new(1, 15, 10000.0));
    layers.push(m2);

    let mut bg2 = layer(a, &fuji, 0.0, 0.0, Blend::Add)?;
    bg2.trail = trail(10200.0);
    bg2.zoom = Track::new(2000.0).to(4000.0, 7000.0, 1500.0, AXL1).to(10200.0, 0.0, 1000.0, Ease::Linear).to(10200.0, 1000.0, 1200.0, Ease::Linear);
    bg2.opacity = Track::new(0.0)
        .to(7000.0, 500.0, 500.0, Ease::Linear)
        .to(8000.0, 500.0, 0.0, Ease::Linear)
        .to(8500.0, 500.0, 800.0, Ease::Linear)
        .to(9500.0, 500.0, 0.0, Ease::Linear)
        .to(10200.0, 0.0, 1000.0, Ease::Linear)
        .to(10200.0, 1000.0, 0.0, Ease::Linear);
    bg2.jitter = Some(Jitter::new(2, 15, 10000.0));
    layers.push(bg2);

    for (file, at) in [("ef004_汎用血雫", 7000.0), ("ef005_汎用血雫", 8500.0)] {
        let mut drops = layer(a, &format!("cg/ef/{file}.jpg"), 0.0, 0.0, Blend::Add)?;
        drops.opacity = Track::new(0.0).to(at, 300.0, 500.0, Ease::Linear).to(at + 1000.0, 700.0, 0.0, Ease::Linear);
        layers.push(drops);
    }

    let mut logo1 = layer(a, &logo, look.logo.0, look.logo.1, Blend::Normal)?;
    logo1.zoom = Track::new(800.0).to(10200.0, 0.0, 1000.0, Ease::Linear);
    logo1.opacity = Track::new(0.0).to(10200.0, 0.0, 1000.0, Ease::Linear);
    layers.push(logo1);

    let mut logo2 = layer(a, &logo, look.logo.0, look.logo.1, Blend::Normal)?;
    logo2.trail = trail(10200.0);
    logo2.zoom = Track::new(800.0).to(10200.0, 0.0, 1000.0, Ease::Linear).to(10200.0, 1000.0, 1200.0, Ease::Linear);
    logo2.opacity = Track::new(0.0).to(10200.0, 0.0, 1000.0, Ease::Linear).to(10200.0, 1000.0, 0.0, Ease::Linear);
    layers.push(logo2);

    // The sprays of blood swing in about a corner (SetVertex), from a
    // quarter turn away, doubling in size, multiplied over the scene.
    for (file, corner, turn, at) in [("ef006_汎用血しぶき_ex", 1024.0, -90.0, 7000.0), ("ef007_汎用血しぶき_ex", 0.0, 90.0, 8500.0)] {
        let mut spray = layer(a, &format!("cg/ef/resize/{file}.png"), 0.0, 0.0, Blend::Mul)?;
        spray.pivot = (corner, 0.0);
        spray.turn = Track::new(turn).to(at, 500.0, 0.0, DXL3);
        spray.zoom = Track::new(1000.0).to(at, 2000.0, 2000.0, DXL3);
        spray.opacity = Track::new(0.0).to(at, 100.0, 500.0, Ease::Linear).to(at + 500.0, 1000.0, 0.0, Ease::Linear);
        layers.push(spray);
    }

    // 善悪相殺 drifts 100 px left as it comes and goes.
    let mut words = layer(a, "cg/sys/telop/tp_魔王編挿入01.png", 492.0, 157.0, Blend::Normal)?;
    words.opacity = Track::new(0.0).to(2000.0, 1000.0, 1000.0, Ease::Linear).to(4000.0, 2000.0, 0.0, Ease::Linear);
    words.dx = Track::new(0.0).to(2000.0, 4000.0, -100.0, Ease::Linear);
    layers.push(words);

    // Priority 150: the white (added) and the red.
    let mut white = Layer::new(Pic::colour([255, 255, 255], SCREEN_W as usize, SCREEN_H as usize), 0.0, 0.0, Blend::Add);
    white.opacity = Track::new(0.0).to(10000.0, 200.0, 1000.0, AXL1).to(10200.0, 1000.0, 0.0, Ease::Linear);
    layers.push(white);
    let mut red = Layer::new(Pic::colour([255, 0, 0], SCREEN_W as usize, SCREEN_H as usize), 0.0, 0.0, Blend::Add);
    red.opacity = Track::new(0.0)
        .to(7000.0, 1000.0, 200.0, Ease::Linear)
        .to(8000.0, 1500.0, 0.0, Ease::Linear)
        .to(8500.0, 1000.0, 200.0, Ease::Linear)
        .to(9500.0, 1500.0, 0.0, Ease::Linear);
    layers.push(red);

    layers.extend(buttons(a, look, 10200.0)?);
    render(a, name, 0.0, end, &layers)
}

/// Coming back to the title (`TitleFade`, `$Logo`): the logo grows out of
/// the dark over 2 s, the background drops in from above, a white flash,
/// the title with the background and logo trailing copies as they spread
/// away. Ends at 3 s, as the white has faded.
pub fn on_return(a: &mut dyn Assets, look: &TitleLook) -> Option<String> {
    let layers = return_layers(a, look)?;
    render(a, "title_return2", 0.0, 3000.0, &layers)
}

/// Back from the extras (`TitleFade`, `$エキストラタイトル`): the white at
/// once, then [`on_return`]'s last second.
pub fn after_extras(a: &mut dyn Assets, look: &TitleLook) -> Option<String> {
    let layers = return_layers(a, look)?;
    render(a, "title_extras2", 2000.0, 3000.0, &layers)
}

fn return_layers(a: &mut dyn Assets, look: &TitleLook) -> Option<Vec<Layer>> {
    // src: 0_boot.nss TitleFade ($Logo, $エキストラタイトル): every time, zoom and opacity below
    let fuji = format!("{}/偽富士背景画像.png", look.dir);
    let logo = format!("{}/ロゴ画像.png", look.dir);
    let mut layers = Vec::new();

    let mut bg = layer(a, &fuji, 0.0, 0.0, Blend::Normal)?;
    bg.dy = Track::new(-576.0).to(1600.0, 400.0, 0.0, Ease::Linear);
    bg.opacity = Track::new(0.0).to(1600.0, 400.0, 1000.0, Ease::Linear);
    layers.push(bg);

    let mut copyright = layer(a, &format!("{}/コピーライト.png", look.dir), 50.0, 540.0, Blend::Normal)?;
    copyright.opacity = Track::new(0.0).to(2000.0, 0.0, 1000.0, Ease::Linear);
    layers.push(copyright);

    let mut bg2 = layer(a, &fuji, 0.0, 0.0, Blend::Add)?;
    bg2.trail = trail(2000.0);
    bg2.zoom = Track::new(1000.0).to(2000.0, 1000.0, 1200.0, Ease::Linear);
    bg2.opacity = Track::new(0.0).to(2000.0, 0.0, 1000.0, Ease::Linear).to(2000.0, 1000.0, 0.0, Ease::Linear);
    layers.push(bg2);

    let mut logo1 = layer(a, &logo, look.logo.0, look.logo.1, Blend::Normal)?;
    logo1.zoom = Track::new(800.0).to(0.0, 2000.0, 1000.0, Ease::Linear);
    logo1.opacity = Track::new(0.0).to(0.0, 2000.0, 1000.0, Ease::Linear);
    layers.push(logo1);

    let mut logo2 = layer(a, &logo, look.logo.0, look.logo.1, Blend::Normal)?;
    logo2.trail = trail(2000.0);
    logo2.zoom = Track::new(1000.0).to(2000.0, 1000.0, 1200.0, Ease::Linear);
    logo2.opacity = Track::new(0.0).to(2000.0, 0.0, 1000.0, Ease::Linear).to(2000.0, 1000.0, 0.0, Ease::Linear);
    layers.push(logo2);

    let mut white = Layer::new(Pic::colour([255, 255, 255], SCREEN_W as usize, SCREEN_H as usize), 0.0, 0.0, Blend::Add);
    white.opacity = Track::new(0.0).to(1800.0, 200.0, 1000.0, AXL1).to(2000.0, 1000.0, 0.0, Ease::Linear);
    layers.push(white);

    layers.extend(buttons(a, look, 2000.0)?);
    Some(layers)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// research/nitroplus/trail_exe.py: FUN_00460530 run in Muramasa.exe;
    /// (lag, passes, the motion's frame, the frame each copy shows, -1
    /// held where it was).
    const EXE_TRAIL: [(u32, u32, u32, &[i32]); 12] = [(50, 2, 0, &[-1, -1]), (50, 2, 2, &[-1, -1]), (50, 2, 3, &[0, -1]), (50, 2, 5, &[2, -1]), (50, 2, 6, &[3, 0]), (50, 2, 30, &[27, 24]), (16, 4, 20, &[19, 18, 17, 16]), (17, 4, 20, &[19, 18, 17, 16]), (40, 3, 20, &[18, 15, 13]), (100, 8, 60, &[54, 48, 42, 35, 29, 23, 17, 10]), (1, 2, 10, &[10, 10]), (33, 5, 40, &[38, 36, 34, 32, 30])];

    #[test]
    fn trail_copies_as_the_exe() {
        for (lag, passes, frame, want) in EXE_TRAIL {
            let tr = Trail { passes, alpha: 500.0, lag: lag as f64, from: 0.0 };
            let got: Vec<i32> = (1..=passes)
                .map(|i| {
                    let back = (tr.behind(i) / FRAME_MS) as i32;
                    if back <= frame as i32 { frame as i32 - back } else { -1 }
                })
                .collect();
            assert_eq!(got, want, "lag {lag}, frame {frame}");
        }
        // FUN_00456810: byte 255 at 500 = 127.5, rounded to even.
        let tr = Trail { passes: 2, alpha: 500.0, lag: 50.0, from: 0.0 };
        assert_eq!(tr.opacity(1.0), 128.0 / 255.0);
    }
}
