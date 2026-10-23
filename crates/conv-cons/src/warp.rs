//! Warp maps for CONscripter's `cons_warp.dll` transitions: the cells
//! that redo the slide, zoom, flip, cube, melt and wave plugins (the
//! format: the engine's COMMANDS.md, "Transitions"; a port of the engine's
//! `tools/cons_warp.py make`, which writes the same pixels).
//!
//! Each map is a PNG in [`DIR`], written lossless and never re-encoded
//! (the CLI keeps it out of `--webp`).

use std::f64::consts::PI;

/// The output folder of the maps (packed into the archive).
pub const DIR: &str = "warp";

/// Screen pixels between two points of the grid.
const GRID: u32 = 8;
/// A block's flags: bilinear sampling; a position outside a picture reads black.
const SMOOTH: u8 = 128;
const BLACK: u8 = 64;

/// A transition drawn by a warp map, with the parameters of the plugin it
/// replaced (retired in CONscripter #72).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// `cons_slide.dll/layer,dx,dy`
    Slide { layer: u8, dx: i8, dy: i8 },
    /// `cons_zoom.dll/mode,scale,curve`
    Zoom { mode: u8, scale: u32, curve: u32 },
    /// `cons_flip.dll/axis,layer`
    Flip { axis: u8, layer: u8 },
    /// `cons_cube.dll/dx,dy`
    Cube { dx: i8, dy: i8 },
    /// Xuse's Trans_MeltDown: `speed` the byte that sets the fall's
    /// acceleration, `seed` MSVC rand()'s seed for the melting edge
    Melt { speed: u8, seed: u32 },
    /// `cons_wave.dll/direction,amplitude,waves`
    Wave { direction: u8, amplitude: u32, waves: u32 },
}

/// One grid point of one cell: where it reads the old and the new picture
/// (pixel indices), the mask (a level, or with `sharp` a signed distance
/// in pixels, positive where the new picture shows) and the block flags.
struct Point {
    old: (f64, f64),
    new: (f64, f64),
    mask: f64,
    sharp: bool,
    flags: u8,
}

impl Kind {
    /// The map's file name in [`DIR`].
    pub fn file_name(&self) -> String {
        let name = match *self {
            Kind::Slide { layer, dx, dy } => format!("slide_{layer}_{dx}_{dy}"),
            Kind::Zoom { mode, scale, curve } => format!("zoom_{mode}_{scale}_{curve}"),
            Kind::Flip { axis, layer } => format!("flip_{axis}_{layer}"),
            Kind::Cube { dx, dy } => format!("cube_{dx}_{dy}"),
            Kind::Melt { speed, seed } => format!("melt_{speed}_{seed}"),
            Kind::Wave { direction, amplitude, waves } => format!("wave_{direction}_{amplitude}_{waves}"),
        };
        format!("{name}.png")
    }

    /// Cells in time: enough that straight steps between them stay within
    /// a fraction of a pixel of the plugin.
    pub fn cells(&self) -> u32 {
        match *self {
            // a diagonal slide in or out has a corner, whose distance isn't straight in time
            Kind::Slide { layer, dx, dy } if layer != 2 && dx != 0 && dy != 0 => 33,
            Kind::Slide { .. } => 2,
            Kind::Zoom { .. } | Kind::Wave { .. } => 65,
            Kind::Flip { .. } | Kind::Cube { .. } | Kind::Melt { .. } => 33,
        }
    }

    /// A grid point of the cell at progress `p`; `column` the melt's shift
    /// of each row 0..=h at this point's x.
    #[allow(clippy::too_many_arguments)]
    fn point(&self, p: f64, x: f64, y: f64, w: f64, h: f64, column: &[i32]) -> Point {
        let ident = (x, y);
        let pt = |old, new, mask, sharp, flags| Point { old, new, mask, sharp, flags };
        match *self {
            Kind::Slide { layer, dx, dy } => {
                let (dx, dy) = (f64::from(dx), f64::from(dy));
                let (ox, oy) = (w * p, h * p);
                if layer == 0 {
                    let (rx, ry) = (-dx * (w - ox), -dy * (h - oy));
                    return pt(ident, (x - rx, y - ry), edge_dist(x, y, rx, ry, w, h, -dx, -dy), true, 0);
                }
                let (rx, ry) = (dx * ox, dy * oy);
                let old = (x - rx, y - ry);
                if layer == 1 {
                    return pt(old, ident, -edge_dist(x, y, rx, ry, w, h, dx, dy), true, 0);
                }
                let new = (x - rx + dx * w, y - ry + dy * h);
                if dx == 0.0 || dy == 0.0 {
                    return pt(old, new, -edge_dist(x, y, rx, ry, w, h, dx, dy), true, 0);
                }
                // diagonal: split by x alone, each picture's part off the
                // other's rows reads black (the plugin's black corners)
                pt(old, new, -edge_dist(x, y, rx, ry, w, h, dx, 0.0), true, BLACK)
            }
            Kind::Zoom { mode, scale, curve } => {
                let range = f64::from(scale) / 100.0 - 1.0;
                let curve = if curve > 0 { f64::from(curve) } else { 1.0 };
                let z = |s: f64| {
                    let k = 1.0 + range * s;
                    ((x + 0.5 - w / 2.0) / k + w / 2.0 - 0.5, (y + 0.5 - h / 2.0) / k + h / 2.0 - 0.5)
                };
                match mode {
                    1 => {
                        let s = p.powf(curve);
                        pt(z(s), ident, 255.0 * s, false, SMOOTH)
                    }
                    2 if p < 0.5 => pt(z((2.0 * p).powf(curve)), ident, 0.0, false, SMOOTH),
                    2 => pt(ident, z((2.0 - 2.0 * p).powf(curve)), 255.0, false, SMOOTH),
                    _ => {
                        let s = (1.0 - p).powf(curve);
                        let old = if mode == 3 { z(p) } else { ident };
                        pt(old, z(s), 255.0 * (1.0 - s), false, SMOOTH)
                    }
                }
            }
            Kind::Flip { axis, layer } => {
                let (len, a) = if axis == 0 { (w, x) } else { (h, y) };
                let f = if layer == 0 { (p * PI / 2.0).sin() } else { (p * PI / 2.0).cos() };
                let span = (len * f + 0.5).trunc(); // as the plugin rounds it
                let start = ((len - span) / 2.0).floor();
                let (src, dist) = squeeze(a, len, start, start + span);
                let moving = if axis == 0 { (src, y) } else { (x, src) };
                if layer == 0 { pt(ident, moving, dist, true, 0) } else { pt(moving, ident, -dist, true, 0) }
            }
            Kind::Cube { dx, dy } => {
                let axis0 = dx != 0;
                let d = if axis0 { dx } else { dy };
                let (len, a) = if axis0 { (w, x) } else { (h, y) };
                let t = p * PI / 2.0;
                let (s, c) = (t.sin(), t.cos());
                let half = len / 2.0;
                let (outer, edge, far) = (half * (1.0 - s - c), half * (1.0 + s - c), half * (1.0 + s + c));
                let (new, old, dist) = if d > 0 {
                    (squeeze(a, len, outer, edge).0, squeeze(a, len, edge, far).0, edge - (a + 0.5))
                } else {
                    (squeeze(a, len, len - edge, len - outer).0, squeeze(a, len, len - far, len - edge).0, (a + 0.5) - (len - edge))
                };
                let pos = |v| if axis0 { (v, y) } else { (x, v) };
                pt(pos(old), pos(new), dist, true, 0)
            }
            Kind::Melt { .. } => {
                // old row y - shift; the new picture where that is above the screen
                let shift = f64::from(column[(y as usize).min(column.len() - 1)]);
                pt((x, y - shift), ident, shift - y - 0.5, true, 0)
            }
            Kind::Wave { direction, amplitude, waves } => {
                let vertical = direction & 1 == 1;
                let sign = if direction >= 2 { -1.0 } else { 1.0 };
                let amp = sign * f64::from(amplitude) / 1000.0 * (if vertical { h } else { w }) * (PI * p).sin();
                let len = if vertical { w } else { h };
                let waves = f64::from(waves) / 10.0;
                let pos = if vertical {
                    (x, y - amp * (2.0 * PI * (waves * x / len + p)).sin())
                } else {
                    (x - amp * (2.0 * PI * (waves * y / len + p)).sin(), y)
                };
                pt(pos, pos, (255.0 * p).round_ties_even(), false, 0)
            }
        }
    }
}

/// Signed distance of a pixel centre to a screen-sized rectangle at rx, ry
/// moved in direction sx, sy from the screen: only its edges inside the
/// screen count (so the distance stays straight for interpolation).
#[allow(clippy::too_many_arguments)]
fn edge_dist(x: f64, y: f64, rx: f64, ry: f64, w: f64, h: f64, sx: f64, sy: f64) -> f64 {
    let ex = if sx > 0.0 { x + 0.5 - rx } else { rx + w - x - 0.5 };
    let ey = if sy > 0.0 { y + 0.5 - ry } else { ry + h - y - 0.5 };
    if sy == 0.0 {
        ex
    } else if sx == 0.0 {
        ey
    } else {
        ex.min(ey)
    }
}

/// Source along one axis for the whole picture drawn on a..b, and the
/// signed distance to that span (positive inside).
fn squeeze(a: f64, len: f64, from: f64, to: f64) -> (f64, f64) {
    let span = (to - from).max(1e-3);
    ((a + 0.5 - from) * len / span - 0.5, (a + 0.5 - from).min(to - a - 0.5))
}

/// Trans_MeltDown's acceleration of the byte `speed` on an `h` px screen,
/// 16.16 px per row per row (src: exe 0x4451e0, truncating).
fn melt_accel(speed: u8, h: u32) -> i32 {
    let b = f64::from(speed) / 255.0;
    (((b * b * 0.4499999899417162 + 0.029999999329447746) / f64::from(h) * 65536.0) as i32).max(3)
}

/// MSVC's rand() from `seed`.
fn msvc_rand(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed;
    move || {
        s = s.wrapping_mul(214013).wrapping_add(2531011);
        (s >> 16) & 0x7fff
    }
}

/// The exe's key-frame spline between keys v1 and v2 at t (src: exe
/// 0x407540: f32 in and out, the x87 at 53 bits between).
#[allow(clippy::too_many_arguments)]
fn melt_hermite(d_prev: f32, d_cur: f32, d_next: f32, v0: f32, v1: f32, v2: f32, v3: f32, t: f32) -> f32 {
    let [d_prev, d_cur, d_next, v0, v1, v2, v3, t] = [d_prev, d_cur, d_next, v0, v1, v2, v3, t].map(f64::from);
    let t2 = t * t;
    let t3 = t2 * t;
    let hh = (t2 * 3.0 - t3) - t3;
    let out = d_cur / (d_next + d_cur) * ((v2 - v1) + (v3 - v2)) * (t3 - t2)
        + (((t3 - t2) - t2) + t) * ((v1 - v0) + (v2 - v1)) * (d_cur / (d_prev + d_cur))
        + hh * v2
        + (1.0 - hh) * v1;
    out as f32
}

/// The melting edge: each column's offset in -h..0 (src: exe 0x445310, a
/// spline through 11 keys rand() & h at x = i * w / 10, scaled to h).
fn melt_edge(w: u32, h: u32, seed: u32) -> Vec<i32> {
    let mut rand = msvc_rand(seed);
    let xs: Vec<u32> = (0..11).map(|i| i * w / 10).collect();
    let keys: Vec<f32> = (0..11).map(|_| (rand() & h) as f32).collect();
    // a key's length from the key before (the first key's 1)
    let len = |i: usize| if i == 0 { 1.0 } else { (xs[i] - xs[i - 1]) as f32 };
    let v: Vec<f32> = (0..w)
        .map(|x| {
            let j = (0..10).rev().find(|&j| xs[j] <= x).unwrap_or(0);
            let t = (((x - xs[j]) << 16) / (xs[j + 1] - xs[j])) as f32 / 65536.0;
            let (m, n) = (j.saturating_sub(1), (j + 2).min(10));
            melt_hermite(len(j), len(j + 1), len(n), keys[m], keys[j], keys[j + 1], keys[n], t)
        })
        .collect();
    let lo = v.iter().copied().fold(0f32, f32::min);
    let hi = v.iter().copied().fold(0f32, f32::max);
    let range = hi - lo;
    if range as i32 == 0 {
        return vec![0; w as usize];
    }
    let scale = (f64::from(h) / f64::from(range)) as f32;
    v.iter().map(|&v| ((f64::from(v) - f64::from(lo)) * f64::from(scale)) as f32 as i32 - h as i32).collect()
}

/// The shift of each row 0..=h of a column with edge offset `off` at
/// progress `t16` (16.16): the old picture's row y shows its row y - shift
/// (src: exe 0x4456b0 sets the columns falling, 0x4458e0 draws them).
fn melt_column(off: i32, accel: i32, t16: u32, h: u32) -> Vec<i32> {
    let h = i64::from(h);
    let a = f64::from(accel) / 65536.0;
    let fall = (2.0 * h as f64 / a).sqrt() as i64; // rows to fall h from rest
    let k = i64::from(off) - h + (((fall + h) * i64::from(t16)) >> 16);
    // rows above -k stand; from there the column falls, already k rows in
    let (mut vel, mut pos) = if k < 1 {
        (0i32, 0i32)
    } else {
        let k = k as f64;
        ((k * a * 65536.0) as i32, (a * 0.5 * k * k * 65536.0) as i32)
    };
    (0..=h)
        .map(|y| {
            if y < -k {
                return 0;
            }
            let shift = i32::from((pos >> 16) as i16);
            vel = vel.wrapping_add(accel);
            pos = pos.wrapping_add(vel);
            shift
        })
        .collect()
}

fn u16_of(v: f64) -> u16 {
    v.round_ties_even().clamp(0.0, 65535.0) as u16
}

/// The map of `kind` for a w x h screen: [`Kind::cells`] cells side by side.
pub fn map(kind: Kind, w: u32, h: u32) -> image::RgbImage {
    let (gw, gh) = (w / GRID + 1, h / GRID + 1);
    let cells = kind.cells();
    let melt = match kind {
        // ours: fixed seed, the exe seeds rand from the clock (srand(_time64) 0x407150)
        Kind::Melt { speed, seed } => Some((melt_edge(w, h, seed), melt_accel(speed, h))),
        _ => None,
    };
    let mut img = image::RgbImage::new(gw * cells, 4 * gh);
    for c in 0..cells {
        let p = f64::from(c) / f64::from(cells - 1);
        for i in 0..gw {
            let x = f64::from(i * w) / f64::from(gw - 1);
            let column = match &melt {
                Some((edge, accel)) => melt_column(edge[(x as usize).min(edge.len() - 1)], *accel, (p * 65536.0).round() as u32, h),
                None => Vec::new(),
            };
            for j in 0..gh {
                let y = f64::from(j * h) / f64::from(gh - 1);
                let pt = kind.point(p, x, y, f64::from(w), f64::from(h), &column);
                // sharp: level 128 + 256 * distance; else the level itself
                let mask = 32768.0 + if pt.sharp { pt.mask * 16.0 } else { (pt.mask - 128.0) * 16.0 };
                let pos = |v: f64, at: f64| u16_of(32768.0 + (v - at) * 16.0);
                let planes = [
                    [pos(pt.old.0, x), pos(pt.old.1, y), u16_of(mask)],
                    [pos(pt.new.0, x), pos(pt.new.1, y), 0],
                ];
                for (k, v) in planes.iter().enumerate() {
                    let hi = image::Rgb([(v[0] >> 8) as u8, (v[1] >> 8) as u8, if k == 1 { pt.flags } else { (v[2] >> 8) as u8 }]);
                    let lo = image::Rgb([v[0] as u8, v[1] as u8, if k == 1 { if pt.sharp { 8 } else { 0 } } else { v[2] as u8 }]);
                    let px = c * gw + i;
                    img.put_pixel(px, (2 * k as u32) * gh + j, hi);
                    img.put_pixel(px, (2 * k as u32 + 1) * gh + j, lo);
                }
            }
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn value(img: &image::RgbImage, cell: u32, plane: u32, ch: usize, i: u32, j: u32, gw: u32, gh: u32) -> i32 {
        let hi = img.get_pixel(cell * gw + i, plane * gh + j)[ch] as i32;
        let lo = img.get_pixel(cell * gw + i, (plane + 1) * gh + j)[ch] as i32;
        (hi << 8 | lo) - 32768
    }

    #[test]
    fn slide_in_from_the_right() {
        let img = map(Kind::Slide { layer: 0, dx: -1, dy: 0 }, 160, 96);
        let (gw, gh) = (21, 13);
        assert_eq!(img.dimensions(), (2 * gw, 4 * gh));
        // at the start the new picture is a screen to the right: read 160 px left
        assert_eq!(value(&img, 0, 2, 0, 5, 3, gw, gh), -160 * 16);
        // at the end it is in place, the old picture never moves
        assert_eq!(value(&img, 1, 2, 0, 5, 3, gw, gh), 0);
        assert_eq!(value(&img, 0, 0, 0, 5, 3, gw, gh), 0);
        // the mask: the distance to the new picture's left edge, 1/16 px
        assert_eq!(value(&img, 1, 0, 2, 5, 3, gw, gh), 8 * (2 * 40 + 1));
        assert_eq!(img.get_pixel(5, 3 * gh + 3)[2], 8); // sharpness
    }

    #[test]
    fn diagonal_push_reads_black_outside() {
        let img = map(Kind::Slide { layer: 2, dx: -1, dy: -1 }, 160, 96);
        assert_eq!(img.get_pixel(0, 2 * 13)[2], BLACK);
    }

    #[test]
    fn zoom_samples_bilinearly_and_cross_fades() {
        let img = map(Kind::Zoom { mode: 0, scale: 200, curve: 1 }, 160, 96);
        let (gw, gh) = (21, 13);
        assert_eq!(img.get_pixel(0, 2 * gh)[2], SMOOTH);
        // halfway the mask is the level 255 * 0.5 (plain level L: 16 * (L - 128))
        let half = Kind::Zoom { mode: 0, scale: 200, curve: 1 }.cells() / 2;
        assert_eq!(value(&img, half, 0, 2, 0, 0, gw, gh), (16.0f64 * (127.5 - 128.0)).round_ties_even() as i32);
    }
    // Trans_MeltDown run in the exe (research/xuse/melt_exe.py):
    // research/xuse/melt_exe.py: 0x4451e0 of 0, 128, 255 at h 600
    const EXE_ACCEL: [u32; 3] = [3, 15, 52];
    // 0x445310 at 800 x 600, seed 1: every 25th column
    const EXE_EDGE: [i32; 32] = [-573, -577, -580, -581, -588, -600, -582, -493, -329, -164, -65, -12, -3, -51, -196, -381, -513, -560, -559, -518, -410, -235, -103, -127, -297, -467, -490, -358, -180, -67, -31, -28];
    // 0x4458e0 on 16 x 12 (edge [-4, -11, -6, 0, -8, -11, -11, -6, 0, -8, -4, 0, -11, -12, -11, -8], accel 2621): source row + 1, 0 the new picture
    const EXE_DRAW: [[[u8; 16]; 12]; 4] = [
        [[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2], [3, 3, 3, 2, 3, 3, 3, 3, 2, 3, 3, 2, 3, 3, 3, 3], [4, 4, 4, 3, 4, 4, 4, 4, 3, 4, 4, 3, 4, 4, 4, 4], [5, 5, 5, 3, 5, 5, 5, 5, 3, 5, 5, 3, 5, 5, 5, 5], [5, 6, 6, 4, 6, 6, 6, 6, 4, 6, 5, 4, 6, 6, 6, 6], [6, 7, 7, 5, 7, 7, 7, 7, 5, 7, 6, 5, 7, 7, 7, 7], [7, 8, 7, 5, 8, 8, 8, 7, 5, 8, 7, 5, 8, 8, 8, 8], [7, 9, 8, 5, 9, 9, 9, 8, 5, 9, 7, 5, 9, 9, 9, 9], [8, 10, 9, 6, 9, 10, 10, 9, 6, 9, 8, 6, 10, 10, 10, 9], [8, 11, 9, 6, 10, 11, 11, 9, 6, 10, 8, 6, 11, 11, 11, 10], [9, 12, 10, 7, 11, 12, 12, 10, 7, 11, 9, 7, 12, 12, 12, 11]],
        [[1, 1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1], [2, 2, 2, 0, 2, 2, 2, 2, 0, 2, 2, 0, 2, 2, 2, 2], [2, 3, 3, 1, 3, 3, 3, 3, 1, 3, 2, 1, 3, 3, 3, 3], [3, 4, 3, 1, 4, 4, 4, 3, 1, 4, 3, 1, 4, 4, 4, 4], [3, 5, 4, 2, 5, 5, 5, 4, 2, 5, 3, 2, 5, 5, 5, 5], [4, 6, 5, 2, 5, 6, 6, 5, 2, 5, 4, 2, 6, 6, 6, 5], [5, 7, 5, 2, 6, 7, 7, 5, 2, 6, 5, 2, 7, 7, 7, 6], [5, 8, 6, 3, 7, 8, 8, 6, 3, 7, 5, 3, 8, 8, 8, 7], [5, 8, 6, 3, 7, 8, 8, 6, 3, 7, 5, 3, 8, 9, 8, 7], [6, 9, 7, 3, 8, 9, 9, 7, 3, 8, 6, 3, 9, 9, 9, 8], [6, 10, 7, 3, 8, 10, 10, 7, 3, 8, 6, 3, 10, 10, 10, 8], [7, 10, 8, 3, 9, 10, 10, 8, 3, 9, 7, 3, 10, 11, 10, 9]],
        [[0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1], [0, 2, 0, 0, 1, 2, 2, 0, 0, 1, 0, 0, 2, 2, 2, 1], [0, 3, 1, 0, 2, 3, 3, 1, 0, 2, 0, 0, 3, 3, 3, 2], [1, 3, 2, 0, 2, 3, 3, 2, 0, 2, 1, 0, 3, 4, 3, 2], [1, 4, 2, 0, 3, 4, 4, 2, 0, 3, 1, 0, 4, 4, 4, 3], [1, 5, 2, 0, 4, 5, 5, 2, 0, 4, 1, 0, 5, 5, 5, 4], [2, 5, 3, 0, 4, 5, 5, 3, 0, 4, 2, 0, 5, 6, 5, 4], [2, 6, 3, 0, 4, 6, 6, 3, 0, 4, 2, 0, 6, 6, 6, 4], [2, 6, 4, 0, 5, 6, 6, 4, 0, 5, 2, 0, 6, 7, 6, 5], [2, 7, 4, 0, 5, 7, 7, 4, 0, 5, 2, 0, 7, 7, 7, 5], [2, 7, 4, 0, 6, 7, 7, 4, 0, 6, 2, 0, 7, 8, 7, 6], [3, 8, 4, 0, 6, 8, 8, 4, 0, 6, 3, 0, 8, 8, 8, 6]],
        [[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0], [0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0], [0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 2, 1, 0], [0, 2, 0, 0, 1, 2, 2, 0, 0, 1, 0, 0, 2, 2, 2, 1], [0, 3, 0, 0, 1, 3, 3, 0, 0, 1, 0, 0, 3, 3, 3, 1], [0, 3, 0, 0, 1, 3, 3, 0, 0, 1, 0, 0, 3, 4, 3, 1], [0, 3, 0, 0, 2, 3, 3, 0, 0, 2, 0, 0, 3, 4, 3, 2], [0, 4, 0, 0, 2, 4, 4, 0, 0, 2, 0, 0, 4, 4, 4, 2], [0, 4, 1, 0, 2, 4, 4, 1, 0, 2, 0, 0, 4, 5, 4, 2], [0, 5, 1, 0, 2, 5, 5, 1, 0, 2, 0, 0, 5, 5, 5, 2], [0, 5, 1, 0, 2, 5, 5, 1, 0, 2, 0, 0, 5, 6, 5, 2], [0, 5, 1, 0, 3, 5, 5, 1, 0, 3, 0, 0, 5, 6, 5, 3]],
    ];

    #[test]
    fn melt_accel_matches_the_exe() {
        assert_eq!([0, 128, 255].map(|b| melt_accel(b, 600)), EXE_ACCEL.map(|a| a as i32));
    }

    #[test]
    fn melt_edge_matches_the_exe() {
        let edge = melt_edge(800, 600, 1);
        assert_eq!(edge.iter().step_by(25).copied().collect::<Vec<_>>(), EXE_EDGE);
        assert_eq!(melt_edge(16, 12, 1), [-4, -11, -6, 0, -8, -11, -11, -6, 0, -8, -4, 0, -11, -12, -11, -8]);
    }

    #[test]
    fn melt_draw_matches_the_exe() {
        let (w, h) = (16, 12);
        let accel = melt_accel(255, h);
        assert_eq!(accel, 2621);
        let edge = melt_edge(w, h, 1);
        for (frame, t16) in [32768, 40960, 49152, 57344].into_iter().enumerate() {
            let cols: Vec<Vec<i32>> = edge.iter().map(|&off| melt_column(off, accel, t16, h)).collect();
            let drawn: Vec<Vec<u8>> = (0..h as i32)
                .map(|y| cols.iter().map(|c| u8::try_from(y - c[y as usize]).ok().filter(|&r| i32::from(r) < h as i32).map_or(0, |r| r + 1)).collect())
                .collect();
            assert_eq!(drawn, EXE_DRAW[frame].map(|r| r.to_vec()).to_vec(), "t16 {t16}");
        }
    }
}
