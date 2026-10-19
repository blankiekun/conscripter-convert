use super::*;

#[test]
fn a_draw_transition_on_a_picture_is_the_engines_masked_fade_from_its_from_level() {
    // src: Muramasa.exe 0044fe00: from `from` whatever the picture showed;
    // the table each frame (00457bf0) is the engine's spfadeto.
    let story = r#"
scene a.nss
{
	CreateTexture("a", 100, 0, 0, "cg/bg/a.jpg");
	Fade("a", 0, 1000, null, true);
	Wait(10);
	DrawTransition("a", 300, 1000, 0, 200, null, "cg/bg/a.jpg", true);
	CreateTexture("b", 100, 0, 0, "cg/bg/a.jpg");
	DrawTransition("b", 300, 0, 400, 100, null, "cg/bg/a.jpg", true);
}
"#;
    let c = compile(story, LIBRARY);
    let fades: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteFadeTo { ms, from, to, softness, .. } => Some((*ms, *from, *to, *softness)),
        _ => None,
    }).collect();
    assert_eq!(fades, [(300, 1000, 0, 200), (300, 0, 400, 100)], "{:?}", c.ops);
    // No print through the mask, nor a note.
    assert!(!c.ops.iter().any(|op| matches!(op, Op::TransitionSoft { .. } | Op::BeginTransition { mask: Some(_), .. })), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("DrawTransition")), "{:?}", c.notes);
}

#[test]
fn a_pictures_child_is_placed_from_its_corner_a_windows_at_its_own_place() {
    // 装甲悪鬼村正.exe under wine: a CreateColor child at (100, 150) of a
    // parent at (50, 100) drew at (150, 250); a window's child at its own.
    let story = r#"
scene a.nss
{
	CreateTexture("P", 100, 50, 100, "cg/st/p.png");
	CreateTexture("P/b", 100, 100, 150, "cg/st/pb.png");
	CreateWindow("W", 100, 300, 100, 200, 200, false);
	CreateTexture("W/a", 100, 350, 150, "cg/st/wa.png");
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let placed: Vec<(String, i32, i32)> = c.ops.iter().filter_map(|op| match op {
        Op::Sprites { changes, .. } => Some(changes.iter().filter_map(|ch| ch.sprite.as_ref().map(|s| (s.image.clone(), s.x, s.y))).collect::<Vec<_>>()),
        _ => None,
    }).flatten().collect();
    assert!(placed.contains(&("cg/st/pb.png".into(), 150, 250)), "{placed:?}");
    assert!(placed.contains(&("cg/st/wa.png".into(), 350, 150)), "{placed:?}");
}

#[test]
fn a_vertex_outside_the_picture_leaves_that_axis_as_it_was() {
    // The exe (FUN_004666c0) skips px outside 0..size: y 704 on a
    // 600 px picture keeps `bottom`, as in dja004.
    let zoomed = |vertex: &str| {
        let story = format!("scene a.nss\n{{\n\tCreateColor(\"P\", 100, 0, 0, 800, 600, BLACK);\n\tSetVertex(\"P\", center, bottom);\n\tSetVertex(\"P\", {vertex});\n\tZoom(\"P\", 0, 500, 500, null, true);\n\tWait(100);\n}}\n");
        format!("{:?}", compile(&story, LIBRARY).ops)
    };
    assert_eq!(zoomed("center, 704"), zoomed("center, bottom"));
    assert_ne!(zoomed("center, 704"), zoomed("center, 300"));
}

#[test]
fn a_wildcard_reaches_a_pictures_children_but_not_a_windows() {
    // 装甲悪鬼村正.exe under wine: Fade("@P*") fades P's child too,
    // Fade("@W*") leaves a window's child; Delete of a window takes its
    // children with it.
    let story = r#"
scene a.nss
{
	CreateWindow("W", 100, 0, 0, 1024, 576, false);
	SetAlias("W", "W");
	CreateTexture("W/a", 100, 0, 0, "cg/st/wa.png");
	CreateTexture("P", 100, 0, 0, "cg/st/p.png");
	SetAlias("P", "P");
	CreateTexture("P/b", 100, 0, 0, "cg/st/pb.png");
	Wait(100);
	Fade("@W*", 0, 0, null, true);
	Fade("@P*", 0, 0, null, true);
	Wait(100);
	CreateWindow("V", 100, 0, 0, 1024, 576, false);
	CreateTexture("V/c", 100, 0, 0, "cg/st/vc.png");
	Wait(100);
	Delete("V");
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let mut shown: BTreeMap<u32, String> = BTreeMap::new();
    for op in &c.ops {
        if let Op::Sprites { changes, .. } = op {
            for ch in changes {
                match &ch.sprite {
                    Some(s) => shown.insert(ch.layer, s.image.clone()),
                    None => shown.remove(&ch.layer),
                };
            }
        }
    }
    let images: Vec<&str> = shown.values().map(String::as_str).collect();
    assert_eq!(images, ["cg/st/wa.png"], "{:?}", c.ops);
}

#[test]
fn a_song_line_shows_from_its_enter_to_its_delete() {
    // 続・殺戮のジャンゴ's song (dja026): markup and dots left out,
    // taken away by Erase until its Enter.
    let story = r#"
scene a.nss
{
	CreateText("songtxt01", 10301, center, 0, auto, auto, "<I>荒.野</I>");
	CreateText("songtxt02", 10301, center, 27, auto, auto, "<I>種.は</I>");
	Request("songtxt*", Erase);
	Move("songtxt*", 0, @15, @460, null, true);
	Request("songtxt01", Enter);
	Wait(100);
	Request("songtxt02", Enter);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let shown: Vec<usize> = c.ops.iter().filter_map(|op| match op {
        Op::Sprites { changes, .. } => Some(changes.iter().filter(|c| c.sprite.is_some()).count()),
        _ => None,
    }).collect();
    assert_eq!(shown, [1, 1], "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("markup") || n.contains("font")), "{:?}", c.notes);
}

#[test]
fn a_song_line_types_out_from_its_enter() {
    // 続・殺戮のジャンゴ's hooks (django/song.rs): 0, 1, 2 letters, 125 ms each.
    let story = r#"
scene a.nss
{
	CreateText("songtxt01", 10301, center, 0, auto, auto, "<I>荒.野</I>");
	Request("songtxt01", Erase);
	Request("songtxt01", Enter);
	WaitAction("songtxt01", null);
}
"#;
    static GAME: Game = Game { call: DJANGO.call, ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut fake()).unwrap();
    let at = c.ops.iter().position(|op| matches!(op, Op::AnimatedSprite { cells: 3, frame_ms: 125, looped: false, .. }));
    assert!(at.is_some_and(|i| c.ops[i + 1..].iter().any(|op| matches!(op, Op::Wait { ms: 250 }))), "{:?}", c.ops);
}

#[test]
fn a_stencil_cuts_its_children_to_its_picture() {
    // 装甲悪鬼村正's ma04_013vs: a red colour drawn only over the
    // standing picture (`絵演立絵覆`); the stencil itself draws nothing.
    // Created off the screen and moved in (`CreateColorEX`): cut where
    // it ends up.
    let story = r##"
scene a.nss
{
	CreateStencil("覆", 1000, 100, 50, 128, "cg/st/s.png", false);
	CreateColor("覆/赤", 1010, 1024, 0, 1024, 576, "#CC0000");
	Wait(16);
	Move("覆/赤", 0, @-1024, @0, null, true);
	Fade("覆/赤", 0, 1000, null, true);
	Wait(100);
}
"##;
    let c = compile(story, LIBRARY);
    let script = format!("{:?}", c.ops);
    assert!(script.contains("color/cc0000_1024x576.png%cg/st/s.png@100,50x1000"), "{script}");
    assert!(!script.contains("\"cg/st/s.png\""), "{script}");
}

#[test]
fn a_mask_multiplies_its_children_by_its_picture() {
    // 装甲悪鬼村正's ma01_019: smoke (an added grey) only where the
    // mask is light; the mask itself draws nothing.
    let story = r##"
scene a.nss
{
	CreateMask("絵Ｍ", 6000, 0, 0, "cg/st/s.png", false);
	CreateColor("絵Ｍ/煙", 6000, 0, 0, 1024, 576, "#BBBBBB");
	Fade("絵Ｍ/煙", 0, 1000, null, true);
	Wait(100);
}
"##;
    let c = compile(story, LIBRARY);
    let script = format!("{:?}", c.ops);
    assert!(script.contains("color/bbbbbb_1024x576.png%cg/st/s.png*@0,0x1000"), "{script}");
    assert!(!c.notes.keys().any(|n| n.contains("CreateMask")), "{:?}", c.notes);
}

#[test]
fn a_window_clips_its_children_to_the_overlap_of_its_windows() {
    // 装甲悪鬼村正's cockpit: the speed tape in CPWS (CockpitSetSpeed).
    let story = r#"
scene a.nss
{
	CreateWindow("CPWS", 15000, 260, 138, 100, 300, false);
	CreateWindow("CPWS/in", 15000, 300, 100, 100, 100, false);
	CreateTexture("CPWS/CPSpeed", 15000, 262, -220, "cg/st/a.png");
	CreateTexture("CPWS/in/b", 15000, 262, -220, "cg/st/a.png");
	CreateTexture("free", 15000, 0, 0, "cg/st/a.png");
	Fade("*", 0, 1000, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let mut clips: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteClip { rect, .. } => Some(*rect),
        _ => None,
    }).collect();
    clips.sort();
    assert_eq!(clips, [(260, 138, 100, 300), (300, 138, 60, 62)], "{:?}", c.ops);
}

#[test]
fn a_window_in_a_window_clips_from_its_parents_place() {
    // 続・殺戮のジャンゴ's 中窓 (`CreateMiddleWin`): the band between the
    // black bars, inside `窓`; a standing picture's hat stops at y 70
    // (dja002_02 under wine).
    let story = r#"
scene a.nss
{
	CreateWindow("窓", 9990, 0, 0, 800, 600, false);
	CreateWindow("@窓/中窓", 10190, 0, 70, 800, 400, true);
	CreateTexture("@窓/中窓/立絵", 10160, 100, -40, "cg/st/a.png");
	Fade("@窓/中窓/立絵", 0, 1000, null, true);
	Wait(100);
}
"#;
    static GAME: Game = Game { family: true, relative_children: true, ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut fake()).unwrap();
    let clips: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteClip { rect, .. } => Some(*rect),
        _ => None,
    }).collect();
    assert_eq!(clips, [(0, 70, 800, 400)], "{:?}", c.ops);
}
