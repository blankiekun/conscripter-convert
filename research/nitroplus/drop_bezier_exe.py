"""Muramasa.exe's Drop Bezier (FUN_00414260) under Unicorn vs CONscripter's swing path.

    research/tools/.venv/bin/python -I research/nitroplus/drop_bezier_exe.py <Muramasa.exe>

00414260(x0, y0, x1, y1, x2, y2, x3, y3, n) writes n points (int x, int y,
12-byte stride) to the buffer in EAX: t = i/n for i = 0..n-1, cubic Bezier,
truncated (_ftol2). The Drop init 00434200 calls it with n = ey - y0 (one
point a row; ey = h + 2*texH), y1 = ey>>2, y2 = (ey>>1)+(ey>>2) from 0, and
x1/x2 = min(x0,x3)-swing / max(x0,x3)+swing, swapped when the drop's seed
is even. Prints the largest |exe - ours| over the points for a few drops,
ours being engine/src/cons/builtin_dll/layer_particle.cpp pathPos.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402


def ours(px, py, xe, ey, swing, flip, pt, old=False):
    """pathPos with prm.swing != 0 (old: before engine PR drop-bezier, y from
    the start, never mirrored)"""
    ln = ey - py
    u = pt / ln
    lo, hi = min(px, xe) - swing, max(px, xe) + swing
    x1, x2 = (hi, lo) if flip and not old else (lo, hi)
    y1, y2 = (py + ln / 4, py + ln * 3 / 4) if old else (ey >> 2, (ey >> 1) + (ey >> 2))
    a = 1 - u
    b = (a * a * a, 3 * a * a * u, 3 * a * u * u, u * u * u)
    return (b[0] * px + b[1] * x1 + b[2] * x2 + b[3] * xe,
            b[0] * py + b[1] * y1 + b[2] * y2 + b[3] * ey)


exe = Exe(sys.argv[1])
BUF = ("buf", 12 * 4000)  # an unused 10th argument: the same heap address every call
_, [(buf, _n)] = exe.call(0x414260, [0] * 9 + [BUF])
H, TEXH = 576, 64
EY = H + 2 * TEXH
for x0, y0, dx, swing, flip in [(100, -10, 50, 128, False), (900, -280, -127, 180, True),
                                (512, 0, 0, 30, False), (40, -150, 90, 90, True)]:
    xe, n = x0 + dx, EY - y0
    lo, hi = min(x0, xe) - swing, max(x0, xe) + swing
    x1, x2 = (hi, lo) if flip else (lo, hi)
    exe.call(0x414260, [x0, y0, x1, EY >> 2, x2, (EY >> 1) + (EY >> 2), xe, EY, n, BUF], regs={"eax": buf})
    pts = [struct.unpack_from("<ii", exe.read(buf + 12 * i, 8)) for i in range(n)]
    err, old = (max(max(abs(pts[i][0] - o[0]), abs(pts[i][1] - o[1]))
                    for i in range(n) for o in [ours(x0, y0, xe, EY, swing, flip, i, k)]) for k in (False, True))
    print(f"x0={x0} y0={y0} xe={xe} swing={swing} flip={flip} n={n} first={pts[0]} mid={pts[n // 2]} last={pts[-1]} max|exe-ours|={err:.3f} old={old:.1f}")
