"""Thumb-2 listing of the decrypted eboot (self2elf.py) with the movw/movt
constants resolved and the strings they point at, for finding the code
that reads a given string. Needs capstone.

    python3 disasm.py <eboot.elf> > eboot.asm

Linear sweep over the code segment (loaded at 0x81000000); data inside
it comes out as nonsense instructions, which is harmless for grepping.
Lines look like `81012345: movt r0, #0x8114  ; =0x81144fc4 "CGMODE/FlagMax"`.
"""
import re
import struct
import sys

from capstone import CS_ARCH_ARM, CS_MODE_THUMB, Cs

elf = open(sys.argv[1], 'rb').read()
phoff, = struct.unpack_from('<I', elf, 0x1c)
segs = []
for i in range(struct.unpack_from('<H', elf, 0x2c)[0]):
    _, off, vaddr, _, filesz, _, _, _ = struct.unpack_from('<8I', elf, phoff + 32 * i)
    if vaddr:
        segs.append((vaddr, off, filesz))


def read_str(addr):
    for vaddr, off, size in segs:
        if vaddr <= addr < vaddr + size:
            p = off + addr - vaddr
            end = elf.find(b'\0', p, p + 200)
            if end <= p:
                return None
            s = elf[p:end]
            try:
                t = s.decode('cp932')
            except UnicodeDecodeError:
                return None
            return t if t.isprintable() and len(t) >= 2 else None
    return None


vaddr, off, size = segs[0]
code = elf[off:off + size]
md = Cs(CS_ARCH_ARM, CS_MODE_THUMB)
md.skipdata = True
low = {}
out = sys.stdout
for ins in md.disasm(code, vaddr):
    note = ''
    ops = ins.op_str
    if ins.mnemonic.startswith('movw'):
        m = re.match(r'(\w+), #(0x[0-9a-f]+|\d+)', ops)
        if m:
            low[m[1]] = int(m[2], 0)
    elif ins.mnemonic.startswith('movt'):
        m = re.match(r'(\w+), #(0x[0-9a-f]+|\d+)', ops)
        if m and m[1] in low:
            value = (int(m[2], 0) << 16) | low.pop(m[1])
            s = read_str(value)
            note = f'  ; ={value:#x}' + (f' "{s}"' if s else '')
    out.write(f'{ins.address:08x}: {ins.mnemonic} {ops}{note}\n')
