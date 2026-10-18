#!/usr/bin/env python3
"""Tie docs/games/<game>.findings.tsv to the kinds the converter sees.

usage: findings_lint.py        (run from anywhere in the repo)

Kinds come from tests/snapshots/<game>.metrics.json. Every kind needs a
token in some row's first column (or an entry in NO_ROW with a reason),
and every name in that column must be a kind (or in OTHER: names that
are not kinds). Xuse opcodes are hex: `0x01-0x0e` is a range (one
member must be a kind), `0x40, 0x41` a list. A game without a table
is skipped. Exits 1 on any finding.
"""
import json
import os
import re
import subprocess
import sys

os.chdir(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True).stdout.strip())

# kinds with no row yet: next mapping work
NO_ROW = {
}
# ceiling per game for NO_ROW: lower it as rows land, never raise it
NO_ROW_MAX = {"django": 0, "tsukihime": 0}
# names in the first column that are not kinds
OTHER = {
    "saihate-no-ima": {"0x81", "0x11"},
    "tsukihime": {"motion", "runtime", "spzoom"},
    "django": {"Camera", "Position", "text"},
    "oretsuba": {"Action", "voice"},
}
# ceiling per game for rows marked inferred or unknown (status/mark column): lower it as rows become seen, never raise it
OPEN_MAX = {"django": 6, "muramasa-en": 4, "muramasa": 33, "saihate-no-ima": 1, "oretsuba": 15, "tsukihime": 35}
GENERIC = {"handler", "command", "all", "dispatch", "hidden", "state", "sound", "timing", "effects", "effect", "waits", "timed"}

bad = 0
def fail(msg):
    global bad
    bad += 1
    print(msg)

for mf in sorted(os.listdir("tests/snapshots")):
    if not mf.endswith(".metrics.json"):
        continue
    g = mf[: -len(".metrics.json")]
    tsv = f"docs/games/{g}.findings.tsv"
    if not os.path.exists(tsv):
        continue
    kinds = set(json.load(open(f"tests/snapshots/{mf}"))["kinds"])
    hexmode = g == "saihate-no-ima"
    if hexmode:
        kinds = {k.split()[1] for k in kinds}
    named, ranges = set(), []
    col, nopen = None, 0
    for line in open(tsv, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        cols = line.rstrip("\n").split("\t")
        if col is None:
            col = next((i for i, c in enumerate(cols) if c in ("status", "mark")), -1)
            continue
        if col >= 0 and len(cols) > col and cols[col].strip() in ("inferred", "unknown"):
            nopen += 1
        cell = cols[0]
        if hexmode:
            for a, b in re.findall(r"(0x[0-9a-f]+)(?:-(0x[0-9a-f]+))?", cell):
                if b:
                    rng = {f"0x{i:02x}" for i in range(int(a, 16), int(b, 16) + 1)}
                    ranges.append((rng, cell))
                    named |= rng
                else:
                    named.add(a)
                    if a not in kinds and a not in OTHER[g]:
                        fail(f"{tsv}: row `{cell}`: {a} is not a kind")
        else:
            # a kind of several words (`port Print`) is named by its whole comma-separated part
            named |= {part.strip() for part in cell.split(",")}
            words = {w for k in kinds for w in k.split()}
            for t in re.findall(r"\w+", cell):
                named.add(t)
                if t not in words and t not in OTHER.get(g, ()) and t.lower() not in GENERIC:
                    fail(f"{tsv}: row `{cell}`: {t} is not a kind")
    for rng, cell in ranges:
        if not rng & kinds:
            fail(f"{tsv}: range `{cell}` holds no kind")
    miss = sorted(kinds - named - NO_ROW.get(g, set()))
    nr = NO_ROW.get(g, set())
    print(f"{g}: {len(nr)} kinds in NO_ROW, {nopen} rows inferred/unknown")
    if nopen > OPEN_MAX.get(g, 0):
        fail(f"{g}: {nopen} inferred/unknown rows, ceiling {OPEN_MAX.get(g, 0)}: look them up in the decompile, don't grow the list")
    elif nopen < OPEN_MAX.get(g, 0):
        print(f"{g}: lower OPEN_MAX[{g!r}] to {nopen}")
    for k in sorted(nr & named):
        fail(f"{tsv}: {k} now has a row: remove {k} from NO_ROW")
    for k in sorted(nr - kinds):
        fail(f"{tsv}: {k} in NO_ROW is not a kind: remove it")
    if len(nr) > NO_ROW_MAX.get(g, 0):
        fail(f"{g}: NO_ROW has {len(nr)} kinds, ceiling {NO_ROW_MAX.get(g, 0)}: add rows, don't grow the list")
    if miss:
        fail(f"{tsv}: kinds with no row (add one, or NO_ROW in research/tools/findings_lint.py): {' '.join(miss)}")
sys.exit(1 if bad else 0)
