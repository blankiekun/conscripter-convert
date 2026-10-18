#!/bin/bash
# reftrace.sh <game> [--resume <save>] | status <game> | summary <game>
# A reference trace of the ORIGINAL game, started with one command (docs/new-game.md, "Reference traces").
# game: saihate muramasa muramasa-en django oretsuba tsukihime. Everything lands in out/<game>-reftrace/
# (git-ignored). Game folders come from the links in $CONS_GAMES (default ~/.config/conscripter-convert/games,
# as tests/snapshot.sh) -- saihate: saihate-no-ima -- or from the environment below; nothing local is hard-coded.
#   CD       per-machine route inputs (default out/reftrace): <mura|django>-choose.txt (a research/nitroplus/paths file: FILE:N lines),
#            saihate-route.arg, mura-patch/ django-patch/ (patched .nss), keys.sh (oretsuba)
#   FONT     a Japanese TTF for the dry run (default: fc-list's first ja font)
#   DJANGO_EXE  Django.exe of the 1.01 no-protection install      EMU  oretsuba: Vita3K setup dir (home/, v3k/, emu/logs)
#   SWITCH_DATA + SWITCH_NSP  tsukihime: Ryujinx data dir and base nsp       SECS  run length cap (default per game)
# Needs for muramasa/django/saihate the uncommitted-at-writing flags of ntrace.sh --patch and xtrace.sh --route.
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd); cd "$ROOT"
CD=${CD:-$ROOT/out/reftrace}; GAMES=${CONS_GAMES:-$HOME/.config/conscripter-convert/games}
cmd=${1:-}; case $cmd in status|summary) g=${2:-}; resume=;; *) g=$cmd; cmd=start; resume=; [ "${2:-}" = --resume ] && resume=${3:-};; esac
case $g in
 saihate) kind=xuse; link=saihate-no-ima; secs=129600;;
 muramasa) kind=nitro; link=muramasa; secs=28800; pre=mura;;
 django) kind=nitro; link=django; secs=21600; pre=django;;
 muramasa-en) kind=fc; link=muramasa-en; secs=7200;;
 oretsuba) kind=vita; link=oretsuba; secs=7200;;
 tsukihime) kind=switch; link=tsukihime; secs=7200;;
 *) sed -n '2,3p' "$0"; exit 2;;
esac
OUT=$ROOT/out/$g-reftrace; mkdir -p "$OUT"; T=$OUT/orig.jsonl; UNIT=reftrace-$g
SECS=${SECS:-$secs}
case $kind in vita) T=${EMU:-/nonexistent}/emu/logs/vita3k.log;; switch) T=${SWITCH_DATA:-/nonexistent}/../logs/ryu.log;; esac
pages() { { grep -a -c '"t":"page"' "$T" 2>/dev/null || grep -a -c 'TTY: @T ' "$T" 2>/dev/null; } | head -1; }  # emulators: text-command lines
age() { [ -f "$T" ] && echo $(( ($(date +%s) - $(stat -c %Y "$T")) / 60 )) || echo -; }
running() { systemctl --user is-active -q "$UNIT.scope" 2>/dev/null; }
last() { tail -c 30000 "$T" 2>/dev/null | grep -a -o '"at":"[^"]*"' | tail -1 | cut -c6-; }

case $cmd in
status)
  echo "$g: $(running && echo running || echo NOT running); pages $(pages); last ${T:+$(last)}"
  echo "minutes since progress: $(age); RAM available: $(free -g | awk '/Mem/{print $7}') GB"
  [ -f "$OUT/watchdog.log" ] && tail -n 2 "$OUT/watchdog.log"; exit 0;;
summary)
  echo "$g: $(running && echo still running || echo stopped); pages $(pages); last $(last)"
  if [ -f "$OUT/ours.map" ] && [ -f "$OUT/ours.jsonl" ] && [ -f "$T" ]; then
    ms=; [ $kind = nitro ] && ms="--ms --thread 0"
    python3 -I research/tools/trace_diff.py "$T" "$OUT/ours.jsonl" --map "$OUT/ours.map" $ms --max 400 2>&1 \
      | grep -a -o -E '^ *[a-z]+:|\b(text|missing|merged|quiet|time)\b' | sort | uniq -c | sort -rn | head -8 | sed 's/^/  diff kind /'
  else echo "  no ours.jsonl/ours.map to diff against (see $OUT/ours.log)"; fi | head -14
  exit 0;;
esac

running && { echo "$g: already running ($UNIT)"; exit 1; }
[ -f "$ROOT/engine/build/conscripter" ] && [ -x "$ROOT/target/release/conscripter-convert" ] || { echo "build first: cargo build --release; research/tools/build-engine.sh"; exit 1; }
GAME=${GAME:-$(readlink -f "$GAMES/$link")}; [ -d "$GAME" ] || { echo "no game folder: $GAMES/$link (or GAME=)"; exit 1; }
FONT=${FONT:-$(fc-list :lang=ja file | head -1 | sed 's/: *$//')}
SPD=; RUN=(); ENV=()
# --- ours: dry-run 0.txt, source map, and our trace on the same route (background, cheap)
ours() {
  local c=() r="$CD/${pre:-x}-choose.txt"
  "$ROOT/target/release/conscripter-convert" convert-game "$GAME" -o "$OUT/ours" --dry-run --source-map "$OUT/ours.map" --font "$FONT" --title "$g" >"$OUT/ours-build.log" 2>&1 || return 1
  case $kind in
   xuse) python3 -I research/xuse/route.py "$OUT/ours/0.txt" >"$OUT/ours.choices" && c=(--choices "$OUT/ours.choices");;
   nitro) [ -f "$r" ] && c=($(python3 -I research/tools/ours_choices.py "$OUT/ours/0.txt" "$OUT/ours.map" $(sed 's/[ \t].*//' "$r" | grep -E '^[A-Za-z0-9_]+:[0-9]+$') | tr '\n' ' '));;
  esac
  ( cd "$OUT/ours" && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 3600 "$ROOT/engine/build/conscripter" --root . \
      --save-dir "$OUT/ours-saves" --trace "$OUT/ours.jsonl" --auto-play 0 --real-time "${c[@]}" >"$OUT/ours.log" 2>&1 )
}
HR=("$ROOT/research/tools/headless-run.sh")
case $kind in
 xuse)
  SPD=4; ENV=(XTRACE_HOME="$OUT/home" GAME="$GAME" SPEED=4)
  RUN=(research/xuse/tracedll/xtrace.sh -o "$T" --route "$(cat "$CD/saihate-route.arg")" --pages 0 --seconds "$SECS" --no-cover ${resume:+--save "$resume"});;
 nitro)
  SPD=8; [ $g = django ] && ENV+=(EXE="${DJANGO_EXE:?set DJANGO_EXE to Django.exe}")
  ENV+=(NTRACE_HOME="$OUT/home" GAME="$GAME" SPEED=8)
  RUN=(research/nitroplus/tracedll/ntrace.sh -o "$T" --pages 0 --seconds "$SECS" --patch "$CD/$pre-patch");;
 fc) RUN=(research/tools/firstcontact/fc.sh "$GAME" -o "$OUT" --seconds "$SECS");;
 vita) ENV=(MEM=3G SHOT=300 V3K="$EMU/v3k")
  RUN=(bash -c "$CD/keys.sh $SECS $EMU/emu/logs/vita3k.log & research/oretsuba/vita-emu.sh $EMU/emu/home $SECS");;
 switch) ENV=(SHOT=300 IDLEA=0.3)
  RUN=(research/typemoon/switch-emu.sh "${SWITCH_DATA:?set SWITCH_DATA}" "$SECS" --graphics-backend Vulkan --skip-user-profiles-manager --disable-ptc "${SWITCH_NSP:?set SWITCH_NSP}");;
esac
[ $kind = nitro ] && [ ! -d "$CD/$pre-patch" ] && { echo "missing $CD/$pre-patch (force_choices.py output)"; exit 1; }
( ours ) &
# --- the run: own scope, capped memory, background; stdout/err in run.log
systemd-run --user --scope -q -u "$UNIT" -p MemoryMax=3G env "${ENV[@]}" "${HR[@]}" "${RUN[@]}" >"$OUT/run.log" 2>&1 &
# --- watchdog: 10 min without progress -> resume from the last save (xuse only; the others log the stall)
( while sleep 60; do
    running || break
    a=$(age); [ "$a" != - ] && [ "$a" -ge 10 ] || continue
    echo "$(date +%T) stalled ${a} min at $(pages) pages" >>"$OUT/watchdog.log"
    if [ $kind = xuse ]; then
      s=$(ls -t "$OUT"/home/prefix/drive_c/xtrace/*.bin 2>/dev/null | head -1); [ -n "$s" ] || continue
      systemctl --user stop "$UNIT.scope"; sleep 5; echo "$(date +%T) resume from $s" >>"$OUT/watchdog.log"
      exec setsid "$0" "$g" --resume "$s"
    fi
  done ) >/dev/null 2>&1 &
disown -a
echo "$g started: out/$g-reftrace/ (speed ${SPD:-1}); check: $0 status $g in ~5 min"
