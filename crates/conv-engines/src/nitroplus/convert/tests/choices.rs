use super::*;

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
