"""月姫's text line pitch run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/typemoon/linefeed_exe.py <main NSO>

The TextWriter's line feed after '\\n' is FUN_00245170(writer) = font line
feed (font vtable +0x58) * writer scaleY (+0x10) + line space (+0x44). The
writer's font is a PairFont (vtable +0x58 = FUN_0023d56c, the larger of its
two fonts' feeds): the bffnt (FUN_0023f750, FINF lineFeed, 47 in
exfont_nx01.bffnt) and the ScalableFont (FUN_00240484, +0x28 set by
FUN_002402a4 = (int)(rec[1] * size + 0.5) + offset, rec[1] = (usWinAscent +
usWinDescent) / unitsPerEm of the set's font with the tallest bbox,
FUN_00243ed0: Seurat (ja3), (1029 + 217) / 1000). Built from fake objects:
FUN_002402a4 first (FUN_00243ed0 stubbed with that rec, size 40,
offset 0), then FUN_00245170 for JP (scaleY 0.76) and EN (0.684), line
space 16 (FUN_00245124: 720 / 45).
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

exe = Exe(sys.argv[1])
M = 0x60000000
exe._map(M, bytes(0x4000))
w = lambda a, fmt, *v: exe.uc.mem_write(a, struct.pack("<" + fmt, *v))  # noqa: E731

REC, PARAM, SCAL, BFF, FINF, PAIR, WRITER = (M + k * 0x400 for k in range(7))
VT_SCAL, VT_BFF, VT_PAIR = M + 0x2000, M + 0x2400, M + 0x2800
w(REC, "3f", 1029 / 1000, (1029 + 217) / 1000, 1029 / 1000)
w(PARAM, "QI", M, 40)       # engine (nonzero), size 40 (FUN_001c9f10)
w(PARAM + 0x14, "i", 0)     # offset (FUN_00240200: 0)
exe.stubs[0x243ed0] = REC   # FUN_00243ed0: the set's tallest font's rec
exe.stubs[0x240390] = 0
exe.call(0x2402a4, [SCAL, PARAM])
feed = struct.unpack("<i", exe.read(SCAL + 0x28, 4))[0]
print(f"ScalableFont feed (FUN_002402a4 +0x28): {feed}")

w(VT_SCAL + 0x58, "Q", 0x240484)
w(VT_BFF + 0x58, "Q", 0x23f750)
w(VT_PAIR + 0x58, "Q", 0x23d56c)
w(SCAL, "Q", VT_SCAL)
w(BFF, "Q", VT_BFF)
w(BFF + 0x20, "Q", FINF)
w(FINF + 4, "h", 47)        # exfont_nx01.bffnt FINF lineFeed
w(PAIR, "Q", VT_PAIR)
w(PAIR + 0x10, "QQ", BFF, SCAL)
print(f"PairFont feed (FUN_0023d56c): {exe.call(0x23d56c, [PAIR])[0]}")
for lang, scale in (("JP", 0.76), ("EN", 0.76 * 0.9)):
    w(WRITER + 0x10, "f", scale)
    w(WRITER + 0x28, "Q", PAIR)
    w(WRITER + 0x44, "f", 16.0)
    pitch = exe.call(0x245170, [WRITER], ret="float")[0]
    print(f"{lang}: line pitch (FUN_00245170) {pitch:.4f} px at 720")
