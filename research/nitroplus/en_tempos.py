"""Pairs the NSS's Fade/DrawTransition tempos with the English port's at
the same calls (same scene, sprite and order) and counts each pair.

    python3 -I research/nitroplus/en_tempos.py <nss dir> <script.npk's media/script>
"""
import re, sys, os, glob, collections, importlib.util
NSS, NUT = sys.argv[1], sys.argv[2]
spec = importlib.util.spec_from_file_location('nut', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'nut.py'))
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
TEMPO = re.compile(r'(Axl|Dxl|AxlDxl|DxlAxl)\d[a-d]?$|Linear$')

def nss_calls(path):
    t = open(path, encoding='cp932', errors='replace').read()
    out = collections.defaultdict(list)
    for mm in re.finditer(r'\b(Fade|DrawTransition)\s*\(([^;]*)\)\s*;', t):
        a = [x.strip() for x in mm.group(2).split(',')]
        k = mm.group(1); ti = 3 if k == 'Fade' else 5
        if len(a) > ti: out[(k, a[0].strip('"'))].append(a[ti])
    return out

def walk(f):
    yield f
    for g in f['functions']: yield from walk(g)

def nut_calls(path):
    out = collections.defaultdict(list)
    for f in walk(m.load(path)):
        lit, code = f['literals'], f['code']
        for i, (op, a0, a1, a2, a3) in enumerate(code):
            if (m.OPS[op] if op < len(m.OPS) else '') != 'PREPCALLK' or not (0 <= a1 < len(lit)) or lit[a1] not in ('Fade', 'Transition'): continue
            k = 'Fade' if lit[a1] == 'Fade' else 'DrawTransition'
            name, tempo = None, 'null'
            for op2, b0, b1, b2, b3 in code[i + 1:i + 14]:
                n = m.OPS[op2] if op2 < len(m.OPS) else ''
                if n == 'CALL': break
                refs = [b1] + ([b3] if n == 'DLOAD' else [])
                for r in refs:
                    if n in ('LOAD', 'GETK', 'DLOAD') and 0 <= r < len(lit) and isinstance(lit[r], str):
                        if name is None and n == 'LOAD': name = lit[r]
                        elif TEMPO.match(lit[r]): tempo = lit[r]
            out[(k, name)].append(tempo)
    return out

pairs = collections.Counter()
for p in sorted(glob.glob(NSS + '/*.nss')):
    b = os.path.basename(p)[:-4]
    q = None
    for d in ('nut', 'function', 'system'):
        if os.path.exists(f'{NUT}/{d}/{b}.nut'): q = f'{NUT}/{d}/{b}.nut'
    if not q: continue
    try: en = nut_calls(q)
    except Exception as e: print('skip', b, e, file=sys.stderr); continue
    for key, ts in nss_calls(p).items():
        es = en.get(key, [])
        if len(es) != len(ts): pairs[(key[0], '?count', '')] += 1; continue
        for t, e in zip(ts, es): pairs[(key[0], t, e)] += 1
for (k, t, e), n in sorted(pairs.items(), key=lambda x: (x[0][0], x[0][1], -x[1])):
    if t != 'null' or e != 'null': print(k, t, '->', e, n)
