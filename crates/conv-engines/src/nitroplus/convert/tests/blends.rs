use super::*;

#[test]
fn a_picture_turned_about_y_shows_mirrored() {
    let story = r#"
scene a.nss
{
	CreateTexture("st", 100, 0, 0, "cg/st/a.png");
	Rotate("st", 0, @0, @180, @0, null, true);
	Fade("st", 0, 1000, null, true);
	Wait(100);
	Rotate("st", 0, @0, @180, @0, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let images: Vec<&str> = c.ops.iter().flat_map(|op| match op {
        Op::Sprites { changes, .. } => changes.iter().filter_map(|c| c.sprite.as_ref().map(|s| s.image.as_str())).collect(),
        _ => Vec::new(),
    }).collect();
    // Turned once: mirrored; twice: back as it was.
    assert_eq!(images.first(), Some(&"cg/st/a.png~x"), "{:?}", c.ops);
    assert_eq!(images.last(), Some(&"cg/st/a.png"), "{:?}", c.ops);
    // A turn in its plane is the engine's, counterclockwise; with x 180
    // as well, upside down too.
    for (turn, image) in [("@0, @0, @180", "cg/st/a.png"), ("180, 0, 180", "cg/st/a.png~y")] {
        let story = format!("scene a.nss\n{{\n\tCreateTexture(\"st\", 100, 0, 0, \"cg/st/a.png\");\n\tRotate(\"st\", 0, {turn}, null, true);\n\tFade(\"st\", 0, 1000, null, true);\n\tWait(100);\n}}\n");
        let c = compile(&story, LIBRARY);
        let shown = c.ops.iter().any(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == image))));
        assert!(shown, "{turn}: {:?}", c.ops);
        assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteAngle { degrees: -180, .. })), "{turn}: {:?}", c.ops);
    }
}

#[test]
fn render_requests_are_sprite_blends() {
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	CreateColor("light", 200, 0, 0, 1024, 576, "White");
	Request("light", AddRender);
	Wait(100);
	Request("light", MulRender);
	Wait(100);
	Request("light", NormalRender);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let modes: Vec<u8> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteBlend { mode, .. } => Some(*mode),
        _ => None,
    }).collect();
    assert_eq!(modes, [BLEND_ADD, BLEND_MULTIPLY, BLEND_NORMAL], "{:?}", c.ops);
    // A blend on a shown picture doesn't draw it again.
    let placed = c.ops.iter().filter(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image.contains("ffffff"))))).count();
    assert_eq!(placed, 1, "{:?}", c.ops);
}

#[test]
fn moves_and_zooms_over_time_are_engine_motions_during_the_other_fades() {
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	CreateColor("white", 200, 0, 0, 1024, 576, "White");
	Fade("white", 0, 1000, null, true);
	Wait(100);
	Move("bg", 1500, @0, @-150, Dxl1, false);
	Fade("white", 300, 0, null, true);
	Zoom("bg", 1000, 2000, 2000, DxlAxl, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let motions: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteMotion2 { keys, end: MotionEnd::Keep, .. } => Some((keys[1].ms, keys[1].x, keys[1].y, keys[1].zoom)),
        _ => None,
    }).collect();
    // The pan goes on through the white's 300 ms fade, which takes only
    // its own time; the zoom is about the centre.
    assert_eq!(motions, [(1500, 0, -150, 100), (1000, 0, 0, 200)], "{:?}", c.ops);
    let pan = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap();
    assert!(matches!(c.ops[pan + 1], Op::BeginTransition { fade_ms: 300, .. }), "{:?}", c.ops);
    assert!(c.ops.iter().any(|op| matches!(op, Op::Wait { ms: 1000 })), "{:?}", c.ops);
    // Each along its tempo's curve.
    let eases: Vec<_> = c.ops.windows(2).filter_map(|w| match w {
        [Op::MoveEase { ease, exponent }, Op::SpriteMotion2 { .. }] => Some((*ease, *exponent)),
        _ => None,
    }).collect();
    assert_eq!(eases, [(2, 2), (7, 1)], "{:?}", c.ops);
}

#[test]
fn a_move_that_also_fades_is_one_motion_with_alpha_keys() {
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Fade("bg", 0, 200, null, true);
	Wait(100);
	Move("bg", 1000, @100, @0, null, false);
	Fade("bg", 1000, 1000, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let at = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { .. })).unwrap_or_else(|| panic!("{:?}", c.ops));
    let Op::SpriteMotion2 { keys, .. } = &c.ops[at] else { unreachable!() };
    assert_eq!((keys[0].opacity, keys[1].opacity, keys[1].x), (51, 256, 100), "{:?}", c.ops);
    // The sprite at the larger opacity, the keys times it.
    assert!(matches!(c.ops[at - 1], Op::SpriteOpacity { opacity: 256, .. }), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("cross-fade")), "{:?}", c.notes);
}

#[test]
fn set_blur_draws_nothing_on_a_still_picture_and_a_trail_behind_a_moving_one() {
    let story = r#"
scene a.nss
{
	CreateTexture("st", 100, 0, 0, "cg/st/a.png");
	SetBlur("st", true, 2, 500, 50, false);
	Fade("st", 0, 1000, null, true);
	Wait(100);
	Move("st", 1000, @-100, @0, null, false);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    // The picture itself, no other copy of it.
    let drawn = c.ops.iter().filter(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.is_some()))).collect::<Vec<_>>();
    assert!(matches!(drawn[..], [Op::Sprites { changes, .. }] if changes[0].sprite.as_ref().unwrap().image == "cg/st/a.png"), "{:?}", c.ops);
    let trails: Vec<_> = c.ops.iter().filter(|op| matches!(op, Op::SpriteTrail { .. })).collect();
    assert!(matches!(trails[..], [Op::SpriteTrail { passes: 2, alpha: 500, lag: 50, over: true, .. }]), "{:?}", c.ops);
}

#[test]
fn a_picture_under_an_opaque_full_screen_one_isnt_drawn() {
    let story = r#"
scene a.nss
{
	CreateTexture("old", 100, 0, 0, "cg/bg/a.jpg");
	Fade("old", 0, 1000, null, true);
	Wait(100);
	CreateTexture("new", 200, 0, 0, "cg/bg/b.jpg");
	Fade("new", 500, 1000, null, true);
	Wait(100);
	Fade("new", 500, 0, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    // Shown, covered (removed once the cover is up), shown again.
    let old: Vec<_> = c.ops.iter().flat_map(|op| match op {
        Op::Sprites { changes, .. } => changes.iter().filter(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/bg/a.jpg")).collect(),
        _ => Vec::new(),
    }).collect();
    assert_eq!(old.len(), 2, "{:?}", c.ops);
    let removals = c.ops.iter().filter(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.is_none()))).count();
    assert_eq!(removals, 2, "{:?}", c.ops);
}
