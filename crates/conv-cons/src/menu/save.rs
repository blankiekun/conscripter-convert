//! The save and load menu ([`SaveMenu`]).

use super::*;

/// `scene_stack`: the game has link scene levels ([`scene_stack`]), whose
/// slots a save keeps.
pub(crate) fn save_menu(out: &mut String, m: &SaveMenu, confirm: Option<&ConfirmDialog>, scene_stack: bool) {
    let rows = (m.rows as usize).min(MAX_ROWS);
    let pages = &m.pages[..m.pages.len().min(MAX_PAGES)];
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let button = |slot: u32, p: &Placed| format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&p.image), p.x, p.y);
    let row_y = |row: usize| m.row_y + row as i32 * m.row_pitch;
    let thumb_centre = |row: usize| (m.thumbnail.x + m.thumbnail.w / 2, row_y(row) + m.thumbnail.y + m.thumbnail.h / 2);
    let scale = m.thumbnail.w * 100 / m.preview.w;

    w(format!("*{SAVE}"));
    w(format!("mov {MODE},1"));
    w("goto *conv_sl".into());
    w(format!("*{LOAD}"));
    w(format!("mov {MODE},2"));
    w("*conv_sl".into());
    // The thumbnail: the screen as it is before the menu covers it.
    w(format!("if {MODE} == 1 getscreenshot {},{}", m.preview.w, m.preview.h));
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    let backdrop_on = m.backdrop.map(|(slot, f)| format!("screenfilter {slot},{}", f.map(|n| n.to_string()).join(",")));
    let backdrop_off = m.backdrop.map(|(slot, _)| format!("screenfilter {slot},0"));
    if let Some(on) = &backdrop_on {
        w(format!("if {IN_GAME} == 1 {on}"));
    }
    w(lsp2(BASE, &m.base, Some(m.base_opacity)));
    w(lsp2(FRAME, &m.frame, None));
    w(format!("if {MODE} == 1 {}", lsp2(HEADER, &m.save_header, None)));
    w(format!("if {MODE} == 2 {}", lsp2(HEADER, &m.load_header, None)));
    w(button(DELETE, &m.delete));
    w(format!("if {MODE} == 1 {}", button(ACTION, &m.save)));
    w(format!("if {MODE} == 2 {}", button(ACTION, &m.load)));
    w(button(CLOSE, &m.close));
    w(format!("if {PAGE} < 1 mov {PAGE},1"));
    w(format!("if {PAGE} > {} mov {PAGE},1", pages.len()));

    // A page: its rows and the page buttons.
    w("*conv_sl_page".into());
    w(format!("mov {SELECTED},0"));
    w(format!("csp2 {SELECTION}:csp2 {PREVIEW}:csp2 {NEW}:csp2 {MARKER}"));
    w(format!("mov {FIRST},{PAGE}*{rows}-{}", rows - 1));
    for row in 0..rows {
        let y = row_y(row);
        w(format!("mov {SLOT},{FIRST}+{row}"));
        w("gosub *conv_sl_slot".into());
        let size = m.font_size;
        let cols = if m.text_width > 0 { (m.text_width / size.max(1)).max(1) } else { 40 };
        for (slot, line, dy) in [(TEXT1, LINE1, 0), (TEXT2, LINE2, m.line_height)] {
            w(format!(
                "strsp {},{line},{},{},{cols},1,{size},{size},0,0,0,1",
                slot + row as u32,
                m.text_x,
                y + m.text_y + dy
            ));
        }
        w(format!("lsp {},\":a/2,0,3;{}\",{},{}", ROW_BUTTON + row as u32, path(&m.row_hover.image), m.row_hover.x, y + m.row_hover.y));
        let (cx, cy) = thumb_centre(row);
        w(format!("if {HAS_DATA} == 0 csp2 {}", THUMB + row as u32));
        w(format!("if {HAS_DATA} == 1 lsp2 {},{THUMB_IMAGE},{cx},{cy},{scale},{scale},0", THUMB + row as u32));
        let icon = &m.new_icon;
        w(format!(
            "if {HAS_DATA} == 1 && {SLOT} == {NEWEST} {}",
            lsp2(NEW, &Placed { y: y + icon.y, ..icon.clone() }, None)
        ));
    }
    for (i, p) in pages.iter().enumerate() {
        let slot = PAGE_BUTTON + i as u32;
        w(button(slot, &p.button));
        let marker = Placed { x: p.marker_x, y: p.marker_y, ..m.page_marker.clone() };
        w(format!("if {PAGE} == {} cell {slot},2:{}", i + 1, lsp2(MARKER, &marker, None)));
    }

    // Enable the buttons that apply to the selection.
    w("*conv_sl_buttons".into());
    w(format!("cell {DELETE},2:cell {ACTION},2"));
    w(format!("if {SELECTED} > 0 && {SELECTED_HAS_DATA} == 1 cell {DELETE},0:cell {ACTION},0"));
    w(format!("if {SELECTED} > 0 && {MODE} == 1 cell {ACTION},0"));
    w("*conv_sl_wait".into());
    for row in 0..rows {
        w(format!("spbtn {},{}", ROW_BUTTON + row as u32, row + 1));
    }
    for i in 0..pages.len() {
        w(format!("if {PAGE} != {} spbtn {},{}", i + 1, PAGE_BUTTON + i as u32, BTN_PAGE + i as u32 + 1));
    }
    w(format!("if {SELECTED} > 0 && {SELECTED_HAS_DATA} == 1 spbtn {DELETE},{BTN_DELETE}:spbtn {ACTION},{BTN_ACTION}"));
    w(format!("if {SELECTED} > 0 && {MODE} == 1 spbtn {ACTION},{BTN_ACTION}"));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    // Right-click closes, as in the original.
    w(format!("if {BUTTON_VAR} == -1 goto *conv_sl_close"));
    w(format!("if {BUTTON_VAR} == {BTN_CLOSE} goto *conv_sl_close"));
    w(format!("if {BUTTON_VAR} == {BTN_DELETE} goto *conv_sl_delete"));
    w(format!("if {BUTTON_VAR} == {BTN_ACTION} goto *conv_sl_act"));
    w(format!(
        "if {BUTTON_VAR} > {BTN_PAGE} && {BUTTON_VAR} <= {} mov {PAGE},{BUTTON_VAR}-{BTN_PAGE}:goto *conv_sl_page",
        BTN_PAGE + pages.len() as u32
    ));
    w(format!("if {BUTTON_VAR} >= 1 && {BUTTON_VAR} <= {rows} goto *conv_sl_row"));
    w("goto *conv_sl_wait".into());

    // A row was clicked: select it, or act on it if it already was.
    w("*conv_sl_row".into());
    w(format!("mov {SLOT},{FIRST}+{BUTTON_VAR}-1"));
    w(format!("if {SLOT} == {SELECTED} goto *conv_sl_act"));
    w(format!("mov {SELECTED},{SLOT}"));
    w("gosub *conv_sl_slot".into());
    w(format!("mov {SELECTED_HAS_DATA},{HAS_DATA}"));
    let sel = &m.row_selected;
    w(format!("mov {TMP},({BUTTON_VAR}-1)*{}+{}", m.row_pitch, m.row_y + sel.y + sel.h / 2));
    w(format!("lsp2add {SELECTION},\":c;{}\",{},{TMP},100,100,0", path(&sel.image), sel.x + sel.w / 2));
    let (px, py) = (m.preview.x + m.preview.w / 2, m.preview.y + m.preview.h / 2);
    // (Switched off, no picture: the flag is 0 in a game without the switch.)
    w(format!("if {HAS_DATA} == 1 && {} == 0 lsp2 {PREVIEW},{THUMB_IMAGE},{px},{py},100,100,0", switch_off(Switch::SavePreview)));
    w(format!("if {HAS_DATA} == 0 csp2 {PREVIEW}"));
    w("goto *conv_sl_buttons".into());

    // Asks "No.NN {question}" about the selected slot; no: back to waiting.
    let ask = |question: &str, kind: Switch| match confirm {
        Some(_) => format!(
            "mov {SLOT},{SELECTED}\ngosub *conv_sl_number\nmov {CONFIRM_KIND},{}:mov {CONFIRM_TEXT},{LINE1}\nadd {CONFIRM_TEXT},\" {}\"\ngosub *{CONFIRM_ROUTINE}\nif {CONFIRM} == 0 goto *conv_sl_buttons",
            switch_index(kind) + 1,
            escape(question)
        ),
        None => String::new(),
    };
    w("*conv_sl_act".into());
    w(format!("if {SELECTED} == 0 goto *conv_sl_wait"));
    w(format!("if {MODE} == 2 goto *conv_sl_load"));
    if let Some(c) = confirm {
        w(format!("if {SELECTED_HAS_DATA} == 0 goto *conv_sl_save"));
        w(ask(&c.overwrite, Switch::ConfirmSave));
    }
    w("*conv_sl_save".into());
    w(format!("mov {SLOT},{SELECTED}"));
    w("gosub *conv_sl_file".into());
    w(format!("savescreenshot2 {FILE}"));
    // Label: "No.NN - 20YY/MM/DD hh:mm:ss|chapter". ONScripter (Shift-JIS)
    // draws half-width characters two per cell and drops a space that
    // starts a cell, so spaces at even positions are full-width.
    w("gosub *conv_sl_number".into());
    w(format!("date {},{},{}", DATE[0], DATE[1], DATE[2]));
    w(format!("time {},{},{}", DATE[3], DATE[4], DATE[5]));
    w(format!("add {LINE1},\" - 20\""));
    for (i, v) in DATE.iter().enumerate() {
        if i > 0 {
            w(format!("add {LINE1},\"{}\"", ["", "/", "/", "　", ":", ":"][i]));
        }
        w(format!("if {v} < 10 add {LINE1},\"0\""));
        w(format!("itoa {STR_TMP},{v}"));
        w(format!("add {LINE1},{STR_TMP}"));
    }
    w(format!("mov {LABEL},{LINE1}"));
    w(format!("add {LABEL},\"|\""));
    w(format!("add {LABEL},{CHAPTER}"));
    match (&backdrop_off, &backdrop_on) {
        (Some(off), Some(on)) => w(format!("{off}:savegame2 {SELECTED},{LABEL}:{on}")),
        _ => w(format!("savegame2 {SELECTED},{LABEL}")),
    }
    if scene_stack {
        w("gosub *conv_scene_keep".into());
    }
    w(format!("mov {NEWEST},{SELECTED}"));
    w("goto *conv_sl_page".into());

    w("*conv_sl_load".into());
    w(format!("if {SELECTED_HAS_DATA} == 0 goto *conv_sl_wait"));
    if let Some(c) = confirm {
        w(ask(&c.load, Switch::ConfirmLoad));
    }
    w("gosub *conv_sl_clear".into());
    if scene_stack {
        w(format!("mov {SCENE_LOAD_SLOT},{SELECTED}"));
    }
    w(format!("loadgame {SELECTED}"));
    // Still here: the load failed.
    w("goto *conv_sl".into());

    w("*conv_sl_delete".into());
    if let Some(c) = confirm {
        w(ask(&c.delete, Switch::ConfirmDelete));
    }
    w(format!("mov {SLOT},{SELECTED}"));
    w("gosub *conv_sl_file".into());
    w(format!("delsavegame {SELECTED},{FILE}"));
    if scene_stack {
        w("gosub *conv_scene_drop".into());
    }
    w(format!("if {SELECTED} == {NEWEST} mov {NEWEST},0"));
    w("goto *conv_sl_page".into());

    w("*conv_sl_close".into());
    // texton first: it drops the pending screen updates except the window's.
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    if let Some(off) = &backdrop_off {
        w(format!("if {IN_GAME} == 1 {off}"));
    }
    w("gosub *conv_sl_clear".into());
    w("print 1".into());
    w("return".into());

    w("*conv_sl_clear".into());
    w((0..=BASE).map(|n| format!("csp2 {n}")).collect::<Vec<_>>().join(":"));
    w(MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("return".into());

    // Slot SLOT: HAS_DATA, LINE1 and LINE2 (its label), FILE (thumbnail).
    w("*conv_sl_slot".into());
    w("gosub *conv_sl_file".into());
    w(format!("mov {TMP},{SLOT}+{DELETED_BASE}"));
    w(format!("if %{TMP} == 1 delsavegame {SLOT},{FILE}:mov %{TMP},0"));
    w(format!("savefileexist {HAS_DATA},{SLOT}"));
    w(format!("if {HAS_DATA} == 0 goto *conv_sl_empty"));
    w(format!("getsavestr {LABEL},{SLOT}"));
    w(format!("split {LABEL},\"|\",{LINE1},{LINE2}"));
    w(format!("if {LINE2} == \"\" mov {LINE2},\"　\""));
    w("return".into());
    w("*conv_sl_empty".into());
    w("gosub *conv_sl_number".into());
    w(format!("add {LINE1},\" - no　data\""));
    w(format!("mov {LINE2},\"　\""));
    w("return".into());

    w("*conv_sl_number".into());
    w(format!("mov {LINE1},\"No.\""));
    w(format!("if {SLOT} < 10 add {LINE1},\"0\""));
    w(format!("itoa {STR_TMP},{SLOT}"));
    w(format!("add {LINE1},{STR_TMP}"));
    w("return".into());

    w("*conv_sl_file".into());
    w(format!("itoa {STR_TMP},{SLOT}"));
    w(format!("mov {FILE},\"savethumb\""));
    w(format!("add {FILE},{STR_TMP}"));
    w(format!("add {FILE},\".bmp\""));
    w(format!("mov {THUMB_IMAGE},\":c;\""));
    w(format!("add {THUMB_IMAGE},{FILE}"));
    w("return".into());
}
