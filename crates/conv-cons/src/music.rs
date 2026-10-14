//! The BGM room (see [`MusicRoom`]), reached from the extras menu. It uses
//! the extras' sprite slots; the song playing is kept in [`PLAYING`],
//! which outlives the room as the song does.

use conv_core::ir::MusicRoom;

use crate::{path, BUTTON_VAR, PROGRESS_BASE};

pub(crate) const MUSIC: &str = "conv_music";
const DRAW: &str = "conv_music_draw";
/// The song playing (1-based; 0 = none).
pub(crate) const PLAYING: &str = "%161";
const HEADER: u32 = 100;
const EXIT: u32 = 101;
const SONG: u32 = 102; // + song
const MAX_SONGS: usize = 98;
const BTN_EXIT: i32 = 99;

/// `background`: the extras menu's; `back`: the extras menu's label.
pub(crate) fn music(out: &mut String, m: &MusicRoom, background: &str, back: &str) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let songs = &m.songs[..m.songs.len().min(MAX_SONGS)];
    w(format!("*{MUSIC}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(m.background.as_deref().unwrap_or(background))));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("lsp {HEADER},\":a;{}\",{},{}", path(&m.header.image), m.header.x, m.header.y));
    w(format!("lsp {EXIT},\":a/2,0,3;{}\",{},{}:spbtn {EXIT},{BTN_EXIT}", path(&m.exit.image), m.exit.x, m.exit.y));
    for (i, s) in songs.iter().enumerate() {
        let (slot, n) = (SONG + i as u32, i + 1);
        let shown = s.progress.map(|bit| format!("%{} == 1 && ", PROGRESS_BASE + bit)).unwrap_or_default();
        let (x, y) = (s.button.x, s.button.y);
        w(format!("if {shown}{PLAYING} != {n} lsp {slot},\":a/2,0,3;{}\",{x},{y}:spbtn {slot},{n}", path(&s.button.image)));
        w(format!("if {shown}{PLAYING} == {n} lsp {slot},\":a/2,0,3;{}\",{x},{y}:spbtn {slot},{n}", path(&s.playing)));
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} csp -1:goto *{back}"));
    // The song playing goes on.
    w(format!("if {BUTTON_VAR} == {PLAYING} goto *{DRAW}"));
    w(format!("if {BUTTON_VAR} < 1 || {BUTTON_VAR} > {} goto *{DRAW}", songs.len()));
    w(format!("mov {PLAYING},{BUTTON_VAR}:sub {BUTTON_VAR},1"));
    w(format!("tablegoto {BUTTON_VAR},{}", (0..songs.len()).map(|i| format!("*conv_music{i}")).collect::<Vec<_>>().join(",")));
    for (i, s) in songs.iter().enumerate() {
        w(format!("*conv_music{i}"));
        w(format!("bgmfadein 0:bgm \"{}\":goto *{DRAW}", path(&s.file)));
    }
}
