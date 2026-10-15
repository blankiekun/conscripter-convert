"""Writes a --choices file that plays 最果てのイマ through all twelve loops.

usage: python3 -I route.py GAME_DIR/0.txt > route.txt

The heroine links of 最初の日 (docs/games/saihate-no-ima.md, Walkthrough)
are found by their text, since their places (`*label:line`) move with
every build. Loop 1, 6 and 7 click none of them (リピート, リピート, 死);
loops 2-5 and 8-11 click あずさ, さやか, ふえこ, ようこ in turn (a link
clicked hides the others for that loop, and a heroine done hides hers
for the cycle, so each loop's first heroine link shown is the one
wanted). Every other choice (the lore links, the title) is left to the
engine's --auto-choice.
"""
import re, sys

LINKS = ['最初の頃', 'たまに早く出る', '違う制服', '静かな朝']  # あずさ, さやか, ふえこ, ようこ

lines = open(sys.argv[1], 'rb').read().decode('cp932').replace('\r', '').split('\n')
places = []
for word in LINKS:
    start = next(i for i, l in enumerate(lines) if l.startswith(f'if %') and f'strsp 51,"{word}"' in l)
    label = None
    for i in range(start, len(lines)):
        if re.fullmatch(r'\*conv_link\d+_wait', lines[i]):
            label = lines[i]
        if lines[i] == 'btnwait %9':
            places.append(f'{label}:{i + 1}')
            break

def none(loop):
    return [f'# loop {loop}: no heroine'] + [f'{p}\t0' for p in places]

out = none(1)
for loop, p in zip(range(2, 6), places):
    out += [f'# loop {loop}', f'{p}\t1']
out += none(6) + none(7)
for loop, p in zip(range(8, 12), places):
    out += [f'# loop {loop}', f'{p}\t1']
print('\n'.join(out))
