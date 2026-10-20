#!/usr/bin/env python3 -I
"""Value census of a converted script: what values does each command's
argument take, and which look like a sentinel the converter misread?

  python3 -I research/tools/argcensus.py <0.txt> [--screen 1920] [--min 20] [--top 5]
  python3 -I research/tools/argcensus.py --self-test

Reads the dry run's 0.txt (any engine: it is NScripter text), splits each
line into ':'-separated statements, and counts per command and argument
position the integer literals. Prints per argument: count, min, max,
top values, and flags:
  SENTINEL  a value within 3 of +-32767, +-65535, +-2^31 (or 0x8000-like
            -32768), or -1 in more than 30% of the calls
  OUTLIER   |value| > 10x --screen
Compare each flagged value with the exe's comparisons against constants
(docs/new-game-reference.md, "Mapping the whole exe"). Variables (%N), strings and labels
are skipped; commands seen fewer than --min times are not listed.
"""
import re, sys, collections

SENT = [32767, 65535, 2**31]


def split_args(s):
    out, cur, q = [], "", False
    for c in s:
        if c == '"':
            q = not q
        if c == "," and not q:
            out.append(cur.strip()); cur = ""
        else:
            cur += c
    out.append(cur.strip())
    return out


def statements(line):
    line = line.split(";")[0] if line.lstrip().startswith(";") else line
    st, cur, q = [], "", False
    for c in line:
        if c == '"':
            q = not q
        if c == ":" and not q:
            st.append(cur); cur = ""
        else:
            cur += c
    return st + [cur]


def census(lines):
    d = collections.defaultdict(collections.Counter)  # (cmd, idx) -> values
    calls = collections.Counter()
    for line in lines:
        for s in statements(line):
            m = re.match(r"\s*(?:if\s.*?\s)?([a-z_][a-z0-9_]*)\s+(\S.*)$", s)
            if not m or m.group(1) in ("if", "mov", "add", "sub", "gosub", "goto", "dim"):
                continue
            calls[m.group(1)] += 1
            for i, a in enumerate(split_args(m.group(2))):
                if re.fullmatch(r"-?\d+", a):
                    d[(m.group(1), i)][int(a)] += 1
    return d, calls


def flags(c, screen):
    n = sum(c.values())
    f = set()
    for v, k in c.items():
        if any(abs(abs(v) - s) <= 3 for s in SENT):
            f.add("SENTINEL")
        if v == -1 and k > 0.3 * n:
            f.add("SENTINEL(-1)")
        if abs(v) > 10 * screen:
            f.add("OUTLIER")
    return sorted(f)


def report(lines, screen=1920, minc=20, top=5):
    d, calls = census(lines)
    rows = []
    for (cmd, i), c in sorted(d.items()):
        if calls[cmd] < minc:
            continue
        f = flags(c, screen)
        if not f:
            continue
        sus = sorted(c.items(), key=lambda kv: -kv[1])
        bad = [(v, k) for v, k in sus if any(abs(abs(v) - s) <= 3 for s in SENT) or abs(v) > 10 * screen or v == -1]
        rows.append(f"{cmd}:{i} n={sum(c.values())} min={min(c)} max={max(c)} "
                    f"top={sus[:top]} flagged={bad[:top]} {','.join(f)}")
    return rows


def self_test():
    ls = ["lsp 1,\"a.png\",10,-32766:vsp 1,1", "lsp 2,\"b.png\",5,-32766", "lsp 3,\"c.png\",5,7"]
    d, _ = census(ls)
    assert d[("lsp", 3)] == {-32766: 2, 7: 1}, d
    assert d[("lsp", 0)] == {1: 1, 2: 1, 3: 1}
    r = report(ls, minc=1)
    assert any(x.startswith("lsp:3") and "SENTINEL" in x for x in r), r
    assert not any(x.startswith("lsp:2") for x in r)
    print("ok")


if __name__ == "__main__":
    a = sys.argv[1:]
    if "--self-test" in a:
        self_test(); sys.exit()
    opt = {"--screen": 1920, "--min": 20, "--top": 5}
    for k in opt:
        if k in a:
            i = a.index(k); opt[k] = int(a[i + 1]); del a[i:i + 2]
    with open(a[0], encoding="cp932", errors="replace") as f:
        print("\n".join(report(f.read().splitlines(), opt["--screen"], opt["--min"], opt["--top"])))
