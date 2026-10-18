#!/bin/bash
# vita-emu.sh <home dir> <secs> [Vita3K args...]
# Runs Vita3K (the AppImage, extracted) with `-r PCSG00299` on its own Xvfb
# (never the desktop), with its own XDG dirs under <home dir> (config, ux0, caches), no audio, memory
# capped, and saves a root-window shot every $SHOT seconds into <home dir>/../logs.
# V3K=<extracted AppImage dir> (default: <home dir>/../AppDir).
# KEYS="30:c 34:c" presses keys (xdotool) at those seconds (Vita3K's default
# keyboard: c = circle, this Japanese game's OK; x = cross; Return = start);
# FPSAT=<secs> grabs the game's 960x544 for 5 s at 60 fps then and prints how many frames differing in any pixel were shown per second.
# WRAP="gdb -batch -x <cmds> --args" runs Vita3K under a wrapper (V3K=<dir> whose bin/
# is a source build: no share/ or lib/ needed).
# config.yml: show-welcome false, check-for-updates-mode 0, discord-rich-presence false.
# Setup of the home dir (app, license, config): docs/games/oretsuba.md, "Testing".
set -u; shopt -s nullglob
H=$(cd "$1" && pwd); T=$2; shift 2
V=${V3K:-$H/../AppDir}; V=$(cd "$V" && pwd)
L=$H/../logs; mkdir -p "$L"
# SDL falls back to the desktop's wayland-0 in $XDG_RUNTIME_DIR even with
# WAYLAND_DISPLAY unset: give it an empty runtime dir and force X11.
mkdir -p -m 700 "$H/run"
# Nothing may reach the user's session: no D-Bus (Qt's openUrl went through it
# and opened a page on the desktop, 2026-10-18), and xdg-open/gio as no-ops.
B=$H/../nourl; mkdir -p "$B"
for c in xdg-open gio kde-open gnome-open; do printf '#!/bin/sh\necho "$0 $*" >>"%s/calls.log"\n' "$B" >"$B/$c"; chmod +x "$B/$c"; done
# The AppImage's AppRun links /tmp/17-Tt (its share/) and /tmp/8_P (its lib/);
# we start bin/Vita3K directly, so link them here, with the host's fonts (the
# save dialog draws with DejaVuSans when there is no firmware font package).
S=$H/../share-shim; mkdir -p "$S"
for f in "$V"/share/*; do [ "${f##*/}" = fonts ] || ln -sfn "$f" "$S/"; done
ln -sfn /usr/share/fonts "$S/fonts"; ln -sfn "$S" /tmp/17-Tt; ln -sfn "$V/lib" /tmp/8_P
for f in "$V"/share/Vita3K/*; do ln -sfn "../share/Vita3K/${f##*/}" "$V/bin/"; done
Xvfb -displayfd 3 -screen 0 1280x720x24 -nolisten tcp 3>"$L/disp" 2>"$L/xvfb.log" &
XPID=$!
for i in $(seq 100); do [ -s "$L/disp" ] && break; sleep 0.1; done
N=:$(cat "$L/disp"); echo "Xvfb $N pid $XPID"
timeout -k 10 "$T" systemd-run --user --scope -q -p MemoryMax=6G \
  env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE -u DBUS_SESSION_BUS_ADDRESS DBUS_SESSION_BUS_ADDRESS=disabled: \
  BROWSER=true PATH="$B:$PATH" DISPLAY=$N HOME="$H" XDG_RUNTIME_DIR="$H/run" \
  SDL_VIDEO_DRIVER=x11 SDL_VIDEODRIVER=x11 QT_QPA_PLATFORM=xcb \
  XDG_DATA_HOME="$H/data" XDG_CONFIG_HOME="$H/config" XDG_CACHE_HOME="$H/cache" \
  SDL_AUDIODRIVER=dummy ${WRAP:-} "$V/bin/Vita3K" ${APP--r PCSG00299} "$@" >"$L/vita3k.log" 2>&1 &
RPID=$!
S=${SHOT:-15}; T0=$SECONDS; next=$S; ks=" ${KEYS:-} "; fps=${FPSAT:-}
while kill -0 $RPID 2>/dev/null; do
  sleep 1; t=$((SECONDS-T0))   # wall-clock seconds since start
  if [ $t -ge $next ]; then import -silent -window root -display $N "$L/shot$(printf %04d $t).png" 2>/dev/null; next=$((next+S)); fi
  for kv in $ks; do [ "${kv%%:*}" -le $t ] || continue; ks=${ks/ $kv / }
    # held 150 ms: the emulator polls the keyboard once a frame
    # the game's window: the last Vita3K one (the Qt list window comes first);
    # Vita3K's `--frontend sdl` ignores xdotool's keys, the Qt one takes them
    W=$(DISPLAY=$N xdotool search --onlyvisible --class Vita3K | tail -1)
    DISPLAY=$N xdotool windowfocus $W keydown "${kv#*:}" sleep 0.15 keyup "${kv#*:}"; echo "${t}s key ${kv#*:}"; done
  if [ -n "$fps" ] && [ $t -ge $fps ]; then fps=
    ffmpeg -nostdin -loglevel info -f x11grab -framerate 60 -video_size 960x544 -t 5 -i $N+0,0 -vf mpdecimate=hi=1:lo=1:frac=0,showinfo -f null - 2>&1 | grep -c 'Parsed_showinfo.* n: *[0-9]' | awk '{printf "%ss: %.1f distinct frames/s\n", '$t', $1/5}'
  fi
done
wait $RPID; echo "exit $?"
kill $XPID; wait $XPID 2>/dev/null
