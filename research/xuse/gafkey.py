"""GAF4 key generator transcribed from Farthest2015.exe (0x40b2e0), plus the
XOR used by the loader (0x4af460): byte ^= key[offset % (len(key) - 1)],
with the key built from the header name field ("Farthest" / "generic").
The Rust version is crates/vn2ons-engines/src/xuse/gaf.rs.
"""
def sx(b):  # sign-extend byte to 16-bit
    return b - 256 if b >= 128 else b

def gaf_key(s: bytes) -> bytes:
    n = len(s)
    h = 0
    for i, c in enumerate(s):
        h = ((((sx(c) + i) & 0xffff) ^ h) + n) & 0xffff
    length = (h & 0xff) + 0x40
    h2 = h
    for c in s:
        h2 = (h2 + sx(c)) & 0xffff
    key = bytearray(length)
    bl = h2 & 0x0f
    key[0] = bl
    key[1] = (h2 >> 8) & 0xff
    key[2] = 0x46
    key[3] = 0x88
    for c in range(4, length - 1):
        dl = ((s[c % n] ^ bl) + c) & 0xff
        bl = (bl + dl) & 0xff
        key[c] = bl
    key[length - 1] = 0
    return bytes(key)

def decrypt(data: bytes, key: bytes, start_pos: int, base: int) -> bytes:
    L = len(key) - 1
    return bytes(x ^ key[(start_pos + i - base) % L] for i, x in enumerate(data))
