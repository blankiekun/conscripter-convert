"""The eboot's three particle effects (EffectStart 51 snow, 52 light balls,
54 confetti) run under Unicorn (research/tools/exe_call.py): the ctor, the
start, then N updates; the particle records are dumped after each. Printed as
the text fixture research/oretsuba/particles_exe.txt, the ground truth an
engine particle layer is checked against.

    research/tools/.venv/bin/python -I research/oretsuba/particles_exe.py <eboot.elf> [--sites]

rand (0x81131cc4) is stubbed by call site (the caller's LR): the spawn test
of an inactive particle (U < 0.002) returns 0, so every particle spawns on
its first step; every other call returns the case's fixed U. All particles
of a pool then move alike, and particle 0 stands for them. --sites prints
the rand call sites each function reaches (how SPAWN_TEST was found).

Each case also reports whether the start ran its warm-up (light balls: 600
steps when :Param 1, effect+0x20, is nonzero; confetti: 1200 steps when
:Param 3, effect+0x28, is nonzero): `warm` = the rand calls the start made.
"""
import collections
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
import exe_call  # noqa: E402
from exe_call import Exe  # noqa: E402
from unicorn import arm_const as A  # noqa: E402

RAND, NEW, DELETE = 0x81131cc4, 0x81131e14, 0x81131de4
INNER = (0x8103fde8, 0x81085e66, 0x81086982)  # base ctor, sprite and draw-object ctors
EFFECT = exe_call.HEAP + 0x80000  # the effect object (0x400 bytes)
ARENA = exe_call.HEAP + 0x90000   # operator_new's bump arena
# kind: (ctor, start, update, pool, pool size, record words, field names)
KINDS = {
    "snow": (0x810479f4, 0x81047ccc, 0x81047dfa, 0x81162b10, 200, 12,
             "vx vy ax x y act c x0 xw v25 - alpha"),
    "light": (0x81043604, 0x81043902, 0x81043c8a, 0x81162718, 200, 12,
              "vx vy ax x y act c x0 xw v25 - alpha"),
    # confetti has no ctor of its own (the pool is made by its start)
    "confetti": (None, 0x81042f66, 0x81043464, 0x81162588, 100, 21,
                 "s vy - ay x y act c alpha x0 xw cellx last period frame fdir time fall mult ramp rgb"),
}
# the rand calls that test an inactive particle's spawn (U < 0.002), by return address
# (--sites: the site reached 400 times per update with U = 0.5, 2 steps x 200; the
# light start's 120000 = 600 warm steps x 200, the confetti step's 1200 x 100)
SPAWN_TEST = {0x81047e2c, 0x810480b6, 0x81043a16, 0x81043cbc, 0x81043f32, 0x8112ad54}

exe = Exe(sys.argv[1])
sites = collections.Counter()
state = {"u": 0.5, "calls": 0, "arena": ARENA, "site_u": {}}


def rand(exe, _):
    lr = exe.uc.reg_read(A.UC_ARM_REG_LR) & ~1
    sites[lr] += 1
    state["calls"] += 1
    u = 0 if lr in SPAWN_TEST else state["site_u"].get(lr, state["u"])
    return min(int(u * 2 ** 31), 2 ** 31 - 1)


def new(exe, _):
    n = exe.uc.reg_read(A.UC_ARM_REG_R0)
    p = state["arena"]
    exe.uc.mem_write(p, bytes(n))
    state["arena"] += (n + 15) & ~15
    return p


exe.stubs[RAND | 1] = rand
exe.stubs[NEW | 1] = new
exe.stubs[DELETE | 1] = 0
for a in INNER:
    exe.stubs[a | 1] = 0


def f32(w):
    return struct.unpack("<f", struct.pack("<I", w))[0]


def record(pool, words):
    p = exe.ptr(pool)
    raw = struct.unpack(f"<{words}I", exe.read(p, 4 * words))
    out = []
    for i, w in enumerate(raw):
        if i in ((5,) if words == 12 else (6, 15)):
            out.append(str(w & 0xff))  # a byte flag
        elif words == 21 and i in (12, 13, 14, 16):
            out.append(str(w))  # ints: last flip, period, frame, update counter
        elif words == 21 and i == 20:
            out.append(bytes(exe.read(p + 0x50, 3)).hex())  # bytes 0x50-0x52: R G B
        else:
            out.append(f"{f32(w):.9g}")
    return out


def run(kind, u, updates, params=(0, 0, 0, 0), every=25, site_u=None):
    ctor, start, update, pool, size, words, names = KINDS[kind]
    state["u"], state["arena"], state["site_u"] = u, ARENA, site_u or {}
    exe.uc.mem_write(EFFECT, bytes(0x400))
    for i in range(size):
        exe.uc.mem_write(pool + 4 * i, bytes(4))
    exe.uc.mem_write(EFFECT + 0x1c, struct.pack("<4f", *params))
    if ctor:
        exe.call(ctor, [EFFECT])
    state["calls"] = 0
    exe.call(start, [EFFECT], limit=1_000_000_000)
    warm = state["calls"]
    rows, last, d0 = [], None, (0, 0)
    rows.append(("start", record(pool, words)))
    for n in range(1, updates + 1):
        exe.call(update, [EFFECT])
        r = record(pool, words)
        # every update near a change of state (spawn, fade, respawn, flip), else every `every`th
        key = [v for i, v in enumerate(r) if names.split()[i] in ("act", "alpha", "frame", "fdir")]
        xi, yi = (3, 4) if words == 12 else (4, 5)
        d = (0, 0) if last is None else (float(r[xi]) - float(last[xi]), float(r[yi]) - float(last[yi]))
        jump = abs(d[0] - d0[0]) > 40 or abs(d[1] - d0[1]) > 40
        d0 = d
        if n <= 4 or n % every == 0 or key != getattr(run, "key", None) or jump:
            rows.append((n, r))
        run.key, last = key, r
    run.key = None
    return names, warm, rows


if "--sites" in sys.argv:
    SPAWN_TEST.clear()
    for kind in KINDS:
        sites.clear()
        run(kind, 0.5, 2, (0, 1, 0, 1))
        print(kind, {f"{a:#x}": n for a, n in sorted(sites.items())})
    sys.exit()

print("# research/oretsuba/particles_exe.py: the eboot's particle effects under Unicorn.")
print("# rand: the spawn test returns 0 (spawn at once), every other call int(U*2^31).")
print("# warm: rand calls made by the start (0: no warm-up). Rows: update, then particle 0's")
print("# record (every update near a state change, else every 25th); x/y in 1280x720 units.")
for kind, params in (("light", (0, 0, 0, 0)), ("light", (50, 0, 0, 0)), ("light", (0, 1, 0, 0)),
                     ("confetti", (40, 1.5, 0.5, 0)), ("confetti", (40, 1.5, 0.5, 1))):
    print(f"warm {kind} params={list(params)} rand_calls={run(kind, 0.5, 0, params)[1]}")
# confetti: the scenes' only :Param (40,1.5,0.5,0); "side" = y sites (first spawn 0x8112adaa,
# respawn 0x8112b080) at 0.75 so the spawn is at the side, the side choice (0x8112b11c) at 0.25
SIDE = {0x8112adae: 0.75, 0x8112b084: 0.75, 0x8112b120: 0.25}
CASES = [("snow", u, 600, (30, 0, 0, 0), None) for u in (0.1, 0.5, 0.85, 0.9)]
CASES += [("light", u, 2400, (100, 0, 0, 0), None) for u in (0.1, 0.5, 0.9)]
CASES += [("light", 0.5, 50, (100, 1, 0, 0), None)]
# "still" (keys are return addresses, blx + 4): the start's ax (snow blx 0x81047d74, light
# 0x8104398a) at 0 and the step noise (snow
# 0x81047ecc/0x81048156, light 0x81043d5c/0x81043fd2) at 0.5: x stays, so past the end line it fades
CASES += [("snow", 0.5, 700, (30, 0, 0, 0), {0x81047d78: 0.0, 0x81047ed0: 0.5, 0x8104815a: 0.5}),
          ("light", 0.5, 1800, (100, 0, 0, 0), {0x8104398e: 0.0, 0x81043d60: 0.5, 0x81043fd6: 0.5})]
CASES += [("confetti", u, 400, (40, 1.5, 0.5, 0), None) for u in (0.1, 0.25, 0.75, 0.9)]
CASES += [("confetti", 0.75, 400, (40, 1.5, 0.5, 0), SIDE), ("confetti", 0.75, 200, (0, 0, 0, 0), None)]
for kind, u, updates, params, site_u in CASES:
    names, warm, rows = run(kind, u, updates, params, site_u=site_u)
    sites_txt = f" sites={ {f'{a:#x}': v for a, v in site_u.items()} }".replace(" ", "") if site_u else ""
    print(f"\ncase {kind} U={u} params={list(params)}{sites_txt} updates={updates} warm={warm}")
    print("n " + names)
    for n, r in rows:
        print(n, " ".join(r))
