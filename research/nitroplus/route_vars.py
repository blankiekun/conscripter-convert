"""The variables of a converted 装甲悪鬼村正 at story files, as its routes
reach them: one `--vars` file a label (engine/DEBUG.md, "Starting
anywhere"), for `research/tools/test_saves.py --vars-dir`.

    python3 -I research/nitroplus/route_vars.py <game dir> <out dir> <path file>... --labels f_ma01_022vs,...

`nstrace.py` plays the paths over `<game dir>/0.txt` in order (as
engrun.py does: the routes one after another, so later routes have the
earlier ones' `#` flags); each label's file holds every variable at the
first time a path enters it. A label no path reaches gets no file.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nstrace  # noqa: E402
from engrun import Recorder  # noqa: E402


class Snapshots(Recorder):
    def enter(self, label):
        if label in self.want and label not in self.seen:
            self.seen[label] = (dict(self.var), dict(self.svar))
        super().enter(label)


def main():
    args = sys.argv[1:]
    i = args.index('--labels')
    want = set(args[i + 1].split(','))
    game, out, *paths = args[:i] + args[i + 2:]
    lines, labels = nstrace.load_script(os.path.join(game, '0.txt'))
    run = Snapshots(lines, labels, None, True)
    run.want, run.seen = want, {}
    for p in paths:
        path = nstrace.read_path(p)
        if run.ended is not None and path and path[0][0] not in ('LOAD', 'NEWGAME'):
            path = [('NEWGAME', None)] + path
        run.play(path)
    os.makedirs(out, exist_ok=True)
    for label, (var, svar) in sorted(run.seen.items()):
        with open(os.path.join(out, label + '.txt'), 'w', encoding='utf-8') as f:
            f.write(f'# {label}, from the route paths (route_vars.py)\n')
            f.writelines(f'%{k}={v}\n' for k, v in sorted(var.items()) if v)
            f.writelines(f'{k if str(k).startswith("$") else "$" + str(k)}={v}\n' for k, v in sorted(svar.items(), key=str) if v)
    for label in sorted(want - set(run.seen)):
        print(f'{label}: no path reaches it')
    print(f'{len(run.seen)} of {len(want)} labels')


if __name__ == '__main__':
    main()
