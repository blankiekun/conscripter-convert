//! A choice whose options are picture buttons ([`ChoicePlates`]): each
//! plate a two-cell button sprite (at rest, under the pointer), its text a
//! string sprite centred over it, in the link words' sprites (a page with
//! links has no choice). A `btnwait` waits for a plate; with a button bar,
//! anything else (a right-click, a key) runs the bar's action and the
//! choice waits again, as link pages do.

use std::fmt::Write;

use conv_core::ir::{ChoicePlates, RuntimeText};

use crate::strtab::PLATE;
use crate::{escape, label, menu, path, var_name, BUTTON_VAR, LINK_SPRITE_COUNT};

/// The sprites there are (two an option).
const SPRITES: usize = LINK_SPRITE_COUNT;

/// Writes the choice; `n` keeps its labels apart from other choices'.
/// `texts[i]`, when given, picks option i's text at run time (centred by
/// a variable set with it).
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit(out: &mut String, strings: &mut crate::strtab::Strings, options: &[(String, String)], texts: &[Option<RuntimeText>], p: &ChoicePlates, n: usize, bar: bool, border: u32) {
    assert!(2 * options.len() <= SPRITES, "more choice options than link sprites");
    let l = format!("conv_cplate{n}");
    // The text over its plate: lower handles are drawn on top.
    let text_slot = |i: usize| crate::link_sprite(2 * i);
    let plate_slot = |i: usize| crate::link_sprite(2 * i + 1);
    let fade = if p.fade_ms > 0 { format!("print 10,{}", p.fade_ms) } else { "print 1".to_string() };
    writeln!(out, "textoff").unwrap();
    let mut text_at = Vec::new();
    // Full-width characters take the font's size, the others half.
    let width = |text: &str| -> i32 { text.chars().map(|c| if (c as u32) < 0x80 || ('｡'..='ﾟ').contains(&c) { p.font_size as i32 / 2 } else { p.font_size as i32 }).sum() };
    assert!(!(p.vertical && p.shot.is_some()), "a shot on vertical plates");
    // The plate's place, and the text's left (by its width) and top.
    let plate_at = |i: usize| if p.vertical { (p.x - p.step * i as i32, p.y) } else { (p.x, p.y + p.step * i as i32) };
    let text_left = |i: usize, text: &str| if p.vertical { p.text_x - p.step * i as i32 } else { p.text_x - width(text) / 2 };
    let cells = if p.chosen_ms > 0 { 3 } else { 2 };
    let strsp = if p.vertical { "vstrsp" } else { "strsp" };
    for (i, (text, _)) in options.iter().enumerate() {
        let (px, y) = plate_at(i);
        let text = escape(text);
        let (text, x, columns) = match texts.get(i).and_then(Option::as_ref) {
            Some(rt) => {
                let (s, x) = crate::runtime_option(i);
                writeln!(out, "{}", strings.lines(PLATE, &format!("mov {s},\"{text}\":mov {x},{}", text_left(i, &text)))).unwrap();
                let mut columns = text.chars().count();
                for (value, t) in &rt.texts {
                    let t = escape(t);
                    columns = columns.max(t.chars().count());
                    let line = format!("if {} == {value} mov {s},\"{t}\":mov {x},{}", var_name(rt.var, border), text_left(i, &t));
                    writeln!(out, "{}", strings.lines(PLATE, &line)).unwrap();
                }
                (s, x, columns + 1)
            }
            None => (format!("\"{text}\""), text_left(i, &text).to_string(), text.chars().count() + 1),
        };
        text_at.push((x.clone(), y + p.text_dy));
        writeln!(out, "lsp {},\":a/{cells},0,3;{}\",{px},{y}", plate_slot(i), path(&p.image)).unwrap();
        let (cols, rows) = if p.vertical { (1, columns) } else { (columns, 1) };
        let line = format!("{strsp} {},{text},{x},{},{cols},{rows},{size},{size},0,0,0,1,\"{c}\",\"{c}\"", text_slot(i), y + p.text_dy, size = p.font_size, c = p.colour);
        // placed by its width: the whole line is the language's
        let line = if texts.get(i).is_some_and(Option::is_some) { line } else { strings.lines(PLATE, &line) };
        writeln!(out, "{line}").unwrap();
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
        if p.chosen_ms > 0 {
            for i in 0..options.len() {
                writeln!(out, "if {BUTTON_VAR} == {} cell {},2", i + 1, plate_slot(i)).unwrap();
            }
            writeln!(out, "print 1:delay {}", p.chosen_ms).unwrap();
        }
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
        let (plate, text, (tx, ty)) = (plate_slot(i), text_slot(i), (&text_at[i].0, text_at[i].1));
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
