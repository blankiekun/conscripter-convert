#!/usr/bin/env bash
# Dry-runs every game whose files are on this machine and compares its
# 0.txt with tests/snapshots/<game>.txt (sha256 and line count), so a
# change to shared code shows which games' scripts it changed.
#
#   tests/snapshot.sh [--update] [--accept-gaps] [--diff REF] [--map RULES] [game ...]
#
# Games are found through $CONS_GAMES (default ~/.config/conscripter-convert/games),
# a folder of links named by game id to each game's folder (<game> in
# its document): saihate-no-ima, muramasa, muramasa-en (the English
# release; needs muramasa too), django, oretsuba, tsukihime (the RomFS of
# 1.0.2, its ExeFS beside it as ../exefs). A game without its
# link is skipped. --update writes the snapshots of the games run.
# Exit status 1 when a snapshot differs, or when the dry run's lint finds
# files the script names that the build wouldn't write (each engine
# records the files it writes, or would: conv_engines::output::record);
# 2 when a game's gaps or left_out total rose against its metrics
# snapshot (each printed per game), unless --accept-gaps.
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
# dir, dry-runs each game with it and shows what changed in 0.txt with
# research/tools/txtdiff.py (hunks under their labels, changed lines
# counted by command). --map RULES (a file of s/old/new/ lines, or one
# such rule) checks that the change is only a rename: the rules map
# REF's commands to the new ones and nothing else may differ; it
# implies --diff origin/main.
set -euo pipefail
cd "$(dirname "$0")/.."
games_dir=${CONS_GAMES:-$HOME/.config/conscripter-convert/games}
bin=target/release/conscripter-convert
update=0 accept=0 rose=0 ref= map=()
while [ $# -gt 0 ]; do
    case $1 in
        --update) update=1 ;;
        --accept-gaps) accept=1 ;;
        --diff) ref=$2; shift ;;
        --map) if [ -f "$2" ]; then map=(--rules "$2"); else map=(--map "$2"); fi; shift ;;
        *) break ;;
    esac
    shift
done
[ ${#map[@]} = 0 ] || [ -n "$ref" ] || ref=origin/main
ids=("$@")
[ ${#ids[@]} -gt 0 ] || ids=(saihate-no-ima muramasa muramasa-en django oretsuba tsukihime)

cargo build --release -q
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
for id in "${ids[@]}"; do
    game=$games_dir/$id
    case $id in
        muramasa-en) game=$games_dir/muramasa; english=$games_dir/muramasa-en ;;
    esac
    if [ ! -e "$game" ] || { [ "$id" = muramasa-en ] && [ ! -e "$english" ]; }; then
        echo "$id: skipped (no $game)"
        continue
    fi
    out=out/snapshot/$id
    mkdir -p out/snapshot
    case $id in
        saihate-no-ima) args=(convert-game "$game" --title "Saihate no Ima COMPLETE") ;;
        muramasa) args=(convert-nss "$game" --file ma00_000.nss --chain 2000) ;;
        muramasa-en) args=(convert-nss "$game" --file ma00_000.nss --chain 2000 --english "$english") ;;
        django) args=(convert-nss "$game") ;;
        oretsuba) args=(mages convert "$game") ;;
        tsukihime) args=(typemoon convert "$game") ;;
        *) echo "$id: unknown game"; exit 2 ;;
    esac
    if ! "$bin" "${args[@]}" -o "$out" --dry-run > "$out.log" 2>&1; then
        echo "$id: dry run failed, see $out.log"; changed=1; continue
    fi
    if [ -n "$ref" ]; then
        rout=$refdir/out/$id
        mkdir -p $refdir/out
        if ! $refdir/target/release/conscripter-convert "${args[@]}" -o "$rout" --dry-run > "$rout.log" 2>&1; then
            echo "$id: dry run at $ref failed, see $rout.log"; changed=1
        elif python3 -I research/tools/txtdiff.py "$rout/0.txt" "$out/0.txt" "${map[@]}" --hunks 5 --top 10 > "$out.diff"; then
            echo "$id: 0.txt $(cat "$out.diff") (vs $ref)"
        else
            echo "$id: 0.txt differs from $ref ($rout/0.txt; full report: python3 -I research/tools/txtdiff.py $rout/0.txt $out/0.txt):"
            sed 's/^/    /' "$out.diff"; changed=1
        fi
    fi
    # The version line (build date, commit) changes with every build, and
    # so does the width of its strsp (a -dirty tree's line is longer).
    missing=$(sed -n 's/^ *\([0-9]*\)  file not in the archives.*/\1/p' "$out.log")
    if [ -n "$missing" ]; then
        echo "$id: $missing files the script names aren't written (lint in $out.log)"; changed=1
    fi
    now="$(LC_ALL=C sed -E -e 's/"build [0-9]{4}-[0-9]{2}-[0-9]{2}, converter [^"]*",([0-9]+,[0-9]+),[0-9]+,/"build VERSION",\1,W,/' \
        -e 's/build [0-9]{4}-[0-9]{2}-[0-9]{2}, converter [0-9a-f]+(-dirty)?, CONscripter [^"]*/build VERSION/' "$out/0.txt" | sha256sum | cut -d' ' -f1) $(wc -l < "$out/0.txt")"
    snap=tests/snapshots/$id.txt
    if [ $update = 1 ]; then
        echo "$now" > "$snap"; echo "$id: written"
    elif [ ! -f "$snap" ]; then
        echo "$id: no snapshot (run with --update)"; changed=1
    elif [ "$(cat "$snap")" = "$now" ]; then
        echo "$id: same"
    else
        echo "$id: CHANGED (was $(cat "$snap"), now $now; script in $out/0.txt)"; changed=1
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
[ $rose = 0 ] || { echo "gaps/left_out rose: fix them, or rerun with --accept-gaps and explain in the PR"; exit 2; }
exit $changed
