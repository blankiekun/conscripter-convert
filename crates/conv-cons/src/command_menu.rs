//! The command menu ([`CommandMenu`]): picture buttons in a column over a
//! full-screen backdrop, opened from a click wait by a right-click or its
//! key ([`crate::menu::button_bar`] routes them here). Its actions are the
//! button bar's routines.
//!
//! The backdrop and the items are `lsp` sprites in the menus' range, over
//! the text window: the items from its first slot down, the backdrop under
//! them. Screens that cover the game (save, load, log, settings, a
//! [`BarAction::Call`]) take the menu away while they are open and put it
//! back after; the yes/no questions are drawn over it.

use conv_core::ir::{BarAction, ButtonBar, CommandMenu};

use crate::menu::{bar_label, disabled_while, goes_on, key_code, BAR_CLEAR, BAR_OUT, BAR_STATE};
use crate::{menu_slot, menu_z, path, BUTTON_VAR, MENU_SLOTS, MENU_SPRITES};

/// Opens the menu; `goto` it from the bar's action routine (it returns to
/// the click wait).
pub(crate) const MENU: &str = "conv_cmd_menu";
/// 1 when the menu closed with an action that goes on to the next page.
pub(crate) const GO_ON: &str = "%140";
const CLOSE: &str = "conv_cmd_close";
const WAIT: &str = "conv_cmd_wait";
/// Takes the menu away (at once) and puts it back.
const HIDE: &str = "conv_cmd_hide";
const SHOW: &str = "conv_cmd_show";

/// The routine that runs item `i`'s [`BarAction::Call`].
pub(crate) fn call_label(i: usize) -> String {
    format!("conv_cmd_call{i}")
}

/// `picture_replay`: see [`crate::menu::BarScreens::picture_replay`].
pub(crate) fn emit(out: &mut String, m: &CommandMenu, bar: &ButtonBar, picture_replay: bool) {
    assert!(m.items.len() < MENU_SLOTS, "too many command menu items");
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let slot = menu_slot;
    // Under the items: 33, or under the slots past it.
    let bd = slot(m.items.len().max(MENU_SPRITES.len() - 1));
    let state = m.items.iter().any(|i| disabled_while(i.action, picture_replay).is_some());
    let backdrop = format!("lsp {bd},\":a;{}\",{},{}{}", path(&m.backdrop.image), m.backdrop.x, m.backdrop.y, menu_z(bd));
    let item = |i: usize| {
        let b = &m.items[i].button;
        format!("lsp {},\":a/{},0,3;{}\",{},{}{}", slot(i), if m.pressed { 4 } else { 3 }, path(&b.image), b.x, b.y, menu_z(slot(i)))
    };
    // The disabled items' cell.
    let cells = || -> Vec<String> {
        m.items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| disabled_while(it.action, picture_replay).map(|off| format!("if {off} == 1 cell {},2", slot(i))))
            .collect()
    };

    w(format!("*{MENU}"));
    w(format!("gosub *{BAR_CLEAR}"));
    if bar.slide.is_some() {
        w(format!("mov {BAR_OUT},0"));
    }
    if state {
        w(format!("gosub *{BAR_STATE}"));
    }
    w(backdrop.clone());
    let fade = if m.fade_ms > 0 { format!("print 10,{}", m.fade_ms) } else { "print 1".into() };
    // The items one by one, from the top, each appearing step_ms after
    // the one before (an spmotion that jumps to its alpha then); with no
    // step, faded in with the backdrop.
    let together = m.step_ms == 0;
    if !together {
        w(fade.clone());
    }
    let disabled = cells();
    let n = m.items.len();
    for i in 0..n {
        w(item(i));
        if let Some(off) = disabled_while(m.items[i].action, picture_replay) {
            w(format!("if {off} == 1 cell {},2", slot(i)));
        }
        if !together && i > 0 {
            w(format!("spmotion {},0,0,0,0,100,100,0,0,0,{},0,0,100,100,0,256,4", slot(i), i as u32 * m.step_ms));
        }
    }
    if together {
        w(fade.clone());
    } else if n > 0 {
        w(format!("print 1:spmotionwait {}:wait {}", slot(n - 1), m.step_ms));
    }

    w(format!("*{WAIT}"));
    if state {
        w(format!("gosub *{BAR_STATE}"));
    }
    for (i, it) in m.items.iter().enumerate() {
        let only = disabled_while(it.action, picture_replay).map_or(String::new(), |off| format!("if {off} == 0 "));
        let press = if m.pressed { ",3" } else { "" };
        w(format!("{only}cell {0},0:{only}spbtn {0},{1}{press}", slot(i), i + 1));
    }
    for line in &disabled {
        w(line.clone());
    }
    w("print 1".into());
    if m.key.is_some() || m.items.iter().any(|i| i.key.is_some()) {
        w("getkey".into());
    }
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 goto *{CLOSE}"));
    if let Some(k) = m.key {
        w(format!("if {BUTTON_VAR} == {} goto *{CLOSE}", key_code(k)));
    }
    for (i, it) in m.items.iter().enumerate() {
        if let Some(k) = it.key {
            w(format!("if {BUTTON_VAR} == {} mov {BUTTON_VAR},{}", key_code(k), i + 1));
        }
    }
    for i in 0..m.items.len() {
        w(format!("if {BUTTON_VAR} == {} goto *conv_cmd_item{i}", i + 1));
    }
    w(format!("goto *{WAIT}"));

    for (i, it) in m.items.iter().enumerate() {
        w(format!("*conv_cmd_item{i}"));
        let label = bar_label(it.action);
        match it.action {
            BarAction::Close => w(format!("gosub *{CLOSE}:return")),
            a if goes_on(a) => w(format!("gosub *{CLOSE}:mov {GO_ON},1:goto *{label}")),
            BarAction::HideWindow => w(format!("gosub *{CLOSE}:goto *{label}")),
            BarAction::Call if it.label.is_some() => w(format!("gosub *{HIDE}:gosub *{}:gosub *{SHOW}:goto *{WAIT}", call_label(i))),
            BarAction::Call => w(format!("goto *{WAIT}")),
            BarAction::Save | BarAction::Load | BarAction::Log | BarAction::Settings => w(format!("gosub *{HIDE}:gosub *{label}:gosub *{SHOW}:goto *{WAIT}")),
            // Questions are asked over the menu.
            _ => w(format!("gosub *{label}:goto *{WAIT}")),
        }
    }

    // The screen, the game's sprites and the music come back after a call.
    for (i, it) in m.items.iter().enumerate() {
        let Some(l) = it.label.as_ref().filter(|_| it.action == BarAction::Call) else { continue };
        w(format!("*{}", call_label(i)));
        w(format!("gosub *{BAR_CLEAR}:pushscreen {},{}", crate::SPRITE_BASE, crate::MAX_SPRITE));
        w(format!("gosub *{}", crate::label(l)));
        w("popscreen:print 1".into());
        w("return".into());
    }

    w(format!("*{HIDE}"));
    w(format!("btndef clear:csp {bd}"));
    w((0..m.items.len()).map(|i| format!("csp {}", slot(i))).collect::<Vec<_>>().join(":"));
    w("print 1".into());
    w("return".into());
    w(format!("*{SHOW}"));
    w(format!("gosub *{BAR_CLEAR}"));
    w(backdrop.clone());
    for i in 0..m.items.len() {
        w(item(i));
    }
    w("return".into());

    // The items go one by one from the bottom (the hover cell at once),
    // then the backdrop fades out.
    w(format!("*{CLOSE}"));
    w("btndef clear".into());
    for i in 0..m.items.len() {
        w(format!("cell {},0", slot(i)));
    }
    for line in &disabled {
        w(line.clone());
    }
    // From the bottom, each going step_ms after the one below (loop 3).
    for k in 0..n {
        match together {
            true => w(format!("csp {}", slot(n - 1 - k))),
            false => w(format!("spmotion {},3,0,0,0,100,100,0,256,0,{},0,0,100,100,0,256,0", slot(n - 1 - k), k as u32 * m.step_ms)),
        }
    }
    if !together && n > 0 {
        w(format!("print 1:spmotionwait {}:wait {}", slot(0), m.step_ms));
    }
    w(format!("csp {bd}"));
    w(fade);
    w("return".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{BarSlide, CommandItem, Placed};

    fn placed(name: &str, x: i32, y: i32) -> Placed {
        Placed { image: format!("system/{name}.png"), x, y, w: 880, h: 20 }
    }

    fn menu() -> CommandMenu {
        let item = |name: &str, y, action, key| CommandItem { button: placed(name, 636, y), action, key, label: None };
        CommandMenu {
            backdrop: Placed { image: "system/back.png".into(), x: 0, y: 0, w: 1024, h: 576 },
            items: vec![
                item("cancel", 89, BarAction::Close, None),
                item("auto", 109, BarAction::Auto, Some('A')),
                item("piece", 129, BarAction::QuickLoad, Some('P')),
                item("save", 149, BarAction::Save, Some('S')),
                item("title", 169, BarAction::Title, Some('T')),
            ],
            fade_ms: 100,
            step_ms: 10,
            key: Some('M'),
            pressed: true,
        }
    }

    #[test]
    fn opens_item_by_item_and_closes_from_the_bottom() {
        let bar = ButtonBar { slide: Some(BarSlide { hidden_dx: 32, edge_x: 968, ms: 200 }), ..Default::default() };
        let mut out = String::new();
        emit(&mut out, &menu(), &bar, false);
        for line in [
            "*conv_cmd_menu\ngosub *conv_bar_clear\nmov %143,0\ngosub *conv_bar_state\nlsp 33,\":a;system\\back.png\",0,0\nprint 10,100\nlsp 6,\":a/4,0,3;system\\cancel.png\",636,89\nlsp 7,",
            // each item 10 ms after the one above, by itself
            "lsp 8,\":a/4,0,3;system\\piece.png\",636,129\nif %141 == 1 cell 8,2\nspmotion 8,0,0,0,0,100,100,0,0,0,20,0,0,100,100,0,256,4\n",
            ",40,0,0,100,100,0,256,4\nprint 1:spmotionwait 10:wait 10\n",
            "if %141 == 0 cell 8,0:if %141 == 0 spbtn 8,3,3\n",
            "getkey\nbtnwait %9\nif %9 == -1 goto *conv_cmd_close\nif %9 == -1077 goto *conv_cmd_close\nif %9 == -1065 mov %9,2\n",
            "*conv_cmd_item0\ngosub *conv_cmd_close:return\n",
            "*conv_cmd_item1\ngosub *conv_cmd_close:mov %140,1:goto *conv_bar_auto\n",
            "*conv_cmd_item3\ngosub *conv_cmd_hide:gosub *conv_bar_save:gosub *conv_cmd_show:goto *conv_cmd_wait\n",
            "*conv_cmd_item4\ngosub *conv_bar_title:goto *conv_cmd_wait\n",
            // closing from the bottom, 10 ms apart
            "spmotion 10,3,0,0,0,100,100,0,256,0,0,0,0,100,100,0,256,0\nspmotion 9,3,0,0,0,100,100,0,256,0,10,0,0,100,100,0,256,0\n",
            "spmotion 6,3,0,0,0,100,100,0,256,0,40,0,0,100,100,0,256,0\nprint 1:spmotionwait 6:wait 10\ncsp 33\nprint 10,100\nreturn\n",
        ] {
            assert!(out.contains(line), "missing:\n{line}\nin:\n{out}");
        }
    }
}
