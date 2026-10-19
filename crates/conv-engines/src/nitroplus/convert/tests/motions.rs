use super::*;

#[test]
fn a_wildcard_matches_aliases() {
    let story = r#"
scene a.nss
{
	CreateTexture("@遠４/StMR01", 100, 1236, 0, "cg/st/a.png");
	SetAlias("@遠４/StMR01", "StMR01");
	Fade("@遠４/StMR01", 0, 1000, null, true);
	Move("@StMR*", 600, @-540, @0, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    // The centre of the 1024-wide picture at x 696.
    let moved = c.ops.iter().any(|op| matches!(op, Op::SpriteMove { x: 1208, ms: 600, .. }));
    assert!(moved, "{:?}", c.ops);
}

#[test]
fn a_new_picture_moved_without_waiting_moves_from_its_start() {
    // ma00_000.nss:370-375: the pans run under the cover's 500 ms fade.
    let story = r#"
scene a.nss
{
	CreateTexture("cover", 2500, 0, 0, "cg/bg/a.jpg");
	Wait(100);
	CreateTexture("st", 600, 182, 0, "cg/st/a.png");
	Move("st", 5000, @-100, @0, null, false);
	Fade("cover", 500, 0, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    let pan = c.ops.iter().find_map(|op| match op { Op::SpriteMotion2 { keys, .. } => Some((keys[0].x, keys[1].x, keys[1].ms)), _ => None });
    assert_eq!(pan, Some((100, 0, 5000)), "{:?}", c.ops);
    let fades: Vec<_> = c.ops.iter().filter_map(|op| match op { Op::BeginTransition { fade_ms, .. } => Some(*fade_ms), _ => None }).collect();
    assert_eq!(fades.last(), Some(&500), "{:?}", c.ops);
}

#[test]
fn a_new_alias_replaces_the_old_one() {
    let story = r#"
scene a.nss
{
	CreateTexture("st", 100, 0, 0, "cg/st/a.png");
	SetAlias("st", "one");
	SetAlias("st", "two");
	Fade("st", 0, 1000, null, true);
	Move("@one", 600, @-540, @0, null, true);
	Move("@two", 300, @-540, @0, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let moves: Vec<_> = c.ops.iter().filter_map(|op| match op { Op::SpriteMove { ms, .. } => Some(*ms), _ => None }).collect();
    assert_eq!(moves, [300], "{:?}", c.ops);
}

#[test]
fn a_face_window_follows_its_frame() {
    // 続・殺戮のジャンゴ's face windows, as compared under wine: the
    // face named through a variable alias, a `*` that doesn't reach
    // it, hidden with its frame and turned edge-on with it.
    let story = r#"
function SetWindowKwL("$顔元ナット名", $nlayer, "$ファイル名")
{
	$顔ナット名 = "$顔元ナット名" + $nlayer;
	$顔窓ナット名 = "@frame03" + "/" + "$顔ナット名";
	CreateTexture("$顔窓ナット名", 101, 25, 25, "$ファイル名");
	SetAlias("$顔窓ナット名", "$顔ナット名");
}
scene a.nss
{
	CreateTexture("frame03", 100, 25, 80, "cg/st/a.png");
	SetWindowKwL("顔窓3", 1, "cg/st/b.png");
	Fade("@frame0*", 0, 0, null, true);
	Wait(100);
	Fade("@frame03", 0, 1000, null, true);
	Rotate("@顔窓31", 0, @0, @180, @0, null, true);
	Wait(100);
	Rotate("@frame03", 0, @90, @0, @0, null, true);
	Wait(100);
}
"#;
    static GAME: Game = Game { family: true, relative_children: true, ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut fake()).unwrap();
    let shown: Vec<Vec<&str>> = c.ops.iter().filter_map(|op| match op {
        Op::Sprites { changes, .. } => Some(changes.iter().filter_map(|c| c.sprite.as_ref().map(|s| s.image.as_str())).collect()),
        _ => None,
    }).collect();
    assert!(shown.iter().any(|s| s.contains(&"cg/st/b.png~x")), "{shown:?} {:?}", c.ops);
    let gone = c.ops.iter().rev().find_map(|op| match op {
        Op::Sprites { changes, .. } if !changes.is_empty() => Some(changes.iter().all(|c| c.sprite.is_none())),
        _ => None,
    });
    assert_eq!(gone, Some(true), "{:?}", c.ops);
}

#[test]
fn a_face_window_turning_in_unfolds_with_its_face() {
    // `WindowRotateR(frame, face, 500, 0,90,0, Dxl1, true)`: turned
    // edge-on at once, then back over 500 ms about the frame's centre.
    let story = r#"
scene a.nss
{
	CreateTexture("frame03", 100, 25, 80, "cg/st/a.png");
	CreateTexture("frame03/顔窓31", 101, 25, 25, "cg/st/b.png");
	Rotate("frame03", 0, @0, @-90, @0, null, true);
	Rotate("frame03", 500, @0, @90, @0, Dxl1, true);
	Wait(100);
}
"#;
    static GAME: Game = Game { family: true, relative_children: true, ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut fake()).unwrap();
    let motions: Vec<&Vec<MotionKey>> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some(keys),
        _ => None,
    }).collect();
    assert_eq!(motions.len(), 2, "{:?}", c.ops);
    for keys in &motions {
        let (first, last) = (keys[0], *keys.last().unwrap());
        // From edge-on (no width) to flat, across only; Dxl1 eases out.
        assert_eq!((first.zoom, first.zoom_y, last.zoom, last.zoom_y, last.x, last.y, last.ms), (0, 100, 100, 100, 0, 0, 500), "{keys:?}");
        assert!(keys[1].zoom > 30, "{keys:?}");
    }
    // The face starts at the frame's centre line.
    let face = motions.iter().find(|k| k[0].x != 0).unwrap_or_else(|| panic!("{motions:?}"));
    assert!(face[0].y == 0, "{face:?}");
    assert!(!c.notes.keys().any(|n| n.starts_with("Rotate over time")), "{:?}", c.notes);
}

#[test]
fn a_picture_turning_as_it_zooms_and_fades_out_does_all_three() {
    // boot.nss TitleLogo: the Nitroplus logo shot away (Zoom, Rotate
    // about x, Fade at once over 500 ms).
    let story = r#"
scene a.nss
{
	CreateTexture("logo", 100, 0, 0, "cg/a.png");
	Fade("logo", 0, 1000, null, true);
	Zoom("logo", 500, 500, 500, Dxl2, false);
	Rotate("logo", 500, @60, @0, @0, Dxl2, false);
	Fade("logo", 500, 0, null, true);
}
"#;
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &Game::PLAIN, &mut fake()).unwrap();
    let keys = c.ops.iter().find_map(|op| match op {
        Op::SpriteMotion2 { keys, .. } => Some(keys),
        _ => None,
    }).unwrap_or_else(|| panic!("{:?}", c.ops));
    let last = *keys.last().unwrap();
    // Half the size, squashed to half again up and down; gone.
    assert_eq!((last.ms, last.zoom, last.zoom_y, last.opacity, last.x, last.y), (500, 50, 25, 0, 0, 0), "{keys:?}");
}

#[test]
fn a_face_window_turning_away_squashes_out_then_goes() {
    // `WindowCloseR(frame, face, 500, 0,90,0, Dxl1, true)`.
    let story = r#"
scene a.nss
{
	CreateTexture("frame03", 100, 25, 80, "cg/st/a.png");
	CreateTexture("frame03/顔窓31", 101, 25, 25, "cg/st/b.png");
	Wait(100);
	Rotate("frame03", 500, 0, 90, 0, Dxl1, true);
	Fade("frame03", 0, 0, null, false);
	Rotate("frame03", 0, 0, 0, 0, null, true);
	Delete("frame03/顔窓31");
	Wait(100);
}
"#;
    static GAME: Game = Game { family: true, relative_children: true, ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut fake()).unwrap();
    let at = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap_or_else(|| panic!("{:?}", c.ops));
    let lasts: Vec<u32> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some(keys.last().unwrap().zoom),
        _ => None,
    }).collect();
    assert_eq!(lasts, [0, 0], "{:?}", c.ops);
    // Both waited for, then taken away.
    assert!(c.ops[at..].iter().any(|op| matches!(op, Op::Wait { ms: 500 }) || matches!(op, Op::BeginTransition { fade_ms: 500, .. })), "{:?}", c.ops);
    let gone = c.ops.iter().rev().find_map(|op| match op {
        Op::Sprites { changes, .. } if !changes.is_empty() => Some(changes.iter().filter(|c| c.sprite.is_none()).count()),
        _ => None,
    });
    assert_eq!(gone, Some(2), "{:?}", c.ops);
}

#[test]
fn a_move_onto_another_layer_is_placed_at_its_end_and_moved_from_its_start() {
    // A new picture between two shown on next layers spreads the layers
    // out again: the moving one is drawn on another.
    let story = r#"
scene a.nss
{
	CreateTexture("a", 100, 0, 0, "cg/st/a.png");
	CreateTexture("b", 133, 0, 0, "cg/st/b.png");
	Wait(100);
	CreateTexture("c", 120, 0, 0, "cg/st/c.png");
	Move("a", 500, @100, @0, null, false);
	Wait(600);
}
"#;
    let c = compile(story, LIBRARY);
    let keys = c.ops.iter().find_map(|op| match op {
        Op::SpriteMotion2 { keys, .. } => Some(keys),
        _ => None,
    }).unwrap_or_else(|| panic!("{:?}", c.ops));
    assert_eq!((keys[0].x, keys[1].x, keys[1].ms), (-100, 0, 500), "{keys:?}");
    // After its placement, in the same change.
    let placed = c.ops.iter().rposition(|op| matches!(op, Op::Sprites { .. })).unwrap();
    let moved = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap();
    assert!(placed < moved, "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("cross-fade")), "{:?}", c.notes);
}

#[test]
fn a_move_that_fades_out_is_one_motion_then_goes() {
    let story = r#"
scene a.nss
{
	CreateTexture("a", 100, 0, 0, "cg/st/a.png");
	Wait(100);
	Move("a", 500, @100, @0, null, false);
	Fade("a", 500, 0, null, true);
	Delete("a");
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let at = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap_or_else(|| panic!("{:?}", c.ops));
    let Op::SpriteMotion2 { layer, keys, .. } = &c.ops[at] else { unreachable!() };
    assert_eq!((keys[0].opacity, keys[1].opacity, keys[1].x), (256, 0, 100), "{keys:?}");
    // Taken away at the next change.
    assert!(c.ops[at..].iter().any(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|ch| ch.layer == *layer && ch.sprite.is_none()))), "{:?}", c.ops);
}

#[test]
fn a_zoom_out_of_a_picture_cut_to_the_screen_places_the_whole_end() {
    // dja001's dives: zoomed to 5x (cut to the screen), then out.
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Zoom("bg", 0, 5000, 5000, null, true);
	Wait(100);
	Zoom("bg", 500, 1000, 1000, null, false);
	Wait(600);
}
"#;
    let c = compile(story, LIBRARY);
    let at = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap_or_else(|| panic!("{:?}", c.ops));
    let Op::SpriteMotion2 { keys, .. } = &c.ops[at] else { unreachable!() };
    assert_eq!((keys[0].zoom, keys[1].zoom, keys[1].x, keys[1].y), (500, 100, 0, 0), "{keys:?}");
    assert!(matches!(&c.ops[at - 1], Op::Sprites { changes, .. } if changes.iter().any(|ch| ch.sprite.as_ref().is_some_and(|s| s.image == "cg/bg/a.jpg"))), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("cross-fade")), "{:?}", c.notes);
}

#[test]
fn a_picture_replaced_as_it_moves_is_the_new_one_moving_from_the_old_place() {
    let story = r#"
scene a.nss
{
	CreateTexture("a", 100, 0, 0, "cg/st/a.png");
	Wait(100);
	CreateTexture("a", 100, 0, 0, "cg/st/b.png");
	Move("a", 500, @100, @0, null, false);
	Wait(600);
}
"#;
    let c = compile(story, LIBRARY);
    let at = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap_or_else(|| panic!("{:?}", c.ops));
    let Op::SpriteMotion2 { keys, .. } = &c.ops[at] else { unreachable!() };
    assert_eq!((keys[0].x, keys[1].x, keys[0].zoom), (-100, 0, 100), "{keys:?}");
    assert!(matches!(&c.ops[at - 1], Op::Sprites { changes, .. } if changes.iter().any(|ch| ch.sprite.as_ref().is_some_and(|s| s.image == "cg/st/b.png"))), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("cross-fade")), "{:?}", c.notes);
}

#[test]
fn a_face_slides_with_its_frame() {
    // `WindowMoveM` moves only the frame (dja003_02): its face, placed
    // from it, moves by the engine with it, not at once to its end.
    let story = r#"
scene a.nss
{
	CreateTexture("frame03", 100, 25, 80, "cg/st/a.png");
	CreateTexture("frame03/顔窓31", 101, 25, 25, "cg/st/b.png");
	Wait(100);
	Move("frame03", 200, @-400, @0, null, false);
	Wait(300);
}
"#;
    static GAME: Game = Game { family: true, relative_children: true, ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut fake()).unwrap();
    let motions = c.ops.iter().filter(|op| matches!(op, Op::SpriteMotion2 { .. })).count();
    assert_eq!(motions, 2, "{:?}", c.ops);
}
