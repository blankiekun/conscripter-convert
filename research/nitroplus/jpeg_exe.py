"""Decode JPEGs with the game exe's own libjpeg 6b (under Unicorn) and write
the RGB rows it returns as binary PPMs: the reference for JPEG decode parity.

    research/tools/.venv/bin/python -I research/nitroplus/jpeg_exe.py django|muramasa <exe> OUT_DIR a.jpg ...

The calls are the ones the game's loader makes (Django 0044bea0, Muramasa
00466d60): jpeg_std_error, jpeg_CreateDecompress, jpeg_read_header(TRUE),
jpeg_start_decompress, then jpeg_read_scanlines one row at a time; nothing
is set in between, as in the loaders. The source manager is ours (the whole
file in memory, its callbacks Python stubs); malloc/free are stubbed.
Muramasa's build passes cinfo in registers (eax or esi).
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

# name: (std_error, create, read_header, start, read_scanlines, malloc, free, resync, cinfo size, getenv)
GAMES = {
    "django": (0x476b00, 0x474bc0, 0x474fd0, 0x474af0, 0x474a50, 0x4b46cb, 0x4b360f, 0x4767e0, 0x1b0, 0x4b4d8d),
    "muramasa": (0x490600, 0x48fcf0, 0x48ffb0, 0x490200, 0x4903d0, 0x4d29e3, 0x4d0ff6, 0x491f20, 0x1b0, 0x4d3409),
}
POOL, POOL_SIZE = 0xc0000000, 0x10000000
CB = 0xf7000000  # unmapped: a call there is a Python stub


def decode(exe, game, data):
    std_error, create, read_header, start, read_rows, malloc, free, resync, size, getenv = GAMES[game]
    top = [POOL]

    def alloc(n):
        a = top[0]
        top[0] = (a + n + 15) & ~15
        assert top[0] < POOL + POOL_SIZE
        return a

    def w32(a, v):
        exe.uc.mem_write(a, struct.pack("<I", v & 0xffffffff))

    exe.stubs[malloc] = lambda e, at: alloc(e._args(at, 1)[0])
    exe.stubs[free] = 0
    exe.stubs[getenv] = 0  # JPEGMEM unset (jinit_memory_mgr)
    src = alloc(len(data) + 2)
    exe.uc.mem_write(src, data)
    eoi = alloc(2)
    exe.uc.mem_write(eoi, b"\xff\xd9")
    mgr = alloc(0x1c)

    def fill(e, at):  # past the end: a fake EOI, as jdatasrc does
        w32(mgr, eoi)
        w32(mgr + 4, 2)
        return 1

    def skip(e, at):
        _, n = e._args(at, 2)
        left = e.ptr(mgr + 4)
        n = min(n, left)
        w32(mgr, e.ptr(mgr) + n)
        w32(mgr + 4, left - n)
        return 0

    cbs = {CB: 0, CB + 0x10: fill, CB + 0x20: skip, CB + 0x30: 0}
    exe.stubs.update(cbs)
    for off, v in ((0, src), (4, len(data)), (8, CB), (0xc, CB + 0x10), (0x10, CB + 0x20), (0x14, resync), (0x18, CB + 0x30)):
        w32(mgr + off, v)
    err, cinfo = alloc(0x100), alloc(size)
    big = 1 << 31
    if game == "django":
        exe.call(std_error, [err])
        w32(cinfo, err)
        exe.call(create, [cinfo, 0x3e, size])
        w32(cinfo + 0x18, mgr)
        exe.call(read_header, [cinfo, 1], limit=big)
        exe.call(start, [cinfo], limit=big)
    else:
        exe.call(std_error, [], regs={"eax": err})
        w32(cinfo, err)
        exe.call(create, [], regs={"eax": cinfo})
        w32(cinfo + 0x18, mgr)
        exe.call(read_header, [], regs={"esi": cinfo}, limit=big)
        exe.call(start, [], regs={"eax": cinfo}, limit=big)
    width, height, comps = exe.ptr(cinfo + 0x5c), exe.ptr(cinfo + 0x60), exe.ptr(cinfo + 0x68)
    row, rows = alloc(width * comps), alloc(4)
    w32(rows, row)
    out = bytearray()
    for _ in range(height):
        if game == "django":
            n, _ = exe.call(read_rows, [cinfo, rows, 1], limit=big)
        else:
            n, _ = exe.call(read_rows, [rows], regs={"esi": cinfo}, limit=big)
        assert n == 1, n
        out += exe.read(row, width * comps)
    unexpected = [h for h in exe.hits if h[0] not in cbs and h[0] not in (malloc, free, getenv)]
    assert not unexpected, [(hex(a), n, w) for a, n, w in unexpected[:5]]
    exe.hits.clear()
    return width, height, comps, bytes(out)


def main():
    game, path, out = sys.argv[1:4]
    exe = Exe(path)
    exe.uc.mem_map(POOL, POOL_SIZE)
    os.makedirs(out, exist_ok=True)
    for f in sys.argv[4:]:
        w, h, c, px = decode(exe, game, open(f, "rb").read())
        dest = os.path.join(out, os.path.splitext(os.path.basename(f))[0] + ".ppm")
        with open(dest, "wb") as o:
            o.write(b"P6\n%d %d\n255\n" % (w, h) if c == 3 else b"P5\n%d %d\n255\n" % (w, h))
            o.write(px)
        print(f"{w}x{h}x{c} {dest}")


if __name__ == "__main__":
    main()
