"""python3 -I research/tools/test_cmdcheck.py"""
import contextlib
import io
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cmdcheck  # noqa: E402

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
script = (b'defsub mine\nnumalias flag,5\nmov %1,2:foo 3\nif %a == 1 && fchk "x" && flag == 1 zzz 1\n'
          b'_baz:quux "a:b":mine\n*l\n;nope\n\x82\xa0text nope\nnotif %v mine\n')
with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as f:
    f.write(script)
out = io.StringIO()
with contextlib.redirect_stdout(out):
    status = cmdcheck.check(os.path.join(root, 'engine/src/cons/ONScripter_lut.cpp'), f.name)
os.unlink(f.name)
found = [line.split('unknown command ')[1].split(' ')[0] for line in out.getvalue().splitlines()]
assert status == 1 and found == ['foo', 'zzz', 'baz', 'quux'], found
print('OK')
