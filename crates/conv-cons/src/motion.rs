//! Sprites moving by themselves ([`Op::SpriteMotion`]) with CONscripter's
//! `spmotion`, which the engine steps during text and click waits too.

use std::fmt::Write;

use crate::SPRITE_BASE;

/// Writes a motion of layer `layer` (no keys: stops it).
pub(crate) fn emit(out: &mut String, layer: u32, keys: &[(u32, i32, i32)], looped: bool) {
    let slot = SPRITE_BASE + layer;
    if keys.is_empty() {
        writeln!(out, "spmotion {slot}").unwrap();
        return;
    }
    let keys: Vec<String> = keys.iter().map(|(t, x, y)| format!("{t},{x},{y}")).collect();
    writeln!(out, "spmotion {slot},{},{}", u8::from(looped), keys.join(",")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_keys_or_stops() {
        let mut out = String::new();
        emit(&mut out, 5, &[(0, 0, 0), (250, 0, -12), (500, 0, 0)], true);
        emit(&mut out, 5, &[], false);
        assert_eq!(out, "spmotion 67,1,0,0,0,250,0,-12,500,0,0\nspmotion 67\n");
    }
}
