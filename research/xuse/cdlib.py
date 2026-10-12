import struct, os
def load(path):
    b = open(path,'rb').read()
    cnt, code, pool = struct.unpack_from('<3I', b, 0x124)
    cs = 0x13c + 12*(cnt-1)
    ins = [struct.unpack_from('<HHiI', b, cs+12*i) for i in range(code//12)]
    labels = [struct.unpack_from('<3I', b, 0x13c+12*i) for i in range(cnt-1)]
    p = b[cs+code: cs+code+pool]
    return labels, ins, p
def kana(s): return sum('\u3040' <= c <= '\u30ff' or c in '、。ー' for c in s) / max(1, len(s))
def scan(p):
    out = {}; i = 0
    while i + 4 <= len(p):
        L = struct.unpack_from('<I', p, i)[0]
        if 2 <= L <= 4096 and i+4+L <= len(p):
            try:
                s = bytes(x ^ 0x53 for x in p[i+4:i+4+L]).decode('cp932')
                if kana(s) > 0.2 or all('\u3000' <= c <= '\u9fff' or '\uff00' <= c <= '\uffef' for c in s):
                    out[i] = s; i += 4+L; continue
            except UnicodeDecodeError: pass
        i += 1
    return out

def try_str(p, i, minlen=2):
    if i + 4 > len(p): return None
    L = struct.unpack_from('<I', p, i)[0]
    if not (minlen <= L <= 8192) or i + 4 + L > len(p): return None
    raw = bytes(x ^ 0x53 for x in p[i+4:i+4+L])
    try: s = raw.decode('cp932')
    except UnicodeDecodeError: return None
    if any(ord(c) < 0x20 and c not in '\r\n\t' for c in s): return None
    return s, 4 + L

def tokens(p, start, end):
    """Split pool[start:end] into ('i', u32) and ('s', nbytes, text) tokens."""
    out = []; i = start
    while i < end:
        st = try_str(p, i)
        if st and i + st[1] <= end:
            out.append(('s', st[1]-4, st[0])); i += st[1]; continue
        if i + 4 <= end:
            out.append(('i', struct.unpack_from('<i', p, i)[0])); i += 4
        else:
            out.append(('b', p[i:end].hex())); i = end
    return out

def fmt(toks):
    return ' '.join(f'"{t[1]}B"' if t[0]=='s' else (str(t[1]) if t[0]=='i' else 'b:'+t[1]) for t in toks)
