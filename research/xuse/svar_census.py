#!/usr/bin/env python3
"""Every variable operand in the numbered scripts: the exe's slot vs ours.

The exe (getters 0x473cb0 / 0x46e360, tables filled by load_svar
0x46de00): bit 15 set -> slot table 0x519554 (150 entries, LocalSave),
clear -> system table 0x519544 (20 entries, GlobalSave); index =
code & 0x7fff, bits 16 up ignored; an index past the table logs
"シナリオ変数 ix=%u は存在していません" and reads nothing.
Ours: `script_var` in crates/conv-engines/src/xuse/convert.rs.

Operands read: 0x01/0x02 sub, 0x08/0x0d sub and arg (by variable),
0x16-0x1b pool[arg] u32, 0x46 record style (u32 at +6). Not read: 0x30
(the debug menu's computed goto, 0001.cd, left out).

usage: python3 -I svar_census.py <install>/HD/CDFiles
"""
import collections
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cdlib  # noqa: E402


def exe_slot(code):
    table, cap = ("slot", 150) if code & 0x8000 else ("system", 20)
    n = code & 0x7FFF
    return f"{table}[{n}]" + ("" if n < cap else " OUT OF RANGE")


def ours(code):
    n = code & 0x7FFF
    if code & 0x8000:
        return f"%{10 + n if n <= 10 else 18 + n}"
    return f"%{230 + n}"


uses = collections.defaultdict(collections.Counter)
for f in sorted(Path(sys.argv[1]).glob("[0-9][0-9][0-9][0-9].cd")):
    _, ins, pool = cdlib.load(f)
    for op, sub, arg, _ in ins:
        found = []
        if op in (0x01, 0x02, 0x08, 0x0D):
            found.append(sub)
        if op in (0x03,) or 0x08 <= op <= 0x0E:
            found.append(arg & 0xFFFF)
        if 0x16 <= op <= 0x1B:
            found.append(struct.unpack_from("<I", pool, arg)[0])
        if op == 0x46:
            found.append(struct.unpack_from("<I", pool, arg + 6)[0])
        for code in found:
            uses[code][f"0x{op:02x}"] += 1

total = sum(sum(c.values()) for c in uses.values())
hi = sum(sum(c.values()) for k, c in uses.items() if k & 0x8000)
print(f"operands {total}, bit 15 set {hi}, distinct {len(uses)}, "
      f"distinct with bit 15 {sum(1 for k in uses if k & 0x8000)}")
ours_seen = collections.defaultdict(set)
for code in sorted(uses):
    ours_seen[ours(code)].add(code)
    ops = " ".join(f"{o}x{n}" for o, n in sorted(uses[code].items()))
    print(f"0x{code:04x}\texe {exe_slot(code)}\tours {ours(code)}\t{ops}")
clash = {v: c for v, c in ours_seen.items() if len(c) > 1}
print("ours collisions:", clash or "none")
