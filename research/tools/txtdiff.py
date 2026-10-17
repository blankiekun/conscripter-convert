#!/usr/bin/env python3
"""Compare two generated 0.txt files (UTF-8, or Shift-JIS from older builds): what changed, and is it only a rename?

usage: txtdiff.py OLD NEW [--map 's/OLD/NEW/'] [--rules FILE] [--hunks N] [--top N]
       txtdiff.py --self-test

Prints each changed hunk under the *label it is in (the first --hunks,
default 10), then the changed lines counted by command with numbers
masked (`lsp N,"…",N,N`): "-" lines only in OLD, "+" only in NEW.
The build/version line is ignored.

--map rules (sed-like s/REGEX/REPLACEMENT/, Python re syntax, any
delimiter; repeatable) are applied to OLD's lines before comparing,
so a pure rename leaves no difference. --rules FILE reads one rule
per line (# comments). Exit status: 0 the same, 1 different.
"""
import argparse
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter

# Same normalisation as tests/snapshot.sh: the version line changes with every build.
VERSION = [
    (re.compile(r'"build \d{4}-\d{2}-\d{2}, converter [^"]*",(\d+,\d+),\d+,'), r'"build VERSION",\1,W,'),
    (re.compile(r"build \d{4}-\d{2}-\d{2}, converter [0-9a-f]+(-dirty)?, CONscripter [^\"]*"), "build VERSION"),
]
MASK = [(re.compile(r'"[^"]*"'), '"…"'), (re.compile(r"(?<![\w$%])-?\d+"), "N")]


def read(path):
    # split on \n only: cp932 text holds bytes that splitlines() would also break on
    data = open(path, "rb").read()
    try:
        lines = data.decode("utf-8").split("\n")
    except UnicodeDecodeError:  # a build from before 2026-10-17
        lines = data.decode("cp932", "replace").split("\n")
    return [norm(l) for l in lines]


def norm(line):
    line = line.rstrip("\r")
    for pat, rep in VERSION:
        line = pat.sub(rep, line)
    return line


def mask(line):
    line = line.strip()
    for pat, rep in MASK:
        line = pat.sub(rep, line)
    return line


def parse_rule(rule):
    if len(rule) < 4 or rule[0] != "s":
        raise ValueError(f"not an s/old/new/ rule: {rule!r}")
    d = rule[1]
    parts = re.split(r"(?<!\\)" + re.escape(d), rule[2:])
    if len(parts) != 3 or parts[2] not in ("", "g"):
        raise ValueError(f"bad rule {rule!r}: use s{d}old{d}new{d}")
    return re.compile(parts[0].replace("\\" + d, d)), parts[1].replace("\\" + d, d)


def load_rules(maps, files):
    rules = [parse_rule(m) for m in maps]
    for f in files:
        for line in open(f, encoding="utf-8"):
            line = line.strip()
            if line and not line.startswith("#"):
                rules.append(parse_rule(line))
    return rules


def apply(rules, lines):
    for pat, rep in rules:
        lines = [pat.sub(rep, l) for l in lines]
    return lines


def labels(lines):
    """The label each line is in."""
    cur, out = "(top)", []
    for l in lines:
        if l.startswith("*"):
            cur = l.split()[0]
        out.append(cur)
    return out


def compare(old, new):
    """Opcodes (tag, i1, i2, j1, j2) of the differing blocks, from GNU diff:
    difflib takes minutes on half a million lines, or misaligns them with autojunk."""
    with tempfile.TemporaryDirectory() as d:
        paths = [os.path.join(d, n) for n in ("old", "new")]
        for path, lines in zip(paths, (old, new)):
            with open(path, "w", encoding="utf-8", newline="\n") as f:
                f.write("\n".join(lines) + "\n")
        r = subprocess.run(["diff", *paths], capture_output=True, text=True, encoding="utf-8")
    if r.returncode > 1:
        sys.exit(f"diff failed: {r.stderr}")
    ops = []
    for m in re.finditer(r"^(\d+)(?:,(\d+))?([acd])(\d+)(?:,(\d+))?$", r.stdout, re.M):
        l1, l2, t, r1, r2 = int(m[1]), int(m[2] or m[1]), m[3], int(m[4]), int(m[5] or m[4])
        i1, i2 = (l1, l1) if t == "a" else (l1 - 1, l2)
        j1, j2 = (r1, r1) if t == "d" else (r1 - 1, r2)
        ops.append(({"a": "insert", "d": "delete", "c": "replace"}[t], i1, i2, j1, j2))
    return ops


def report(old, new, ops, hunks, top, out=sys.stdout):
    lab = labels(new)
    for tag, i1, i2, j1, j2 in ops[:hunks]:
        where = lab[j1] if j1 < len(lab) else lab[-1]
        print(f"@@ {where}  old {i1 + 1}, new {j1 + 1}", file=out)
        for l in old[i1:i2][:8]:
            print(f"- {l[:200]}", file=out)
        for l in new[j1:j2][:8]:
            print(f"+ {l[:200]}", file=out)
        if i2 - i1 > 8 or j2 - j1 > 8:
            print(f"  ... {i2 - i1} old, {j2 - j1} new lines in this hunk", file=out)
    if len(ops) > hunks:
        print(f"... {len(ops) - hunks} more hunks", file=out)
    count = Counter()
    for _, i1, i2, j1, j2 in ops:
        count.update(("-", mask(l)) for l in old[i1:i2])
        count.update(("+", mask(l)) for l in new[j1:j2])
    print(f"changed lines by command ({sum(count.values())} lines, {len(ops)} hunks):", file=out)
    for (sign, cmd), n in count.most_common(top):
        print(f"{n:7} {sign} {cmd[:160]}", file=out)
    return count


def self_test():
    assert mask('lsp 10,":a;x.png",-20,300') == 'lsp N,"…",N,N'
    assert mask("mov %12,$3:spmotion2 5") == "mov %12,$3:spmotion2 N"
    rules = [parse_rule("s/spfade (\\d+)/spalpha \\1/"), parse_rule("s|a/b|c|")]
    assert apply(rules, ["spfade 3,200", "x a/b"]) == ["spalpha 3,200", "x c"]
    old = ["*start", '"build 2026-10-01, converter abc, CONscripter 0.18"', "spfade 3,200", "end"]
    new = ["*start", '"build 2026-10-16, converter def-dirty, CONscripter 0.19"', "spalpha 3,200", "end"]
    old, new = [norm(l) for l in old], [norm(l) for l in new]
    assert compare(old, new) == [("replace", 2, 3, 2, 3)]
    assert compare(apply(rules, old), new) == []
    ops = compare(apply(rules, old), new + ["lsp 1,\"x\",0,0"])
    assert ops == [("insert", 4, 4, 4, 5)]
    assert compare(["a", "b", "c"], ["a", "c", "d"]) == [("delete", 1, 2, 1, 1), ("insert", 3, 3, 2, 3)]
    print("self-test ok")


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("old")
    ap.add_argument("new")
    ap.add_argument("--map", action="append", default=[], help="s/OLD/NEW/ applied to OLD's lines")
    ap.add_argument("--rules", action="append", default=[], help="file of s/OLD/NEW/ rules")
    ap.add_argument("--hunks", type=int, default=10, help="hunks to print (default 10)")
    ap.add_argument("--top", type=int, default=20, help="command kinds to count (default 20)")
    a = ap.parse_args()
    try:
        rules = load_rules(a.map, a.rules)
    except (OSError, ValueError, re.error) as e:
        ap.error(str(e))
    old, new = read(a.old), read(a.new)
    if rules:
        old = apply(rules, old)
    ops = compare(old, new)
    if not ops:
        print("same" + (f" after {len(rules)} rules: only a rename" if rules else ""))
        return 0
    report(old, new, ops, a.hunks, a.top)
    if rules:
        print("NOT only a rename: the lines above differ after the rules")
    return 1


if __name__ == "__main__":
    sys.exit(main())
