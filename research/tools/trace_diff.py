#!/usr/bin/env python3
"""Compare a trace of the original game with CONscripter's debug trace.

    trace_diff.py ORIG.jsonl OURS.jsonl --map MAP [--pages N] [--fps F]
                  [--ms] [--merge-into OP[,OP]] [--thread T] [--max K] [--self-test]

ORIG: the original's trace, "cmd" and "page" records whose "at" is
"<script>:<instruction>" and whose "f" counts frames (Xuse:
research/xuse/tracedll/xtrace.sh), or, with --ms, whose "ms" times them
(Nitroplus NSS: research/nitroplus/tracedll/ntrace.sh; --thread 0 keeps
the story's thread, leaving out the processes that run beside it). OURS: CONscripter's `--trace` of a
run with `--auto-play N --real-time` (engine/DEBUG.md), "at" =
"*label:line" of 0.txt. MAP: the converter's `--source-map` for that
0.txt (`<line>\\t<file>:<instruction>`, "0004.cd" read as script 4).

Pages are paired in order by the instruction that shows them; then, per
page, between it and the page before:
- text: the page's text differs (speaker, spaces and line breaks left
  out), as the whole screen's or, where our page continues the one
  before, as its own lines (TYPE-MOON's trace has only those);
- missing: an instruction the original ran that none of our commands
  came from (the page's own message left out), for opcodes that did give one of ours elsewhere in the run
  (variables and jumps never do);
- merged: a missing one whose next command of the page that has ours
  is one of --merge-into's ops (those that commit staged changes, e.g.
  Tsukihime's _FADS): our ops for it sit under that command's mark
  (counted, not a finding). Without the option, none;
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
    text = re.sub(r'\(([^/()]+)/[^/()]+\)', r'\1', text)  # ours: ruby as (base/reading)
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


def orig_side(path, max_pages, clock='f', thread=None):
    recs = [r for r in records(path) if r['t'] in ('cmd', 'page') and (thread is None or r.get('th') == thread)]
    for a, b in zip(recs, recs[1:]):
        a['end'] = b[clock]  # a command ends when the next record starts
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
        pages.append({'at': page['at'], 'text': norm(''.join(segs), speaker), 'shown': page[clock], 'ends': ends, 'ops': ops, 'pg': page['pg']})
    return pages


def ours_side(path, m, max_pages):
    recs = [r for r in records(path) if r['t'] in ('ev', 'page')]
    st = stretches(recs, lambda r: True)[:max_pages]
    pages, before = [], None
    for page, evs in st:
        ends, helpers = {}, 0
        for e in evs:
            k = lookup(m, line_of(e.get('at')))
            if k is None:
                helpers += 1
            else:
                ends[k] = e['ms']  # the last command from an instruction
        speaker = (page.get('tag') or '').split('|')[0] or None
        text = page.get('text') or ''
        # a page added below the one before (full-screen text): its own lines too
        own = norm(text[len(before):], speaker) if before and text.startswith(before) else None
        before = text
        pages.append({'at': lookup(m, line_of(page.get('at'))), 'text': norm(text, speaker), 'own': own, 'shown': page['ms'],
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


def compare(orig, ours, fps, unit=None, merge_into=()):
    """unit: ms per step of the original's clock (default: a frame)."""
    frame = 1000.0 / fps
    unit = unit or frame
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
        if o['text'] not in (p['text'], p.get('own')):
            findings.append((o['pg'], 'text', f"{where}: original {o['text']!r}, ours {p['text']!r}"))
        keys = list(o['ops'])
        for i, (k, op) in enumerate(o['ops'].items()):
            if k not in p['ends'] and op in visible and k != o['at']:
                nxt = next((y for y in keys[i + 1:] if y in p['ends']), None)
                if nxt and o['ops'][nxt] in merge_into:  # ours emits its ops under the next command's mark
                    findings.append((o['pg'], 'merged', f"{where}: {k} (op {op}) credited to {nxt}"))
                else:
                    findings.append((o['pg'], 'missing', f"{where}: {k} (op {op}) gave none of ours"))
        for k in p['ends']:
            if k not in o['ops'] and k != o['at']:
                findings.append((o['pg'], 'extra', f"{where}: ours ran a command from {k}, the original didn't run it here"))
        both = sorted((o['ends'][k], k) for k in o['ends'] if k in p['ends'])
        if not both:
            continue
        for (f0, k0), (f, k) in zip(both, both[1:]):
            want = (f - f0) * unit
            got = p['ends'][k] - p['ends'][k0]
            if abs(got - want) > frame:
                steps = f' ({f - f0} frames)' if unit == frame else ''
                findings.append((o['pg'], 'time', f"{where}: {k0} to {k} {got:.0f} ms, original {want:.0f} ms{steps}"))
    return findings


def self_test():
    m = ([10, 20, 30], ['4:1', '4:2', None])
    assert lookup(m, 25) == '4:2' and lookup(m, 31) is None and lookup(m, 5) is None
    assert source_key('0004.cd:19') == '4:19' and source_key('-') is None
    assert norm('　遠く\n　から', None) == '遠くから' and norm('？？「あ」', '？？') == '「あ」'
    assert norm('(纏/まと)い(元/・)') == '纏い元'
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
    # in ms (Nitroplus NSS): 4:0 to 4:9 10 ms against 400 ms
    assert sorted(k for _, k, _ in compare(orig, ours, 60.0, 1.0)) == ['extra', 'page', 'time']
    orig[0]['ends']['4:9'] = 400
    assert sorted(k for _, k, _ in compare(orig, ours, 60.0, 1.0)) == ['extra', 'page']
    # a missing command whose next one has ours is merged into it
    orig[0]['ops'] = {'4:0': '0x76', '4:8': '0x76', '4:9': '0x76'}
    orig[0]['ends'] = {'4:0': 10, '4:9': 400}
    ours[0]['ends'] = {'4:0': 0, '4:9': 400, '4:1': 400}
    assert sorted(k for _, k, _ in compare(orig, ours, 60.0, 1.0)) == ['missing', 'page']
    orig[0]['ops']['4:9'] = '_FADS'
    got = sorted(k for _, k, _ in compare(orig, ours, 60.0, 1.0, {'_FADS'}))
    assert got == ['merged', 'page'], got
    # a continued page: the original's text is its own lines
    ours[0].update(text='za', own='a')
    assert 'text' not in [k for _, k, _ in compare(orig, ours, 60.0, 1.0)]
    print('self-test ok')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('orig', nargs='?')
    ap.add_argument('ours', nargs='?')
    ap.add_argument('--map')
    ap.add_argument('--pages', type=int, default=50, help='compare the first N pages (default 50)')
    ap.add_argument('--fps', type=float, default=30.1, help="the original's frames a second (Xuse: 30.1; Nitroplus NSS: 60); the time tolerance is one frame")
    ap.add_argument('--ms', action='store_true', help="time the original by its \"ms\" (Nitroplus NSS: its waits are timeGetTime waits), not its frames")
    ap.add_argument('--thread', type=int, help='only the original\'s records of this "th" (Nitroplus NSS: 0, the story; the others are processes)')
    ap.add_argument('--merge-into', default='', help="the original's ops that commit staged changes (Tsukihime: _FADS): a missing command before one that has ours is 'merged'")
    ap.add_argument('--max', type=int, default=20, help='findings shown per kind (default 20)')
    ap.add_argument('--self-test', action='store_true')
    a = ap.parse_args()
    if a.self_test:
        return self_test()
    if not (a.orig and a.ours and a.map):
        ap.error('ORIG, OURS and --map are needed')
    m = read_map(a.map)
    orig = orig_side(a.orig, a.pages, 'ms' if a.ms else 'f', a.thread)
    findings = compare(orig, ours_side(a.ours, m, a.pages), a.fps, 1.0 if a.ms else None, set(filter(None, a.merge_into.split(','))))
    shown = {}
    for pg, kind, text in sorted(findings, key=lambda f: f[0]):
        shown[kind] = shown.get(kind, 0) + 1
        if shown[kind] <= a.max and kind != 'merged':
            print(f'{kind:8} {text}')
    print('totals: ' + (', '.join(f'{k} {n}' for k, n in sorted(shown.items())) or 'no differences'))


if __name__ == '__main__':
    main()
