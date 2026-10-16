"""Lists the upper-case words a decompiled function compares one
character at a time (Muramasa.exe inlines strcmp against NSS built-in
names and constants): `<address> <word> ...` per function. Django.exe
compares in either case (`== 'F' || == 'f'`): such pairs are folded to
the capital (its text-tag parser 00463700 then reads FONT, INCOLOR=, ...,
RUBY, VOICE, NAME=, CLASS=, MODE=, SRC=). Upper-case
words only (argument constants such as LOCAL, GLOBAL); the built-in
names themselves are decoded by dispatch.py and evalnames.py.

    python3 -I research/nitroplus/charchains.py <ghidra export>/decomp"""
import os, re, sys
d = sys.argv[1]
for f in sorted(os.listdir(d)):
    t = open(os.path.join(d, f), encoding='utf-8', errors='replace').read()
    chars = []
    for m in re.finditer(r"==\s*(?:'(\\0|[^'\\])'|(0x[0-9a-f]{1,2})\b)", t):
        c = m.group(1)
        if c == '\\0' or (m.group(2) and int(m.group(2), 16) == 0):
            chars.append('|')
        elif c:
            chars.append(c)
        else:
            v = int(m.group(2), 16)
            chars.append(chr(v) if 0x20 < v < 0x7f else '|')
    s = re.sub(r'([A-Z])([a-z])', lambda m: m.group(1) if m.group(2) == m.group(1).lower() else m.group(0), ''.join(chars))
    words = [w for w in s.split('|') if len(w) >= 4 and re.fullmatch(r'[A-Z_0-9=<>/ "\',]+', w) and re.search(r'[A-Z]{3}', w)]
    if words:
        print(f[:8], ' '.join(words))
