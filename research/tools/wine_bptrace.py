#!/usr/bin/env python3
"""Breakpoint trace of an original game under wine, from a names.tsv.

  wine_bptrace.py <names.tsv> [name ...] [--args N] -o bp.gdb
  # terminal 1 (scratch prefix, own Xvfb, LANG=ja_JP.UTF-8, see CLAUDE.md):
  winedbg --gdb --no-start game.exe      # prints "target remote localhost:PORT"
  # terminal 2:
  gdb -batch -x bp.gdb -ex 'target remote localhost:PORT' > bp.log 2>&1
  # then keep it small: grep -E '^[{0-9]' bp.log > bp.jsonl (time line, then record); query with grep/jq.

Each listed name (default: every row) gets `break *0x<addr>`; on a hit gdb
prints one JSON line {"fn","esp","a0".."aN"} (the first N stack words above
the return address: the cdecl/stdcall arguments of 32-bit x86; --args
default 4) after a line with the time (`shell date +%s.%N`), then continues. Names
are matched whole. Run `--self-test` for the check.
Untested against a game: see docs/new-game.md step 19.
"""
import sys


def rows(text):
    for ln in text.splitlines():
        if ln and not ln.startswith("#"):
            f = ln.split("\t")
            if len(f) >= 2:
                yield f[0].strip(), f[1].strip()


def gdb_script(text, names=(), nargs=4):
    sel = [(a, n) for a, n in rows(text) if not names or n in names]
    out = ["set pagination off", "set confirm off"]
    for i, (a, n) in enumerate(sel, 1):
        words = ",".join(f'\\"a{k}\\":%u' for k in range(nargs))
        vals = ",".join(f"*(unsigned*)($esp+{4 * (k + 1)})" for k in range(nargs))
        out += [f"break *0x{a}", "commands",
                "silent",
                'shell date +%s.%N',
                f'printf "{{\\"fn\\":\\"{n}\\",\\"esp\\":%u,{words}}}\\n", $esp,{vals}',
                "continue", "end"]
    out.append("continue")
    return "\n".join(out) + "\n", len(sel)


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        t = "# c\n00401000\tfoo\tcomment\n00402000\tbar\n"
        s, n = gdb_script(t)
        assert n == 2 and "break *0x00401000" in s and "break *0x00402000" in s
        s, n = gdb_script(t, ["bar"], 2)
        assert n == 1 and "foo" not in s and "$esp+8" in s and "$esp+12" not in s
        print("ok")
        sys.exit()
    a = sys.argv[1:]
    nargs, outp = 4, None
    if "--args" in a:
        i = a.index("--args"); nargs = int(a[i + 1]); del a[i:i + 2]
    if "-o" in a:
        i = a.index("-o"); outp = a[i + 1]; del a[i:i + 2]
    assert len(a) >= 1 and outp, __doc__
    s, n = gdb_script(open(a[0], encoding="utf-8").read(), a[1:], nargs)
    open(outp, "w").write(s)
    print(f"{n} breakpoints -> {outp}")
