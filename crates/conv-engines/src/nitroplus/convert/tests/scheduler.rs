//! The NSS scheduler questions, one test each, run through the games' own
//! helpers that chain timed built-ins (Django's function.nss, reduced to
//! their built-in calls). The answers are what the snapshot oracle saw in
//! the original: a 0 ms action stops one in flight where it is; a new
//! `Move` replaces the one in flight; recreating the picture drops its
//! move; unwaited actions start at the next waiting statement.

use super::*;

/// Django's function.nss:500-519, 887-896 (later definitions win over
/// [`LIBRARY`]'s `TypeBegin`).
const HELPERS: &str = r#"
function TypeBegin("$boxtype","$textdecor")
{
	Fade("$boxtype",100,1000,null,false);
	WaitAction("$boxtype",null);
	Request("$textdecor","Enter");
	WaitAction("$textdecor",null);
}
function TypeEndFade("ナット名")
{
	Fade("ナット名",100,0,null,false);
}
function FadeFade("ナット名",描画時間, nalpha, テンポ,待ち)
{
	Fade("ナット名", 0, 0, null, true);
	Fade("ナット名", 描画時間, nalpha, テンポ, 待ち);
}
function FadeMove("ナット名",描画時間, nalpha, nx,ny, テンポ,待ち)
{
	Fade("ナット名", 描画時間, nalpha, テンポ, false);
	Move("ナット名", 描画時間, nx, ny, テンポ, 待ち);
}
"#;

fn run(body: &str) -> Compiled {
    compile(&format!("scene a.nss\n{{\n{body}\n}}\n"), &format!("{LIBRARY}{HELPERS}"))
}

/// Each motion's last key time.
fn motions(c: &Compiled) -> Vec<u32> {
    c.ops.iter().filter_map(|op| match op { Op::SpriteMotion2 { keys, .. } => keys.last().map(|k| k.ms), _ => None }).collect()
}

const SHOWN: &str = "\tCreateTexture(\"bg\", 100, 0, 0, \"cg/bg/a.jpg\");\n\tWait(0);\n\tMove(\"bg\", 16000, @320, @0, null, false);\n";

#[test]
fn fade_fade_after_an_unwaited_move_keeps_the_move() {
    // dja010_01:76-78: the helper's 0 ms fade hides the new picture; its
    // 16 s pan runs on under the 1 s fade in (#564).
    let c = run("\tCreateTexture(\"bg\", 100, -560, 0, \"cg/bg/a.jpg\");\n\tWait(0);\n\tMove(\"bg\", 16000, @320, @0, null, false);\n\tFadeFade(\"bg\", 1000, 1000, null, true);");
    assert_eq!(motions(&c), [16000], "{:?}", c.ops);
    assert!(c.notes.is_empty(), "{:?}", c.notes);
}

#[test]
#[ignore = "ours lets the 16 s move run on to its end; the exe stops it at 1/16 of the way"]
fn a_zero_ms_move_stops_the_one_in_flight_where_it_is() {
    let c = run(&format!("{SHOWN}\tWait(1000);\n\tMove(\"bg\", 0, @0, @0, null, true);\n\tWait(1000);"));
    // Stopped at 20 px of 320 (its centre at 532): moved or placed there.
    let stopped = c.ops.iter().any(|op| matches!(op, Op::SpriteMove { x, ms: 0, .. } if *x == 532) || matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|ch| ch.sprite.as_ref().is_some_and(|s| s.x == 20))));
    assert!(stopped, "{:?}", c.ops);
}

#[test]
fn a_new_move_replaces_the_one_in_flight() {
    let c = run(&format!("{SHOWN}\tWait(1000);\n\tMove(\"bg\", 2000, @0, @100, null, false);\n\tWait(3000);"));
    assert_eq!(motions(&c), [16000, 2000], "{:?}", c.ops);
    assert!(c.notes.is_empty(), "{:?}", c.notes);
}

#[test]
fn recreating_the_picture_drops_its_move() {
    let c = run(&format!("{SHOWN}\tCreateTexture(\"bg\", 100, 0, 0, \"cg/bg/b.jpg\");\n\tWait(1000);"));
    // (A motion may stay on the layer, but one that goes nowhere.)
    let travels = c.ops.iter().any(|op| matches!(op, Op::SpriteMotion2 { keys, .. } if keys.iter().any(|k| (k.x, k.y) != (keys[0].x, keys[0].y))));
    assert!(!travels, "{:?}", c.ops);
}

#[test]
fn type_begin_waits_only_for_its_box() {
    // An unwaited move goes on in the engine: the text box's 100 ms only.
    let box_ = "\tCreateColor(\"box\", 1000, 0, 0, 100, 100, \"BLACK\");\n\tFade(\"box\", 0, 0, null, true);\n";
    let c = run(&format!("\tCreateTexture(\"bg\", 100, 0, 0, \"cg/bg/a.jpg\");\n{box_}\tMove(\"bg\", 1000, @320, @0, null, false);\n\tTypeBegin(\"box\", \"$SYSTEM_present_text\");"));
    assert!(c.notes.is_empty(), "{:?}", c.notes);
    assert!(c.ops.iter().all(|op| !matches!(op, Op::BeginTransition { fade_ms, .. } if *fade_ms > 100)), "{:?}", c.ops);
    // So does an unwaited fade of a shown picture.
    let c = run(&format!("\tCreateTexture(\"bg\", 100, 0, 0, \"cg/bg/a.jpg\");\n{box_}\tWait(0);\n\tFade(\"bg\", 1000, 500, null, false);\n\tTypeBegin(\"box\", \"$SYSTEM_present_text\");"));
    assert!(c.notes.is_empty(), "{:?}", c.notes);
    assert!(motions(&c).contains(&1000) && c.ops.iter().all(|op| !matches!(op, Op::BeginTransition { fade_ms, .. } if *fade_ms > 100)), "{:?}", c.ops);
}
