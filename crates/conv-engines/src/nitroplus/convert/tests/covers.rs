use super::*;

#[test]
fn pictures_above_the_text_window_are_drawn_over_it() {
    let story = r#"
scene a.nss
{
	CreateWindow("box0", 20001, 0, 0, 1024, 576, false);
<PRE @box0>
[text0010]
　一行目
</PRE>
	CreateColor("絵色下", 1000, 0, 0, 1024, 576, "Black");
	CreateColor("絵色上", 25000, 0, 0, 100, 100, "White");
	Wait(10);
}
"#;
    let c = compile(story, LIBRARY);
    let layers: Vec<u32> = c.ops.iter().filter_map(|op| if let Op::SpriteOverText { layer } = op { Some(*layer) } else { None }).collect();
    let placed: Vec<(u32, String)> = c.ops.iter().filter_map(|op| if let Op::Sprites { changes, .. } = op { Some(changes) } else { None }).flatten().filter_map(|c| Some((c.layer, c.sprite.as_ref()?.image.clone()))).collect();
    let white = placed.iter().find(|p| p.1.contains("ffffff")).unwrap().0;
    assert_eq!(layers, [white], "{placed:?}");
}

#[test]
fn a_page_showing_a_runtime_flag_is_given_once_for_each_of_its_texts() {
    let story = r#"
scene a.nss
{
	if($分岐 == 1){$名前 = "甲";}else{$名前 = "乙";}
	if($名前 == "甲"){$分岐 = 2;}
<PRE @box0>
[text0010]
<VALUE name=$名前>だ。
</PRE>
	SetText();
	TypeBegin();
}
"#;
    // The whole story: `$分岐` may hold anything when the file starts.
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_story(&files, "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap();
    let texts: Vec<&str> = c.ops.iter().filter_map(|op| if let Op::Text { text, .. } = op { Some(text.as_str()) } else { None }).collect();
    assert!(texts.iter().any(|t| t.contains('甲')) && texts.iter().any(|t| t.contains('乙')), "{texts:?}");
    assert_eq!(texts.len(), 3, "{texts:?}");
}

#[test]
fn pictures_above_a_plain_cover_still_change() {
    let story = r#"
scene a.nss
{
	CreateTexture("絵背景100", 10000, 0, 0, "cg/bg/a.jpg");
	Wait(10);
	CreateEffect("絵板写", 30000, 0, 0, 1024, 576, "Plain");
	Delete("絵背景100");
	CreateColor("絵色100", 30200, 0, 0, 1024, 576, "Black");
	Wait(10);
}
"#;
    let c = compile(story, LIBRARY);
    let Some(Op::Sprites { changes, .. }) = c.ops.iter().rev().find(|op| matches!(op, Op::Sprites { .. })) else { panic!() };
    // The black drawn over the frozen screen, whose background stays.
    assert!(changes.iter().all(|c| c.sprite.is_some()), "{changes:?}");
    assert!(changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "color/000000_1024x576.png")));
}

#[test]
fn a_plain_cover_shown_again_draws_what_it_froze() {
    // 装甲悪鬼村正's ma00_000: each fighter's screen kept in a cover,
    // faded out, and faded in again after the scene under it changed.
    let story = r#"
scene a.nss
{
	CreateTexture("絵背景100", 100, 0, 0, "cg/bg/a.jpg");
	Wait(10);
	CreateEffect("絵鷺沼側", 1200, 0, 0, 1024, 576, "Plain");
	Delete("絵背景100");
	CreateTexture("絵背景200", 100, 0, 0, "cg/bg/b.jpg");
	Fade("絵鷺沼側", 300, 0, null, true);
	Fade("絵鷺沼側", 300, 1000, null, true);
}
"#;
    let c = compile(story, LIBRARY);
    let Some(Op::Sprites { changes, .. }) = c.ops.iter().rev().find(|op| matches!(op, Op::Sprites { .. })) else { panic!() };
    assert!(changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/bg/a.jpg")), "{changes:?}");
}

#[test]
fn a_blended_zoomed_plain_cover_is_a_copy_of_the_screen() {
    // 装甲悪鬼村正's md05_016vs: CreatePlainEXadd, faded in as it zooms, waved.
    let story = r#"
scene a.nss
{
	CreateTexture("絵背景100", 100, 0, 0, "cg/bg/a.jpg");
	Wait(10);
	CreateEffect("絵板写", 1000, 0, 0, 1024, 576, "Plain");
	Fade("絵板写", 0, 0, null, true);
	Request("絵板写", AddRender);
	Fade("絵板写", 300, 500, null, false);
	Zoom("絵板写", 300, 1500, 1500, Dxl2, false);
	Wait(200);
	DrawEffect("絵板写", 1000, "HighWave", 300, 300, Dxl2);
	Wait(200);
}
"#;
    let c = compile(story, LIBRARY);
    let placed = c.ops.iter().find_map(|op| match op {
        Op::Sprites { changes, .. } => changes.iter().find(|c| c.sprite.as_ref().is_some_and(|s| s.image == SCREEN_UNDER)).map(|c| (c.layer, c.sprite.clone().unwrap())),
        _ => None,
    });
    let Some((layer, at)) = placed else { panic!("{:?}", c.ops) };
    assert_eq!((at.x, at.y), (0, 0));
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteAngle { layer: l, zoom: (150, 150), .. } if *l == layer)), "{:?}", c.ops);
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteBlend { layer: l, mode: BLEND_ADD } if *l == layer)), "{:?}", c.ops);
    // Zoomed from 1000 (2/3 of its end) as it fades in.
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteMotion2 { layer: l, keys, .. } if *l == layer && keys[0].zoom == 66 && keys[0].opacity == 0)), "{:?}", c.ops);
    assert!(c.ops.iter().any(|op| matches!(op, Op::SpriteWave { layer: l, .. } if *l == layer)), "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.contains("DrawEffect")), "{:?}", c.notes);
}

#[test]
fn a_plain_cover_turns_the_scene_change_into_one_fade() {
    let story = r#"
scene a.nss
{
	CreateTexture("絵背景100", 10000, 0, 0, "cg/bg/a.jpg");
	Wait(10);
	CreateEffect("上背景", 30000, 0, 0, 1024, 576, "Plain");
	Request("上背景", Lock);
	Delete("*");
	Request("上背景", UnLock);
	CreateColor("絵色100", 1500, 0, 0, 1024, 576, "Black");
	FadeDelete("上背景", 300, true);
}
"#;
    let c = compile(story, LIBRARY);
    let fades: Vec<u32> = c.ops.iter().filter_map(|op| if let Op::BeginTransition { fade_ms, .. } = op { Some(*fade_ms) } else { None }).collect();
    assert_eq!(fades, [0, 300]);
    let Some(Op::Sprites { changes, .. }) = c.ops.iter().rev().find(|op| matches!(op, Op::Sprites { .. })) else { panic!() };
    assert_eq!(changes.len(), 2, "{changes:?}");
    assert!(changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "color/000000_1024x576.png")));
}
