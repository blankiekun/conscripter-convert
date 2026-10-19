#!/usr/bin/env python3
"""Step an original exe's scheduler under Unicorn, from a memory dump of the
running game: the engine-neutral half. An adapter per engine says where the
objects are, how to queue an action with the exe's own functions and how to
run one frame (research/nitroplus/sched_django.py is Django's).

    h = Harness(exe, dump_prefix)        # exe_call.Exe with the dump mapped
    h.cstr("name")                       # a string in emulated memory
    h.alloc(n)                           # n zero bytes (also HeapAlloc's stub)
    h.frame(step)                        # the clock +1/fps, then step(h)

Stubs every adapter gets: timeGetTime/GetTickCount from the frame clock, the
mutex and critical-section calls succeed, HeapAlloc from a bump arena
(HeapFree does nothing). Anything else an exe calls returns 0 and is listed
in h.e.hits.
"""
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
