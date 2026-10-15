#!/usr/bin/env python3
"""Make save files that stand at chosen scenes of a converted game.

usage: test_saves.py GAME_DIR SLOTS.txt -o OUT.zip [--engine EXE] [--work DIR]

SLOTS.txt: one save a line, "slot label description" (`#` lines and
blank lines skipped), e.g. `20 f_md05_025 立方陣`. For each line the
engine starts at the label (`--goto`, no route flags), auto-plays to the
first click wait, saves there (`--checkpoint-*`) and screenshots it.
Each save then gets what the save menu would have written: a 160x90
`savethumbN.bmp` and the `"date|play time|text"*` string `getsavestr`
reads from the end of the file (the description goes in the text).
OUT.zip holds the files flat, uncompressed, as the web player's Export
Saves writes them; its Import Saves (and a native save folder) take it.
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
        r = subprocess.run(["timeout", "-k", "5", "300", engine, "-r", ".", "--save-dir", saves,
                            "--trace", os.path.join(work, f"save{slot}.jsonl"), "--goto", label,
                            "--auto-play", "0", "--checkpoint-every", "1", "--checkpoint-slot", slot,
                            "--max-pages", "2", "--shot-pages", "1", "--shot-dir", shots],
                           cwd=game, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        dat = os.path.join(saves, f"save{slot}.dat")
        if r.returncode != 0 or not os.path.exists(dat):
            raise SystemExit(f"slot {slot} ({label}): no save, exit code {r.returncode}")
        data = open(dat, "rb").read()
        text = f"{stamp}|000:00:00|{label.removeprefix('f_')} {desc[0].strip() if desc else ''}"
        z.writestr(f"save{slot}.dat", data + b'"' + text.replace('"', "").encode("cp932") + b'"*')
        thumb = os.path.join(work, f"savethumb{slot}.bmp")
        subprocess.run(["magick", os.path.join(shots, "page000001.png"), "-resize", "160x90!", "BMP3:" + thumb], check=True)
        z.write(thumb, f"savethumb{slot}.bmp")
        print(slot, label, "ok")

if not a.work:
    shutil.rmtree(work)
print(a.out)
