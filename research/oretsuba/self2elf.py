"""Decrypts a PS Vita NPDRM SELF (the game's eboot.bin) to an ELF, after
Vita3K's decrypt_fself (vita3k/packages/src/sce_utils.cpp). The keys are
read from that file in a Vita3K checkout, not kept here.

    python3 -I self2elf.py <eboot.bin> <work.bin> <Vita3K>/vita3k/packages/src/sce_utils.cpp <out.elf>

work.bin is the game's license; its klicensee is at 0x50.

The SELF: SCE header (platform u8 at 8, key revision u8 at 9, type u16,
metadata offset u32 at 12, header length u64 at 16), then the SELF header
(app info, ELF header, program headers, segment infos at 0x20 + 24/32/40/
56). The metadata after metadata offset + 48 is two-layer encrypted:
64 bytes of key/iv under AES-128-CBC with the klicensee (itself under the
NPDRM key) and AES-256-CBC with the metadata key, then the section table
and key vault under that key/iv. Each segment is AES-128-CTR and usually
deflated.
"""
import re
import struct
import sys
import zlib

from Crypto.Cipher import AES

eboot, work, keyfile, out = sys.argv[1:5]
data = open(eboot, 'rb').read()
klic = open(work, 'rb').read()[0x50:0x60]

# register_keys(KeyType, SceType, keyrev, key, iv, minver, maxver, SelfType)
src = open(keyfile).read()
keys = {}
for m in re.finditer(r'register_keys\(\s*KeyType::(\w+),\s*SceType::(\w+),\s*(\d+),\s*"(\w+)",\s*"(\w+)",[^;]*?SelfType::(\w+)\)', src):
    keys.setdefault((m[1], m[2], int(m[3]), m[6]), (bytes.fromhex(m[4]), bytes.fromhex(m[5])))

assert data[:4] == b'SCE\0'
key_rev = data[9]
meta_off, = struct.unpack_from('<I', data, 12)
header_len, = struct.unpack_from('<Q', data, 16)
appinfo, elf_off, phdr_off, _, seginfo = struct.unpack_from('<5Q', data, 0x20 + 24)
self_type, = struct.unpack_from('<I', data, appinfo + 12)
assert self_type == 8, 'not an application SELF'

np_key, np_iv = keys[('NPDRM', 'SELF', 1 if key_rev >= 2 else 0, 'APP')]
meta_key, meta_iv = keys[('METADATA', 'SELF', key_rev, 'APP')]
klic_dec = AES.new(np_key, AES.MODE_CBC, np_iv).decrypt(klic)
meta = data[meta_off + 48:header_len]
info = AES.new(klic_dec, AES.MODE_CBC, np_iv).decrypt(meta[:64])
info = AES.new(meta_key, AES.MODE_CBC, meta_iv).decrypt(info)
rest = AES.new(info[:16], AES.MODE_CBC, info[32:48]).decrypt(meta[64:len(meta) - (len(meta) - 64) % 16])
_, _, sections, key_count, _, _, _ = struct.unpack_from('<QIIIIII', rest, 0)
vault_at = 32 + sections * 48
vault = [rest[vault_at + 16 * i:vault_at + 16 * (i + 1)] for i in range(key_count)]
segkeys = {}
for i in range(sections):
    off, size, typ, seg, _, _, enc, ki, ii, comp = struct.unpack_from('<QQIIIIIIII', rest, 32 + i * 48)
    if enc == 3:  # AES128CTR
        segkeys[seg] = (vault[ki], vault[ii])

e_phnum, = struct.unpack_from('<H', data, elf_off + 0x2c)
elf = bytearray(data[elf_off:elf_off + 0x34])
phdrs = []
for i in range(e_phnum):
    ph = data[phdr_off + 32 * i:phdr_off + 32 * (i + 1)]
    phdrs.append(struct.unpack('<8I', ph))
    elf += ph
for i in range(e_phnum):
    p_offset, p_filesz = phdrs[i][1], phdrs[i][4]
    if p_filesz == 0:
        continue
    off, size, compressed, _, plaintext, _ = struct.unpack_from('<QQIIII', data, seginfo + 32 * i)
    seg = data[off:off + size]
    if plaintext == 1:  # SecureBool NO
        k, iv = segkeys[i]
        seg = AES.new(k, AES.MODE_CTR, nonce=b'', initial_value=iv).decrypt(seg)
    if compressed == 2:
        seg = zlib.decompress(seg)
    elf += bytes(max(0, p_offset - len(elf)))
    elf[p_offset:p_offset + len(seg)] = seg
    print(f'segment {i}: offset {p_offset:#x}, {len(seg)} bytes')
open(out, 'wb').write(elf)
