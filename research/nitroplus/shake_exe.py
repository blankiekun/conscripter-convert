"""Muramasa.exe's Shake flip interval, run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/shake_exe.py <Muramasa.exe>

Calls queue_shake 004462d0 (arg 2 the object name, arg 7 n) with
for_each_named_object 0044cb80 stubbed to read the job record it gets: +0x4c
is the flip interval in ms (1000 / (n * fps / 1000), fps the double at
005dcb40 + 8 * [005dcaf0], through _ftol; FUN_0048c5d0 fills them first). Prints (n, ms) for the n the
scenes use as the Rust table `EXE_STEPS` in shake.rs's tests.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

QUEUE_SHAKE, NAME_LOOKUP, FOR_EACH = 0x4462d0, 0x44c0d0, 0x44cb80
exe = Exe(sys.argv[1])
got = []


def for_each(e, args):
    record = struct.unpack("<I", e.read(args + 4, 4))[0]
    got.append(struct.unpack("<i", e.read(record + 0x4c, 4))[0])
    return 0


exe.call(0x48c5d0, regs={"esi": 0x5dcad8})  # frame tables: 005dcaf0 = 4, 005dcb40 + 8 * 4 = 60.0
exe.stubs[NAME_LOOKUP] = 0
exe.stubs[FOR_EACH] = for_each
rows = []
for n in (1, 7, 30, 50, 60, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000):
    got.clear()
    exe.call(QUEUE_SHAKE, [0, 0, ("&", "2s", b"x\0"), 0, 1000, 0, 0, n, 0, 0], regs={"eax": 0, "ecx": 0})
    rows.append(f"({n}, {got[0]})")
print("    const EXE_STEPS: [(i64, u32); %d] = [%s];" % (len(rows), ", ".join(rows)))
