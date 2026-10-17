//! dja050's 400-vs-3 counter (`function_count.nss`): three digits at the
//! top of the screen counting the men shot, drawn as [`Kind::Strip`]s.
//!
//! In the original the digits are two copies of `count.png` (9 at the
//! top, 0 at the bottom, 52 px each) per place, in a 194×52 window at
//! 296,10 that crops them, at x 346, 378 and 410. `CounterPlay(next,
//! time, shake, attack, wait)` runs `CounterShow` beside the script: it
//! counts up one at a time, each step rolling the places that change
//! down by a digit over `time` ms (at once for 0), and shakes the digits
//! for `attack` ms (`shake` 1 at each step, 10 × 30 px; 2 once, 50 px
//! across; 3 once, 100 px up and down; 4 the same, easing in). When 380
//! men in all have been counted the digits fade out over 2 s, and when
//! the count reaches 400 the `CountFadeOut` process (created before)
//! starts. `CountFadeIn` (a process the chapter creates) brings in the
//! counter: three blurred zeros fly in from the sides as they shrink from
//! five times their size, a flash over each, the digits fading in.
//!
//! Here each `CounterPlay` is drawn as one strip of frames on the
//! counter's object, shown once while the script goes on and stopping on
//! the new count; the fade-in and fade-out are strips of their own over
//! the top of the screen, drawn at half size and zoomed.

use conv_core::ir::Op;

use super::super::{Compiler, Kind, StripObject, Value};

const DIR: &str = "cg/sys/count";
/// The counter's window. src: function_count.nss (window, places, 52 px digits,
/// fly-ins, shakes, 380 and 2000 ms).
const WINDOW: (i64, i64, u32, u32) = (296, 10, 194, 52);
/// The places' x in the window: hundreds, tens, ones.
const PLACES: [i64; 3] = [50, 82, 114];
const DIGIT_H: i64 = 52;
/// A frame of the counter's strips.
const FRAME_MS: u32 = 33; // ours
/// The fade-in's and fade-out's: x 80-720, y 0-168, drawn at half size.
const FLY: (i64, i64, u32, u32) = (80, 0, 640, 168); // ours
const FLY_FRAME_MS: u32 = 50; // ours
/// The count at which the digits fade out, over [`GONE_MS`].
const GONE_AT: i64 = 380;
const GONE_MS: u32 = 2000;
const COUNTER: &str = "カウンタ窓";
/// Bumped when the drawing changes: strips already written are kept.
const VERSION: u32 = 1;

/// NSS tempos: `Axl`n eases in, `Dxl`n out (`research/nitroplus/builtins.md`).
fn axl(n: i32, p: f64) -> f64 {
    p.clamp(0.0, 1.0).powi(n + 1)
}

fn dxl(n: i32, p: f64) -> f64 {
    1.0 - (1.0 - p.clamp(0.0, 1.0)).powi(n + 1)
}

/// `src` drawn on `dst` with its centre at (`cx`, `cy`), `zoom` times its
/// size, `opacity` (0-1) of its own.
fn draw(dst: &mut image::RgbaImage, src: &image::RgbaImage, cx: f64, cy: f64, zoom: f64, opacity: f64) {
    if opacity <= 0.0 || zoom <= 0.0 {
        return;
    }
    let (w, h) = (((src.width() as f64 * zoom).round() as u32).max(1), ((src.height() as f64 * zoom).round() as u32).max(1));
    let scaled;
    let src = if (w, h) == src.dimensions() {
        src
    } else {
        scaled = image::imageops::resize(src, w, h, image::imageops::FilterType::Triangle);
        &scaled
    };
    blit(dst, src, (0, 0, w, h), (cx - w as f64 / 2.0).round() as i64, (cy - h as f64 / 2.0).round() as i64, opacity);
}

/// The `rect` (x, y, w, h) of `src` drawn over `dst` at (`x`, `y`).
fn blit(dst: &mut image::RgbaImage, src: &image::RgbaImage, (sx, sy, w, h): (u32, u32, u32, u32), x: i64, y: i64, opacity: f64) {
    for j in 0..h {
        let dy = y + j as i64;
        if dy < 0 || dy >= dst.height() as i64 {
            continue;
        }
        for i in 0..w {
            let dx = x + i as i64;
            if dx < 0 || dx >= dst.width() as i64 {
                continue;
            }
            let s = src.get_pixel(sx + i, sy + j).0;
            let a = s[3] as f64 / 255.0 * opacity;
            if a <= 0.0 {
                continue;
            }
            let d = dst.get_pixel_mut(dx as u32, dy as u32);
            let da = d.0[3] as f64 / 255.0;
            let oa = a + da * (1.0 - a);
            for (c, s) in d.0.iter_mut().zip(s).take(3) {
                *c = ((s as f64 * a + *c as f64 * da * (1.0 - a)) / oa).round() as u8;
            }
            d.0[3] = (oa * 255.0).round() as u8;
        }
    }
}

/// `img` blurred, with room around it for the blur.
fn blurred(img: &image::RgbaImage) -> image::RgbaImage {
    const PAD: u32 = 8;
    let mut padded = image::RgbaImage::new(img.width() + 2 * PAD, img.height() + 2 * PAD);
    image::imageops::replace(&mut padded, img, PAD as i64, PAD as i64);
    image::imageops::blur(&padded, 2.0)
}

/// A number in [0, 1) from `n` (the shakes' jitter, the same in each build).
fn noise(n: u64) -> f64 {
    let mut x = n.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x2545_F491_4F6C_DD1D;
    x ^= x >> 29;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 32;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// One `CounterPlay`: the count from `from`, `steps` steps of `time` ms.
struct Play {
    from: i64,
    steps: i64,
    time: u32,
    shake: i64,
    attack: u32,
    /// When the digits start fading out (ms).
    gone_from: Option<u32>,
}

impl Play {
    fn ms(&self) -> u32 {
        let roll = self.steps as u32 * self.time;
        let shake = match self.shake {
            1 if self.steps > 0 => (self.steps as u32 - 1) * self.time + self.attack,
            2..=4 if self.steps > 0 => self.attack,
            _ => 0,
        };
        roll.max(shake).max(self.gone_from.map_or(0, |t| t + GONE_MS))
    }

    /// The shake's offset at `t`.
    fn shake_at(&self, t: u32, frame: u32) -> (f64, f64) {
        type Shake = (u32, (f64, f64), fn(f64) -> f64);
        let (start, amp, curve): Shake = match self.shake {
            1 if self.time > 0 => ((t / self.time).min(self.steps.max(1) as u32 - 1) * self.time, (10.0, 30.0), |p| 1.0 - dxl(1, p)),
            1 => (0, (10.0, 30.0), |p| 1.0 - dxl(1, p)),
            2 => (0, (50.0, 0.0), |p| 1.0 - dxl(1, p)),
            3 => (0, (0.0, 100.0), |p| 1.0 - dxl(1, p)),
            4 => (0, (0.0, 100.0), |p| 1.0 - axl(1, p)),
            _ => return (0.0, 0.0),
        };
        if self.steps == 0 || self.attack == 0 || t < start || t - start >= self.attack {
            return (0.0, 0.0);
        }
        let k = curve((t - start) as f64 / self.attack as f64);
        let r = |n: u64| noise(frame as u64 * 2 + n) * 2.0 - 1.0;
        (amp.0 * k * r(0), amp.1 * k * r(1))
    }

    /// Frame `frame` (at `t` ms) of the counter's window.
    fn draw(&self, out: &mut image::RgbaImage, digits: &image::RgbaImage, t: u32, frame: u32) {
        let (k, f) = match self.time {
            0 => (self.steps, 0.0),
            time => match (t / time) as i64 {
                k if k >= self.steps => (self.steps, 0.0),
                k => (k, (t % time) as f64 / time as f64),
            },
        };
        let (old, new) = (self.from + k, self.from + k + 1);
        let opacity = self.gone_from.map_or(1.0, |g| 1.0 - (t.saturating_sub(g) as f64 / GONE_MS as f64).min(1.0));
        let (dx, dy) = self.shake_at(t, frame);
        for (i, x) in PLACES.iter().enumerate() {
            let p = 10i64.pow(2 - i as u32);
            let (d0, d1) = ((old / p).rem_euclid(10), (new / p).rem_euclid(10));
            let x = (*x as f64 + dx).round() as i64;
            let mut digit = |d: i64, y: f64| {
                let cell = (0, ((9 - d) * DIGIT_H) as u32, digits.width(), DIGIT_H as u32);
                blit(out, digits, cell, x, (y + dy).round() as i64, opacity);
            };
            if f == 0.0 || d0 == d1 {
                digit(d0, 0.0);
            } else {
                digit(d0, f * DIGIT_H as f64);
                digit(d1, (f - 1.0) * DIGIT_H as f64);
            }
        }
    }
}

impl Compiler<'_, '_> {
    fn count_var(&self, name: &str) -> i64 {
        self.vars.get(name).map_or(0, Value::int)
    }

    /// `CountInit()`: the counter at 000, not shown yet.
    pub(in super::super) fn count_init(&mut self) {
        self.vars.insert("$CountBase".into(), Value::Int(0));
        self.vars.insert("$CountTotal".into(), Value::Int(0));
        self.vars.remove("$CountFadeOut");
        let play = Play { from: 0, steps: 0, time: 0, shake: 0, attack: 0, gone_from: None };
        self.counter_strip(&play, None);
        if let Some(o) = self.objects.get_mut(COUNTER) {
            o.opacity = 0;
        }
    }

    /// The counter's object showing `play` (or the digits fading in over
    /// `fade_in` ms).
    fn counter_strip(&mut self, play: &Play, fade_in: Option<u32>) {
        let ms = fade_in.unwrap_or_else(|| play.ms());
        let frames = ms / FRAME_MS + 1;
        let name = match fade_in {
            Some(ms) => format!("count_in_{ms}_v{VERSION}"),
            None => {
                let gone = play.gone_from.map_or(String::new(), |t| format!("_gone{t}"));
                format!("count_{}_{}_{}_{}_{}{gone}_v{VERSION}", play.from, play.steps, play.time, play.shake, play.attack)
            }
        };
        let digits = self.assets.rgba(&format!("{DIR}/count.png"));
        let (_, _, w, h) = WINDOW;
        let image = self.assets.frames_strip(&name, w, h, frames, &mut |i, out| {
            let Some(digits) = &digits else { return };
            let t = (i * FRAME_MS).min(ms);
            play.draw(out, digits, t, i);
            if fade_in.is_some() {
                let a = t as f64 / ms as f64;
                out.pixels_mut().for_each(|p| p.0[3] = (p.0[3] as f64 * a).round() as u8);
            }
        });
        let last = self.assets.frames_strip(&format!("{name}_end"), w, h, 1, &mut |_, out| {
            if let Some(digits) = &digits {
                play.draw(out, digits, ms, frames);
            }
        });
        let (Some(image), Some(last)) = (image, last) else { return };
        let strip = StripObject { image, cells: frames, frame_ms: FRAME_MS, w: w as i64, h: h as i64, zoom: (100, 100), last, looped: false };
        self.vars.insert("$CountOp".into(), Value::Int(self.ops.len() as i64));
        self.vars.insert("$CountMs".into(), Value::Int(ms as i64));
        match self.objects.get_mut(COUNTER) {
            Some(o) => o.kind = Kind::Strip(strip),
            None => {
                let o = self.create(COUNTER.into(), Kind::Strip(strip), 10200, w as i64, h as i64);
                (o.x, o.y) = (WINDOW.0, WINDOW.1);
            }
        }
    }

    /// `WaitAction("カウント１")`: waits for the counter's last change to
    /// end, counting the waits and fades since it started; a page or a
    /// choice in between ends it.
    pub(in super::super) fn count_wait(&mut self) {
        let (at, ms) = (self.count_var("$CountOp") as usize, self.count_var("$CountMs") as u32);
        let mut done = 0u32;
        for op in self.ops.get(at..).unwrap_or_default() {
            done += match op {
                Op::Wait { ms } => *ms,
                Op::BeginTransition { fade_ms, .. } => *fade_ms,
                Op::Text { .. } | Op::ClickWait { .. } | Op::Choice { .. } | Op::RuntimeChoice { .. } | Op::HoldMovie { .. } | Op::Movie { .. } => ms,
                _ => 0,
            };
        }
        if done < ms {
            self.sync(0);
            self.ops.push(Op::Wait { ms: ms - done });
        }
        self.vars.insert("$CountMs".into(), Value::Int(0));
    }

    /// `CounterPlay(next, time, shake, attack, wait)`, once the last one
    /// has ended.
    pub(in super::super) fn counter_play(&mut self, args: &[Value]) {
        self.count_wait();
        let arg = |i: usize| args.get(i).map_or(0, Value::int);
        let (next, time, shake, attack) = (arg(0), arg(1).max(0) as u32, arg(2), arg(3).max(0) as u32);
        let from = self.count_var("$CountBase");
        let total = self.count_var("$CountTotal");
        let steps = (next - from).max(0);
        self.vars.insert("$CountBase".into(), Value::Int(next));
        self.vars.insert("$CountTotal".into(), Value::Int(total + steps));
        let gone_from = (total < GONE_AT && total + steps >= GONE_AT).then(|| (GONE_AT - total) as u32 * time);
        let play = Play { from, steps, time, shake, attack, gone_from };
        if total >= GONE_AT {
            if let Some(o) = self.objects.get_mut(COUNTER) {
                o.opacity = 0;
            }
        } else {
            self.counter_strip(&play, None);
        }
        if next == 400 && self.vars.remove("$CountFadeOut").is_some() {
            self.count_fly(false, steps as u32 * time);
        }
    }

    /// `CreateProcess(…, "CountFadeIn")`: the digits fade in over 3 s
    /// while the zeros fly in. `CountFadeOut` waits for the count to
    /// reach 400.
    pub(in super::super) fn count_process(&mut self, function: &str) -> bool {
        match function {
            "CountFadeIn" => {
                let play = Play { from: self.count_var("$CountBase"), steps: 0, time: 0, shake: 0, attack: 0, gone_from: None };
                self.counter_strip(&play, Some(3000));
                if let Some(o) = self.objects.get_mut(COUNTER) {
                    o.opacity = 1000;
                }
                self.count_fly(true, 0);
                true
            }
            "CountFadeOut" => {
                self.vars.insert("$CountFadeOut".into(), Value::Int(1));
                true
            }
            _ => false,
        }
    }

    /// The fade-in's (`fade_in`) or fade-out's flying digits, starting
    /// after `delay` ms.
    fn count_fly(&mut self, fade_in: bool, delay: u32) {
        let (x0, y0, w, h) = FLY;
        let ms = delay + if fade_in { 3600 } else { 4100 };
        let frames = ms / FLY_FRAME_MS + 1;
        let load = |c: &mut Self, f: &str| c.assets.rgba(&format!("{DIR}/{f}.png"));
        let (zero, glow0) = (load(self, "00"), load(self, "000"));
        let (four, glow4) = (load(self, "44"), load(self, "444"));
        let (zero, four) = (zero.as_ref().map(blurred), four.as_ref().map(blurred));
        let name = format!("count_{}_{delay}_v{VERSION}", if fade_in { "fly_in" } else { "fly_out" });
        let image = self.assets.frames_strip(&name, w / 2, h / 2, frames, &mut |i, out| {
            let (Some(zero), Some(glow0), Some(four), Some(glow4)) = (&zero, &glow0, &four, &glow4) else { return };
            let Some(t) = (i * FLY_FRAME_MS).checked_sub(delay).map(f64::from) else { return };
            // Each texture's centre on the screen, its zoom, its opacity.
            let mut put = |img: &image::RgbaImage, x: f64, zoom: f64, opacity: f64| {
                let (cx, cy) = ((x + 16.0 - x0 as f64) / 2.0, (10.0 + 26.0 - y0 as f64) / 2.0);
                draw(out, img, cx, cy, zoom / 2.0, opacity.clamp(0.0, 1.0));
            };
            if fade_in {
                // `カウント初弾`1-3: from 610, 378 and 146 to 410, 378, 346,
                // five times as large to their size (Dxl2, 3 s), fading in
                // then out (1.5 s each); then a flash over the places.
                let p = dxl(2, t / 3000.0);
                let zoom = 5.0 - 4.0 * p;
                let opacity = if t < 1500.0 { t / 1500.0 } else { 1.0 - (t - 1500.0) / 1500.0 };
                for (x, dx) in [(610.0, -200.0), (378.0, 0.0), (146.0, 200.0)] {
                    put(zero, x + dx * p, zoom, opacity);
                }
                let flash = if t < 3000.0 { 0.0 } else if t < 3100.0 { (t - 3000.0) / 100.0 } else { 1.0 - (t - 3100.0) / 500.0 };
                for x in [410.0, 378.0, 346.0] {
                    put(glow0, x, 1.0, flash);
                }
            } else {
                // A flash over 400 (100 ms), `カウント終弾` (4, 0, 0) fading
                // in under it (500 ms), the flash fading out (500 ms); then
                // the 4 and the last 0 fly apart, all three growing to five
                // times their size (Axl2) as they fade out (3 s).
                let flash = if t < 100.0 { t / 100.0 } else if t < 600.0 { 1.0 } else { 1.0 - (t - 600.0) / 500.0 };
                let shown = if t < 100.0 { 0.0 } else if t < 600.0 { (t - 100.0) / 500.0 } else if t < 1100.0 { 1.0 } else { 1.0 - (t - 1100.0) / 3000.0 };
                let p = axl(2, (t - 1100.0) / 3000.0);
                let zoom = 1.0 + 4.0 * p;
                for (img, glow, x, dx) in [(zero, glow0, 410.0, 200.0), (zero, glow0, 378.0, 0.0), (four, glow4, 346.0, -200.0)] {
                    put(img, x + dx * p, zoom, shown);
                    put(glow, x, 1.0, flash);
                }
            }
        });
        let Some(image) = image else { return };
        let last = self.assets.transparent();
        let strip = StripObject { image, cells: frames, frame_ms: FLY_FRAME_MS, w: (w / 2) as i64, h: (h / 2) as i64, zoom: (200, 200), last, looped: false };
        let name = if fade_in { "カウント初弾" } else { "カウント終弾" };
        let o = self.create(name.into(), Kind::Strip(strip), 10200, (w / 2) as i64, (h / 2) as i64);
        (o.x, o.y) = (x0 + w as i64 / 4, y0 + h as i64 / 4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_length() {
        // 379 → 395 at 80 ms a step: the fade starts after the first step.
        let play = Play { from: 379, steps: 16, time: 80, shake: 0, attack: 0, gone_from: Some(80) };
        assert_eq!(play.ms(), 2080);
        // A shake at each step outlasts the roll.
        let play = Play { from: 0, steps: 3, time: 100, shake: 1, attack: 500, gone_from: None };
        assert_eq!(play.ms(), 700);
        // At once, shaken once.
        let play = Play { from: 0, steps: 7, time: 0, shake: 3, attack: 1000, gone_from: None };
        assert_eq!(play.ms(), 1000);
        assert_eq!(play.shake_at(1000, 30), (0.0, 0.0));
    }
}
