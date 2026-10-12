import sys, os, struct
sys.path.insert(0, os.path.dirname(__file__))
from gafkey import gaf_key, decrypt

def entries(path):
    b = open(path, 'rb').read()
    d = b[:0x4a] + decrypt(b[0x4a:], gaf_key(b[6:0x46].split(b"\0")[0]), 0x4a, 0)
    out = []; i = 0
    while True:
        p = d.find(b'\x89PNG\r\n\x1a\n', i); j = d.find(b'\xff\xd8\xff', i)
        cands = [x for x in (p, j) if x >= 0]
        if not cands: break
        s = min(cands)
        if s == p:
            k = s + 8
            while k + 8 <= len(d):
                n, t = struct.unpack_from('>I4s', d, k); k += 12 + n
                if t == b'IEND': break
            out.append(('png', d[s:k])); i = k
        else:
            e = d.find(b'\xff\xd9', s + 2)
            out.append(('jpg', d[s:e + 2])); i = e + 2
    return struct.unpack_from('<I', d, 0x46)[0], out
if __name__ == '__main__':
    outdir = sys.argv[1]
    for path in sys.argv[2:]:
        count, es = entries(path)
        stem = os.path.splitext(os.path.basename(path))[0]
        for n, (kind, data) in enumerate(es):
            open(os.path.join(outdir, f'{stem}_{n:03d}.{kind}'), 'wb').write(data)
        print(f'{os.path.basename(path)}: count {count}, images {len(es)} ({", ".join(sorted({k for k, _ in es}))})')
