#!/usr/bin/env python3
"""Which findings rows the original runs: xtrace's one-shot breakpoints.

usage: cover.py addrs <Farthest2015.exe>       > cover.txt
       cover.py mark <trace.jsonl>... [--write]

addrs: the code addresses to arm (research/xuse/tracedll/xtrace.c,
XTRACE_COVER): every address in research/xuse/names.tsv and in column 4
of docs/games/saihate-no-ima.findings.tsv that objdump decodes as an
instruction start in .text (an int3 inside an instruction would break
the game).

mark: reads the traces' "cover" (armed addresses), "hit" and "cmd"
records and fills the findings table's last column, "trace": per row,
`runs: <addresses hit>` (plus `; not: <armed ones never hit>`) or
`never runs`, from the row's armed addresses; a row without one goes
by its opcodes (column 1) in the cmd records; `-` when it has neither.
"never runs" means: not in these traces. Prints the marks; --write
rewrites the table.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TSV = ROOT / "docs/games/saihate-no-ima.findings.tsv"
NAMES = ROOT / "research/xuse/names.tsv"
TEXT = (0x401000, 0x4F630B)  # .text of the exe (sha256 efb7ec8e...)
HEAD = "trace (research/xuse/cover.py)"


def addrs_in(text):
    return {int(a, 16) for a in re.findall(r"(?<![0-9a-f])(004[0-9a-f]{5})(?![0-9a-f])", text) if TEXT[0] <= int(a, 16) < TEXT[1]}


def ops_in(cell):
    ops = set()
    for a, b in re.findall(r"(0x[0-9a-f]+)(?:-(0x[0-9a-f]+))?", cell):
        ops |= set(range(int(a, 16), int(b or a, 16) + 1))
    return ops


def rows():
    return [l.rstrip("\n").split("\t") for l in open(TSV, encoding="utf-8")]


def addrs(exe):
    want = set()
    for l in open(NAMES, encoding="utf-8"):
        if not l.startswith("#") and l.strip():
            want |= addrs_in(l.split("\t")[0])
    for r in rows()[1:]:
        if len(r) > 3:
            want |= addrs_in(r[3])
    asm = subprocess.run(["objdump", "-d", "--no-show-raw-insn", exe], capture_output=True, text=True, check=True).stdout
    starts = {int(m, 16) for m in re.findall(r"^\s+([0-9a-f]+):", asm, re.M)}
    for a in sorted(want & starts):
        print(f"{a:08x}")
    print(f"{len(want & starts)} of {len(want)} addresses are instruction starts", file=sys.stderr)


def mark(traces, write):
    armed, hit, ran = set(), set(), set()
    for t in traces:
        for l in open(t, encoding="utf-8"):
            r = json.loads(l)
            if r["t"] == "cover":
                armed |= {int(a, 16) for a in r["addrs"]}
            elif r["t"] == "hit":
                hit.add(int(r["addr"], 16))
            elif r["t"] == "cmd":
                ran.add(int(r["op"], 16))
    if not armed:
        sys.exit("no cover record: run xtrace.sh (it arms cover.py addrs)")
    out = rows()
    width = len(out[0])
    if out[0][-1] == HEAD:
        width -= 1
    out[0] = out[0][:width] + [HEAD]
    for r in out[1:]:
        r[:] = (r + [""] * width)[:width]
        mine = addrs_in(r[3]) & armed
        ops = ops_in(r[0]) if not r[0].startswith("(") else set()
        if mine:
            yes, no = sorted(mine & hit), sorted(mine - hit)
            m = "runs: " + ", ".join(f"{a:08x}" for a in yes) + ("; not: " + ", ".join(f"{a:08x}" for a in no) if no else "") if yes else "never runs"
        elif ops:
            m = "runs: op " + ", ".join(f"0x{o:02x}" for o in sorted(ops & ran)) if ops & ran else "never runs"
        else:
            m = "-"
        r.append(m)
        print(f"{r[0]} | {r[1]}: {m}")
    print(f"{len(hit & armed)} of {len(armed)} armed addresses hit, {len(ran)} opcodes ran", file=sys.stderr)
    if write:
        TSV.write_text("".join("\t".join(r) + "\n" for r in out), encoding="utf-8")


if __name__ == "__main__":
    a = sys.argv[1:]
    if a[:1] == ["addrs"] and len(a) == 2:
        addrs(a[1])
    elif a[:1] == ["mark"] and len(a) > 1:
        mark([x for x in a[1:] if x != "--write"], "--write" in a)
    else:
        sys.exit(__doc__)
