#!/bin/bash
# ntrace.sh: play 装甲悪鬼村正 (Muramasa.exe) under wine with ntrace.dll and
# write its JSONL trace. Headless: own Xvfb (-displayfd), ja_JP locale,
# null audio. No clicks: a patched copy of nss.npa skips the logos and the
# title (0_boot.nss: $Logo, $TitleSelect=1, as research/nitroplus/local
# patch/game) and turns auto mode on as the text window unlocks it
# (function.nss line 1139, TypeBeginCIO; set while locked, as the
# boot's `$SYSTEM_text_auto` would be, it stays off); every patched line
# stays on its line, so the
# trace's lines are the shipped scripts' lines.
#
#   ntrace.sh -o trace.jsonl [--pages N] [--seconds S] [--shot NAME.png]
#
# GAME = the folder holding Muramasa.exe and the *.npa (required).
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
[ -n "$OUT" ] && [ -f "$GAME/Muramasa.exe" ] || { echo "need -o and GAME=<folder of Muramasa.exe>" >&2; exit 2; }

export WINEPREFIX=$H/prefix WINEDEBUG=-all WINEARCH=win64 WAYLAND_DISPLAY=
export LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8 PULSE_SERVER=unix:/nonexistent
export ALSA_CONFIG_PATH=$H/alsa-null.conf
export WINEDLLOVERRIDES="winepulse.drv,wineoss.drv,winewayland.drv=d;winemenubuilder.exe=d"
mkdir -p "$H/build"
printf 'pcm.!default { type null }\nctl.!default { type hw card 0 }\n' > "$ALSA_CONFIG_PATH"

i686-w64-mingw32-gcc -O2 -Wall -shared -static-libgcc -o "$H/build/ntrace.dll" "$here/ntrace.c"
i686-w64-mingw32-gcc -O2 -Wall -static-libgcc -o "$H/build/xlaunch.exe" "$repo/research/xuse/tracedll/xlaunch.c"

# the patched nss.npa (the game's archive is only read)
npa="python3 -I $repo/research/nitroplus/npa.py $GAME/Muramasa.exe $GAME/nss.npa"
x=$H/patch/x; rm -rf "$H/patch"; mkdir -p "$x" "$H/patch/nss"
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
M=$C/muramasa; rm -rf "$M"; mkdir -p "$M" "$C/ntrace"  # a farm of links: nothing writes into the game
for f in "$GAME"/*; do ln -s "$f" "$M/"; done
ln -sfn "$H/nss.npa" "$M/nss.npa"
cp "$H/build/ntrace.dll" "$H/build/xlaunch.exe" "$C/ntrace/"
rm -f "$C/ntrace/trace.jsonl"

export NTRACE_OUT='C:\ntrace\trace.jsonl' NTRACE_PAGES=$PAGES
wine 'C:\ntrace\xlaunch.exe' 'C:\muramasa\Muramasa.exe' 'C:\ntrace\ntrace.dll' > "$H/wine.log" 2>&1 &
wpid=$!
end=$((SECONDS + SECS))
while kill -0 $wpid 2>/dev/null && [ $SECONDS -lt $end ]; do sleep 1; done
[ -n "$SHOT" ] && import -window root "$SHOT"
cp "$C/ntrace/trace.jsonl" "$OUT"
echo "$(grep -c '"t":"page"' "$OUT") pages, $(wc -l < "$OUT") records -> $OUT"
