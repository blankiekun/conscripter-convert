#!/usr/bin/env python3
"""Saves that start every story scene, chained so one run of the original
loads them one after another (装甲悪鬼村正, 続・殺戮のジャンゴ; docs/games/muramasa.md,
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
next save loads (--at page: after every text block of the file instead,
the first one run going on, for `ntrace.sh --scene-at page --clicks`; a
scene with no text block is left out). scene_chain.py runs the chain. Every edit stays on its line. Prints the manifest
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
    p.add_argument("--only", help="scene files (regex on the name; default: the game's story scenes)")
    p.add_argument("--max", type=int, default=0)
    p.add_argument("--at", choices=["start", "page"], default="start",
                   help="where the next Load goes: the _MAIN scene's first statement, or after the scene's first text block")
    a = p.parse_args()
    tab = ns.table(a.exe)
    django = "django" in os.path.basename(a.exe).lower()
    v = ns.dump(tab, a.template)["vars"]
    here = v["$SYSTEM_spt_name"] + ".nss" if django else v["$GameName"]
    a.only = a.only or (r"^dja\d{3}_.*_\d\d" if django else r"^m[a-z]\d\d_\d\d\d")
    scenes = []
    for f in sorted(os.listdir(a.nss)):
        if f.endswith(".nss") and re.search(a.only, f) and f != here:
            t = open(os.path.join(a.nss, f), "rb").read()
            name = re.escape(f.encode("cp932"))
            m = re.search(rb"\nchapter\s+main\s*\r?\n\{" if django else rb"\nscene\s+" + name + rb"_MAIN\s*\r?\n\{", t)
            at = m and m.end()
            if at and a.at == "page":  # after every </PRE> of the file: the first one run goes on
                at = [m.end() for m in re.finditer(rb"</PRE>", t)] or None
            if at:
                scenes.append((f, t, at if isinstance(at, list) else [at]))
    scenes = scenes[:a.max or None]
    os.makedirs(a.patch, exist_ok=True)
    shutil.copytree(os.path.join(os.path.dirname(a.template), "common"), os.path.join(a.saves, "common"), dirs_exist_ok=True)
    boot_name = "boot.nss" if django else "0_boot.nss"
    boot = open(os.path.join(a.nss, boot_name), "rb").read()
    line = ("$SelectDebug = 0;" if django else "#play_speed_plus = #SYSTEM_play_speed;").encode("cp932")
    boot = boot.replace(line, line + b"Load(1);", 1)
    if django:  # windowed, as ntrace.sh's own boot patch (full screen draws twice under wine)
        code = '#SYSTEM_product_code="ZokuSatsurikunoJyango";'.encode("cp932")
        boot = boot.replace(code, code + b"#SYSTEM_window_full=false;", 1)
    open(os.path.join(a.patch, boot_name), "wb").write(boot)
    for i, (f, t, at) in enumerate(scenes):
        slot = os.path.join(a.saves, f"{i + 1:04d}")
        ns.write(tab, a.template, {"scene": f, "comment": f}, slot)
        nxt = f"Load({i + 2});".encode() if i + 1 < len(scenes) else b""
        for x in reversed(at):
            t = t[:x] + nxt + t[x:]
        open(os.path.join(a.patch, f), "wb").write(t)
        print(f"{i + 1}\t{f}")


if __name__ == "__main__":
    main()
