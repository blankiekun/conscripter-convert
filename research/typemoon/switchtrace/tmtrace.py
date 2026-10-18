#!/usr/bin/env python3
"""The hook's log lines (Ryubing's log, "TMT <script> <offset> <command>")
to trace_diff.py's JSONL of the original.

    python3 -I tmtrace.py <ryu.log> <scr dir> <txt dir> -o orig.jsonl [--pages N]

<scr dir>: `typemoon extract allscr.mrg -o <scr dir>` (entry 0 the names,
3 + i script i as text); <txt dir>: the same for script_text.mrg (entries
0 and 1, the first language's offsets and lines). "at" is
"<script>:<command>", the command counted as the converter counts it
(`script::parse`, the malformed one left out); "ms" is the log's clock.
A command the exe runs again while it waits (a fade, `_TOMW`: the hook
sees it each frame) is one record. A text (`_ZM`, `_MSAD`) is a page,
with the `@n` lines before it joined to it.
"""
import argparse
import bisect
import json
import os
import re
import struct

LINE = re.compile(r'^(\d+):(\d\d):(\d\d)\.(\d{3}) .*OutputDebugString: TMT ([0-9a-f]+) ([0-9a-f]+) (.*)$')


def well_formed(c):
    """script::parse's test."""
    name, paren, rest = c.partition('(')
    return paren and rest.endswith(')') and name.startswith('_') and ')' not in name


def opcode(name):
    """script::Command::opcode: `_ZM01602` is `_ZM`."""
    return name[:3] if re.fullmatch(r'_Z.[0-9a-fA-F]{5}', name) else name


class Scripts:
    def __init__(self, scr):
        raw = open(os.path.join(scr, '00000.bin'), 'rb').read()
        self.names = [n for n in (raw[i:i + 32].split(b'\0')[0].decode('utf-8').strip() for i in range(0, len(raw), 32)) if n]
        self.scr, self.starts = scr, {}

    def at(self, i, off):
        if i not in self.starts:
            text = open(os.path.join(self.scr, f'{i + 3:05d}.bin'), 'rb').read().decode('utf-8')
            starts, pos = [], 0
            for c in text.split(';'):
                if c and well_formed(c):
                    starts.append(len(text[:pos].encode()))
                pos += len(c) + 1
            self.starts[i] = starts
        return f'{self.names[i]}:{bisect.bisect_right(self.starts[i], off) - 1}'


def text_lines(txt):
    offs = open(os.path.join(txt, '00000.bin'), 'rb').read()
    data = open(os.path.join(txt, '00001.bin'), 'rb').read()
    starts = [o for (o,) in struct.iter_unpack('>I', offs[:len(offs) // 4 * 4]) if o != 0xFFFFFFFF]
    return [data[a:b].decode('utf-8').removesuffix('\r\n') for a, b in zip(starts, starts[1:] + [len(data)])]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('log')
    ap.add_argument('scr')
    ap.add_argument('txt')
    ap.add_argument('-o', '--out', required=True)
    ap.add_argument('--pages', type=int, default=0, help='stop after this many pages (0: all)')
    a = ap.parse_args()
    scripts, lines = Scripts(a.scr), text_lines(a.txt)
    out, last, held, pg = [], None, '', 0
    for row in open(a.log, encoding='utf-8', errors='replace'):
        m = LINE.match(row.rstrip('\n'))
        if not m or not m[7].startswith('_'):
            continue  # a wait's way back: the ';' before the command
        h, mi, s, ms, i, off, cmd = m.groups()
        key = (int(i, 16), int(off, 16))
        if key == last:
            continue
        last = key
        t = ((int(h) * 60 + int(mi)) * 60 + int(s)) * 1000 + int(ms)
        name, _, args = cmd.partition('(')
        op, at = opcode(name), scripts.at(*key)
        if op in ('_ZM', '_MSAD'):
            n = re.search(r'\$(\d{6})', args)
            held += re.sub(r'<([^|<>]*)\|[^<>]*>', r'\1', lines[int(n[1])]) if n else ''
            if '@n' in args:
                continue
            pg += 1
            out.append({'t': 'page', 'pg': pg, 'ms': t, 'at': at, 'segs': [held]})
            held = ''
            if pg == a.pages:
                break
        else:
            out.append({'t': 'cmd', 'pg': pg, 'ms': t, 'at': at, 'op': op})
    with open(a.out, 'w', encoding='utf-8') as f:
        f.writelines(json.dumps(r, ensure_ascii=False) + '\n' for r in out)
    print(f'{pg} pages, {len(out)} records -> {a.out}')


if __name__ == '__main__':
    main()
