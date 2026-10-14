"""Decodes the built-in dispatcher of Muramasa.exe (00405250): it compares
the statement text one byte at a time (`pbVar6[3] == 0x61`) with each
built-in name, followed by `(`, a space, a tab, a full-width space or a
line break, then calls that built-in's function. Prints
`<name>\t<handler address>\t<handler name>\t<line in the .c file>` per
built-in.

    python3 -I research/nitroplus/dispatch.py <ghidra export>/decomp/00405250_*.c"""
import re, sys

src = open(sys.argv[1], encoding='utf-8', errors='replace').read()
# The export's "// calls:" header names each callee's address.
addr = {n: a for a, n in re.findall(r"([0-9a-f]{8}) (\w+)", src.split('\n', 3)[2])}
V = r"(0x[0-9a-f]+|\d+|'.')"
tok = re.compile(
    rf"pbVar6\[(?P<i>0x[0-9a-f]+|\d+)\]\s*(?P<op>[=!])=\s*(?P<v>{V})"
    rf"|(?:bVar1|\*pbVar6)\s*(?P<fop>[=!])=\s*(?P<fv>{V})"
    r"|bVar\d = pbVar6\[(?P<ei>0x[0-9a-f]+|\d+)\]"
    r"|(?:=\s*|\n\s*)(?P<call>\w+)\((?:[^;()]|\([^()]*\))*(?:pbVar6|in_EAX)\);\s*\*unaff_EDI")


def num(s):
    return ord(s[1]) if s.startswith("'") else int(s, 0)


first, chars, end, neg = None, {}, None, False
for m in tok.finditer(src):
    if m.group('fv'):
        v = num(m.group('fv'))
        if 0x41 <= v <= 0x7a:  # a letter: the first byte, not the terminator
            first = v
        neg |= m.group('fop') == '!'
    elif m.group('i'):
        neg |= m.group('op') == '!'
        i = int(m.group('i'), 0)
        if end is None or i < end:
            chars[i] = num(m.group('v'))
    elif m.group('ei'):
        end = int(m.group('ei'), 0)
    elif m.group('call') and neg:
        # The last test is negated: this call is for every other name.
        line = src.count('\n', 0, m.start()) + 1
        call = m.group('call')
        print(f"(not a built-in)\t{addr.get(call, '?')}\t{call}\t{line}")
        neg = False
    elif m.group('call'):
        chars.setdefault(0, first)
        n = end if end is not None else max(chars) + 1
        name = ''.join(chr(chars[i]) if chars.get(i) else '?' for i in range(n))
        line = src.count('\n', 0, m.start()) + 1
        call = m.group('call')
        print(f"{name}\t{addr.get(call, '?')}\t{call}\t{line}")
        chars, end = {}, None
