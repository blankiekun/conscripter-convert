#!/usr/bin/env python3
"""Light lint of docs/games/<id>.md against TEMPLATE.md: missing template
headings (prefix match, so "Status: ..." counts) plus a missing Executables
table, and the length of the Status section. Per-game ceilings as of
2026-10-18: lower them as docs improve, never raise. Exit 1 over a ceiling.
usage: python3 -I research/tools/doclint.py"""
import glob, os, sys
T = ["Scope", "Build", "Executables", "Files and formats", "Script language",
     "Story structure and walkthrough", "Progression variables", "Testing", "Status", "History"]
MISSING = {"django": 1, "muramasa-en": 7, "muramasa": 4, "oretsuba": 5, "saihate-no-ima": 3, "tsukihime": 0}
STATUS = {"django": 107, "muramasa-en": 76, "muramasa": 132, "oretsuba": 84, "saihate-no-ima": 85, "tsukihime": 129}  # lines
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../.."))
bad = 0
for f in sorted(glob.glob("docs/games/*.md")):
    g = os.path.basename(f)[:-3]
    if g == "TEMPLATE":
        continue
    L = open(f, encoding="utf-8").read().split("\n")
    H = [i for i, l in enumerate(L) if l.startswith("## ")] + [len(L)]
    sec = {L[i][3:]: L[i:H[k + 1]] for k, i in enumerate(H[:-1])}
    find = lambda t: next((v for h, v in sec.items() if h.startswith(t)), None)
    miss = [t for t in T if find(t) is None]
    ex = find("Executables")
    if ex is not None and not any(l.startswith("|") for l in ex):
        miss.append("Executables table")
    st = len(find("Status") or [])
    for what, n, cap in (("template items missing " + str(miss), len(miss), MISSING.get(g, 0)),
                         ("Status lines", st, STATUS.get(g, 0))):
        if n > cap:
            print(f"{f}: {what}: {n} > ceiling {cap}"); bad = 1
sys.exit(bad)
