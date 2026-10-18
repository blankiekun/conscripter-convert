"""Cuts the GXP shader programs out of the decrypted eboot (research/tools/ghidra/self2elf.py):
every blob starting with "GXP\\0", named by its address, its size from
the header. The InOut_Act types' fragment shaders are at 0x8115f63c (1),
0x8115fb80 (2), 0x8115fe60 (3), 0x81160510 (6).

    python3 -I gxp.py local/eboot.elf local/gxp/
"""
import os
import re
import struct
import sys

elf = open(sys.argv[1], 'rb').read()
out = sys.argv[2]
os.makedirs(out, exist_ok=True)
phoff, = struct.unpack_from('<I', elf, 0x1c)
segs = []
for i in range(struct.unpack_from('<H', elf, 0x2c)[0]):
    _, off, vaddr, _, filesz, _, _, _ = struct.unpack_from('<8I', elf, phoff + 32 * i)
    if vaddr:
        segs.append((vaddr, off, filesz))
for m in re.finditer(b'GXP\x00', elf):
    o = m.start()
    size, = struct.unpack_from('<I', elf, o + 8)
    addr = next((v + o - off for v, off, n in segs if off <= o < off + n), None)
    if addr is None or not 0 < size < 0x10000:
        continue
    with open(os.path.join(out, f'{addr:08x}.gxp'), 'wb') as f:
        f.write(elf[o:o + size])
