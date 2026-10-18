#!/bin/bash
# capped.sh <GiB> <cmd> [args...]: run cmd with its memory capped, without a
# kernel OOM kill (each one pops a "killed" notification on the user's desktop).
# Own systemd --user scope: MemoryHigh=<GiB> (throttles), MemoryMax=<GiB>+1 (backstop).
# A watchdog reads the scope's memory.current every 2 s and at 90 % of <GiB>
# sends SIGTERM (SIGKILL 5 s later) and exits 137. Refuses to start (exit 75)
# when MemAvailable < <GiB>+2 GiB. Otherwise cmd's exit code is passed through.
# Works under headless-run.sh (uses HEADLESS_USER_RUNTIME for the user bus only;
# cmd gets the caller's XDG_RUNTIME_DIR and DBUS_SESSION_BUS_ADDRESS back).
# Self-test: research/tools/test_capped.sh
[ $# -ge 2 ] && [[ $1 =~ ^[0-9]+(\.[0-9]+)?$ ]] || { echo "usage: $0 <GiB> <cmd> [args...]" >&2; exit 2; }
G=$1; shift
LIM=$(awk -v g="$G" 'BEGIN{printf "%d", g*1024}')          # MiB
AVAIL=$(awk '/^MemAvailable:/{printf "%d", $2/1024}' /proc/meminfo)
if [ "$AVAIL" -lt $((LIM + 2048)) ]; then
  echo "capped.sh: only $AVAIL MiB available, need $G GiB + 2 GiB: wait a few minutes and retry" >&2; exit 75
fi
R=${HEADLESS_USER_RUNTIME:-${XDG_RUNTIME_DIR:-/run/user/$(id -u)}}
BUS=(env XDG_RUNTIME_DIR="$R" DBUS_SESSION_BUS_ADDRESS="unix:path=$R/bus")
BACK=(env)   # restore the caller's runtime dir and bus for cmd
for v in XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS; do
  if [ -n "${!v+x}" ]; then BACK+=("$v=${!v}"); else BACK+=(-u "$v"); fi
done
U=capped-$$-$RANDOM.scope
F=$(mktemp) || exit 1
trap 'rm -f "$F"' EXIT
"${BUS[@]}" systemd-run --user --scope -q --unit="$U" -p MemoryHigh="${LIM}M" -p MemoryMax="$((LIM + 1024))M" \
  "${BACK[@]}" "$@" &
P=$!
(
  STOP=$((LIM * 9 / 10)) CG=
  while kill -0 $P 2>/dev/null; do
    sleep 2
    [ -n "$CG" ] || CG=$("${BUS[@]}" systemctl --user show -P ControlGroup "$U" 2>/dev/null)
    [ -n "$CG" ] && [ -r "/sys/fs/cgroup$CG/memory.current" ] || continue
    M=$(( $(cat "/sys/fs/cgroup$CG/memory.current" 2>/dev/null || echo 0) / 1048576 ))
    if [ "$M" -ge "$STOP" ]; then
      echo "capped.sh: stopped at $M MiB of $G GiB (leak or too big)" >&2
      echo 1 >"$F"
      "${BUS[@]}" systemctl --user kill -s TERM "$U" 2>/dev/null
      for _ in 1 2 3 4 5; do kill -0 $P 2>/dev/null || exit 0; sleep 1; done
      "${BUS[@]}" systemctl --user kill -s KILL "$U" 2>/dev/null
      exit 0
    fi
  done
) &
W=$!
trap 'rm -f "$F"; kill $W 2>/dev/null; "${BUS[@]}" systemctl --user kill -s TERM "$U" 2>/dev/null' INT TERM
wait $P; rc=$?
wait $W 2>/dev/null
[ -s "$F" ] && rc=137
exit $rc
