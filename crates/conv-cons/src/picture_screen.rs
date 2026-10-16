//! [`PictureScreen`] as NScripter code: one sprite per layer in the menus'
//! slots (over the scene and its text window, under the yes/no dialog), a
//! picture with a condition behind an `if`, then a `btnwait` with no
//! buttons for the click. The scene's own sprites stay as they are.

use conv_core::ir::PictureScreen;

use crate::{menu, menu_slot, menu_z, path, var_name, BUTTON_VAR, MENU_SLOTS};

/// Writes the screen. `n` keeps its labels apart from other screens';
/// `fade` is the effect of its fades; `bar`: the script has a button bar
/// to hide; `border`: the first global variable.
pub(crate) fn emit(out: &mut String, s: &PictureScreen, n: usize, fade: &str, bar: bool, border: u32) {
    let mut layers: Vec<u32> = s.pictures.iter().map(|p| p.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    assert!(layers.len() <= MENU_SLOTS, "a picture screen has more layers than the menus' sprites");
    // Lower numbers are drawn on top: the highest layer first.
    let slot = |layer: u32| menu_slot(layers.len() - 1 - layers.iter().position(|&l| l == layer).unwrap());
    let mut w = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    w("; picture screen".into());
    if bar {
        w(format!("gosub *{}", menu::BAR_HIDE));
    }
    for p in &s.pictures {
        let image = &p.picture.image;
        let mode = crate::sprite_mode(image);
        let lsp = format!("lsp {0},\":{mode};{1}\",{2},{3}{4}", slot(p.layer), path(image), p.picture.x, p.picture.y, menu_z(slot(p.layer)));
        match p.when {
            Some((var, cmp, value)) => w(format!("if {} {} {value} {lsp}", var_name(var, border), cmp.operator())),
            None => w(lsp),
        }
    }
    w(format!("print {fade}"));
    // A click or a right-click closes it.
    w(format!("*conv_pscreen{n}"));
    w("btndef clear".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} != 0 && {BUTTON_VAR} != -1 goto *conv_pscreen{n}"));
    w(layers.iter().map(|&l| format!("csp {}", slot(l))).collect::<Vec<_>>().join(":"));
    w(format!("print {fade}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{Compare, Placed, ScreenPicture, Var};

    #[test]
    fn layers_from_the_top_and_conditions_behind_ifs() {
        let picture = |image: &str, x| Placed { image: format!("cg/{image}.png"), x, y: 10, w: 48, h: 58 };
        let flag = Var { global: false, index: 300 };
        let s = PictureScreen {
            pictures: vec![
                ScreenPicture { layer: 0, picture: picture("back", 0), when: None },
                ScreenPicture { layer: 1, picture: picture("alive", 385), when: None },
                ScreenPicture { layer: 1, picture: picture("dead", 385), when: Some((flag, Compare::Ne, 0)) },
            ],
            fade_ms: 300,
        };
        let mut out = String::new();
        emit(&mut out, &s, 0, "11", true, 800);
        assert!(out.contains("; picture screen\ngosub *conv_bar_hide_all\nlsp 7,\":a;cg\\back.png\",0,10\nlsp 6,\":a;cg\\alive.png\",385,10\nif %310 != 0 lsp 6,\":a;cg\\dead.png\",385,10\nprint 11\n"), "{out}");
        assert!(out.contains("btnwait %9\nif %9 != 0 && %9 != -1 goto *conv_pscreen0\ncsp 7:csp 6\nprint 11\n"), "{out}");
    }
}
