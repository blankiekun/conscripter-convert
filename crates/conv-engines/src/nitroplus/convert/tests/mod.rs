use super::*;
use super::probe::Placeholder;
use crate::nitroplus::nss;

/// The placeholder assets, with the font that draws `CreateText`.
fn fake() -> Placeholder {
    Placeholder { font: include_bytes!("../../../../../conv-cli/fonts/ipag.ttf") }
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
    compile_scene(&nss_files(library, &[("a.nss", story)]), "a.nss", "a.nss", &Game::PLAIN, &mut fake()).unwrap()
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

mod calls;
mod effects;
mod motions;
mod sync;
mod scheduler;
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

mod blends;
mod processes;
mod layers;
mod one_shots;
mod sounds;
mod choices;
mod route;
mod covers;
mod fades;
