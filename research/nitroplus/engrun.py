"""Skip runs of a converted 装甲悪鬼村正 in CONscripter, one per route
(docs/new-game.md step 4): the engine plays each ending of the choice
paths, every page drawn, so that engine exits deep in the
game show up.

    python3 -I research/nitroplus/engrun.py <game dir> <path file>...
        --engine <conscripter binary> [--display :79] [--out <dir>]
        [--only N] [--all] [--stall 300] [--at <file>]

`nstrace.py` first runs the paths over `<game dir>/0.txt` and records
the options taken, the puzzle results and the story files passed, per
playthrough (each new game and each LOAD starts one; the paths of the
four routes make five, one per ending). Each becomes one engine run, in
a copy of the game (`<out>/<N>/`, symlinks and a patched `0.txt`):
- every `select` takes the recorded option (`gosub *eng_pick`, which
  hands out the next one in `%1098`, then a jump to `*eng_selectN`, one
  `if` per option), and the 立方陣 is skipped with its
  recorded result;
- `*conv_click_wait` returns at once; the `#` flags the run starts with
  (from the routes before it) are set after `erasetextwindow 0`; a path
  that starts with LOAD replays the saved run's options and sets the
  flags of the load time at the first select after the save;
- each story file logs `TRACE:f_<file>` on stderr (a missing-file
  message), the end logs `TRACE:END`.
The engine runs on its own Xvfb display, silent, in skip mode (Ctrl
held down; a key release would end it). A run passes if the engine reached
`TRACE:END`, exited with 0 and passed the same files as nstrace.
Bad ends (game over files), which the paths reach through LOAD, are
left out unless `--all` is given; `--only N` runs only the Nth.
`--at ma03_027vs` starts each run at that story file instead, with
every variable as nstrace had it there (a crafted save: the variables,
not the pictures or the music on screen), to reproduce a problem deep
in a route in seconds; the run then goes on to the route's end.
A run whose trace doesn't move for `--stall` seconds is stopped, with a
screenshot (`<out>/<N>/stall.png`).
"""
import os
import re
import signal
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nstrace  # noqa: E402

PICK, NEXT = 1098, 1099  # %1098: the option to take; %1099: picks so far


class Saves(dict):
    """nstrace's saves, noting the options taken up to each save."""

    def __init__(self, run):
        super().__init__()
        self.run = run
        self.picks = {}

    def __setitem__(self, name, state):
        super().__setitem__(name, state)
        seg = self.run.seg
        self.picks[name] = (dict(seg['globals']), list(seg['picks']), dict(seg['inject']))

    def __getitem__(self, name):
        # LOAD: the replay starts with the saved run's flags and options;
        # the flags of now are set at the next select.
        run = self.run
        start, picks, inject = self.picks[name]
        now = {k: v for k, v in run.var.items() if k >= nstrace.GLOBAL_BORDER}
        run.begin_segment(start, list(picks), {**inject, len(picks): now})
        return super().__getitem__(name)


class Recorder(nstrace.Run):
    def __init__(self, *a, at=None, **k):
        self.at_file = at
        self.segs = []
        self.seg = None
        super().__init__(*a, **k)
        self.saves = Saves(self)

    def finish_segment(self):
        if self.seg is not None and 'files' not in self.seg:
            self.seg['files'] = [f for f, _ in self.files]
            self.seg['ended'] = self.ended

    def begin_segment(self, start, picks, inject):
        self.finish_segment()
        self.seg = {'globals': start, 'picks': picks, 'inject': inject}
        self.segs.append(self.seg)

    def new_game(self):
        self.finish_segment()  # before the reset clears the files
        super().new_game()
        self.begin_segment({k: v for k, v in self.var.items() if k >= nstrace.GLOBAL_BORDER and v}, [], {})

    def enter(self, label):
        # The state at the first start of the file `--at` names.
        if label == f'f_{self.at_file}' and 'at' not in self.seg:
            self.seg['at'] = (dict(self.var), dict(self.svar), len(self.seg['picks']))
        super().enter(label)

    def choose(self, items):
        pc = super().choose(items)
        self.seg['picks'].append(self.files[-1][1][-1][0])
        return pc

    def puzzle(self, pc):
        pc = super().puzzle(pc)
        self.seg['picks'].append(self.files[-1][1][-1][0])
        return pc


def record(script, paths, at=None):
    lines, labels = nstrace.load_script(script)
    run = Recorder(lines, labels, None, True, at=at)
    for p in paths:
        path = nstrace.read_path(p)
        if run.ended is not None and path and path[0][0] not in ('LOAD', 'NEWGAME'):
            path = [('NEWGAME', None)] + path
        run.play(path)
    run.finish_segment()
    if run.problems:
        sys.exit(f'nstrace: {run.problems} problem(s); fix the paths first')
    # NEWGAME and LOAD each start a segment: one engine run each.
    return run.segs


def puzzles(lines):
    """{line of `; tile puzzle`: (result variable, line of the black
    screen after it)}, found as nstrace finds them."""
    found = {}
    for i, line in enumerate(lines):
        if line != '; tile puzzle':
            continue
        answer = next(j for j in range(i, len(lines)) if re.fullmatch(r'\*conv_puzzle\d+_answer', lines[j]))
        result = next(j for j in range(answer, len(lines)) if re.fullmatch(r'mov %\d+,%191', lines[j]))
        black = next(j for j in range(result, len(lines)) if lines[j] == 'bg "#000000",0')
        found[i] = (lines[result].split(',')[0][4:], black)
    return found


def patch(text, seg, at=None):
    lines = text.split('\n')
    tiles = puzzles(lines)
    done = {black: n for n, (_, black) in enumerate(tiles.values())}
    out = []
    selects = []
    started = False
    for i, line in enumerate(lines):
        if line == '*conv_click_wait':
            out += [line, 'return']
        elif line == 'erasetextwindow 0' and not started:
            started = True
            if at:
                # Start at the file, with every variable as nstrace had it.
                var, svar, picks = seg['at']
                movs = [f'mov %{k},{v}' for k, v in sorted(var.items()) if v]
                movs += [f'mov {k},"{v}"' for k, v in sorted(svar.items())]
                movs += [f'mov %{NEXT},{picks}', f'goto *f_{at}']
            else:
                movs = [f'mov %{k},{v}' for k, v in sorted(seg['globals'].items())] + [f'mov %{NEXT},0']
            out.append(line)
            out += [':'.join(movs[i:i + 20]) for i in range(0, len(movs), 20)]
        elif re.fullmatch(r'\*f_\w+', line):
            out += [line, f'lsp 999,"TRACE:{line[1:]}",0,0']
        elif i in tiles:
            n = list(tiles).index(i)
            out += [line, f'gosub *eng_pick:mov {tiles[i][0]},%{PICK}:goto *eng_puzzle{n}_done']
        elif i in done:
            out += [line, f'*eng_puzzle{done[i]}_done']
        elif line == 'end':
            out += ['lsp 999,"TRACE:END",0,0', line]
        elif m := re.search(r'select ((?:"[^"]*"\s*,\s*\*\w+\s*,?\s*)+)$', line):
            # One `if` per line: a false `if` skips the rest of its line.
            targets = re.findall(r'"[^"]*"\s*,\s*\*(\w+)', m.group(1))
            out.append(line[:m.start()] + f'gosub *eng_pick:goto *eng_select{len(selects)}')
            selects.append([f'*eng_select{len(selects)}'] + [f'if %{PICK}=={k} goto *{t}' for k, t in enumerate(targets, 1)])
        else:
            out.append(line)
    for block in selects:
        out += block
    out += ['*eng_pick', f'inc %{NEXT}']
    for k, v in enumerate(seg['picks'], 1):
        movs = [f'mov %{g},{x}' for g, x in sorted(seg['inject'].get(k - 1, {}).items())]
        out.append(f'if %{NEXT}=={k} ' + ':'.join(movs + [f'mov %{PICK},{v}']))
    out += [f'if %{NEXT}>{len(seg["picks"])} lsp 999,"TRACE:NO_PICK",0,0:mov %{PICK},1', 'return', '']
    return '\n'.join(out)


def is_bad_end(file):
    return file is None or file.startswith('mz') or 'gameover' in file


def engine_run(game, folder, engine, display, stall):
    log_path = os.path.join(folder, 'engine.log')
    xvfb = subprocess.Popen(['Xvfb', display, '-nolisten', 'tcp', '-screen', '0', '1024x576x24'],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    env = dict(os.environ, DISPLAY=display, SDL_VIDEODRIVER='x11', SDL_AUDIODRIVER='dummy')
    env.pop('WAYLAND_DISPLAY', None)
    with open(log_path, 'wb') as log:
        eng = subprocess.Popen([engine], cwd=folder, env=env, stdout=log, stderr=subprocess.STDOUT)
    last, moved, why = None, time.time(), None
    try:
        while eng.poll() is None:
            # Held, never released: the engine leaves skip mode on any key
            # release.
            subprocess.run(['xdotool', 'keydown', 'Control_L'], env=env, stderr=subprocess.DEVNULL)
            time.sleep(3)
            trace = traces(log_path)
            if trace[-1:] != last:
                last, moved = trace[-1:], time.time()
            elif time.time() - moved > stall:
                why = f'stalled {stall} s in {last}'
                subprocess.run(['import', '-display', display, '-window', 'root',
                                os.path.join(folder, 'stall.png')], stderr=subprocess.DEVNULL)
                eng.kill()
                break
    finally:
        if eng.poll() is None:
            eng.kill()
        status = eng.wait()
        xvfb.kill()
        xvfb.wait()
    return status, why


def traces(log_path):
    with open(log_path, 'rb') as f:
        names = re.findall(rb'TRACE:(\w+)', f.read())
    out = []
    for n in names:
        n = n.decode()
        if not out or out[-1] != n:
            out.append(n)
    return out


def main():
    # Killed: stop the engine and Xvfb on the way out (`finally`).
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    args = sys.argv[1:]

    def opt(name, default=None):
        if name in args:
            i = args.index(name)
            v = args[i + 1]
            del args[i:i + 2]
            return v
        return default

    engine = opt('--engine')
    engine = engine and os.path.abspath(engine)
    display = opt('--display', ':79')
    out = opt('--out', 'out/muramasa-engrun')
    only = opt('--only')
    stall = int(opt('--stall', '300'))
    at = opt('--at')
    every = '--all' in args
    args = [a for a in args if a != '--all']
    if not engine:
        sys.exit('--engine <conscripter binary> is needed')
    game, paths = args[0], args[1:]
    segs = record(os.path.join(game, '0.txt'), paths, at)
    text = open(os.path.join(game, '0.txt'), 'rb').read().decode('cp932')
    failed = 0
    for n, seg in enumerate(segs, 1):
        if only and str(n) != only or not every and not only and is_bad_end(seg['ended']):
            continue
        if at and 'at' not in seg:
            print(f'== run {n} doesn\'t pass {at}')
            continue
        folder = os.path.join(out, str(n))
        os.makedirs(folder, exist_ok=True)
        for name in os.listdir(game):
            dst = os.path.join(folder, name)
            if name not in ('0.txt', 'gloval.sav', 'envdata', 'kidoku.dat') and not os.path.lexists(dst):
                os.symlink(os.path.abspath(os.path.join(game, name)), dst)
        for name in ('gloval.sav', 'kidoku.dat', 'envdata', 'engine.log', 'stall.png'):
            if os.path.exists(os.path.join(folder, name)):
                os.remove(os.path.join(folder, name))
        open(os.path.join(folder, '0.txt'), 'wb').write(patch(text, seg, at).encode('cp932'))
        print(f'== run {n}: {len(seg["picks"])} options, {len(seg["files"])} files, ends in {seg["ended"]}', flush=True)
        t = time.time()
        status, why = engine_run(game, folder, engine, display, stall)
        got = traces(os.path.join(folder, 'engine.log'))
        want = [f[:-4] if f.endswith('.nss') else f for f in seg['files']]
        if at:
            want = want[want.index(at):]
        files = [g[2:] for g in got if g.startswith('f_')]
        uniq = [f for i, f in enumerate(want) if i == 0 or want[i - 1] != f]
        problems = []
        if why:
            problems.append(why)
        if 'END' not in got:
            problems.append(f'no TRACE:END; exit status {status}; last file {files[-1:] or "none"}')
        elif status != 0:
            problems.append(f'exit status {status}')
        if 'NO_PICK' in got:
            problems.append('more selects than recorded options')
        if files != uniq:
            k = next((i for i, (a, b) in enumerate(zip(files, uniq)) if a != b), min(len(files), len(uniq)))
            problems.append(f'files differ at {k}: engine {files[k:k + 3]}, nstrace {uniq[k:k + 3]}')
        print(f'  {len(files)} files in {time.time() - t:.0f} s: ' + ('; '.join(problems) or 'ok'), flush=True)
        failed += bool(problems)
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
