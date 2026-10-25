#!/usr/bin/env python3
"""Lists every F/G script word the exe touches at a constant index.

    engine_vars.py <ghidra decomp dir> > engine_vars.tsv

F words are u16 at 0x146B442 (pointer at 0xBE0290), G words at 0x146C892
(0xBE04E0); 0xBE0890 points at F1200, 0xBE07A8 at G2000 (relocations of
main-upd.elf). Matches `PTR + const` and local copies of the pointer
(`puVar1 = PTR_DAT_00be0290;`) up to their next assignment in source
order, so a copy set on one branch can be misattributed: check the line. `R[]`/`W[]`: an array
from that word, indexed by a variable. Prints var, index, access
(R/W), function, the source line.
"""
import os
import re
import sys

BASES = {
    "PTR_DAT_00be0290": ("F", 0),
    "PTR_DAT_00be0890": ("F", 1200),
    "PTR_DAT_00be04e0": ("G", 0),
    "PTR_DAT_00be07a8": ("G", 2000),
}
ASSIGN = re.compile(r"(?<![\w.>*])(\w+) = ([^;=][^;]*);")
WRITE = re.compile(r"^\s*(=(?!=)|\|=|&=|\+=|-=|\^=)")


def scan(path):
    src = open(path, encoding="utf-8", errors="replace").read()
    func = os.path.basename(path)[:-2]
    names = dict(BASES)
    for line in src.splitlines():
        alt = "|".join(re.escape(n) for n in names)
        pat = re.compile(
            r"\*\((?:\w+ )*\w+ \*\)(?:\((" + alt + r")(?: \+ (0x[0-9a-f]+|\d+))?\)|("
            + alt + r")(?![\w +\[]))"
        )
        for m in pat.finditer(line):
            var, base = names[m.group(1) or m.group(3)]
            off = int(m.group(2), 0) if m.group(2) else 0
            if off % 2:
                continue
            acc = "W" if WRITE.match(line[m.end():]) else "R"
            print(f"{var}\t{base + off // 2}\t{acc}\t{func}\t{line.strip()}")
        arr = re.compile(r"\((" + alt + r") \+ [^;]*?\* 2 \+ (0x[0-9a-f]+)\)")
        for m in arr.finditer(line):
            var, base = names[m.group(1)]
            acc = "W[]" if WRITE.match(line[m.end():]) else "R[]"
            print(f"{var}\t{base + int(m.group(2), 16) // 2}\t{acc}\t{func}\t{line.strip()}")
        # a copy holds from its assignment to the next one (source order)
        for name, rhs in ASSIGN.findall(line):
            if name in BASES:
                continue
            if rhs in BASES:
                names[name] = BASES[rhs]
            else:
                names.pop(name, None)


def main():
    d = sys.argv[1]
    for f in sorted(os.listdir(d)):
        if f.endswith(".c"):
            scan(os.path.join(d, f))


main()
