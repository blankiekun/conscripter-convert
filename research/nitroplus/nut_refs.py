"""Static counts over the English Muramasa's compiled scripts (script.npk
extracted with npk.py), for the coverage table in docs/games/muramasa-en.md.

    python3 -I nut_refs.py <script>/media/script calls [--nss NAMES]
    python3 -I nut_refs.py <script>/media/script tables

calls: every name called by a literal (PREPCALLK), with its count per file
group (nut = the scenario, sys:<file>, fn:<file>, framework, ...) and the
file defining it. With --nss (a file of NSS function and built-in names, one
a line), only the names the scenario calls that the NSS doesn't have.
tables: the rows of each lang/en table (TransText, TransChoice, ...).
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nut  # noqa: E402


def group(rel):
    p = rel.split('/')
    if p[0] == 'system':
        return 'sys:' + p[1][:-4]
    if p[0] == 'function':
        return 'fn:' + p[1][:-4]
    return p[0] if len(p) > 1 else p[0][:-4]


def calls(root):
    out, defs = collections.defaultdict(collections.Counter), {}

    def walk(f, g, rel):
        lit = f['literals']
        for op, _, a1, _, _ in f['code']:  # 8: PREPCALLK
            if op == 8 and 0 <= a1 < len(lit) and isinstance(lit[a1], str):
                out[lit[a1]][g] += 1
        for h in f['functions']:
            if isinstance(h['name'], str):
                defs.setdefault(h['name'], rel)
            walk(h, g, rel)
    for d, _, fs in os.walk(root):
        for fn in fs:
            rel = os.path.relpath(os.path.join(d, fn), root)
            if fn.endswith('.nut') and not rel.startswith('lang/'):
                walk(nut.load(os.path.join(root, rel)), group(rel), rel)
    return out, defs


def tables(root):
    """{table: (rows, files)}; keys are LOADINT/LOAD + LOAD or one DLOAD."""
    rows, files = collections.Counter(), collections.defaultdict(set)
    d = os.path.join(root, 'lang/en')
    for fn in os.listdir(d):
        f = nut.load(os.path.join(d, fn))
        lit, code = f['literals'], f['code']
        ops = [nut.OPS[c[0]] for c in code]
        for i in range(len(code) - 2):
            if ops[i] != 'GETK':
                continue
            if ops[i + 1:i + 3] == ['DLOAD', 'NEWSLOT'] or (
                    ops[i + 1] in ('LOAD', 'LOADINT') and ops[i + 2:i + 4] == ['LOAD', 'NEWSLOT']):
                t = lit[code[i][2]]
                rows[t] += 1
                files[t].add(fn)
    return {t: (rows[t], len(files[t])) for t in rows}


if __name__ == '__main__':
    root, what = sys.argv[1], sys.argv[2]
    if what == 'tables':
        for t, (n, nf) in sorted(tables(root).items()):
            print('%s: %d rows, %d files' % (t, n, nf))
    else:
        out, defs = calls(root)
        nss = None
        if '--nss' in sys.argv:
            nss = {l.strip().lower() for l in open(sys.argv[sys.argv.index('--nss') + 1])}
        for name, c in sorted(out.items(), key=lambda kv: -sum(kv[1].values())):
            if nss is not None and ('nut' not in c or name.lower() in nss):
                continue
            print(sum(c.values()), name, defs.get(name, '-'), dict(c))
