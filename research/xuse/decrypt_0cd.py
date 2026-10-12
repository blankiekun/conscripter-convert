"""Decrypt HD/CDFiles/0.cd (Xuse, 最果てのイマ COMPLETE).

Cipher from Farthest2015.exe at 0x4bcb20: the keystream starts at 0x1b and
after each byte becomes ((k ^ 0xa1) >> 1) + (index & 0xff). (A first attempt
recovered a 256-byte key from byte frequencies; it differed from this in only
7 of 88,470 bytes.) The Rust version is `vn2ons cd symbols`.

usage: python3 -I decrypt_0cd.py <path/to/0.cd> <out.bin>
"""
import sys

data = bytearray(open(sys.argv[1], 'rb').read())
k = 0x1b
for i in range(len(data)):
    data[i] ^= k
    k = (((k ^ 0xa1) >> 1) + (i & 0xff)) & 0xff
open(sys.argv[2], 'wb').write(data)
