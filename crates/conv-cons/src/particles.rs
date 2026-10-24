//! Live particles ([`Op::Particles`]): CONscripter's `"particle"` layer
//! (COMMANDS.md `setlayer`), shown by a `spnew` handle under the animation.
//! The keys in effect stay in a string variable, so a load or `csp -1`
//! brings the particles back.

use conv_core::ir::Particles;
use std::fmt::Write;

/// The layer's number (`setlayer`), its sprite handle and the message in
/// effect (`""`: none).
const LAYER: u32 = 1;
pub(crate) const SPRITE: &str = "%conv_particles";
const MESSAGE: &str = "$conv_particles_message";

/// The handle and the message variable, in the define section; with the
/// layer itself.
pub(crate) fn define() -> String {
    format!("defvar conv_particles\ndefstr conv_particles_message\nspnew {SPRITE}\nsetlayer {LAYER},16,\"particle\"")
}

/// Shows the particles again after `csp -1`.
pub(crate) fn restore() -> String {
    format!("if {MESSAGE} != \"\" lsp {SPRITE},\"*{LAYER}\",0,0\n")
}

/// After a load: (re)starts the layer with the save's message, or stops it.
pub(crate) fn after_load() -> String {
    format!("layermessage {LAYER},\"x\":if {MESSAGE} != \"\" layermessage {LAYER},{MESSAGE}")
}

/// Starts or stops the particles (the caller prints).
pub(crate) fn emit(out: &mut String, particles: &Option<Particles>) {
    match particles {
        Some(p) => {
            writeln!(out, "mov {MESSAGE},\"s|img={},{}\"", crate::path(&p.image), p.keys).unwrap();
            writeln!(out, "layermessage {LAYER},{MESSAGE}").unwrap();
            writeln!(out, "lsp {SPRITE},\"*{LAYER}\",0,0").unwrap();
        }
        None => writeln!(out, "csp {SPRITE}:layermessage {LAYER},\"x\":mov {MESSAGE},\"\"").unwrap(),
    }
}
