//! `system.lua`, the Lua CONscripter runs at startup: the commands the
//! script starts and stops processes with ([`Op::StartProcess`]), then the
//! game's own Lua ([`Script::lua`]).

use super::*;

/// The commands `0.txt` declares with `luasub` when it uses processes.
pub(crate) const COMMANDS: [&str; 2] = ["cons_start", "cons_stop"];

/// `cons_start "name","function",args...` and `cons_stop "name"`.
const PRELUDE: &str = r#"function NSCOM_cons_start()
  local name = NSPopStr()
  NSPopComma()
  local fn = NSPopStr()
  local args = {}
  while NSCheckComma() do
    NSPopComma()
    args[#args + 1] = NSPopInt()
  end
  cons_process(name, fn, (table.unpack or unpack)(args))
end
function NSCOM_cons_stop()
  cons_process_stop(NSPopStr())
end
"#;

/// The script starts or stops processes.
pub(crate) fn uses_processes(script: &Script) -> bool {
    script.ops.iter().any(|op| matches!(op, Op::StartProcess { .. } | Op::StopProcess { .. }))
}

/// `system.lua`'s text (UTF-8; [`encode`] it like `0.txt`), `None` when
/// the script needs no Lua.
pub fn system_lua(script: &Script) -> Option<String> {
    let processes = uses_processes(script);
    if !processes && script.lua.is_empty() {
        return None;
    }
    let mut out = String::new();
    if processes {
        out.push_str(PRELUDE);
    }
    out.push_str(&script.lua);
    Some(out)
}

/// A string argument of `cons_start`/`cons_stop` (no `"` inside).
pub(crate) fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processes_are_started_through_system_lua() {
        let var = Var { global: false, index: 3 };
        let ops = vec![
            Op::StartProcess { name: "fire".into(), function: "flicker".into(), args: vec![Operand::Const(5), Operand::Var(var)] },
            Op::StopProcess { name: "fire".into() },
        ];
        let script = Script { ops, lua: "function flicker(n, m) cons_frame() end\n".into(), ..Default::default() };
        let text = emit(&script);
        assert!(text.contains("luasub cons_start\nluasub cons_stop\n"));
        assert!(text.contains(&format!("cons_start \"fire\",\"flicker\",5,%{}\ncons_stop \"fire\"\n", LOCAL_VAR_BASE + 3)));
        let lua = system_lua(&script).unwrap();
        assert!(lua.starts_with("function NSCOM_cons_start()") && lua.ends_with("function flicker(n, m) cons_frame() end\n"));
        assert_eq!(system_lua(&Script::default()), None);
        assert!(!emit(&Script::default()).contains("luasub"));
    }
}
