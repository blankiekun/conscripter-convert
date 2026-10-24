//! The eboot's particle effects (EffectStart 51 snow, 52 light balls, 54
//! confetti) as CONscripter's `"particle"` layer keys (COMMANDS.md
//! `setlayer`). Every number is the eboot's, run under exe_call
//! (research/oretsuba/particles_exe.py, fixture particles_exe.txt;
//! docs/games/oretsuba.md Status). Units: a 1280×720 screen, `U` =
//! rand()/2^31; every effect steps twice an update, 60 updates a second.

/// Which effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 雪 (51): falling snow.
    Snow,
    /// 光玉 (52): rising, twinkling balls of light.
    LightBalls,
    /// 紙吹雪 (54): falling paper.
    Confetti,
}

impl Kind {
    pub fn of(effect: i64) -> Option<Kind> {
        match effect {
            51 => Some(Kind::Snow),
            52 => Some(Kind::LightBalls),
            54 => Some(Kind::Confetti),
            _ => None,
        }
    }

    /// The ETC.cpk sheet: 32-px cells, row = class, column = frame.
    pub fn sheet(self) -> &'static str {
        match self {
            Kind::Snow => "snow.png",
            Kind::LightBalls => "lightball.png",
            Kind::Confetti => "paper.png",
        }
    }
}

/// Classes per speed-table entry. ours: the eboot computes each speed from
/// the particle's class; the layer looks it up in a table (class buckets of
/// 0.05, the speed at the bucket's middle).
const BUCKET: f64 = 0.05;

/// The speed table over classes [0, `span`).
fn table(span: f64, v: impl Fn(f64) -> f64) -> String {
    let n = (span / BUCKET).round() as usize;
    (0..n).map(|i| num(v((i as f64 + 0.5) * BUCKET))).collect::<Vec<_>>().join(":")
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".to_string() } else { s.to_string() }
}

/// The layer's keys (without `img`) for `kind` with the task's `:Param`
/// values (`param(i)`; a missing one is 0, cfg_number 0x8108c8a8).
pub fn keys(kind: Kind, param: impl Fn(usize) -> Option<f64>) -> String {
    let p = |i| param(i).unwrap_or(0.0);
    // The count: 0, a negative one or one over the pool is the pool (snow
    // start 0x81047ccc, light balls 0x81043902, confetti 0x81042f66).
    let pool = if kind == Kind::Confetti { 100 } else { 200 };
    let n = match p(0) as i64 {
        c if c > 0 && c <= pool => c,
        _ => pool,
    };
    // Every pool starts inactive and a free particle is born with P 0.002
    // a step; the birth step does not move it (pool sizes: the ctors
    // 0x810479f4, 0x81043604, the start 0x81042f66).
    let common = format!("n={n},hz=60,steps=2,spawn=0.002,cw=32,ch=32,scale=0.75555557");
    match kind {
        // Update 0x81047dfa. Class 4U + [U > 0.7]; born at x = 1280U,
        // y = -720 (a respawn keeps vx and ax); x walks (vx += ax, ax +=
        // (U-0.5)·0.2, vx in ±1.3, ax in ±0.2, ax starts U). Speed 5/(4.5-c),
        // then /5 past 5, ×0.55 past 4, ×0.75 past 3, in turn (c ≥ 4.5:
        // negative, it rises for good). Past y = 470 + 50c: outside x
        // [0, 1280) reborn, else alpha -20 a step, reborn at ≤ 25. White,
        // drawn +31 px.
        Kind::Snow => {
            let vy = table(5.0, |c| {
                let mut v = 5.0 / (4.5 - c);
                if v > 5.0 {
                    v /= 5.0;
                }
                if v > 4.0 {
                    v *= 0.55;
                }
                if v > 3.0 {
                    v *= 0.75;
                }
                v
            });
            format!("{common},ox=31,oy=31,cls=4:1:0.7,vy={vy},x=0:1280,y=-720:0,walk=x,wv=-1.3:1.3,wa=0.2:0.2:1,end=470:50:1,fade=20:25")
        }
        // Update 0x81043c8a. As snow, but class 3U + [U > 0.7], born at
        // y = 1440, vx in ±1.6, rising 1.25/(4.5-c); the line y = 80 - 20c
        // above. Colour 0xffff00 (0x810438d2), alpha × one of 20 triangle
        // waves (±0.032 an update, particle i takes wave i mod 20). 600
        // steps at the start only when :Param 1 ≠ 0 (0x81043902).
        Kind::LightBalls => {
            let vy = table(4.0, |c| -1.25 / (4.5 - c));
            let warm = if p(1) != 0.0 { ",warm=600" } else { "" };
            format!("{common},ox=32,oy=32,cls=3:1:0.7,vy={vy},x=0:1280,y=1440:0,walk=x,wv=-1.6:1.6,wa=0.2:0.2:1,end=80:-20:-1,fade=20:25,tint=ffff00,twinkle=20:0.032{warm}")
        }
        // Start 0x81042f66, steps 0x8112ad40. Class 3U + [U > 0.7]; y
        // walks (vy += ay, ay += (U-0.5)·0.025, vy in [-0.2, 0.6], ay in
        // ±0.1, ay starts 0.4U) and falls :Param 1 a step (0: 10); x
        // drifts ±(0.6c + 0.2 + 0.05U)·:Param 2, the frame steps every 50
        // updates 0-1-2-1-0, turning the drift at the ends and ramping it
        // back up by r = (r + 0.008)·1.2. Born at y = 1440U - 720: ≤ 0 at
        // the top (y = -32, x = 1280U), else at that y on a side. Reborn
        // at x ≥ 1312, x < -64 or y ≥ 720. Red, yellow, blue or green. 1200
        // steps at the start only when :Param 3 ≠ 0.
        Kind::Confetti => {
            let fall = match p(1) {
                f if f == 0.0 => 10.0,
                f => f,
            };
            let warm = if p(3) != 0.0 { ",warm=1200" } else { "" };
            // ours: a side birth is at x -64 or 1344 (half each, 0x8112b11c's
            // respawn), also the first time, where the eboot puts it at -32.
            format!(
                "{common},ox=32,oy=32,cls=3:1:0.7,vy={},x=0:1280,y=-32:0,side=0.5:-64:1344:0:720,walk=y,wv=-0.2:0.6,wa=0.1:0.025:0.4,box=-64:-100000:1312:720,drift=0.6:0.2:0.05:{},flip=3:50:0.008:1.2,tint=ff0000:ffff00:0000ff:00ff00{warm}",
                num(fall),
                num(p(2))
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snow_speeds_match_the_eboot_run() {
        // particles_exe.txt: c 0.4 → 1.2195, 2 → 2, 4.4 → 4.125, 4.6 → -50.
        let k = keys(Kind::Snow, |_| Some(30.0));
        let vy: Vec<f64> = k.split("vy=").nth(1).unwrap().split(',').next().unwrap().split(':').map(|v| v.parse().unwrap()).collect();
        assert_eq!(vy.len(), 100);
        assert!((vy[8] - 5.0 / 4.075).abs() < 1e-3);
        assert!(vy[89] > 0.0 && vy[90] < -10.0);
        assert!(k.starts_with("n=30,"));
    }

    #[test]
    fn confetti_params() {
        let k = keys(Kind::Confetti, |i| [40.0, 1.5, 0.5, 0.0].get(i).copied());
        assert!(k.contains(",vy=1.5,") && k.contains("drift=0.6:0.2:0.05:0.5,") && !k.contains("warm"));
        let k = keys(Kind::Confetti, |_| None);
        assert!(k.starts_with("n=100,") && k.contains(",vy=10,") && k.contains("drift=0.6:0.2:0.05:0,"));
    }
}
