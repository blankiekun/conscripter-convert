"""Does any Shake/RandomShake in the English Muramasa's scripts shake a text
object? (docs/games/muramasa-en.md, static checks.) Over every compiled
script (lang/ left out):

    python3 -I text_shake.py <script>/media/script

1. the names given to CreateText/CreateTextArea/CreateTextContext as a
   literal, in any file;
2. each Shake/RandomShake whose name is a literal, matched (fnmatch, the
   name or a child "name/...") against them: HIT lines;
3. each Shake/RandomShake whose name is built at run time: the expression
   (Route.<field>, a parameter), with the string constants any file stores
   in that Route field. Parameters are chased by hand (grep the callers).
"""
import collections
import fnmatch
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nut  # noqa: E402

OPS = nut.OPS
TEXT = ('CreateText', 'CreateTextArea', 'CreateTextContext')
SHAKE = ('Shake', 'RandomShake')


def op(ins):
    return OPS[ins[0]] if ins[0] < len(OPS) else ''


def s(lit, k):
    return lit[k] if 0 <= k < len(lit) and isinstance(lit[k], str) else None


def files(root):
    for dp, _, fs in os.walk(root):
        for fn in sorted(fs):
            rel = os.path.relpath(os.path.join(dp, fn), root)
            if fn.endswith('.nut') and not rel.startswith('lang'):
                yield rel, nut.load(os.path.join(root, rel))


def funcs(f):
    yield f
    for g in f['functions']:
        yield from funcs(g)


def scan(root):
    texts = collections.defaultdict(set); lit_sites = []; dyn_sites = []
    route = collections.defaultdict(set)
    for rel, top in files(root):
        for f in funcs(top):
            lit = f['literals']; code = f['code']
            for i, ins in enumerate(code):
                o = op(ins)
                if o == 'DLOAD':  # Route.<a1> <- "<a3>" (NEWSLOT follows)
                    k, v = s(lit, ins[2]), s(lit, ins[4])
                    if k and v is not None and i + 1 < len(code) and op(code[i + 1]) == 'NEWSLOT':
                        route[k].add(v)
                if o != 'PREPCALLK' or s(lit, ins[2]) not in TEXT + SHAKE:
                    continue
                name = None
                for j in range(i + 1, min(i + 4, len(code))):
                    if op(code[j]) in ('LOAD', 'DLOAD') and s(lit, code[j][2]) is not None:
                        name = s(lit, code[j][2]); break
                call = lit[ins[2]]
                if call in TEXT:
                    if name:
                        texts[name].add(rel)
                elif name is not None:
                    lit_sites.append((rel, name))
                else:
                    expr = [s(lit, c[2]) or op(c) for c in code[i + 1:i + 3] if op(c) == 'GETK']
                    dyn_sites.append((rel, f['name'], f['params'], '.'.join(expr)))
    return texts, lit_sites, dyn_sites, route


def main():
    texts, lit_sites, dyn_sites, route = scan(sys.argv[1])
    print('text names', len(texts), 'literal shakes', len(lit_sites), 'run-time', len(dyn_sites))
    for rel, a in lit_sites:
        hit = sorted(t for t in texts if fnmatch.fnmatchcase(t, a) or t.startswith(a + '/'))
        if hit:
            print('HIT', rel, a, [(t, sorted(texts[t])) for t in hit])
    for rel, fn, params, expr in dyn_sites:
        field = expr.split('.')[-1] if expr.startswith('Route.') else None
        print('RUN', rel, fn, params[1:], expr or '(parameter)', sorted(route.get(field, ())))


main()
