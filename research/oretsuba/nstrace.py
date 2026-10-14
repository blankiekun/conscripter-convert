"""Route tracer for a converted 俺たちに翼はない 0.txt (docs/new-game.md
step 11.3).

Runs the control flow `conscripter-convert mages convert` writes (labels,
goto, if, mov/add/sub, select, reset; everything else is skipped) along
a choice path and prints the scenes it passes, the choices it takes and
where the run ends (`reset`: back to the title). Variables from %200 on
are globals: they survive NEWGAME; the others are cleared.

    python3 -I nstrace.py <0.txt> <path>... [--quiet] [--watch %60,%410]

A path is a comma-separated list of `scene:option` (S01_02C:2 takes
option 2 at the choice in S01_02C; `scene:"text"` the option containing
the text); choices not listed take option 1, and the scene-skip offers
(このシーンを読む) are always answered "read".
Each further path starts a NEWGAME (globals kept), like playing the
routes one after another. A path may end with `=SCENE`: the run must end
in that scene.
"""
import re
import sys

args = [a for a in sys.argv[1:] if not a.startswith('--')]
quiet = '--quiet' in sys.argv
watch = sys.argv[sys.argv.index('--watch') + 1].split(',') if '--watch' in sys.argv else []
src = open(args[0], 'rb').read().decode('cp932')
lines = src.split('\n')
labels = {}
for i, l in enumerate(lines):
    if l.startswith('*'):
        labels[l[1:].strip().lower()] = i
# A scene label is followed by its title (`mov $150,"…"`).
scenes = {i: lines[i][1:].strip() for i in labels.values()
          if lines[i].startswith('*mg_') and i + 1 < len(lines) and lines[i + 1].startswith('mov $150,')}

# The scenes that set each ending's flag.
ENDINGS = {'s01_16a': 'asuka', 's01_18a': 'miyako', 'm09_01': 'misaki', 's02_16a': 'hiyoko', 's03_22a': 'naru',
           's03c_20a': 'ai', 's04_28b': 'kobato', 's04_06l': 'BAD'}
glob = {}


def run(path):
    want_end = None
    if path and path[-1].startswith('='):
        want_end = path.pop()[1:].lower()
    var = dict(glob)
    pc = labels['mg_s01_01'] if 'mg_s01_01' in labels else 0
    passed = []
    choices = {e.split(':')[0].lower(): e.split(':', 1)[1] for e in path}
    steps = 0

    def val(x):
        x = x.strip()
        return var.get(int(x[1:]), 0) if x.startswith('%') else int(x)

    while pc < len(lines):
        steps += 1
        if steps > 5_000_000:
            print('  !! no end after 5M steps (loop?)')
            break
        if pc in scenes:
            passed.append(scenes[pc][3:])
            if watch:
                print('   ', scenes[pc][3:], ' '.join(f'{w}={val(w)}' for w in watch))
        line = lines[pc].split(';')[0].strip() if not lines[pc].startswith('select') else lines[pc].strip()
        pc += 1
        for stmt in [line] if line.startswith('select') else line.split(':'):
            stmt = stmt.strip()
            m = re.match(r'goto \*(\w+)$', stmt)
            if m:
                pc = labels[m.group(1).lower()]
                break
            m = re.match(r'if (%\d+) *(==|!=|<=|>=|<|>) *(-?%?\d+) goto \*(\w+)$', stmt)
            if m:
                a, op, b = val(m.group(1)), m.group(2), val(m.group(3))
                if {'==': a == b, '!=': a != b, '<': a < b, '>': a > b, '<=': a <= b, '>=': a >= b}[op]:
                    pc = labels[m.group(4).lower()]
                    break
                continue
            m = re.match(r'(mov|add|sub) %(\d+),(-?%?\d+)$', stmt)
            if m:
                n, v = int(m.group(2)), val(m.group(3))
                var[n] = {'mov': v, 'add': var.get(n, 0) + v, 'sub': var.get(n, 0) - v}[m.group(1)]
                continue
            if stmt.startswith('select '):
                opts = re.findall(r'"([^"]*)",\*(\w+)', stmt)
                if opts[0][0] == 'このシーンを読む':
                    pick = 0
                else:
                    c = choices.pop(passed[-1], '*')
                    if c == '*':
                        pick = 0
                    elif c.startswith('"'):
                        pick = next(i for i, o in enumerate(opts) if c.strip('"') in o[0])
                    else:
                        pick = int(c) - 1
                    if not quiet:
                        print(f'  choice in {passed[-1]}: {opts[pick][0]}')
                pc = labels[opts[pick][1].lower()]
                break
            if stmt == 'reset':
                for k in list(var):
                    if k >= 200:
                        glob[k] = var[k]
                end = passed[-1] if passed else '?'
                ok = '' if want_end is None else (' OK' if end == want_end else f' !! expected {want_end}')
                seen = [e for e in ENDINGS if e in passed]
                print(f'  ends in {end} after {len(passed)} scenes{ok}; endings: {" ".join(ENDINGS[e] for e in seen) or "none"}')
                return passed, end
    return passed, None


for n, p in enumerate(args[1:]):
    path = [c.strip() for c in p.split(',') if c.strip()]
    print(f'run {n + 1}: {p}')
    passed, end = run(path)
    if not quiet:
        print('  ' + ' '.join(passed))
