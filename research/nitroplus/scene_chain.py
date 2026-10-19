#!/usr/bin/env python3
"""Run scene_saves.py's save chain to the end: ntrace.sh in segments, a new
one from the next scene whenever the original stalls (no dump of a scene
in the manifest for --stall seconds: Django's wine hang on timed actions,
a scene whose Load isn't reached). Run it under research/tools/headless-run.sh
(and capped.sh); ntrace.sh then runs in its display.

    research/tools/headless-run.sh research/nitroplus/scene_chain.py <manifest> <patch dir> <out dir> \\
        [--stall S] -- <ntrace.sh options: --patch, --saves, --clicks, --scene-at ...>

Each segment k writes <out dir>/seg<k>/ (its own base dump) and
<out dir>/seg<k>.jsonl; <out dir>/index.tsv lists every scene's dump
(scene, dump prefix relative to <out dir>). The boot file in <patch dir>
gets `Load(<slot>)` for each segment's first scene. GAME, EXE, NTRACE_HOME
as ntrace.sh takes them.
"""
import json
import os
import re
import subprocess
import sys
import time

here = os.path.dirname(os.path.abspath(__file__))


def scenes(trace, want, seg):
    """scene -> dump prefix of the manifest's scenes a segment's trace dumped (the first dump of each)"""
    out = {}
    for l in open(trace, errors="replace") if os.path.exists(trace) else ():
        if '"t":"dump"' in l:
            r = json.loads(l)
            if r["scene"] in want:
                out.setdefault(r["scene"], f"{seg}/{r['label']}")
    return out


def main():
    a = sys.argv[1:]
    cut = a.index("--")
    pos, rest = a[:cut], a[cut + 1:]
    stall = 120
    if "--stall" in pos:
        i = pos.index("--stall")
        stall = int(pos[i + 1])
        del pos[i:i + 2]
    manifest, patch, out = pos
    slots = [(int(l.split("\t")[0]), l.split("\t")[1].strip()) for l in open(manifest)]
    want = {s for _, s in slots}
    boot = next(os.path.join(patch, f) for f in ("0_boot.nss", "boot.nss") if os.path.exists(os.path.join(patch, f)))
    home = os.environ.get("NTRACE_HOME", os.path.join(here, "..", "..", "out", "ntrace"))
    live = os.path.join(home, "prefix", "drive_c", "ntrace", "trace.jsonl")
    os.makedirs(out, exist_ok=True)
    done, skipped, seg = {}, set(), 0
    while os.path.exists(os.path.join(out, f"seg{seg + 1}.jsonl")):  # a run stopped before: go on from it
        seg += 1
        done.update(scenes(os.path.join(out, f"seg{seg}.jsonl"), want, f"seg{seg}"))
    while True:
        nxt = next(((k, s) for k, s in slots if s not in done and s not in skipped), None)
        if not nxt:
            break
        seg += 1
        b = open(boot, "rb").read()
        open(boot, "wb").write(re.sub(rb"Load\(\d+\);", f"Load({nxt[0]});".encode(), b, count=1))
        name = f"seg{seg}"
        cmd = [os.path.join(here, "tracedll", "ntrace.sh"), "-o", os.path.join(out, name + ".jsonl"),
               "--scene-dumps", os.path.join(out, name), "--pages", "0", "--seconds", "86400"] + rest
        p = subprocess.Popen(cmd, stdout=open(os.path.join(out, name + ".log"), "w"), stderr=subprocess.STDOUT)
        last, seen, t0 = time.time(), 0, time.time()
        while p.poll() is None:
            time.sleep(5)
            if os.path.exists(live) and os.path.getmtime(live) >= t0:  # new scenes only: a scene seen again is no progress
                n = len(set(scenes(live, want, "")) - set(done))
                if n > seen:
                    seen, last = n, time.time()
            if time.time() - last > stall or seen + len(done) >= len(want):
                subprocess.run(["wineserver", "-k"], env=dict(os.environ, WINEPREFIX=os.path.join(home, "prefix")))
                p.wait()
        got = {k: v for k, v in scenes(os.path.join(out, name + ".jsonl"), want, name).items() if k not in done}
        done.update(got)
        new = len(got)
        if not new:
            skipped.add(nxt[1])  # it stalls before its own dump: go on after it
        print(f"{name}: from slot {nxt[0]} ({nxt[1]}), {new} scenes, {len(done)}/{len(want)} done, "
              f"{len(skipped)} skipped, {time.time() - t0:.0f} s", flush=True)
    with open(os.path.join(out, "index.tsv"), "w") as f:
        for _, s in slots:
            if s in done:
                f.write(f"{s}\t{done[s]}\n")
    print(f"skipped: {' '.join(sorted(skipped)) or 'none'}")


if __name__ == "__main__":
    main()
