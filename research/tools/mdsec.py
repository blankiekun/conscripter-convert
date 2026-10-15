#!/usr/bin/env python3
"""Print markdown sections instead of reading the whole file.

usage: mdsec.py FILE            list the headings, with line numbers
       mdsec.py FILE TEXT       print every section whose heading contains
                                TEXT (case-insensitive), down to the next
                                heading of the same or a higher level
"""
import sys

if len(sys.argv) not in (2, 3):
    sys.exit(__doc__)
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
fence = False
heads = []  # (line index, level, text)
for i, l in enumerate(lines):
    if l.startswith("```"):
        fence = not fence
    elif not fence and l.startswith("#"):
        level = len(l) - len(l.lstrip("#"))
        heads.append((i, level, l[level:].strip()))

if len(sys.argv) == 2:
    for i, level, text in heads:
        print(f"{i + 1}:{'  ' * (level - 1)}{text}")
    sys.exit()

want = sys.argv[2].lower()
found = False
for n, (i, level, text) in enumerate(heads):
    if want not in text.lower():
        continue
    end = next((j for j, lv, _ in heads[n + 1:] if lv <= level), len(lines))
    print(f"--- {sys.argv[1]}:{i + 1}")
    print("\n".join(lines[i:end]).rstrip())
    found = True
if not found:
    sys.exit(f"no heading contains {sys.argv[2]!r}; run without TEXT to list them")
