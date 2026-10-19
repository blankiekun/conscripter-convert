#!/usr/bin/env python3
"""Self-test of exe_call.py: builds a tiny PE32 in memory and calls its
functions (int arguments, a float return, an import stubbed with a value).
Run by setup-venv.sh and CI: research/tools/.venv/bin/python -I research/tools/test_exe_call.py"""
import os
import struct
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import exe_call  # noqa: E402
from exe_call import Exe, STUBS  # noqa: E402

BASE, TEXT = 0x400000, 0x1000
CODE = {
    0x1000: "8b442404 03442408 c3",        # add(a, b): mov eax,[esp+4]; add eax,[esp+8]; ret
    0x1010: "d9442404 d8c0 c3",            # twice(float x): fld [esp+4]; fadd st0,st0; ret
    0x1020: "ff1550114000 03442404 c3",    # imp(a): call [IAT]; add eax,[esp+4]; ret
    0x1030: "a100000010 c3",                # glob(): mov eax,[0x10000000]; ret
    0x1040: "6a05 ff1550114000 03442404 c3",  # imp1(a): push 5; call [IAT] (stdcall, 1 arg); add eax,[esp+4]; ret
    0x1100: "40110000 00000000 00000000 60110000 50110000",  # import descriptor, then a zero one
    0x1140: "70110000",                    # lookup table: the hint/name at 0x1170
    0x1150: "70110000",                    # IAT
    0x1160: "6b2e646c6c00",                # "k.dll"
    0x1170: "0000 4600",                   # hint 0, "F"
}


def tiny_pe():
    sec = bytearray(0x200)
    for va, hx in CODE.items():
        b = bytes.fromhex(hx.replace(" ", ""))
        sec[va - TEXT:va - TEXT + len(b)] = b
    h = bytearray(0x200)
    h[:2], h[0x3c:0x40] = b"MZ", struct.pack("<I", 0x40)
    h[0x40:0x44] = b"PE\0\0"
    struct.pack_into("<HHIIIHH", h, 0x44, 0x14c, 1, 0, 0, 0, 0xe0, 0x102)
    o = 0x58
    struct.pack_into("<H", h, o, 0x10b)
    struct.pack_into("<I", h, o + 28, BASE)
    struct.pack_into("<II", h, o + 56, 0x2000, 0x200)       # SizeOfImage, SizeOfHeaders
    struct.pack_into("<II", h, o + 104, 0x1100, 40)         # imports directory
    struct.pack_into("<8sIIII", h, o + 0xe0, b".text", 0x200, TEXT, 0x200, 0x200)
    struct.pack_into("<I", h, o + 0xe0 + 36, 0x60000020)       # code, executable
    return bytes(h + sec)


def main():
    with tempfile.NamedTemporaryFile(suffix=".exe") as f:
        f.write(tiny_pe())
        f.flush()
        e = Exe(f.name)
    assert e.call(BASE + 0x1000, [40, 2])[0] == 42
    assert e.call(BASE + 0x1000, [-1, 0])[0] == 0xffffffff
    assert e.call(BASE + 0x1010, [1.25], ret="float")[0] == 2.5
    assert e.imports == {STUBS: "k.dll!F"}, e.imports
    e.stubs[STUBS] = 100
    assert e.call(BASE + 0x1020, [7])[0] == 107
    assert e.hits[-1][:2] == (STUBS, "k.dll!F")
    # a stub as a function of its arguments, popping them as stdcall
    exe_call.STDCALL["F"] = 1
    e.stub_import("K.DLL!f", lambda e, at: 10 * struct.unpack("<I", e.read(at, 4))[0])
    assert e.call(BASE + 0x1040, [7])[0] == 57
    # a process dump: a region of its own and the image's code put back
    with tempfile.TemporaryDirectory() as d:
        pre = os.path.join(d, "x")
        open(pre + ".bin", "wb").write(struct.pack("<I", 1234).ljust(0x1000, b"\0") + b"\xcc" * 0x1000)
        open(pre + ".map", "w").write("10000000 00001000 4 20000 0\n00401000 00001000 20 1000000 1000\n20000000 00010000 4 20000 -\n")
        assert e.load_dump(pre) == 2
    assert e.call(BASE + 0x1030)[0] == 1234
    assert e.call(BASE + 0x1000, [40, 2])[0] == 42
    print("exe_call self-test: ok")


if __name__ == "__main__":
    main()
