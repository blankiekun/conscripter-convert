"""Decrypt HD/CDFiles/0.cd (Xuse, 最果てのイマ COMPLETE).

0.cd is XORed with a repeating 256-byte key. The key is not stored here: it is
recovered from the file itself, assuming the most common plaintext byte in
each key column is 0 (each column's top byte covers 31-48% of its samples).
There is no checksum to confirm it, so a few columns may still be wrong.

usage: python3 -I decrypt_0cd.py <path/to/0.cd> <out.bin>
"""
import collections
import sys

data = open(sys.argv[1], 'rb').read()
key = bytes(collections.Counter(data[i::256]).most_common(1)[0][0] for i in range(256))
open(sys.argv[2], 'wb').write(bytes(b ^ key[i % 256] for i, b in enumerate(data)))
