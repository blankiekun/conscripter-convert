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

/// A numbered layer of its own ([`Op::ParticleLayer`](conv_core::ir::Op::ParticleLayer)),
/// shown by the sprites whose picture is
/// [`particle_sprite`](conv_core::ir::particle_sprite): its message
/// variable and the layer, in the define section.
pub(crate) fn define_layer(layer: u32) -> String {
    format!("defstr conv_particles_{layer}\nsetlayer {layer},16,\"particle\"")
}

/// After a load: layer `layer` (re)started with the save's message, or stopped.
pub(crate) fn layer_after_load(layer: u32) -> String {
    format!("layermessage {layer},\"x\":if $conv_particles_{layer} != \"\" layermessage {layer},$conv_particles_{layer}")
}

/// Starts or stops layer `layer` (nothing printed: its sprite shows it).
pub(crate) fn emit_layer(out: &mut String, layer: u32, particles: &Option<Particles>) {
    match particles {
        Some(p) => writeln!(out, "mov $conv_particles_{layer},\"s|img={},{}\":layermessage {layer},$conv_particles_{layer}", crate::path(&p.image), p.keys).unwrap(),
        None => writeln!(out, "layermessage {layer},\"x\":mov $conv_particles_{layer},\"\"").unwrap(),
    }
}
