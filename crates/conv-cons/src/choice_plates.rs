//! A choice whose options are picture buttons ([`ChoicePlates`]): each
//! plate a two-cell button sprite (at rest, under the pointer), its text a
//! string sprite centred over it, in the link words' sprites (a page with
//! links has no choice). A `btnwait` waits for a plate; with a button bar,
//! anything else (a right-click, a key) runs the bar's action and the
//! choice waits again, as link pages do.

use std::fmt::Write;

use conv_core::ir::ChoicePlates;

use crate::{escape, label, menu, path, BUTTON_VAR, LINK_SPRITES};

/// The sprites there are (two an option).
const SPRITES: usize = (*LINK_SPRITES.end() - *LINK_SPRITES.start() + 1) as usize;

/// Writes the choice; `n` keeps its labels apart from other choices'.
pub(crate) fn emit(out: &mut String, options: &[(String, String)], p: &ChoicePlates, n: usize, bar: bool) {
    assert!(2 * options.len() <= SPRITES, "more choice options than link sprites");
    let l = format!("conv_cplate{n}");
    // The text over its plate: lower numbers are drawn on top.
    let text_slot = |i: usize| *LINK_SPRITES.start() + 2 * i as u32;
    let plate_slot = |i: usize| text_slot(i) + 1;
    let fade = if p.fade_ms > 0 { format!("print 10,{}", p.fade_ms) } else { "print 1".to_string() };
    writeln!(out, "textoff").unwrap();
    let mut text_at = Vec::new();
    for (i, (text, _)) in options.iter().enumerate() {
        let y = p.y + p.step * i as i32;
        let text = escape(text);
        // Full-width characters take the font's size, the others half.
        let width: i32 = text.chars().map(|c| if (c as u32) < 0x80 || ('｡'..='ﾟ').contains(&c) { p.font_size as i32 / 2 } else { p.font_size as i32 }).sum();
        let columns = text.chars().count() + 1;
        text_at.push((p.text_x - width / 2, y + p.text_dy));
        writeln!(out, "lsp {},\":a/2,0,3;{}\",{},{y}", plate_slot(i), path(&p.image), p.x).unwrap();
        writeln!(
            out,
            "strsp {},\"{text}\",{},{},{columns},1,{size},{size},0,0,0,1,\"{c}\",\"{c}\"",
            text_slot(i),
            p.text_x - width / 2,
            y + p.text_dy,
            size = p.font_size,
            c = p.colour
        )
        .unwrap();
    }
    writeln!(out, "{fade}").unwrap();
    writeln!(out, "*{l}_wait").unwrap();
    if bar {
        // Clears the buttons: register the plates after it.
        writeln!(out, "gosub *{}", menu::BAR_DRAW).unwrap();
    } else {
        writeln!(out, "btndef clear").unwrap();
    }
    for i in 0..options.len() {
        writeln!(out, "spbtn {},{}", plate_slot(i), i + 1).unwrap();
    }
    writeln!(out, "btnwait {BUTTON_VAR}").unwrap();
    writeln!(out, "if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {} goto *{l}_c", options.len()).unwrap();
    if bar {
        writeln!(out, "gosub *{}", menu::BAR_ACTION).unwrap();
    }
    writeln!(out, "goto *{l}_wait").unwrap();
    writeln!(out, "*{l}_c").unwrap();
    let clear = |i: usize| format!("csp {}:csp {}", text_slot(i), plate_slot(i));
    let Some(shot) = &p.shot else {
        let all: Vec<String> = (0..options.len()).map(clear).collect();
        writeln!(out, "{}:{fade}", all.join(":")).unwrap();
        for (i, (_, target)) in options.iter().enumerate() {
            writeln!(out, "if {BUTTON_VAR} == {} goto *{}", i + 1, label(target)).unwrap();
        }
        return;
    };
    writeln!(out, "getmousepos %0,%1:add %0,{}:add %1,{}", shot.hole_dx, shot.hole_dy).unwrap();
    writeln!(out, "dwave {SHOT_CHANNEL},\"{}\"", path(&shot.sound)).unwrap();
    for i in 0..options.len() {
        writeln!(out, "if {BUTTON_VAR} == {} goto *{l}_s{}", i + 1, i + 1).unwrap();
    }
    for (i, (_, target)) in options.iter().enumerate() {
        let (plate, text, (tx, ty)) = (plate_slot(i), text_slot(i), text_at[i]);
        let py = p.y + p.step * i as i32;
        let hole = path(&shot.hole);
        writeln!(out, "*{l}_s{}", i + 1).unwrap();
        writeln!(out, "cell {plate},1:spcut {plate},\"{hole}\",%0,%1:spcut {text},\"{hole}\",%0,%1:print 1").unwrap();
        writeln!(out, "delay 300").unwrap();
        let others: Vec<String> = (0..options.len()).filter(|&j| j != i).map(clear).collect();
        writeln!(out, "{}:print 10,300", others.join(":")).unwrap();
        writeln!(out, "delay 200").unwrap();
        // About the plate's bottom: spzoom scales about a sprite's centre,
        // so each moves down by (bottom - its centre) × (1 - zoom).
        let bottom = py + p.step;
        let (plate_c, text_c) = (py + p.step / 2, ty + p.font_size as i32 / 2);
        for k in 1..TIP_STEPS {
            let t = k as f64 / TIP_STEPS as f64;
            let z = (std::f64::consts::FRAC_PI_2 * t.powi(3)).cos();
            let alpha = (255.0 * (1.0 - t * TIP_MS as f64 / 1000.0)).round() as i32;
            let shift = |c: i32| ((bottom - c) as f64 * (1.0 - z)).round() as i32;
            let zy = (z * 100.0).round() as i32;
            writeln!(
                out,
                "spzoom {plate},100,{zy}:spzoom {text},100,{zy}:amsp {plate},{},{},{alpha}:amsp {text},{tx},{},{alpha}:print 1:delay {}",
                p.x,
                py + shift(plate_c),
                ty + shift(text_c),
                TIP_MS / TIP_STEPS
            )
            .unwrap();
        }
        writeln!(out, "{}:print 1", clear(i)).unwrap();
        writeln!(out, "goto *{}", label(target)).unwrap();
    }
}

/// The shot's sound: an effect channel the script's own sounds don't use.
const SHOT_CHANNEL: u32 = 10;
/// The chosen plate tips over in this many steps, over this long.
const TIP_STEPS: u32 = 16;
const TIP_MS: u32 = 800;
