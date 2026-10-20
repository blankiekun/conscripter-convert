//! [`SoundRoom`] drawn by the engine (`screen "sound"`, COMMANDS.md
//! "Screens") when it has [`SoundRoom::engine_screen`]: the picture, the
//! track buttons (a locked one's picture in its place), the marker by the
//! track playing (its cells in turn), the movies' thumbnails under their
//! cursor buttons, the exit. A click closes the screen holding its frame
//! with its value; the script plays or stops the track, or plays the
//! movie, and opens the screen again (at once after a track, fading from
//! black after a movie, over [`FADE`]'s value).

use std::fmt::Write as _;

use conv_core::ir::SoundRoom;

use crate::picture_gallery::FADE;
use crate::picture_sound::{number, BLACK, DRAW, MENU, MUSIC, PLAY, SOUND};
use crate::screens::lua;
use crate::{path, var_name, BUTTON_VAR};

/// A movie's value: this plus its index.
const MOVIE: usize = 101;

/// [`SOUND`], [`DRAW`] and what a click runs.
pub(crate) fn labels(out: &mut String, r: &SoundRoom, width: u32, height: u32) {
    writeln!(out, "*{SOUND}\nmov {FADE},{}\n*{DRAW}\ncsp -1\nscreen \"sound\",{BUTTON_VAR}\nif {BUTTON_VAR} == 0 goto *{MENU}", r.open_ms).unwrap();
    for i in 0..r.tracks.len() {
        writeln!(out, "if {BUTTON_VAR} == {} goto *conv_psnd_t{i}", i + 1).unwrap();
    }
    for j in 0..r.movies.len() {
        writeln!(out, "if {BUTTON_VAR} == {} goto *conv_psnd_v{j}", MOVIE + j).unwrap();
    }
    writeln!(out, "goto *{DRAW}").unwrap();
    // A track: plays, or stops if it is the one playing; the screen again at once.
    for (i, t) in r.tracks.iter().enumerate() {
        let n = number(r, i);
        writeln!(out, "*conv_psnd_t{i}\nmov {FADE},0\nif {MUSIC} == {n} mov {MUSIC},-1:chvolfade -1,0,{}:goto *{DRAW}", r.stop_fade_ms).unwrap();
        writeln!(out, "mov {MUSIC},{n}:chvolfade -1,100:bgm \"{}\"\ngoto *{DRAW}", path(&t.file)).unwrap();
    }
    // A movie: the music down and black over the room, the movie, then the
    // music back and the room fading in from black.
    let black = format!("lsp {BLACK},\">{width},{height},#000000\",0,0");
    for (j, m) in r.movies.iter().enumerate() {
        writeln!(out, "*conv_psnd_v{j}\nchvolfade -1,0,{}:{black}:print 10,{}", r.movie_fade_ms, r.movie_fade_ms).unwrap();
        writeln!(out, "mpegplay \"{}\",1\nchvolfade -1,0:gosub *{PLAY}:chvolfade -1,100,{}", path(&m.file), r.movie_fade_ms).unwrap();
        writeln!(out, "{black}:print 1:csp {BLACK}\nmov {FADE},{}:goto *{DRAW}", r.movie_back_ms).unwrap();
    }
}

fn opaque(p: &str) -> bool {
    crate::sprite_mode(p) == 'c'
}

/// `screens/sound.lua`.
pub(crate) fn lua_file(r: &SoundRoom, border: u32) -> String {
    let mut out = format!(
        "-- The sound room (conscripter-convert, from SoundRoom).\nlocal music = var({})\nlocal widgets = {{{{\"image\", file={}, opaque={}, x=0, y=0}}}}\nlocal function add(w) widgets[#widgets + 1] = w end\n",
        lua(MUSIC),
        lua(&r.background),
        opaque(&r.background)
    );
    let unlocked = |v: Option<conv_core::ir::Var>| v.map_or("true".to_string(), |v| format!("var({}) ~= 0", lua(&var_name(v, border))));
    // From the bottom: the movies' cursors, their thumbnails, the tracks, the marker, the exit.
    for (j, m) in r.movies.iter().enumerate() {
        let p = &m.thumbnail;
        writeln!(
            out,
            "if {} then\n  add({{\"button\", file={}, cells=2, x={}, y={}, action={{\"close\", {}, hold=true}}}})\n  add({{\"image\", file={}, opaque={}, x={}, y={}}})\nelse\n  add({{\"image\", file={}, opaque={}, x={}, y={}}})\nend",
            unlocked(m.unlocked_by),
            lua(&m.cursor),
            p.x,
            p.y,
            MOVIE + j,
            lua(&p.image),
            opaque(&p.image),
            p.x,
            p.y,
            lua(&m.locked),
            opaque(&m.locked),
            p.x,
            p.y
        )
        .unwrap();
    }
    for (i, t) in r.tracks.iter().enumerate() {
        let b = &t.button;
        let n = number(r, i);
        // While it plays, its `playing` picture is the button.
        let file = match &t.playing {
            Some(p) => format!("music == {n} and {} or {}", lua(p), lua(&b.image)),
            None => lua(&b.image),
        };
        let cells = if t.playing.is_some() { format!("music == {n} and 1 or 2") } else { "2".into() };
        writeln!(out, "if {} then\n  add({{\"button\", file={file}, cells={cells}, x={}, y={}, action={{\"close\", {}, hold=true}}}})", unlocked(t.unlocked_by), b.x, b.y, i + 1).unwrap();
        if let Some(l) = &t.locked {
            writeln!(out, "else\n  add({{\"image\", file={}, opaque={}, x={}, y={}}})", lua(l), opaque(l), b.x, b.y).unwrap();
        }
        writeln!(out, "end").unwrap();
        writeln!(
            out,
            "if music == {n} then add({{\"image\", file={}, cells={}, ms={}, x={}, y={}}}) end",
            lua(&r.marker.image),
            r.marker_cells,
            r.marker_ms,
            b.x - r.marker_dx,
            b.y
        )
        .unwrap();
    }
    crate::screens::exit_footer(&mut out, &r.exit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use conv_core::ir::{Placed, RoomTrack, RoomVideo, Var};

    #[test]
    fn sound_room_becomes_a_screen() {
        let p = |name: &str, x, y| Placed { image: format!("sys/{name}.png"), x, y, w: 100, h: 20 };
        let track = |i: u16, locked: bool| RoomTrack {
            button: p("t", 100, 40 + 20 * i as i32),
            file: format!("bgm/{i}.ogg"),
            unlocked_by: locked.then_some(Var { global: true, index: i }),
            locked: locked.then(|| "sys/locked.png".into()),
            playing: None,
        };
        let r = SoundRoom {
            background: "sys/bg.jpg".into(),
            exit: p("exit", 0, 438),
            tracks: vec![track(0, false), track(1, true)],
            movies: vec![RoomVideo { thumbnail: p("mv", 600, 100), cursor: "sys/cur.png".into(), file: "movie/op.mpg".into(), unlocked_by: None, locked: "sys/locked.png".into() }],
            title_track: 0,
            menu_track: None,
            menu_fade_ms: 0,
            marker: p("marker", 0, 0),
            marker_cells: 8,
            marker_ms: 100,
            marker_dx: 30,
            open_ms: 500,
            stop_fade_ms: 1000,
            movie_fade_ms: 1000,
            movie_back_ms: 300,
            engine_screen: true,
        };
        let lua = lua_file(&r, 4000);
        for part in [
            "local music = var(\"%2961\")",
            "if var(\"%4001\") ~= 0 then\n  add({\"button\", file=\"sys/t.png\", cells=2, x=100, y=60, action={\"close\", 2, hold=true}})\nelse\n  add({\"image\", file=\"sys/locked.png\", opaque=false, x=100, y=60})\nend",
            // The title's track is number 0.
            "if music == 0 then add({\"image\", file=\"sys/marker.png\", cells=8, ms=100, x=70, y=40}) end",
            "add({\"button\", file=\"sys/cur.png\", cells=2, x=600, y=100, action={\"close\", 101, hold=true}})",
            "fade = var(\"%132\"),",
        ] {
            assert!(lua.contains(part), "missing {part:?} in\n{lua}");
        }
        let mut out = String::new();
        labels(&mut out, &r, 1024, 576);
        for part in ["*conv_pextras_sound\nmov %132,500\n*conv_psnd_draw\ncsp -1\nscreen \"sound\",%9\n", "*conv_psnd_t1\nmov %132,0\nif %2961 == 2 mov %2961,-1:chvolfade -1,0,1000:goto *conv_psnd_draw\n", "mov %132,300:goto *conv_psnd_draw\n"] {
            assert!(out.contains(part), "missing {part:?} in\n{out}");
        }
    }
}
