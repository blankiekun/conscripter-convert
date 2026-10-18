"""Patched story scripts whose choices take themselves, for ntrace.sh --patch.

    python3 -I force_choices.py <exe or table> <nss.npa> <out dir> FILE:N ...
    python3 -I force_choices.py <exe or table> <nss.npa> <out dir> --path <path file>

Each FILE:N answers the next `select` in FILE (story file name without
.nss, or the part before an `_` of the one such file with a select, as
`dja004`; a file listed twice answers its first, then its second select)
with option N. A path file is research/nitroplus/paths' format: its
`FILE:N` lines are taken, `*` and the rest left out. In each select,
`select` becomes `if(true)`, the case of option N `if(true)`, the other
cases `if(false)`, and the line that turns auto mode off there
(`if($SYSTEM_skip||$SYSTEM_text_auto)`) `if(false)`: every edit stays on
its line, so the trace's lines are the shipped scripts' lines. The case
names are `@選択肢N` / `選択肢N` (装甲悪鬼村正, 続・殺戮のジャンゴ).
"""
import pathlib, re, subprocess, sys, tempfile

FW = str.maketrans('０１２３４５６７８９', '0123456789')
CASE = re.compile(r'case\s*@?選択肢([0-9０-９]+)')
OFF = 'if($SYSTEM_skip||$SYSTEM_text_auto)'


def force(text, answers):
    """text: a script (str, CR LF kept); answers: options for its selects in order."""
    lines = text.split('\n')
    i, done = 0, 0
    while done < len(answers):
        while not re.fullmatch(r'\s*select\s*\r?', lines[i]):
            i += 1
        lines[i] = lines[i].replace('select', 'if(true)', 1)
        depth, opened = 0, False
        while True:
            i += 1
            code = lines[i].split('//', 1)[0]
            code = CASE.sub(lambda m: 'if(true)' if int(m[1].translate(FW)) == answers[done] else 'if(false)', code)
            code = code.replace(OFF, 'if(false)')
            rest = lines[i][len(lines[i].split('//', 1)[0]):]
            lines[i] = code + rest
            depth += code.count('{') - code.count('}')
            opened = opened or '{' in code
            if opened and depth <= 0:
                break
        done += 1
    return '\n'.join(lines)


def main():
    a = sys.argv[1:]
    if len(a) < 4:
        sys.exit(__doc__)
    key, npa, out, picks = a[0], a[1], pathlib.Path(a[2]), a[3:]
    if picks[0] == '--path':
        picks = [l.split('#', 1)[0].strip() for l in open(picks[1], encoding='utf-8')]
        picks = [l for l in picks if re.fullmatch(r'[^\s:]+:\d+', l)]
    by_file = {}
    for p in picks:
        f, n = p.rsplit(':', 1)
        by_file.setdefault(f.removesuffix('.nss'), []).append(int(n))
    out.mkdir(parents=True, exist_ok=True)
    npa_py = pathlib.Path(__file__).with_name('npa.py')
    for f, answers in by_file.items():
        with tempfile.TemporaryDirectory() as tmp:
            subprocess.run([sys.executable, '-I', npa_py, key, npa, tmp, f'*{f}*.nss'], check=True,
                           stdout=subprocess.DEVNULL)
            # the one file of that name (or prefix) that has a select
            src, = [p for p in pathlib.Path(tmp).rglob('*.nss') if (p.stem == f or p.stem.startswith(f + '_'))
                    and re.search(r'(?m)^\s*select\s*$', p.read_bytes().decode('cp932'))]
            text = src.read_bytes().decode('cp932')
            new = force(text, answers)
            assert new.count('\n') == text.count('\n')
            (out / src.name).write_bytes(new.encode('cp932'))
            print(f'{f}: {answers}')


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        s = 'x\r\n\tselect\r\n\t{\r\n\t\tif($SYSTEM_skip||$SYSTEM_text_auto){a;}\r\n\t\tcase\t@選択肢１{\r\n' \
            '\t\t\tb;\r\n\t\t}case\t@選択肢２{\r\n\t\t\tc;\r\n\t\t}\r\n\t}\r\ny\r\n'
        r = force(s, [2])
        assert r.count('\n') == s.count('\n') and 'select' not in r and 'case' not in r
        assert '\t\tif(false){\r' in r and '}if(true){' in r and 'if(false){a;}' in r, r
        print('ok')
    else:
        main()
