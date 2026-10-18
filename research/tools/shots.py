#!/usr/bin/env python3
"""Index of filmed shots: research/<engine>/local/shots/INDEX.tsv (git-ignored).

  shots.py add [--lossless] <file> <game> <scene> <page> <what...>   (file under research/<engine>/local/shots/)
  shots.py find <words...>   every word must match somewhere in a row, any INDEX.tsv
"""
import datetime
import glob
import os
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
args = sys.argv[1:]
lossless = "--lossless" in args
args = [a for a in args if a != "--lossless"]
if len(args) >= 6 and args[0] == "add":
    f, game, scene, page, *what = args[1:]
    d = os.path.dirname(os.path.abspath(f))
    if not d.endswith(os.path.join("local", "shots")):
        sys.exit("shots.py: file must be in research/<engine>/local/shots/")
    if f.lower().endswith((".png", ".bmp")):  # WebP: lossy q80, or lossless with --lossless (pixel compares)
        try:
            from PIL import Image
            n = os.path.splitext(f)[0] + ".webp"
            Image.open(f).save(n, **({"lossless": True, "quality": 100} if lossless else {"quality": 80}), method=6)
            os.remove(f)
            f = n
        except Exception as e:  # keep the original when Pillow has no WebP
            print("shots.py: kept", f, "(%s)" % e, file=sys.stderr)
    with open(os.path.join(d, "INDEX.tsv"), "a") as o:
        o.write("\t".join([os.path.basename(f), game, scene, page, " ".join(what), str(datetime.date.today())]) + "\n")
elif len(args) >= 2 and args[0] == "find":
    for ix in sorted(glob.glob(os.path.join(ROOT, "*/local/shots/INDEX.tsv"))):
        for line in open(ix):
            if all(w.lower() in line.lower() for w in args[1:]):
                print(os.path.join(os.path.dirname(ix), line.rstrip("\n")))
else:
    sys.exit(__doc__)
