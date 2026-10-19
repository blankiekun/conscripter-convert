#!/usr/bin/env python3
"""Tie docs/games/<game>.findings.tsv to the kinds the converter sees.

usage: findings_lint.py        (run from anywhere in the repo)

Kinds come from tests/snapshots/<game>.metrics.json. Every kind needs a
token in some row's first column (or an entry in NO_ROW with a reason),
and every name in that column must be a kind (or in OTHER: names that
are not kinds). Xuse opcodes are hex: `0x01-0x0e` is a range (one
member must be a kind), `0x40, 0x41` a list. A game without a table
is skipped. Exits 1 on any finding.

`(scheduler)` rows (docs/new-game.md step 15) answer the overlap
questions in QUESTIONS, one row each, the question in the second column.
Each row's last column names its check: a test (`file.rs::fn`, a path
under crates/ ending so, holding `fn <fn>`) or a ledger reason
(`ledger: "<reason>"`, a string the converter's source holds). A name
that doesn't exist fails. A game missing questions only warns, up to its
SCHED_MAX ceiling (lowered as rows land, never raised). Every run
self-tests the check first (`--self-test`: only that).

The command reference: each snapshot's `engine_doc` names
docs/engines/<engine_doc>.md, whose one table needs a row per kind (the
Command cell the kind in backticks), and every row names a kind of a game that uses the doc.
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
# the overlap questions every game with timed actions answers in `(scheduler)` rows
QUESTIONS = ("unwaited goes on", "0 ms on in-flight", "wait on in-flight", "replace or delete while moving")
# ceiling per game of questions with no `(scheduler)` row: a warning up to it, a failure over it.
# shortcut: every game starts at all 4 (no rows yet, 2026-10-18); a game not listed gets all 4 too.
# Lower each as rows land; at 0 a missing row fails, so set every game to 0 once its rows exist.
SCHED_MAX = {g: 4 for g in ("django", "muramasa", "muramasa-en", "oretsuba", "saihate-no-ima", "tsukihime")}
GENERIC = {"handler", "command", "all", "dispatch", "hidden", "state", "sound", "timing", "effects", "effect", "waits", "timed"}

# an engine doc that holds only its own commands, the rest in the base doc's

bad = 0
def fail(msg):
    global bad
    bad += 1
    print(msg)


def sched_check(rows, files, source):
    """rows: the `(scheduler)` rows' columns; files: repo paths; source(path) -> text.
    Returns (questions with no row, errors)."""
    errs, have = [], set()
    for cols in rows:
        q, check = cols[1].strip() if len(cols) > 1 else "", cols[-1].strip()
        if q not in QUESTIONS:
            errs.append(f"`(scheduler)` row `{q}`: not one of {', '.join(QUESTIONS)}")
            continue
        have.add(q)
        m = re.search(r'ledger: "([^"]+)"', check)
        if m:
            if not any(f.startswith("crates/") and f.endswith(".rs") and m.group(1) in source(f) for f in files):
                errs.append(f"`(scheduler)` row `{q}`: no ledger reason \"{m.group(1)}\" in crates/")
            continue
        m = re.search(r"([\w/.-]+\.rs)::(\w+)", check)
        if not m:
            errs.append(f"`(scheduler)` row `{q}`: last column names no test (`file.rs::fn`) or ledger reason (`ledger: \"...\"`)")
        elif not any(f.startswith("crates/") and (f == m.group(1) or f.endswith("/" + m.group(1)))
                     and re.search(rf"\bfn {m.group(2)}\b", source(f)) for f in files):
            errs.append(f"`(scheduler)` row `{q}`: no `fn {m.group(2)}` in a crates/ file ending {m.group(1)}")
    return [q for q in QUESTIONS if q not in have], errs


def self_test():
    src = {"crates/a/tests/sync.rs": "fn keeps_the_pan() {}", "crates/a/sync.rs": 'note("timed action dropped")'}
    rows = [["(scheduler)", "unwaited goes on", "seen", "tests/sync.rs::keeps_the_pan"],
            ["(scheduler)", "0 ms on in-flight", "seen", 'ledger: "timed action dropped"'],
            ["(scheduler)", "wait on in-flight", "seen", "sync.rs::gone"],
            ["(scheduler)", "made up", "seen", "x"]]
    miss, errs = sched_check(rows, list(src), src.get)
    assert miss == ["replace or delete while moving"], miss
    assert len(errs) == 2 and "fn gone" in errs[0] and "made up" in errs[1], errs
    assert sched_check([], [], src.get)[0] == list(QUESTIONS)


self_test()  # cheap: every run (CI's too) checks the checker
if sys.argv[1:] == ["--self-test"]:
    sys.exit(print("self-test ok"))
files = subprocess.run(["git", "ls-files", "crates"], capture_output=True, text=True, check=True).stdout.split()
_src = {}
def source(f):
    if f not in _src:
        _src[f] = open(f, encoding="utf-8", errors="replace").read()
    return _src[f]

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
    seen = set()
    sched = []
    for line in open(tsv, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        cols = line.rstrip("\n").split("\t")
        if line in seen:
            fail(f"{tsv}: duplicate row `{cols[0]}` (a union merge can double a row)")
        seen.add(line)
        if col is None:
            col = next((i for i, c in enumerate(cols) if c in ("status", "mark")), -1)
            continue
        if col >= 0 and len(cols) > col and cols[col].strip() in ("inferred", "unknown"):
            nopen += 1
        cell = cols[0]
        if cell == "(scheduler)":
            sched.append(cols)
            continue
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
    smiss, serrs = sched_check(sched, files, source)
    for e in serrs:
        fail(f"{tsv}: {e}")
    smax = SCHED_MAX.get(g, len(QUESTIONS))
    if len(smiss) > smax:
        fail(f"{g}: {len(smiss)} overlap questions with no `(scheduler)` row, ceiling {smax}: {'; '.join(smiss)}")
    elif smiss:
        print(f"{g}: WARN {len(smiss)} overlap questions with no `(scheduler)` row (new-game.md step 15): {'; '.join(smiss)}")
    if len(smiss) < smax:
        print(f"{g}: lower SCHED_MAX[{g!r}] to {len(smiss)}")
    if miss:
        fail(f"{tsv}: kinds with no row (add one, or NO_ROW in research/tools/findings_lint.py): {' '.join(miss)}")


def doc_rows(stem):
    """The Command names of docs/engines/<stem>.md's one table (None: no file)."""
    path = f"docs/engines/{stem}.md"
    if not os.path.exists(path):
        return None
    lines = [l for l in open(path, encoding="utf-8") if l.startswith("|")]
    heads = [i for i, l in enumerate(lines) if re.match(r"\|\s*-", l)]
    if len(heads) != 1:
        fail(f"{path}: {len(heads)} tables, want exactly one")
        return set()
    cols = [c.strip() for c in lines[heads[0] - 1].strip().strip("|").split("|")]
    if "Command" not in cols or "What it does" not in cols:
        fail(f"{path}: the table's header lacks a Command or a What it does column")
        return set()
    names = set()
    for l in lines[heads[0] + 1:]:
        cells = [c.strip() for c in re.split(r"(?<!\\)\|", l.strip())[1:-1]]
        m = re.fullmatch(r"`([^`]+)`", cells[cols.index("Command")])
        if not m:
            fail(f"{path}: row `{cells[0]}`: the Command cell is not one name in backticks")
            continue
        if not cells[cols.index("What it does")]:
            fail(f"{path}: row `{m.group(1)}`: What it does is empty (write `unknown`)")
        names.add(m.group(1))
    return names


docs, used = {}, {}
for mf in sorted(os.listdir("tests/snapshots")):
    if not mf.endswith(".metrics.json"):
        continue
    m = json.load(open(f"tests/snapshots/{mf}"))
    stem = m.get("engine_doc")
    if not stem:
        fail(f"tests/snapshots/{mf}: no engine_doc")
        continue
    if stem not in docs:
        docs[stem] = doc_rows(stem)
        if docs[stem] is None:
            fail(f"tests/snapshots/{mf}: engine_doc {stem}: no docs/engines/{stem}.md")
            docs[stem] = set()
    miss = sorted(set(m["kinds"]) - docs[stem])
    if miss:
        fail(f"docs/engines/{stem}.md: kinds of {mf[:-13]} with no row: {', '.join(miss)}")
    used.setdefault(stem, set()).update(m["kinds"])
for st, names in docs.items():
    for n in sorted(names - used.get(st, set())):
        fail(f"docs/engines/{st}.md: row `{n}` is no game's kind (renamed? remove it)")
print(f"engine docs: {len(docs)} files, {sum(map(len, docs.values()))} rows")
sys.exit(1 if bad else 0)
