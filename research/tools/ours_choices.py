# ours_choices.py <0.txt> <map> FILE:N ... -> engine --choices lines (select places by source file)
import bisect, re, sys
lines = open(sys.argv[1], 'rb').read().decode('cp932', 'replace').split('\n')
m = {}
for l in open(sys.argv[2], encoding='utf-8'):
    a, b = l.rstrip('\n').split('\t'); m[int(a)] = b
keys = sorted(m)
sel = []  # (file, place)
label = None
for n, l in enumerate(lines, 1):
    if l.startswith('*'): label = l.split()[0].split(';')[0]
    elif l.startswith('select ') or l.startswith('btnwait '):
        k = keys[bisect.bisect_right(keys, n) - 1]
        sel.append((m[k].split(':')[0], f'{label}:{n}', l.split()[0]))
# --choose arguments: every select of FILE (variants under flags too) takes N
by = {}
for p in sys.argv[3:]:
    f, n = p.rsplit(':', 1); by.setdefault(f, []).append(n)
for f, ns in by.items():
    c = [s for s in sel if s[0] == f + '.nss' or s[0].startswith(f + '_')]
    for i, s in enumerate(c):  # as many selects as answers: one each, in order
        print(f"--choose {s[1]}={ns[i] if len(c) == len(ns) else ','.join(ns)}")
