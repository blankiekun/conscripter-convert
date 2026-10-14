"""Muramasa.exe builds its strings on the stack one byte at a time
(`local_100c = 'C'; local_100b = 0x72; ...`), and the compiler stores the
bytes out of order, so the strings aren't in the file. Prints, per
decompiled function, the strings those byte stores spell once they are
put back in stack order: `<address>\t<string>` (cp932 decoded).

    python3 -I research/nitroplus/stackstrings.py <ghidra export>/decomp"""
import os, re, sys

d = sys.argv[1]
store = re.compile(r"^\s*local_([0-9a-f]+)\s*=\s*(0x[0-9a-f]{1,2}|\d{1,3}|'(?:\\.|[^'])');\s*$")
ESC = {'\\0': 0, '\\n': 10, '\\t': 9, "\\'": 39, '\\\\': 92}


def byte(v):
    if v.startswith("'"):
        c = v[1:-1]
        return ESC.get(c, ord(c) if len(c) == 1 else None)
    return int(v, 0)


def strings(cluster):
    """Contiguous stack runs in one cluster of stores (offset -> byte)."""
    out, cur, prev = [], bytearray(), None
    for off in sorted(cluster, reverse=True):  # lowest address first
        b = cluster[off]
        if prev is not None and off != prev - 1:
            out.append(bytes(cur)); cur = bytearray()
        prev = off
        if b == 0:
            out.append(bytes(cur)); cur = bytearray(); prev = None
        else:
            cur.append(b)
    out.append(bytes(cur))
    return [s for s in out if len(s) >= 3]


for f in sorted(os.listdir(d)):
    cluster, gap, found = {}, 0, []
    for line in open(os.path.join(d, f), encoding='utf-8', errors='replace'):
        m = store.match(line)
        b = byte(m.group(2)) if m else None
        if m and b is not None and b <= 255:
            cluster[int(m.group(1), 16)] = b
            gap = 0
            continue
        gap += 1
        if gap > 2 and cluster:  # a few unrelated lines may sit between stores
            found += strings(cluster); cluster = {}
    if cluster:
        found += strings(cluster)
    for s in found:
        try:
            t = s.decode('cp932')
        except UnicodeDecodeError:
            continue
        if t.isprintable():
            print(f'{f[:8]}\t{t}')
