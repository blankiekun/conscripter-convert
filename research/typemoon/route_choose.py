"""--choose lines for a route through our 月姫 0.txt, from option texts.

Usage: python3 -I route_choose.py <0.txt> <text.ja.txt> <route> > choose.txt
  <route>  lines `take <option text>`, `avoid <option text>` or
           `button <script> <n>` (# comments)

Every select (each variant a flag test chose) offering a `take` text takes
it; one offering an `avoid` text takes its first option neither avoided
nor greyed (`seldisable`); every btnwait in the script
(as `tm_QACALL`) takes button n (the lesson question: 2, yes). Other selects are left to --auto-choice first.
Prints `*label:line=N`, one per select, for the engine's --choose.
"""
import re
import sys


def main():
    script, table, route = sys.argv[1:4]
    text = dict(l.rstrip('\n').split('\t', 1) for l in open(table, encoding='utf-8') if '\t' in l)
    take, avoid, buttons = set(), set(), []
    for l in open(route, encoding='utf-8'):
        l = l.strip()
        if l and not l.startswith('#'):
            kind, t = l.split(None, 1)
            if kind == 'button':
                buttons.append(tuple(t.split()))
            else:
                (take if kind == 'take' else avoid).add(t)
    seen = set()
    label, scene, disabled = None, None, set()
    for n, l in enumerate(open(script, encoding='utf-8', errors='replace'), 1):
        if l.startswith('*'):
            label = l.split()[0].split(';')[0]
            if label.startswith('*tm_'):
                scene = label[1:].split('__')[0]
        elif l.startswith('seldisable '):
            disabled.add(int(l.split()[1]))
            continue
        elif l.startswith('btnwait '):
            for sc, b in buttons:
                if sc == scene:
                    print(f'{label}:{n}={b}')
        elif l.startswith('select '):
            opts = [text.get(h, h) for h in re.findall(r'"\^(\w+)"', l)]
            pick = next((i for i, o in enumerate(opts, 1) if o in take), None)
            if pick is None and avoid & set(opts):
                pick = next((i for i, o in enumerate(opts, 1) if o not in avoid and i not in disabled), None)
            if pick:
                print(f'{label}:{n}={pick}')
                seen.update(opts)
        disabled = set()
    for t in sorted((take | avoid) - seen):
        print(f'# not found: {t}', file=sys.stderr)


if __name__ == '__main__':
    main()
