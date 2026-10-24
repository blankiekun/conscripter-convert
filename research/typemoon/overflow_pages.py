r"""Which pages of a 月姫 build need more text rows than the window has.

    python3 -I research/typemoon/overflow_pages.py make <filled 0.txt> <dir>
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy <engine> --root <dir> \
        --trace <dir>/t.jsonl --auto-play 0 --save-dir <dir>/save
    python3 -I research/typemoon/overflow_pages.py read <dir>

`make` writes <dir>/0.txt: the dry run's text lines (`tests/snapshot.sh`
leaves them filled in as out/snapshot/<game>.filled) in file order, with
the `setwindow3`, `language` and `textclear` lines between them and
nothing else, so the engine lays out every page as the game does; and
<dir>/map.tsv, each line's source line. The engine's pastPage
(ONScripter_cons.cpp) waits and starts a new page when the text would run
past the window's last row: `read` lists the page waits that are not a
script `@` or `\` (a page past its rows), by source line, with the rows
its first source line and the lines it splits on.
"""
import json
import os
import sys


def is_text(line):
    return bool(line) and ord(line[0]) > 127


def make(src, out):
    lines = open(src, encoding="utf-8").read().split("\n")
    body = ["*define", "rubyon", "game", "*start"]
    where = [0] * len(body)
    start = lines.index("*start")
    pages = 0
    for n, line in enumerate(lines[start + 1:], start + 2):
        if is_text(line) or line.startswith(("setwindow3 ", "language ")) or line == "textclear":
            body.append(line.replace("%907", "0"))
            where.append(n)
            pages += line.endswith("\\")
            if line.endswith("\\") and pages % 500 == 0:  # auto-play stops after 3000 pages without a label
                body.append(f"*p{n}")
                where.append(0)
    body.append("end")
    where.append(0)
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, "0.txt"), "w", encoding="utf-8") as f:
        f.write(";$S1280,720,value800,V8192\n" + "\n".join(body) + "\n")
    with open(os.path.join(out, "ons_args"), "w") as f:
        f.write("--enc:utf8\n")
    with open(os.path.join(out, "map.tsv"), "w") as f:
        f.write("\n".join(map(str, where)) + "\n")
    print(f"{sum(map(is_text, body))} text lines, {sum(l.endswith(chr(92)) for l in body)} pages")


def read(out):
    where = [0, 0] + [int(x) for x in open(os.path.join(out, "map.tsv")).read().split()]
    body = [""] + open(os.path.join(out, "0.txt"), encoding="utf-8").read().split("\n")
    count = {}
    for rec in map(json.loads, open(os.path.join(out, "t.jsonl"), encoding="utf-8")):
        if rec.get("t") == "page" and ":" in rec.get("at", ""):
            n = int(rec["at"].rsplit(":", 1)[1])
            count[n] = count.get(n, 0) + 1
    split = {n: c - body[n].endswith(("@", "\\")) for n, c in count.items()}
    split = {n: c for n, c in split.items() if c > 0}
    # the source page each split is on: the page's first line in the source
    first = {}
    for n in split:
        k = n
        while k > 1 and not body[k - 1].endswith("\\") and body[k - 1] != "textclear" and is_text(body[k - 1]):
            k -= 1
        first.setdefault(where[k], []).append(where[n])
    for page, at in sorted(first.items()):
        print(f"{page}\tsplit at {','.join(map(str, at))}")
    print(f"{len(first)} pages past the window's rows ({len(split)} extra waits)")


if __name__ == "__main__":
    {"make": make, "read": read}[sys.argv[1]](*sys.argv[2:])
