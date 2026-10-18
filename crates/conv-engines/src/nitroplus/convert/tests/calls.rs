use super::*;

#[test]
fn colour_names_are_the_exes() {
    assert_eq!(color(&Value::Str("Green".into())), Some(0x008000));
    assert_eq!(color(&Value::Str("LIME".into())), Some(0x00FF00));
    assert_eq!(color(&Value::Str("#336600".into())), Some(0x336600));
}

#[test]
fn a_whole_screen_shake_may_grow_from_nothing() {
    // src: Muramasa.exe queue_shake 004462d0: the size goes from args 2-3 to 4-5.
    let story = r#"
scene a.nss
{
	CreateColor("震", 100, 0, 0, 1024, 576, "BLACK");
	Shake("震", 2000, 0, 0, 0, 30, 1000, null, false);
}
"#;
    let c = compile(story, LIBRARY);
    assert!(c.ops.iter().any(|op| matches!(op, Op::ScreenEffect { effect: Some(_), .. })), "{:?}", c.ops);
}

#[test]
fn the_english_port_writes_vertical_boxes_across_in_its_own_window() {
    let story = r#"
scene a.nss
{
	CreateTexture("boxCC", 20000, Center, 40, "cg/sys/adv/TextWindow.png");
	SetAlias("boxCC", "boxCC");
	Request("boxCC", Hideable);
	Move("boxCC", 0, @+2, @0, null, true);
	Fade("boxCC", 0, 1000, null, true);
<PRE @box0>
[text0010]
　一行目
</PRE>
	SetText();
	TypeBegin();
}
"#;
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let w = compile_scene(&files, "a.nss", "a.nss", &MURAMASA_EN, &mut Fake).unwrap().window.unwrap();
    assert!(!w.vertical);
    assert_eq!(w.image, "cg/sys/adv/en/textwindow.png@1000");
    assert_eq!((w.x, w.y, w.text_x, w.text_y, w.columns, w.rows), (360, 51, 375, 192, 14, 12));
    assert_eq!(w.line_gap, 5);
}

#[test]
fn ma01_009s_side_boxes_stand_apart_until_the_next_side_text() {
    let story = r#"
scene ma01_009.nss
{
	CreateTexture("boxRT", 20000, 0, 40, "cg/sys/adv/TextWindow_Fw.png");
	Request("boxRT", Hideable);
	Fade("boxRT", 0, 1000, null, true);
	SetFwR("cg/fw/a.png");
<PRE @box0>
[text0010]
　一行目
</PRE>
	SetText();
	TypeBegin();
	SetTextEXR();
<PRE @box0>
[text0020]
　二行目
</PRE>
	SetText();
	TypeBegin();
}
"#;
    let files = nss_files(LIBRARY, &[("ma01_009.nss", story)]);
    let c = compile_scene(&files, "ma01_009.nss", "ma01_009.nss", &MURAMASA_EN, &mut Fake).unwrap();
    let mut xs = vec![c.window.unwrap().x];
    xs.extend(c.ops.iter().filter_map(|op| if let Op::SetWindow(w) = op { Some(w.x) } else { None }));
    assert_eq!(xs, [690, 616]);
}

#[test]
fn with_two_boxes_up_the_text_goes_in_the_one_set_text_names() {
    let story = r#"
scene a.nss
{
	CreateTexture("boxLT", 20000, 0, 40, "cg/sys/adv/TextWindow_Fw.png");
	Request("boxLT", Hideable);
	Fade("boxLT", 0, 1000, null, true);
	CreateTexture("boxRT", 20000, 0, 40, "cg/sys/adv/TextWindow_Fw.png");
	Request("boxRT", Hideable);
	Fade("boxRT", 0, 1000, null, true);
<PRE @box0>
[text0010]
　一行目
</PRE>
	SetTextEXL();
	SetText();
	TypeBegin();
}
"#;
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let w = compile_scene(&files, "a.nss", "a.nss", &MURAMASA_EN, &mut Fake).unwrap().window.unwrap();
    assert_eq!(w.text_x, 118);
}

#[test]
fn the_english_port_puts_horizontal_text_where_its_box_info_says() {
    let story = r#"
scene a.nss
{
	CreateTexture("boxTH", 20000, 224, InBottom, "cg/sys/adv/THTextWindow_Fw.png");
	Request("boxTH", Hideable);
	Move("boxTH", 0, @0, @+4, null, true);
	Fade("boxTH", 0, 1000, null, true);
<PRE @box0>
[text0010]
　一行目
</PRE>
	SetText();
	TypeBegin();
}
"#;
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let w = compile_scene(&files, "a.nss", "a.nss", &MURAMASA_EN, &mut Fake).unwrap().window.unwrap();
    assert!(!w.vertical);
    assert_eq!(w.image, "cg/sys/adv/en/thtextwindow_fw.png@1000");
    assert_eq!((w.text_x, w.text_y), (238, 428));
}

#[test]
fn the_english_ports_fw_tag_takes_the_face_away_until_the_next_code() {
    let story = r#"
scene a.nss
{
	CreateTexture("boxCT", 20000, Center, 40, "cg/sys/adv/TextWindow_Fw.png");
	Request("boxCT", Hideable);
	Fade("boxCT", 0, 1000, null, true);
	CreateTexture("boxCC", 20000, Center, 40, "cg/sys/adv/TextWindow.png");
	Request("boxCC", Hideable);
	Fade("boxCC", 0, 0, null, true);
	CreateTexture("FwC01", 20001, 451, 4, "cg/fw/face.png");
	Request("FwC01", Hideable);
	Fade("FwC01", 0, 1000, null, true);
<PRE @box0>
[text0010]
"......"
<FW img=''>
  She glances over her shoulder.

  She turns away.

{	Wait(1);}
"Thanks!"
</PRE>
	SetText();
	TypeBegin();
}
"#;
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let c = compile_scene(&files, "a.nss", "a.nss", &MURAMASA_EN, &mut Fake).unwrap();
    let face = c.window.unwrap().image;
    assert_ne!(face, "cg/sys/adv/en/textwindow.png@1000");
    let windows: Vec<_> = c.ops.iter().filter_map(|op| if let Op::SetWindow(w) = op { Some(w.image.as_str()) } else { None }).collect();
    assert_eq!(windows, ["cg/sys/adv/en/textwindow.png@1000", face.as_str()]);
}

#[test]
fn helpers_are_inlined_and_fades_become_transitions() {
    let story = r#"
scene a.nss
{
	CreateTexture("box", 20000, 447, 40, "cg/sys/adv/TextWindow.png");
	SetAlias("box", "box");
	Fade("box", 0, 0, null, false);
	Request("box", Hideable);
	CreateTextureEX("絵背景100", 10000, Center, Middle, "cg/bg/a.jpg");
	SoundPlay("@mbgm29", 0, 1000, true);
	Fade("絵背景100", 2000, 1000, null, true);
<PRE @box0>
[text0010]
　一行目
　二行目

　次のページ

{	OnSE("se01");}
　三ページ目
</PRE>
	SetText();
	TypeBegin();
	FadeDelete("絵背景100", 500, true);
}
"#;
    let c = compile(story, LIBRARY);
    let ops = &c.ops;
    let fade = ops.iter().position(|op| matches!(op, Op::BeginTransition { fade_ms: 2000, .. })).expect("2 s fade");
    assert!(matches!(&ops[fade + 1], Op::Sprites { changes, .. } if changes[0].sprite.as_ref().unwrap().image == "cg/bg/a.jpg"));
    let texts: Vec<&str> = ops.iter().filter_map(|op| if let Op::Text { text, .. } = op { Some(text.as_str()) } else { None }).collect();
    assert_eq!(texts, ["　一行目\n　二行目", "　次のページ", "　三ページ目"]);
    let w = c.window.unwrap();
    assert!(w.vertical);
    assert_eq!((w.x, w.y, w.text_x, w.text_y, w.rows, w.columns, w.line_gap), (447, 40, 430, 55, 5, 27, 9));
    assert_eq!(w.image, "cg/sys/adv/textwindow.png@800");
    // The box fades in with the page, its text after the waited 150 ms,
    // and out over its 150 ms as a waited `Fade` takes it.
    let fade_in = ops.iter().position(|op| matches!(op, Op::WindowFade { alpha: 256, ms: 150, from: Some(0), show: true, .. })).expect("fade in");
    assert!(matches!(ops[fade_in + 1], Op::Wait { ms: 150 }), "{:?}", &ops[fade_in..]);
    assert!(matches!(ops[fade_in..].iter().find(|op| matches!(op, Op::Text { .. })), Some(Op::Text { text, .. }) if text.starts_with("　一行目")));
    assert!(ops.iter().any(|op| matches!(op, Op::WindowFade { alpha: 0, ms: 150, .. })));
    let last = ops.iter().rposition(|op| matches!(op, Op::BeginTransition { fade_ms: 500, .. })).expect("fade out");
    assert!(matches!(&ops[last + 1], Op::Sprites { changes, .. } if changes[0].sprite.is_none()));
    // SoundPlay and OnSE aren't defined here, and aren't built-ins.
    assert!(c.notes.contains_key("skipped: no function SoundPlay"));
}

#[test]
fn calls_left_out_or_harmless_are_no_gaps() {
    let story = r#"
scene a.nss
{
	Waitkey(500);
	Conquest("nss/a.nss", null, true);
	WaitAction("x", null);
	WaitAction("x", 100);
}
"#;
    let c = compile(story, LIBRARY);
    let k = |name: &str| c.metrics.kinds[name].clone();
    assert_eq!((k("Waitkey").left_out, k("Conquest").left_out, k("Conquest").gaps), (1, 1, 0));
    // A null limit changes nothing; 100 ms does.
    assert_eq!((k("WaitAction").calls, k("WaitAction").gaps), (2, 1));
}

#[test]
fn the_choice_screen_s_own_calls_are_left_out() {
    let mut files = BTreeMap::new();
    files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n#include \"nss/function_select.nss\"\n").unwrap());
    files.insert("function.nss".to_string(), nss::parse(LIBRARY).unwrap());
    let select = "function SetChoiceBase()\n{\n\tCreateChoice(\"c\");\n\tSetNextFocus(\"a\", \"b\", DOWN);\n\tCreateText(\"c/txt\", 1, 0, 0, auto, auto, \"x\");\n\tSave(9999);\n}\n";
    files.insert("function_select.nss".to_string(), nss::parse(select).unwrap());
    files.insert("a.nss".to_string(), nss::parse("scene a.nss\n{\n\tSetChoiceBase();\n\tSetNextFocus(\"a\", \"b\", UP);\n}\n").unwrap());
    let c = compile_scene(&files, "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap();
    let k = |name: &str| c.metrics.kinds[name].clone();
    assert_eq!([k("CreateChoice").left_out, k("CreateText").left_out, k("Save").left_out], [1, 1, 1]);
    // Outside function_select.nss it stays a gap.
    assert_eq!((k("SetNextFocus").left_out, k("SetNextFocus").gaps), (1, 1));
}

#[test]
fn names_are_case_sensitive_as_in_the_exe() {
    let story = r#"
scene a.nss
{
	Waitkey(500);
	WaitKey(300);
	sl_rightdown(1000);
}
"#;
    let library = "function SL_rightdown($t)\n{\n\tWait($t);\n}\n";
    let c = compile(story, library);
    // `Waitkey` and `sl_rightdown` are neither built-ins nor functions.
    let waits: Vec<u32> = c.ops.iter().filter_map(|op| match op { Op::Wait { ms } => Some(*ms), _ => None }).collect();
    assert_eq!(waits, [300], "{:?}", c.ops);
    assert!(c.notes.contains_key("skipped: no function Waitkey"));
}

#[test]
fn auto_mode_ends_a_waitkey_after_1500_ms() {
    let story = "scene a.nss\n{\n\tWaitKey(2000);\n\tWaitKey(1000);\n\tWaitKey();\n}\n";
    let c = compile(story, LIBRARY);
    let waits: Vec<String> = c.ops.iter().filter_map(|op| match op {
        Op::Wait { ms } => Some(format!("w{ms}")),
        Op::AutoWait { ms } => Some(format!("a{ms}")),
        Op::ClickWait { .. } => Some("click".into()),
        _ => None,
    }).collect();
    // The shorter one needs none.
    assert_eq!(waits, ["a1500", "w2000", "w1000", "a1500", "click"], "{:?}", c.ops);
}

#[test]
fn a_cut_the_script_waits_for_plays_as_a_movie_over_its_last_frame() {
    // MovieSEStart: wait for the cut, then for RemainTime + 500.
    let story = r#"
scene a.nss
{
	CreateMovie("cut", 100, 0, 0, false, false, "dx/mv剣戟.ngs");
	$t = RemainTime("cut");
	WaitAction("cut", 100);
	WaitKey($t + 500);
	Delete("cut");
}
"#;
    let c = compile(story, LIBRARY);
    let movie = c.ops.iter().position(|op| matches!(op, Op::SceneMovie { file, .. } if file == "movie/dx/mv剣戟.ngs-mute"));
    let last = c.ops.iter().position(|op| matches!(op, Op::AnimatedSprite { cells: 1, sprite, .. } if sprite.image.contains("Some(1934)")));
    // Loaded before the movie, shown (print 1) right after it.
    assert!(last.is_some() && movie > last, "{:?}", c.ops);
    let movie = movie.unwrap();
    assert!(matches!(c.ops[movie + 1], Op::EndTransition), "{:?}", c.ops);
    assert!(c.ops[..movie].iter().rev().find_map(|op| match op { Op::BeginTransition { fade_ms, .. } => Some(*fade_ms == 0), _ => None }).unwrap(), "{:?}", c.ops);
    // The movie's 2 s count as waited: 500 ms are left.
    assert!(c.ops.iter().any(|op| matches!(op, Op::Wait { ms: 500 })), "{:?}", c.ops);
    assert!(!c.ops.iter().any(|op| matches!(op, Op::Wait { ms } if *ms > 500)), "{:?}", c.ops);
}

#[test]
fn boot_logos_end_label_is_not_conv_cons_boot_guard() {
    static GAME: Game = Game { boot_logos: ("a.nss", "function BootLogos() { Wait(10); }"), ..Game::PLAIN };
    let files = nss_files(LIBRARY, &[("a.nss", "scene a.nss\n{\n\tWait(100);\n}")]);
    let c = compile_story(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
    let labels: Vec<_> = c.ops.iter().filter_map(|o| if let Op::Label(l) = o { Some(l.as_str()) } else { None }).collect();
    assert!(labels.contains(&BOOT_LABEL), "{labels:?}");
    // conv-cons defines `*conv_boot_end` after the boot guard.
    assert!(!labels.contains(&"conv_boot_end"), "{labels:?}");
}
