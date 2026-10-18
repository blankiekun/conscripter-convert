#!/bin/bash
# switch-emu.sh <data dir> <secs> <Ryujinx args...>
# Runs the Ryubing flatpak's built-in headless front end (`--no-gui`) on its
# own Xvfb (never the desktop), with its own data dir, no audio, memory capped,
# and saves a root-window shot every $SHOT seconds into <data dir>/../logs.
# KEYS="40:z 50:z" presses keys (xdotool) at those seconds (Ryubing's default
# keyboard: z = A, x = B, Return = +); FPSAT=<secs> grabs 5 s at 60 fps then and
# prints how many frames differing in any pixel were shown per second.
# IDLEA=<secs> presses A whenever the last command the trace hook logged
# (switchtrace/, "TMT ...") has stayed the same that long and is a text
# (_ZM, _MSAD), the opening's wait (_TOMW), a script's end (_RTMN, _END_: the
# title follows) or none is logged yet: the trace's auto mode, leaving timed
# waits alone.
# Setup of the data dir (firmware, keys, update): docs/games/tsukihime.md, "Testing".
set -u
D=$(cd "$1" && pwd); T=$2; shift 2
L=$D/../logs; mkdir -p "$L" "$D/xdg"
Xvfb -displayfd 3 -screen 0 1280x720x24 -nolisten tcp 3>"$L/disp" 2>"$L/xvfb.log" &
XPID=$!
for i in $(seq 100); do [ -s "$L/disp" ] && break; sleep 0.1; done
N=:$(cat "$L/disp"); echo "Xvfb $N pid $XPID"
unset XDG_SESSION_TYPE WAYLAND_DISPLAY
export WAYLAND_DISPLAY= DISPLAY=$N
timeout -k 10 "$T" systemd-run --user --scope -q -p MemoryMax=8G \
  flatpak run --nosocket=wayland --nosocket=fallback-x11 --nosocket=pulseaudio --socket=x11 \
  --env=WAYLAND_DISPLAY= --env=DISPLAY=$N --env=XDG_SESSION_TYPE=x11 --env=SDL_AUDIODRIVER=dummy \
  --env=SDL_VIDEODRIVER=x11 --env=XDG_RUNTIME_DIR="$D/xdg" \
  --filesystem="$D" --command=/app/bin/Ryujinx io.github.ryubing.Ryujinx \
  --no-gui --root-data-dir "$D" "$@" >"$L/ryu.log" 2>&1 &
RPID=$!
# never the desktop: SDL finds a Wayland socket in XDG_RUNTIME_DIR even without
# WAYLAND_DISPLAY, hence the empty one above; stop unless the window is on ours
for i in $(seq 60); do xdotool search --name '^Ryujinx' >/dev/null 2>&1 && break; sleep 1; done
xdotool search --name '^Ryujinx' >/dev/null 2>&1 && echo "window on $N" || { echo "no window on $N: stopping"; kill $RPID; }
S=${SHOT:-15}; T0=$SECONDS; next=$S; ks=" ${KEYS:-} "; fps=${FPSAT:-}
idle=${IDLEA:-}; prev=; since=0
while kill -0 $RPID 2>/dev/null; do
  sleep 1; t=$((SECONDS-T0))   # wall-clock seconds since start
  if [ $t -ge $next ]; then import -silent -window root -display $N "$L/shot$(printf %04d $t).png" 2>/dev/null; next=$((next+S)); fi
  for kv in $ks; do [ "${kv%%:*}" -le $t ] || continue; ks=${ks/ $kv / }
    # held 150 ms: the emulator polls the keyboard once a frame
    xdotool search --name '^Ryujinx' windowfocus keydown "${kv#*:}" sleep 0.15 keyup "${kv#*:}" 2>/dev/null; echo "${t}s key ${kv#*:}"; done
  if [ -n "$idle" ]; then
    last=$(grep -a 'TMT [0-9a-f]* [0-9a-f]* _' "$L/ryu.log" | tail -n 1); last=${last#*TMT }
    if [ "$last" != "$prev" ]; then prev=$last; since=$t
    elif [ $((t-since)) -ge "$idle" ] && [[ -z $last || $last =~ ^[0-9a-f]+\ [0-9a-f]+\ _(ZM|MSAD|TOMW|RTMN|END_) ]]; then
      xdotool search --name '^Ryujinx' windowfocus keydown z sleep 0.15 keyup z 2>/dev/null; since=$t; fi
  fi
  if [ -n "$fps" ] && [ $t -ge $fps ]; then fps=
    ffmpeg -nostdin -loglevel info -f x11grab -framerate 60 -video_size 1280x720 -t 5 -i $N -vf mpdecimate=hi=1:lo=1:frac=0,showinfo -f null - 2>&1 | grep -c 'Parsed_showinfo.* n: *[0-9]' | awk '{printf "%ss: %.1f distinct frames/s\n", '$t', $1/5}'
  fi
done
wait $RPID; echo "exit $?"
# flatpak moves the emulator into a scope of its own: stop only the one on our display
# (it outlives SIGTERM: SIGKILL after 10 s)
mine=$(for p in $(pgrep -x Ryujinx); do grep -qxz "DISPLAY=$N" /proc/$p/environ 2>/dev/null && echo $p; done)
if [ -n "$mine" ]; then kill $mine; for i in $(seq 10); do kill -0 $mine 2>/dev/null || break; sleep 1; done; kill -9 $mine 2>/dev/null; fi
kill $XPID; wait $XPID 2>/dev/null
