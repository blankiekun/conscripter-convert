use super::*;
use conv_core::ir::{Compare, MotionEnd, MotionKey};
use conv_core::ir::{Animation, ButtonScreen, ChoiceStyle, Noise, Placed, SourceLoc, SpriteChange, SpriteFilter, SpritePlacement, Voice, FILTER_BLEND_COPY, FILTER_BLEND_MULTIPLY, FILTER_BRIGHTNESS, FILTER_COLOUR};

#[test]
fn ons_args_ignore_the_bar_for_auto_play() {
    assert_eq!(engine_args(&Script::default()), ENGINE_ARGS);
    let script = Script { button_bar: Some(conv_core::ir::ButtonBar::default()), ..Default::default() };
    assert_eq!(engine_args(&script), format!("{ENGINE_ARGS}--trace-ignore\n34-48\n"));
}

fn script(ops: Vec<Op>) -> Script {
    Script { title: "Test".into(), width: 800, height: 600, ops, ..Default::default() }
}

fn link(start: usize, len: usize, target: Option<&str>) -> TextLink {
    TextLink { start, len, target: target.map(String::from), scene: None }
}

fn window() -> TextWindow {
    use conv_core::ir::NamePlate;
    TextWindow {
        wait_icon_at: None,
        image: "system/window.png".into(),
        x: 47,
        y: 452,
        text_x: 105,
        text_y: 470,
        columns: 25,
        rows: 3,
        font_size: 24,
        line_gap: 13,
        vertical: false,
        name_plate: Some(NamePlate {
            image: "system/name.png".into(),
            x: 55,
            y: 412,
            text_x: 75,
            text_y: 422,
            font_size: 20,
        }),
    }
}

#[test]
fn a_choice_of_plates_is_picture_buttons_with_centred_text() {
    let window = TextWindow { wait_icon_at: None, image: "system/w.png".into(), x: 0, y: 0, text_x: 0, text_y: 0, columns: 10, rows: 2, font_size: 22, line_gap: 0, vertical: false, name_plate: None };
    let plates = conv_core::ir::ChoicePlates { image: "system/plate.png".into(), x: 138, y: 244, step: 68, text_x: 388, text_dy: 18, font_size: 22, colour: "#FFFFFF".into(), fade_ms: 300, shot: None, vertical: false, chosen_ms: 0 };
    let options = vec![("はい".to_string(), "a".to_string()), ("OK".to_string(), "b".to_string())];
    let style = Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) });
    let text = emit(&script(vec![Op::Choice { options, style, disabled: vec![] }]));
    for part in [
        "lsp %conv_link1,\":a/2,0,3;system\\plate.png\",138,244\nstrsp %conv_link0,\"はい\",366,262,3,1,22,22,0,0,0,1,\"#FFFFFF\",\"#FFFFFF\"\n",
        "strsp %conv_link2,\"OK\",377,330,",
        "print 10,300\n*conv_cplate0_wait\nbtndef clear\nspbtn %conv_link1,1\nspbtn %conv_link3,2\nbtnwait %9\n",
        "if %9 == 2 goto *b\n",
    ] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
    assert!(!text.contains("select "));
}

#[test]
fn vertical_plates_step_left_with_column_text_and_a_chosen_cell() {
    let window = TextWindow { wait_icon_at: None, image: "system/w.png".into(), x: 0, y: 0, text_x: 0, text_y: 0, columns: 10, rows: 2, font_size: 20, line_gap: 0, vertical: true, name_plate: None };
    let plates = conv_core::ir::ChoicePlates { image: "system/plate.png".into(), x: 562, y: 68, step: 100, text_x: 577, text_dy: 40, font_size: 20, colour: "#000000".into(), fade_ms: 500, shot: None, vertical: true, chosen_ms: 1000 };
    let options = vec![("はい".to_string(), "a".to_string()), ("いいえ".to_string(), "b".to_string())];
    let style = Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) });
    let text = emit(&script(vec![Op::Choice { options, style, disabled: vec![] }]));
    for part in [
        "lsp %conv_link1,\":a/3,0,3;system\\plate.png\",562,68\nvstrsp %conv_link0,\"はい\",577,108,1,3,20,20,",
        "lsp %conv_link3,\":a/3,0,3;system\\plate.png\",462,68\nvstrsp %conv_link2,\"いいえ\",477,108,1,4,",
        "if %9 == 2 cell %conv_link3,2\nprint 1:delay 1000\ncsp %conv_link0",
    ] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
}

#[test]
fn a_runtime_choice_sets_its_option_texts_from_a_variable() {
    let var = Var { global: false, index: 5 };
    let rt = conv_core::ir::RuntimeText { var, texts: vec![(1, "頼む".into()), (2, "Leave".into())] };
    let options = vec![("見る".to_string(), "a".to_string()), ("頼む".to_string(), "b".to_string())];
    let text = emit(&script(vec![Op::RuntimeChoice { options: options.clone(), texts: vec![None, Some(rt.clone())], style: None }]));
    for part in ["mov $conv_choice_text1,\"頼む\"", "if %15 == 2 mov $conv_choice_text1,\"　Leave\"", "select \"見る\",*a,$conv_choice_text1,*b"] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
    // On plates: the text and its centred x, set together.
    let window = TextWindow { wait_icon_at: None, image: "system/w.png".into(), x: 0, y: 0, text_x: 0, text_y: 0, columns: 10, rows: 2, font_size: 22, line_gap: 0, vertical: false, name_plate: None };
    let plates = conv_core::ir::ChoicePlates { image: "system/plate.png".into(), x: 138, y: 244, step: 68, text_x: 388, text_dy: 18, font_size: 22, colour: "#FFFFFF".into(), fade_ms: 300, shot: None, vertical: false, chosen_ms: 0 };
    let style = Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) });
    let text = emit(&script(vec![Op::RuntimeChoice { options, texts: vec![None, Some(rt)], style }]));
    for part in ["if %15 == 2 mov $conv_choice_text1,\"Leave\":mov %conv_choice_x1,361", "strsp %conv_link2,$conv_choice_text1,%conv_choice_x1,330,6,"] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
    let text = emit(&script(vec![Op::RuntimeBacklogLine { text: "【】".into(), runtime: conv_core::ir::RuntimeText { var, texts: vec![(2, "【Leave】".into())] } }]));
    assert!(text.contains("mov $conv_log_line,\"【】\"\nif %15 == 2 mov $conv_log_line,\"【Leave】\"\nlogadd $conv_log_line\n"), "{text}");
}

#[test]
fn a_shot_plate_gets_its_hole_and_tips_over_about_its_bottom() {
    let window = TextWindow { wait_icon_at: None, image: "system/w.png".into(), x: 0, y: 0, text_x: 0, text_y: 0, columns: 10, rows: 2, font_size: 22, line_gap: 0, vertical: false, name_plate: None };
    let shot = Some(conv_core::ir::ChoiceShot { hole: "system/hole.png".into(), hole_dx: -47, hole_dy: -41, sound: "se/shot.ogg".into() });
    let plates = conv_core::ir::ChoicePlates { image: "system/plate.png".into(), x: 138, y: 244, step: 68, text_x: 388, text_dy: 18, font_size: 22, colour: "#FFFFFF".into(), fade_ms: 300, shot, vertical: false, chosen_ms: 0 };
    let options = vec![("はい".to_string(), "a".to_string()), ("OK".to_string(), "b".to_string())];
    let style = Some(ChoiceStyle { window, on_color: 0, off_color: 0, plates: Some(plates) });
    let text = emit(&script(vec![Op::Choice { options, style, disabled: vec![] }]));
    for part in [
        "getmousepos %0,%1:add %0,-47:add %1,-41\ndwave 10,\"se\\shot.ogg\"\nif %9 == 1 goto *conv_cplate0_s1\n",
        "*conv_cplate0_s2\ncell %conv_link3,1:spcut %conv_link3,\"system\\hole.png\",%0,%1:spcut %conv_link2,\"system\\hole.png\",%0,%1:print 1\ndelay 300\ncsp %conv_link0:csp %conv_link1:print 10,300\n",
        // The last step: a quarter high, its bottom still at 380.
        "spzoom %conv_link3,100,27:spzoom %conv_link2,100,27:amsp %conv_link3,138,337,64:amsp %conv_link2,377,358,64:print 1:delay 50\ncsp %conv_link2:csp %conv_link3:print 1\ngoto *b\n",
    ] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
}

#[test]
fn a_held_movie_draws_its_prompt_and_plays_its_sound_on_an_effect_channel() {
    use conv_core::ir::{HoldPrompt, HoldSound};
    let prompt = Some(HoldPrompt { image: "system/push.png".into(), x: 690, y: 20, cells: 10, frame_ms: 100 });
    let sound = Some(HoldSound { file: "sound/se/fire.ogg".into(), fade_ms: 500 });
    let text = emit(&script(vec![Op::HoldMovie { file: "movie/a.mp4".into(), prompt, sound }]));
    let part = "lsp 6,\":a/10,100,0;system\\push.png\",690,20\nmpeghold \"movie\\a.mp4\",6,1,\"sound\\se\\fire.ogg\",500\ncsp 6\n";
    assert!(text.contains(part), "missing {part:?} in\n{text}");
    let text = emit(&script(vec![Op::HoldMovie { file: "movie/a.mp4".into(), prompt: None, sound: None }]));
    assert!(text.contains("mpeghold \"movie\\a.mp4\"\n"));
}

#[test]
fn a_flashing_sprite_is_added_on_with_its_keys_repeated() {
    let keys = vec![(0, 76), (17, 76), (97, 0), (114, 0)];
    let text = emit(&script(vec![Op::SpriteFlashing { layer: 3, keys: keys.clone(), times: 60 }]));
    let slot = SPRITE_BASE + 3;
    assert!(text.contains(&format!("spblend {slot},1:spmotion {slot},-60,0,0,0,100,100,0,76,0,17,0,0,100,100,0,76,0,97,0,0,100,100,0,0,0,114,0,0,100,100,0,0,0\nprint 1\n")), "{text}");
    // Once: spmotion's loop 2, keeping the last key (1 would repeat it for ever).
    let text = emit(&script(vec![Op::SpriteFlashing { layer: 3, keys, times: 1 }]));
    assert!(text.contains(&format!("spmotion {slot},2,0,0,0,100,100,0,76,")), "{text}");
}

#[test]
fn clickable_links_call_their_target() {
    let mut s = script(vec![
        Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
        Op::BgmPlay { file: "bgm/a.ogg".into(), fade_in_ms: 0 },
        Op::Text {
            speaker: None,
            text: "一行目\n学校に―――".into(),
            voice: None,
            links: vec![link(4, 2, Some("s10_l1")), link(0, 1, None)],
            ruby: vec![],
            styles: vec![],
            hang: None,
            continues: false,
        },
    ]);
    s.window = Some(window());
    let text = emit(&s);
    assert!(text.contains("defarr conv_scene_replayed,446,global\nwindowback\nhumanz 58\n"));
    // Row 2, column 1: x = 105 + 0 * 25, y = 470 + 1 * 37.
    let expected = "\
*conv_link1
csp %conv_name_plate
csp %conv_name_text
print 1
#FFE193一#FFFFFF行目
#FFE193学校#FFFFFFに―――
if ?conv_link_visited[0] == 0 strsp %conv_link0,\"学校\",105,507,3,1,24,24,1,13,0,1,\"#FFE193\",\"#FFCA5E\"
if ?conv_link_visited[0] != 0 strsp %conv_link0,\"学校\",105,507,3,1,24,24,1,13,0,1,\"#DDB9FF\",\"#CC96FF\"
*conv_link1_wait
if ?conv_link_visited[0] == 0 lsp 6,\":c;>49,1,#FFE193\",107,532
if ?conv_link_visited[0] != 0 lsp 6,\":c;>49,1,#DDB9FF\",107,532
lsp 7,\":c;>49,1,#000000\",108,533
spbtn %conv_link0,1
print 1
btnwait %9
csp %conv_link0
csp 6
csp 7
print 1
textclear
if %9 != 1 goto *conv_link1_1
mov ?conv_link_visited[0],1
pushscreen 62,999
gosub *s10_l1
popscreen
setwindow3 105,470,25,3,24,24,1,13,20,0,1,\":a;system\\window.png\",47,452
print 1
goto *conv_link1
*conv_link1_1
";
    assert!(text.contains(expected), "{text}");
}

#[test]
fn link_scenes_nest_at_most_seven_deep() {
    let mut s = script(vec![
        Op::Overlay { image: Some("bg/frame.png".into()), over_sprites: true },
        Op::Text {
            speaker: None,
            text: "学校に".into(),
            voice: None,
            links: vec![link(0, 2, Some("s10_l1"))],
            ruby: vec![],
            styles: vec![],
            hang: None,
            continues: false,
        },
    ]);
    s.window = Some(window());
    let text = emit(&s);
    assert!(text.contains("\nif %conv_overlay_depth < 7 spbtn %conv_link0,1\n"), "{text}");
    // popscreen leaves the overlay's handle (over 999) as the scene left it.
    assert!(text.contains("sub %conv_overlay_depth,1\ncsp %conv_overlay\nif %conv_overlay_slot != 0 lsp %conv_overlay_slot,"), "{text}");
}

#[test]
fn effects_past_255_are_written_inline() {
    let ops: Vec<Op> = (1..=300)
        .flat_map(|ms| [Op::BeginTransition { effect: "Fade".into(), fade_ms: ms, mask: None, style: TransitionStyle::Fade }, Op::EndTransition])
        .collect();
    let text = emit(&script(ops));
    assert!(!text.contains("\neffect "), "{text}");
    assert!(text.contains("print 10,300\n"));
    let few = emit(&script(vec![Op::BeginTransition { effect: "Fade".into(), fade_ms: 300, mask: None, style: TransitionStyle::Fade }, Op::EndTransition]));
    assert!(few.contains("effect 2,10,300\n") && few.contains("print 2\n"), "{few}");
}

#[test]
fn an_eased_transition_prints_after_its_curve() {
    let fade = |ease| vec![Op::BeginTransition { effect: "Fade".into(), fade_ms: 300, mask: None, style: TransitionStyle::Fade }, Op::TransitionEase { ease, exponent: 3 }, Op::EndTransition];
    let text = emit(&script(fade(2)));
    assert!(text.contains("effectcurve 32\nprint 2\n"), "{text}");
    assert!(text.contains("\nspcurve 32,0,"), "{text}");
    // An unknown curve: straight.
    assert!(!emit(&script(fade(0))).contains("effectcurve"));
}

#[test]
fn a_second_motion_runs_beside_the_first_with_spmotionadd() {
    let key = |ms, x, opacity| MotionKey { ms, x, y: 0, zoom: 100, zoom_y: 100, angle: 0, opacity, ease: None };
    let text = emit(&script(vec![
        Op::SpriteMotion2 { layer: 3, keys: vec![key(0, -200, 256), key(2000, 0, 256)], end: MotionEnd::Keep },
        Op::MoveEase { ease: 2, exponent: 3 },
        Op::SpriteMotionAdd { layer: 3, keys: vec![key(0, 0, 0), key(500, 0, 256)], end: MotionEnd::Back },
        Op::SpriteMotionAdd { layer: 3, keys: vec![], end: MotionEnd::Keep },
    ]));
    let slot = SPRITE_BASE + 3;
    assert!(text.contains(&format!("spmotion {slot},2,0,-200,0,100,100,0,256,0,2000,0,0,100,100,0,256,0\n")), "{text}");
    // (Eased by the MoveEase before it, as a SpriteMotion2 is.)
    assert!(text.contains(&format!("spmotionadd {slot},0,0,0,0,100,100,0,0,")), "{text}");
    assert!(!text.contains(&format!("spmotionadd {slot},0,0,0,0,100,100,0,0,0,")), "{text}");
    assert!(text.contains(&format!("spmotionadd {slot},2\n")), "{text}");
}

#[test]
fn a_click_skipping_effects_jumps_the_skip_gate() {
    let gate = vec![Op::IfSkipping { label: "fx".into() }, Op::Label("fx".into())];
    let text = emit(&Script { click_skips_effects: true, ..script(gate.clone()) });
    assert!(text.contains("clickskip\ngame\n") && text.contains("getclickskip "), "{text}");
    assert!(!emit(&script(gate)).contains("clickskip"));
}

#[test]
fn jpeg_sprites_are_opaque() {
    let jpeg = SpritePlacement { image: "cg/bg/a.JPG".into(), x: 0, y: 0 };
    assert_eq!(lsp(70, &jpeg), "lsp 70,\":c;cg\\bg\\a.JPG\",0,0");
    let png = SpritePlacement { image: "cg/st/a.png".into(), x: 1, y: 2 };
    assert_eq!(lsp(70, &png), "lsp 70,\":a;cg\\st\\a.png\",1,2");
    let under = SpritePlacement { image: conv_core::ir::SCREEN_UNDER.into(), x: 0, y: 0 };
    assert_eq!(lsp(70, &under), "spgrab 70,-3");
    let moved = SpritePlacement { image: conv_core::ir::SCREEN_UNDER.into(), x: 2, y: 0 };
    assert_eq!(lsp(70, &moved), "spgrab 70,-3:msp 70,2,0,0");
}

#[test]
fn vertical_window_on_a_wide_screen() {
    let vertical = TextWindow { vertical: true, name_plate: None, x: 447, y: 40, text_x: 430, text_y: 55, columns: 27, rows: 5, font_size: 20, line_gap: 9, ..window() };
    let mut s = script(vec![Op::SetWindow(window())]);
    (s.width, s.height, s.window) = (1024, 576, Some(vertical));
    let text = emit(&s);
    assert!(text.starts_with(";$S1024,576,V8192\n"), "{text}");
    assert!(text.contains("tateyoko 1:setwindow3 430,55,5,27,20,20,9,0,20,0,1,\":a;system\\window.png\",447,40\n"), "{text}");
    assert!(text.contains("tateyoko 0:setwindow3 105,"), "{text}");
}

#[test]
fn styled_choice_has_its_own_window_and_colours() {
    let plates = TextWindow { vertical: true, name_plate: None, x: 462, y: 68, text_x: 477, text_y: 108, columns: 18, rows: 2, font_size: 20, line_gap: 80, ..window() };
    let style = Some(ChoiceStyle { window: plates, on_color: 0xA00000, off_color: 0, plates: None });
    let options = vec![("はい".into(), "a".into()), ("いいえ".into(), "b".into())];
    let text = emit(&script(vec![Op::Choice { options, style, disabled: vec![] }]));
    // No shadow; no ruby here, so text_x as given.
    assert!(text.contains("selectcolor \"#A00000\",\"#000000\"\ntateyoko 1:setwindow3 477,108,2,18,20,20,80,0,20,0,0,"), "{text}");
    assert!(text.contains("select \"はい\",*a,\"いいえ\",*b\n"), "{text}");
}

#[test]
fn disabled_options_are_greyed_just_before_the_select() {
    let options = vec![("一".into(), "a".into()), ("二".into(), "b".into()), ("三".into(), "c".into())];
    let text = emit(&script(vec![Op::Choice { options, style: None, disabled: vec![0, 2] }]));
    assert!(text.contains("seldisable 1,3\nselect \"一\",*a,"), "{text}");
}

#[test]
fn button_screen_waits_for_its_buttons_and_fades_out() {
    let placed = |image: &str, x, y| Placed { image: image.into(), x, y, w: 10, h: 10 };
    let s = ButtonScreen {
        pictures: vec![placed("system/bg.png", 0, 0), placed("system/text.png", 300, 319)],
        buttons: vec![(placed("system/yes.png", 385, 230), "scene_jump".into()), (placed("system/no.png", 515, 230), "scene_play".into())],
        fade_ms: 300,
    };
    let text = emit(&script(vec![Op::ButtonScreen(Box::new(s)), Op::Label("scene_jump".into()), Op::Label("scene_play".into())]));
    // Buttons on top (6, 7), then the pictures from the top down.
    assert!(text.contains("textoff\nlsp 9,\":a;system\\bg.png\",0,0\nlsp 8,\":a;system\\text.png\",300,319\nlsp 6,\":a/2,0,3;system\\yes.png\",385,230\nlsp 7,"), "{text}");
    assert!(text.contains("*conv_bscreen0\nbtndef clear\nspbtn 6,1\nspbtn 7,2\nbtnwait %9\nif %9 == 1 csp 6:csp 7:csp 8:csp 9:print 2:goto *scene_jump\nif %9 == 2 "), "{text}");
    assert!(text.contains("goto *scene_play\ngoto *conv_bscreen0\n"), "{text}");
}

#[test]
fn button_screen_past_28_sprites_takes_the_extra_slots() {
    let placed = |x| Placed { image: "system/b.png".into(), x, y: 0, w: 10, h: 10 };
    let buttons = (0..40).map(|i| (placed(i), "end".to_string())).collect();
    let s = ButtonScreen { pictures: vec![placed(-1)], buttons, fade_ms: 0 };
    let text = emit(&script(vec![Op::ButtonScreen(Box::new(s)), Op::Label("end".into())]));
    // 6-33 as before, then 980 up drawn under 33 and over the bars.
    assert!(text.contains("lsp 33,\":a/2,0,3;system\\b.png\",27,0\nlsp 980,\":a/2,0,3;system\\b.png\",28,0:spz 980,34\n"), "{text}");
    assert!(text.contains("\nlsp 992,\":a;system\\b.png\",-1,0:spz 992,34\n"), "{text}");
    assert!(text.contains("spbtn 991,40\nbtnwait"), "{text}");
    assert_eq!(menu_slot(MENU_SLOTS - 1), 998);
}

#[test]
fn scene_replay_unlocks_are_global_flags() {
    let text = emit(&script(vec![Op::SceneEnd, Op::UnlockScene { scene: 5 }, Op::Progress { bit: 3 }, Op::CgSeen { number: 519 }]));
    assert!(text.contains("; scene end\nmov ?conv_scene_unlocked[5],1\nmov ?conv_progress[3],1\nmov ?conv_cg_seen[519],1\n"), "{text}");
}

#[test]
fn dissolve_passes_through_white() {
    let text = emit(&script(vec![
        Op::BeginTransition { effect: "Dissolve".into(), fade_ms: 1000, mask: None, style: TransitionStyle::ThroughWhite },
        Op::EndTransition,
    ]));
    let n = |ms: u32| text.lines().find(|l| l.starts_with("effect ") && l.ends_with(&format!(",10,{ms}"))).unwrap().split([' ', ',']).nth(1).unwrap().to_string();
    let expected = format!("lsp2 24,\":c;>800,600,#ffffff\",400,300,100,100,0,255\nprint {}\ncsp2 24\nprint {}\n", n(500), n(500));
    assert!(text.contains(&expected), "{text}");
}

#[test]
fn sprite_turns_are_engine_motions() {
    let turn = Op::SpriteTurn { layer: 7, image: "sprite/a.png".into(), from: (200, 400), to: (200, 900), degrees: -90, ms: 800 };
    let text = emit(&script(vec![turn]));
    let expected = "lsp 69,\":a;sprite\\a.png\",0,0:getspsize 69,%1,%2:amsp 69,200-%1/2,400-%2/2\n\
        spmotion 69,0,0,0,0,100,100,0,256,0,800,0,500,100,100,90,256,0:spmotionwait 69\ncsp 69\nprint 1\n";
    assert!(text.contains(expected), "{text}");
}

#[test]
fn return_to_root_unwinds_link_scenes() {
    let text = emit(&script(vec![Op::ReturnToRoot, Op::Wait { ms: 1 }]));
    assert!(text.contains("if %118 > 0 mov %119,1:return\n*conv_root1\ndelay 1\n"), "{text}");
    assert!(text.contains("*conv_unwind\nif %118 > 0 return\nmov %105,%119:mov %119,0\nif %105 == 1 goto *conv_root1\n"), "{text}");
}

#[test]
fn sound_rates_bend_the_sound_on_its_channel() {
    let s = script(vec![
        Op::SePlay { file: "se\\a.ogg".into(), looped: false },
        Op::SoundRate { file: "se\\a.ogg".into(), rate: 1100, ms: 1000 },
        Op::SoundRate { file: "se\\a.ogg".into(), rate: 1000, ms: 0 },
        Op::SoundRate { file: "se\\b.ogg".into(), rate: 900, ms: 0 },
    ]);
    let text = emit(&s);
    let ch = text.split("dwave ").nth(1).unwrap().split(',').next().unwrap();
    assert!(text.contains(&format!("chfreq {ch},1100,1000\nchfreq {ch},1000\n")), "{text}");
    assert!(!text.contains(",900"), "{text}");
}

#[test]
fn eased_sound_ramps_take_their_curve() {
    let s = script(vec![
        Op::SePlay { file: "se\\a.ogg".into(), looped: true },
        Op::SoundEase { ease: 2, exponent: 4 },
        Op::SoundRate { file: "se\\a.ogg".into(), rate: 400, ms: 1000 },
        Op::SoundEase { ease: 1, exponent: 3 },
        Op::SeFadeOut { file: Some("se\\a.ogg".into()), ms: 1500 },
    ]);
    let text = emit(&s);
    let ch = text.split("dwaveloop ").nth(1).unwrap().split(',').next().unwrap();
    assert!(text.contains(&format!("chfreq {ch},400,1000,33\nchvolfade {ch},0,1500,1,12\n")), "{text}");
    assert!(text.contains("spcurve 33,") && text.contains("spcurve 12,"), "{text}");
}

#[test]
fn bgm_crossfades_over_the_music_playing() {
    let text = emit(&script(vec![Op::BgmPlay { file: "bgm/a.ogg".into(), fade_in_ms: 0 }, Op::BgmCrossfade { file: "bgm/b.ogg".into(), ms: 3000 }]));
    assert!(text.contains("bgm \"bgm\\b.ogg\",3000\n"), "{text}");
}

#[test]
fn sprites_fading_fade_by_themselves() {
    let picture = SpritePlacement { image: "sprite/a.png".into(), x: 1, y: 2 };
    let fade = Op::SpritesFading {
        changes: vec![SpriteChange { layer: 6, sprite: None, keep_filters: false }, SpriteChange { layer: 5, sprite: Some(picture), keep_filters: false }],
        fade_ms: 300,
    };
    let hop = Op::SpriteMotion { layer: 5, keys: vec![(0, 0, 0), (150, 0, 37), (600, 0, 0)], looped: false };
    let text = emit(&script(vec![fade.clone(), hop.clone(), Op::Wait { ms: 100 }, hop]));
    assert!(text.contains("spmotion 68,3,0,0,0,100,100,0,256,0,300,0,0,100,100,0,0,0
spfilter 67,0
lsp 67,\":a;sprite\\a.png\",1,2
spmotion 67,0,0,0,0,100,100,0,0,0,300,0,0,100,100,0,256,0
print 1
"), "{text}");
    // the hop started with the fade takes it into its keys; one after a wait doesn't
    assert!(text.contains("spmotion 67,0,0,0,0,100,100,0,0,0,150,0,37,100,100,0,128,0,300,0,25,100,100,0,256,0,600,0,0,100,100,0,256,0\n"), "{text}");
    assert!(text.contains("spmotion 67,0,0,0,0,100,100,0,256,0,150,0,37,100,100,0,256,0,600,0,0,100,100,0,256,0\n"), "{text}");
}

#[test]
fn a_picture_after_a_zoomed_animation_is_drawn_unzoomed() {
    let movie = SpritePlacement { image: "system/movie/smoke.png".into(), x: 1, y: 2 };
    let picture = SpritePlacement { image: "sprite/a.png".into(), x: 1, y: 2 };
    let s = script(vec![
        Op::AnimatedSprite { layer: 5, sprite: movie, cells: 4, frame_ms: 66, looped: true, zoom: (400, 400) },
        Op::Sprites { changes: vec![SpriteChange { layer: 5, sprite: Some(picture.clone()), keep_filters: false }], fade_ms: 0 },
        Op::Sprites { changes: vec![SpriteChange { layer: 6, sprite: Some(picture), keep_filters: false }], fade_ms: 0 },
    ]);
    let text = emit(&s);
    assert!(text.contains("spzoom 67,400,400\n"), "{text}");
    assert!(text.contains("lsp 67,\":a;sprite\\a.png\",1,2\nspzoom 67,100,100\n"), "{text}");
    assert!(!text.contains("spzoom 68"), "{text}");
}

#[test]
fn transitions_reveal_changes_at_once() {
    let sprite = SpritePlacement { image: "sprite/a.png".into(), x: 1, y: 2 };
    let s = script(vec![
        Op::BeginTransition { effect: "CrossFade".into(), fade_ms: 2000, mask: None, style: TransitionStyle::Fade },
        Op::ClearSprites,
        Op::Background { image: "bg/a.png".into(), fade_ms: 500 },
        Op::Sprites { changes: vec![SpriteChange { layer: 5, sprite: Some(sprite), keep_filters: false }], fade_ms: 500 },
        Op::EndTransition,
        Op::BeginTransition {
            effect: "MaskBlend2".into(),
            fade_ms: 1000,
            mask: Some("system/mask0009r.png".into()),
            style: TransitionStyle::Fade,
        },
        Op::Background { image: "bg/b.png".into(), fade_ms: 500 },
        Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
    ]);
    let text = emit(&s);
    assert!(text.contains("effect 2,10,500\neffect 3,18,1000,\"system\\mask0009r.png\"\neffect 4,10,2000\n"), "{text}");
    let expected = "\
; transition: CrossFade
csp -1
csp -1
bg \"bg\\a.png\",0
spfilter 67,0
lsp 67,\":a;sprite\\a.png\",1,2
print 4
; transition: MaskBlend2
csp -1
bg \"bg\\b.png\",0
print 3
あ\\
";
    assert!(text.contains(expected), "{text}");
}

#[test]
fn transition_styles() {
    let begin = |style| Op::BeginTransition { effect: "x".into(), fade_ms: 1000, mask: None, style };
    let slide = |moving, dx, dy| begin(TransitionStyle::Slide { moving, dx, dy });
    let s = script(vec![
        slide(SlideLayer::New, -1, 0),
        Op::EndTransition,
        slide(SlideLayer::Both, 0, -1),
        Op::EndTransition,
        begin(TransitionStyle::Mosaic),
        Op::EndTransition,
        begin(TransitionStyle::Zoom { style: ZoomStyle::Cross, max_percent: 220, curve: 5 }),
        Op::EndTransition,
        begin(TransitionStyle::Melt { speed: 128 }),
        Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
    ]);
    let text = emit(&s);
    let defs = "\
effect 2,16,500
effect 3,17,500
effect 4,14,1000
effect 5,99,1000,\"cons_warp.dll/warp\\melt_128_1.png,33\"
effect 6,99,1000,\"cons_warp.dll/warp\\slide_0_-1_0.png,2\"
effect 7,99,1000,\"cons_warp.dll/warp\\zoom_3_220_5.png,65\"
";
    assert!(text.contains(defs), "{text}");
    let maps: Vec<_> = warp_maps(&s).into_iter().map(|(p, m)| (p, m.dimensions())).collect();
    assert!(maps.contains(&("warp/slide_0_-1_0.png".to_string(), (2 * (s.width / 8 + 1), 4 * (s.height / 8 + 1)))), "{maps:?}");
    assert_eq!(maps.len(), 3);
    assert!(text.contains("; transition: x\nprint 6\n; transition: x\nprint 4\n; transition: x\nprint 2\nprint 3\n; transition: x\nprint 7\n; transition: x\nprint 5\nあ"), "{text}");
}

#[test]
fn instant_text_sets_the_text_speed() {
    let text = |t: &str| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false };
    let s = script(vec![Op::InstantText(true), text("一"), Op::InstantText(false), text("二")]);
    let out = emit(&s);
    assert!(out.contains("textspeed 0\n一\\\ntextspeed 20\n二\\\n"), "{out}");
}

#[test]
fn menu_sounds_at_startup() {
    let mut s = script(vec![]);
    s.menu_sounds = Some(conv_core::ir::MenuSounds { hover: Some("se/tick.ogg".into()), confirm: Some("se/ok.ogg".into()) });
    let out = emit(&s);
    assert!(out.contains("*start\nmov $118,\"se\\tick.ogg\":selectvoice \"\",$118,\"\"\nbtnse \"se\\ok.ogg\"\n"), "{out}");
}

#[test]
fn title_buttons_hidden_without_saves_and_routes() {
    use conv_core::ir::{TitleButton, Var};
    let mut s = script(vec![]);
    let button = |action| TitleButton { image: "system/b.png".into(), x: 0, y: 0, action, click_se: None, shown_by: None };
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        buttons: vec![button(TitleAction::Continue), button(TitleAction::Start), button(TitleAction::Load), button(TitleAction::Extras)],
        hide_without_saves: true,
        extras_shown_by: vec![Var { global: true, index: 1001 }, Var { global: true, index: 1002 }],
        ..Default::default()
    });
    let text = emit(&s);
    for part in [
        // The save Continue loads: any slot, the newest, the quick save.
        "mov %104,0\nfor %103 = 1 to 100\nif %104 == 0 savefileexist %105,%103:if %105 == 1 mov %104,%103\nnext\nif %901 > 0 savefileexist %105,%901:if %105 == 1 mov %104,%901\nsavefileexist %105,890:if %105 == 1 mov %104,890\n",
        "if %104 > 0 lsp 6,",
        "if %104 > 0 lsp 8,",
        "if %1201 != 0 || %1202 != 0 lsp 9,",
        "if %104 > 0 spbtn 6,1\nspbtn 7,2\nif %104 > 0 spbtn 8,3\nif %1201 != 0 || %1202 != 0 spbtn 9,4\n",
        "if %9 == 1 csp -1:loadgame %104\n",
    ] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
    assert!(!text.contains(",128\n"), "{text}");
}

#[test]
fn title_screen_sounds_pressed_cell_and_continue() {
    use conv_core::ir::TitleButton;
    let mut s = script(vec![Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false }]);
    s.confirm = Some(menu::tests::dialog());
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        fade_ms: 1000,
        buttons: vec![
            TitleButton { image: "system/quick.png".into(), x: 822, y: 150, action: TitleAction::Continue, click_se: Some("sound/se/a.ogg".into()), shown_by: None },
            TitleButton { image: "system/start.png".into(), x: 762, y: 150, action: TitleAction::Start, click_se: Some("sound/se/a.ogg".into()), shown_by: None },
            // Shown only once a flag is set.
            TitleButton { image: "system/exit.png".into(), x: 162, y: 150, action: TitleAction::Exit, click_se: None, shown_by: Some(conv_core::ir::Var { global: false, index: 50 }) },
        ],
        pressed: true,
        hover_se: Some("sound/se/over.ogg".into()),
        bgm: Some("sound/bgm/title.ogg".into()),
        start_bgm_fade_ms: 3000,
        start_wait_ms: 2000,
        exit_without_asking: true,
        glow: Some(conv_core::ir::TitleGlow { image: "system/fire.jpg".into(), keys: vec![(0, 0), (600, 256), (1200, 0)] }),
        openings: vec![conv_core::ir::TitleOpening {
            movie: "movie/op.mp4".into(),
            at_launch: true,
            sound: Some("sound/se/op.ogg".into()),
            glow_keys: Some(vec![(0, 256), (100, 256), (700, 0)]),
            ..Default::default()
        }],
        ..Default::default()
    });
    let text = emit(&s);
    for part in [
        // The glow behind the buttons, redrawn with them.
        "*conv_title_draw\nbgm \"sound\\bgm\\title.ogg\"\nlsp 33,\":c;system\\fire.jpg\",0,0:spmotion 33,1,0,0,0,100,100,0,0,0,600,0,0,100,100,0,256,0,1200,0,0,100,100,0,0,0\nsavefileexist %105,890\nlsp 6,\":a/3,0,3;system\\quick.png\",822,150\nif %105 == 0 lsp 6,\":a/3,0,3;system\\quick.png\",822,150,128\n",
        "if %60 != 0 lsp 8,\":a/3,0,3;system\\exit.png\",162,150\n",
        "*conv_title\nselectvoice \"\",\"sound\\se\\over.ogg\",\"\"\nsavefileexist %105,890\nif %105 == 1 spbtn 6,1,2\nspbtn 7,2,2\nif %60 != 0 spbtn 8,3,2\nbtnwait %9\nselectvoice \"\",$118,\"\"\n",
        "if %9 == 1 dwave 1,\"sound\\se\\a.ogg\":csp -1:loadgame 890\n",
        // No question before quitting.
        "if %9 == 3 end\n",
        "*conv_title_start\ncsp -1\nbgmfadeout 3000:bgmstop:bgmfadeout 0\nbg black,",
        "wait 2000\n",
    ] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
    assert!(!text.contains("conv_title_exit"));
    // The opening plays once a run: cleared by the define section,
    // then the music, its sound, the movie and the title at once.
    for part in [
        "mov %2960,0\ngame\n",
        "if %2960 == 0 mov %2960,1:goto *conv_title_opening0\nmov %2960,1\n*conv_title_draw\n",
        "*conv_title_opening0\nbgm \"sound\\bgm\\title.ogg\"\ndwave 1,\"sound\\se\\op.ogg\"\nlsp 33,\":c;system\\fire.jpg\",0,0,255\n",
        "mpegplay \"movie\\op.mp4\",1,1\nspmotion 33,1,0,0,0,100,100,0,256,0,100,0,0,100,100,0,256,0,700,0,0,100,100,0,0,0\nprint 1\n",
    ] {
        assert!(text.contains(part), "missing {part:?} in\n{text}");
    }
}

#[test]
fn title_openings_by_flags_then_at_random() {
    use conv_core::ir::{TitleOpening, Var};
    let (done, seen) = (Var { global: true, index: 5 }, Var { global: true, index: 6 });
    let opening = |movie: &str, when: Vec<(Var, i32)>, sets: Vec<Var>, random| TitleOpening { movie: movie.into(), at_launch: true, when, sets, random, ..Default::default() };
    let openings = [
        opening("first", vec![(done, 1), (seen, 0)], vec![seen], false),
        opening("route", vec![(done, 1)], vec![], true),
        opening("plain", vec![], vec![], true),
    ];
    let mut out = String::new();
    title_openings(&mut out, &openings, 800);
    assert_eq!(
        out,
        "if %2960 == 0 && %805 == 1 && %806 == 0 mov %2960,1:mov %806,1:goto *conv_title_opening0\n\
         mov %105,0\n\
         if %2960 == 0 && %805 == 1 inc %105\n\
         if %2960 == 0 inc %105\n\
         if %105 > 0 rnd %9,%105\n\
         mov %105,0\n\
         if %2960 == 0 && %805 == 1 && %9 == %105 mov %2960,1:goto *conv_title_opening1\n\
         if %2960 == 0 && %805 == 1 inc %105\n\
         if %2960 == 0 && %9 == %105 mov %2960,1:goto *conv_title_opening2\n\
         if %2960 == 0 inc %105\n"
    );
}

/// The startup notice's lines, which the tests comparing a whole script leave out.
fn notice_of(s: &Script) -> String {
    let mut out = String::new();
    notice::emit(&mut out, s.width, s.height, &s.language);
    out
}

#[test]
fn title_screen_runs_before_the_script() {
    use conv_core::ir::TitleButton;
    let button = |image: &str, action| TitleButton { image: image.into(), x: 600, y: 200, action, ..Default::default() };
    let mut s = script(vec![Op::Text { speaker: None, text: "あ".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false }]);
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        fade_ms: 1000,
        buttons: vec![
            button("system/start.png", TitleAction::Start),
            button("system/option.png", TitleAction::Unavailable),
            button("system/load.png", TitleAction::Load),
            button("system/exit.png", TitleAction::Exit),
        ],
        ..Default::default()
    });
    let out = emit(&s).replace(&notice_of(&s), "");
    let expected = "*start\nbg \"system\\title.png\",0\n*conv_title_draw\n\
        lsp 6,\":a/2,0,3;system\\start.png\",600,200\n\
        lsp 7,\":a;system\\option.png\",600,200\n\
        lsp 8,\":a/2,0,3;system\\load.png\",600,200\n\
        lsp 9,\":a/2,0,3;system\\exit.png\",600,200\n\
        print 2\n*conv_title\nspbtn 6,1\nspbtn 8,2\nspbtn 9,3\nbtnwait %9\n\
        if %9 == 1 goto *conv_title_start\n\
        if %9 == 2 systemcall load:goto *conv_title\n\
        if %9 == 3 end\n\
        goto *conv_title\n*conv_title_start\ncsp -1\nbg black,2\nあ\\\n";
    assert!(out.contains(expected), "{out}");
}

#[test]
fn boot_logos_once_before_the_title_music_kept() {
    let bgm = "sound/bgm/title.ogg";
    let mut s = script(vec![Op::Label("conv_boot".into()), Op::BgmPlay { file: bgm.into(), fade_in_ms: 0 }, Op::Return]);
    s.boot = vec![conv_core::ir::BootStep { gosub: Some("conv_boot".into()), ..Default::default() }];
    s.title_screen = Some(TitleScreen { background: "system/title.png".into(), bgm: Some(bgm.into()), ..Default::default() });
    let out = emit(&s);
    assert!(out.contains("defvar conv_boot_bgm\n"), "{out}");
    assert!(out.contains("defvar conv_booted,0,global\nmov %conv_booted,0\n"), "{out}");
    assert!(out.contains("if %conv_booted == 1 goto *conv_boot_end\nmov %conv_booted,1\neffectskip 0\ngosub *conv_boot:mov %conv_boot_bgm,1\n"), "{out}");
    assert!(out.contains("if %conv_boot_bgm == 0 bgm \"sound\\bgm\\title.ogg\"\nmov %conv_boot_bgm,0\n"), "{out}");
    // Music the boot stops isn't kept.
    s.ops.insert(2, Op::BgmStop { fade_out_ms: 0 });
    let out = emit(&s);
    assert!(out.contains("gosub *conv_boot\n") && !out.contains("conv_boot_bgm"), "{out}");
}

#[test]
fn a_script_opening_plays_once_on_the_music_the_boot_left() {
    let bgm = "sound/bgm/title.ogg";
    let mut s = script(vec![Op::Label("conv_boot".into()), Op::BgmPlay { file: bgm.into(), fade_in_ms: 0 }, Op::Return, Op::Label("conv_open".into()), Op::Return]);
    s.boot = vec![conv_core::ir::BootStep { gosub: Some("conv_boot".into()), ..Default::default() }];
    let opening = conv_core::ir::TitleOpening { at_launch: true, script: Some("conv_open".into()), ..Default::default() };
    s.title_screen = Some(TitleScreen { background: "system/title.png".into(), bgm: Some(bgm.into()), openings: vec![opening], ..Default::default() });
    let out = emit(&s);
    assert!(out.contains("if %2960 == 0 mov %2960,1:goto *conv_title_opening0\n"), "{out}");
    assert!(
        out.contains("*conv_title_opening0\nif %conv_boot_bgm == 0 bgm \"sound\\bgm\\title.ogg\"\nmov %conv_boot_bgm,0\nbg black,0\ngosub *conv_open\nspmotionwait -1\ncsp -1\nbg \"system\\title.png\",0\nprint 1\ngoto *conv_title\n"),
        "{out}"
    );
}

#[test]
fn a_title_motion_goes_on_from_the_script_opening() {
    use conv_core::ir::{MotionKey, TitleButton, TitleMotion, TitleSprite};
    let key = |ms, y| MotionKey { ms, x: 0, y, zoom: 100, angle: 0, opacity: 256, zoom_y: 100, ease: None };
    let mut s = script(vec![Op::Label("conv_open".into()), Op::Return]);
    let opening = conv_core::ir::TitleOpening { script: Some("conv_open".into()), clocks: vec![(TitleSprite::Button(0), 900)], ..Default::default() };
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        buttons: vec![TitleButton { image: "system/start.png".into(), x: 10, y: 20, ..Default::default() }],
        motions: vec![TitleMotion { sprite: TitleSprite::Button(0), keys: vec![key(0, 0), key(100, 50), key(200, 0)], from_key: 0 }],
        openings: vec![opening],
        ..Default::default()
    });
    let out = emit(&s);
    let motion = "spmotion 6,1,0,0,0,100,100,0,256,0,100,0,50,100,100,0,256,0,200,0,0,100,100,0,256,0\n";
    assert!(out.contains(&format!("spmotionwait -1\nspmotionclock 6,962\nfor %105=62 to 999:csp %105:next\nbg \"system\\title.png\",0\nlsp 6,\":a/2,0,3;system\\start.png\",10,20\n{motion}print 1\n")), "{out}");
    // Drawn again (after the settings): from its start.
    assert!(out.contains(&format!("*conv_title_draw\nlsp 6,\":a/2,0,3;system\\start.png\",10,20\n{motion}")), "{out}");
}

#[test]
fn extras_open_from_the_title_option() {
    use conv_core::ir::{CgGallery, CgViewer, ExtrasAction, ExtrasItem, ExtrasMenu, GallerySlot, GalleryTab, Placed, TitleButton};
    let placed = |image: &str| Placed { image: image.into(), x: 10, y: 20, w: 30, h: 40 };
    let mut s = script(vec![]);
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        fade_ms: 1000,
        buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
        ..Default::default()
    });
    let slot = GallerySlot { thumbnail: "system/t.png".into(), cgs: vec![(213, "bg/a.png".into()), (214, "bg/b.png".into())] };
    s.extras = Some(ExtrasMenu {
        background: "system/extras.png".into(),
        items: vec![
            ExtrasItem { button: placed("system/cg.png"), action: ExtrasAction::Gallery },
            ExtrasItem { button: placed("system/exit.png"), action: ExtrasAction::Back },
        ],
        gallery: Some(CgGallery {
            header: placed("h"),
            exit: placed("x"),
            tabs: vec![GalleryTab { button: placed("tab"), selected: "tab_on".into(), title: placed("title"), viewer_name: placed("name"), pages: vec![vec![slot]] }],
            grid_x: 44,
            grid_y: 142,
            grid_dx: 108,
            grid_dy: 92,
            columns: 5,
            locked: "system/locked.png".into(),
            page_label: placed("page"),
            page_numbers: vec!["p1".into()],
            pages_x: 67,
            pages_y: 521,
            page_dx: 24,
            page_marker: "on".into(),
            marker_dx: 0,
            marker_dy: -3,
            viewer: CgViewer { panel: vec![placed("panel")], name_x: 1, name_y: 2, previous: placed("p"), next: placed("n"), hide: placed("c"), back: placed("b") },
        }),
        replay: None,
        music: None,
        movies: None,
        menu_background: None,
    });
    let out = emit(&s);
    assert!(out.contains("*conv_title_back\nbg \"system\\title.png\",0\n"), "{out}");
    assert!(out.contains("if %9 == 1 csp -1:goto *conv_extras\n"), "{out}");
    // The gallery opens once one of its CGs is seen.
    assert!(out.contains("mov %128,0\nif ?conv_cg_seen[213] == 1 mov %128,1\nif ?conv_cg_seen[214] == 1 mov %128,1\n"), "{out}");
    assert!(out.contains("if %128 == 0 cell 100,2\nif %128 == 1 spbtn 100,1\n"), "{out}");
    // Drawn by the engine: the screen, its slots unlocked by the same flags.
    assert!(out.contains("*conv_cg\ncsp -1:mov %121,0:screen \"gallery\"\nreturn\n") && !out.contains("*conv_cg_view"), "{out}");
    let files = screen_files(&s);
    let lua = &files.iter().find(|f| f.0 == "gallery.lua").unwrap().1;
    assert!(lua.contains("add{\"gallery\", page=1, x=44, y=142,") && lua.contains("seen=var(\"?conv_cg_seen[214]\")}"), "{lua}");
}

#[test]
fn scene_replay_plays_unlocked_scenes_to_their_end() {
    use conv_core::ir::{ExtrasAction, ExtrasItem, ExtrasMenu, Placed, ReplayList, ReplayScene, SceneReplay, TitleButton};
    let placed = |image: &str| Placed { image: image.into(), x: 10, y: 20, w: 30, h: 40 };
    let mut s = script(vec![Op::Label("s4_l2".into()), Op::SceneEnd, Op::UnlockScene { scene: 5 }]);
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        fade_ms: 1000,
        buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
        ..Default::default()
    });
    let scene = |scene, title: &str, label: &str, highlight| ReplayScene { scene, title: title.into(), label: label.into(), highlight };
    s.extras = Some(ExtrasMenu {
        background: "system/extras.png".into(),
        items: vec![ExtrasItem { button: placed("system/chapter.png"), action: ExtrasAction::Replay }],
        gallery: None,
        replay: Some(SceneReplay {
            header: placed("h"),
            exit: placed("x"),
            lists: vec![ReplayList {
                button: placed("list"),
                selected: placed("list_on"),
                scenes: vec![scene(5, "工場への道", "s14_l3", false), scene(7, "Ｈ", "s14_l5", true)],
            }],
            list_background: "system/back.png".into(),
            frame: placed("frame"),
            close: placed("close"),
            selector: "system/row.png".into(),
            new_mark: "system/new.png".into(),
            rows: 23,
            columns: 3,
            column_dx: 258,
            row_dy: 21,
            row_y: 58,
            selector_x: 15,
            new_x: 16,
            text_x: 21,
            text_y: 63,
            font_size: 14,
            text_color: "#FEFEFE".into(),
            highlight_color: "#FEC7C7".into(),
            leave_label: "シーン選択に戻る".into(),
            leave_question: "シーン選択に戻りますか？".into(),
        }),
        music: None,
        movies: None,
        menu_background: None,
    });
    let out = emit(&s);
    assert!(out.contains("if %9 == 1 goto *conv_replay\n"), "{out}");
    // A replay stops at the scene's end.
    assert!(out.contains("*s4_l2\nif %152 == 1 goto *conv_replay_end\nmov ?conv_scene_unlocked[5],1\n"), "{out}");
    // The screens give the list, then the title (from 1).
    assert!(out.contains("*conv_replay\ncsp -1:screen \"replay\",%9\nif %9 == 0 csp -1:goto *conv_extras_draw\nmov %153,%9-1\n"), "{out}");
    assert!(out.contains("*conv_replay_list\ncsp -1:screen \"replay_list\",%9\nif %9 == 0 goto *conv_replay\nmov %160,%9-1\n"), "{out}");
    let files = screen_files(&s);
    let menu = &files.iter().find(|f| f.0 == "replay.lua").unwrap().1;
    // A list is open once one of its scenes is unlocked.
    assert!(menu.contains("enabled=(var(\"?conv_scene_unlocked[5]\") == 1 or var(\"?conv_scene_unlocked[7]\") == 1), action={\"close\", 1}"), "{menu}");
    // Unlocked scenes get a row, "new" until replayed.
    let list = &files.iter().find(|f| f.0 == "replay_list.lua").unwrap().1;
    assert!(list.contains("{text=\"Ｈ\", shown=var(\"?conv_scene_unlocked[7]\") == 1, new=var(\"?conv_scene_replayed[7]\") == 0, color=\"#FEC7C7\", action={\"close\", 2}}"), "{list}");
    assert!(out.contains("*conv_replay_go0_1\nmov ?conv_scene_replayed[7],1\ngosub *conv_replay_begin:goto *s14_l5\n"), "{out}");
}

#[test]
fn movie_room_shows_played_movies() {
    use conv_core::ir::{ExtrasAction, ExtrasItem, ExtrasMenu, MovieRoom, Placed, RoomMovie, TitleButton};
    let placed = |image: &str| Placed { image: image.into(), x: 120, y: 71, w: 30, h: 20 };
    let mut s = script(vec![Op::Movie { file: "movie/op01.mp4".into(), skippable: true }]);
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        fade_ms: 1000,
        buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
        ..Default::default()
    });
    let movie = |image: &str, file: &str, open| RoomMovie { button: placed(image), file: file.into(), open };
    s.extras = Some(ExtrasMenu {
        background: "system/extras.png".into(),
        items: vec![ExtrasItem { button: placed("system/mv.png"), action: ExtrasAction::Movies }],
        gallery: None,
        replay: None,
        music: None,
        movies: Some(MovieRoom {
            exit: placed("x"),
            movies: vec![movie("system/m0.png", "movie/op00.mp4", true), movie("system/m1.png", "movie/op01.mp4", false)],
            background: None,
        }),
        menu_background: Some("system/title.png".into()),
    });
    let out = emit(&s);
    assert!(out.contains("if %9 == 1 goto *conv_movies\n"), "{out}");
    // The menu over its own picture, the room over the extras' one.
    assert!(out.contains("*conv_extras_draw\ncsp -1\nbg \"system\\title.png\",0\n"), "{out}");
    // A scene playing a movie of the room marks it.
    assert!(out.contains("mpegplay \"movie\\op01.mp4\",1\nmov ?conv_movie_seen[1],1\n"), "{out}");
    assert!(out.contains("*conv_movie1\nbgmstop:mov %161,0:mpegplay \"movie\\op01.mp4\",1:goto *conv_movies\n"), "{out}");
    let files = screen_files(&s);
    let lua = &files.iter().find(|f| f.0 == "movies.lua").unwrap().1;
    assert!(lua.contains("file=\"system/extras.png\"") && lua.contains("if var(\"?conv_movie_seen[1]\") == 1 then add{\"button\", file=\"system/m1.png\""), "{lua}");
}

#[test]
fn music_room_plays_a_song_and_marks_it() {
    use conv_core::ir::{ExtrasAction, ExtrasItem, ExtrasMenu, MusicRoom, MusicSong, Placed, TitleButton};
    let placed = |image: &str| Placed { image: image.into(), x: 120, y: 71, w: 30, h: 20 };
    let mut s = script(vec![]);
    s.title_screen = Some(TitleScreen {
        background: "system/title.png".into(),
        fade_ms: 1000,
        buttons: vec![TitleButton { image: "system/option.png".into(), x: 600, y: 200, action: TitleAction::Settings, ..Default::default() }],
        ..Default::default()
    });
    let song = |image: &str, file: &str, progress| MusicSong { button: placed(image), playing: format!("{image}_on"), file: file.into(), progress };
    s.extras = Some(ExtrasMenu {
        background: "system/extras.png".into(),
        items: vec![ExtrasItem { button: placed("system/bgm.png"), action: ExtrasAction::Music }],
        gallery: None,
        replay: None,
        music: Some(MusicRoom {
            header: placed("h"),
            exit: placed("x"),
            songs: vec![song("system/s1.png", "bgm/bgm01.ogg", None), song("system/s2.png", "bgm/bgm22.ogg", Some(0))],
            background: Some("system/music.png".into()),
            player: None,
        }),
        movies: None,
        menu_background: None,
    });
    let out = emit(&s);
    assert!(out.contains("if %9 == 1 goto *conv_music\n"), "{out}");
    assert!(out.contains("*conv_music\ncsp -1:screen \"music\"\ngoto *conv_extras_draw\n"), "{out}");
    // A locked song is left out until its progress bit.
    let files = screen_files(&s);
    let lua = &files.iter().find(|f| f.0 == "music.lua").unwrap().1;
    assert!(lua.contains("file=\"system/music.png\"") && lua.contains("if var(\"?conv_progress[0]\") == 1 then add{\"radio\", file=\"system/s2.png\""), "{lua}");
}

#[test]
fn full_screen_pages_continue() {
    let mut nvl = window();
    nvl.name_plate = None;
    nvl.image = "system/textwnd5.png".into();
    let text = |t: &str, continues| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues };
    let s = script(vec![Op::SetWindow(nvl), text("一行目", true), text("二行目", false), text("新頁", false)]);
    let out = emit(&s);
    assert!(out.contains("\":a;system\\textwnd5.png\",47,452\ncsp %conv_name_plate:csp %conv_name_text\n一行目@\n二行目\\\n新頁\\\n"), "{out}");
}

#[test]
fn a_page_goes_on_after_code_inside_it() {
    let text = |t: &str| Op::Text { speaker: None, text: t.into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false };
    let image = Op::WindowImage("system/face2.png".into());
    let s = script(vec![text("《\u{5}"), Op::Wait { ms: 50 }, image, text("興\n\u{5}"), Op::Wait { ms: 50 }, text("》")]);
    let out = emit(&s);
    assert!(out.contains("《/\ndelay 50\nsetwindow2 \":a;system\\face2.png\"\n興\ndelay 50\n》\\\n"), "{out}");
    let mut s = script(vec![text("One\u{5}"), Op::Wait { ms: 50 }, text(" two")]);
    s.single_byte_text = true;
    assert!(emit(&s).contains("`One`/\ndelay 50\n` two\\\n"));
}

#[test]
fn link_positions() {
    let w = window();
    assert_eq!(char_position(&w, "あいう", 2), (105 + 2 * 25, 470));
    assert_eq!(char_position(&w, "あ\nabc", 2), (105 + 25, 507));
}

fn local(index: u16) -> Operand {
    Operand::Var(Var { global: false, index })
}

#[test]
fn emits_a_scene() {
    let s = script(vec![
        Op::Label("scene_1".into()),
        Op::BgmPlay { file: "bgm/track01.ogg".into(), fade_in_ms: 1000 },
        Op::Background { image: "bg/room.png".into(), fade_ms: 500 },
        Op::Sprites {
            changes: vec![SpriteChange {
                layer: 3,
                sprite: Some(SpritePlacement { image: "sprite/a.png".into(), x: 10, y: -20 }),
                keep_filters: true,
            }],
            fade_ms: 500,
        },
        Op::SpriteFilters {
            layer: 3,
            filters: vec![
                SpriteFilter {
                    kind: Operand::Const(FILTER_COLOUR),
                    blend: Operand::Const(FILTER_BLEND_MULTIPLY),
                    strength: Operand::Const(128),
                    params: [255, 200, 100].map(Operand::Const),
                },
                SpriteFilter {
                    kind: Operand::Const(0),
                    blend: Operand::Const(0),
                    strength: Operand::Const(256),
                    params: [Operand::Const(0); 3],
                },
                SpriteFilter {
                    kind: local(30),
                    blend: local(31),
                    strength: local(32),
                    params: [local(33), local(34), local(35)],
                },
            ],
        },
        Op::SpriteOpacity { layer: Some(3), opacity: 128 },
        Op::SpriteOpacity { layer: None, opacity: 256 },
        Op::SpriteMove { layer: 3, x: -400, y: 300, ms: 0 },
        Op::Sprites { changes: vec![SpriteChange { layer: 3, sprite: None, keep_filters: false }], fade_ms: 0 },
        Op::Text { speaker: Some("名前".into()), text: "「こんにちは」".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        Op::Text { speaker: None, text: "地の文。".into(), voice: Some(Voice { file: "voice/v1.ogg".into(), character: 3 }), links: vec![link(1, 2, None)], ruby: vec![], styles: vec![], hang: None, continues: false },
        Op::SePlay { file: "se/k001.ogg".into(), looped: true },
        Op::SeStop { file: Some("se/k001.ogg".into()) },
        Op::BgmStop { fade_out_ms: 2000 },
        Op::Wait { ms: 500 },
        Op::Unsupported { what: "op 0x76".into(), at: SourceLoc { file: "0003.cd".into(), index: 7 } },
    ]);
    let text = emit(&s).replace(&notice_of(&s), "");
    let expected = "\
;mode800,V8192
*define
nsa
globalon
defvar conv_notice_seen,global
defvar conv_name_text
defvar conv_name_plate
spnew %conv_name_text,ui
spnew %conv_name_plate,ui
defvar conv_wait_icon_x
defvar conv_wait_icon_y
defstr conv_choice_text0
defvar conv_choice_x0
defstr conv_choice_text1
defvar conv_choice_x1
defstr conv_choice_text2
defvar conv_choice_x2
defstr conv_choice_text3
defvar conv_choice_x3
defstr conv_choice_text4
defvar conv_choice_x4
defstr conv_choice_text5
defvar conv_choice_x5
defstr conv_log_line
defvar conv_link0
defvar conv_link1
defvar conv_link2
defvar conv_link3
defvar conv_link4
defvar conv_link5
defvar conv_link6
defvar conv_link7
defvar conv_link8
defvar conv_link9
defvar conv_link10
defvar conv_link11
spnew %conv_link0,ui
spnew %conv_link1,ui
spnew %conv_link2,ui
spnew %conv_link3,ui
spnew %conv_link4,ui
spnew %conv_link5,ui
spnew %conv_link6,ui
spnew %conv_link7,ui
spnew %conv_link8,ui
spnew %conv_link9,ui
spnew %conv_link10,ui
spnew %conv_link11,ui
defarr conv_cg_seen,576,global
defarr conv_movie_seen,32,global
defarr conv_scene_unlocked,600,global
defarr conv_progress,32,global
defarr conv_scene_replayed,446,global
loadgosub *conv_load_clear
caption \"Test\"
effect 2,10,500
game
*start
*scene_1
bgmfadein 1000
bgm \"bgm\\track01.ogg\"
csp -1
bg \"bg\\room.png\",2
lsp 65,\":a;sprite\\a.png\",10,-20
print 2
spfilter 65,0
spfilter 65,1,1,128,255,200,100
if %40 != 0 spfilter 65,%40,%41,%42,%43,%44,%45
print 1
msp 65,0,0,-255:msp 65,0,0,127
print 1
if %conv_overlay_slot != 0 msp %conv_overlay_slot,0,0,-255:msp %conv_overlay_slot,0,0,255
print 1
getspsize 65,%1,%2:amsp 65,-400-%1/2,300-%2/2
print 1
csp 65
print 1
【名前】「こんにちは」\\
dwave 0,\"voice\\v1.ogg\"
地#FFE193の文#FFFFFF。\\
dwaveloop 1,\"se\\k001.ogg\"
dwavestop 1
bgmfadeout 2000
bgmstop
delay 500
; unsupported: op 0x76 (0003.cd #7)
end
*conv_load_clear
return
";
    assert_eq!(text, expected);
}

#[test]
fn adv_window_and_name_plate() {
    use conv_core::ir::{NamePlate, TextWindow};
    let mut s = script(vec![
        Op::Text { speaker: Some("名前".into()), text: "「台詞」".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        Op::Text { speaker: None, text: "Ghost".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
    ]);
    s.window = Some(TextWindow {
        wait_icon_at: None,
        image: "system/window.png".into(),
        x: 47,
        y: 452,
        text_x: 105,
        text_y: 470,
        columns: 25,
        rows: 3,
        font_size: 24,
        line_gap: 13,
        vertical: false,
        name_plate: Some(NamePlate {
            image: "system/name.png".into(),
            x: 55,
            y: 412,
            text_x: 75,
            text_y: 422,
            font_size: 20,
        }),
    });
    let text = emit(&s);
    assert!(text.contains("setwindow3 105,470,25,3,24,24,1,13,20,0,1,\":a;system\\window.png\",47,452\nerasetextwindow 0\n"));
    assert!(text.contains("lsp %conv_name_plate,\":a;system\\name.png\",55,412\nstrsp %conv_name_text,\"名前\",75,422,16,1,20,20,1,0,0,1\nprint 1\n「台詞」\\\n"));
    assert!(text.contains("csp %conv_name_plate\ncsp %conv_name_text\nprint 1\n　Ghost \\\n"));
    // With a backlog, lines carry their page's first speaker and voice.
    use conv_core::ir::{Backlog, Placed, Rect};
    let p = Placed { image: "system/x.png".into(), x: 0, y: 0, w: 10, h: 10 };
    s.backlog = Some(Backlog {
        base: p.clone(),
        base_opacity: 200,
        frame: p.clone(),
        track: p.clone(),
        up: p.clone(),
        down: p.clone(),
        close: p,
        area: Rect { x: 30, y: 44, w: 700, h: 516 },
        font_size: 16,
        line_height: 24,
        columns: 41,
    });
    let voice = |n: &str| Some(Voice { file: format!("voice/{n}.ogg"), character: 1 });
    s.ops = vec![
        Op::Text { speaker: None, text: "地の文".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: true },
        Op::Text { speaker: Some("名前".into()), text: "「一」\n「二」".into(), voice: voice("a"), links: vec![], ruby: vec![], styles: vec![], hang: None, continues: true },
        Op::Text { speaker: Some("他".into()), text: "「三」".into(), voice: voice("b"), links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
        Op::Text { speaker: None, text: "地の文".into(), voice: None, links: vec![], ruby: vec![], styles: vec![], hang: None, continues: false },
    ];
    let text = emit(&s);
    assert!(text.contains("pretextgosub *conv_pretext\n"));
    assert!(text.contains("\n地の文@\n"));
    assert!(text.contains("\n[名前|voice/a.ogg]「一」\n[名前|voice/a.ogg]「二」@\n"));
    assert!(text.contains("\n[名前|voice/a.ogg]「三」\\\n"));
    assert!(text.contains("\n地の文\\\n"));
    assert!(text.contains("*conv_pretext\nsaveon\nreturn\n"));
}

#[test]
fn multi_line_text() {
    let s = script(vec![Op::Text {
        speaker: None,
        text: "一行目\nabc\n三行目".into(),
        voice: None,
        links: vec![link(4, 3, None)],
        ruby: vec![],
        styles: vec![],
        hang: None,
        continues: false,
    }]);
    assert!(emit(&s).contains("一行目\n　#FFE193abc #FFFFFF\n三行目\\\n"));
}

#[test]
fn links_are_coloured() {
    assert_eq!(with_markup("あいうえお", &[link(1, 2, None), link(3, 9, None)], &[], &[]), "あ#FFE193いう#FFFFFFえお");
    assert_eq!(with_markup("a#b", &[], &[], &[]), "a＃b");
}

#[test]
fn flow_and_variables() {
    let g = Var { global: true, index: 7 };
    let l = Var { global: false, index: 2 };
    let s = script(vec![
        Op::Arith { var: g, op: ArithOp::Set, value: Operand::Const(1) },
        Op::Arith { var: l, op: ArithOp::Add, value: Operand::Var(g) },
        Op::If { var: g, cmp: Compare::Eq, value: Operand::Const(1), label: "s3_l2".into() },
        Op::Gosub("s2_l18".into()),
        Op::Return,
    ]);
    assert!(emit(&s).contains("mov %207,1\nadd %12,%207\nif %207 == 1 goto *s3_l2\ngosub *s2_l18\nreturn\n"));
}

#[test]
fn bitwise_ops_use_engine_commands() {
    let g = Var { global: true, index: 9 };
    let h = Var { global: true, index: 12 };
    let text = emit(&script(vec![Op::Arith { var: g, op: ArithOp::BitAnd, value: Operand::Var(h) }]));
    assert!(text.contains("bitand %209,%209,%212\n"), "{text}");
}

#[test]
fn sprite_moves_are_engine_motions() {
    let text = emit(&script(vec![Op::SpriteMove { layer: 5, x: 550, y: 300, ms: 300 }]));
    assert!(text.contains(",300,550-%1/2-%3,300-%2/2-%4,100,100,0,256,0:spmotionwait 67\n"), "{text}");
    assert!(!text.contains("spcurve"));
    // An eased one: its curve's table at *start, the motion along it.
    let text = emit(&script(vec![Op::MoveEase { ease: 2, exponent: 3 }, Op::SpriteMove { layer: 5, x: 550, y: 300, ms: 300 }]));
    assert!(text.contains("\n*start\nspcurve 32,0,"), "{text}");
    assert!(text.contains(",256,32:spmotionwait 67\n"), "{text}");
    // One not waited for, then waited for later; the next one waits.
    let move_to = |x| Op::SpriteMove { layer: 5, x, y: 300, ms: 300 };
    let text = emit(&script(vec![Op::MoveNoWait, move_to(550), Op::MotionWait { layer: 5 }, move_to(600)]));
    assert!(text.contains("550-%1/2-%3,300-%2/2-%4,100,100,0,256,0\nspmotionwait 67\n"), "{text}");
    assert!(text.contains("600-%1/2-%3,300-%2/2-%4,100,100,0,256,0:spmotionwait 67\n"), "{text}");
}

/// Runs the helper's algorithm (as emitted) in Rust to check the maths.
#[test]
fn bit_helper_algorithm() {
    fn run(a: i32, b: i32, and: bool) -> i32 {
        let (mut r, mut bit, mut x, mut y) = (0, 1, a, b);
        while !(x == 0 && y == 0) {
            let t = x % 2 + y % 2;
            if (and && t == 2) || (!and && t > 0) {
                r += bit;
            }
            x /= 2;
            y /= 2;
            bit *= 2;
        }
        r
    }
    for (a, b) in [(0, 0), (1, 1), (1, 0), (6, 3), (12, 10), (255, 15), (1 << 20, (1 << 20) + 5)] {
        assert_eq!(run(a, b, true), a & b, "{a} & {b}");
        assert_eq!(run(a, b, false), a | b, "{a} | {b}");
    }
}

#[test]
fn text_is_made_safe() {
    assert_eq!(text_safe("Ghost: 50% @home"), "　Ghost: 50％ ＠home");
    assert_eq!(text_safe("「a/b」"), "「a／b」");
    assert_eq!(single_byte_text("\"Hey! 50% (a/b) #1 @me\\`\"", &[]), "\"Hey! 50% (a/b) #1 ＠me＼｀\"");
    let slant = |start, len, centred: bool| TextStyle { start, len, italic: !centred, centred, ..Default::default() };
    assert_eq!(single_byte_text("Oh!it\nab cd", &[slant(4, 4, false), slant(6, 5, true)]), "Oh！i!i1t!i0\n!c!i1ab!i0 cd");
    assert_eq!(single_byte_text("x\nab", &[slant(2, 2, true)]), "x\n!cab");
    // Tag-like text turns full-width; colours and sizes become tags,
    // ended at the range's or the line's end.
    assert_eq!(single_byte_text("!w5 !sd #00ff00 !wow", &[]), "！w5 ！sd ＃00ff00 !wow");
    let red = TextStyle { start: 0, len: 2, colour: Some(0xCC0000), ..Default::default() };
    let small = TextStyle { start: 3, len: 9, size_percent: Some(75), ..Default::default() };
    assert_eq!(single_byte_text("ab cd\nef", &[red, small]), "#CC0000ab#FFFFFF !z075cd!z100\n!z075ef!z100");
    // Outlines: "!o" before '#' or a digit is a tag; closed at the line's end.
    assert_eq!(single_byte_text("!o# !o1 !ok", &[]), "！o# ！o1 !ok");
    let grey = TextStyle { start: 1, len: 3, outline: Some(0x808080), ..Default::default() };
    assert_eq!(single_byte_text("ab\ncd", &[grey]), "a!o#808080b!o0\n!o#808080c!o0d");
    // Waits: a space ends a number before a digit; one before spaces
    // goes after them (the engine skips spaces after the number).
    let (t, v, e) = (conv_core::ir::TIMED_WAIT_MARK, conv_core::ir::VOICE_WAIT_MARK, conv_core::ir::WAIT_END_MARK);
    assert_eq!(wait_tags(&format!("0{t}300{e}1 a{v}5000{e}  b")), "0!d300 1 a  !v5000b");
    // 最果てのイマ, *conv_link19: the '?' stopped the engine (array).
    assert_eq!(text_safe("嘘ー！?  投稿者: x"), "嘘ー！？  投稿者: x");
    // Every ASCII character the engine's text parser reads specially:
    // variables (% ? $), waits (@ \ _), tags (! # ( / ) < > [ ]), and
    // what starts a token or a comment (` " ;). A ',' is only special
    // at the start of a line, which text_safe guards.
    let safe = text_safe("%?$@\\_!#(/)<>[]`\";*");
    assert!(!safe.contains(|c: char| c.is_ascii()), "{safe}");
}

#[test]
fn single_byte_runs_keep_their_pairs() {
    // A space where a pair would start is skipped by ONScripter.
    assert_eq!(align_single_bytes("ABC続き"), "ABC 続き");
    assert_eq!(align_single_bytes("AB続き"), "AB続き");
    assert_eq!(align_single_bytes(" xyz続"), " xyz 続");
    assert_eq!(align_single_bytes("a b続"), "a b 続");
    assert_eq!(align_single_bytes("ｼｬﾄﾞｳshadow）"), "ｼｬﾄﾞｳshadow ）");
    assert_eq!(align_single_bytes("x#FFE193y#FFFFFF。"), "x #FFE193y #FFFFFF。");
    assert_eq!(align_single_bytes("ab c"), "ab c ");
}

#[test]
fn sprite_texts_are_not_padded() {
    assert_eq!(sprite_text("ＳＡＴＩＥ'Ｓ・ＴＩＭＥ"), "ＳＡＴＩＥ'Ｓ・ＴＩＭＥ");
    assert_eq!(sprite_text("『TCP/IP』"), "『TCP／IP』");
}

#[test]
fn encodes_utf8() {
    let (bytes, lost) = encode("あé\u{1F600}");
    assert_eq!(bytes, "あé\u{1F600}".as_bytes());
    assert_eq!(lost, 1);
    assert_eq!(decode(&bytes), "あé\u{1F600}");
}

#[test]
fn se_channels_rotate_and_stop() {
    let mut ch = SeChannels::default();
    assert_eq!(ch.start("a", false), 1);
    assert_eq!(ch.start("b", false), 2);
    assert_eq!(ch.stop("a"), Some(1));
    assert_eq!(ch.stop("a"), None);
    assert_eq!(ch.stop_all(), vec![2]);
}

#[test]
fn se_channels_keep_loops() {
    let mut ch = SeChannels::default();
    // Nine loops fill 1-9; the next sounds go past them.
    let loops: Vec<u32> = (0..9).map(|i| ch.start(&format!("l{i}"), true)).collect();
    assert_eq!(loops, (1..=9).collect::<Vec<_>>());
    assert_eq!(ch.start("x", false), 11);
    assert_eq!((ch.start("y", true), ch.start("v", false)), (11, 12));
    // A stopped loop frees its channel; one-shots take turns as before.
    assert_eq!(ch.stop("l2"), Some(3));
    assert_eq!((ch.start("z", false), ch.start("w", false)), (3, 3));
}

#[test]
fn movies_stop_the_music() {
    let out = emit(&script(vec![Op::Movie { file: "movie/mv0001.mp4".into(), skippable: true }]));
    assert!(out.contains("bgmstop\nmpegplay \"movie\\mv0001.mp4\",1\n"), "{out}");
}

#[test]
fn japanese_caption() {
    let mut ir = script(Vec::new());
    ir.title = "最果てのイマ \"COMPLETE\"".into();
    assert!(emit(&ir).contains("caption \"最果てのイマ COMPLETE\"\n"));
}

#[test]
fn screen_filters() {
    let dark = SpriteFilter {
        kind: Operand::Const(FILTER_BRIGHTNESS),
        blend: Operand::Const(FILTER_BLEND_COPY),
        strength: Operand::Const(256),
        params: [-200, 0, 0].map(Operand::Const),
    };
    let s = script(vec![
        Op::ScreenFilter { slot: 1, filter: Some(dark), fade_ms: 1000 },
        Op::ScreenFilter { slot: 1, filter: None, fade_ms: 0 },
        Op::ScreenFilter { slot: 0, filter: None, fade_ms: 0 },
    ]);
    let out = emit(&s);
    assert!(out.contains("effect 2,10,1000\n"), "{out}");
    assert!(out.contains("screenfilter 1,4,0,256,-200,0,0\nprint 2\nscreenfilter 1,0\nprint 1\nscreenfilter 0\nprint 1\n"), "{out}");
}

#[test]
fn animations_play_and_outlast_the_background() {
    let a = Animation { image: "fx/fx0401.png".into(), frame_ms: vec![33, 50], looped: false, opacity: 256 };
    let s = script(vec![Op::Animation(Some(a)), Op::Background { image: "bg/room.png".into(), fade_ms: 0 }, Op::Animation(None)]);
    let out = emit(&s);
    assert!(
        out.contains(
            "csp %conv_animation\nmov %conv_animation_alpha,255:mov $conv_animation_image,\":c/2,<33,50>,1;fx\\fx0401.png\"\nlsp %conv_animation,$conv_animation_image,0,0,%conv_animation_alpha\nprint 1\n\
             csp -1\nif %conv_animation_alpha != 0 lsp %conv_animation,$conv_animation_image,0,0,%conv_animation_alpha\nbg \"bg\\room.png\",1\n\
             csp %conv_animation\nmov %conv_animation_alpha,0\nprint 1\n"
        ),
        "{out}"
    );
}

#[test]
fn overlays_outlast_the_background() {
    let s = script(vec![
        Op::Overlay { image: Some("bg/frame01.png".into()), over_sprites: true },
        Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
        Op::Overlay { image: None, over_sprites: true },
    ]);
    let out = emit(&s);
    assert!(
        out.contains(
            "if %conv_overlay_slot != 0 csp %conv_overlay_slot\nmov %conv_overlay_slot,%conv_overlay:mov $conv_overlay_image,\":a;bg\\frame01.png\"\nlsp %conv_overlay,$conv_overlay_image,0,0\nprint 1\n\
             csp -1\nif %conv_overlay_slot != 0 lsp %conv_overlay_slot,$conv_overlay_image,0,0\nbg \"bg\\room.png\",1\n\
             if %conv_overlay_slot != 0 csp %conv_overlay_slot\nmov %conv_overlay_slot,0\nprint 1\n"
        ),
        "{out}"
    );
}

#[test]
fn noise_outlasts_the_background() {
    let noise = Noise { image: "system/noise.png".into(), frames: 4, frame_ms: 100, opacity: 120 };
    let s = script(vec![
        Op::Noise(Some(noise.clone())),
        Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
        Op::Noise(None),
    ]);
    let out = emit(&s);
    assert!(
        out.contains(
            "csp %conv_noise\nmov %conv_noise_alpha,120:mov $conv_noise_image,\":c/4,100,0;system\\noise.png\"\nlsp %conv_noise,$conv_noise_image,0,0,%conv_noise_alpha\nprint 1\n\
             csp -1\nif %conv_noise_alpha != 0 lsp %conv_noise,$conv_noise_image,0,0,%conv_noise_alpha\nbg \"bg\\room.png\",1\n\
             csp %conv_noise\nmov %conv_noise_alpha,0\nprint 1\n"
        ),
        "{out}"
    );
    let out = emit(&script(vec![Op::NoiseFadeIn { ms: 2000 }, Op::Noise(Some(noise))]));
    assert!(out.contains("lsp %conv_noise,$conv_noise_image,0,0,%conv_noise_alpha\nspmotion %conv_noise,0,0,0,0,100,100,0,0,0,2000,0,0,100,100,0,256,0\nprint 1\n"), "{out}");
}

#[test]
fn screen_effects() {
    let shake = ScreenEffect::Shake { delay_ms: 0, time_ms: 1000, interval_ms: 100, path: false, speed: (800, 800), range: [(0, 0), (30, 30)], end_ms: 0 };
    let ripple = ScreenEffect::Ripple {
        map: "system/mask1001.png".into(),
        time_ms: 3000,
        interval_ms: 100,
        opacity: (100, 150),
        amplitude: (5, 10),
        angle: 3150,
        angle_step: 20,
        phase_step: 30,
        multiply: false,
        phase: 10,
    };
    let s = script(vec![
        Op::ScreenEffect { slot: 1, effect: Some(shake) },
        Op::ScreenEffect { slot: 2, effect: Some(ripple) },
        Op::ScreenEffect { slot: 1, effect: None },
        Op::ScreenEffect { slot: 0, effect: None },
        Op::ScreenEffect {
            slot: 3,
            effect: Some(ScreenEffect::Pulse {
                time_ms: 500,
                interval_ms: 10,
                zoom: (150, 150),
                opacity: 150,
                colour: 0xff8080,
                tint_opacity: 256,
                tint: 1,
            }),
        },
    ]);
    let out = emit(&s);
    assert!(
        out.contains(
            "screenshake 1,1000,100,0,800,800,0,0,30,30\nprint 1\n\
             screenripple 2,\"system\\mask1001.png\",3000,100,100,150,5,10,3150,20,30,0,10\nprint 1\n\
             screenshake 1\nprint 1\nscreenshake 0\nprint 1\n\
             screenpulse 3,500,10,150,150,150,16744576,256,1\nprint 1\n"
        ),
        "{out}"
    );
}

#[test]
fn sprite_shakes() {
    let list = ScreenEffect::ShakeList { interval_ms: 50, offsets: vec![(1, -2), (0, 3)], looped: true, delay_ms: 100 };
    let out = emit(&script(vec![Op::SpriteShake { layer: 3, slot: 2, effect: Some(list.clone()) }, Op::SpriteShake { layer: 3, slot: 2, effect: None }, Op::SpriteShake { layer: 4, slot: 1, effect: Some(list) }]));
    // The list once, in the define section.
    assert_eq!(out.matches("1,-2 0,3").count(), 1, "{out}");
    assert!(out.contains("stralias conv_shake0,\"1,-2 0,3\"\n"), "{out}");
    assert!(out.contains("spshake 65,2,0,50,3,conv_shake0,1,0,100\nprint 1\nspshake 65,2\nprint 1\nspshake 66,1,0,50,3,conv_shake0,1,0,100\n"), "{out}");
}

#[test]
fn sprites_in_front_of_the_overlay() {
    let out = emit(&script(vec![Op::SpriteInFront { layer: 7, front: true }, Op::SpriteInFront { layer: 7, front: false }]));
    assert!(out.contains("spz 69,%conv_overlay\nspz 69\n"), "{out}");
}

#[test]
fn screen_flashes() {
    let out = emit(&script(vec![Op::ScreenFlash { colour: 0xff0000, opacity: 128, ms: 100 }]));
    assert!(out.contains("screenflash 128,\"#ff0000\"\nprint 1\ndelay 100\nscreenflash 0\nprint 1\n"), "{out}");
}

#[test]
fn flashes() {
    let out = emit(&script(vec![Op::Flash { colour: 0xff0000, opacity: 128, ms: 100 }]));
    assert!(
        out.contains("lsp2add 24,\":c;>800,600,#ff0000\",400,300,100,100,0,127\nprint 1\ndelay 100\ncsp2 24\nprint 1\n"),
        "{out}"
    );
}

#[test]
fn sound_effects_fade() {
    let out = emit(&script(vec![
        Op::SeFadeIn { ms: 2000 },
        Op::SePlay { file: "se/rain.ogg".into(), looped: true },
        Op::SeFadeOut { file: Some("se/rain.ogg".into()), ms: 1500 },
    ]));
    assert!(out.contains("dwavefadein 2000\ndwaveloop 1,\"se\\rain.ogg\"\nchvolfade 1,0,1500,1\n"), "{out}");
}

#[test]
fn sound_effects_pan() {
    let shot = || "se/shot.ogg".to_string();
    let mut ops = vec![
        Op::SoundPan { file: shot(), pan: -1000, ms: 0 },
        Op::SoundVolume { file: Some(shot()), percent: 50, ms: 0 },
        Op::SePlay { file: shot(), looped: false },
        Op::SoundPan { file: shot(), pan: 1000, ms: 1500 },
    ];
    // Round the channels to the panned one again: centred.
    let span = SE_CHANNELS.end() - SE_CHANNELS.start() + 1;
    ops.extend((0..span).map(|i| Op::SePlay { file: format!("se/{i}.ogg"), looped: false }));
    let out = emit(&script(ops));
    assert!(out.contains("chpan 1,-1000\nchvolfade 1,50\ndwave 1,\"se\\shot.ogg\"\nchpan 1,1000,1500\n"), "{out}");
    assert_eq!(out.matches("chpan 1,0\n").count(), 1, "{out}");
    assert_eq!(out.matches("chpan").count(), 3, "{out}");
}

#[test]
fn some_sounds_are_left_out_while_skipping() {
    let out = emit(&script(vec![
        Op::SeNotSkipped,
        Op::SeFadeIn { ms: 300 },
        Op::SePlay { file: "se/door.ogg".into(), looped: false },
        Op::SePlay { file: "se/rain.ogg".into(), looped: true },
    ]));
    assert!(
        out.contains("isskip %105\nif %105 != 1 dwavefadein 300\nif %105 != 1 dwave 1,\"se\\door.ogg\"\ndwaveloop 2,\"se\\rain.ogg\"\n"),
        "{out}"
    );
}

#[test]
fn auto_wait_replaces_the_next_wait_in_auto_mode() {
    let out = emit(&script(vec![Op::AutoWait { ms: 1500 }, Op::Wait { ms: 2000 }, Op::AutoWait { ms: 1500 }, Op::ClickWait { skippable: true }]));
    assert!(out.contains("isskip %105\nif %105 == 2 delay 1500\nif %105 != 2 delay 2000\n"), "{out}");
    assert!(out.contains("if %105 == 2 delay 1500\nif %105 != 2 mov %9,1:gosub *"), "{out}");
}

#[test]
fn pauses_ignore_clicks() {
    let out = emit(&script(vec![Op::Wait { ms: 300 }, Op::Pause { ms: 500 }]));
    assert!(out.contains("delay 300\nwait 500\n"), "{out}");
}

#[test]
fn fades_to_a_colour_until_the_next_background() {
    let out = emit(&script(vec![
        Op::ScreenFade { colour: 0xffffff, ms: 2000 },
        Op::Background { image: "bg/room.png".into(), fade_ms: 0 },
    ]));
    assert!(out.contains("effect 2,10,2000\n"), "{out}");
    let fade = out.find("lsp2 26,\":c;>800,600,#ffffff\",400,300,100,100,0,255\nprint 2\n").expect(&out);
    let clear = out.find("csp -1:csp2 26\n").expect(&out);
    assert!(fade < clear, "{out}");
}

#[test]
fn hanging_indent() {
    // "　Ghost 　": the space before ASCII, five letters and a pad.
    assert_eq!(hang_indent("Ghost　忍、いるか", 6), 5);
    assert_eq!(hang_indent("θ　すべて", 2), 2);
    let mut s = script(vec![Op::Text {
        speaker: None,
        text: "NIN-NIN　長い行".into(),
        voice: None,
        links: vec![],
        ruby: vec![],
        styles: vec![],
        hang: Some(8),
        continues: true,
    }]);
    s.window = Some(window());
    assert!(emit(&s).contains("indent 6\n　NIN-NIN 　長い行@\nindent 0\n"), "{}", emit(&s));
}

#[test]
fn style_colours() {
    let st = |start, len, colour| TextStyle { start, len, colour, ..Default::default() };
    // Each line returns to the text colour; links win over styles.
    assert_eq!(
        with_markup("あいう\nえお", &[], &[], &[st(1, 4, Some(0xffff00))]),
        "あ#FFFF00いう#FFFFFF\n#FFFF00え#FFFFFFお"
    );
    assert_eq!(
        with_markup("あいうえ", &[link(2, 1, None)], &[], &[st(0, 4, Some(0x00ff00)), st(3, 1, Some(0x0000ff))]),
        "#00FF00あい#FFE193う#0000FFえ#FFFFFF"
    );
    assert_eq!(with_markup("あい", &[], &[], &[st(0, 2, None)]), "あい");
    let sized = |start, len, p| TextStyle { start, len, size_percent: Some(p), ..Default::default() };
    assert_eq!(
        with_markup("あい\nう", &[], &[], &[sized(0, 4, 150), TextStyle { colour: Some(0xffff00), ..sized(3, 1, 75) }]),
        "!z150あい!z100\n#FFFF00!z075う!z100#FFFFFF"
    );
    assert_eq!(align_single_bytes("Ghost!z100の"), "Ghost !z100の");
    assert_eq!(align_single_bytes("ab!i1cd!i0の!c"), "ab!i1cd!i0の!c");
}

#[test]
fn ruby_markup() {
    let r = |start, len, text: &str| Ruby { start, len, text: text.into() };
    // After escaping: the text's own parentheses stay full-width.
    assert_eq!(with_markup("(戦ぐ)", &[], &[r(1, 1, "そよ")], &[]), "（(戦/そよ)ぐ）");
    // Inside a link, but not across its edge or a line.
    assert_eq!(
        with_markup("あ獄い\nう", &[link(0, 3, None)], &[r(1, 1, "ひとや"), r(2, 3, "x")], &[]),
        "#FFE193あ(獄/ひとや)い#FFFFFF\nう"
    );
    assert_eq!(with_markup("あいう", &[link(1, 2, None)], &[r(0, 2, "x")], &[]), "あ#FFE193いう#FFFFFF");
    // ONScripter reads the markup one byte at a time.
    assert_eq!(align_single_bytes("a(獄/ひとや)"), "a (獄/ひとや)");
}

#[test]
fn rubyon_lifts_the_text() {
    let mut s = script(vec![Op::Text {
        speaker: None,
        text: "獄".into(),
        voice: None,
        links: vec![],
        ruby: vec![Ruby { start: 0, len: 1, text: "ひとや".into() }],
        styles: vec![],
        hang: None,
        continues: false,
    }]);
    s.window = Some(window());
    let out = emit(&s);
    assert!(out.contains("rubyon\n"));
    assert!(out.contains("setwindow3 105,457,25,3,24,24,1,13,"), "{out}");
    assert!(out.contains("(獄/ひとや)\\\n"), "{out}");
}

#[test]
fn source_map_lines() {
    let at = |index| SourceLoc { file: "1.cd".into(), index };
    let ops = vec![Op::Label("a".into()), Op::Wait { ms: 5 }, Op::Wait { ms: 7 }];
    // instruction 0 lowered to nothing: instruction 1's line wins
    let script = Script { ops, source_map: vec![(0, at(0)), (0, at(1)), (2, at(2))], ..Default::default() };
    let Emitted { script: out, lines: map, .. } = emit_mapped(&script, strtab::Strings::default());
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(map.len(), 3, "{map:?}");
    assert_eq!((lines[map[0].0 - 1], &map[0].1), ("*a", &Some(at(1))));
    assert_eq!((lines[map[1].0 - 1], &map[1].1), ("delay 7", &Some(at(2))));
    assert_eq!((lines[map[2].0 - 1], &map[2].1), ("end", &None));
}
