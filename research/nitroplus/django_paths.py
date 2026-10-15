"""続・殺戮のジャンゴ: --choices files for the engine, one per path.

Usage: python3 -I django_paths.py <game-cons>/0.txt <out dir>

Writes good.txt (every choice answered the way that goes on) and
bad01.txt … bad10.txt (the good answers up to that choice, then the
deadly one), in the order the choices come in the story. The deadly
answer of each choice is from docs/games/django.md ("Story structure").
"""
import os, re, sys

# Deadly option (1 or 2) of each choice, in story order.
BAD = [1, 2, 1, 1, 2, 1, 2, 1, 2, 1]

script, out = sys.argv[1], sys.argv[2]
lines = open(script, encoding='cp932', errors='replace').read().split('\n')
places = []
label = None
for n, line in enumerate(lines, 1):
    if line.startswith('*'):
        label = line.split()[0]
    elif line.startswith('select ') or (line.startswith('btnwait ') and label.startswith('*conv_cplate')):
        # A select, or a choice of plates (its btnwait's result: 1, 2).
        places.append(f'{label}:{n}')
if len(places) != len(BAD):
    sys.exit(f'{len(places)} choices found, expected {len(BAD)}')
os.makedirs(out, exist_ok=True)

def write(name, answers):
    with open(os.path.join(out, name), 'w', encoding='utf-8') as f:
        for at, a in zip(places, answers):
            f.write(f'{at}\t{a}\n')

good = [3 - b for b in BAD]
write('good.txt', good)
for k, b in enumerate(BAD):
    write(f'bad{k + 1:02}.txt', good[:k] + [b])
print(f'{len(places)} choices; paths in {out}')
