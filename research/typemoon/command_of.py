#!/usr/bin/env python3
"""Which script command reaches each function.

    command_of.py <ghidra decomp dir> FUN_... > out.tsv

script_command's call sites get the command above them (the last
`case` letter and three-letter compare in source order, as in
field_writers.py); a function's commands are those of every
script_command call site it is reached from through the `called by`
headers (up to 6 levels). Prints function, commands, other roots.
"""
import os
import re
import sys

d = sys.argv[1]
callers, files = {}, {}
for f in os.listdir(d):
    if not f.endswith(".c"):
        continue
    name = f[:-2].split("_", 1)[1]
    files[name] = f
    with open(os.path.join(d, f), encoding="utf-8", errors="replace") as h:
        for line in h:
            if line.startswith("// called by:"):
                callers[name] = re.findall(r"[0-9a-f]{8} (\S+?)(?:,|$)", line[13:].strip())
                break

site = {}
letter, cmd = "", ""
for line in open(os.path.join(d, files["script_command"]), encoding="utf-8", errors="replace"):
    m = re.search(r"case (0x[0-9a-f]{2}):", line)
    if m:
        letter, cmd = chr(int(m.group(1), 16)), ""
    m = re.search(r"(?:== |case )(0x[0-9a-f]{6})\b", line)
    if m:
        cmd = bytes.fromhex(m.group(1)[2:]).decode("latin-1")
    for fn in re.findall(r"\b((?:FUN|cmd)_\w+)\(", line):
        site.setdefault(fn, set()).add(letter + cmd)


def roots(fn, depth=0, seen=None):
    seen = seen if seen is not None else set()
    cmds, other = set(), set()
    if fn in seen:
        return cmds, other
    seen.add(fn)
    cmds |= site.get(fn, set())
    ups = [c for c in callers.get(fn, []) if c != "script_command"]
    if not callers.get(fn):
        other.add(fn)
    if depth >= 6:
        other |= set(ups)
        return cmds, other
    for c in ups:
        a, b = roots(c, depth + 1, seen)
        cmds |= a
        other |= b
    return cmds, other


for fn in sys.argv[2:]:
    c, o = roots(fn)
    print(f"{fn}\t{','.join(sorted(c))}\t{','.join(sorted(o))}")
