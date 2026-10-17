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
    "tsukihime": set("""_ATSV _FADE _FANX _FCLN _FCSP _IFNX _IFP4 _IFSC _IFUR _MSQK _MVOL _NOP_ _PGST _RDAS _RDED _RTM2 _RTMN _RTM_ _RTST _SACL _SBAS _SELA _SELD _SELS _SESP _SEVL _SGMA _SHAF _SHK2 _SHOK _SHS2 _SNX2 _SNX3 _SNX4 _SOXY _SPFC _SPMA _SPRC _SPRD _SQK2 _SQS2 _SSS2 _SSW2 _STCF _STCG _STCL _STNS _STPV _STQK _STRA _STRR _STTR _TOMP _TOMW _TRPY _VCWT _VPL2 _WKOR _WTSV _XRSM _XSTP""".split()),  # no row yet
}
# ceiling per game for NO_ROW: lower it as rows land, never raise it
NO_ROW_MAX = {"django": 0, "tsukihime": 61}
# names in the first column that are not kinds
OTHER = {
    "saihate-no-ima": {"0x81", "0x11"},
    "tsukihime": {"motion", "runtime", "spzoom"},
    "django": {"Camera", "Position", "text"},
    "oretsuba": {"Action", "voice"},
}
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
    for line in open(tsv, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        cell = line.split("\t")[0]
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
            for t in re.findall(r"\w+", cell):
                named.add(t)
                if t not in kinds and t not in OTHER.get(g, ()) and t.lower() not in GENERIC:
                    fail(f"{tsv}: row `{cell}`: {t} is not a kind")
    for rng, cell in ranges:
        if not rng & kinds:
            fail(f"{tsv}: range `{cell}` holds no kind")
    miss = sorted(kinds - named - NO_ROW.get(g, set()))
    nr = NO_ROW.get(g, set())
    print(f"{g}: {len(nr)} kinds in NO_ROW")
    for k in sorted(nr & named):
        fail(f"{tsv}: {k} now has a row: remove {k} from NO_ROW")
    for k in sorted(nr - kinds):
        fail(f"{tsv}: {k} in NO_ROW is not a kind: remove it")
    if len(nr) > NO_ROW_MAX.get(g, 0):
        fail(f"{g}: NO_ROW has {len(nr)} kinds, ceiling {NO_ROW_MAX.get(g, 0)}: add rows, don't grow the list")
    if miss:
        fail(f"{tsv}: kinds with no row (add one, or NO_ROW in research/tools/findings_lint.py): {' '.join(miss)}")
sys.exit(1 if bad else 0)
