"""Which framing does a story picture show: its whole texture, or the middle half?

Usage: python3 -I cg_framing.py <gallery dir> <story dir>
  <gallery dir>  `typemoon extract <game-102>/allpaccg.mrg -o <dir> --prefix IMG` (the JPGs)
  <story dir>    `typemoon extract <game-102>/allpac.mrg -o <dir> --prefix IMG --png --max-bytes 600000`
                 (the bare pictures' third-size copies)

The gallery shows a CG (2560 x 1440) on the whole screen. For each one, find
the story picture with the same art, compared two ways: the story picture
whole, and its middle half (what the screen shows if a picture is drawn at
its record size, two screens wide). Prints how many CGs match best each way.
"""
import pathlib
import sys

import numpy as np
from PIL import Image

SIZE = (32, 18)


def signature(img):
    a = np.asarray(img.convert('L').resize(SIZE, Image.BILINEAR), dtype=np.float32).ravel()
    a -= a.mean()
    n = np.linalg.norm(a)
    return a / n if n else a


def centre_half(img):
    w, h = img.size
    return img.crop((w // 4, h // 4, w - w // 4, h - h // 4))


def main():
    gallery, story = map(pathlib.Path, sys.argv[1:3])
    cgs = []
    for p in sorted(gallery.glob('*.JPG')):
        img = Image.open(p)
        if abs(img.width / img.height - 16 / 9) < 0.02:
            cgs.append((p.stem, signature(img)))
    names, whole, middle = [], [], []
    for p in sorted(story.glob('*/*.png')):
        img = Image.open(p)
        if img.width >= 400 and abs(img.width / img.height - 16 / 9) < 0.05:
            names.append(p.parent.name)
            whole.append(signature(img))
            middle.append(signature(centre_half(img)))
    whole, middle = np.array(whole), np.array(middle)
    print(f'{len(cgs)} gallery CGs (16:9), {len(names)} story pictures (16:9 thumbnails)')
    wins = {'whole': 0, 'middle': 0, 'neither': 0}
    examples = []
    for cg, s in cgs:
        cw, cm = whole @ s, middle @ s
        iw, im = int(cw.argmax()), int(cm.argmax())
        best = max(cw[iw], cm[im])
        kind = 'neither' if best < 0.95 else 'whole' if cw[iw] >= cm[im] else 'middle'
        wins[kind] += 1
        if len(examples) < 8 and kind != 'neither':
            examples.append(f'  {cg}: whole {names[iw]} {cw[iw]:.3f}, middle {names[im]} {cm[im]:.3f}')
    print('best match (correlation >= 0.95):', wins)
    print('\n'.join(examples))


main()
