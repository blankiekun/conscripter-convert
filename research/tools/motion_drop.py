#!/usr/bin/env python3
"""List the source places whose emitted motion time went down between two dry runs.

usage: motion_drop.py BEFORE/0.txt BEFORE.map AFTER/0.txt AFTER.map [--top N]
       motion_drop.py --self-test

Motion time of a 0.txt line: each `spmotion`'s last key time (its
length) plus each `print N,ms`'s ms. A line counts for the source place
(`--source-map`: `<line>\\t<file>:<index>`) whose entry it falls under;
lines under a `-` entry count for the last place before it (Django
Next 2: `spmotion` lines after a mapped line carry no entry of their
own). A place in a library file is named with its call site
(read_map). Prints one line per place whose total went down, largest drop
first: a lost motion (Django's `FadeFade`: 16 s of pan gone) shows here,
where a line count doesn't. tests/pr-report.sh asks a "why" for each.
"""
import bisect
import re
import sys
from collections import Counter


def file_of(loc):
    return loc.rsplit(":", 1)[0]


def read_map(src):
    """(first 0.txt line of each entry, its place). A place in a library file
    (a script whose lines are mostly emitted more than 3 times over: the
    helpers a compiler inlines per call, Django's function.nss) is named with
    the place it follows, the last one before it in another file (the call
    itself emits no line, so this is the call's nearest emitting line above)."""
    rows = []
    for row in src:
        n, loc = row.rstrip("\n").split("\t")
        rows.append((int(n), loc if loc != "-" else (rows[-1][1] if rows else None)))
    runs, prev = Counter(), None
    for _, loc in rows:
        if loc and loc != prev:
            runs[loc] += 1
        prev = loc
    files = Counter(file_of(k) for k in runs)
    lib = {f for f, n in Counter(file_of(k) for k, c in runs.items() if c > 3).items() if n * 2 > files[f]}
    lines, keys, site = [], [], None
    for n, loc in rows:
        if loc and file_of(loc) not in lib:
            site = loc
        lines.append(n)
        keys.append(f"{loc} (after {site})" if loc and file_of(loc) in lib and site else loc)
    return lines, keys


def motion_ms(cmd):
    """The motion time one command emits (0 for any other)."""
    name, _, args = cmd.strip().partition(" ")
    a = [x.strip() for x in args.split(",")]
    if name == "print" and len(a) >= 2 and a[1].isdigit():
        return int(a[1])
    # spmotion num,loop,(t,x,y,zx,zy,angle,alpha,curve)*: its length is the last key's t
    if name == "spmotion" and len(a) >= 10 and (len(a) - 2) % 8 == 0 and a[-8].isdigit():
        return int(a[-8])
    return 0


def per_place(txt, mp):
    lines, keys = read_map(open(mp, encoding="utf-8"))
    out = Counter()
    for n, line in enumerate(open(txt, encoding="utf-8", errors="replace"), 1):
        if "spmotion" not in line and "print" not in line:
            continue
        # shortcut: splits on every ':', also one inside a string; only print/spmotion are read
        ms = sum(motion_ms(c) for c in line.split(":"))
        i = bisect.bisect_right(lines, n) - 1
        if ms and i >= 0 and keys[i]:
            out[keys[i]] += ms
    return out


def drops(before, after):
    return sorted(((k, ms, after.get(k, 0)) for k, ms in before.items() if after.get(k, 0) < ms),
                  key=lambda d: d[2] - d[1])


def self_test():
    assert motion_ms("print 10,400") == 400 and motion_ms("print 1") == 0
    assert motion_ms("spmotion 6,1,0,0,0,100,100,0,256,0,5000,0,0,100,100,0,256,0") == 5000
    assert motion_ms("spmotion 6,1,%t,0,0,100,100,0,256,0") == 0 and motion_ms("csp 6") == 0
    b, a = Counter({"x:1": 16000, "x:2": 300, "x:3": 5}), Counter({"x:1": 1000, "x:2": 300, "x:4": 9})
    assert drops(b, a) == [("x:1", 16000, 1000), ("x:3", 5, 0)], drops(b, a)
    # lib.nss:5 is emitted 4 times over: a library line, named after the scene line before it
    m = ["1\ts:1", "2\tlib:5", "3\t-", "4\ts:2", "5\tlib:5", "6\ts:3", "7\tlib:5", "8\ts:4", "9\tlib:5"]
    assert read_map(m)[1] == ["s:1", "lib:5 (after s:1)", "lib:5 (after s:1)", "s:2", "lib:5 (after s:2)",
                              "s:3", "lib:5 (after s:3)", "s:4", "lib:5 (after s:4)"], read_map(m)[1]


def main():
    self_test()  # cheap: every run checks the checker
    if sys.argv[1:] == ["--self-test"]:
        return print("self-test ok")
    top = 10
    if "--top" in sys.argv:
        i = sys.argv.index("--top")
        top = int(sys.argv[i + 1])
        del sys.argv[i:i + 2]
    if len(sys.argv) != 5:
        sys.exit(__doc__.split("\n\n")[1])
    found = drops(per_place(*sys.argv[1:3]), per_place(*sys.argv[3:5]))
    for k, b, a in found[:top]:
        print(f"{k}: {b} ms of motion -> {a}")
    if len(found) > top:
        print(f"... {len(found) - top} more places")


if __name__ == "__main__":
    main()
