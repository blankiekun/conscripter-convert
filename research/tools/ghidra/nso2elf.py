#!/usr/bin/env python3
"""Turn a Switch NSO (ExeFS `main`, `sdk`, ...) into an ELF Ghidra loads.

    python3 -I nso2elf.py <main> <main.elf>

Decompresses the three segments (LZ4 blocks), lays them out at their
memory offsets from address 0, and writes an ELF64 AArch64 ET_DYN with
one PT_LOAD per segment (bss included) and a PT_DYNAMIC from the
module's MOD0 header, so Ghidra's ELF loader reads the dynamic symbols
and applies the relocations (the data's pointers are only in the RELA
entries until then). No section headers: Ghidra works from the program
headers. Formats: switchbrew "NSO" and "NSO#MOD".
"""
import struct
import sys


def lz4_block(src, size):
    out = bytearray()
    i = 0
    while i < len(src):
        token = src[i]
        i += 1
        n = token >> 4
        if n == 15:
            while True:
                b = src[i]
                i += 1
                n += b
                if b != 255:
                    break
        out += src[i:i + n]
        i += n
        if i >= len(src):
            break
        back = src[i] | src[i + 1] << 8
        i += 2
        m = token & 15
        if m == 15:
            while True:
                b = src[i]
                i += 1
                m += b
                if b != 255:
                    break
        m += 4
        start = len(out) - back
        for k in range(m):  # may overlap itself
            out.append(out[start + k])
    if len(out) != size:
        raise ValueError(f"LZ4 gave {len(out)} bytes, expected {size}")
    return bytes(out)


def main(src, dst):
    d = open(src, "rb").read()
    if d[:4] != b"NSO0":
        sys.exit("not an NSO")
    flags = struct.unpack_from("<I", d, 0xC)[0]
    segs = []
    for k, (hdr, csize_at) in enumerate([(0x10, 0x60), (0x20, 0x64), (0x30, 0x68)]):
        foff, moff, size = struct.unpack_from("<III", d, hdr)
        csize = struct.unpack_from("<I", d, csize_at)[0]
        raw = d[foff:foff + csize]
        segs.append((moff, lz4_block(raw, size) if flags >> k & 1 else raw[:size]))
    bss = struct.unpack_from("<I", d, 0x3C)[0]
    image_end = segs[2][0] + len(segs[2][1])
    image = bytearray(image_end)
    for moff, data in segs:
        image[moff:moff + len(data)] = data
    mod0 = struct.unpack_from("<I", image, 4)[0]
    if image[mod0:mod0 + 4] != b"MOD0":
        sys.exit("no MOD0")
    dynamic = mod0 + struct.unpack_from("<i", image, mod0 + 4)[0]
    bss_end = mod0 + struct.unpack_from("<i", image, mod0 + 12)[0]
    mem_end = max(image_end + bss, bss_end)
    dyn_size = 0
    while struct.unpack_from("<q", image, dynamic + dyn_size)[0] != 0:  # DT_NULL ends it
        dyn_size += 16
    dyn_size += 16

    phnum = 4
    data_at = 0x1000  # file offset of the image, page-aligned for the loads
    ehdr = b"\x7fELF" + bytes([2, 1, 1, 0]) + bytes(8)
    ehdr += struct.pack("<HHIQQQIHHHHHH", 3, 183, 1, 0, 64, 0, 0, 64, 56, phnum, 64, 0, 0)
    ph = b""
    for (moff, data), flags_ in zip(segs, (5, 4, 6)):  # R-X, R--, RW-
        memsz = mem_end - moff if flags_ == 6 else len(data)
        ph += struct.pack("<IIQQQQQQ", 1, flags_, data_at + moff, moff, moff, len(data), memsz, 0x1000)
    ph += struct.pack("<IIQQQQQQ", 2, 6, data_at + dynamic, dynamic, dynamic, dyn_size, dyn_size, 8)
    out = ehdr + ph
    out += bytes(data_at - len(out)) + image
    open(dst, "wb").write(out)
    print(f"{dst}: text {len(segs[0][1]):#x} at {segs[0][0]:#x}, ro at {segs[1][0]:#x}, "
          f"data at {segs[2][0]:#x}, bss to {mem_end:#x}, dynamic at {dynamic:#x} ({dyn_size // 16} entries)")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])
