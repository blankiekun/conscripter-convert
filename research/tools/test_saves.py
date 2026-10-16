#!/usr/bin/env python3
"""Make save files that stand at chosen scenes of a converted game.

usage: test_saves.py GAME_DIR SLOTS.txt -o OUT.zip [--vars-dir DIR] [--engine EXE] [--work DIR]

SLOTS.txt: one save a line, "slot label description" (`#` lines and
blank lines skipped), e.g. `20 f_md05_025 立方陣`. For each line the
engine starts at the label (`--goto`) and saves there before any of the
scene runs (`--checkpoint-start`), so loading the save plays the whole
scene; it then plays to the first page and screenshots it for the
thumbnail.
Each save then gets what the save menu would have written: a 160x90
`savethumbN.bmp` and the `"date|play time|text"*` string `getsavestr`
reads from the end of the file (the description goes in the text).
OUT.zip holds the files flat, uncompressed, as the web player's Export
Saves writes them; its Import Saves (and a native save folder) take it.
With `--vars-dir`, a label's `<label>.txt` there (a `--vars` file, e.g.
from research/nitroplus/route_vars.py) sets the route's variables first;
without one the save has none of the route's earlier choices.
"""
import argparse
import datetime
import os
import shutil
import subprocess
import tempfile
import zipfile

p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
p.add_argument("game")
p.add_argument("slots")
p.add_argument("-o", "--out", required=True)
p.add_argument("--vars-dir", help="per-label --vars files, <label>.txt")
p.add_argument("--engine", help="default: the game folder's own launcher, else engine/build/conscripter")
p.add_argument("--work", help="scratch folder (default: a temporary one)")
a = p.parse_args()

game = os.path.abspath(a.game)
engine = a.engine
if not engine:
    # The launcher is the only executable file in a build's folder.
    exes = [f for f in os.listdir(game) if os.path.isfile(os.path.join(game, f)) and os.access(os.path.join(game, f), os.X_OK)]
    engine = os.path.join(game, exes[0]) if len(exes) == 1 else "engine/build/conscripter"
engine = os.path.abspath(engine)
work = a.work or tempfile.mkdtemp(prefix="test_saves.")
saves = os.path.join(work, "saves")
os.makedirs(saves, exist_ok=True)
env = dict(os.environ, SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy")
stamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")

with zipfile.ZipFile(a.out, "w", zipfile.ZIP_STORED) as z:
    for line in open(a.slots, encoding="utf-8"):
        if not line.strip() or line.startswith("#"):
            continue
        slot, label, *desc = line.split(maxsplit=2)
        shots = os.path.join(work, "shot" + slot)
        vars_file = a.vars_dir and os.path.join(os.path.abspath(a.vars_dir), label + ".txt")
        if vars_file and not os.path.exists(vars_file):
            raise SystemExit(f"slot {slot}: no {vars_file}")
        r = subprocess.run(["timeout", "-k", "5", "300", engine, "-r", ".", "--save-dir", saves,
                            "--trace", os.path.join(work, f"save{slot}.jsonl"), "--goto", label,
                            "--auto-play", "0", "--checkpoint-start", "--checkpoint-slot", slot,
                            "--max-pages", "1", "--shot-pages", "1", "--shot-after", "3000", "--shot-dir", shots]
                           + (["--vars", vars_file] if vars_file else []),
                           cwd=game, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        dat = os.path.join(saves, f"save{slot}.dat")
        # the save is made before the scene runs: a run stuck in it later (exit 1) still made it
        if r.returncode not in (0, 1) or not os.path.exists(dat):
            raise SystemExit(f"slot {slot} ({label}): no save, exit code {r.returncode}")
        data = open(dat, "rb").read()
        text = f"{stamp}|000:00:00|{label.removeprefix('f_')} {desc[0].strip() if desc else ''}"
        z.writestr(f"save{slot}.dat", data + b'"' + text.replace('"', "").encode("cp932") + b'"*')
        thumb = os.path.join(work, f"savethumb{slot}.bmp")
        pics = sorted(f for f in os.listdir(shots) if f.endswith(".png")) if os.path.isdir(shots) else []
        if not pics:
            raise SystemExit(f"slot {slot} ({label}): no picture for the thumbnail")
        # the first page's picture, else the one --shot-after took
        pic = "page000001.png" if "page000001.png" in pics else pics[0]
        subprocess.run(["magick", os.path.join(shots, pic), "-resize", "160x90!", "BMP3:" + thumb], check=True)
        z.write(thumb, f"savethumb{slot}.bmp")
        print(slot, label, "ok")

if not a.work:
    shutil.rmtree(work)
print(a.out)
