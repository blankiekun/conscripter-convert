//! The favour screen (`sys_hmode.nss`, `judgment_of_count`), ported: the
//! heroines' icons, the black box over 村正 until she is known, and a
//! column of gauges per heroine as high as her route counter. The NSS
//! builds it with loops over arrays and counters known only at run time,
//! which the compiler can't follow (after a runtime `if` it shows the first
//! branch's pictures); here it is an [`Op::PictureScreen`] whose pictures
//! each carry their runtime test, drawn over the story as the original's is.
//!
//! Left out from the story's visit: the new gauges blinking in, and a
//! heroine's icon being struck out (with its sound) the first time she is
//! dead; the screen shows how things stand.

use conv_core::ir::{Compare, Op, Placed, PictureScreen, ScreenPicture, Var};

use super::super::{Compiler, Value};
use super::{hex_counter, hex_flag, HEX};

const DIR: &str = "cg/sys/Fav";
/// Icons (`ArrayHex`): x of each heroine's icon at y 371 and of her mask
/// (the dead heroine's column cover) at y 93, in `HEX` order.
// src: sys_hmode.nss ArrayHex ($H_Name[…][1], [2]); y 371, 93: judgment_of_count's CreateTexture
const ICON_X: [i32; 4] = [385, 453, 521, 589];
const MASK_X: [i32; 4] = [389, 457, 525, 594];
/// Gauges (`CreateHex`): the first at y 333, each next one 30 px higher,
/// columns 68 px apart from x 389.
// src: sys_hmode.nss CreateHex ($H_x 389 += 68, $H_y 333 -= 30); RouteChicker's 10; H_video2's Fade 300
const GAUGE_X: i32 = 389;
const GAUGE_DX: i32 = 68;
const GAUGE_Y: i32 = 333;
const GAUGE_DY: i32 = 30;
/// Route counters count to 10 (`RouteChicker`).
const MAX_GAUGES: i32 = 10;
/// Its fades in and out (`Fade("H_video2", 300, …)`).
const FADE_MS: u32 = 300;
/// Layers, bottom up: the background, gauges, icons, masks, 村正's box.
// ours
const BACK: u32 = 0;
const GAUGES: u32 = 1;
const ICONS: u32 = 5;
const MASKS: u32 = 9;
const HIDDEN: u32 = 13;

impl Compiler<'_, '_> {
    /// `judgment_of_count`: the screen until a click, then `SetHex`.
    pub(super) fn favour_screen(&mut self) -> bool {
        self.sync(0);
        let Some(background) = self.picture(&format!("{DIR}/好感度画像背景.png"), 0, 0) else {
            self.note("favour screen: a picture is missing");
            return false;
        };
        let mut pictures = vec![ScreenPicture { layer: BACK, picture: background, when: None }];
        let mut show = |c: &mut Self, layer, picture: Option<Placed>, when: Result<Option<(Var, Compare, i32)>, bool>| {
            // A test known now is kept or left out here.
            match (picture, when) {
                (Some(picture), Ok(when)) => pictures.push(ScreenPicture { layer, picture, when }),
                (Some(picture), Err(true)) => pictures.push(ScreenPicture { layer, picture, when: None }),
                (None, _) => c.note("favour screen: a picture is missing"),
                _ => {}
            }
        };
        for (i, &(name, jp)) in HEX.iter().enumerate() {
            let layer = i as u32;
            let dead = self.test(&hex_flag(name, "死亡"), Compare::Ne, 0);
            let alive = self.picture(&format!("{DIR}/{jp}_001.png"), ICON_X[i], 371);
            show(self, ICONS + layer, alive, Ok(None));
            let struck = self.picture(&format!("{DIR}/{jp}_002.png"), ICON_X[i], 371);
            show(self, ICONS + layer, struck, dead);
            let mask = self.picture(&format!("{DIR}/ゲージマスクicon.png"), MASK_X[i], 93);
            show(self, MASKS + layer, mask, dead);
            // The gauges as one picture per count, chosen by the counter.
            let Some((gauge, w, h)) = self.assets.image(&format!("{DIR}/ゲージicon.png")) else { continue };
            let counter = hex_counter(name);
            for k in 1..=MAX_GAUGES {
                let parts: Vec<(String, i64, i64, i64)> = (0..k).map(|j| (gauge.clone(), 0, -(GAUGE_DY * j) as i64, 1000)).collect();
                let column = self.assets.composite(&parts).map(|(image, _, top)| {
                    let (x, y) = (GAUGE_X + GAUGE_DX * i as i32, GAUGE_Y + top as i32);
                    Placed { image, x, y, w: w as i32, h: h as i32 + GAUGE_DY * (k - 1) }
                });
                // The last picture also stands for higher counts.
                let cmp = if k == MAX_GAUGES { Compare::Ge } else { Compare::Eq };
                let when = self.test(&counter, cmp, k);
                show(self, GAUGES + layer, column, when);
            }
        }
        // 村正 is hidden until her counter moves (`村正隠蔽`).
        let unknown = self.test(&hex_counter("Muramasa"), Compare::Eq, 0);
        // src: sys_hmode.nss 村正隠蔽 (CreateColor 48×58 at her icon)
        let black = self.assets.color(0x000000, 48, 58);
        show(self, HIDDEN, Some(Placed { image: black, x: ICON_X[1], y: 371, w: 48, h: 58 }), unknown);
        let mask = self.picture(&format!("{DIR}/ゲージマスクicon.png"), MASK_X[1], 93);
        show(self, HIDDEN + 1, mask, unknown);
        self.ops.push(Op::PictureScreen(Box::new(PictureScreen { pictures, fade_ms: FADE_MS })));
        self.set_hex();
        true
    }

    /// A picture the scripts name, placed at (x, y).
    fn picture(&mut self, path: &str, x: i32, y: i32) -> Option<Placed> {
        let (image, w, h) = self.assets.image(path)?;
        Some(Placed { image, x, y, w: w as i32, h: h as i32 })
    }

    /// The story flag `key` compared with `value`: a runtime test, or
    /// `Err` with the answer when the value is known now.
    fn test(&mut self, key: &str, cmp: Compare, value: i32) -> Result<Option<(Var, Compare, i32)>, bool> {
        match self.flags.get(key) {
            Some(&var) => Ok(Some((var, cmp, value))),
            None => {
                let v = self.vars.get(key).map_or(0, Value::int) as i32;
                Err(match cmp {
                    Compare::Eq => v == value,
                    Compare::Ne => v != value,
                    Compare::Lt => v < value,
                    Compare::Le => v <= value,
                    Compare::Gt => v > value,
                    Compare::Ge => v >= value,
                })
            }
        }
    }
}
