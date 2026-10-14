"""Finds the parts of a system atlas (a PNG with pictures on transparency):
prints each connected opaque region's box, below a starting row.

    python3 -I atlas.py <file.png> [first_row] [alpha_threshold]
"""
import sys

import numpy as np
from PIL import Image

img = np.array(Image.open(sys.argv[1]).convert('RGBA'))
top = int(sys.argv[2]) if len(sys.argv) > 2 else 0
thr = int(sys.argv[3]) if len(sys.argv) > 3 else 8
a = img[:, :, 3] > thr
a[:top] = False
# Row bands of opaque pixels, then column runs inside each band.
rows = a.any(axis=1)
y = 0
while y < len(rows):
    if not rows[y]:
        y += 1
        continue
    y0 = y
    while y < len(rows) and rows[y]:
        y += 1
    band = a[y0:y].any(axis=0)
    x = 0
    while x < len(band):
        if not band[x]:
            x += 1
            continue
        x0 = x
        while x < len(band) and (band[x] or band[x:x + 4].any()):
            x += 1
        print(f"{x0},{y0},{x - x0},{y - y0}")
