#!/usr/bin/env bash
# Playback check (nightly, after tests/snapshot.sh): per game, a headless
# auto-play of its first pages in --real-time (each moving sprite
# sampled, each effect timed), the trace checked by
# engine/tools/cons_trace_check.py, and its "jump" (a sprite stepping
# much faster than its motion, or off and back) and "duration" (an effect
# or motion off its asked time) counts held against
# tests/snapshots/<game>.playback, a ceiling per kind ("N  kind" as the
# .lint files; no file, no line: 0).
#
#   tests/playback.sh [--update] [--pages N] [game ...]
#
# Each game plays the files its dry run just wrote in out/snapshot/<game>
# (tests/snapshot.sh: run it first; a game without one is skipped) with
# no pictures or sounds: no full build is needed, and a sprite's motion
# and an effect's time are traced all the same (missing pictures are
# "missing" findings, not counted here). The font is the bundled IPA
# Gothic. It starts at *conv_title_start (--goto: the title's buttons
# have no picture to click), the story's start in every game.
# The engine is engine/build/conscripter (made here first), run as
# a renamed copy (other agents `pkill -x conscripter`), SDL dummy video
# and audio, in 3 GiB (research/tools/capped.sh) and $PLAYBACK_TIMEOUT s
# (default 400) per game; --pages N text pages (default 3000) or the end
# (all seven games: about 10 minutes, the engine under 100 MB). A game stops
# early where it must press a button that has no picture (Django: its
# first choice plate, page 559).
# Each run is left in out/playback/<game> (trace.jsonl, check.json).
#
# Exit 1 when a count rose above its ceiling or a run left no trace;
# a count below its ceiling prints "lower to N" and --update writes the
# lower counts (it never raises one: a new ceiling goes in by hand, with
# the reason in the PR). --self-test checks the ceiling compare.
set -uo pipefail
cd "$(dirname "$0")/.."
update=0 pages=3000 timeout=${PLAYBACK_TIMEOUT:-400}

# compare <ceiling file> <counts "N  kind" lines> <id>: prints ROSE/lower lines
compare() {
    awk -F'  ' -v id="$3" 'FILENAME == ARGV[1] { c[$2] = $1; next } { n[$2] = $1 }
        END { for (k in n) if (n[k] > c[k] + 0) print "ROSE " id ": " k " " c[k] + 0 " -> " n[k]
              for (k in c) if (n[k] + 0 < c[k]) print id ": " k " " c[k] " -> " n[k] + 0 ", lower to " n[k] + 0 }' \
        <(cat "$1" 2>/dev/null) <(echo -n "$2")
}

if [ "${1:-}" = --self-test ]; then
    t=$(mktemp); printf '3  jump\n2  duration\n' > "$t"
    r=$(compare "$t" $'4  jump\n1  duration\n' g | LC_ALL=C sort)
    rm -f "$t"
    [ "$r" = $'ROSE g: jump 3 -> 4\ng: duration 2 -> 1, lower to 1' ] || { echo "self-test FAILED: $r"; exit 1; }
    [ -z "$(compare /nonexistent '' g)" ] || { echo "self-test FAILED: empty"; exit 1; }
    echo "self-test ok"; exit 0
fi
while [ $# -gt 0 ]; do
    case $1 in
        --update) update=1 ;;
        --pages) pages=$2; shift ;;
        *) break ;;
    esac
    shift
done
ids=("$@")
# every game with a dry run
[ ${#ids[@]} -gt 0 ] || for d in out/snapshot/*/0.txt; do d=${d%/0.txt}; ids+=("${d##*/}"); done

engine=engine/build/conscripter
# configured once, then made each run (quick when the engine is unchanged)
L=$HOME/.local/sdl2_mixer
{ [ -f engine/build/Makefile ] || (cd engine && PKG_CONFIG_PATH=$L/lib/pkgconfig cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo \
    -DCMAKE_C_FLAGS=-I$L/include -DCMAKE_CXX_FLAGS=-I$L/include "-DCMAKE_EXE_LINKER_FLAGS=-L$L/lib -Wl,-rpath,$L/lib" > /dev/null 2>&1); } &&
    research/tools/capped.sh 4 make -C engine/build -j4 > /dev/null || { echo "engine build failed"; exit 1; }

failed=0
for id in "${ids[@]}"; do
    dry=out/snapshot/$id run=$PWD/out/playback/$id
    [ -f "$dry/0.txt" ] || { echo "$id: skipped (no $dry/0.txt: run tests/snapshot.sh $id)"; continue; }
    case $id in
        django|muramasa|muramasa-en) cfg=research/nitroplus ;;
        saihate-no-ima) cfg=research/xuse ;;
        tsukihime*) cfg=research/typemoon ;;
        *) cfg=research/$id ;;
    esac
    rm -rf "$run"; mkdir -p "$run/save"
    cp -r "$dry"/. "$run/game"
    cp crates/conv-cli/fonts/ipag.ttf "$run/game/default.ttf"
    cp "$engine" "$run/cons-playback"   # renamed: survives `pkill -x conscripter`
    cp "$cfg/cons_trace_check.json" "$run/game/" 2>/dev/null
    # the variables the checker's engine-var rule looks at (the config's "engine_vars")
    watch=$(python3 -I engine/tools/cons_trace_check.py --trace-vars-of "$cfg/cons_trace_check.json" 2>/dev/null)
    start=$(date +%s)
    # in the game folder: the engine opens default.ttf from there
    (cd "$run/game" && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy "$OLDPWD/research/tools/capped.sh" 3 timeout -k 5 "$timeout" \
        "$run/cons-playback" --root "$run/game" --save-dir "$run/save" --trace "$run/trace.jsonl" \
        --goto conv_title_start --auto-play "$pages" --real-time ${watch:+--trace-vars "$watch"} > "$run/stdout" 2>&1)
    status=$?
    secs=$(( $(date +%s) - start ))
    if [ ! -s "$run/trace.jsonl" ]; then
        echo "$id: no trace (exit $status, see $run/stdout)"; failed=1; continue
    fi
    python3 -I engine/tools/cons_trace_check.py "$run/trace.jsonl" --script "$run/game/0.txt" --config "$run/game/cons_trace_check.json" --json > "$run/check.json" 2> "$run/check.err"
    counts=$(python3 -Ic 'import json,sys; c=json.load(open(sys.argv[1]))["findings"]; print("\n".join("%d  %s" % (len(c[k]), k) for k in ("jump", "duration") if c.get(k)))' "$run/check.json")
    pg=$(python3 -Ic 'import json,sys; print(sum(r["pages"] for r in json.load(open(sys.argv[1]))["runs"]))' "$run/check.json")
    echo "$id: $pg pages in ${secs}s (exit $status): ${counts//$'\n'/, }"
    snap=tests/snapshots/$id.playback
    cmp=$(compare "$snap" "$counts" "$id")
    [ -z "$cmp" ] || echo "$cmp"
    if grep -q '^ROSE' <<< "$cmp"; then
        echo "$id: a playback count rose (python3 -I engine/tools/cons_trace_check.py $run/trace.jsonl --script $run/game/0.txt --max 5): fix it, or raise $snap by hand and say why in the PR"; failed=1
    elif [ $update = 1 ] && [ -n "$cmp" ]; then
        if [ -n "$counts" ]; then echo "$counts" > "$snap"; else rm -f "$snap"; fi; echo "$id: playback ceilings lowered"
    fi
done
exit $failed
