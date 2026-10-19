#!/usr/bin/env python3
"""B, a timeboxed prototype: run Muramasa.exe's NSS interpreter under Unicorn
from a scene dump (ntrace.sh --scene-dumps, the record's th/ctx), statement
by statement, as the process thread's loop (0x4708b0) does, and log each
statement the dispatcher gets until something stops it.

    research/tools/.venv/bin/python -I research/nitroplus/interp.py <Muramasa.exe> <dump prefix> <th> <ctx> [--steps N]
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from sched_harness import Harness  # noqa: E402
from unicorn import UC_HOOK_CODE  # noqa: E402
from unicorn import x86_const as X  # noqa: E402

RUN_STATEMENT, CALL_DISPATCH, GETPTD_NOEXIT = 0x470a50, 0x470b7c, 0x4d9f21
NEXT, PREV = 0x7ab * 4, 0x7ac * 4  # a process frame's callee / caller (the thread loop's frame stack)


def main():
    exe, dump, th, ctx = sys.argv[1], sys.argv[2], int(sys.argv[3], 16), int(sys.argv[4], 16)
    steps = int(sys.argv[sys.argv.index("--steps") + 1]) if "--steps" in sys.argv else 200
    h = Harness(exe, dump)
    e = h.e
    ptd = h.alloc(0x214)  # VS2005 _tiddata: ptmbcinfo +0x68, ptlocinfo +0x6c, the process's current ones
    e.uc.mem_write(ptd + 0x68, struct.pack("<II", e.ptr(0x533518), e.ptr(0x5336f8)))
    e.stubs[GETPTD_NOEXIT] = ptd  # the CRT's per-thread data: one zeroed block (the dump has no thread's)
    for name in ("KERNEL32.dll!EnterCriticalSection", "KERNEL32.dll!LeaveCriticalSection", "KERNEL32.dll!SetEvent",
                 "KERNEL32.dll!ResetEvent", "USER32.dll!PostMessageA", "KERNEL32.dll!Sleep"):
        try:
            e.stub_import(name, 1 if "Sleep" not in name else (lambda e_, a: setattr(h, "ms", h.ms + struct.unpack("<I", e_.read(a, 4))[0]) or 0))
        except KeyError:
            pass
    said = []

    def at_dispatch(uc, addr, size, data):
        edi = uc.reg_read(X.UC_X86_REG_EDI)
        p = struct.unpack("<I", bytes(uc.mem_read(edi, 4)))[0]
        if not said:
            print(f"  (pos at {edi:#x}, process {th:#x} {edi - th:+#x})")
        said.append(bytes(uc.mem_read(p, 60)).split(b"\n")[0].decode("cp932", "replace").strip())
    e.uc.hook_add(UC_HOOK_CODE, at_dispatch, begin=CALL_DISPATCH, end=CALL_DISPATCH)
    ring = []
    from unicorn import UC_HOOK_BLOCK
    e.uc.hook_add(UC_HOOK_BLOCK, lambda uc, a, sz, d: (ring.append(a), len(ring) > 40 and ring.pop(0)))
    funcs = sorted((int(l.split("\t")[0], 16), l.split("\t")[1]) for l in open(os.environ["FUNCS"]) if l[0] != "a") if "FUNCS" in os.environ else []

    def where(a):
        import bisect
        i = bisect.bisect_right([f for f, _ in funcs], a) - 1
        return f"{a:#x} {funcs[i][1]}+{a - funcs[i][0]:#x}" if i >= 0 else hex(a)
    # the dump stands on the chain's own `Load(i+1);` (scene_saves.py): go on after it
    at = e.ptr(th + 0xdfc)
    text = e.read(at, 16)
    if text.startswith(b"Load("):
        after = at + text.index(b";") + 1
        e.uc.mem_write(th + 0xdfc, struct.pack("<I", after))
        e.uc.mem_write(th + 0xe00, struct.pack("<I", after))
    p = th
    for n in range(steps):
        before = len(e.hits)
        try:
            e.call(RUN_STATEMENT, [ctx, p], limit=50_000_000)
        except SystemExit as x:
            print(f"step {n}: stopped: {x}")
            print("  at", where(e._pc()), " regs", {r: hex(e.uc.reg_read(getattr(X, "UC_X86_REG_" + r))) for r in "EAX ECX EDX EBX ESI EDI".split()})
            print("  last blocks:", " < ".join(where(a) for a in ring[::-1][:12]))
            print("  stub hits:", [(hex(a), nm) for a, nm, _ in e.hits[-8:]])
            break
        stubs = sorted({nm or hex(a) for a, nm, _ in e.hits[before:]})
        state = struct.unpack("<I", e.read(0x5d4040, 4))[0]
        print(f"step {n}: {said[-1] if said else '-'}  state {state:#x} frame {p:#x} next {e.ptr(p + NEXT):#x} prev {e.ptr(p + PREV):#x}  {' '.join(stubs)[:120]}")
        said.clear()
        nxt = e.ptr(p + NEXT)
        if nxt:
            e.uc.mem_write(nxt + PREV, struct.pack("<I", p))
            p = nxt
        elif e.ptr(p + PREV):
            q = e.ptr(p + PREV)
            e.uc.mem_write(p + PREV, bytes(4))
            e.uc.mem_write(q + NEXT, bytes(4))
            p = q


if __name__ == "__main__":
    main()
