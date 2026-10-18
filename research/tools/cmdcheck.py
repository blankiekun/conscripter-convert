"""Every command a 0.txt uses is one the engine knows.

    python3 -I research/tools/cmdcheck.py engine/src/cons/ONScripter_lut.cpp out/<game>/0.txt

The engine's names are read from its command table (ONScripter_lut.cpp, the
same table `conscripter --list-commands` prints); names the script makes with
defsub or luasub count too. Prints each unknown name with its count and first
line, and exits 1 if there is any. --counts FILE also writes how often each
command is used ({name: count}, JSON; vN/dvN voices as "v#"/"dv#").

    python3 -I research/tools/cmdcheck.py --compare old.json new.json
    python3 -I research/tools/cmdcheck.py --pair a.json b.json

--compare prints each command whose count moved by over 5 % (or appeared, or
went), exit 2 if any (tests/snapshot.sh). --pair prints, for two editions of
one game, each command whose count ratio is over 20 % off the median ratio
of their common commands (a feature one edition lost: 2 dwave against 36,055); informative.
"""
import json
import re
import sys
from collections import OrderedDict

IDENT = re.compile(rb'_?([a-z][a-z0-9_]*)', re.I)
# a statement's head: a word, unless the line is text (it starts with anything else)
TOKEN = re.compile(rb'"[^"]*"|`[^`]*`|[A-Za-z_][A-Za-z0-9_]*|.', re.S)
CONDITION_WORDS = {b'fchk', b'lchk'}


def engine_commands(lut):
    return {m.encode() for m in re.findall(r'\{"([a-z0-9_]+)",\s*&ONScripter::', open(lut, encoding='latin-1').read())}


def statements(line):
    """The statements of one script line: (head word, the tokens after it)."""
    tokens = [t for t in TOKEN.findall(line) if not t.isspace()]
    start = 0
    for i, t in enumerate(tokens + [b':']):
        if t in (b':', b';'):
            if start < i:
                yield tokens[start:i]
            if t == b';':
                return
            start = i + 1


def check(lut, script, counts_file=None):
    known = engine_commands(lut)
    lines = open(script, 'rb').read().split(b'\n')
    aliases = set(CONDITION_WORDS)
    for line in lines:
        m = re.match(rb'\s*_?(defsub|luasub|numalias|stralias)\s+([A-Za-z_][A-Za-z0-9_]*)', line)
        if m:
            (aliases if m.group(1).endswith(b'alias') else known).add(m.group(2).lower())
    unknown = OrderedDict()
    counts = {}
    for n, line in enumerate(lines, 1):
        s = line.lstrip(b' \t')
        if not s or not (s[:1].isalpha() or s[:1] == b'_'):
            continue  # a label, a comment, text, a jump target
        for st in statements(s.rstrip(b'\r')):
            head = st[0]
            if not IDENT.fullmatch(head):
                break  # the rest of the line is text
            name = head.lstrip(b'_').lower()
            if name in (b'if', b'notif'):
                # the condition's words are aliases and fchk/lchk; the first other word is the command
                rest = [t.lstrip(b'_').lower() for prev, t in zip(st, st[1:])
                        if IDENT.fullmatch(t) and prev not in (b'%', b'$', b'?')]  # not a named variable
                rest = [t for t in rest if t not in aliases]
                if not rest:
                    continue
                name = rest[0]
            voice = re.fullmatch(rb'(d?v)[0-9]+', name)
            key = (voice.group(1) + b'#' if voice else name).decode()
            counts[key] = counts.get(key, 0) + 1
            if name in known or voice:
                continue
            u = unknown.setdefault(name.decode(), [0, n])
            u[0] += 1
    for name, (count, first) in unknown.items():
        print('%s: unknown command %s (%d times, first at line %d)' % (script, name, count, first))
    if counts_file:
        with open(counts_file, 'w') as f:
            json.dump(dict(sorted(counts.items())), f, indent=0)
            f.write('\n')
    return 1 if unknown else 0


def compare(old, new, tolerance=0.05):
    """The commands whose count moved over `tolerance` (lines to print)."""
    return ['%s %d -> %d' % (k, old.get(k, 0), new.get(k, 0)) for k in sorted(old.keys() | new.keys())
            if abs(new.get(k, 0) - old.get(k, 0)) > tolerance * max(old.get(k, 0), 1)]


def pair(a, b, tolerance=0.2, least=20):
    """The commands whose a:b ratio is off the median one by over `tolerance`
    (the median: a lost command doesn't move it, as it would the totals')."""
    ratios = sorted(b[k] / a[k] for k in a.keys() & b.keys() if min(a[k], b[k]) >= least)
    r = ratios[len(ratios) // 2] if ratios else 1
    return ['%s %d : %d (expected about %d)' % (k, a.get(k, 0), b.get(k, 0), a.get(k, 0) * r)
            for k in sorted(a.keys() | b.keys()) if max(a.get(k, 0), b.get(k, 0)) >= least
            and abs(b.get(k, 0) - a.get(k, 0) * r) > tolerance * max(a.get(k, 0) * r, b.get(k, 0))]


def test():
    assert compare({'dwave': 100, 'bg': 10}, {'dwave': 104, 'bg': 12, 'new': 1}) == ['bg 10 -> 12', 'new 0 -> 1']
    same = {'bg': 1000, 'ld': 500, 'print': 800}
    assert pair({'dwave': 36055, 'x': 5, **same}, {'dwave': 2, **same}) == ['dwave 36055 : 2 (expected about 36055)']


if __name__ == '__main__':
    a = sys.argv[1:]
    if a == ['--test']:
        test()
        sys.exit(0)
    if a[:1] in (['--compare'], ['--pair']):
        old, new = (json.load(open(f)) for f in a[1:3])
        lines = (compare if a[0] == '--compare' else pair)(old, new)
        print('\n'.join(lines)) if lines else None
        sys.exit(2 if lines and a[0] == '--compare' else 0)
    counts_file = a[a.index('--counts') + 1] if '--counts' in a else None
    a = [x for x in a if x not in ('--counts', counts_file)]
    sys.exit(check(a[0], a[1], counts_file))
