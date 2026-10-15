"""List or extract a Nitroplus NPK3 archive (Full Metal Daemon Muramasa,
JAST USA 2021, the "Diesel" engine port: muramasa_en.exe).

    python3 -I npk.py system.npk              # list: size name
    python3 -I npk.py system.npk OUT ['GLOB'] # extract

Header: "NPK3", u32 version (1), 16-byte IV, u32 entries, u32 index size;
the index follows, AES-256-CBC with KEY and that IV. KEY is stored in the
exe as eight dword immediates (0x401c28, passed to the archive opener
0x54cf40), not as plain bytes.
Entry: u8 flag, u16 name length, name (UTF-8), u32 size, 32-byte hash,
u32 segments, then per segment u64 offset, u32 stored size (16-aligned),
u32 packed size, u32 size. A segment is AES-256-CBC (same key) and then
zstd-compressed when packed size < size (otherwise stored as is:
the zstd frame came out bigger).
"""
import fnmatch
import os
import struct
import sys

import zstandard
from Crypto.Cipher import AES

KEY = struct.pack('<8I', 0x9e1a8351, 0x5ea8ef69, 0xc251c5e6, 0x0418db08,
                  0x235037b4, 0x5e1fae56, 0x8828b129, 0xa8783e40)


def read_index(f):
    head = f.read(32)
    assert head[:4] == b'NPK3', head[:4]
    iv = head[8:24]
    count, size = struct.unpack_from('<II', head, 24)
    idx = AES.new(KEY, AES.MODE_CBC, iv).decrypt(f.read(size))
    pos, entries = 0, []
    for _ in range(count):
        flag, n = struct.unpack_from('<BH', idx, pos); pos += 3
        name = idx[pos:pos + n].decode(); pos += n
        size, = struct.unpack_from('<I', idx, pos); pos += 4 + 32
        nseg, = struct.unpack_from('<I', idx, pos); pos += 4
        segs = []
        for _ in range(nseg):
            segs.append(struct.unpack_from('<QIII', idx, pos)); pos += 20
        entries.append((name, flag, size, segs))
    return iv, entries


def zstd(d, size):
    # some frames don't record their size, so stream them
    return zstandard.ZstdDecompressor().decompressobj().decompress(d)


def read_entry(f, iv, segs, decode=zstd):
    out = b''
    for off, stored, packed, size in segs:
        f.seek(off)
        d = AES.new(KEY, AES.MODE_CBC, iv).decrypt(f.read(stored))
        out += d[:size] if packed >= size else decode(d[:packed], size)
    return out


if __name__ == '__main__':
    with open(sys.argv[1], 'rb') as f:
        iv, entries = read_index(f)
        if len(sys.argv) < 3:
            for name, flag, size, segs in entries:
                print(size, flag, len(segs), name)
            sys.exit()
        glob = sys.argv[3] if len(sys.argv) > 3 else '*'
        for name, flag, size, segs in entries:
            if not fnmatch.fnmatch(name, glob):
                continue
            data = read_entry(f, iv, segs)
            path = os.path.join(sys.argv[2], name)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            open(path, 'wb').write(data)
