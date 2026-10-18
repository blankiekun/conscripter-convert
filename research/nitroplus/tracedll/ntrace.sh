#!/bin/bash
# ntrace.sh: play 装甲悪鬼村正 (Muramasa.exe) or 続・殺戮のジャンゴ (Django.exe)
# under wine with ntrace.dll and write its JSONL trace. Headless: own Xvfb
# (-displayfd), ja_JP locale, null audio. Muramasa, with no clicks: a patched copy of nss.npa skips the logos and the
# title (0_boot.nss: $Logo, $TitleSelect=1, as research/nitroplus/local
# patch/game) and turns auto mode on as the text window unlocks it
# (function.nss line 1139, TypeBeginCIO; set while locked, as the
# boot's `$SYSTEM_text_auto` would be, it stays off); every patched line
# stays on its line, so the
# trace's lines are the shipped scripts' lines.
#
#   ntrace.sh -o trace.jsonl [--pages N] [--seconds S] [--shot NAME.png]
#
# SPEED=N runs the game N times as fast (its clock and waits scaled by
# research/tools/trace_speed.h; "ms" stays in game time, so the trace
# diff's times hold: docs/new-game.md, "Trace speeds").
#
# Django: its 1.01 Django.exe is packed and asks for the disc: EXE = the
# unpacked 1.01 exe (docs/games/django.md "The exe"), copied in; the boot
# skips the logos and the title ($TitleSelect=1) and the start script turns
# auto mode on where it unlocks the menu (dja000, `$SYSTEM_menu_lock = 0;`);
# it runs in wine's virtual desktop (windowed; full screen draws twice)
# with the pointer nudged; ntrace.dll feeds its message loop and keeps
# SuspendThread from hanging it (ntrace.c); where it still stops for 60 s,
# a click goes on (logged in $NTRACE_HOME/clicks.log).
#
# GAME = the folder holding Muramasa.exe or Django.exe (1.01 files over the
# install) and the *.npa (required).
# NTRACE_HOME (default out/ntrace in the repo) keeps the wine prefix
# between runs, the built DLL and launcher, and the patched nss.npa.
# --pages stops after N pages (default 60), --seconds after S seconds
# (default 900). Records: ntrace.c.
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
H=${NTRACE_HOME:-$repo/out/ntrace}
OUT= PAGES=60 SECS=900 SHOT=
while [ $# -gt 0 ]; do case $1 in
  -o) OUT=$2; shift 2;; --pages) PAGES=$2; shift 2;; --seconds) SECS=$2; shift 2;;
  --shot) SHOT=$2; shift 2;; *) sed -n '2,17p' "$0"; exit 2;; esac; done
EXE_NAME=Muramasa.exe; [ -f "$GAME/Django.exe" ] && EXE_NAME=Django.exe
[ -n "$OUT" ] && [ -f "$GAME/$EXE_NAME" ] || { echo "need -o and GAME=<folder of Muramasa.exe or Django.exe>" >&2; exit 2; }
[ $EXE_NAME = Muramasa.exe ] || [ -f "$EXE" ] || { echo "Django: need EXE=<the unpacked 1.01 Django.exe>" >&2; exit 2; }

export WINEPREFIX=$H/prefix WINEDEBUG=-all WINEARCH=win64 WAYLAND_DISPLAY=
export LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8 PULSE_SERVER=unix:/nonexistent
export ALSA_CONFIG_PATH=$H/alsa-null.conf
export WINEDLLOVERRIDES="winepulse.drv,wineoss.drv,winewayland.drv=d;winemenubuilder.exe=d"
mkdir -p "$H/build"
printf 'pcm.!default { type null }\nctl.!default { type hw card 0 }\n' > "$ALSA_CONFIG_PATH"

i686-w64-mingw32-gcc -O2 -Wall -shared -static-libgcc -o "$H/build/ntrace.dll" "$here/ntrace.c" -lwinmm
# -mwindows: no console window over the game in wine's desktop
i686-w64-mingw32-gcc -O2 -Wall -static-libgcc -mwindows -o "$H/build/xlaunch.exe" "$repo/research/xuse/tracedll/xlaunch.c"

# the patched nss.npa (the game's archive is only read)
key=$GAME/Muramasa.exe
if [ $EXE_NAME = Django.exe ]; then  # its table isn't plain bytes in the exe: npa.rs DJANGO_TABLE
  key=$H/django_table.bin
  python3 -I - "$repo/crates/conv-engines/src/nitroplus/npa.rs" "$key" <<'PY'
import re, sys
src = open(sys.argv[1], encoding='utf-8').read()
body = src[src.index('pub const DJANGO_TABLE'):]
body = body[body.index('= [') + 3:body.index('];')]
t = bytes(int(v, 0) for v in re.findall(r'0x[0-9a-fA-F]+|\d+', re.sub(r'//[^\n]*', '', body)))
assert len(t) == 256 and len(set(t)) == 256, len(t)
open(sys.argv[2], 'wb').write(t)
PY
fi
npa="python3 -I $repo/research/nitroplus/npa.py $key $GAME/nss.npa"
x=$H/patch/x; rm -rf "$H/patch"; mkdir -p "$x" "$H/patch/nss"
if [ $EXE_NAME = Django.exe ]; then
$npa "$x" 'boot.nss' >/dev/null; $npa "$x" 'dja000_*' >/dev/null
python3 -I - "$x" "$H/patch/nss" <<'PY'
import pathlib, sys
src, dst = map(pathlib.Path, sys.argv[1:])
# (file, old, new): each once, on its own line, so no line moves
for name, old, new in [('boot.nss', '\t$Logo += 0;', '\t$Logo = 1;'),
                       ('boot.nss', '\t\tTitleSelect();', '\t\t$TitleSelect=1;'),
                       ('boot.nss', '#SYSTEM_product_code="ZokuSatsurikunoJyango";',
                        '#SYSTEM_product_code="ZokuSatsurikunoJyango";#SYSTEM_window_full=false;'),
                       ('dja000_*', '\t$SYSTEM_menu_lock = 0;\r\n\r\n\tcall_scene @->main_root;',
                        '\t$SYSTEM_menu_lock = 0;$SYSTEM_text_auto=true;\r\n\r\n\tcall_scene @->main_root;')]:
    f = next(dst.glob(name), None) or next(src.rglob(name))
    t = f.read_bytes()
    o, n = old.encode('cp932'), new.encode('cp932')
    assert t.count(o) == 1, (name, old)
    (dst / f.name).write_bytes(t.replace(o, n))
PY
else
$npa "$x" '*0_boot.nss' >/dev/null; $npa "$x" 'function.nss' >/dev/null
python3 -I - "$x" "$H/patch/nss" <<'PY'
import pathlib, sys
src, dst = map(pathlib.Path, sys.argv[1:])
# (file, line, old, new): byte for byte, so no line moves
for name, line, old, new in [('0_boot.nss', 85, '\t\t//$Logo = true;', '\t\t$Logo = true;'),
                             ('0_boot.nss', 157, '\t\tTitleSelect();', '\t\t$TitleSelect=1;'),
                             ('function.nss', 1139, '\t\t$SYSTEM_text_auto_lock=false;', '\t\t$SYSTEM_text_auto_lock=false;$SYSTEM_text_auto=true;')]:
    f = dst / name if (dst / name).exists() else next(src.rglob(name))
    lines = f.read_bytes().split(b'\n')
    assert lines[line - 1].rstrip(b'\r') == old.encode('cp932'), (name, line, lines[line - 1])
    lines[line - 1] = lines[line - 1].replace(old.encode('cp932'), new.encode('cp932'))
    (dst / name).write_bytes(b'\n'.join(lines))
PY
fi
$npa --pack "$H/patch/nss" "$H/nss.npa" >/dev/null

rm -f "$H/xvfb.d"
Xvfb -displayfd 3 -screen 0 1280x800x24 -nolisten tcp 3>"$H/xvfb.d" 2>/dev/null & xpid=$!
until [ -s "$H/xvfb.d" ]; do sleep 0.1; done
export DISPLAY=:$(cat "$H/xvfb.d")
cleanup() { wineserver -k 2>/dev/null || true; kill "$xpid" 2>/dev/null || true; }
trap cleanup EXIT

C=$WINEPREFIX/drive_c
if [ ! -e "$H/prefix/.ready" ]; then
  wineboot -i >/dev/null 2>&1; wineserver -w
  for u in "$C"/users/*/; do for d in "$u"*; do [ -L "$d" ] && rm "$d" && mkdir "$d"; done; done
  wine reg add 'HKCU\Software\Wine\Drivers' /v Audio /d alsa /f >/dev/null 2>&1
  wine reg add 'HKCU\Software\Wine\Drivers' /v Graphics /d x11 /f >/dev/null 2>&1
  wineserver -w
  touch "$H/prefix/.ready"
fi
M=$C/game; rm -rf "$M"; mkdir -p "$M" "$C/ntrace"  # a farm of links: nothing writes into the game
for f in "$GAME"/*; do ln -s "$f" "$M/"; done
ln -sfn "$H/nss.npa" "$M/nss.npa"
[ $EXE_NAME = Django.exe ] && { rm "$M/Django.exe"; cp "$EXE" "$M/Django.exe"; }
cp "$H/build/ntrace.dll" "$H/build/xlaunch.exe" "$C/ntrace/"
rm -f "$C/ntrace/trace.jsonl" "$H/clicks.log"

# wine's explorer may hand the game on and exit; shortcut: another prefix's
# Django.exe counts too (a run then lasts --seconds)
alive() { kill -0 $wpid 2>/dev/null || { [ $EXE_NAME = Django.exe ] && pgrep -x Django.exe >/dev/null; }; }
export NTRACE_OUT='C:\ntrace\trace.jsonl' NTRACE_PAGES=$PAGES TRACE_SPEED=${SPEED:-1}
if [ $EXE_NAME = Django.exe ]; then
  (cd "$M" && wine explorer /desktop=ntrace,1024x768 'C:\ntrace\xlaunch.exe' 'C:\game\Django.exe' 'C:\ntrace\ntrace.dll') > "$H/wine.log" 2>&1 &
  wpid=$!
  # the pointer nudged over the window: auto mode stops without input
  (sleep 5; xdotool mousemove 512 600; while alive; do xdotool mousemove_relative -- 1 0; sleep 0.015; xdotool mousemove_relative -- -1 0; sleep 0.015; done) >/dev/null 2>&1 &
  # and should it still stop: a click after 60 s without a record (a text
  # block's pages are all logged as it starts, so a long one is quiet)
  (T="$C/ntrace/trace.jsonl"; sleep 10; while alive; do
     sleep 2; [ -f "$T" ] && [ $(( $(date +%s) - $(stat -c %Y "$T") )) -ge 60 ] || continue
     xdotool mousedown 1; sleep 0.15; xdotool mouseup 1; echo "click at $(wc -l < "$T") records" >> "$H/clicks.log"; sleep 10
   done) >/dev/null 2>&1 &
else
  wine 'C:\ntrace\xlaunch.exe' 'C:\game\Muramasa.exe' 'C:\ntrace\ntrace.dll' > "$H/wine.log" 2>&1 &
  wpid=$!
fi
end=$((SECONDS + SECS))
while alive && [ $SECONDS -lt $end ]; do sleep 1; done
[ -n "$SHOT" ] && import -window root "$SHOT"
cp "$C/ntrace/trace.jsonl" "$OUT"
echo "$(grep -c '"t":"page"' "$OUT") pages, $(wc -l < "$OUT") records -> $OUT"
