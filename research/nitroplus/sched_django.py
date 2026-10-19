#!/usr/bin/env python3
"""Django.exe 1.01 (unpacked) adapter for research/tools/sched_harness.py,
and the overlap cases of docs/new-game.md step 15 run on it.

    research/tools/.venv/bin/python -I research/nitroplus/sched_django.py <Django.exe> <dumps dir>

The dumps are ntrace.sh's for research/nitroplus/sched_probe.nss
(docs/games/django.md, "Memory snapshots"). Each case starts from a dump,
queues actions with the exe's own job code (job_for_objects 0x437040, the
callbacks timed_steps 0x437980 and cb_00438fb0), runs its per-frame
stepper (step_objects 0x436040) and prints x, y, alpha per frame where it
matters; a `wait` steps frames until the object is idle, as
wait_object_idle (0x433bf0) does. The numbers are checked against the later
dumps of the same case, and are the fixtures of the NSS converter's
`scheduler_*` tests (crates/conv-engines/src/nitroplus/convert/tests.rs).
"""
import gc
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from sched_harness import Harness  # noqa: E402

OBJECTS = 0x50fb1c      # object list heads; [7] when 0x529370 == 400 (node +0xc object, +0x18 next)
PROCESSES = 0x50fb48    # NSS processes (dword [1] id, [0x231] its object, [0x232] next)
JOB_FOR_OBJECTS = 0x437040
STEP_OBJECTS = 0x436040
TIMED_STEPS, FADE_STEPS = 0x437980, 0x438fb0
START_ALL = 0x4350b0    # EDI 1: every object's queued actions start (the move, fade, ... running flags)
IDLE = 0x43e7b0         # a per-frame slot with nothing to do
SLOTS = range(0x820, 0x840, 4)
FADE_SLOT, MOVE_SLOT = [0x820], [0x828]
X, Y, MOVE_ON, MOVE_STEP, MOVE_STEPS, RENDER = 0x3ec, 0x3f0, 0x428, 0x44a, 0x458, 0x804


class Django:
    def __init__(self, exe, dump):
        self.h = Harness(exe, dump)
        self.h.tls(0x4ebacc, 0x4ebad0)  # __getptd's TlsGetValue, TlsSetValue (VS2003 CRT)
        e = self.h.e
        self.h.ms = 0  # set from the first object's last frame time below
        self.owner = next(e.ptr(p + 0x231 * 4) for p in self._procs() if e.ptr(p + 0x231 * 4))
        self.h.ms = max(self.h.i32(o + 0x3ac) for _, o in self.objects())

    def _procs(self):
        p = PROCESSES
        while p:
            yield p
            p = self.h.e.ptr(p + 0x232 * 4)

    def objects(self):
        e = self.h.e
        sel = struct.unpack("<I", e.read(0x529370, 4))[0]
        node = e.ptr(OBJECTS + 4 * 7 * (sel == 400))
        while node and e.ptr(node + 0xc):
            o = e.ptr(node + 0xc)
            yield e.read(o, 64).split(b"\0")[0].decode("cp932"), o
            node = e.ptr(node + 0x18)

    def obj(self, name):
        return dict(self.objects())[name]

    def state(self, name):
        h, o = self.h, self.obj(name)
        r = h.e.ptr(o + RENDER)
        return {"x": h.i32(o + X), "y": h.i32(o + Y), "alpha": h.e.read(r + 0x14, 1)[0] if r else None,
                "move_step": h.u16(o + MOVE_STEP), "move_steps": h.i32(o + MOVE_STEPS)}

    def busy(self, name, slots=SLOTS):
        o = self.obj(name)
        return any(self.h.e.ptr(o + k) != IDLE for k in slots)

    def _job(self, name, ms, cb, **strs):
        h = self.h
        job = h.alloc(0x1d8)
        w = h.e.uc.mem_write
        w(job + 4, struct.pack("<i", ms))
        # the name as the built-ins pass it (resolved by 0x455330): under the process's object
        path = h.e.read(self.owner, 64).split(b"\0")[0] + b"/" + name.encode("cp932")
        w(job + 0xc4, path + b"\0")
        for off, s in strs.items():
            if s is not None:
                w(job + int(off[1:], 16), struct.pack("<I", h.cstr(str(s))))
        w(job + 0x1d4, struct.pack("<I", cb))
        h.e.call(JOB_FOR_OBJECTS, [self.owner, job])

    def move(self, name, ms, x, y, tempo=None):
        """Move(name, ms, x, y, tempo, false): x, y as the script writes them ("@320", "0")"""
        self._job(name, ms, TIMED_STEPS, o70=x, o74=y, oa0=tempo)

    def fade(self, name, ms, alpha, tempo=None):
        self._job(name, ms, FADE_STEPS, o6c=alpha, oa0=tempo)

    def start(self):
        """what every waiting statement does first (timed_wait, Wait, WaitMove ...): queued actions start"""
        self.h.e.call(START_ALL, [], regs={"edi": 1})

    def frame(self):
        self.h.frame(lambda h: h.e.call(STEP_OBJECTS, []))

    def sleep(self, ms):
        """Wait(ms): start, then the frames of ms"""
        self.start()
        for _ in range(round(ms * self.h.fps / 1000)):
            self.frame()

    def wait(self, name, slots=SLOTS, limit=100000):
        """start, then frames until the slots are idle: a waited Fade (FADE_SLOT),
        Move (MOVE_SLOT), or WaitAction (all; wait_object_idle)"""
        self.start()
        n = 0
        while self.busy(name, slots) and n < limit:
            self.frame()
            n += 1
        return n


def dumped(exe, d, lab, name):
    """the object's state in a dump, as the harness reads it"""
    return Django(exe, os.path.join(d, lab)).state(name)


def cases(exe, d):
    """each case: what the harness computes from a dump, against the next dump; yields (case, numbers)"""
    g = lambda lab: Django(exe, os.path.join(d, lab))  # noqa: E731
    keep = ("x", "y", "alpha")
    # unwaited goes on: FadeFade after an unwaited 16 s pan (dja010_01:76-78), from c0_1 (the Move queued)
    c = g("c0_1")
    c.fade("背景21", 0, 0)
    hidden = c.state("背景21")
    c.fade("背景21", 1000, 1000)
    c.start()  # Fade(..., true): queued actions start, then its wait
    xs, n, frames, after = {}, 0, None, None
    while c.state("背景21")["move_steps"]:
        st = c.state("背景21")
        xs.setdefault(st["move_step"], st["x"])
        if frames is None and not c.busy("背景21", FADE_SLOT):
            frames, after = n, st
        c.frame()
        n += 1
    end = c.state("背景21")
    c3, c4 = dumped(exe, d, "c0_3", "背景21"), dumped(exe, d, "c0_4", "背景21")
    assert hidden["alpha"] == 0 and hidden["move_step"] == 0, hidden
    assert xs[c3["move_step"]] == c3["x"] and xs[c4["move_step"]] == c4["x"], (xs, c3, c4)
    yield "unwaited goes on", {"fade_in_frames": frames, "after_fade_in": after, "x_at_step": {k: xs.get(k) for k in (0, 1, 60, 480, 959)},
                               "end": {k: end[k] for k in keep}}
    # 0 ms on in-flight: a 0 ms Move on a 4 s one (from c1_1, step 63 of 240); a 0 ms Fade on a 4 s one (from c2_2)
    c = g("c1_1")
    before = c.state("probeA")
    c.move("probeA", 0, "@0", "@100")
    st = c.state("probeA")
    ref = dumped(exe, d, "c2_0", "probeA")
    assert (st["x"], st["y"], c.busy("probeA", MOVE_SLOT)) == (ref["x"], ref["y"], False), (st, ref)
    c = g("c2_2")
    fbefore = c.state("probeB")
    c.fade("probeB", 0, 1000)
    fst = c.state("probeB")
    assert fst["alpha"] == dumped(exe, d, "c2_3", "probeB")["alpha"] and not c.busy("probeB", FADE_SLOT)
    yield "0 ms on in-flight", {"move_before": {k: before[k] for k in keep + ("move_step", "move_steps")},
                                "move_after": {k: st[k] for k in keep}, "move_end_was": [before["x"] + 400 - before["x"] + 0, None],
                                "fade_before": fbefore["alpha"], "fade_after": fst["alpha"]}
    # wait on in-flight: a waited 500 ms Move over an unwaited 4 s one (from c3_0)
    c = g("c3_0")
    before = c.state("probeA")
    c.move("probeA", 500, "@0", "@100")
    frames = c.wait("probeA", MOVE_SLOT)
    st = c.state("probeA")
    ref = dumped(exe, d, "c3_1", "probeA")
    assert (st["x"], st["y"]) == (ref["x"], ref["y"]), (st, ref)
    yield "wait on in-flight", {"before": {k: before[k] for k in keep + ("move_step", "move_steps")},
                                "frames": frames, "after": {k: st[k] for k in keep}}
    # replace or delete while moving: from the dumps alone (CreateColor isn't run here)
    a0, a1, a2 = (dumped(exe, d, lab, "probeA") for lab in ("c4_0", "c4_1", "c4_2"))
    yield "replace or delete while moving", {"moving": {k: a0[k] for k in keep + ("move_step", "move_steps")},
                                             "recreated": {k: a1[k] for k in keep + ("move_steps",)},
                                             "500 ms later": {k: a2[k] for k in keep}}


def step_dump(job):
    """one dump: every object's state as loaded, then after a start (what the
    next waiting statement does) and 1, 30, 60 frames"""
    exe, prefix = job
    g = Django(exe, prefix)
    names = [n for n, _ in g.objects()]
    out = {0: {n: g.state(n) for n in names}}
    g.start()
    for f in range(1, 61):
        g.frame()
        if f in (1, 30, 60):
            out[f] = {n: g.state(n) for n in names}
    del g
    gc.collect()  # the stubs' closures hold the emulator in a cycle: free its ~130 MB now
    return os.path.basename(prefix), out


def main():
    import argparse
    import time
    from sched_harness import pool_map
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("exe")
    p.add_argument("dumps")
    p.add_argument("--jobs", type=int, help="worker processes (default: CPUs, as free memory allows)")
    p.add_argument("--cases", action="store_true", help="the overlap cases, checked against the later dumps")
    a = p.parse_args()
    if a.cases:
        for case, numbers in cases(a.exe, a.dumps):
            print(f"{case}: {numbers}")
        return
    labels = sorted(f[:-4] for f in os.listdir(a.dumps) if f.endswith(".map"))
    t = time.time()
    res = pool_map(step_dump, [(a.exe, os.path.join(a.dumps, lab)) for lab in labels], a.jobs)
    for lab, out in res:
        for n in out[0]:
            if n == "SelectStoryMode":
                continue
            row = " | ".join(f"{f}: x {out[f][n]['x']} y {out[f][n]['y']} a {out[f][n]['alpha']} mv {out[f][n]['move_step']}/{out[f][n]['move_steps']}"
                             for f in (0, 1, 30, 60) if n in out[f])
            print(f"{lab} {n}: {row}")
    print(f"{len(labels)} dumps, {time.time() - t:.1f} s")


if __name__ == "__main__":
    main()
