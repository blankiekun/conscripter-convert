//! The yes/no dialog ([`ConfirmDialog`]).

use super::*;

/// The yes/no dialog (see [`ConfirmDialog`]) over whatever is on screen:
/// asks [`CONFIRM_TEXT`], sets [`CONFIRM`] (1 yes, 0 no or right-click)
/// and takes itself away. The message is centred by its length in bytes
/// (Shift-JIS: half-width characters are one byte, half as wide).
///
/// A dialog with pictures shows the question's picture (or, for a question
/// without one, the base and the text) over the backdrop; its sprites are
/// yes, no, text, picture, backdrop from the top.
pub(crate) fn confirm(out: &mut String, d: &ConfirmDialog) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let size = d.font_size;
    let label_size = d.yes.h as u32 * 2 / 3;
    let centre = d.base.x + d.base.w / 2;
    let pictures = !d.pictures.is_empty();
    let (yes_slot, no_slot, text_slot, base_slot) = if pictures { (DIALOG_TEXT, DIALOG_YES_LABEL, DIALOG_NO_LABEL, DIALOG_NO) } else { (DIALOG_YES, DIALOG_NO, DIALOG_TEXT, DIALOG_BASE) };
    let cells = if d.pressed { 4 } else { 3 };
    w(format!("*{CONFIRM_ROUTINE}"));
    // A confirmation switched off in the settings answers yes at once.
    w(format!("mov {CONFIRM},1"));
    // The switch's flag (switch_off).
    for line in crate::switches::off_var_lines(CONFIRM_KIND, TMP).lines() {
        w(line.to_string());
    }
    w(format!("if {CONFIRM_KIND} > 0 if %%{} == 1 mov {CONFIRM_KIND},0:return", &TMP[1..]));
    if let Some((b, opacity)) = d.backdrop.as_ref().filter(|_| pictures) {
        w(format!("lsp {DIALOG_BASE},\":c;{}\",{},{},{opacity}", path(&b.image), b.x, b.y));
    }
    let base_and_text = [
        format!("lsp {base_slot},\":a;{}\",{},{}", path(&d.base.image), d.base.x, d.base.y),
        format!("len {TMP},{CONFIRM_TEXT}"),
        format!("mov {TMP},{centre}-{TMP}*{size}/4"),
        format!("strsp {text_slot},{CONFIRM_TEXT},{TMP},{},40,1,{size},{size},0,0,0,1", d.base.y + d.text_y),
    ];
    if pictures {
        for (kind, p) in &d.pictures {
            w(format!("if {CONFIRM_KIND} == {} lsp {base_slot},\":a;{}\",{},{}:goto *conv_confirm_drawn", switch_index(*kind) + 1, path(&p.image), p.x, p.y));
        }
        for line in &base_and_text {
            w(line.clone());
        }
        w("*conv_confirm_drawn".into());
    } else {
        for line in &base_and_text {
            w(line.clone());
        }
    }
    w(format!("mov {CONFIRM_KIND},0"));
    for (button, label_slot, p, label) in [(yes_slot, DIALOG_YES_LABEL, &d.yes, &d.yes_label), (no_slot, DIALOG_NO_LABEL, &d.no, &d.no_label)] {
        w(format!("lsp {button},\":a/{cells},0,3;{}\",{},{}", path(&p.image), p.x, p.y));
        if pictures {
            continue;
        }
        let chars = label.chars().count() as i32;
        let x = p.x + p.w / (2 * cells) - chars * label_size as i32 / 2;
        let y = p.y + (p.h - label_size as i32) / 2;
        w(format!("strsp {label_slot},\"{}\",{x},{y},10,1,{label_size},{label_size},0,0,0,1", escape(label)));
    }
    // The question and buttons unrolling: the sprites at each step (slot,
    // left x, top y, one cell's width, height), the backdrop's opacity.
    let unroll = d.unroll.filter(|_| pictures && d.fade_ms > 0).zip(d.pictures.first()).map(|(u, (_, p))| {
        let sprites = [(base_slot, p.x, p.y, p.w, p.h), (yes_slot, d.yes.x, d.yes.y, d.yes.w / cells, d.yes.h), (no_slot, d.no.x, d.no.y, d.no.w / cells, d.no.h)];
        (u, sprites, d.backdrop.as_ref().map(|(b, o)| (b.x, b.y, *o)))
    });
    if let Some((u, sprites, backdrop)) = &unroll {
        unroll_steps(&mut w, u, sprites, *backdrop, d.fade_ms, d.fade_ms, true);
    } else if d.fade_ms > 0 {
        w(format!("print 10,{}", d.fade_ms));
    }
    w("*conv_confirm_wait".into());
    let press = if d.pressed { ",3" } else { "" };
    w(format!("spbtn {yes_slot},1{press}:spbtn {no_slot},2{press}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("mov {CONFIRM},0"));
    w(format!("if {BUTTON_VAR} == 1 mov {CONFIRM},1"));
    w(format!("if {BUTTON_VAR} != 1 && {BUTTON_VAR} != 2 && {BUTTON_VAR} != -1 goto *conv_confirm_wait"));
    if let Some((u, sprites, backdrop)) = &unroll {
        unroll_steps(&mut w, u, sprites, *backdrop, d.fade_ms, u.close_ms, false);
    }
    w(DIALOG_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w(if d.fade_ms > 0 && unroll.is_none() { format!("print 10,{}", d.fade_ms) } else { "print 1".into() });
    w("return".into());
}

/// The unroll (`ConfirmDialog::unroll`): the sprites (slot, x, y, width,
/// height) stretched sideways about the line `u.x`, or up and down about
/// `u.y` (eased out over `zoom_ms`), and faded (linearly over `fade_ms`),
/// in from nothing (`opening`) or out to it, each an `spmotion` the engine
/// steps (a key a frame, about every 16 ms, as straight lines between),
/// waited for with `spmotionwait`. The backdrop (x, y, opacity) fades with
/// them. Closing, the sprites are removed at the end (loop 3).
pub(super) fn unroll_steps(w: &mut impl FnMut(String), u: &conv_core::ir::Unroll, sprites: &[(u32, i32, i32, i32, i32)], backdrop: Option<(i32, i32, u8)>, zoom_ms: u32, fade_ms: u32, opening: bool) {
    let total = zoom_ms.max(fade_ms);
    // (spmotion takes 32 keys)
    let steps = (total / 16).clamp(1, 31);
    // At step k: the zoom (0-1) and the alpha (0-1).
    let at = |k: u32| {
        let t = (total * k / steps) as f64;
        let p = (t / zoom_ms.max(1) as f64).min(1.0);
        let eased = 1.0 - (1.0 - p) * (1.0 - p);
        let f = (t / fade_ms.max(1) as f64).min(1.0);
        if opening { (eased, f) } else { (1.0 - eased, 1.0 - f) }
    };
    let mode = if opening { 0 } else { 3 };
    let mut line = Vec::new();
    let mut slots = Vec::new();
    for &(slot, sx, sy, sw, sh) in sprites {
        let keys: Vec<String> = (0..=steps)
            .map(|k| {
                let (s, a) = at(k);
                let zoom = ((s * 100.0).round() as u32).max(1);
                // The sprite's centre moves toward the line as it narrows.
                let toward = |line: i32, from: i32, size: i32| ((line as f64 - (from as f64 + size as f64 / 2.0)) * (1.0 - s)).round() as i32;
                let (dx, dy, zx, zy) = match u.y {
                    Some(y) => (0, toward(y, sy, sh), 100, zoom),
                    None => (toward(u.x, sx, sw), 0, zoom, 100),
                };
                format!("{},{dx},{dy},{zx},{zy},0,{},0", total * k / steps, (256.0 * a).round() as u32)
            })
            .collect();
        line.push(format!("amsp {slot},{sx},{sy},255:spzoom {slot},100:spmotion {slot},{mode},{}", keys.join(",")));
        slots.push(slot);
    }
    if let Some((bx, by, o)) = backdrop {
        let keys: Vec<String> = (0..=steps).map(|k| format!("{},0,0,100,100,0,{},0", total * k / steps, (256.0 * at(k).1).round() as u32)).collect();
        line.push(format!("amsp {DIALOG_BASE},{bx},{by},{o}:spmotion {DIALOG_BASE},{mode},{}", keys.join(",")));
        slots.push(DIALOG_BASE);
    }
    for l in line {
        w(l);
    }
    w(format!("print 1:{}", slots.iter().map(|n| format!("spmotionwait {n}")).collect::<Vec<_>>().join(":")));
}
