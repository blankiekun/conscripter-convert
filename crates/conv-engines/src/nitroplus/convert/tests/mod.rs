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
        include_bytes!("../../../../../conv-cli/fonts/ipag.ttf")
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

mod calls;
mod effects;
mod motions;
mod sync;
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
