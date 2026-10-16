"""月姫 route map from the scripts: each script's choices, flag tests and jumps.

Usage: python3 -I routes.py <dump> <names.bin> <main.elf> [--all]
  <dump>       `conscripter-convert typemoon script <game-102> -o <dump>`
  <names.bin>  entry 0 of allscr.mrg (`typemoon extract <game-102>/allscr.mrg`)
  <main.elf>   1.0.2's main through research/tools/ghidra/nso2elf.py

_JUMP(n[,label]) goes to the script whose scene number is n: a table of
551 shorts at 0x450D58 in the exe, one per script in allscr order (the
_JUMP case of script_command). The label is a place inside that script;
none means its START. Prints only scripts with a choice or a flag test
unless --all; options show the conditions they are under (an `_IF__` runs
the lines up to its label only when its test holds) and `(greyed)` for a
shown option that cannot be picked (`_SELR`'s third argument 1).
"""
import re
import struct
import sys

# Flags that are not progression: the per-scene housekeeping.
HOUSEKEEPING = re.compile(r'(F02[2-8]|[FC]8\d\d|F1\d{3})$')


def scene_table(elf):
    d = open(elf, 'rb').read()
    phoff, = struct.unpack_from('<Q', d, 0x20)
    va = 0x450D58 - 0x100000  # Ghidra loads at 0x100000; the ELF at 0
    for i in range(struct.unpack_from('<H', d, 0x38)[0]):
        t, _, off, v, _, fs, _, _ = struct.unpack_from('<IIQQQQQQ', d, phoff + i * 56)
        if t == 1 and v <= va < v + fs:
            return struct.unpack_from('<551h', d, off + va - v)
    sys.exit('table not found')


def main():
    dump, names_bin, elf = sys.argv[1:4]
    raw = open(names_bin, 'rb').read()
    names = [raw[i:i + 32].split(b'\0')[0].decode() for i in range(0, len(raw), 32)]
    ids = scene_table(elf)
    by_id = {ids[i]: n for i, n in enumerate(names) if n}
    for n in names:
        if not n:
            continue
        lines = open(f'{dump}/{n}.txt', encoding='utf-8').read().splitlines()
        if n.startswith(('JUMP', 'TEST_', 'ARCHIVE')) and '--all' not in sys.argv:
            continue  # chapter select and debug: jumps to every scene
        out, sels, conds = [], [], []
        for c in lines:
            if m := re.match(r'_ZY[0-9a-f]{5}\((\w+)\)', c):
                conds = [x for x in conds if x[1] != m.group(1)]
            if m := re.match(r'_SELR\((\d+),\$\d+(,1)?\)\s*« (.*) »', c):
                pre = f'[{" and ".join(x[0] for x in conds)}] ' if conds else ''
                sels.append((int(m.group(1)), pre + m.group(3) + (' (greyed)' if m.group(2) else '')))
            elif m := re.match(r'_GOTS\(_D000,(.*)\)', c):
                # Option i goes to label i; an option shown under different conditions has several variants.
                out.append('choice: ' + ' / '.join(' | '.join(t for k, t in sels if k == i) + f' [{l}]' for i, l in enumerate(m.group(1).split(','))))
                sels = []
            elif m := re.match(r'_Z[ZY][0-9a-f]{5}\((\w+)\)', c):
                if not m.group(1).startswith(('LV', 'START', 'LBL_0')):
                    out.append(f'  {m.group(1)}:')
            elif m := re.match(r'_JUMP\((\d+),?(\w*)\)', c):
                t = by_id.get(int(m.group(1)), f'?{m.group(1)}')
                out.append(f'    -> {t}' + (f' {m.group(2)}' if m.group(2) not in ('', 'START') and not m.group(2).startswith('LBL_0') else ''))
            elif m := re.match(r'_GOTO\((\w+)\)', c):
                out.append(f'    goto {m.group(1)}')
            elif m := re.match(r'_IF__\((\w+),(\w+),(-?\w+),', c):
                if not HOUSEKEEPING.match(m.group(1)):
                    out.append(f'  if {m.group(1)} {m.group(2)} {m.group(3)}:')
                    lv = re.search(r',(LV\w+)\)', c)
                    conds.append((f'{m.group(1)} {m.group(2)} {m.group(3)}', lv.group(1) if lv else ''))
            elif m := re.match(r'_WK(ST|AD|OR)\((\w+),(?:eq,)?(-?\w+)\)', c):
                if not HOUSEKEEPING.match(m.group(2)) and m.group(2) != 'G008':
                    out.append(f'    {m.group(2)} {"=" if m.group(1) == "ST" else "+=" if m.group(1) == "AD" else "|="} {m.group(3)}')
        if '--all' in sys.argv or any(o.startswith(('choice', '  if')) for o in out):
            print(f'{n} ({ids[names.index(n)]})')
            # Collapse runs of jumps/sets that repeat.
            prev = None
            for o in out:
                if o != prev:
                    print(o)
                prev = o


main()
