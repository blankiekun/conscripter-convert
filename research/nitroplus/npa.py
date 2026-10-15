"""List or extract a Nitroplus NPA archive (装甲悪鬼村正, 2009).

    python3 -I npa.py Muramasa.exe nss.npa             # list: size name
    python3 -I npa.py Muramasa.exe nss.npa OUT ['GLOB']  # extract (GLOB uses '\\')
    python3 -I npa.py Muramasa.exe nss.npa --pack DIR NEW.npa  # repack, files in DIR replaced

In place of the exe, a 256-byte file is taken as the table itself (Django's,
which its exe doesn't hold as plain bytes: `npa::DJANGO_TABLE`).

Header: "NPA" u32 version, u32 key1, u32 key2, u8 compressed, u8 encrypted,
u32 total, u32 folders, u32 files, u32, u32, u32 index size; data starts at
0x29 + index size. Entry: u32 name length, name, u8 type (1 dir, 2 file),
u32 file id, u32 offset, u32 stored size, u32 size.

Names: byte x += 0xFC*x - bytes(key1*key2) - bytes(entry index).
Data: the first 0x1000 + len(name) bytes are b = inv[b] - k - i, where inv is
the inverse of the 256-byte table at exe offset 0x134830 and
k = ((0x87654321 - sum(name)) * len(name) + key1*key2) * size  (low byte).
Then zlib if compressed. Checked on all 913 scripts.

--pack writes a copy with the same index (names, keys, order), taking each
file from DIR (same relative path, '/' for '\\') when it is there and from
the archive otherwise, compressed and encrypted the same way.
"""
import fnmatch
import os
import struct
import sys

TABLE_OFFSET = 0x134830


def header(d):
    _v, k1, k2, comp, enc, total, folders, files, _a, _b, size = struct.unpack_from('<IIIBBIIIIII', d, 3)
    return dict(k1=k1, k2=k2, comp=comp, enc=enc, total=total, folders=folders, files=files, start=0x29 + size)


def entries(d, h):
    key = (h['k1'] * h['k2']) & 0xffffffff
    p = 0x29
    for i in range(h['total']):
        n = struct.unpack_from('<I', d, p)[0]
        p += 4
        raw = bytearray(d[p:p + n])
        p += n
        for x in range(n):
            t = 0xFC * x - sum((key >> s) & 0xff for s in (24, 16, 8, 0)) - sum((i >> s) & 0xff for s in (24, 16, 8, 0))
            raw[x] = (raw[x] + t) & 0xff
        typ = d[p]
        fid, off, csz, osz = struct.unpack_from('<IIII', d, p + 1)
        p += 17
        yield bytes(raw), typ, off, csz, osz


def pack(inv, src, h, index, srcdir, dst):
    table = bytes(inv.index(i) for i in range(256))
    key = (h['k1'] * h['k2']) & 0xffffffff
    index = bytearray(index)
    blobs = []
    pos = 0
    p = 0x29
    replaced = 0
    for nb, typ, off, csz, osz in entries(bytes(index), h):
        p += 4 + len(nb)
        if typ == 2:
            path = os.path.join(srcdir, nb.decode('cp932').replace('\\', '/'))
            if os.path.exists(path):
                import zlib
                plain = open(path, 'rb').read()
                osz = len(plain)
                b = bytearray(zlib.compress(plain, 9) if h['comp'] else plain)
                if h['enc']:
                    k = ((0x87654321 - sum(nb)) * len(nb) + key) * osz & 0xff
                    for i in range(min(len(b), 0x1000 + len(nb))):
                        b[i] = table[(b[i] + k + i) & 0xff]
                b = bytes(b)
                replaced += 1
            else:
                src.seek(h['start'] + off)
                b = src.read(csz)
            struct.pack_into('<III', index, p + 5, pos, len(b), osz)
            blobs.append(b)
            pos += len(b)
        p += 17
    with open(dst, 'wb') as f:
        f.write(index)
        for b in blobs:
            f.write(b)
    print(f'{replaced} files replaced')


def main():
    exe = open(sys.argv[1], 'rb').read()
    table = exe if len(exe) == 256 else exe[TABLE_OFFSET:TABLE_OFFSET + 256]
    inv = bytes(table.index(i) for i in range(256))
    f = open(sys.argv[2], 'rb')
    h = header(f.read(0x29))
    f.seek(0)
    index = f.read(h['start'])
    out = sys.argv[3] if len(sys.argv) > 3 else None
    pat = sys.argv[4] if len(sys.argv) > 4 else '*'
    if out == '--pack':
        pack(inv, f, h, index, sys.argv[4], sys.argv[5])
        return
    key = (h['k1'] * h['k2']) & 0xffffffff
    for nb, typ, off, csz, osz in entries(index, h):
        name = nb.decode('cp932')
        if typ != 2:
            continue
        if not out:
            print(f'{osz:10d} {name}')
            continue
        if not fnmatch.fnmatch(name, pat):
            continue
        f.seek(h['start'] + off)
        b = bytearray(f.read(csz))
        if h['enc']:
            k = 0x87654321 - sum(nb)
            k = ((k * len(nb) + key) * osz) & 0xff
            for i in range(min(len(b), 0x1000 + len(nb))):
                b[i] = (inv[b[i]] - k - i) & 0xff
        if h['comp']:
            import zlib
            b = zlib.decompress(b)
        assert len(b) == osz, name
        path = os.path.join(out, name.replace('\\', '/'))
        os.makedirs(os.path.dirname(path) or '.', exist_ok=True)
        open(path, 'wb').write(b)


if __name__ == '__main__':
    main()
