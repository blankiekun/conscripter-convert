#!/bin/bash
# xtrace.sh: play 最果てのイマ (Farthest2015.exe) under wine with xtrace.dll
# and write its JSONL trace. Headless: own Xvfb, ja_JP locale, null audio.
#
#   xtrace.sh -o trace.jsonl [--save SLOT.bin] [--choices 12,34|none]
#             [--pages N] [--seconds S] [--display :N] [--shot NAME.png]
#
# GAME = the install folder (required). XTRACE_HOME (default out/xtrace in
# the repo) keeps the wine prefix between runs (prepared on the first run,
# about a minute) and the built DLL. --save copies a slot save (made with
# research/tools/xuse_save.py) to slot 1 and loads it from the title;
# without it the run starts with FIRST START. --pages stops after N
# pages (default 200), --seconds after S seconds (default 600). --shot
# saves the screen at the end. Records: research/xuse/tracedll/xtrace.c.
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
H=${XTRACE_HOME:-$repo/out/xtrace}
OUT= SAVE= CHOICES= PAGES=200 SECS=600 D=:71 SHOT=
while [ $# -gt 0 ]; do case $1 in
  -o) OUT=$2; shift 2;; --save) SAVE=$2; shift 2;; --choices) CHOICES=$2; shift 2;;
  --pages) PAGES=$2; shift 2;; --seconds) SECS=$2; shift 2;; --display) D=$2; shift 2;;
  --shot) SHOT=$2; shift 2;; *) sed -n '2,15p' "$0"; exit 2;; esac; done
[ -n "$OUT" ] && [ -d "$GAME/HD" ] || { echo "need -o and GAME=<install folder>" >&2; exit 2; }

export WINEPREFIX=$H/prefix WINEDEBUG=-all WINEARCH=win64 DISPLAY=$D WAYLAND_DISPLAY=
export LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8 PULSE_SERVER=unix:/nonexistent
export ALSA_CONFIG_PATH=$H/alsa-null.conf
export WINEDLLOVERRIDES="winepulse.drv,wineoss.drv,winewayland.drv=d;winemenubuilder.exe=d"
mkdir -p "$H/build"
echo 'pcm.!default { type null }' > "$ALSA_CONFIG_PATH"

for f in xtrace.c xlaunch.c; do  # rebuild when the source is newer
  b=$H/build/${f%.c}.$([ $f = xtrace.c ] && echo dll || echo exe)
  [ "$b" -nt "$here/$f" ] || i686-w64-mingw32-gcc -O2 -Wall $([ $f = xtrace.c ] && echo -shared) \
    -static-libgcc -o "$b" "$here/$f"
done

[ -e /tmp/.X11-unix/X${D#:} ] && { echo "display $D is taken: pass --display" >&2; exit 2; }
Xvfb $D -screen 0 1024x768x24 >/dev/null 2>&1 & xpid=$!; sleep 1
cleanup() { wineserver -k 2>/dev/null || true; kill $xpid 2>/dev/null || true; }
trap cleanup EXIT

C=$WINEPREFIX/drive_c
if [ ! -e "$H/prefix/.ready" ]; then
  wineboot -i >/dev/null 2>&1; wineserver -w
  for u in "$C"/users/*/; do for d in Documents Desktop Downloads Music Pictures Videos; do
    [ -L "$u$d" ] && rm "$u$d" && mkdir "$u$d"; done; done
  k='HKLM\Software\XUSE_CORP\Farthest2015'
  wine reg add "$k" /v Install /d 'C:\Xuse\ima\' /f /reg:32 >/dev/null 2>&1
  wine reg add "$k" /v HD /d 'C:\Xuse\ima\HD\' /f /reg:32 >/dev/null 2>&1
  wine reg add "$k" /v CD /d 'C:\Xuse\ima\CD\' /f /reg:32 >/dev/null 2>&1
  wine reg add 'HKCU\Software\Wine\Drivers' /v Audio /d alsa /f >/dev/null 2>&1
  wine reg add 'HKCU\Software\Wine\Drivers' /v Graphics /d x11 /f >/dev/null 2>&1
  wineserver -w
  touch "$H/prefix/.ready"
fi
mkdir -p "$C/Xuse/ima" "$C/xtrace"  # the install as a farm of links: nothing writes into it
for f in "$GAME"/*; do ln -sfn "$f" "$C/Xuse/ima/$(basename "$f")"; done
cp "$H/build/xtrace.dll" "$H/build/xlaunch.exe" "$C/xtrace/"
sd=$(echo "$C"/users/*/Documents)/Xuse/最果てのイマ\ COMPLETE
mkdir -p "$sd"
[ -n "$SAVE" ] && cp "$SAVE" "$sd/Farthest2015_001.bin"
rm -f "$C/xtrace/trace.jsonl"

export XTRACE_OUT='C:\xtrace\trace.jsonl' XTRACE_PAGES=$PAGES
[ -n "$CHOICES" ] && export XTRACE_CHOICES=$CHOICES
wine 'C:\xtrace\xlaunch.exe' 'C:\Xuse\ima\Farthest2015.exe' 'C:\xtrace\xtrace.dll' > "$H/wine.log" 2>&1 &
wpid=$!

click() { xdotool mousemove $(($1 + 3)) $(($2 + 29)) mousedown 1; sleep 0.15; xdotool mouseup 1; }
for i in $(seq 60); do xdotool search --onlyvisible --name . >/dev/null 2>&1 && break; sleep 1; done
T=$C/xtrace/trace.jsonl
sleep 5
# ENTER, clicks past the intro (none of them hits a title button), then FIRST
# START or load slot 1 (load, はい); again until the script runs (a cmd record)
for try in 1 2 3; do
  click 405 463; for i in 1 2 3 4 5 6; do sleep 3; click 400 300; done; sleep 3
  if [ -n "$SAVE" ]; then click 670 262; sleep 2; click 300 90; sleep 1; click 652 544; sleep 1; click 370 320
  else click 675 240; fi
  sleep 8; grep -q '"t":"cmd"' "$T" && break
done

end=$((SECONDS + SECS))
while kill -0 $wpid 2>/dev/null && [ $SECONDS -lt $end ]; do sleep 1; done
[ -n "$SHOT" ] && import -window root -crop 800x600+3+29 "$SHOT"
cp "$C/xtrace/trace.jsonl" "$OUT"
echo "$(grep -c '"t":"page"' "$OUT") pages, $(wc -l < "$OUT") records -> $OUT"
