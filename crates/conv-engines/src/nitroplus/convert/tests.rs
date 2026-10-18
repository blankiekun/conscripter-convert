use super::*;
use crate::nitroplus::nss;

#[derive(Default)]
struct Fake;

impl Assets for Fake {
    fn image(&mut self, path: &str) -> Option<(String, u32, u32)> {
        Some((path.to_lowercase(), 1024, 576))
    }
    fn sound(&mut self, path: &str) -> Option<String> {
        Some(format!("{path}.ogg"))
    }
    fn color(&mut self, rgb: u32, w: u32, h: u32) -> String {
        format!("color/{rgb:06x}_{w}x{h}.png")
    }
    fn composite(&mut self, parts: &[(String, i64, i64, i64)]) -> Option<(String, i64, i64)> {
        let (_, x, y, _) = parts.first()?;
        let names: Vec<String> = parts.iter().map(|(i, _, _, o)| format!("{i}@{o}")).collect();
        Some((names.join("+"), *x, *y))
    }
    fn transparent(&mut self) -> String {
        "none.png".into()
    }
    fn scaled(&mut self, image: &str, zx: i64, zy: i64, at: (i64, i64, i64, i64)) -> Option<(String, i64, i64, u32, u32)> {
        let (w, h) = (1024 * zx / 1000, 576 * zy / 1000);
        let (x, y) = (at.0 + (at.2 - w) / 2, at.1 + (at.3 - h) / 2);
        // Cut to the screen past 4096 px, as Assets::scaled.
        if w.max(h) > 4096 {
            return Some((format!("{image}@{zx}x{zy}_screen"), x.max(0), y.max(0), (x + w).min(1024).saturating_sub(x.max(0)) as u32, (y + h).min(576).saturating_sub(y.max(0)) as u32));
        }
        Some((format!("{image}@{zx}x{zy}"), x, y, w as u32, h as u32))
    }
    fn mirrored(&mut self, image: &str, x: bool, y: bool) -> Option<String> {
        Some(format!("{image}~{}{}", if x { "x" } else { "" }, if y { "y" } else { "" }))
    }
    fn strip(&mut self, parts: &[&str]) -> Option<(String, u32, u32)> {
        Some((parts.join("|"), 100 * parts.len() as u32, 100))
    }
    fn movie_info(&mut self, path: &str) -> Option<MovieInfo> {
        let small = path.contains("k100");
        Some(MovieInfo { w: if small { 400 } else { 1024 }, h: if small { 432 } else { 576 }, ms: 2000, sound: path.contains("OP") })
    }
    fn movie_file(&mut self, path: &str, sound: bool) -> Option<String> {
        Some(format!("movie/{path}{}", if sound { "" } else { "-mute" }))
    }
    fn rgba(&mut self, _path: &str) -> Option<image::RgbaImage> {
        None
    }
    fn frames_movie(&mut self, name: &str, _fps: u32, _frames: u32, _draw: &mut dyn FnMut(u32, &mut image::RgbImage)) -> Option<String> {
        Some(format!("movie/{name}.mp4"))
    }
    fn frames_strip(&mut self, name: &str, _w: u32, _h: u32, _frames: u32, _draw: &mut dyn FnMut(u32, &mut image::RgbaImage)) -> Option<String> {
        Some(format!("system/strip/{name}.png"))
    }
    fn font(&self) -> &[u8] {
        include_bytes!("../../../../conv-cli/fonts/ipag.ttf")
    }
    fn delayed_sound(&mut self, path: &str, delay_ms: u32) -> Option<String> {
        Some(format!("{path}+{delay_ms}.ogg"))
    }
    fn sound_ms(&mut self, _path: &str) -> Option<u32> {
        Some(4000)
    }
    fn overlaid(&mut self, base: &str, top: &str, zoom: i64, opacity: i64) -> Option<String> {
        Some(format!("{base}+{top}@{zoom}x{opacity}"))
    }
    fn stenciled(&mut self, image: &str, mask: &str, colour: bool, (dx, dy): (i64, i64), zoom: (i64, i64)) -> Option<String> {
        Some(format!("{image}%{mask}{}@{dx},{dy}x{}", if colour { "*" } else { "" }, zoom.0))
    }
    fn veil(&mut self, mask: &str, table: &[u8; 256]) -> Option<String> {
        Some(format!("{mask}~{}", table.iter().filter(|&&v| v > 0).count()))
    }
    fn movie_frames(&mut self, path: &str, at: Option<u32>, alpha: bool, dark: bool) -> Option<(String, u32, u32, u32)> {
        let (w, h) = if alpha { (200, 108) } else { (1024, 576) };
        let cells = if at.is_some() { 1 } else { 30 };
        Some((format!("{path}@{at:?}{}", if dark { "d" } else { "" }), cells, w, h))
    }
    fn movie_sprite(&mut self, path: &str, alpha: bool, dark: bool) -> Option<(String, u32, u32)> {
        let (w, h) = if alpha { (400, 216) } else { (1024, 576) };
        Some((format!("{path}{}{}.mp4", if dark { "_dark" } else { "" }, if alpha { "_alpha" } else { "" }), w, h))
    }
}

/// The boot script, a `function.nss` holding `library` and the `(name, source)` scripts.
fn nss_files(library: &str, scripts: &[(&str, &str)]) -> BTreeMap<String, NssFile> {
    let mut files = BTreeMap::new();
    files.insert(BOOT.to_string(), nss::parse("#include \"nss/function.nss\"\n").unwrap());
    files.insert("function.nss".to_string(), nss::parse(library).unwrap());
    for (name, src) in scripts {
        files.insert(name.to_string(), nss::parse(src).unwrap());
    }
    files
}

fn compile(story: &str, library: &str) -> Compiled {
    compile_scene(&nss_files(library, &[("a.nss", story)]), "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap()
}

const LIBRARY: &str = r#"
function CreateTextureEX("ナット名", 描画優先度, $x, $y, $画像)
{
	CreateTexture("ナット名", 描画優先度, $x, $y, $画像);
	Fade("ナット名", 0, 0, null, true);
}
function FadeDelete("ナット名", 所要時間, 待ち)
{
	Fade("ナット名", 所要時間, 0, null, 待ち);
	Request("ナット名", UnLock);
	Request("ナット名", Disused);
}
function SetText()
{
	SetFont("@ＭＳ 明朝", 20, #FFFFFF, #222222, 500, RIGHTDOWN);
	LoadText("$構文名","$SYSTEM_present_preprocess","$SYSTEM_present_text",550,150,0,29);
	Rotate("$SYSTEM_present_text", 0, @0, @0, 90, null,true);
	Move("$SYSTEM_present_text", 0, 575, 55, null, true);
}
function TypeBegin()
{
	Fade("@box", 150, 800, null, true);
	Request($SYSTEM_present_text, Enter);
	Fade("@box", 150, 0, null, true);
}
"#;

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
fn a_sepia_effect_tints_the_pictures_below_it() {
    // dja038_03: `CreateEffect(…, "Sepia")` at 2000, faded in at once.
    let story = r#"
scene a.nss
{
	CreateTexture("bg", 100, 0, 0, "cg/bg/a.png");
	CreateTexture("bar", 10000, 0, 0, "cg/bg/b.png");
	Wait(100);
	CreateEffect("効果", 2000, 0, 0, 1024, 576, "Sepia");
	Fade("効果", 0, 1000, null, true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let tinted: Vec<u32> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteFilters { layer, filters } if filters.iter().any(|f| f.kind == Operand::Const(conv_core::ir::FILTER_GREY_WEIGHTS)) => Some(*layer),
        _ => None,
    }).collect();
    assert_eq!(tinted.len(), 1, "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("CreateEffect Sepia")), "{:?}", c.notes);
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
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
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
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
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
    let c = compile_scene(&files, "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap();
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
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
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
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
    let motions = c.ops.iter().filter(|op| matches!(op, Op::SpriteMotion2 { .. })).count();
    assert_eq!(motions, 2, "{:?}", c.ops);
}

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

#[test]
fn a_branch_that_left_the_chapter_early_goes_on_from_where_it_left() {
    let mut files = nss_files(LIBRARY, &[]);
    // As 続・殺戮のジャンゴ's deaths: the chapter returns right after the
    // deadly answer, the caller then plays the bad end.
    let a = r#"scene a.nss
{
	call_chapter nss/b.nss;
	if($x == 1)
	{
		CreateTexture("bad", 200, 0, 0, "cg/bad.png");
		Fade("bad", 0, 1000, null, true);
		Wait(100);
		return;
	}
}"#;
    let b = r#"chapter main
{
	CreateTexture("one", 100, 0, 0, "cg/one.png");
	Fade("one", 0, 1000, null, true);
	select { case @選択肢１ { $x = 1; } case @選択肢２ { $x = 0; } }
	if($x == 1)
	{
		return;
	}
	CreateTexture("two", 300, 0, 0, "cg/two.jpg");
	Fade("two", 0, 1000, null, true);
	Wait(100);
}"#;
    files.insert("a.nss".into(), nss::parse(a).unwrap());
    files.insert("b.nss".into(), nss::parse(b).unwrap());
    let c = compile_story(&files, "a.nss", "a.nss", &Game::PLAIN, &mut Fake).unwrap();
    // The bad end is drawn over "one", not under the other answer's
    // "two", which it never saw.
    let bad = c.ops.iter().position(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/bad.png"))));
    assert!(bad.is_some(), "{:?}", c.ops);
    let two_shown = c.ops[..bad.unwrap()].iter().rposition(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.as_ref().is_some_and(|s| s.image == "cg/two.jpg"))));
    let label = c.ops[..bad.unwrap()].iter().rposition(|op| matches!(op, Op::Label(_)));
    assert!(two_shown < label, "{:?}", c.ops);
}

#[test]
fn a_music_process_plays_its_sounds_on_its_clock() {
    // 続・殺戮のジャンゴ's Play_DJ24x and PlayOWA: an intro, then its loop
    // a set time later; a jingle's second part when the first ends.
    let story = r#"
scene a.nss
{
	CreateSound("intro", BGM, "sound/bgm/intro");
	CreateSound("loop", BGM, "sound/bgm/loop");
	CreateProcess("P", 0, 0, 0, "Music");
	Request("P", Start);
	Wait(100);
}
function Music()
{
	begin:
	SoundPlay("intro", 0, 1000, false);
	WaitPlay("intro", null);
	Wait(500);
	SoundPlay("loop", 0, 1000, true);
}
function SoundPlay("ナット名", 秒数, ボリウム, ループ設定)
{
	Request("ナット名", Play);
	SetLoop("ナット名", ループ設定);
	SetVolume("ナット名", 秒数, ボリウム, null);
}
"#;
    let c = compile(story, LIBRARY);
    let later: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::BgmLater { file, after_ms, looped } => Some((file.as_str(), *after_ms, *looped)),
        _ => None,
    }).collect();
    // (Fake's sounds last 4 s.)
    assert_eq!(later, [("sound/bgm/intro.ogg", 0, false), ("sound/bgm/loop.ogg", 4500, true)], "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("CreateProcess")));
}

#[test]
fn a_music_process_decides_its_ifs_and_refuses_a_late_sound_effect() {
    // 装甲悪鬼村正's EndSong: the second song once the first has ended,
    // its fade out waited for; md01_023's ZansyuSet: a sound effect 6 s
    // in, which only the run could play (noted).
    let story = |two: &str| format!(r#"
scene a.nss
{{
	CreateSound("song", BGM, "sound/bgm/song");
	CreateSound("song2", BGM, "sound/bgm/song2");
	CreateSound("se", SE, "sound/se/a");
	$Two = {two};
	CreateProcess("P", 0, 0, 0, "Music");
	Request("P", Start);
	Wait(100);
}}
function Music()
{{
	if($Two){{
		SoundPlay("song", 0, 1000, false);
		WaitPlay("song", null);
		SetVolume("song", 1000, 0, null);
		WaitAction("song", null);
		SoundPlay("song2", 0, 1000, false);
	}}else{{
		Wait(6000);
		SoundPlay("se", 0, 1000, false);
	}}
}}
function SoundPlay("ナット名", 秒数, ボリウム, ループ設定)
{{
	Request("ナット名", Play);
	SetLoop("ナット名", ループ設定);
	SetVolume("ナット名", 秒数, ボリウム, null);
}}
"#);
    let c = compile(&story("true"), LIBRARY);
    let later: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::BgmLater { file, after_ms, .. } => Some((file.as_str(), *after_ms)),
        _ => None,
    }).collect();
    assert_eq!(later, [("sound/bgm/song.ogg", 0), ("sound/bgm/song2.ogg", 5000)], "{:?}", c.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("CreateProcess")), "{:?}", c.notes);
    let c = compile(&story("false"), LIBRARY);
    assert!(!c.ops.iter().any(|op| matches!(op, Op::SePlay { .. })), "{:?}", c.ops);
    assert!(c.notes.contains_key("CreateProcess Music"), "{:?}", c.notes);
}

#[test]
fn an_argument_a_built_in_does_not_read_is_noted() {
    // DrawTransition's softness is read (effectsoft; its start opacity is
    // noted where the picture isn't shown at it); a process's place is harmless.
    let story = r#"
scene a.nss
{
	CreateTexture("a", 100, 0, 0, "cg/bg/a.jpg");
	Fade("a", 0, 0, null, true);
	Fade("a", 300, 1000, Axl1, true);
	DrawTransition("a", 300, 0, 1000, 100, null, "cg/bg/a.jpg", true);
	CreateProcess("P", 100, 5, 5, "Nothing");
}
function Nothing()
{
}
"#;
    let c = compile(story, LIBRARY);
    assert!(!c.notes.keys().any(|n| n.starts_with("DrawTransition argument")), "{:?}", c.notes);
    assert!(c.notes.contains_key("DrawTransition from another opacity than the picture's"), "{:?}", c.notes);
    // Fade's tempo is read: its transition goes along Axl1 (t²) where
    // the game's exe eases fades.
    assert!(!c.ops.iter().any(|op| matches!(op, Op::TransitionEase { .. })), "{:?}", c.ops);
    let files = nss_files(LIBRARY, &[("a.nss", story)]);
    let eased = compile_scene(&files, "a.nss", "a.nss", &Game { fade_tempo: Some(tempo), ..Game::PLAIN }, &mut Fake).unwrap();
    assert!(eased.ops.iter().any(|op| matches!(op, Op::TransitionEase { ease: 1, exponent: 2 })), "{:?}", eased.ops);
    assert!(!c.notes.keys().any(|n| n.starts_with("Fade argument") || n.starts_with("CreateProcess argument")), "{:?}", c.notes);
}

#[test]
fn a_full_draw_transition_out_sweeps_reversed_and_a_partial_one_cuts_its_picture() {
    // src: Muramasa.exe 00457bf0: a falling threshold leaves the bright parts first.
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
    assert!(c.ops.iter().any(|op| matches!(op, Op::TransitionSoft { softness: 200, reverse: true, .. })), "{:?}", c.ops);
    assert_eq!(c.ops.iter().filter(|op| matches!(op, Op::TransitionSoft { .. })).count(), 1, "{:?}", c.ops);
    // The partial one: its picture cut by the level's table, not noted.
    assert!(!c.notes.contains_key("DrawTransition partial: ends masked in the exe"), "{:?}", c.notes);
    assert!(format!("{:?}", c.ops).contains(".jpg~"), "{:?}", c.ops);
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
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
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
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
    let clips: Vec<_> = c.ops.iter().filter_map(|op| match op {
        Op::SpriteClip { rect, .. } => Some(*rect),
        _ => None,
    }).collect();
    assert_eq!(clips, [(0, 70, 800, 400)], "{:?}", c.ops);
}

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
fn rain_is_a_looping_strip_zoomed_and_turned() {
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
}
"#;
    let c = compile(story, LIBRARY);
    let strip = c.ops.iter().find_map(|op| match op {
        Op::AnimatedSprite { sprite, cells, frame_ms, looped, zoom, .. } => Some((sprite.image.as_str(), sprite.x, sprite.y, *cells, *frame_ms, *looped, *zoom)),
        _ => None,
    });
    assert_eq!(strip, Some(("system/strip/rain_288x512_16_128.png", 368, 32, 24, 42, true, (200, 200))), "{:?}", c.ops);
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
fn delete_all_in_a_chapter_keeps_what_was_made_before_it() {
    static GAME: Game = Game { chapter_objects: true, ..Game::PLAIN };
    let mut files = nss_files(LIBRARY, &[]);
    let a = r#"scene a.nss
{
	CreateSound("bgm1", BGM, "sound/bgm/one");
	call_chapter nss/b.nss;
	Request("bgm1", Play);
	SetLoop("bgm1", true);
	Wait(100);
}"#;
    let b = r#"chapter main
{
	CreateTexture("pic", 100, 0, 0, "cg/one.png");
	Fade("pic", 0, 1000, null, true);
	Wait(100);
	Delete("*");
	Wait(100);
}"#;
    files.insert("a.nss".into(), nss::parse(a).unwrap());
    files.insert("b.nss".into(), nss::parse(b).unwrap());
    let c = compile_story(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
    assert!(c.ops.iter().any(|op| matches!(op, Op::BgmPlay { file, .. } if file == "sound/bgm/one.ogg")), "{:?}", c.ops);
    let removed = c.ops.iter().any(|op| matches!(op, Op::Sprites { changes, .. } if changes.iter().any(|c| c.sprite.is_none())));
    assert!(removed, "{:?}", c.ops);
}

#[test]
fn wait_play_waits_for_a_fade_to_0() {
    // A loop faded out over 1500 ms stops at the fade's end; a one-shot
    // (4000 ms) ends first.
    let story = r#"
scene a.nss
{
	CreateSound("m", BGM, "sound/se/loop_L");
	Request("m", Play);
	SetLoop("m", true);
	Wait(100);
	SetVolume("m", 1500, 0, null);
	WaitPlay("m", null);
	CreateSound("se", SE, "sound/se/a");
	Request("se", Play);
	Request("se", Disused);
	Wait(3000);
	SetVolume("se", 2000, 0, null);
	WaitPlay("se", null);
	Wait(1000);
	WaitPlay("se", null);
}
"#;
    let c = compile(story, LIBRARY);
    let waits: Vec<u32> = c.ops.iter().filter_map(|op| match op { Op::Wait { ms } | Op::Pause { ms } => Some(*ms), _ => None }).collect();
    assert_eq!(waits, [100, 1500, 3000, 1000, 1000], "{:?}", c.ops);
}

#[test]
fn play_on_a_playing_loop_doesnt_start_it_again() {
    let story = r#"
scene a.nss
{
	CreateSound("SE01", SE, "sound/se/loop_L");
	Request("SE01", Play);
	SetLoop("SE01", true);
	Wait(100);
	Request("SE01", Play);
	SetLoop("SE01", true);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let plays = c.ops.iter().filter(|op| matches!(op, Op::SePlay { .. })).count();
    assert_eq!(plays, 1);
}

#[test]
fn set_frequency_bends_a_sound() {
    let story = r#"
scene a.nss
{
	CreateSound("SE10", SE, "sound/se/rain_L");
	Request("SE10", Play);
	SetFrequency("SE10", 1000, 1100, null);
	Wait(100);
	SetFrequency("SE10", 0, 1000, null);
	Wait(100);
}
"#;
    let c = compile(story, LIBRARY);
    let rates: Vec<_> = c.ops.iter().filter_map(|op| match op { Op::SoundRate { rate, ms, .. } => Some((*rate, *ms)), _ => None }).collect();
    assert_eq!(rates, [(1100, 1000), (1000, 0)], "{:?}", c.ops);
    let play = c.ops.iter().position(|op| matches!(op, Op::SePlay { .. })).unwrap();
    assert!(matches!(c.ops[play + 1], Op::SoundRate { rate: 1100, .. }), "{:?}", c.ops);
}

#[test]
fn sound_volumes_follow_the_exe_curve() {
    let story = r#"
scene a.nss
{
	CreateSound("SE01", SE, "sound/se/rain_L");
	SoundPlay("SE01", 1000, 500, true);
	Wait(100);
	SetVolume("SE01", 2000, 1, null);
	Wait(100);
	SetVolume("SE01", 500, 0, null);
	Wait(100);
}
"#;
    // function.nss's own.
    let library = r#"
function SoundPlay($ナット名,秒数,ボリウム,ループ設定)
{
	SetVolume($ナット名, 0, 1, null);
	Request($ナット名, Play);
	SetVolume($ナット名, 秒数, ボリウム, null);
	SetLoop($ナット名, ループ設定);
	Request($ナット名, Disused);
}
"#;
    let c = compile(story, library);
    let file = Some("sound/se/rain_L.ogg".to_string());
    let sound: Vec<_> = c.ops.iter().filter(|op| matches!(op, Op::SeFadeIn { .. } | Op::SoundVolume { .. } | Op::SePlay { .. } | Op::SeFadeOut { .. })).collect();
    assert_eq!(
        sound,
        [
            &Op::SeFadeIn { ms: 1000 },
            &Op::SoundVolume { file: file.clone(), percent: 32, ms: 0 },
            &Op::SePlay { file: "sound/se/rain_L.ogg".into(), looped: true },
            &Op::SoundVolume { file: file.clone(), percent: 0, ms: 2000 },
            &Op::SeFadeOut { file, ms: 500 },
        ]
    );
}

#[test]
fn code_inside_a_page_runs_where_it_stands() {
    let story = r#"
scene a.nss
{
<PRE @box0>
[text0030]
《<?>
{OnSE("se01");}
興<?>
{OnSE("se01");
WaitKey(50);}
隆<?>
{OnSE("se02");}
》

次<?>
</PRE>
	SetText();
	TypeBegin();
}
"#;
    let c = compile(story, LIBRARY);
    let texts: Vec<&str> = c.ops.iter().filter_map(|op| if let Op::Text { text, .. } = op { Some(text.as_str()) } else { None }).collect();
    // Each letter, then its code (the key's wait): the page stops for it
    // and goes on on the same line.
    assert_eq!(texts, ["《\u{5}", "興\u{5}", "隆\u{5}", "》", "次"]);
    let first = c.ops.iter().position(|op| matches!(op, Op::Text { .. })).unwrap();
    assert!(matches!(c.ops[first + 2], Op::Wait { ms: 50 }), "{:?}", &c.ops[first..first + 4]);
}

#[test]
fn k_waits_in_place_and_code_after_it_runs_after_the_click() {
    let story = r#"
scene a.nss
{
<PRE @box0>
[text0010]
　怪我をする。<k>痛い。<k>
もがき苦しみ<k>
{	Wait(5);}
　兵士
</PRE>
	SetText();
	TypeBegin();
}
"#;
    let c = compile(story, LIBRARY);
    let texts: Vec<(&str, bool)> =
        c.ops.iter().filter_map(|op| if let Op::Text { text, continues, .. } = op { Some((text.as_str(), *continues)) } else { None }).collect();
    assert_eq!(texts, [("　怪我をする。\u{1}痛い。\u{1}\nもがき苦しみ", true), ("　兵士", false)]);
    let wait = c.ops.iter().position(|op| matches!(op, Op::Pause { ms: 5 })).unwrap();
    assert!(matches!(c.ops[wait - 1], Op::Text { continues: true, .. }));
}

#[test]
fn a_choice_forks_the_run_to_each_branch_file() {
    let mut files = nss_files("function SetChoice02($テキストデータ１,$テキストデータ２){}", &[]);
    let main = |name: &str, body: &str| format!("scene {name}_MAIN\n{{\n{body}\n}}\n");
    files.insert(
        "a.nss".into(),
        nss::parse(&main(
            "a.nss",
            r#"SetChoice02("はい","いいえ");
	select { if($x){$x=0;} case @選択肢１ { $GameName = "b.nss"; $flag = 1; } case @選択肢２ { $GameName = "c.nss"; } }"#,
        ))
        .unwrap(),
    );
    files.insert("b.nss".into(), nss::parse(&main("b.nss", r#"$GameName = "d.nss";"#)).unwrap());
    files.insert("c.nss".into(), nss::parse(&main("c.nss", r#"if($flag){$GameName = "b.nss";}else{$GameName = "d.nss";}"#)).unwrap());
    files.insert("d.nss".into(), nss::parse(&main("d.nss", "$GameCircle = 0;")).unwrap());
    let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
    assert_eq!(played, ["a.nss", "b.nss", "c.nss", "d.nss"]);
    let Some(Op::Choice { options, style, .. }) = c.ops.iter().find(|op| matches!(op, Op::Choice { .. })) else { panic!() };
    assert_eq!(options.iter().map(|o| o.0.as_str()).collect::<Vec<_>>(), ["はい", "いいえ"]);
    // Two plates, the first at x 562; one vertical line on each.
    let w = &style.as_ref().unwrap().window;
    assert_eq!((w.x, w.rows, w.vertical, w.font_size + w.line_gap), (562, 2, true, 100));
    let var = conv_core::ir::Var { global: false, index: FORK_VARS.start };
    assert!(c.ops.contains(&Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(0), label: "f_b".into() }));
    assert!(c.ops.contains(&Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(1), label: "f_c".into() }));
    // $flag is a story flag (set and tested by story files): set in the
    // first branch at run time, tested in c.nss at run time.
    let flag = story_flags(&files, &Game::PLAIN)["$flag"];
    assert!(c.ops.contains(&Op::Arith { var: flag, op: ArithOp::Set, value: Operand::Const(1) }));
    let c_start = c.ops.iter().position(|op| *op == Op::Label("f_c".into())).unwrap();
    assert!(matches!(&c.ops[c_start + 1], Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(0), .. } if *var == flag), "{:?}", &c.ops[c_start..]);
}

#[test]
fn a_choice_option_set_in_a_nested_runtime_if_is_picked_at_run_time() {
    let mut files = nss_files("function SetChoice02($テキストデータ１,$テキストデータ２){}", &[]);
    // As md05_004hazure01: the option depends on a companion, inside
    // another runtime if.
    let a = r#"scene a.nss_MAIN
{
	select { case @選択肢１ { $day = 1; $friend = "x"; } case @選択肢２ { $day = 1; $friend = ""; } case @選択肢３ { $day = 2; } }
	if($day == 1){
		if($friend == ""){ $ask = "頼む"; } else { $ask = "別れる"; }
	}
	SetChoice02("見る", $ask);
	select { case @選択肢１ { $GameName = "b.nss"; } case @選択肢２ { $GameName = "b.nss"; } }
}"#;
    files.insert("a.nss".into(), nss::parse(a).unwrap());
    files.insert("b.nss".into(), nss::parse("scene b.nss_MAIN\n{\n\tif($day == 1 && $friend == 2){$x = 1;}\n\t$GameCircle = 0;\n}").unwrap());
    let (c, _) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
    let ask = story_flags(&files, &Game::PLAIN)["$ask"];
    assert!(CHOICE_FLAGS.contains(&ask.index), "{ask:?}");
    let Some(Op::RuntimeChoice { options, texts, .. }) = c.ops.iter().find(|op| matches!(op, Op::RuntimeChoice { .. })) else { panic!("{:?}", c.ops) };
    assert_eq!(options[0].0, "見る");
    assert!(texts[0].is_none());
    let t = texts[1].as_ref().unwrap();
    assert_eq!(t.var, ask);
    assert_eq!(t.texts.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>(), ["頼む", "別れる"]);
    // Its line in the log too (`SetBacklog("【" + $ask + "】")`).
    let mark = format!("【{}{}{}】x{}9{}", RUNTIME_MARK.0, ask.index, RUNTIME_MARK.1, RUNTIME_MARK.0, RUNTIME_MARK.1);
    assert_eq!(runtime_text(&mark), Some((ask, "【".to_string(), "】x".to_string())));
}

#[test]
fn a_choice_that_names_its_own_file_replays_it() {
    // As md05_010vsaa: option 1 shows some text and asks again.
    let mut files = nss_files("function SetChoice02($テキストデータ１,$テキストデータ２){}", &[]);
    let main = |name: &str, body: &str| format!("scene {name}_MAIN\n{{\n{body}\n}}\n");
    let a = r#"SetChoice02("もう一度","終わる");
	select { case @選択肢１ { $GameName = "a.nss"; } case @選択肢２ { $GameName = "b.nss"; } }"#;
    files.insert("a.nss".into(), nss::parse(&main("a.nss", a)).unwrap());
    files.insert("b.nss".into(), nss::parse(&main("b.nss", "$GameCircle = 0;")).unwrap());
    let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
    assert_eq!(played, ["a.nss", "b.nss"]);
    let var = conv_core::ir::Var { global: false, index: FORK_VARS.start };
    assert!(c.ops.contains(&Op::If { var, cmp: conv_core::ir::Compare::Eq, value: Operand::Const(0), label: "f_a".into() }), "{:?}", c.ops);
}

/// Runs the variable and jump ops of `ops` (everything else is skipped).
fn run_ir(ops: &[Op]) -> HashMap<conv_core::ir::Var, i32> {
    run_ir_picking(ops, &[])
}

/// [`run_ir`], taking option `picks[n]` (0-based) at the nth choice.
fn run_ir_picking(ops: &[Op], picks: &[usize]) -> HashMap<conv_core::ir::Var, i32> {
    use conv_core::ir::Compare;
    let mut picks = picks.iter();
    let mut vars: HashMap<conv_core::ir::Var, i32> = HashMap::new();
    let labels: HashMap<&str, usize> = ops.iter().enumerate().filter_map(|(i, op)| if let Op::Label(l) = op { Some((l.as_str(), i)) } else { None }).collect();
    let mut pc = 0;
    let mut steps = 0;
    while pc < ops.len() && steps < 100_000 {
        steps += 1;
        let value = |o: &Operand, vars: &HashMap<_, i32>| match o {
            Operand::Const(n) => *n,
            Operand::Var(v) => vars.get(v).copied().unwrap_or(0),
        };
        match &ops[pc] {
            Op::Jump(l) => {
                pc = labels[l.as_str()];
                continue;
            }
            Op::Choice { options, .. } => {
                pc = labels[options[*picks.next().unwrap_or(&0)].1.as_str()];
                continue;
            }
            Op::If { var, cmp, value: v, label } => {
                let (a, b) = (vars.get(var).copied().unwrap_or(0), value(v, &vars));
                let hit = match cmp {
                    Compare::Eq => a == b,
                    Compare::Ne => a != b,
                    Compare::Lt => a < b,
                    Compare::Gt => a > b,
                    Compare::Le => a <= b,
                    Compare::Ge => a >= b,
                };
                if hit {
                    pc = labels[label.as_str()];
                    continue;
                }
            }
            Op::Arith { var, op, value: v } => {
                let n = value(v, &vars);
                let e = vars.entry(*var).or_insert(0);
                match op {
                    ArithOp::Set => *e = n,
                    ArithOp::Add => *e += n,
                    ArithOp::Sub => *e -= n,
                    _ => {}
                }
            }
            _ => {}
        }
        pc += 1;
    }
    vars
}

#[test]
fn a_scene_name_set_by_a_runtime_if_is_called_at_run_time_in_each_file() {
    let mut files = nss_files("function F(){}", &[]);
    // As md05_004*: the scene to call is picked by a story flag, and
    // every file uses the same helper variable for it.
    let file = |name: &str, x: i32, next: &str| {
        format!(
            r#"scene {name}_MAIN
{{
	if($f == true){{$S = "@->"+$GameName+"_X";}}
	else{{$S = "@->"+$GameName+"_Y";}}
	call_scene $S;
	$GameName = "{next}";
}}
scene {name}_X
{{
	$g = {x};
}}
scene {name}_Y
{{
	$g = {};
}}"#,
            x + 1
        )
    };
    files.insert("a.nss".into(), nss::parse(&file("a.nss", 1, "b.nss")).unwrap());
    files.insert("b.nss".into(), nss::parse(&file("b.nss", 3, "d.nss")).unwrap());
    files.insert("d.nss".into(), nss::parse("scene d.nss_MAIN\n{\n\tif($g){$f = 1;}\n\t$GameCircle = 0;\n}").unwrap());
    let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
    assert_eq!(played, ["a.nss", "b.nss", "d.nss"]);
    let flags = story_flags(&files, &Game::PLAIN);
    // $f is 0: each file calls its own _Y scene, b.nss's last.
    assert_eq!(run_ir(&c.ops).get(&flags["$g"]), Some(&4));
    assert!(!c.notes.contains_key("call_scene to a runtime value with no known names"), "{:?}", c.notes);
}

#[test]
fn a_replay_compiles_a_file_again_with_its_variables_set() {
    const GAME: Game = Game {
        replays: |_| vec![Replay { label: "conv_r".into(), file: "r.nss".into(), set: vec![("$PLACE_reco".into(), 1)] }],
        ..Game::PLAIN
    };
    let files = nss_files("function F(){}", &[("a.nss", "scene a.nss_MAIN\n{\n\tif($f){$g = 1;}\n\t$GameCircle = 0;\n}"), ("r.nss", "scene r.nss_MAIN\n{\n\tif($PLACE_reco){$f = 7;}else{$f = 3;}\n\t$GameCircle = 0;\n}")]);
    let (c, _) = compile_run(&files, "a.nss", 10, &GAME, &mut Fake).unwrap();
    let f = story_flags(&files, &GAME)["$f"];
    let at = c.ops.iter().position(|op| *op == Op::Label("conv_r".into())).expect("the replay's label");
    let sets: Vec<_> = c.ops[at..].iter().filter_map(|op| match op {
        Op::Arith { var, op: ArithOp::Set, value: Operand::Const(n) } if *var == f => Some(*n),
        _ => None,
    }).collect();
    assert_eq!(sets.first(), Some(&7), "{:?}", &c.ops[at..]);
    assert!(c.ops[at..].iter().any(|op| matches!(op, Op::Return)));
}

#[test]
fn flags_a_choice_sets_are_tested_at_run_time_after_it() {
    let mut files = nss_files("function F(){}", &[]);
    // As mc01_010vs: the cases set flags, the next file is picked after
    // the select.
    let a = r#"scene a.nss_MAIN
{
	select { case @選択肢１ { $x = true; } case @選択肢２ { $y = true; } }
	if($x == true){ $GameName = "b.nss"; } else if($y){ $GameName = "c.nss"; }
}"#;
    files.insert("a.nss".into(), nss::parse(a).unwrap());
    for n in ["b.nss", "c.nss"] {
        files.insert(n.into(), nss::parse(&format!("scene {n}_MAIN\n{{\n\t$GameCircle = 0;\n}}")).unwrap());
    }
    let (_, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
    assert_eq!(played, ["a.nss", "b.nss", "c.nss"]);
}

#[test]
fn a_choice_inside_a_runtime_if_hands_each_option_s_file_to_the_code_after_it() {
    let mut files = nss_files("function F(){}", &[]);
    // As the airship rooms (mc02_031souko2): the select sets $GameName,
    // the code after it hands that to a story flag and goes through the
    // clock file, which dispatches on the flag.
    let a = r#"scene a.nss_MAIN
{
	if($joined == true){ $GameName = "a.nss"; }
	else{
		select { case @選択肢１ { $GameName = "b.nss"; } case @選択肢２ { $GameName = "c.nss"; } }
	}
	$Next = $GameName;
	$GameName = "t.nss";
}"#;
    files.insert("a.nss".into(), nss::parse(a).unwrap());
    files.insert("t.nss".into(), nss::parse("scene t.nss_MAIN
{
	$GameName = $Next;
}").unwrap());
    let t_reads = "scene x.nss_MAIN
{
	if($Next == \"b.nss\"){$joined = true;}
	$GameCircle = 0;
}";
    for n in ["b.nss", "c.nss"] {
        files.insert(n.into(), nss::parse(&t_reads.replace("x.nss", n)).unwrap());
    }
    let (c, played) = compile_run(&files, "a.nss", 10, &Game::PLAIN, &mut Fake).unwrap();
    assert!(played.contains(&"c.nss".to_string()), "{played:?}");
    let flags = story_flags(&files, &Game::PLAIN);
    let next = flags["$Next"];
    let code = |name: &str| c.ops.iter().find_map(|op| match op {
        Op::If { var, value: Operand::Const(n), label, .. } if *var == next && label == &file_label(name) => Some(*n),
        _ => None,
    });
    for (pick, file) in [(0, "b.nss"), (1, "c.nss")] {
        let vars = run_ir_picking(&c.ops, &[pick]);
        assert_eq!(Some(vars[&next]), code(file), "option {} should hand on {file}", pick + 1);
    }
}

#[test]
fn full_width_maps_ascii_and_half_width_kana_one_for_one() {
    assert_eq!(full_width("A!? ･ｶﾞｰ｡"), "Ａ！？　・カ゛ー。");
    assert_eq!(HALF_KANA.chars().count(), 0xFF9F - 0xFF61 + 1);
}

#[test]
fn route_ranking_runs_as_runtime_comparisons() {
    let story = r#"scene a.nss_MAIN
{
	$Others_Flag = 1; $Ichizyou_Flag = 3; $Kanae_Flag = 5; $Muramasa_Flag = 5;
	RouteChicker();
	$GameCircle = 0;
}"#;
    let files = nss_files("function RouteChicker(){}", &[("a.nss", story)]);
    let (c, _) = compile_run(&files, "a.nss", 1, &MURAMASA, &mut Fake).unwrap();
    let vars = run_ir(&c.ops);
    let flags = story_flags(&files, &MURAMASA);
    let rank = |n: &str| vars.get(&flags[&format!("$RC_N[{n}]")]).copied();
    // 香奈枝 and 村正 tie at 5: 香奈枝 is checked first.
    assert_eq!([rank("香奈枝"), rank("村正"), rank("一条"), rank("アナザー")], [Some(1), Some(2), Some(3), Some(4)]);
}

#[test]
fn the_heroine_with_the_highest_counter_dies_and_set_hex_clears_her_counter() {
    let story = r#"scene a.nss_MAIN
{
	$Others_Flag = 1; $Ichizyou_Flag = 4; $Kanae_Flag = 4; $Muramasa_Flag = 2;
	SatugaiSystem();
	SetHex();
	if($一条死亡 == true){ $x = 1; }
	$GameCircle = 0;
}"#;
    let files = nss_files("function SatugaiSystem(){}
function SetHex(){}", &[("a.nss", story)]);
    let (c, _) = compile_run(&files, "a.nss", 1, &MURAMASA, &mut Fake).unwrap();
    let vars = run_ir(&c.ops);
    let flags = story_flags(&files, &MURAMASA);
    let get = |k: &str| vars.get(&flags[k]).copied().unwrap_or(0);
    // 一条 and 香奈枝 tie at 4: 一条 comes first in SatugaiSystem's order.
    assert_eq!([get("$一条死亡"), get("$香奈枝死亡"), get("$その他死亡"), get("$村正死亡")], [1, 0, 0, 0]);
    assert_eq!([get("$Ichizyou_Flag"), get("$Kanae_Flag")], [0, 4]);
}

#[test]
fn glob_matches_stars_anywhere() {
    assert!(glob("選択肢*/MouseUsual/*", "選択肢１/MouseUsual/選択肢１板１"));
    assert!(!glob("選択肢*/MouseUsual/*", "選択肢１/MouseOver/選択肢１板２"));
    assert!(glob("a*", "a/b"));
    assert!(glob("*b", "ab") && !glob("*b", "ba"));
    assert!(!glob("a*c", "ab"));
}

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
    let c = compile_scene(&files, "a.nss", "a.nss", &GAME, &mut Fake).unwrap();
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
