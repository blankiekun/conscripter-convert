"""Recovers an NPA archive's data substitution table by known plaintext.

Usage: python3 -I npa_table.py <archive with Ogg files> [...]

Uses Muramasa's key formula (npa.rs) and the fixed bytes of an Ogg
Vorbis identification header (and the zlib header 78 9c) to collect
table[plain + k + i] = cipher pairs, checks they never contradict and
prints the table as hex (256 bytes) once every slot is known.
"""
import struct, sys

def bsum(v): return sum((v & 0xffffffff).to_bytes(4, 'little'))

OGG = {0: b'O', 1: b'g', 2: b'g', 3: b'S', 4: 0, 5: 2, 18: 0, 19: 0, 20: 0, 21: 0,
       26: 1, 27: 0x1e, 28: 1, 35: 0, 36: 0, 37: 0, 38: 0}
for j in range(6, 14): OGG[j] = 0
for j, c in enumerate(b'vorbis'): OGG[29 + j] = c
OGG = {k: (v[0] if isinstance(v, bytes) else v) for k, v in OGG.items()}

table = {}
bad = 0
for path in sys.argv[1:]:
    d = open(path, 'rb').read()
    k1, k2 = struct.unpack_from('<II', d, 7)
    key12 = (k1 * k2) & 0xffffffff
    comp, enc = d[15], d[16]
    total = struct.unpack_from('<I', d, 17)[0]
    start = 0x29 + struct.unpack_from('<I', d, 37)[0]
    p = 0x29
    for i in range(total):
        n = struct.unpack_from('<I', d, p)[0]
        raw = bytearray(d[p + 4:p + 4 + n])
        for x in range(n):
            raw[x] = (raw[x] + 0xFC * x - bsum(key12) - bsum(i)) & 0xff
        p += 4 + n
        kind = d[p]
        off, stored, size = struct.unpack_from('<III', d, p + 5)
        p += 17
        if kind != 2: continue
        k = (((0x87654321 - sum(raw)) * n + key12) * size) & 0xff
        known = {0: 0x78, 1: 0x9c} if comp else (OGG if raw.lower().endswith(b'.ogg') else {})
        for j, plain in known.items():
            c = d[start + off + j]
            slot = (plain + k + j) & 0xff
            if table.setdefault(slot, c) != c: bad += 1
print(f'{len(table)} slots known, {bad} contradictions, values unique: {len(set(table.values())) == len(table)}', file=sys.stderr)
if len(table) == 256: print(bytes(table[s] for s in range(256)).hex())
