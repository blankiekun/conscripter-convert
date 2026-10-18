#!/usr/bin/env bash
# Dry-runs every game whose files are on this machine and compares its
# 0.txt with tests/snapshots/<game>.txt (sha256 and line count), so a
# change to shared code shows which games' scripts it changed.
#
#   tests/snapshot.sh [--update] [--accept-gaps] [--no-cache] [--diff REF] [--map RULES] [game ...]
#
# Games are found through $CONS_GAMES (default ~/.config/conscripter-convert/games),
# a folder of links named by game id to each game's folder (<game> in
# its document). The ids and each dry run's arguments come from the
# registry (`conscripter-convert games --json`, crates/conv-engines/src/registry.rs):
# saihate-no-ima, muramasa, muramasa-en (the English
# release; needs muramasa too), django, oretsuba, tsukihime (the RomFS of
# 1.0.2, its ExeFS beside it as ../exefs). A game without its
# link is skipped. --update writes the snapshots of the games run.
# Each 0.txt's commands (text filled in) are checked against the engine's command table
# (research/tools/cmdcheck.py): an unknown one is printed.
# The snapshot also holds a sha256 per other file the dry run writes
# (screens/*.lua, ons_args, …; not metrics.json or version.txt), so a
# change to a shared menu shows which game's screen it changed.
#
# The dry run's lint findings (build.rs summary: labels, strings, files
# the script names that the build wouldn't write, …) are counted per kind
# against <game>.lint, a ceiling per kind ("N  kind"; no file, no line:
# 0). A count above its ceiling fails; one below prints "lower to N", and
# --update writes the lower counts (it never raises one: a new ceiling
# goes in by hand, with the reason in the PR).
#
# Exit status 1 when a snapshot differs, a command is unknown, or a lint
# count rose; 2 when a game's gaps or left_out total rose against its
# metrics snapshot (each printed per game), unless --accept-gaps.
#
# How often each command is used (cmdcheck.py --counts) is kept as
# <game>.cmds.json: a command whose count moved over 5 %, appeared or went
# is printed and exits 2 like gaps (a feature silently gone: 2 dwave
# against 36,055), unless --accept-gaps; --update rewrites it. An edition
# pair (muramasa and muramasa-en) prints the commands whose count ratio is
# over 20 % off the pair's median ratio (informative: the editions differ
# on purpose in places).
#
# Each game's completion counts (the dry run's metrics.json: source
# calls fully converted or with gaps) are kept as <game>.metrics.json
# the same way, compared without their commit lines; --update rewrites
# one only when its counts changed, naming the commit on origin/main
# they were measured on top of (the merge base: a branch's own commits
# are gone once it is squash-merged).
#
# --diff REF also builds REF (a commit, branch or tag; origin/main for a
# fresh base) in its own worktree out/snapshot-ref with its own target
# dir, dry-runs each game with it and shows what changed in 0.txt (its
# text filled in from its table: `conscripter-convert fill`) with
# research/tools/txtdiff.py (hunks under their labels, changed lines
# counted by command). --map RULES (a file of s/old/new/ lines, or one
# such rule) checks that the change is only a rename: the rules map
# REF's commands to the new ones and nothing else may differ; it
# implies --diff origin/main.
set -euo pipefail
cd "$(dirname "$0")/.."
games_dir=${CONS_GAMES:-$HOME/.config/conscripter-convert/games}
bin=target/release/conscripter-convert
update=0 accept=0 rose=0 ref= map=() cache=1
while [ $# -gt 0 ]; do
    case $1 in
        --update) update=1 ;;
        --accept-gaps) accept=1 ;;
        --no-cache) cache=0 ;;
        --diff) ref=$2; shift ;;
        --map) if [ -f "$2" ]; then map=(--rules "$2"); else map=(--map "$2"); fi; shift ;;
        *) break ;;
    esac
    shift
done
[ ${#map[@]} = 0 ] || [ -n "$ref" ] || ref=origin/main
ids=("$@")

cargo build --release -q
# the games and their dry-run arguments, from the registry: id, the id of
# the install the command reads, then the command and options (tab-separated)
declare -A row
all=()
while IFS= read -r line; do all+=("${line%%$'\t'*}"); row[${all[-1]}]=$line; done < <("$bin" games --json | python3 -Ic '
import json, sys
for g in json.load(sys.stdin):
    if g["japanese"]:  # an English edition, built on the Japanese game
        print("\t".join([g["id"], g["japanese"], *g["command"], "{}", *g["options"], "--english", "{en}"]))
    else:
        print("\t".join([g["id"], g["id"], *g["command"], "{}", *g["options"]]))')
[ ${#ids[@]} -gt 0 ] || ids=("${all[@]}")
if [ -n "$ref" ]; then
    sha=$(git rev-parse --verify "$ref^{commit}")
    refdir=out/snapshot-ref
    [ -e $refdir/.git ] || git worktree add -q --detach $refdir "$sha"
    git -C $refdir checkout -q --detach "$sha"
    git -C $refdir submodule update --init -q engine
    CARGO_TARGET_DIR=$PWD/$refdir/target cargo build --release -q --manifest-path $refdir/Cargo.toml
    echo "comparing with $ref ($(git rev-parse --short "$sha"))"
fi
base=$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD)
base_line="\"commit\": \"$(git rev-parse --short "$base")\","
base_date="\"commit_date\": \"$(git log -1 --format=%cs "$base")\","
mkdir -p tests/snapshots
changed=0
# A dry run is reused when its key (the binary, the arguments, where the
# game links lead, the engine's command table and this script) matches
# the one its last good run left in out/snapshot/<game>.key; --no-cache
# reruns them all. shortcut: an edited game install behind an unchanged
# link is not seen, rerun with --no-cache after changing one.
key_of() { # binary, then the dry run's arguments
    { sha256sum < "$1"; printf '%s\n' "${@:2}"; readlink -f "$game" "$english"
      cat engine/src/cons/ONScripter_lut.cpp tests/snapshot.sh | sha256sum; } | sha256sum | cut -d' ' -f1
}
dry() { # binary, out dir, key, then the arguments
    [ $cache = 1 ] && [ -d "$2" ] && [ "$(cat "$2.key" 2>/dev/null)" = "$3" ] && return
    rm -rf "$2" "$2.key" "$2.map" # files from an older run would be hashed
    # the source map beside the out dir (not hashed), for pr-report's motion_drop.py; not every command has one
    local sm=(); "$1" "$4" --help 2>/dev/null | grep -q -- --source-map && sm=(--source-map "$2.map")
    # its peak memory (KB) goes in <out>.peak, for the next run's scheduling
    local time=(); [ -x /usr/bin/time ] && time=(/usr/bin/time -o "$2.peak.new" -f %M)
    "${time[@]}" "$1" "${@:4}" "${sm[@]}" -o "$2" --dry-run > "$2.log" 2>&1 && echo "$3" > "$2.key" || true
    [ ! -s "$2.peak.new" ] || mv "$2.peak.new" "$2.peak"
}
# Starts a game's job only when MemAvailable, less the peaks of the jobs
# running, leaves its own last peak (3 GB when unknown) plus 2 GB, and at
# most $SNAPSHOT_JOBS (2) at once; alone it always starts (bash_guards
# refuses a run below 6 GiB). Peaks measured 2026-10-18: tsukihime 2.5 GB,
# muramasa and muramasa-en 1.6 GB each, oretsuba 0.6, the rest under 0.4.
declare -A running
start() { # peak (KB), then the command
    local need=$(( $1 + 2097152 )) used n p avail
    while :; do
        used=0 n=0
        for p in "${!running[@]}"; do
            if kill -0 "$p" 2>/dev/null; then used=$(( used + running[$p] )) n=$(( n + 1 )); else unset "running[$p]"; fi
        done
        avail=$(awk '/^MemAvailable:/ { print $2 }' /proc/meminfo)
        [ $n = 0 ] && break
        [ $n -lt "${SNAPSHOT_JOBS:-2}" ] && [ $(( avail - used )) -ge $need ] && break
        wait -n || true
    done
    "${@:2}" &
    running[$!]=$1
}
# The dry runs run side by side (start); their reports follow in game order.
declare -A key rkey skip
job() { # id, then the arguments
    dry "$bin" "out/snapshot/$1" "${key[$1]}" "${@:2}"
    [ -z "$ref" ] || dry $refdir/target/release/conscripter-convert "$refdir/out/$1" "${rkey[$1]}" "${@:2}"
}
# A dry run's 0.txt with its text filled in from its table, as the engine
# reads it (--diff compares these): <out>.filled.
filled() { if ls "$1"/text.*.txt >/dev/null 2>&1; then "$bin" fill "$1" > "$1.filled"; else cp "$1/0.txt" "$1.filled"; fi; }
reused=()
mkdir -p out/snapshot
[ -z "$ref" ] || mkdir -p $refdir/out
for id in "${ids[@]}"; do
    [ -n "${row[$id]:-}" ] || { echo "$id: unknown game"; exit 2; }
    IFS=$'\t' read -ra f <<< "${row[$id]}"
    game=$games_dir/${f[1]} english=$games_dir/$id
    if [ ! -e "$game" ] || { [ "${f[1]}" != "$id" ] && [ ! -e "$english" ]; }; then
        [ -e "$game" ] && game=$english
        skip[$id]="$id: skipped (no $game)"
        continue
    fi
    args=()
    for a in "${f[@]:2}"; do
        case $a in "{}") args+=("$game") ;; "{en}") args+=("$english") ;; *) args+=("$a") ;; esac
    done
    out=out/snapshot/$id
    key[$id]=$(key_of "$bin" "${args[@]}")
    hit=0; [ $cache = 1 ] && [ -d "$out" ] && [ "$(cat "$out.key" 2>/dev/null)" = "${key[$id]}" ] && { hit=1; reused+=("$id"); }
    [ -z "$ref" ] || rkey[$id]=$(key_of $refdir/target/release/conscripter-convert "${args[@]}")
    peak=$(cat "$out.peak" 2>/dev/null || echo 3145728)
    [ -z "$ref" ] && [ $hit = 1 ] && peak=0
    start "$peak" job "$id" "${args[@]}"
done
wait
[ ${#reused[@]} = 0 ] || echo "dry runs reused (same binary and arguments; --no-cache reruns): ${reused[*]}"
for id in "${ids[@]}"; do
    [ -z "${skip[$id]:-}" ] || { echo "${skip[$id]}"; continue; }
    out=out/snapshot/$id
    if [ "$(cat "$out.key" 2>/dev/null)" != "${key[$id]}" ]; then
        echo "$id: dry run failed, see $out.log"; changed=1; continue
    fi
    # every command it uses is in the engine's command table (the submodule's)
    filled "$out"
    python3 -I research/tools/cmdcheck.py engine/src/cons/ONScripter_lut.cpp "$out.filled" --counts "$out.cmds.json" || changed=1
    csnap=tests/snapshots/$id.cmds.json
    if [ $update = 1 ]; then
        cp "$out.cmds.json" "$csnap"
    elif [ ! -f "$csnap" ]; then
        echo "$id: no command counts snapshot (run with --update)"; changed=1
    elif ! moved=$(python3 -I research/tools/cmdcheck.py --compare "$csnap" "$out.cmds.json"); then
        echo "$id: command counts moved over 5 % (diff $csnap $out.cmds.json):"; echo "$moved" | sed 's/^/    /'
        [ $accept = 1 ] || rose=1
    fi
    if [ -n "$ref" ]; then
        rout=$refdir/out/$id
        if [ "$(cat "$rout.key" 2>/dev/null)" != "${rkey[$id]}" ]; then
            echo "$id: dry run at $ref failed, see $rout.log"; changed=1
        elif filled "$rout" && python3 -I research/tools/txtdiff.py "$rout.filled" "$out.filled" "${map[@]}" --hunks 5 --top 10 > "$out.diff"; then
            echo "$id: 0.txt $(cat "$out.diff") (vs $ref)"
        else
            echo "$id: 0.txt differs from $ref ($rout.filled; full report: python3 -I research/tools/txtdiff.py $rout.filled $out.filled):"
            sed 's/^/    /' "$out.diff"; changed=1
        fi
    fi
    # The version line (build date, commit) changes with every build, and
    # so does the width of its strsp (a -dirty tree's line is longer).
    ver='s/build [0-9]{4}-[0-9]{2}-[0-9]{2}, converter [0-9a-f]+(-dirty)?, CONscripter [^"]*/build VERSION/;s/(build VERSION.*cols=)[0-9]+/\1W/'
    now="$(LC_ALL=C sed -E -e 's/"build [0-9]{4}-[0-9]{2}-[0-9]{2}, converter [^"]*",([0-9]+,[0-9]+),[0-9]+,/"build VERSION",\1,W,/' \
        -e "$ver" "$out/0.txt" | sha256sum | cut -d' ' -f1) $(wc -l < "$out/0.txt")"
    others=$(cd "$out" && find . -type f ! -name 0.txt ! -name metrics.json ! -name version.txt | LC_ALL=C sort | while read -r f; do
        echo "$(LC_ALL=C sed -E "$ver" "$f" | sha256sum | cut -d' ' -f1)  ${f#./}"; done)
    [ -z "$others" ] || now="$now
$others"
    snap=tests/snapshots/$id.txt
    if [ $update = 1 ]; then
        echo "$now" > "$snap"; echo "$id: written"
    elif [ ! -f "$snap" ]; then
        echo "$id: no snapshot (run with --update)"; changed=1
    elif [ "$(cat "$snap")" = "$now" ]; then
        echo "$id: same"
    else
        [ "$(head -n1 "$snap")" = "$(head -n1 <<< "$now")" ] ||
            echo "$id: CHANGED (was $(head -n1 "$snap"), now $(head -n1 <<< "$now"); script in $out/0.txt)"
        for f in $(diff <(tail -n +2 "$snap") <(tail -n +2 <<< "$now") | sed -n 's/^[<>] [0-9a-f]*  //p' | sort -u); do
            echo "$id: $f CHANGED ($out/$f)"
            [ -z "$ref" ] || [ ! -f "$rout/$f" ] || { diff -u "$rout/$f" "$out/$f" | head -n 40 | sed 's/^/    /'; } || true
        done
        changed=1
    fi
    lsnap=tests/snapshots/$id.lint
    lint=$(sed -n '/^  lint:$/,$s/^ *\([0-9][0-9]*\)  /\1  /p' "$out.log")
    cmp=$(awk -F'  ' -v id="$id" 'FILENAME == ARGV[1] { c[$2] = $1; next } { n[$2] = $1 }
        END { for (k in n) if (n[k] > c[k] + 0) print "ROSE " id ": lint " c[k] + 0 " -> " n[k] ": " k
              for (k in c) if (n[k] + 0 < c[k]) print id ": lint " c[k] " -> " n[k] + 0 ", lower to " n[k] + 0 ": " k }' \
        <(cat "$lsnap" 2>/dev/null) <(echo -n "$lint"))
    [ -z "$cmp" ] || echo "$cmp"
    if grep -q '^ROSE' <<< "$cmp"; then
        echo "$id: a lint count rose (findings in $out.log): fix it, or raise $lsnap by hand and say why in the PR"; changed=1
    elif [ $update = 1 ] && [ -n "$cmp" ]; then
        if [ -n "$lint" ]; then echo "$lint" > "$lsnap"; else rm -f "$lsnap"; fi; echo "$id: lint ceilings lowered"
    fi
    msnap=tests/snapshots/$id.metrics.json
    counts() { grep -v '^"commit' "$1" 2>/dev/null; }
    totals() { sed -n 's/^"totals": \(.*\),$/\1/p' "$1"; }
    if [ "$(counts "$msnap")" = "$(counts "$out/metrics.json")" ]; then
        echo "$id: metrics same"
    elif [ $update = 1 ]; then
        sed -e "s/^\"commit\": .*/$base_line/" -e "s/^\"commit_date\": .*/$base_date/" "$out/metrics.json" > "$msnap"; echo "$id: metrics written ($(totals "$msnap"))"
    else
        echo "$id: metrics CHANGED (now $(totals "$out/metrics.json"); diff $msnap $out/metrics.json)"; changed=1
        for k in gaps left_out; do
            o=$(totals "$msnap" | sed -n "s/.*\"$k\": \([0-9]*\).*/\1/p"); n=$(totals "$out/metrics.json" | sed -n "s/.*\"$k\": \([0-9]*\).*/\1/p")
            [ "${o:-0}" = "${n:-0}" ] || echo "$id: $k ${o:-0} -> ${n:-0}"
            [ "${n:-0}" -le "${o:-0}" ] || [ $accept = 1 ] || rose=1
        done
    fi
done
# edition pairs: the English row reads its Japanese twin's install (f[1])
for id in "${ids[@]}"; do
    IFS=$'\t' read -ra f <<< "${row[$id]}"
    a=out/snapshot/${f[1]}.cmds.json b=out/snapshot/$id.cmds.json
    if [ "${f[1]}" != "$id" ] && [[ " ${ids[*]} " == *" ${f[1]} "* ]] && [ -f "$a" ] && [ -f "$b" ] && off=$(python3 -I research/tools/cmdcheck.py --pair "$a" "$b") && [ -n "$off" ]; then
        echo "${f[1]} : $id command counts off the pair's ratio (informative):"; echo "$off" | sed 's/^/    /'
    fi
done
[ $rose = 0 ] || { echo "gaps/left_out rose or command counts moved: fix them, or rerun with --accept-gaps and explain in the PR"; exit 2; }
exit $changed
