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
byte at +0x18; none for sounds and processes), whose +0x24 points at the
texture record: rotation x/y/z in degrees at +0xc0/+0xc4/+0xc8, zoom x/y per
mille at +0xcc/+0xd0. The Zoom and Rotate frame callbacks (0x456f30,
0x457240) write them there; the probe's Zoom 1500/2500 and Rotate 33, then
800/900 and -50, read back.

A container's children (CreateName/CreateWindow, `CPW/CP_Frame`) are not
in the list: the parent's +0x12d8 points at its first child, a child's
+0x12d4 at the next (for_each_named_object 0x44cb80 descends so for `a/b`
names). The battle HUD (`cg/cp/*`) lives under the name `CPW`.
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


SIBLING, CHILD = 0x12d4, 0x12d8  # object: next sibling, first child (for_each_named_object 0x44cb80)
R_TEX, T_ROT, T_ZOOM = 0x24, 0xc0, 0xcc  # render +0x24: the texture record; rotation x/y/z (degrees), zoom x/y (per mille)
LIST, SEL, CALL, PIC, X, Y, PRIO, RENDER, R_X, R_Y, R_ALPHA = 0x55e74c, 0x5d4040, 0x180, 0x1a4, 0x9f8, 0x9fc, 0x9ac, 0x1200, 0xc, 0x10, 0x18


def objects(exe, prefix, mem=None):
    """exe: unused (the dump holds the image; kept for scene_check's adapter interface)"""
    e = mem or Memory(prefix)
    i32 = lambda a: struct.unpack("<i", e.read(a, 4))[0]  # noqa: E731
    cstr = lambda a: e.read(a, 260).split(b"\0")[0].decode("cp932", "replace")  # noqa: E731
    out = []

    def one(o, parent):
        name = cstr(o)
        if parent and "/" not in name:
            name = parent + "/" + name
        r = e.ptr(o + RENDER)
        if r:
            t = e.ptr(r + R_TEX)
            zr = {"zx": i32(t + T_ZOOM), "zy": i32(t + T_ZOOM + 4), "rx": i32(t + T_ROT), "ry": i32(t + T_ROT + 4),
                  "rz": i32(t + T_ROT + 8)} if t else {}
            out.append({"name": name, "pic": picture(cstr(o + CALL)), "x": i32(r + R_X), "y": i32(r + R_Y),
                        "alpha": e.read(r + R_ALPHA, 1)[0], "prio": i32(o + PRIO), **zr})
        c = e.ptr(o + CHILD)
        while c and len(out) < 5000:
            one(c, name)
            c = e.ptr(c + SIBLING)

    node = e.ptr(LIST + 28 * (i32(SEL) == 400))
    while node and e.ptr(node + 0xc) and len(out) < 5000:
        one(e.ptr(node + 0xc), "")
        node = e.ptr(node + 0x18)
    return out


def self_test():
    """a list of one name `CPW` holding two textures, the way CP_AllSet leaves the HUD"""
    m = {}

    class Mem:
        def read(self, a, n):
            return bytes(m.get(a + i, 0) for i in range(n))

        def ptr(self, a):
            return struct.unpack("<I", self.read(a, 4))[0]

    def put(a, b):
        m.update({a + i: x for i, x in enumerate(b)})

    def obj(a, name, call, x, alpha):
        put(a, name.encode("cp932") + b"\0")
        put(a + CALL, call.encode("cp932") + b"\0")
        if call:
            put(a + RENDER, struct.pack("<I", a + 0x2000))
            put(a + 0x2000 + R_X, struct.pack("<ii", x, 7))
            put(a + 0x2000 + R_ALPHA, bytes([alpha]))

    cpw, f, c2, node = 0x100000, 0x110000, 0x120000, 0x130000
    put(SEL, struct.pack("<i", 0))
    put(LIST, struct.pack("<I", node))
    put(node + 0xc, struct.pack("<I", cpw))
    obj(cpw, "CPW", "", 0, 0)
    obj(f, "CP_Frame", 'CreateTexture("CPW/CP_Frame",14999,Center,Middle,"cg/cp/shin/フレーム.png");', -40, 255)
    obj(c2, "CP002", 'CreateTexture("CPW/CP002",15000,652,282,"cg/cp/shin/a.png");', 652, 0)
    put(cpw + CHILD, struct.pack("<I", f))
    put(f + SIBLING, struct.pack("<I", c2))
    got = [(o["name"], o["pic"], o["x"], o["alpha"]) for o in objects(None, None, Mem())]
    assert got == [("CPW/CP_Frame", "cg/cp/shin/フレーム.png", -40, 255), ("CPW/CP002", "cg/cp/shin/a.png", 652, 0)], got
    print("scene_objects self-test: ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        sys.exit(self_test())
    for o in objects(*sys.argv[1:3]):
        print(o)
