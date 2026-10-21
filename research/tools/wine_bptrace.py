#!/usr/bin/env python3
r"""Breakpoint trace of an original game under wine, from a names.tsv.

  wine_bptrace.py <names.tsv> [name ...] [--args N] -o bp.gdb
  wine_bptrace.py --fold bp.log          # one line per run of hits of the same function
  # terminal 1 (scratch prefix, own Xvfb, LANG=ja_JP.UTF-8, see CLAUDE.md);
  # winedbg wants the Windows path, and prints the port on stderr:
  winedbg --gdb --no-start 'C:\Game\game.exe'   # "target remote localhost:PORT"
  # terminal 2 (connect first: the file ends with `continue`):
  gdb -batch -ex 'target remote localhost:PORT' -x bp.gdb > bp.log 2>&1

Each listed name (default: every row) gets `break *0x<addr>`; on a hit gdb
prints one JSON line {"fn","esp","a0".."aN"} (the first N stack words above
the return address: the cdecl/stdcall arguments of 32-bit x86; --args
default 4) after a line with the time (`shell date +%s.%N`), then continues. Names
are matched whole. Run `--self-test` for the check.
--fold prints, per run of consecutive hits of one function, the first and
last time (seconds from the first hit) and the count: a per-frame callback
becomes one line whose count is the frames it ran. Tried on 最果てのイマ
(wine 11, 32-bit exe): docs/new-game-reference.md, "Breakpoint trace".
"""
import json
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


def fold(lines):
    """Folds a gdb log into [fn, first, last, count, record] runs."""
    t, out = None, []
    for ln in lines:
        ln = ln.strip()
        if ln.replace(".", "", 1).isdigit():
            t = float(ln)
        elif ln.startswith('{"fn"'):
            r = json.loads(ln)
            if out and out[-1][0] == r["fn"]:
                out[-1][2] = t
                out[-1][3] += 1
            else:
                out.append([r["fn"], t, t, 1, r])
    return out


if __name__ == "__main__":
    if "--fold" in sys.argv:
        runs = fold(open(sys.argv[sys.argv.index("--fold") + 1], errors="replace"))
        t0 = runs[0][1] if runs else 0
        for fn, a, b, n, r in runs:
            args = " ".join(f"{k}={v}" for k, v in r.items() if k.startswith("a"))
            print(f"{fn:20s} {a - t0:9.3f} {b - t0:9.3f} n={n} {args}")
        sys.exit()
    if "--self-test" in sys.argv:
        t = "# c\n00401000\tfoo\tcomment\n00402000\tbar\n"
        s, n = gdb_script(t)
        assert n == 2 and "break *0x00401000" in s and "break *0x00402000" in s
        s, n = gdb_script(t, ["bar"], 2)
        assert n == 1 and "foo" not in s and "$esp+8" in s and "$esp+12" not in s
        log = ['Breakpoint 1 at 0x401000', '10.5', '{"fn":"foo","esp":1,"a0":2}',
               '0x7bcf0636 in ?? ()', '10.6', '{"fn":"cb","esp":1,"a0":2}',
               '10.7', '{"fn":"cb","esp":1,"a0":2}', '10.9', '{"fn":"foo","esp":1,"a0":3}']
        assert [(f, a, b, k) for f, a, b, k, _ in fold(log)] == [
            ("foo", 10.5, 10.5, 1), ("cb", 10.6, 10.7, 2), ("foo", 10.9, 10.9, 1)]
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
