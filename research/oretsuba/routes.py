"""Plays the route map's eight paths (docs/games/oretsuba.md, "Story structure and walkthrough")
in the engine, one after another in one save folder so the global clears
carry over, each with the debug trace. --auto-choice untaken is steered by a
--taken file that marks every option but the path's "avoid" (choices the
path doesn't name: the first option); the title takes START and the story
menu 始めから.

    python3 -I research/oretsuba/routes.py out/oretsuba-cons <engine> <outdir> [--only 2,7]

Use a copy of the engine under another name (other agents' `pkill -x
conscripter` would stop it). Each run ends when auto-play gets stuck after
the ending (it replays the route from the story menu): the ending scene
reached is what to look at. Check each trace with cons_trace_check.py.
"""
import argparse
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "engine", "tools"))
import cons_trace_check  # noqa: E402

ROUTES = [
    ("明日香", "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:1,S01_08D4:2,S01_09C0:1", "mg_s01_16a"),
    ("京", "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:2,S01_08D0:3,S01_08D4:1,S01_09C0:2,S01_12B:1,S01_13B:1", "mg_s01_18a"),
    ("美咲", "S01_02C:2,S01_02E:2,S01_03B:1,S01_04E:1,S01_04L:2,S01_08C0:2,S01_08D4:2,S01_09C0:3", "mg_m09_01"),
    ("日和子", "", "mg_s02_16a"),
    ("亜衣", "S02_05A:2,S03_03B:2,S03_03I:2,S03_05C:2,S03C_01D:1", "mg_s03c_20a"),
    ("bad end", "S02_05A:2,S03_03B:2,S03_03I:2,S03_05C:2,S03C_01D:2", "mg_s04_06l"),
    ("鳴", "S02_05A:2", "mg_s03_22a"),
    ("小鳩", "S04_05I:2", "mg_s04_28b"),
]
# The after-stories, runs 9-12: the story menu's option and the last
# scene (run them after the routes, in the same out folder: the menu
# needs an ending, 狩男's a part cleared).
AFTER = [
    ("ある日の林田美咲", 2, "mg_p03_01d"),
    ("ある日の渡来明日香", 3, "mg_p01_01d"),
    ("ある日の山科京", 4, "mg_p02_01e"),
    ("ある日の軽部狩男", 5, "mg_p04_02b"),
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("game")
    ap.add_argument("engine")
    ap.add_argument("out")
    ap.add_argument("--only", help="route numbers (1-8, 9-12 the after-stories), comma-separated; default 1-8")
    args = ap.parse_args()
    game, engine, out = (os.path.abspath(p) for p in (args.game, args.engine, args.out))
    os.makedirs(out, exist_ok=True)
    src = open(os.path.join(game, "0.txt"), "rb").read().decode("utf-8", "replace").split("\n")
    selects, label = {}, None
    for i, line in enumerate(src, 1):
        if line.startswith("*"):
            label = line.strip()
        if line.startswith("select "):
            m = re.match(r"\*mg_([a-z0-9]+_[a-z0-9]+)", label)
            selects[m.group(1).upper() if m else label] = (f"{label}:{i}", len(re.findall(r",\*", line)))
    title = next(f"*conv_title:{i}" for i, line in enumerate(src, 1) if line.startswith("btnwait") and src[i - 5].startswith("*conv_title"))
    # The title's buttons: all but START avoided.
    start_no = next(int(m.group(1)) for line in src if (m := re.match(r"if %9 == (\d+) goto \*conv_title_start", line)))
    at = next(i for i, line in enumerate(src) if line.strip() == "*conv_title")
    end = next(i for i in range(at, len(src)) if src[i].startswith("btnwait"))
    title_nos = [int(m.group(1)) for line in src[at:end] if (m := re.match(r"spbtn \d+,(\d+)", line))]
    # The story menu: two button screens (*conv_bscreenN, the second once an
    # after-story opens), keyed by their btnwait line, with their button count.
    menus, lab, k = [], None, 0
    for i in range(next(i for i, line in enumerate(src) if line.strip() == "*mg_story_menu"), len(src)):
        if src[i].startswith("*conv_bscreen"):
            lab, k = src[i].strip(), 0
        k += src[i].startswith("spbtn ")
        if lab and src[i].startswith("btnwait"):
            menus.append((f"{lab}:{i + 1}", k)); lab = None
        if len(menus) == 2:
            break
    only = {int(n) for n in args.only.split(",")} if args.only else None
    saves = os.path.join(out, "saves")
    runs = [(name, path, ending, 1) for name, path, ending in ROUTES] + [(name, "", ending, option) for name, option, ending in AFTER]
    for n, (name, path, ending, story) in enumerate(runs, 1):
        if (only and n not in only) or (not only and n > len(ROUTES)):
            continue
        want = dict(p.split(":") for p in path.split(",") if p)
        lines = [f"{title}\t{o}\tavoid" for o in [0, *title_nos] if o != start_no]
        lines += [f"{menu}\t{o}\tavoid" for menu, k in menus for o in range(k + 1) if o != story]
        for scene, (at, k) in selects.items():
            if scene != "STORY_MENU":
                pick = int(want.get(scene, 1))
                lines += [f"{at}\t{o}\tavoid" for o in range(1, k + 1) if o != pick]
        taken = os.path.join(out, f"taken{n}.txt")
        open(taken, "w").write("\n".join(lines) + "\n")
        trace = os.path.join(out, f"route{n}.jsonl")
        # the variables the checker's engine-var rule looks at (cons_trace_check.json's "engine_vars")
        watch = cons_trace_check.trace_vars_of(os.path.join(os.path.dirname(os.path.abspath(__file__)), "cons_trace_check.json"))
        env = dict(os.environ, SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy")
        p = subprocess.run(["timeout", "-k", "5", "7200", engine, "--root", ".", "--save-dir", saves, "--trace", trace,
                            "--trace-ignore", "34-48", "--auto-play", "0", "--auto-choice", "untaken", "--taken", taken,
                            "--roundtrip-every", "1000",
                            *(["--trace-vars", watch] if watch else [])], cwd=game, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        reached = None
        for line in open(trace):
            if '"t":"page"' in line and f"*{ending}" in line:
                reached = json.loads(line)["pg"]
                break
        print(f"route {n} ({name}): exit {p.returncode}, ending {ending} " + (f"at page {reached}" if reached else "NOT reached"), flush=True)


if __name__ == "__main__":
    main()
