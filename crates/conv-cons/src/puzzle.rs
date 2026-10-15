//! [`TilePuzzle`] as NScripter code: a `btnwait` loop over sprite buttons.
//!
//! State, in variables the rest of the script leaves alone (%190-%289,
//! locals with the game's global border at 800):
//! - %190: the tile picked (index + 1), 0 when none;
//! - %200 + c: the tile in cell c (index + 1), 0 when empty;
//! - %230 + c: 1 when cell c was filled by the easy mode (it stays);
//! - %260 + t: the cell tile t is in (index + 1), 0 when in the tray;
//! - $200 + t: the picture of tile t's number inside a cell.
//!
//! Cells and tiles are found through `%%n` (ONScripter reads the variable
//! whose number is in %n), so a tile can go into any cell without code for
//! each pair. `draw` lays out the whole screen from the state and makes
//! the buttons of the current step clickable: tiles in the tray, filled
//! cells and the three buttons with no tile picked; empty cells once one
//! is (the screen dimmed by half under them, the picked tile blinking).
//!
//! Clicks go to the button whose picture is opaque under the pointer
//! (`transbtn`): cells are diamonds whose rectangles overlap. So the
//! first cell of a strip must not be fully transparent where it should
//! be clickable.
//!
//! Sprites, from `base` (lower numbers are drawn on top): help text and
//! its dim, the white-out, numbers in cells, the picked tile, cell
//! buttons, the dim, tray tiles, the three buttons, the backdrop.

use conv_core::ir::{Placed, TilePuzzle};

use crate::{escape, path, BUTTON_VAR};

const PICKED: &str = "%190";
const TMP: &str = "%191";
const TMP2: &str = "%192";
const CELL: u32 = 200;
const FIXED: u32 = 230;
const TILE_CELL: u32 = 260;
/// Also the string variables holding each tile's picture in a cell.
const TILE_IMAGE: u32 = 200;
/// Most cells and tiles (the variable ranges above).
const MAX_ITEMS: usize = 30;
const MAX_HELP_LINES: usize = 16;

// Sprite offsets from the base.
const HELP_TEXT: u32 = 0;
const HELP_DIM: u32 = HELP_TEXT + MAX_HELP_LINES as u32;
const WHITE: u32 = HELP_DIM + 1;
const CELL_NUMBER: u32 = WHITE + 1;
const PICKED_SPRITE: u32 = CELL_NUMBER + MAX_ITEMS as u32;
const CELL_BUTTON: u32 = PICKED_SPRITE + 1;
const DIM: u32 = CELL_BUTTON + MAX_ITEMS as u32;
const TILE: u32 = DIM + 1;
const ANSWER: u32 = TILE + MAX_ITEMS as u32;
const RESET: u32 = ANSWER + 1;
const HELP: u32 = RESET + 1;
const BACKDROP: u32 = HELP + 1;

// Buttons btnwait returns.
const BTN_TILE: usize = 1;
const BTN_CELL: usize = 101;
const BTN_ANSWER: u32 = 201;
const BTN_RESET: u32 = 202;
const BTN_HELP: u32 = 203;

/// Writes the puzzle. `n` keeps its labels apart from other puzzles';
/// `base` is its first sprite; `effect` gives the effect number of a
/// cross-fade of that many milliseconds; `screen` is the screen size.
pub(crate) fn emit(out: &mut String, p: &TilePuzzle, n: usize, base: u32, effect: &dyn Fn(u32) -> String, screen: (u32, u32), border: u32) {
    assert!(p.tiles.len() <= MAX_ITEMS && p.cells.len() <= MAX_ITEMS && p.help_lines.len() <= MAX_HELP_LINES);
    assert!(base + BACKDROP + p.backdrop.len() as u32 <= crate::MAX_SPRITE + 1);
    let l = |s: &str| format!("conv_puzzle{n}_{s}");
    let sp = |o: u32| base + o;
    let fade = effect(p.fade_ms);
    let (sw, sh) = screen;
    let button = |slot: u32, b: &Placed| format!("lsp {slot},\":a/2,0,3;{}\",{},{}", path(&b.image), b.x, b.y);
    let mut w = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };

    w("; tile puzzle".into());
    w("textoff".into());
    w("csp -1".into());
    w(format!("mov {PICKED},0"));
    for c in 0..p.cells.len() as u32 {
        w(format!("mov %{},0:mov %{},0", CELL + c, FIXED + c));
    }
    for (t, tile) in p.tiles.iter().enumerate() {
        w(format!("mov %{},0:mov ${},\"{}\"", TILE_CELL + t as u32, TILE_IMAGE + t as u32, path(&tile.in_cell)));
    }
    if let Some(easy) = p.easy {
        let easy = crate::var_name(easy, border);
        for (c, cell) in p.cells.iter().enumerate().filter(|(_, c)| c.easy) {
            let (c, t) = (c as u32, cell.solution as u32);
            w(format!("if {easy} == 1 mov %{},{}:mov %{},1:mov %{},{}", CELL + c, t + 1, FIXED + c, TILE_CELL + t, c + 1));
        }
    }
    for (i, b) in p.backdrop.iter().enumerate() {
        let slot = sp(BACKDROP) + (p.backdrop.len() - 1 - i) as u32;
        let mode = crate::sprite_mode(&b.image);
        w(format!("lsp {slot},\":{mode};{}\",{},{}", path(&b.image), b.x, b.y));
    }
    w(button(sp(ANSWER), &p.answer));
    w(button(sp(RESET), &p.reset));
    w(button(sp(HELP), &p.help));
    if let Some(bgm) = &p.bgm {
        w(format!("bgmfadein 0:bgm \"{}\"", path(bgm)));
    }
    w(format!("gosub *{}", l("draw")));
    w(format!("print {fade}"));

    // The loop: what the click means depends on whether a tile is picked.
    // btnwait forgets its buttons once it returns, so every way back to it
    // draws the screen again, which defines them anew.
    w(format!("goto *{}", l("loop")));
    w(format!("*{}", l("redraw")));
    w(format!("gosub *{}:print 1", l("draw")));
    w(format!("*{}", l("loop")));
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {PICKED} > 0 goto *{}", l("picked")));
    let t_last = BTN_TILE + p.tiles.len() - 1;
    w(format!("if {BUTTON_VAR} >= {BTN_TILE} && {BUTTON_VAR} <= {t_last} mov {PICKED},{BUTTON_VAR}:gosub *{}:print {fade}:goto *{}", l("draw"), l("loop")));
    let c_last = BTN_CELL + p.cells.len() - 1;
    // A filled cell: its tile goes back to the tray.
    w(format!("if {BUTTON_VAR} >= {BTN_CELL} && {BUTTON_VAR} <= {c_last} mov {TMP},{BUTTON_VAR}:add {TMP},{}:gosub *{}:gosub *{}:print 1:goto *{}", CELL as usize - BTN_CELL, l("empty"), l("draw"), l("loop")));
    w(format!("if {BUTTON_VAR} == {BTN_ANSWER} goto *{}", l("answer")));
    if !p.cells.is_empty() {
        w(format!("if {BUTTON_VAR} == {BTN_RESET} mov {TMP},{CELL}:gosub *{}:gosub *{}:print 1", l("reset"), l("draw")));
    }
    w(format!("if {BUTTON_VAR} == {BTN_HELP} gosub *{}", l("help")));
    w(format!("goto *{}", l("redraw")));

    // A tile is picked: an empty cell takes it, a right-click puts it back.
    w(format!("*{}", l("picked")));
    w(format!("if {BUTTON_VAR} == -1 mov {PICKED},0:gosub *{}:print {fade}:goto *{}", l("draw"), l("loop")));
    w(format!("if {BUTTON_VAR} < {BTN_CELL} || {BUTTON_VAR} > {c_last} goto *{}", l("redraw")));
    // Cell c (button 101 + c) holds the tile, the tile is in cell c.
    w(format!("mov {TMP},{BUTTON_VAR}:add {TMP},{}:mov %{TMP},{PICKED}", CELL as usize - BTN_CELL));
    w(format!("mov {TMP},{PICKED}:add {TMP},{}:mov %{TMP},{BUTTON_VAR}:sub %{TMP},{}", TILE_CELL - 1, BTN_CELL - 1));
    w(format!("mov {PICKED},0:gosub *{}:print {fade}:goto *{}", l("draw"), l("loop")));

    // Empties the cell whose variable number is in TMP (unless the easy
    // mode filled it), sending its tile back to the tray. Keeps TMP.
    w(format!("*{}", l("empty")));
    w(format!("if %{TMP} == 0 return"));
    w(format!("mov {TMP2},{TMP}:add {TMP2},{}", FIXED - CELL));
    w(format!("if %{TMP2} == 1 return"));
    w(format!("mov {TMP2},%{TMP}:add {TMP2},{}:mov %{TMP2},0", TILE_CELL - 1));
    w(format!("mov %{TMP},0"));
    w("return".into());

    // Empties every cell from the one in TMP on.
    w(format!("*{}", l("reset")));
    w(format!("gosub *{}", l("empty")));
    w(format!("add {TMP},1"));
    w(format!("if {TMP} < {} goto *{}", CELL + p.cells.len() as u32, l("reset")));
    w("return".into());

    // Draws the screen from the state.
    w(format!("*{}", l("draw")));
    // Buttons are hit by their pictures' alpha, not their rectangles (the
    // cells overlap their neighbours'); btndef turns it off again.
    w("btndef clear:transbtn".into());
    w(format!("csp {}:csp {}", sp(DIM), sp(PICKED_SPRITE)));
    for (t, tile) in p.tiles.iter().enumerate() {
        let (slot, v) = (sp(TILE) + t as u32, TILE_CELL + t as u32);
        w(format!("csp {slot}"));
        w(format!("if %{v} == 0 && {PICKED} != {} {}", t + 1, button(slot, &tile.button)));
        w(format!("if %{v} == 0 && {PICKED} == 0 spbtn {slot},{}", BTN_TILE + t));
        w(format!("if {PICKED} == {} lsp {},\":a/{},{},0;{}\",{},{}", t + 1, sp(PICKED_SPRITE), tile.frames, tile.frame_ms, path(&tile.picked), tile.button.x, tile.button.y));
    }
    for (c, cell) in p.cells.iter().enumerate() {
        let (number, slot, v, fixed) = (sp(CELL_NUMBER) + c as u32, sp(CELL_BUTTON) + c as u32, CELL + c as u32, FIXED + c as u32);
        w(format!("csp {number}:csp {slot}"));
        w(format!("if %{v} > 0 mov {TMP},%{v}:add {TMP},{}:lsp {number},\":a;\"+${TMP},{},{}", TILE_IMAGE - 1, cell.x, cell.y));
        let filled = Placed { image: p.filled_cell.clone(), x: cell.x, y: cell.y, w: 2 * p.cell_w, h: p.cell_h };
        let empty = Placed { image: p.empty_cell.clone(), ..filled.clone() };
        w(format!("if {PICKED} == 0 && %{v} > 0 && %{fixed} == 0 {}:spbtn {slot},{}", button(slot, &filled), BTN_CELL + c));
        w(format!("if {PICKED} > 0 && %{v} == 0 {}:spbtn {slot},{}", button(slot, &empty), BTN_CELL + c));
    }
    w(format!("if {PICKED} > 0 lsp {},\":c;>{sw},{sh},#000000\",0,0,128", sp(DIM)));
    w(format!("if {PICKED} == 0 spbtn {},{BTN_ANSWER}:spbtn {},{BTN_RESET}:spbtn {},{BTN_HELP}", sp(ANSWER), sp(RESET), sp(HELP)));
    w("return".into());

    // The rules, until a click.
    w(format!("*{}", l("help")));
    w(format!("lsp {},\":c;>{sw},{sh},#000000\",0,0,128", sp(HELP_DIM)));
    for (i, (text, x, y, colour)) in p.help_lines.iter().enumerate() {
        let size = p.help_font_size;
        w(format!("lsp {},\":s/{size},{size},0;#{colour:06X}{}\",{x},{y}", sp(HELP_TEXT) + i as u32, escape(text)));
    }
    w(format!("print {fade}"));
    w("click".into());
    for i in 0..p.help_lines.len() as u32 {
        w(format!("csp {}", sp(HELP_TEXT) + i));
    }
    w(format!("csp {}", sp(HELP_DIM)));
    w(format!("print {fade}"));
    w("return".into());

    // The answer: white out, check, clear to black.
    w(format!("*{}", l("answer")));
    w(format!("mov {TMP},1"));
    for (c, cell) in p.cells.iter().enumerate() {
        w(format!("if %{} != {} mov {TMP},0", CELL + c as u32, cell.solution + 1));
    }
    w(format!("mov {},{TMP}", crate::var_name(p.solved, border)));
    w(format!("lsp {},\":c;>{sw},{sh},#FFFFFF\",0,0", sp(WHITE)));
    if p.bgm.is_some() {
        w(format!("bgmfadeout {}:bgmstop", p.fade_ms));
    }
    w(format!("print {fade}"));
    w("btndef clear".into());
    w("csp -1".into());
    w("bg \"#000000\",0".into());
    w(format!("print {fade}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{PuzzleCell, PuzzleTile, Var};

    fn placed(image: &str, x: i32) -> Placed {
        Placed { image: image.into(), x, y: 10, w: 72, h: 36 }
    }

    pub(crate) fn puzzle() -> TilePuzzle {
        let tile = |v: u32, x: i32| PuzzleTile { button: placed(&format!("p/{v}.png"), x), picked: format!("p/{v}s.png"), frames: 2, frame_ms: 500, in_cell: format!("c/{v}.png") };
        TilePuzzle {
            backdrop: vec![placed("bg.jpg", 0)],
            tiles: vec![tile(7, 0), tile(9, 40)],
            cells: vec![PuzzleCell { x: 100, y: 20, solution: 1, easy: true }, PuzzleCell { x: 130, y: 20, solution: 0, easy: false }],
            cell_w: 52,
            cell_h: 30,
            empty_cell: "c/empty.png".into(),
            filled_cell: "c/filled.png".into(),
            answer: placed("a.png", 0),
            reset: placed("r.png", 0),
            help: placed("h.png", 0),
            help_lines: vec![("説明".into(), 50, 50, 0xFFFF00)],
            help_font_size: 20,
            easy: Some(Var { global: false, index: 290 }),
            solved: Var { global: false, index: 291 },
            bgm: Some("bgm/m11.ogg".into()),
            fade_ms: 500,
        }
    }

    #[test]
    fn the_answer_compares_every_cell_with_its_solution() {
        let mut out = String::new();
        emit(&mut out, &puzzle(), 0, 62, &|ms| if ms == 0 { "1".into() } else { "2".into() }, (1024, 576), 800);
        assert!(out.contains("if %200 != 2 mov %191,0\nif %201 != 1 mov %191,0\nmov %301,%191\n"), "{out}");
        // The easy mode fills cell 0 with tile 1 and fixes it.
        assert!(out.contains("if %300 == 1 mov %200,2:mov %230,1:mov %261,1\n"), "{out}");
        assert!(out.contains("mov $200,\"c\\7.png\""), "{out}");
    }
}
