use super::*;

#[test]
fn a_branch_that_left_the_chapter_early_goes_on_from_where_it_left() {
    let mut files = nss_files(LIBRARY, &[]);
    // As 続・殺戮のジャンゴ's deaths: the chapter returns right after the
    // deadly answer, the caller then plays the bad end.
    let a = r#"scene a.nss
{
	call_chapter nss/b.nss;
	if($x == 1)
	{
		CreateTexture("bad", 200, 0, 0, "cg/bad.png");
		Fade("bad", 0, 1000, null, true);
		Wait(100);
		return;
	}
}"#;
    let b = r#"chapter main
{
	CreateTexture("one", 100, 0, 0, "cg/one.png");
	Fade("one", 0, 1000, null, true);
	select { case @選択肢１ { $x = 1; } case @選択肢２ { $x = 0; } }
	if($x == 1)
	{
		return;
	}
	CreateTexture("two", 300, 0, 0, "cg/two.jpg");
	Fade("two", 0, 1000, null, true);
	Wait(100);
}"#;
    files.insert("a.nss".into(), nss::parse(a).unwrap());
    files.insert("b.nss".into(), nss::parse(b).unwrap());
    let c = compile_story(&files, "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap();
    // The bad end is drawn over "one", not under the other answer's
    // "two", which it never saw.
    let bad = c.ops.iter().position(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/bad.png"))));
    assert!(bad.is_some(), "{:?}", c.ops);
    let two_shown = c.ops[..bad.unwrap()].iter().rposition(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/two.jpg"))));
    let label = c.ops[..bad.unwrap()].iter().rposition(|op| matches!(op, Op::Label(_)));
    assert!(two_shown < label, "{:?}", c.ops);
}

#[test]
fn a_music_process_plays_its_sounds_on_its_clock() {
    // 続・殺戮のジャンゴ's Play_DJ24x and PlayOWA: an intro, then its loop
    // a set time later; a jingle's second part when the first ends.
    let story = r#"
scene a.nss
{
	CreateSound("intro", BGM, "sound/bgm/intro");
	CreateSound("loop", BGM, "sound/bgm/loop");
	CreateProcess("P", 0, 0, 0, "Music");
	Request("P", Start);
	Wait(100);
}
function Music()
{
	begin:
	SoundPlay("intro", 0, 1000, false);
	WaitPlay("intro", null);
	Wait(500);
	SoundPlay("loop", 0, 1000, true);
}
function SoundPlay("ナット名", 秒数, ボリウム, ループ設定)
{
	Request("ナット名", Play);
	SetLoop("ナット名", ループ設定);
	SetVolume("ナット名", 秒数, ボリウム, null);
}
"#;
    let c = compile(story, LIBRARY);
    let later: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::BgmLater { file, after_ms, looped } => Some((file.as_str(), *after_ms, *looped)),
        _ => None,
    }).collect();
    // (Fake's sounds last 4 s.)
    assert_eq!(later, [("sound/bgm/intro.ogg", 0, false), ("sound/bgm/loop.ogg", 4500, true)], "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("CreateProcess")));
}

#[test]
fn a_music_process_decides_its_ifs_and_refuses_a_late_sound_effect() {
    // 装甲悪鬼村正's EndSong: the second song once the first has ended,
    // its fade out waited for; md01_023's ZansyuSet: a sound effect 6 s
    // in, which only the run could play (noted).
    let story = |two: &str| format!(r#"
scene a.nss
{{
	CreateSound("song", BGM, "sound/bgm/song");
	CreateSound("song2", BGM, "sound/bgm/song2");
	CreateSound("se", SE, "sound/se/a");
	$Two = {two};
	CreateProcess("P", 0, 0, 0, "Music");
	Request("P", Start);
	Wait(100);
}}
function Music()
{{
	if($Two){{
		SoundPlay("song", 0, 1000, false);
		WaitPlay("song", null);
		SetVolume("song", 1000, 0, null);
		WaitAction("song", null);
		SoundPlay("song2", 0, 1000, false);
	}}else{{
		Wait(6000);
		SoundPlay("se", 0, 1000, false);
	}}
}}
function SoundPlay("ナット名", 秒数, ボリウム, ループ設定)
{{
	Request("ナット名", Play);
	SetLoop("ナット名", ループ設定);
	SetVolume("ナット名", 秒数, ボリウム, null);
}}
"#);
    let c = compile(&story("true"), LIBRARY);
    let later: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::BgmLater { file, after_ms, .. } => Some((file.as_str(), *after_ms)),
        _ => None,
    }).collect();
    assert_eq!(later, [("sound/bgm/song.ogg", 0), ("sound/bgm/song2.ogg", 5000)], "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("CreateProcess")), "{:?}", c.notes);
    let c = compile(&story("false"), LIBRARY);
    assert!(!c.ops.iter().any(|op| matches!(op, Op::SePlay { .. })), "{:?}", c.ops);
    assert!(c.notes.contains_key("CreateProcess Music"), "{:?}", c.notes);
}

#[test]
fn an_argument_a_built_in_does_not_read_is_noted() {
    // DrawTransition's softness is read (effectsoft; its start opacity is
    // noted where the picture isn't shown at it); a process's place is harmless.
    let story = r#"
scene a.nss
{
	CreateTexture("a", 100, 0, 0, "cg/bg/a.jpg");
	Fade("a", 0, 0, null, true);
	Fade("a", 300, 1000, Axl1, true);
	DrawTransition("a", 300, 0, 1000, 100, null, "cg/bg/a.jpg", true);
	CreateProcess("P", 100, 5, 5, "Nothing");
}
function Nothing()
{
}
"#;
    let c = compile(story, LIBRARY);
    assert!(!c.notes.keys().any(|n| n.starts_with("DrawTransition argument")), "{:?}", c.notes);
    assert!(c.notes.contains_key("DrawTransition from another opacity than the picture's"), "{:?}", c.notes);
    // Fade's tempo is read: its transition goes along Axl1 (t²) where
    // the game's exe eases fades.
    assert!(!c.ops.iter().any(|op| matches!(op, Op::TransitionEase { .. })), "{:?}", c.ops);
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let eased = compile_scene(&files, "a.nss", "a.nss", &Game { fade_tempo: Some(tempo), ..Game::PLAIN }, &mut Fake).unwrap();
    assert!(eased.ops.iter().any(|op| matches!(op, Op::TransitionEase { ease: 1, exponent: 2 })), "{:?}", eased.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("Fade argument") || n.starts_with("CreateProcess argument")), "{:?}", c.notes);
}
