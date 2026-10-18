#!/usr/bin/env python3
"""Flag left-out features in docs/games/*.md that state no reason.

A sentence with a phrase from DROP but none from REASON is flagged.
Convention: docs/new-game.md, "Say why a feature is left out".
Usage: python3 -I research/tools/dropped_lint.py [--max N] [files...]
With files: exit 1 if anything is flagged. Without (CI): exit 1 when a game
exceeds its CEIL; lower CEIL as reasons are added, never raise it.
"Not yet" is not in DROP (open work, not a dropped feature) and TEMPLATE.md
is skipped (it names the phrase). Tune the two regexes below."""
import re, sys, glob, os

DROP = re.compile(r"left out|not converted|dropped|skipped", re.I)
REASON = re.compile(
    r"because|since |the exe|the user|decided|\(user|on purpose|src:|dropped: ", re.I)

CEIL = {"django": 5, "muramasa-en": 13, "muramasa": 25, "oretsuba": 27, "saihate-no-ima": 29, "tsukihime": 21}  # game id -> sentences without a reason, as of 2026-10-18

def sentences(path):
    """Yield (line, sentence) per sentence; paragraphs are joined, fences skipped."""
    para, fence = [], False
    def flush():
        if not para: return
        text, pos = "", []
        for n, l in para:
            pos.append((len(text), n)); text += l.strip() + " "
        start = 0
        for m in list(re.finditer(r"(?<=[.!?;]) +(?=[A-Z`(*|])", text)) + [None]:
            end = m.start() if m else len(text)
            s = text[start:end].strip()
            if s:
                yield [n for p, n in pos if p <= start][-1], s
            start = m.end() if m else end
        para.clear()
    for n, l in enumerate(open(path, encoding="utf-8"), 1):
        if l.lstrip().startswith("```"):
            yield from flush(); fence = not fence; continue
        if fence: continue
        if not l.strip() or l.startswith("#"):
            yield from flush(); continue
        if re.match(r"\s*([-*]|\d+\.) ", l) or l.startswith("|"):
            yield from flush()
        para.append((n, l))
    yield from flush()

def main(a):
    mx = None
    if "--max" in a:
        i = a.index("--max"); mx = int(a[i + 1]); del a[i:i + 2]
    files = a or [f for f in sorted(glob.glob("docs/games/*.md")) if not f.endswith("TEMPLATE.md")]
    hits = [(f, n, s) for f in files for n, s in sentences(f)
            if DROP.search(s) and not REASON.search(s)]
    for f, n, s in hits[:mx]:
        print(f"{f}:{n}: {s[:200]}")
    print(f"{len(hits)} left-out sentences without a reason", file=sys.stderr)
    if a:
        return 1 if hits else 0
    bad = 0
    for f in files:
        g = os.path.basename(f)[:-3]
        n = sum(1 for h in hits if h[0] == f)
        if n > CEIL.get(g, 0):
            print(f"{g}: {n} left-out sentences without a reason, ceiling {CEIL.get(g, 0)}: say why (docs/new-game.md)"); bad = 1
        elif n < CEIL.get(g, 0):
            print(f"{g}: {n} below ceiling {CEIL[g]}: lower CEIL in research/tools/dropped_lint.py")
    return bad

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
