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

/// A transition drawn by a warp map, with the plugin's parameters.
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
    /// `cons_melt.dll/stretch,seed` (the seed picks the melting edge)
    Melt { stretch: u32, seed: u32 },
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
            Kind::Melt { stretch, seed } => format!("melt_{stretch}_{seed}"),
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

    fn point(&self, p: f64, x: f64, y: f64, w: f64, h: f64, delay: &[f64]) -> Point {
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
            Kind::Melt { stretch, .. } => {
                let d = delay[(x as usize).min(delay.len() - 1)];
                let f = if p > d { (p - d) / (1.0 - d) } else { 0.0 };
                let fallen = h * f * f;
                let scale = 1.0 + f64::from(stretch) / 100.0 * fallen / h;
                // old shows from row floor(fallen) on, read from floor((y - fallen) / scale)
                pt((x, (y - fallen) / scale - 0.5), ident, fallen - y - 0.5, true, 0)
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

/// cons_melt's delay of each column (in f32, as the plugin computes it).
fn melt_delay(w: u32, seed: u32) -> Vec<f64> {
    let mut r = seed;
    let mut pt = [0f32; 13];
    for i in 0..11 {
        r = r.wrapping_mul(1103515245).wrapping_add(12345);
        pt[i + 1] = ((r >> 16) & 0x7fff) as f32 / 32768.0;
    }
    pt[0] = pt[1];
    pt[12] = pt[11];
    let v: Vec<f32> = (0..w)
        .map(|x| {
            let u = x as f32 * 10.0 / w as f32;
            let i = u as usize;
            let t = u - i as f32;
            let (q0, q1, q2, q3) = (pt[i], pt[i + 1], pt[i + 2], pt[i + 3]);
            0.5 * (2.0 * q1 + (q2 - q0) * t + (2.0 * q0 - 5.0 * q1 + 4.0 * q2 - q3) * t * t + (3.0 * q1 - q0 - 3.0 * q2 + q3) * t * t * t)
        })
        .collect();
    let lo = v.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    v.iter()
        .map(|&v| {
            let n = if hi > lo { (v - lo) / (hi - lo) } else { 0.0 };
            f64::from(0.2 + 0.1 * (0f32).max((n - 0.4) / 0.6))
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
    let delay = match kind {
        Kind::Melt { seed, .. } => melt_delay(w, seed),
        _ => Vec::new(),
    };
    let mut img = image::RgbImage::new(gw * cells, 4 * gh);
    for c in 0..cells {
        let p = f64::from(c) / f64::from(cells - 1);
        for j in 0..gh {
            for i in 0..gw {
                let x = f64::from(i * w) / f64::from(gw - 1);
                let y = f64::from(j * h) / f64::from(gh - 1);
                let pt = kind.point(p, x, y, f64::from(w), f64::from(h), &delay);
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
}
