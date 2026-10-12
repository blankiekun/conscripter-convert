import sys, os, struct, glob, collections, json
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from cdlib import load
g = sys.argv[1]; out = sys.argv[2]
durs = []
for path in sorted(glob.glob(g + '/CD/*.xsa')):
    b = open(path, 'rb').read()
    for s in range(256):
        sz, off = struct.unpack_from('<II', b, 4 + 16*s)
        if not (sz or off): continue
        clip = b[off:off+sz]; k = clip.rfind(b'OggS')
        durs.append(struct.unpack_from('<q', clip, k + 6)[0] / 44100)
durs = np.array(durs)
size = {}
for path in glob.glob(g + '/HD/CDFiles/0*.cd'):
    if os.path.basename(path) == '0.cd': continue
    labels, ins, p = load(path)
    for op, sub, arg, src in ins:
        if op == 0x41:
            vid, sz = struct.unpack_from('<II', p, arg); size[vid] = max(size.get(vid, 0), sz)
order = [30, 40, 50, 60, 90, 100, 101, 121, 131, 71, 81, 111, 11, 1]
ids = [v for gid in order for v in sorted(x for x in size if x // 10000 == gid)]
x = np.log(np.array([size[v] for v in ids], float))
y = np.log(np.maximum(durs, 0.05))
# fit log(duration) ~ a*log(size)+b on group 71 at its found position
g71 = [i for i, v in enumerate(ids) if v // 10000 == 71]
a, b = np.polyfit(x[g71], y[11373:11373 + len(g71)], 1)
pred = a * x + b
resid_sd = np.std(y[11373:11373 + len(g71)] - pred[g71])
print(f"fit: log dur = {a:.2f} log size + {b:.2f}, residual sd {resid_sd:.2f}")
# banded global alignment: rows = ids, cols = clips. moves: match (i-1,j-1), skip clip (i,j-1), skip id (i-1,j)
n, m = len(ids), len(durs); band = 400; GAP_CLIP = 2.5; GAP_ID = 4.0
INF = 1e18
def col_range(i):
    c = int(round(i * m / n)); return max(0, c - band), min(m, c + band) + 1
prev_lo, prev_hi = col_range(0)
prev = np.full(prev_hi - prev_lo, INF); prev[:] = GAP_CLIP * np.arange(prev_lo, prev_hi)
moves = []
for i in range(1, n + 1):
    lo, hi = col_range(i)
    cur = np.full(hi - lo, INF); mv = np.zeros(hi - lo, np.int8)
    def P(j):  # prev row value at column j
        k = j - prev_lo
        return prev[k] if 0 <= k < len(prev) else INF
    js = np.arange(lo, hi)
    prev_full = np.full(hi - lo + 1, INF)
    for t, j in enumerate(range(lo - 1, hi)):
        prev_full[t] = P(j) if j >= 0 else INF
    cost = ((y[np.clip(js - 1, 0, m - 1)] - pred[i - 1]) / resid_sd) ** 2
    diag = np.where(js >= 1, prev_full[:-1] + cost, INF)
    up = prev_full[1:] + GAP_ID
    best = np.minimum(diag, up); mv[:] = np.where(diag <= up, 0, 2)
    # left moves (skip clip) need a sequential scan
    for t in range(1, hi - lo):
        if cur_left := best[t - 1] + GAP_CLIP:
            if cur_left < best[t]: best[t] = cur_left; mv[t] = 1
    cur = best
    moves.append((lo, mv)); prev, prev_lo = cur, lo
# traceback from (n, m)
i, j = n, m; pairs = {}
while i > 0:
    lo, mv = moves[i - 1]
    if j == 0 or not (lo <= j < lo + len(mv)): break
    k = mv[j - lo]
    if k == 0: pairs[ids[i - 1]] = j - 1; i -= 1; j -= 1
    elif k == 1: j -= 1
    else: i -= 1
matched = len(pairs)
xs = np.array([x[ids.index(v)] for v in pairs]); ys = np.array([y[c] for c in pairs.values()])
print(f"matched {matched} of {n} ids to {m} clips; r(log size, log dur) over matches = {np.corrcoef(xs, ys)[0,1]:.2f}")
json.dump({str(k): v for k, v in pairs.items()}, open(out, 'w'))
