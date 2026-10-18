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
# waits alone. IDLEA may be a fraction (checked every 0.2 s): 0.3 is the fast
# run (docs/new-game.md, "Trace speeds"). At a choice (_SELD) it moves the
# cursor down to the first option neither greyed nor listed in AVOID (string
# ids of the options' text, "042614 042606"), then presses A.
# Setup of the data dir (firmware, keys, update): docs/games/tsukihime.md, "Testing".
set -u
[ -n "${HEADLESS:-}" ] || exec "$(dirname "$0")/../../research/tools/headless-run.sh" "$0" "$@"
D=$(cd "$1" && pwd); T=$2; shift 2
L=$D/../logs; mkdir -p "$L" "$D/xdg"
N=$DISPLAY
timeout -k 10 "$T" "$(dirname "$0")/../tools/capped.sh" 8 \
  flatpak run --nosocket=wayland --nosocket=fallback-x11 --nosocket=pulseaudio --nosocket=session-bus --socket=x11 \
  --env=WAYLAND_DISPLAY= --env=DBUS_SESSION_BUS_ADDRESS=disabled: --env=DISPLAY=$N --env=XDG_SESSION_TYPE=x11 --env=SDL_AUDIODRIVER=dummy \
  --env=SDL_VIDEODRIVER=x11 --env=XDG_RUNTIME_DIR="$D/xdg" \
  --filesystem="$D" --command=/app/bin/Ryujinx io.github.ryubing.Ryujinx \
  --no-gui --root-data-dir "$D" "$@" >"$L/ryu.log" 2>&1 &
RPID=$!
# never the desktop: SDL finds a Wayland socket in XDG_RUNTIME_DIR even without
# WAYLAND_DISPLAY, hence the empty one above; stop unless the window is on ours
for i in $(seq 60); do xdotool search --name '^Ryujinx' >/dev/null 2>&1 && break; sleep 1; done
xdotool search --name '^Ryujinx' >/dev/null 2>&1 && echo "window on $N" || { echo "no window on $N: stopping"; kill $RPID; }
S=${SHOT:-15}; T0=$SECONDS; next=$S; ks=" ${KEYS:-} "; fps=${FPSAT:-}
idle=${IDLEA:+$(awk "BEGIN{print int($IDLEA*1000)}")}; prev=; since=0
while kill -0 $RPID 2>/dev/null; do
  sleep 0.2; t=$((SECONDS-T0)); now=$((${EPOCHREALTIME/./}/1000))   # wall-clock s since start; ms
  if [ $t -ge $next ]; then import -silent -window root -display $N "$L/shot$(printf %04d $t).png" 2>/dev/null; next=$((next+S)); fi
  for kv in $ks; do [ "${kv%%:*}" -le $t ] || continue; ks=${ks/ $kv / }
    # held 150 ms: the emulator polls the keyboard once a frame
    xdotool search --name '^Ryujinx' windowfocus keydown "${kv#*:}" sleep 0.15 keyup "${kv#*:}" 2>/dev/null; echo "${t}s key ${kv#*:}"; done
  if [ -n "$idle" ]; then
    last=$(grep -a 'TMT [0-9a-f]* [0-9a-f]* _' "$L/ryu.log" | tail -n 1); last=${last#*TMT }
    if [ "$last" != "$prev" ]; then prev=$last; since=$now
    elif [ $((now-since)) -ge "$idle" ] && [[ -z $last || $last =~ ^[0-9a-f]+\ [0-9a-f]+\ _(ZM|MSAD|TOMW|RTMN|END_) ]]; then
      xdotool search --name '^Ryujinx' windowfocus keydown z sleep 0.15 keyup z 2>/dev/null; since=$now
    elif [ $((now-since)) -ge "$idle" ] && [[ $last =~ ^[0-9a-f]+\ [0-9a-f]+\ _SELD ]]; then
      # a choice: Down to the first option that is neither greyed nor in AVOID, then A
      k=$(grep -a 'TMT [0-9a-f]* [0-9a-f]* _SEL[RD]' "$L/ryu.log" | sed 's/.*TMT //' | uniq | tail -n 20 | awk -v avoid=" ${AVOID:-} " '
        /_SELR\(/ { if (shown) { split("", o); shown = 0 }
                    match($0, /_SELR\([0-9]+,\$[0-9]+(,1)?/); split(substr($0, RSTART + 6, RLENGTH - 6), a, ",")
                    o[a[1]] = (a[3] == 1 ? "" : substr(a[2], 2)); next }
        /_SELD/ { shown = 1 }
        END { k = 0; for (i = 0; i < 16; i++) if ((i in o) && o[i] != "") { if (!index(avoid, " " o[i] " ")) break; k++ } print k }')
      for _ in $(seq "$k"); do xdotool search --name '^Ryujinx' windowfocus keydown Down sleep 0.15 keyup Down 2>/dev/null; sleep 0.1; done
      xdotool search --name '^Ryujinx' windowfocus keydown z sleep 0.15 keyup z 2>/dev/null; echo "${t}s choice: down $k"; since=$now; fi
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
