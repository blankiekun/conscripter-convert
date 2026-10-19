#!/usr/bin/env python3
"""Motions of the original, frame by frame, against our engine's.

    python3 -I research/tools/frame_diff.py ORIG.jsonl OURS.jsonl --map MAP [--tol-px N] [--tol-ms N] [--out F]
    python3 -I research/tools/frame_diff.py --self-test

ORIG: ntrace.sh --frames' trace ("obj" records: an object's x, y, alpha
each frame it changed, the call that made it on its first record).
OURS: CONscripter's --trace of a --real-time --auto-play run, whose
"sample" records give each moving sprite every few frames (engine/DEBUG.md).
MAP: the converter's --source-map. Pages pair by the instruction that shows
them (in order, any distance ahead: a run may start earlier). Between two
paired pages, each object the original moves (two records differing in x,
y or alpha) is a motion, matched to our sprite showing the same picture
(research/tools/scene_check.py's key). Kinds: missing (ours doesn't move
it), extra (ours moves a picture the original doesn't), copy (a second
sprite of a picture both move, as our cross-fades between zooms make;
not a difference), end (where it
ends, more than --tol-px off, default 4), duration (more than --tol-ms off,
default 50), path (the largest gap at the same fraction of the motion,
over --tol-px; positions as top-left corners, alpha left out of it). For
our pre-zoomed copies (`system/zoom/…`), whose box corner isn't the
object's position, the motion is compared from where each starts.
"""
import argparse
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scene_check import key, zoomed  # noqa: E402
import trace_diff as td  # noqa: E402

WINDOW = 30000  # ms after a page that its motions are looked for in


def picture(call):
    q = re.findall(r'"([^"]*)"', call or "")
    m = re.match(r'CreateColor\("[^"]*",[^,]*,[^,]*,[^,]*,\s*(\d+)\s*,\s*(\d+)\s*,\s*"?(?:0x)?([0-9A-Fa-f]{6})', call or "")
    if m:
        return f"color:{m.group(3).lower()}:{m.group(1)}x{m.group(2)}"
    return q[-1] if (call or "").startswith("Create") and len(q) > 1 else ""


def orig_side(path):
    pages, objs, pic = [], [], {}
    for line in open(path, errors="replace"):
        r = json.loads(line)
        if r["t"] == "page":
            pages.append({"at": r["at"], "ms": r["ms"]})
        elif r["t"] == "obj" and "gone" not in r:
            if "call" in r:
                pic[r["n"]] = picture(r["call"])
            objs.append({"ms": r["ms"], "n": r["n"], "pic": pic.get(r["n"], ""), "x": r["x"], "y": r["y"], "a": r["a"]})
    return pages, objs


def ours_side(path, m):
    pages, samples, files = [], [], {}
    for line in open(path, errors="replace"):
        r = json.loads(line)
        for s in r.get("sp") or []:
            files[s["n"]] = s.get("file", "")
        if r["t"] == "page":
            pages.append({"at": td.lookup(m, td.line_of(r.get("at"))), "ms": r["ms"]})
        elif r["t"] == "sample" and "n" in r:
            samples.append({"ms": r["ms"], "n": r["n"], "pic": files.get(r["n"], ""), "x": r["x"] - r.get("w", 0) // 2,
                            "y": r["y"] - r.get("h", 0) // 2, "a": r.get("alpha", 256) * 255 // 256})
    return pages, samples


def pair(orig, ours):
    out, j = [], 0
    for i, o in enumerate(orig):
        k = next((k for k in range(j, len(ours)) if ours[k]["at"] == o["at"]), None)
        if k is not None:
            out.append((i, k))
            j = k + 1
    return out


def motions(recs, lo, hi, by):
    """moving things between lo and hi: by -> [records], two or more that differ"""
    out = {}
    for r in recs:
        if lo <= r["ms"] < hi and r["pic"]:
            out.setdefault(r[by], []).append(r)
    return {k: v for k, v in out.items() if len({(r["x"], r["y"], r["a"]) for r in v}) > 1}


def at_fraction(seq, f):
    t = seq[0]["ms"] + f * (seq[-1]["ms"] - seq[0]["ms"])
    best = min(seq, key=lambda r: abs(r["ms"] - t))
    return best["x"], best["y"]


def compare(o_pages, o_objs, u_pages, u_samples, tol_px=4, tol_ms=50):
    finds = []
    pairs = pair(o_pages, u_pages)
    for (i, k), (i2, k2) in zip(pairs, pairs[1:] + [(len(o_pages), len(u_pages))]):
        # to the next paired page, at most WINDOW ms (the last page's runs on to the end of the trace)
        olo = o_pages[i]["ms"]
        ohi = min(o_pages[i2]["ms"] if i2 < len(o_pages) else olo + WINDOW, olo + WINDOW)
        ulo = u_pages[k]["ms"]
        uhi = min(u_pages[k2]["ms"] if k2 < len(u_pages) else ulo + WINDOW, ulo + WINDOW)
        om = motions(o_objs, olo, ohi, "n")
        um = motions(u_samples, ulo, uhi, "n")
        left = dict(um)
        page = o_pages[i]["at"]
        for name, seq in om.items():
            kk = key(seq[0]["pic"])
            cand = [n for n, s in left.items() if key(s[0]["pic"]) == kk]
            if not cand:
                finds.append(("missing", page, f"{name} {seq[0]['pic']} {seq[0]['x']},{seq[0]['y']} -> {seq[-1]['x']},{seq[-1]['y']} over {seq[-1]['ms'] - seq[0]['ms']} ms"))
                continue
            n = min(cand, key=lambda n: abs((left[n][0]["ms"] - ulo) - (seq[0]["ms"] - olo)))
            us = left.pop(n)
            if zoomed(us[0]["pic"]):  # our pre-zoomed copy: its box's corner isn't the object's; compare how far it moved
                dx, dy = us[0]["x"] - seq[0]["x"], us[0]["y"] - seq[0]["y"]
                us = [dict(r, x=r["x"] - dx, y=r["y"] - dy) for r in us]
            if abs(us[-1]["x"] - seq[-1]["x"]) > tol_px or abs(us[-1]["y"] - seq[-1]["y"]) > tol_px:
                finds.append(("end", page, f"{seq[0]['pic']}: original {seq[-1]['x']},{seq[-1]['y']}, ours {us[-1]['x']},{us[-1]['y']}"))
            do, du = seq[-1]["ms"] - seq[0]["ms"], us[-1]["ms"] - us[0]["ms"]
            if abs(do - du) > tol_ms:
                finds.append(("duration", page, f"{seq[0]['pic']}: original {do} ms, ours {du} ms"))
            gap = max(max(abs(a - b) for a, b in zip(at_fraction(seq, f / 10), at_fraction(us, f / 10))) for f in range(11))
            if gap > tol_px:
                finds.append(("path", page, f"{seq[0]['pic']}: {gap} px apart at the same fraction"))
        shown = {key(q[0]["pic"]) for q in om.values()}  # a second sprite of a picture the original moves: our cross-fade copy
        finds += [("copy" if key(s[0]["pic"]) in shown else "extra", page, f"{s[0]['pic']} {s[0]['x']},{s[0]['y']} -> {s[-1]['x']},{s[-1]['y']}")
                  for s in left.values()]
    return pairs, finds


def self_test():
    op = [{"at": "a.nss:1", "ms": 0}, {"at": "a.nss:9", "ms": 1000}]
    up = [{"at": "x.nss:5", "ms": 0}, {"at": "a.nss:1", "ms": 100}, {"at": "a.nss:9", "ms": 1100}]
    oo = [{"ms": t, "n": "bg", "pic": "cg/bg/a.jpg", "x": t // 10, "y": 0, "a": 255} for t in range(0, 500, 16)]
    oo += [{"ms": t, "n": "st", "pic": "cg/st/b.png", "x": 0, "y": t // 10, "a": 255} for t in range(0, 300, 16)]
    us = [{"ms": 100 + t, "n": 5, "pic": "cg\\bg\\A.png", "x": t // 10, "y": 0, "a": 255} for t in range(0, 500, 64)]
    us += [{"ms": 100 + t, "n": 9, "pic": "cg/ef/c.png", "x": t, "y": 0, "a": 255} for t in range(0, 200, 64)]
    pairs, f = compare(op, oo, up, us)
    assert pairs == [(0, 1), (1, 2)], pairs
    assert sorted(k for k, _, _ in f) == ["end", "extra", "missing", "path"], f  # bg ends at 49 vs 44 (448 ms vs 496: within 50)
    assert picture('CreateColor("k",1,0,0,800,600,0x00FF00);') == "color:00ff00:800x600"
    print("frame_diff self-test: ok")


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("orig")
    p.add_argument("ours")
    p.add_argument("--map", required=True)
    p.add_argument("--tol-px", type=int, default=4)
    p.add_argument("--tol-ms", type=int, default=50)
    p.add_argument("--out")
    a = p.parse_args()
    op, oo = orig_side(a.orig)
    up, us = ours_side(a.ours, td.read_map(a.map))
    pairs, finds = compare(op, oo, up, us, a.tol_px, a.tol_ms)
    counts = {}
    for k, _, _ in finds:
        counts[k] = counts.get(k, 0) + 1
    print(f"{len(op)} original pages, {len(up)} ours, {len(pairs)} paired; " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    byp = {}
    for k, pg, d in finds:
        byp.setdefault(pg, []).append(f"{k} {d}")
    for pg, ds in sorted(byp.items(), key=lambda x: -len(x[1]))[:10]:
        print(f"{pg}: {len(ds)}: " + "; ".join(ds)[:300])
    if a.out:
        with open(a.out, "w") as f:
            for k, pg, d in finds:
                f.write(json.dumps({"kind": k, "page": pg, "detail": d}, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
