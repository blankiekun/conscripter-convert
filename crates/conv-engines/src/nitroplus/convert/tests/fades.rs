use super::*;

#[test]
fn a_waited_action_on_no_object_does_not_wait() {
    // ma00_000:2501's `Fade("silver_son*", 500, 0, null, true)` after they went.
    let story = "scene a.nss\n{\n\tCreateColor(\"c1\", 100, 0, 0, 10, 10, \"BLACK\");\n\tDelete(\"c*\");\n\tFade(\"c*\", 500, 0, null, true);\n\tMove(\"c1\", 400, 0, 0, null, true);\n\tCreateTexture(\"gone\", 100, 0, 0, \"cg/none.png\");\n\tFade(\"gone\", 300, 0, null, true);\n}\n";
    let c = compile(story, LIBRARY);
    let waited: u32 = c.ops.iter().map(|op| match op {
        Op::Wait { ms } | Op::Pause { ms } | Op::BeginTransition { fade_ms: ms, .. } => *ms,
        _ => 0,
    }).sum();
    // Only the fade of the object the compiler left out waits.
    assert_eq!(waited, 300, "{:?}", c.ops);
}

#[test]
fn each_op_maps_to_the_statement_it_came_from() {
    let story = "scene a.nss\n{\n\tCreateTextureEX(\"bg\", 100, 0, 0, \"cg/bg/a.jpg\");\n\tFade(\"bg\", 500, 1000, null, true);\n<PRE @box0>\n[text0010]\nあ\n</PRE>\n\tSetText();\n\tTypeBegin();\n\tWait(300);\n}\n";
    // (The box TypeBegin fades: a fade of no object doesn't wait.)
    let c = compile(&story.replacen("{\n", "{\tCreateWindow(\"box\", 20000, 0, 0, 100, 100, true);\n", 1), LIBRARY);
    let at = |i: usize| c.source_map.iter().rev().find(|m| m.0 <= i).map(|m| format!("{}:{}", m.1.file, m.1.index)).unwrap();
    let find = |f: &dyn Fn(&Op) -> bool| c.ops.iter().position(f).unwrap_or_else(|| panic!("{:#?}", c.ops));
    // the page from its <PRE> line, though TypeBegin shows it
    assert_eq!(at(find(&|op| matches!(op, Op::Text { .. }))), "a.nss:5");
    // the box faded out by TypeBegin's line in the library
    assert_eq!(at(find(&|op| matches!(op, Op::HideTextWindow))), "function.nss:24");
    assert_eq!(at(find(&|op| matches!(op, Op::Pause { ms: 300 } | Op::Wait { ms: 300 }))), "a.nss:11");
    assert!(c.source_map.windows(2).all(|w| w[0].0 < w[1].0), "{:?}", c.source_map);
}

#[test]
fn a_sound_maps_to_the_statement_that_played_it() {
    // 続・殺戮のジャンゴ's TitleLogo: SoundPlay inside a function the scene calls.
    let story = "scene a.nss\n{\n\tLogo();\n}\n";
    let library = format!(
        "{LIBRARY}\nfunction SoundPlay(\"ナット名\")\n{{\n\tRequest(\"ナット名\", Play);\n}}\nfunction Logo()\n{{\n\tCreateSound(\"se\", SE, \"sound/se/a\");\n\tSoundPlay(\"se\");\n}}\n"
    );
    let c = compile(story, &library);
    let at = |i: usize| c.source_map.iter().rev().find(|m| m.0 <= i).map(|m| format!("{}:{}", m.1.file, m.1.index)).unwrap();
    let play = c.ops.iter().position(|op| matches!(op, Op::SePlay { .. })).unwrap_or_else(|| panic!("{:#?}", c.ops));
    let line = library.lines().position(|l| l == "\tSoundPlay(\"se\");").unwrap() + 1;
    assert_eq!(at(play), format!("function.nss:{line}"));
}

#[test]
fn a_waited_fade_of_the_texts_window_fades_the_text_out() {
    // 続・殺戮のジャンゴ's TypeEnd: the window drawn under the text faded out.
    let story = "scene a.nss\n{\n\tCreateWindow(\"box00\", 10300, 50, 472, 750, 130, true);\n\tRequest(\"@text001\", \"Enter\");\n<PRE box00>\n[text001]\nあ\n</PRE>\n\tFade(\"@box*\", 100, 0, null, true);\n}\n";
    static GAME: Game = Game { family: true, relative_children: true, ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut fake()).unwrap();
    assert!(c.ops.iter().any(|op| matches!(op, Op::WindowFade { alpha: 0, ms: 100, .. })), "{:?}", c.ops);
    assert!(!c.ops.iter().any(|op| matches!(op, Op::HideTextWindow)), "{:?}", c.ops);
}

#[test]
fn an_unwaited_fade_in_runs_on_and_is_waited_for_later() {
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Fade("bg", 0, 0, null, true);
	Fade("bg", 3000, 1000, null, false);
	Wait(100);
	WaitFade("bg", null);
}
"#;
    let c = compile(story, LIBRARY);
    let fades: Vec<u32> = c.ops.iter().filter_map(|op| if let Op::BeginTransition { fade_ms, .. } = op { Some(*fade_ms) } else { None }).collect();
    assert!(fades.iter().all(|&ms| ms <= 100), "{:?}", c.ops);
    let keys = c.ops.iter().find_map(|op| if let Op::SpriteMotion2 { keys, .. } = op { Some(keys.clone()) } else { None }).expect("motion");
    assert_eq!((keys[0].opacity, keys[1].ms, keys[1].opacity), (0, 3000, 256));
    assert!(c.ops.iter().any(|op| matches!(op, Op::MotionWait { .. })), "{:?}", c.ops);
}

#[test]
fn an_unwaited_fade_out_runs_on_and_is_waited_for_later() {
    // ma00_000:1624: the text goes on as the white fades out.
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.jpg");
	Wait(100);
	Fade("bg", 300, 0, null, false);
	Wait(10);
	WaitFade("bg", null);
}
"#;
    let c = compile(story, LIBRARY);
    let fades: Vec<u32> = c.ops.iter().filter_map(|op| if let Op::BeginTransition { fade_ms, .. } = op { Some(*fade_ms) } else { None }).collect();
    assert!(fades.iter().all(|&ms| ms <= 10), "{:?}", c.ops);
    let keys = c.ops.iter().find_map(|op| if let Op::SpriteMotion2 { keys, .. } = op { Some(keys.clone()) } else { None }).expect("motion");
    assert_eq!((keys[0].opacity, keys[1].ms, keys[1].opacity), (256, 300, 0));
    assert!(c.ops.iter().any(|op| matches!(op, Op::MotionWait { .. })), "{:?}", c.ops);
}

#[test]
fn a_picture_disused_as_its_unwaited_fade_starts_fades_out_first() {
    // ma00_000:137's FadeDelete("絵背景10", 3000, false), then Wait(1000).
    let story = r#"
scene a.nss
{
	CreateTexture("sky", 100, 0, 0, "cg/bg/a.jpg");
	Wait(100);
	Fade("sky", 3000, 0, null, false);
	Request("sky", Disused);
	Wait(1000);
	Wait(5000);
}
"#;
    let c = compile(story, LIBRARY);
    let at = c.ops.iter().position(|op| matches!(op, Op::SpriteMotion2 { keys, .. } if keys[1].ms == 3000 && keys[1].opacity == 0)).unwrap_or_else(|| panic!("{:?}", c.ops));
    let Op::SpriteMotion2 { layer, .. } = c.ops[at] else { unreachable!() };
    // Taken away only at the sync after its fade.
    let gone = c.ops.iter().position(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|ch| ch.layer == layer && ch.sprite.is_none()))).unwrap_or_else(|| panic!("{:?}", c.ops));
    assert!(gone > at && c.ops[at..gone].iter().any(|op| matches!(op, Op::Pause { ms: 1000 })), "{:?}", c.ops);
}

#[test]
fn a_disused_fading_picture_wiped_or_zoomed_goes_at_once() {
    // md01_029vs:2107's FadeDelete then DrawTransition, mb02_028vs:123's Zoom then FadeDelete.
    let story = r#"
scene a.nss
{
	CreateTexture("c", 100, 0, 0, "cg/bg/a.jpg");
	CreateTexture("z", 100, 0, 0, "cg/bg/a.jpg");
	Wait(100);
	Fade("c", 400, 0, null, false);
	Request("c", Disused);
	DrawTransition("c", 200, 1000, 0, 100, null, "cg/bg/a.jpg", true);
	Zoom("z", 500, 4000, 4000, null, false);
	Fade("z", 500, 0, null, false);
	Request("z", Disused);
	Wait(50);
}
"#;
    let c = compile(story, LIBRARY);
    assert!(!c.notes.contains_key("DrawTransition from another opacity than the picture's"), "{:?}", c.notes);
    assert!(!c.notes.keys().any(|n| n.contains("drawn as a cross-fade")), "{:?}", c.notes);
}

#[test]
#[allow(clippy::approx_constant)] // the exe's numbers as printed
fn tempos_are_the_exes() {
    // research/nitroplus/tempo_exe.py <Muramasa.exe>: apply_tempo 00449f40's table builders, 16 steps.
    const EXE_TEMPOS: &[(&str, [f64; 17])] = &[
        ("null", [0.0, 0.0625, 0.125, 0.1875, 0.25, 0.3125, 0.375, 0.4375, 0.5, 0.5625, 0.625, 0.6875, 0.75, 0.8125, 0.875, 0.9375, 1.0]),  // 0x459870
        ("Axl1", [0.0, 0.00390625, 0.015625, 0.03515625, 0.0625, 0.09765625, 0.140625, 0.1914062, 0.25, 0.3164062, 0.390625, 0.4726562, 0.5625, 0.6601562, 0.765625, 0.8789062, 1.0]),  // 0x459930
        ("Axl2", [0.0, 0.0002441406, 0.001953125, 0.006591797, 0.015625, 0.03051758, 0.05273438, 0.08374023, 0.125, 0.1779785, 0.2441406, 0.3249512, 0.421875, 0.536377, 0.6699219, 0.8239746, 1.0]),  // 0x459a00
        ("Axl3", [0.0, 1.525879e-05, 0.0002441406, 0.001235962, 0.00390625, 0.009536743, 0.01977539, 0.03663635, 0.0625, 0.1001129, 0.1525879, 0.2234039, 0.3164062, 0.4358063, 0.5861816, 0.7724762, 1.0]),  // 0x459ad0
        ("Dxl1", [0.0, 0.1210938, 0.234375, 0.3398438, 0.4375, 0.5273438, 0.609375, 0.6835938, 0.75, 0.8085938, 0.859375, 0.9023438, 0.9375, 0.9648438, 0.984375, 0.9960938, 1.0]),  // 0x459cd0
        ("Dxl2", [0.0, 0.1760254, 0.3300781, 0.463623, 0.578125, 0.6750488, 0.7558594, 0.8220215, 0.875, 0.9162598, 0.9472656, 0.9694824, 0.984375, 0.9934082, 0.9980469, 0.9997559, 1.0]),  // 0x459db0
        ("Dxl3", [0.0, 0.2275238, 0.4138184, 0.5641937, 0.6835938, 0.7765961, 0.8474121, 0.8998871, 0.9375, 0.9633636, 0.9802246, 0.9904633, 0.9960938, 0.998764, 0.9997559, 0.9999847, 1.0]),  // 0x459eb0
        ("AxlAuto", [0.0, 0.004815273, 0.01921472, 0.04305966, 0.07612048, 0.1180787, 0.1685304, 0.2269895, 0.2928932, 0.3656067, 0.4444298, 0.5286032, 0.6173166, 0.7097154, 0.8049096, 0.9019829, 1.0]),  // 0x459bb0
        ("DxlAuto", [0.0, 0.09801714, 0.1950903, 0.2902847, 0.3826834, 0.4713967, 0.5555703, 0.6343933, 0.7071068, 0.7730104, 0.8314696, 0.8819212, 0.9238796, 0.9569404, 0.9807853, 0.9951847, 1.0]),  // 0x459fb0
        ("AxlDxl", [0.0, 0.009607375, 0.03806025, 0.0842652, 0.1464466, 0.2222149, 0.3086583, 0.4024548, 0.5, 0.5975451, 0.6913417, 0.7777851, 0.8535534, 0.9157348, 0.9619397, 0.9903927, 1.0]),  // 0x459c40
        ("DxlAxl", [0.0, 0.1608613, 0.2300535, 0.285099, 0.3333333, 0.3776427, 0.4195694, 0.4601069, 0.5, 0.5398931, 0.5804306, 0.6223573, 0.6666667, 0.714901, 0.7699466, 0.8391388, 1.0]),  // 0x45a030
        ("Linear", [0.0, 0.0625, 0.125, 0.1875, 0.25, 0.3125, 0.375, 0.4375, 0.5, 0.5625, 0.625, 0.6875, 0.75, 0.8125, 0.875, 0.9375, 1.0]),  // 0x459870
    ];
    for (name, shares) in EXE_TEMPOS {
        let ease = tempo(&Value::Str(name.to_string()));
        for (i, share) in shares.iter().enumerate() {
            let ours = crate::nitroplus::convert::turn::eased(ease, i as f64 / 16.0);
            assert!((ours - share).abs() < 1e-6, "{name} {i}: {ours} vs {share}");
        }
    }
}
