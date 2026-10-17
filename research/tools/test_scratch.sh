#!/usr/bin/env bash
# Self-check of scratch.sh on a fake build: run it after changing scratch.sh.
set -euo pipefail
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
g=$t/game; s=$t/scratch
mkdir -p "$g/movie" "$s"
for f in arc.nsa arc1.nsa default.ttf 0.txt kidoku.dat envdata gloval.sav save1.dat; do echo "$f" > "$g/$f"; done
echo --enc:sjis > "$g/ons_args"; echo mine > "$t/my.txt"; cp /bin/true "$t/eng"
ln -s "$g/0.txt" "$s/0.txt"; ln -s "$g/envdata" "$s/envdata"   # a bad earlier set-up
"$(dirname "$0")/scratch.sh" "$g" "$s" "$t/my.txt" "$t/eng" > /dev/null
links=$(find "$s" -maxdepth 1 -type l -printf '%f\n' | LC_ALL=C sort | tr '\n' ' ')
[ "$links" = "arc.nsa arc1.nsa default.ttf movie " ] || { echo "FAIL links: $links"; exit 1; }
[ "$(cat "$g/0.txt")" = 0.txt ] && [ "$(cat "$s/0.txt")" = mine ] || { echo "FAIL 0.txt"; exit 1; }
[ -x "$s/cons-scratch" ] && [ ! -L "$s/ons_args" ] && [ -d "$s/save" ] || { echo "FAIL copies"; exit 1; }
"$s/run.sh" || { echo "FAIL run.sh"; exit 1; }
if "$(dirname "$0")/scratch.sh" "$g" "$g/sub" "$t/my.txt" "$t/eng" > /dev/null 2>&1; then echo "FAIL inside"; exit 1; fi
[ ! -e "$g/sub" ] || { echo "FAIL made a dir in the build"; exit 1; }
echo ok
