"""Muramasa.exe's SetVolume, run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/volume_exe.py <Muramasa.exe>

sound_set_volume 00475f50 (EAX: the volume string, cdecl: the sound, time 0)
sets the DirectSound buffer's volume (vtable +0x3c, SetVolume, hundredths of a
dB) to 3322*log10(f*v/1000), f the float at +0x118 (1.0 here). The sound is a
zeroed block in the heap; its buffer's SetVolume points at a `ret 8` in the
image, hooked to read the value. Its lock FUN_00478620 is stubbed. Prints the
Rust table `EXE_VOLUMES` (volume, amplitude percent) sounds.rs's tests check
`percent` against.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe, HEAP  # noqa: E402
from unicorn import UC_HOOK_CODE  # noqa: E402
from unicorn import x86_const as X  # noqa: E402

SET_VOLUME, LOCK = 0x475f50, 0x478620
SOUND, BUFFER, VTABLE, TEXT = HEAP + 0x80000, HEAP + 0x81000, HEAP + 0x82000, HEAP + 0x83000

exe = Exe(sys.argv[1])
exe.stubs[LOCK] = 0
lo, hi = exe.range
ret8 = lo + exe.read(lo, hi - lo).index(b"\xc2\x08\x00")
got = []
exe.uc.hook_add(UC_HOOK_CODE, lambda uc, *_: got.append(struct.unpack("<i", exe.read(uc.reg_read(X.UC_X86_REG_ESP) + 8, 4))[0]),
                begin=ret8, end=ret8)
rows = []
for v in [1, 10, 50, 100, 200, 250, 300, 400, 500, 600, 700, 750, 800, 900, 950, 1000]:
    sound = bytearray(0x400)
    struct.pack_into("<f", sound, 0x118, 1.0)
    struct.pack_into("<I", sound, 0x3dc, BUFFER)
    exe.uc.mem_write(SOUND, bytes(sound))
    exe.uc.mem_write(BUFFER, struct.pack("<I", VTABLE))
    exe.uc.mem_write(VTABLE + 0x3c, struct.pack("<I", ret8))
    exe.uc.mem_write(TEXT, f"{v}\0".encode())
    got.clear()
    exe.call(SET_VOLUME, [SOUND, 0], regs={"eax": TEXT})
    db = got[-1]
    rows.append(f"({v}, {db})")
print(f"    // (volume, DirectSound hundredths of a dB) from sound_set_volume 00475f50\n"
      f"    const EXE_VOLUMES: [(i64, i32); {len(rows)}] = [{', '.join(rows)}];")
