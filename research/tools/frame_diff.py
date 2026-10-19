#!/usr/bin/env python3
"""Motions of the original, frame by frame, against our engine's.

    python3 -I research/tools/frame_diff.py ORIG.jsonl[,MORE.jsonl...] OURS.jsonl --map MAP [--tol-px N] [--tol-ms N]
        [--groups N] [--out F] [--alias NAME=FILE ...] [--ignore REGEX]
    python3 -I research/tools/frame_diff.py --self-test

ORIG: ntrace.sh --frames' trace ("obj" records: an object's x, y, alpha
each frame it changed, the call that made it on its first record).
OURS: CONscripter's --trace of a --real-time --auto-play run, whose
"sample" records give each moving sprite every few frames (engine/DEBUG.md).
For xtrace's records (最果てのイマ: an object id and a picture slot,
no file name), a motion matches the nearest of ours that starts within
64 px instead.
MAP: the converter's --source-map. Pages pair by the instruction that shows
them (in order, any distance ahead: a run may start earlier). Between two
paired pages (one window), each object the original moves (two records differing in x,
y or alpha) is a motion, matched to our sprite showing the same picture in that window
or the one before or after it (the two runs' page times differ)
(research/tools/scene_check.py's key). Kinds: missing (ours doesn't move
it), extra (ours moves a picture the original doesn't), copy (a second
sprite of a picture both move, as our cross-fades between zooms make;
not a difference), end (where it
ends, more than --tol-px off, default 4), duration (more than --tol-ms off,
default 50), path (the largest gap at the same fraction of the motion,
over --tol-px; positions as top-left corners, alpha left out of it), zoom
(where it ends, more than 2 % off; the original's as drawn, from ntrace's
texture record; not for our pre-zoomed copies, whose zoom is in the picture). For
our pre-zoomed copies (`system/zoom/…`), whose box corner isn't the
object's position, the motion is compared from where each starts.
Not differences either, by how each side draws: screen (a whole-screen colour
box the original fades, CreateColor 800x600 at 0,0, where ours runs a screen
transition: an `effect` record in the window or the next/previous one) and
camera (ours moves a picture that is a child in the original, `@A/B/name`, while
the original runs MoveCamera in that window, the next/previous one or 20 s before: the
original's objects keep their own place as the camera moves).
--alias NAME=FILE: an `@NAME` picture is FILE (LoadImage, which the trace
doesn't record; 続・殺戮のジャンゴ's face-window frames: LoadFrame=cg/sys/windowframe.png).
--ignore REGEX: pictures (as written on either side) left out, for what our
engine draws another way (Django's hold prompt, cells of an `mpeghold` sprite).
"""
import argparse
import json
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scene_check import key, zoomed  # noqa: E402
import trace_diff as td  # noqa: E402

WINDOW = 30000  # ms after a page that its motions are looked for in
TOL_ZOOM = 2  # percent
CAMERA_MS = 20000  # the longest MoveCamera
POS_PX = 64  # a nameless motion (xtrace's) matches ours starting this close


def picture(call, alias=None):
    q = re.findall(r'"([^"]*)"', call or "")
    if q and q[-1].startswith("@") and q[-1][1:] in (alias or {}):
        q[-1] = alias[q[-1][1:]]
    m = re.match(r'CreateColor\("[^"]*",[^,]*,[^,]*,[^,]*,\s*(\d+)\s*,\s*(\d+)\s*,\s*"?(?:0x)?([0-9A-Fa-f]{6})', call or "")
    if m:
        return f"color:{m.group(3).lower()}:{m.group(1)}x{m.group(2)}"
    return q[-1] if (call or "").startswith("Create") and len(q) > 1 else ""


def orig_side(path, alias=None):
    """pages, objects, MoveCamera times, and the keys of pictures that are some object's child"""
    pages, objs, pic, cams, kids = [], [], {}, [], set()
    for line in open(path, errors="replace"):
        r = json.loads(line)
        if r["t"] == "cmd" and r.get("op") == "MoveCamera":
            cams.append(r["ms"])
        elif r["t"] == "page":
            pages.append({"at": r["at"], "ms": r["ms"]})  # already in source_key's form ("4:19", "x.nss:12")
        elif r["t"] == "obj" and "gone" not in r:
            if "id" in r:  # xtrace's (Xuse): an object id and a picture slot, no file name; scale in percent
                objs.append({"ms": r["ms"], "n": f"obj{r['id']}", "pic": f"slot:{r['pic']}", "x": r["x"], "y": r["y"],
                             "a": r["a"] * 255 // 256, "z": r.get("sx", 100)})
                continue
            if "call" in r:
                pic[r["n"]] = picture(r["call"], alias)
                if "/" in (re.findall(r'"([^"]*)"', r["call"]) or [""])[0]:
                    kids.add(key(pic[r["n"]]))
            # turned about y (the face windows' frames): drawn |cos| as wide
            objs.append({"ms": r["ms"], "n": r["n"], "pic": pic.get(r["n"], ""), "x": r["x"], "y": r["y"], "a": r["a"],
                         "z": r.get("zx", 1000) / 10 * abs(math.cos(math.radians(r.get("ry", 0)))), "ry": r.get("ry", 0)})
    return pages, objs, cams, kids


def ours_side(path, m):
    pages, samples, files, effects, keys = [], [], {}, [], {}
    for line in open(path, errors="replace"):
        r = json.loads(line)
        for s in r.get("sp") or []:
            files[s["n"]] = s.get("file", "")
        if r["t"] == "motion" and "keys" in r and "in" not in r:
            keys[r["n"]] = r["keys"][0]
        if r["t"] == "ev" and r.get("cmd") == "spmotion":  # where it starts: samples come only every few frames
            for s in r.get("sp") or []:
                k = keys.pop(s["n"], None)
                if k:
                    samples.append({"ms": r["ms"], "n": s["n"], "pic": files[s["n"]], "x": s["x"] + k[1], "y": s["y"] + k[2],
                                    "a": k[6] * 255 // 256, "z": (s.get("zoom") or [100])[0] * k[3] / 100,
                                    "cx": s["x"] + s["w"] // 2 + k[1]})
        if r["t"] == "effect" and r.get("len", 0) > 0:
            effects.append(r["ms"] - r.get("ran", 0))
        if r["t"] == "page":
            pages.append({"at": td.lookup(m, td.line_of(r.get("at"))), "ms": r["ms"]})
        elif r["t"] == "sample" and "n" in r:
            samples.append({"ms": r["ms"], "n": r["n"], "pic": files.get(r["n"], ""), "x": r["x"] - r.get("w", 0) // 2,
                            "cx": r["x"],
                            "y": r["y"] - r.get("h", 0) // 2, "a": r.get("alpha", 256) * 255 // 256,
                            "z": (r.get("zoom") or [100])[0]})
    return pages, samples, effects


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
    for seq in out.values():  # hidden at once (a reset to 0, not a fade's last step): not part of what was seen
        if len(seq) > 1 and seq[-1]["a"] == 0 and seq[-2]["a"] > 128:
            seq.pop()
    return {k: v for k, v in out.items() if len({(r["x"], r["y"], r["a"], r.get("z")) for r in v}) > 1}


def at_fraction(seq, f):
    t = seq[0]["ms"] + f * (seq[-1]["ms"] - seq[0]["ms"])
    best = min(seq, key=lambda r: abs(r["ms"] - t))
    return best["x"], best["y"]


def compare(o_pages, o_objs, u_pages, u_samples, tol_px=4, tol_ms=50, u_effects=(), o_cams=(), o_kids=()):
    """motions per paired page window, matched by picture within the same
    window or the next/previous one (page timing differs between the two)"""
    finds, om_all, um_all, fx_w, cam_w = [], [], [], set(), set()
    pairs = pair(o_pages, u_pages)
    for w, ((i, k), (i2, k2)) in enumerate(zip(pairs, pairs[1:] + [(len(o_pages), len(u_pages))])):
        # to the next paired page, at most WINDOW ms (the last page's runs on to the end of the trace)
        olo = o_pages[i]["ms"]
        ohi = min(o_pages[i2]["ms"] if i2 < len(o_pages) else olo + WINDOW, olo + WINDOW)
        ulo = u_pages[k]["ms"]
        uhi = min(u_pages[k2]["ms"] if k2 < len(u_pages) else ulo + WINDOW, ulo + WINDOW)
        page = o_pages[i]["at"]
        om_all += [(w, page, name, seq) for name, seq in motions(o_objs, olo, ohi, "n").items()]
        um_all += [(w, page, n, seq) for n, seq in motions(u_samples, ulo, uhi, "n").items()]
        if any(ulo <= t < uhi for t in u_effects):
            fx_w.add(w)
        # shortcut: the trace gives no MoveCamera's time; 20 s is Django's longest (dja*.nss), take it from the call when one is longer
        if any(olo - CAMERA_MS <= t < ohi for t in o_cams):
            cam_w.add(w)
    left = list(um_all)
    keys_o = {}
    for w, page, name, seq in om_all:
        keys_o.setdefault(key(seq[0]["pic"]), set()).add(w)
    for w, page, name, seq in om_all:
        kk = key(seq[0]["pic"])
        if kk.startswith("slot:"):  # no file name: the nearest of our motions starting within POS_PX, by where it starts
            cand = [c for c in left if abs(c[0] - w) <= 1 and abs(c[3][0]["x"] - seq[0]["x"]) + abs(c[3][0]["y"] - seq[0]["y"]) <= POS_PX]
            cand.sort(key=lambda c: abs(c[3][0]["x"] - seq[0]["x"]) + abs(c[3][0]["y"] - seq[0]["y"]))
            cand = cand[:1]
        else:
            cand = [c for c in left if key(c[3][0]["pic"]) == kk and abs(c[0] - w) <= 1]
        whole = kk.endswith(":800x600") and kk.startswith("color:") and len({(r["x"], r["y"]) for r in seq}) == 1
        if not cand and whole and any(abs(x - w) <= 1 for x in fx_w):
            finds.append(("screen", page, f"{name} {seq[0]['pic']} over {seq[-1]['ms'] - seq[0]['ms']} ms"))
            continue
        if not cand:
            finds.append(("missing", page, f"{name} {seq[0]['pic']} {seq[0]['x']},{seq[0]['y']} -> {seq[-1]['x']},{seq[-1]['y']} over {seq[-1]['ms'] - seq[0]['ms']} ms"))
            continue
        # (two of one picture, as the face windows' two frames: the one starting nearer)
        c = min(cand, key=lambda c: (abs(c[0] - w), zoomed(c[3][0]["pic"]),
                                     abs(c[3][0]["x"] - seq[0]["x"]) + abs(c[3][0]["y"] - seq[0]["y"])))
        left.remove(c)
        us = c[3]
        if len({r.get("ry") for r in seq}) > 1:  # turning about y: the original's x is the unturned box's; ours by its centre
            us = [dict(r, x=r["cx"] - us[0]["cx"] + seq[0]["x"]) for r in us]
        elif zoomed(us[0]["pic"]):  # our pre-zoomed copy: its box's corner isn't the object's; compare how far it moved
            dx, dy = us[0]["x"] - seq[0]["x"], us[0]["y"] - seq[0]["y"]
            us = [dict(r, x=r["x"] - dx, y=r["y"] - dy) for r in us]
        if abs(us[-1]["x"] - seq[-1]["x"]) > tol_px or abs(us[-1]["y"] - seq[-1]["y"]) > tol_px:
            finds.append(("end", page, f"{seq[0]['pic']}: original {seq[-1]['x']},{seq[-1]['y']}, ours {us[-1]['x']},{us[-1]['y']}"))
        do, du = seq[-1]["ms"] - seq[0]["ms"], us[-1]["ms"] - us[0]["ms"]
        if abs(do - du) > tol_ms:
            finds.append(("duration", page, f"{seq[0]['pic']}: original {do} ms, ours {du} ms"))
        if not zoomed(us[0]["pic"]) and abs(us[-1].get("z", 100) - seq[-1].get("z", 100)) > TOL_ZOOM:
            finds.append(("zoom", page, f"{seq[0]['pic']}: original {seq[-1]['z']:.0f} %, ours {us[-1]['z']:.0f} %"))
        gap = max(max(abs(a - b) for a, b in zip(at_fraction(seq, f / 10), at_fraction(us, f / 10))) for f in range(11))
        if gap > tol_px:
            finds.append(("path", page, f"{seq[0]['pic']}: {gap} px apart at the same fraction"))
    for w, page, n, s in left:  # a second sprite of a picture the original moves there: our cross-fade copy
        near = any(abs(x - w) <= 1 for x in keys_o.get(key(s[0]["pic"]), ()))
        cam = key(s[0]["pic"]) in o_kids and any(abs(x - w) <= 1 for x in cam_w)
        finds.append(("copy" if near else "camera" if cam else "extra", page, f"{s[0]['pic']} {s[0]['x']},{s[0]['y']} -> {s[-1]['x']},{s[-1]['y']}"))
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
    assert picture('CreateTexture("f",1,25,80,"@LoadFrame");', {"LoadFrame": "cg/sys/windowframe.png"}) == "cg/sys/windowframe.png"
    # a whole-screen black the original fades where ours runs an effect; a camera child ours moves while the original's camera moves
    oo = [{"ms": t, "n": "黒板", "pic": "color:000000:800x600", "x": 0, "y": 0, "a": t // 4, "z": 100} for t in range(0, 400, 16)]
    oo += [{"ms": 0, "n": "f", "pic": "cg/sys/f.png", "x": 0, "y": 0, "a": 255, "z": 100}, {"ms": 600, "n": "f", "pic": "cg/sys/f.png", "x": 0, "y": 0, "a": 0, "z": 100}]
    us = [{"ms": 100 + t, "n": 5, "pic": "cg\\bg\\a.png", "x": t // 10, "y": 0, "a": 255} for t in range(0, 500, 64)]
    pairs, f = compare(op, oo, up, us, u_effects=[150], o_cams=[200], o_kids={"cg/bg/a"})
    assert sorted(k for k, _, _ in f) == ["camera", "screen"], f  # f's reset to 0 isn't a motion
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
    p.add_argument("--groups", type=int, default=0, help="the N largest groups of findings by kind and picture")
    p.add_argument("--alias", action="append", default=[], help="NAME=FILE: an @NAME picture is FILE")
    p.add_argument("--ignore", help="pictures left out (a regex, on names with /)")
    a = p.parse_args()
    alias = dict(x.split("=", 1) for x in a.alias)
    kept = (lambda recs: [r for r in recs if not re.search(a.ignore, r["pic"].replace("\\", "/"))]) if a.ignore else (lambda recs: recs)
    up, us, ufx = ours_side(a.ours, td.read_map(a.map))
    us = kept(us)
    op, pairs, finds = [], [], []
    for f in a.orig.split(","):  # several traces (scene_chain.py's segments): each paired with ours on its own
        o_pages, oo, cams, kids = orig_side(f, alias)
        pp, ff = compare(o_pages, kept(oo), up, us, a.tol_px, a.tol_ms, ufx, cams, kids)
        op += o_pages
        pairs += pp
        finds += ff
    counts = {}
    for k, _, _ in finds:
        counts[k] = counts.get(k, 0) + 1
    print(f"{len(op)} original pages, {len(up)} ours, {len(pairs)} paired; " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    if a.groups:  # (kind, picture with its digits as N): what repeats
        grp = {}
        for k, pg, d in finds:
            m = re.search(r"(cg[\\/][^ :,]+|color:[0-9a-f]+:\d+x\d+|system[\\/][^ :,]+|slot:\d+)", d)
            fam = re.sub(r"\d+", "N", m.group(1).replace("\\", "/").lower()) if m else d.split(" ")[0]
            grp.setdefault((k, fam), []).append(pg)
        for (k, fam), pgs in sorted(grp.items(), key=lambda x: -len(x[1]))[:a.groups]:
            print(f"  {len(pgs):5} {k:9} {fam}  e.g. {pgs[0]}")
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
