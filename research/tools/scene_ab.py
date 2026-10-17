#!/usr/bin/env python3
"""A/B frames of a scene, and save compatibility between two engines.

usage:
  scene_ab.py frames GAME_DIR OUT --engine A [--engine-b B]
              [--script S] [--script-b S2 | --drop-line N]
              [--goto LABEL [--vars F]] [--pages 3] [-- engine options]
  scene_ab.py saves GAME_DIR OUT --engine A --engine-b B
              [--script S] [--goto LABEL [--vars F]] [--pages 3]
  scene_ab.py selftest

GAME_DIR is a full build (out/<game>-cons), only read: each side runs in
a scratch folder under OUT made by scratch.sh (links to the archives,
font and movies; its own 0.txt, renamed engine and --save-dir).

frames: plays --pages pages from --goto (or the start) under side a
(engine A, script S) and side b (engine B, or script S2, or S with line
N, counted from 1, turned into a `;` comment so the other lines keep
their numbers), with a picture of every page (--shot-every 1) plus
whatever shot options follow `--` (--shot-effects 25,50,75,
--shot-at '*label:line', --shot-after ms). Prints `compare -metric AE
-fuzz 2%` for each picture both sides took; exit 1 when any differs.

saves: engine A plays --pages pages and saves there (slot 900); then A
and B each load that save and the loaded state is compared: the dump
taken right after the load, the first page shown, its picture, the exit
code and stderr. A loading its own save is the reference; every line
printed is where B differs from it. Exit 1 when anything differs.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys

TOOLS = os.path.dirname(os.path.abspath(__file__))
SKIP = {"ms", "time", "shot", "dir"}  # differ between any two runs


def scratch(game, d, script, engine):
    subprocess.run([os.path.join(TOOLS, "scratch.sh"), game, d, script, engine],
                   check=True, stdout=subprocess.DEVNULL)


def run(d, args, timeout):
    env = dict(os.environ, TIMEOUT=str(timeout))
    with open(os.path.join(d, "run.log"), "w") as log:
        return subprocess.run([os.path.join(d, "run.sh"), "--trace", "t.jsonl", "--seed", "1", *args],
                              stdout=log, stderr=subprocess.STDOUT, env=env).returncode


def records(d):
    try:
        with open(os.path.join(d, "t.jsonl"), encoding="utf-8") as f:
            return [json.loads(line) for line in f if line.strip()]
    except FileNotFoundError:
        return []


def ae(a, b, fuzz):
    """Pixels that differ by more than fuzz; None when compare fails (sizes differ)."""
    r = subprocess.run(["compare", "-metric", "AE", "-fuzz", fuzz, a, b, "null:"],
                       capture_output=True, text=True)
    try:
        return int(float(r.stderr.split()[0]))
    except (ValueError, IndexError):
        return None


def compare_shots(da, db, fuzz):
    sa = set(os.listdir(os.path.join(da, "shots"))) if os.path.isdir(os.path.join(da, "shots")) else set()
    sb = set(os.listdir(os.path.join(db, "shots"))) if os.path.isdir(os.path.join(db, "shots")) else set()
    bad = 0
    for name in sorted(sa & sb):
        n = ae(os.path.join(da, "shots", name), os.path.join(db, "shots", name), fuzz)
        print(f"{name}\t{'size differs' if n is None else n}")
        bad += n != 0
    for side, only in (("a", sa - sb), ("b", sb - sa)):
        if only:
            print(f"only in {side}: {' '.join(sorted(only))}")
            bad += 1
    return bad


def flatten(v, path="", out=None):
    """{"sp": [{"n": 3, "x": 1}]} -> {"sp[n=3].x": 1}: lists of records keyed by their "n"."""
    out = {} if out is None else out
    if isinstance(v, dict):
        for k, x in v.items():
            if not (path == "" and k in SKIP):
                flatten(x, f"{path}.{k}" if path else k, out)
    elif isinstance(v, list) and v and all(isinstance(x, dict) and "n" in x for x in v):
        for x in v:
            flatten(x, f"{path}[n={x['n']}]", out)
    elif isinstance(v, list):
        for i, x in enumerate(v):
            flatten(x, f"{path}[{i}]", out)
    else:
        out[path] = v
    return out


def diff(a, b):
    fa, fb = flatten(a or {}), flatten(b or {})
    return [(k, fa.get(k), fb.get(k)) for k in sorted(fa.keys() | fb.keys()) if fa.get(k) != fb.get(k)]


def frames(o, extra):
    if not (o.engine_b or o.script_b or o.drop_line):
        sys.exit("frames: give --engine-b, --script-b or --drop-line")
    script_a = o.script or os.path.join(o.game, "0.txt")
    script_b = o.script_b or script_a
    if o.drop_line:
        with open(script_a, "rb") as f:
            lines = f.read().split(b"\n")
        print(f"b drops line {o.drop_line}: {lines[o.drop_line - 1].decode('utf-8', 'replace').strip()}")
        lines[o.drop_line - 1] = b";" + (b"\r" if lines[o.drop_line - 1].endswith(b"\r") else b"")
        os.makedirs(o.out, exist_ok=True)
        script_b = os.path.join(o.out, "b.txt")
        with open(script_b, "wb") as f:
            f.write(b"\n".join(lines))
    # shortcut: pictures only per page plus the shot options after `--`;
    # when the trace's timed --shot-at (TOOLS item 3) lands, add an --at option passing it on.
    args = ["--auto-play", str(o.pages), "--shot-every", "1", *start(o), *extra]
    rc = {}
    for side, script, engine in (("a", script_a, o.engine), ("b", script_b, o.engine_b or o.engine)):
        d = os.path.join(o.out, side)
        shutil.rmtree(os.path.join(d, "shots"), ignore_errors=True)
        scratch(o.game, d, script, engine)
        rc[side] = run(d, args, o.timeout)
    print(f"exit a {rc['a']}, b {rc['b']} (logs: {o.out}/a/run.log, b/run.log)")
    bad = compare_shots(os.path.join(o.out, "a"), os.path.join(o.out, "b"), o.fuzz)
    return 1 if bad or rc["a"] != rc["b"] else 0


def start(o):
    return (["--goto", o.goto] if o.goto else []) + (["--vars", os.path.abspath(o.vars)] if o.vars else [])


def saves(o):
    if not o.engine_b:
        sys.exit("saves: give --engine-b")
    script = o.script or os.path.join(o.game, "0.txt")
    d = os.path.join(o.out, "save")
    scratch(o.game, d, script, o.engine)
    shutil.rmtree(os.path.join(d, "save"), ignore_errors=True)
    run(d, ["--auto-play", str(o.pages), "--checkpoint-every", str(o.pages), *start(o)], o.timeout)
    cp = [r for r in records(d) if r.get("t") == "checkpoint"]
    if not cp:
        sys.exit(f"engine A made no save: see {d}/run.log")
    print(f"saved by A at page {cp[0]['pg']}, {cp[0]['at']} (slot {cp[0]['slot']})")
    side = {}
    for name, engine in (("a", o.engine), ("b", o.engine_b)):
        sd = os.path.join(o.out, name)
        scratch(o.game, sd, script, engine)
        shutil.rmtree(os.path.join(sd, "save"))
        shutil.rmtree(os.path.join(sd, "shots"), ignore_errors=True)
        shutil.copytree(os.path.join(d, "save"), os.path.join(sd, "save"),
                        ignore=shutil.ignore_patterns("stdout.txt", "stderr.txt"))
        rc = run(sd, ["--load", str(cp[0]["slot"]), "--auto-play", "1", "--shot-after", "0", "--shot-every", "1"],
                 o.timeout)
        recs = records(sd)
        with open(os.path.join(sd, "run.log"), errors="replace") as f:
            log = {line for line in f if not line.startswith("CONscripter ")}  # the version banner
        side[name] = (rc, next((r for r in recs if r.get("t") == "dump"), None),
                      next((r for r in recs if r.get("t") == "page"), None), log)
    (rca, dump_a, page_a, log_a), (rcb, dump_b, page_b, log_b) = side["a"], side["b"]
    bad = 0
    if rca != rcb:
        print(f"exit: a {rca}, b {rcb}")
        bad += 1
    for what, ra, rb in (("dump after load", dump_a, dump_b), ("first page", page_a, page_b)):
        if ra is None or rb is None:
            print(f"{what}: a {'none' if ra is None else 'ok'}, b {'none' if rb is None else 'ok'}")
            bad += 1
            continue
        for k, va, vb in diff(ra, rb)[:o.max]:
            print(f"{what} {k}: a {json.dumps(va, ensure_ascii=False)}, b {json.dumps(vb, ensure_ascii=False)}")
            bad += 1
    for line in sorted(log_b - log_a)[:o.max]:
        print(f"only b's log: {line.rstrip()}")
    bad += compare_shots(os.path.join(o.out, "a"), os.path.join(o.out, "b"), o.fuzz) and 1
    print("same" if not bad else f"{bad} difference(s); runs in {o.out}/a, {o.out}/b")
    return 1 if bad else 0


def selftest():
    a = {"t": "page", "ms": 5, "vars": {"%1": 2}, "sp": [{"n": 3, "x": 1}, {"n": 4, "x": 0}]}
    b = {"t": "page", "ms": 9, "vars": {"%1": 2, "%2": 1}, "sp": [{"n": 4, "x": 0}, {"n": 3, "x": 2}]}
    assert flatten(a) == {"t": "page", "vars.%1": 2, "sp[n=3].n": 3, "sp[n=3].x": 1, "sp[n=4].n": 4, "sp[n=4].x": 0}
    assert diff(a, b) == [("sp[n=3].x", 1, 2), ("vars.%2", None, 1)], diff(a, b)
    assert diff(a, a) == []
    print("ok")


def main():
    argv = sys.argv[1:]
    extra = argv[argv.index("--") + 1:] if "--" in argv else []
    argv = argv[:argv.index("--")] if "--" in argv else argv
    if argv[:1] == ["selftest"]:
        return selftest()
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("mode", choices=["frames", "saves"])
    p.add_argument("game")
    p.add_argument("out")
    p.add_argument("--engine", required=True)
    p.add_argument("--engine-b")
    p.add_argument("--script")
    p.add_argument("--script-b")
    p.add_argument("--drop-line", type=int)
    p.add_argument("--goto")
    p.add_argument("--vars")
    p.add_argument("--pages", type=int, default=3)
    p.add_argument("--fuzz", default="2%")
    p.add_argument("--timeout", type=int, default=600)
    p.add_argument("--max", type=int, default=40, help="differences printed per record")
    o = p.parse_args(argv)
    o.game, o.out = os.path.abspath(o.game), os.path.abspath(o.out)
    o.engine = os.path.abspath(o.engine)
    o.engine_b = o.engine_b and os.path.abspath(o.engine_b)
    return frames(o, extra) if o.mode == "frames" else saves(o)


if __name__ == "__main__":
    sys.exit(main())
