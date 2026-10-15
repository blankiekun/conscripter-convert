#!/usr/bin/env python3
"""Outline a Rust or C++ file, or print one item, instead of paging with sed.

usage: code.py FILE                 list items (fn, struct, enum, impl, ...;
                                    C++ functions defined at column 0)
       code.py FILE NAME [LINES]    print the item NAME (fn, type, or C++
                                    "Class::name" / "name"), its doc
                                    comments included, at most LINES lines
                                    (default 150); every match is printed

Line numbers match the file.
"""
import re
import sys

RS = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+|extern\s+\"C\"\s+)*"
                r"(fn|struct|enum|impl|mod|trait|type|const|static|macro_rules!)\s*(?:<[^>]*>\s*)?(\w+(?:::\w+)*)?")
CPP = re.compile(r"^(?![#{}/\s])[^;=()]*?([\w:~]+)\s*\(")  # a definition starts at column 0
STRIP = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'|//.*')  # strings, char literals, line comments


def items(lines, rust):
    for i, l in enumerate(lines):
        m = RS.match(l) if rust else CPP.match(l)
        if not m or (not rust and l.rstrip().endswith(";")):
            continue
        if rust:
            name = m.group(2) or ""
            if m.group(1) == "impl":  # impl Foo / impl Trait for Foo
                name = re.sub(r"^impl\s*(<[^>]*>)?\s*", "", l.strip()).split("{")[0].strip()
            yield i, m.group(1), name
        else:
            yield i, "", m.group(1)


def end_of(lines, start):
    # shortcut: brace counting skips strings and line comments but not block comments or raw strings
    depth, opened = 0, False
    for j in range(start, len(lines)):
        code = STRIP.sub("", lines[j])
        depth += code.count("{") - code.count("}")
        opened = opened or "{" in code
        if (opened and depth <= 0) or (not opened and code.rstrip().endswith(";")):
            return j + 1
    return len(lines)


if len(sys.argv) < 2:
    sys.exit(__doc__)
path = sys.argv[1]
rust = path.endswith(".rs")
lines = open(path, encoding="utf-8", errors="replace").read().split("\n")

if len(sys.argv) == 2:
    for i, kind, name in items(lines, rust):
        indent = len(lines[i]) - len(lines[i].lstrip())
        print(f"{i + 1}:{' ' * indent}{kind} {name}".rstrip())
    sys.exit()

want = sys.argv[2]
limit = int(sys.argv[3]) if len(sys.argv) > 3 else 150
hits = [i for i, kind, name in items(lines, rust)
        if name == want or name.split("::")[-1] == want or (kind == "impl" and want in name.split())]
if not hits:
    sys.exit(f"no item {want!r}; run without NAME to list them")
for i in hits:
    start = i
    while start > 0 and lines[start - 1].strip().startswith(("///", "//!", "#[", "/**", "* ", "*/")):
        start -= 1
    end = end_of(lines, i)
    for j in range(start, min(end, start + limit)):
        print(f"{j + 1}:{lines[j]}")
    if end - start > limit:
        print(f"... {end - start - limit} more lines, to line {end}")
