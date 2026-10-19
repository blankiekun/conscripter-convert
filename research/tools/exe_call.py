#!/usr/bin/env python3
"""Run one function of a game's executable under Unicorn and print what it returns.

    research/tools/.venv/bin/python -I research/tools/exe_call.py <exe> <addr> [arg ...]
        [--cc cdecl|stdcall|thiscall|fastcall] [--ret int|float|double]
        [--stub ADDR=VALUE ...] [--dump ADDR:LEN ...] [--arm] [--trace]

<exe>: a PE (x86-32, loaded at its ImageBase; imports go to unmapped stubs),
an ELF (the Vita's decrypted eboot, type 0xfe04, ARM, loaded at its own
addresses, which its code is linked for) or an NSO (Switch AArch64, loaded at
0x100000 as Ghidra loads nso2elf.py's ELF, with its relocations applied).
<addr>: the function's address, or `*EXPR` to read it from a pointer table
(`*0x5e40a0+4*17`). ARM code is Thumb unless --arm.
Arguments: `5`, `0x10` int; `1.5` float; `d:1.5` double; `&1.5`, `&i:5`,
`&d:1.5` a pointer to one; `buf:N` a pointer to N zero bytes (printed after).
Float arguments go on the stack (x86), in s/d registers (ARM hard-float,
AArch64). A call outside the image (an import) returns 0, or the value given
with --stub ADDR=VALUE, and is reported; so is a hit on an address --stub
names inside the image (stubbing an inner function). A stub's value may be
a function of (exe, address of its arguments): a clock, an allocator. A
Win32 import in STDCALL pops its arguments.

As a module: `Exe(path).call(addr, args, cc=..., ret=...)` for many calls;
`Exe(path).load_dump(prefix)` first maps a process dump (ntrace.c,
NTRACE_DUMPS) so calls run against the game's real state. --dump-in PREFIX
does that from the command line.
"""
import argparse
import io
import re
import struct
import sys
import os

from unicorn import (Uc, UcError, UC_ARCH_X86, UC_MODE_32, UC_ARCH_ARM, UC_MODE_THUMB, UC_MODE_ARM,
                     UC_ARCH_ARM64, UC_MODE_ARM as UC_MODE_LE, UC_HOOK_CODE, UC_PROT_ALL)
from unicorn import x86_const as X, arm_const as A, arm64_const as A64

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "ghidra"))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # dumppack
from nso2elf import lz4_block  # noqa: E402

STUBS = 0xf0000000        # imports point here, unmapped: a fetch is a stub hit
STACK, STACK_SIZE = 0xe0000000, 0x100000
HEAP = 0xe8000000         # argument buffers
RET = 0xef000000          # the return address; the call ends when it is reached
TIB = 0xef100000          # x86: the thread block FS points at
NSO_BASE = 0x100000       # where Ghidra puts an ET_DYN image
PAGE = 0x1000
# argument counts of the stdcall Win32 imports a stub pops; one not listed is
# looked up in the mingw-w64 import libraries (stdcall_counts)
STDCALL = {"timeGetTime": 0, "GetTickCount": 0, "GetProcessHeap": 0, "GetCurrentThreadId": 0, "GetForegroundWindow": 0,
           "QueryPerformanceCounter": 1, "Sleep": 1, "ReleaseMutex": 1, "SetEvent": 1, "ResetEvent": 1, "CloseHandle": 1,
           "EnterCriticalSection": 1, "LeaveCriticalSection": 1, "SuspendThread": 1, "ResumeThread": 1,
           "WaitForSingleObject": 2, "HeapAlloc": 3, "HeapFree": 3, "HeapReAlloc": 4, "PostMessageA": 4,
           "SendMessageA": 4, "VirtualAlloc": 4, "VirtualFree": 3, "GlobalAlloc": 2, "GlobalFree": 1,
           "TlsGetValue": 1, "TlsSetValue": 2, "FlsGetValue": 1, "FlsSetValue": 2, "GetLastError": 0,
           "SetLastError": 1, "GetCurrentProcess": 0, "InterlockedIncrement": 1, "InterlockedDecrement": 1,
           "InterlockedExchange": 2, "OutputDebugStringA": 1, "timeBeginPeriod": 1, "timeEndPeriod": 1,
           "EncodePointer": 1, "DecodePointer": 1, "GetModuleHandleA": 1, "GetProcAddress": 2}


def stdcall_counts():
    """argument counts of every stdcall export in the mingw-w64 import libraries
    (`_SetCursorPos@8`), when the cross toolchain is there; else {}"""
    import glob
    import subprocess
    out = {}
    libs = glob.glob("/usr/i686-w64-mingw32/lib/lib*.a")
    if not libs:
        return out
    try:
        nm = subprocess.run(["i686-w64-mingw32-nm", "-g"] + libs, capture_output=True, text=True).stdout
    except OSError:
        return out
    for m in re.finditer(r" T _(\w+)@(\d+)$", nm, re.M):
        out.setdefault(m.group(1), int(m.group(2)) // 4)
    return out


def up(n):
    return (n + PAGE - 1) & ~(PAGE - 1)


class Exe:
    def __init__(self, path, thumb=True):
        d = open(path, "rb").read()
        self.imports = {}  # stub address -> name
        if d[:2] == b"MZ":
            self.arch = "x86"
            self.uc = Uc(UC_ARCH_X86, UC_MODE_32)
            self._pe(d)
        elif d[:4] == b"\x7fELF":
            self.arch = "arm"
            self.uc = Uc(UC_ARCH_ARM, UC_MODE_THUMB if thumb else UC_MODE_ARM)
            self.thumb = thumb
            self._elf(d)
            # VFP on: CPACR full access to cp10/cp11, FPEXC.EN
            self.uc.reg_write(A.UC_ARM_REG_C1_C0_2, self.uc.reg_read(A.UC_ARM_REG_C1_C0_2) | 0xf00000)
            self.uc.reg_write(A.UC_ARM_REG_FPEXC, 0x40000000)
        elif d[:4] == b"NSO0":
            self.arch = "arm64"
            self.uc = Uc(UC_ARCH_ARM64, UC_MODE_LE)
            self._nso(d)
            self.uc.reg_write(A64.UC_ARM64_REG_CPACR_EL1, 0x300000)  # FP/SIMD on
        else:
            sys.exit("not a PE, ELF or NSO")
        self.uc.mem_map(STACK, STACK_SIZE)
        self.uc.mem_map(HEAP, 0x100000)
        self.uc.mem_map(RET, PAGE)
        if self.arch == "x86":  # at the return: fstp qword [RET+0x100] (st0 if a float came back)
            self.uc.mem_write(RET, b"\xdd\x1d" + struct.pack("<I", RET + 0x100))
            # a thread block at FS (MSVC's SEH prologs read fs:[0]): an empty SEH chain, the stack's bounds, itself
            # (32-bit mode has no FS base register: a GDT entry with the block as its base, at TIB + 0x800)
            self.uc.mem_map(TIB, PAGE)
            self.uc.mem_write(TIB, struct.pack("<7I", 0xffffffff, STACK + STACK_SIZE, STACK, 0, 0, 0, TIB))
            def seg(base, limit, access, flags):  # a GDT descriptor
                return (limit & 0xffff | (base & 0xffffff) << 16 | access << 40 | (limit >> 16) << 48
                        | flags << 52 | (base >> 24) << 56)
            # 1: flat data (DS, ES, SS once a GDT is set), 2: the thread block (FS); ring 0, 32-bit
            self.uc.mem_write(TIB + 0x800, struct.pack("<3Q", 0, seg(0, 0xfffff, 0x93, 0xc), seg(TIB, 0xfff, 0x93, 0x4)))
            self.uc.reg_write(X.UC_X86_REG_GDTR, (0, TIB + 0x800, 0x18 - 1, 0))
            for r in (X.UC_X86_REG_DS, X.UC_X86_REG_ES, X.UC_X86_REG_SS):
                self.uc.reg_write(r, 0x8)
            self.uc.reg_write(X.UC_X86_REG_FS, 0x10)
        self.stubs = {}   # address -> value, for --stub
        self.hits = []

    def _map(self, addr, data, size=0):
        lo, hi = addr & ~(PAGE - 1), up(addr + max(size, len(data)))
        try:
            self.uc.mem_map(lo, hi - lo, UC_PROT_ALL)
        except UcError:  # overlaps a neighbouring segment's pages: map the rest page by page
            for p in range(lo, hi, PAGE):
                try:
                    self.uc.mem_map(p, PAGE, UC_PROT_ALL)
                except UcError:
                    pass
        self.uc.mem_write(addr, bytes(data))

    def _pe(self, d):
        pe = struct.unpack_from("<I", d, 0x3c)[0]
        nsec, opt = struct.unpack_from("<H", d, pe + 6)[0], struct.unpack_from("<H", d, pe + 20)[0]
        o = pe + 24
        base, size_img = struct.unpack_from("<I", d, o + 28)[0], struct.unpack_from("<I", d, o + 56)[0]
        hdr = struct.unpack_from("<I", d, o + 60)[0]
        img = bytearray(up(size_img))
        img[:hdr] = d[:hdr]
        self.code, self.iat = [], {}  # executable sections (va, size); IAT slot -> stub: load_dump puts them back
        for s in range(nsec):
            vsize, va, rsize, roff = struct.unpack_from("<IIII", d, o + opt + 40 * s + 8)
            if struct.unpack_from("<I", d, o + opt + 40 * s + 36)[0] & 0x20000000:
                self.code.append((base + va, min(rsize, vsize or rsize)))
            img[va:va + min(rsize, vsize or rsize)] = d[roff:roff + min(rsize, vsize or rsize)]
        imp = struct.unpack_from("<I", d, o + 104)[0]  # DataDirectory[1]: imports
        n = 0
        while imp and struct.unpack_from("<I", img, imp + 12)[0]:
            ilt, _, _, name, iat = struct.unpack_from("<IIIII", img, imp)
            dll = img[name:img.index(0, name)].decode()
            ilt = ilt or iat
            for k in range(0, 1 << 20, 4):
                e = struct.unpack_from("<I", img, ilt + k)[0]
                if not e:
                    break
                fn = f"#{e & 0xffff}" if e >> 31 else img[e + 2:img.index(0, e + 2)].decode()
                addr = STUBS + 8 * n
                n += 1
                self.imports[addr] = f"{dll}!{fn}"
                struct.pack_into("<I", img, iat + k, addr)
                self.iat[base + iat + k] = addr
            imp += 20
        self._map(base, img)
        self.range = (base, base + len(img))
        self.image = (base, bytes(img))

    def _elf(self, d):
        phoff, phes, phn = struct.unpack_from("<I", d, 0x1c)[0], *struct.unpack_from("<HH", d, 0x2a)
        lo, hi = 1 << 32, 0
        for k in range(phn):
            t, off, va, _, fs, ms = struct.unpack_from("<IIIIII", d, phoff + phes * k)
            if t == 1:
                self._map(va, d[off:off + fs], ms)
                lo, hi = min(lo, va), max(hi, va + ms)
        # shortcut: the SCE relocations and import stubs are not read: right at the ELF's own
        # addresses, and an import's stub runs as the file has it; read them if a call needs it
        self.range = (lo, hi)

    def _nso(self, d):
        flags = struct.unpack_from("<I", d, 0xc)[0]
        img = bytearray()
        for k, (hdr, csz) in enumerate([(0x10, 0x60), (0x20, 0x64), (0x30, 0x68)]):
            foff, moff, size = struct.unpack_from("<III", d, hdr)
            raw = d[foff:foff + struct.unpack_from("<I", d, csz)[0]]
            data = lz4_block(raw, size) if flags >> k & 1 else raw[:size]
            img += bytes(moff + size - len(img))
            img[moff:moff + size] = data
        mod0 = struct.unpack_from("<I", img, 4)[0]
        dyn = mod0 + struct.unpack_from("<i", img, mod0 + 4)[0]
        bss_end = mod0 + struct.unpack_from("<i", img, mod0 + 12)[0]
        img += bytes(max(0, up(bss_end) - len(img)))
        tags = {}
        while (t := struct.unpack_from("<q", img, dyn)[0]) != 0:
            tags[t] = struct.unpack_from("<Q", img, dyn + 8)[0]
            dyn += 16
        sym, strt = tags.get(6), tags.get(5)
        for at, size in ((tags.get(7), tags.get(8)), (tags.get(23), tags.get(2))):  # RELA, JMPREL
            for r in range(at or 0, (at or 0) + (size or 0), 24):
                off, info, add = struct.unpack_from("<QQq", img, r)
                typ, s = info & 0xffffffff, info >> 32
                if typ == 1027:  # RELATIVE
                    struct.pack_into("<Q", img, off, NSO_BASE + add)
                elif typ in (257, 1025, 1026):  # ABS64, GLOB_DAT, JUMP_SLOT
                    name_at, _, _, _, value = struct.unpack_from("<IBBHQ", img, sym + 24 * s)
                    if value:
                        struct.pack_into("<Q", img, off, NSO_BASE + value + add)
                    else:
                        addr = STUBS + 8 * len(self.imports)
                        self.imports[addr] = img[strt + name_at:img.index(0, strt + name_at)].decode()
                        struct.pack_into("<Q", img, off, addr)
        self._map(NSO_BASE, img)
        self.range = (NSO_BASE, NSO_BASE + len(img))

    def load_dump(self, prefix):
        """Map a process memory dump over the image: <prefix>.map (a line per
        region: base size protect type offset, hex; offset `-`: left out) and
        <prefix>.bin (ntrace.c NTRACE_DUMPS); a scene dump (NTRACE_SCENE_DUMPS)
        names its base in its first line and holds only the pages that differ. The exe's code and import slots
        are put back as the file has them (a trace DLL's patches, the real
        DLLs' addresses), so imports stay stubs. Returns the regions loaded."""
        n = 0
        if os.path.exists(prefix + ".bin.zst"):  # packed (research/tools/dumppack.py; Python 3.14)
            from dumppack import unpack
            b = io.BytesIO(unpack(os.path.dirname(prefix) or ".", os.path.basename(prefix)))
        else:
            b = open(prefix + ".bin", "rb")
        with b:
            for line in open(prefix + ".map"):
                f = line.split()
                if f[0] == "base":  # a scene dump: the base first, then its changed pages ("P addr size offset")
                    n += self.load_dump(os.path.join(os.path.dirname(prefix), f[1]))
                    continue
                if f[0] == "zstd-from":  # how dumppack packed it
                    continue
                if f[0] == "P":
                    b.seek(int(f[3], 16))
                    self._map(int(f[1], 16), b.read(int(f[2], 16)))
                    continue
                if f[4] in ("-", "d"):  # left out; a scene dump's region (its pages: P lines)
                    continue
                b.seek(int(f[4], 16))
                self._map(int(f[0], 16), b.read(int(f[1], 16)))
                n += 1
        if self.arch == "x86":
            base, img = self.image
            for va, size in self.code:
                self.uc.mem_write(va, img[va - base:va - base + size])
            for slot, stub in self.iat.items():
                self.uc.mem_write(slot, struct.pack("<I", stub))
        return n

    def stub_import(self, name, value):
        """Stub the import named `dll!fn` (case-insensitive) with value, or a
        function called at each hit (a clock)."""
        for a, n in self.imports.items():
            if n.lower() == name.lower():
                self.stubs[a] = value
                return a
        raise KeyError(name)

    def read(self, addr, n):
        return bytes(self.uc.mem_read(addr, n))

    def ptr(self, addr):
        return struct.unpack("<Q" if self.arch == "arm64" else "<I", self.read(addr, 8 if self.arch == "arm64" else 4))[0]

    def call(self, addr, args=(), cc="cdecl", ret="int", limit=10_000_000, regs=None):
        """Call addr with args (int, float, ('d', x) a double, ('&', fmt, x) a pointer to one,
        ('buf', n) a buffer); regs: registers set first by name ({"edi": 1}, a value the
        code takes unset); return (value, [buffer addresses])."""
        uc, heap, bufs = self.uc, HEAP, []
        for r, v in (regs or {}).items():
            uc.reg_write(getattr({"x86": X, "arm": A, "arm64": A64}[self.arch], f"UC_{self.arch.upper()}_REG_{r.upper()}"), v)
        vals = []
        for a in args:  # pointers first, into the heap
            if isinstance(a, tuple) and a[0] in ("&", "buf"):
                data = bytes(a[1]) if a[0] == "buf" else struct.pack("<" + a[1], a[2])
                uc.mem_write(heap, data)
                if a[0] == "buf":
                    bufs.append((heap, len(data)))
                vals.append(heap)
                heap += up(len(data) + 1)
            else:
                vals.append(a)
        if self.arch == "x86":
            regs = {"thiscall": [X.UC_X86_REG_ECX], "fastcall": [X.UC_X86_REG_ECX, X.UC_X86_REG_EDX]}.get(cc, [])
            stack = b""
            for v in vals:
                if isinstance(v, int) and regs:
                    uc.reg_write(regs.pop(0), v & 0xffffffff)
                elif isinstance(v, float):
                    stack += struct.pack("<f", v)
                elif isinstance(v, tuple):
                    stack += struct.pack("<d", v[1])
                else:
                    stack += struct.pack("<I", v & 0xffffffff)
            sp = STACK + STACK_SIZE - 0x1000 - len(stack)
            uc.mem_write(sp - 4, struct.pack("<I", RET) + stack)
            uc.reg_write(X.UC_X86_REG_ESP, sp - 4)
            uc.reg_write(X.UC_X86_REG_EBP, 0)
            end = RET + 6
        else:
            a64 = self.arch == "arm64"
            r = [getattr(A64, f"UC_ARM64_REG_X{i}") for i in range(8)] if a64 else [getattr(A, f"UC_ARM_REG_R{i}") for i in range(4)]
            nf, stack = 0, b""
            for v in vals:
                if isinstance(v, (float, tuple)):
                    x = v[1] if isinstance(v, tuple) else v
                    if a64:
                        reg = getattr(A64, f"UC_ARM64_REG_Q{nf}")
                        uc.reg_write(reg, int.from_bytes(struct.pack("<d" if isinstance(v, tuple) else "<f", x), "little"))
                        nf += 1
                    elif isinstance(v, tuple):  # shortcut: no back-filling of odd s registers after a double
                        nf += nf & 1
                        uc.reg_write(getattr(A, f"UC_ARM_REG_D{nf // 2}"), struct.unpack("<Q", struct.pack("<d", x))[0])
                        nf += 2
                    else:
                        reg = getattr(A, f"UC_ARM_REG_D{nf // 2}")
                        old = struct.pack("<Q", uc.reg_read(reg))
                        new = old[:4] + struct.pack("<f", x) if nf & 1 else struct.pack("<f", x) + old[4:]
                        uc.reg_write(reg, struct.unpack("<Q", new)[0])
                        nf += 1
                elif r:
                    uc.reg_write(r.pop(0), v & (2 ** 64 - 1 if a64 else 0xffffffff))
                else:
                    stack += struct.pack("<Q" if a64 else "<I", v & (2 ** 64 - 1 if a64 else 0xffffffff))
            sp = (STACK + STACK_SIZE - 0x1000 - len(stack)) & ~15
            uc.mem_write(sp, stack)
            uc.reg_write(A64.UC_ARM64_REG_SP if a64 else A.UC_ARM_REG_SP, sp)
            uc.reg_write(A64.UC_ARM64_REG_LR if a64 else A.UC_ARM_REG_LR, RET)
            end = RET
            if not a64 and self.thumb:
                addr |= 1
        self._hooks()
        pc, steps = addr, 0
        while True:
            try:
                uc.emu_start(pc, end, count=limit)
            except UcError as e:
                pc = self._pc()
                if pc in self.imports or pc not in range(*self.range):
                    pc = self._stub(pc, self.stubs.get(pc, 0), f"{e}")
                    continue
                raise SystemExit(f"stopped at {pc:#x}: {e}")
            if self._pc() == self._stub_pc:  # an inner --stub hit: return from it and go on
                pc = self._stub(self._pc(), self.stubs[self._pc()], "stubbed")
                continue
            break
        if self._pc() != end:
            raise SystemExit(f"no return after {limit} instructions (at {self._pc():#x})")
        return self._ret(ret), bufs

    def _hooks(self):
        self._stub_pc = None
        if getattr(self, "_hooked", None) == set(self.stubs):
            return
        self._hooked = set(self.stubs)
        done = self.__dict__.setdefault("_hook_at", set())  # a hook stays once added: one per address
        for a in self.stubs:
            if a in range(*self.range) and a not in done:
                done.add(a)
                at = a & ~1 if self.arch == "arm" else a  # (a Thumb address's low bit; x86 code sits at odd ones)
                self.uc.hook_add(UC_HOOK_CODE, self._inner, begin=at, end=at)
                self.uc.ctl_remove_cache(at, at + 1)  # a block translated before has no hook

    def _inner(self, uc, address, size, data):
        # return from the stubbed function right here (an emu_stop takes effect only at the block's end)
        key = address | 1 if self.arch == "arm" and address | 1 in self.stubs else address
        if key not in self.stubs:  # a stub taken away since
            return
        back = self._stub(address, self.stubs[key], "stubbed")
        uc.reg_write({"x86": X.UC_X86_REG_EIP, "arm": A.UC_ARM_REG_PC, "arm64": A64.UC_ARM64_REG_PC}[self.arch], back)

    def _pc(self):
        return self.uc.reg_read({"x86": X.UC_X86_REG_EIP, "arm": A.UC_ARM_REG_PC, "arm64": A64.UC_ARM64_REG_PC}[self.arch])

    def _stub(self, pc, value, why):
        """Return value from a call that reached pc; the address to go on from."""
        uc = self.uc
        self.hits.append((pc, self.imports.get(pc, ""), why))
        if self.arch == "x86":
            sp = uc.reg_read(X.UC_X86_REG_ESP)
            if callable(value):
                value = value(self, sp + 4)
            # a Win32 import is stdcall: it pops its arguments (STDCALL); others are taken as cdecl
            fn = self.imports.get(pc, "").split("!")[-1]
            if fn and fn not in STDCALL:
                if not hasattr(Exe, "_counts"):
                    Exe._counts = stdcall_counts()
                STDCALL[fn] = Exe._counts.get(fn, 0)  # 0: a cdecl import (msvcrt), or one no import library knows
            pop = 4 * STDCALL.get(fn, 0)
            uc.reg_write(X.UC_X86_REG_ESP, sp + 4 + pop)
            uc.reg_write(X.UC_X86_REG_EAX, value & 0xffffffff)
            return struct.unpack("<I", self.read(sp, 4))[0]
        if callable(value):
            value = value(self, None)
        if self.arch == "arm":
            uc.reg_write(A.UC_ARM_REG_R0, value & 0xffffffff)
            return uc.reg_read(A.UC_ARM_REG_LR)
        uc.reg_write(A64.UC_ARM64_REG_X0, value & (2 ** 64 - 1))
        return uc.reg_read(A64.UC_ARM64_REG_LR)

    def _ret(self, kind):
        uc = self.uc
        if self.arch == "x86":
            if kind == "int":
                return uc.reg_read(X.UC_X86_REG_EAX)
            v = struct.unpack("<d", self.read(RET + 0x100, 8))[0]
            return struct.unpack("<f", struct.pack("<f", v))[0] if kind == "float" else v
        if self.arch == "arm":
            if kind == "int":
                return uc.reg_read(A.UC_ARM_REG_R0)
            d0 = struct.pack("<Q", uc.reg_read(A.UC_ARM_REG_D0))
            return struct.unpack("<f", d0[:4])[0] if kind == "float" else struct.unpack("<d", d0)[0]
        if kind == "int":
            return uc.reg_read(A64.UC_ARM64_REG_X0)
        q0 = uc.reg_read(A64.UC_ARM64_REG_Q0).to_bytes(16, "little")
        return struct.unpack("<f", q0[:4])[0] if kind == "float" else struct.unpack("<d", q0[:8])[0]


def arg(s):
    if s.startswith("buf:"):
        return ("buf", bytes(int(s[4:], 0)))
    if s.startswith("&"):
        v = arg(s[1:])
        return ("&", "d", v[1]) if isinstance(v, tuple) else ("&", "f" if isinstance(v, float) else "i", v)
    if s.startswith("d:"):
        return ("d", float(s[2:]))
    if s.startswith("i:"):
        return int(s[2:], 0)
    try:
        return int(s, 0)
    except ValueError:
        return float(s)


def num(s):
    return int(eval(s, {"__builtins__": {}}))  # an address expression: 0x5e40a0+4*17


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("exe")
    p.add_argument("addr")
    p.add_argument("args", nargs="*")
    p.add_argument("--cc", default="cdecl", choices=["cdecl", "stdcall", "thiscall", "fastcall"])
    p.add_argument("--ret", default="int", choices=["int", "float", "double"])
    p.add_argument("--stub", action="append", default=[], metavar="ADDR=VALUE")
    p.add_argument("--dump", action="append", default=[], metavar="ADDR:LEN")
    p.add_argument("--dump-in", metavar="PREFIX", help="map PREFIX.map/.bin, a process dump, first")
    p.add_argument("--arm", action="store_true", help="ARM code in ARM state, not Thumb")
    a = p.parse_args()
    exe = Exe(a.exe, thumb=not a.arm)
    if a.dump_in:
        exe.load_dump(a.dump_in)
    for s in a.stub:
        k, v = s.split("=")
        exe.stubs[num(k)] = num(v)
    addr = exe.ptr(num(a.addr[1:])) if a.addr.startswith("*") else num(a.addr)
    value, bufs = exe.call(addr, [arg(s) for s in a.args], a.cc, a.ret)
    print(f"{addr:#x} -> {value if a.ret != 'int' else f'{value} ({value:#x})'}")
    for pc, name, why in exe.hits:
        print(f"stub hit {pc:#x} {name} ({why})")
    for at, n in bufs:
        print(f"arg buffer {at:#x}: {exe.read(at, n).hex()}")
    for s in a.dump:
        at, n = s.split(":")
        print(f"{num(at):#x}: {exe.read(num(at), num(n)).hex()}")


if __name__ == "__main__":
    main()
