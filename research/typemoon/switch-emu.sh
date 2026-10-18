#!/bin/bash
# switch-emu.sh <data dir> <secs> <Ryujinx args...>
# Runs the Ryubing flatpak's built-in headless front end (`--no-gui`) on its
# own Xvfb (never the desktop), with its own data dir, no audio, memory capped,
# and saves a root-window shot every $SHOT seconds into <data dir>/../logs.
# KEYS="40:z 50:z" presses keys (xdotool) at those seconds (Ryubing's default
# keyboard: z = A, x = B, Return = +); FPSAT=<secs> grabs 5 s at 60 fps then and
# prints how many frames differing in any pixel were shown per second.
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
S=${SHOT:-15}; T0=$SECONDS; next=$S; ks=" ${KEYS:-} "; fps=${FPSAT:-}
while kill -0 $RPID 2>/dev/null; do
  sleep 1; t=$((SECONDS-T0))   # wall-clock seconds since start
  if [ $t -ge $next ]; then import -silent -window root -display $N "$L/shot$(printf %04d $t).png" 2>/dev/null; next=$((next+S)); fi
  for kv in $ks; do [ "${kv%%:*}" -le $t ] || continue; ks=${ks/ $kv / }
    # held 150 ms: the emulator polls the keyboard once a frame
    xdotool search --name '^Ryujinx' windowfocus keydown "${kv#*:}" sleep 0.15 keyup "${kv#*:}" 2>/dev/null; echo "${t}s key ${kv#*:}"; done
  if [ -n "$fps" ] && [ $t -ge $fps ]; then fps=
    ffmpeg -nostdin -loglevel info -f x11grab -framerate 60 -video_size 1280x720 -t 5 -i $N -vf mpdecimate=hi=1:lo=1:frac=0,showinfo -f null - 2>&1 | grep -c 'Parsed_showinfo.* n: *[0-9]' | awk '{printf "%ss: %.1f distinct frames/s\n", '$t', $1/5}'
  fi
done
wait $RPID; echo "exit $?"
flatpak kill io.github.ryubing.Ryujinx 2>/dev/null
kill $XPID; wait $XPID 2>/dev/null
