"""Progression variables of 装甲悪鬼村正 (docs/new-game-reference.md, "Route simulation").

Lists every variable that the story files (m*.nss) both set and test, with
the places that set it (and the value) and the places that read it. `#`
variables are the engine's persistent ones (system data, kept across
playthroughs); `$` ones live in the save.

    python3 -I research/nitroplus/progress.py <utf-8 nss dir> [--sites N]

The NSS files must be converted to UTF-8 first (out/muramasa-utf8).
Bookkeeping that doesn't gate anything is left out: CG/BG seen flags,
voice_on_*, av_*, イベントファイル名, $SYSTEM_*, $GameName handling.
"""
import re
import sys
from collections import defaultdict
from pathlib import Path

SKIP = re.compile(r'^[$#](SYSTEM_|voice_on_|av_|ev|bg|Cg|cg|イベントファイル名|GameName$|GameStart$|GameCircle$|PreGameName$|LOCAL_|Csr|回想判定)')
ASSIGN = re.compile(r'([$#][^\s=!<>+\-*/;,()"\[\]{}]+(?:\[[^\]]*\])?)\s*(\+\+|--|[+\-]?=(?!=))\s*([^;]*)')
TEST = re.compile(r'\b(?:if|while)\s*\((.*)')
VAR = re.compile(r'[$#][^\s=!<>+\-*/;,()"\[\]{}&|]+(?:\[[^\]]*\])?')


def strip_comments(text):
    text = re.sub(r'/\*.*?\*/', '', text, flags=re.S)  # non-nested is enough here
    return [re.sub(r'//.*', '', line) for line in text.split('\n')]


def main():
    src = Path(sys.argv[1])
    sites = int(sys.argv[sys.argv.index('--sites') + 1]) if '--sites' in sys.argv else 4
    sets, tests = defaultdict(list), defaultdict(list)
    for f in sorted(src.glob('m*.nss')):
        for n, line in enumerate(strip_comments(f.read_text()), 1):
            where = f'{f.stem}:{n}'
            for m in TEST.finditer(line):
                for v in set(VAR.findall(m.group(1))):
                    tests[v].append(where)
            for m in ASSIGN.finditer(line):
                name, op, value = m.group(1), m.group(2), m.group(3).strip()
                if line[:m.start()].rstrip().endswith(('==', '!=', '<', '>')):
                    continue
                sets[name].append((where, op if op in ('++', '--') else f'{op} {value}'))
    names = sorted(n for n in sets if n in tests and not SKIP.match(n))
    print(f'{len(names)} progression variables ({sum(n[0] == "#" for n in names)} persistent #)\n')
    for name in names:
        kind = 'system' if name[0] == '#' else 'save'
        values = sorted({v for _, v in sets[name]})
        files_set = sorted({w.split(':')[0] for w, _ in sets[name]})
        files_test = sorted({w.split(':')[0] for w in tests[name]})
        print(f'{name}  [{kind}]  values: {", ".join(values[:6])}{" …" if len(values) > 6 else ""}')
        print(f'    set in {len(files_set)}: {" ".join(files_set[:sites])}{" …" if len(files_set) > sites else ""}')
        print(f'    read in {len(files_test)}: {" ".join(files_test[:sites])}{" …" if len(files_test) > sites else ""}')


if __name__ == '__main__':
    main()
