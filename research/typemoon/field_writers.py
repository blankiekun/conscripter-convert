#!/usr/bin/env python3
"""Lists every access to a struct-of-arrays block in the decompile.

    field_writers.py <ghidra decomp dir> <PTR name> > out.tsv

The layer block is PTR_DAT_00be0250 (0x19b8 bytes, 40 layers, one array
per field). Matches `BASE + ... + off` and `BASE[... + off]` where BASE
is the pointer or a local copy of it (`puVar13 = PTR_DAT_00be0250;`,
held until its next assignment, in source order), and copies with a
constant offset (`p = PTR + 0x16da;`). Prints offset, stride (0: a
constant index), access (W write, R read, A address taken: passed on
or kept), function, command context (inside script_command: the last
`case` letter and three-letter compare above the line), source line.
"""
import os
import re
import sys

ASSIGN = re.compile(r"(?<![\w.>*\[])(\w+) = ([^;=][^;]*);")
WRITE = re.compile(r"^\s*(=(?!=)|\|=|&=|\+=|-=|\^=|\+\+|--)")
SIZE = {"byte": 1, "char": 1, "undefined1": 1, "bool": 1,
        "short": 2, "ushort": 2, "undefined2": 2,
        "int": 4, "uint": 4, "float": 4, "undefined4": 4,
        "long": 8, "ulong": 8, "undefined8": 8, "double": 8}


def close(s, i, open_ch, close_ch):
    """Index of the bracket closing the one opened before s[i]."""
    depth = 1
    while i < len(s):
        if s[i] == open_ch:
            depth += 1
        elif s[i] == close_ch:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return len(s)


def terms(expr):
    """Constant offset and stride of `a * 0x12 + 0x16da` style sums."""
    off, stride = 0, 0
    depth, cur, parts = 0, "", []
    for ch in expr:
        depth += ch in "(["
        depth -= ch in ")]"
        cur += ch
        if depth == 0 and cur.endswith(" + "):
            parts.append(cur[:-3])
            cur = ""
    parts.append(cur)
    for p in parts:
        p = p.strip()
        if re.fullmatch(r"0x[0-9a-f]+|\d+", p):
            off += int(p, 0)
        else:
            m = re.search(r"\* (0x[0-9a-f]+|\d+)$", p)
            stride = int(m.group(1), 0) if m else max(stride, 1)
    return off, stride


def scan(path, base, out):
    src = open(path, encoding="utf-8", errors="replace").read()
    func = os.path.basename(path)[:-2]
    names = {base: 0}
    letter, cmd = "", ""
    for line in src.splitlines():
        if func.endswith("script_command"):
            m = re.search(r"case (0x[0-9a-f]{2}):", line)
            if m:
                letter, cmd = chr(int(m.group(1), 16)), ""
            m = re.search(r"(?:== |case )(0x[0-9a-f]{6})\b", line)
            if m:
                cmd = bytes.fromhex(m.group(1)[2:]).decode("latin-1")
        ctx = letter + cmd if letter else ""
        alt = "|".join(re.escape(n) for n in names)
        for m in re.finditer(r"(?<![\w])(" + alt + r")(?![\w])", line):
            name = m.group(1)
            rest = line[m.end():]
            pre = line[:m.start()]
            if rest.startswith("["):
                end = close(rest, 1, "[", "]")
                off, stride = terms(rest[1:end])
                size, acc_rest, deref = 1, rest[end + 1:], True
            elif rest.startswith(" + ") and pre.endswith("("):
                end = close(rest, 0, "(", ")")
                off, stride = terms(rest[3:end])
                acc_rest = rest[end + 1:]
                c = re.search(r"\*\((\w+)(?: \w+)? \*\)\($", pre)
                deref = bool(c)
                size = SIZE.get(c.group(1), 0) if c else 0
            elif re.match(r"\s*[;),]", rest) or rest.startswith(" + "):
                off, stride, size, acc_rest, deref = 0, 0, 0, rest, False
                if rest.startswith(" + "):
                    end = re.search(r"[;),]", rest).start()
                    off, stride = terms(rest[3:end])
                    acc_rest = rest[end:]
                if not pre.rstrip().endswith("=") or "(" in pre.split("=")[-1]:
                    pass
                else:
                    continue  # a plain copy, handled below
            else:
                continue
            off += names[name]
            if deref:
                acc = "W" if WRITE.match(acc_rest) else "R"
            else:
                acc = "A"
            out.append((off, stride, size, acc, func, ctx, line.strip()))
        for n, rhs in ASSIGN.findall(line):
            if n == base:
                continue
            m = re.fullmatch(r"(" + alt + r")(?: \+ (0x[0-9a-f]+|\d+))?", rhs)
            if m:
                names[n] = names[m.group(1)] + (int(m.group(2), 0) if m.group(2) else 0)
            else:
                names.pop(n, None)


def main():
    d, base = sys.argv[1], sys.argv[2]
    out = []
    for f in sorted(os.listdir(d)):
        if f.endswith(".c"):
            scan(os.path.join(d, f), base, out)
    for off, stride, size, acc, func, ctx, line in out:
        print(f"0x{off:x}\t{stride}\t{size}\t{acc}\t{func}\t{ctx}\t{line}")


main()
