//! The BGM room's script (see [`MusicRoom`]; the screen is
//! [`crate::extras_screens`]'s `music.lua`): the song playing is kept in
//! [`PLAYING`], which outlives the room as the song does.

use conv_core::ir::{MusicRoom, Setting};

use crate::{menu, path};

pub(crate) const MUSIC: &str = "conv_music";
pub(crate) const VOLUME_SET: &str = "conv_music_volume";
/// The engine screen's player: labels its buttons run.
pub(crate) const PREVIOUS_SONG: &str = "conv_music_previous";
pub(crate) const NEXT_SONG: &str = "conv_music_next";
pub(crate) const TOGGLE: &str = "conv_music_toggle";
pub(crate) const PLAY_SONG: &str = "conv_music_play";
/// The song playing (1-based; 0 = none).
pub(crate) const PLAYING: &str = "%161";
/// The player's: the room's music volume (0-100) and the song its play
/// button plays again (1-based; 0 = the first).
pub(crate) const VOLUME: &str = "%162";
const LAST: &str = "%163";
pub(crate) const MAX_SONGS: usize = 98;

/// The room as the engine's screen (`screens/music.lua`,
/// [`crate::extras_screens`]): the player's buttons and the songs' radios
/// run these labels with the screen up.
pub(crate) fn screen_labels(out: &mut String, m: &MusicRoom, back: &str, settings: bool) {
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let Some(_) = &m.player else {
        w(format!("*{MUSIC}\ncsp -1:screen \"music\"\ngoto *{back}"));
        return;
    };
    w(format!("*{MUSIC}"));
    w("csp -1".into());
    let songs = m.songs.len().min(MAX_SONGS);
    let volume = if settings { format!("{}*10", menu::setting_var(Setting::BgmVolume)) } else { "100".into() };
    w(format!("mov {VOLUME},{volume}:if {PLAYING} > 0 mov {LAST},{PLAYING}"));
    w("screen \"music\"".into());
    // Leaving puts the settings' music volume back.
    w(format!("{}goto *{back}", if settings { format!("gosub *{}:", menu::SETTINGS_APPLY) } else { "bgmvol 100:".into() }));
    w(format!("*{PREVIOUS_SONG}"));
    w(format!("if {PLAYING} > 1 sub {PLAYING},1:goto *{PLAY_SONG}"));
    w("return".into());
    w(format!("*{NEXT_SONG}"));
    w(format!("if {PLAYING} > 0 && {PLAYING} < {songs} add {PLAYING},1:goto *{PLAY_SONG}"));
    w("return".into());
    // The play toggle set 0 (stop) or -1 (the last song again).
    w(format!("*{TOGGLE}"));
    w(format!("if {PLAYING} == 0 bgmstop:return"));
    w(format!("if {LAST} < 1 mov {LAST},1"));
    w(format!("mov {PLAYING},{LAST}"));
    w(format!("*{PLAY_SONG}"));
    w(format!("mov {LAST},{PLAYING}:mov {},{PLAYING}-1", menu::TMP));
    w(format!("tablegoto {},{}", menu::TMP, (0..songs).map(|i| format!("*conv_music{i}")).collect::<Vec<_>>().join(",")));
    for (i, s) in m.songs.iter().take(songs).enumerate() {
        w(format!("*conv_music{i}"));
        w(format!("bgmfadein 0:bgm \"{}\":return", path(&s.file)));
    }
    w(format!("*{VOLUME_SET}"));
    w(format!("if {VOLUME} < 0 mov {VOLUME},0"));
    w(format!("if {VOLUME} > 100 mov {VOLUME},100"));
    w(format!("bgmvol {VOLUME}:return"));
}
