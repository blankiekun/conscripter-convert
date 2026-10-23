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
    0x1050: "64a100000000 c3",              # seh(): mov eax,fs:[0]; ret (an MSVC SEH prologue's read)
    0x1060: "6a02 6a28 e897ffffff 83c408 c3",  # outer(): push 2; push 40; call add; add esp,8; ret
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
    assert e.call(BASE + 0x1050)[0] == 0xffffffff  # the empty SEH chain at fs:[0]
    assert e.imports == {STUBS: "k.dll!F"}, e.imports
    e.stubs[STUBS] = 100
    assert e.call(BASE + 0x1020, [7])[0] == 107
    assert e.hits[-1][:2] == (STUBS, "k.dll!F")
    assert e.call(BASE + 0x1060)[0] == 42
    e.stubs[BASE + 0x1000] = 9  # a function inside the image stubbed: a call to it returns at once
    assert e.call(BASE + 0x1060)[0] == 9
    del e.stubs[BASE + 0x1000]
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
        # a scene dump over that base: one changed page
        open(pre + "s.bin", "wb").write(struct.pack("<I", 5678).ljust(0x1000, b"\0"))
        open(pre + "s.map", "w").write("base x\n10000000 00001000 4 20000 d\nP 10000000 1000 0\n")
        assert e.call(BASE + 0x1030)[0] == 1234
        assert e.load_dump(pre + "s") == 2  # the base's regions
        assert e.call(BASE + 0x1030)[0] == 5678
        # an unstubbed memset/memcpy import does its work (args on a cdecl stack at 0x10000800)
        e.imports[STUBS] = "msvcrt.dll!memset"
        e.uc.mem_write(0x10000800, struct.pack("<III", 0x10000900, 0x41, 3))
        assert e._builtin(STUBS)(e, 0x10000800) == 0x10000900 and e.read(0x10000900, 4) == b"AAA\0"
        e.imports[STUBS] = "msvcrt.dll!_memcpy"
        e.uc.mem_write(0x10000800, struct.pack("<III", 0x10000a00, 0x10000900, 2))
        e._builtin(STUBS)(e, 0x10000800)
        assert e.read(0x10000a00, 3) == b"AA\0"
        e.imports[STUBS] = "k.dll!F"
        assert e._builtin(STUBS) == 0
        try:  # a packed scene dump reads the same (Python 3.14's zstd; CI's older Python skips this)
            from compression import zstd
        except ImportError:
            zstd = None
        if zstd:
            open(pre + "s.bin.zst", "wb").write(zstd.compress(open(pre + "s.bin", "rb").read()))
            os.remove(pre + "s.bin")
            e.uc.mem_write(0x10000000, bytes(4))
            e.load_dump(pre + "s")
            assert e.call(BASE + 0x1030)[0] == 5678
    assert e.call(BASE + 0x1000, [40, 2])[0] == 42
    x64_elf()
    print("exe_call self-test: ok")


def x64_elf():
    """an x86-64 ELF (a shared object gcc builds here): System V arguments, a double, an import stubbed"""
    import shutil
    import subprocess
    if not shutil.which("gcc"):
        return print("(no gcc: the x86-64 ELF part skipped)")
    with tempfile.TemporaryDirectory() as d:
        src, so = os.path.join(d, "t.c"), os.path.join(d, "t.so")
        open(src, "w").write("int puts(const char *);\nint add(int a, int b) { return a + b; }\n"
                             "double twice(double x) { return x * 2; }\nint callsym(int a) { return puts(\"x\") + a; }\n")
        subprocess.run(["gcc", "-O1", "-fPIC", "-shared", "-fno-stack-protector", "-o", so, src], check=True)
        e = Exe(so)
    assert e.arch == "x64"
    assert e.call(e.symbol("add"), [40, 2])[0] == 42
    assert e.call(e.symbol("twice"), [("d", 1.25)], ret="double")[0] == 2.5
    e.stubs[next(a for a, n in e.imports.items() if n == "puts")] = 100
    assert e.call(e.symbol("callsym"), [7])[0] == 107


if __name__ == "__main__":
    main()
