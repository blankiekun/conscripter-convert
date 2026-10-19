#!/usr/bin/env python3
"""Scene-start check: the original's objects in a scene's memory dump
against our engine's sprites on the same page.

    research/tools/.venv/bin/python -I research/tools/scene_check.py <adapter> <exe> <page dumps> <start dumps>
        <index.tsv> <scratch> [--jobs N] [--out results.jsonl] [--only REGEX]
    python3 -I research/tools/scene_check.py --self-test

<adapter>: a module with objects(exe, dump) -> [{name, pic, x, y, alpha}],
label(scene) -> our label, IGNORE (a regex of object names our engine
draws otherwise); research/nitroplus/scene_objects.py is 装甲悪鬼村正's.
<page dumps>: dumps at each scene's first text block (ntrace.sh
--scene-at page): a folder indexed by its trace's `dump` records in
<folder>.jsonl, or scene_chain.py's index.tsv; several, comma-separated
(a later one wins for a scene); <start dumps> + <index.tsv>: the dumps at each scene's
first statement (the template's leftovers there are "inherited").
<scratch>: research/tools/scratch.sh's folder; each scene runs as
`run.sh --goto <label> --auto-play 1 --trace` and its first page's `sp`
is ours. Objects match by picture (case, `\\`/`/` and our `:c;`-style
prefixes ignored). Per scene: missing (the original shows it, we don't),
extra, position (more than TOL_XY px off), alpha (more than TOL_A off),
inherited (missing but a leftover of the template; not an error),
effect (an engine effect object such as rain, which ours draws without a
sprite; not an error), occluded (missing, but under an opaque full-screen
object above it; not an error), no-label. A full-screen colour object matches our
page's `bg` colour; our pre-zoomed copies match their source picture,
their position not compared; mirrored copies likewise. Prints the count per kind and the 10 worst scenes; --out keeps
every scene's findings.
"""
import importlib
import json
import os
import re
import subprocess
import sys
import tempfile
import unicodedata

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
TOL_XY, TOL_A = 2, 8
NOT_ERRORS = ("inherited", "effect", "occluded", "no-dump")  # counted, but not a difference


def key(pic):
    """a picture's name both sides share: no `:c;` prefix, `/`, NFKC, lower case, no
    extension; our CreateColor pictures as `color:rrggbb:WxH`; our pre-zoomed
    copies (`system/zoom/cg_ev_resize/x_625x625.png`) as their source"""
    # NFKC: the archives' names and the scripts' differ in width (bu宇宙紳士C against bu宇宙紳士ｃ)
    pic = unicodedata.normalize("NFKC", re.sub(r"^:[a-z]+(,[^;]*)?;", "", pic).replace("\\", "/")).lower()
    m = re.match(r"(?:.*/)?color_([0-9a-f]{6})_(\d+x\d+)\.png$", pic)
    if m:
        return f"color:{m.group(1)}:{m.group(2)}"
    z = re.match(r"system/zoom/([^/]+)/(.+)_\d+x\d+\.\w+$", pic) or re.match(r"system/mirror/([^/]+)/(.+)_[xy]+\.\w+$", pic)
    if z:
        pic = z.group(1).replace("_", "/") + "/" + z.group(2)
    return re.sub(r"\.(png|jpg|bmp|webp)$", "", pic)


def covered(o, shown, size):
    """o lies under an opaque full-screen object above it (a colour, or a
    background .jpg placed at or above the corner): unseen, so ours may drop it"""
    def full(c):
        k = key(c["pic"])
        return c["alpha"] == 255 and c.get("x", 0) <= 0 and c.get("y", 0) <= 0 and (
            k == f"color:{k[6:12]}:{size[0]}x{size[1]}" or c["pic"].lower().endswith(".jpg"))
    return any(c is not o and c.get("prio", 0) > o.get("prio", 0) and full(c) for c in shown)


def zoomed(file):
    return "system\\zoom\\" in file or "system/zoom/" in file


BG = {"black": "000000", "white": "ffffff"}


def compare(orig, ours, inherited=(), ignore=None, bg=None, size=(1024, 576)):
    """orig: [{name, pic, x, y, alpha}]; ours: the page's sp list, bg its `bg`
    (a full-screen colour there stands for a colour object); inherited: keys
    shown at the scene's start already. Returns [(kind, detail)]."""
    shown = [o for o in orig if o["alpha"] > 0 and o["pic"] and not (ignore and re.search(ignore, o["name"]))]
    left = [s for s in ours if s.get("alpha", 255) > 0]
    if bg:
        c = BG.get(bg.lower(), bg.lower().lstrip("#"))
        left.append({"file": f"color_{c}_{size[0]}x{size[1]}.png", "x": 0, "y": 0, "alpha": 255, "bg": True})
    out = []
    for o in shown:
        k = key(o["pic"])
        if k.startswith("effect:"):  # an engine effect on our side (rain, ...): not a picture to match
            out.append(("effect", f"{o['name']} {o['pic']}"))
            continue
        m = min((s for s in left if key(s["file"]) == k), key=lambda s: abs(s["x"] - o["x"]) + abs(s["y"] - o["y"]), default=None)
        if m is None:
            kind = "inherited" if k in inherited else "occluded" if covered(o, shown, size) else "missing"
            out.append((kind, f"{o['name']} {o['pic']} at {o['x']},{o['y']}"))
            continue
        left.remove(m)
        if zoomed(m["file"]):  # placed for its zoom: positions don't compare
            pass
        elif abs(m["x"] - o["x"]) > TOL_XY or abs(m["y"] - o["y"]) > TOL_XY:
            out.append(("position", f"{o['pic']}: original {o['x']},{o['y']}, ours {m['x']},{m['y']}"))
        if abs(m.get("alpha", 255) - o["alpha"]) > TOL_A:
            out.append(("alpha", f"{o['pic']}: original {o['alpha']}, ours {m.get('alpha', 255)}"))
    out += [("extra", f"{s['file']} at {s['x']},{s['y']}") for s in left if not s.get("bg")]
    return out


def ours_page(scratch, label):
    with tempfile.TemporaryDirectory() as d:
        t = os.path.join(d, "t.jsonl")
        args = open(os.path.join(scratch, "ons_args")).read().split()
        subprocess.run([os.path.join(scratch, "run.sh"), "--goto", label, "--auto-play", "1", "--trace", t] + args,
                       capture_output=True, env=dict(os.environ, TIMEOUT="60"))
        for line in open(t) if os.path.exists(t) else ():
            if '"t":"page"' in line:
                r = json.loads(line)
                return r.get("sp", []), r.get("bg")
    return None, None


def one(job):
    adapter, exe, page_dump, start_dump, label, scratch, scene = job
    a = importlib.import_module(adapter)
    try:
        orig = a.objects(exe, page_dump)
    except (OSError, ValueError) as e:  # a dump that isn't there (a segment cut short)
        return scene, [("no-dump", f"{page_dump}: {e}")]
    inh = {key(o["pic"]) for o in a.objects(exe, start_dump) if o["alpha"] > 0} if start_dump else set()
    ours, bg = ours_page(scratch, label) if label else (None, None)
    if ours is None:
        return scene, [("no-label", label or scene)]
    return scene, compare(orig, ours, inh, a.IGNORE, bg)


def self_test():
    o = [{"name": "bg", "pic": "cg/bg/a.jpg", "x": 0, "y": 0, "alpha": 255},
         {"name": "st", "pic": "cg/st/b.png", "x": 100, "y": 0, "alpha": 255},
         {"name": "box", "pic": "cg/sys/w.png", "x": 0, "y": 400, "alpha": 255},
         {"name": "old", "pic": "cg/bg/old.jpg", "x": 0, "y": 0, "alpha": 255},
         {"name": "fx", "pic": "cg/ef/c.png", "x": 0, "y": 0, "alpha": 255},
         {"name": "k", "pic": "color:000000:1024x576", "x": 0, "y": 0, "alpha": 0}]
    s = [{"file": ":c;CG\\bg\\A.jpg", "x": 1, "y": 0, "alpha": 255}, {"file": "cg\\st\\b.png", "x": 110, "y": 0, "alpha": 128},
         {"file": "system\\color_000000_1024x576.png", "x": 0, "y": 0, "alpha": 255}]
    got = compare(o, s, {key("cg/bg/old.jpg")}, r"^box")
    kinds = sorted(k for k, _ in got)
    assert kinds == ["alpha", "extra", "inherited", "missing", "position"], got
    assert key("system\\color_ff0000_10x20.png") == "color:ff0000:10x20"
    assert key("cg/bu/bu紳士C.png") == key("cg\\bu\\bu紳士ｃ.png")
    assert key("system\\zoom\\cg_ev_resize\\ev1_a_625x625.png") == key("cg/ev/resize/ev1_a.jpg")
    blk = [{"name": "k", "pic": "color:000000:1024x576", "x": 0, "y": 0, "alpha": 255}]
    assert compare(blk, [], bg="black") == [] and compare(blk, [])[0][0] == "missing"
    print("scene_check self-test: ok")


def main():
    a = sys.argv[1:]
    if a == ["--self-test"]:
        return self_test()
    import argparse
    from sched_harness import pool_map
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    for n in ("adapter", "exe", "page", "start", "index", "scratch"):
        p.add_argument(n)
    p.add_argument("--jobs", type=int)
    p.add_argument("--out")
    p.add_argument("--only", default="")
    o = p.parse_args(a)
    adir, mod = os.path.split(os.path.abspath(o.adapter))
    sys.path.insert(0, adir)
    mod = mod[:-3]
    ad = importlib.import_module(mod)
    start = {l.split("\t")[0]: l.split("\t")[2] for l in open(o.index) if not l.startswith(("#", "scene\t"))}
    pages = []  # (scene, dump prefix)
    for pg in o.page.split(","):
        if pg.endswith(".tsv"):  # scene_chain.py's index: scene, prefix relative to it
            pages += [(l.split("\t")[0], os.path.join(os.path.dirname(pg), l.split("\t")[1].strip())) for l in open(pg)]
        else:
            pages += [(json.loads(l)["scene"], os.path.join(pg, json.loads(l)["label"]))
                      for l in open(pg.rstrip("/") + ".jsonl") if '"t":"dump"' in l]
    jobs = [(mod, o.exe, pre, start.get(sc) and os.path.join(o.start, start[sc]), ad.label(sc), os.path.abspath(o.scratch), sc)
            for sc, pre in dict(pages).items() if re.search(o.only, sc)]
    res = pool_map(one, jobs, o.jobs or None)
    counts, worst = {}, []
    with open(o.out, "w") if o.out else open(os.devnull, "w") as f:
        for sc, finds in res:
            f.write(json.dumps({"scene": sc, "findings": finds}, ensure_ascii=False) + "\n")
            for k, _ in finds:
                counts[k] = counts.get(k, 0) + 1
            worst.append((sum(k not in NOT_ERRORS for k, _ in finds), sc, finds))
    print(f"{len(res)} scenes; " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    for n, sc, finds in sorted(worst, reverse=True)[:10]:
        print(f"{sc}: {n}: " + "; ".join(f"{k} {d}" for k, d in finds if k not in NOT_ERRORS)[:300])


if __name__ == "__main__":
    main()
