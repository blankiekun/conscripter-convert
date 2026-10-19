use super::*;

#[test]
fn set_pan_pans_a_sound_from_its_start_or_over_time() {
    let story = r#"
scene a.nss
{
	CreateSound("se", SE, "sound/se/a");
	MusicStart("se", 0, 1000, -1000, 1000, null, false);
	Wait(100);
	SetPan("se", 500, 1000, null);
}
"#;
    // As the games' MusicStart: its 4th argument is the pan.
    let music_start = r#"
function MusicStart("ナット名",秒数,ボリウム,再生方向,再生スピード,テンポ,ループ設定)
{
	Request("ナット名", "Play");
	SetPan("ナット名", 0, 再生方向, NULL);
	SetLoop("ナット名", ループ設定);
	SetVolume("ナット名", 秒数, ボリウム, テンポ);
}
"#;
    let c = compile(story, &format!("{LIBRARY}{music_start}"));
    let pans: Vec<(i32, u32)> = c.ops.iter().filter_map(|op| match op { Op::SoundPan { pan, ms, .. } => Some((*pan, *ms)), _ => None }).collect();
    assert_eq!(pans, [(-1000, 0), (1000, 500)], "{:?}", c.ops);
    let first = c.ops.iter().position(|op| matches!(op, Op::SoundPan { .. })).unwrap();
    assert!(matches!(c.ops[first + 1], Op::SePlay { .. }), "{:?}", c.ops);
}

#[test]
fn wait_play_waits_for_the_rest_of_a_sound_or_its_limit() {
    // The fake sounds last 4000 ms; 1000 have passed at each WaitPlay,
    // whose limit counts from the WaitPlay.
    let story = r#"
scene a.nss
{
	CreateSound("se", SE, "sound/se/a");
	Request("se", Play);
	Wait(1000);
	WaitPlay("se", null);
	CreateSound("se2", SE, "sound/se/b");
	Request("se2", Play);
	Wait(1000);
	WaitPlay("se2", 2500);
	CreateSound("se3", SE, "sound/se/c");
	Request("se3", Play);
	Request("se3", Disused);
	Wait(500);
	WaitPlay("se*", null);
	WaitPlay("se3", null);
}
"#;
    // A disused one-shot is still waited for until its end.
    let c = compile(story, LIBRARY);
    let waits: Vec<u32> = c.ops.iter().filter_map(|op| match op { Op::Wait { ms } | Op::Pause { ms } => Some(*ms), _ => None }).collect();
    assert_eq!(waits, [1000, 3000, 1000, 2500, 500, 3500], "{:?}", c.ops);
}

#[test]
fn wait_action_on_one_object_waits_only_for_its_actions() {
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Fade("bg", 0, 1000, null, true);
	CreateTexture("hp", 200, 0, 0, "cg/cp/hp.png");
	Zoom("bg", 15000, 1200, 1200, null, false);
	Fade("hp", 300, 500, null, false);
	WaitAction("hp", null);
	Wait(500);
}
"#;
    let c = compile(story, LIBRARY);
    let longest = c.ops.iter().filter_map(|op| match op {
        Op::BeginTransition { fade_ms, .. } => Some(*fade_ms),
        Op::Wait { ms } | Op::Pause { ms } => Some(*ms),
        _ => None,
    }).max();
    assert!(longest.is_some_and(|ms| ms < 15000), "{:?}", c.ops);
}

#[test]
fn draw_does_not_wait_for_running_actions() {
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Fade("bg", 5000, 500, null, false);
	Draw();
}
"#;
    let c = compile(story, LIBRARY);
    let longest = c.ops.iter().filter_map(|op| match op {
        Op::BeginTransition { fade_ms, .. } => Some(*fade_ms),
        Op::Wait { ms } | Op::Pause { ms } => Some(*ms),
        _ => None,
    }).max();
    assert_eq!(longest, Some(UNWAITED_MS), "{:?}", c.ops);
}

#[test]
fn a_fade_that_changes_nothing_does_not_wait() {
    // A new picture is at 1000 (255): fading it to 1000 starts no action.
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Fade("bg", 700, 1000, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    let waits = |c: &Compiled| c.ops.iter().any(|op| matches!(op, Op::BeginTransition { fade_ms: 700, .. } | Op::Wait { ms: 700 } | Op::Pause { ms: 700 }));
    assert!(!waits(&c), "{:?}", c.ops);
    let c = compile(&story.replace("Fade(\"bg\", 700", "Fade(\"bg\", 0, 0, null, true);\n\tFade(\"bg\", 700"), LIBRARY);
    assert!(waits(&c), "{:?}", c.ops);
}

#[test]
fn a_hidden_pictures_unwaited_move_goes_on() {
    // FadeFade (dja010_01:76-78): hidden at once, then faded in while it pans.
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, -560, 0, "cg/bg/a.jpg");
	Wait(0);
	Move("bg", 16000, @320, @0, null, false);
	Fade("bg", 0, 0, null, true);
	Fade("bg", 1000, 1000, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    let fades: Vec<u32> = c.ops.iter().filter_map(|op| match op { Op::BeginTransition { fade_ms, .. } => Some(*fade_ms), _ => None }).collect();
    // (The hide at once, not over the move's time: one 1 s fade, the last.)
    assert_eq!((fades.iter().filter(|&&f| f == 1000).count(), fades.last()), (1, Some(&1000)), "{:?}", c.ops);
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteMotion2 { keys, .. } if keys.iter().any(|k| k.ms == 16000))), "{:?}", c.ops);
}

#[test]
fn time_ledger_notes_a_sync_held_by_an_unwaited_action() {
    // Draw holds the script for the running fade (the exe goes on).
    let held = |wait: &str| {
        let story = format!("scene a.nss\n{{\n\tCreateTexture(\"bg\", 100, 0, 0, \"cg/bg/a.jpg\");\n\tFade(\"bg\", 5000, 500, null, {wait});\n\tDraw();\n}}\n");
        compile(&story, LIBRARY).notes.contains_key("unwaited action held the script")
    };
    assert!(held("false"));
    assert!(!held("true"));
}

#[test]
fn time_ledger_notes_a_motion_dropped_by_a_sync() {
    // shortcut: a turned picture cut to the screen (zoomed past 4096 px)
    // that moves unwaited stays cut to the whole screen, so its move ends
    // at once (mb04_025vs:2987); a fix of that case needs another one here.
    let story = r#"
scene a.nss
{
	CreateColor("white", 300, 0, 0, 1024, 576, "WHITE");
	CreateTexture("ef", 200, Center, Middle, "cg/bg/a.jpg");
	Zoom("ef", 0, 20000, 20000, null, true);
	Rotate("ef", 0, @0, @0, @-90, null, true);
	Move("ef", 400, @0, @1000, null, false);
	Fade("white", 100, 0, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    assert!(c.notes.contains_key("timed action dropped by a sync"), "{:?} {:?}", c.notes, c.ops);
}
