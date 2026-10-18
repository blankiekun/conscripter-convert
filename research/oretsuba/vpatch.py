#!/usr/bin/env python3
"""A trace of 俺たちに翼はない (Vita) under Vita3K, from an eboot patch.

    vpatch.py patch <Vita3K data dir>/patch/PCSG00299.txt
    vpatch.py log <emu>/logs/vita3k.log orig.jsonl

`patch` writes a Vita3K patch file (applied to the eboot as it loads; the
game's files stay as they are) that hooks three functions with a `b.w` over
their first instruction (a push, which the hook runs before jumping back):
- task_start (0x8100da30, r0 = the game context): printf("@T scene task op")
  for the record about to run, and printf("@M text") for a Message (task
  types 0 and 2; the text at record +0xb8, cp932). The scene is the top of
  the context's scene stack (+0x4440/+0x4444, 0x1f0 bytes each; empty: the
  scene at +0x424c); its current index is +0x1b8, its task numbers (the N of
  [TaskN]) a vector at +4, the current record +0x4450 of the context.
- scene_load (0x8102840a): printf("@S scene path") (r0, r1).
- the frame flip (0x81000898, which queues each frame for display):
  printf("@F").
MoviePlay and MovieStop (0x48, 0x49) become the no-op task 0x7e (what
scene_load makes of an unknown type) once logged: Vita3K crashes in a
worker thread (a memcpy out of guest memory) when the game starts a movie.
It also turns the scene jump dialog off (task_run's case 0x84 takes the
setting シーンスキップ機能 as OFF: the `cbnz` at 0x810265e2 becomes a nop),
which Vita3K (0.2.1 4190) crashes drawing (`Unhandled access`), so a scene
starts as an unread one does: at its :UnreadTag.
The hooks live in FUN_810a8c9e, a function nothing calls or points at.
printf (the libc import at 0x81131c24) reaches Vita3K's log as
`*** TTY:` lines at log level 0 (Trace); `log` turns them into
research/tools/trace_diff.py's records: "cmd" {at "<scene>:<task>", op,
f}, "page" {at, f, pg, segs} after a Message's cmd, f counting flips.

Needs llvm-mc and llvm-objcopy.
"""
import json
import os
import re
import subprocess
import sys
import tempfile

BASE = 0x81000000           # segment 0, where Vita3K loads it (its log says so)
CAVE = 0x810a8ca0           # in FUN_810a8c9e (1072 bytes, dead), 4-aligned for the literals
NO_JUMP_DIALOG = 0x810265e2  # task_run case 0x84: cbnz r1 (the setting) over movs r0, #0
PRINTF = 0x81131c24         # SceLibc printf stub (ARM)
HOOKS = {  # address: (its first 4 bytes, as asm)
    'task': (0x8100da30, 'push.w {r4, r5, r6, r7, r8, r9, r10, r11, lr}'),
    'scene': (0x8102840a, 'push.w {r4, r5, r6, r7, r8, r9, r10, r11, lr}'),
    'flip': (0x81000898, 'push {r4, r5, r6, r7, lr}\n sub sp, #28'),
}

ASM = r'''
 .syntax unified
 .thumb
 .macro enter
 push.w {r0-r12, lr}
 vpush {d0-d7}
 .endm
 .macro leave addr, displaced
 vpop {d0-d7}
 pop.w {r0-r12, lr}
 \displaced
 ldr.w pc, 7f
 .p2align 2
7: .word \addr+5
 .endm
 .macro call fmt
 adr.w r0, \fmt
 ldr.w r12, =%(printf)d
 blx r12
 .endm

task:
 enter
 mov r4, r0
 movw r6, #0x4400
 add r6, r4
 ldr r7, [r6, #0x50]
 cbz r7, 9f
 ldr r0, [r6, #0x40]
 ldr r1, [r6, #0x44]
 subs r2, r1, r0
 cmp.w r2, #0x1f0
 blt 1f
 sub.w r5, r1, #0x1f0
 b 2f
1:
 movw r5, #0x424c
 add r5, r4
2:
 ldr.w r2, [r5, #0x1b8]
 ldr r3, [r5, #4]
 ldr.w r2, [r3, r2, lsl #2]
 ldr r3, [r7]
 mov r1, r5
 call fmt_t
 ldr r3, [r7]
 cmp r3, #0x48
 beq 4f
 cmp r3, #0x49
 bne 5f
4:
 movs r3, #0x7e
 str r3, [r7]
 b 9f
5:
 cmp r3, #0
 beq 3f
 cmp r3, #2
 bne 9f
3:
 add.w r1, r7, #0xb8
 call fmt_m
9:
 leave %(task)d, "push.w {r4, r5, r6, r7, r8, r9, r10, r11, lr}"
 .ltorg

scene:
 enter
 call fmt_s
 leave %(scene)d, "push.w {r4, r5, r6, r7, r8, r9, r10, r11, lr}"
 .ltorg

flip:
 enter
 call fmt_f
 leave %(flip)d, "push {r4, r5, r6, r7, lr} ; sub sp, #28"
 .ltorg

 .p2align 2
fmt_t: .asciz "@T %%x %%d %%x\n"
 .p2align 2
fmt_m: .asciz "@M %%s\n"
 .p2align 2
fmt_s: .asciz "@S %%x %%s\n"
 .p2align 2
fmt_f: .asciz "@F\n"
'''


def assemble():
    src = ASM % {'printf': PRINTF, **{k: a for k, (a, _) in HOOKS.items()}}
    # the scene hook gets r0/r1 untouched: printf's r1, r2 = scene, path
    src = src.replace('scene:\n enter\n call fmt_s', 'scene:\n enter\n mov r2, r1\n mov r1, r0\n call fmt_s')
    with tempfile.TemporaryDirectory() as t:
        open(f'{t}/c.s', 'w').write(src)
        subprocess.run(['llvm-mc', '-triple=thumbv7', '-mattr=+vfp3', '-filetype=obj', f'{t}/c.s', '-o', f'{t}/c.o'], check=True)
        syms = subprocess.run(['llvm-nm', f'{t}/c.o'], capture_output=True, text=True, check=True).stdout
        subprocess.run(['llvm-objcopy', '-O', 'binary', '--only-section=.text', f'{t}/c.o', f'{t}/c.bin'], check=True)
        code = open(f'{t}/c.bin', 'rb').read()
    labels = {m.group(2): int(m.group(1), 16) for m in re.finditer(r'^([0-9a-f]+) t (\w+)$', syms, re.M)}
    assert len(code) <= 1072 and labels['task'] == 0, (len(code), labels)
    return code, labels


def b_w(src, dst):
    """Thumb-2 B.W (T4) at src to dst."""
    off = dst - (src + 4)
    assert -(1 << 24) <= off < (1 << 24) and off % 2 == 0
    s = (off >> 24) & 1
    i1, i2 = (off >> 23) & 1, (off >> 22) & 1
    j1, j2 = (1 - i1) ^ s, (1 - i2) ^ s
    imm10, imm11 = (off >> 12) & 0x3ff, (off >> 1) & 0x7ff
    h1 = 0xf000 | (s << 10) | imm10
    h2 = 0x9000 | (j1 << 13) | (j2 << 11) | imm11
    return h1.to_bytes(2, 'little') + h2.to_bytes(2, 'little')


def line(addr, data):
    return f'0:0x{addr - BASE:X} ' + ' '.join(f'0x{b:02X}' for b in data)


def patch(path):
    code, labels = assemble()
    rows = ['# vpatch.py: trace hooks (research/oretsuba/vpatch.py)', '[eboot.bin]', line(CAVE, code)]
    for k, (addr, _) in HOOKS.items():
        rows.append(line(addr, b_w(addr, CAVE + labels[k])))
    rows.append(line(NO_JUMP_DIALOG, b'\x00\xbf'))  # nop over `cbnz r1` (setting on)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, 'w').write('\n'.join(rows) + '\n')
    print(f'{len(code)} bytes at {CAVE:#x}; hooks {", ".join(k for k in HOOKS)}')


TTY = re.compile(rb'^\[[\d:.]+\] \|.\| \[write_file\]: \*\*\* TTY: ?(.*)$')
# Vita3K's log folds repeated lines: "Skipped N duplicate messages.." before the next one
SKIPPED = re.compile(rb'^\[[\d:.]+\] \|.\| \[write_file\]: Skipped (\d+) duplicate messages')


def log(path, out):
    frames, pages, scenes, n, last = 0, 0, {}, 0, None
    with open(path, 'rb') as f, open(out, 'w') as o:
        lines = f.read().split(b'\n')
        i = 0
        while i < len(lines):
            m = TTY.match(lines[i])
            k = SKIPPED.match(lines[i])
            i += 1
            if k and last == '@F':
                frames += int(k.group(1))
            if not m:
                continue
            body = m.group(1)
            # a message's line breaks continue the TTY line
            while body.startswith(b'@M ') and i < len(lines) and not lines[i].startswith((b'[', b'Unhandled')):
                body += b'\n' + lines[i]
                i += 1
            text = last = body.decode('cp932', 'replace').rstrip('\n')
            if text == '@F':
                frames += 1
            elif text.startswith('@S '):
                ptr, p = text[3:].split(' ', 1)
                scenes[ptr] = p.replace('\\', '/').rsplit('/', 1)[-1].split('.')[0]
            elif text.startswith('@T '):
                ptr, task, op = text[3:].split()
                at = f'{scenes.get(ptr, ptr)}:{task}'
                o.write(json.dumps({'t': 'cmd', 'at': at, 'op': '0x' + op, 'f': frames}) + '\n')
                n += 1
            elif text.startswith('@M '):
                pages += 1
                # the record keeps the script's `\n`; the line layout breaks there (0x81016eca)
                o.write(json.dumps({'t': 'page', 'at': at, 'f': frames, 'pg': pages, 'segs': [text[3:].replace('\\n', '\n')]}, ensure_ascii=False) + '\n')
    print(f'{n} tasks, {pages} pages, {frames} frames, scenes {sorted(set(scenes.values()))}')


if __name__ == '__main__':
    {'patch': lambda: patch(sys.argv[2]), 'log': lambda: log(sys.argv[2], sys.argv[3])}[sys.argv[1]]()
