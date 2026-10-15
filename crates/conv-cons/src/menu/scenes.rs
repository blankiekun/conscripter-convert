//! The link scenes' history bar (最果てのイマ's scene stack).

use super::*;

/// Sprites of the scene list's rows.
pub(super) fn scene_row_sprites() -> std::ops::Range<u32> {
    SCENE_ROW_SPRITE..SCENE_ROW_SPRITE + 2 * SCENE_ROWS
}

/// Draws the top bars: chapter and scene titles, the scene list (if
/// open), the level numbers and the buttons that browse the levels.
pub(super) fn scene_bar_draw(w: &mut impl FnMut(String), top: &TopBars, chapter_text: u32, scene_text: u32, buttons: [u32; 4]) {
    let [back, forward, exit, list] = buttons;
    let mut sprites2 = TOP_SPRITES2;
    for p in [&top.chapter_bar, &top.scene_bar] {
        w(lsp2(sprites2.next().unwrap(), p, None));
    }
    let text = |slot: u32, t: &TextSpot, y: i32, var: &str, colour: &str| {
        format!("strsp {slot},{var},{},{y},{},1,{},{},0,0,0,1{colour}", t.x, t.columns, t.font_size, t.font_size)
    };
    // Four cells (normal, pointer, pressed, disabled); `y` may be an
    // expression.
    let lsp4 = |slot: u32, p: &Placed, y: &str| format!("lsp {slot},\":a/4,0,3;{}\",{},{y}", path(&p.image), p.x);
    w(text(chapter_text, &top.chapter_text, top.chapter_text.y, CHAPTER, ""));
    // The scene: its title at the current depth, or the root's.
    w(format!("mov {SCENE_TMP},\"{}\"", top.root_scene));
    w(format!("mov {TMP},{SCENE_TITLES}+{SCENE_DEPTH}"));
    w(format!("if {SCENE_DEPTH} > 0 mov {SCENE_TMP},${TMP}"));
    w(text(scene_text, &top.scene_text, top.scene_text.y, SCENE_TMP, ""));

    w(lsp4(list, &top.list, &top.list.y.to_string()));
    w(format!("spbtn {list},{BTN_SCENE_LIST}"));
    // The list: a row per level from 1 to the deepest; the current one
    // marked, those that can't be gone to disabled.
    let pitch = top.row_pitch;
    w(format!("mov {SCENE_SHIFT},0"));
    w(format!("if {SCENE_LIST_OPEN} == 0 goto *conv_scene_rows_end"));
    w(format!("mov {SCENE_SHIFT},{SCENE_MAX_DEPTH}*{pitch}"));
    w(format!("if {SCENE_MAX_DEPTH} > {SCENE_ROWS} mov {SCENE_SHIFT},{}", SCENE_ROWS as i32 * pitch));
    for k in 1..=SCENE_ROWS {
        let r = k - 1;
        let (label, button) = (SCENE_ROW_SPRITE + 2 * r, SCENE_ROW_SPRITE + 2 * r + 1);
        let (y, text_y) = (top.row.y + r as i32 * pitch, top.row_text.y + r as i32 * pitch);
        let title = format!("${}", SCENE_TITLES + k);
        let colour = |c: &str| format!(",\"{c}\"");
        w(format!("if {SCENE_MAX_DEPTH} < {k} goto *conv_scene_rows_end"));
        w(format!(
            "if {SCENE_DEPTH} == {k} {}:{}",
            lsp4(button, &top.current_row, &y.to_string()),
            text(label, &top.row_text, text_y, &title, &colour(&top.current_row_color))
        ));
        w(format!(
            "if {SCENE_DEPTH} != {k} {}:{}",
            lsp4(button, &top.row, &y.to_string()),
            text(label, &top.row_text, text_y, &title, &colour(&top.row_color))
        ));
        w(format!("if {SCENE_DEPTH} != {k} && {SCENE_FLOOR} <= {k} spbtn {button},{}", BTN_SCENE_ROW + k as i32));
        w(format!("if {SCENE_FLOOR} > {k} cell {button},3"));
    }
    w("*conv_scene_rows_end".into());

    // The level row, below the list.
    let shifted = |y: i32| format!("{y}+{SCENE_SHIFT}");
    let b = &top.level_box;
    w(format!("lsp2 {},\":a;{}\",{},{},100,100,0", sprites2.next().unwrap(), path(&b.image), b.x + b.w / 2, shifted(b.y + b.h / 2)));
    for (var, (x, y), slot) in [(SCENE_DEPTH, top.depth_at, sprites2.next().unwrap()), (SCENE_MAX_DEPTH, top.max_depth_at, sprites2.next().unwrap())] {
        for (d, image) in top.digits.iter().enumerate() {
            w(format!("if {var} == {d} lsp2 {slot},\":a;{}\",{},{},100,100,0", path(image), x + 4, shifted(y + 6)));
        }
    }
    // Disabled unless: back, a level above that can be gone to; forward, a
    // level below; exit, the root can be gone to.
    let enabled = [
        (back, &top.back, format!("{SCENE_DEPTH} > {SCENE_FLOOR}"), BTN_SCENE_BACK),
        (forward, &top.forward, format!("{SCENE_DEPTH} < {SCENE_MAX_DEPTH}"), BTN_SCENE_FORWARD),
        (exit, &top.exit, format!("{SCENE_DEPTH} > 0 && {SCENE_FLOOR} == 0"), BTN_SCENE_EXIT),
    ];
    for (slot, p, condition, number) in enabled {
        w(format!("{}:cell {slot},3", lsp4(slot, p, &shifted(p.y))));
        w(format!("if {condition} cell {slot},0:spbtn {slot},{number}"));
    }
}

/// The routines behind the scene buttons, and the `loadgosub` routine.
///
/// Going to another level saves the game to the current level's slot
/// (the page the player is on, as a save from the menu would) and loads
/// the target level's. Entering a link scene saves the link page to the
/// current level's slot; the link pages in `lib.rs` keep the rest of the
/// bookkeeping (see [`SCENE_DEPTH`]). The deepest level and the titles are
/// locals and go back with a load, so they are carried over in globals.
///
/// A save from the save menu keeps the slots of the levels that can be
/// gone to (`conv_scene_keep`, with CONscripter's `copysavegame`), and a
/// load puts them back, as the original keeps its scene stack in a save.
/// A save without them (made by an older build) loads at the level saved,
/// whose levels below are dropped and above can't be gone back to.
/// `chapters`: there is a [`ChapterHistory`](conv_core::ir::ChapterHistory),
/// whose entries a load may carry over.
pub(super) fn scene_stack(w: &mut impl FnMut(String), chapters: bool) {
    let titles = || (1..=SCENE_ROWS).map(|k| (SCENE_TITLES + k, SCENE_CARRY + k));
    w("*conv_scene_action".into());
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_LIST} goto *conv_scene_list"));
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_BACK} mov {SCENE_TARGET},{SCENE_DEPTH}-1"));
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_FORWARD} mov {SCENE_TARGET},{SCENE_DEPTH}+1"));
    w(format!("if {BUTTON_VAR} == {BTN_SCENE_EXIT} mov {SCENE_TARGET},0"));
    w(format!("if {BUTTON_VAR} > {BTN_SCENE_ROW} mov {SCENE_TARGET},{BUTTON_VAR}-{BTN_SCENE_ROW}"));
    // Only levels that can be gone to.
    w(format!("if {SCENE_TARGET} < {SCENE_FLOOR} return"));
    w(format!("if {SCENE_TARGET} > {SCENE_MAX_DEPTH} return"));
    w(format!("if {SCENE_TARGET} == {SCENE_DEPTH} return"));
    w(format!("savegame {SCENE_SLOT}+{SCENE_DEPTH}"));
    w(format!("mov {SCENE_LOADING},1:mov {SCENE_CARRY_MAX},{SCENE_MAX_DEPTH}:mov {SCENE_CARRY_FLOOR},{SCENE_FLOOR}"));
    w(titles().map(|(local, carry)| format!("mov ${carry},${local}")).collect::<Vec<_>>().join(":"));
    w(format!("loadgame {SCENE_SLOT}+{SCENE_TARGET}"));
    // Only reached if the slot couldn't be loaded.
    w(format!("mov {SCENE_LOADING},0"));
    w("return".into());

    w("*conv_scene_list".into());
    w(format!("if {SCENE_LIST_OPEN} == 1 mov {SCENE_LIST_OPEN},0:return"));
    w(format!("mov {SCENE_LIST_OPEN},1"));
    w("return".into());

    // SELECTED: the save slot.
    let kept = |slot: &str| format!("{SCENE_KEPT}+{slot}*10+{SLOT}");
    w("*conv_scene_keep".into());
    w(format!("for {SLOT}=0 to {}", SCENE_LEVELS - 1));
    w(format!("delsavegame {}", kept(SELECTED)));
    w(format!(
        "if {SLOT} >= {SCENE_FLOOR} && {SLOT} <= {SCENE_MAX_DEPTH} && {SLOT} != {SCENE_DEPTH} copysavegame {SCENE_SLOT}+{SLOT},{}",
        kept(SELECTED)
    ));
    w("next".into());
    w("return".into());
    w("*conv_scene_drop".into());
    w(format!("for {SLOT}=0 to {}", SCENE_LEVELS - 1));
    w(format!("delsavegame {}", kept(SELECTED)));
    w("next".into());
    w("return".into());

    w(format!("*{SCENE_LOADED}"));
    if chapters {
        w(format!("gosub *{}", crate::chapters::LOADED));
    }
    w(format!("if {SCENE_LOADING} == 1 goto *conv_scene_switched"));
    // A load from the save menu: its levels back, if it kept them.
    w(format!("mov {SCENE_RESTORED},0"));
    w(format!("for {SLOT}=0 to {}", SCENE_LEVELS - 1));
    w(format!("savefileexist {HAS_DATA},{}", kept(SCENE_LOAD_SLOT)));
    w(format!("if {HAS_DATA} == 1 copysavegame {},{SCENE_SLOT}+{SLOT}:mov {SCENE_RESTORED},1", kept(SCENE_LOAD_SLOT)));
    w("next".into());
    w(format!("if {SCENE_RESTORED} == 0 mov {SCENE_MAX_DEPTH},{SCENE_DEPTH}:mov {SCENE_FLOOR},{SCENE_DEPTH}"));
    w("return".into());
    w("*conv_scene_switched".into());
    w(format!("mov {SCENE_LOADING},0:mov {SCENE_MAX_DEPTH},{SCENE_CARRY_MAX}:mov {SCENE_FLOOR},{SCENE_CARRY_FLOOR}"));
    w(titles().map(|(local, carry)| format!("mov ${local},${carry}")).collect::<Vec<_>>().join(":"));
    w("return".into());
}
