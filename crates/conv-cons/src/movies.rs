//! The movie room (see [`MovieRoom`]), reached from the extras menu. It
//! uses the extras' sprite slots; a movie is shown once a scene has played
//! it ([`SEEN_BASE`] + its place in the room, set by [`seen`]).

use conv_core::ir::{MovieRoom, Script};

use crate::{music, path, BUTTON_VAR};

pub(crate) const MOVIES: &str = "conv_movies";
const DRAW: &str = "conv_movies_draw";
/// Global flags: movie n of the room was played; above the CGs' flags.
const SEEN_BASE: u32 = 2800;
const MAX_MOVIES: usize = 32;
const EXIT: u32 = 101;
const MOVIE: u32 = 102; // + movie
const BTN_EXIT: i32 = 99;

/// What playing `file` sets: its seen flag, when the room has it.
pub(crate) fn seen(script: &Script, file: &str) -> Option<String> {
    let room = script.extras.as_ref()?.movies.as_ref()?;
    let n = room.movies.iter().take(MAX_MOVIES).position(|m| m.file == file)?;
    Some(format!("mov %{},1", SEEN_BASE + n as u32))
}

/// `background`: the extras menu's; `back`: the extras menu's label.
pub(crate) fn movies(out: &mut String, m: &MovieRoom, background: &str, back: &str) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let movies = &m.movies[..m.movies.len().min(MAX_MOVIES)];
    w(format!("*{MOVIES}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(m.background.as_deref().unwrap_or(background))));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("lsp {EXIT},\":a/2,0,3;{}\",{},{}:spbtn {EXIT},{BTN_EXIT}", path(&m.exit.image), m.exit.x, m.exit.y));
    for (i, movie) in movies.iter().enumerate() {
        let (slot, n) = (MOVIE + i as u32, i + 1);
        let shown = if movie.open { String::new() } else { format!("if %{} == 1 ", SEEN_BASE + i as u32) };
        let b = &movie.button;
        w(format!("{shown}lsp {slot},\":a/2,0,3;{}\",{},{}:spbtn {slot},{n}", path(&b.image), b.x, b.y));
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} csp -1:goto *{back}"));
    w(format!("if {BUTTON_VAR} < 1 || {BUTTON_VAR} > {} goto *{DRAW}", movies.len()));
    w(format!("sub {BUTTON_VAR},1"));
    w(format!("tablegoto {BUTTON_VAR},{}", (0..movies.len()).map(|i| format!("*conv_movie{i}")).collect::<Vec<_>>().join(",")));
    for (i, movie) in movies.iter().enumerate() {
        w(format!("*conv_movie{i}"));
        // The music room's song stops with the movie.
        w(format!("bgmstop:mov {},0:mpegplay \"{}\",1:goto *{DRAW}", music::PLAYING, path(&movie.file)));
    }
}
