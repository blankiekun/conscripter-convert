//! The movie room's script (see [`MovieRoom`]; the screen is
//! [`crate::extras_screens`]'s `movies.lua`): a movie is shown once a
//! scene has played it ([`SEEN`] at its place in the room, set by [`seen`]).

use conv_core::ir::{MovieRoom, Script};

use crate::{music, path, Flags, BUTTON_VAR};

pub(crate) const MOVIES: &str = "conv_movies";
/// Movie n of the room was played.
pub(crate) const SEEN: Flags = Flags { name: "conv_movie_seen", size: MAX_MOVIES as u32 };
pub(crate) const MAX_MOVIES: usize = 32;

/// What playing `file` sets: its seen flag, when the room has it.
pub(crate) fn seen(script: &Script, file: &str) -> Option<String> {
    let room = script.extras.as_ref()?.movies.as_ref()?;
    let n = room.movies.iter().take(MAX_MOVIES).position(|m| m.file == file)?;
    Some(format!("mov {},1", SEEN.at(n as u32)))
}

/// The room as the engine's screen (`screens/movies.lua`,
/// [`crate::extras_screens`]): it closes with the movie to play (from 1),
/// or 0 to go back.
pub(crate) fn screen_labels(out: &mut String, m: &MovieRoom, back: &str) {
    let movies = &m.movies[..m.movies.len().min(MAX_MOVIES)];
    out.push_str(&format!("*{MOVIES}\ncsp -1:screen \"movies\",{BUTTON_VAR}\n"));
    out.push_str(&format!("if {BUTTON_VAR} < 1 || {BUTTON_VAR} > {} csp -1:goto *{back}\n", movies.len()));
    out.push_str(&format!("sub {BUTTON_VAR},1\ntablegoto {BUTTON_VAR},{}\n", (0..movies.len()).map(|i| format!("*conv_movie{i}")).collect::<Vec<_>>().join(",")));
    for (i, movie) in movies.iter().enumerate() {
        // The music room's song stops with the movie.
        out.push_str(&format!("*conv_movie{i}\nbgmstop:mov {},0:mpegplay \"{}\",1:goto *{MOVIES}\n", music::PLAYING, path(&movie.file)));
    }
}

