//! The button bar and the top bars ([`ButtonBar`]), and the screens they open.

use super::*;

/// What the bar's actions can open, and the screen's size.
#[derive(Clone, Copy, Default)]
pub(crate) struct BarScreens<'a> {
    pub save_menu: bool,
    pub settings: bool,
    pub backlog: bool,
    /// There is a scene replay, during which save and load are left out.
    pub replay: bool,
    /// There is a picture extras' scene replay ([`crate::picture_replay`]),
    /// during which save, load, quick save and load and back to a choice
    /// are disabled, and back to the title goes back to its list.
    pub picture_replay: bool,
    pub command_menu: Option<&'a CommandMenu>,
    pub confirm: Option<&'a ConfirmDialog>,
    pub width: u32,
    pub height: u32,
    /// The click-wait icon and the variables holding its top-left corner.
    pub wait_icon: Option<(&'a WaitIcon, [&'static str; 2])>,
    /// `Some(drop)`: the icon has no place of its own and is drawn after
    /// the text, `drop` below where `getcursorpos` puts the next character,
    /// as the engine draws its cursor (which it doesn't while `textgosub`
    /// runs this routine).
    pub icon_after_text: Option<i32>,
    /// The variable holding the story's `gamespeed` percent: the click wait
    /// (and the menus opened from it) run at 100, the story at that.
    pub game_speed: Option<&'static str>,
}

/// The routine that runs a bar action (`gosub` it).
pub(crate) fn bar_label(a: BarAction) -> &'static str {
    match a {
        BarAction::Auto => "conv_bar_auto",
        BarAction::Skip => "conv_bar_skip",
        BarAction::Log => "conv_bar_log",
        BarAction::Save => "conv_bar_save",
        BarAction::Load => "conv_bar_load",
        BarAction::Settings => "conv_bar_settings",
        BarAction::HideWindow | BarAction::Unavailable => "conv_bar_hide",
        BarAction::QuickSave => "conv_bar_qsave",
        BarAction::QuickLoad => "conv_bar_qload",
        BarAction::Title => "conv_bar_title",
        BarAction::Quit => "conv_bar_quit",
        BarAction::Fullscreen => "conv_bar_full",
        BarAction::FastSkip => "conv_bar_fskip",
        BarAction::BackToChoice => "conv_bar_back",
        BarAction::Lock { .. } => "conv_bar_lock",
        // The command menu's own.
        BarAction::Close | BarAction::Call => "",
    }
}

/// Actions after which the click wait goes on to the next page.
pub(crate) fn goes_on(a: BarAction) -> bool {
    matches!(a, BarAction::Auto | BarAction::Skip | BarAction::FastSkip)
}

/// The variable that is 1 while `a` can't be run (no quick save, no save
/// before a choice, a replay playing), set by [`BAR_STATE`]; `None`: always
/// available.
pub(crate) fn disabled_while(a: BarAction, picture_replay: bool) -> Option<&'static str> {
    match a {
        BarAction::QuickLoad => Some(NO_QUICK_SAVE),
        BarAction::BackToChoice => Some(NO_BACK_SAVE),
        BarAction::Save | BarAction::Load | BarAction::QuickSave if picture_replay => Some(NO_SAVE),
        _ => None,
    }
}

/// The letter that hides the text window in a game whose waits read
/// letters (`getkey`); the gamepad's Y sends it (`padmap`).
const HIDE_KEY: char = 'W';

/// The key the gamepad's Y sends to hide the text window (`padmap "y"`):
/// the hide letter where the waits read letters, Esc where a right click
/// hides; `None` where nothing hides it from a key.
pub(crate) fn pad_hide_key(menu: Option<&CommandMenu>) -> Option<String> {
    match menu {
        None => Some("Escape".into()),
        Some(_) if reads_keys(menu) => Some(HIDE_KEY.to_string()),
        Some(_) => None,
    }
}

/// The waits read letters (`getkey`): the command menu has key shortcuts.
fn reads_keys(menu: Option<&CommandMenu>) -> bool {
    menu.is_some_and(|m| m.key.is_some() || m.items.iter().any(|i| i.key.is_some()))
}

/// The value CONscripter's `getkey` makes `btnwait` return for a letter.
pub(crate) fn key_code(c: char) -> i32 {
    -1000 - c.to_ascii_uppercase() as i32
}

/// The click-wait routine (`textgosub`): shows the bars the pointer has
/// revealed, waits, runs the clicked button's action, and ends the wait
/// with `texec`. Link pages call [`BAR_DRAW`], [`BAR_HOVER`] and
/// [`BAR_ACTION`] around their own `btnwait`.
///
/// Bar buttons return [`BAR_BUTTON_BASE`] + 1 + index; a right-click (-1)
/// hides the text window, or opens the [`CommandMenu`] when there is one
/// (as does its key; the items' keys run their actions). `textbtnwait`
/// returns at once in skip mode and after the delay in auto mode; a
/// click turns auto mode off.
///
/// Bars without a [`BarSlide`] are always up during a click wait. A bar
/// with one is drawn off screen; an invisible hotspot (`btn`) beyond its
/// edge, with `getmouseover`, ends the wait when the pointer comes onto
/// it, and the bar slides in; the hotspot then covers the rest of the
/// screen, and the bar slides out when the pointer goes back.
pub(crate) fn button_bar(out: &mut String, bar: &ButtonBar, s: BarScreens) {
    let mut slots = BAR_SPRITES;
    // The base is drawn over the buttons (a lower number).
    let base_slot = if bar.base.is_some() { slots.next().unwrap() } else { 0 };
    let buttons: Vec<_> = bar.buttons.iter().zip(slots.by_ref()).collect();
    let (chapter_text, scene_text) = (slots.next().unwrap(), slots.next().unwrap());
    // Back, forward, exit, list.
    let scene_buttons: [u32; 4] = std::array::from_fn(|_| slots.next().unwrap());
    // An invisible button over the chapter bar (see [`ChapterHistory`]).
    let chapters_slot = slots.next();
    // The click-wait icon: over the text window (below humanz), cleared
    // with the bar.
    let icon_slot = s.wait_icon.map(|_| slots.next().expect("no bar sprite left for the click-wait icon"));
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let slide = bar.slide;
    // Where an unpinned bar shows ([`BarAction::Lock`]).
    let reveal = bar.buttons.iter().find_map(|b| match b.action {
        BarAction::Lock { reveal_y, fade_ms } => Some((reveal_y, fade_ms)),
        _ => None,
    });
    // A sliding bar's x is offset by BAR_OFFSET.
    let x = |p: &Placed| if slide.is_some() { format!("{}+{BAR_OFFSET}", p.x) } else { p.x.to_string() };
    let lsp = |slot: u32, p: &Placed, cells: &str| format!("lsp {slot},\":a{cells};{}\",{},{}", path(&p.image), x(p), p.y);
    let cells = if bar.pressed { "/4,0,3" } else { "/3,0,3" };
    let press = if bar.pressed { ",3" } else { "" };
    let number = |i: usize| BAR_BUTTON_BASE + 1 + i as i32;
    let menu = s.command_menu;
    let menu_items = menu.map_or(&[][..], |m| &m.items[..]);
    let keys = reads_keys(menu);

    w(format!("*{WAIT}"));
    // *start's erasetextwindow 0 again: a run started with --goto (a test
    // save, a trace) skips *start, and the print below would hide the window.
    w("erasetextwindow 0".into());
    if s.game_speed.is_some() {
        w("consset \"gamespeed\",100".into());
    }
    if menu.is_some() {
        w(format!("mov {},0", crate::command_menu::GO_ON));
    }
    w(format!("gosub *{BAR_DRAW}"));
    // The icon (drawing the bar cleared it).
    if let (Some((i, [x, y])), Some(slot)) = (s.wait_icon, icon_slot) {
        let ms: Vec<String> = i.frame_ms.iter().map(u32::to_string).collect();
        if let Some(drop) = s.icon_after_text {
            w(format!("getcursorpos {TMP},{MOUSE_Y}"));
            w(format!("lsp {slot},\":a/{},<{}>,0;{}\",{TMP},{MOUSE_Y} + {drop}:print 1", ms.len(), ms.join(","), path(&i.image)));
        } else {
            w(format!("if {x} > 0 || {y} > 0 lsp {slot},\":a/{},<{}>,0;{}\",{x},{y}:print 1", ms.len(), ms.join(","), path(&i.image)));
        }
    }
    if let Some(sl) = slide {
        // The pointer may have crossed the edge since the bar was drawn. Off
        // the screen (no pointer: auto-play reports the far corner) is away.
        w(format!("getmousepos {TMP},{MOUSE_Y}"));
        w(format!("if {TMP} >= {} || {MOUSE_Y} >= {} mov {TMP},0", s.width, s.height));
        w(format!("if {BAR_OUT} == 0 && {TMP} > {} gosub *{BAR_SLIDE_IN}:goto *{WAIT}", sl.edge_x));
        w(format!("if {BAR_OUT} == 1 && {TMP} <= {} gosub *{BAR_SLIDE_OUT}:goto *{WAIT}", sl.edge_x));
        w(format!("getmouseover {BTN_EDGE},{BTN_EDGE}"));
    }
    if reveal.is_some() {
        // Unpinned: the pointer crossing the line redraws the bar.
        w(format!("if {BAR_UNPINNED} == 1 getmouseover {BTN_EDGE},{BTN_EDGE}"));
    }
    if keys {
        w("getkey".into());
    }
    w(format!("textbtnwait {BUTTON_VAR}"));
    if slide.is_some() || reveal.is_some() {
        w(format!("if {BUTTON_VAR} == {BTN_EDGE} goto *{WAIT}"));
    }
    // Wheel down (-3) reads on, like a click; wheel up (-2) opens the log.
    w(format!("if {BUTTON_VAR} == -3 && {} == 0 goto *conv_wait_end", switch_off(Switch::WheelReadsOn)));
    w(format!("if {BUTTON_VAR} == -3 goto *{WAIT}"));
    if slide.is_some() {
        // While the bar is out a click (not auto mode's time) does nothing.
        w(format!("isskip {MODE_STATE}"));
        w(format!("if {BUTTON_VAR} == 0 && {BAR_OUT} == 1 && {MODE_STATE} == 0 goto *{WAIT}"));
    }
    w(format!("if {BUTTON_VAR} >= 0 && {BUTTON_VAR} <= {BAR_BUTTON_BASE} goto *conv_wait_end"));
    w(format!("gosub *{BAR_ACTION}"));
    if menu.is_some() {
        w(format!("if {} == 1 goto *conv_wait_end", crate::command_menu::GO_ON));
    }
    // Auto and skip go on to the next page; the rest come back here.
    for (i, (b, _)) in buttons.iter().enumerate() {
        if goes_on(b.action) {
            w(format!("if {BUTTON_VAR} == {} goto *conv_wait_end", number(i)));
        }
    }
    w(format!("goto *{WAIT}"));
    w("*conv_wait_end".into());
    if let Some(slot) = icon_slot {
        w(format!("csp {slot}"));
    }
    if let Some(v) = s.game_speed {
        w(format!("consset \"gamespeed\",{v}"));
    }
    w("texec".into());
    w("return".into());

    // Removes every bar sprite.
    w(format!("*{BAR_HIDE}"));
    w(format!("gosub *{BAR_CLEAR}"));
    w("print 1".into());
    w("return".into());
    w(format!("*{BAR_CLEAR}"));
    w(BAR_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));

    if bar.top.is_some() {
        w(scene_row_sprites().map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    }
    w(TOP_SPRITES2.map(|n| format!("csp2 {n}")).collect::<Vec<_>>().join(":"));
    w("return".into());

    let all_actions: Vec<BarAction> = bar.buttons.iter().map(|b| b.action).chain(menu_items.iter().map(|i| i.action)).collect();
    let state = all_actions.iter().any(|&a| disabled_while(a, s.picture_replay).is_some());
    if state {
        w(format!("*{BAR_STATE}"));
        w(format!("savefileexist {TMP},{QUICK_SLOT}:mov {NO_QUICK_SAVE},1-{TMP}"));
        w(format!("savefileexist {TMP},{BACK_SLOT}:mov {NO_BACK_SAVE},1-{TMP}"));
        if s.picture_replay {
            let playing = crate::picture_replay::REPLAYING;
            w(format!("mov {NO_SAVE},{playing}"));
            w(format!("if {playing} == 1 mov {NO_QUICK_SAVE},1:mov {NO_BACK_SAVE},1"));
        }
        w("return".into());
    }

    // Draws the bars and registers their buttons; link words are
    // registered after this.
    w(format!("*{BAR_DRAW}"));
    w("btndef clear".into());
    w(format!("gosub *{BAR_CLEAR}"));
    w(format!("isskip {MODE_STATE}"));
    if let Some(sl) = slide {
        w(format!("mov {BAR_OFFSET},{}:if {BAR_OUT} == 1 mov {BAR_OFFSET},0", sl.hidden_dx));
    }
    if let Some((y, _)) = reveal {
        // Unpinned and the pointer above the line: only the hotspot below
        // it. Off the screen (auto-play) is below.
        w(format!("getmousepos {TMP},{MOUSE_Y}"));
        w(format!("if {BAR_UNPINNED} == 1 && {MOUSE_Y} < {y} btn {BTN_EDGE},0,{y},{},{},0,0:mov {BAR_SHOWN},0:print 1:return", s.width, s.height as i32 - y));
    }
    if state {
        w(format!("gosub *{BAR_STATE}"));
    }
    // Only a bar that is out has buttons.
    let only_out = if slide.is_some() { format!("if {BAR_OUT} == 1 ") } else { String::new() };
    for (i, (b, slot)) in buttons.iter().enumerate() {
        if b.action == BarAction::Unavailable {
            w(lsp(*slot, &b.image, ""));
            continue;
        }
        if s.replay && matches!(b.action, BarAction::Save | BarAction::Load) {
            w(format!("if {} == 0 {}:spbtn {slot},{}", crate::replay::REPLAYING, lsp(*slot, &b.image, cells), number(i)));
            continue;
        }
        w(lsp(*slot, &b.image, cells));
        if let Some(active) = &b.active {
            let on = match b.action {
                BarAction::Auto => format!("{MODE_STATE} == 2"),
                BarAction::Lock { .. } => format!("{BAR_UNPINNED} == 1"),
                _ => format!("{MODE_STATE} == 1"),
            };
            w(format!("if {on} {}", lsp(*slot, active, cells)));
        }
        match disabled_while(b.action, s.picture_replay) {
            Some(off) => w(format!("if {off} == 1 cell {slot},2\n{only_out}if {off} == 0 spbtn {slot},{}{press}", number(i))),
            None => w(format!("{only_out}spbtn {slot},{}{press}", number(i))),
        }
    }
    if let Some(base) = &bar.base {
        w(lsp(base_slot, base, ""));
    }
    if let Some(sl) = slide {
        // The hotspot: past the edge while the bar is in, the rest while out.
        let (edge, width, height) = (sl.edge_x + 1, s.width as i32, s.height as i32);
        w(format!("if {BAR_OUT} == 0 btn {BTN_EDGE},{edge},0,{},{height},0,0", width - edge));
        w(format!("if {BAR_OUT} == 1 btn {BTN_EDGE},0,0,{edge},{height},0,0"));
    }
    if let Some((y, fade_ms)) = reveal {
        // The hotspot above the bar ends the wait when the pointer leaves.
        w(format!("if {BAR_UNPINNED} == 1 btn {BTN_EDGE},0,0,{},{y},0,0", s.width));
        // Coming down to a hidden bar, it fades in.
        let fades: Vec<String> = buttons.iter().map(|(_, slot)| *slot).chain(bar.base.as_ref().map(|_| base_slot)).map(|n| crate::motion::fade_in(n, fade_ms)).collect();
        if fade_ms > 0 && !fades.is_empty() {
            w(format!("if {BAR_UNPINNED} == 1 && {BAR_SHOWN} == 0 {}", fades.join(":")));
        }
        w(format!("mov {BAR_SHOWN},1"));
    }
    if let Some(top) = &bar.top {
        // Unpinned, the top bars stay hidden.
        if reveal.is_some() {
            w(format!("if {BAR_UNPINNED} == 1 goto *conv_bar_draw_end"));
        }
        scene_bar_draw(&mut w, top, chapter_text, scene_text, scene_buttons);
        if let (Some(_), Some(slot)) = (&top.chapter_history, chapters_slot) {
            w(crate::chapters::hotspot(top, slot));
        }
    }
    if reveal.is_some() {
        w("*conv_bar_draw_end".into());
    }
    w("print 1".into());
    w("return".into());

    if let Some(sl) = slide {
        // Eases out (cubic): the offset left is dx·(1 - t)³.
        let (dx, ms) = (sl.hidden_dx, sl.ms.max(1));
        w(format!("*{BAR_SLIDE_IN}"));
        w(format!("mov {BAR_OUT},1:goto *conv_bar_slide"));
        w(format!("*{BAR_SLIDE_OUT}"));
        w(format!("mov {BAR_OUT},0"));
        w("*conv_bar_slide".into());
        w("btndef clear".into());
        w("resettimer".into());
        w("*conv_bar_slide_loop".into());
        w(format!("gettimer {TMP}:if {TMP} > {ms} mov {TMP},{ms}"));
        w(format!("mov {SLIDE_K},({ms}-{TMP})*1000/{ms}"));
        w(format!("mov {BAR_OFFSET},{dx}*{SLIDE_K}*{SLIDE_K}/1000*{SLIDE_K}/1000000"));
        w(format!("if {BAR_OUT} == 0 mov {BAR_OFFSET},{dx}-{BAR_OFFSET}"));
        for (b, slot) in &buttons {
            w(format!("amsp {slot},{},{}", x(&b.image), b.image.y));
        }
        if let Some(base) = &bar.base {
            w(format!("amsp {base_slot},{},{}", x(base), base.y));
        }
        w("print 1".into());
        w(format!("if {TMP} < {ms} wait 1:goto *conv_bar_slide_loop"));
        w("return".into());
    }

    w(format!("*{BAR_ACTION}"));
    let right_click = if menu.is_some() { crate::command_menu::MENU } else { "conv_bar_hide" };
    w(format!("if {BUTTON_VAR} == -1 goto *{right_click}"));
    if let Some(k) = menu.and_then(|m| m.key) {
        w(format!("if {BUTTON_VAR} == {} goto *{right_click}", key_code(k)));
    }
    if keys {
        assert!(menu_items.iter().all(|i| i.key != Some(HIDE_KEY)), "a menu item takes the hide key {HIDE_KEY}");
        w(format!("if {BUTTON_VAR} == {} goto *conv_bar_hide", key_code(HIDE_KEY)));
    }
    for (i, item) in menu_items.iter().enumerate() {
        let Some(k) = item.key else { continue };
        let go = if goes_on(item.action) { format!("mov {},1:", crate::command_menu::GO_ON) } else { String::new() };
        let call;
        let target = match item.action {
            BarAction::Call if item.label.is_some() => {
                call = crate::command_menu::call_label(i);
                call.as_str()
            }
            BarAction::Call | BarAction::Close => continue,
            a => bar_label(a),
        };
        w(format!("if {BUTTON_VAR} == {} {go}goto *{target}", key_code(k)));
    }
    w(format!("if {BUTTON_VAR} == -2 goto *conv_bar_log"));
    if bar.top.is_some() {
        let last = BTN_SCENE_ROW + SCENE_ROWS as i32;
        w(format!("if {BUTTON_VAR} >= {BTN_SCENE_BACK} && {BUTTON_VAR} <= {last} goto *conv_scene_action"));
        if bar.top.as_ref().is_some_and(|t| t.chapter_history.is_some()) {
            w(format!("if {BUTTON_VAR} == {} goto *{}", crate::chapters::BTN_OPEN, crate::chapters::OPEN));
        }
    }
    for (i, (b, _)) in buttons.iter().enumerate() {
        if b.action != BarAction::Unavailable {
            w(format!("if {BUTTON_VAR} == {} goto *{}", number(i), bar_label(b.action)));
        }
    }
    w("return".into());
    let mut actions = all_actions;
    actions.push(BarAction::HideWindow);
    actions.push(BarAction::Log);
    actions.retain(|&a| !bar_label(a).is_empty());
    actions.sort_by_key(|a| bar_label(*a));
    actions.dedup_by_key(|a| bar_label(*a));
    let fast_skip = actions.contains(&BarAction::FastSkip);
    // The save menu covers the screen: put the bars away first.
    let menu_entry = |entry: &str| format!("gosub *{BAR_CLEAR}\nmov {IN_GAME},1\ngosub *{entry}");
    // Fades to black (the dialog's top sprite) before a load.
    let black = format!("gosub *{BAR_CLEAR}\nlsp {},\":c;>{},{},#000000\",0,0\nprint 10,{}", DIALOG_SPRITES.start, s.width, s.height, bar.load_fade_ms.max(1));
    for a in actions {
        w(format!("*{}", bar_label(a)));
        match a {
            BarAction::Auto => w("systemcall automode".into()),
            BarAction::Skip if fast_skip => w(format!("mov {FAST_SKIP},0:systemcall skip")),
            BarAction::Skip => w("systemcall skip".into()),
            BarAction::FastSkip => w(format!("mov {FAST_SKIP},1:systemcall skip")),
            BarAction::Log if s.backlog => w(menu_entry(BACKLOG)),
            BarAction::Log => w("systemcall lookback".into()),
            BarAction::Settings if s.settings => w(menu_entry(SETTINGS)),
            BarAction::Settings => w("return".into()),
            BarAction::Save if s.save_menu => w(menu_entry(SAVE)),
            BarAction::Load if s.save_menu => w(menu_entry(LOAD)),
            BarAction::Save => w("systemcall save".into()),
            BarAction::Load => w("systemcall load".into()),
            BarAction::HideWindow | BarAction::Unavailable => {
                if slide.is_some() {
                    w(format!("if {BAR_OUT} == 1 gosub *{BAR_SLIDE_OUT}"));
                }
                let (off, on) = hidden_while_erased(bar.top.is_some());
                w(off);
                w("systemcall windowerase".into());
                w(on)
            }
            BarAction::QuickSave => {
                w(format!("savegame {QUICK_SLOT}"));
                if let Some(se) = &bar.quick_save_se {
                    w(format!("dwave {},\"{}\"", crate::SE_CHANNELS.start(), path(se)));
                }
                if let Some(n) = &bar.quick_saved {
                    let (back, notice) = (DIALOG_SPRITES.start + 1, DIALOG_SPRITES.start);
                    w(format!("lsp {back},\":c;>{},{},#000000\",0,0,204", s.width, s.height));
                    w(format!("lsp {notice},\":a;{}\",{},{}", path(&n.image), n.x, n.y));
                    w(format!("print 1:click:csp {back}:csp {notice}:print 1"));
                }
            }
            BarAction::QuickLoad => {
                w(format!("savefileexist {TMP},{QUICK_SLOT}:if {TMP} == 0 return"));
                if s.confirm.is_some_and(|c| c.pictures.iter().any(|p| p.0 == Switch::ConfirmQuickLoad)) {
                    w(confirm_call("", Switch::ConfirmQuickLoad));
                    w(format!("if {CONFIRM} == 0 return"));
                }
                w(black.clone());
                w(format!("loadgame {QUICK_SLOT}"));
            }
            BarAction::BackToChoice => {
                w(format!("savefileexist {TMP},{BACK_SLOT}:if {TMP} == 0 return"));
                if let Some(c) = s.confirm {
                    w(confirm_call(&c.load, Switch::ConfirmBackToChoice));
                    w(format!("if {CONFIRM} == 0 return"));
                }
                w(black.clone());
                w(format!("mov {BACK_LOADING},1"));
                w(format!("loadgame {BACK_SLOT}"));
            }
            BarAction::Title | BarAction::Quit => {
                let (kind, then) = if a == BarAction::Title { (Switch::ConfirmTitle, "reset") } else { (Switch::ConfirmQuit, "end") };
                if let Some(c) = s.confirm {
                    w(confirm_call(if a == BarAction::Title { &c.title } else { &c.quit }, kind));
                    w(format!("if {CONFIRM} == 0 return"));
                }
                if a == BarAction::Title && s.picture_replay {
                    // The title goes on to the replay's list.
                    w(format!("if {} == 1 mov {},1", crate::picture_replay::REPLAYING, crate::picture_replay::BACK_TO_LIST));
                }
                w(then.into());
            }
            BarAction::Fullscreen => w(format!("isfull {TMP}\nif {TMP} == 1 menu_window:return\nmenu_full")),
            BarAction::Lock { .. } => w(format!("mov {BAR_UNPINNED},1-{BAR_UNPINNED}")),
            BarAction::Close | BarAction::Call => unreachable!(),
        }
        w("return".into());
    }
    if has_back_to_choice(bar, menu) {
        // Every load but the one going back to the choice drops that save
        // (it belongs to the game left), as a new game does.
        w(format!("*{LOADED}"));
        w(format!("if {BACK_LOADING} == 1 mov {BACK_LOADING},0:goto *conv_loaded_keep"));
        w(format!("delsavegame {BACK_SLOT}"));
        w("*conv_loaded_keep".into());
        if bar.top.is_some() {
            w(format!("goto *{SCENE_LOADED}"));
        }
        w("return".into());
    }
    if bar.top.is_some() {
        scene_stack(&mut w, bar.top.as_ref().is_some_and(|t| t.chapter_history.is_some()));
    }
}

/// The bar or the command menu can go back to the last choice.
pub(crate) fn has_back_to_choice(bar: &ButtonBar, menu: Option<&CommandMenu>) -> bool {
    bar.buttons.iter().any(|b| b.action == BarAction::BackToChoice) || menu.is_some_and(|m| m.items.iter().any(|i| i.action == BarAction::BackToChoice))
}
