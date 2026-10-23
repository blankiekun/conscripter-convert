"""Lists the sprite records the eboot's screens build: every call of the
sprite-add function 0x8108a780 (s0, s1, r0 list, r1 record, ...) in the
Ghidra decompile, with the record's fields as the code sets them before
the call (stack locals named by offset: local_e8 at -0xe8).

    python3 -I sprite_records.py <ghidra>/decomp [function-address ...]

Record fields: +4 id, +8 SystemData picture (ITOC ID), +0x18/+0x1c the
centre x, y, +0x20/+0x24 width, height, +0x28/+0x2c the atlas cell's
x, y, +0x30/+0x34 its width, height (title_menu_init 0x81083fd8: id
0x65, picture 23 title_bg, (786, 258), 173x36, cell (0, 545)).
Values the code computes print as the decompile's expression.
"""
import os
import re
import struct
import sys

FIELDS = {4: "id", 8: "pic", 0x18: "x", 0x1c: "y", 0x20: "w", 0x24: "h", 0x28: "u", 0x2c: "v", 0x30: "uw", 0x34: "vh"}
ASSIGN = re.compile(r"^\s*(local|uStack|auStack)_([0-9a-f]+)(?:\[(\d+)\])?\s*=\s*(.+);\s*$")
CALL = re.compile(r"FUN_8108a780\((.*)\)")


def val(s):
    s = s.strip()
    m = re.fullmatch(r"(0x[0-9a-f]+|\d+)", s)
    if not m:
        return s
    n = int(s, 0)
    if n >= 0x3c000000 and n < 0x4b000000 or n >= 0xbc000000 and n < 0xcb000000:
        return round(struct.unpack("<f", struct.pack("<I", n))[0], 4)
    return n


def scan(path):
    env, out = {}, []
    for line in open(path, encoding="utf-8", errors="replace"):
        m = ASSIGN.match(line)
        if m:
            off = int(m.group(2), 16) - 4 * int(m.group(3) or 0)
            env[off] = m.group(4)
            continue
        m = CALL.search(line)
        if m:
            r = re.search(r"&(?:local|uStack|auStack)_([0-9a-f]+)", m.group(1))
            if not r:
                out.append({"call": m.group(1)})
                continue
            base = int(r.group(1), 16)
            out.append({name: val(env.get(base - off, "?")) for off, name in FIELDS.items()})
    return out


d = sys.argv[1]
want = {a.lower().removeprefix("0x") for a in sys.argv[2:]}
for f in sorted(os.listdir(d)):
    if want and f.split("_")[0] not in want:
        continue
    if "FUN_8108a780(" not in open(os.path.join(d, f), encoding="utf-8", errors="replace").read() or f.startswith("8108a780"):
        continue
    print(f"## {f[:-2]}")
    for r in scan(os.path.join(d, f)):
        print("  " + " ".join(f"{k}={v}" for k, v in r.items()))
