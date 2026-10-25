"""What the converter's re-encoded JPEG copies lose (assets.rs: zoom and
mirror copies at quality 90).

For each sample: the source JPEG decoded as the exe does (Pillow's libjpeg
matches the exe bit for bit, jpeg_parity.tsv), transformed exactly as the
converter does (jpeg_loss/, image 0.25.10: Triangle resize or flip) but not
encoded, against the converter's copy (image-rs decode, transform, encode)
decoded by libjpeg (as the engine does). Rows: q90 (today), q100, q100_444 (no chroma subsampling, jpeg-encoder), png.

cd research/nitroplus; python3 -I jpeg_loss.py jpeg_loss_samples.tsv local/jpeg/loss > jpeg_loss.tsv
jpeg_loss_samples.tsv: game, source jpg (conscripter-convert npa extract ... -o local/jpeg/src/<mu,dj> --prefix <bg/name.>), mode, args (zoom: ZX ZY in 1/1000; mirror: x|y|xy)
Build first: in jpeg_loss/, CARGO_TARGET_DIR=../local/jpeg_loss-target cargo build --release.
"""
import os
import subprocess
import sys

import numpy as np
from PIL import Image

here = os.path.dirname(os.path.abspath(__file__))
tool = os.path.join(here, "local", "jpeg_loss-target", "release", "jpeg_loss")
samples, work = sys.argv[1], sys.argv[2]
os.makedirs(work, exist_ok=True)
print("game\tsource\ttransform\tcopy\tbytes\tmax\tmean\tpixels_over_1\tpixels")
for n, line in enumerate(open(samples, encoding="utf-8")):
    game, src, mode, *args = line.split()
    out = os.path.join(work, str(n))
    Image.open(src).convert("RGB").save(out + ".exe.png")
    subprocess.run([tool, src, out + ".exe.png", mode, *args, out], check=True)
    ref = np.asarray(Image.open(out + ".ref.png").convert("RGB"), dtype=np.int16)
    for kind in ("q90.jpg", "q100.jpg", "q100_444.jpg", "png"):
        path = f"{out}.{kind}"
        got = np.asarray(Image.open(path).convert("RGB"), dtype=np.int16)
        d = np.abs(got - ref)
        per_px = d.max(axis=2)
        print(f"{game}\t{os.path.basename(src)}\t{mode} {' '.join(args)}\t{kind.split('.')[0]}\t"
              f"{os.path.getsize(path)}\t{d.max()}\t{d.mean():.4f}\t{int((per_px > 1).sum())}\t{per_px.size}")
