#!/usr/bin/env python3
"""Write 1.0.2's `main` NSO with the trace hook, for an ExeFS mod.

    python3 -I mkmain.py <1.0.2 exefs/main> <hook.bin> <out main>

The text segment is lengthened over the zeros before the rodata page
(0x1542c0 to 0x155000, mapped executable all the same) to hold hook.s
(linked at 0x2542c0), and `script_command`'s loop head (0x177828) branches
to it. The segments are written uncompressed and unhashed; the build ID
stays, so Ryubing takes it as `main`. Addresses are Ghidra's (0x100000 on).
"""
import hashlib
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../tools/ghidra'))
from nso2elf import lz4_block  # noqa: E402

BASE = 0x100000
HOOK_AT, CAVE = 0x177828, 0x2542c0
MAIN_SHA256 = 'c128d79d3abd02b5581348618aa6403fdd651e77f4955f99ae43b34b90395faa'  # 1.0.2


def main(src, hook, dst):
    d = open(src, 'rb').read()
    if hashlib.sha256(d).hexdigest() != MAIN_SHA256:
        sys.exit("not 1.0.2's main")
    flags = struct.unpack_from('<I', d, 0xC)[0]
    segs = []
    for k, (hdr, csize_at) in enumerate([(0x10, 0x60), (0x20, 0x64), (0x30, 0x68)]):
        foff, moff, size = struct.unpack_from('<III', d, hdr)
        raw = d[foff:foff + struct.unpack_from('<I', d, csize_at)[0]]
        segs.append([moff, bytearray(lz4_block(raw, size) if flags >> k & 1 else raw[:size])])
    text = segs[0][1]
    code = open(hook, 'rb').read()
    cave = CAVE - BASE
    assert len(text) == cave and cave + len(code) <= segs[1][0], 'no room before the rodata'
    assert text[HOOK_AT - BASE:HOOK_AT - BASE + 4] == bytes.fromhex('e8031faa'), 'not mov x8, xzr'
    text += code
    text[HOOK_AT - BASE:HOOK_AT - BASE + 4] = struct.pack('<I', 0x14000000 | ((CAVE - HOOK_AT) >> 2 & 0x3ffffff))
    h = bytearray(d[:0x100])
    struct.pack_into('<I', h, 0xC, 0)  # uncompressed, no hash checks
    off = 0x101  # the module name (1 byte) at 0x100, as in the original
    for k, (moff, data) in enumerate(segs):
        struct.pack_into('<III', h, 0x10 + 0x10 * k, off, moff, len(data))
        struct.pack_into('<I', h, 0x60 + 4 * k, len(data))
        h[0xA0 + 0x20 * k:0xC0 + 0x20 * k] = hashlib.sha256(data).digest()
        off += len(data)
    with open(dst, 'wb') as f:
        f.write(h + d[0x100:0x101] + b''.join(s[1] for s in segs))


if __name__ == '__main__':
    main(*sys.argv[1:])
