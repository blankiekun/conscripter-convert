#!/usr/bin/env python3
"""Saves that start every story scene, chained so one run of the original
loads them one after another (装甲悪鬼村正; docs/games/muramasa.md,
"Per-scene snapshots").

    python3 -I research/nitroplus/scene_saves.py <Muramasa.exe> <template slot> <nss dir> <saves dir> <patch dir>
        [--only REGEX] [--max N]

<template slot>: a save the game wrote on a page of a story scene (its
`common` beside it is copied too). <nss dir>: nss.npa extracted
(research/nitroplus/npa.py). Slot i (from 0001) is the template sent to
the i-th scene (nitroplus_save.py write, "scene"); <patch dir> gets
0_boot.nss with `Load(1);` after its first game-speed line, and each
scene with `Load(i+1);` as the first statement of its _MAIN scene, so the
scene starts, ntrace.sh --scene-dumps dumps it at that statement, and the
next save loads. Every edit stays on its line. Prints the manifest
(slot, scene) as TSV.
"""
import argparse
import os
import re
import shutil
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
import nitroplus_save as ns  # noqa: E402


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    for a in ("exe", "template", "nss", "saves", "patch"):
        p.add_argument(a)
    p.add_argument("--only", default=r"^m[a-z]\d\d_\d\d\d", help="scene files (regex on the name)")
    p.add_argument("--max", type=int, default=0)
    a = p.parse_args()
    tab = ns.table(a.exe)
    here = ns.dump(tab, a.template)["vars"]["$GameName"]
    scenes = []
    for f in sorted(os.listdir(a.nss)):
        if f.endswith(".nss") and re.search(a.only, f) and f != here:
            t = open(os.path.join(a.nss, f), "rb").read()
            m = re.search(rb"\nscene\s+" + re.escape(f.encode("cp932")) + rb"_MAIN\s*\r?\n\{", t)
            if m:
                scenes.append((f, t, m.end()))
    scenes = scenes[:a.max or None]
    os.makedirs(a.patch, exist_ok=True)
    shutil.copytree(os.path.join(os.path.dirname(a.template), "common"), os.path.join(a.saves, "common"), dirs_exist_ok=True)
    boot = open(os.path.join(a.nss, "0_boot.nss"), "rb").read()
    line = "#play_speed_plus = #SYSTEM_play_speed;".encode("cp932")
    open(os.path.join(a.patch, "0_boot.nss"), "wb").write(boot.replace(line, line + b"Load(1);", 1))
    for i, (f, t, at) in enumerate(scenes):
        slot = os.path.join(a.saves, f"{i + 1:04d}")
        ns.write(tab, a.template, {"scene": f, "comment": f}, slot)
        nxt = f"Load({i + 2});".encode() if i + 1 < len(scenes) else b""
        open(os.path.join(a.patch, f), "wb").write(t[:at] + nxt + t[at:])
        print(f"{i + 1}\t{f}")


if __name__ == "__main__":
    main()
