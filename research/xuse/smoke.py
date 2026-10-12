"""Headless ONScripter smoke test with scripted clicks.

usage: python3 -I smoke.py GAME_DIR WORK_DIR CLICKS [--timeout S] [--onscripter PATH]

GAME_DIR holds the converted 0.txt, arc.nsa and default.ttf; WORK_DIR gets a
patched 0.txt, symlinks to the rest, and the screenshots (shotNN.png).

CLICKS is a comma-separated queue consumed at every wait:
  at a btnwait (menus, link pages): the number is the clicked button
      (0 = click elsewhere, -1 = right-click);
  at a text page wait: 0 = click on; with the button bar (textgosub in
      0.txt) the bar's button numbers (1001 auto, 1002 skip, 1003 log,
      1004 save, 1005 load, 1007 hide); without it 998 = open the save
      menu, 997 = the load menu;
  anywhere: 999 = screenshot (shot01.png, ...), then take the next entry.
When the queue is empty the game ends.

Note: the save menu's thumbnail uses the same screenshot buffer, so a 999
between opening the save menu and saving makes the thumbnail a menu shot.
Effects and delays are shortened to 1 ms.
"""
import os, re, subprocess, sys

game, work, clicks = sys.argv[1], sys.argv[2], sys.argv[3]
timeout = int(sys.argv[sys.argv.index('--timeout') + 1]) if '--timeout' in sys.argv else 60
ons = sys.argv[sys.argv.index('--onscripter') + 1] if '--onscripter' in sys.argv else \
    os.path.join(os.path.dirname(__file__), '..', '..', 'out', 'onscripter')
os.makedirs(work, exist_ok=True)
for f in ('arc.nsa', 'default.ttf'):
    dst = os.path.join(work, f)
    if not os.path.lexists(dst):
        os.symlink(os.path.abspath(os.path.join(game, f)), dst)
for f in os.listdir(work):
    if f.startswith('shot') or f.startswith('save') or f in ('gloval.sav', 'envdata'):
        os.remove(os.path.join(work, f))

src = open(os.path.join(game, '0.txt'), 'rb').read()
src = re.sub(rb'^effect (\d+),(\d+),\d+', rb'effect \1,\2,1', src, flags=re.M)
src = re.sub(rb'^delay \d+', b'delay 1', src, flags=re.M)
has_bar = b'\ntextgosub ' in src
src = src.replace(b'textbtnwait %9\n', b'gosub *smoke_pop\n')
src = src.replace(b'btnwait %9\n', b'gosub *smoke_pop\n')
queue = [int(c) for c in clicks.split(',') if c.strip()]
harness = b'''
*smoke_pop
if %993 >= %994 end
mov %991,%993+1000
mov %990,%%991
add %993,1
if %990 == 999 goto *smoke_shot
mov %9,%990
return
*smoke_shot
add %992,1
mov $991,"shot"
if %992 < 10 add $991,"0"
itoa $992,%992
add $991,$992
add $991,".bmp"
getscreenshot 800,600
savescreenshot $991
goto *smoke_pop
*smoke_text
gosub *smoke_pop
if %9 == 998 goto *smoke_save
if %9 == 997 goto *smoke_load
texec
return
*smoke_save
mov %101,1
gosub *vn2ons_save
goto *smoke_text
*smoke_load
mov %101,1
gosub *vn2ons_load
goto *smoke_text
'''
init = b'mov %994,' + str(len(queue)).encode() + b'\n' + b''.join(
    b'mov %' + str(1000 + i).encode() + b',' + str(c).encode() + b'\n' for i, c in enumerate(queue))
if not has_bar:
    src = src.replace(b'\ngame\n', b'\ntextgosub *smoke_text\ngame\n', 1)
src = src.replace(b'\n*start\n', b'\n*start\n' + init, 1)
src = src.rstrip(b'\n') + b'\n' + harness
open(os.path.join(work, '0.txt'), 'wb').write(src)

env = dict(os.environ, SDL_VIDEODRIVER='dummy', SDL_AUDIODRIVER='dummy')
r = subprocess.run(['timeout', '-k', '5', str(timeout), os.path.abspath(ons), '--root', '.'],
                   cwd=work, env=env, capture_output=True, text=True, errors='replace')
print('exit', r.returncode)
print('\n'.join(l for l in (r.stdout + r.stderr).splitlines()
                if not l.startswith(('Initialize', 'Display:', 'Audio:'))))
for f in sorted(os.listdir(work)):
    if f.startswith('shot') and f.endswith('.bmp'):
        subprocess.run(['magick', os.path.join(work, f), os.path.join(work, f[:-4] + '.png')])
        os.remove(os.path.join(work, f))
        print('wrote', os.path.join(work, f[:-4] + '.png'))
