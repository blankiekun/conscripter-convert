//! Eased moves and zooms ([`Op::MoveEase`]): the engine's `spmotion`
//! reaches the key along an `spcurve` table sampled from the curve, one
//! table per curve a game uses, defined at `*start`.

use std::collections::BTreeMap;
use std::fmt::Write;

pub(crate) use conv_core::ease::eased;

/// The `spcurve` id of curve `ease` with `exponent` (see
/// [`Op::MoveEase`](conv_core::ir::Op::MoveEase)), given on first use and
/// kept in `ids`: 10-14, 30-34 and 50-54 for 1-3 with exponents 1-5,
/// 70-73 for 4-7, the free ones between for the rest in order of first
/// use; `None` once the engine's 64 are taken.
pub(crate) fn curve_id(ease: u8, exponent: u32, ids: &mut BTreeMap<(u8, u32), u32>) -> Option<u32> {
    let key = (ease, if matches!(ease, 1..=3 | 8) { exponent.clamp(1, 20) } else { 0 });
    if let Some(&id) = ids.get(&key) {
        return Some(id);
    }
    let fixed = |id: u32| matches!(id, 10..=14 | 30..=34 | 50..=54 | 70..=73);
    let id = match key {
        (1..=3, n) if n <= 5 => 10 + u32::from(ease - 1) * 20 + n - 1,
        (4..=7, _) => 66 + u32::from(ease),
        _ => (10..74).find(|&id| !fixed(id) && !ids.values().any(|&v| v == id))?,
    };
    ids.insert(key, id);
    Some(id)
}

/// A move's, zoom's or motion key's curve: 0 straight, or the `spcurve` id
/// of an eased one, noted in `ids` for its table at `*start` (straight
/// once the ids run out).
pub(crate) fn curve(ease: Option<(u8, u32)>, ids: &mut BTreeMap<(u8, u32), u32>) -> u32 {
    match ease {
        Some((e, n)) if n > 1 || e > 3 => curve_id(e, n, ids).unwrap_or(0),
        _ => 0,
    }
}

/// The `spcurve` line defining curve `id`: its share (0-1024) at 33 even
/// steps of the time (129 for 9-29, whose bounces and swings are short).
pub(crate) fn curve_def(id: u32, ease: u8, exponent: u32) -> String {
    let mut out = format!("spcurve {id}");
    let steps = if ease >= 9 { 128 } else { 32 };
    for i in 0..=steps {
        write!(out, ",{}", (eased(ease, exponent, f64::from(i) / f64::from(steps)) * 1024.0).round() as i64).unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_tables() {
        let ids = &mut BTreeMap::new();
        let d = curve_def(11, 1, 2);
        assert!(d.starts_with("spcurve 11,0,1,4,9,16,"), "{d}");
        assert!(d.ends_with(",1024"));
        assert_eq!(d.split(',').count(), 34);
        let mut id = |e, n| curve_id(e, n, ids).unwrap();
        assert_eq!((id(1, 1), id(3, 2), id(4, 1), id(7, 1), id(4, 3)), (10, 51, 70, 73, 70));
        // the others take the free ids in order of use
        assert_eq!((id(3, 99), id(8, 2), id(27, 0), id(3, 20)), (15, 16, 17, 15));
        // the sine ones (NSS's AxlAuto, DxlAuto, AxlDxl, DxlAxl): half way
        let half = |e| curve_def(70, e, 1).split(',').nth(17).unwrap().parse::<i64>().unwrap();
        assert_eq!([4, 5, 6, 7].map(half), [300, 724, 512, 512]);
        // muramasa_en.exe's DxlAxl1 (0x40cf50): out to the middle, then t²
        let q = |t: f64| eased(8, 2, t);
        assert_eq!([0.0, 0.25, 0.5, 0.75, 1.0].map(q), [0.0, 0.375, 0.5, 0.625, 1.0]);
        // 64 at most
        let ids = &mut BTreeMap::new();
        for n in 1..=30 {
            curve_id(8, n, ids).unwrap();
            curve_id(1, n, ids).unwrap();
        }
        assert_eq!(ids.len(), 40);
        // 19 fixed, 45 given: 10 left
        assert_eq!((9..=29).filter_map(|e| curve_id(e, 0, ids)).count(), 10);
        assert_eq!((ids.len(), curve_id(2, 9, ids), curve(Some((29, 0)), ids)), (50, None, 0));
    }
}
