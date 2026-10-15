"""Patched boots of 続・殺戮のジャンゴ for running the original under wine.

    python3 -I django_patch.py TABLE NSS_NPA OUT.npa [CHAPTER [SKIP]]

TABLE is the 256-byte NPA table (`npa::DJANGO_TABLE` as a file, see
npa.py), NSS_NPA the 1.01 `nss.npa`. Always: no logos, every ending seen
(`#ClearG`, `#Clear01`-`#Clear10`: the title shows EXTRA and LINER NOTES),
windowed.
With CHAPTER (a prefix such as `dja023`), START plays from that chapter
file: `main_root`'s earlier `call_chapter`s are commented out. With SKIP
too (a name in the chapter file, `cut` or `scene`), the chapter file's own
calls before that one are commented out as well, and in the scene file
SKIP names, timed zooms and camera moves take no time (they hang under
wine). A CHAPTER starting
with `extra_` (`extra_sound`) is instead opened by the title's EXTRA, in
place of the extras' menu (whose doors' zoom stalls under wine).
"""
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))


def npa(*args):
    subprocess.run([sys.executable, '-I', os.path.join(HERE, 'npa.py'), *args], check=True,
                   stdout=subprocess.DEVNULL)


def comment_before(text, start, end, stop, call):
    """Comment out the `call` lines between start and end until one naming `stop`."""
    lines = text[start:end].split('\r\n')
    for i, line in enumerate(lines):
        if stop in line and call in line:
            break
        if call in line and not line.lstrip().startswith('//'):
            lines[i] = '//' + line
    else:
        sys.exit(f'{stop}: no {call} names it')
    return text[:start] + '\r\n'.join(lines) + text[end:]


def main():
    table, src, out = sys.argv[1:4]
    chapter = sys.argv[4] if len(sys.argv) > 4 else None
    skip = sys.argv[5] if len(sys.argv) > 5 else None
    with tempfile.TemporaryDirectory() as d:
        npa(table, src, d, 'boot.nss')
        p = os.path.join(d, 'boot.nss')
        t = open(p, 'rb').read().decode('cp932')
        old = '\t$Logo += 0;\r\n\r\n\tif($Logo == 0)'
        assert t.count(old) == 1
        t = t.replace(old, '\t$Logo = 1;\r\n\r\n\tif($Logo == 0)')
        for v in ['#ClearG = 1;'] + [f'#Clear{i:02d} = 1;' for i in range(1, 11)]:
            assert t.count('//\t\t' + v) == 1, v
            t = t.replace('//\t\t' + v, '\t\t' + v)
        # Windowed: in wine's virtual desktop (full screen draws two windows).
        old = '#SYSTEM_product_code="ZokuSatsurikunoJyango";'
        assert t.count(old) == 1
        t = t.replace(old, old + '\r\n\t#SYSTEM_window_full = false;')
        if chapter and chapter.startswith('extra_'):
            old = 'call_chapter nss/extra_menu.nss;'
            assert t.count(old) == 1
            t = t.replace(old, f'call_chapter nss/{chapter}.nss;')
        open(p, 'wb').write(t.encode('cp932'))
        if chapter and not chapter.startswith('extra_'):
            start = 'dja000_開始スクリプト.nss'
            npa(table, src, d, start)
            p = os.path.join(d, start)
            t = open(p, 'rb').read().decode('cp932')
            a = t.index('scene main_root')
            t = comment_before(t, a, len(t), 'nss/' + chapter, 'call_chapter')
            open(p, 'wb').write(t.encode('cp932'))
            if skip:
                listing = subprocess.run([sys.executable, '-I', os.path.join(HERE, 'npa.py'), table, src],
                                         capture_output=True, check=True).stdout.decode()
                names = [line.split(None, 1)[1] for line in listing.splitlines()]
                name = next(n for n in names if n.startswith(chapter) and n.count('_') == 1)
                npa(table, src, d, name)
                p = os.path.join(d, name)
                t = open(p, 'rb').read().decode('cp932')
                a = re.search(r'chapter main\s*\{', t).end()
                t = comment_before(t, a, len(t), skip, 'call_')
                open(p, 'wb').write(t.encode('cp932'))
                # Timed zooms hang under wine: in the scene SKIP names they
                # (and camera moves, which zoom) take no time.
                scene = next(n for n in names if n.startswith(chapter) and skip in n)
                npa(table, src, d, scene)
                p = os.path.join(d, scene)
                t = open(p, 'rb').read().decode('cp932')
                t = re.sub(r'\b((?:Move)?Zoom|MoveCamera\w*)\((\s*"[^"]*"\s*),\s*\d+', r'\1(\2,0', t)
                open(p, 'wb').write(t.encode('cp932'))
        npa(table, src, '--pack', d, out)


main()
