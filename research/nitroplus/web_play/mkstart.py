"""mkstart.py <build> <scratch> [--at file] [--path p.txt] [--args 'a b']: a scratch
game for the web build: symlinks to the build's read-only inputs, 0.txt patched by
engrun (route choices; --at starts at that story file with nstrace's variables),
cons_index.json (lazyload, every file, extra engine args)."""
import sys, os, json, shutil
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
import engrun
a = sys.argv[1:]
def opt(n, d=None):
    return a[a.index(n) + 1] if n in a else d
build, scratch = os.path.abspath(a[0]), a[1]
at, path, extra = opt('--at'), opt('--path'), opt('--args', '').split()
shutil.rmtree(scratch, ignore_errors=True); os.makedirs(scratch)
for name in os.listdir(build):
    if name not in ('0.txt', 'gloval.sav', 'kidoku.dat', 'envdata', 'saves', 'cons_index.json') and not name.endswith(('.log', '.jsonl')):
        os.symlink(os.path.join(build, name), os.path.join(scratch, name))
text = open(os.path.join(build, '0.txt'), 'rb').read().decode('utf-8')
if path:
    segs = engrun.record(os.path.join(build, '0.txt'), [path], at)
    seg = next(s for s in segs if not at or 'at' in s)
    text = engrun.patch(text, seg, at)
open(os.path.join(scratch, '0.txt'), 'wb').write(text.encode('utf-8'))
files = []
for root, _d, names in os.walk(scratch, followlinks=True):
    for n in names:
        files.append({"path": os.path.relpath(os.path.join(root, n), scratch)})
t = 'muramasa'
json.dump({"title": t, "gamedir": "/cons/" + t, "savedir": "/cons_save/" + t, "args": extra,
           "lazyload": True, "files": files}, open(os.path.join(scratch, 'cons_index.json'), 'w'))
print(len(files), 'files;', 'at', at, 'path', path)
