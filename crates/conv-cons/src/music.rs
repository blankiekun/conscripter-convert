//! The BGM room (see [`MusicRoom`]), reached from the extras menu. It uses
//! the extras' sprite slots; the song playing is kept in [`PLAYING`],
//! which outlives the room as the song does.

use conv_core::ir::{MusicPlayer, MusicRoom, Setting};

use crate::{menu, path, BUTTON_VAR, PROGRESS_BASE};

pub(crate) const MUSIC: &str = "conv_music";
const DRAW: &str = "conv_music_draw";
const SONG: &str = "conv_music_song";
const VOLUME_SET: &str = "conv_music_volume";
/// The song playing (1-based; 0 = none).
pub(crate) const PLAYING: &str = "%161";
/// The player's: the room's music volume (0-100) and the song its play
/// button plays again (1-based; 0 = the first).
const VOLUME: &str = "%162";
const LAST: &str = "%163";
const HEADER: u32 = 100;
const EXIT: u32 = 101;
const SONG_SLOT: u32 = 102; // + song
const MAX_SONGS: usize = 98;
const BTN_EXIT: i32 = 99;
/// The player's sprites (after the songs') and buttons.
const PREVIOUS: u32 = 200;
const PLAY: u32 = 201;
const NEXT: u32 = 202;
const LOWER: u32 = 203;
const RAISE: u32 = 204;
const KNOB: u32 = 205;
const BAR: u32 = 206;
const BTN_PREVIOUS: i32 = 101;
const BTN_PLAY: i32 = 102;
const BTN_NEXT: i32 = 103;
const BTN_LOWER: i32 = 104;
const BTN_RAISE: i32 = 105;
const BTN_BAR: i32 = 106;

/// `background`: the extras menu's; `back`: the extras menu's label;
/// `settings`: the game has a settings menu (whose music volume the
/// player's own replaces while the room is open).
pub(crate) fn music(out: &mut String, m: &MusicRoom, background: &str, back: &str, settings: bool) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let songs = &m.songs[..m.songs.len().min(MAX_SONGS)];
    w(format!("*{MUSIC}"));
    w("csp -1".into());
    w(format!("bg \"{}\",0", path(m.background.as_deref().unwrap_or(background))));
    if m.player.is_some() {
        let volume = if settings { format!("{}*10", menu::setting_var(Setting::BgmVolume)) } else { "100".into() };
        w(format!("mov {VOLUME},{volume}:if {PLAYING} > 0 mov {LAST},{PLAYING}"));
    }
    w(format!("*{DRAW}"));
    w("csp -1".into());
    w(format!("lsp {HEADER},\":a;{}\",{},{}", path(&m.header.image), m.header.x, m.header.y));
    w(format!("lsp {EXIT},\":a/2,0,3;{}\",{},{}:spbtn {EXIT},{BTN_EXIT}", path(&m.exit.image), m.exit.x, m.exit.y));
    for (i, s) in songs.iter().enumerate() {
        let (slot, n) = (SONG_SLOT + i as u32, i + 1);
        let shown = s.progress.map(|bit| format!("%{} == 1 && ", PROGRESS_BASE + bit)).unwrap_or_default();
        let (x, y) = (s.button.x, s.button.y);
        w(format!("if {shown}{PLAYING} != {n} lsp {slot},\":a/2,0,3;{}\",{x},{y}:spbtn {slot},{n}", path(&s.button.image)));
        w(format!("if {shown}{PLAYING} == {n} lsp {slot},\":a/2,0,3;{}\",{x},{y}:spbtn {slot},{n}", path(&s.playing)));
    }
    if let Some(p) = &m.player {
        player_draw(&mut w, p);
    }
    w("print 1".into());
    w(format!("btnwait {BUTTON_VAR}"));
    // Leaving puts the settings' music volume back.
    let restore = match (&m.player, settings) {
        (None, _) => String::new(),
        (Some(_), true) => format!("gosub *{}:", menu::SETTINGS_APPLY),
        (Some(_), false) => "bgmvol 100:".into(),
    };
    w(format!("if {BUTTON_VAR} == -1 || {BUTTON_VAR} == {BTN_EXIT} csp -1:{restore}goto *{back}"));
    if let Some(p) = &m.player {
        player_buttons(&mut w, p, songs.len());
    }
    // The song playing goes on.
    w(format!("if {BUTTON_VAR} == {PLAYING} goto *{DRAW}"));
    w(format!("if {BUTTON_VAR} < 1 || {BUTTON_VAR} > {} goto *{DRAW}", songs.len()));
    if m.player.is_some() {
        w(format!("*{SONG}"));
    }
    w(format!("mov {PLAYING},{BUTTON_VAR}:sub {BUTTON_VAR},1"));
    w(format!("tablegoto {BUTTON_VAR},{}", (0..songs.len()).map(|i| format!("*conv_music{i}")).collect::<Vec<_>>().join(",")));
    for (i, s) in songs.iter().enumerate() {
        w(format!("*conv_music{i}"));
        w(format!("bgmfadein 0:bgm \"{}\":goto *{DRAW}", path(&s.file)));
    }
    if m.player.is_some() {
        volume_set(&mut w);
    }
}

fn player_draw(w: &mut impl FnMut(String), p: &MusicPlayer) {
    let button = |slot: u32, b: &conv_core::ir::Placed, image: &str, no: i32| {
        format!("lsp {slot},\":a/2,0,3;{}\",{},{}:spbtn {slot},{no}", path(image), b.x, b.y)
    };
    w(button(PREVIOUS, &p.previous, &p.previous.image, BTN_PREVIOUS));
    w(format!("if {PLAYING} == 0 {}", button(PLAY, &p.play, &p.play.image, BTN_PLAY)));
    w(format!("if {PLAYING} != 0 {}", button(PLAY, &p.play, &p.playing, BTN_PLAY)));
    w(button(NEXT, &p.next, &p.next.image, BTN_NEXT));
    w(button(LOWER, &p.lower, &p.lower.image, BTN_LOWER));
    w(button(RAISE, &p.raise, &p.raise.image, BTN_RAISE));
    let b = &p.bar;
    w(format!("lsp {BAR},\">{},{},#000000\",{},{},0:spbtn {BAR},{BTN_BAR}", b.w, b.h, b.x, b.y));
    w(format!(
        "mov {},{VOLUME}*{}/100+{}:lsp {KNOB},\":a;{}\",{},{}",
        menu::TMP,
        p.knob_travel,
        p.knob.x,
        path(&p.knob.image),
        menu::TMP,
        p.knob.y
    ));
}

fn player_buttons(w: &mut impl FnMut(String), p: &MusicPlayer, songs: usize) {
    let tmp = menu::TMP;
    w(format!("if {BUTTON_VAR} == {BTN_PREVIOUS} && {PLAYING} > 1 mov {BUTTON_VAR},{PLAYING}-1:goto *{SONG}"));
    w(format!("if {BUTTON_VAR} == {BTN_NEXT} && {PLAYING} > 0 && {PLAYING} < {songs} mov {BUTTON_VAR},{PLAYING}+1:goto *{SONG}"));
    w(format!("if {BUTTON_VAR} == {BTN_PLAY} && {PLAYING} > 0 mov {LAST},{PLAYING}:mov {PLAYING},0:bgmstop:goto *{DRAW}"));
    w(format!("if {BUTTON_VAR} == {BTN_PLAY} && {LAST} < 1 mov {LAST},1"));
    w(format!("if {BUTTON_VAR} == {BTN_PLAY} mov {BUTTON_VAR},{LAST}:goto *{SONG}"));
    w(format!("if {BUTTON_VAR} == {BTN_LOWER} sub {VOLUME},10"));
    w(format!("if {BUTTON_VAR} == {BTN_RAISE} add {VOLUME},10"));
    // A click on the bar: the volume where the knob's middle would be.
    let zero = p.knob.x + p.knob.w / 2;
    w(format!(
        "if {BUTTON_VAR} == {BTN_BAR} getmousepos {tmp},{BUTTON_VAR}:mov {VOLUME},({tmp}-{zero})*100/{}:mov {BUTTON_VAR},{BTN_BAR}",
        p.knob_travel.max(1)
    ));
    w(format!("if {BUTTON_VAR} >= {BTN_LOWER} && {BUTTON_VAR} <= {BTN_BAR} goto *{VOLUME_SET}"));
}

fn volume_set(w: &mut impl FnMut(String)) {
    w(format!("*{VOLUME_SET}"));
    w(format!("if {VOLUME} < 0 mov {VOLUME},0"));
    w(format!("if {VOLUME} > 100 mov {VOLUME},100"));
    w(format!("bgmvol {VOLUME}:goto *{DRAW}"));
}

#[cfg(test)]
mod tests {
    use conv_core::ir::{MusicPlayer, MusicRoom, MusicSong, Placed, Rect};

    fn placed(image: &str, x: i32) -> Placed {
        Placed { image: image.into(), x, y: 10, w: 20, h: 20 }
    }

    #[test]
    fn player_bar_steps_songs_and_sets_the_room_volume() {
        let song = |i: i32| MusicSong { button: placed(&format!("s{i}"), 0), playing: format!("s{i}_on"), file: format!("bgm/{i}.ogg"), progress: None };
        let player = MusicPlayer {
            previous: placed("prev", 100),
            play: placed("play", 130),
            playing: "play_on".into(),
            next: placed("next", 160),
            lower: placed("lower", 200),
            raise: placed("raise", 500),
            bar: Rect { x: 220, y: 10, w: 270, h: 20 },
            knob: placed("knob", 225),
            knob_travel: 250,
        };
        let room = MusicRoom { header: placed("h", 0), exit: placed("x", 0), songs: vec![song(0), song(1), song(2)], background: None, player: Some(player) };
        let mut out = String::new();
        super::music(&mut out, &room, "bg", "back", true);
        assert!(out.contains("bg \"bg\",0\nmov %162,%902*10:if %161 > 0 mov %163,%161\n*conv_music_draw\n"), "{out}");
        assert!(out.contains("if %161 != 0 lsp 201,\":a/2,0,3;play_on\",130,10:spbtn 201,102\n"), "{out}");
        assert!(out.contains("mov %105,%162*250/100+225:lsp 205,\":a;knob\",%105,10\n"), "{out}");
        assert!(out.contains("csp -1:gosub *conv_settings_apply:goto *back\n"), "{out}");
        assert!(out.contains("if %9 == 103 && %161 > 0 && %161 < 3 mov %9,%161+1:goto *conv_music_song\n"), "{out}");
        assert!(out.contains("if %9 == 106 getmousepos %105,%9:mov %162,(%105-235)*100/250:mov %9,106\n"), "{out}");
        assert!(out.contains("*conv_music_song\nmov %161,%9:sub %9,1\n"), "{out}");
        assert!(out.ends_with("*conv_music_volume\nif %162 < 0 mov %162,0\nif %162 > 100 mov %162,100\nbgmvol %162:goto *conv_music_draw\n"), "{out}");
    }
}
