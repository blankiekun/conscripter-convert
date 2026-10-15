#!/usr/bin/env python3
"""Make one small PNG to look at instead of reading full screenshots.

usage: view.py A.png [B.png ...] [--crop WxH+X+Y] [--scale 50] [-o OUT.png]

Crops every image to the same box (optional), scales it (default 50%)
and puts them side by side, so comparing the original with our build is
one small read. Prints the output path and its size.
"""
import argparse
import subprocess

p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
p.add_argument("images", nargs="+")
p.add_argument("--crop", help="ImageMagick geometry, e.g. 400x120+0+480")
p.add_argument("--scale", type=int, default=50, help="percent (default 50)")
p.add_argument("-o", "--out", help="default: <first image>.view.png")
a = p.parse_args()

out = a.out or a.images[0].rsplit(".", 1)[0] + ".view.png"
cmd = ["magick", *a.images]
if a.crop:
    cmd += ["-crop", a.crop, "+repage"]
cmd += ["-resize", f"{a.scale}%", "-background", "red", "-splice", "2x0", "+append", "-chop", "2x0", out]
subprocess.run(cmd, check=True)
size = subprocess.run(["magick", "identify", "-format", "%wx%h", out], capture_output=True, text=True, check=True).stdout
print(out, size)
