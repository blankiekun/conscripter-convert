#!/bin/bash
# switch-emu.sh <data dir> <secs> <Ryujinx args...>
# Runs the Ryubing flatpak's built-in headless front end (`--no-gui`) on its
# own Xvfb (never the desktop), with its own data dir, no audio, memory capped,
# and saves a root-window shot every $SHOT seconds into <data dir>/../logs.
# Setup of the data dir (firmware, keys, update): docs/games/tsukihime.md, "Testing".
set -u
D=$(cd "$1" && pwd); T=$2; shift 2
L=$D/../logs; mkdir -p "$L"
Xvfb -displayfd 3 -screen 0 1280x720x24 -nolisten tcp 3>"$L/disp" 2>"$L/xvfb.log" &
XPID=$!
for i in $(seq 100); do [ -s "$L/disp" ] && break; sleep 0.1; done
N=:$(cat "$L/disp"); echo "Xvfb $N pid $XPID"
unset XDG_SESSION_TYPE WAYLAND_DISPLAY
export WAYLAND_DISPLAY= DISPLAY=$N
timeout -k 10 "$T" systemd-run --user --scope -q -p MemoryMax=8G \
  flatpak run --nosocket=wayland --nosocket=pulseaudio --socket=x11 \
  --env=WAYLAND_DISPLAY= --env=DISPLAY=$N --env=XDG_SESSION_TYPE=x11 --env=SDL_AUDIODRIVER=dummy \
  --filesystem="$D" --command=/app/bin/Ryujinx io.github.ryubing.Ryujinx \
  --no-gui --root-data-dir "$D" "$@" >"$L/ryu.log" 2>&1 &
RPID=$!
k=0; while kill -0 $RPID 2>/dev/null; do sleep "${SHOT:-15}"; k=$((k+1)); import -silent -window root -display $N "$L/shot$(printf %03d $k).png" 2>/dev/null; done
wait $RPID; echo "exit $?"
pkill -x Ryujinx 2>/dev/null
kill $XPID; wait $XPID 2>/dev/null
