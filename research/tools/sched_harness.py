#!/usr/bin/env python3
"""Step an original exe's scheduler under Unicorn, from a memory dump of the
running game: the engine-neutral half. An adapter per engine says where the
objects are, how to queue an action with the exe's own functions and how to
run one frame (research/nitroplus/sched_django.py is Django's).

    h = Harness(exe, dump_prefix)        # exe_call.Exe with the dump mapped
    h.cstr("name")                       # a string in emulated memory
    h.alloc(n)                           # n zero bytes (also HeapAlloc's stub)
    h.frame(step)                        # the clock +1/fps, then step(h)

Many dumps at once: pool_map(fn, items, jobs) runs fn(item) in worker
processes (one Unicorn each; processes, not threads, for the GIL), jobs from
default_jobs(): the CPUs, or fewer when free memory holds fewer workers of
~200 MB each.

Stubs every adapter gets: timeGetTime/GetTickCount from the frame clock, the
mutex and critical-section calls succeed, HeapAlloc from a bump arena
(HeapFree does nothing). Anything else an exe calls returns 0 and is listed
in h.e.hits.
"""
import multiprocessing
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from exe_call import Exe  # noqa: E402

ARENA, ARENA_SIZE = 0xd0000000, 0x4000000  # above a 32-bit process's 2 GB, below exe_call's own


class Harness:
    def __init__(self, exe, dump, fps=60, clock=0):
        self.e = Exe(exe)
        self.e.load_dump(dump)
        self.e.uc.mem_map(ARENA, ARENA_SIZE)
        self.top, self.fps, self.ms = ARENA, fps, float(clock)
        stubs = {"WINMM.dll!timeGetTime": lambda e, a: int(self.ms),
                 "KERNEL32.dll!GetTickCount": lambda e, a: int(self.ms),
                 "KERNEL32.dll!WaitForSingleObject": 0, "KERNEL32.dll!ReleaseMutex": 1,
                 "KERNEL32.dll!GetProcessHeap": 1, "KERNEL32.dll!HeapFree": 1,
                 "KERNEL32.dll!HeapAlloc": lambda e, a: self.alloc(struct.unpack("<3I", e.read(a, 12))[2])}
        for name, v in stubs.items():
            try:
                self.e.stub_import(name, v)
            except KeyError:
                pass  # this exe doesn't import it

    def tls(self, get_slot, set_slot):
        """A CRT reaching TlsGetValue/TlsSetValue through pointers it saved at
        start (MSVC's __getptd): its slots hold the real DLL's addresses, so
        stub those as one thread's TLS."""
        vals = {}
        for slot, name, fn in ((get_slot, "TlsGetValue", lambda e, a: vals.get(e.ptr(a), 0)),
                               (set_slot, "TlsSetValue", lambda e, a: vals.__setitem__(e.ptr(a), e.ptr(a + 4)) or 1)):
            at = self.e.ptr(slot)
            self.e.imports[at] = "KERNEL32.dll!" + name
            self.e.stubs[at] = fn

    def alloc(self, n):
        at, self.top = self.top, self.top + ((n + 15) & ~15)
        assert self.top <= ARENA + ARENA_SIZE, "arena full"
        self.e.uc.mem_write(at, bytes(n))
        return at

    def cstr(self, s, enc="cp932"):
        b = s.encode(enc) + b"\0"
        at = self.alloc(len(b))
        self.e.uc.mem_write(at, b)
        return at

    def i32(self, a):
        return struct.unpack("<i", self.e.read(a, 4))[0]

    def u16(self, a):
        return struct.unpack("<H", self.e.read(a, 2))[0]

    def frame(self, step):
        self.ms += 1000 / self.fps
        step(self)


def default_jobs(mb_each=200, spare_mb=2048):
    """worker processes the machine holds now: CPUs, and MemAvailable less spare_mb at mb_each"""
    avail = next(int(l.split()[1]) // 1024 for l in open("/proc/meminfo") if l.startswith("MemAvailable:"))
    return max(1, min(os.cpu_count() or 1, (avail - spare_mb) // mb_each))


def pool_map(fn, items, jobs=None):
    """[fn(item) for item in items], in `jobs` worker processes (fn: a module-level function)"""
    items = list(items)
    jobs = min(jobs or default_jobs(), len(items)) or 1
    if jobs == 1:
        return [fn(i) for i in items]
    with multiprocessing.get_context("fork").Pool(jobs) as p:
        return p.map(fn, items, chunksize=1)
