use super::*;

#[test]
fn a_muzzle_flash_with_a_random_pause_plays_on_paper() {
    // 装甲悪鬼村正's MFlashProcess: flashes, a 1 s pause after the 5th and
    // then every Random(4)+3 (Random is 0 when converting).
    let library = format!("{LIBRARY}{}", r#"
function MFlashProcess()
{
	begin:
	$MFlashLoop = true;
	$MFlashLoopC = 0;
	$MFlashLoopW = 5;
	$MFlashLoopWP = Random(4)+3;
	while($MFlashLoop){
		Fade("@MFP", $MFlashTime, 1000, null, true);
		Fade("@MFP", $MFlashTime, 0, null, true);
		if($MFlashLoopC==$MFlashLoopW){
			Wait(1000);
			$MFlashLoopW = $MFlashLoopW+$MFlashLoopWP;
		}
		if($MFlashLoopC==$MFlashCount){$MFlashLoop = false;}
		$MFlashLoopC++;
	}
	Delete("@MFP");
}
"#);
    let story = r#"
scene a.nss
{
	$MFlashTime = 30;
	$MFlashCount = 8;
	CreateColor("MFP", 19999, 0, 0, 1024, 576, "White");
	Fade("MFP", 0, 0, null, true);
	CreateProcess("MFlash", 150, 0, 0, "MFlashProcess");
	Request("MFlash", Start);
	Wait(5000);
}
"#;
    let c = compile(story, &library);
    let keys = c.ops.iter().find_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some(keys.clone()),
        _ => None,
    }).unwrap_or_else(|| panic!("{:?}", c.ops));
    // 9 flashes of 60 ms, pauses after the 6th and the 9th.
    let lit = keys.iter().filter(|k| k.opacity == 256).count();
    assert_eq!((lit, keys.last().map(|k| k.ms)), (9, Some(9 * 60 + 2000 + 1)), "{keys:?}");
    assert!(!c.notes.keys().any(|n| n.contains("Process")), "{:?}", c.notes);
}

#[test]
fn a_random_is_picked_at_run_time_among_its_values() {
    // MFlash() starts MFlashProcess: its Random(4)+3 is compiled once
    // per value, `rnd` choosing; a picture moved by Random over a few
    // samples, the statements reading what it set going with it.
    let library = format!("{LIBRARY}{}", r#"
function MFlash()
{
	CreateProcess("MFlash", 150, 0, 0, "MFlashProcess");
	Request("MFlash", Start);
}
function MFlashProcess()
{
	$MFlashLoopC = 0;
	$MFlashLoopW = 2;
	$MFlashLoopWP = Random(4)+3;
	while($MFlashLoopC < 8){
		Fade("@MFP", 30, 1000, null, true);
		Fade("@MFP", 30, 0, null, true);
		if($MFlashLoopC==$MFlashLoopW){
			Wait(1000);
			$MFlashLoopW = $MFlashLoopW+$MFlashLoopWP;
		}
		$MFlashLoopC++;
	}
	Delete("@MFP");
}
"#);
    let story = r#"
scene a.nss
{
	CreateColor("MFP", 19999, 0, 0, 1024, 576, "White");
	Fade("MFP", 0, 0, null, true);
	MFlash();
	Wait(5000);
	CreateColor("box", 100, 0, 0, 100, 100, "Red");
	$a = Random(5) + Random(5);
	$b = $a * 10;
	Move("box", 300, @$b, @0, null, true);
	Move("box", 300, @-$b, @0, null, true);
}
"#;
    let c = compile(story, &library);
    let picks: Vec<i32> = c.ops.iter().filter_map(|op| match op {
        Op::Random { below, .. } => Some(*below),
        _ => None,
    }).collect();
    assert_eq!(picks, vec![4, random::SAMPLES as i32], "{:?}", c.ops);
    // The pauses after the 3rd flash and then every 3rd to 6th.
    let flashes: std::collections::BTreeSet<String> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } if keys.len() > 2 => Some(format!("{keys:?}")),
        _ => None,
    }).collect();
    assert_eq!(flashes.len(), 4, "{flashes:?}");
    assert!(!c.notes.keys().any(|n| n.contains("Random")), "{:?}", c.notes);
}

#[test]
fn a_loop_after_a_move_has_the_move_as_its_lead_in() {
    // 装甲悪鬼村正's CP_HighChange: CP_HighMove slides the meter to its
    // value, CP_HighAuto waits for that in the process (not the
    // script), steps once more and wobbles; at 0 it doesn't.
    let story = r#"
scene a.nss
{
	CreateTexture("meter", 100, 0, 0, "cg/st/a.png");
	Fade("meter", 0, 1000, null, true);
	Wait(100);
	$Count = 5;
	CreateProcess("M", 15000, 0, 0, "MeterMove");
	CreateProcess("A", 15000, 0, 0, "MeterAuto");
	Request("M", Start);
	Request("A", Start);
	Wait(100);
}
function MeterMove()
{
	Move("@meter", 2000, @0, @400, null, false);
}
function MeterAuto()
{
	begin:
	if($Count==0){
	}else{
		WaitAction("@meter", null);
		Move("@meter", 500, @0, @+3, null, true);
		while(1){
			Move("@meter", 1000, @0, @-6, null, true);
			Move("@meter", 1000, @0, @+6, null, true);
		}
	}
}
"#;
    let c = compile(story, LIBRARY);
    let at = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { end: MotionEnd::LoopFrom(_), .. })).unwrap_or_else(|| panic!("{:?}", c.ops));
    let Op::SpriteMotion2 { keys, end: MotionEnd::LoopFrom(k), .. } = &c.ops[at] else { unreachable!() };
    // From 403 px up (where it was) to its place over the move, still
    // until the process's 2500 ms, then the loop.
    assert_eq!((keys[0].ms, keys[0].y, keys[1].ms, keys[1].y, *k, keys[*k].ms), (0, -403, 2000, 0, 2, 2500), "{keys:?}");
    assert_eq!(keys.last().map(|k| (k.ms, k.y)), Some((4500, 0)), "{keys:?}");
    assert!(matches!(c.ops[at + 1], Op::SpriteMove { ms: 0, .. }), "{:?}", c.ops);
    // The script waits only its own 100 ms, twice.
    let waited: u32 = c.ops.iter().map(|op| match op {
        Op::Wait { ms } | Op::Pause { ms } | Op::BeginTransition { fade_ms: ms, .. } => *ms,
        _ => 0,
    }).sum();
    assert_eq!(waited, 200, "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("CreateProcess")), "{:?}", c.notes);
}

#[test]
fn a_process_turning_pictures_turns_them_over_time_by_the_engine() {
    // 装甲悪鬼村正's CP_RollBarMove: the sky turns one way and the roll
    // bar the other, clockwise in NSS (the bar, on top, comes first);
    // WaitAction waits for both.
    let story = r#"
scene a.nss
{
	CreateTexture("sky", 100, 0, 0, "cg/bg/a.jpg");
	CreateTexture("bar", 15000, Center, Middle, "cg/st/a.png");
	Fade("sky", 0, 1000, null, true);
	Fade("bar", 0, 1000, null, true);
	Wait(100);
	$Rote = 45;
	CreateProcess("P", 15000, 0, 0, "Roll");
	Request("P", Start);
	WaitAction("P", null);
	Rotate("bar", 0, @0, @0, 30, null, true);
	Wait(100);
}
function Roll()
{
	begin:
	Rotate("@sky", 1000, @0, @0, $Rote, null, false);
	Rotate("@bar", 1000, @0, @0, -$Rote, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    let angles: Vec<i32> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some(keys[1].angle),
        _ => None,
    }).collect();
    assert_eq!(angles, [45, -45], "{:?}", c.ops);
    assert!(c.ops.iter().any(|op| matches!(op, Op::Wait { ms: 1000 })), "{:?}", c.ops);
    // Turned at once: drawn anew at its angle.
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteAngle { degrees: -30, .. })), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("CreateProcess") || n.starts_with("Rotate")), "{:?}", c.notes);
}

#[test]
fn a_one_shot_counter_flips_its_digits_on_paper() {
    // 装甲悪鬼村正's MyTr_* counters: each digit shown in turn, the one
    // before it hidden, 100 ms apart; only the process waits.
    let story = r#"
scene a.nss
{
	CreateTexture("d1", 100, 0, 0, "cg/st/1.png");
	CreateTexture("d2", 100, 0, 0, "cg/st/2.png");
	CreateTexture("d3", 100, 0, 0, "cg/st/3.png");
	Fade("d1", 0, 0, null, true);
	Fade("d2", 0, 0, null, true);
	Fade("d3", 0, 0, null, true);
	Wait(100);
	CreateProcess("P", 15000, 0, 0, "Count");
	Request("P", Start);
	Wait(100);
}
function Count()
{
	$n = 1;
	while($n < 4){
		$cur = "@d" + $n;
		$prev = "@d" + ($n - 1);
		Fade($cur, 0, 1000, null, true);
		Fade($prev, 0, 0, null, false);
		Wait(100);
		$n++;
	}
}
"#;
    let c = compile(story, LIBRARY);
    let alphas: Vec<Vec<(u32, u32)>> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some(keys.iter().map(|k| (k.ms, k.opacity)).collect()),
        _ => None,
    }).collect();
    // Shown at its turn, hidden at the next one's; the last stays.
    assert_eq!(alphas, [
        vec![(0, 0), (1, 256), (100, 256), (101, 0), (300, 0)],
        vec![(0, 0), (100, 0), (101, 256), (200, 256), (201, 0), (300, 0)],
        vec![(0, 0), (200, 0), (201, 256), (300, 256)],
    ], "{:?}", c.ops);
    let waited: u32 = c.ops.iter().map(|op| if let Op::Wait { ms } | Op::Pause { ms } = op { *ms } else { 0 }).sum();
    assert_eq!(waited, 200, "{:?}", c.ops);
    assert!(c.notes.is_empty(), "{:?}", c.notes);
}

#[test]
fn a_one_shot_zooming_unlike_in_x_and_y_keys_both() {
    // 装甲悪鬼村正's ComicProcess sign 12: flat, then up to its height.
    let story = r#"
scene a.nss
{
	CreateTexture("p", 100, 0, 0, "cg/st/a.png");
	Zoom("p", 0, 800, 0, null, true);
	Fade("p", 0, 1000, null, true);
	Wait(100);
	CreateProcess("P", 15000, 0, 0, "Up");
	Request("P", Start);
	Wait(1000);
}
function Up()
{
	Wait(1);
	Zoom("@p", 200, 800, 800, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    let keys = c.ops.iter().find_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some(keys.clone()),
        _ => None,
    }).unwrap_or_else(|| panic!("{:?}", c.ops));
    assert_eq!(keys.iter().map(|k| (k.ms, k.zoom, k.zoom_y)).collect::<Vec<_>>(), [(0, 100, 1), (1, 100, 1), (201, 100, 100)], "{:?}", c.ops);
    assert!(c.notes.is_empty(), "{:?}", c.notes);
}

#[test]
fn a_zoom_to_0_squashes_the_picture_away_and_grows_it_back_from_0() {
    // ma04_015vs: squashed across to nothing; mc02_029: grown from 0.
    let story = r#"
scene a.nss
{
	CreateTexture("p", 100, 0, 0, "cg/st/a.png");
	Fade("p", 0, 1000, null, true);
	Wait(100);
	Zoom("p", 150, 0, 2000, null, true);
	Wait(100);
	Zoom("p", 300, 1000, 1000, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let keys: Vec<Vec<(u32, u32, u32)>> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteMotion2 { keys, .. } => Some(keys.iter().map(|k| (k.ms, k.zoom, k.zoom_y)).collect()),
        _ => None,
    }).collect();
    assert_eq!(keys, [vec![(0, 100, 100), (150, 0, 200)], vec![(0, 0, 200), (300, 100, 100)]], "{:?}", c.ops);
}

#[test]
fn a_one_shot_fading_out_then_disused_goes_after_its_fade() {
    let story = r#"
scene a.nss
{
	CreateTexture("p", 100, 0, 0, "cg/st/a.png");
	Fade("p", 0, 1000, null, true);
	Wait(100);
	CreateProcess("P", 15000, 0, 0, "Out");
	Request("P", Start);
	Wait(100);
	Wait(1000);
}
function Out()
{
	Fade("@p", 500, 0, null, true);
	Request("@p", Disused);
}
"#;
    let c = compile(story, LIBRARY);
    let fade = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { end: MotionEnd::Keep, .. })).unwrap_or_else(|| panic!("{:?}", c.ops));
    let Op::SpriteMotion2 { layer, keys, .. } = &c.ops[fade] else { unreachable!() };
    assert_eq!(keys.iter().map(|k| (k.ms, k.opacity)).collect::<Vec<_>>(), [(0, 256), (500, 0), (501, 0)]);
    // Deleted at the first sync after its end, not before.
    let gone = c.ops.iter().position(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|ch| ch.layer == *layer && ch.sprite.is_none()))).unwrap_or_else(|| panic!("{:?}", c.ops));
    let waited: u32 = c.ops[fade..gone].iter().map(|op| if let Op::Wait { ms } | Op::Pause { ms } = op { *ms } else { 0 }).sum();
    assert_eq!(waited, 1100, "{:?}", c.ops);
    assert!(c.notes.is_empty(), "{:?}", c.notes);
}

#[test]
fn rain_is_a_particle_layer_zoomed_and_turned() {
    // 装甲悪鬼村正's mc01_019vsa: sideways speed lines.
    let story = r#"
scene a.nss
{
	$SYSTEM_effect_rain_dencity = 16;
	$SYSTEM_effect_rain_speed = 128;
	CreateEffect("線", 19990, Center, Middle, 288, 512, "Rain");
	Rotate("線", 0, @0, @0, @-90, null, true);
	Zoom("線", 0, 2000, 2000, null, true);
	Wait(100);
	Delete("線");
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let layers: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::ParticleLayer { layer, particles } => Some((*layer, particles.as_ref().map(|p| p.keys.clone()))),
        _ => None,
    }).collect();
    assert!(matches!(&layers[..], [(2, Some(k)), (2, None)] if k.starts_with("cw=1,ch=196,n=16,hz=24,cls=12:0:1,vy=333.333:312.5:") && k.ends_with(",ey=512,tex=288:512:0:0")), "{layers:?}");
    let sprite = c.ops.iter().find_map(|op| match op {
        Op::AnimatedSprite { sprite, cells, looped, zoom, .. } => Some((sprite.image.as_str(), sprite.x, sprite.y, *cells, *looped, *zoom)),
        _ => None,
    });
    assert_eq!(sprite, Some(("(particles 2)", 368, 32, 1, true, (200, 200))), "{:?}", c.ops);
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteAngle { degrees: 90, zoom: (200, 200), .. })), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("Rain")), "{:?}", c.notes);
}

#[test]
fn a_comic_sign_that_runs_once_shakes_on_paper_and_one_that_loops_loops() {
    // 装甲悪鬼村正's ComicProcess: one branch per sign, chosen as it starts.
    let library = format!("{LIBRARY}{}", r#"
function Comic()
{
	Begin:
	Wait(1);
	if($Sign==1){
		Fade("@c", 300, 1000, null, false);
		Shake("@c", 500, 2, 2, 0, 0, 500, null, true);
		Wait(2000);
		Fade("@c", 2000, 0, null, false);
	}else if($Sign==2){
		Fade("@c", 100, 1000, null, true);
		while(1){
			Zoom("@c", 150, 700, 700, null, true);
			Wait(100);
			Zoom("@c", 150, 600, 600, null, true);
			Wait(100);
		}
	}
}
"#);
    let story = |sign| format!(r#"
scene a.nss
{{
	CreateTexture("c", 100, 100, 100, "cg/ef/a.png");
	Fade("c", 0, 0, null, true);
	Zoom("c", 0, 500, 500, null, true);
	$Sign = {sign};
	CreateProcess("P", 150, 0, 0, "Comic");
	Request("P", Start);
	Wait(100);
}}
"#);
    let c = compile(&story(1), &library);
    let keys = c.ops.iter().find_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some(keys.clone()),
        _ => None,
    }).unwrap_or_else(|| panic!("{:?}", c.ops));
    // (ComicProcess's sign 6.) To and fro 2 px as it fades in, still
    // for 2 s, then fading out: 4.5 s in all.
    assert!(keys.iter().any(|k| k.y == 2) && keys.iter().any(|k| k.y == -2), "{keys:?}");
    assert_eq!(keys.last().map(|k| (k.ms, k.x, k.y, k.opacity)), Some((4501, 0, 0, 0)), "{keys:?}");
    assert!(c.notes.is_empty(), "{:?}", c.notes);
    let c = compile(&story(2), &library);
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteMotion2 { end: MotionEnd::Loop, .. })), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("Comic") || n.contains("paper")), "{:?}", c.notes);
}

#[test]
fn a_loop_process_rolls_in_after_a_wait_then_loops() {
    // boot.nss TumbleweedMove: still for 3 s, faded in and rolled once,
    // then rolling by again and again.
    let library = format!("{LIBRARY}{}", r#"
function Roll()
{
	Wait(3000);
	Fade("@c", 1000, 1000, null, false);
	Move("@c", 2000, @600, @0, null, true);
	$Rolling = 1;
	while($Rolling == 1)
	{
		Move("@c", 0, -100, 100, null, true);
		Move("@c", 6000, @900, @0, null, true);
		Wait(3000);
	}
}
"#);
    // Faded out before the process starts, or (TitleSet's `$Logo` 1) just after.
    for story in [
        "scene a.nss\n{\n\tCreateTexture(\"c\", 100, 100, 100, \"cg/ef/a.png\");\n\tFade(\"c\", 0, 0, null, true);\n\tCreateProcess(\"P\", 150, 0, 0, \"Roll\");\n\tRequest(\"P\", Start);\n\tWait(100);\n}\n",
        "scene a.nss\n{\n\tCreateTexture(\"c\", 100, 100, 100, \"cg/ef/a.png\");\n\tCreateProcess(\"P\", 150, 0, 0, \"Roll\");\n\tRequest(\"P\", Start);\n\tFade(\"c\", 0, 0, null, true);\n\tWait(100);\n}\n",
    ] {
        let c = compile(story, &library);
        let (keys, from) = c.ops.iter().find_map(|op| match op {
            Op::SpriteMotion2 { keys, end: MotionEnd::LoopFrom(from), .. } => Some((keys.clone(), *from)),
            _ => None,
        }).unwrap_or_else(|| panic!("{:?}", c.ops));
        let at = |ms: u32| keys.iter().find(|k| k.ms == ms).map(|k| (k.x, k.opacity)).unwrap_or_else(|| panic!("{ms}: {keys:?}"));
        assert_eq!((at(0), at(3000), at(4000), at(5000)), ((0, 0), (0, 0), (300, 256), (600, 256)), "{keys:?}");
        // The loop: from where the roll ends, back to the left and across.
        assert_eq!((keys[from].ms, keys.last().map(|k| (k.ms, k.x))), (5000, Some((14000, 700))), "{keys:?}");
        assert!(c.notes.is_empty(), "{:?}", c.notes);
    }
}

#[test]
fn a_disused_process_holds_nothing() {
    // 装甲悪鬼村正's CP_HighMove: started, disused at once, the script
    // going on; its 1 s turn runs in the engine, the 100 ms wait is 100 ms.
    let story = r#"
scene a.nss
{
	CreateTexture("bar", 15000, Center, Middle, "cg/st/a.png");
	Fade("bar", 0, 1000, null, true);
	Wait(100);
	CreateProcess("P", 15000, 0, 0, "Roll");
	Request("P", Start);
	Request("P", Disused);
	Wait(100);
}
function Roll()
{
	Rotate("@bar", 1000, @0, @0, 45, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    assert!(!c.notes.contains_key("unwaited action held the script"), "{:?} {:?}", c.notes, c.ops);
}
