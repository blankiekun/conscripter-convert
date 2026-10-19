#!/usr/bin/env python3
"""The objects on screen in a 装甲悪鬼村正 memory dump (Muramasa.exe, the
game's own exe): name, picture, x, y, alpha, priority. The adapter of
research/tools/scene_check.py for this game.

Layout (2026-10-18, from a probe dump: CreateTexture at 123,45, priority
1234, Fade 700, then Move @10,@20 and Fade 350, and from the Move
callback 0x44d9b0's time-0 branch): the object list at 0x55e74c ([7] when
0x5d4040 == 400; node +0xc the object, +0x18 the next), the object's name
at +0, the call that made it at +0x180 (`CreateTexture("…",100,0,0,"cg/…");`), x/y at +0x9f8/+0x9fc, priority at
+0x9ac, the render record at +0x1200 (x/y at +0xc/+0x10, alpha 0-255 a
byte at +0x18; none for sounds and processes). Zoom and rotation not
located yet (a time-0 Zoom/Rotate in the probe changed no field).
"""
import os
import re
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from dumppack import Memory  # noqa: E402

COLOURS = {"black": "000000", "white": "ffffff", "red": "ff0000", "green": "00ff00", "blue": "0000ff"}
# the game's own interface, which our engine draws otherwise (text window, quick bar, cursor)
IGNORE = r"^(box|Quick|hit$|img$|Mouse|バグフィクス$|上背景$)"


def picture(call):
    """what a creating call shows: a texture's file, or `color:rrggbb:WxH`"""
    m = re.match(r'CreateColor\("[^"]*",[^,]*,[^,]*,[^,]*,\s*(\d+)\s*,\s*(\d+)\s*,\s*"?([^");]+)"?\)', call)
    if m:
        c = m.group(3).strip().lower()
        c = COLOURS.get(c, c[2:] if c.startswith("0x") else c[1:] if c.startswith("#") else c)
        return f"color:{c}:{m.group(1)}x{m.group(2)}"
    if call.startswith("CreateEffect"):
        q = re.findall(r'"([^"]*)"', call)
        return "effect:" + (q[-1] if len(q) > 1 else "?")
    q = re.findall(r'"([^"]*)"', call)
    return q[-1] if call.startswith("Create") and len(q) > 1 else ""


def label(scene):
    """our 0.txt's label for a scene file"""
    return "f_" + scene[:-4] if scene.endswith(".nss") else None


LIST, SEL, CALL, PIC, X, Y, PRIO, RENDER, R_X, R_Y, R_ALPHA = 0x55e74c, 0x5d4040, 0x180, 0x1a4, 0x9f8, 0x9fc, 0x9ac, 0x1200, 0xc, 0x10, 0x18


def objects(exe, prefix):
    """exe: unused (the dump holds the image; kept for scene_check's adapter interface)"""
    e = Memory(prefix)
    i32 = lambda a: struct.unpack("<i", e.read(a, 4))[0]  # noqa: E731
    cstr = lambda a: e.read(a, 260).split(b"\0")[0].decode("cp932", "replace")  # noqa: E731
    node, out = e.ptr(LIST + 28 * (i32(SEL) == 400)), []
    while node and e.ptr(node + 0xc) and len(out) < 5000:
        o = e.ptr(node + 0xc)
        r = e.ptr(o + RENDER)
        if r:
            out.append({"name": cstr(o), "pic": picture(cstr(o + CALL)), "x": i32(r + R_X), "y": i32(r + R_Y),
                        "alpha": e.read(r + R_ALPHA, 1)[0], "prio": i32(o + PRIO)})
        node = e.ptr(node + 0x18)
    return out


if __name__ == "__main__":
    for o in objects(*sys.argv[1:3]):
        print(o)
