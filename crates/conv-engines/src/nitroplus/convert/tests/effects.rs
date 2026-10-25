use super::*;

#[test]
fn a_sepia_effect_tints_the_pictures_below_it() {
    // dja038_03: `CreateEffect(…, "Sepia")` at 2000, faded in at once,
    // over a bust alone: the exe (crk.exe 0042db00) tints the black around
    // it too, so a black backing goes under it, tinted; an opaque
    // background filling the screen hides that backing.
    let sepia = |bg: &str| {
        let story = format!(r#"
scene a.nss
{{
	CreateTexture("bg", 100, 0, 0, "{bg}");
	CreateTexture("bar", 10000, 0, 0, "cg/bg/b.png");
	Wait(100);
	CreateEffect("効果", 2000, 0, 0, 1024, 576, "Sepia");
	Fade("効果", 0, 1000, null, true);
	Wait(100);
}}
"#);
        let c = compile(&story, LIBRARY);
        let tinted: Vec<u32> = c.ops.iter().filter_map(|op| match op {
            Op::SpriteFilters { layer, filters } if filters.iter().any(|f| f.kind == Operand::Const(conv_core::ir::FILTER_GREY_WEIGHTS)) => Some(*layer),
            _ => None,
        }).collect();
        let backing = c.ops.iter().any(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|ch| ch.sprite.as_ref().is_some_and(|s| s.image.starts_with("color/000000_")))));
        assert!(!c.notes.keys().any(|n| n.starts_with("CreateEffect Sepia")), "{:?}", c.notes);
        (tinted.len(), backing, c.ops)
    };
    let (n, backing, ops) = sepia("cg/st/bust.png");
    assert_eq!((n, backing), (2, true), "{ops:?}");
    let (n, backing, ops) = sepia("cg/bg/a.jpg");
    assert_eq!((n, backing), (1, false), "{ops:?}");
}

#[test]
fn set_tone_tints_a_picture_and_what_a_plain_cover_froze() {
    // ma02_016: `SetTone` on a picture; mb04_011vs: on a `Plain` cover.
    let grey = |c: &Compiled| -> Vec<u32> {
        c.ops.iter().filter_map(|op| match op {
            Op::SpriteFilters { layer, filters } if filters.iter().any(|f| f.kind == Operand::Const(conv_core::ir::FILTER_GREY_WEIGHTS)) => Some(*layer),
            _ => None,
        }).collect()
    };
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	CreateTexture("bar", 10000, 0, 0, "cg/bg/b.png");
	SetTone("bg", Monochrome);
	Wait(100);
}
"#;
    assert_eq!(grey(&compile(story, LIBRARY)).len(), 1);
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	Wait(100);
	CreateEffect("板", 2000, 0, 0, 1024, 576, "Plain");
	Fade("板", 0, 0, null, true);
	SetTone("板", Monochrome);
	Fade("板", 500, 1000, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    assert_eq!(grey(&c).len(), 1, "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("SetTone")), "{:?}", c.notes);
}

#[test]
fn set_shade_blurs_a_picture_by_its_passes() {
    // ma03_016: `SetShade` on a background; HEAVY is 16 passes, and a
    // second call adds to them (the exe blurs the blurred pixels).
    let blurs = |c: &Compiled| -> Vec<Operand> {
        c.ops.iter().filter_map(|op| match op {
            Op::SpriteFilters { filters, .. } => filters.iter().find(|f| f.kind == Operand::Const(conv_core::ir::FILTER_BLUR)).map(|f| f.params[0]),
            _ => None,
        }).collect()
    };
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	SetShade("bg", HEAVY);
	Wait(100);
	SetShade("bg", MEDIUM);
	Wait(100);
	SetShade("bg", NULL);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    // r(r + 1) = 2 × passes: 16 → 5, 20 → 6.
    assert_eq!(blurs(&c), [Operand::Const(5), Operand::Const(6)], "{:?}", c.ops);
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteFilters { filters, .. } if filters.is_empty())), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("SetShade")), "{:?}", c.notes);
}

#[test]
fn a_camera_shows_its_children_the_other_way_and_by_depth() {
    // 装甲悪鬼村正's ma04_005a: a picture seen through a camera, which
    // moves; at depth 500 a camera of zoom 500 shows it at its size.
    let story = r#"
scene a.nss
{
	CreateCamera("C", Center, Middle, 500);
	CreateTexture("C/bg", 500, 0, 0, "cg/bg/a.png");
	MoveCamera("C", 0, @100, @0, @0, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let placed: Vec<(i32, i32)> = c.ops.iter().filter_map(|op| match op {
        Op::Sprites { changes, .. } => changes.iter().find_map(|c| c.sprite.as_ref().map(|s| (s.x, s.y))),
        _ => None,
    }).collect();
    assert_eq!(placed.last(), Some(&(-100, 0)), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("Camera")), "{:?}", c.notes);
}

#[test]
fn a_bezier_move_goes_through_its_path() {
    // function_move.nss's FadeXH: a hop to the right (cubic, 3 pieces).
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	Wait(100);
	BezierMove("bg", 600, (@0,@0){@100,@-90}{@200,@-90}(@300,@0), null, true);
}
"#;
    let c = compile(story, LIBRARY);
    let keys = c.ops.iter().find_map(|op| match op {
        Op::SpriteMotion2 { keys, .. } => Some(keys.clone()),
        _ => None,
    }).expect("a motion");
    assert_eq!(keys.iter().map(|k| (k.ms, k.x, k.y)).collect::<Vec<_>>(), [(0, 0, 0), (200, 100, -60), (400, 200, -60), (600, 300, 0)]);
    assert!(!c.notes.keys().any(|n| n.contains("BezierMove")), "{:?}", c.notes);
}

#[test]
fn a_draw_effect_wave_is_an_engine_wave_held_at_its_end() {
    // mc04_019: HighWave 20 → 20 px at once, on a picture shown.
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	Wait(100);
	DrawEffect("bg", 0, "HighWave ", 20, 20, null);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let waves: Vec<(u32, u32, i32, i32)> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteWave { ms, period, from, to, .. } => Some((*ms, *period, *from, *to)),
        _ => None,
    }).collect();
    assert_eq!(waves, [(0, 1024, 20, 20)], "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("DrawEffect")), "{:?}", c.notes);
}

#[test]
fn a_tint_over_part_of_the_screen_is_left_out() {
    // 装甲悪鬼村正's mc01_011vsa: a 200×150 KitanoBlue box.
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	CreateEffect("効果", 2000, 120, 205, 200, 150, "KitanoBlue");
	Fade("効果", 0, 1000, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    assert!(!c.ops.iter().any(|op| matches!(op, Op::SpriteFilters { .. })), "{:?}", c.ops);
    assert!(c.notes.keys().any(|n| n == "CreateEffect KitanoBlue on part of the screen"), "{:?}", c.notes);
}

#[test]
fn a_movie_turned_over_plays_mirrored() {
    // dja003_02's blood: the exe turns a movie's quad as a picture's.
    let story = r#"
scene a.nss
{
	CreateMovie("cut", 100, 0, 0, false, false, "dx/mv剣戟.ngs");
	Rotate("cut", 0, @0, @180, @0, null, true);
	WaitAction("cut", null);
}
"#;
    let c = compile(story, LIBRARY);
    assert!(c.ops.iter().any(|op| matches!(op, Op::SceneMovie { file, .. } if file == "movie/dx/mv剣戟.ngs-mute~x")), "{:?}", c.ops);
    let last = |op: &Op| matches!(op, Op::AnimatedSprite { sprite, .. } if sprite.image.contains("Some(1934)~x"));
    assert!(c.ops.iter().any(last), "{:?}", c.ops);
}

#[test]
fn a_movie_moved_and_zoomed_over_time_keeps_playing_as_a_motion() {
    // function.nss's 背景21: a looping movie panned and zoomed.
    let story = r#"
scene a.nss
{
	CreateMovie("bg", 100, 0, 0, true, true, "dx/mvk100.ngs");
	Wait(100);
	Move("bg", 500, @100, @0, null, false);
	Zoom("bg", 500, 2000, 2000, null, false);
	Wait(600);
}
"#;
    let c = compile(story, LIBRARY);
    assert!(!c.notes.keys().any(|n| n.contains("cross-fade")), "{:?}", c.notes);
    // Placed once: the movie isn't started again.
    assert_eq!(c.ops.iter().filter(|op| matches!(op, Op::AnimatedSprite { .. })).count(), 1, "{:?}", c.ops);
    let keys = c.ops.iter().find_map(|op| match op {
        Op::SpriteMotion2 { keys, .. } => Some(keys),
        _ => None,
    }).unwrap_or_else(|| panic!("{:?}", c.ops));
    assert_eq!((keys[1].x, keys[1].zoom, keys[1].ms), (100, 200, 500), "{keys:?}");
}

#[test]
fn a_smoke_loop_plays_as_a_movie_zoomed_and_a_paused_cut_as_one_frame() {
    let story = r#"
scene a.nss
{
	CreateMovie("smoke", 300, 0, 0, true, true, "dx/mvk100.ngs");
	Zoom("smoke", 0, 3000, 5000, null, true);
	CreateMovie("cut", 400, 0, 0, false, false, "dx/mv剣戟.ngs");
	Wait(300);
	Request("cut", Pause);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    // The movie at its own size (400x216), zoomed 3x.
    let smoke = c.ops.iter().any(|op| matches!(op, Op::AnimatedSprite { cells: 1, frame_ms: 0, looped: true, zoom: (300, 500), sprite, .. } if sprite.image.ends_with("_alpha.mp4")));
    let paused = c.ops.iter().any(|op| matches!(op, Op::AnimatedSprite { cells: 1, sprite, .. } if sprite.image.contains("Some(300)")));
    assert!(smoke && paused, "{:?}", c.ops);
    assert!(!c.ops.iter().any(|op| matches!(op, Op::SceneMovie { .. })), "{:?}", c.ops);
}

#[test]
fn a_lens_is_a_screen_lens_its_loop_process_a_list_of_offsets() {
    // 装甲悪鬼村正's md05_020vs (faded to 500) and mc01_020vs's DelusionIn
    // (LensMove: a figure 8 of BezierMoves).
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	$SYSTEM_effect_lens_curvature = 1000;
	$SYSTEM_effect_lens_distance = 10;
	CreateEffect("レンズ１", 1000, -100, -300, 1200, 1200, "Lens");
	Fade("レンズ１", 0, 500, null, true);
	Wait(100);
	Delete("レンズ１");
	Wait(100);
	$SYSTEM_effect_lens_curvature = 10000;
	$SYSTEM_effect_lens_distance = 15;
	CreateEffect("レンズ１", 19000, -88, -312, 1200, 1200, "Lens");
	CreateProcess("p", 1000, 0, 0, "LensMove");
	Request("p", Start);
	Wait(100);
	Delete("p");
	Delete("レンズ１");
	Wait(100);
}
function LensMove()
{
	while(1)
	{
		BezierMove("@レンズ１", 3000, (@0,@0){@5,@-30}{@25,@-30}(@30,@0){@25,@30}{@5,@30}(@0,@0), null, true);
		BezierMove("@レンズ１", 3000, (@0,@0){@-5,@-30}{@-25,@-30}(@-30,@0){@-25,@30}{@-5,@30}(@0,@0), null, true);
	}
}
"#;
    let c = compile(story, LIBRARY);
    let lens: Vec<&Option<ScreenEffect>> = c.ops.iter().filter_map(|op| match op {
        Op::ScreenEffect { slot: 2, effect } => Some(effect),
        _ => None,
    }).collect();
    assert_eq!(lens.len(), 4, "{lens:?}");
    assert_eq!(lens[0], &Some(ScreenEffect::Lens { centre: (500, 300), diameter: 1200, curvature: 1000, distance: 10, opacity: 128, interval_ms: 16, offsets: Vec::new(), looped: false }));
    assert_eq!((lens[1], lens[3]), (&None, &None));
    let Some(ScreenEffect::Lens { centre: (512, 288), opacity: 255, offsets, looped: true, .. }) = lens[2] else { panic!("{lens:?}") };
    // 6 s a 16 ms frame; ±30 px across, ±22 down (¾ of the controls' 30).
    assert_eq!(offsets.len(), 375);
    assert_eq!(offsets.iter().map(|o| o.0).max(), Some(30));
    assert_eq!(offsets.iter().map(|o| o.1).min(), Some(-22));
    assert!(c.notes.keys().all(|n| !n.contains("Lens")), "{:?}", c.notes);
}

#[test]
fn mode_locks_and_speeds_are_set_where_they_change() {
    // 装甲悪鬼村正: each page writes text_auto_lock false; the ending roll
    // locks auto mode and runs at the normal speed; the text interval and
    // waitkey are left alone.
    let story = r#"
scene a.nss
{
	$SYSTEM_text_interval = 48;
	$SYSTEM_text_auto_lock = false;
	$SYSTEM_text_auto_lock = false;
	$SYSTEM_text_auto_lock = true;
	$SYSTEM_text_waitkey = false;
	$SYSTEM_text_interval = 128;
	#SYSTEM_play_speed = 3;
	$SYSTEM_text_waitkey = true;
	#SYSTEM_play_speed = 1;
	$SYSTEM_skip_lock = true;
	$SYSTEM_skip_lock = true;
}
"#;
    let c = compile(story, LIBRARY);
    let set: Vec<&Op> = c.ops.iter().filter(|op| matches!(op, Op::ModeLock { .. } | Op::NormalSpeed(_))).collect();
    assert_eq!(
        set,
        [
            &Op::ModeLock { mask: 2, on: false },
            &Op::ModeLock { mask: 2, on: true },
            &Op::NormalSpeed(true),
            &Op::NormalSpeed(false),
            &Op::ModeLock { mask: 1, on: true },
        ]
    );
}
