#!/usr/bin/env python3
"""Picture diff per page: the original's frames against ours.

    picdiff.py ORIG.jsonl ORIG_SHOTS OURS.jsonl OURS_SHOTS --map MAP
               [--mask WxH+X+Y ...] [--fuzz P] [--top N] [-o DIR]
               [--ms] [--thread T] [--pages N] [--self-test]

ORIG.jsonl / OURS.jsonl and MAP: as research/tools/trace_diff.py takes
them (its pairing of pages is used). ORIG_SHOTS: the original's frames,
<pg>.webp or .png named by the original's page (ntrace.sh --shots; one
per text block, so only those pages are compared). OURS_SHOTS: the
engine's --shot-every 1 (or --shot-pages) folder, page<pg 6 digits>.png.

Each pair is masked (--mask boxes painted black in both: our own UI that
differs on purpose, the text window) and compared (as ImageMagick's
`compare -metric AE -fuzz` / `RMSE`, which 7.1.2 no longer counts in whole
pixels): AE = pixels whose biggest channel difference is over --fuzz % of
255 (default 2, rounding; lossless shots of the same picture give 0),
RMSE = root mean square error (0-1); box = where the differing pixels
lie (a narrow box: the text window moved; the whole frame: the picture).
Lossy WebP q80 shots (fc.sh's default) need --fuzz 15: q80 alone leaves
0.2 % of pixels over it, 4 % over 6 (docs/new-game-reference.md, "Picture diff").
Prints every pair, worst AE first, and writes ORIG|OURS|differing-in-red
views (half size) of the --top N (default 5) into -o DIR (default:
OURS_SHOTS/../picdiff).
"""
import argparse
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from trace_diff import orig_side, ours_side, pair, read_map  # noqa: E402


def load(path, masks):
    """RGB as int16, every mask box painted black."""
    a = np.asarray(Image.open(path).convert('RGB'), dtype=np.int16).copy()
    for m in masks:
        x0, y0, x1, y1 = box(m)
        a[y0:y1 + 1, x0:x1 + 1] = 0
    return a


def metric(a, b, fuzz):
    """(pixels whose biggest channel difference is over fuzz % of 255, RMSE 0-1, the mask of those pixels)."""
    d = np.abs(a - b)
    over = d.max(axis=2) > fuzz * 255 / 100
    return int(over.sum()), float(np.sqrt(np.mean((d / 255.0) ** 2))), over


def box(geom):
    """'WxH+X+Y' -> (x0, y0, x1, y1)."""
    wh, x, y = geom.split('+')
    w, h = map(int, wh.split('x'))
    return int(x), int(y), int(x) + w - 1, int(y) + h - 1


def shot(folder, *names):
    return next((os.path.join(folder, n) for n in names if os.path.exists(os.path.join(folder, n))), None)


def self_test():
    assert box('128x576+448+0') == (448, 0, 575, 575)
    a = np.full((20, 40, 3), 128, np.int16)
    b = a.copy()
    b[:10, :10] = 255
    b[10, 10] = 130  # under the fuzz
    assert metric(a, b, 6)[0] == 100
    b[:10, :10] = 0
    a[:10, :10] = 0  # what a 10x10+0+0 mask does
    assert metric(a, b, 6)[0] == 0
    print('picdiff self-test ok')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('orig', nargs='?')
    ap.add_argument('orig_shots', nargs='?')
    ap.add_argument('ours', nargs='?')
    ap.add_argument('ours_shots', nargs='?')
    ap.add_argument('--map')
    ap.add_argument('--mask', action='append', default=[], help='WxH+X+Y painted black in both (repeatable)')
    ap.add_argument('--fuzz', type=float, default=2)
    ap.add_argument('--top', type=int, default=5)
    ap.add_argument('-o', '--out')
    ap.add_argument('--ms', action='store_true', help="the original's trace times by \"ms\" (Nitroplus NSS)")
    ap.add_argument('--thread', type=int, help='only the original\'s records of this "th" (Nitroplus NSS: 0)')
    ap.add_argument('--pages', type=int, default=100000)
    ap.add_argument('--self-test', action='store_true')
    a = ap.parse_args()
    if a.self_test:
        return self_test()
    if not (a.orig and a.orig_shots and a.ours and a.ours_shots and a.map):
        ap.error('ORIG ORIG_SHOTS OURS OURS_SHOTS and --map are needed')
    pairs = pair(orig_side(a.orig, a.pages, 'ms' if a.ms else 'f', a.thread), ours_side(a.ours, read_map(a.map), a.pages))
    rows, unpaired = [], 0
    for o, p in pairs:
        if not o:
            continue
        f = shot(a.orig_shots, f"{o['pg']}.webp", f"{o['pg']}.png")
        if not f:
            continue
        g = p and shot(a.ours_shots, f"page{p['pg']:06d}.png")
        if not g:
            unpaired += 1
            continue
        ae, rmse, over = metric(load(f, a.mask), load(g, a.mask), a.fuzz)
        ys, xs = np.nonzero(over)
        bb = f'{xs.max() - xs.min() + 1}x{ys.max() - ys.min() + 1}+{xs.min()}+{ys.min()}' if ae else '-'
        rows.append((ae, rmse, o, p, f, g, bb))
    rows.sort(key=lambda r: -r[0])
    out = a.out or os.path.join(os.path.dirname(os.path.abspath(a.ours_shots)), 'picdiff')
    os.makedirs(out, exist_ok=True)
    print(f'{len(rows)} pages compared, {unpaired} original shots without our page; fuzz {a.fuzz}%, masks {a.mask or "none"}')
    print(f"{'AE px':>8} {'AE %':>6} {'RMSE':>6}  orig pg  ours pg  {'box':18}  text")
    for i, (ae, rmse, o, p, f, g, bb) in enumerate(rows):
        x, y = load(f, a.mask), load(g, a.mask)
        view = ''
        if i < a.top:  # original | ours | ours with the differing pixels red
            view = os.path.relpath(os.path.join(out, f"pg{o['pg']}-{p['pg']}.png"))
            hl = y.copy()
            hl[metric(x, y, a.fuzz)[2]] = (255, 0, 0)
            strip = np.concatenate([x, np.full((x.shape[0], 4, 3), 255, np.int16), y,
                                    np.full((x.shape[0], 4, 3), 255, np.int16), hl], axis=1).astype(np.uint8)
            im = Image.fromarray(strip)
            im.resize((im.width // 2, im.height // 2)).save(view)
        print(f"{ae:8d} {100 * ae / (x.shape[0] * x.shape[1]):6.2f} {rmse:6.4f}  {o['pg']:7d}  {p['pg']:7d}  {bb:18}  {o['text'][:12]}  {view}")


if __name__ == '__main__':
    main()
