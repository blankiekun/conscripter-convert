//! [`SoundRoom`] as NScripter code, a mode of
//! [`picture_extras`](crate::picture_extras) entered at [`SOUND`].
//!
//! [`MUSIC`] is the extras' music, a global kept across games: 0 the
//! title's (playing at first), -1 none, n track n - 1. [`PLAY`] starts it;
//! the extras' menu calls it when it opens, and going back to the title
//! plays the title's music again where it differs ([`back_to_title`]). A
//! track stopped fades with `chvolfade -1` and plays on silent, so every
//! track started puts the volume back first.
//!
//! The screen is drawn again after every click (`btnwait` drops its
//! buttons); it doesn't change, so nothing flickers.

use conv_core::ir::SoundRoom;

use crate::picture_extras::pic;
use crate::{path, BUTTON_VAR, DIALOG_SPRITES, MAX_SPRITE, MENU_SPRITES};

pub(crate) const SOUND: &str = "conv_pextras_sound";
pub(crate) const MENU: &str = "conv_pextras_draw";
pub(crate) const DRAW: &str = "conv_psnd_draw";
const LOOP: &str = "conv_psnd_loop";
const WAIT: &str = "conv_psnd_wait";
/// Plays [`MUSIC`] (a subroutine).
pub(crate) const PLAY: &str = "conv_psnd_play";
/// The extras' music: 0 the title's, -1 none, n track n - 1 (a global).
pub(crate) const MUSIC: &str = "%2961";
// Sprites in the game's range (lower numbers drawn on top).
const BACKGROUND: u32 = MAX_SPRITE;
const TRACK: u32 = 900; // + track
const VIDEO: u32 = 960; // + movie
const VIDEO_CURSOR: u32 = 970; // + movie
const MARKER: u32 = 880;
const MAX_TRACKS: usize = 60;
const MAX_VIDEOS: usize = 10;
/// The black over everything before and after a movie.
pub(crate) const BLACK: u32 = DIALOG_SPRITES.start;
const EXIT: u32 = MENU_SPRITES.start;
const BTN_VIDEO: usize = 101; // + movie
const BTN_EXIT: usize = 201;

/// Lines that start the extras' music when they open: the menu's own,
/// fading in, or the one from before where it isn't the title's (playing
/// already).
pub(crate) fn on_open(r: &SoundRoom) -> String {
    match r.menu_track {
        Some(t) => format!("mov {MUSIC},{}:chvolfade -1,100:bgmfadein {}:gosub *{PLAY}:bgmfadein 0", number(r, t), r.menu_fade_ms),
        None => format!("if {MUSIC} != 0 gosub *{PLAY}"),
    }
}

/// Lines that play the title's music again, going back to it, where the
/// extras' music differs.
pub(crate) fn back_to_title(r: &SoundRoom) -> String {
    format!("if {MUSIC} != 0 chvolfade -1,100:bgm \"{}\"", path(&r.tracks[r.title_track].file))
}

/// The track's number in [`MUSIC`].
pub(crate) fn number(r: &SoundRoom, i: usize) -> i32 {
    if i == r.title_track { 0 } else { i as i32 + 1 }
}

/// [`PLAY`], its lines.
pub(crate) fn play_label(r: &SoundRoom) -> String {
    let mut out = format!("*{PLAY}\nif {MUSIC} == -1 bgmstop:return\n");
    for (i, t) in r.tracks.iter().enumerate() {
        out.push_str(&format!("if {MUSIC} == {} bgm \"{}\":return\n", number(r, i), path(&t.file)));
    }
    out + "return"
}

pub(crate) fn emit(out: &mut String, r: &SoundRoom, border: u32, width: u32, height: u32) {
    if r.engine_screen {
        crate::sound_screen::labels(out, r, width, height);
        out.push_str(&play_label(r));
        out.push('\n');
        return;
    }
    assert!(r.tracks.len() <= MAX_TRACKS && r.movies.len() <= MAX_VIDEOS, "a sound room larger than its sprites");
    let mut w = lines!(out);
    // The track's number in MUSIC.
    let number = |i: usize| if i == r.title_track { 0 } else { i as i32 + 1 };
    let unlocked = |v: Option<conv_core::ir::Var>| v.map(|v| crate::var_name(v, border));
    w(format!("*{SOUND}"));
    w(format!("gosub *{DRAW}:print 10,{}", r.open_ms));
    w(format!("goto *{WAIT}"));
    w(format!("*{LOOP}"));
    w(format!("gosub *{DRAW}:print 1"));
    w(format!("*{WAIT}"));
    w(format!("btnwait {BUTTON_VAR}"));
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} goto *{MENU}"));
    for (i, _) in r.tracks.iter().enumerate() {
        w(format!("if {BUTTON_VAR} == {} goto *conv_psnd_t{i}", i + 1));
    }
    for (j, _) in r.movies.iter().enumerate() {
        w(format!("if {BUTTON_VAR} == {} goto *conv_psnd_v{j}", BTN_VIDEO + j));
    }
    w(format!("goto *{LOOP}"));
    // A track: plays, or stops if it is the one playing.
    for (i, t) in r.tracks.iter().enumerate() {
        w(format!("*conv_psnd_t{i}"));
        w(format!("if {MUSIC} == {} mov {MUSIC},-1:chvolfade -1,0,{}:goto *{LOOP}", number(i), r.stop_fade_ms));
        w(format!("mov {MUSIC},{}:chvolfade -1,100:bgm \"{}\"", number(i), path(&t.file)));
        w(format!("goto *{LOOP}"));
    }
    for (j, m) in r.movies.iter().enumerate() {
        w(format!("*conv_psnd_v{j}"));
        w(format!("chvolfade -1,0,{}:lsp {BLACK},\">{width},{height},#000000\",0,0:print 10,{}", r.movie_fade_ms, r.movie_fade_ms));
        w(format!("mpegplay \"{}\",1", path(&m.file)));
        w(format!("chvolfade -1,0:gosub *{PLAY}:chvolfade -1,100,{}", r.movie_fade_ms));
        w(format!("gosub *{DRAW}:lsp {BLACK},\">{width},{height},#000000\",0,0:print 1"));
        w(format!("csp {BLACK}:print 10,{}", r.movie_back_ms));
        w(format!("goto *{WAIT}"));
    }
    w(play_label(r));
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("lsp {BACKGROUND},\"{}\",0,0", pic(&r.background)));
    w(format!("lsp {EXIT},\":a/3,0,3;{}\",{},{}:spbtn {EXIT},{BTN_EXIT},2", path(&r.exit.image), r.exit.x, r.exit.y));
    for (i, t) in r.tracks.iter().enumerate() {
        let b = &t.button;
        let slot = TRACK + i as u32;
        let open = format!("lsp {slot},\":a/2,0,3;{}\",{},{}:spbtn {slot},{}", path(&b.image), b.x, b.y, i + 1);
        match unlocked(t.unlocked_by) {
            Some(v) => {
                w(format!("if {v} != 0 {open}"));
                if let Some(l) = &t.locked {
                    w(format!("if {v} == 0 lsp {slot},\"{}\",{},{}", pic(l), b.x, b.y));
                }
            }
            None => w(open),
        }
        if let Some(p) = &t.playing {
            w(format!("if {MUSIC} == {} lsp {slot},\"{}\",{},{}", number(i), pic(p), b.x, b.y));
        }
        w(format!(
            "if {MUSIC} == {} lsp {MARKER},\":a/{},{},0;{}\",{},{}",
            number(i),
            r.marker_cells,
            r.marker_ms,
            path(&r.marker.image),
            b.x - r.marker_dx,
            b.y
        ));
    }
    for (j, m) in r.movies.iter().enumerate() {
        let p = &m.thumbnail;
        let open = format!(
            "lsp {},\"{}\",{},{}:lsp {},\":a/2,0,3;{}\",{},{}:spbtn {},{}",
            VIDEO + j as u32,
            pic(&p.image),
            p.x,
            p.y,
            VIDEO_CURSOR + j as u32,
            path(&m.cursor),
            p.x,
            p.y,
            VIDEO_CURSOR + j as u32,
            BTN_VIDEO + j
        );
        match unlocked(m.unlocked_by) {
            Some(v) => {
                w(format!("if {v} != 0 {open}"));
                w(format!("if {v} == 0 lsp {},\"{}\",{},{}", VIDEO + j as u32, pic(&m.locked), p.x, p.y));
            }
            None => w(open),
        }
    }
    w("return".into());
}
