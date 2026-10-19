use super::*;

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
    let (c, _) = compile_run(&files, "a.nss", 1, &MURAMASA, &mut fake()).unwrap();
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
    let (c, _) = compile_run(&files, "a.nss", 1, &MURAMASA, &mut fake()).unwrap();
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
