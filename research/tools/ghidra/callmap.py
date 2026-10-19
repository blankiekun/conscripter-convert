#!/usr/bin/env python3
"""Which scheduler functions each built-in reaches: a call map over a
ghidra_export.sh export.

    python3 -I research/tools/ghidra/callmap.py <export> [--roots REGEX]
        [--targets REGEX] [--depth N] [--skip REGEX] [--names TSV] [--tsv]

Edges: the calls in each decomp file's header, and the functions its body
names without calling them (a callback stored in a job or an object, as
`local_c = timed_steps`). From each root (default `^builtin_`) it walks
the edges to --depth (default 4) and lists every target it reaches
(default: names matching sched|queue|job|wait_|timed_|frame|step_|cb_),
with the shortest path. --tsv prints draft `(scheduler)` rows for a
findings table, marked inferred (a path in the call graph is not yet what
the code does at run time). --names: a names.tsv newer than the export
(its names win; no rerun of Ghidra needed).
"""
import argparse
import collections
import os
import re
import sys


def load(export):
    names, edges = {}, collections.defaultdict(set)
    d = os.path.join(export, "decomp")
    files = sorted(os.listdir(d))
    for f in files:
        a, n = f[:-2].split("_", 1)
        names[a] = n
    by_name = {n: a for a, n in names.items()}
    ident = re.compile(r"\b(FUN_[0-9a-f]{8}|[A-Za-z_]\w*)\b")
    for f in files:
        a = f[:-2].split("_", 1)[0]
        text = open(os.path.join(d, f), encoding="utf-8", errors="replace").read()
        head, _, body = text.partition("\n\n")
        for line in head.splitlines():
            if line.startswith("// calls:"):
                edges[a] |= {c.split()[0] for c in line[9:].split(",") if c.strip() not in ("", "-") and not c.strip().startswith("EXTERNAL")}
        for m in ident.findall(body):  # pointers to functions the body names
            t = by_name.get(m) or (m[4:] if m.startswith("FUN_") and m[4:] in names else None)
            if t and t != a:
                edges[a].add(t)
    return names, edges


def walk(edges, root, depth, skip=frozenset()):
    """shortest path to every function within depth"""
    path = {root: [root]}
    q = collections.deque([root])
    while q:
        a = q.popleft()
        if len(path[a]) > depth:
            continue
        for b in sorted(edges.get(a, ())):
            if b not in path and b not in skip:
                path[b] = path[a] + [b]
                q.append(b)
    return path


def self_test():
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        os.mkdir(os.path.join(d, "decomp"))
        for name, head, body in [("00000010_builtin_A", "// calls: 00000020 queue_b, EXTERNAL:00000001 K::Sleep", "queue_b();"),
                                 ("00000020_queue_b", "// calls: -", "job.cb = FUN_00000030;"),
                                 ("00000030_FUN_00000030", "// calls: 00000010 builtin_A", "builtin_A();")]:
            open(os.path.join(d, "decomp", name + ".c"), "w").write(f"// {name}\n{head}\n\n{body}\n")
        names, edges = load(d)
        assert edges["00000010"] == {"00000020"} and edges["00000020"] == {"00000030"}, dict(edges)
        assert walk(edges, "00000010", 4)["00000030"] == ["00000010", "00000020", "00000030"]
        assert "00000030" not in walk(edges, "00000010", 1)
        assert walk(edges, "00000010", 4, {"00000020"}).keys() == {"00000010"}
    print("callmap self-test: ok")


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("export")
    p.add_argument("--roots", default=r"^builtin_")
    p.add_argument("--targets", default=r"sched|queue|job|wait_|timed_|frame|step_|cb_")
    p.add_argument("--depth", type=int, default=4)
    p.add_argument("--skip", default=r"dispatch|evaluate", help="functions not walked through (the interpreter)")
    p.add_argument("--names", help="names.tsv to apply over the export's names")
    p.add_argument("--tsv", action="store_true")
    a = p.parse_args()
    names, edges = load(a.export)
    if a.names:
        for line in open(a.names, encoding="utf-8"):
            f = line.rstrip("\n").split("\t")
            if len(f) >= 2 and not line.startswith("#"):
                names[f[0].lower()] = f[1]
    roots, targets = re.compile(a.roots), re.compile(a.targets)
    skip = {x for x in names if re.search(a.skip, names[x])}
    for r in sorted(x for x in names if roots.search(names[x])):
        hits = {t: pa for t, pa in walk(edges, r, a.depth, skip).items() if t != r and targets.search(names.get(t, ""))}
        if not hits:
            continue
        bi = names[r].removeprefix("builtin_")
        if a.tsv:
            via = ", ".join(f"{t} {names[t]}" for t in sorted(hits, key=lambda t: (len(hits[t]), t)))
            print(f"(scheduler)\t{bi}\tinferred\t{r} → {via}\treaches these in the call graph (callmap.py, depth {a.depth})\t")
        else:
            print(f"{r} {names[r]}")
            for t in sorted(hits, key=lambda t: (len(hits[t]), t)):
                print(f"    {t} {names[t]:<28} via {' > '.join(hits[t][1:-1]) or '-'}")


if __name__ == "__main__":
    main()
