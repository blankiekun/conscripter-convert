#!/usr/bin/env bash
# Dry-runs every game whose files are on this machine and compares its
# 0.txt with tests/snapshots/<game>.txt (sha256 and line count), so a
# change to shared code shows which games' scripts it changed.
#
#   tests/snapshot.sh [--update] [game ...]
#
# Games are found through $CONS_GAMES (default ~/.config/conscripter-convert/games),
# a folder of links named by game id to each game's folder (<game> in
# its document): saihate-no-ima, muramasa, muramasa-en (the English
# release; needs muramasa too), django, oretsuba. A game without its
# link is skipped. --update writes the snapshots of the games run.
# Exit status 1 when a snapshot differs.
set -euo pipefail
cd "$(dirname "$0")/.."
games_dir=${CONS_GAMES:-$HOME/.config/conscripter-convert/games}
bin=target/release/conscripter-convert
update=0
if [ "${1:-}" = --update ]; then update=1; shift; fi
ids=("$@")
[ ${#ids[@]} -gt 0 ] || ids=(saihate-no-ima muramasa muramasa-en django oretsuba)

cargo build --release -q
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
        *) echo "$id: unknown game"; exit 2 ;;
    esac
    if ! "$bin" "${args[@]}" -o "$out" --dry-run > "$out.log" 2>&1; then
        echo "$id: dry run failed, see $out.log"; changed=1; continue
    fi
    # The version line (build date, commit) changes with every build.
    now="$(LC_ALL=C sed -E 's/build [0-9]{4}-[0-9]{2}-[0-9]{2}, converter [0-9a-f]+(-dirty)?, CONscripter [^"]*/build VERSION/' "$out/0.txt" | sha256sum | cut -d' ' -f1) $(wc -l < "$out/0.txt")"
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
done
exit $changed
