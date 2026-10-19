#!/usr/bin/env python3
"""Read and write the saves of the Nitroplus NSS games (装甲悪鬼村正, 続・殺戮のジャンゴ).

usage:
  nitroplus_save.py dump EXE DIR                    print a save folder as JSON
  nitroplus_save.py write EXE TEMPLATE JSON -o OUT  a copy of TEMPLATE with JSON applied
  nitroplus_save.py selftest EXE DIR                decode and re-encode every file

EXE is the game's exe (its byte table decrypts the files; Django's exe
doesn't hold it as plain bytes, so `npa::DJANGO_TABLE` is read from
crates/conv-engines/src/nitroplus/npa.rs). DIR is a slot folder
(`<save dir>/0001`, the game's 記録 slot 1) or `common` (the global
variables). Format: docs/games/muramasa.md, "Save format".

`write` takes a folder the game wrote as the template; JSON keys
(each optional; a `dump` output works, keys left out keep the
template's):
- `vars`: name -> value (number, true/false, string, null). `$` names
  in a slot's val.npf, `#` names in common/val.npf. New names are added.
- `comment`, `chapter`, `date`, `time`: the texts the load screen shows.
- `scene` (装甲悪鬼村正 only): a scene file (`ma05_023.nss`): the game
  goes there one click after loading. The template must stand on a page of a scene; its
  stored script text (the game recompiles the saved text, not nss.npa) is
  cut after that page and the scene's _MAIN then sets `$GameName` to the
  target and returns to the boot loop, which starts the target at its
  first line. Code before the current position stays byte-identical, so
  the saved positions stay valid. A template saved at a choice won't do.
"""
import json, os, re, struct, sys, zlib

MAGIC = b'NPSF'
MURAMASA = bytes.fromhex('c557f2f1')  # second magic of 装甲悪鬼村正's inner files (Django: c35ffcf1)


def table(exe):
    t = open(exe, 'rb').read()[0x134830:0x134830 + 256]   # 装甲悪鬼村正 (npa.py)
    if len(set(t)) == 256:
        return t
    src = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../crates/conv-engines/src/nitroplus/npa.rs')
    m = re.search(r'DJANGO_TABLE: \[u8; 256\] = \[(.*?)\];', open(src).read(), re.S)
    t = bytes(int(x, 16) for x in re.findall(r'0x([0-9a-f]{2})', m.group(1)))
    assert len(set(t)) == 256
    return t


def unpack(tab, raw):
    """NPSF u32 size, u32 1 (encrypted), u32 1 (zlib), data: the first
    0x1000 bytes are table[b + key]; key is per file kind and comes back
    from the zlib header (first plain byte 0x78)."""
    assert raw[:4] == MAGIC and struct.unpack_from('<II', raw, 8) == (1, 1), 'not an npf file'
    inv = bytes(tab.index(i) for i in range(256))
    key = (inv[raw[16]] - 0x78) & 0xff
    b = bytearray(raw[16:])
    for i in range(min(len(b), 0x1000)):
        b[i] = (inv[b[i]] - key) & 0xff
    plain = zlib.decompress(bytes(b))
    assert len(plain) == struct.unpack_from('<I', raw, 4)[0]
    return plain, key


def pack(tab, plain, key):
    b = bytearray(zlib.compress(plain, 9))
    for i in range(min(len(b), 0x1000)):
        b[i] = tab[(b[i] + key) & 0xff]
    return MAGIC + struct.pack('<III', len(plain), 1, 1) + bytes(b)


# ---- val.npf: NPSL (slot) / NPSG (common) -------------------------------
def cstr(b, p):
    e = b.index(b'\0', p)
    return b[p:e], e


def vals_read(d):
    """tag, u32 magic, u32 size, u32 count, count x 1024-byte entries:
    u8 type (0 null, 1 string, 2 number), name, 4 x NUL, f64 number,
    string, 4 x NUL, array key (`$X/0` elements), 4 x NUL."""
    n = struct.unpack_from('<I', d, 12)[0]
    out = {}
    for i in range(n):
        e = d[16 + i * 1024:16 + (i + 1) * 1024]
        name, k = cstr(e, 1)
        num = struct.unpack_from('<d', e, k + 4)[0]
        s, k2 = cstr(e, k + 12)
        key, _ = cstr(e, k2 + 4)
        out[name.decode('cp932')] = (e[0], num, s.decode('cp932'), key.decode('cp932'))
    return out


def vals_write(head, v):
    body = b''
    for name, (t, num, s, key) in v.items():
        e = bytes([t]) + name.encode('cp932') + b'\0' * 4 + struct.pack('<d', num) + s.encode('cp932') \
            + b'\0' * 4 + key.encode('cp932') + b'\0' * 4
        assert len(e) <= 1024, name
        body += e + b'\0' * (1024 - len(e))
    return head[:8] + struct.pack('<II', 16 + len(body), len(v)) + body


def val_json(t, num, s, key):
    if t == 0:
        return None
    if t == 1:
        return s
    return int(num) if num == int(num) else num


def val_set(old, x):
    key = old[3] if old else ''
    if x is None:
        return (0, 0.0, '', key)
    if isinstance(x, str):
        return (1, 0.0, x, key)
    num = float(x)
    s = ('%d' % num if num == int(num) else repr(num)) if old and old[2] else ''
    return (2, num, s, key)


# ---- script.npf: NPSS, the program's state --------------------------------
def modules(d):
    """u32 module count; each module (call_chapter level) starts at
    16 + text bytes so far + (records so far + module index) * 0x200:
    0x200 file slots (u32 0, or u32 id, name, 4 x NUL, u32 length, text),
    then 7 u32 (module, ?, ?, ?, current record, two links), then one
    record per function/scene/chapter in compile order (name, NUL, 9 u32,
    16 parameter names, u32 position, 4 x 8 u32 positions, 10 u32 flags,
    16 local strings, 2 u32 links). The position is how far the function's
    code is compiled: the game compiles the saved texts as they run.
    This is 装甲悪鬼村正's layout; Django's (0x40 file slots, 0x400) differs."""
    u = lambda p: struct.unpack_from('<I', d, p)[0]
    nmod, texts, nrec, p = u(12), 0, 0, 16
    out = []
    for m in range(nmod):
        p = 16 + texts + (nrec + m) * 0x200
        files = []
        for _ in range(0x200):
            fid = u(p); p += 4
            if fid == 0:
                continue
            name, e = cstr(d, p)
            n = u(e + 4)
            files.append(dict(name=name.decode('cp932'), at=e + 8, size=n))
            texts += n; p = e + 8 + n
        hdr = [u(p + 4 * i) for i in range(7)]; p += 28
        recs = []
        while p + 40 < len(d):
            name, q = cstr(d, p)
            v = [u(q + 4 * i) for i in range(9)]
            if v[0] != 0 or v[1] == 0:
                break
            q += 36
            params = []
            for _ in range(16):
                if u(q) == 1:
                    s, q = cstr(d, q + 4); params.append(s.decode('cp932', 'replace'))
                q += 4
            pos = u(q); q += 4 + 128 + 40
            for _ in range(16):
                q += 4 + u(q)
            recs.append(dict(index=len(recs), ptr=v[1], name=name.decode('cp932', 'replace'), pos=pos,
                             params=params))
            p = q + 8
        nrec += len(recs)
        out.append(dict(files=files, current=hdr[4], records=recs))
    return out


def braces(t, a, b):
    """Brace depth change over t[a:b], skipping comments and strings."""
    depth = 0
    for m in re.finditer(r'//[^\n]*|/\*.*?\*/|"[^"\n]*"|[{}]', t[a:b], re.S):
        if m.group() == '{': depth += 1
        elif m.group() == '}': depth -= 1
    return depth


def block_end(t, open_at):
    depth = 0
    for m in re.finditer(r'//[^\n]*|/\*.*?\*/|"[^"\n]*"|[{}]', t[open_at:], re.S):
        if m.group() == '{': depth += 1
        elif m.group() == '}':
            depth -= 1
            if depth == 0:
                return open_at + m.start()
    raise ValueError('unbalanced block')


def retarget(text, scene, page, target):
    """Muramasa's scene file: `scene S_MAIN {... call_scene $MainGameName;
    ... $GameName = "next";}` then `scene S {... <PRE>[textNNNN]...</PRE> ...}`.
    Cut S after the current page and _MAIN after its call_scene."""
    t = text.decode('cp932')
    m = re.search(r'\nscene\s+' + re.escape(scene) + r'\s*\{', t)
    end = block_end(t, m.end() - 1)
    pg = t.index('[%s]' % page, m.end())
    cut = t.index('</PRE>', pg) + len('</PRE>')
    assert cut < end, 'page outside the scene'
    t = t[:cut] + '\n' + '}' * braces(t, m.end(), cut) + t[end:]
    mm = re.search(r'\nscene\s+' + re.escape(scene) + r'_MAIN\s*\{', t)
    mend = block_end(t, mm.end() - 1)
    call = re.search(r'call_scene\s+\$MainGameName\s*;', t[mm.end():mend])
    c = mm.end() + call.end()
    tail = '\n\t\tGameMainSet2();' if 'GameMainSet2' in t[c:c + 80] else ''
    t = t[:c] + tail + '\n' + '}' * braces(t, mm.end(), c) + '\n\t$GameName = "%s";\n' % target + t[mend:]
    return t.encode('cp932')


def blank(b):
    """bytes as spaces, line breaks kept: code gone, every later offset and line the same"""
    return re.sub(rb'[^\r\n]', b' ', b)


def last_page_django(text, cut, page):
    """True when page is the last text block of Django's cut C: a save there
    needs no cut (blanking the rest of a cut, as Muramasa's retarget cuts,
    hangs the load: its compiled records reach past the resume point)"""
    t = text.decode('cp932')
    m = re.search(r'\ncut\s+' + re.escape(cut) + r'\s*\{', t)
    end = block_end(t, m.end() - 1)
    pg = t.index('[%s]' % page, m.end())
    return '<PRE' not in t[pg:end]


def call_next(text, called, target):
    """In a saved file that `call_chapter`s `called`: the calls after it in
    its block become one call of target, blanked to the same length in
    bytes. None when no call follows it there or the room is too small."""
    h = text.decode('cp932')
    me = re.search(r'call_chapter\s+nss/' + re.escape(called) + r'[^\n]*', h)
    if not me:
        return None
    bend = block_end(h, h.rfind('{', 0, me.start()))
    nxt = re.compile(r'call_chapter\s+nss/').search(h, me.end(), bend)
    if not nxt:
        return None
    a, b = len(h[:nxt.start()].encode('cp932')), len(h[:bend].encode('cp932'))
    call = ('call_chapter nss/' + target + ';').encode('cp932')
    room = blank(text[a:b])
    if len(call) > len(room):
        return None
    return text[:a] + call + room[len(call):] + text[b:]


def script_files(d):
    """(name, offset, length) of every saved script text: `nss\\x.nss`, 4 x NUL, u32 length, text."""
    out = []
    for m in re.finditer(rb'nss\\[^\0\\]{1,200}?\.nss\0\0\0\0', d):
        n = struct.unpack_from('<I', d, m.end())[0]
        if m.end() + 4 + n <= len(d):
            out.append((m.group()[:-4].decode('cp932'), m.end() + 4, n))
    return out


def script_replace(d, name, new):
    """d with the saved text of `name` replaced by new (the total size kept right)"""
    at, size = [(a, n) for f, a, n in script_files(d) if f.lower() == ('nss\\' + name).lower()][-1]
    d = d[:at - 4] + struct.pack('<I', len(new)) + new + d[at + size:]
    return d[:8] + struct.pack('<I', struct.unpack_from('<I', d, 8)[0] + len(new) - size) + d[12:]


def script_text(d, name):
    at, size = [(a, n) for f, a, n in script_files(d) if f.lower() == ('nss\\' + name).lower()][-1]
    return d[at:at + size]


def script_retarget_django(d, scene, cut, page, target):
    """Django: the template stands on the last page of its scene (no cut);
    from the innermost saved file calling the scene outwards (the chapter
    file, then dja000's main_root), the first with a later call goes to the
    target instead"""
    if not last_page_django(script_text(d, scene + '.nss'), cut, page):
        sys.exit(f'scene: a Django template must stand on the last page of its scene ({scene} {page} is not)')
    called = scene + '.nss'
    for f, a, n in reversed(script_files(d)):
        new = call_next(d[a:a + n], called, target)
        if new is not None:
            return script_replace(d, f[4:], new)
        if ('call_chapter nss/' + called).encode('cp932') in d[a:a + n]:
            called = f[4:]  # no room after it here: the file that called this one
    sys.exit('no saved file goes on after ' + scene)


def script_retarget(d, scene, page, target):
    at, size = [(a, n) for name, a, n in script_files(d) if name.lower() == ('nss\\' + scene).lower()][-1]
    new = retarget(d[at:at + size], scene, page, target)
    delta = len(new) - size
    d = d[:at - 4] + struct.pack('<I', len(new)) + new + d[at + size:]
    return d[:8] + struct.pack('<I', struct.unpack_from('<I', d, 8)[0] + delta) + d[12:]


# ---- folders ----------------------------------------------------------------
TEXTS = dict(comment='cmt', chapter='cpt', date='date', time='tm')
KEYS = dict(cmt=93, cpt=72, date=8, tm=52)  # the fixed key of a text file the template lacks (a 9999 save)


def read_dir(tab, path):
    return {f[:-4]: unpack(tab, open(os.path.join(path, f), 'rb').read())
            for f in sorted(os.listdir(path)) if f.endswith('.npf')}


def dump(tab, path):
    files = read_dir(tab, path)
    out = {'files': sorted(files)}
    for k, f in TEXTS.items():
        if f in files:
            out[k] = files[f][0].decode('cp932')
    v = vals_read(files['val'][0]) if 'val' in files else {}
    out['vars'] = {n: val_json(*x) for n, x in v.items()}
    if 'script' in files:
        d = files['script'][0]
        pos = {'scene': out['vars'].get('$GameName'), 'page': out['vars'].get('$SYSTEM_present_text'),
               'texts': [n for n, _, _ in script_files(d)]}
        if d[4:8] == MURAMASA:
            pos['modules'] = [{'files': [f['name'] for f in m['files']],
                               'current': next((r['index'] for r in m['records'] if r['ptr'] == m['current']), None),
                               'records': len(m['records'])} for m in modules(d)]
        out['position'] = pos
    return out


def write(tab, src, edit, dst):
    files = read_dir(tab, src)
    plain = {k: v[0] for k, v in files.items()}
    for k, f in TEXTS.items():
        if k in edit:
            plain[f] = edit[k].encode('cp932')
    v = vals_read(plain['val'])
    for n, x in edit.get('vars', {}).items():
        v[n] = val_set(v.get(n), x)
    if 'scene' in edit:
        cur = v['$GameName'][2] if '$GameName' in v else None
        if plain['script'][4:8] != MURAMASA:  # 続・殺戮のジャンゴ: the scene file, its cut, the page
            cur = v['$SYSTEM_spt_name'][2]
            if edit['scene'] != cur + '.nss':
                plain['script'] = script_retarget_django(plain['script'], cur, v['$単元名'][2],
                                                         v['$textdecor'][2].lstrip('@'), edit['scene'])
        elif edit['scene'] != cur:
            page = v['$SYSTEM_present_text'][2].lstrip('@')
            plain['script'] = script_retarget(plain['script'], cur, page, edit['scene'])
    plain['val'] = vals_write(plain['val'], v)
    os.makedirs(dst, exist_ok=True)
    for f in os.listdir(src):    # thumbnails' images/ and other files are copied as they are
        if not f.endswith('.npf') and os.path.isfile(os.path.join(src, f)):
            open(os.path.join(dst, f), 'wb').write(open(os.path.join(src, f), 'rb').read())
    for k, p in plain.items():
        open(os.path.join(dst, k + '.npf'), 'wb').write(pack(tab, p, files[k][1] if k in files else KEYS[k]))


def selftest(tab, path):
    for k, (p, key) in read_dir(tab, path).items():
        assert unpack(tab, pack(tab, p, key))[0] == p, k
        if k == 'val':
            assert vals_write(p, vals_read(p)) == p, 'val round trip'
        if k == 'script' and p[4:8] == MURAMASA:
            mods = modules(p)
            assert all(any(r['ptr'] == m['current'] for r in m['records']) or not m['current'] for m in mods)
            v = vals_read(read_dir(tab, path)['val'][0])
            scene, page = v['$GameName'][2], v['$SYSTEM_present_text'][2].lstrip('@')
            q = script_retarget(p, scene, page, 'x.nss')
            old, new = [[(n, x[a:a + l]) for n, a, l in script_files(x)] for x in (p, q)]
            assert [n for n, _ in old] == [n for n, _ in new] and struct.unpack_from('<I', q, 8)[0] == len(q)
            for (n, a), (_, b) in zip(old, new):   # code before the current positions is untouched
                if not n.endswith(scene):
                    assert a == b, n
                    continue
                c = a.index(b'call_scene $MainGameName;') + 25
                s0 = re.search(rb'\nscene\s+' + re.escape(scene.encode()) + rb'\s*\{', a).start()
                assert a[:c] == b[:c] and a[s0:a.index(b'</PRE>', a.index(b'[%s]' % page.encode()))] in b
    print('ok', path)


def main(a):
    if len(a) < 3:
        sys.exit(__doc__)
    tab = table(a[1])
    if a[0] == 'dump':
        print(json.dumps(dump(tab, a[2]), ensure_ascii=False, indent=1))
    elif a[0] == 'write':
        write(tab, a[2], json.load(open(a[3], encoding='utf-8')), a[a.index('-o') + 1])
    elif a[0] == 'selftest':
        selftest(tab, a[2])
    else:
        sys.exit(__doc__)


if __name__ == '__main__':
    main(sys.argv[1:])
