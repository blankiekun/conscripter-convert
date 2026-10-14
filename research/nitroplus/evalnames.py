"""Lists the built-in names a decompiled Muramasa.exe function compares
one character at a time when the name must be followed by `(`
(`pcVar14[1] == 'a' && ... && pcVar14[6] == '('`): the value-returning
built-ins in the expression evaluator (0043a780). Prints
`<name>\t<line in the .c file>`.

    python3 -I research/nitroplus/evalnames.py <ghidra export>/decomp/0043a780_*.c"""
import re, sys

src = open(sys.argv[1], encoding='utf-8', errors='replace').read()
V = r"(0x[0-9a-f]+|\d+|'(?:\\.|[^'])')"
cmp_ = re.compile(rf"(\*?\w+)(?:\[(0x[0-9a-f]+|\d+)\])?\s*[=!]=\s*{V}")


def num(s):
    if s.startswith("'"):
        return {"'\\0'": 0}.get(s, ord(s[1]) if len(s) == 3 else ord(s[2]))
    return int(s, 0)


first = None
for m in re.finditer(r"if \(([^;]*?)\)\s*\{", src):
    chars, firsts = {}, []
    for c in cmp_.finditer(m.group(1)):
        var, i, v = c.group(1), c.group(2), num(c.group(3))
        if i is None:
            if 0x41 <= v <= 0x7a:
                firsts.append(v)
        else:
            chars[int(i, 0)] = v
    if firsts:
        first = firsts[-1]
    if chars and chars.get(max(chars)) == 0x28:
        chars.setdefault(0, first)
        n = max(chars)
        name = ''.join(chr(chars[i]) if chars.get(i) else '?' for i in range(n))
        print(f"{name}\t{src.count(chr(10), 0, m.start()) + 1}")
