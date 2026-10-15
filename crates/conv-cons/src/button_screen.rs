//! [`ButtonScreen`] as NScripter code: sprites in the menus' slots (over
//! the scene, under the yes/no dialog) and a `btnwait` loop over the
//! buttons. The scene's own sprites stay as they are, so fading the
//! screen's sprites out shows the scene again.

use conv_core::ir::{ButtonScreen, Placed};

use crate::{label, menu, path, BUTTON_VAR, MENU_SPRITES};

/// Writes the screen. `n` keeps its labels apart from other screens';
/// `fade` is the effect number of its fades; `bar`: the script has a
/// button bar to hide.
pub(crate) fn emit(out: &mut String, s: &ButtonScreen, n: usize, fade: &str, bar: bool) {
    let count = (s.pictures.len() + s.buttons.len()) as u32;
    assert!(count <= MENU_SPRITES.len() as u32, "a button screen has more pictures and buttons than the menus' sprites");
    // Lower numbers are drawn on top: the buttons, then the pictures from
    // the top down.
    let slot = |i: usize| MENU_SPRITES.start + i as u32;
    let picture_slot = |i: usize| slot(s.buttons.len() + s.pictures.len() - 1 - i);
    let l = format!("conv_bscreen{n}");
    let mut w = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    w("; button screen".into());
    w("textoff".into());
    if bar {
        w(format!("gosub *{}", menu::BAR_HIDE));
    }
    for (i, p) in s.pictures.iter().enumerate() {
        let mode = crate::sprite_mode(&p.image);
        w(format!("lsp {},\":{mode};{}\",{},{}", picture_slot(i), path(&p.image), p.x, p.y));
    }
    for (i, (b, _)) in s.buttons.iter().enumerate() {
        w(button(slot(i), b));
    }
    w(format!("print {fade}"));
    // btnwait forgets its buttons once it returns: they are made again
    // every time round.
    w(format!("*{l}"));
    w("btndef clear".into());
    for i in 0..s.buttons.len() {
        w(format!("spbtn {},{}", slot(i), i + 1));
    }
    w(format!("btnwait {BUTTON_VAR}"));
    let clear: Vec<String> = (0..count as usize).map(|i| format!("csp {}", slot(i))).collect();
    for (i, (_, target)) in s.buttons.iter().enumerate() {
        w(format!("if {BUTTON_VAR} == {} {}:print {fade}:goto *{}", i + 1, clear.join(":"), label(target)));
    }
    w(format!("goto *{l}"));
}

fn button(slot: u32, b: &Placed) -> String {
    format!("lsp {slot},\":a/2,0,3;{}\",{},{}", path(&b.image), b.x, b.y)
}
