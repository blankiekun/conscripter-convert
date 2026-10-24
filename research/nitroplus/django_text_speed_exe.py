"""Django.exe's per-character text time, run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/django_text_speed_exe.py <unpacked 1.01 Django.exe>

reset_system_defaults (0045cb30) sets the defaults; the exe's own
#SYSTEM_text_speed store (set_system_variable case 0xb, 0045f11a-0045f12c:
the value x 0.001 into the float at 005293d0) sets each text speed. Then
the message window's frame function 00461e40 runs over a message of
characters with the clock (timeGetTime) at 0, 1, 2, ... ms since its last
character; each call to 00465100 (one character put) is counted. Prints,
per text speed, the first elapsed ms that puts a character (0: the first
call puts them all) and the characters put after 1000 ms, as the Rust
fixture in django/screens.rs's tests.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

FRAME, PUT, STORE, STORED = 0x461e40, 0x465100, 0x45f11a, 0x45f12c
P, S = 0xd0000000, 0xd0100000  # the window object (param_1) and its text state (param_1 + 0x808)
CHARS = 0x700

exe = Exe(sys.argv[1])
exe.uc.mem_map(P, 0x1000)
exe.uc.mem_map(S, 0x70000)
exe.uc.mem_map(0xd0200000, 0x1000)  # a variable for the store: its double at +0x88
# its setup calls (window, sound, threads) succeed and do nothing; then the defaults are stored
for a, v in ((0x460440, 1), (0x4010a0, 1), (0x460370, 1), (0x465d30, 1), (0x45ffb0, 0),
             (0x460080, 0), (0x44c3e0, 0), (0x44d920, 0), (0x4600a0, 0)):
    exe.stubs[a] = v
exe.call(0x45cb30, [0, 0, ("&", "i", 0), 0])
for a in (0x460440, 0x4010a0, 0x460370, 0x465d30, 0x45ffb0, 0x460080, 0x44c3e0, 0x44d920, 0x4600a0):
    del exe.stubs[a]
assert exe.read(0x5293d0, 4) == struct.pack("<f", 0.5), "reset_system_defaults: text speed not 0.5"
level = struct.unpack("<I", exe.read(0x529390, 4))[0]
factor = struct.unpack("<d", exe.read(0x529408 + 8 * level, 8))[0]
# 004605b0 sets the interval 005295b0 to 30 when the scripts set no $SYSTEM_text_interval
exe.uc.mem_write(0x5295b0, struct.pack("<I", 30))

now = [0]
puts = [0]
exe.stub_import("WINMM.dll!timeGetTime", lambda e, at: now[0])
exe.stubs[PUT] = lambda e, at: puts.__setitem__(0, puts[0] + 1) or 0


def w(addr, v):
    exe.uc.mem_write(addr, struct.pack("<I", v & 0xffffffff))


def frame(t):
    """characters put by one frame at t ms after the last character"""
    exe.uc.mem_write(S, bytes(0x70000))
    w(P + 0x808, S)
    w(S + 0x2064, -1)  # no new page pending
    w(S + 0x2c, -1)
    w(S + 0x18, 1)  # the page already shown
    for i in range(0x800):
        w(S + 0x58114 + 4 * i, -1)  # no interval tag at any character
    for i in range(CHARS):
        w(S + 0x4164 + 0x54 * i, 0x41)  # a character
    now[0], puts[0] = t, 0
    exe.call(FRAME, [P])
    return puts[0]


def set_text_speed(v):
    exe.uc.mem_write(0xd0200000 + 0x88, struct.pack("<d", float(v)))
    exe.uc.reg_write(__import__("unicorn").x86_const.UC_X86_REG_EBP, 0xd0200000)
    exe.uc.emu_start(STORE, STORED)  # up to its jmp out of the switch


print(f"play_speed level {level}, factor {factor}, interval 30")
for v in (0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000):
    set_text_speed(v)
    f = struct.unpack("<f", exe.read(0x5293d0, 4))[0]
    first = next(t for t in range(0, 200) if frame(t))
    print(f"text_speed {v:4} float {f:.9f} first_char_ms {first:3} chars_after_1000ms {frame(1000)}")
