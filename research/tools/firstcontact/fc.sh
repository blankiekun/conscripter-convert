#!/bin/bash
# fc.sh: one logged first run of a new Windows game under wine, with
# fctrace.dll (fctrace.c has the records). Headless (research/tools/
# headless-run.sh: own Xvfb, no audio), ja_JP locale, a scratch prefix with
# real user folders, the install as a farm of links (nothing writes into it).
#
#   fc.sh <game dir> [-o DIR] [--exe NAME.exe] [--seconds S] [--advance MS]
#         [--click X,Y] [--key KEY] [--hooks FILE] [--shot-every N]
#         [--max-shots N] [--cap-mb MB] [--lossless] [--desktop WxH]
#         [--dll X.dll ...]
#
# -o: the run folder (default out/<game dir name>-first-contact; scratch,
#   never research/*/local). --seconds: stop after S (default 300).
# --advance: every MS (default 2000; 0: never) a click at X,Y (default the
#   screen's middle) and a press of KEY (default Return), to walk on.
# --hooks: breakpoints for the DLL (first_contact.py writes hooks.tsv).
# Screenshots: every N presents (default 120, about 2 s at 60 fps) the
#   game waits while the screen is saved as shots/<n>.webp, lossy (quality
#   80: for looking and coarse compares, RMSE or a fuzz; not AE 0), or
#   lossless with --lossless; one equal to the last kept is left out.
#   No more than --max-shots (default 500), and none once the run folder
#   passes --cap-mb (default 200): the log goes on, shots.log says so.
# --desktop: run in wine's virtual desktop of that size (games that draw
#   twice or misbehave full screen). --dll: also inject that DLL (an
#   engine-specific trace; its environment passes through), after fctrace.
# The folder gets trace.jsonl.gz, shots/, shots.log, wine.log and
# summary.txt (records per kind, folder size). FC_HOME (default
# out/first-contact in the repo) keeps the prefixes and the built DLLs.
set -e
[ -n "${HEADLESS:-}" ] || exec "$(dirname "$0")/../headless-run.sh" "$0" "$@"
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
H=${FC_HOME:-$repo/out/first-contact}
GAME= OUT= EXE= SECS=300 ADV=2000 CLICK= KEY=Return HOOKS= EVERY=120 MAXSHOTS=500 CAP=200 LOSSLESS= DESK= DLLS=()
while [ $# -gt 0 ]; do case $1 in
  -o) OUT=$2; shift 2;; --exe) EXE=$2; shift 2;; --seconds) SECS=$2; shift 2;; --advance) ADV=$2; shift 2;;
  --click) CLICK=$2; shift 2;; --key) KEY=$2; shift 2;; --hooks) HOOKS=$(realpath "$2"); shift 2;;
  --shot-every) EVERY=$2; shift 2;; --max-shots) MAXSHOTS=$2; shift 2;; --cap-mb) CAP=$2; shift 2;;
  --lossless) LOSSLESS=1; shift;; --desktop) DESK=$2; shift 2;; --dll) DLLS+=("$(realpath "$2")"); shift 2;;
  -*) sed -n '2,30p' "$0"; exit 2;; *) GAME=$(realpath "$1"); shift;; esac; done
[ -d "$GAME" ] || { sed -n '2,30p' "$0"; exit 2; }
OUT=$(realpath -m "${OUT:-$repo/out/$(basename "$GAME")-first-contact}")
# the exe: the biggest one that isn't an installer or a settings tool
[ -n "$EXE" ] || EXE=$(cd "$GAME" && ls -S -- *.exe *.EXE 2>/dev/null | grep -viE '^(unins|setup|config|install|launcher)' | head -1)
[ -f "$GAME/$EXE" ] || { echo "no exe in $GAME (--exe)" >&2; exit 2; }
bits=$(python3 -I -c 'import struct,sys; b=open(sys.argv[1],"rb").read(4096); o=struct.unpack_from("<I",b,0x3c)[0]; print(64 if struct.unpack_from("<H",b,o+4)[0]==0x8664 else 32)' "$GAME/$EXE")
cc=$([ $bits = 64 ] && echo x86_64 || echo i686)-w64-mingw32-gcc
mkdir -p "$H/build$bits" "$OUT/shots"
B=$H/build$bits
"$cc" -O2 -Wall -shared -static-libgcc -o "$B/fctrace.dll" "$here/fctrace.c" -lwinmm -lgdi32
"$cc" -O2 -Wall -static-libgcc -mwindows -o "$B/xlaunch.exe" "$repo/research/xuse/tracedll/xlaunch.c"

export WINEPREFIX=$H/prefix$bits WINEDEBUG=-all WINEARCH=win64
export LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8 PULSE_SERVER=unix:/nonexistent
export ALSA_CONFIG_PATH=$H/alsa-null.conf
export WINEDLLOVERRIDES="$WINEDLLOVERRIDES;winemenubuilder.exe=d"
printf 'pcm.!default { type null }\nctl.!default { type hw card 0 }\n' > "$ALSA_CONFIG_PATH"
cleanup() { wineserver -k 2>/dev/null || true; }
trap cleanup EXIT
C=$WINEPREFIX/drive_c
if [ ! -e "$WINEPREFIX/.ready" ]; then
  wineboot -i >/dev/null 2>&1; wineserver -w
  for u in "$C"/users/*/; do for d in "$u"*; do [ -L "$d" ] && rm "$d" && mkdir "$d"; done; done
  wine reg add 'HKCU\Software\Wine\Drivers' /v Audio /d alsa /f >/dev/null 2>&1
  wine reg add 'HKCU\Software\Wine\Drivers' /v Graphics /d x11 /f >/dev/null 2>&1
  wineserver -w
  touch "$WINEPREFIX/.ready"
fi
M=$C/game; rm -rf "$M"; mkdir -p "$M" "$C/fc"
for f in "$GAME"/*; do ln -s "$f" "$M/"; done
cp "$B/fctrace.dll" "$B/xlaunch.exe" "$C/fc/"
win=('C:\fc\fctrace.dll')
for i in "${!DLLS[@]}"; do cp "${DLLS[$i]}" "$C/fc/x$i.dll"; win+=("C:\\fc\\x$i.dll"); done
[ -n "$HOOKS" ] && cp "$HOOKS" "$C/fc/hooks.tsv" && export FC_HOOKS='C:\fc\hooks.tsv'
rm -f "$C/fc/trace.jsonl" "$C/fc/shot.req" "$OUT"/shots/*.webp "$OUT/shots.log"
export FC_OUT='C:\fc\trace.jsonl' FC_SHOT_EVERY=$EVERY FC_SHOT_REQ='C:\fc\shot.req'

launch=(wine)
[ -n "$DESK" ] && launch=(wine explorer "/desktop=fc,$DESK")
(cd "$M" && "${launch[@]}" 'C:\fc\xlaunch.exe' "C:\\game\\$EXE" "${win[@]}") > "$OUT/wine.log" 2>&1 &
wpid=$!
alive() { kill -0 $wpid 2>/dev/null; }

# screenshots, when the DLL asks (it waits for the request file to go)
REQ=$C/fc/shot.req
(last= kept=0 capped=
 while alive; do
   [ -f "$REQ" ] || { sleep 0.05; continue; }
   n=$(cat "$REQ" 2>/dev/null); n=${n//[^0-9]/}
   if [ -n "$capped" ] || [ $kept -ge $MAXSHOTS ]; then echo "$n skipped: ${capped:-$MAXSHOTS shots}" >> "$OUT/shots.log"
   elif [ $(du -sm "$OUT" | cut -f1) -ge $CAP ]; then capped="folder over $CAP MB"; echo "$n skipped: $capped" >> "$OUT/shots.log"
   else
     import -window root "$H/shot.miff" 2>/dev/null
     sig=$(identify -format '%#' "$H/shot.miff" 2>/dev/null)
     if [ "$sig" = "$last" ]; then echo "$n same" >> "$OUT/shots.log"
     else
       magick "$H/shot.miff" $([ -n "$LOSSLESS" ] && echo -define webp:lossless=true || echo -quality 80) "$OUT/shots/$n.webp"
       last=$sig kept=$((kept + 1)); echo "$n saved" >> "$OUT/shots.log"
     fi
   fi
   rm -f "$REQ"
 done) &

if [ "$ADV" -gt 0 ]; then
  (sleep 5; read -r W Hh < <(xdpyinfo | awk '/dimensions/{split($2,a,"x"); print a[1], a[2]}')
   XY=${CLICK:-$((W / 2)),$((Hh / 2))}
   while alive; do
     xdotool mousemove ${XY%,*} ${XY#*,} mousedown 1; sleep 0.1; xdotool mouseup 1; xdotool key "$KEY"
     sleep $(awk "BEGIN{print $ADV/1000}")
   done) >/dev/null 2>&1 &
fi

end=$((SECONDS + SECS))
while alive && [ $SECONDS -lt $end ]; do sleep 1; done
cleanup
sleep 1
gzip -c "$C/fc/trace.jsonl" > "$OUT/trace.jsonl.gz"
{ echo "exe: $EXE ($bits-bit), $SECS s cap, ran $SECONDS s"
  echo "records:"; zcat "$OUT/trace.jsonl.gz" | grep -o '"t":"[a-z]*"' | sort | uniq -c | sort -rn
  echo "shots: $(grep -c saved "$OUT/shots.log" 2>/dev/null) kept, $(grep -c same "$OUT/shots.log" 2>/dev/null) unchanged, $(grep -c skipped "$OUT/shots.log" 2>/dev/null) skipped"
  grep -m1 skipped "$OUT/shots.log" 2>/dev/null || true
  # each count breakpoint: hits, and how its per-second hits follow the presents (r near 1: per frame)
  zcat "$OUT/trace.jsonl.gz" | python3 -I -c '
import json, sys
import collections
recs = list(map(json.loads, sys.stdin))
secs = [r for r in recs if r["t"] == "sec"]
for t in ("present", "open", "dsbuf", "play"):  # the innermost exe call sites
    c = collections.Counter(r["stack"][0] for r in recs if r["t"] == t and r.get("stack"))
    if c: print(f"{t} called from (exe addresses): " + ", ".join(f"{a} x{n}" for a, n in c.most_common(3)))
p = [r.get("present", 0) for r in secs]
def corr(a, b):
    n = len(a); ma, mb = sum(a) / n, sum(b) / n
    va = sum((x - ma) ** 2 for x in a); vb = sum((y - mb) ** 2 for y in b)
    return sum((x - ma) * (y - mb) for x, y in zip(a, b)) / (va * vb) ** 0.5 if va and vb else 0
names = sorted({k for r in secs for k in r.get("hits", {})})
if names: print("breakpoints (hits, per present, r with presents):")
for k in names:
    h = [r.get("hits", {}).get(k, 0) for r in secs]
    print(f"  {k:50s} {sum(h):9d} {sum(h) / max(sum(p), 1):8.2f} {corr(h, p):6.2f}")'
  echo "folder: $(du -sh "$OUT" | cut -f1)"; } > "$OUT/summary.txt"
cat "$OUT/summary.txt"
