#!/usr/bin/env python3
"""Compare a trace of the original game with CONscripter's debug trace.

    trace_diff.py ORIG.jsonl OURS.jsonl --map MAP [--pages N] [--fps F]
                  [--max K] [--self-test]

ORIG: the original's trace, "cmd" and "page" records whose "at" is
"<script>:<instruction>" and whose "f" counts frames (Xuse:
research/xuse/tracedll/xtrace.sh). OURS: CONscripter's `--trace` of a
run with `--auto-play N --real-time` (engine/DEBUG.md), "at" =
"*label:line" of 0.txt. MAP: the converter's `--source-map` for that
0.txt (`<line>\\t<file>:<instruction>`, "0004.cd" read as script 4).

Pages are paired in order by the instruction that shows them; then, per
page, between it and the page before:
- text: the page's text differs (speaker, spaces and line breaks left
  out);
- missing: an instruction the original ran that none of our commands
  came from (the page's own message left out), for opcodes that did give one of ours elsewhere in the run
  (variables and jumps never do);
- extra: an instruction ours ran a command from that the original
  didn't run there;
- time: the time from one command both ran to the next one off by more
  than one frame. The original's command ends when its next record
  starts; ours when its last `ev` is written. Auto mode's wait for the
  reader differs, and the original's page record comes as the message
  starts, ours once its text is drawn: nothing is timed across a page.
"""
import argparse
import bisect
import json
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../engine/tools'))
from cons_trace_check import records, line_of  # noqa: E402


def source_key(loc):
    """'0004.cd:19' -> '4:19' (the original's form); '-' -> None."""
    if loc == '-':
        return None
    file, index = loc.rsplit(':', 1)
    stem = file.split('.')[0]
    return f'{int(stem)}:{index}' if stem.isdigit() else f'{file}:{index}'


def read_map(path):
    lines, keys = [], []
    with open(path, encoding='utf-8') as f:
        for row in f:
            n, loc = row.rstrip('\n').split('\t')
            lines.append(int(n))
            keys.append(source_key(loc))
    return lines, keys


def lookup(m, line):
    lines, keys = m
    i = bisect.bisect_right(lines, line) - 1
    return keys[i] if i >= 0 and line is not None else None


def norm(text, speaker=None):
    text = re.sub(r'[\s　]', '', text or '')
    return text[len(speaker):] if speaker and text.startswith(speaker) else text


def stretches(recs, page_of):
    """[(page record or None, [cmds before it])], in order."""
    out, cmds = [], []
    for r in recs:
        if r['t'] == 'page':
            out.append((r, cmds))
            cmds = []
        elif page_of(r):
            cmds.append(r)
    return out


def orig_side(path, max_pages):
    recs = [r for r in records(path) if r['t'] in ('cmd', 'page')]
    for a, b in zip(recs, recs[1:]):
        a['end'] = b['f']  # a command ends when the next record starts
    st = stretches(recs, lambda r: True)[:max_pages]
    pages = []
    for page, cmds in st:
        segs = page.get('segs') or []
        # a speech page's first segment is the name
        speaker = segs[0] if len(segs) > 1 and segs[1].startswith('「') else None
        ends = {}
        for c in cmds:
            ends[c['at']] = c['end']
        ops = {c['at']: c['op'] for c in cmds}
        pages.append({'at': page['at'], 'text': norm(''.join(segs), speaker), 'shown': page['f'], 'ends': ends, 'ops': ops, 'pg': page['pg']})
    return pages


def ours_side(path, m, max_pages):
    recs = [r for r in records(path) if r['t'] in ('ev', 'page')]
    st = stretches(recs, lambda r: True)[:max_pages]
    pages = []
    for page, evs in st:
        ends, helpers = {}, 0
        for e in evs:
            k = lookup(m, line_of(e.get('at')))
            if k is None:
                helpers += 1
            else:
                ends[k] = e['ms']  # the last command from an instruction
        speaker = (page.get('tag') or '').split('|')[0] or None
        pages.append({'at': lookup(m, line_of(page.get('at'))), 'text': norm(page.get('text'), speaker), 'shown': page['ms'],
                      'ends': ends, 'helpers': helpers, 'pg': page['pg'], 'line': page.get('at')})
    return pages


def pair(orig, ours, look=5):
    """[(orig page or None, our page or None)] by the instruction showing them."""
    out, j = [], 0
    for o in orig:
        k = next((k for k in range(j, min(j + look, len(ours))) if ours[k]['at'] == o['at']), None)
        if k is None:
            out.append((o, None))
            continue
        out.extend((None, p) for p in ours[j:k])
        out.append((o, ours[k]))
        j = k + 1
    out.extend((None, p) for p in ours[j:])
    return out


def compare(orig, ours, fps):
    frame = 1000.0 / fps
    pairs = pair(orig, ours)
    visible = {o['ops'][k] for o, p in pairs if o and p for k in o['ops'] if k in p['ends']}
    findings = []
    for o, p in pairs:
        if p is None:
            findings.append((o['pg'], 'page', f"original page {o['pg']} ({o['at']}) not shown by ours"))
            continue
        if o is None:
            findings.append((p['pg'], 'page', f"our page {p['pg']} ({p['line']}, {p['at']}) not in the original"))
            continue
        where = f"page {o['pg']}/{p['pg']} ({o['at']})"
        if o['text'] != p['text']:
            findings.append((o['pg'], 'text', f"{where}: original {o['text']!r}, ours {p['text']!r}"))
        for k, op in o['ops'].items():
            if k not in p['ends'] and op in visible and k != o['at']:
                findings.append((o['pg'], 'missing', f"{where}: {k} (op {op}) gave none of ours"))
        for k in p['ends']:
            if k not in o['ops'] and k != o['at']:
                findings.append((o['pg'], 'extra', f"{where}: ours ran a command from {k}, the original didn't run it here"))
        both = sorted((o['ends'][k], k) for k in o['ends'] if k in p['ends'])
        if not both:
            continue
        for (f0, k0), (f, k) in zip(both, both[1:]):
            want = (f - f0) * frame
            got = p['ends'][k] - p['ends'][k0]
            if abs(got - want) > frame:
                findings.append((o['pg'], 'time', f"{where}: {k0} to {k} {got:.0f} ms, original {want:.0f} ms ({f - f0} frames)"))
    return findings


def self_test():
    m = ([10, 20, 30], ['4:1', '4:2', None])
    assert lookup(m, 25) == '4:2' and lookup(m, 31) is None and lookup(m, 5) is None
    assert source_key('0004.cd:19') == '4:19' and source_key('-') is None
    assert norm('　遠く\n　から', None) == '遠くから' and norm('？？「あ」', '？？') == '「あ」'
    orig = [{'at': '4:1', 'text': 'a', 'shown': 40, 'ends': {'4:0': 10, '4:9': 20}, 'ops': {'4:0': '0x76', '4:9': '0x01'}, 'pg': 1},
            {'at': '4:5', 'text': 'b', 'shown': 50, 'ends': {}, 'ops': {}, 'pg': 2}]
    ours = [{'at': '4:1', 'text': 'a', 'shown': 1000, 'ends': {'4:0': 0, '4:3': 5}, 'pg': 1, 'line': '*a:1'}]
    kinds = sorted(k for _, k, _ in compare(orig, ours, 30.0))
    # 4:9 (0x01) never gave a command: not missing; page 2 not shown; 4:3 extra
    assert kinds == ['extra', 'page'], kinds
    orig[0]['ops']['4:9'] = '0x76'
    ours[0]['ends'].update({'4:9': 400, '4:1': 400})
    found = compare(orig, ours, 30.0)
    # 4:0 to 4:9: 10 frames (333 ms) against 400 ms; 4:1 is the page: not extra
    assert sorted(k for _, k, _ in found) == ['extra', 'page', 'time'], found
    print('self-test ok')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('orig', nargs='?')
    ap.add_argument('ours', nargs='?')
    ap.add_argument('--map')
    ap.add_argument('--pages', type=int, default=50, help='compare the first N pages (default 50)')
    ap.add_argument('--fps', type=float, default=30.1, help="the original's frames a second (Xuse: 30.1)")
    ap.add_argument('--max', type=int, default=20, help='findings shown per kind (default 20)')
    ap.add_argument('--self-test', action='store_true')
    a = ap.parse_args()
    if a.self_test:
        return self_test()
    if not (a.orig and a.ours and a.map):
        ap.error('ORIG, OURS and --map are needed')
    m = read_map(a.map)
    findings = compare(orig_side(a.orig, a.pages), ours_side(a.ours, m, a.pages), a.fps)
    shown = {}
    for pg, kind, text in sorted(findings, key=lambda f: f[0]):
        shown[kind] = shown.get(kind, 0) + 1
        if shown[kind] <= a.max:
            print(f'{kind:8} {text}')
    print('totals: ' + (', '.join(f'{k} {n}' for k, n in sorted(shown.items())) or 'no differences'))


if __name__ == '__main__':
    main()
