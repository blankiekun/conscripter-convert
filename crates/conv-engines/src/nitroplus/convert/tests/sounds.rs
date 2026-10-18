use super::*;

#[test]
fn delete_all_in_a_chapter_keeps_what_was_made_before_it() {
    static GAME: Game = Game { chapter_objects: true, ..Game::PLAIN };
    let mut files = nss_files(LIBRARY, &[]);
    let a = r#"scene a.nss
{
	CreateSound("bgm1", BGM, "sound/bgm/one");
	call_chapter nss/b.nss;
	Request("bgm1", Play);
	SetLoop("bgm1", true);
	Wait(100);
}"#;
    let b = r#"chapter main
{
	CreateTexture("pic", 100, 0, 0, "cg/one.png");
	Fade("pic", 0, 1000, null, true);
	Wait(100);
	Delete("*");
	Wait(100);
}"#;
    files.insert("a.nss".into(), nss::parse(a).unwrap());
    files.insert("b.nss".into(), nss::parse(b).unwrap());
    let c = compile_story(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
    assert!(c.ops.iter().any(|op| matches!(op, Op::BgmPlay { file, .. } if file == "sound/bgm/one.ogg")), "{:?}", c.ops);
    let removed = c.ops.iter().any(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.is_none())));
    assert!(removed, "{:?}", c.ops);
}

#[test]
fn wait_play_waits_for_a_fade_to_0() {
    // A loop faded out over 1500 ms stops at the fade's end; a one-shot
    // (4000 ms) ends first.
    let story = r#"
scene a.nss
{
	CreateSound("m", BGM, "sound/se/loop_L");
	Request("m", Play);
	SetLoop("m", true);
	Wait(100);
	SetVolume("m", 1500, 0, null);
	WaitPlay("m", null);
	CreateSound("se", SE, "sound/se/a");
	Request("se", Play);
	Request("se", Disused);
	Wait(3000);
	SetVolume("se", 2000, 0, null);
	WaitPlay("se", null);
	Wait(1000);
	WaitPlay("se", null);
}
"#;
    let c = compile(story, LIBRARY);
    let waits: Vec<u32> = c.ops.iter().filter_map(|op| match op { Op::Wait { ms } | Op::Pause { ms } => Some(*ms), _ => None }).collect();
    assert_eq!(waits, [100, 1500, 3000, 1000, 1000], "{:?}", c.ops);
}

#[test]
fn play_on_a_playing_loop_doesnt_start_it_again() {
    let story = r#"
scene a.nss
{
	CreateSound("SE01", SE, "sound/se/loop_L");
	Request("SE01", Play);
	SetLoop("SE01", true);
	Wait(100);
	Request("SE01", Play);
	SetLoop("SE01", true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let plays = c.ops.iter().filter(|op| matches!(op, Op::SePlay { .. })).count();
    assert_eq!(plays, 1);
}

#[test]
fn set_frequency_bends_a_sound() {
    let story = r#"
scene a.nss
{
	CreateSound("SE10", SE, "sound/se/rain_L");
	Request("SE10", Play);
	SetFrequency("SE10", 1000, 1100, null);
	Wait(100);
	SetFrequency("SE10", 0, 1000, null);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let rates: Vec<_> = c.ops.iter().filter_map(|op| match op { Op::SoundRate { rate, ms, .. } => Some((*rate, *ms)), _ => None }).collect();
    assert_eq!(rates, [(1100, 1000), (1000, 0)], "{:?}", c.ops);
    let play = c.ops.iter().position(|op| matches!(op, Op::SePlay { .. })).unwrap();
    assert!(matches!(c.ops[play + 1], Op::SoundRate { rate: 1100, .. }), "{:?}", c.ops);
}

#[test]
fn sound_volumes_follow_the_exe_curve() {
    let story = r#"
scene a.nss
{
	CreateSound("SE01", SE, "sound/se/rain_L");
	SoundPlay("SE01", 1000, 500, true);
	Wait(100);
	SetVolume("SE01", 2000, 1, null);
	Wait(100);
	SetVolume("SE01", 500, 0, null);
	Wait(100);
}
"#;
    // function.nss's own.
    let library = r#"
function SoundPlay($ナット名,秒数,ボリウム,ループ設定)
{
	SetVolume($ナット名, 0, 1, null);
	Request($ナット名, Play);
	SetVolume($ナット名, 秒数, ボリウム, null);
	SetLoop($ナット名, ループ設定);
	Request($ナット名, Disused);
}
"#;
    let c = compile(story, library);
    let file = Some("sound/se/rain_L.ogg".to_string());
    let sound: Vec<_> = c.ops.iter().filter(|op| matches!(op, Op::SeFadeIn { .. } | Op::SoundVolume { .. } | Op::SePlay { .. } | Op::SeFadeOut { .. })).collect();
    assert_eq!(
        sound,
        [
            &Op::SeFadeIn { ms: 1000 },
            &Op::SoundVolume { file: file.clone(), percent: 32, ms: 0 },
            &Op::SePlay { file: "sound/se/rain_L.ogg".into(), looped: true },
            &Op::SoundVolume { file: file.clone(), percent: 0, ms: 2000 },
            &Op::SeFadeOut { file, ms: 500 },
        ]
    );
}
