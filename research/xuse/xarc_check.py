import struct, sys, binascii
def crc16(data, poly, init, refin, xorout=0):
    crc = init
    for b in data:
        if refin:
            crc ^= b
            for _ in range(8):
                crc = (crc >> 1) ^ poly if crc & 1 else crc >> 1
        else:
            crc ^= b << 8
            for _ in range(8):
                crc = ((crc << 1) ^ poly) & 0xffff if crc & 0x8000 else (crc << 1) & 0xffff
    return crc ^ xorout
cands = {
 'sum16': lambda d: sum(d) & 0xffff,
 'crc16_arc': lambda d: crc16(d, 0xA001, 0, True),
 'crc16_modbus': lambda d: crc16(d, 0xA001, 0xffff, True),
 'crc_ccitt_ffff': lambda d: crc16(d, 0x1021, 0xffff, False),
 'crc_ccitt_0': lambda d: crc16(d, 0x1021, 0, False),
 'kermit': lambda d: crc16(d, 0x8408, 0, True),
 'x25': lambda d: crc16(d, 0x8408, 0xffff, True, 0xffff),
 'crc16_buypass': lambda d: crc16(d, 0x8005, 0, False),
 'xor16': lambda d: (lambda x:[x])(0) and 0,
}
for path in sys.argv[1:]:
    b = open(path,'rb').read()
    n, = struct.unpack_from('<I', b, 4)
    offs = struct.unpack_from(f'<{n}I', b, 8)
    hend = 8 + 4*n
    regions = [('header', b[0:hend], b[hend:hend+2])]
    for i,o in enumerate(offs[:3]):
        nl, ds = struct.unpack_from('<II', b, o+24)
        ne = o + 32 + nl
        name = bytes(((c<<4)|(c>>4))&0xff for c in b[o+32:ne])
        regions.append((f'ent{i} hdr {name}', b[o:ne], b[ne:ne+2]))
        regions.append((f'ent{i} hdr-from-name', b[o+32:ne], b[ne:ne+2]))
        regions.append((f'ent{i} data', b[ne+2:ne+2+ds], b[ne+2+ds:ne+4+ds]))
    for label, data, chk in regions:
        want_le, = struct.unpack('<H', chk); want_be, = struct.unpack('>H', chk)
        hits = [k for k,f in cands.items() if f(data) in (want_le, want_be)]
        print(path.split('/')[-1], label, chk.hex(), hits)
