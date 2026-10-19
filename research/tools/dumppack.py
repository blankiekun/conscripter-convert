#!/usr/bin/env python3
"""Pack a folder of scene dumps (ntrace.c NTRACE_SCENE_DUMPS) with zstd.

    python3 -I research/tools/dumppack.py DIR [--key N] [--watch FILE]
    python3 -I research/tools/dumppack.py --self-test

Each scene's .bin becomes .bin.zst. A keyframe is packed alone: the first
scene, then one every N (default 32), or sooner when the keyframe holds
less than half a scene's pages (a boot file). The scenes after it are packed
with the keyframe's raw .bin as a zstd prefix ("patch from": consecutive
scenes share most of their changed pages, about a tenth of the size
again), and their .map gets a line `zstd-from <keyframe>`. A raw keyframe
is deleted once a later keyframe is packed. --watch FILE: pack while FILE
exists (every 10 s, all but the newest dump), then the rest. exe_call's
load_dump reads every form. Needs Python 3.14 (compression.zstd).
"""
import os
import re
import sys
import time

from compression import zstd
from compression.zstd import CompressionParameter as C, DecompressionParameter as D, ZstdDict

OPTS = {C.compression_level: 3, C.window_log: 28, C.enable_long_distance_matching: 1}


def pack_one(d, lab, key):
    b = open(os.path.join(d, lab + ".bin"), "rb").read()
    if key:
        ref = ZstdDict(open(os.path.join(d, key + ".bin"), "rb").read(), is_raw=True)
        c = zstd.compress(b, options=OPTS, zstd_dict=ref.as_prefix)
        with open(os.path.join(d, lab + ".map"), "a") as m:
            m.write(f"zstd-from {key}\n")
    else:
        c = zstd.compress(b, options=OPTS)
    open(os.path.join(d, lab + ".bin.zst.tmp"), "wb").write(c)
    os.replace(os.path.join(d, lab + ".bin.zst.tmp"), os.path.join(d, lab + ".bin.zst"))


def unpack(d, lab):
    """the raw .bin of a packed dump (its keyframe's too, if it names one)"""
    raw = os.path.join(d, lab + ".bin")
    if os.path.exists(raw):
        return open(raw, "rb").read()
    key = next((l.split()[1] for l in open(os.path.join(d, lab + ".map")) if l.startswith("zstd-from ")), None)
    c = open(raw + ".zst", "rb").read()
    if key:
        return zstd.decompress(c, zstd_dict=ZstdDict(unpack(d, key), is_raw=True).as_prefix, options={D.window_log_max: 31})
    return zstd.decompress(c, options={D.window_log_max: 31})


class Memory:
    """A dump's memory read without an emulator (a few hundred MB less than
    exe_call's load_dump): read(addr, n), ptr(addr). The exe's own image comes
    from the dump too (its globals)."""

    def __init__(self, prefix):
        self.pages = {}  # page number -> 4 KB view; a scene's pages replace the base's
        self._load(prefix)

    def _load(self, prefix):
        d, lab = os.path.dirname(prefix) or ".", os.path.basename(prefix)
        blob = memoryview(unpack(d, lab))
        for line in open(prefix + ".map"):
            f = line.split()
            if f[0] == "base":
                self._load(os.path.join(d, f[1]))
            elif f[0] == "P":
                self._put(int(f[1], 16), blob[int(f[3], 16):int(f[3], 16) + int(f[2], 16)])
            elif f[0] != "zstd-from" and f[4] not in ("-", "d"):
                self._put(int(f[0], 16), blob[int(f[4], 16):int(f[4], 16) + int(f[1], 16)])

    def _put(self, addr, data):  # page by page, so a scene's pages replace the base's
        if addr & 0xfff or len(data) & 0xfff:  # not page-aligned (a GDB stub's chunks): byte ranges into the pages
            for i in range(len(data)):
                a = addr + i
                pg = self.pages.get(a >> 12)
                if not isinstance(pg, bytearray):
                    pg = self.pages[a >> 12] = bytearray(pg if pg is not None else bytes(4096))
                pg[a & 0xfff] = data[i]
            return
        for i in range(0, len(data), 4096):
            self.pages[(addr + i) >> 12] = data[i:i + 4096]

    def read(self, addr, n):
        out = bytearray()
        while n > 0:
            pg = self.pages.get(addr >> 12)
            o = addr & 0xfff
            k = min(n, 4096 - o)
            out += pg[o:o + k] if pg is not None else bytes(k)
            addr, n = addr + k, n - k
        return bytes(out)

    def ptr(self, addr):
        return int.from_bytes(self.read(addr, 4), "little")


def pack(d, every, final):
    labs = sorted(f[:-4] for f in os.listdir(d) if re.fullmatch(r"s\d{4}\.bin", f)
                  and not os.path.exists(os.path.join(d, f + ".zst")))  # (a keyframe's raw stays)
    if not final:
        labs = labs[:-1]  # the newest may still be written
    kf = os.path.join(d, ".keyframe")  # the current keyframe, kept across runs of pack()
    for lab in labs:
        n = int(lab[1:])
        raw = os.path.join(d, lab + ".bin")
        if n == 0:
            pack_one(d, lab, None)
            os.remove(raw)
            continue
        key = open(kf).read() if os.path.exists(kf) else None
        kraw = key and os.path.join(d, key + ".bin")
        # a new keyframe every N scenes, or when the old one is much smaller (a boot file's few pages)
        if not key or n - int(key[1:]) >= every or not os.path.exists(kraw) or os.path.getsize(kraw) * 2 < os.path.getsize(raw):
            pack_one(d, lab, None)  # raw kept for the scenes after it
            open(kf, "w").write(lab)
            if kraw and os.path.exists(kraw):
                os.remove(kraw)
        else:
            pack_one(d, lab, key)
            os.remove(raw)
    if final:  # the last raw keyframe
        for f in os.listdir(d):
            if re.fullmatch(r"s\d{4}\.bin", f) and os.path.exists(os.path.join(d, f + ".zst")):
                os.remove(os.path.join(d, f))


def self_test():
    import random
    import tempfile
    rnd = random.Random(1)
    base = bytes(rnd.getrandbits(8) for _ in range(1 << 16))
    with tempfile.TemporaryDirectory() as d:
        want = {}
        for n in range(6):
            b = bytearray(base)
            b[n * 100:n * 100 + 50] = bytes(rnd.getrandbits(8) for _ in range(50))
            want[f"s{n:04d}"] = bytes(b)
            open(os.path.join(d, f"s{n:04d}.bin"), "wb").write(b)
            open(os.path.join(d, f"s{n:04d}.map"), "w").write("base s0000\n" if n else "")
        pack(d, 3, False)
        assert os.path.exists(os.path.join(d, "s0005.bin"))  # the newest waits
        pack(d, 3, True)
        assert not [f for f in os.listdir(d) if f.endswith(".bin")], os.listdir(d)
        assert all(unpack(d, k) == v for k, v in want.items())
        assert "zstd-from s0001" in open(os.path.join(d, "s0002.map")).read()
        assert "zstd-from s0004" in open(os.path.join(d, "s0005.map")).read()
        assert "zstd-from" not in open(os.path.join(d, "s0004.map")).read()
    print("dumppack self-test: ok")


def main():
    a = sys.argv[1:]
    if a == ["--self-test"]:
        return self_test()
    if not a:
        sys.exit(__doc__)
    d, every = a[0], int(a[a.index("--key") + 1]) if "--key" in a else 32
    if "--watch" in a:
        flag = a[a.index("--watch") + 1]
        while os.path.exists(flag):
            pack(d, every, False)
            time.sleep(10)
    pack(d, every, True)


if __name__ == "__main__":
    main()
