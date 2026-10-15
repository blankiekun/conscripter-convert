#!/usr/bin/env python3
"""Print one label's block of a generated 0.txt (cp932), with line numbers.

usage: label.py 0.txt LABEL [MAX_LINES]     (default 60 lines)
       label.py 0.txt -l TEXT               list labels containing TEXT

The block runs to the next "*label" line. Line numbers match the file,
so a longer stretch can be read on with sed -n a,bp.
"""
import sys

if len(sys.argv) < 3:
    sys.exit(__doc__)
# split on \n only: cp932 text holds bytes that splitlines() would also break on
lines = open(sys.argv[1], "rb").read().decode("cp932", "replace").split("\n")

if sys.argv[2] == "-l":
    want = sys.argv[3].lower() if len(sys.argv) > 3 else ""
    for i, l in enumerate(lines):
        if l.startswith("*") and want in l.lower():
            print(f"{i + 1}:{l.rstrip()}")
    sys.exit()

name = sys.argv[2].lstrip("*").lower()
limit = int(sys.argv[3]) if len(sys.argv) > 3 else 60
start = next((i for i, l in enumerate(lines) if l.startswith("*") and l[1:].strip().lower() == name), None)
if start is None:
    sys.exit(f"no label *{name}; try -l {name}")
end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("*")), len(lines))
for i in range(start, min(end, start + limit)):
    print(f"{i + 1}:{lines[i].rstrip()}")
if end - start > limit:
    print(f"... {end - start - limit} more lines, to line {end}")
