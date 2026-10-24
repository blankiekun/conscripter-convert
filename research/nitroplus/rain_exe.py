"""Muramasa.exe's CreateEffect "Rain" streak, run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/rain_exe.py <Muramasa.exe>

FUN_0042f680 (startup) fills the 196 ARGB pixels at 0x558ad8 that the Rain
update FUN_00436390 copies down one texture column per drop: grey 0xADADAD,
alpha 20 + floor(i * 8/7) for i = 0..97 (K = 1.1428571939 at 0x515008),
then one less a pixel to row 195. Prints the alphas, top row first.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

TABLE, ROWS = 0x558ad8, 196

exe = Exe(sys.argv[1])
exe.call(0x42f680, [])
px = exe.read(TABLE, 4 * ROWS)
assert all(px[i * 4:i * 4 + 3] == b"\xad\xad\xad" for i in range(ROWS))
print(",".join(str(px[i * 4 + 3]) for i in range(ROWS)))
