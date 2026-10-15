//! The text history ([`Backlog`]).

use super::*;

/// The text history (see [`Backlog`]). Pages come from `getlog` (0 = the
/// page on screen); a page's line count is its length with line breaks
/// (`getlog`) minus without (`getlogtext`), plus one. [`LOG_OFFSET`] is
/// the newest page shown; up/wheel-up go back one page, down/wheel-down
/// forward (past the newest: close).
///
/// Each text line is tagged `[name|voice]` with its page's speaker (when a
/// name plate shows it) and voice clip; the log reads the tags back with
/// `gettaglog`, shows the name above the page and replays the voice when
/// the name is clicked. A tag is only kept when there is a pretextgosub,
/// so [`PRETEXT`] is one that does nothing but allow the save point
/// (pretextgosub makes the text wait for `saveon`).
/// `ruby`: `rubyon` is set, so `strsp` draws text with a line pitch a line
/// gap lower (the ruby goes above it); the log is drawn that much higher.
pub(crate) fn backlog(out: &mut String, b: &Backlog, ruby: bool) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let others = hidden_while_open();
    let vsp = |on: u32| others.iter().map(|n| format!("vsp {n},{on}")).collect::<Vec<_>>().join(":");
    let button = |slot: u32, p: &Placed| format!("lsp {slot},\":a/3,0,3;{}\",{},{}", path(&p.image), p.x, p.y);
    let a = &b.area;
    let size = b.font_size;
    let gap = b.line_height - size as i32;
    let lift = if ruby { format!("-{gap}") } else { String::new() };
    w(format!("*{PRETEXT}"));
    w("saveon".into());
    w("return".into());
    w(format!("*{BACKLOG}"));
    w(format!("if {IN_GAME} == 1 textoff:{}", vsp(0)));
    w(lsp2(BASE, &b.base, Some(b.base_opacity)));
    w(lsp2(FRAME, &b.frame, None));
    w(lsp2(HEADER, &b.track, None));
    w(button(LOG_UP, &b.up));
    w(button(LOG_DOWN, &b.down));
    w(button(CLOSE, &b.close));
    w(format!("mov {LOG_OFFSET},0"));
    w("*conv_log_draw".into());
    w((M..LOG_NAME_SPRITE + LOG_PAGES).map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w((0..LOG_PAGES).map(|k| format!("mov ${},\"\"", LOG_VOICES + k)).collect::<Vec<_>>().join(":"));
    w(format!("mov {LOG_Y},{}", a.y + a.h));
    w(format!("mov {LOG_K},0"));
    w("*conv_log_page".into());
    w(format!("if {LOG_K} >= {LOG_PAGES} goto *conv_log_shown"));
    w(format!("mov {TMP},{LOG_OFFSET}+{LOG_K}"));
    w(format!("getlog {LINE1},{TMP}"));
    w(format!("if {LINE1} == \"\" goto *conv_log_shown"));
    w(format!("getlogtext {LINE2},{TMP}"));
    w(format!("gettaglog {LOG_TAG},{TMP}"));
    w(format!("split {LOG_TAG},\"|\",{LOG_NAME},{LOG_VOICE}"));
    w(format!("mov {LOG_NAMED},1"));
    w(format!("if {LOG_NAME} == \"\" && {LOG_VOICE} == \"\" mov {LOG_NAMED},0"));
    w(format!("len {LOG_LINES},{LINE1}"));
    w(format!("len {TMP},{LINE2}"));
    w(format!("mov {LOG_LINES},{LOG_LINES}-{TMP}+1"));
    w(format!("mov {TMP},{LOG_Y}-({LOG_LINES}+{LOG_NAMED})*{}-{}", b.line_height, b.line_height / 2));
    // Doesn't fit: stop (but always show at least one page).
    w(format!("if {TMP} < {} && {LOG_K} > 0 goto *conv_log_shown", a.y));
    w(format!("if {TMP} < {} mov {TMP},{}", a.y, a.y));
    w(format!("mov {LOG_SPRITE},{LOG_K}+{M}"));
    w(format!("mov {LOG_TEXT_Y},{TMP}+{LOG_NAMED}*{}", b.line_height));
    w(format!("strsp {LOG_SPRITE},{LINE1},{},{LOG_TEXT_Y}{lift},{},{LOG_LINES},{size},{size},0,{gap},0,1", a.x, b.columns));
    w(format!("if {LOG_NAMED} == 1 gosub *conv_log_name"));
    w(format!("mov {LOG_Y},{TMP}"));
    w(format!("add {LOG_K},1"));
    w("goto *conv_log_page".into());
    // The name of page LOG_K at TMP; with a voice it's a button replaying it.
    w("*conv_log_name".into());
    w(format!("mov {LOG_SPRITE},{LOG_K}+{LOG_NAME_SPRITE}"));
    let name = |text: &str, colors: &str| {
        format!("strsp {LOG_SPRITE},{text},{},{TMP}{lift},{},1,{size},{size},0,{gap},0,1,{colors}", a.x, b.columns)
    };
    w(format!("if {LOG_VOICE} == \"\" {}:return", name(LOG_NAME, &format!("\"{LOG_NAME_COLOR}\""))));
    w(format!("add {LOG_NAME},\"　♪\""));
    w(name(LOG_NAME, &format!("\"{LOG_NAME_COLOR}\",\"#FFFFFF\"")));
    w(format!("mov {LOG_TEXT_Y},{LOG_VOICES}+{LOG_K}"));
    w(format!("mov ${LOG_TEXT_Y},{LOG_VOICE}"));
    w("return".into());
    w("*conv_log_shown".into());
    w(format!("cell {LOG_DOWN},0:cell {LOG_UP},0"));
    w(format!("if {LOG_OFFSET} == 0 cell {LOG_DOWN},2"));
    // LOG_OLDER: there is an older page to scroll to.
    w(format!("mov {TMP},{LOG_OFFSET}+1"));
    w(format!("getlog {LINE2},{TMP}"));
    w(format!("mov {LOG_OLDER},1"));
    w(format!("if {LINE2} == \"\" mov {LOG_OLDER},0:cell {LOG_UP},2"));
    w("*conv_log_wait".into());
    w(format!("if {LOG_OLDER} == 1 spbtn {LOG_UP},1"));
    w(format!("if {LOG_OFFSET} > 0 spbtn {LOG_DOWN},2"));
    w(format!("spbtn {CLOSE},{BTN_CLOSE}"));
    for k in 0..LOG_PAGES {
        w(format!("if ${} != \"\" spbtn {},{}", LOG_VOICES + k, LOG_NAME_SPRITE + k, BTN_LOG_VOICE + k));
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_CLOSE} goto *conv_log_close"));
    w(format!(
        "if {BUTTON_VAR} >= {BTN_LOG_VOICE} && {BUTTON_VAR} < {} mov {TMP},{BUTTON_VAR}-{BTN_LOG_VOICE}+{LOG_VOICES}:dwave 0,${TMP}:goto *conv_log_wait",
        BTN_LOG_VOICE + LOG_PAGES
    ));
    w(format!("if {BUTTON_VAR} == -3 && {LOG_OFFSET} == 0 && {} == 1 goto *conv_log_wait", switch_off(Switch::WheelClosesLog)));
    w(format!("if {BUTTON_VAR} == -2 mov {BUTTON_VAR},1"));
    w(format!("if {BUTTON_VAR} == -3 mov {BUTTON_VAR},2"));
    w(format!("if {BUTTON_VAR} == 1 && {LOG_OLDER} == 1 add {LOG_OFFSET},1:goto *conv_log_draw"));
    w(format!("if {BUTTON_VAR} == 2 && {LOG_OFFSET} == 0 goto *conv_log_close"));
    w(format!("if {BUTTON_VAR} == 2 sub {LOG_OFFSET},1:goto *conv_log_draw"));
    w("goto *conv_log_wait".into());
    w("*conv_log_close".into());
    w(format!("if {IN_GAME} == 1 texton:{}", vsp(1)));
    w(format!("csp2 {BASE}:csp2 {FRAME}:csp2 {HEADER}"));
    w(MENU_SPRITES.map(|n| format!("csp {n}")).collect::<Vec<_>>().join(":"));
    w("print 1".into());
    w("return".into());
}
