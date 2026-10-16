//! [`Op::HoldMovie`]: a movie that plays while the player holds the button
//! down (CONscripter's `mpeghold`), its prompt on a menu sprite slot (no
//! menu opens during the movie) and its sound on the next effect channel.

use std::fmt::Write;

use conv_core::ir::{HoldPrompt, HoldSound};

use crate::{path, sprite_mode, SeChannels, MENU_SPRITES};

pub(crate) fn hold_movie(out: &mut String, se: &mut SeChannels, file: &str, prompt: Option<&HoldPrompt>, sound: Option<&HoldSound>) {
    let slot = MENU_SPRITES.start;
    let mut args = format!("\"{}\"", path(file));
    if let Some(p) = prompt {
        let mode = sprite_mode(&p.image);
        writeln!(out, "lsp {slot},\":{mode}/{},{},0;{}\",{},{}", p.cells, p.frame_ms, path(&p.image), p.x, p.y).unwrap();
        write!(args, ",{slot}").unwrap();
    } else if sound.is_some() {
        args.push_str(",-1");
    }
    if let Some(s) = sound {
        let ch = se.start(&s.file, false);
        if se.volumes {
            writeln!(out, "chvolfade {ch},100").unwrap();
        }
        write!(args, ",{ch},\"{}\",{}", path(&s.file), s.fade_ms).unwrap();
    }
    writeln!(out, "mpeghold {args}").unwrap();
    if prompt.is_some() {
        writeln!(out, "csp {slot}").unwrap();
    }
}
