#!/usr/bin/env bash
# Self-check of capped.sh (needs the systemd --user bus and 3+ GiB free):
# exit codes pass through; a growing python is stopped by the watchdog at
# 90 % of 1 GiB, with no kernel OOM kill.
set -u
c=$(dirname "$0")/capped.sh
"$c" 1 sh -c 'exit 7'; [ $? = 7 ] || { echo "FAIL exit code"; exit 1; }
"$c" 1 true || { echo "FAIL true"; exit 1; }
"$c" 100000 true; [ $? = 75 ] || { echo "FAIL low-memory refusal"; exit 1; }
t0=$(date '+%Y-%m-%d %H:%M:%S')
out=$("$c" 1 python3 -c '
import time
a = []
for _ in range(1600):  # up to 1.6 GiB, 1 MiB each 5 ms
    a.append(bytearray(1 << 20)); time.sleep(0.005)' 2>&1); rc=$?
[ $rc = 137 ] || { echo "FAIL watchdog rc $rc: $out"; exit 1; }
[[ $out == *"capped.sh: stopped at"*"of 1 GiB"* ]] || { echo "FAIL message: $out"; exit 1; }
if journalctl -k --since "$t0" 2>/dev/null | grep -qi 'oom-kill\|out of memory'; then
  echo "FAIL kernel OOM kill"; exit 1
fi
echo ok
