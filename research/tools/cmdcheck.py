"""Every command a 0.txt uses is one the engine knows.

    python3 -I research/tools/cmdcheck.py engine/src/cons/ONScripter_lut.cpp out/<game>/0.txt

The engine's names are read from its command table (ONScripter_lut.cpp, the
same table `conscripter --list-commands` prints); names the script makes with
defsub or luasub count too. Prints each unknown name with its count and first
line, and exits 1 if there is any.
"""
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


def check(lut, script):
    known = engine_commands(lut)
    lines = open(script, 'rb').read().split(b'\n')
    aliases = set(CONDITION_WORDS)
    for line in lines:
        m = re.match(rb'\s*_?(defsub|luasub|numalias|stralias)\s+([A-Za-z_][A-Za-z0-9_]*)', line)
        if m:
            (aliases if m.group(1).endswith(b'alias') else known).add(m.group(2).lower())
    unknown = OrderedDict()
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
            if name in known or re.fullmatch(rb'd?v[0-9]+', name):
                continue
            u = unknown.setdefault(name.decode(), [0, n])
            u[0] += 1
    for name, (count, first) in unknown.items():
        print('%s: unknown command %s (%d times, first at line %d)' % (script, name, count, first))
    return 1 if unknown else 0


if __name__ == '__main__':
    sys.exit(check(sys.argv[1], sys.argv[2]))
