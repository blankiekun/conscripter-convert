#!/bin/bash
# headless-run.sh <cmd> [args...]: run a command isolated from the user's desktop.
# Own Xvfb (-displayfd, a free display), no Wayland/dbus/portals, xdg-open and
# gio shims, dummy audio (also for wine). Sets HEADLESS=1, DISPLAY=:N, HEADLESS_TMP.
[ $# -gt 0 ] || { echo "usage: $0 <cmd> [args...]" >&2; exit 2; }
T=$(mktemp -d) || exit 1
mkdir "$T/bin" "$T/run"; chmod 700 "$T/run"
printf '#!/bin/sh\nexit 0\n' > "$T/bin/xdg-open"; chmod +x "$T/bin/xdg-open"; cp "$T/bin/xdg-open" "$T/bin/gio"
Xvfb -displayfd 3 -screen 0 "${HEADLESS_SCREEN:-1280x800x24}" -nolisten tcp 3>"$T/disp" >/dev/null 2>&1 & XP=$!
trap 'kill $XP 2>/dev/null; rm -rf "$T"' EXIT
for _ in $(seq 100); do [ -s "$T/disp" ] && break; sleep 0.1; done
[ -s "$T/disp" ] || { echo "headless-run: Xvfb did not start" >&2; exit 1; }
unset WAYLAND_DISPLAY XDG_SESSION_TYPE
export DISPLAY=:$(cat "$T/disp") XDG_RUNTIME_DIR="$T/run" DBUS_SESSION_BUS_ADDRESS=disabled: BROWSER=true \
  PATH="$T/bin:$PATH" SDL_AUDIODRIVER=dummy HEADLESS=1 HEADLESS_TMP="$T"
export WINEDLLOVERRIDES="winealsa.drv,winepulse.drv,wineoss.drv,winewayland.drv=d${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
"$@"; rc=$?   # not exec: the trap must run
exit $rc
