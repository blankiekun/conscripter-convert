//! The 立方陣 minigame (`function_magiccube.nss`), ported as a
//! [`TilePuzzle`].
//!
//! The final battle (`md05_025`) names `function_magiccube.nss` as the next
//! file. Its `_MAIN` scene runs the content scene, then goes on to
//! `md05_025end.nss` if `$立方陣正解` is true, else to
//! `md05_025gameover.nss` (which offers a retry; the second failure's
//! retry, `md05_025gameoveraa.nss`, sets `$立方陣簡易フラグ`, the easy
//! mode). The compiler runs `_MAIN` like any story file and turns the call
//! of the content scene into the puzzle, which sets `$立方陣正解` at run
//! time.
//!
//! The puzzle: a 5×5×5 magic cube drawn as five layers. Layers 1-3 are
//! given; the player completes the 18 empty cells of layers 4 and 5 with
//! 18 numbered tiles. The content scene builds every button from these
//! numbers (`CreateChoiceCount`, `CreateChoiceCountM`), checks the answer in
//! `MathAnswer` and fills six cells in the easy mode (`To_LevelDwon`). Its
//! debug button (`Deb`, drawn off screen) solves the puzzle; it isn't
//! ported.

use conv_core::ir::{Placed, PuzzleCell, PuzzleTile, TilePuzzle, Var};

use crate::nitroplus::convert::Assets;

/// The file and the content scene the puzzle replaces.
pub const FILE: &str = "function_magiccube.nss";
/// Set by the puzzle: true when solved.
pub const SOLVED: &str = "$立方陣正解";
/// Set by the second retry: six cells start filled.
pub const EASY: &str = "$立方陣簡易フラグ";

const DIR: &str = "cg/sys/magiccube";
/// The tray: tile values in button order (`Count_1`-`Count_18`), six per
/// column, columns at `$MCSetX1`-`3`, rows at `$MCSetY1`-`6`.
// src: function_magiccube.nss (CreateChoiceCount, $MCSetX1-3, $MCSetY1-6, $マス目X/Y, MathAnswer, To_LevelDwon, To_Manual)
const TILES: [i64; 18] = [62, 49, 8, 55, 84, 24, 9, 105, 22, 35, 60, 74, 41, 96, 101, 46, 99, 124];
const TRAY_X: [i64; 3] = [52, 92, 132];
const TRAY_Y: [i64; 6] = [218, 258, 298, 338, 378, 418];
/// Cell positions (`$マス目X1`-`10`, `$マス目Y1`-`5`) and the cells in
/// `MathCount_1`-`18` order as (x index, y index): layer 5 (left), then
/// layer 4 (right).
const CELL_X: [i64; 10] = [154, 180, 206, 232, 258, 462, 488, 514, 540, 566];
const CELL_Y: [i64; 5] = [98, 113, 128, 143, 158];
const CELLS: [(usize, usize); 18] = [
    (2, 0), (3, 1), (4, 2), (1, 1), (2, 2), (3, 3), (0, 2), (1, 3), (2, 4),
    (7, 0), (8, 1), (9, 2), (6, 1), (7, 2), (8, 3), (5, 2), (6, 3), (7, 4),
];
/// `MathAnswer`'s solution, per cell.
const SOLUTION: [i64; 18] = [41, 124, 84, 99, 24, 60, 46, 22, 101, 105, 8, 96, 9, 62, 74, 55, 49, 35];
/// Cells the easy mode fills (`To_LevelDwon`: `MathCount_3`, 7, 9, 12, 16, 18).
const EASY_CELLS: [usize; 6] = [2, 6, 8, 11, 15, 17];
/// `To_Manual`: text, x, y, yellow. Font 20 px.
const HELP: [(&str, i64, i64, bool); 10] = [
    ("【ゲーム説明】", 50, 50, false),
    ("辰気の調和とは、辰気の強さの均衡を意味する。", 70, 80, false),
    ("従って、この立方体空間に辰気の安定をもたらすのならば", 70, 110, false),
    ("あらゆる直線と、あらゆる対角線が同等の強さを持つよう", 70, 140, false),
    ("に辰気１２５種を配分しなくてはならない。", 70, 170, false),
    ("数字にカーソルを合わせてクリックした後、マスに", 70, 200, true),
    ("カーソルを合わせてクリックすると、選択した数字が", 70, 230, true),
    ("選択したマスに納まります。", 70, 260, true),
    ("全ての数字をマスに納め終わりましたら『解答する』", 70, 290, true),
    ("をクリックしてください。", 70, 320, true),
];

/// The puzzle, with its pictures taken from the game; `None` if one is
/// missing.
pub fn puzzle(assets: &mut dyn Assets, easy: Option<Var>, solved: Var) -> Option<TilePuzzle> {
    let strip = |assets: &mut dyn Assets, cells: &[(&str, i64)]| -> Option<(String, i32, i32)> {
        let mut parts = Vec::new();
        let (mut x, mut size) = (0, (0, 0));
        for (name, opacity) in cells {
            let (path, w, h) = assets.image(&format!("{DIR}/{name}"))?;
            parts.push((path, x, 0, *opacity));
            x += w as i64;
            size = (w as i32, h as i32);
        }
        let (path, _, _) = assets.composite(&parts)?;
        Some((path, size.0, size.1))
    };
    let placed = |s: (String, i32, i32), cells: i32, x: i64, y: i64| Placed { image: s.0, x: x as i32, y: y as i32, w: s.1 * cells, h: s.2 };
    // src: function_magiccube.nss (下敷き, 選択用マス, Answer, State, Manual, mbgm11, Over01's 500 ms blink)
    let backdrop = [("立方陣説明キューブ背景-58.png", 0, 0), ("問題パネル盤.png", 46, 212)]
        .iter()
        .map(|&(name, x, y)| assets.image(&format!("{DIR}/{name}")).map(|(path, w, h)| Placed { image: path, x, y, w: w as i32, h: h as i32 }))
        .collect::<Option<Vec<_>>>()?;
    let mut tiles = Vec::new();
    for (i, v) in TILES.iter().enumerate() {
        let n = |k: u32| format!("パネル数字/{v}-00{k}.png");
        let button = strip(assets, &[(&n(1), 1000), (&n(2), 1000)])?;
        let picked = strip(assets, &[(&n(3), 500), (&n(3), 1000)])?;
        let (in_cell, _, _) = assets.image(&format!("{DIR}/キューブ入れ込み数字/{v}.png"))?;
        tiles.push(PuzzleTile { button: placed(button, 2, TRAY_X[i / 6], TRAY_Y[i % 6]), picked: picked.0, frames: 2, frame_ms: 500, in_cell });
    }
    let cells = CELLS
        .iter()
        .zip(SOLUTION)
        .enumerate()
        .map(|(i, (&(x, y), v))| PuzzleCell { x: CELL_X[x] as i32, y: CELL_Y[y] as i32, solution: TILES.iter().position(|&t| t == v).unwrap(), easy: EASY_CELLS.contains(&i) })
        .collect();
    let (normal, over) = ("問題用キューブボタン-通常.png", "問題用キューブボタン-オーバーキャンセル.png");
    let empty = strip(assets, &[(normal, 1000), (over, 1000)])?;
    // A filled cell shows its number; its button is the cell's shape, all
    // but invisible (buttons are hit by their alpha).
    let filled = strip(assets, &[(normal, 8), (over, 1000)])?;
    let mut button = |name: &str, x: i64, y: i64| strip(assets, &[(&format!("{name}-001.png"), 1000), (&format!("{name}-002.png"), 1000)]).map(|s| placed(s, 2, x, y));
    let answer = button("解答する", 56, 460)?;
    let reset = button("状態を戻す", 174, 224)?;
    let help = button("説明を聞く", 174, 338)?;
    let bgm = assets.sound("sound/bgm/mbgm11");
    Some(TilePuzzle {
        backdrop,
        tiles,
        cells,
        cell_w: empty.1,
        cell_h: empty.2,
        empty_cell: empty.0,
        filled_cell: filled.0,
        answer,
        reset,
        help,
        help_lines: HELP.iter().map(|&(t, x, y, yellow)| (t.to_string(), x as i32, y as i32, if yellow { 0xFFFF00 } else { 0xFFFFFF })).collect(),
        // src: function_magiccube.nss To_Manual (SetFont 20; Fade 500)
        help_font_size: 20,
        easy,
        solved,
        bgm,
        fade_ms: 500,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_solution_uses_every_tile_once_and_the_easy_cells_match_to_level_dwon() {
        let mut used: Vec<usize> = SOLUTION.iter().map(|v| TILES.iter().position(|t| t == v).unwrap()).collect();
        used.sort();
        assert_eq!(used, (0..18).collect::<Vec<_>>());
        // To_LevelDwon: cell 3 = 84, 7 = 46, 9 = 101, 12 = 96, 16 = 55, 18 = 35.
        let easy: Vec<i64> = EASY_CELLS.iter().map(|&c| SOLUTION[c]).collect();
        assert_eq!(easy, [84, 46, 101, 96, 55, 35]);
    }
}
