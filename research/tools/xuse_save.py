#!/usr/bin/env python3
"""Read and write 最果てのイマ (Xuse, Farthest2015.exe) save files.

usage:
  xuse_save.py dump SAVE                      print the save as JSON
  xuse_save.py write TEMPLATE JSON -o OUT [--cd CDFILES_DIR]

Slot saves are `Farthest2015_NNN.bin` (NNN = slot, 001-050), the global
save is `Farthest2015_000.bin`; format in docs/games/saihate-no-ima.md,
"Save format". `write` takes a save the game wrote as the template and
applies the JSON (a `dump` output, edited) to it, recomputing every CRC:
- slot: `comment`, `chapter`, `title`, `vars` (name: value, the per-slot
  variables), `scene.script` + `scene.index` (instruction number, as
  `conscripter-convert cd dump` prints it) or `scene.label` (`s9_l3`,
  looked up in `--cd`'s `<script>.cd`; also sets `chapter_open` -1, as a
  scene's entry label starts a chapter and an open one stops the game),
  `chapter_open`, `chapter_start`. The screen state (background,
  sprites, sound) and the thumbnails stay the template's.
- global: `vars` and each chunk's `hex` (CG/RMSN bits, ...).
Keys left out keep the template's value.
"""
import json, struct, sys, datetime

def crc(data):  # CRC-16/XMODEM (CCITT 0x1021, init 0), stored big-endian
    c = 0
    for b in data:
        c ^= b << 8
        for _ in range(8):
            c = ((c << 1) ^ 0x1021) & 0xffff if c & 0x8000 else (c << 1) & 0xffff
    return c

def sjis(b): return b.split(b'\0')[0].decode('cp932', 'replace')
def put_sjis(s, n):
    b = s.encode('cp932')
    assert len(b) < n, f'{s!r} longer than {n - 1} bytes'
    return b + b'\0' * (n - len(b))

def filetime(b):
    t = struct.unpack('<Q', b)[0]
    return (datetime.datetime(1601, 1, 1) + datetime.timedelta(microseconds=t // 10)).isoformat()

def svar_read(d, o):
    """SVAR: tag, u32 count, count x (u32 len, name ^ 0x65, i32 value), crc."""
    assert d[o:o + 4] == b'SVAR'
    n = struct.unpack_from('<I', d, o + 4)[0]; p = o + 8; v = {}
    for _ in range(n):
        ln = struct.unpack_from('<I', d, p)[0]
        name = bytes(c ^ 0x65 for c in d[p + 4:p + 4 + ln]).decode('cp932')
        v[name] = struct.unpack_from('<i', d, p + 4 + ln)[0]; p += 8 + ln
    assert struct.unpack_from('>H', d, p)[0] == crc(d[o + 8:p]), 'SVAR crc'
    return v, p + 2

def svar_bytes(v):
    body = b''.join(struct.pack('<I', len(k.encode('cp932'))) + bytes(c ^ 0x65 for c in k.encode('cp932'))
                    + struct.pack('<i', x) for k, x in v.items())  # names stay in the template's order
    return b'SVAR' + struct.pack('<I', len(v)) + body + struct.pack('>H', crc(body))

# ---- slot saves ----------------------------------------------------------
LOCL = 4 + 8 + 8 + 4 + 4 + 0x24 + 0x40          # tag + fields, then crc
SRIO = 4 + 0x78 + 1 + 0x80 + 12
LSNP = 4 + 0x24 + 4 * 4 + 1 + 13 * 4 + 0x172 + 4 + 0x50 + 0x50 + 4
P_SCRIPT, P_OFFSET = 4 + 0x24 + 17, 4 + 0x24 + 21  # in LSNP
P_STACK = 4 + 0x24 + 17 + 52 + 0x172 + 4           # 20 scripts, 20 return offsets, depth

def slot_parse(d):
    assert d[:4] == b'LOCL'
    assert struct.unpack_from('>H', d, LOCL)[0] == crc(d[:LOCL]), 'LOCL crc'
    p = LOCL + 2; thumbs = []
    for _ in range(2):  # 100x75 and 400x300 PNG, each after its u32 size
        n = struct.unpack_from('<I', d, p)[0]; thumbs.append(n); p += 4 + n
    o_rio = p; assert d[p:p + 4] == b'sRIO'
    assert struct.unpack_from('>H', d, p + SRIO)[0] == crc(d[p:p + SRIO]), 'sRIO crc'
    o_svar = p + SRIO + 2
    v, p = svar_read(d, o_svar)
    o_lsnl = p; assert d[p:p + 4] == b'LSNL'
    o_lsnp = p + 14 + 4; assert d[o_lsnp:o_lsnp + 4] == b'LSNP'
    assert struct.unpack_from('>H', d, o_lsnp + LSNP)[0] == crc(d[o_lsnp:o_lsnp + LSNP]), 'LSNP crc'
    return dict(rio=o_rio, svar=o_svar, lsnl=o_lsnl, lsnp=o_lsnp, svar_end=p, thumbs=thumbs, vars=v)

def slot_dump(d):
    o = slot_parse(d); q = o['lsnp']; r = o['rio']
    stack = struct.unpack_from('<20I20Ii', d, q + P_STACK)
    script, off = struct.unpack_from('<ii', d, q + P_SCRIPT)
    return {
        'kind': 'slot',
        'saved': filetime(d[4:12]), 'time2': filetime(d[12:20]),
        'version': struct.unpack_from('<i', d, 20)[0], 'unk24': struct.unpack_from('<i', d, 24)[0],
        'chapter': sjis(d[28:28 + 0x24]), 'comment': sjis(d[64:64 + 0x40]),
        'thumbnail_png_sizes': o['thumbs'],
        'title': sjis(d[r + 4 + 0x79:r + 4 + 0xf9]),
        'chapter_start': list(struct.unpack_from('<2i', d, r + 4 + 0xf9)),
        'chapter_open': struct.unpack_from('<i', d, r + 4 + 0xf9 + 8)[0],
        'vars': o['vars'],
        'scene': {'name': sjis(d[q + 4:q + 4 + 0x24]), 'script': script, 'offset': off, 'index': off // 12,
                  'call_depth': stack[40], 'call_scripts': list(stack[:20]), 'call_returns': list(stack[20:40])},
        'lsnl_count': struct.unpack_from('<I', d, o['lsnl'] + 4)[0],
        'history_entries': struct.unpack_from('<I', d, d.rfind(b'CPHS') + 4)[0] if b'CPHS' in d else None,
    }

def label_index(cd_dir, label):
    """(script, code byte offset) of `s<script>_l<label>` from the .cd's label table."""
    s, l = (int(x[1:]) for x in label.split('_'))
    b = open(f'{cd_dir}/{s:04d}.cd', 'rb').read()
    n = struct.unpack_from('<I', b, 0x124)[0]
    for i in range(n - 1):
        if struct.unpack_from('<3I', b, 0x13c + 12 * i)[:2] == (s, l):
            return s, struct.unpack_from('<3I', b, 0x13c + 12 * i)[2]
    sys.exit(f'{label}: not in {s:04d}.cd')

def slot_write(d, j, cd_dir):
    d = bytearray(d); o = slot_parse(bytes(d)); r = o['rio']; q = o['lsnp']
    if 'chapter' in j: d[28:28 + 0x24] = put_sjis(j['chapter'], 0x24)
    if 'comment' in j: d[64:64 + 0x40] = put_sjis(j['comment'], 0x40)
    struct.pack_into('>H', d, LOCL, crc(d[:LOCL]))
    if 'title' in j: d[r + 4 + 0x79:r + 4 + 0xf9] = put_sjis(j['title'], 0x80)
    sc = j.get('scene', {})
    if 'chapter_start' in j: struct.pack_into('<2i', d, r + 4 + 0xf9, *j['chapter_start'])
    # a chapter start (0x36) while one is open stops the game: a jump closes it
    if 'chapter_open' in j or 'label' in sc:
        struct.pack_into('<i', d, r + 4 + 0xf9 + 8, j.get('chapter_open', -1))
    struct.pack_into('>H', d, r + SRIO, crc(d[r:r + SRIO]))
    if 'label' in sc:
        sc = dict(sc); sc['script'], sc['offset'] = label_index(cd_dir, sc['label'])
    if 'script' in sc: struct.pack_into('<i', d, q + P_SCRIPT, sc['script'])
    if 'index' in sc or 'offset' in sc:
        struct.pack_into('<i', d, q + P_OFFSET, sc.get('offset', sc.get('index', 0) * 12))
    if 'name' in sc: d[q + 4:q + 4 + 0x24] = put_sjis(sc['name'], 0x24)
    if 'call_depth' in sc: struct.pack_into('<i', d, q + P_STACK + 160, sc['call_depth'])
    struct.pack_into('>H', d, q + LSNP, crc(d[q:q + LSNP]))
    v = dict(o['vars']); v.update(j.get('vars', {}))
    return bytes(d[:o['svar']]) + svar_bytes(v) + bytes(d[o['svar_end']:])

# ---- global save ---------------------------------------------------------
def glbl_chunks(d):
    """GLBL: chunks tag, u32 size, data, crc over tag+size+data; then SVAR, then
    OTHR to the end of the file with one crc."""
    assert d[:4] == b'GLBL'; p = 4; out = []
    while d[p:p + 4] not in (b'SVAR', b''):
        n = struct.unpack_from('<I', d, p + 4)[0]
        assert struct.unpack_from('>H', d, p + 8 + n)[0] == crc(d[p:p + 8 + n]), f'{d[p:p+4]} crc'
        out.append((d[p:p + 4], d[p + 8:p + 8 + n])); p += 10 + n
    v, p = svar_read(d, p)
    assert d[p:p + 4] == b'OTHR' and struct.unpack_from('>H', d, len(d) - 2)[0] == crc(d[p:-2]), 'OTHR crc'
    return out, v, d[p + 4:-2]

def glbl_dump(d):
    ch, v, othr = glbl_chunks(d)
    return {'kind': 'global', 'chunks': [{'tag': t.decode('latin1'), 'hex': b.hex()} for t, b in ch],
            'vars': v, 'othr_hex': othr.hex(), 'progress': struct.unpack_from('<I', othr, 14)[0]}

def glbl_write(d, j):
    ch, v, othr = glbl_chunks(d)
    ch = [(c['tag'].encode('latin1'), bytes.fromhex(c['hex'])) for c in j['chunks']] if 'chunks' in j else ch
    v.update(j.get('vars', {}))
    othr = bytearray(bytes.fromhex(j['othr_hex']) if 'othr_hex' in j else othr)
    if 'progress' in j: struct.pack_into('<I', othr, 14, j['progress'])
    out = b'GLBL'
    for t, b in ch:
        c = t + struct.pack('<I', len(b)) + b; out += c + struct.pack('>H', crc(c))
    tail = b'OTHR' + bytes(othr)
    return out + svar_bytes(v) + tail + struct.pack('>H', crc(tail))

def main(a):
    if len(a) == 2 and a[0] == 'dump':
        d = open(a[1], 'rb').read()
        print(json.dumps(glbl_dump(d) if d[:4] == b'GLBL' else slot_dump(d), ensure_ascii=False, indent=1))
    elif len(a) >= 5 and a[0] == 'write' and '-o' in a:
        d = open(a[1], 'rb').read(); j = json.load(open(a[2], encoding='utf-8'))
        cd = a[a.index('--cd') + 1] if '--cd' in a else '.'
        out = glbl_write(d, j) if d[:4] == b'GLBL' else slot_write(d, j, cd)
        (glbl_dump if out[:4] == b'GLBL' else slot_dump)(out)  # re-parse: every crc checks
        open(a[a.index('-o') + 1], 'wb').write(out)
    else:
        sys.exit(__doc__)

if __name__ == '__main__':
    main(sys.argv[1:])
