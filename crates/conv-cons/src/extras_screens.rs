//! The extras' CG gallery, scene replay lists and BGM room as screens the
//! engine draws (CONscripter's `screen`, COMMANDS.md "Screens"), for an
//! [`ExtrasMenu`] with `engine_screen`. Each is a Lua table computed when
//! it opens, so what is unlocked is read then from the named arrays
//! ([`crate::CG_SEEN`], [`crate::SCENE_UNLOCKED`], [`crate::PROGRESS`]).
//!
//! The scripts around them (opening, playing a scene) stay in
//! [`crate::extras`] and [`crate::replay`].

use std::fmt::Write as _;

use conv_core::ir::{CgGallery, ExtrasMenu, MovieRoom, MusicPlayer, MusicRoom, Placed, SceneReplay};

use crate::screens::{image, lua};
use crate::{replay, CG_SEEN, PROGRESS, SCENE_UNLOCKED};

/// The gallery's screen page, read when it opens (0: the first).
pub(crate) const GALLERY_PAGE: &str = "%121";

/// The files under `screens/` for the extras.
pub(crate) fn files(m: &ExtrasMenu) -> Vec<(String, String)> {
    let mut files = Vec::new();
    if let Some(g) = m.gallery.as_ref().filter(|_| m.engine_screen) {
        files.push(("gallery.lua".to_string(), gallery_lua(g, &m.background)));
    }
    if let Some(r) = &m.replay {
        files.push(("replay.lua".to_string(), replay_lua(r, &m.background)));
        files.push(("replay_list.lua".to_string(), replay_list_lua(r)));
    }
    if let Some(music) = m.music.as_ref().filter(|_| m.engine_screen) {
        files.push(("music.lua".to_string(), music_lua(music, &m.background)));
    }
    if let Some(movies) = m.movies.as_ref().filter(|_| m.engine_screen) {
        files.push(("movies.lua".to_string(), movies_lua(movies, &m.background)));
    }
    files
}

fn seen(n: u32) -> String {
    format!("var({})", lua(&CG_SEEN.at(n)))
}

/// The start of a table built with `add`: widgets added in order, some
/// only when a variable says so.
fn prelude(what: &str) -> String {
    format!("-- {what} (conscripter-convert).\nlocal w = {{}}\nlocal function add(t) w[#w + 1] = t end\n")
}

fn background(file: &str) -> String {
    format!("add{{\"image\", file={}, x=0, y=0, opaque=true}}\n", lua(file))
}

fn add(out: &mut String, widget: String) {
    writeln!(out, "add{}", widget).unwrap();
}

/// The gallery: every tab's pages are the screen's pages, one after
/// another; a tab's button shows its `selected` picture on its pages, the
/// page numbers their marker on theirs. A gallery widget per page, its
/// slots unlocked by [`CG_SEEN`], the viewer named after the tab.
fn gallery_lua(g: &CgGallery, bg: &str) -> String {
    let tabs = &g.tabs[..g.tabs.len().min(10)];
    let mut out = prelude("The CG gallery, from CgGallery");
    out.push_str(&background(bg));
    add(&mut out, image("image", &g.header, ""));
    add(&mut out, image("button", &g.exit, ", cells=2, action={\"close\"}"));
    add(&mut out, image("image", &g.page_label, ""));
    let v = &g.viewer;
    let mut first = 1;
    let mut pages = Vec::new();
    for t in tabs {
        let n = t.pages.len().clamp(1, 4);
        let (a, b) = (first, first + n - 1);
        add(&mut out, image("button", &t.button, &format!(", cells=2, current={{{a}, {b}}}, selected={}, action={{\"page\", {a}}}", lua(&t.selected))));
        add(&mut out, image("image", &t.title, &format!(", pages={{{a}, {b}}}")));
        for p in 0..t.pages.len().min(4).min(g.page_numbers.len()) {
            let page = a + p;
            let number = Placed { image: g.page_numbers[p].clone(), x: g.pages_x + g.page_dx * p as i32, y: g.pages_y, w: 0, h: 0 };
            add(&mut out, image("image", &number, &format!(", cells=2, page={page}")));
            let marker = Placed { image: g.page_marker.clone(), x: number.x + g.marker_dx, y: number.y + g.marker_dy, ..number.clone() };
            add(&mut out, image("image", &marker, &format!(", page={page}")));
            add(&mut out, image("button", &number, &format!(", cells=2, pages={{{a}, {b}}}, current={{{page}, {page}}}, action={{\"page\", {page}}}")));
        }
        for (p, slots) in t.pages.iter().take(4).enumerate() {
            pages.push((a + p, t, slots));
        }
        first = b + 1;
    }
    let button = |p: &Placed| format!("{{file={}, cells=2, x={}, y={}}}", lua(&p.image), p.x, p.y);
    let panel: Vec<String> = v.panel.iter().map(|p| format!("{{file={}, x={}, y={}}}", lua(&p.image), p.x, p.y)).collect();
    for (page, t, slots) in pages {
        let cells: Vec<String> = slots
            .iter()
            .take(25)
            .map(|s| {
                let cgs: Vec<String> = s.cgs.iter().map(|(n, file)| format!("{{file={}, seen={}}}", lua(file), seen(*n))).collect();
                format!("    {{thumb={}, cgs={{{}}}}},\n", lua(&s.thumbnail), cgs.join(", "))
            })
            .collect();
        // The thumbnail strip has its frame 4 px up and left.
        writeln!(
            out,
            "add{{\"gallery\", page={page}, x={}, y={}, dx={}, dy={}, cols={}, thumb_dx=-4, thumb_dy=-4, locked={},\n  viewer={{panel={{{}}}, name={{file={}, x={}, y={}}},\n    previous={}, next={}, hide={}, back={}}},\n  cells={{\n{}  }}}}",
            g.grid_x,
            g.grid_y,
            g.grid_dx,
            g.grid_dy,
            g.columns,
            lua(&g.locked),
            panel.join(", "),
            lua(&t.viewer_name.image),
            v.name_x,
            v.name_y,
            button(&v.previous),
            button(&v.next),
            button(&v.hide),
            button(&v.back),
            cells.concat()
        )
        .unwrap();
    }
    writeln!(out, "return {{kind=\"extras\", pages={}, page_var={}, widgets=w}}", (first - 1).max(1), lua(GALLERY_PAGE)).unwrap();
    out
}

/// The scene replay's menu of lists: a list is disabled while none of its
/// scenes is unlocked; it closes with the list's number (from 1).
fn replay_lua(r: &SceneReplay, bg: &str) -> String {
    let mut out = prelude("The scene replay's lists, from SceneReplay");
    out.push_str(&background(bg));
    add(&mut out, image("image", &r.header, ""));
    add(&mut out, image("button", &r.exit, ", cells=2, action={\"close\"}"));
    for (i, l) in r.lists.iter().take(replay::MAX_LISTS).enumerate() {
        let open: Vec<String> = l.scenes.iter().map(|s| format!("var({}) == 1", lua(&SCENE_UNLOCKED.at(s.scene)))).collect();
        let open = if open.is_empty() { "false".to_string() } else { open.join(" or ") };
        add(&mut out, image("button", &l.button, &format!(", cells=3, disabled=2, enabled=({open}), action={{\"close\", {}}}", i + 1)));
    }
    out.push_str("return {kind=\"extras\", widgets=w}\n");
    out
}

/// An open list (the one in [`replay::LIST_VAR`]): its unlocked scenes'
/// titles, "new" until replayed; a title closes with its place in the
/// list (from 1).
fn replay_list_lua(r: &SceneReplay) -> String {
    let mut out = prelude("A scene replay list, from SceneReplay; the list is var(LIST_VAR)");
    out.push_str(&background(&r.list_background));
    let max_rows = (r.rows * r.columns).min(replay::MAX_ROWS);
    writeln!(out, "local list = var({})", lua(replay::LIST_VAR)).unwrap();
    for (i, l) in r.lists.iter().take(replay::MAX_LISTS).enumerate() {
        writeln!(out, "if list == {i} then").unwrap();
        writeln!(out, "  add{}", image("image", &l.selected, "")).unwrap();
        writeln!(out, "end").unwrap();
    }
    add(&mut out, image("image", &r.frame, ""));
    add(&mut out, image("button", &r.close, ", cells=2, action={\"close\"}"));
    let lists: Vec<String> = r
        .lists
        .iter()
        .take(replay::MAX_LISTS)
        .map(|l| {
            let items: Vec<String> = l
                .scenes
                .iter()
                .take(max_rows)
                .enumerate()
                .map(|(k, s)| {
                    let new = if s.scene < replay::REPLAYED.size { format!("var({}) == 0", lua(&replay::REPLAYED.at(s.scene))) } else { "false".into() };
                    let color = if s.highlight { format!(", color={}", lua(&r.highlight_color)) } else { String::new() };
                    format!(
                        "    {{text={}, shown=var({}) == 1, new={new}{color}, action={{\"close\", {}}}}},\n",
                        lua(&s.title),
                        lua(&SCENE_UNLOCKED.at(s.scene)),
                        k + 1
                    )
                })
                .collect();
            format!("  {{\n{}  }},\n", items.concat())
        })
        .collect();
    writeln!(out, "local items = ({{\n{}}})[list + 1] or {{}}", lists.concat()).unwrap();
    // The unlocked titles fill a column of `rows`, then the next (a list
    // widget per column: its `cols` would also be the titles' width).
    writeln!(out, "local shown = {{}}\nfor _, it in ipairs(items) do if it.shown then shown[#shown + 1] = it end end").unwrap();
    writeln!(
        out,
        "for c = 0, {} do\n  local col = {{}}\n  for r = 1, {rows} do col[r] = shown[c * {rows} + r] end\n  if #col > 0 then\n    add{{\"list\", source=\"items\", x=c * {}, y=0, dy={}, rows={rows}, size={}, shadow=true, color={},\n      selector={{file={}, cells=2, x={}, y={}}}, new_mark={{file={}, x={}, y={}}}, text_x={}, text_y={}, items=col}}\n  end\nend",
        max_rows.div_ceil(r.rows.max(1)).max(1) - 1,
        r.column_dx,
        r.row_dy,
        r.font_size,
        lua(&r.text_color),
        lua(&r.selector),
        r.selector_x,
        r.row_y,
        lua(&r.new_mark),
        r.new_x,
        r.row_y,
        r.text_x,
        r.text_y,
        rows = r.rows.max(1)
    )
    .unwrap();
    out.push_str("return {kind=\"extras\", widgets=w}\n");
    out
}

/// The BGM room: a title per song (left out until its progress bit), a
/// radio on [`crate::music::PLAYING`] that plays the song when it changes.
fn music_lua(m: &MusicRoom, bg: &str) -> String {
    let mut out = prelude("The BGM room, from MusicRoom");
    out.push_str(&background(m.background.as_deref().unwrap_or(bg)));
    add(&mut out, image("image", &m.header, ""));
    add(&mut out, image("button", &m.exit, ", cells=2, action={\"close\"}"));
    for (i, s) in m.songs.iter().take(crate::music::MAX_SONGS).enumerate() {
        let widget = format!(
            "{{\"radio\", file={}, on_file={}, cells=2, x={}, y={}, var={}, value={}, action={{{}}}}}",
            lua(&s.button.image),
            lua(&s.playing),
            s.button.x,
            s.button.y,
            lua(crate::music::PLAYING),
            i + 1,
            if m.player.is_some() { format!("\"gosub\", \"*{}\"", crate::music::PLAY_SONG) } else { format!("\"bgm\", {}", lua(&s.file)) }
        );
        match s.progress {
            Some(bit) => writeln!(out, "if var({}) == 1 then add{widget} end", lua(&PROGRESS.at(bit))).unwrap(),
            None => add(&mut out, widget),
        }
    }
    if let Some(p) = &m.player {
        player(&mut out, p);
    }
    out.push_str("return {kind=\"extras\", widgets=w}\n");
    out
}

/// The music room's player bar ([`MusicPlayer`]): its buttons run
/// [`crate::music::screen_labels`]' labels; the play button is a toggle
/// on the song playing, lit while one plays (0: off), which it sets to 0
/// (stop) or -1 (play the last again); the volume a picture slider.
fn player(out: &mut String, p: &MusicPlayer) {
    use crate::music::{NEXT_SONG, PLAYING, PREVIOUS_SONG, TOGGLE, VOLUME, VOLUME_SET};
    let gosub = |label: &str| format!(", cells=2, action={{\"gosub\", \"*{label}\"}}");
    add(out, image("button", &p.previous, &gosub(PREVIOUS_SONG)));
    add(out, format!("{{\"toggle\", file={}, on_file={}, cells=2, x={}, y={}, var={}, on=0, off=-1, action={{\"gosub\", \"*{TOGGLE}\"}}}}", lua(&p.playing), lua(&p.play.image), p.play.x, p.play.y, lua(PLAYING)));
    add(out, image("button", &p.next, &gosub(NEXT_SONG)));
    let (b, k) = (&p.bar, &p.knob);
    let side = |q: &Placed| format!("{{file={}, cells=2, x={}, y={}}}", lua(&q.image), q.x, q.y);
    add(
        out,
        format!(
            "{{\"slider\", var={}, min=0, max=100, step=10, travel={}, knob={{file={}, x={}, y={}}}, track={{x={}, y={}, w={}, h={}}}, minus={}, plus={}, action={{\"gosub\", \"*{VOLUME_SET}\"}}}}",
            lua(VOLUME),
            p.knob_travel,
            lua(&k.image),
            k.x,
            k.y,
            b.x,
            b.y,
            b.w,
            b.h,
            side(&p.lower),
            side(&p.raise)
        ),
    );
}

/// The movie room: a button per movie (left out until a scene has played
/// it, unless always open) closing with its number, which
/// [`crate::movies::screen_labels`] plays.
fn movies_lua(m: &MovieRoom, bg: &str) -> String {
    let mut out = prelude("The movie room, from MovieRoom");
    out.push_str(&background(m.background.as_deref().unwrap_or(bg)));
    add(&mut out, image("button", &m.exit, ", cells=2, action={\"close\"}"));
    for (i, movie) in m.movies.iter().take(crate::movies::MAX_MOVIES).enumerate() {
        let widget = image("button", &movie.button, &format!(", cells=2, action={{\"close\", {}}}", i + 1));
        if movie.open {
            add(&mut out, widget);
        } else {
            writeln!(out, "if var({}) == 1 then add{widget} end", lua(&crate::movies::SEEN.at(i as u32))).unwrap();
        }
    }
    out.push_str("return {kind=\"extras\", widgets=w}\n");
    out
}

#[cfg(test)]
mod tests {
    use conv_core::ir::{MovieRoom, MusicPlayer, MusicRoom, MusicSong, Placed, Rect, RoomMovie};

    fn placed(image: &str, x: i32) -> Placed {
        Placed { image: image.into(), x, y: 10, w: 20, h: 20 }
    }

    #[test]
    fn player_and_movie_room_become_widgets() {
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
        let room = MusicRoom { header: placed("h", 0), exit: placed("x", 0), songs: vec![song(0), song(1)], background: None, player: Some(player) };
        let lua = super::music_lua(&room, "bg");
        assert!(lua.contains("value=2, action={\"gosub\", \"*conv_music_play\"}}"), "{lua}");
        assert!(lua.contains("{\"toggle\", file=\"play_on\", on_file=\"play\", cells=2, x=130, y=10, var=\"%161\", on=0, off=-1, action={\"gosub\", \"*conv_music_toggle\"}}"), "{lua}");
        assert!(lua.contains("{\"slider\", var=\"%162\", min=0, max=100, step=10, travel=250, knob={file=\"knob\", x=225, y=10}, track={x=220, y=10, w=270, h=20}, minus={file=\"lower\", cells=2, x=200, y=10}"), "{lua}");
        let mut out = String::new();
        crate::music::screen_labels(&mut out, &room, "back", false);
        assert!(out.contains("screen \"music\"\nbgmvol 100:goto *back\n"), "{out}");
        assert!(out.contains("*conv_music_toggle\nif %161 == 0 bgmstop:return\nif %163 < 1 mov %163,1\nmov %161,%163\n*conv_music_play\nmov %163,%161:mov %105,%161-1\n"), "{out}");
        assert!(out.contains("*conv_music1\nbgmfadein 0:bgm \"bgm\\1.ogg\":return\n"), "{out}");

        let movie = |i: i32, open| RoomMovie { button: placed(&format!("m{i}"), i * 100), file: format!("movie/{i}.mp4"), open };
        let movies = MovieRoom { exit: placed("x", 0), movies: vec![movie(0, true), movie(1, false)], background: None };
        let lua = super::movies_lua(&movies, "bg");
        assert!(lua.contains("add{\"button\", file=\"m0\", x=0, y=10, cells=2, action={\"close\", 1}}\nif var(\"?conv_movie_seen[1]\") == 1 then add{\"button\", file=\"m1\", x=100, y=10, cells=2, action={\"close\", 2}} end\n"), "{lua}");
        let mut out = String::new();
        crate::movies::screen_labels(&mut out, &movies, "back");
        assert!(out.starts_with("*conv_movies\ncsp -1:screen \"movies\",%9\nif %9 < 1 || %9 > 2 csp -1:goto *back\n"), "{out}");
    }
}
