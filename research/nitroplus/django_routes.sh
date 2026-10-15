#!/bin/sh
# 続・殺戮のジャンゴ: plays the good path and the ten bad ends in the engine,
# in parallel, and prints each trace's summary.
# Usage: django_routes.sh <game-cons> <engine> <out dir>
set -u
game=$(realpath "$1"); engine=$(realpath "$2"); out=$(realpath -m "$3")
here=$(dirname "$(realpath "$0")")
tools=$(realpath "$here/../../engine/tools")
python3 -I "$here/django_paths.py" "$game/0.txt" "$out" || exit 1
cd "$game" || exit 1
for p in good bad01 bad02 bad03 bad04 bad05 bad06 bad07 bad08 bad09 bad10; do
    rm -rf "$out/$p-saves"
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 1200 "$engine" --root . \
        --save-dir "$out/$p-saves" --trace "$out/$p.jsonl" --auto-play 0 \
        --choices "$out/$p.txt" > "$out/$p.log" 2>&1 &
done
wait
for p in good bad01 bad02 bad03 bad04 bad05 bad06 bad07 bad08 bad09 bad10; do
    printf '%s: %s; bad ends shown: %s\n' "$p" \
        "$(python3 -I "$tools/cons_trace_check.py" "$out/$p.jsonl" --script 0.txt 2>&1 | sed -n 1p | sed 's/.*jsonl: //')" \
        "$(grep -o 'bad[\\/]*title0[0-9]*' "$out/$p.jsonl" | sort -u | sed 's/.*title//' | tr '\n' ' ')"
done
