#!/usr/bin/env python3
"""Flag numbers added to an engine converter without a source comment.

A behaviour number (a place, duration, colour, size, curve) says where it came
from on its line or the comment line above: an exe address (0x..., FUN_, DAT_),
"exe"/"eboot", a data file, or `// ours: <reason>`. Convention:
docs/new-game.md, step 12. Only lines added since BASE are checked; values
already on main get a source when they are touched.
Usage: python3 -I research/tools/source_lint.py [BASE]   (default origin/main)
Heuristic: format parsing, tests and 0/1/2 are skipped; a flag is a prompt
to add a source, not a failure. Tune the regexes below."""
import re, subprocess, sys

NUM = re.compile(r"(?<![\w.])-?(\d+\.\d+|\d{1,6})(?![\w.])")
HEX = re.compile(r"0x[0-9a-fA-F]+")
SRC = re.compile(r"0x[0-9a-fA-F]{4,}|FUN_|DAT_|\bexe\b|eboot|ours|from the|names\.tsv|\.png|\.ini|\.nss|script"
                 r"|spec|NScripter|ONScripter|engine", re.I)
PARSE = re.compile(r"read_|u(8|16|32|64)::from|\[\s*\d+\s*\.\.|\.\.\d|offset|bytes|magic|header|<<|>>|& 0x"
                   r"|get\(\d|arg\(\d|len\(\)|with_capacity|assert|panic|bail|format!|println|eprintln")


def flagged(lines, added):
    """Line numbers (1-based) in `added` holding an unsourced behaviour number."""
    out = []
    for i, line in enumerate(lines, 1):
        s = line.strip()
        if s.startswith("#[cfg(test)]"):
            break
        if i not in added or s.startswith(("//", "use ", "mod ", "#[")):
            continue
        code = s.split("//")[0]
        above = lines[i - 2].strip() if i > 1 and lines[i - 2].strip().startswith("//") else ""
        if [n for n in NUM.findall(HEX.sub("", code)) if n not in ("0", "1", "2")] \
                and not PARSE.search(code) and not SRC.search(line) and not SRC.search(above):
            out.append(i)
    return out


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else "origin/main"
    diff = subprocess.run(["git", "diff", "-U0", f"{base}...HEAD", "--", "crates/conv-engines/src/*.rs"],
                          capture_output=True, text=True, check=True).stdout
    added, path = {}, None
    for l in diff.split("\n"):
        if l.startswith("+++ "):
            path = None if l == "+++ /dev/null" or "/tests/" in l else l[6:]
        elif l.startswith("@@") and path:
            start, _, count = re.search(r"\+(\d+)(,(\d+))?", l).groups()
            added.setdefault(path, set()).update(range(int(start), int(start) + int(count or 1)))
    n = 0
    for path, rows in sorted(added.items()):
        lines = open(path, encoding="utf-8").read().split("\n")
        for i in flagged(lines, rows):
            print(f"{path}:{i}: {lines[i - 1].strip()[:100]}")
            n += 1
    return n


if __name__ == "__main__":
    assert flagged(["let x = 473;", "// exe 0x469850", "let y = 581;", "let z = 2;"], {1, 3, 4}) == [1]
    sys.exit(1 if main() else 0)
