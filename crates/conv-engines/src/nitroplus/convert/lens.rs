//! `CreateEffect(…, "Lens")`: a refraction disc over the screen
//! (Muramasa.exe init `004338f0`, draw `004409f0`; findings row
//! "CreateEffect Lens"), drawn by the engine's `screenlens` in
//! [`LENS_SLOT`]. Its diameter is the object's width, its centre the
//! object's; `$SYSTEM_effect_lens_curvature` and `_distance` as set when
//! it is made. A loop process moving it (装甲悪鬼村正's `LensMove`, a
//! `BezierMove` figure 8) becomes the lens's list of offsets, one a frame
//! ([`Compiler::lens_wobble`]).

use conv_core::ir::{Op, ScreenEffect};

use super::{Compiler, Kind, Value};

/// The screen-effect slot of the lens (1 is `Shake`'s).
pub(super) const LENS_SLOT: u32 = 2;
/// The exe's frame (`0048c5d0`): a wobble's offsets are sampled at it.
pub(super) const FRAME_MS: u32 = 16;

/// A [`Kind::Lens`]: its settings, and while a loop process moves it the
/// process and the offsets (px from its place, one a [`FRAME_MS`]).
#[derive(Debug, Clone)]
pub(super) struct LensObject {
    curvature: i64,
    distance: i64,
    pub(super) wobble: Option<(String, Vec<(i32, i32)>)>,
}

impl Compiler<'_, '_> {
    /// A lens with the settings the script has set (the exe's defaults
    /// 1 and 1, findings row).
    pub(super) fn lens(&self) -> LensObject {
        let setting = |k: &str| self.vars.get(k).filter(|v| !matches!(v, Value::Null)).map_or(1, Value::int);
        LensObject { curvature: setting("$SYSTEM_effect_lens_curvature"), distance: setting("$SYSTEM_effect_lens_distance"), wobble: None }
    }

    /// The `screenlens` that brings the engine's lens to the one the
    /// objects now show (the topmost seen; a clear when none), or `None`
    /// when it is so already.
    pub(super) fn lens_op(&mut self) -> Option<Op> {
        let mut lenses: Vec<_> = self.objects.iter().filter(|(_, o)| o.opacity > 0 && matches!(o.kind, Kind::Lens(_))).collect();
        lenses.sort_by_key(|(_, o)| (o.priority, o.seq));
        // ours: the whole screen below the text window, not only what lies
        // below the lens's z; one lens (both uses show one)
        let want = lenses.last().map(|&(n, o)| {
            let Kind::Lens(l) = &o.kind else { unreachable!() };
            let (px, py) = self.parent_offset(n);
            let offsets = l.wobble.as_ref().map(|(_, v)| v.clone()).unwrap_or_default();
            ScreenEffect::Lens {
                centre: ((o.x + px + o.w / 2) as i32, (o.y + py + o.h / 2) as i32),
                diameter: o.w as i32,
                curvature: l.curvature as i32,
                distance: l.distance as i32,
                // ours: the effect's 0-1000 opacity as the engine's 0-255 (500 gives 128)
                opacity: ((o.opacity.min(1000) * 255 + 500) / 1000) as u32,
                interval_ms: FRAME_MS,
                looped: !offsets.is_empty(),
                offsets,
            }
        });
        if want == self.lens {
            return None;
        }
        self.lens = want.clone();
        Some(Op::ScreenEffect { slot: LENS_SLOT, effect: want })
    }

    /// Loop process `owner` moves lens `target` along `at` (its place at
    /// a time) over `len` ms, looped: the offsets from its place at 0,
    /// one a frame.
    pub(super) fn lens_wobble(&mut self, owner: &str, target: &str, len: u32, at: impl Fn(u32) -> (f64, f64)) -> bool {
        let Some(o) = self.objects.get_mut(target) else { return false };
        let Kind::Lens(l) = &mut o.kind else { return false };
        let (x0, y0) = at(0);
        let offsets = (0..len).step_by(FRAME_MS as usize).map(at).map(|(x, y)| ((x - x0).round() as i32, (y - y0).round() as i32)).collect();
        l.wobble = Some((owner.to_string(), offsets));
        (o.x, o.y) = (x0.round() as i64, y0.round() as i64);
        true
    }
}
